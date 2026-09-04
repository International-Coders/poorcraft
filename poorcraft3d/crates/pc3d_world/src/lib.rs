//! POORCRAFT 3D world substrate — the spatial language
//! (P3D-101, docs/POORCRAFT-3D/16-IMPLEMENTATION-WORK-BREAKDOWN.md).
//!
//! Pure integer geometry: world positions in millimeters, the
//! region/patch/cell hierarchy from the terrain blueprint, bounds algebra,
//! and bounded spatial queries. No dependencies, no IO, no generation —
//! this is the ground every later system stands on, and the one place the
//! world's scales are declared.

pub mod bounds;
pub mod build;
pub mod edit;
pub mod coords;
pub mod debug_overlay;
pub mod gen;
pub mod hydro;
pub mod lod;
pub mod proof;
pub mod query;
pub mod scales;
pub mod stream;
pub mod terrain;

pub use bounds::{WorldBounds, WorldBoundsXz};
pub use debug_overlay::{lod_color, rows_for, render_overlay, PatchDebugRow};
pub use gen::{Biome, CellMaterial, MacroField, WorldGen};
pub use build::{effective_answer, replay_builds, BuildBlock, BuildKind, BuildOp, Construction, PlaceError, RemoveError};
pub use edit::{apply_edit, affected_patches, replay, Brush, EditKind, EditOp, Snapshot, COMPACT_THRESHOLD};
pub use coords::{Axis, CellCoord, LocalPos, PatchCoord, RegionCoord, WorldPos};
pub use query::{patches_in_region, patches_touching, regions_touching, QueryError};
pub use scales::{
    CELL_MM, CELL_METERS, MAX_QUERY_PATCHES, MM_PER_METER, PATCH_CELL_AXIS, PATCH_MM,
    PATCH_METERS, REGION_MM, REGION_METERS, REGION_PATCH_AXIS,
};
