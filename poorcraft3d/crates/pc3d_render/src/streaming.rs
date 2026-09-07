//! Streamed terrain: real bounded mesh work on the existing P3D-105/P3D-206
//! machinery (visual reset R3DV-006).
//!
//! This module CONSUMES the world's own streaming primitives —
//! `stream::interest_patches` (disc rings), `stream::interest_diff`
//! (load/unload plans), `stream::BoundedQueue` (bounded work with honest
//! counters), `lod::lod_for` (detail bands) — and turns them into real
//! per-frame meshing and GPU uploads with caps:
//!
//! - **bounded_mesh_queue**: at most `max_mesh_per_frame` patches mesh per
//!   frame; overflow is DEFERRED by the caller-side list (the P3D-105
//!   teleport pattern: rejected jobs are held, counted, re-admitted over
//!   following frames — never dropped, never unbounded).
//! - **gpu_upload_budget**: per-frame upload cap plus a total GPU-byte
//!   budget; over budget, the FARTHEST patches evict first.
//! - **frustum_culling**: Gribb–Hartmann planes from the same view-proj
//!   matrix the shader uses; patches behind the camera do not draw
//!   (loading is unaffected).
//! - **no_visible_or_collision_seams**: LOD culls only bottom (Mid) and
//!   side (Far) faces — top faces render at every level, so the visible
//!   shell cannot crack between rings; collision exists only in the Full
//!   ring (P3D-105 tier contract), where the mesh is the full-face,
//!   collision-aligned R3DV-005 mesh.

use crate::camera::CameraPose;
use crate::scene::{SceneVertex, FACE_BASIS};
use crate::terrain::{mesh_patch_lod, MeshLod};
use pc3d_world::coords::{PatchCoord, WorldPos};
use pc3d_world::gen::WorldGen;
use pc3d_world::lod::{lod_for, LodLevel};
use pc3d_world::stream::{interest_diff, interest_patches, Admit, BoundedQueue, Tier};
use std::collections::BTreeMap;
use std::rc::Rc;

const VERTEX_BYTES: usize = std::mem::size_of::<SceneVertex>();
const INDEX_BYTES: usize = std::mem::size_of::<u16>();

#[derive(Clone, Copy, Debug)]
pub struct StreamConfig {
    /// CPU mesh jobs per frame (the bounded queue drains at most this).
    pub max_mesh_per_frame: usize,
    /// GPU buffer uploads per frame.
    pub max_uploads_per_frame: usize,
    /// Total GPU bytes budget for terrain meshes.
    pub gpu_byte_budget: usize,
    /// Interest tiers to load (Full/Lod/Macro rings).
    pub tiers: &'static [Tier],
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            max_mesh_per_frame: 2,
            max_uploads_per_frame: 4,
            gpu_byte_budget: 24 * 1024 * 1024,
            tiers: &[Tier::Full, Tier::Lod, Tier::Macro],
        }
    }
}

/// Honest counters — the streaming evidence surface.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StreamCounters {
    pub queue_len: usize,
    pub queue_pushed: u64,
    pub queue_admitted: u64,
    pub queue_rejected: u64,
    pub deferred: usize,
    pub meshed: u64,
    pub uploaded: u64,
    pub evicted: u64,
    pub upload_bytes: u64,
    pub gpu_bytes: usize,
    pub loaded: usize,
    pub loaded_full: usize,
    pub loaded_mid: usize,
    pub loaded_far: usize,
    pub frustum_culled: u64,
    pub drawn_patches: u64,
    /// Maxima over the run — the per-frame bound evidence.
    pub max_mesh_per_frame_seen: usize,
    pub max_uploads_per_frame_seen: usize,
    /// Cumulative CPU mesh time.
    pub mesh_us_total: u128,
}

pub struct StreamFrameStats {
    pub meshed: usize,
    pub uploaded: usize,
    pub evicted: usize,
    pub deferred: usize,
    pub mesh_us: u128,
}

struct PatchSlot {
    lod: LodLevel,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    bytes: usize,
    /// Patch center for distance ordering and culling.
    center: [f32; 3],
    /// World AABB (patch slab) for frustum culling.
    aabb_min: [f32; 3],
    aabb_max: [f32; 3],
}

pub struct TerrainStreamer {
    gen: Rc<WorldGen>,
    cfg: StreamConfig,
    queue: BoundedQueue<PatchCoord>,
    /// Jobs the bounded queue rejected — held here, re-admitted next frame
    /// (never dropped; the P3D-105 teleport law).
    deferred: Vec<PatchCoord>,
    /// Coords with a live queued job (no duplicate pushes — a re-push storm
    /// would starve real work).
    pending: std::collections::BTreeSet<PatchCoord>,
    loaded: BTreeMap<PatchCoord, PatchSlot>,
    counters: StreamCounters,
    /// Vertical y-patch level the viewer walks on (terrain columns).
    y_level: i32,
}

impl TerrainStreamer {
    pub fn new(gen: Rc<WorldGen>, cfg: StreamConfig, y_level: i32) -> Self {
        let queue = BoundedQueue::new(64);
        TerrainStreamer {
            gen,
            cfg,
            queue,
            deferred: Vec::new(),
            pending: std::collections::BTreeSet::new(),
            loaded: BTreeMap::new(),
            counters: StreamCounters::default(),
            y_level,
        }
    }

    pub fn counters(&self) -> StreamCounters {
        let mut c = self.counters;
        c.queue_len = self.queue.len();
        c.deferred = self.deferred.len();
        c.loaded = self.loaded.len();
        c.loaded_full = self
            .loaded
            .values()
            .filter(|s| s.lod == LodLevel::Full)
            .count();
        c.loaded_mid = self
            .loaded
            .values()
            .filter(|s| s.lod == LodLevel::Mid)
            .count();
        c.loaded_far = self
            .loaded
            .values()
            .filter(|s| s.lod == LodLevel::Far)
            .count();
        c.gpu_bytes = self.loaded.values().map(|s| s.bytes).sum();
        c
    }

    /// The desired patch set with LOD levels, from the world's own interest
    /// rings (discs by tier, per-patch detail by `lod_for` distance).
    fn desired(&self, viewer: WorldPos) -> BTreeMap<PatchCoord, LodLevel> {
        let mut want = BTreeMap::new();
        for tier in self.cfg.tiers {
            for col in interest_patches(viewer, *tier).expect("interest ring") {
                let coord = PatchCoord { x: col.x, y: self.y_level, z: col.z };
                let center = WorldPos::from_mm(
                    coord.x as i64 * 16_000 + 8_000,
                    coord.y as i64 * 16_000 + 8_000,
                    coord.z as i64 * 16_000 + 8_000,
                );
                let lod = lod_for(viewer, center);
                if lod == LodLevel::Horizon {
                    continue;
                }
                want.insert(coord, lod);
            }
        }
        want
    }

    /// One frame of streaming: interest diff, bounded queue admission,
    /// bounded meshing, bounded uploads, budget-driven eviction.
    pub fn update(&mut self, device: &wgpu::Device, viewer: WorldPos) -> StreamFrameStats {
        use wgpu::util::DeviceExt;
        let t0 = std::time::Instant::now();
        let want = self.desired(viewer);

        // Unload: loaded patches no longer desired.
        let stale: Vec<PatchCoord> = self
            .loaded
            .keys()
            .filter(|k| !want.contains_key(k))
            .copied()
            .collect();
        for k in stale {
            self.loaded.remove(&k);
            self.counters.evicted += 1;
        }
        // Also drop queued jobs that are no longer desired (the pending set
        // mirrors the queue; rebuilding both from the survivors keeps them
        // in sync without a queue-erase API).
        let mut keep = Vec::new();
        while let Some(job) = self.queue.pop() {
            if want.contains_key(&job) {
                keep.push(job);
            }
        }
        for job in &keep {
            self.queue.push(*job);
        }
        self.pending.retain(|j| want.contains_key(j));
        self.deferred.retain(|j| want.contains_key(j));

        // Load / re-LOD: desired patches missing or at the wrong level.
        // ONE live job per coord (the pending set blocks duplicate pushes),
        // NEAREST FIRST — the viewer's own ring must load before the horizon.
        let mut jobs: Vec<(PatchCoord, LodLevel)> = want
            .iter()
            .filter(|(coord, lod)| {
                let current = self.loaded.get(*coord).map(|s| s.lod);
                current != Some(**lod) && !self.pending.contains(*coord)
            })
            .map(|(c, l)| (*c, *l))
            .collect();
        let vx = viewer.x as f32 / 1000.0;
        let vz = viewer.z as f32 / 1000.0;
        jobs.sort_by_key(|(c, _)| {
            let dx = c.x as f32 * 16.0 + 8.0 - vx;
            let dz = c.z as f32 * 16.0 + 8.0 - vz;
            ((dx * dx + dz * dz) * 1000.0) as i64
        });
        for (coord, _lod) in jobs {
            self.pending.insert(coord);
            if let Admit::RejectedFull(c) = self.queue.push(coord) {
                // Queue full: HOLD the job in the deferred list (pending
                // stays live) — never dropped, never stuck.
                self.deferred.push(c);
            }
        }

        // Re-admit deferred jobs ONLY into remaining capacity (bounded
        // re-admission; pending entries stay live either way).
        let mut still_deferred = Vec::new();
        for job in std::mem::take(&mut self.deferred) {
            if !want.contains_key(&job) {
                self.pending.remove(&job);
                continue; // stale deferral
            }
            if self.queue.len() >= self.queue.capacity() {
                still_deferred.push(job);
                continue;
            }
            match self.queue.push(job) {
                Admit::Admitted => {}
                Admit::RejectedFull(job) => still_deferred.push(job),
            }
        }
        self.deferred = still_deferred;

        // Bounded mesh + upload work this frame.
        let mut meshed = 0;
        let mut uploaded = 0;
        let mut evicted = 0;
        while meshed < self.cfg.max_mesh_per_frame && uploaded < self.cfg.max_uploads_per_frame {
            let Some(coord) = self.queue.pop() else { break };
            self.pending.remove(&coord);
            let Some(&lod) = want.get(&coord) else { continue };
            // Already at the right level (a stale/duplicate survivor):
            // free, skip — it must not consume mesh budget.
            if self.loaded.get(&coord).map(|s| s.lod) == Some(lod) {
                continue;
            }
            let mesh_lod = match lod {
                LodLevel::Full => MeshLod::Full,
                LodLevel::Mid => MeshLod::Mid,
                _ => MeshLod::Far,
            };
            let t_mesh = std::time::Instant::now();
            let (verts, idx) = mesh_patch_lod(&self.gen, coord, mesh_lod);
            let mesh_us = t_mesh.elapsed().as_micros();
            self.counters.mesh_us_total += mesh_us;
            self.counters.meshed += 1;
            meshed += 1;

            let bytes = verts.len() * VERTEX_BYTES + idx.len() * INDEX_BYTES;
            if bytes > self.cfg.gpu_byte_budget {
                // A single patch larger than the whole budget can never
                // load — skip it honestly (counted) instead of exceeding.
                self.counters.evicted += 1;
                continue;
            }
            // GPU budget: evict farthest FIRST until the new patch fits.
            while self.loaded.values().map(|s| s.bytes).sum::<usize>() + bytes
                > self.cfg.gpu_byte_budget
            {
                let farthest = self
                    .loaded
                    .iter()
                    .max_by_key(|(_, s)| {
                        let dx = s.center[0] - viewer.x as f32 / 1000.0;
                        let dz = s.center[2] - viewer.z as f32 / 1000.0;
                        ((dx * dx + dz * dz) * 1000.0) as i64
                    })
                    .map(|(k, _)| *k);
                match farthest {
                    Some(k) => {
                        self.loaded.remove(&k);
                        self.counters.evicted += 1;
                        evicted += 1;
                    }
                    None => break,
                }
            }

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("stream vertices"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("stream indices"),
                contents: bytemuck::cast_slice(&idx),
                usage: wgpu::BufferUsages::INDEX,
            });
            let o = coord.origin();
            let to_m = |mm: i64| mm as f32 / 1000.0;
            self.loaded.insert(
                coord,
                PatchSlot {
                    lod,
                    vertex_buffer,
                    index_buffer,
                    index_count: idx.len() as u32,
                    bytes,
                    center: [
                        to_m(o.x) + 8.0,
                        to_m(o.y) + 8.0,
                        to_m(o.z) + 8.0,
                    ],
                    aabb_min: [to_m(o.x), to_m(o.y), to_m(o.z)],
                    aabb_max: [to_m(o.x) + 16.0, to_m(o.y) + 16.0, to_m(o.z) + 16.0],
                },
            );
            self.counters.uploaded += 1;
            self.counters.upload_bytes += bytes as u64;
            uploaded += 1;
        }
        // Push deferrals for anything the queue rejected this frame.
        self.counters.queue_pushed = self.queue.pushed;
        self.counters.queue_admitted = self.queue.admitted;
        self.counters.queue_rejected = self.queue.rejected;
        self.counters.max_mesh_per_frame_seen =
            self.counters.max_mesh_per_frame_seen.max(meshed);
        self.counters.max_uploads_per_frame_seen = self
            .counters
            .max_uploads_per_frame_seen
            .max(uploaded);
        StreamFrameStats {
            meshed,
            uploaded,
            evicted,
            deferred: self.deferred.len(),
            mesh_us: t0.elapsed().as_micros(),
        }
    }

    /// Frustum planes (Gribb–Hartmann) from the view-proj matrix the shader
    /// uses — plane = [a,b,c,d] with a*x+b*y+c*z+d >= 0 inside.
    pub fn frustum_planes(view_proj: &[f32; 16]) -> [[f32; 4]; 6] {
        let m = |r: usize, c: usize| view_proj[c * 4 + r];
        let mut planes = [[0.0f32; 4]; 6];
        // left, right, bottom, top, near, far — row combos of M.
        let rows = [
            (m(3, 0) + m(0, 0), m(3, 1) + m(0, 1), m(3, 2) + m(0, 2), m(3, 3) + m(0, 3)),
            (m(3, 0) - m(0, 0), m(3, 1) - m(0, 1), m(3, 2) - m(0, 2), m(3, 3) - m(0, 3)),
            (m(3, 0) + m(1, 0), m(3, 1) + m(1, 1), m(3, 2) + m(1, 2), m(3, 3) + m(1, 3)),
            (m(3, 0) - m(1, 0), m(3, 1) - m(1, 1), m(3, 2) - m(1, 2), m(3, 3) - m(1, 3)),
            (m(3, 0) + m(2, 0), m(3, 1) + m(2, 1), m(3, 2) + m(2, 2), m(3, 3) + m(2, 3)),
            (m(3, 0) - m(2, 0), m(3, 1) - m(2, 1), m(3, 2) - m(2, 2), m(3, 3) - m(2, 3)),
        ];
        for (i, (a, b, c, d)) in rows.into_iter().enumerate() {
            let l = (a * a + b * b + c * c).sqrt().max(1e-9);
            planes[i] = [a / l, b / l, c / l, d / l];
        }
        planes
    }

    /// AABB-vs-frustum: inside (or intersecting) when some corner lies on
    /// the positive side of every plane.
    pub fn aabb_in_frustum(planes: &[[f32; 4]; 6], min: [f32; 3], max: [f32; 3]) -> bool {
        for p in planes {
            // Pick the AABB corner most aligned with the plane normal.
            let x = if p[0] >= 0.0 { max[0] } else { min[0] };
            let y = if p[1] >= 0.0 { max[1] } else { min[1] };
            let z = if p[2] >= 0.0 { max[2] } else { min[2] };
            if p[0] * x + p[1] * y + p[2] * z + p[3] < 0.0 {
                return false;
            }
        }
        true
    }

    /// Draws all loaded, frustum-visible patches through the caller's bound
    /// lit-mesh pipeline. Returns the draw list stats (culled vs drawn).
    pub fn draw<'rp>(
        &mut self,
        pass: &mut wgpu::RenderPass<'rp>,
        view_proj: &[f32; 16],
    ) -> (usize, usize) {
        let planes = Self::frustum_planes(view_proj);
        let mut drawn = 0usize;
        let mut culled = 0usize;
        for slot in self.loaded.values() {
            if slot.index_count == 0 {
                continue;
            }
            if !Self::aabb_in_frustum(&planes, slot.aabb_min, slot.aabb_max) {
                culled += 1;
                continue;
            }
            pass.set_vertex_buffer(0, slot.vertex_buffer.slice(..));
            pass.set_index_buffer(slot.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..slot.index_count, 0, 0..1);
            drawn += 1;
        }
        self.counters.frustum_culled += culled as u64;
        self.counters.drawn_patches += drawn as u64;
        (drawn, culled)
    }

    /// Test hook: the current desired set (viewer hardcoded to the walk
    /// diagnostic pose — used only by tests).
    pub fn debug_desired(&self) -> BTreeMap<PatchCoord, LodLevel> {
        let viewer = viewer_of(crate::camera::CameraPose::new([0.0, 30.0, 0.0], 0.0, 0.0));
        self.desired(viewer)
    }
}

/// Viewer position (world mm) from a camera pose.
pub fn viewer_of(pose: CameraPose) -> WorldPos {
    WorldPos::from_mm(
        (pose.position[0] * 1000.0) as i64,
        (pose.position[1] * 1000.0) as i64,
        (pose.position[2] * 1000.0) as i64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen() -> Rc<WorldGen> {
        Rc::new(WorldGen::new(3))
    }

    fn cfg(mesh: usize, uploads: usize, budget: usize) -> StreamConfig {
        StreamConfig {
            max_mesh_per_frame: mesh,
            max_uploads_per_frame: uploads,
            gpu_byte_budget: budget,
            ..Default::default()
        }
    }

    #[test]
    fn frustum_culls_patches_behind_the_camera() {
        // Camera at origin looking north (-Z): the patch AHEAD must draw,
        // the patch BEHIND must cull.
        let pose = CameraPose::new([0.0, 30.0, 0.0], 0.0, 0.0);
        let cam = crate::camera::Camera::new(pose);
        let vp = cam.view_proj(1.6);
        let planes = TerrainStreamer::frustum_planes(&vp);
        assert!(TerrainStreamer::aabb_in_frustum(
            &planes,
            [0.0, 29.0, -40.0],
            [16.0, 45.0, -24.0]
        ), "patch ahead of the camera must be visible");
        assert!(
            !TerrainStreamer::aabb_in_frustum(
                &planes,
                [0.0, 29.0, 24.0],
                [16.0, 45.0, 40.0]
            ),
            "patch behind the camera must cull"
        );
        // A patch at the side edge (outside the 70 deg fov) culls too.
        assert!(
            !TerrainStreamer::aabb_in_frustum(
                &planes,
                [-90.0, 20.0, -40.0],
                [-74.0, 36.0, -24.0]
            ),
            "patch far off to the side must cull"
        );
    }

    #[test]
    fn desired_set_is_ring_banded_and_horizon_free() {
        let s = TerrainStreamer::new(gen(), cfg(1, 1, usize::MAX), 1);
        let viewer = WorldPos::from_meters(0, 30, 0);
        let want = s.desired(viewer);
        assert!(want.len() > 50, "macro ring loads a real vista");
        assert!(want.values().all(|l| *l != LodLevel::Horizon));
        // The viewer's own patch is Full.
        let own = PatchCoord { x: 0, y: 1, z: 0 };
        assert_eq!(want.get(&own), Some(&LodLevel::Full));
        // Far-away desired patches are not Full.
        let far = want
            .iter()
            .find(|(_, l)| **l == LodLevel::Far)
            .map(|(c, _)| *c)
            .expect("a far patch in the macro ring");
        let center = WorldPos::from_mm(
            far.x as i64 * 16_000 + 8_000,
            0,
            far.z as i64 * 16_000 + 8_000,
        );
        assert_eq!(lod_for(viewer, center), LodLevel::Far);
    }

    #[test]
    fn lod_meshing_shrinks_remote_patches() {
        // A surface patch without caves has NO bottom faces, so Full==Mid
        // there is correct; the strict shrink needs a cave ceiling.
        let (seed, coord) = pc3d_world::terrain::SceneSpec::SmoothHills.patch();
        let g = WorldGen::new(seed);
        let full = mesh_patch_lod(&g, coord, MeshLod::Full);
        let mid = mesh_patch_lod(&g, coord, MeshLod::Mid);
        let far = mesh_patch_lod(&g, coord, MeshLod::Far);
        assert!(full.1.len() >= mid.1.len());
        assert!(mid.1.len() > far.1.len(), "far drops side faces (slopes)");
        assert!(!far.1.is_empty());

        // The cave patch: Full renders ceiling undersides, Mid drops them.
        let (c_seed, near) = pc3d_world::terrain::SceneSpec::Highlands.patch();
        let cg = WorldGen::new(c_seed);
        let (air, _, _) = crate::terrain::find_cave_pocket_near(&cg, near, 1)
            .expect("cave pocket for the LOD test");
        let cave_patch = PatchCoord {
            x: air.x.div_euclid(16),
            y: air.y.div_euclid(16),
            z: air.z.div_euclid(16),
        };
        let c_full = mesh_patch_lod(&cg, cave_patch, MeshLod::Full);
        let c_mid = mesh_patch_lod(&cg, cave_patch, MeshLod::Mid);
        assert!(
            c_full.1.len() > c_mid.1.len(),
            "mid must drop the cave ceiling undersides"
        );
    }

    /// GPU-level: the teleport law at the streamer level — the queue is
    /// bounded, deferrals are held (never dropped), per-frame mesh/upload
    /// caps hold EVERY frame, and the vista eventually completes. (Scoped
    /// to the Full tier: a full 1024 m macro ring is ~12.8k patches and
    /// CANNOT complete at an honest 60 fps mesh budget — that is exactly
    /// what the bounded queue is for.)
    #[test]
    fn teleport_keeps_work_bounded_and_eventually_completes() {
        let mut r = crate::renderer::Renderer::offscreen(320, 200);
        r.set_placeholder_scene(false);
        r.attach_streaming(
            gen(),
            StreamConfig {
                max_mesh_per_frame: 2,
                max_uploads_per_frame: 2,
                gpu_byte_budget: usize::MAX,
                tiers: &[Tier::Full],
            },
            1,
        );
        // Teleport far away: the whole vista is new work.
        r.set_pose(crate::camera::CameraPose::new([80_000.0, 30.0, 0.0], 0.0, 0.0));
        let mut deferred_seen = 0usize;
        for frame in 0..400 {
            let stats = r.stream_frame().expect("streaming attached");
            assert!(
                stats.meshed <= 2,
                "frame {frame}: mesh cap violated ({})",
                stats.meshed
            );
            assert!(
                stats.uploaded <= 2,
                "frame {frame}: upload cap violated ({})",
                stats.uploaded
            );
            deferred_seen = deferred_seen.max(stats.deferred);
            if stats.meshed == 0 && stats.deferred == 0 && stats.evicted == 0 {
                // Vista complete — and the counters prove the bounds.
                let c = r.stream_counters().unwrap();
                assert!(c.loaded > 50, "a full ring is ~100 patches");
                assert!(c.max_mesh_per_frame_seen <= 2);
                assert!(c.max_uploads_per_frame_seen <= 2);
                assert!(deferred_seen > 0, "the burst must have deferred some frames");
                println!(
                    "teleport vista: {} frames, {} patches, {} meshed total, {} us mesh, deferral peak {}",
                    frame + 1,
                    c.loaded,
                    c.meshed,
                    c.mesh_us_total,
                    deferred_seen
                );
                return;
            }
        }
        panic!("full-ring teleport vista never completed within 400 frames");
    }

    /// GPU-level: the memory budget invariant — GPU bytes NEVER exceed the
    /// budget, pressure evicts, and the full ring around the viewer is what
    /// survives (an artificially tiny budget intentionally churns: the
    /// evicted far patches stay DESIRED, so the queue re-requests them —
    /// honest behavior under starvation, documented here).
    #[test]
    fn gpu_budget_never_exceeds_and_full_ring_survives() {
        let mut r = crate::renderer::Renderer::offscreen(320, 200);
        r.set_placeholder_scene(false);
        r.attach_streaming(
            gen(),
            StreamConfig {
                max_mesh_per_frame: 2,
                max_uploads_per_frame: 2,
                gpu_byte_budget: 96 * 1024,
                tiers: &[Tier::Full],
            },
            1,
        );
        r.set_pose(crate::camera::CameraPose::new([0.0, 30.0, 0.0], 0.0, 0.0));
        for _ in 0..200 {
            let _ = r.stream_frame();
            let c = r.stream_counters().unwrap();
            assert!(
                c.gpu_bytes <= 96 * 1024,
                "gpu budget exceeded: {} bytes",
                c.gpu_bytes
            );
        }
        let c = r.stream_counters().unwrap();
        assert!(c.evicted > 0, "a tiny budget must have evicted");
        assert!(c.loaded_full > 0, "the viewer's own ring must survive");
        println!(
            "budget: {} loaded, {} evicted, {} bytes (cap 98304)",
            c.loaded, c.evicted, c.gpu_bytes
        );
    }

    /// Diagnostic/proof: the walk configuration (Full+Lod tiers, 24 MB
    /// budget) fills the viewer's OWN full ring quickly under budget
    /// pressure — the near ring must not starve behind far-ring churn.
    #[test]
    fn walk_config_fills_the_full_ring_first() {
        let mut r = crate::renderer::Renderer::offscreen(320, 200);
        r.set_placeholder_scene(false);
        r.attach_streaming(
            gen(),
            StreamConfig {
                max_mesh_per_frame: 3,
                max_uploads_per_frame: 6,
                gpu_byte_budget: 24 * 1024 * 1024,
                tiers: &[Tier::Full, Tier::Lod],
            },
            1,
        );
        r.set_pose(crate::camera::CameraPose::new([0.0, 30.0, 0.0], 0.0, -0.08));
        {
            let want = r.stream_desired_debug().unwrap();
            eprintln!(
                "desired (0,1,0) -> {:?}; sample near entries: {:?}",
                want.get(&PatchCoord { x: 0, y: 1, z: 0 }),
                want.iter().take(3).collect::<Vec<_>>()
            );
        }
        let mut full_at = None;
        for frame in 0..200 {
            let _ = r.stream_frame();
            let c = r.stream_counters().unwrap();
            if c.loaded_full >= 100 {
                full_at = Some((frame, c.loaded, c.loaded_full, c.loaded_mid, c.evicted, c.gpu_bytes));
                break;
            }
            if frame % 40 == 0 {
                eprintln!(
                    "f{frame}: loaded {} (full {} mid {}), evicted {}, gpu {} KB, deferred {}",
                    c.loaded, c.loaded_full, c.loaded_mid, c.evicted, c.gpu_bytes / 1024, c.deferred
                );
            }
        }
        let (f, loaded, full, mid, evicted, bytes) =
            full_at.expect("full ring (>=100 patches) never loaded in 200 frames");
        println!(
            "full ring complete at frame {f}: loaded {loaded} (full {full} mid {mid}), evicted {evicted}, gpu {bytes} bytes"
        );
    }

    #[test]
    fn top_shell_survives_every_lod_no_ring_crack() {
        // For every column, the topmost solid cell's TOP face exists in the
        // Full AND Far meshes — LOD cannot open a visible crack in the
        // shell. (Seam heights/materials across borders are already proven
        // by P3D-206's border_agrees.)
        let (seed, coord) = pc3d_world::terrain::SceneSpec::SmoothHills.patch();
        let g = WorldGen::new(seed);
        let full = mesh_patch_lod(&g, coord, MeshLod::Full);
        let far = mesh_patch_lod(&g, coord, MeshLod::Far);
        let top_faces = |m: &(Vec<SceneVertex>, Vec<u16>)| -> std::collections::HashSet<(i32, i32, i32)> {
            let mut set = std::collections::HashSet::new();
            for &vi in &m.1 {
                let v = &m.0[vi as usize];
                if v.normal == [0.0, 1.0, 0.0] {
                    set.insert((
                        v.pos[0].floor() as i32,
                        (v.pos[1] - 1.0) as i32,
                        v.pos[2].floor() as i32,
                    ));
                }
            }
            set
        };
        let tf = top_faces(&full);
        let t_far = top_faces(&far);
        assert_eq!(tf, t_far, "the visible top shell must be identical at every LOD");
        assert!(!tf.is_empty());
    }
}
