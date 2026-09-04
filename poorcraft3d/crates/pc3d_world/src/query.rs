//! P3D-101: bounded spatial queries over the patch/region hierarchy.
//!
//! Every query is O(patches touched) with a hard cap: a caller that asks
//! for a planet's worth of patches gets an error naming the count, never a
//! hang. This is the substrate's promise behind "one patch rebuild
//! independent of total world size".

use crate::bounds::{WorldBounds, WorldBoundsXz};
use crate::coords::{PatchCoord, RegionCoord};
use crate::scales::{MAX_QUERY_PATCHES, PATCH_MM, PATCH_CELL_AXIS, REGION_MM, REGION_PATCH_AXIS};

/// Why a query refused to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryError {
    /// The bound touches more patches than MAX_QUERY_PATCHES — callers must
    /// split the work (streaming rings do exactly that).
    TooManyPatches { requested: u64, cap: usize },
    /// The bound's per-axis span exceeds i32 patch indices.
    TooLarge,
}

/// All patches whose 16 m footprints intersect `bounds`, in
/// (x, y, z) ascending order. Refuses instead of hanging when the bound
/// covers more than the cap.
pub fn patches_touching(bounds: &WorldBounds) -> Result<Vec<PatchCoord>, QueryError> {
    let min = bounds.min.patch();
    let max = bounds.max.patch();
    let count = |lo: i32, hi: i32| -> Option<u64> {
        u64::try_from(i64::from(hi) - i64::from(lo) + 1).ok()
    };
    let (nx, ny, nz) = match (
        count(min.x, max.x),
        count(min.y, max.y),
        count(min.z, max.z),
    ) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => return Err(QueryError::TooLarge),
    };
    let total = nx.saturating_mul(ny).saturating_mul(nz);
    if total > MAX_QUERY_PATCHES as u64 {
        return Err(QueryError::TooManyPatches { requested: total, cap: MAX_QUERY_PATCHES });
    }
    let mut out = Vec::with_capacity(total as usize);
    for x in min.x..=max.x {
        for y in min.y..=max.y {
            for z in min.z..=max.z {
                out.push(PatchCoord { x, y, z });
            }
        }
    }
    Ok(out)
}

/// All macro regions whose 256 m footprints intersect the horizontal span
/// of `bounds`. Regions are cheap; the same cap philosophy applies.
pub fn regions_touching(bounds: &WorldBounds) -> Result<Vec<RegionCoord>, QueryError> {
    let min = bounds.min.region();
    let max = bounds.max.region();
    let nx = u64::try_from(i64::from(max.x) - i64::from(min.x) + 1)
        .map_err(|_| QueryError::TooLarge)?;
    let nz = u64::try_from(i64::from(max.z) - i64::from(min.z) + 1)
        .map_err(|_| QueryError::TooLarge)?;
    if nx.saturating_mul(nz) > MAX_QUERY_PATCHES as u64 {
        return Err(QueryError::TooManyPatches {
            requested: nx.saturating_mul(nz),
            cap: MAX_QUERY_PATCHES,
        });
    }
    let mut out = Vec::with_capacity((nx * nz) as usize);
    for x in min.x..=max.x {
        for z in min.z..=max.z {
            out.push(RegionCoord { x, z });
        }
    }
    Ok(out)
}

/// The x/z patch columns of one macro region at y = 0 (16×16 = 256 patches).
/// Regions are horizontal; a caller with a y range filters the column set.
pub fn patches_in_region(region: RegionCoord) -> Vec<PatchCoord> {
    let base = region.origin();
    let px = base.x.div_euclid(PATCH_MM) as i32;
    let pz = base.z.div_euclid(PATCH_MM) as i32;
    let axis = REGION_PATCH_AXIS as i32;
    let mut out = Vec::with_capacity((axis * axis) as usize);
    for x in px..px + axis {
        for z in pz..pz + axis {
            out.push(PatchCoord { x, y: 0, z });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::WorldPos;

    /// A bounds spanning -1 m..16 m crosses THREE patch columns per axis
    /// ([-16,0), [0,16), [16,32)) → 27 patches, ascending, all intersecting.
    #[test]
    fn p3d101_patch_query_returns_exactly_the_touched_set() {
        let b = WorldBounds::from_points(
            WorldPos::from_meters(-1, -1, -1),
            WorldPos::from_meters(16, 16, 16),
        );
        let patches = patches_touching(&b).expect("small query");
        assert_eq!(patches.len(), 27);
        assert_eq!(
            patches[0],
            PatchCoord { x: -1, y: -1, z: -1 },
            "ascending order starts at the min corner"
        );
        assert_eq!(patches[26], PatchCoord { x: 1, y: 1, z: 1 });
        // Every returned patch genuinely intersects the bound.
        for p in &patches {
            assert!(p.footprint().intersects(&b));
        }
        // Single-point bound -> exactly one patch.
        let one = patches_touching(&WorldBounds::of_point(WorldPos::from_meters(20, 0, 20)))
            .expect("point query");
        assert_eq!(one, vec![PatchCoord { x: 1, y: 0, z: 1 }]);
        // A bound inside one patch returns exactly that patch.
        let inner = WorldBounds::from_points(
            WorldPos::from_meters(1, 1, 1),
            WorldPos::from_meters(2, 2, 2),
        );
        assert_eq!(
            patches_touching(&inner).expect("inner"),
            vec![PatchCoord { x: 0, y: 0, z: 0 }]
        );
    }

    /// A planet-sized bound is REFUSED with the count, never iterated —
    /// the substrate's no-hang promise.
    #[test]
    fn p3d101_queries_refuse_to_hang_on_absurd_bounds() {
        let absurd = WorldBounds {
            min: WorldPos::from_mm(i64::MIN / 4, i64::MIN / 4, i64::MIN / 4),
            max: WorldPos::from_mm(i64::MAX / 4, i64::MAX / 4, i64::MAX / 4),
        };
        let err = patches_touching(&absurd).expect_err("must refuse");
        match err {
            QueryError::TooManyPatches { requested, cap } => {
                assert!(requested > cap as u64);
                assert_eq!(cap, MAX_QUERY_PATCHES);
            }
            other => panic!("wrong error: {other:?}"),
        }
        assert!(matches!(
            regions_touching(&absurd),
            Err(QueryError::TooManyPatches { .. }) | Err(QueryError::TooLarge)
        ));
    }

    /// Region queries tile horizontally: a region-sized bound touches
    /// exactly its own region.
    #[test]
    fn p3d101_region_queries_tile() {
        let r = RegionCoord { x: -1, z: 3 };
        let fp = r.footprint_xz();
        let b = WorldBounds {
            min: WorldPos::from_mm(fp.min_x, 0, fp.min_z),
            max: WorldPos::from_mm(fp.max_x, 0, fp.max_z),
        };
        assert_eq!(regions_touching(&b).expect("region query"), vec![r]);

        // A bound straddling four regions returns those four.
        let straddle = WorldBounds::from_points(
            WorldPos::from_mm(REGION_MM - 1, 0, -1),
            WorldPos::from_mm(REGION_MM + 1, 0, 1),
        );
        let mut got = regions_touching(&straddle).expect("straddle");
        got.sort_by_key(|r| (r.x, r.z));
        assert_eq!(
            got,
            vec![
                RegionCoord { x: 0, z: -1 },
                RegionCoord { x: 0, z: 0 },
                RegionCoord { x: 1, z: -1 },
                RegionCoord { x: 1, z: 0 },
            ]
        );
    }

    /// A region's patch column at y=0 is exactly 16×16 and starts at the
    /// region origin's patch.
    #[test]
    fn p3d101_region_patch_columns_are_16x16() {
        let cols = patches_in_region(RegionCoord { x: -1, z: 1 });
        assert_eq!(cols.len(), (REGION_PATCH_AXIS * REGION_PATCH_AXIS) as usize);
        assert_eq!(cols[0], PatchCoord { x: -16, y: 0, z: 16 });
        for p in &cols {
            assert_eq!(p.region(), RegionCoord { x: -1, z: 1 });
        }
    }
}
