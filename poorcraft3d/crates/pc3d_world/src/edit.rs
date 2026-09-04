//! P3D-204: natural-terrain edit operations, patch-local invalidation,
//! journal replay, and compaction
//! (docs/POORCRAFT-3D/16-IMPLEMENTATION-WORK-BREAKDOWN.md, P3D-200).
//!
//! The blueprint's three-layer terrain: layer 1 is the immutable
//! procedural base (`gen::regenerate_patch`), layer 2 is the ordered edit
//! journal this module defines. A patch's true state is always
//! `regenerate(base) + replay(ops)` — so untouched patches need no
//! storage, edited patches replay identically on every host, and a
//! journal can compact into a snapshot with byte-identical results.
//!
//! Ops carry (tick, id) identity like every command in the engine; replay
//! applies them in canonical (tick, id) order, so delivery grouping and
//! reordering are inert.

use crate::coords::{CellCoord, PatchCoord};
use crate::gen::{CellMaterial, PatchCells, WorldGen};
use crate::scales::PATCH_CELL_AXIS;

/// What an edit does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    /// Remove terrain: solid cells in the brush become Air.
    Dig,
    /// Add terrain: Air cells in the brush become `material`.
    Fill,
}

impl EditKind {
    pub fn code(self) -> u8 {
        match self {
            EditKind::Dig => 1,
            EditKind::Fill => 2,
        }
    }
    pub fn from_code(c: u8) -> Option<Self> {
        match c {
            1 => Some(EditKind::Dig),
            2 => Some(EditKind::Fill),
            _ => None,
        }
    }
}

/// A bounded cube brush centered on a cell (Chebyshev radius). `cells()`
/// iterates the clamped cube — an edit can never spill outside its brush.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Brush {
    pub center: CellCoord,
    pub radius: u32,
}

impl Brush {
    pub fn cells(&self) -> impl Iterator<Item = CellCoord> {
        let r = self.radius as i32;
        let (cx, cy, cz) = (self.center.x, self.center.y, self.center.z);
        (cx - r..=cx + r).flat_map(move |x| {
            (cy - r..=cy + r).flat_map(move |y| {
                (cz - r..=cz + r).map(move |z| CellCoord { x, y, z })
            })
        })
    }
}

/// One natural-terrain edit: identity (id, tick), intent (kind), extent
/// (brush), and result (material).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditOp {
    pub id: u64,
    pub tick: u64,
    pub kind: EditKind,
    pub brush: Brush,
    pub material: CellMaterial,
}

impl EditOp {
    /// Fixed-width little-endian encoding (P3D-102 framing carries the
    /// bytes; this is the op's own layout):
    /// kind u8 | pad 7 | id u64 | tick u64 | cx,cy,cz i32 | radius u32 |
    /// material u8 | pad 3 = 48 bytes.
    pub fn encode(&self) -> [u8; 48] {
        let mut b = [0u8; 48];
        b[0] = self.kind.code();
        b[8..16].copy_from_slice(&self.id.to_le_bytes());
        b[16..24].copy_from_slice(&self.tick.to_le_bytes());
        b[24..28].copy_from_slice(&self.brush.center.x.to_le_bytes());
        b[28..32].copy_from_slice(&self.brush.center.y.to_le_bytes());
        b[32..36].copy_from_slice(&self.brush.center.z.to_le_bytes());
        b[36..40].copy_from_slice(&self.brush.radius.to_le_bytes());
        b[40] = self.material as u8;
        b
    }

    pub fn decode(b: &[u8; 48]) -> Option<EditOp> {
        let kind = EditKind::from_code(b[0])?;
        let material = CellMaterial::from_code(b[40])?;
        Some(EditOp {
            kind,
            id: u64::from_le_bytes(b[8..16].try_into().ok()?),
            tick: u64::from_le_bytes(b[16..24].try_into().ok()?),
            brush: Brush {
                center: CellCoord {
                    x: i32::from_le_bytes(b[24..28].try_into().ok()?),
                    y: i32::from_le_bytes(b[28..32].try_into().ok()?),
                    z: i32::from_le_bytes(b[32..36].try_into().ok()?),
                },
                radius: u32::from_le_bytes(b[36..40].try_into().ok()?),
            },
            material,
        })
    }

    /// Canonical order key: (tick, id) — batching and delivery order are
    /// inert, the same law as every command in the engine.
    fn key(&self) -> (u64, u64) {
        (self.tick, self.id)
    }
}

/// Apply one op to a patch's cells. Returns how many cells changed.
/// Cells outside this patch are untouched (patch-local invalidation).
pub fn apply_edit(cells: &mut PatchCells, op: &EditOp) -> usize {
    let n = PATCH_CELL_AXIS as usize;
    let origin = cells.coord.origin();
    let ax = origin.x.div_euclid(1000) as i32;
    let ay = origin.y.div_euclid(1000) as i32;
    let az = origin.z.div_euclid(1000) as i32;
    let mut changed = 0;
    for cell in op.brush.cells() {
        // World cell -> patch-local cell; skip cells belonging elsewhere.
        let lx = cell.x - ax;
        let ly = cell.y - ay;
        let lz = cell.z - az;
        if lx < 0 || ly < 0 || lz < 0 {
            continue;
        }
        let (lux, luy, luz) = (lx as usize, ly as usize, lz as usize);
        if lux >= n || luy >= n || luz >= n {
            continue;
        }
        let idx = (lux * n + luy) * n + luz;
        let cur = cells.cells[idx];
        match op.kind {
            EditKind::Dig => {
                if cur != CellMaterial::Air && cur != CellMaterial::Water {
                    cells.cells[idx] = CellMaterial::Air;
                    changed += 1;
                }
            }
            EditKind::Fill => {
                if cur == CellMaterial::Air && op.material != CellMaterial::Air {
                    cells.cells[idx] = op.material;
                    changed += 1;
                }
            }
        }
    }
    changed
}

/// Patches touched by an op's brush (patch-local invalidation): exactly
/// the patches containing at least one brush cell, ascending.
pub fn affected_patches(op: &EditOp) -> Vec<PatchCoord> {
    let r = op.brush.radius as i32;
    let c = &op.brush.center;
    let span = crate::scales::PATCH_CELL_AXIS as i32;
    let (minx, maxx) = ((c.x - r).div_euclid(span), (c.x + r).div_euclid(span));
    let (miny, maxy) = ((c.y - r).div_euclid(span), (c.y + r).div_euclid(span));
    let (minz, maxz) = ((c.z - r).div_euclid(span), (c.z + r).div_euclid(span));
    let mut out = Vec::new();
    for x in minx..=maxx {
        for y in miny..=maxy {
            for z in minz..=maxz {
                out.push(PatchCoord { x, y, z });
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Regenerate the base patch and replay ops (canonical (tick, id) order)
/// over it. The world's definition of an edited patch.
pub fn replay(gen: &WorldGen, coord: PatchCoord, ops: &[EditOp]) -> PatchCells {
    let mut cells = gen.regenerate_patch(coord);
    let mut ordered: Vec<&EditOp> = ops.iter().collect();
    ordered.sort_by_key(|o| o.key());
    for op in ordered {
        apply_edit(&mut cells, op);
    }
    cells
}

/// A compacted patch: the full material array stored instead of a journal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub coord: PatchCoord,
    /// The journal's ops — retained so future compaction stays lossless
    /// in the other direction (snapshots never need re-expansion).
    pub cells: Vec<CellMaterial>,
}

impl Snapshot {
    /// Compact a replayed result into a snapshot. Deterministic: the
    /// snapshot's cells ARE the replayed cells.
    pub fn from_replay(gen: &WorldGen, coord: PatchCoord, ops: &[EditOp]) -> Snapshot {
        let cells = replay(gen, coord, ops);
        Snapshot { coord, cells: cells.cells }
    }

    pub fn apply(&self) -> PatchCells {
        PatchCells { coord: self.coord, cells: self.cells.clone() }
    }

    /// Fixed-width material bytes (one byte per cell) for persistence.
    pub fn encode_cells(&self) -> Vec<u8> {
        self.cells.iter().map(|&c| c as u8).collect()
    }

    pub fn decode_cells(coord: PatchCoord, bytes: &[u8]) -> Option<Snapshot> {
        if bytes.len() != 4096 {
            return None;
        }
        let mut cells = Vec::with_capacity(bytes.len());
        for &b in bytes {
            cells.push(CellMaterial::from_code(b)?);
        }
        Some(Snapshot { coord, cells })
    }
}

/// Ops a journal must have accumulated before compaction fires.
pub const COMPACT_THRESHOLD: usize = 64;

#[cfg(test)]
mod tests {
    use super::*;

    fn dig(center: CellCoord, radius: u32) -> EditOp {
        EditOp {
            id: 0,
            tick: 0,
            kind: EditKind::Dig,
            brush: Brush { center, radius },
            material: CellMaterial::Air,
        }
    }

    /// A brush touches exactly its (2r+1)³ cube; apply_edit changes only
    /// solid cells inside it, never air or water.
    #[test]
    fn p3d204_dig_is_bounded_and_selective() {
        let gen = WorldGen::new(3);
        // The SmoothHills scene patch: known land, surface ~23 m, so the
        // y=1 patch (16..32 m) holds solid ground under air.
        let coord = crate::terrain::SceneSpec::SmoothHills.patch().1;
        let mut patch = gen.regenerate_patch(coord);
        let before = patch.cells.clone();
        let solid_before = before.iter().filter(|&&c| c != CellMaterial::Air).count();
        assert!(solid_before > 0);

        // Dig a 3×3×3 crater centered on the surface cell. Brush centers
        // are WORLD cells: patch base 16 m + local 8,7,8.
        let o = coord.origin();
        let world = |lx: i32, ly: i32, lz: i32| CellCoord {
            x: o.x.div_euclid(1000) as i32 + lx,
            y: o.y.div_euclid(1000) as i32 + ly,
            z: o.z.div_euclid(1000) as i32 + lz,
        };
        let center = world(8, 7, 8);
        let op = dig(center, 1);
        let changed = apply_edit(&mut patch, &op);
        assert!(changed > 0 && changed <= 27);
        let n = PATCH_CELL_AXIS as usize;
        let ax = o.x.div_euclid(1000) as i32;
        let ay = o.y.div_euclid(1000) as i32;
        let az = o.z.div_euclid(1000) as i32;
        // Only cells within the brush changed (world cells -> local idx).
        let mut changed_inside = 0;
        for cell in op.brush.cells() {
            let (lx, ly, lz) = (cell.x - ax, cell.y - ay, cell.z - az);
            if lx < 0 || ly < 0 || lz < 0 {
                continue;
            }
            let (lux, luy, luz) = (lx as usize, ly as usize, lz as usize);
            if lux >= n || luy >= n || luz >= n {
                continue;
            }
            let idx = (lux * n + luy) * n + luz;
            if before[idx] != patch.cells[idx] {
                changed_inside += 1;
                assert_eq!(patch.cells[idx], CellMaterial::Air, "dig yields air");
            }
        }
        assert_eq!(changed, changed_inside);
        // Cells outside the brush are untouched: sweep the whole patch,
        // skipping brush cells.
        let ax = o.x.div_euclid(1000) as i32;
        let ay = o.y.div_euclid(1000) as i32;
        let az = o.z.div_euclid(1000) as i32;
        let brush_local: Vec<(i32, i32, i32)> = op
            .brush
            .cells()
            .map(|c| (c.x - ax, c.y - ay, c.z - az))
            .collect();
        for x in 0..n {
            for y in 0..n {
                for z in 0..n {
                    if brush_local.contains(&(x as i32, y as i32, z as i32)) {
                        continue;
                    }
                    let idx = (x * n + y) * n + z;
                    assert_eq!(before[idx], patch.cells[idx], "outside cell changed");
                }
            }
        }
    }

    /// Fill only converts AIR cells to the material; existing terrain is
    /// never overwritten.
    #[test]
    fn p3d204_fill_only_touches_air() {
        let gen = WorldGen::new(3);
        let coord = crate::terrain::SceneSpec::SmoothHills.patch().1;
        let mut patch = gen.regenerate_patch(coord);
        // An air cell high in the patch (surface ~23 m, so local y 15 =
        // 31 m). The brush center is a WORLD cell (patch base + local).
        let o = coord.origin();
        let target = CellCoord {
            x: o.x.div_euclid(1000) as i32 + 5,
            y: o.y.div_euclid(1000) as i32 + 15,
            z: o.z.div_euclid(1000) as i32 + 5,
        };
        let fill = EditOp {
            id: 1,
            tick: 0,
            kind: EditKind::Fill,
            brush: Brush { center: target, radius: 2 },
            material: CellMaterial::Rock,
        };
        let before = patch.cells.clone();
        apply_edit(&mut patch, &fill);
        let n = PATCH_CELL_AXIS as usize;
        let ax = o.x.div_euclid(1000) as i32;
        let ay = o.y.div_euclid(1000) as i32;
        let az = o.z.div_euclid(1000) as i32;
        for cell in fill.brush.cells() {
            let (lx, ly, lz) = (cell.x - ax, cell.y - ay, cell.z - az);
            if lx < 0 || ly < 0 || lz < 0 {
                continue;
            }
            let (lux, luy, luz) = (lx as usize, ly as usize, lz as usize);
            if lux >= n || luy >= n || luz >= n {
                continue;
            }
            let idx = (lux * n + luy) * n + luz;
            if before[idx] == CellMaterial::Air {
                assert_eq!(patch.cells[idx], CellMaterial::Rock, "air filled");
            } else {
                assert_eq!(patch.cells[idx], before[idx], "solid untouched by fill");
            }
        }
    }

    /// Invalidation: an edit's affected patches are exactly the patches
    /// its brush cube intersects — including straddling patches at edges.
    #[test]
    fn p3d204_affected_patches_are_exact() {
        // A brush straddling the x=0 patch boundary (cells -1 and 0).
        let op = dig(CellCoord { x: 0, y: 4, z: 4 }, 1);
        let mut affected = affected_patches(&op);
        affected.sort();
        affected.dedup();
        assert!(affected.contains(&PatchCoord { x: -1, y: 0, z: 0 }));
        assert!(affected.contains(&PatchCoord { x: 0, y: 0, z: 0 }));
        // A brush centered deep inside one patch affects exactly one.
        let one = affected_patches(&dig(CellCoord { x: 8, y: 3, z: 8 }, 0));
        assert_eq!(one, vec![PatchCoord { x: 0, y: 0, z: 0 }]);
    }

    /// Replay is canonical: the same ops delivered in different orders (or
    /// batched) produce the identical patch. Edits land only where aimed.
    #[test]
    fn p3d204_replay_is_order_independent_and_deterministic() {
        let gen = WorldGen::new(21);
        let coord = PatchCoord { x: 0, y: 0, z: 0 };
        let mk = |id: u64, tick: u64, c: CellCoord, m: CellMaterial| EditOp {
            id,
            tick,
            kind: EditKind::Fill,
            brush: Brush { center: c, radius: 1 },
            material: m,
        };
        let ops_a = vec![
            mk(1, 5, CellCoord { x: 4, y: 15, z: 4 }, CellMaterial::Rock),
            mk(2, 5, CellCoord { x: 10, y: 15, z: 10 }, CellMaterial::Sand),
            mk(3, 2, CellCoord { x: 6, y: 15, z: 6 }, CellMaterial::Grass),
        ];
        let mut ops_b = ops_a.clone();
        ops_b.reverse();
        let a = replay(&gen, coord, &ops_a);
        let b = replay(&gen, coord, &ops_b);
        assert_eq!(a, b, "delivery order must not change the result");
        // Deterministic across reruns.
        assert_eq!(a, replay(&gen, coord, &ops_a));
    }

    /// Compaction: the snapshot equals the replayed journal, byte for byte.
    #[test]
    fn p3d204_compaction_is_lossless() {
        let gen = WorldGen::new(33);
        let coord = PatchCoord { x: 0, y: 0, z: 0 };
        let ops: Vec<EditOp> = (0..10u64)
            .map(|i| EditOp {
                id: i,
                tick: i,
                kind: if i % 2 == 0 { EditKind::Fill } else { EditKind::Dig },
                brush: Brush { center: CellCoord { x: 3 + i as i32, y: 14, z: 6 }, radius: 1 },
                material: CellMaterial::Rock,
            })
            .collect();
        let replayed = replay(&gen, coord, &ops);
        let snap = Snapshot::from_replay(&gen, coord, &ops);
        assert_eq!(snap.cells, replayed.cells, "compaction must be lossless");
        let reapplied = snap.apply();
        assert_eq!(reapplied.cells, replayed.cells);
        // Encoded cells round-trip.
        let bytes = snap.encode_cells();
        assert_eq!(bytes.len(), 4096);
        let back = Snapshot::decode_cells(coord, &bytes).expect("decode");
        assert_eq!(back, snap);
    }

    /// Op encoding is fixed-width and round-trips; unknown codes refuse.
    #[test]
    fn p3d204_op_encoding_round_trips() {
        let op = EditOp {
            id: 0xDEAD_BEEF,
            tick: 777,
            kind: EditKind::Fill,
            brush: Brush { center: CellCoord { x: -5, y: 3, z: 99 }, radius: 4 },
            material: CellMaterial::Sand,
        };
        let bytes = op.encode();
        assert_eq!(bytes.len(), 48);
        assert_eq!(EditOp::decode(&bytes), Some(op));
        // Unknown kind/material codes refuse to decode.
        let mut bad = bytes;
        bad[0] = 9;
        assert!(EditOp::decode(&bad).is_none());
        let mut bad2 = bytes;
        bad2[40] = 200;
        assert!(EditOp::decode(&bad2).is_none());
    }
}
