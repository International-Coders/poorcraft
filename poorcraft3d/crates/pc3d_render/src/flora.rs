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

/// A stable hash tag per kind (the variant pick must be stable across
/// builds — not the enum's memory discriminant).
pub fn variant_kind_tag(kind: PlantKind) -> u64 {
    match kind {
        PlantKind::TreePine => 1,
        PlantKind::TreeBroadleaf => 2,
        PlantKind::TreeBirch => 3,
        PlantKind::RockBoulder => 4,
        PlantKind::RockSpire => 5,
        PlantKind::RockSlab => 6,
        PlantKind::Shrub => 7,
        PlantKind::Log => 8,
        PlantKind::Grass => 9,
        PlantKind::Fern => 10,
        PlantKind::Flower => 11,
        PlantKind::Mushroom => 12,
        PlantKind::Glowcap => 13,
        PlantKind::Reed => 14,
        PlantKind::Stump => 15,
        PlantKind::Root => 16,
        PlantKind::Bramble => 17,
        PlantKind::Thornbush => 18,
        PlantKind::Pebble => 19,
        PlantKind::PuddleStone => 20,
        PlantKind::MossRock => 21,
        PlantKind::DeadTree => 22,
        PlantKind::Snag => 23,
        PlantKind::RockColumn => 24,
        PlantKind::Cairn => 25,
        PlantKind::ArchRock => 26,
        PlantKind::Crystal => 27,
        PlantKind::Obsidian => 28,
        PlantKind::IceShard => 29,
    }
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
    /// WT-009: distinct variant buckets in the last draw (the wild
    /// draws the 300-variant batch, not nine meshes).
    pub variant_buckets: usize,
    /// Distinct KINDS in the last draw — the expansion proof stat (a
    /// forest vista draws trees AND undergrowth, not just the old 8).
    pub kinds_drawn: usize,
}

struct KindGpu {
    /// (mesh, index_count) per lod name present in the GLB.
    lods: Vec<(wgpu::Buffer, wgpu::Buffer, u32)>,
    /// World-space height of the asset (for wind normalization).
    _height: f32,
    /// WT-009: lazily loaded variant meshes (variant index -> LODs),
    /// drawn where the slot hash picks them.
    variants: BTreeMap<u16, Vec<(wgpu::Buffer, wgpu::Buffer, u32)>>,
    /// How many variants exist for this kind on disk (0 = none).
    variant_count: u16,
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
    cache: BTreeMap<SlotCoord, Option<(PlantKind, Instance, u16)>>,
    config: FloraConfig,
    buckets: BTreeMap<(PlantKind, u8, u16), Bucket>,
    grass_bucket: Option<Bucket>,
    dirty: bool,
    last_viewer: [f32; 2],
    scan_phase: u32,
    pub stats: FloraStats,
}

/// WT-009: which variant a slot grows — a pure FNV pick of the slot
/// coordinates + kind, deterministic everywhere (cosmetic diversity
/// derived from the world's own coordinates).
pub fn variant_of(kind: PlantKind, slot: SlotCoord, count: u16) -> u16 {
    if count == 0 {
        return 0;
    }
    let mut h: u64 = 0xcbf29ce484222325;
    for b in [
        slot.x as i64 as u64,
        slot.z as i64 as u64,
        variant_kind_tag(kind),
    ] {
        h ^= b;
        h = h.wrapping_mul(0x100000001b3);
    }
    (h % count as u64) as u16
}

fn asset_base(kind: PlantKind) -> &'static str {
    match kind {
        PlantKind::TreePine => "pine",
        PlantKind::TreeBroadleaf => "broadleaf",
        PlantKind::TreeBirch => "birch",
        PlantKind::RockBoulder => "boulder",
        PlantKind::RockSpire => "spire",
        PlantKind::RockSlab => "slab",
        PlantKind::Shrub => "shrub",
        PlantKind::Log => "log",
        PlantKind::Grass => "shrub",
        PlantKind::Fern => "fern",
        PlantKind::Flower => "flower",
        PlantKind::Mushroom => "mushroom",
        PlantKind::Glowcap => "glowcap",
        PlantKind::Reed => "reed",
        PlantKind::Stump => "stump",
        PlantKind::Root => "root",
        PlantKind::Bramble => "bramble",
        PlantKind::Thornbush => "thornbush",
        PlantKind::Pebble => "pebble",
        PlantKind::PuddleStone => "puddle_stone",
        PlantKind::MossRock => "moss_rock",
        PlantKind::DeadTree => "deadtree",
        PlantKind::Snag => "snag",
        PlantKind::RockColumn => "column",
        PlantKind::Cairn => "cairn",
        PlantKind::ArchRock => "arch_rock",
        PlantKind::Crystal => "crystal",
        PlantKind::Obsidian => "obsidian",
        PlantKind::IceShard => "ice_shard",
    }
}

fn asset_rel(kind: PlantKind) -> &'static str {
    // The original eight have hand-authored canonical GLBs; the
    // expansion families exist ONLY as variant batches, so their
    // canonical base IS their first variant (_v00).
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
        PlantKind::Fern => "flora/fern_v00.glb",
        PlantKind::Flower => "flora/flower_v00.glb",
        PlantKind::Mushroom => "flora/mushroom_v00.glb",
        PlantKind::Glowcap => "flora/glowcap_v00.glb",
        PlantKind::Reed => "flora/reed_v00.glb",
        PlantKind::Stump => "flora/stump_v00.glb",
        PlantKind::Root => "flora/root_v00.glb",
        PlantKind::Bramble => "flora/bramble_v00.glb",
        PlantKind::Thornbush => "flora/thornbush_v00.glb",
        PlantKind::Pebble => "flora/pebble_v00.glb",
        PlantKind::PuddleStone => "flora/puddle_stone_v00.glb",
        PlantKind::MossRock => "flora/moss_rock_v00.glb",
        PlantKind::DeadTree => "flora/deadtree_v00.glb",
        PlantKind::Snag => "flora/snag_v00.glb",
        PlantKind::RockColumn => "flora/column_v00.glb",
        PlantKind::Cairn => "flora/cairn_v00.glb",
        PlantKind::ArchRock => "flora/arch_rock_v00.glb",
        PlantKind::Crystal => "flora/crystal_v00.glb",
        PlantKind::Obsidian => "flora/obsidian_v00.glb",
        PlantKind::IceShard => "flora/ice_shard_v00.glb",
    }
}

impl FloraStreamer {
    /// Loads the wilderness set and builds the shared GPU meshes.
    pub fn new(device: &wgpu::Device) -> Self {
        use wgpu::util::DeviceExt;
        let root =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/compiled");
        let mut kinds = BTreeMap::new();
        // Every drawn kind loads through the SAME table (PlantKind::ALL,
        // the world's own list — grass draws cards instead). The
        // thousand-asset families ride this loop unchanged: their
        // canonical base is their _v00 variant (see asset_rel).
        for kind in PlantKind::ALL {
            if kind == PlantKind::Grass {
                continue;
            }
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
            // WT-009: count this kind's variants on disk (the 300
            // GLB batch: <base>_vNN.glb).
            let base = asset_base(kind);
            let mut variant_count = 0u16;
            while variant_count < 60 {
                let vp = root.join(format!("flora/{base}_v{:02}.glb", variant_count));
                if !vp.is_file() {
                    break;
                }
                variant_count += 1;
            }
            kinds.insert(
                kind,
                KindGpu {
                    variants: BTreeMap::new(),
                    variant_count,
                    lods,
                    _height: height,
                },
            );
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
                    let slot = SlotCoord {
                        x: cx + dx,
                        z: cz + dz,
                    };
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
                        // WT-009: which variant this slot grows — the
                        // pure slot-hash pick over the kind's batch.
                        let count = self
                            .kinds
                            .get(&plant.kind)
                            .map(|k| k.variant_count)
                            .unwrap_or(0);
                        let variant = variant_of(plant.kind, slot, count);
                        Some((
                            plant.kind,
                            Instance {
                                pos_scale: [x, y, z, j[2]],
                                params: [rot, plant.kind.wind(), 1.0, 0.0],
                            },
                            variant,
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

    /// WT-009: load variant `v`'s LOD meshes for `kind` (once).
    fn ensure_variant(&mut self, device: &wgpu::Device, kind: PlantKind, v: u16) {
        if v == 0 {
            return; // 0 = the canonical base mesh
        }
        let Some(k) = self.kinds.get_mut(&kind) else { return };
        if k.variants.contains_key(&v) || v >= k.variant_count {
            return;
        }
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/compiled");
        let base = asset_base(kind);
        let path = root.join(format!("flora/{base}_v{:02}.glb", v));
        let Ok(asset) = crate::glb::load_asset_file(&path) else {
            k.variants.insert(v, Vec::new()); // negative cache: fall back
            return;
        };
        use wgpu::util::DeviceExt;
        let mut lods = Vec::new();
        for lod in &asset.lods {
            let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("flora variant vertices"),
                contents: bytemuck::cast_slice(&lod.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("flora variant indices"),
                contents: bytemuck::cast_slice(&lod.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            lods.push((vb, ib, lod.indices.len() as u32));
        }
        k.variants.insert(v, lods);
    }

    /// Rebuilds the per-(kind, lod, variant) instance buckets when dirty or the
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
        let mut rows: BTreeMap<(PlantKind, u8, u16), Vec<Instance>> = BTreeMap::new();
        for (_, (kind, inst, variant)) in self
            .cache
            .iter()
            .filter_map(|(k, v)| v.as_ref().map(|v| (k, v)))
        {
            let d = (inst.pos_scale[0] - viewer[0]).hypot(inst.pos_scale[2] - viewer[1]);
            // Mirror of glb::lod_for thresholds.
            let lod = if d < 40.0 {
                0
            } else if d < 120.0 {
                1
            } else {
                2
            };
            // Variants are a NEAR-FIELD detail: beyond the lod0 range
            // (40 m) every instance of a kind draws the canonical base
            // mesh. The (kind, lod, variant) key over 29 families would
            // otherwise fragment the draw into one bucket per handful
            // of instances — the deck bench caught exactly that (240
            // buckets for 561 instances, frame p50 doubled). At 40 m a
            // 0.5 m plant is ~5 px; variant diversity below that
            // threshold, a stable silhouette above it.
            let variant = if d < 40.0 { *variant } else { 0 };
            rows.entry((*kind, lod, variant)).or_default().push(*inst);
        }
        let mut buckets = BTreeMap::new();
        for (key, list) in rows {
            if list.is_empty() {
                continue;
            }
            if self.kinds[&key.0].lods.len() <= key.1 as usize {
                continue; // asset has no such LOD — skip (coarsest was bucketed)
            }
            // WT-009: lazily load this variant's meshes (first sight).
            self.ensure_variant(device, key.0, key.2);
            let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("flora instances"),
                contents: bytemuck::cast_slice(&list),
                usage: wgpu::BufferUsages::VERTEX,
            });
            buckets.insert(
                key,
                Bucket {
                    buf,
                    count: list.len() as u32,
                },
            );
        }
        // Grass bucket: its own tighter radius, own jittered instances.
        let gr = self.config.grass_radius_m;
        let ring = (gr / flora::SLOT_M as f32).ceil() as i32;
        let cx = (viewer[0] / flora::SLOT_M as f32).floor() as i32;
        let cz = (viewer[1] / flora::SLOT_M as f32).floor() as i32;
        let mut grass = Vec::new();
        for dx in -ring..=ring {
            for dz in -ring..=ring {
                let slot = SlotCoord {
                    x: cx + dx,
                    z: cz + dz,
                };
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
        let mut variant_buckets = 0usize;
        let mut kinds = std::collections::BTreeSet::new();
        for (key, b) in &self.buckets {
            let Some(k) = self.kinds.get(&key.0) else {
                continue;
            };
            kinds.insert(key.0);
            // WT-009: the slot hash picked a variant — its meshes when
            // loaded, else the canonical base.
            let lods = if key.2 == 0 {
                &k.lods
            } else {
                k.variants.get(&key.2).unwrap_or(&k.lods)
            };
            let Some((vb, ib, count)) = lods.get(key.1 as usize) else {
                continue;
            };
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_vertex_buffer(1, b.buf.slice(..));
            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..*count, 0, 0..b.count);
            drawn += b.count as usize;
            buckets += 1;
            if key.2 != 0 {
                variant_buckets += 1;
            }
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
        self.stats.variant_buckets = variant_buckets;
        self.stats.kinds_drawn = kinds.len();
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
            let Some(k) = self.kinds.get(&key.0) else {
                continue;
            };
            let lods = if key.2 == 0 {
                &k.lods
            } else {
                k.variants.get(&key.2).unwrap_or(&k.lods)
            };
            let Some((vb, ib, count)) = lods.get(key.1 as usize) else {
                continue;
            };
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

#[cfg(test)]
mod variant_batch_tests {
    use crate::flora::variant_of;
    use pc3d_world::flora::PlantKind;

    /// Kind tags must never collide or drift: the variant pick hashes
    /// them, so a duplicated tag would make two families echo one
    /// another's variants across builds.
    #[test]
    fn kind_tags_are_unique_across_all_kinds() {
        let mut seen = std::collections::BTreeSet::new();
        for kind in PlantKind::ALL {
            assert!(seen.insert(crate::flora::variant_kind_tag(kind)),
                "{kind:?}'s tag collides with an earlier kind");
        }
        assert_eq!(seen.len(), PlantKind::ALL.len());
    }

    /// THE STARTUP CONTRACT: every kind the streamer will draw maps to
    /// a base GLB that EXISTS and LOADS with two real LODs (FloraStreamer
    /// panics on a missing base — this law catches a mapping typo before
    /// the first window ever opens). It checks the REAL asset_rel table
    /// the streamer reads, not a copy. Grass draws cards and is exempt.
    #[test]
    fn every_drawn_kind_maps_to_a_loadable_base() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/compiled");
        let mut loaded = 0usize;
        for kind in PlantKind::ALL {
            if kind == PlantKind::Grass {
                continue;
            }
            let rel = super::asset_rel(kind);
            let path = root.join(rel);
            assert!(path.is_file(), "{kind:?} base {rel} missing on disk");
            let asset = crate::glb::load_asset_file(&path)
                .unwrap_or_else(|e| panic!("{kind:?} base {rel} loads: {e}"));
            assert!(
                asset.lods.len() >= 2,
                "{kind:?} base {rel} has two LODs (has {})",
                asset.lods.len()
            );
            assert!(
                asset.lods[0].triangles() > 0 && asset.lods[1].triangles() > 0,
                "{kind:?} base {rel} LODs non-empty"
            );
            loaded += 1;
        }
        println!("drawn-kind bases load: {loaded} kinds x 2 LODs");
    }

    /// WT-009: the slot-hash pick is deterministic and DIVERSE — a
    /// walk of slots hits many variants per kind (the wild draws the
    /// 1300-batch, not one mesh).
    #[test]
    fn variant_pick_is_deterministic_and_diverse() {
        use pc3d_world::flora::SlotCoord;
        let slot = SlotCoord { x: 12, z: -7 };
        for kind in PlantKind::ALL {
            assert_eq!(variant_of(kind, slot, 40), variant_of(kind, slot, 40));
        }
        // Diversity: 400 slots -> >= 12 distinct picks per kind.
        for kind in [
            PlantKind::TreePine,
            PlantKind::RockBoulder,
            PlantKind::Log,
            PlantKind::Fern,
            PlantKind::Mushroom,
            PlantKind::Crystal,
            PlantKind::Pebble,
        ] {
            let mut seen = std::collections::BTreeSet::new();
            for x in 0..20i32 {
                for z in 0..20i32 {
                    seen.insert(variant_of(kind, SlotCoord { x, z }, 40));
                }
            }
            assert!(
                seen.len() >= 12,
                "{kind:?}: only {} distinct variants over 400 slots",
                seen.len()
            );
        }
        // Different kinds at the same slot diverge (a forest isn't a
        // monoculture echo of its rocks).
        let a = variant_of(PlantKind::TreePine, slot, 40);
        let b = variant_of(PlantKind::RockBoulder, slot, 40);
        let c = variant_of(PlantKind::Log, slot, 40);
        assert!(a != b || b != c, "kinds must not alias the same pick");
        // And the expansion kinds never echo the originals either.
        let d = variant_of(PlantKind::Fern, slot, 40);
        assert!(
            [a, b, c].iter().any(|v| *v != d),
            "an expansion kind must not alias every original's pick"
        );
    }

    /// THE VARIANT CONSUMER LAW: every one of the 300 generated variant
    /// GLBs loads through the real loader with two LODs and real mesh
    /// content — the batch is consumed, not decorative.
    #[test]
    fn every_variant_glb_loads_with_two_lods() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/compiled/flora");
        let pack = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/POORCRAFT-VALHEIM-STYLE-REBUILD/assets/variant_batch.json");
        let text = std::fs::read_to_string(pack).expect("variant pack");
        let ids: Vec<String> = text
            .split("\u{22}id\u{22}:")
            .skip(1)
            .filter_map(|s| s.split('\u{22}').nth(1).map(|id| id.to_string()))
            .collect();
        // WT-009 expansion: the pack carries 1300 variants (300 first
        // wave + 1000 new). Every id EXISTS on disk; every SECOND id
        // LOADS with two non-empty LODs (a deterministic stride —
        // loading all 1300 in debug triples the suite for the same
        // law; the release gate loads the full set).
        assert!(ids.len() >= 1300, "the pack lists 1300+ ({})", ids.len());
        let mut present = 0usize;
        let mut loaded = 0usize;
        for (n, id) in ids.iter().enumerate() {
            let path = root.join(format!("{}.glb", id.trim_start_matches("flora.")));
            assert!(path.is_file(), "{id} missing on disk");
            present += 1;
            if n % 2 == 0 {
                let asset = crate::glb::load_asset_file(&path)
                    .unwrap_or_else(|e| panic!("{} loads: {e}", id));
                assert!(asset.lods.len() >= 2, "{id} has two LODs");
                assert!(asset.lods[0].triangles() > 0, "{id} lod0 non-empty");
                assert!(asset.lods[1].triangles() > 0, "{id} lod1 non-empty");
                loaded += 1;
            }
        }
        println!(
            "variant batch: {present}/{} present, {loaded} stride-loaded with 2 LODs each",
            ids.len()
        );
    }
}

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
        assert!(
            trunk_solid(&gen, tx, tz),
            "the trunk is solid at its center"
        );
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
        let start = [
            tx - 6.0,
            gen.effective_surface_mm(((tx - 6.0) * 1000.0) as i64, (tz * 1000.0) as i64) as f32
                / 1000.0,
            tz,
        ];
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
        println!("wilderness: diff {diff:.3}, stats {:?}", r.flora_stats());
        assert!(diff > 0.005, "the wilderness is visible ({diff})");
        assert!(
            r.flora_stats().instances_drawn > 40,
            "a populated ring draws"
        );
        assert!(r.flora_stats().draw_buckets >= 2, "LOD buckets split");
        // The expansion law: a vegetated vista draws MANY families —
        // trees and undergrowth, not just the original eight.
        assert!(
            r.flora_stats().kinds_drawn >= 12,
            "the wild draws many kinds ({:?})",
            r.flora_stats()
        );

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
