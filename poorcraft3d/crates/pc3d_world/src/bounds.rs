//! P3D-101: world-space bounds (closed min/max, inclusive) and algebra.

use crate::coords::{CellCoord, WorldPos};
use crate::scales::CELL_MM;

/// Inclusive axis-aligned bounds in world millimeters. Closed on both ends
/// (a 1 m cell is min == max corner-to-corner span of 999 mm, so "max" is
/// the last contained millimeter).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldBounds {
    pub min: WorldPos,
    pub max: WorldPos,
}

/// Horizontal-only bounds (regions; y unbounded).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldBoundsXz {
    pub min_x: i64,
    pub min_z: i64,
    pub max_x: i64,
    pub max_z: i64,
}

impl WorldBounds {
    /// The bounds containing exactly one cell.
    pub fn of_cell(c: CellCoord) -> Self {
        let o = c.origin();
        WorldBounds {
            min: o,
            max: WorldPos::from_mm(o.x + CELL_MM - 1, o.y + CELL_MM - 1, o.z + CELL_MM - 1),
        }
    }

    /// The degenerate bounds containing exactly one point.
    pub fn of_point(p: WorldPos) -> Self {
        WorldBounds { min: p, max: p }
    }

    pub fn from_points(a: WorldPos, b: WorldPos) -> Self {
        WorldBounds {
            min: WorldPos::from_mm(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)),
            max: WorldPos::from_mm(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)),
        }
    }

    pub fn contains(&self, p: WorldPos) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    pub fn intersects(&self, other: &WorldBounds) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// The covering bounds, or None for disjoint inputs.
    pub fn union(&self, other: &WorldBounds) -> Option<WorldBounds> {
        if self.intersects(other) {
            Some(WorldBounds {
                min: WorldPos::from_mm(
                    self.min.x.min(other.min.x),
                    self.min.y.min(other.min.y),
                    self.min.z.min(other.min.z),
                ),
                max: WorldPos::from_mm(
                    self.max.x.max(other.max.x),
                    self.max.y.max(other.max.y),
                    self.max.z.max(other.max.z),
                ),
            })
        } else {
            None
        }
    }

    /// Number of cells the bounds touch per axis (span / cell, rounded up
    /// onto the inclusive end). Saturates on absurd spans instead of
    /// overflowing: bounds wider than i32 cells per axis are not iterable.
    pub fn cell_extent(&self) -> Option<(u64, u64, u64)> {
        let span = |lo: i64, hi: i64| -> Option<u64> {
            let cells_lo = lo.div_euclid(CELL_MM);
            let cells_hi = hi.div_euclid(CELL_MM);
            let count = cells_hi.checked_sub(cells_lo)?.checked_add(1)?;
            u64::try_from(count).ok()
        };
        Some((
            span(self.min.x, self.max.x)?,
            span(self.min.y, self.max.y)?,
            span(self.min.z, self.max.z)?,
        ))
    }

    /// Total cell count, saturating at u64::MAX for absurd bounds.
    pub fn cell_count(&self) -> u64 {
        let (x, y, z) = match self.cell_extent() {
            Some(t) => t,
            None => return u64::MAX,
        };
        x.saturating_mul(y).saturating_mul(z)
    }

    pub fn center(self) -> WorldPos {
        WorldPos::from_mm(
            self.min.x + (self.max.x - self.min.x) / 2,
            self.min.y + (self.max.y - self.min.y) / 2,
            self.min.z + (self.max.z - self.min.z) / 2,
        )
    }
}

impl WorldBoundsXz {
    pub fn contains_xz(&self, x: i64, z: i64) -> bool {
        x >= self.min_x && x <= self.max_x && z >= self.min_z && z <= self.max_z
    }

    pub fn intersects_xz(&self, other: &WorldBoundsXz) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_z <= other.max_z
            && self.max_z >= other.min_z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// contains/intersects/union agree on edge-inclusive semantics: a point
    /// ON max is inside; bounds sharing exactly one millimeter plane
    /// intersect; truly disjoint bounds refuse to union.
    #[test]
    fn p3d101_bounds_algebra_is_edge_inclusive() {
        let a = WorldBounds::from_points(
            WorldPos::from_meters(0, 0, 0),
            WorldPos::from_meters(10, 10, 10),
        );
        assert!(a.contains(WorldPos::from_meters(0, 0, 0)));
        assert!(a.contains(WorldPos::from_meters(10, 10, 10)));
        assert!(a.contains(WorldPos::from_mm(10_000 - 1, 5_000, 3)));
        assert!(!a.contains(WorldPos::from_meters(11, 5, 5)));

        let touching = WorldBounds::from_points(
            WorldPos::from_meters(10, 0, 0),
            WorldPos::from_meters(20, 10, 10),
        );
        assert!(a.intersects(&touching), "shared plane at x=10 counts");
        let disjoint = WorldBounds::from_points(
            WorldPos::from_meters(11, 0, 0),
            WorldPos::from_meters(20, 10, 10),
        );
        assert!(!a.intersects(&disjoint));
        assert!(a.union(&disjoint).is_none());
        let u = a.union(&touching).expect("touching union");
        assert_eq!(u.max.x, WorldPos::from_meters(20, 0, 0).x);
        assert_eq!(u.min, a.min);

        // from_points normalizes inverted inputs.
        let inv = WorldBounds::from_points(
            WorldPos::from_meters(10, 10, 10),
            WorldPos::from_meters(0, 0, 0),
        );
        assert_eq!(inv, a);
    }

    /// Cell counting: exact for small spans, saturating for absurd ones,
    /// and correct across negative territory.
    #[test]
    fn p3d101_cell_counting_is_exact_and_guards_overflow() {
        let one = WorldBounds::of_cell(CellCoord { x: -3, y: 7, z: 12 });
        assert_eq!(one.cell_count(), 1);
        assert_eq!(one.cell_extent(), Some((1, 1, 1)));

        let block = WorldBounds::from_points(
            WorldPos::from_meters(-10, -10, -10),
            WorldPos::from_meters(9, 9, 9),
        );
        assert_eq!(block.cell_extent(), Some((20, 20, 20)));
        assert_eq!(block.cell_count(), 8_000);

        // A planet-wide span: per-axis cell counts still fit u64 (9.2e15),
        // but the 3-axis product saturates instead of overflowing.
        let planet = WorldBounds {
            min: WorldPos::from_mm(i64::MIN / 2, 0, i64::MIN / 2),
            max: WorldPos::from_mm(i64::MAX / 2, 0, i64::MAX / 2),
        };
        let (ex, ey, ez) = planet.cell_extent().expect("axes fit u64");
        assert_eq!(ey, 1);
        assert!(ex > 9_000_000_000_000_000 && ez > 9_000_000_000_000_000);
        assert_eq!(planet.cell_count(), u64::MAX, "the product saturates, never wraps");

        assert_eq!(block.center(), WorldPos::from_mm(-500, -500, -500));
    }

    #[test]
    fn p3d101_xz_bounds_work_horizontally() {
        use crate::coords::RegionCoord;
        let r = RegionCoord { x: 2, z: -1 }.footprint_xz();
        assert!(r.contains_xz(r.min_x, r.min_z));
        assert!(r.contains_xz(r.max_x, r.max_z));
        assert!(!r.contains_xz(r.max_x + 1, r.min_z));
        assert!(r.intersects_xz(&r));
        let other = RegionCoord { x: 3, z: -1 }.footprint_xz();
        assert!(!r.intersects_xz(&other), "adjacent regions tile, not overlap");
    }
}
