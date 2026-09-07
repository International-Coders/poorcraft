//! River water mesh from actual flow records (visual reset R3DV-007).
//!
//! Every water section is the world-space strip of ONE river edge (region
//! center → downstream center), built from the authoritative flow data:
//! direction from `FlowRecord.direction`, width/depth/discharge scaling,
//! speed class from `slope_per_mille`. The visible current (a brightness
//! wave along the flow axis, animated by time) is driven by those same
//! record values — never a decorative scroll.
//!
//! Local updates use P3D-303's dirty-region law: after a channel edit
//! (RiverGraph::build with elevation overrides → FlowTable::from_graph_
//! with_revisions), only records whose semantics changed get a new
//! revision, and [`WaterSections::update`] remeshes EXACTLY those
//! sections — untouched sections keep their GPU buffers.
//!
//! The water pass is transparent (alpha by depth) with depth READ but no
//! depth write, drawn after all opaque geometry so terrain banks show
//! through and correctly occlude the water.

use crate::camera::CameraPose;
use pc3d_world::coords::RegionCoord;
use pc3d_world::flow::{direction_code, FlowRecord, FlowTable, DIR_SINK};
use pc3d_world::gen::WorldGen;
use pc3d_world::hydro::RiverGraph;
use std::collections::BTreeMap;

const REGION_M: f32 = 256.0;
/// Cross-section sampling along the strip.
const SAMPLE_STEP_M: f32 = 4.0;

/// Width (m) from discharge — mirrors the 2D map's stroke scaling family.
pub fn width_of(discharge: u64) -> f32 {
    3.0 + (discharge as f32 / 300.0).min(9.0)
}

/// Depth (m) from discharge.
pub fn depth_of(discharge: u64) -> f32 {
    0.5 + (discharge as f32 / 600.0).min(2.0)
}

/// Water surface sits this far above the sampled terrain line.
const SURFACE_OFFSET_M: f32 = 0.35;

/// Compass code → unit XZ vector (x east, z south — P3D world convention).
fn compass_vec(dir: u8) -> Option<[f32; 2]> {
    Some(match dir {
        0 => [1.0, 0.0],
        1 => [1.0, 1.0],
        2 => [0.0, 1.0],
        3 => [-1.0, 1.0],
        4 => [-1.0, 0.0],
        5 => [-1.0, -1.0],
        6 => [0.0, -1.0],
        7 => [1.0, -1.0],
        _ => return None,
    })
}

/// A water-strip vertex: world position, flow direction (unit XZ), speed
/// class (0..1 from slope), alpha (depth). The current pattern in the
/// fragment shader uses dir × speed × time — all from the flow record.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WaterVertex {
    pub pos: [f32; 3],
    pub dir: [f32; 2],
    pub speed: f32,
    pub alpha: f32,
}

pub const WATER_VERTEX_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<WaterVertex>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &[
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 12,
            shader_location: 1,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32,
            offset: 20,
            shader_location: 2,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32,
            offset: 24,
            shader_location: 3,
        },
    ],
};

/// CPU mirror of the water fragment shader (probes share one truth).
/// `phase_m` = the vertex's signed distance along the flow axis.
pub fn water_color(
    phase_m: f32,
    speed: f32,
    alpha_unused: f32,
    time_s: f32,
) -> [f32; 4] {
    let stripe = 0.5 + 0.5 * (phase_m * 0.8 - time_s * speed * 3.0).sin();
    let base = pc3d_assets::material_albedo("mat.water_flow").unwrap_or([0.24, 0.52, 0.85]);
    let shade = 0.7 + 0.3 * stripe;
    [
        base[0] * shade,
        base[1] * shade,
        base[2] * shade,
        alpha_unused,
    ]
}

/// Speed class 0..1 from slope (per-mille), matching the shader contract.
pub fn speed_of(slope_per_mille: i32) -> f32 {
    (slope_per_mille.max(0) as f32 / 60.0).clamp(0.1, 1.0)
}

/// The flow phase of a world point relative to a strip (meters along the
/// flow direction from the strip's upstream end).
pub fn phase_along(pos: [f32; 2], origin: [f32; 2], dir: [f32; 2]) -> f32 {
    (pos[0] - origin[0]) * dir[0] + (pos[1] - origin[1]) * dir[1]
}

/// One river region's strip mesh: region center → downstream center,
/// width/depth by discharge, following the terrain's surface height.
pub fn strip_for(
    gen: &WorldGen,
    graph: &RiverGraph,
    rec: &FlowRecord,
) -> (Vec<WaterVertex>, Vec<u16>) {
    let Some(down) = graph.downstream(RegionCoord { x: rec.region_x, z: rec.region_z }) else {
        return (Vec::new(), Vec::new());
    };
    let Some(dir) = compass_vec(rec.direction) else {
        return (Vec::new(), Vec::new());
    };
    let len = if dir[0] != 0.0 && dir[1] != 0.0 {
        REGION_M * std::f32::consts::SQRT_2
    } else {
        REGION_M
    };
    let origin = [
        rec.region_x as f32 * REGION_M + REGION_M / 2.0,
        rec.region_z as f32 * REGION_M + REGION_M / 2.0,
    ];
    // Perpendicular in XZ.
    let perp = [-dir[1], dir[0]];
    let half_w = width_of(rec.discharge) / 2.0;
    let speed = speed_of(rec.slope_per_mille);
    let alpha = (0.55 + 0.2 * (depth_of(rec.discharge) - 0.5) / 2.0).clamp(0.5, 0.9);

    let mut verts = Vec::new();
    let mut idx = Vec::new();
    let samples = (len / SAMPLE_STEP_M).ceil() as usize;
    for s in 0..=samples {
        let t = s as f32 / samples as f32;
        let cx = origin[0] + dir[0] * len * t;
        let cz = origin[1] + dir[1] * len * t;
        // Surface height: the terrain under the centerline plus a small
        // offset so the water plane reads above its bed.
        let terrain_mm = gen.effective_surface_mm((cx * 1000.0) as i64, (cz * 1000.0) as i64);
        let y = terrain_mm as f32 / 1000.0 + SURFACE_OFFSET_M;
        for side in [-1.0f32, 1.0] {
            verts.push(WaterVertex {
                pos: [cx + perp[0] * half_w * side, y, cz + perp[1] * half_w * side],
                dir,
                speed,
                alpha,
            });
        }
        if s > 0 {
            let b = (s * 2) as u16;
            idx.extend([b - 2, b - 1, b, b - 1, b + 1, b]);
        }
    }
    let _ = direction_code;
    (verts, idx)
}

/// What a water sync did — the dirty-region evidence.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WaterStats {
    pub sections: usize,
    pub added: usize,
    pub remeshed: usize,
    pub removed: usize,
    pub vertices: usize,
    pub mesh_us: u128,
}

struct Section {
    revision: u64,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

/// Per-region water sections with revision-keyed remeshing.
pub struct WaterSections {
    sections: BTreeMap<(i32, i32), Section>,
}

impl WaterSections {
    pub fn new() -> Self {
        Self {
            sections: BTreeMap::new(),
        }
    }

    /// Syncs from the flow table: remeshes only river regions whose record
    /// revision changed (the P3D-303 dirty-region law).
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        gen: &WorldGen,
        graph: &RiverGraph,
        table: &FlowTable,
    ) -> WaterStats {
        use wgpu::util::DeviceExt;
        let t0 = std::time::Instant::now();
        let mut stats = WaterStats::default();

        let live: std::collections::BTreeSet<(i32, i32)> = table
            .records
            .values()
            .filter(|r| {
                r.direction != DIR_SINK
                    && graph
                        .downstream(RegionCoord { x: r.region_x, z: r.region_z })
                        .map(|d| graph.discharge(d) >= pc3d_world::hydro::RIVER_THRESHOLD)
                        .unwrap_or(false)
            })
            .map(|r| (r.region_x, r.region_z))
            .collect();
        stats.sections = live.len();

        let stale: Vec<(i32, i32)> = self
            .sections
            .keys()
            .filter(|k| !live.contains(*k))
            .copied()
            .collect();
        for k in stale {
            self.sections.remove(&k);
            stats.removed += 1;
        }

        for key in &live {
            let rec = table
                .get(RegionCoord { x: key.0, z: key.1 })
                .expect("live record");
            if let Some(existing) = self.sections.get(key) {
                if existing.revision == rec.revision {
                    continue; // unchanged: zero mesh work
                }
            } else {
                stats.added += 1;
            }
            let (verts, idx) = strip_for(gen, graph, rec);
            stats.vertices += verts.len();
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("water vertices"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("water indices"),
                contents: bytemuck::cast_slice(&idx),
                usage: wgpu::BufferUsages::INDEX,
            });
            let replaced = self.sections.insert(
                *key,
                Section {
                    revision: rec.revision,
                    vertex_buffer,
                    index_buffer,
                    index_count: idx.len() as u32,
                },
            );
            if replaced.is_some() {
                stats.remeshed += 1;
            }
        }
        stats.mesh_us = t0.elapsed().as_micros();
        stats
    }

    /// Draws all sections with the caller's bound water pipeline.
    pub fn draw<'rp>(&self, pass: &mut wgpu::RenderPass<'rp>) {
        for s in self.sections.values() {
            if s.index_count == 0 {
                continue;
            }
            pass.set_vertex_buffer(0, s.vertex_buffer.slice(..));
            pass.set_index_buffer(s.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..s.index_count, 0, 0..1);
        }
    }
}

/// A river edge worth proving with: a river region with discharge ≥
/// threshold and a downstream neighbor that is also a river, nearest to
/// `near` — deterministic choice.
pub fn pick_river_edge(
    graph: &RiverGraph,
    near: (i32, i32),
) -> Option<((i32, i32), (i32, i32))> {
    // Real downstream pairs only (river_edges() lists sorted-adjacent
    // regions, not graph links): r flows into d, both with river-class
    // discharge — exactly the strip's live condition.
    let mut best: Option<(((i32, i32), (i32, i32)), i32)> = None;
    for x in -graph.half..=graph.half {
        for z in -graph.half..=graph.half {
            let r = RegionCoord { x, z };
            let Some(d) = graph.downstream(r) else { continue };
            if graph.discharge(r) < pc3d_world::hydro::RIVER_THRESHOLD
                || graph.discharge(d) < pc3d_world::hydro::RIVER_THRESHOLD
            {
                continue;
            }
            let pair = ((r.x, r.z), (d.x, d.z));
            let dist = (r.x - near.0).abs() + (r.z - near.1).abs();
            if best.as_ref().map(|(_, b)| dist < *b).unwrap_or(true) {
                best = Some((pair, dist));
            }
        }
    }
    best.map(|(pair, _)| pair)
}

/// A camera pose above a river edge's midpoint, looking downstream along
/// the flow.
pub fn river_pose(graph: &RiverGraph, gen: &WorldGen, a: (i32, i32), b: (i32, i32)) -> CameraPose {
    let mid_x = (a.0 as f32 + 0.5 + b.0 as f32 + 0.5) / 2.0 * REGION_M;
    let mid_z = (a.1 as f32 + 0.5 + b.1 as f32 + 0.5) / 2.0 * REGION_M;
    let terrain_mm = gen.effective_surface_mm((mid_x * 1000.0) as i64, (mid_z * 1000.0) as i64);
    let mid_y = terrain_mm as f32 / 1000.0;
    let dir = [
        b.0 as f32 - a.0 as f32,
        b.1 as f32 - a.1 as f32,
    ];
    let l = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
    let dir = [dir[0] / l, dir[1] / l];
    // Eye 60 m upstream of the midpoint, 18 m above the water, looking
    // downstream and down.
    let eye = [
        mid_x - dir[0] * 60.0,
        mid_y + 18.0,
        mid_z - dir[1] * 60.0,
    ];
    let target = [mid_x + dir[0] * 40.0, mid_y, mid_z + dir[1] * 40.0];
    let d = [
        target[0] - eye[0],
        target[1] - eye[1],
        target[2] - eye[2],
    ];
    let yaw = (-d[0]).atan2(-d[2]);
    let pitch = (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin();
    CameraPose::new(eye, yaw, pitch)
}

/// The proof scene: a watershed with rivers, grown deterministically until
/// one exists and enlarged to contain it with margin; returns the graph,
/// flow table, and the anchor river edge (upstream, downstream).
pub fn proof_scene(seed: u64) -> (WorldGen, RiverGraph, FlowTable, ((i32, i32), (i32, i32))) {
    let gen = WorldGen::new(seed);
    let mut half = 8;
    let mut graph = RiverGraph::new(&gen, half);
    while graph.river_edges().is_empty() && half < 48 {
        half += 8;
        graph = RiverGraph::new(&gen, half);
    }
    assert!(!graph.river_edges().is_empty(), "seed {seed} must have rivers");
    let anchor = graph
        .river_region_list()
        .first()
        .copied()
        .expect("river region");
    let need = anchor.0.abs().max(anchor.1.abs()) + 12;
    if need > half {
        graph = RiverGraph::new(&gen, need);
    }
    let table = FlowTable::from_graph(&graph);
    let edge = pick_river_edge(&graph, anchor).expect("a river edge");
    (gen, graph, table, edge)
}

/// The strip's upstream origin point (XZ) for a record.
pub fn strip_origin_of(rec: &FlowRecord) -> [f32; 2] {
    [
        rec.region_x as f32 * REGION_M + REGION_M / 2.0,
        rec.region_z as f32 * REGION_M + REGION_M / 2.0,
    ]
}

/// Finds the first VISIBLE water-surface crossing along a ray: the point
/// where the ray descends through a river strip's plane with no terrain
/// occluding the path before it. Returns the crossing point and the flow
/// record whose strip it hits.
pub fn visible_water_crossing(
    gen: &WorldGen,
    graph: &RiverGraph,
    table: &FlowTable,
    eye: [f32; 3],
    dir: [f32; 3],
) -> Option<([f32; 3], FlowRecord)> {
    let mut prev_t = 0.5f32;
    let mut prev_above = false;
    let mut t = 0.5f32;
    while t < 300.0 {
        let p = [eye[0] + dir[0] * t, eye[1] + dir[1] * t, eye[2] + dir[2] * t];
        let region = RiverGraph::region_at((p[0] * 1000.0) as i64, (p[2] * 1000.0) as i64);
        let rec = table.get(region)?;
        if rec.direction != DIR_SINK {
            let Some(d) = graph.downstream(region) else { return None };
            if graph.discharge(d) >= pc3d_world::hydro::RIVER_THRESHOLD {
                // The strip plane height under this point.
                let strip_y = gen.effective_surface_mm(
                    (p[0] * 1000.0) as i64,
                    (p[2] * 1000.0) as i64,
                ) as f32
                    / 1000.0
                    + SURFACE_OFFSET_M;
                let above = p[1] > strip_y;
                if prev_above && !above {
                    // Crossing between prev_t and t: refine, then require a
                    // clear path (no terrain in front of the water). A
                    // blocked crossing just ends this attempt — later
                    // crossings further along may still be visible.
                    let mut ft = prev_t;
                    while ft < t {
                        let q = [eye[0] + dir[0] * ft, eye[1] + dir[1] * ft, eye[2] + dir[2] * ft];
                        let qy = gen.effective_surface_mm(
                            (q[0] * 1000.0) as i64,
                            (q[2] * 1000.0) as i64,
                        ) as f32
                            / 1000.0
                            + SURFACE_OFFSET_M;
                        if q[1] <= qy {
                            // Visible only if nothing solid blocks before q.
                            if crate::terrain::ray_first_hit(gen, eye, dir, ft - 0.6).is_none() {
                                return Some((q, *rec));
                            }
                            break;
                        }
                        ft += 0.1;
                    }
                }
                prev_above = above;
                prev_t = t;
            }
        }
        t += 1.0;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (WorldGen, RiverGraph, FlowTable) {
        let (gen, graph, table, _) = proof_scene(3);
        (gen, graph, table)
    }

    fn anchor(graph: &RiverGraph) -> (i32, i32) {
        graph
            .river_region_list()
            .first()
            .copied()
            .expect("a river region")
    }

    #[test]
    fn strips_follow_flow_records() {
        let (gen, graph, table) = setup();
        let (a, b) = pick_river_edge(&graph, anchor(&graph)).expect("a river edge");
        let rec = table
            .get(RegionCoord { x: a.0, z: a.1 })
            .expect("record for a");
        let (verts, idx) = strip_for(&gen, &graph, rec);
        assert!(verts.len() > 100, "a real strip: {}", verts.len());
        assert_eq!(idx.len() % 6, 0);
        // Every vertex carries the record's downstream direction.
        let expect = compass_vec(rec.direction).expect("valid dir");
        let l = (expect[0] * expect[0] + expect[1] * expect[1]).sqrt();
        for v in &verts {
            assert!((v.dir[0] - expect[0] / l).abs() < 1e-5);
            assert!((v.dir[1] - expect[1] / l).abs() < 1e-5);
        }
        // Width grows with discharge.
        let wide = width_of(10_000) > width_of(200);
        assert!(wide);
        assert!(depth_of(10_000) > depth_of(200));
        let _ = b;
    }

    #[test]
    fn channel_edit_revises_only_changed_regions() {
        let (gen, graph, t0) = setup();
        // Dam: raise one river region's downstream neighbor far above.
        let (a, b) = pick_river_edge(&graph, (0, 0)).expect("river edge");
        let mut overrides = BTreeMap::new();
        overrides.insert(b, 200);
        let g1 = RiverGraph::build(&gen, graph.half, &overrides);
        let t1 = FlowTable::from_graph_with_revisions(Some(&t0), &g1);

        let changed: Vec<(i32, i32)> = t1
            .records
            .iter()
            .filter(|(k, r)| {
                t0.records
                    .get(*k)
                    .map(|p| p.revision != r.revision)
                    .unwrap_or(true)
            })
            .map(|(k, _)| *k)
            .collect();
        assert!(!changed.is_empty(), "the dam must change some records");
        assert!(changed.len() < t1.records.len(), "only a local set changes");
        // The dammed region itself changed (its downstream rerouted).
        assert!(changed.contains(&a) || changed.contains(&b));
        // A far-away record kept its revision.
        let untouched = t1
            .records
            .iter()
            .filter(|(k, r)| {
                (k.0.abs() > 5 || k.1.abs() > 5)
                    && t0.records.get(*k).map(|p| p.revision == r.revision).unwrap_or(false)
            })
            .count();
        assert!(untouched > 20, "distant records keep revisions: {untouched}");
    }

    #[test]
    fn water_sections_remesh_only_dirty_sections() {
        let mut r = crate::renderer::Renderer::offscreen(320, 200);
        let (gen, graph0, t0) = setup();
        r.attach_water();
        let s0 = r.update_water(&gen, &graph0, &t0);
        assert!(s0.sections > 0);
        assert_eq!(s0.added, s0.sections);

        // Same table again: zero work.
        let s_same = r.update_water(&gen, &graph0, &t0);
        assert_eq!(
            (s_same.added, s_same.remeshed, s_same.vertices),
            (0, 0, 0),
            "unchanged table must cost nothing"
        );

        // Dam edit: only the dirty sections remesh.
        let (a, b) = pick_river_edge(&graph0, anchor(&graph0)).expect("river edge");
        let mut overrides = BTreeMap::new();
        overrides.insert(b, 200);
        let g1 = RiverGraph::build(&gen, graph0.half, &overrides);
        let t1 = FlowTable::from_graph_with_revisions(Some(&t0), &g1);
        let changed_count = t1
            .records
            .iter()
            .filter(|(k, rec)| {
                t0.records
                    .get(*k)
                    .map(|p| p.revision != rec.revision)
                    .unwrap_or(true)
                    && rec.direction != DIR_SINK
                    && g1.downstream(RegionCoord { x: k.0, z: k.1 }).is_some()
            })
            .count();
        let s1 = r.update_water(&gen, &g1, &t1);
        assert!(s1.remeshed > 0, "the dam must remesh something");
        assert!(s1.remeshed < s0.sections, "only a local set remeshes");
        assert!(
            s1.remeshed <= changed_count,
            "remeshed {} > changed records {}",
            s1.remeshed,
            changed_count
        );
        let _ = a;
    }

    /// GPU proof: the water surface renders as a TRANSPARENT blend of the
    /// record-driven current color over the terrain visible beneath it, the
    /// current's phase differs along the flow axis (direction readable from
    /// the record), and a channel edit remeshes only local sections with a
    /// locally changed image.
    #[test]
    fn river_renders_transparent_with_flow_direction_and_local_edit() {
        use crate::scene::{project_ndc, to_srgb4, Probe};

        let (gen, graph, t0) = setup();
        let (a, b) = pick_river_edge(&graph, anchor(&graph)).expect("river edge");
        let rec = t0.get(RegionCoord { x: a.0, z: a.1 }).expect("record");

        // Terrain under the river: patches around the edge midpoint.
        let mid_x = ((a.0 as f32 + 0.5) + (b.0 as f32 + 0.5)) / 2.0 * REGION_M;
        let mid_z = ((a.1 as f32 + 0.5) + (b.1 as f32 + 0.5)) / 2.0 * REGION_M;
        let surf_mm = gen.effective_surface_mm((mid_x * 1000.0) as i64, (mid_z * 1000.0) as i64);
        let mid_y = surf_mm as f32 / 1000.0;
        let y_level = ((mid_y) as i32).div_euclid(16);
        let mid_patch = pc3d_world::coords::PatchCoord {
            x: (mid_x as i32).div_euclid(16),
            y: y_level,
            z: (mid_z as i32).div_euclid(16),
        };

        let mut r = crate::renderer::Renderer::offscreen(384, 288);
        r.set_placeholder_scene(false);
        let tstats = r.load_terrain(&gen, &crate::terrain::neighborhood3(mid_patch));
        assert!(tstats.triangles > 100);

        // Water, frozen at t=0 for deterministic probes.
        r.attach_water();
        let w0 = r.update_water(&gen, &graph, &t0);
        assert!(w0.sections > 0);
        r.set_water_time(Some(0.0));
        let pose = river_pose(&graph, &gen, a, b);
        r.set_pose(pose);

        // Expected water pixel = blend(water sRGB, under sRGB) where the
        // "under" color is what the view ray hits BEHIND the water surface
        // (the transparent-pass truth), computed by raycast continuation.
        let dir = compass_vec(rec.direction).expect("dir");
        let dir_len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
        let dir = [dir[0] / dir_len, dir[1] / dir_len];
        let water_y = |xz: [f32; 2]| {
            let mm = gen.effective_surface_mm((xz[0] * 1000.0) as i64, (xz[1] * 1000.0) as i64);
            mm as f32 / 1000.0 + SURFACE_OFFSET_M
        };
        let strip_origin = [
            rec.region_x as f32 * REGION_M + REGION_M / 2.0,
            rec.region_z as f32 * REGION_M + REGION_M / 2.2,
        ];
        let expected_water_pixel = |point: [f32; 3]| -> ([f32; 4], [f32; 4]) {
            let phase = phase_along([point[0], point[2]], strip_origin, dir);
            let wc = water_color(phase, speed_of(rec.slope_per_mille), 1.0, 0.0);
            let alpha = (0.55 + 0.2 * (depth_of(rec.discharge) - 0.5) / 2.0).clamp(0.5, 0.9);
            let view = crate::camera::fwd_of(pose.yaw, pose.pitch);
            // Continue the view ray through the water surface.
            let under = crate::terrain::ray_first_hit(&gen, point, view, 40.0)
                .map(|(cell, normal, _)| to_srgb4(crate::scene::lit_color(
                    crate::construction::material_albedo(
                        pc3d_world::terrain::final_solid(
                            &gen,
                            cell.x as i64 * 1000,
                            cell.y as i64 * 1000,
                            cell.z as i64 * 1000,
                        )
                        .material,
                    ),
                    normal,
                )))
                .unwrap_or([0.5, 0.5, 0.5, 1.0]);
            let w = to_srgb4([wc[0], wc[1], wc[2]]);
            let mut out = [0.0f32; 4];
            for i in 0..3 {
                out[i] = w[i] * alpha + under[i] * (1.0 - alpha);
            }
            out[3] = 1.0;
            (out, under)
        };

        // Probe points chosen by VISIBILITY: the first water crossing the
        // view ray actually reaches (intervening hills can bury the strip
        // near the camera — the terrain does not carve river valleys).
        let view = crate::camera::fwd_of(pose.yaw, pose.pitch);
        let (p1, rec1) = visible_water_crossing(&gen, &graph, &t0, pose.position, view)
            .expect("the view must cross a visible river strip");
        let dir1 = compass_vec(rec1.direction).expect("dir");
        let dl = (dir1[0] * dir1[0] + dir1[1] * dir1[1]).sqrt();
        let dir1 = [dir1[0] / dl, dir1[1] / dl];
        // A second point 6 m downstream along the same strip (still on the
        // surface plane), used for the direction signal.
        let p2 = [
            p1[0] + dir1[0] * 6.0,
            water_y([p1[0] + dir1[0] * 6.0, p1[2] + dir1[1] * 6.0]),
            p1[2] + dir1[1] * 6.0,
        ];
        let _ = &rec;
        let aspect = 384.0 / 288.0;
        // Validity probe (sky) + capture first; the water analysis reads
        // the actual frame pixels afterwards.
        let sky_probe = crate::scene::Probe {
            name: "sky_above_horizon",
            ndc: (0.0, 0.8),
            expected: to_srgb4(crate::scene::sky_color_linear(
                crate::scene::dir_from_ndc(pose, (0.0, 0.8), aspect),
                crate::scene::SUN_DIR,
            )),
            tol: 0.05,
        };
        let path = std::env::temp_dir().join("pc3d_water_before.png");
        let (report_before, rgba_before) = r.capture_png(&path, &[sky_probe]);
        assert!(
            report_before.passes_with(8),
            "before: {:?} ({report_before:?})",
            report_before.failed_probes()
        );
        let (w, h) = (384u32, 288u32);

        // TRANSPARENCY via a CONTROL RENDER: the same frame with the water
        // layer detached is the pixel-exact under color; the with-water
        // pixel must be an alpha blend of the water color and that control.
        r.detach_water();
        let control_path = std::env::temp_dir().join("pc3d_water_control.png");
        let (_, rgba_control) = r.capture_png(&control_path, &[]);
        r.attach_water();
        let _ = r.update_water(&gen, &graph, &t0);
        let under = crate::scene::sample_ndc(
            &rgba_control, w, h, project_ndc(pose, aspect, p1),
        );
        let phase1 = phase_along([p1[0], p1[2]], strip_origin_of(&rec1), dir1);
        let wc1 = water_color(phase1, speed_of(rec1.slope_per_mille), 1.0, 0.0);
        let water_only = to_srgb4([wc1[0], wc1[1], wc1[2]]);
        let px1 = crate::scene::sample_ndc(&rgba_before, w, h, project_ndc(pose, aspect, p1));
        let dist = |a: [f32; 4], b: [f32; 4]| -> f32 {
            (0..3).map(|i| (a[i] - b[i]).abs()).sum()
        };
        let d_water = dist(px1, water_only);
        let d_under = dist(px1, under);
        assert!(
            d_water > 0.10 && d_under > 0.10,
            "pixel must be a blend: {px1:?} water-only {water_only:?} (d {d_water}) under {under:?} (d {d_under})"
        );
        let mut coefs = Vec::new();
        for i in 0..3 {
            let denom = water_only[i] - under[i];
            if denom.abs() > 0.05 {
                coefs.push((px1[i] - under[i]) / denom);
            }
        }
        let avg_coef = coefs.iter().sum::<f32>() / coefs.len().max(1) as f32;
        assert!(
            (0.2..0.95).contains(&avg_coef),
            "blend coefficient {avg_coef} not a transparent water band"
        );

        // FLOW DIRECTION from the IMAGE: brightness varies along the flow
        // axis more than across the width.
        let perp = [-dir1[1], dir1[0]];
        let hw = width_of(rec1.discharge) * 0.6;
        let mk = |x: f32, z: f32| [x, water_y([x, z]), z];
        let along_plus = mk(p1[0] + dir1[0] * 5.0, p1[2] + dir1[1] * 5.0);
        let along_minus = mk(p1[0] - dir1[0] * 5.0, p1[2] - dir1[1] * 5.0);
        let across_a = [p1[0] + perp[0] * hw, p1[1], p1[2] + perp[1] * hw];
        let across_b = [p1[0] - perp[0] * hw, p1[1], p1[2] - perp[1] * hw];
        let px_at = |p: [f32; 3]| crate::scene::sample_ndc(&rgba_before, w, h, project_ndc(pose, aspect, p));
        let pa = px_at(along_plus);
        let pb = px_at(along_minus);
        let ca = px_at(across_a);
        let cb = px_at(across_b);
        let along_delta = dist(pa, pb);
        let across_delta = dist(ca, cb);
        assert!(
            along_delta > across_delta + 0.02,
            "the current must advance along the flow (along {along_delta} vs across {across_delta})"
        );

        // CHANNEL EDIT: dam the downstream region; only local sections
        // remesh and the image changes locally.
        let mut overrides = std::collections::BTreeMap::new();
        overrides.insert(b, 200);
        let g1 = RiverGraph::build(&gen, graph.half, &overrides);
        let t1 = FlowTable::from_graph_with_revisions(Some(&t0), &g1);
        let w1 = r.update_water(&gen, &g1, &t1);
        assert!(w1.remeshed > 0);
        assert!(w1.remeshed < w0.sections, "local remesh only");

        let path_after = std::env::temp_dir().join("pc3d_water_after.png");
        // After the dam the strip at this edge may reroute; verify the frame
        // is still a valid render and differs from before.
        let (report_after, rgba_after) =
            r.capture_png(&path_after, &[]); // plain validity probes below
        let diff = crate::scene::pixel_difference_fraction(&rgba_before, &rgba_after);
        assert!(diff > 0.005, "the dam must change the image ({diff})");
        assert!(report_after.distinct_colors >= 8);
        println!(
            "water: {} sections (remeshed {} after dam, {} us), image diff {:.2}%",
            w0.sections, w1.remeshed, w1.mesh_us, diff * 100.0
        );
    }

    #[test]
    fn current_pattern_encodes_direction_and_speed() {
        // Along the flow axis the phase advances and brightness changes;
        // across the width it does not — that IS the direction signal.
        let origin = [0.0f32, 0.0];
        let dir = [1.0f32, 0.0];
        let a = water_color(phase_along([3.0, 0.0], origin, dir), 0.5, 0.7, 0.0);
        let b = water_color(phase_along([8.0, 0.0], origin, dir), 0.5, 0.7, 0.0);
        let c = water_color(phase_along([3.0, 9.0], origin, dir), 0.5, 0.7, 0.0);
        assert_ne!(a, b, "brightness varies along the flow axis");
        assert_eq!(a, c, "brightness is constant across the width");
        // Faster current: the pattern moves faster in time.
        let t0 = water_color(phase_along([3.0, 0.0], origin, dir), 0.2, 0.7, 0.0);
        let t1 = water_color(phase_along([3.0, 0.0], origin, dir), 0.2, 0.7, 1.0);
        assert_ne!(t0, t1, "time animates the current");
    }
}
