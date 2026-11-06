//! N04 — the universal grounding authority.
//!
//! ONE pure answer to "what does the world hold at this column?": the
//! voxel truth (`final_solid` = analytic surface + the N03 river carve)
//! scanned from just above the analytic surface, plus the derived data
//! every consumer class needs (normal, slope, material, clearance).
//! Trees, rocks, structures, loot, workspots, NPCs and decoration all
//! consume `GroundingResult`; renderer-side Y offsets are forbidden.
//!
//! THE LAYERING LAW: this answers the BASE WORLD truth (generator +
//! carve). The streamed dig-delta layer composes ON TOP at the render
//! boundary (`renderer::ground_y_at`: stream first, this authority as
//! the fallback) — the world crate stays pure, the streamer stays the
//! only edit holder.
//!
//! Consumer classes (the card's classification):
//! - `Point` — a pivot stands at `surface_point` (props, loot, decor).
//!   Contact tolerance ≤ 2 cm.
//! - `Footprint` — the LOWEST `surface_point` over an area (buildings:
//!   the foundation uses the whole area, never the pivot).
//! - `Capsule` — feet at `surface_point`; `clearance_m` must fit the
//!   body (NPCs, the player).
//! - `Foundation` — per-column siting via [`foundation_ok`]: every
//!   column dry, solid, channel-free.

use crate::gen::{CellMaterial, WorldGen};
use crate::terrain::final_solid;

/// One column's ground answer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundingResult {
    /// World millimeters where feet rest: the top of the topmost solid
    /// cell (feet cell = the first non-solid cell above it).
    pub surface_y_mm: i64,
    /// Surface normal (unit, y-up dominant) from central differences.
    pub normal: [f32; 3],
    /// Rise per meter along the steepest sampled direction.
    pub slope: f32,
    /// The material of the topmost solid cell (Grass/Sand/Rock/…).
    pub material: CellMaterial,
    /// Consecutive passable cells above the feet (meters, capped at 16
    /// = open sky). A capsule must fit inside.
    pub clearance_m: f32,
    /// The column holds solid ground at all (false = fully carved/void).
    pub grounded: bool,
    /// The carved river channel at this column, if any (water support
    /// law: consumers must refuse standing IN water unless aquatic).
    pub channel_hit: Option<crate::river_carve::ChannelHit>,
}

impl GroundingResult {
    /// Surface in meters (the render-side convenience).
    pub fn surface_m(&self) -> f32 {
        self.surface_y_mm as f32 / 1000.0
    }
}

/// What a caller wants grounded.
#[derive(Clone, Copy, Debug)]
pub enum GroundQuery {
    /// One column (props, loot, decor, NPC feet).
    Point,
    /// An area: answer the LOWEST column surface over the footprint
    /// (building bases sink to the lowest corner, never float).
    Footprint {
        /// Half-extents in meters (the query samples the corners,
        /// edge midpoints and the center of the w×h area).
        half_w_m: f32,
        half_h_m: f32,
    },
    /// A body: point answer with a real headroom probe for `height_m`.
    Capsule { radius_m: f32, height_m: f32 },
}

/// THE COLUMN SCAN: the topmost solid cell in a column, plus its
/// material. Above the analytic surface `cell_material` yields only
/// Air/Water, so the scan starts one cell up and needs at most the
/// carve band (~8 cells) — O(1) on normal ground.
fn column_top(gen: &WorldGen, wx_mm: i64, wz_mm: i64) -> Option<(i64, CellMaterial)> {
    let analytic = gen.effective_surface_mm(wx_mm, wz_mm);
    let top_y = analytic.div_euclid(1000) + 1;
    let floor_y = analytic.div_euclid(1000) - 8;
    let mut y = top_y;
    while y >= floor_y {
        let cell = final_solid(gen, wx_mm, y * 1000, wz_mm);
        if cell.solid {
            return Some((y, cell.material));
        }
        y -= 1;
    }
    None
}

/// The pure authority: ground one (x, z) column from the voxel truth.
/// Deterministic; honors the N03 river carve (final_solid carves).
pub fn ground_at(gen: &WorldGen, wx_mm: i64, wz_mm: i64, query: GroundQuery) -> GroundingResult {
    let channel_hit = gen.river_channel(wx_mm, wz_mm);
    let Some((top_y, material)) = column_top(gen, wx_mm, wz_mm) else {
        return GroundingResult {
            surface_y_mm: gen.effective_surface_mm(wx_mm, wz_mm),
            normal: [0.0, 1.0, 0.0],
            slope: 0.0,
            material: CellMaterial::Air,
            clearance_m: 0.0,
            grounded: false,
            channel_hit,
        };
    };

    // Normal + slope from neighbor tops (central difference, 1 m span).
    let sample = |dx: i64, dz: i64| -> i64 {
        column_top(gen, wx_mm + dx * 1000, wz_mm + dz * 1000)
            .map(|(y, _)| y)
            .unwrap_or_else(|| gen.effective_surface_mm(wx_mm + dx * 1000, wz_mm + dz * 1000).div_euclid(1000))
    };
    let nx = (sample(-1, 0) - sample(1, 0)) as f32 * 0.5;
    let nz = (sample(0, -1) - sample(0, 1)) as f32 * 0.5;
    let len = (nx * nx + 1.0 + nz * nz).sqrt();
    let slope = (nx * nx + nz * nz).sqrt();
    let normal = [nx / len, 1.0 / len, nz / len];

    // Headroom: consecutive passable cells above the feet cell.
    let feet_y = top_y + 1;
    let mut passable = 0i64;
    'up: for dy in 0..16i64 {
        let cell = final_solid(gen, wx_mm, (feet_y + dy) * 1000, wz_mm);
        if cell.solid {
            break 'up;
        }
        passable += 1;
    }
    let clearance_m = passable as f32;

    match query {
        GroundQuery::Point => GroundingResult {
            surface_y_mm: (top_y + 1) * 1000,
            normal,
            slope,
            material,
            clearance_m,
            grounded: true,
            channel_hit,
        },
        GroundQuery::Capsule { height_m, .. } => {
            // The capsule demands its height in real headroom.
            let ok = clearance_m >= height_m.ceil() as i64 as f32;
            GroundingResult {
                surface_y_mm: (top_y + 1) * 1000,
                normal,
                slope,
                material,
                clearance_m: if ok { clearance_m } else { 0.0 },
                grounded: true,
                channel_hit,
            }
        }
        GroundQuery::Footprint { half_w_m, half_h_m } => {
            // The LOWEST column top over the footprint (corners + edges +
            // center) — the foundation sinks, it never floats.
            let hw = half_w_m.max(0.0) as i64;
            let hh = half_h_m.max(0.0) as i64;
            let mut lowest_y = top_y;
            let mut lowest_mat = material;
            for (dx, dz) in [
                (hw, 0i64),
                (-hw, 0),
                (0, hh),
                (0, -hh),
                (hw, hh),
                (-hw, hh),
                (hw, -hh),
                (-hw, -hh),
            ] {
                if let Some((y, m)) = column_top(gen, wx_mm + dx * 1000, wz_mm + dz * 1000) {
                    if y < lowest_y {
                        lowest_y = y;
                        lowest_mat = m;
                    }
                }
            }
            GroundingResult {
                surface_y_mm: (lowest_y + 1) * 1000,
                normal,
                slope,
                material: lowest_mat,
                clearance_m,
                grounded: true,
                channel_hit,
            }
        }
    }
}

/// Foundation siting: EVERY column in the footprint must be solid at the
/// surface, not water, and channel-free (the castle law).
pub fn foundation_ok(gen: &WorldGen, x0_mm: i64, z0_mm: i64, w_m: i64, h_m: i64) -> bool {
    for dx in 0..w_m {
        for dz in 0..h_m {
            let wx = x0_mm + dx * 1000;
            let wz = z0_mm + dz * 1000;
            if gen.river_channel(wx, wz).is_some() {
                return false;
            }
            let analytic = gen.effective_surface_mm(wx, wz);
            let cell = final_solid(gen, wx, analytic, wz);
            if !cell.solid {
                return false;
            }
        }
    }
    true
}

/// THE FORBIDDEN OFFSETS (card item 6): known renderer-side Y fixes this
/// card removes. The pc3d_render gate scans its own sources for these
/// exact magic patterns and fails the suite if any survives.
pub const FORBIDDEN_OFFSETS: &[(&str, &str)] = &[
    // The historic NPC float: surface + 1.0 with a feet-pivot GLB.
    ("npc_world_pos +1.0 m float", "terrain surface, then the body draws above"),
    // Anchor boxes centered half a meter above the ground.
    ("anchor boxes +0.5 m center", "cell_ground(c) + 0.5"),
    // Water wheel sunk by eye below the analytic bank.
    ("water wheel -0.2 m by eye", "ground - 0.2"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::RegionCoord;

    fn channel_column(gen: &WorldGen) -> (i64, i64) {
        let graph = crate::hydro::RiverGraph::new(gen, crate::river_carve::WATERSHED_HALF);
        let threshold = crate::hydro::RIVER_THRESHOLD;
        for rx in -28..=28i32 {
            for rz in -28..=28i32 {
                let r = RegionCoord { x: rx, z: rz };
                let Some(d) = graph.downstream(r) else { continue };
                if graph.discharge(r) < threshold || graph.discharge(d) < threshold {
                    continue;
                }
                let mx = ((rx as f32 + 0.5) * 256.0 * 1000.0) as i64;
                let mz = ((rz as f32 + 0.5) * 256.0 * 1000.0) as i64;
                if gen.river_channel(mx, mz).is_some() {
                    return (mx, mz);
                }
            }
        }
        panic!("no carved channel near origin");
    }

    fn find_flat_column(gen: &WorldGen) -> (i64, i64) {
        for x in -2000..=2000i64 {
            for z in -2000..=2000i64 {
                let r = ground_at(gen, x * 1000, z * 1000, GroundQuery::Point);
                if r.grounded && r.slope < 0.05 && r.channel_hit.is_none() {
                    return (x * 1000, z * 1000);
                }
            }
        }
        panic!("no flat column near origin");
    }

    #[test]
    fn p3dn04_point_answer_matches_voxel_truth() {
        let gen = WorldGen::new(42);
        let (wx, wz) = find_flat_column(&gen);
        let r = ground_at(&gen, wx, wz, GroundQuery::Point);
        assert!(r.grounded);
        // The answer IS the voxel law: the feet cell is free, the cell
        // below is solid.
        let feet = r.surface_y_mm.div_euclid(1000);
        assert!(!final_solid(&gen, wx, feet * 1000, wz).solid, "feet cell free");
        assert!(
            final_solid(&gen, wx, (feet - 1) * 1000, wz).solid,
            "ground solid below the feet"
        );
        // Normal is unit and y-up dominant on flat ground.
        let n = r.normal;
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!((len - 1.0).abs() < 1e-3, "unit normal, len {len}");
        assert!(n[1] > 0.9, "flat ground normal y-dominant, got {n:?}");
        assert!(r.clearance_m >= 16.0, "open sky reads as the cap, got {}", r.clearance_m);
    }

    #[test]
    fn p3dn04_channel_columns_answer_the_carved_bed_not_the_bank() {
        // THE RED LAW: inside a carved channel the analytic surface reads
        // the BANK; the authority must answer the voxel bed and name the
        // channel (measured divergence was 2.3–3.8 m over 149/150 cols).
        let gen = WorldGen::new(42);
        let (mx, mz) = channel_column(&gen);
        let g = ground_at(&gen, mx, mz, GroundQuery::Point);
        assert!(g.grounded, "the bed holds ground");
        assert!(
            g.surface_m() <= gen.effective_surface_mm(mx, mz) as f32 / 1000.0,
            "channel answer must sit at/below the analytic bank: bed {:.2} vs bank {:.2}",
            g.surface_m(),
            gen.effective_surface_mm(mx, mz) as f32 / 1000.0
        );
        assert!(g.channel_hit.is_some(), "the channel must be named");
        // The bed material is not Air: the answer carries a real material.
        assert_ne!(g.material, CellMaterial::Air);
    }

    #[test]
    fn p3dn04_footprint_answers_the_lowest_corner() {
        let gen = WorldGen::new(42);
        let (wx, wz) = find_flat_column(&gen);
        let p = ground_at(&gen, wx, wz, GroundQuery::Point);
        let f = ground_at(
            &gen,
            wx,
            wz,
            GroundQuery::Footprint { half_w_m: 2.0, half_h_m: 2.0 },
        );
        let diff = p.surface_m() - f.surface_m();
        assert!(diff >= 0.0, "footprint never floats above its lowest column");
        assert!(diff <= 1.5, "a small footprint sinks at most a step: {diff}");
    }

    #[test]
    fn p3dn04_slope_and_normal_track_terrain() {
        let gen = WorldGen::new(42);
        for x in -3000..=3000i64 {
            for z in -3000..=3000i64 {
                let r = ground_at(&gen, x * 1000, z * 1000, GroundQuery::Point);
                if r.grounded && r.slope > 0.6 {
                    assert!(r.normal[1] < 0.95, "slope {} but normal straight up", r.slope);
                    // Normal geometry: slope = sqrt(1 - ny²)/ny within the
                    // central-difference approximation.
                    let geom = ((1.0 - r.normal[1] * r.normal[1]).sqrt()) / r.normal[1];
                    assert!(
                        (r.slope - geom).abs() < 0.35,
                        "slope {} vs normal geometry {geom}",
                        r.slope
                    );
                    return;
                }
            }
        }
        panic!("no sloped column found near origin (seed 42)");
    }

    #[test]
    fn p3dn04_foundation_refuses_channels_and_accepts_land() {
        let gen = WorldGen::new(42);
        let (mx, mz) = channel_column(&gen);
        assert!(
            !foundation_ok(&gen, mx - 1500, mz - 1500, 3, 3),
            "a channel column must refuse a foundation"
        );
        let (wx, wz) = find_flat_column(&gen);
        assert!(
            foundation_ok(&gen, wx - 1000, wz - 1000, 3, 3),
            "flat dry land accepts a foundation"
        );
    }

    #[test]
    fn p3dn04_capsule_clearance_reports_headroom() {
        let gen = WorldGen::new(42);
        let (wx, wz) = find_flat_column(&gen);
        let r = ground_at(&gen, wx, wz, GroundQuery::Capsule { radius_m: 0.4, height_m: 1.8 });
        assert!(r.grounded);
        assert!(r.clearance_m >= 1.8, "sky clearance {} fits a 1.8 m body", r.clearance_m);
    }

    #[test]
    fn p3dn04_grounding_is_deterministic() {
        let gen = WorldGen::new(42);
        let (wx, wz) = find_flat_column(&gen);
        assert_eq!(
            ground_at(&gen, wx, wz, GroundQuery::Point),
            ground_at(&gen, wx, wz, GroundQuery::Point),
            "same query, same answer"
        );
    }
}
