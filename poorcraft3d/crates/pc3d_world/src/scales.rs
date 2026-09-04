//! The world's scales — declared ONCE (P3D-101).
//!
//! These are the terrain blueprint's PROPOSED defaults (decision register
//! P-001/P-002, owner confirmation pending). They live in this one module
//! so an owner answer re-scales the world in a single file; every other
//! module derives from these constants and the coherence checks below make
//! a mismatched edit fail to compile or fail tests immediately.

/// Sub-meter precision: world positions are millimeter integers.
pub const MM_PER_METER: i64 = 1_000;

/// Construction/collision cell: 1 m cube (blueprint "base terrain cell").
pub const CELL_METERS: i64 = 1;
pub const CELL_MM: i64 = CELL_METERS * MM_PER_METER;

/// Streaming/editing/meshing unit: 16 m (blueprint "terrain patch").
pub const PATCH_METERS: i64 = 16;
pub const PATCH_MM: i64 = PATCH_METERS * MM_PER_METER;
pub const PATCH_CELL_AXIS: i64 = PATCH_METERS / CELL_METERS;

/// Macro region for climate/watersheds/sites: 256 m horizontal
/// (blueprint "macro region"). Regions are horizontal-only: the world is
/// region-partitioned in x/z and unbounded in y.
pub const REGION_METERS: i64 = 256;
pub const REGION_MM: i64 = REGION_METERS * MM_PER_METER;
pub const REGION_PATCH_AXIS: i64 = REGION_METERS / PATCH_METERS;

/// Compile-time coherence: the hierarchy must nest exactly.
const _: () = {
    assert!(MM_PER_METER == 1000);
    assert!(CELL_METERS == 1);
    assert!(PATCH_METERS == PATCH_CELL_AXIS * CELL_METERS);
    assert!(REGION_METERS == REGION_PATCH_AXIS * PATCH_METERS);
};

/// Hard cap on patches a single spatial query may return — a planet-sized
/// bound must fail loudly, not hang the caller.
pub const MAX_QUERY_PATCHES: usize = 1 << 16;

#[cfg(test)]
mod tests {
    use super::*;

    /// The blueprint's proposed values, pinned: 1 m cells, 16 m patches,
    /// 256 m regions, nested exactly.
    #[test]
    fn p3d101_blueprint_scales_are_pinned_and_nested() {
        assert_eq!(CELL_MM, 1_000);
        assert_eq!(PATCH_MM, 16_000);
        assert_eq!(REGION_MM, 256_000);
        assert_eq!(PATCH_CELL_AXIS, 16);
        assert_eq!(REGION_PATCH_AXIS, 16);
        // Regions contain a whole number of patches per axis.
        assert_eq!(REGION_PATCH_AXIS * PATCH_CELL_AXIS, 256);
    }
}
