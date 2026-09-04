//! POORCRAFT 3D world substrate — the spatial language
//! (P3D-101, docs/POORCRAFT-3D/16-IMPLEMENTATION-WORK-BREAKDOWN.md).
//!
//! Pure integer geometry: world positions in millimeters, the
//! region/patch/cell hierarchy from the terrain blueprint, bounds algebra,
//! and bounded spatial queries. No dependencies, no IO, no generation —
//! this is the ground every later system stands on, and the one place the
//! world's scales are declared.

pub mod bounds;
pub mod coords;
pub mod gen;
pub mod query;
pub mod scales;

pub use bounds::{WorldBounds, WorldBoundsXz};
pub use coords::{CellCoord, LocalPos, PatchCoord, RegionCoord, WorldPos};
pub use query::{patches_in_region, patches_touching, regions_touching, QueryError};
pub use scales::{
    CELL_MM, CELL_METERS, MAX_QUERY_PATCHES, MM_PER_METER, PATCH_CELL_AXIS, PATCH_MM,
    PATCH_METERS, REGION_MM, REGION_METERS, REGION_PATCH_AXIS,
};
