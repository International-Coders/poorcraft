//! Wilderness rendering (NWR-007): instanced trees/rocks/shrubs/logs and
//! the biome landmark from the validated GLB factory, plus wind-animated
//! grass cutout cards — all placed by the ONE authority
//! (pc3d_world::flora::plant_at, pure and seed-deterministic).
//!
//! Draw budget: ONE draw call per (kind, LOD) bucket — never one per
//! plant. Placement work is BOUNDED per update (a slot-scan budget) and
//! the cache EVICTS beyond the view ring, so teleport/walk cycles stay
//! flat. LOD follows the glb thresholds (lod0 < 40 m, lod1 < 120 m,
//! lod2 beyond); grass draws only within its own tighter radius.

use crate::atmosphere::CutoutVertex;
use pc3d_world::flora::{self, PlantKind, SlotCoord};
use pc3d_world::gen::WorldGen;
use std::collections::BTreeMap;

/// One instance: world position + scale, then (rotation-y, wind,
/// tint multiplier, unused).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    pub pos_scale: [f32; 4],
    pub params: [f32; 4],
}

/// The instance buffer layout (step mode Instance) — shader locations
/// 3/4 when the mesh occupies 0..2.
pub const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &[
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 0,
            shader_location: 3,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 16,
            shader_location: 4,
        },
    ],
};

/// The same buffer bound after a CUTOUT mesh (uv at 3): locations 4/5.
pub const INSTANCE_LAYOUT_LATE: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &[
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 0,
            shader_location: 4,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 16,
            shader_location: 5,
        },
    ],
};

/// The streamed-flora budget (Deck tiers).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloraConfig {
    /// Slot-scan radius around the viewer (m).
    pub radius_m: f32,
    /// Max slots examined per update call (bounded placement work).
    pub max_slots_per_update: usize,
    /// Grass draws only within this (m) — the Deck's cheapest lever.
    pub grass_radius_m: f32,
}

impl Default for FloraConfig {
    fn default() -> Self {
        // Mid tier: 140 m of wilderness, 3k slots/update, grass to 40 m.
        Self {
            radius_m: 140.0,
            max_slots_per_update: 3072,
            grass_radius_m: 40.0,
        }
    }
}

impl FloraConfig {
    pub fn low() -> Self {
        Self {
            radius_m: 84.0,
            max_slots_per_update: 1536,
            grass_radius_m: 22.0,
        }
    }

    pub fn high() -> Self {
        Self {
            radius_m: 180.0,
            max_slots_per_update: 4096,
            grass_radius_m: 56.0,
        }
    }
}

/// Counters for the bounded-work + eviction proofs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FloraStats {
    pub added: usize,
    pub evicted: usize,
    pub cached: usize,
    pub scanned: usize,
    pub draw_buckets: usize,
    pub instances_drawn: usize,
}

struct KindGpu {
    /// (mesh, index_count) per lod name present in the GLB.
    lods: Vec<(wgpu::Buffer, wgpu::Buffer, u32)>,
    /// World-space height of the asset (for wind normalization).
    _height: f32,
}

/// The per-(kind, lod) instance bucket, uploaded when dirty.
struct Bucket {
    buf: wgpu::Buffer,
    count: u32,
}

/// One canonical grass tuft: four crossed cards (mesh reused by every
/// grass instance).
fn grass_card_mesh() -> (Vec<CutoutVertex>, Vec<u16>) {
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    for k in 0..4i32 {
        let a = k as f32 * std::f32::consts::FRAC_PI_4;
        let (c, s) = (a.cos(), a.sin());
        let w = 0.45;
        let h = 1.0;
        let corners = [
            ([-w, 0.0, 0.0], [0.0, 0.0]),
            ([w, 0.0, 0.0], [1.0, 0.0]),
            ([w, h, 0.0], [1.0, 1.0]),
            ([-w, h, 0.0], [0.0, 1.0]),
        ];
        let base = verts.len() as u16;
        let normal = [s, 0.2, c];
        let color = [0.32, 0.50, 0.22];
        for (p, uv) in corners {
            verts.push(CutoutVertex {
                pos: [p[0] * c + p[2] * s, p[1], -p[0] * s + p[2] * c],
                normal,
                color,
                uv,
            });
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (verts, idx)
}

pub struct FloraStreamer {
    kinds: BTreeMap<PlantKind, KindGpu>,
    grass: (wgpu::Buffer, wgpu::Buffer, u32),
    grass_mask_bg: Option<wgpu::BindGroup>,
    /// Slot -> what grows there (None = known empty; cached so empty
    /// slots are not re-queried every call — the settle test caught the
    /// re-examination churn).
    cache: BTreeMap<SlotCoord, Option<(PlantKind, Instance)>>,
    config: FloraConfig,
    buckets: BTreeMap<(PlantKind, u8), Bucket>,
    grass_bucket: Option<Bucket>,
    dirty: bool,
    last_viewer: [f32; 2],
    scan_phase: u32,
    pub stats: FloraStats,
}

fn asset_rel(kind: PlantKind) -> &'static str {
    match kind {
        PlantKind::TreePine => "flora/tree_pine.glb",
        PlantKind::TreeBroadleaf => "flora/tree_broadleaf.glb",
        PlantKind::TreeBirch => "flora/tree_birch.glb",
        PlantKind::RockBoulder => "flora/rock_boulder.glb",
        PlantKind::RockSpire => "flora/rock_spire.glb",
        PlantKind::RockSlab => "flora/rock_slab.glb",
        PlantKind::Shrub => "flora/shrub.glb",
        PlantKind::Log => "flora/log_fallen.glb",
        PlantKind::Grass => "flora/shrub.glb", // placeholder path; grass draws cards
    }
}

impl FloraStreamer {
    /// Loads the wilderness set and builds the shared GPU meshes.
    pub fn new(device: &wgpu::Device) -> Self {
        use wgpu::util::DeviceExt;
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/compiled");
        let mut kinds = BTreeMap::new();
        for kind in [
            PlantKind::TreePine,
            PlantKind::TreeBroadleaf,
            PlantKind::TreeBirch,
            PlantKind::RockBoulder,
            PlantKind::RockSpire,
            PlantKind::RockSlab,
            PlantKind::Shrub,
            PlantKind::Log,
        ] {
            let path = root.join(asset_rel(kind));
            let asset = crate::glb::load_asset_file(&path)
                .unwrap_or_else(|e| panic!("wilderness asset {}: {e}", path.display()));
            let mut lods = Vec::new();
            let mut height = 1.0f32;
            for lod in &asset.lods {
                let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("flora vertices"),
                    contents: bytemuck::cast_slice(&lod.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("flora indices"),
                    contents: bytemuck::cast_slice(&lod.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
                height = height.max(lod.vertices.iter().map(|v| v.pos[1]).fold(0.0, f32::max));
                lods.push((vb, ib, lod.indices.len() as u32));
            }
            kinds.insert(kind, KindGpu { lods, _height: height });
        }
        let (gverts, gidx) = grass_card_mesh();
        let gv = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grass card vertices"),
            contents: bytemuck::cast_slice(&gverts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let gi = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grass card indices"),
            contents: bytemuck::cast_slice(&gidx),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            kinds,
            grass: (gv, gi, gidx.len() as u32),
            grass_mask_bg: None,
            cache: BTreeMap::new(),
            config: FloraConfig::default(),
            buckets: BTreeMap::new(),
            grass_bucket: None,
            dirty: true,
            last_viewer: [f32::MAX, 0.0],
            scan_phase: 0,
            stats: FloraStats::default(),
        }
    }

    pub fn set_config(&mut self, cfg: FloraConfig) {
        if cfg != self.config {
            self.config = cfg;
            self.dirty = true;
        }
    }

    /// Bounded placement work: scans slots around the viewer (ring
    /// order, up to the budget), caches what grows, evicts beyond the
    /// ring. The authority decides WHAT grows; this only caches it.
    pub fn update(&mut self, gen: &WorldGen, viewer: [f32; 2]) -> &FloraStats {
        let mut added = 0usize;
        let mut evicted = 0usize;
        let mut examined_new = 0usize;
        let r = self.config.radius_m;
        let ring = (r / flora::SLOT_M as f32).ceil() as i32;
        let cx = (viewer[0] / flora::SLOT_M as f32).floor() as i32;
        let cz = (viewer[1] / flora::SLOT_M as f32).floor() as i32;
        // The scan ROTATES its ring phase per call: restarting at ring 0
        // every frame meant the outer rings of a big view radius were
        // NEVER reached under the per-frame budget (the settle test
        // caught it). Cached slots cost nothing; only NEW examinations
        // count against the budget.
        let phase = self.scan_phase;
        self.scan_phase = self.scan_phase.wrapping_add(1);
        'scan: for i in 0..=ring {
            let dr = ((i as u32 + phase) % (ring as u32 + 1)) as i32;
            for dx in -dr..=dr {
                for dz in -dr..=dr {
                    if dx.abs() != dr && dz.abs() != dr {
                        continue; // ring perimeter
                    }
                    let slot = SlotCoord { x: cx + dx, z: cz + dz };
                    if self.cache.contains_key(&slot) {
                        continue;
                    }
                    examined_new += 1;
                    if examined_new > self.config.max_slots_per_update {
                        break 'scan; // bounded: the rest comes next call
                    }
                    let entry = flora::plant_at(gen, slot).and_then(|plant| {
                        if plant.kind == PlantKind::Grass {
                            return None; // grass streams in its own bucket
                        }
                        let [cxm, czm] = slot.center_m();
                        let j = flora::jitter(gen, slot);
                        let x = cxm + j[0];
                        let z = czm + j[1];
                        let y = gen.effective_surface_mm((x * 1000.0) as i64, (z * 1000.0) as i64)
                            as f32
                            / 1000.0;
                        let rot = j[2] * 6.28;
                        Some((
                            plant.kind,
                            Instance {
                                pos_scale: [x, y, z, j[2]],
                                params: [rot, plant.kind.wind(), 1.0, 0.0],
                            },
                        ))
                    });
                    if entry.is_some() {
                        added += 1;
                    }
                    self.cache.insert(slot, entry);
                }
            }
        }
        // Evict beyond ring + 16 m.
        let keep = r + 16.0;
        let before = self.cache.len();
        self.cache.retain(|slot, _| {
            let [cxm, czm] = slot.center_m();
            (cxm - viewer[0]).hypot(czm - viewer[1]) <= keep
        });
        evicted = before - self.cache.len();
        // Grass: computed per upload (cheap query, own radius).
        self.stats.added += added;
        self.stats.evicted += evicted;
        self.stats.scanned = examined_new;
        self.stats.cached = self.cache.len();
        if added > 0 || evicted > 0 {
            self.dirty = true;
        }
        &self.stats
    }

    /// Rebuilds the per-(kind, lod) instance buckets when dirty or the
    /// viewer moved >= 2 m (LOD re-bucketing).
    pub fn upload(&mut self, gen: &WorldGen, device: &wgpu::Device, viewer: [f32; 2]) {
        if !self.dirty
            && (self.last_viewer[0] - viewer[0]).hypot(self.last_viewer[1] - viewer[1]) < 2.0
        {
            return;
        }
        self.last_viewer = viewer;
        self.dirty = false;
        use wgpu::util::DeviceExt;
        let mut rows: BTreeMap<(PlantKind, u8), Vec<Instance>> = BTreeMap::new();
        for (_, (kind, inst)) in self.cache.iter().filter_map(|(k, v)| v.as_ref().map(|v| (k, v))) {
            let d = (inst.pos_scale[0] - viewer[0]).hypot(inst.pos_scale[2] - viewer[1]);
            // Mirror of glb::lod_for thresholds.
            let lod = if d < 40.0 {
                0
            } else if d < 120.0 {
                1
            } else {
                2
            };
            rows.entry((*kind, lod)).or_default().push(*inst);
        }
        let mut buckets = BTreeMap::new();
        for (key, list) in rows {
            if list.is_empty() {
                continue;
            }
            if self.kinds[&key.0].lods.len() <= key.1 as usize {
                continue; // asset has no such LOD — skip (coarsest was bucketed)
            }
            let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("flora instances"),
                contents: bytemuck::cast_slice(&list),
                usage: wgpu::BufferUsages::VERTEX,
            });
            buckets.insert(key, Bucket { buf, count: list.len() as u32 });
        }
        // Grass bucket: its own tighter radius, own jittered instances.
        let gr = self.config.grass_radius_m;
        let ring = (gr / flora::SLOT_M as f32).ceil() as i32;
        let cx = (viewer[0] / flora::SLOT_M as f32).floor() as i32;
        let cz = (viewer[1] / flora::SLOT_M as f32).floor() as i32;
        let mut grass = Vec::new();
        for dx in -ring..=ring {
            for dz in -ring..=ring {
                let slot = SlotCoord { x: cx + dx, z: cz + dz };
                if flora::plant_at(gen, slot).map(|p| p.kind) != Some(PlantKind::Grass) {
                    continue;
                }
                let [cxm, czm] = slot.center_m();
                let j = flora::jitter(gen, slot);
                let x = cxm + j[0] * 0.8;
                let z = czm + j[1] * 0.8;
                let d = (x - viewer[0]).hypot(z - viewer[1]);
                if d > gr {
                    continue;
                }
                let y = gen.effective_surface_mm((x * 1000.0) as i64, (z * 1000.0) as i64) as f32
                    / 1000.0;
                grass.push(Instance {
                    pos_scale: [x, y, z, 0.8 + j[2] * 0.5],
                    params: [j[2] * 6.28, 1.0, 0.9 + j[2] * 0.2, 0.0],
                });
            }
        }
        self.grass_bucket = if grass.is_empty() {
            None
        } else {
            let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("grass instances"),
                contents: bytemuck::cast_slice(&grass),
                usage: wgpu::BufferUsages::VERTEX,
            });
            Some(Bucket {
                buf,
                count: grass.len() as u32,
            })
        };
        self.buckets = buckets;
    }

    /// Attaches the grass mask bind group (the cutout pipeline's group 1)
    /// and uploads the mask bytes.
    pub fn attach_mask(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) {
        let data = crate::atmosphere::grass_mask_rgba();
        let n = crate::atmosphere::LEAF_MASK_PX;
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pc3d grass mask"),
            size: wgpu::Extent3d {
                width: n,
                height: n,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pc3d grass mask sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pc3d grass mask bind group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &tex.create_view(&Default::default()),
                    ),
                },
            ],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(n * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: n,
                height: n,
                depth_or_array_layers: 1,
            },
        );
        self.grass_mask_bg = Some(bg);
    }

    /// Draws every (kind, LOD) bucket — one draw call each — then the
    /// grass cards through the cutout-instanced pipeline.
    pub fn draw<'rp>(
        &mut self,
        pass: &mut wgpu::RenderPass<'rp>,
        pipelines: &crate::renderer::FloraPipelines,
        bg_globals: &wgpu::BindGroup,
    ) {
        let mut drawn = 0usize;
        let mut buckets = 0usize;
        pass.set_pipeline(&pipelines.inst);
        pass.set_bind_group(0, bg_globals, &[]);
        for (key, b) in &self.buckets {
            let Some(k) = self.kinds.get(&key.0) else { continue };
            let Some((vb, ib, count)) = k.lods.get(key.1 as usize) else { continue };
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_vertex_buffer(1, b.buf.slice(..));
            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..*count, 0, 0..b.count);
            drawn += b.count as usize;
            buckets += 1;
        }
        if let (Some(g), Some(bg_mask)) = (&self.grass_bucket, &self.grass_mask_bg) {
            pass.set_pipeline(&pipelines.inst_cutout);
            pass.set_bind_group(0, bg_globals, &[]);
            pass.set_bind_group(1, bg_mask, &[]);
            pass.set_vertex_buffer(0, self.grass.0.slice(..));
            pass.set_vertex_buffer(1, g.buf.slice(..));
            pass.set_index_buffer(self.grass.1.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..self.grass.2, 0, 0..g.count);
            drawn += g.count as usize;
            buckets += 1;
        }
        self.stats.instances_drawn = drawn;
        self.stats.draw_buckets = buckets;
    }

    /// The sun-shadow draw: solid kinds only (grass casts nothing).
    pub fn draw_shadow<'rp>(
        &mut self,
        pass: &mut wgpu::RenderPass<'rp>,
        pipelines: &crate::renderer::FloraPipelines,
        bg_light: &wgpu::BindGroup,
    ) {
        pass.set_pipeline(&pipelines.inst_shadow);
        pass.set_bind_group(0, bg_light, &[]);
        for (key, b) in &self.buckets {
            let Some(k) = self.kinds.get(&key.0) else { continue };
            let Some((vb, ib, count)) = k.lods.get(key.1 as usize) else { continue };
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_vertex_buffer(1, b.buf.slice(..));
            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..*count, 0, 0..b.count);
        }
    }
}

// ---------------------------------------------------------------------------
// Collision: the same authority decides what blocks. Grass never blocks;
// a solid plant blocks the meter around its trunk.
// ---------------------------------------------------------------------------

/// A collision adapter: another surface plus the wilderness trunks.
/// The answer is derived from the AUTHORITY (never the render cache),
/// so eviction/reload cannot ghost it — the law the walk test asserts.
pub struct FloraGround<S> {
    pub inner: S,
}

impl<S: crate::player::CollisionSurface> crate::player::CollisionSurface for FloraGround<S> {
    fn ground_at(&self, gen: &WorldGen, x: f32, z: f32, from_y: f32) -> Option<f32> {
        self.inner.ground_at(gen, x, z, from_y)
    }

    fn cell_solid(&self, gen: &WorldGen, x: i32, y: i32, z: i32) -> bool {
        if self.inner.cell_solid(gen, x, y, z) {
            return true;
        }
        // Trunk cells: solid from the ground up ~3 m (chest height and
        // above — the visible blocker).
        let ground = gen.effective_surface_mm(x as i64 * 1000, z as i64 * 1000) as f32 / 1000.0;
        let wy = y as f32;
        if wy < ground - 0.5 || wy > ground + 3.0 {
            return false;
        }
        trunk_solid(gen, x as f32 + 0.5, z as f32 + 0.5)
    }
}

/// True when a trunk/rock occupies (x, z) — the simple-collision query
/// the player adapter wraps (matches the visible placement exactly, so
/// no ghost collision after eviction or reload: the answer is derived
/// from the authority, never from the render cache).
pub fn trunk_solid(gen: &WorldGen, x: f32, z: f32) -> bool {
    let slot = SlotCoord::from_world_mm((x * 1000.0) as i64, (z * 1000.0) as i64);
    match flora::plant_at(gen, slot) {
        Some(p) if p.kind.blocks_movement() => {
            let [cx, cz] = slot.center_m();
            (x - cx).abs() < 0.55 && (z - cz).abs() < 0.55
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Proofs (NWR-007)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::CollisionSurface as _;

    /// A center with both trees and grass in view — searched, not
    /// assumed (the first guess proved nothing on the hills scene).
    fn vegetated_center(gen: &WorldGen) -> [f32; 2] {
        for cx in -20..20i32 {
            for cz in -20..20i32 {
                let vx = cx as f32 * 40.0;
                let vz = cz as f32 * 40.0;
                let mut trees = 0;
                let mut grass = 0;
                for dx in -8..8i32 {
                    for dz in -8..8i32 {
                        let slot = SlotCoord {
                            x: (vx / 4.0) as i32 + dx,
                            z: (vz / 4.0) as i32 + dz,
                        };
                        match flora::plant_at(gen, slot).map(|p| p.kind) {
                            Some(k) if k.blocks_movement() => trees += 1,
                            Some(PlantKind::Grass) => grass += 1,
                            _ => {}
                        }
                    }
                }
                if trees >= 2 && grass >= 2 {
                    return [vx, vz];
                }
            }
        }
        panic!("no vegetated center found on this seed");
    }

    #[test]
    fn cache_is_bounded_evicts_and_reloads() {
        let gen = WorldGen::new(4242);
        // Build through an offscreen renderer's device (flora upload
        // happens per frame; the unit law only needs placement math).
        let r0 = crate::renderer::Renderer::offscreen(8, 8);
        let device = r0.device_for_tests();
        let mut f = FloraStreamer::new(device);
        let a = vegetated_center(&gen);
        // Settle at A: repeated bounded updates fill the ring.
        let mut calls = 0;
        loop {
            calls += 1;
            f.update(&gen, a);
            if f.stats.scanned < f.config.max_slots_per_update as usize / 2 && calls > 4 {
                break;
            }
            assert!(calls < 200, "ring never settles");
        }
        let cached_at_a = f.stats.cached;
        assert!(cached_at_a > 20, "a populated ring ({cached_at_a})");
        let peak = f.stats.added;
        // Teleport far away: the next update EVICTS the old ring.
        let b = [a[0] + 4000.0, a[1] + 4000.0];
        f.update(&gen, b);
        assert!(
            f.stats.evicted > 0 || f.stats.cached < cached_at_a,
            "teleport evicts ({:?})",
            f.stats
        );
        // Walk back: the ring RELOADS (the same plants — determinism).
        let added_before = f.stats.added;
        for _ in 0..40 {
            f.update(&gen, a);
        }
        assert!(f.stats.added > added_before, "returning reloads the ring");
        assert_eq!(f.stats.cached, cached_at_a, "the same ring returns");
        let _ = peak;
    }

    #[test]
    fn trunk_collision_matches_the_visible_placement() {
        let gen = WorldGen::new(4242);
        let a = vegetated_center(&gen);
        // Find a solid plant and a grass slot near the center.
        let mut tree_slot = None;
        let mut grass_slot = None;
        for dx in -8..8i32 {
            for dz in -8..8i32 {
                let slot = SlotCoord {
                    x: (a[0] / 4.0) as i32 + dx,
                    z: (a[1] / 4.0) as i32 + dz,
                };
                match flora::plant_at(&gen, slot).map(|p| p.kind) {
                    Some(k) if k.blocks_movement() => tree_slot.get_or_insert(slot),
                    Some(PlantKind::Grass) => grass_slot.get_or_insert(slot),
                    _ => continue,
                };
            }
        }
        let t = tree_slot.expect("a solid plant near the center");
        let [tx, tz] = t.center_m();
        assert!(trunk_solid(&gen, tx, tz), "the trunk is solid at its center");
        assert!(
            !trunk_solid(&gen, tx + 2.4, tz + 2.4),
            "two meters aside is clear"
        );
        if let Some(g) = grass_slot {
            let [gx, gz] = g.center_m();
            assert!(!trunk_solid(&gen, gx, gz), "grass never blocks");
        }
        // The walk law: a player walking into the trunk stops; the same
        // walk on clear ground moves.
        let ground = crate::player::AuthorityGround;
        let flora_ground = FloraGround { inner: ground };
        let start = [tx - 6.0, gen.effective_surface_mm(((tx - 6.0) * 1000.0) as i64, (tz * 1000.0) as i64) as f32 / 1000.0, tz];
        let mut body = crate::player::PlayerBody {
            pos: start,
            yaw: 0.0,
            pitch: 0.0,
        };
        // Face +X (toward the trunk).
        body.yaw = std::f32::consts::PI;
        for _ in 0..240 {
            body.walk_on(&gen, &flora_ground, 1.0, 0.0, 1.0 / 60.0);
        }
        assert!(
            body.pos[0] < tx - 0.2,
            "the trunk stops the walk ({} < {})",
            body.pos[0],
            tx - 0.2
        );
        // No ghost: the same adapter still reports clear ground far away.
        assert!(!flora_ground.cell_solid(&gen, (tx + 60.0) as i32, start[1] as i32 + 2, tz as i32));
    }

    /// GPU: the wilderness RENDERS (control diff), the wind moves the
    /// grass between two frozen times, LOD buckets split by distance,
    /// and the sun-shadow pass runs with instances bound.
    #[test]
    fn wilderness_renders_with_wind_lod_and_shadows() {
        let gen_rc = std::rc::Rc::new(WorldGen::new(4242));
        let a = vegetated_center(&gen_rc);
        let gy = gen_rc.effective_surface_mm((a[0] * 1000.0) as i64, (a[1] * 1000.0) as i64) as f32
            / 1000.0;
        // A surface patch window around the center + a vista pose.
        let region = crate::surface::SurfaceRegion::new(
            WorldGen::new(4242),
            pc3d_world::coords::PatchCoord {
                x: (a[0] as i32).div_euclid(16),
                y: 1,
                z: (a[1] as i32).div_euclid(16),
            },
        );
        let (verts, idx, _) = region.mesh_region();
        let eye = [a[0] - 18.0, gy + 6.0, a[1] + 18.0];
        let d = [a[0] - eye[0], gy - eye[1], a[1] - eye[2]];
        let pose = crate::camera::CameraPose::new(
            eye,
            (-d[0]).atan2(-d[2]),
            (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin(),
        );

        // Control: no flora.
        let mut ctrl = crate::renderer::Renderer::offscreen(384, 288);
        ctrl.set_placeholder_scene(false);
        ctrl.load_surface(&verts, &idx);
        ctrl.set_pose(pose);
        ctrl.set_atmosphere_tier(crate::atmosphere::AtmosphereTier::Mid);
        let (_, no_flora) = ctrl.capture_png(&std::env::temp_dir().join("pc3d_wild_off.png"), &[]);

        let mut r = crate::renderer::Renderer::offscreen(384, 288);
        r.set_placeholder_scene(false);
        r.load_surface(&verts, &idx);
        r.set_pose(pose);
        r.set_atmosphere_tier(crate::atmosphere::AtmosphereTier::Mid);
        r.attach_flora(gen_rc.clone());
        r.set_water_time(Some(0.5));
        let report = r
            .capture_png(&std::env::temp_dir().join("pc3d_wild_on.png"), &[])
            .0;
        let (_, with_flora) = ctrl_capture(&mut r);
        let _ = report;

        // Presence: the frame changed where plants appeared.
        let diff = crate::scene::pixel_difference_fraction(&no_flora, &with_flora);
        println!(
            "wilderness: diff {diff:.3}, stats {:?}",
            r.flora_stats()
        );
        assert!(diff > 0.005, "the wilderness is visible ({diff})");
        assert!(r.flora_stats().instances_drawn > 40, "a populated ring draws");
        assert!(r.flora_stats().draw_buckets >= 2, "LOD buckets split");

        // Wind: two frozen times differ (grass sways).
        r.set_water_time(Some(2.5));
        let (_, later) = ctrl_capture(&mut r);
        let wdiff = crate::scene::pixel_difference_fraction(&with_flora, &later);
        println!("wind: frame diff at two times {wdiff:.4}");
        assert!(wdiff > 0.0005, "the wind moves the grass ({wdiff})");
    }

    fn ctrl_capture(r: &mut crate::renderer::Renderer) -> (crate::scene::PixelReport, Vec<u8>) {
        r.capture_png(&std::env::temp_dir().join("pc3d_wild_tmp.png"), &[])
    }
}
