//! P3D-101: world coordinates and the region/patch/cell hierarchy.
//!
//! [`WorldPos`] is the stable large-world identity: signed 64-bit
//! millimeters (±9.2e12 meters — no floating-point drift, ever). Every
//! mapping down the hierarchy uses Euclidean floor division, so negative
//! coordinates behave like a globe, not like C truncation: the cell
//! containing x = -0.5 m is cell -1, whose local origin is x = -1 m.

use crate::scales::{CELL_MM, MM_PER_METER, PATCH_MM, REGION_MM};

/// A point in the world, millimeters. Derived from meters × 1000.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct WorldPos {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl WorldPos {
    pub const fn from_meters(x: i64, y: i64, z: i64) -> Self {
        WorldPos { x: x * MM_PER_METER, y: y * MM_PER_METER, z: z * MM_PER_METER }
    }
    pub const fn from_mm(x: i64, y: i64, z: i64) -> Self {
        WorldPos { x, y, z }
    }
    /// The 1 m cell containing this position (floor semantics).
    pub fn cell(self) -> CellCoord {
        CellCoord {
            x: self.x.div_euclid(CELL_MM) as i32,
            y: self.y.div_euclid(CELL_MM) as i32,
            z: self.z.div_euclid(CELL_MM) as i32,
        }
    }
    /// The 16 m patch containing this position.
    pub fn patch(self) -> PatchCoord {
        PatchCoord {
            x: self.x.div_euclid(PATCH_MM) as i32,
            y: self.y.div_euclid(PATCH_MM) as i32,
            z: self.z.div_euclid(PATCH_MM) as i32,
        }
    }
    /// The 256 m macro region containing this position (x/z only).
    pub fn region(self) -> RegionCoord {
        RegionCoord {
            x: self.x.div_euclid(REGION_MM) as i32,
            z: self.z.div_euclid(REGION_MM) as i32,
        }
    }
}

/// A 1 m construction/collision cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct CellCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl CellCoord {
    /// The cell's minimum corner in world millimeters.
    pub fn origin(self) -> WorldPos {
        WorldPos::from_mm(
            self.x as i64 * CELL_MM,
            self.y as i64 * CELL_MM,
            self.z as i64 * CELL_MM,
        )
    }
    pub fn patch(self) -> PatchCoord {
        PatchCoord {
            x: self.x.div_euclid(PATCH_CELL_AXIS()),
            y: self.y.div_euclid(PATCH_CELL_AXIS()),
            z: self.z.div_euclid(PATCH_CELL_AXIS()),
        }
    }
}

// Patch cells per axis as i32 for CellCoord math (scales are i64 there).
fn PATCH_CELL_AXIS() -> i32 {
    crate::scales::PATCH_CELL_AXIS as i32
}

/// A 16 m terrain patch — the streaming/edit/mesh/persistence unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PatchCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl PatchCoord {
    /// The patch's minimum corner in world millimeters.
    pub fn origin(self) -> WorldPos {
        WorldPos::from_mm(
            self.x as i64 * PATCH_MM,
            self.y as i64 * PATCH_MM,
            self.z as i64 * PATCH_MM,
        )
    }
    /// The patch's 16 m closed footprint as world bounds.
    pub fn footprint(self) -> crate::bounds::WorldBounds {
        let o = self.origin();
        crate::bounds::WorldBounds {
            min: o,
            max: WorldPos::from_mm(
                o.x + PATCH_MM - 1,
                o.y + PATCH_MM - 1,
                o.z + PATCH_MM - 1,
            ),
        }
    }
    pub fn region(self) -> RegionCoord {
        RegionCoord {
            x: self.x.div_euclid(REGION_PATCH_AXIS_I32()),
            z: self.z.div_euclid(REGION_PATCH_AXIS_I32()),
        }
    }
}

fn REGION_PATCH_AXIS_I32() -> i32 {
    crate::scales::REGION_PATCH_AXIS as i32
}

/// A 256 m macro region (horizontal only — y is unbounded).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RegionCoord {
    pub x: i32,
    pub z: i32,
}

impl RegionCoord {
    /// The region's min corner (y = 0: regions span all heights).
    pub fn origin(self) -> WorldPos {
        WorldPos::from_mm(self.x as i64 * REGION_MM, 0, self.z as i64 * REGION_MM)
    }
    /// Horizontal (y-unbounded) footprint of the region.
    pub fn footprint_xz(self) -> crate::bounds::WorldBoundsXz {
        let o = self.origin();
        crate::bounds::WorldBoundsXz {
            min_x: o.x,
            min_z: o.z,
            max_x: o.x + REGION_MM - 1,
            max_z: o.z + REGION_MM - 1,
        }
    }
}

/// Position in millimeters relative to a patch's origin (0 .. PATCH_MM).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LocalPos {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl WorldPos {
    /// Split into (patch, local) — the streaming/persistence decomposition.
    pub fn patch_local(self) -> (PatchCoord, LocalPos) {
        let p = self.patch();
        let o = p.origin();
        (
            p,
            LocalPos { x: self.x - o.x, y: self.y - o.y, z: self.z - o.z },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scales::REGION_PATCH_AXIS;

    /// Floor semantics across zero: -0.5 m lives in cell -1, whose origin is
    /// -1 m. C-style truncation would put it in cell 0 — wrong for a world.
    #[test]
    fn p3d101_negative_coordinates_floor_like_a_globe() {
        let p = WorldPos::from_mm(-500, -500, -500);
        assert_eq!(p.cell(), CellCoord { x: -1, y: -1, z: -1 });
        assert_eq!(p.cell().origin(), WorldPos::from_mm(-1_000, -1_000, -1_000));
        assert_eq!(p.patch(), PatchCoord { x: -1, y: -1, z: -1 });
        assert_eq!(p.patch().origin(), WorldPos::from_mm(-16_000, -16_000, -16_000));
        assert_eq!(p.region(), RegionCoord { x: -1, z: -1 });
        // Just above zero is cell 0.
        assert_eq!(WorldPos::from_mm(1, 1, 1).cell(), CellCoord { x: 0, y: 0, z: 0 });
    }

    /// pos -> cell -> origin round-trip: every position projects into its
    /// cell's span. Sampled across all eight octants and both sides of
    /// patch boundaries.
    #[test]
    fn p3d101_round_trips_hold_across_signs_and_boundaries() {
        let samples = [
            0i64, 1, 999, 1_000, 15_999, 16_000, -1, -999, -1_000, -16_000, -16_001,
            256_000, -256_001, 4_096_000, -4_096_001,
        ];
        for &sx in &samples {
            for &sy in &samples {
                for &sz in &samples {
                    let p = WorldPos::from_mm(sx, sy, sz);
                    let c = p.cell();
                    let o = c.origin();
                    assert!(p.x >= o.x && p.x < o.x + CELL_MM, "x {sx}");
                    assert!(p.y >= o.y && p.y < o.y + CELL_MM, "y {sy}");
                    assert!(p.z >= o.z && p.z < o.z + CELL_MM, "z {sz}");
                    // patch_local decomposes and recomposes exactly.
                    let (patch, local) = p.patch_local();
                    assert!(local.x >= 0 && local.x < PATCH_MM);
                    assert!(local.y >= 0 && local.y < PATCH_MM);
                    assert!(local.z >= 0 && local.z < PATCH_MM);
                    assert_eq!(patch.origin().x + local.x, p.x);
                    assert_eq!(patch.origin().y + local.y, p.y);
                    assert_eq!(patch.origin().z + local.z, p.z);
                    // Hierarchy nests: cell ⊂ patch ⊂ region.
                    assert_eq!(p.patch(), c.patch());
                    assert_eq!(p.region(), patch.region());
                }
            }
        }
    }

    /// A patch footprint contains exactly its 16³ cells and sits inside one
    /// region; region footprints tile without overlap for a strip of neighbors.
    #[test]
    fn p3d101_footprints_nest_and_tile() {
        let fp = PatchCoord { x: -1, y: 0, z: 2 }.footprint();
        assert_eq!(fp.cell_count(), 16 * 16 * 16);
        // All eight corners map back to this patch.
        for &cx in &[fp.min.x, fp.max.x] {
            for &cy in &[fp.min.y, fp.max.y] {
                for &cz in &[fp.min.z, fp.max.z] {
                    assert_eq!(WorldPos::from_mm(cx, cy, cz).patch().footprint(), fp);
                }
            }
        }
        // Neighbor patches tile exactly (closed bounds, +1 step).
        let a = PatchCoord { x: 0, y: 0, z: 0 }.footprint();
        let b = PatchCoord { x: 1, y: 0, z: 0 }.footprint();
        assert_eq!(a.max.x + 1, b.min.x);
        assert!(!a.intersects(&b));

        // Regions tile in x/z and contain 16×16 patches per level.
        let r = RegionCoord { x: -1, z: -1 }.footprint_xz();
        assert_eq!((r.max_x - r.min_x + 1) / PATCH_MM, REGION_PATCH_AXIS as i64);        let right = RegionCoord { x: 0, z: -1 }.footprint_xz();
        assert_eq!(r.max_x + 1, right.min_x);
        assert_eq!(
            PatchCoord { x: -1, y: -1, z: -1 }.region(),
            RegionCoord { x: -1, z: -1 }
        );
        assert_eq!(
            PatchCoord { x: 15, y: 5, z: 15 }.region(),
            RegionCoord { x: 0, z: 0 }
        );
    }
}
