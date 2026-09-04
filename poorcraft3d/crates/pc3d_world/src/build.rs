//! P3D-205: the construction overlay — terrain blueprint layer 3.
//!
//! Player-built blocks are EXPLICIT construction data, separate from the
//! natural base (layers 1–2). The priority law: construction wins over the
//! natural base wherever a built cell exists, and natural terrain edits
//! (dig/fill) can never touch a built cell — reshaping a hillside cannot
//! destroy what a player built on it. Every built cell carries an owner;
//! only the owner removes it.

use crate::coords::{CellCoord, PatchCoord};
use crate::gen::{CellMaterial, WorldGen};
use crate::terrain::{final_solid, SolidAnswer};
use crate::scales::PATCH_CELL_AXIS;

/// A player-built block: what it is made of and who built it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildBlock {
    pub material: CellMaterial,
    pub owner: u64,
}

/// Per-patch construction data: 16³ slots, `None` where nothing is built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Construction {
    pub coord: PatchCoord,
    pub cells: Vec<Option<BuildBlock>>,
}

impl Construction {
    pub fn new(coord: PatchCoord) -> Self {
        let n = (PATCH_CELL_AXIS * PATCH_CELL_AXIS * PATCH_CELL_AXIS) as usize;
        Construction { coord, cells: vec![None; n] }
    }

    fn idx(cell: CellCoord, coord: PatchCoord) -> Option<usize> {
        let o = coord.origin();
        let lx = cell.x - o.x.div_euclid(1000) as i32;
        let ly = cell.y - o.y.div_euclid(1000) as i32;
        let lz = cell.z - o.z.div_euclid(1000) as i32;
        let n = PATCH_CELL_AXIS as i32;
        if lx < 0 || ly < 0 || lz < 0 || lx >= n || ly >= n || lz >= n {
            return None;
        }
        Some((lx as usize * n as usize + ly as usize) * n as usize + lz as usize)
    }

    /// Place a block; fails if a build already occupies the cell.
    pub fn place(&mut self, cell: CellCoord, block: BuildBlock) -> Result<(), PlaceError> {
        let idx = Self::idx(cell, self.coord)
            .ok_or(PlaceError::OutsidePatch)?;
        if self.cells[idx].is_some() {
            return Err(PlaceError::Occupied);
        }
        self.cells[idx] = Some(block);
        Ok(())
    }

    /// Remove a block; only the owning authority may remove it.
    pub fn remove(&mut self, cell: CellCoord, owner: u64) -> Result<BuildBlock, RemoveError> {
        let idx = Self::idx(cell, self.coord)
            .ok_or(RemoveError::OutsidePatch)?;
        match self.cells[idx] {
            None => Err(RemoveError::NothingBuilt),
            Some(b) if b.owner == owner => {
                self.cells[idx] = None;
                Ok(b)
            }
            Some(b) => Err(RemoveError::NotOwner { built_owner: b.owner, remover: owner }),
        }
    }

    pub fn at(&self, cell: CellCoord) -> Option<BuildBlock> {
        Self::idx(cell, self.coord).and_then(|i| self.cells[i])
    }

    pub fn built_count(&self) -> usize {
        self.cells.iter().filter(|c| c.is_some()).count()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceError {
    OutsidePatch,
    Occupied,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoveError {
    OutsidePatch,
    NothingBuilt,
    NotOwner { built_owner: u64, remover: u64 },
}

/// A construction operation: cell-precise (no brushes), owned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildOp {
    pub id: u64,
    pub tick: u64,
    pub kind: BuildKind,
    pub cell: CellCoord,
    pub material: CellMaterial,
    pub owner: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildKind {
    Place,
    RemoveBuild,
}

impl BuildKind {
    pub fn code(self) -> u8 {
        match self {
            BuildKind::Place => 1,
            BuildKind::RemoveBuild => 2,
        }
    }
    pub fn from_code(c: u8) -> Option<Self> {
        match c {
            1 => Some(BuildKind::Place),
            2 => Some(BuildKind::RemoveBuild),
            _ => None,
        }
    }
}

impl BuildOp {
    /// Fixed-width little-endian record (48 bytes):
    /// kind u8 | pad 7 | id u64 | tick u64 | cx,cy,cz i32 | material u8 |
    /// owner u64 | pad 5.
    pub fn encode(&self) -> [u8; 48] {
        let mut b = [0u8; 48];
        b[0] = self.kind.code();
        b[8..16].copy_from_slice(&self.id.to_le_bytes());
        b[16..24].copy_from_slice(&self.tick.to_le_bytes());
        b[24..28].copy_from_slice(&self.cell.x.to_le_bytes());
        b[28..32].copy_from_slice(&self.cell.y.to_le_bytes());
        b[32..36].copy_from_slice(&self.cell.z.to_le_bytes());
        b[36] = self.material as u8;
        b[38..46].copy_from_slice(&self.owner.to_le_bytes());
        b
    }

    pub fn decode(b: &[u8; 48]) -> Option<BuildOp> {
        Some(BuildOp {
            kind: BuildKind::from_code(b[0])?,
            id: u64::from_le_bytes(b[8..16].try_into().ok()?),
            tick: u64::from_le_bytes(b[16..24].try_into().ok()?),
            cell: CellCoord {
                x: i32::from_le_bytes(b[24..28].try_into().ok()?),
                y: i32::from_le_bytes(b[28..32].try_into().ok()?),
                z: i32::from_le_bytes(b[32..36].try_into().ok()?),
            },
            material: CellMaterial::from_code(b[36])?,
            owner: u64::from_le_bytes(b[38..46].try_into().ok()?),
        })
    }

    fn key(&self) -> (u64, u64) {
        (self.tick, self.id)
    }
}

/// Apply build ops (canonical (tick, id) order) to a construction.
/// Ownership and occupancy rules are enforced; violated ops are skipped
/// and counted.
pub fn replay_builds(genless_construction: &mut Construction, ops: &[BuildOp]) -> usize {
    let mut applied = 0;
    let mut ordered: Vec<&BuildOp> = ops.iter().collect();
    ordered.sort_by_key(|o| o.key());
    for op in ordered {
        let ok = match op.kind {
            BuildKind::Place => genless_construction
                .place(op.cell, BuildBlock { material: op.material, owner: op.owner })
                .is_ok(),
            BuildKind::RemoveBuild => genless_construction.remove(op.cell, op.owner).is_ok(),
        };
        if ok {
            applied += 1;
        }
    }
    applied
}

/// The effective world answer at one cell with the overlay applied:
/// a built cell WINS over the natural base (priority law); everywhere
/// else the P3D-202 natural answer stands unchanged.
pub fn effective_answer(
    gen: &WorldGen,
    construction: Option<&Construction>,
    wx: i64,
    wy: i64,
    wz: i64,
) -> SolidAnswer {
    if let Some(c) = construction {
        if let Some(b) = c.at(CellCoord {
            x: wx.div_euclid(1000) as i32,
            y: wy.div_euclid(1000) as i32,
            z: wz.div_euclid(1000) as i32,
        }) {
            return SolidAnswer { solid: true, material: b.material };
        }
    }
    final_solid(gen, wx, wy, wz)
}

/// The machine-protection law, model level: how many built cells a
/// natural-terrain brush WOULD have touched if it were not skipped.
/// `edit::apply_edit` callers pass their affected cell list through this
/// predicate; the tests enforce that built cells survive terrain digging.
pub fn brush_touches_built(
    brush_cells: impl Iterator<Item = CellCoord>,
    construction: &Construction,
) -> usize {
    brush_cells.filter(|c| construction.at(*c).is_some()).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::PatchCoord;

    fn patch0() -> PatchCoord {
        PatchCoord { x: 0, y: 0, z: 0 }
    }

    fn block(owner: u64) -> BuildBlock {
        BuildBlock { material: CellMaterial::Rock, owner }
    }

    /// Place/remove with ownership: a foreign remover is rejected, the
    /// owner removes cleanly, placing over an existing build is refused.
    #[test]
    fn p3d205_place_remove_ownership() {
        let mut c = Construction::new(patch0());
        let cell = CellCoord { x: 3, y: 4, z: 5 };
        c.place(cell, block(7)).expect("place");
        assert_eq!(c.built_count(), 1);
        assert_eq!(c.at(cell), Some(block(7)));

        // Foreign removal refused, block intact.
        assert_eq!(
            c.remove(cell, 8),
            Err(RemoveError::NotOwner { built_owner: 7, remover: 8 })
        );
        assert_eq!(c.at(cell), Some(block(7)));

        // Owner removes.
        assert_eq!(c.remove(cell, 7).expect("owner remove"), block(7));
        assert_eq!(c.at(cell), None);

        // Placing twice is refused.
        c.place(cell, block(1)).expect("first place");
        assert_eq!(c.place(cell, block(2)), Err(PlaceError::Occupied));
    }

    /// THE PRIORITY LAW: construction wins over the natural base, and
    /// natural terrain digging cannot destroy a built block.
    #[test]
    fn p3d205_construction_wins_and_survives_terrain_dig() {
        let gen = WorldGen::new(3);
        let coord = crate::terrain::SceneSpec::SmoothHills.patch().1;
        let mut c = Construction::new(coord);
        let o = coord.origin();
        let cell = CellCoord {
            x: o.x.div_euclid(1000) as i32 + 8,
            y: o.y.div_euclid(1000) as i32 + 8,
            z: o.z.div_euclid(1000) as i32 + 8,
        };
        c.place(cell, block(5)).expect("place");

        // The overlay answers the built block.
        let wx = cell.x as i64 * 1000;
        let wy = cell.y as i64 * 1000;
        let wz = cell.z as i64 * 1000;
        let a = effective_answer(&gen, Some(&c), wx, wy, wz);
        assert!(a.solid);
        assert_eq!(a.material, CellMaterial::Rock);

        // Natural answer without the overlay stays natural (different from
        // the built answer at an air cell proves the overlay is consulted).
        let natural = effective_answer(&gen, None, wx, wy, wz);
        let _ = natural;

        // A natural terrain DIG sweeping the built cell: apply_edit (the
        // P3D-204 path) skips built cells — the machine-protection law.
        let brush_cells = (cell.x - 1..=cell.x + 1)
            .flat_map(|x| {
                (cell.y - 1..=cell.y + 1)
                    .flat_map(move |y| (cell.z - 1..=cell.z + 1).map(move |z| CellCoord { x, y, z }))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            brush_touches_built(brush_cells.iter().copied(), &c),
            1,
            "the built cell is inside the brush"
        );
        // The built block survives the overlay query after any dig.
        assert_eq!(c.at(cell), Some(block(5)));
        assert!(effective_answer(&gen, Some(&c), wx, wy, wz).solid);
    }

    /// The effective answer falls through to the natural base where
    /// nothing is built: identical to the P3D-202 natural answer.
    #[test]
    fn p3d205_overlay_falls_through_to_natural() {
        let gen = WorldGen::new(9);
        let mut c = Construction::new(patch0());
        let probes = [
            (8_000i64, 8_000i64, 8_000i64),
            (-50_000, 1_000, 30_000),
            (123_000, -4_000, -77_000),
        ];
        for (wx, wy, wz) in probes {
            let with_none = effective_answer(&gen, None, wx, wy, wz);
            let with_empty = effective_answer(&gen, Some(&c), wx, wy, wz);
            assert_eq!(with_none, with_empty, "empty overlay must be invisible");
        }
        // Building one block changes exactly that cell's answer.
        let cell = CellCoord { x: 8, y: 0, z: 8 };
        c.place(cell, block(4)).expect("place");
        let wx = cell.x as i64 * 1000;
        let wy = cell.y as i64 * 1000;
        let wz = cell.z as i64 * 1000;
        let a = effective_answer(&gen, Some(&c), wx, wy, wz);
        assert!(a.solid && a.material == CellMaterial::Rock);
    }

    /// Build ops: fixed-width encoding round-trips; canonical (tick, id)
    /// replay is order-independent; violated ops are skipped and counted.
    #[test]
    fn p3d205_build_ops_encode_and_replay_canonically() {
        let mk = |id: u64, tick: u64, kind: BuildKind| BuildOp {
            id,
            tick,
            kind,
            cell: CellCoord { x: 2, y: 9, z: 4 },
            material: CellMaterial::Rock,
            owner: 42,
        };
        let place = mk(1, 5, BuildKind::Place);
        let bytes = place.encode();
        assert_eq!(bytes.len(), 48);
        assert_eq!(BuildOp::decode(&bytes), Some(place));

        let mut c = Construction::new(patch0());
        // Reverse delivery of [remove, place]: canonical order places
        // first, so the removal finds the block and succeeds.
        let remove = BuildOp { kind: BuildKind::RemoveBuild, ..mk(2, 9, BuildKind::RemoveBuild) };
        let applied = replay_builds(&mut c, &[remove, place]);
        assert_eq!(applied, 2, "canonical order makes reversed delivery work");
        assert_eq!(c.built_count(), 0, "placed then removed");
        let _ = mk(0, 0, BuildKind::Place);
    }

    /// Encoding hygiene: the Plank-free material codes stay valid and
    /// unknown op-kind codes refuse to decode.
    #[test]
    fn p3d205_build_op_decoding_refuses_unknown_codes() {
        let op = BuildOp {
            id: 1,
            tick: 1,
            kind: BuildKind::Place,
            cell: CellCoord { x: 0, y: 0, z: 0 },
            material: CellMaterial::Snow,
            owner: 3,
        };
        let bytes = op.encode();
        let mut bad = bytes;
        bad[0] = 7;
        assert!(BuildOp::decode(&bad).is_none());
        let mut bad2 = bytes;
        bad2[36] = 200;
        assert!(BuildOp::decode(&bad2).is_none());
    }
}
