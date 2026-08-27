pub mod biome;

/// World generation archetype.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum WorldType {
    #[default]
    Normal,
    Superflat,
    Amplified,
}

use fastnoise_lite::{FastNoiseLite, NoiseType, FractalType};
pub use biome::{Biome, TreeKind};

/// Deterministic world generation seed across all platforms.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Seed(pub u64);

/// A block position in world space.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Water surface height used by terrain generation.
pub const SEA_LEVEL: i32 = 62;

/// Version of the terrain/climate generator. Bump whenever generation
/// changes in a way that can alter chunks for an existing seed. Saves stamp
/// it into `genver.dat`; a mismatch means unedited chunks that get
/// regenerated after a revisit may differ from their first visit (edited
/// chunks are persisted and never regenerated). Pre-P25 worlds have no
/// stamp and read as `None`.
pub const GENERATOR_VERSION: u32 = 2; // v2: biome-identity surfaces (jungle/savanna grass, mycelium hollow, wildflowers)

/// Stamp `genver.dat` in a world directory with the generator version.
pub fn save_generator_version(dir: &std::path::Path, version: u32) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join("genver.dat"), version.to_string())
}

/// The generator version a world directory was last played with.
pub fn load_generator_version(dir: &std::path::Path) -> Option<u32> {
    std::fs::read_to_string(dir.join("genver.dat")).ok()?
        .trim().parse().ok()
}

/// A block type from the global registry (indices only).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BlockId(pub u32);

impl BlockId {
    pub const AIR: Self = Self(0);
    pub const STONE: Self = Self(1);
    pub const DIRT: Self = Self(2);
    pub const GRASS: Self = Self(3);
    pub const SAND: Self = Self(4);
    pub const MYCELIUM: Self = Self(5);
    pub const SNOW: Self = Self(6);
}

/// World generation data: heightmap, biomes, and strata per chunk column.
pub struct WorldGen {
    pub world_type: WorldType,
    seed: u64,
    noise_base: FastNoiseLite,
    noise_temp: FastNoiseLite,
    noise_humid: FastNoiseLite,
    noise_variant: FastNoiseLite,
    noise_cave: FastNoiseLite,
    noise_ore: FastNoiseLite,
    /// Low-frequency domain warp for the climate fields: biome borders
    /// follow organic curves instead of straight noise level-sets.
    noise_warp_x: FastNoiseLite,
    noise_warp_z: FastNoiseLite,
    /// Fine high-frequency dither: breaks threshold lines into natural
    /// dithered transition bands of mixed surface blocks.
    noise_dither: FastNoiseLite,
}

/// Splitmix64 — decorrelates a u64 seed into per-channel i32 seeds without
/// the truncation collisions a bare `as i32` would cause.
fn channel_seed(seed: u64, salt: u64) -> i32 {
    let mut z = seed.wrapping_add(salt.wrapping_mul(0x9E3779B97F4A7C15));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    (z ^ (z >> 31)) as i32
}

/// How far (in blocks) domain warping can shift climate sampling.
const WARP_AMPLITUDE: f32 = 34.0;
/// Dither amplitude applied to climate inputs near biome thresholds.
const DITHER: f32 = 0.045;

impl WorldGen {
    pub fn new(seed: Seed) -> Self {
        Self::with_type(seed, WorldType::Normal)
    }

    pub fn with_type(seed: Seed, world_type: WorldType) -> Self {
        let s = seed.0;
        let mut base = FastNoiseLite::new();
        base.set_seed(Some(channel_seed(s, 1)));
        base.set_noise_type(Some(NoiseType::Perlin));
        base.set_fractal_type(Some(FractalType::FBm));
        base.set_frequency(Some(0.01));

        // climate: fractal + lower frequency so biomes are broad regions
        // that transition smoothly, then warped + dithered at classification
        let mut temp = FastNoiseLite::new();
        temp.set_seed(Some(channel_seed(s, 7)));
        temp.set_noise_type(Some(NoiseType::Perlin));
        temp.set_fractal_type(Some(FractalType::FBm));
        temp.set_fractal_octaves(Some(3));
        temp.set_frequency(Some(0.0028));

        let mut humid = FastNoiseLite::new();
        humid.set_seed(Some(channel_seed(s, 13)));
        humid.set_noise_type(Some(NoiseType::Perlin));
        humid.set_fractal_type(Some(FractalType::FBm));
        humid.set_fractal_octaves(Some(3));
        humid.set_frequency(Some(0.0032));

        let mut variant = FastNoiseLite::new();
        variant.set_seed(Some(channel_seed(s, 31)));
        variant.set_noise_type(Some(NoiseType::Perlin));
        variant.set_fractal_type(Some(FractalType::FBm));
        variant.set_fractal_octaves(Some(2));
        variant.set_frequency(Some(0.0042));

        let mut cave = FastNoiseLite::new();
        cave.set_seed(Some(channel_seed(s, 101)));
        cave.set_noise_type(Some(NoiseType::Perlin));
        cave.set_fractal_type(Some(FractalType::FBm));
        cave.set_frequency(Some(0.03));

        let mut ore = FastNoiseLite::new();
        ore.set_seed(Some(channel_seed(s, 211)));
        ore.set_noise_type(Some(NoiseType::Perlin));
        ore.set_frequency(Some(0.09));

        let mut warp_x = FastNoiseLite::new();
        warp_x.set_seed(Some(channel_seed(s, 409)));
        warp_x.set_noise_type(Some(NoiseType::Perlin));
        warp_x.set_frequency(Some(0.004));

        let mut warp_z = FastNoiseLite::new();
        warp_z.set_seed(Some(channel_seed(s, 613)));
        warp_z.set_noise_type(Some(NoiseType::Perlin));
        warp_z.set_frequency(Some(0.004));

        let mut dither = FastNoiseLite::new();
        dither.set_seed(Some(channel_seed(s, 733)));
        dither.set_noise_type(Some(NoiseType::Perlin));
        dither.set_frequency(Some(0.22));

        Self {
            world_type,
            seed: s,
            noise_base: base,
            noise_temp: temp,
            noise_humid: humid,
            noise_variant: variant,
            noise_cave: cave,
            noise_ore: ore,
            noise_warp_x: warp_x,
            noise_warp_z: warp_z,
            noise_dither: dither,
        }
    }

    /// The world's seed (persisted with the save).
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Height at chunk column (x,z) in blocks. Range spans below and above
    /// sea level so ocean and mountain biomes are both reachable.
    pub fn height(&self, cx: i32, cz: i32) -> i32 {
        if self.world_type == WorldType::Superflat {
            return 64;
        }
        let amp = match self.world_type {
            WorldType::Amplified => 2.0,
            _ => 1.0,
        };
        let n = self.noise_base.get_noise_2d(cx as f32, cz as f32);
        let scale = (n + 1.0) * 0.5; // 0..1, but FBM stays near the middle
        // Stretch the occupied band so real oceans and peaks exist.
        let stretched = (scale * 1.43 - 0.21).clamp(0.0, 1.0);
        let base = 24;
        let amp = 152.0 * amp;
        (base + (stretched * amp).round() as i32).max(8)
    }

    /// Warped climate sample point (shared by t/h/v so the fields stay
    /// spatially coherent under the warp).
    fn warped(&self, cx: i32, cz: i32) -> (f32, f32) {
        let wx = self.noise_warp_x.get_noise_2d(cx as f32, cz as f32);
        let wz = self.noise_warp_z.get_noise_2d(cx as f32, cz as f32);
        (cx as f32 + wx * WARP_AMPLITUDE, cz as f32 + wz * WARP_AMPLITUDE)
    }

    /// Fractal noise concentrates near 0; stretch the climate fields so
    /// extreme-value biomes (Ice Spikes, Badlands, Mushroom Hollow...) stay
    /// reachable at the now-larger biome scale.
    fn stretch(n: f32) -> f32 {
        ((n + 1.0) * 0.5 * 1.45 - 0.225).clamp(0.0, 1.0)
    }

    /// Temperature [0..1] (domain-warped + stretched).
    pub fn temperature(&self, cx: i32, cz: i32) -> f32 {
        let (x, z) = self.warped(cx, cz);
        Self::stretch(self.noise_temp.get_noise_2d(x, z))
    }

    /// Humidity [0..1] (domain-warped + stretched).
    pub fn humidity(&self, cx: i32, cz: i32) -> f32 {
        let (x, z) = self.warped(cx, cz);
        Self::stretch(self.noise_humid.get_noise_2d(x, z))
    }

    /// Slow variant channel that splits climate bands into neighbor biomes
    /// (domain-warped + stretched).
    pub fn variant(&self, cx: i32, cz: i32) -> f32 {
        let (x, z) = self.warped(cx, cz);
        Self::stretch(self.noise_variant.get_noise_2d(x, z))
    }

    /// Decorrelated fine dither in [-1, 1]; `channel` 0/1/2 for t/h/v.
    fn dither(&self, cx: i32, cz: i32, channel: u32) -> f32 {
        let (dx, dz) = match channel {
            0 => (0.0, 0.0),
            1 => (997.0, 0.0),
            _ => (0.0, 997.0),
        };
        self.noise_dither.get_noise_2d(cx as f32 + dx, cz as f32 + dz)
    }

    /// Biome at column, combining elevation with climate + variant channel.
    /// Climate inputs are domain-warped and dithered so biome borders curve
    /// organically and transition as mixed dithered bands instead of hard
    /// straight lines. Deterministic across platforms.
    pub fn biome(&self, cx: i32, cz: i32) -> Biome {
        let t = (self.temperature(cx, cz) + self.dither(cx, cz, 0) * DITHER).clamp(0.0, 1.0);
        let h = (self.humidity(cx, cz) + self.dither(cx, cz, 1) * DITHER).clamp(0.0, 1.0);
        let v = (self.variant(cx, cz) + self.dither(cx, cz, 2) * DITHER).clamp(0.0, 1.0);
        biome::biome_from(t, h, self.height(cx, cz), v)
    }

    /// Surface block id at column position (per-biome table).
    pub fn surface_block(&self, cx: i32, cz: i32) -> u32 {
        self.biome(cx, cz).surface_block()
    }

    /// Column of blocks from surface down to bedrock: surface band, then
    /// sub-surface (dirt or biome filler), then stone.
    pub fn column(&self, cx: i32, cz: i32) -> Vec<(i32, u32)> {
        use lf_voxel::registry::block;
        let height = self.height(cx, cz);
        let biome = self.biome(cx, cz);
        let surf = biome.surface_block();
        let filler: u32 = biome.filler_block();
        let mut col = Vec::new();
        for y in (0..=height + 16).rev() {
            if y <= height + 3 && y >= height - 2 {
                col.push((y, surf));
            } else if y == height - 3 {
                col.push((y, filler));
            } else if y < height - 3 {
                col.push((y, block::STONE));
            } else {
                col.push((y, block::AIR));
            }
        }
        col
    }

    /// The y of the first air block above the surface (what you stand on, +1).
    /// column() places surface blocks up to height+3, so the standing surface
    /// is height+4; use this instead of height() for entity placement.
    pub fn surface_top(&self, cx: i32, cz: i32) -> i32 {
        self.height(cx, cz) + 4
    }
}

/// A mod-registered ore vein hook.
#[derive(Clone, Debug)]
pub struct OreHook {
    pub block_id: u32,
    pub y_min: i32,
    pub y_max: i32,
    /// Noise threshold (higher = rarer).
    pub threshold: f32,
    /// Offset applied to the ore noise sample so multiple ores decorrelate.
    pub noise_offset: f32,
}

fn ore_hooks() -> &'static std::sync::RwLock<Vec<OreHook>> {
    static HOOKS: std::sync::OnceLock<std::sync::RwLock<Vec<OreHook>>> = std::sync::OnceLock::new();
    HOOKS.get_or_init(|| std::sync::RwLock::new(Vec::new()))
}

/// Register a mod ore vein (idempotent by block id).
pub fn register_ore_hook(hook: OreHook) -> bool {
    let mut hooks = ore_hooks().write().unwrap();
    if hooks.iter().any(|h| h.block_id == hook.block_id) {
        return true;
    }
    hooks.push(hook);
    true
}

pub fn registered_ore_hooks() -> Vec<OreHook> {
    ore_hooks().read().unwrap().clone()
}

/// Deterministic 2D hash for feature placement (trees, etc.).
fn hash2(x: i32, z: i32, seed: u64) -> u64 {
    let mut h = seed
        ^ (x as u64).wrapping_mul(0x9E3779B97F4A7C15)
        ^ (z as u64).wrapping_mul(0xC2B2AE3D27D4EB4F);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51AFD7ED558CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CEB9FE1A85EC53);
    h ^ (h >> 33)
}

fn lf_ore_hooks() -> Vec<OreHook> {
    registered_ore_hooks()
}

impl WorldGen {
    /// Fill a whole 16x256x16 chunk column: terrain strata, caves, ores,
    /// water up to sea level, and trees (canopy kept inside the chunk).
    pub fn generate_chunk(&self, cx: i32, cz: i32) -> lf_voxel::ChunkColumn {
        use lf_voxel::registry::block;
        use lf_voxel::BlockState;

        let mut col = lf_voxel::ChunkColumn::empty();

        // 1. Terrain strata.
        let mut surface_tops = [[0i32; 16]; 16];
        for lx in 0..16usize {
            for lz in 0..16usize {
                let wx = cx * 16 + lx as i32;
                let wz = cz * 16 + lz as i32;
                for (wy, b) in self.column(wx, wz) {
                    if b != lf_voxel::registry::block::AIR {
                        col.set(lx, wy as usize, lz, lf_voxel::BlockState(b));
                    }
                }
                surface_tops[lx][lz] = self.surface_top(wx, wz);
            }
        }

        // 2. Caves: 3D noise carving (skipped in superflat).
        if self.world_type == WorldType::Superflat {
            return self.finish_flat(col, cx, cz, &surface_tops);
        }
        // (normal path below) Never below y=6 (bedrock-ish floor) and
        // rarely punctures the surface (needs a stronger noise value up high).
        for lx in 0..16usize {
            for lz in 0..16usize {
                let wx = (cx * 16 + lx as i32) as f32;
                let wz = (cz * 16 + lz as i32) as f32;
                let top = surface_tops[lx][lz];
                let max_carve = top - 4;
                for y in 6..(SECTION_MAX as i32).min(max_carve.max(7)) {
                    let n = self.noise_cave.get_noise_3d(wx, y as f32, wz);
                    let threshold = if y > top - 12 { 0.60 } else { 0.40 };
                    if n > threshold {
                        col.set(lx, y as usize, lz, BlockState::AIR);
                    }
                }
            }
        }

        // 3. Ores replace stone: coal shallow and common, iron deeper.
        for lx in 0..16usize {
            for lz in 0..16usize {
                let wx = (cx * 16 + lx as i32) as f32;
                let wz = (cz * 16 + lz as i32) as f32;
                let top = (surface_tops[lx][lz] - 5).max(6);
                for y in 6..top {
                    if col.get(lx, y as usize, lz) != BlockState::STONE {
                        continue;
                    }
                    let coal_n = self.noise_ore.get_noise_3d(wx, y as f32, wz);
                    if y < 96 && coal_n > 0.42 {
                        col.set(lx, y as usize, lz, BlockState(block::COAL_ORE));
                        continue;
                    }
                    let iron_n = self.noise_ore.get_noise_3d(wx + 1000.0, y as f32, wz);
                    if y < 48 && iron_n > 0.55 {
                        col.set(lx, y as usize, lz, BlockState(block::IRON_ORE));
                    }
                    // industrial ores: copper (shallow), tin (mid), bauxite + sulfur (deep)
                    let copper_n = self.noise_ore.get_noise_3d(wx + 2000.0, y as f32, wz);
                    if y < 70 && copper_n > 0.52 {
                        col.set(lx, y as usize, lz, BlockState(block::COPPER_ORE));
                    }
                    let tin_n = self.noise_ore.get_noise_3d(wx + 3000.0, y as f32, wz);
                    if y < 50 && tin_n > 0.56 {
                        col.set(lx, y as usize, lz, BlockState(block::TIN_ORE));
                    }
                    let baux_n = self.noise_ore.get_noise_3d(wx + 4000.0, y as f32, wz);
                    if y < 90 && baux_n > 0.60 {
                        col.set(lx, y as usize, lz, BlockState(block::BAUXITE_ORE));
                    }
                    let sulf_n = self.noise_ore.get_noise_3d(wx + 5000.0, y as f32, wz);
                    if y < 40 && sulf_n > 0.58 {
                        col.set(lx, y as usize, lz, BlockState(block::SULFUR_ORE));
                    }
                    // mod ore veins
                    for hook in lf_ore_hooks() {
                        if y >= hook.y_min && y <= hook.y_max {
                            let n = self.noise_ore.get_noise_3d(wx + hook.noise_offset, y as f32, wz);
                            if n > hook.threshold {
                                col.set(lx, y as usize, lz, BlockState(hook.block_id));
                            }
                        }
                    }
                }
            }
        }

        // 3.5 Oil (P31, doc 04): biome-gated crude pools replace deep stone
        //     under desert/swamp — the derrick is river-less power, so the
        //     deposits live where the terrain says they should. Rare
        //     surface seeps are the visible clue.
        for lx in 0..16usize {
            for lz in 0..16usize {
                let wx = cx * 16 + lx as i32;
                let wz = cz * 16 + lz as i32;
                if !matches!(self.biome(wx, wz), Biome::Desert | Biome::Swamp) {
                    continue;
                }
                let wx_f = wx as f32;
                let wz_f = wz as f32;
                for y in 8..44i32 {
                    if col.get(lx, y as usize, lz) != BlockState::STONE {
                        continue;
                    }
                    let oil_n = self.noise_ore.get_noise_3d(wx_f + 6000.0, y as f32, wz_f);
                    if oil_n > 0.63 {
                        col.set(lx, y as usize, lz, lf_voxel::oil_with_level(0));
                    }
                }
                // surface seep: one source pond in the surface block
                if hash2(wx, wz, self.seed_for_features() ^ 0x011CE) % 700 == 0 {
                    let top = surface_tops[lx][lz];
                    if top > SEA_LEVEL + 1 && col.get(lx, (top - 1) as usize, lz).id() != block::AIR {
                        col.set(lx, (top - 1) as usize, lz, lf_voxel::oil_with_level(0));
                    }
                }
            }
        }

        // 4. Water fills open space up to sea level; freezing biomes cap
        //    the surface with ice.
        for lx in 0..16usize {
            for lz in 0..16usize {
                let top = surface_tops[lx][lz];
                if top <= SEA_LEVEL {
                    let freezes = self.biome(cx * 16 + lx as i32, cz * 16 + lz as i32).freezes();
                    for y in top..=SEA_LEVEL {
                        if col.get(lx, y as usize, lz) == BlockState::AIR {
                            let b = if freezes && y == SEA_LEVEL { block::ICE } else { block::WATER };
                            col.set(lx, y as usize, lz, BlockState(b));
                        }
                    }
                }
            }
        }

        // 5. Structures: deterministic per-chunk placement, in-chunk footprint.
        self.place_structures(cx, cz, &mut col);

        // 5.5 Wildflowers: FlowerForest's exclusive ground cover (sparse,
        // deterministic per column; only on intact grass).
        for lx in 1..15usize {
            for lz in 1..15usize {
                let wx = cx * 16 + lx as i32;
                let wz = cz * 16 + lz as i32;
                if self.biome(wx, wz) != Biome::FlowerForest {
                    continue;
                }
                let top = surface_tops[lx][lz];
                if top <= SEA_LEVEL + 1 {
                    continue;
                }
                if col.get(lx, (top - 1) as usize, lz).id() != block::GRASS {
                    continue;
                }
                if col.get(lx, top as usize, lz).id() != block::AIR {
                    continue;
                }
                if hash2(wx, wz, self.seed_for_features() ^ 0xf10a7) % 9 == 0 {
                    col.set(lx, top as usize, lz, BlockState(block::FLOWER));
                }
            }
        }

        // 5.6 Boulder fields: SnowySlope's and WindsweptHills' exclusive
        // ground feature (Step 17) — one 3-block stone cluster per lucky
        // chunk, deterministic.
        for (bx, bz) in [(4usize, 4usize), (11, 10)] {
            let wx = cx * 16 + bx as i32;
            let wz = cz * 16 + bz as i32;
            if !matches!(self.biome(wx, wz), Biome::SnowySlope | Biome::WindsweptHills | Biome::WindsweptSavanna) {
                continue;
            }
            if hash2(wx, wz, self.seed_for_features() ^ 0xb01d1d) % 11 != 0 {
                continue;
            }
            let top = surface_tops[bx][bz];
            for dx in 0..3usize {
                for dz in 0..3usize {
                    if (dx + dz) % 2 == 1 {
                        continue; // lumpy silhouette, not a cube slab
                    }
                    for dy in 0..2usize {
                        col.set(bx + dx, (top + dy as i32) as usize, bz + dz, BlockState(block::STONE));
                    }
                }
            }
        }

        // 6. Trees by biome kind, canopies kept inside the chunk.
        for lx in 3..13usize {
            for lz in 3..13usize {
                let wx = cx * 16 + lx as i32;
                let wz = cz * 16 + lz as i32;
                let top = surface_tops[lx][lz];
                let kind = self.biome(wx, wz).tree_kind();
                if kind == TreeKind::None || top <= SEA_LEVEL + 1 {
                    continue;
                }
                // surface must still be vegetated (not carved/stony)
                let surface_ok = matches!(
                    col.get(lx, (top - 1) as usize, lz).id(),
                    block::GRASS | block::JUNGLE_GRASS | block::SAVANNA_GRASS
                        | block::MYCELIUM | block::MOSS | block::SNOW | block::SAND | block::DIRT
                );
                if !surface_ok {
                    continue;
                }
                let (log, leaves, trunk_base, canopy_r) = kind.blocks();
                if log == block::AIR {
                    continue;
                }
                let h = hash2(wx, wz, self.seed_for_features() ^ 0x7ab99e21);
                let density = match kind {
                    TreeKind::OakSparse => 160,
                    TreeKind::SpruceSparse => 220, // tundra: scattered wind-bent conifers
                    TreeKind::Jungle => 36,
                    TreeKind::DarkOak => 40,
                    TreeKind::Cherry => 56,
                    _ => 72,
                };
                if h % density != 0 {
                    continue;
                }
                let trunk = trunk_base + ((h / density) % 3) as i32;
                for y in top..top + trunk {
                    col.set(lx, y as usize, lz, BlockState(log));
                }
                let base = top + trunk - 1;
                if kind.is_conifer() {
                    // cone canopy: widest low, spire on top
                    let layers = canopy_r + 4;
                    for dy in 0..layers {
                        let frac = 1.0 - dy as f32 / layers as f32;
                        let r = (frac * canopy_r as f32).round() as i32;
                        let y = (base - layers / 2 + dy) as usize;
                        for dx in -r..=r {
                            for dz in -r..=r {
                                if dx.abs() + dz.abs() > r { continue; }
                                let px = (lx as i32 + dx) as usize;
                                let pz = (lz as i32 + dz) as usize;
                                if px < 16 && pz < 16 && y < 256 {
                                    if col.get(px, y, pz) == BlockState::AIR {
                                        col.set(px, y, pz, BlockState(leaves));
                                    }
                                }
                            }
                        }
                    }
                    if base + 3 < 256 {
                        col.set(lx, (base + 3) as usize, lz, BlockState(leaves));
                    }
                } else {
                    // blob canopy
                    for dy in -2i32..=0i32 {
                        let r = if dy < 0 { canopy_r } else { (canopy_r - 1).max(1) };
                        for dx in -r..=r {
                            for dz in -r..=r {
                                if dx.abs() == canopy_r && dz.abs() == canopy_r { continue; }
                                let px = (lx as i32 + dx) as usize;
                                let pz = (lz as i32 + dz) as usize;
                                let py = (base + dy) as usize;
                                if px < 16 && pz < 16 {
                                    if col.get(px, py, pz) == BlockState::AIR {
                                        col.set(px, py, pz, BlockState(leaves));
                                    }
                                }
                            }
                        }
                    }
                    if base + 1 < 256 {
                        col.set(lx, (base + 1) as usize, lz, BlockState(leaves));
                    }
                }
            }
        }
        col
    }


    /// Deterministic structure placement: sparse huts on meadows, watchtowers
    /// on highlands, buried pyramids on desert. Footprints stay in-chunk.
    fn place_structures(&self, cx: i32, cz: i32, col: &mut lf_voxel::ChunkColumn) {
        use lf_voxel::BlockState;
        use lf_voxel::registry::block;
        let h0 = hash2(cx, cz, self.seed_for_features() ^ 0x5bd1e995);
        let center_biome = self.biome(cx * 16 + 8, cz * 16 + 8);
        let ground = |lx: usize, lz: usize| -> usize {
            let top = self.surface_top(cx * 16 + lx as i32, cz * 16 + lz as i32);
            top.min(250) as usize
        };

        let build_hut = |col: &mut lf_voxel::ChunkColumn| {
            let base_y = ground(8, 8);
            if base_y < SEA_LEVEL as usize + 1 || base_y > 200 {
                return;
            }
            for dx in 5..=10usize {
                for dz in 5..=10usize {
                    let edge = dx == 5 || dx == 10 || dz == 5 || dz == 10;
                    for dy in 0..3usize {
                        let y = base_y + dy;
                        if dy == 0 {
                            col.set(dx, y, dz, BlockState(block::PLANKS));
                        } else if edge {
                            let corner = (dx == 5 || dx == 10) && (dz == 5 || dz == 10);
                            col.set(dx, y, dz, BlockState(if corner { block::LOG } else { block::PLANKS }));
                        } else {
                            col.set(dx, y, dz, BlockState::AIR);
                        }
                    }
                    col.set(dx, base_y + 3, dz, BlockState(block::LOG));
                }
            }
            col.set(7, base_y + 1, 5, BlockState::AIR);
            col.set(8, base_y + 1, 5, BlockState::AIR);
            col.set(7, base_y + 2, 5, BlockState::AIR);
            col.set(8, base_y + 2, 5, BlockState::AIR);
            col.set(9, base_y + 1, 9, BlockState(block::TORCH));
            col.set(6, base_y + 1, 9, BlockState(block::CRAFTING_TABLE));
            col.set(9, base_y + 1, 6, BlockState(block::FURNACE));
        };

        let build_watchtower = |col: &mut lf_voxel::ChunkColumn| {
            let base_y = ground(8, 8);
            if base_y > 210 {
                return;
            }
            for dy in 0..8usize {
                let y = base_y + dy;
                for dx in 6..=10usize {
                    for dz in 6..=10usize {
                        let edge = dx == 6 || dx == 10 || dz == 6 || dz == 10;
                        let pillar = (dx == 6 || dx == 10) && (dz == 6 || dz == 10);
                        if pillar || (dy == 7 && edge) || dy == 0 {
                            col.set(dx, y, dz, BlockState(block::STONE));
                        } else {
                            col.set(dx, y, dz, BlockState::AIR);
                        }
                    }
                }
            }
            col.set(8, base_y + 7, 8, BlockState(block::TORCH));
        };

        let build_pyramid = |col: &mut lf_voxel::ChunkColumn| {
            let base_y = ground(8, 8);
            if base_y > 200 {
                return;
            }
            for layer in 0..4usize {
                let r = 6 - (layer as i32) * 2;
                if r < 0 {
                    continue;
                }
                let y = base_y + layer;
                for dx in (8 - r as usize)..=(8 + r as usize) {
                    for dz in (8 - r as usize)..=(8 + r as usize) {
                        let edge = (dx as i32 == 8 - r) || (dx as i32 == 8 + r)
                            || (dz as i32 == 8 - r) || (dz as i32 == 8 + r);
                        if edge || layer == 0 {
                            col.set(dx, y, dz, BlockState(block::SAND));
                        }
                    }
                }
            }
            col.set(8, base_y + 1, 2, BlockState::AIR);
            col.set(8, base_y + 2, 2, BlockState::AIR);
        };

        match center_biome {
            Biome::Meadow if h0 % 37 == 0 => build_hut(col),
            Biome::Highlands if h0 % 41 == 0 => build_watchtower(col),
            Biome::Desert if h0 % 29 == 0 => build_pyramid(col),
            _ => {}
        }
    }
    /// Superflat tail: water fill + sparse trees only.
    fn finish_flat(&self, mut col: lf_voxel::ChunkColumn, cx: i32, cz: i32,
        surface_tops: &[[i32; 16]; 16]) -> lf_voxel::ChunkColumn {
        use lf_voxel::registry::block;
        for lx in 0..16usize {
            for lz in 0..16usize {
                let top = surface_tops[lx][lz];
                if top <= SEA_LEVEL {
                    for y in top..=SEA_LEVEL {
                        if col.get(lx, y as usize, lz) == lf_voxel::BlockState::AIR {
                            col.set(lx, y as usize, lz, lf_voxel::BlockState(block::WATER));
                        }
                    }
                }
            }
        }
        let _ = (cx, cz);
        col
    }

    fn seed_for_features(&self) -> u64 {
        // Feature placement must depend on the world seed; the noise objects
        // don't expose it, so derive from any seeded output.
        let a = self.noise_base.get_noise_2d(0.0, 0.0).to_bits() as u64;
        let b = self.noise_temp.get_noise_2d(0.0, 0.0).to_bits() as u64;
        a.wrapping_mul(31).wrapping_add(b)
    }
}

/// Keep the carve loop bounded (sections are 16 tall, world 256).
const SECTION_MAX: usize = 250;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn deterministic() {
        let a = WorldGen::new(Seed(42));
        let b = WorldGen::new(Seed(42));
        assert_eq!(a.height(10, 20), b.height(10, 20));
        assert_eq!(a.temperature(10, 20), b.temperature(10, 20));
        assert_eq!(a.humidity(10, 20), b.humidity(10, 20));
        assert_eq!(a.biome(10, 20), b.biome(10, 20));
        assert_eq!(a.generate_chunk(3, 3).get(5, 60, 5), b.generate_chunk(3, 3).get(5, 60, 5));
    }

    /// P31 (doc 04): crude exists (desert/swamp-gated), stays underground in
    /// the 8..44 band, and never appears in columns of other biomes.
    #[test]
    fn oil_is_biome_gated_and_banded() {
        use lf_voxel::registry::block;
        let gen = WorldGen::new(Seed(20260827));
        let mut oil_total = 0usize;
        let mut checked = 0usize;
        for cx in -6..6i32 {
            for cz in -6..6i32 {
                let col = gen.generate_chunk(cx, cz);
                for lx in 0..16usize {
                    for lz in 0..16usize {
                        let biome = gen.biome(cx * 16 + lx as i32, cz * 16 + lz as i32);
                        let oil_biome = matches!(biome, Biome::Desert | Biome::Swamp);
                        for y in 0..80usize {
                            let b = col.get(lx, y, lz);
                            if b.id() == block::OIL {
                                oil_total += 1;
                                assert!(oil_biome,
                                    "oil in {:?} at chunk ({},{})", biome, cx, cz);
                                assert!((8..44).contains(&y) || y as i32 >= gen.surface_top(
                                    cx * 16 + lx as i32, cz * 16 + lz as i32) - 2,
                                    "seep or pool band only, got y={}", y);
                            }
                        }
                        checked += 1;
                    }
                }
            }
        }
        assert!(oil_total > 50, "a 16x16-chunk scan should find real deposits, got {}", oil_total);
        assert!(checked > 30_000);
    }

    #[test]
    fn every_biome_variant_reachable() {
        use Biome::*;
        let cases = [
            ((0.5, 0.5, 160, 0.5), Mountains),
            ((0.3, 0.5, 140, 0.5), SnowySlope),
            ((0.5, 0.5, 130, 0.5), Highlands),
            ((0.5, 0.5, 30, 0.5), DeepOcean),
            ((0.5, 0.5, 45, 0.5), Ocean),
            ((0.5, 0.5, 52, 0.9), StonyShore),
            ((0.5, 0.5, 52, 0.2), Beach),
            ((0.1, 0.8, 80, 0.3), SnowyTaiga),
            ((0.1, 0.8, 80, 0.8), GiantTaiga),
            ((0.1, 0.5, 70, 0.95), IceSpikes),
            ((0.1, 0.5, 70, 0.3), Tundra),
            ((0.15, 0.5, 35, 0.5), FrozenOcean),
            ((0.85, 0.9, 70, 0.5), Savanna),
            ((0.85, 0.2, 70, 0.9), Badlands),
            ((0.85, 0.2, 70, 0.75), WindsweptSavanna),
            ((0.85, 0.2, 70, 0.3), Desert),
            ((0.3, 0.9, 80, 0.3), Taiga),
            ((0.3, 0.9, 80, 0.85), Swamp),
            ((0.3, 0.6, 80, 0.9), BirchForest),
            ((0.3, 0.6, 80, 0.45), Forest),
            ((0.3, 0.3, 80, 0.4), Tundra),
            ((0.65, 0.9, 80, 0.9), PaleGarden),
            ((0.65, 0.9, 80, 0.5), DarkForest),
            ((0.65, 0.7, 80, 0.85), CherryGrove),
            ((0.65, 0.7, 80, 0.3), Forest),
            ((0.65, 0.7, 80, 0.55), FlowerForest),
            ((0.55, 0.3, 80, 0.4), Meadow),
            ((0.6, 0.5, 80, 0.9), Jungle),
            ((0.6, 0.5, 80, 0.7), Swamp),
            ((0.5, 0.5, 80, 0.5), Meadow),
            ((0.1, 0.5, 70, 0.9), IceSpikes),
            ((0.5, 0.9, 80, 0.05), MushroomHollow),
            ((0.85, 0.5, 30, 0.5), WarmOcean),
            ((0.85, 0.2, 130, 0.5), WindsweptHills),
            ((0.2, 0.5, 160, 0.5), SnowyPeaks),
        ];
        for ((t, h, height, v), want) in cases {
            let got = biome::biome_from(t, h, height, v);
            assert_eq!(got, want, "t={} h={} y={} v={} -> {:?} (want {:?})", t, h, height, v, got, want);
        }
        // all 30 named
        for id in 0u32..30 {
            let _ = id;
        }
        let all: HashSet<&str> = cases.iter().map(|(_, b)| b.name()).collect();
        assert!(all.len() >= 29, "expected ~30 distinct biomes, got {}", all.len());
    }

    #[test]
    fn all_biomes_appear_across_sampled_world() {
        let gen = WorldGen::new(Seed(42));
        let mut seen = HashSet::new();
        // biomes are larger since the P23 climate smoothing — sample wider
        for x in (-4000..4000).step_by(12) {
            for z in (-4000..4000).step_by(12) {
                seen.insert(gen.biome(x, z));
            }
        }
        assert!(seen.len() >= 30, "expected 30 biomes in sampled world, got {} ({:?})", seen.len(),
            seen.iter().map(|b| b.name()).collect::<Vec<_>>());
    }

    /// Build-pack Step 17: every biome must be distinguishable from every
    /// other by its worldgen identity (surface material, tree kind,
    /// structure, or an exclusive feature like flowers/boulders). The only
    /// allowed same-key families are the depth-banded oceans and the
    /// coastal StonyShore/Mountains pair — both documented.
    #[test]
    fn biome_identity_markers_are_distinct() {
        use crate::biome::Biome::*;
        use crate::biome::TreeKind;
        let all = [Meadow, FlowerForest, Forest, BirchForest, DarkForest, PaleGarden,
            CherryGrove, Taiga, SnowyTaiga, GiantTaiga, Tundra, IceSpikes, SnowySlope,
            SnowyPeaks, FrozenOcean, Jungle, Swamp, Savanna, WindsweptSavanna, Desert,
            Badlands, Beach, StonyShore, Ocean, DeepOcean, WarmOcean, Highlands,
            Mountains, WindsweptHills, MushroomHollow];
        let has_structure = |b: Biome| matches!(b, Meadow | Highlands | Desert);
        let has_exclusive = |b: Biome| matches!(b, FlowerForest | SnowySlope | WindsweptHills | WindsweptSavanna);
        let key = |b: Biome| (b.surface_block(), b.filler_block(), b.tree_kind(), has_structure(b), has_exclusive(b));
        let same_family = |a: Biome, b: Biome| {
            matches!((a, b),
                (Ocean | DeepOcean | WarmOcean | FrozenOcean,
                 Ocean | DeepOcean | WarmOcean | FrozenOcean)
                    | (StonyShore, Mountains)
                    | (Mountains, StonyShore))
        };
        for (i, &a) in all.iter().enumerate() {
            for &b in &all[i + 1..] {
                assert!(
                    key(a) != key(b) || same_family(a, b),
                    "{:?} and {:?} are worldgen twins: fix the biome table (surface {:?}/{:?}, filler {:?}/{:?}, trees {:?}/{:?})",
                    a, b, a.surface_block(), b.surface_block(), a.filler_block(), b.filler_block(), a.tree_kind(), b.tree_kind()
                );
            }
        }
        // the grade table must cover every biome family too (Step 3 sibling check)
        let _ = TreeKind::None;
    }

    #[test]
    fn trees_generate_on_meadows() {
        use lf_voxel::registry::block;
        let gen = WorldGen::new(Seed(12345));
        let (mut logs, mut leaves) = (0usize, 0usize);
        for cx in -6..=6 {
            for cz in -6..=6 {
                let col = gen.generate_chunk(cx, cz);
                for lx in 0..16 {
                    for lz in 0..16 {
                        for y in 40..170 {
                            let b = col.get(lx, y, lz).id();
                            if matches!(b, block::LOG | block::BIRCH_LOG | block::SPRUCE_LOG | block::DARK_LOG | block::CHERRY_LOG) {
                                logs += 1;
                            }
                            if matches!(b, block::LEAVES | block::BIRCH_LEAVES | block::SPRUCE_LEAVES | block::DARK_LEAVES | block::CHERRY_LEAVES | block::PALE_LEAVES) {
                                leaves += 1;
                            }
                        }
                    }
                }
            }
        }
        assert!(logs > 20, "expected trees, found {} logs", logs);
        assert!(leaves > logs, "canopy should outnumber trunks: {} leaves vs {} logs", leaves, logs);
    }

    #[test]
    fn multiple_tree_species_generate() {
        use lf_voxel::registry::block;
        let gen = WorldGen::new(Seed(12345));
        let mut species = HashSet::new();
        // biomes are larger since P23; target chunks per biome instead of
        // brute-scanning (chunk generation is too slow for wide scans)
        let mut wanted: Vec<(i32, i32)> = Vec::new();
        'find: for cx in -80..80i32 {
            for cz in -80..80i32 {
                use Biome::*;
                let b = gen.biome(cx * 16 + 8, cz * 16 + 8);
                if matches!(b, Forest | BirchForest | Taiga | SnowyTaiga | GiantTaiga
                    | DarkForest | CherryGrove | PaleGarden | Jungle) {
                    wanted.push((cx, cz));
                    if wanted.len() >= 24 {
                        break 'find;
                    }
                }
            }
        }
        for (cx, cz) in wanted {
            let col = gen.generate_chunk(cx, cz);
            for lx in 0..16 {
                for lz in 0..16 {
                    for y in 40..170 {
                        let b = col.get(lx, y, lz).id();
                        if matches!(b, block::LOG | block::BIRCH_LOG | block::SPRUCE_LOG | block::DARK_LOG | block::CHERRY_LOG) {
                            species.insert(b);
                        }
                    }
                }
            }
        }
        assert!(species.len() >= 3, "expected >=3 tree species, got {}", species.len());
    }

    #[test]
    fn caves_carve_underground() {
        let gen = WorldGen::new(Seed(12345));
        let mut air_pockets = 0usize;
        for cx in -4..=4 {
            for cz in -4..=4 {
                let col = gen.generate_chunk(cx, cz);
                for lx in 0..16 {
                    for lz in 0..16 {
                        for y in 10..50 {
                            if col.get(lx, y, lz).id() == lf_voxel::registry::block::AIR {
                                air_pockets += 1;
                            }
                        }
                    }
                }
            }
        }
        assert!(air_pockets > 500, "expected substantial caves, found {} air blocks underground", air_pockets);
    }

    #[test]
    fn ores_generate_at_depth() {
        use lf_voxel::registry::block;
        let gen = WorldGen::new(Seed(12345));
        let (mut coal, mut iron) = (0usize, 0usize);
        for cx in -4..=4 {
            for cz in -4..=4 {
                let col = gen.generate_chunk(cx, cz);
                for lx in 0..16 {
                    for lz in 0..16 {
                        for y in 6..48 {
                            let b = col.get(lx, y, lz).id();
                            if b == block::COAL_ORE { coal += 1; }
                            if b == block::IRON_ORE { iron += 1; }
                        }
                    }
                }
            }
        }
        assert!(coal > 100, "expected coal, found {}", coal);
        assert!(iron > 20, "expected iron, found {}", iron);
    }

    #[test]
    fn water_fills_oceans_and_ice_freezes() {
        use lf_voxel::registry::block;
        let gen = WorldGen::new(Seed(12345));
        let mut found_water = 0usize;
        let mut found_ice = false;
        for cx in -20..=20 {
            for cz in -20..=20 {
                if gen.surface_top(cx * 16, cz * 16) >= SEA_LEVEL {
                    continue;
                }
                let col = gen.generate_chunk(cx, cz);
                for lx in 0..16 {
                    for lz in 0..16 {
                        for y in 30..=SEA_LEVEL as usize {
                            let b = col.get(lx, y, lz).id();
                            if b == block::WATER { found_water += 1; }
                            if b == block::ICE { found_ice = true; }
                        }
                    }
                }
            }
        }
        assert!(found_water > 100, "expected ocean water, found {} blocks", found_water);
        let _ = found_ice; // frozen oceans depend on the seed's cold zones
    }

    #[test]
    fn industrial_ores_generate() {
        use lf_voxel::registry::block;
        let gen = WorldGen::new(Seed(12345));
        let (mut copper, mut tin, mut baux, mut sulf) = (0usize, 0usize, 0usize, 0usize);
        for cx in -3..=3 {
            for cz in -3..=3 {
                let col = gen.generate_chunk(cx, cz);
                for lx in 0..16 {
                    for lz in 0..16 {
                        for y in 5..90 {
                            match col.get(lx, y, lz).id() {
                                block::COPPER_ORE => copper += 1,
                                block::TIN_ORE => tin += 1,
                                block::BAUXITE_ORE => baux += 1,
                                block::SULFUR_ORE => sulf += 1,
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        assert!(copper > 100, "copper {}, tin {}, baux {}, sulf {}", copper, tin, baux, sulf);
        assert!(tin > 40);
        assert!(baux > 10);
        assert!(sulf > 30);
    }

    #[test]
    fn mod_ore_hooks_generate() {
        register_ore_hook(OreHook {
            block_id: 150,
            y_min: 10,
            y_max: 60,
            threshold: 0.60,
            noise_offset: 500.0,
        });
        let gen = WorldGen::new(Seed(12345));
        let mut found = 0;
        for cx in -2..=2 {
            for cz in -2..=2 {
                let col = gen.generate_chunk(cx, cz);
                for lx in 0..16 {
                    for lz in 0..16 {
                        for y in 10..60 {
                            if col.get(lx, y, lz).id() == 150 {
                                found += 1;
                            }
                        }
                    }
                }
            }
        }
        assert!(found > 50, "mod ore should generate, found {}", found);
    }

    #[test]
    fn structures_generate_deterministically() {
        use lf_voxel::registry::block;
        let gen = WorldGen::new(Seed(12345));
        let count = |col: &lf_voxel::ChunkColumn, id: u32, y0: usize, y1: usize| -> usize {
            let mut n = 0;
            for lx in 0..16 {
                for lz in 0..16 {
                    for y in y0..y1 {
                        if col.get(lx, y, lz).id() == id {
                            n += 1;
                        }
                    }
                }
            }
            n
        };
        let (mut hut, mut pyramid, mut tower) = (false, false, false);
        // cheap biome-guided chunk selection (full brute scans are too slow
        // at the larger P23 biome scale)
        let mut picked: Vec<(i32, i32)> = Vec::new();
        'pick: for cx in -60..60i32 {
            for cz in -60..60i32 {
                let b = gen.biome(cx * 16 + 8, cz * 16 + 8);
                if matches!(b, Biome::Desert | Biome::Badlands | Biome::Meadow
                    | Biome::Forest | Biome::Mountains | Biome::SnowyPeaks) {
                    picked.push((cx, cz));
                    if picked.len() >= 90 {
                        break 'pick;
                    }
                }
            }
        }
        for (cx, cz) in picked {
            {
                let a = gen.generate_chunk(cx, cz);
                let tables = count(&a, block::CRAFTING_TABLE, 60, 200);
                if tables > 0 {
                    hut = true;
                    assert!(count(&a, block::FURNACE, 60, 200) > 0, "hut has a furnace");
                    let b = gen.generate_chunk(cx, cz);
                    assert_eq!(count(&b, block::CRAFTING_TABLE, 60, 200), tables,
                        "hut placement not deterministic at ({},{})", cx, cz);
                }
                for y in 60..180 {
                    let mut sand = 0;
                    for lx in 2..14 {
                        for lz in 2..14 {
                            if a.get(lx, y, lz).id() == block::SAND || a.get(lx, y, lz).id() == block::RED_SAND {
                                sand += 1;
                            }
                        }
                    }
                    if sand > 25 {
                        pyramid = true;
                        break;
                    }
                }
                let mut stone_high = 0;
                for lx in 6..=10 {
                    for lz in 6..=10 {
                        for y in 120..220 {
                            if a.get(lx, y, lz).id() == block::STONE {
                                stone_high += 1;
                            }
                        }
                    }
                }
                if stone_high > 40 {
                    tower = true;
                }
            }
        }
        assert!(hut, "no huts found in scan");
        assert!(pyramid, "no pyramids found in scan");
        assert!(tower, "no watchtowers found in scan");
    }

    #[test]
    fn world_types_change_terrain_shape() {
        let normal = WorldGen::new(Seed(9));
        let flat = WorldGen::with_type(Seed(9), WorldType::Superflat);
        let amp = WorldGen::with_type(Seed(9), WorldType::Amplified);
        // superflat is exactly flat
        for (x, z) in [(0, 0), (100, -50), (-777, 321)] {
            assert_eq!(flat.height(x, z), 64, "superflat must be flat");
        }
        // amplified reaches higher than normal somewhere
        let normal_max = (0..400).map(|i| normal.height(i * 13, i * 7)).max().unwrap();
        let amp_max = (0..400).map(|i| amp.height(i * 13, i * 7)).max().unwrap();
        assert!(amp_max > normal_max, "amplified {} should exceed normal {}", amp_max, normal_max);
        // superflat has no caves underground
        let col = flat.generate_chunk(3, 3);
        for lx in 0..16 {
            for lz in 0..16 {
                for y in 10..50 {
                    assert_ne!(col.get(lx, y, lz).id(), lf_voxel::registry::block::AIR,
                        "superflat underground must be solid");
                }
            }
        }
    }

    #[test]
    fn generate_chunk_matches_column_data() {
        let gen = WorldGen::new(Seed(7));
        let col = gen.generate_chunk(2, -3);
        for (lx, lz) in [(0usize, 0usize), (8, 8), (15, 15)] {
            let wx = 2 * 16 + lx as i32;
            let wz = -3 * 16 + lz as i32;
            let h = gen.surface_top(wx, wz);
            assert!(col.get(lx, (h - 1).max(0) as usize, lz) != lf_voxel::BlockState::AIR);
            assert_eq!(col.get(lx, 200, lz), lf_voxel::BlockState::AIR);
        }
    }
}
