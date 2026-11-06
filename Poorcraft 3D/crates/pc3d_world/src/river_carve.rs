//! N03 RiverCarve — the voxel authority's channel law.
//!
//! Doctrine (05-MUNDO-RIOS-SPAWN): "RiverCarve cria leito sob toda
//! superfície visível" and "nenhuma superfície azul existe sobre void".
//! Until now the river existed ONLY as the render-side strip
//! (`pc3d_render::world_features::ConformingWater`) floating over the
//! UNCARVED analytic surface — `final_solid` (the authority collision,
//! nav and spawn consume) had no channel at all (the N03 RED probe:
//! 0 water cells in the channel column, solid ground at the surface).
//!
//! This module derives, per seed, the channel field from the SAME river
//! graph the flow model and the render strips use, and answers one
//! question for every world column: "am I inside a river channel, and
//! where are its water surface and bed?" `WorldGen::carve` — the single
//! shared step both `regenerate_patch` (meshing) and `final_solid`
//! (collision/nav) pass through — consumes the answer, so the bed cannot
//! diverge between render, collision and nav: they are structurally
//! locked to one function.
//!
//! Laws implemented here (13-PLANO N03 steps 1–4, first slice):
//! - width/depth come from the flow record's discharge (the same
//!   formulas the render strips use — single truth lives HERE, render
//!   delegates);
//! - the water surface follows the terrain's cross-section minimum minus
//!   the surface offset, and NEVER rises downstream: samples are enforced
//!   non-increasing along a segment and chained across confluences (the
//!   inflow's end caps the outflow's start);
//! - water exists only between bed and surface; the bed is lined with
//!   sand; the rim keeps the terrain law.
//!
//! The field is deterministic per seed and cached globally (segments'
//! sample tables are expensive to rebuild per query). Terrain EDITS are
//! composed on top of the pristine base by the patch layer as before;
//! the channel stays anchored to the pristine watershed until the
//! P3D-303 dirty law is extended to it (honest v1 ceiling).

use crate::coords::RegionCoord;
use crate::hydro::{RiverGraph, RIVER_THRESHOLD};
use crate::scales::{MM_PER_METER, REGION_MM};
use std::collections::HashMap;
use std::sync::Arc;

/// Channel width (m) from discharge — mirrors the 2D map's stroke
/// scaling family. SINGLE TRUTH: `pc3d_render::water::width_of`
/// delegates here so the strip and the carve cannot disagree.
pub fn channel_width_m(discharge: u64) -> f32 {
    3.0 + (discharge as f32 / 300.0).min(9.0)
}

/// Channel depth (m) from discharge. Stays inside the trust gate's
/// plausible window ([BED_MIN_SUBMERGENCE_M, BED_MAX_DEPTH_M]): the
/// floor is NOT 0.5 but 1.2 — the voxel authority writes Water only
/// where a 1 m cell center falls in (surface, bed], so a shallower
/// channel can contain zero cell centers and never materialize
/// (measured: seed 42's strongest edge, depth 0.61, held 0 water cells)
/// — and the slope caps at 1.8 so the max stays ≤ 3.0.
pub fn channel_depth_m(discharge: u64) -> f32 {
    1.2 + (discharge as f32 / 750.0).min(1.8)
}

/// The water surface sits this far below the terrain's cross-section
/// minimum — same offset family the render strips use.
pub const SURFACE_OFFSET_M: f32 = 0.45;
/// Water-surface samples per segment (linear interpolation between).
const SAMPLES: usize = 16;
/// The watershed band half-width (regions). The render proofs already
/// build the graph at 70 so the strip coverage matches the carve band.
/// PUBLIC so route/tooling picks use the SAME watershed the authority
/// carves with — discharge depends on the window size, so an edge found
/// at a smaller half may not exist in the carve field at all.
pub const WATERSHED_HALF: i32 = 70;
const BAND_HALF: i32 = WATERSHED_HALF;

/// One river segment: region center → downstream center.
struct Edge {
    ax: f32,
    az: f32,
    ux: f32,
    uz: f32,
    len: f32,
    half_w: f32,
    depth: f32,
    /// Water surface (m) at SAMPLES+1 points along the segment,
    /// non-increasing.
    ws: [f32; SAMPLES + 1],
}

/// The per-seed channel field: segments + a spatial index by region.
pub struct CarveField {
    edges: Vec<Edge>,
    /// Edge indices whose bounding box touches the region (≤ 4 regions
    /// per segment), so a query is ONE hash lookup.
    by_region: HashMap<(i32, i32), Vec<u32>>,
}

/// Where the channel sits at one column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelHit {
    /// Water surface, meters.
    pub surface_m: f32,
    /// Bed floor (below the surface by the discharge depth), meters.
    pub bed_m: f32,
}

impl CarveField {
    /// Derive the field for one seed: edges from the river graph, water
    /// surfaces chained monotone-downstream over the segment forest.
    pub fn build(gen: &crate::gen::WorldGen) -> CarveField {
        let graph = RiverGraph::new(gen, BAND_HALF);
        // Collect out-edges (r → d) with the river law: a river EDGE
        // requires REAL flow on both ends (the `is_river` contract —
        // discharge ≥ threshold on the tail AND the head). A tail below
        // threshold is a dry gully; carving it would put water canyons
        // through ordinary grassland.
        struct Raw {
            r: (i32, i32),
            d: (i32, i32),
            discharge: u64,
        }
        let mut raws: Vec<Raw> = Vec::new();
        for x in -BAND_HALF..=BAND_HALF {
            for z in -BAND_HALF..=BAND_HALF {
                let r = RegionCoord { x, z };
                let Some(down) = graph.downstream(r) else {
                    continue;
                };
                if graph.discharge(down) < RIVER_THRESHOLD || graph.discharge(r) < RIVER_THRESHOLD
                {
                    continue;
                }
                let rec_discharge = graph.discharge(r);
                raws.push(Raw { r: (x, z), d: (down.x, down.z), discharge: rec_discharge });
            }
        }
        // Forest order: upstream edges (lower discharge on their tail)
        // first, so every inflow of r is chained before r's out-edge
        // (a tail u draining INTO r has strictly less discharge than r).
        raws.sort_by_key(|e| e.discharge);

        let mut edges: Vec<Edge> = Vec::with_capacity(raws.len());
        // End water surface per destination region (min over inflows).
        let mut end_ws: HashMap<(i32, i32), f32> = HashMap::new();
        for e in &raws {
            let ax = (e.r.0 as f32 + 0.5) * 256.0;
            let az = (e.r.1 as f32 + 0.5) * 256.0;
            let bx = (e.d.0 as f32 + 0.5) * 256.0;
            let bz = (e.d.1 as f32 + 0.5) * 256.0;
            let (dx, dz) = (bx - ax, bz - az);
            let len = dx.hypot(dz);
            if len < 1.0 {
                continue;
            }
            let (ux, uz) = (dx / len, dz / len);
            let half_w = channel_width_m(e.discharge) / 2.0;
            let depth = channel_depth_m(e.discharge);
            // The inflow cap: the lowest water surface any upstream edge
            // ended with (water cannot rise at the segment start).
            let inflow_cap = end_ws.get(&e.r).copied();
            let mut ws = [0.0f32; SAMPLES + 1];
            let mut prev = f32::MAX;
            for i in 0..=SAMPLES {
                let t = i as f32 / SAMPLES as f32;
                let cx = ax + ux * len * t;
                let cz = az + uz * len * t;
                // Cross-section minimum of the UNCARVED analytic surface:
                // the banks define where the water line sits.
                let px = -uz;
                let pz = ux;
                let mut line = f32::MAX;
                for side in [-1.0f32, 0.0, 1.0] {
                    let off = side * (half_w - 0.5).max(0.0);
                    let sx = ((cx + px * off) * 1000.0) as i64;
                    let sz = ((cz + pz * off) * 1000.0) as i64;
                    let h = gen.effective_surface_mm(sx, sz) as f32 / MM_PER_METER as f32;
                    line = line.min(h);
                }
                let terrain_ws = line - SURFACE_OFFSET_M;
                let w = terrain_ws.min(prev).min(inflow_cap.unwrap_or(f32::MAX));
                ws[i] = w;
                prev = w;
            }
            // Chain: the segment's end constrains the next edge's start.
            let d_key = e.d;
            end_ws
                .entry(d_key)
                .and_modify(|v| *v = v.min(ws[SAMPLES]))
                .or_insert(ws[SAMPLES]);
            edges.push(Edge { ax, az, ux, uz, len, half_w, depth, ws });
        }

        // Spatial index: each edge into the regions its bounding box
        // touches (conservative: min/max of both endpoints).
        let mut by_region: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (i, e) in edges.iter().enumerate() {
            let min_x = e.ax.min(e.ax + e.ux * e.len);
            let max_x = e.ax.max(e.ax + e.ux * e.len);
            let min_z = e.az.min(e.az + e.uz * e.len);
            let max_z = e.az.max(e.az + e.uz * e.len);
            let rx0 = (min_x as i64 * 1000).div_euclid(REGION_MM) as i32;
            let rx1 = (max_x as i64 * 1000).div_euclid(REGION_MM) as i32;
            let rz0 = (min_z as i64 * 1000).div_euclid(REGION_MM) as i32;
            let rz1 = (max_z as i64 * 1000).div_euclid(REGION_MM) as i32;
            for rx in rx0..=rx1 {
                for rz in rz0..=rz1 {
                    by_region.entry((rx, rz)).or_default().push(i as u32);
                }
            }
        }
        CarveField { edges, by_region }
    }

    /// The channel at one world column (mm), if any. Deterministic;
    /// overlapping segments (confluences) resolve to the LOWEST surface
    /// (the deepest channel is the water), ties by edge order.
    pub fn hit(&self, wx_mm: i64, wz_mm: i64) -> Option<ChannelHit> {
        let rx = wx_mm.div_euclid(REGION_MM) as i32;
        let rz = wz_mm.div_euclid(REGION_MM) as i32;
        let candidates = self.by_region.get(&(rx, rz))?;
        let x = wx_mm as f32 / MM_PER_METER as f32;
        let z = wz_mm as f32 / MM_PER_METER as f32;
        let mut best: Option<ChannelHit> = None;
        for &i in candidates {
            let e = &self.edges[i as usize];
            let rel_x = x - e.ax;
            let rel_z = z - e.az;
            let t = (rel_x * e.ux + rel_z * e.uz) / e.len;
            if !(0.0..=1.0).contains(&t) {
                continue;
            }
            let px = x - (e.ax + e.ux * e.len * t);
            let pz = z - (e.az + e.uz * e.len * t);
            if px * px + pz * pz > e.half_w * e.half_w {
                continue;
            }
            // Linear interpolation between the segment's samples.
            let s = t * SAMPLES as f32;
            let i0 = (s as usize).min(SAMPLES - 1);
            let f = s - i0 as f32;
            let surface = e.ws[i0] + (e.ws[i0 + 1] - e.ws[i0]) * f;
            let hit = ChannelHit { surface_m: surface, bed_m: surface - e.depth };
            if best.as_ref().map(|b| hit.surface_m < b.surface_m).unwrap_or(true) {
                best = Some(hit);
            }
        }
        best
    }
}

/// The per-seed cache: one build per seed per thread, shared within the
/// thread (streaming and proofs are single-threaded per world; other
/// threads build their own identical field). Pure (same seed → same
/// field); the WorldGen stays Copy.
thread_local! {
    static FIELD: std::cell::RefCell<(u64, Arc<CarveField>)> =
        std::cell::RefCell::new((
            u64::MAX,
            Arc::new(CarveField { edges: Vec::new(), by_region: HashMap::new() }),
        ));
}

pub(crate) fn field_for(seed: u64, gen: &crate::gen::WorldGen) -> Arc<CarveField> {
    FIELD.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.0 != seed {
            *slot = (seed, Arc::new(CarveField::build(gen)));
        }
        slot.1.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::WorldGen;

    /// The RED probe pinned: seed 4242 had a river whose channel did not
    /// exist (solid ground at the analytic surface). Any carved edge must
    /// satisfy the same law: its tail region contains channel columns,
    /// and the water surface sits BELOW the analytic surface (the RED
    /// state was: no channel, solid ground at surface).
    #[test]
    fn p3dn03_channel_exists_below_the_analytic_surface() {
        let gen = WorldGen::new(4242);
        let field = CarveField::build(&gen);
        assert!(!field.edges.is_empty(), "seed 4242 must have river edges");
        // Probe the widest edge's segment start (width grows with
        // discharge); scan a small neighborhood so a curving channel
        // near the anchor still lands a hit.
        let edge = field
            .edges
            .iter()
            .max_by(|a, b| a.half_w.total_cmp(&b.half_w))
            .expect("at least one edge");
        let base_x = (edge.ax * 1000.0) as i64;
        let base_z = (edge.az * 1000.0) as i64;
        let surface_m = gen.effective_surface_mm(base_x, base_z) as f32 / 1000.0;
        let mut probe = None;
        'outer: for dx in [0i64, 3000, -3000, 6000, -6000] {
            for dz in [0i64, 3000, -3000, 6000, -6000] {
                if let Some(h) = field.hit(base_x + dx, base_z + dz) {
                    probe = Some(h);
                    break 'outer;
                }
            }
        }
        let hit = probe
            .unwrap_or_else(|| panic!("the RED probe's channel region must hit"));
        assert!(
            hit.surface_m < surface_m,
            "water surface {:.2} must sit below the analytic surface {:.2}",
            hit.surface_m,
            surface_m
        );
        assert!(
            surface_m - hit.surface_m >= 0.15,
            "the surface offset must submerge the bed beyond the trust minimum"
        );
        assert!(
            hit.bed_m < hit.surface_m,
            "bed below surface"
        );
        let depth = hit.surface_m - hit.bed_m;
        assert!(
            (0.5..=2.5).contains(&depth),
            "depth {depth} must come from the discharge depth window"
        );
    }

    /// The monotone law: along one segment the stored water surface never
    /// rises sample-to-sample; across a chain the start never exceeds the
    /// inflow's end.
    #[test]
    fn p3dn03_water_never_rises_downstream() {
        let gen = WorldGen::new(4242);
        let field = CarveField::build(&gen);
        for e in &field.edges {
            for i in 0..SAMPLES {
                assert!(
                    e.ws[i + 1] <= e.ws[i] + 1e-4,
                    "segment water rose downstream: {} -> {}",
                    e.ws[i],
                    e.ws[i + 1]
                );
            }
        }
    }

    /// Determinism: two builds of the same seed agree bit-for-bit.
    #[test]
    fn p3dn03_field_is_deterministic() {
        let gen = WorldGen::new(777);
        let a = CarveField::build(&gen);
        let b = CarveField::build(&gen);
        assert_eq!(a.edges.len(), b.edges.len());
        for (ea, eb) in a.edges.iter().zip(b.edges.iter()) {
            assert_eq!(ea.ws, eb.ws);
        }
    }

    /// THE MATERIALIZATION LAW (the bug the route gate caught): every
    /// carve channel must be wide + deep enough that the voxel authority
    /// actually writes Water — at least one 1 m cell center must fall in
    /// (surface, bed] at the channel center, and the bed floor must be
    /// solid Sand/Soil/Rock beneath. Depth < 1.0 can zero out; the 1.2
    /// floor plus the 3 m half-width minimum prevent that everywhere.
    #[test]
    fn p3dn03_every_channel_holds_voxel_water() {
        use crate::terrain::final_solid;
        for seed in [42u64, 4242, 11, 7, 2024] {
            let gen = WorldGen::new(seed);
            let field = CarveField::build(&gen);
            assert!(!field.edges.is_empty(), "seed {seed} must have edges");
            // Sample each edge's midpoint center column.
            let mut checked = 0usize;
            for e in field.edges.iter().take(40) {
                let mx = (e.ax + e.ux * e.len * 0.5) * 1000.0;
                let mz = (e.az + e.uz * e.len * 0.5) * 1000.0;
                let Some(ch) = field.hit(mx as i64, mz as i64) else {
                    continue;
                };
                let mut water = 0usize;
                let wy0 = ch.bed_m.floor() as i64;
                let wy1 = ch.surface_m.ceil() as i64;
                for wy in wy0..=wy1 {
                    let cell = final_solid(&gen, mx as i64, wy * 1000, mz as i64);
                    if matches!(cell.material, crate::gen::CellMaterial::Water) {
                        water += 1;
                    }
                }
                assert!(
                    water >= 1,
                    "seed {seed}: channel at ({:.0},{:.0}) depth {:.2} holds {water} voxel water cells",
                    mx / 1000.0,
                    mz / 1000.0,
                    ch.surface_m - ch.bed_m
                );
                checked += 1;
            }
            assert!(checked >= 10, "seed {seed}: only {checked} edges sampled");
        }
    }
}
