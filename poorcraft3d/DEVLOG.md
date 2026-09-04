
## 2026-09-04 — loop 366: P3D-101 world coordinates/patches/regions/queries

WHAT: P3D-100 stage opener. The world substrate's spatial language —
coordinates, the region/patch/cell hierarchy, bounds algebra, bounded
spatial queries — as a new pure crate.

HOW: Contract at docs/POORCRAFT-3D/contracts/P3D-101.md (blueprint
15-TERRAIN-TECHNICAL-BLUEPRINT.md read first per the pack's reading
order). pc3d_world joined to the poorcraft3d workspace (zero deps).
scales.rs: MM_PER_METER=1000, CELL 1m, PATCH 16m, REGION 256m +
const-block coherence asserts + MAX_QUERY_PATCHES=65536 cap. coords.rs:
WorldPos(i64 mm) with cell()/patch()/region()/patch_local(); CellCoord/
PatchCoord/RegionCoord origins invert exactly; div_euclid everywhere.
bounds.rs: WorldBounds closed-interval algebra, cell_extent -> Option
(u64 per axis), cell_count saturating, WorldBoundsXz for regions.
query.rs: patches_touching (ascending, capped, Err::TooManyPatches),
regions_touching, patches_in_region (16x16 y=0 columns). Files: new
crate + workspace Cargo.toml + contract + docs.

TEST-SIDE FIXES BEFORE COMMIT: (a) the -1m..16m sample spans THREE patch
columns per axis ([-16,0),[0,16),[16,32)) — 27 patches, not 8; (b)
planet-scale per-axis cell extents DO fit u64 (9.2e15) — only the 3-axis
product saturates; test now asserts both facts precisely. Also removed a
leftover placeholder loop in patches_in_region and a nonexistent helper
call in a bounds test before they ever compiled.

VERIFICATION: P3D workspace cargo test 43 passed / 0 failed (+11: scale
pinning, negative-floor globe semantics, 15-value 3-axis round-trip
matrix incl. boundary straddles, footprint nesting/tiling, edge-inclusive
bounds algebra, exact+saturating cell counting, XZ region tiling,
ascending patch queries with all-intersecting invariant, absurd-bound
refusal, region columns). make p3d-smoke OK. Root cargo test --workspace
474 green (unchanged; zero lf_* edits). Runtimes not rebuilt.

HONESTLY DEFERRED: no content behind the coordinates (generation is
P3D-103, storage P3D-102); scales remain PROPOSAL values pending
P-001/P-002 (by design one-file changeable); no pc3d_core dependency
yet — header integration lands with the save path; interest rings are
P3D-105.
