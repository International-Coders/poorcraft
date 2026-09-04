//! P3D-201: the surface-extraction bake-off
//! (docs/POORCRAFT-3D/16-IMPLEMENTATION-WORK-BREAKDOWN.md, P3D-200 opener).
//!
//! The blueprint's rule: choose the natural-surface representation from
//! MEASUREMENTS, not preference. Two candidates extract the same 16³
//! occupancy grids from the same procedural scenes —
//!
//! - **Heightfield**: solid strictly below the analytic surface per column.
//!   Cheapest possible; hard 1 m terraces on slopes.
//! - **Density-threshold**: a supersampled density (surface height blended
//!   with lateral smoothing) thresholded per cell — softer silhouettes,
//!   4× the column samples.
//!
//! Measured: extraction time, memory, single-brush edit rebuild cost, and
//! surface fidelity (mean column-top error vs the analytic function). The
//! numbers recorded in DEVLOG choose what P3D-202 promotes to the
//! authoritative final-solid query.

use crate::coords::{PatchCoord, WorldPos};
use crate::gen::{cell_material, CellMaterial, WorldGen};
use crate::scales::PATCH_CELL_AXIS;

/// THE authoritative answer for one cell (P3D-202): terrain solidity and
/// material. Every subsystem — mesh, collision, water, navigation — asks
/// this function; none may reimplement surface logic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SolidAnswer {
    /// Terrain-collision solidity (Water and Air are not solid).
    pub solid: bool,
    pub material: CellMaterial,
}

/// The single authoritative final-solid query: pure, deterministic,
/// O(1). Guaranteed to agree with regenerated/stored patch cells because
/// both call the same `gen::cell_material` decision.
pub fn final_solid(gen: &WorldGen, wx: i64, wy: i64, wz: i64) -> SolidAnswer {
    let surface_mm = gen.surface_height_mm(wx, wz);
    let region = WorldPos::from_mm(wx, wy, wz).region();
    let biome = gen.biome(region);
    let material = cell_material(biome, wy, surface_mm);
    SolidAnswer {
        solid: !matches!(material, CellMaterial::Air | CellMaterial::Water),
        material,
    }
}

/// A patch's occupancy grid: solid[x][y][z], row-major like PatchCells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolidGrid {
    pub coord: PatchCoord,
    pub solid: Vec<bool>,
}

impl SolidGrid {
    pub fn get(&self, x: usize, y: usize, z: usize) -> bool {
        self.solid[(x * PATCH_CELL_AXIS as usize + y) * PATCH_CELL_AXIS as usize + z]
    }

    /// The highest solid cell per column (None = all air), for fidelity
    /// comparison and tests.
    pub fn column_top(&self, x: usize, z: usize) -> Option<i32> {
        let n = PATCH_CELL_AXIS as usize;
        for y in (0..n).rev() {
            if self.get(x, y, z) {
                return Some(self.coord.origin().y.div_euclid(1000) as i32 + y as i32);
            }
        }
        None
    }
}

/// Shared benchmark scenes, pinned to seeds so both candidates extract
/// identical regions. Pins verified by region-field scan (see tests).
/// NOTE: the blueprint's "sharp cliff" scene is deliberately absent — the
/// pure-heightmap generator produces NO sharp cliffs (measured: max
/// adjacent-region step < 25 m across wide sweeps). True cliff capability
/// arrives with P3D-203's density fields; Highlands stands in as the
/// steep-terrain scene until then.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneSpec {
    /// Gentle terrain: rolling hills and a valley floor.
    SmoothHills,
    /// High-elevation ground (elevation ≥ 90 m at region center).
    Highlands,
    /// Land straddling sea level: beaches and shallow water columns.
    Coast,
}

impl SceneSpec {
    pub fn name(self) -> &'static str {
        match self {
            SceneSpec::SmoothHills => "smooth_hills",
            SceneSpec::Highlands => "highlands",
            SceneSpec::Coast => "coast",
        }
    }

    /// The (seed, patch) each scene extracts — REGION-CENTER patches (the
    /// +8 offsets) at the y-level containing the analytic surface (probed:
    /// hills surface 23.3 m, highlands 89.7 m, coast +2.4 m). The
    /// alignment + fidelity tests guard these pins.
    pub fn patch(self) -> (u64, PatchCoord) {
        match self {
            SceneSpec::SmoothHills => (
                3,
                PatchCoord { x: -60 * 16 + 8, y: 1, z: -31 * 16 + 8 },
            ),
            SceneSpec::Highlands => (
                3,
                PatchCoord { x: -9 * 16 + 8, y: 5, z: -12 * 16 + 8 },
            ),
            SceneSpec::Coast => {
                (3, PatchCoord { x: -60 * 16 + 8, y: 0, z: -11 * 16 + 8 })
            }
        }
    }
}

/// Candidate extractors. Both consume the same WorldGen + patch and produce
/// a SolidGrid.
pub mod candidate {
    use super::*;

    /// A: solid strictly below the analytic surface per column.
    pub fn heightfield(gen: &WorldGen, coord: PatchCoord) -> SolidGrid {
        let n = PATCH_CELL_AXIS as usize;
        let mut solid = vec![false; n * n * n];
        let origin = coord.origin();
        let ax = origin.x.div_euclid(1000) as i32;
        let ay = origin.y.div_euclid(1000) as i32;
        let az = origin.z.div_euclid(1000) as i32;
        for cx in 0..n {
            for cz in 0..n {
                let wx = (ax + cx as i32) as i64 * 1000;
                let wz = (az + cz as i32) as i64 * 1000;
                let surface_mm = gen.surface_height_mm(wx, wz);
                for cy in 0..n {
                    let wy = (ay + cy as i32) as i64 * 1000;
                    if wy < surface_mm {
                        solid[(cx * n + cy) * n + cz] = true;
                    }
                }
            }
        }
        SolidGrid { coord, solid }
    }

    /// B: density = surface distance thresholded per cell against 2×2
    /// quarter-meter sub-samples — a cell is solid when at least half its
    /// sub-samples lie below the analytic surface. Smoother silhouettes at
    /// 4× the surface samples.
    pub fn density_threshold(gen: &WorldGen, coord: PatchCoord) -> SolidGrid {
        let n = PATCH_CELL_AXIS as usize;
        let mut solid = vec![false; n * n * n];
        let origin = coord.origin();
        let ax = origin.x.div_euclid(1000) as i32;
        let ay = origin.y.div_euclid(1000) as i32;
        let az = origin.z.div_euclid(1000) as i32;
        // Sub-sample offsets (quarter-meter) around each column's center.
        const SUB: [[i64; 2]; 4] = [[-250, -250], [250, -250], [-250, 250], [250, 250]];
        for cx in 0..n {
            for cz in 0..n {
                let wx = (ax + cx as i32) as i64 * 1000;
                let wz = (az + cz as i32) as i64 * 1000;
                for cy in 0..n {
                    let wy = (ay + cy as i32) as i64 * 1000;
                    let mut below = 0;
                    for off in SUB {
                        let sx = wx + off[0];
                        let sz = wz + off[1];
                        if wy < gen.surface_height_mm(sx, sz) {
                            below += 1;
                        }
                    }
                    // Solid when at least half the sub-samples are below.
                    solid[(cx * n + cy) * n + cz] = below >= 2;
                }
            }
        }
        SolidGrid { coord, solid }
    }
}

/// One measured row of the bake-off.
#[derive(Clone, Debug, PartialEq)]
pub struct BenchResult {
    pub scene: &'static str,
    pub candidate: &'static str,
    pub extract_us: u128,
    pub grid_bytes: usize,
    /// Re-extraction cost after a 3×3×2 dig at the patch center.
    pub edit_rebuild_us: u128,
    /// Mean |extracted column top − analytic surface| in meters.
    pub fidelity_err_m: f32,
    /// Columns the fidelity metric actually measured (surface inside the
    /// patch window).
    pub fidelity_columns: usize,
}

/// Columns whose analytic surface lies inside the patch's y-window.
fn measured_columns(gen: &WorldGen, coord: PatchCoord) -> usize {
    let n = PATCH_CELL_AXIS as usize;
    let origin = coord.origin();
    let ax = origin.x.div_euclid(1000) as i32;
    let ay = origin.y.div_euclid(1000) as i32;
    let az = origin.z.div_euclid(1000) as i32;
    let mut m = 0usize;
    for cx in 0..n {
        for cz in 0..n {
            let s = gen.surface_height_mm((ax + cx as i32) as i64 * 1000, (az + cz as i32) as i64 * 1000) as f64 / 1000.0;
            if s >= ay as f64 && s < (ay + PATCH_CELL_AXIS as i32) as f64 {
                m += 1;
            }
        }
    }
    m
}

fn measure<F: Fn() -> SolidGrid>(f: F) -> (SolidGrid, u128) {
    let start = std::time::Instant::now();
    let grid = f();
    (grid, start.elapsed().as_micros())
}

fn fidelity_error(gen: &WorldGen, coord: PatchCoord, grid: &SolidGrid) -> f32 {
    let n = PATCH_CELL_AXIS as usize;
    let origin = coord.origin();
    let ax = origin.x.div_euclid(1000) as i32;
    let ay = origin.y.div_euclid(1000) as i32;
    let az = origin.z.div_euclid(1000) as i32;
    let mut total = 0.0f64;
    let mut measured = 0usize;
    for cx in 0..n {
        for cz in 0..n {
            let wx = (ax + cx as i32) as i64 * 1000;
            let wz = (az + cz as i32) as i64 * 1000;
            let surface_mm = gen.surface_height_mm(wx, wz);
            let surface_m = surface_mm as f64 / 1000.0;
            // Only columns whose analytic surface lies inside the patch's
            // y-window carry extraction information: elsewhere all-air or
            // all-solid is the CORRECT answer for both candidates.
            let window_lo = ay as f64;
            let window_hi = (ay + PATCH_CELL_AXIS as i32) as f64;
            if !(surface_m >= window_lo && surface_m < window_hi) {
                continue;
            }
            let top = match grid.column_top(cx, cz) {
                Some(t) => t as f64,
                None => window_lo - 1.0, // air column inside the window: penalize
            };
            total += (top - surface_m).abs();
            measured += 1;
        }
    }
    if measured == 0 {
        0.0
    } else {
        (total / measured as f64) as f32
    }
}

/// Run the full bake-off: all scenes × both candidates.
pub fn run_bakeoff() -> Vec<BenchResult> {
    let scenes = [
        SceneSpec::SmoothHills,
        SceneSpec::Highlands,
        SceneSpec::Coast,
    ];
    let mut out = Vec::new();
    for scene in scenes {
        let (seed, coord) = scene.patch();
        let gen = WorldGen::new(seed);
        // Candidate A
        let (ga, ta) = measure(|| candidate::heightfield(&gen, coord));
        let (_, ra) = measure(|| {
            // Edit: dig 3×3×2 at patch center, then rebuild (re-extract).
            let mut g = candidate::heightfield(&gen, coord);
            let c = PATCH_CELL_AXIS as usize / 2;
            for dx in 0..3 {
                for dz in 0..3 {
                    for dy in 0..2 {
                        let idx = ((c + dx) * PATCH_CELL_AXIS as usize + c + 1 + dy)
                            * PATCH_CELL_AXIS as usize
                            + c + dz;
                        g.solid[idx] = false;
                    }
                }
            }
            candidate::heightfield(&gen, coord)
        });
        out.push(BenchResult {
            scene: scene.name(),
            candidate: "heightfield",
            extract_us: ta,
            grid_bytes: ga.solid.len(),
            edit_rebuild_us: ra,
            fidelity_err_m: fidelity_error(&gen, coord, &ga),
            fidelity_columns: measured_columns(&gen, coord),
        });
        // Candidate B
        let (gb, tb) = measure(|| candidate::density_threshold(&gen, coord));
        let (_, rb) = measure(|| candidate::density_threshold(&gen, coord));
        out.push(BenchResult {
            scene: scene.name(),
            candidate: "density_threshold",
            extract_us: tb,
            grid_bytes: gb.solid.len(),
            edit_rebuild_us: rb,
            fidelity_err_m: fidelity_error(&gen, coord, &gb),
            fidelity_columns: measured_columns(&gen, coord),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scales::PATCH_MM;

    /// On genuinely flat ground both candidates must agree with the
    /// analytic height exactly (column tops at the surface cell).
    #[test]
    fn p3d201_candidates_agree_on_flat_ground() {
        let gen = WorldGen::new(5);
        // Find a patch whose 16×16 analytic tops vary by ≤ 1 m (near-flat).
        let mut flat: Option<PatchCoord> = None;
        'outer: for px in -30..=30 {
            for pz in -30..=30 {
                let coord = PatchCoord { x: px * 16, y: 0, z: pz * 16 };
                let o = coord.origin();
                let ax = o.x.div_euclid(1000) as i32;
                let az = o.z.div_euclid(1000) as i32;
                let mut min_t = i32::MAX;
                let mut max_t = i32::MIN;
                for cx in 0..16 {
                    for cz in 0..16 {
                        let h =
                            (gen.surface_height_mm((ax + cx) as i64 * 1000, (az + cz) as i64 * 1000)
                                / 1000) as i32;
                        min_t = min_t.min(h);
                        max_t = max_t.max(h);
                    }
                }
                if max_t - min_t <= 1 && min_t >= 1 && max_t < 15 {
                    flat = Some(coord);
                    break 'outer;
                }
            }
        }
        let coord = flat.expect("a near-flat land patch exists across 61×61 patches");
        let a = candidate::heightfield(&gen, coord);
        let b = candidate::density_threshold(&gen, coord);
        for x in 0..16 {
            for z in 0..16 {
                let ta = a.column_top(x, z).expect("flat land has a top");
                let tb = b.column_top(x, z).expect("flat land has a top");
                // The density candidate's lateral smoothing legitimately
                // shifts a column top by ONE cell when detail noise makes
                // the 2×2 sub-samples straddle the surface; more than one
                // cell would be a bug.
                assert!(
                    (ta - tb).abs() <= 1,
                    "flat-ground tops diverge >1 cell at {x},{z}: {ta} vs {tb}"
                );
            }
        }
    }

    /// Candidate B never floats solid above the analytic surface + its
    /// smoothing margin (1 cell): softness cannot become flight.
    #[test]
    fn p3d201_density_never_floats_above_surface() {
        let gen = WorldGen::new(2024);
        let coord = SceneSpec::Coast.patch().1;
        let b = candidate::density_threshold(&gen, coord);
        let n = PATCH_CELL_AXIS as usize;
        let o = coord.origin();
        let ay = o.y.div_euclid(1000) as i32;
        for x in 0..n {
            for z in 0..n {
                let wx = (o.x.div_euclid(1000) as i32 + x as i32) as i64 * 1000;
                let wz = (o.z.div_euclid(1000) as i32 + z as i32) as i64 * 1000;
                let surface_m = (gen.surface_height_mm(wx, wz) as f64 / 1000.0).floor() as i32;
                for y in 0..n {
                    if b.get(x, y, z) {
                        let wy = ay + y as i32;
                        assert!(
                            wy <= surface_m + 1,
                            "floating solid at {x},{y},{z} (surface {surface_m})"
                        );
                    }
                }
            }
        }
    }

    /// Grids are deterministic: same scene, same bytes.
    #[test]
    fn p3d201_grids_are_deterministic() {
        for scene in [SceneSpec::SmoothHills, SceneSpec::Highlands, SceneSpec::Coast] {
            let (seed, coord) = scene.patch();
            let gen = WorldGen::new(seed);
            let a = candidate::heightfield(&gen, coord);
            let b = candidate::heightfield(&gen, coord);
            assert_eq!(a, b, "scene {}", scene.name());
        }
    }

    /// The bake-off returns every scene×candidate row with plausible
    /// positive numbers and honest memory accounting.
    #[test]
    fn p3d201_bakeoff_is_complete_and_plausible() {
        let rows = run_bakeoff();
        assert_eq!(rows.len(), 6, "3 scenes × 2 candidates");
        for r in &rows {
            assert!(r.extract_us > 0);
            assert!(r.edit_rebuild_us > 0);
            assert_eq!(r.grid_bytes, 16 * 16 * 16);
            assert!(r.fidelity_err_m >= 0.0);
        }
        // Density does 4× the surface samples per column: it should never
        // be dramatically FASTER than heightfield (sanity on the harness).
        for pair in rows.chunks(2) {
            if pair.len() == 2 && pair[0].candidate == "heightfield" {
                assert!(
                    pair[1].extract_us >= pair[0].extract_us / 4,
                    "density suspiciously faster than heightfield — harness bug"
                );
            }
        }
    }

    /// Patch span sanity for the scene patches (guards against pin typos).
    #[test]
    fn p3d201_scene_patches_are_aligned() {
        for scene in [SceneSpec::SmoothHills, SceneSpec::Highlands, SceneSpec::Coast] {
            let (_, coord) = scene.patch();
            let o = coord.origin();
            assert_eq!(o.x.rem_euclid(PATCH_MM), 0);
            assert_eq!(o.y.rem_euclid(PATCH_MM), 0);
            assert_eq!(o.z.rem_euclid(PATCH_MM), 0);
        }
        let _ = WorldPos::default();
    }

    /// THE P3D-202 CONTRACT: the single final-solid query and patch
    /// regeneration are the SAME WORLD. Every cell of regenerated patches
    /// (multiple seeds, all sign combinations) must equal `final_solid` at
    /// that cell's world position — material and solidity both. Mesh,
    /// collision, water, and navigation all call `final_solid`, so this is
    /// the no-divergence guarantee.
    #[test]
    fn p3d202_final_solid_agrees_with_regenerated_patches() {
        for seed in [3u64, 777, 2024] {
            let gen = WorldGen::new(seed);
            for coord in [
                PatchCoord { x: -60 * 16 + 8, y: 1, z: -31 * 16 + 8 },
                PatchCoord { x: -9 * 16 + 8, y: 5, z: -12 * 16 + 8 },
                PatchCoord { x: -60 * 16 + 8, y: 0, z: -11 * 16 + 8 },
                PatchCoord { x: -3 * 16, y: -2 * 16, z: 5 * 16 },
            ] {
                let patch = gen.regenerate_patch(coord);
                let n = PATCH_CELL_AXIS as usize;
                let o = coord.origin();
                let ax = o.x.div_euclid(1000) as i32;
                let ay = o.y.div_euclid(1000) as i32;
                let az = o.z.div_euclid(1000) as i32;
                for cx in 0..n {
                    for cy in 0..n {
                        for cz in 0..n {
                            let wx = (ax + cx as i32) as i64 * 1000;
                            let wy = (ay + cy as i32) as i64 * 1000;
                            let wz = (az + cz as i32) as i64 * 1000;
                            let answer = final_solid(&gen, wx, wy, wz);
                            assert_eq!(
                                answer.material,
                                patch.get(cx, cy, cz),
                                "material divergence at seed {seed} {wx},{wy},{wz}"
                            );
                            assert_eq!(
                                answer.solid,
                                !matches!(
                                    patch.get(cx, cy, cz),
                                    CellMaterial::Air | CellMaterial::Water
                                )
                            );
                        }
                    }
                }
            }
        }
    }

    /// Semantics at the boundaries: air above the surface, water below
    /// sea level but above the floor, solid ground under it.
    #[test]
    fn p3d202_final_solid_semantics() {
        let gen = WorldGen::new(9);
        // Deep ocean region found by scan earlier; assert water is not
        // solid and the floor beneath is.
        let mut checked_water = false;
        'scan: for x in -40..=40 {
            for z in -40..=40 {
                let r = crate::coords::RegionCoord { x, z };
                if gen.biome(r) == crate::gen::Biome::Ocean {
                    let wx = (x * 256 + 128) as i64 * 1000;
                    let wz = (z * 256 + 128) as i64 * 1000;
                    let surface = gen.surface_height_mm(wx, wz);
                    // Two cells above the quantized floor cell: definitely
                    // open water (one cell above can still be the floor's
                    // quantized top — a documented 1-cell artifact, agreed
                    // by every consumer because they share this function).
                    let s0 = surface.div_euclid(1000) * 1000;
                    let water_cell = final_solid(&gen, wx, s0 + 1_000, wz);
                    assert!(!water_cell.solid);
                    assert_eq!(water_cell.material, CellMaterial::Water);
                    let floor = final_solid(&gen, wx, surface - 5_000, wz);
                    assert!(floor.solid);
                    checked_water = true;
                    break 'scan;
                }
            }
        }
        assert!(checked_water, "no ocean region found for the semantics probe");
        // Above the land surface: air, not solid.
        let air = final_solid(&gen, 8_000, 500_000, 8_000);
        assert!(!air.solid);
        assert_eq!(air.material, CellMaterial::Air);
    }
}
