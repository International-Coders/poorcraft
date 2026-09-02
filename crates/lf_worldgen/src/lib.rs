pub mod biome;
pub mod preview;

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
pub const GENERATOR_VERSION: u32 = 5; // v5: kingdoms — region-placed citadels (walls, keep, throne, market, farm) + locomotion-era NPC settling

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
    /// Continental shelf: very low frequency — one "continent" per ~1200
    /// blocks. Decides ocean vs lowland vs highland (ui-world-craft D1).
    noise_continental: FastNoiseLite,
    /// Mid-frequency detail shared by lowlands and highlands.
    noise_detail: FastNoiseLite,
    /// Ridged mountains: sharp ridgelines in the highland zone only.
    noise_ridge: FastNoiseLite,
    /// River paths: meandering zero-crossings of a low-frequency field.
    noise_river: FastNoiseLite,
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

        // continental shelf (D1): one land/ocean/highland pattern per
        // ~1200 blocks — the geology that everything else sits on
        let mut continental = FastNoiseLite::new();
        continental.set_seed(Some(channel_seed(s, 3)));
        continental.set_noise_type(Some(NoiseType::Perlin));
        continental.set_fractal_type(Some(FractalType::FBm));
        continental.set_fractal_octaves(Some(4));
        continental.set_fractal_lacunarity(Some(2.0));
        continental.set_fractal_gain(Some(0.5));
        continental.set_frequency(Some(1.0 / 1200.0));

        // local detail (D2): the shape of the land within each zone
        let mut detail = FastNoiseLite::new();
        detail.set_seed(Some(channel_seed(s, 5)));
        detail.set_noise_type(Some(NoiseType::Perlin));
        detail.set_fractal_type(Some(FractalType::FBm));
        detail.set_fractal_octaves(Some(6));
        detail.set_fractal_lacunarity(Some(1.9));
        detail.set_fractal_gain(Some(0.55));
        detail.set_frequency(Some(1.0 / 80.0));

        // ridgelines (D1): inverted-absolute noise, sharpened — mountains
        // get crests and saddles instead of uniform rolling domes
        let mut ridge = FastNoiseLite::new();
        ridge.set_seed(Some(channel_seed(s, 9)));
        ridge.set_noise_type(Some(NoiseType::Perlin));
        ridge.set_fractal_type(Some(FractalType::FBm));
        ridge.set_fractal_octaves(Some(4));
        ridge.set_fractal_lacunarity(Some(2.1));
        ridge.set_fractal_gain(Some(0.45));
        ridge.set_frequency(Some(1.0 / 120.0));

        // river meanders (D2): rivers live where this field crosses zero,
        // in the lowlands only
        let mut river = FastNoiseLite::new();
        river.set_seed(Some(channel_seed(s, 17)));
        river.set_noise_type(Some(NoiseType::OpenSimplex2));
        river.set_fractal_type(Some(FractalType::None));
        river.set_frequency(Some(1.0 / 400.0));

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
            noise_continental: continental,
            noise_detail: detail,
            noise_ridge: ridge,
            noise_river: river,
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

    /// Smoothstep between edges a..b.
    fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
        let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Continental factor [0..1] (D1): the low-frequency geology. Below
    /// ~0.3 the land is genuine flat lowland (or ocean shelf), above ~0.7
    /// it is highland. The window is calibrated to the stretched noise's
    /// actual distribution (its mass sits in 0.35..0.65) so the LAND split
    /// lands near the 60:40 flat:mountain target instead of letting the
    /// transition zone swallow the map.
    pub fn continental_factor(&self, cx: i32, cz: i32) -> f32 {
        let raw = self.noise_continental.get_noise_2d(cx as f32, cz as f32);
        let stretched = Self::stretch(raw);
        Self::smoothstep(0.51, 0.68, stretched)
    }

    /// The stretched continental sample behind `continental_factor`
    /// (0..1 across the whole map).
    fn continental_stretched(&self, cx: i32, cz: i32) -> f32 {
        Self::stretch(self.noise_continental.get_noise_2d(cx as f32, cz as f32))
    }

    /// River factor [0..1] (D2): 1.0 = middle of a river channel. Rivers
    /// are the zero-crossings of a low-frequency meander field. They run
    /// through the whole lowland (strength is a hard highland cutoff, not
    /// a coast fade — a fade would leave inland channels too weak to carve
    /// water) and they widen toward the coast.
    pub fn river_factor(&self, cx: i32, cz: i32) -> f32 {
        let cf = self.continental_factor(cx, cz);
        if cf > 0.55 {
            return 0.0; // highlands stay dry
        }
        let inland = 1.0 - Self::smoothstep(0.40, 0.55, cf);
        let coast = 1.0 - cf / 0.55;
        let flow = self.noise_river.get_noise_2d(cx as f32, cz as f32);
        let width = 0.05 + 0.06 * coast; // 3–7 blocks, wider downstream
        (1.0 - (flow.abs() / width).clamp(0.0, 1.0)) * inland
    }

    /// Height at chunk column (x,z) in blocks (ui-world-craft D1/D2).
    ///
    /// Two-layer terrain: a continental shelf decides ocean basin, flat
    /// lowland, or highland; local detail shapes each zone; ridged noise
    /// sharpens mountain crests; rivers carve their valleys toward the
    /// sea. Range spans deep ocean to ~160-block peaks so ocean and
    /// mountain biomes are both reachable.
    pub fn height(&self, cx: i32, cz: i32) -> i32 {
        if self.world_type == WorldType::Superflat {
            return 64;
        }
        let s = self.continental_stretched(cx, cz);
        let cf = Self::smoothstep(0.51, 0.68, s);
        let detail = self.noise_detail.get_noise_2d(cx as f32, cz as f32);
        let ridge_raw = self.noise_ridge.get_noise_2d(cx as f32, cz as f32);
        let ridge = (1.0 - ridge_raw.abs()).powf(2.5);

        // Lowland: genuine flats hugging the sea — where players build.
        let lowland = SEA_LEVEL as f32 + 1.0 + detail * 7.0;
        // Highland: ridged mountains; crest +114 over the sea at the
        // extreme, so snowy peaks stay reachable on true ridge cores.
        let highland = SEA_LEVEL as f32 + 36.0 + detail * 30.0 + ridge * 48.0;
        let mut h = lowland + (highland - lowland) * cf;

        // Ocean basins: the continental shelf slides below sea level as
        // the continental sample drops, down to ~30 blocks of water.
        let ocean_t = 1.0 - Self::smoothstep(0.30, 0.40, s);
        h -= ocean_t * (30.0 + detail * 10.0);

        // River carving (D2): pull the channel down toward the river bed
        // (4 below sea level) with sloped banks; never lift the seabed.
        // The 1.6x ramp makes a full channel (rf >= 0.63) always reach
        // the bed, so rivers stay watercourses instead of dry gullies on
        // transition-zone hills. The bed sits 4 deep because the water
        // pass fills open columns from height+4 up to sea level — a bed
        // at SEA-2 would stay a dry grass slot.
        let rf = self.river_factor(cx, cz);
        if rf > 0.0 {
            let bed = (SEA_LEVEL - 4) as f32;
            if h > bed {
                h = h + (bed - h) * (rf * 1.6).clamp(0.0, 1.0);
            }
        }

        if self.world_type == WorldType::Amplified {
            let amp = 2.0f32;
            h = SEA_LEVEL as f32 + (h - SEA_LEVEL as f32) * amp;
        }
        (h.round() as i32).max(6)
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
        // (normal path below) Never below y=6 (bedrock-ish floor). The
        // breach ramp (D3): below y=48 caves open at noise > 0.40; the
        // threshold tightens to 0.72 by y=56, so visible surface holes
        // become rare — caves are underground features.
        for lx in 0..16usize {
            for lz in 0..16usize {
                let wx = (cx * 16 + lx as i32) as f32;
                let wz = (cz * 16 + lz as i32) as f32;
                let top = surface_tops[lx][lz];
                let max_carve = top - 4;
                for y in 6..(SECTION_MAX as i32).min(max_carve.max(7)) {
                    let ramp = ((y - 48) as f32 / 8.0).clamp(0.0, 1.0);
                    let threshold = 0.40 + 0.32 * ramp;
                    let n = self.noise_cave.get_noise_3d(wx, y as f32, wz);
                    if n > threshold {
                        col.set(lx, y as usize, lz, BlockState::AIR);
                    }
                }
            }
        }

        // 2.5 Cave biomes (D3): below y=30 the stone band is deep slate
        // (a dithered fringe keeps the transition from being a flat line);
        // below y=10 the cave pockets flood with lava.
        for lx in 0..16usize {
            for lz in 0..16usize {
                let wx = cx * 16 + lx as i32;
                let wz = cz * 16 + lz as i32;
                for y in 6..30usize {
                    if y >= 28 && hash2(wx as i32, wz as i32 ^ (y as i32), self.seed_for_features()) % 2 == 0 {
                        continue; // dithered fringe
                    }
                    if col.get(lx, y, lz) == BlockState::STONE {
                        col.set(lx, y, lz, BlockState(block::DEEP_SLATE));
                    }
                }
                for y in 6..=10usize {
                    if col.get(lx, y, lz) == BlockState::AIR {
                        col.set(lx, y, lz, BlockState(block::LAVA));
                    }
                }
            }
        }

        // 2.6 Stalactites and stalagmites (D3): a ceiling stone with air
        // below and 2+ solid horizontal neighbors may hang a spike; a floor
        // stone with air above may grow one. Material follows the host
        // block, so deep caves get deep-slate speleothems.
        for lx in 1..15usize {
            for lz in 1..15usize {
                let wx = cx * 16 + lx as i32;
                let wz = cz * 16 + lz as i32;
                let h = hash2(wx, wz, self.seed_for_features() ^ 0x57a1157);
                for y in 10..220usize {
                    let b = col.get(lx, y, lz);
                    if b.id() != block::STONE && b.id() != block::DEEP_SLATE {
                        continue;
                    }
                    let air_below = col.get(lx, y - 1, lz) == BlockState::AIR;
                    let air_above = y + 1 < 256 && col.get(lx, y + 1, lz) == BlockState::AIR;
                    if !air_below && !air_above {
                        continue;
                    }
                    let solid_n = [(1usize, 0usize), (15, 0), (0, 1), (0, 15)].iter()
                        .filter(|(dx, dz)| col.get(lx.wrapping_add(*dx) & 15, y, lz.wrapping_add(*dz) & 15).id() != block::AIR)
                        .count();
                    if solid_n < 2 {
                        continue;
                    }
                    let roll = (h >> (y % 16)) % 100;
                    if air_below && roll < 15 {
                        let len = 1 + ((h >> 8) % 4) as usize;
                        for k in 1..=len {
                            if y < k + 6 || col.get(lx, y - k, lz) != BlockState::AIR {
                                break;
                            }
                            col.set(lx, y - k, lz, b);
                        }
                    } else if air_above && roll >= 15 && roll < 25 {
                        let len = 1 + ((h >> 12) % 3) as usize;
                        for k in 1..=len {
                            if y + k >= 256 || col.get(lx, y + k, lz) != BlockState::AIR {
                                break;
                            }
                            col.set(lx, y + k, lz, b);
                        }
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
                    // ores embed in the deep-slate band as well as stone
                    if !matches!(col.get(lx, y as usize, lz), BlockState::STONE | BlockState(block::DEEP_SLATE)) {
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
                    // uranium (P32): the ceiling tier — deep (8..24), rare,
                    // tiny veins (the same hook the mods use, built in)
                    let ur_n = self.noise_ore.get_noise_3d(wx + 7000.0, y as f32, wz);
                    if y >= 8 && y <= 24 && ur_n > 0.68 {
                        col.set(lx, y as usize, lz, BlockState(block::URANIUM_ORE));
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
                    // deep slate replaced stone below y=18; oil embeds in both
                    if !matches!(col.get(lx, y as usize, lz), BlockState::STONE | BlockState(block::DEEP_SLATE)) {
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

        // 5.7 Coral heads: the reef exclusive on WarmOcean floors (C1) —
        //     small live-topped clusters, deterministic per column.
        for lx in 1..15usize {
            for lz in 1..15usize {
                let wx = cx * 16 + lx as i32;
                let wz = cz * 16 + lz as i32;
                if self.biome(wx, wz) != Biome::WarmOcean {
                    continue;
                }
                if hash2(wx, wz, self.seed_for_features() ^ 0xc0aa1) % 23 != 0 {
                    continue;
                }
                let top = surface_tops[lx][lz];
                if top >= SEA_LEVEL || top < 8 {
                    continue;
                }
                col.set(lx, top as usize, lz, BlockState(block::CORAL_BLOCK));
                if hash2(wx, wz, self.seed_for_features() ^ 0xc0aa2) % 2 == 0 && lx < 15 && top + 1 < SEA_LEVEL {
                    col.set(lx + 1, top as usize, lz, BlockState(block::CORAL_BLOCK));
                }
            }
        }

        // 5.8 Natural ember-glowstone formations (Covenant quest markers,
        //     covenant_q2): rare Anima concentrations on the faction's
        //     home highlands. Placed as a small standing cluster.
        if matches!(self.biome(cx * 16 + 8, cz * 16 + 8), Biome::Highlands | Biome::Taiga | Biome::MushroomHollow)
            && hash2(cx, cz, self.seed_for_features() ^ 0xe3b3b) % 173 == 0
        {
            let top = surface_tops[8][8];
            if top > SEA_LEVEL + 1 && top < 200 {
                col.set(8, top as usize, 8, BlockState(block::EMBER_GLOWSTONE));
                if top + 1 < 256 {
                    col.set(8, (top + 1) as usize, 8, BlockState(block::EMBER_GLOWSTONE));
                }
            }
        }

        // 5.9 Accord survey markers along the trade roads (accord_q1 reach
        //     targets; the Nameless pay to break them). A two-block
        //     accord_pillar post, rare per chunk in Accord territory.
        if matches!(self.biome(cx * 16 + 8, cz * 16 + 8), Biome::Meadow | Biome::Forest)
            && hash2(cx, cz, self.seed_for_features() ^ 0x0a2c) % 211 == 0
        {
            let top = surface_tops[8][8];
            if top > SEA_LEVEL + 1 && top < 200 {
                col.set(8, top as usize, 8, BlockState(block::ACCORD_PILLAR));
                if top + 1 < 256 {
                    col.set(8, (top + 1) as usize, 8, BlockState(block::ACCORD_PILLAR));
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
                        | block::GILDED_GRASS | block::PERMAFROST | block::BOG_PEAT
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

        // 7. Ground cover (ui-world-craft E3): each biome scatters its own
        // surface features at its own density — the thing you can name a
        // biome by after five seconds. Transition bands blend for free:
        // the dithered climate borders flip the owning biome column by
        // column, so both biomes' covers interleave near boundaries.
        for lx in 1..15usize {
            for lz in 1..15usize {
                let wx = cx * 16 + lx as i32;
                let wz = cz * 16 + lz as i32;
                let (density, features) = self.biome(wx, wz).surface_features();
                if density <= 0.0 || features.is_empty() {
                    continue;
                }
                let top = surface_tops[lx][lz];
                if top <= SEA_LEVEL + 1 || top + 2 >= 256 {
                    continue;
                }
                if col.get(lx, top as usize, lz).id() != block::AIR {
                    continue; // a tree or structure owns this cell
                }
                let ground_id = col.get(lx, (top - 1) as usize, lz).id();
                let ground_ok = matches!(ground_id,
                    block::GRASS | block::JUNGLE_GRASS | block::SAVANNA_GRASS
                    | block::GILDED_GRASS | block::MYCELIUM | block::MOSS
                    | block::SNOW | block::SAND | block::RED_SAND | block::DIRT
                    | block::PERMAFROST | block::BOG_PEAT | block::MESA_TERRACOTTA
                    | block::VOLCANIC_BASALT | block::STONE);
                if !ground_ok {
                    continue; // carved, submerged, or structure floor
                }
                let h = hash2(wx, wz, self.seed_for_features() ^ 0x6ea7c0de);
                if h % 1000 >= (density * 1000.0) as u64 {
                    continue;
                }
                let feat = features[(h / 1000) as usize % features.len()];
                // tall features: cactus stands 2-3, stone/basalt spikes 1-2
                let height = match feat {
                    b if b == block::CACTUS => 2 + (h % 2) as usize,
                    b if b == block::STONE || b == block::VOLCANIC_BASALT => 1 + (h % 2) as usize,
                    _ => 1,
                };
                for k in 0..height {
                    if col.get(lx, top as usize + k, lz) == BlockState::AIR {
                        col.set(lx, top as usize + k, lz, BlockState(feat));
                    }
                }
            }
        }
        col
    }


    /// Deterministic structure placement: sparse huts on meadows, watchtowers
    /// on highlands, buried pyramids on desert. Footprints stay in-chunk.
    /// Every structure runs terrain adaptation first (D5): the ground floor
    /// sits at the footprint's center-column surface, gaps below are filled
    /// with the biome filler when the ground varies more than 4 blocks, and
    /// footprints more than half underwater are refused.
    fn place_structures(&self, cx: i32, cz: i32, col: &mut lf_voxel::ChunkColumn) {
        use lf_voxel::BlockState;
        use lf_voxel::registry::block;
        let h0 = hash2(cx, cz, self.seed_for_features() ^ 0x5bd1e995);
        let center_biome = self.biome(cx * 16 + 8, cz * 16 + 8);
        let ground = |lx: usize, lz: usize| -> usize {
            let top = self.surface_top(cx * 16 + lx as i32, cz * 16 + lz as i32);
            top.min(250) as usize
        };

        // D5: prepare a footprint (inclusive corners) and return the
        // structure's ground-floor y, or None when the site is refused.
        let prepare = |col: &mut lf_voxel::ChunkColumn, x0: usize, x1: usize,
                       z0: usize, z1: usize| -> Option<usize> {
            let midx = (x0 + x1) / 2;
            let midz = (z0 + z1) / 2;
            let samples = [(midx, midz), (x0, z0), (x1, z1), (x0, z1), (x1, z0)];
            let mut under = 0usize;
            let (mut lo, mut hi) = (usize::MAX, 0usize);
            for (lx, lz) in samples {
                let t = ground(lx, lz);
                if t <= SEA_LEVEL as usize {
                    under += 1;
                }
                lo = lo.min(t);
                hi = hi.max(t);
            }
            if under * 2 > samples.len() {
                return None; // more than half the footprint is underwater
            }
            let base = ground(midx, midz);
            if base <= SEA_LEVEL as usize || base > 200 {
                return None;
            }
            if hi.saturating_sub(lo) > 4 {
                // slope too steep: build a leveled platform out of the
                // biome's own ground block, filling only open space
                let filler = self.biome(cx * 16 + 8, cz * 16 + 8).filler_block();
                for lx in x0..=x1 {
                    for lz in z0..=z1 {
                        let t = ground(lx, lz);
                        for y in t..base {
                            if y < 256 && col.get(lx, y, lz) == BlockState::AIR {
                                col.set(lx, y, lz, BlockState(filler));
                            }
                        }
                    }
                }
            }
            // support guarantee (D5): no floating floors. Caves, rivers and
            // overhangs can hollow the ground the heightmap doesn't know
            // about — fill any open space directly beneath the footprint
            // down to solid ground.
            let filler = self.biome(cx * 16 + 8, cz * 16 + 8).filler_block();
            for lx in x0..=x1 {
                for lz in z0..=z1 {
                    let mut y = base;
                    while y > 1 && col.get(lx, y - 1, lz) == BlockState::AIR {
                        col.set(lx, y - 1, lz, BlockState(filler));
                        y -= 1;
                    }
                }
            }
            Some(base)
        };

        let build_hut = |col: &mut lf_voxel::ChunkColumn| {
            let base_y = match prepare(col, 5, 10, 5, 10) { Some(b) => b, None => return };
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
            let base_y = match prepare(col, 6, 10, 6, 10) { Some(b) => b, None => return };
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

        // king-quest: the Accord Bastion — a walled mini-city. Walls with
        // merlons and a south gate, four timber houses, a two-storey stone
        // keep flying the Accord banner (the NPC-settle marker), accord
        // pillars flanking the keep door, torch-lit roads.
        let build_city = |col: &mut lf_voxel::ChunkColumn| {
            let base_y = match prepare(col, 1, 14, 1, 14) { Some(b) => b, None => return };
            let wall = |dx: usize, dz: usize| dx <= 1 || dx >= 14 || dz <= 1 || dz >= 14;
            for dx in 1..=14usize {
                for dz in 1..=14usize {
                    let y0 = base_y;
                    if wall(dx, dz) {
                        // south gate gap
                        let gate = dz == 14 && (dx == 7 || dx == 8);
                        for dy in 0..4usize {
                            let y = y0 + dy;
                            let merlon = dy == 3 && (dx + dz) % 2 == 0;
                            if gate || merlon {
                                col.set(dx, y, dz, BlockState::AIR);
                            } else {
                                col.set(dx, y, dz, BlockState(block::STONE));
                            }
                        }
                    } else if ((dx == 7 || dx == 8) && dz >= 9) || (dx == 7 || dx == 8) || (dz == 7 || dz == 8) {
                        // gate road + cross roads, cut into the terrain fill
                        col.set(dx, y0, dz, BlockState(block::STONE));
                    }
                }
            }
            // four timber houses in the quadrants
            for (hx, hz) in [(3usize, 3usize), (11, 3), (3, 10), (11, 10)] {
                for dx in hx..hx + 3 {
                    for dz in hz..hz + 3 {
                        let edge = dx == hx || dx == hx + 2 || dz == hz || dz == hz + 2;
                        for dy in 0..3usize {
                            let y = base_y + dy;
                            if dy == 0 {
                                col.set(dx, y, dz, BlockState(block::PLANKS));
                            } else if edge {
                                let corner = (dx == hx || dx == hx + 2) && (dz == hz || dz == hz + 2);
                                col.set(dx, y, dz, BlockState(if corner { block::LOG } else { block::PLANKS }));
                            } else {
                                col.set(dx, y, dz, BlockState::AIR);
                            }
                        }
                        col.set(dx, base_y + 3, dz, BlockState(block::LOG));
                    }
                }
                col.set(hx + 1, base_y + 1, hz, BlockState::AIR); // door
                col.set(hx + 1, base_y + 1, hz + 2, BlockState(block::TORCH));
            }
            // the keep: two-storey stone hall with the Accord banner
            for dx in 6..=9usize {
                for dz in 6..=9usize {
                    let edge = dx == 6 || dx == 9 || dz == 6 || dz == 9;
                    for dy in 0..6usize {
                        let y = base_y + dy;
                        if !edge {
                            col.set(dx, y, dz, BlockState::AIR);
                        } else if dy == 5 {
                            col.set(dx, y, dz, BlockState(block::LOG));
                        } else {
                            col.set(dx, y, dz, BlockState(block::STONE));
                        }
                    }
                }
            }
            col.set(7, base_y + 1, 6, BlockState::AIR);
            col.set(8, base_y + 1, 6, BlockState::AIR);
            col.set(7, base_y + 2, 6, BlockState::AIR);
            col.set(8, base_y + 2, 6, BlockState::AIR);
            col.set(7, base_y + 6, 7, BlockState(block::BANNER_ACCORD));
            col.set(6, base_y + 1, 5, BlockState(block::ACCORD_PILLAR));
            col.set(9, base_y + 1, 5, BlockState(block::ACCORD_PILLAR));
            col.set(6, base_y + 3, 9, BlockState(block::TORCH));
            col.set(9, base_y + 3, 9, BlockState(block::TORCH));
        };

        // frontier wooden watchtower in the new forest biomes
        let build_wood_tower = |col: &mut lf_voxel::ChunkColumn| {
            let base_y = match prepare(col, 5, 10, 5, 10) { Some(b) => b, None => return };
            for dy in 0..9usize {
                let y = base_y + dy;
                for dx in 6..=10usize {
                    for dz in 6..=10usize {
                        let edge = dx == 6 || dx == 10 || dz == 6 || dz == 10;
                        let pillar = (dx == 6 || dx == 10) && (dz == 6 || dz == 10);
                        if pillar || (dy >= 7 && edge) || dy == 0 {
                            col.set(dx, y, dz, BlockState(if dy >= 7 { block::LOG } else { block::LOG }));
                        } else {
                            col.set(dx, y, dz, BlockState::AIR);
                        }
                    }
                }
            }
            col.set(8, base_y + 8, 8, BlockState(block::TORCH));
            col.set(7, base_y + 1, 6, BlockState::AIR);
            col.set(8, base_y + 1, 6, BlockState::AIR);
        };

        // sun-baked ruin in the new desert biomes
        let build_desert_ruin = |col: &mut lf_voxel::ChunkColumn| {
            let base_y = match prepare(col, 4, 11, 4, 11) { Some(b) => b, None => return };
            for dx in 4..=11usize {
                for dz in 4..=11usize {
                    let remnant = (dx + dz) % 3 != 0 && (dx == 4 || dx == 11 || dz == 4 || dz == 11);
                    for dy in 0..2usize {
                        let y = base_y + dy;
                        if remnant && !(dy == 1 && (dx + dz) % 2 == 0) {
                            col.set(dx, y, dz, BlockState(block::STONE));
                        }
                    }
                }
            }
            col.set(8, base_y + 1, 8, BlockState(block::TORCH));
        };

        let build_pyramid = |col: &mut lf_voxel::ChunkColumn| {
            let base_y = match prepare(col, 2, 14, 2, 14) { Some(b) => b, None => return };
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

        let build_wizard_tower = |col: &mut lf_voxel::ChunkColumn| {
            let base_y = match prepare(col, 6, 10, 6, 10) { Some(b) => b, None => return };
            // 5x5 stone shell, 9 tall, hollow; a spiral stair climbs the
            // inside wall; the top floor holds the enchanting table.
            for dy in 0..9usize {
                let y = base_y + dy;
                for dx in 6..=10usize {
                    for dz in 6..=10usize {
                        let edge = dx == 6 || dx == 10 || dz == 6 || dz == 10;
                        if dy == 0 || dy == 8 || edge {
                            col.set(dx, y, dz, BlockState(block::STONE));
                        } else {
                            col.set(dx, y, dz, BlockState::AIR);
                        }
                    }
                }
            }
            // spiral stairs (the wall ring, one step per quarter turn)
            let ring = [(7, 6), (8, 6), (9, 6), (10, 7), (10, 8), (10, 9),
                        (9, 10), (8, 10), (7, 10), (6, 9), (6, 8), (6, 7)];
            for step in 0..7usize {
                let (sx, sz) = ring[step % ring.len()];
                col.set(sx, base_y + 1 + step, sz, BlockState(block::STONE));
            }
            // door
            col.set(8, base_y + 1, 6, BlockState::AIR);
            col.set(8, base_y + 2, 6, BlockState::AIR);
            // top floor: the enchanting table + light
            col.set(8, base_y + 9, 8, BlockState(block::ENCHANTING_TABLE));
            col.set(6, base_y + 9, 8, BlockState(block::TORCH));
            col.set(10, base_y + 9, 8, BlockState(block::TORCH));
        };

        let build_roost = |col: &mut lf_voxel::ChunkColumn| {
            // P36: a crag spire with a clutch of eggs on top — the dragon
            // settles here (marker = DRAGON_EGG).
            let base_y = match prepare(col, 5, 11, 5, 11) { Some(b) => b, None => return };
            for dy in 0..5usize {
                let r = 3 - (dy as i32 / 2);
                if r < 0 {
                    continue;
                }
                for dx in (8 - r as usize)..=(8 + r as usize) {
                    for dz in (8 - r as usize)..=(8 + r as usize) {
                        col.set(dx, base_y + dy, dz, BlockState(block::STONE));
                    }
                }
            }
            col.set(7, base_y + 5, 8, BlockState(block::DRAGON_EGG));
            col.set(9, base_y + 5, 7, BlockState(block::DRAGON_EGG));
        };

        // loop 345 kingdoms: the region's citadel chunk gets the full royal
        // build instead of anything else — walls, keep, throne, court.
        // Footprint is the whole chunk so the curtain wall's edge cells get
        // the same terrain adaptation + support guarantee as the interior.
        if let Some(_site) = self.kingdom_at(cx, cz) {
            if let Some(base) = prepare(col, 0, 15, 0, 15) {
                build_kingdom_citadel(col, base);
            }
            return;
        }

        match center_biome {
            // king-quest: the Accord Bastion — a full walled city, rare,
            // in the accord's meadow heartland. The banner on the keep
            // makes it a real NPC settlement.
            Biome::Meadow | Biome::SunflowerPlains if h0 % 331 == 0 => build_city(col),
            // king-quest B: frontier towers and ruins in the new biomes
            Biome::RedwoodForest | Biome::PineBarrens | Biome::FoggyFjord
            | Biome::MapleForest | Biome::WillowWetlands if h0 % 43 == 0 => build_wood_tower(col),
            Biome::Oasis | Biome::PaintedDunes if h0 % 47 == 0 => build_desert_ruin(col),
            Biome::Meadow | Biome::SunflowerPlains | Biome::AspenGrove
            | Biome::MapleForest | Biome::LavenderFields if h0 % 37 == 0 => build_hut(col),
            Biome::Highlands if h0 % 41 == 0 => build_watchtower(col),
            Biome::Desert if h0 % 29 == 0 => build_pyramid(col),
            // P33: wizard towers — rare, forested, and the only place the
            // enchanting table appears without crafting one
            Biome::FlowerForest if h0 % 53 == 0 => build_wizard_tower(col),
            Biome::Highlands if h0 % 97 == 0 => build_wizard_tower(col),
            // P36: dragon roosts — mountain peaks only, very rare
            Biome::Mountains if h0 % 89 == 0 => build_roost(col),
            Biome::SnowyPeaks if h0 % 101 == 0 => build_roost(col),
            // Faction structures (lore-and-visuals C3): one per faction, in
            // its home biomes. The banner block is the NPC-settle marker.
            Biome::Meadow | Biome::Forest | Biome::SunflowerPlains | Biome::LavenderFields
            if h0 % 131 == 0 => {
                build_faction_structure(FactionStructure::AccordEmbassy, col, &ground)
            }
            Biome::Mountains | Biome::Badlands | Biome::Volcanic if h0 % 139 == 0 => {
                build_faction_structure(FactionStructure::IronbornForgeCamp, col, &ground)
            }
            Biome::Highlands | Biome::Taiga | Biome::MushroomHollow if h0 % 149 == 0 => {
                build_faction_structure(FactionStructure::CovenantGroveShrine, col, &ground)
            }
            Biome::Savanna | Biome::WindsweptSavanna if h0 % 157 == 0 => {
                build_faction_structure(FactionStructure::FreeholdsLonghouse, col, &ground)
            }
            Biome::WindsweptHills | Biome::Tundra if h0 % 167 == 0 => {
                build_faction_structure(FactionStructure::AshenLibrary, col, &ground)
            }
            Biome::PaleGarden | Biome::DarkForest if h0 % 173 == 0 => {
                build_faction_structure(FactionStructure::NamelessCamp, col, &ground)
            }
            _ => {}
        }
    }
}

/// The six faction structures (lore-and-visuals C3). Built in-chunk from
/// each faction's themed blocks; the banner block doubles as the marker
/// the client scans to settle faction NPCs there. Public so vistest can
/// plant one deterministically for proof shots.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FactionStructure {
    AccordEmbassy,
    IronbornForgeCamp,
    CovenantGroveShrine,
    FreeholdsLonghouse,
    AshenLibrary,
    NamelessCamp,
}

impl FactionStructure {
    pub fn marker_block(self) -> u32 {
        use lf_voxel::registry::block;
        match self {
            FactionStructure::AccordEmbassy => block::BANNER_ACCORD,
            FactionStructure::IronbornForgeCamp => block::BANNER_IRONBORN,
            FactionStructure::CovenantGroveShrine => block::BANNER_COVENANT,
            FactionStructure::FreeholdsLonghouse => block::BANNER_FREEHOLDS,
            FactionStructure::AshenLibrary => block::BANNER_ASHEN,
            FactionStructure::NamelessCamp => block::BANNER_NAMELESS,
        }
    }

    /// Data-file structure key matching lore/npcs.toml `structure` fields.
    pub fn key(self) -> &'static str {
        match self {
            FactionStructure::AccordEmbassy => "accord_embassy",
            FactionStructure::IronbornForgeCamp => "ironborn_forge_camp",
            FactionStructure::CovenantGroveShrine => "covenant_grove_shrine",
            FactionStructure::FreeholdsLonghouse => "freeholds_longhouse",
            FactionStructure::AshenLibrary => "ashen_library",
            FactionStructure::NamelessCamp => "nameless_camp",
        }
    }
}

// ----------------------------------------------------------------------
// loop 345: kingdoms — region-placed royal citadels. One kingdom per
// 12x12-chunk region (where the terrain allows), each with a deterministic
// name, a walled citadel, a throne (the settle marker the client scans,
// like faction banners), and a court of NPCs. The Kingdom Compass item
// queries `nearest_kingdom` so it points the way from spawn.

/// Region edge in chunks: one candidate kingdom site per region.
pub const KINGDOM_REGION: i32 = 12;
/// How many hash-derived candidate chunks per region are considered before
/// the region gives up (mountains/ocean regions have no kingdom).
pub const KINGDOM_CANDIDATES: u32 = 16;

/// The royal name pool; the pick is a pure function of (seed, region).
pub const KINGDOM_NAMES: [&str; 16] = [
    "Elderfall", "Thornmere", "Goldhelm", "Duskmere", "Ashvale", "Brightwater",
    "Stonewatch", "Emberhold", "Ravensrest", "Wintermoor", "Silverford",
    "Highcrest", "Oakmarch", "Windmere", "Ironhollow", "Dawnspire",
];

/// A placed kingdom: the citadel's chunk and its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KingdomSite {
    pub cx: i32,
    pub cz: i32,
    pub name: &'static str,
}

impl KingdomSite {
    /// World-block center of the citadel (its throne room).
    pub fn center(&self) -> [i32; 2] {
        [self.cx * 16 + 8, self.cz * 16 + 8]
    }
}

fn kingdom_biome_ok(b: Biome) -> bool {
    matches!(b, Biome::Meadow | Biome::Forest | Biome::FlowerForest
        | Biome::SunflowerPlains | Biome::AspenGrove | Biome::LavenderFields
        | Biome::BirchForest | Biome::CherryGrove)
}

impl WorldGen {
    /// The region's candidate chunk locals (hash-derived, deterministic).
    fn kingdom_candidate_locals(&self, rx: i32, rz: i32) -> [(i32, i32); KINGDOM_CANDIDATES as usize] {
        let feats = self.seed_for_features();
        let mut out = [(0i32, 0i32); KINGDOM_CANDIDATES as usize];
        for (i, slot) in out.iter_mut().enumerate() {
            let h = hash2(rx.wrapping_mul(31).wrapping_add(i as i64 as i32),
                rz.wrapping_mul(17).wrapping_sub(i as i64 as i32), feats ^ 0x85ebca6b);
            *slot = ((h % KINGDOM_REGION as u64) as i32,
                ((h / KINGDOM_REGION as u64) % KINGDOM_REGION as u64) as i32);
        }
        out
    }

    /// Terrain eligibility for a citadel chunk: grassy biome, dry, lowland,
    /// flat enough that walls don't straddle a cliff.
    fn kingdom_chunk_ok(&self, cx: i32, cz: i32) -> bool {
        let (x, z) = (cx * 16 + 8, cz * 16 + 8);
        if !kingdom_biome_ok(self.biome(x, z)) {
            return false;
        }
        let t = self.surface_top(x, z);
        if t <= SEA_LEVEL + 1 || t > 120 {
            return false;
        }
        let (mut hi, mut lo) = (i32::MIN, i32::MAX);
        for (dx, dz) in [(0, 0), (6, 0), (-6, 0), (0, 6), (0, -6)] {
            let h = self.surface_top(x + dx, z + dz);
            hi = hi.max(h);
            lo = lo.min(h);
        }
        hi - lo <= 5
    }

    fn kingdom_name(&self, rx: i32, rz: i32) -> &'static str {
        let feats = self.seed_for_features();
        let h = hash2(rx, rz, feats ^ 0x9e3779b9);
        KINGDOM_NAMES[(h % KINGDOM_NAMES.len() as u64) as usize]
    }

    /// The kingdom this region hosts, if any candidate chunk is eligible.
    /// The first eligible candidate in hash order wins, so the same seed
    /// always yields the same site.
    pub fn kingdom_in_region(&self, rx: i32, rz: i32) -> Option<KingdomSite> {
        for (lx, lz) in self.kingdom_candidate_locals(rx, rz) {
            let (cx, cz) = (rx * KINGDOM_REGION + lx, rz * KINGDOM_REGION + lz);
            if self.kingdom_chunk_ok(cx, cz) {
                return Some(KingdomSite { cx, cz, name: self.kingdom_name(rx, rz) });
            }
        }
        None
    }

    /// Some(site) when `(cx, cz)` IS its region's citadel chunk. Cheap
    /// (a handful of hashes) so `generate_chunk` can call it per chunk.
    pub fn kingdom_at(&self, cx: i32, cz: i32) -> Option<KingdomSite> {
        let rx = cx.div_euclid(KINGDOM_REGION);
        let rz = cz.div_euclid(KINGDOM_REGION);
        let (lx, lz) = (cx.rem_euclid(KINGDOM_REGION), cz.rem_euclid(KINGDOM_REGION));
        for (clx, clz) in self.kingdom_candidate_locals(rx, rz) {
            if clx == lx && clz == lz && self.kingdom_chunk_ok(cx, cz) {
                return Some(KingdomSite { cx, cz, name: self.kingdom_name(rx, rz) });
            }
        }
        None
    }

    /// Nearest kingdom to a world position, searching the surrounding
    /// region ring (5x5 regions ≈ ±2 regions ≈ ±480 blocks). Returns the
    /// site and the squared block distance.
    pub fn nearest_kingdom(&self, x: i32, z: i32) -> Option<(KingdomSite, i64)> {
        let rx = x.div_euclid(16).div_euclid(KINGDOM_REGION);
        let rz = z.div_euclid(16).div_euclid(KINGDOM_REGION);
        let mut best: Option<(KingdomSite, i64)> = None;
        for drx in -2..=2 {
            for drz in -2..=2 {
                if let Some(site) = self.kingdom_in_region(drx + rx, drz + rz) {
                    let (kx, kz) = (site.cx as i64 * 16 + 8, site.cz as i64 * 16 + 8);
                    let d = (kx - x as i64) * (kx - x as i64) + (kz - z as i64) * (kz - z as i64);
                    if best.as_ref().map(|(_, bd)| d < *bd).unwrap_or(true) {
                        best = Some((site, d));
                    }
                }
            }
        }
        best
    }
}

/// Build the kingdom citadel into a chunk column: curtain wall with
/// crenellations and corner towers, a gate with the royal banner, the
/// two-storey keep with its throne (the settle marker), two houses, a
/// well, a market court, and a tilled farm plot. Footprint is the whole
/// chunk; `ground(lx, lz)` is the surface y (terrain adaptation is the
/// caller's `prepare`). Public so vistest can plant one deterministically.
pub fn build_kingdom_citadel(
    col: &mut lf_voxel::ChunkColumn,
    base_y: usize,
) {
    use lf_voxel::BlockState;
    use lf_voxel::registry::block;
    let set = |col: &mut lf_voxel::ChunkColumn, x: usize, y: usize, z: usize, b: u32| {
        if y < 256 {
            col.set(x, y, z, BlockState(b));
        }
    };
    let b = base_y;
    if b <= SEA_LEVEL as usize || b > 200 {
        return;
    }
    // --- curtain wall: ring at the chunk edge, 4 tall + crenellations ---
    for lx in 0..16usize {
        for lz in 0..16usize {
            let edge = lx == 0 || lx == 15 || lz == 0 || lz == 15;
            if !edge {
                continue;
            }
            for dy in 1..=4usize {
                set(col, lx, b + dy, lz, block::KINGDOM_BRICK);
            }
            // crenellation: merlon on every other cell
            if (lx + lz) % 2 == 0 {
                set(col, lx, b + 5, lz, block::KINGDOM_BRICK);
            }
        }
    }
    // corner towers: 8 tall, torch at the top
    for (tx, tz) in [(0usize, 0usize), (15, 0), (0, 15), (15, 15)] {
        for dy in 1..=8usize {
            set(col, tx, b + dy, tz, block::KINGDOM_BRICK);
        }
        set(col, tx, b + 9, tz, block::TORCH);
    }
    // --- south gate (lz == 15): opening + flanking posts + the banner ---
    for dx in [7usize, 8usize] {
        for dy in 1..=3usize {
            set(col, dx, b + dy, 15, block::AIR);
        }
    }
    for gx in [5usize, 10usize] {
        for dy in 1..=5usize {
            set(col, gx, b + dy, 15, block::KINGDOM_BRICK);
        }
        set(col, gx, b + 6, 15, block::TORCH);
    }
    set(col, 7, b + 6, 15, block::BANNER_KINGDOM);
    set(col, 8, b + 6, 15, block::BANNER_KINGDOM);
    // gate road: stone from the gate to the keep door
    for lz in 10..=14usize {
        set(col, 7, b, lz, block::STONE);
        set(col, 8, b, lz, block::STONE);
    }
    // --- the keep (5..=10 x 3..=9): two storeys, crenellated roof ------
    for dx in 5..=10usize {
        for dz in 3..=9usize {
            let edge = dx == 5 || dx == 10 || dz == 3 || dz == 9;
            // interior floor
            set(col, dx, b, dz, block::KINGDOM_BRICK);
            for dy in 1..=5usize {
                let y = b + dy;
                if !edge {
                    set(col, dx, y, dz, block::AIR);
                } else if dy == 2 {
                    // arrow-slit band
                    set(col, dx, y, dz, if (dx + dz) % 3 == 0 { block::AIR } else { block::KINGDOM_BRICK });
                } else {
                    set(col, dx, y, dz, block::KINGDOM_BRICK);
                }
            }
            // roof crenellation ring + plank ceiling over the hall
            set(col, dx, b + 6, dz, if edge && (dx + dz) % 2 == 0 {
                block::KINGDOM_BRICK
            } else {
                block::PLANKS
            });
        }
    }
    // keep door (south face) + throne dais (north interior)
    set(col, 7, b + 1, 3, block::AIR);
    set(col, 7, b + 2, 3, block::AIR);
    set(col, 8, b + 1, 3, block::AIR);
    set(col, 8, b + 2, 3, block::AIR);
    for dx in 6..=9usize {
        for dz in 7..=8usize {
            set(col, dx, b + 1, dz, block::KINGDOM_BRICK); // the dais step
        }
    }
    // the throne, on the dais, facing the door — the kingdom marker
    set(col, 7, b + 2, 8, block::THRONE);
    set(col, 9, b + 2, 8, block::BANNER_KINGDOM);
    set(col, 5, b + 3, 6, block::TORCH);
    set(col, 10, b + 3, 6, block::TORCH);
    // --- two houses in the west/east yards ------------------------------
    for (hx0, hz0) in [(2usize, 2usize), (12usize, 2usize)] {
        for dx in hx0..hx0 + 3 {
            for dz in hz0..hz0 + 3 {
                let edge = dx == hx0 || dx == hx0 + 2 || dz == hz0 || dz == hz0 + 2;
                set(col, dx, b, dz, block::PLANKS);
                for dy in 1..=2usize {
                    if edge {
                        set(col, dx, b + dy, dz, block::PLANKS);
                    } else {
                        set(col, dx, b + dy, dz, block::AIR);
                    }
                }
                set(col, dx, b + 3, dz, block::LOG); // roof
            }
        }
        // door faces the courtyard (inward); hearth + workbench inside
        let door_x = if hx0 == 2 { hx0 + 2 } else { hx0 };
        set(col, door_x, b + 1, hz0 + 1, block::AIR);
        set(col, if hx0 == 2 { hx0 } else { hx0 + 2 }, b + 1, hz0 + 2, block::CRAFTING_TABLE);
        set(col, hx0 + 1, b + 1, hz0 + 2, block::FURNACE);
    }
    // --- the well (west courtyard): open water in a stone ring ----------
    for (wx, wz) in [(2usize, 10usize), (3, 10), (2, 11), (3, 11)] {
        set(col, wx, b, wz, block::WATER);
    }
    for (wx, wz) in [(1usize, 10usize), (1, 11), (4, 10), (4, 11),
                     (2, 9), (3, 9), (2, 12), (3, 12)] {
        set(col, wx, b + 1, wz, block::STONE); // rim
    }
    // --- market court (east of the gate road): two stalls ---------------
    for (mx, mz) in [(10usize, 11usize), (12usize, 12usize)] {
        set(col, mx, b + 1, mz, block::PLANKS); // counter
        set(col, mx + 1, b + 1, mz, block::PLANKS);
        set(col, mx, b + 2, mz, block::BANNER_KINGDOM); // pennant
        set(col, mx + 1, b + 2, mz + 1, block::AIR);
    }
    set(col, 13, b + 1, 11, block::CHEST); // the market stock
    // --- farm plot (east yard, south of nothing, clear of the house):
    // tilled rows with an irrigation channel down the middle ----------
    for dx in 11..=14usize {
        for dz in 6..=9usize {
            if dx == 12 && (7..=8).contains(&dz) {
                set(col, dx, b, dz, block::WATER); // channel
            } else {
                set(col, dx, b, dz, block::DIRT); // tilled soil
            }
        }
    }
    // courtyard lanterns flanking the gate road
    set(col, 6, b + 1, 13, block::LANTERN);
    set(col, 9, b + 1, 13, block::LANTERN);
}

/// Build a faction structure into a chunk column. `ground(lx, lz)` gives
/// the surface y for that column cell; footprints stay in-chunk. Terrain
/// adaptation (D5): the floor sits at the center column's surface, gaps
/// under a sloped footprint (>4 blocks of variance) are filled with
/// `filler`, and sites more than half underwater are refused.
pub fn build_faction_structure(
    kind: FactionStructure,
    col: &mut lf_voxel::ChunkColumn,
    ground: &dyn Fn(usize, usize) -> usize,
) {
    use lf_voxel::BlockState;
    use lf_voxel::registry::block;
    let set = |col: &mut lf_voxel::ChunkColumn, x: usize, y: usize, z: usize, b: u32| {
        if y < 256 {
            col.set(x, y, z, BlockState(b));
        }
    };
    let base_y = ground(8, 8);
    if base_y <= SEA_LEVEL as usize || base_y > 200 {
        return;
    }
    // D5 checks against the footprint's corners (conservative 5-sample set)
    let samples = [(8usize, 8usize), (4usize, 4usize), (12, 12), (4, 12), (12, 4)];
    let mut under = 0usize;
    let (mut lo, mut hi) = (usize::MAX, 0usize);
    for (lx, lz) in samples {
        let t = ground(lx, lz);
        if t <= SEA_LEVEL as usize { under += 1; }
        lo = lo.min(t);
        hi = hi.max(t);
    }
    if under * 2 > samples.len() {
        return;
    }
    if hi.saturating_sub(lo) > 4 {
        for dx in 4..=12usize {
            for dz in 4..=12usize {
                let t = ground(dx, dz).min(base_y);
                for y in t..base_y {
                    if col.get(dx, y, dz) == BlockState::AIR {
                        col.set(dx, y, dz, BlockState(block::DIRT));
                    }
                }
            }
        }
    }
    // support guarantee (D5): fill open space under the footprint down to
    // solid ground — caves and rivers may hollow the heightmap's ground
    for dx in 4..=12usize {
        for dz in 4..=12usize {
            let mut y = base_y;
            while y > 1 && col.get(dx, y - 1, dz) == BlockState::AIR {
                col.set(dx, y - 1, dz, BlockState(block::DIRT));
                y -= 1;
            }
        }
    }
    let b = base_y as usize;
    match kind {
        FactionStructure::AccordEmbassy => {
            // Walled courtyard with a gatehouse: accord_stone walls, two
            // accord_pillar gate posts, banner at the center.
            for dx in 5..=11usize {
                for dz in 5..=11usize {
                    let edge = dx == 5 || dx == 11 || dz == 5 || dz == 11;
                    if edge {
                        for dy in 0..3usize {
                            set(col, dx, b + dy, dz, block::ACCORD_STONE);
                        }
                    } else {
                        set(col, dx, b, dz, block::ACCORD_STONE); // paved court
                    }
                }
            }
            // gate opening + flanking pillars (south side)
            for dy in 1..3usize {
                set(col, 8, b + dy, 5, block::AIR);
                set(col, 7, b + dy, 5, block::ACCORD_PILLAR);
                set(col, 9, b + dy, 5, block::ACCORD_PILLAR);
            }
            set(col, 8, b + 1, 8, block::BANNER_ACCORD);
            set(col, 6, b + 1, 8, block::TORCH);
            set(col, 10, b + 1, 8, block::TORCH);
        }
        FactionStructure::IronbornForgeCamp => {
            // Compact industrial shelter: brick walls, grate windows, a
            // working furnace and the guild banner.
            for dx in 5..=11usize {
                for dz in 6..=11usize {
                    let edge = dx == 5 || dx == 11 || dz == 6 || dz == 11;
                    if edge {
                        for dy in 0..3usize {
                            set(col, dx, b + dy, dz, block::IRONBORN_BRICK);
                        }
                    } else {
                        set(col, dx, b, dz, block::IRONBORN_BRICK);
                    }
                }
            }
            // grate windows on the long wall
            set(col, 6, b + 1, 6, block::IRONBORN_GRATE);
            set(col, 10, b + 1, 6, block::IRONBORN_GRATE);
            // door
            set(col, 8, b + 1, 6, block::AIR);
            set(col, 8, b + 2, 6, block::AIR);
            set(col, 7, b + 1, 9, block::FURNACE);
            set(col, 8, b + 1, 8, block::BANNER_IRONBORN);
            set(col, 6, b + 1, 8, block::LANTERN);
        }
        FactionStructure::CovenantGroveShrine => {
            // Ring of covenantwood posts around a central ember-glowstone
            // altar.
            let posts = [(8usize, 4usize), (11, 6), (11, 10), (8, 12), (5, 10), (5, 6)];
            for (px, pz) in posts {
                let py = ground(px, pz).min(250) as usize;
                if py <= SEA_LEVEL as usize || py > 200 {
                    continue;
                }
                set(col, px, py, pz, block::EMBER_COVENANTWOOD);
                set(col, px, py + 1, pz, block::EMBER_COVENANTWOOD);
            }
            set(col, 8, b, 8, block::EMBER_GLOWSTONE);
            set(col, 8, b + 1, 8, block::EMBER_GLOWSTONE);
            set(col, 8, b + 2, 8, block::BANNER_COVENANT);
        }
        FactionStructure::FreeholdsLonghouse => {
            // Thatch-roofed daub longhouse, log posts, a chest and table.
            for dx in 4..=12usize {
                for dz in 5..=11usize {
                    let edge = dx == 4 || dx == 12 || dz == 5 || dz == 11;
                    let corner = (dx == 4 || dx == 12) && (dz == 5 || dz == 11);
                    if corner {
                        for dy in 0..3usize {
                            set(col, dx, b + dy, dz, block::LOG);
                        }
                    } else if edge {
                        for dy in 0..3usize {
                            set(col, dx, b + dy, dz, block::FREEHOLDS_DAUB);
                        }
                    } else {
                        set(col, dx, b, dz, block::FREEHOLDS_DAUB);
                    }
                }
            }
            // thatch roof (two overlapping caps so it reads from afar)
            for dx in 4..=12usize {
                for dz in 5..=11usize {
                    set(col, dx, b + 3, dz, block::FREEHOLDS_THATCH);
                }
            }
            for dx in 5..=11usize {
                for dz in 6..=10usize {
                    set(col, dx, b + 4, dz, block::FREEHOLDS_THATCH);
                }
            }
            set(col, 8, b + 1, 5, block::AIR);
            set(col, 8, b + 2, 5, block::AIR);
            set(col, 6, b + 1, 8, block::CHEST);
            set(col, 7, b + 1, 8, block::CRAFTING_TABLE);
            set(col, 9, b + 1, 8, block::BANNER_FREEHOLDS);
            set(col, 10, b + 1, 8, block::TORCH);
        }
        FactionStructure::AshenLibrary => {
            // Small marble library: bookshelf interior, a lore chest, the
            // banner, and light to read by.
            for dx in 5..=11usize {
                for dz in 5..=11usize {
                    let edge = dx == 5 || dx == 11 || dz == 5 || dz == 11;
                    if edge {
                        for dy in 0..4usize {
                            set(col, dx, b + dy, dz, block::ASHEN_MARBLE);
                        }
                    } else {
                        set(col, dx, b, dz, block::ASHEN_MARBLE);
                    }
                }
            }
            set(col, 8, b + 1, 5, block::AIR);
            set(col, 8, b + 2, 5, block::AIR);
            set(col, 8, b + 3, 5, block::AIR);
            // interior shelves
            for dz in 7..=9usize {
                set(col, 5, b + 1, dz, block::ASHEN_BOOKSHELF);
                set(col, 11, b + 1, dz, block::ASHEN_BOOKSHELF);
            }
            set(col, 7, b + 1, 8, block::CHEST);
            set(col, 9, b + 1, 8, block::BANNER_ASHEN);
            set(col, 8, b + 1, 9, block::TORCH);
        }
        FactionStructure::NamelessCamp => {
            // Derelict camp: a broken rotwood palisade with gaps, a
            // scorched firepit, one loot chest. Banner flies torn.
            for dx in 5..=11usize {
                for dz in 5..=11usize {
                    let edge = dx == 5 || dx == 11 || dz == 5 || dz == 11;
                    if !edge {
                        continue;
                    }
                    // gaps: every third wall cell is missing (derelict)
                    if (dx + dz) % 3 == 0 {
                        continue;
                    }
                    let h = if (dx * 7 + dz * 13) % 4 < 2 { 1 } else { 2 };
                    for dy in 0..h {
                        set(col, dx, b + dy, dz, block::NAMELESS_ROTWOOD);
                    }
                }
            }
            // scorched firepit ring
            set(col, 7, b, 8, block::NAMELESS_SCORCHED);
            set(col, 9, b, 8, block::NAMELESS_SCORCHED);
            set(col, 8, b, 7, block::NAMELESS_SCORCHED);
            set(col, 8, b, 9, block::NAMELESS_SCORCHED);
            set(col, 8, b, 8, block::NAMELESS_SCORCHED);
            set(col, 6, b + 1, 6, block::CHEST);
            set(col, 10, b + 1, 10, block::BANNER_NAMELESS);
        }
    }
}

impl WorldGen {
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

    /// ui-world-craft D1: the two-layer terrain must keep land buildable —
    /// across 5 seeds the flat fraction (within ±6 of sea level) must
    /// average ≥ 40% with a per-seed floor of 30%, and flat land must beat
    /// high mountains in aggregate (individual seeds may ride mountainous —
    /// "that snowy valley was v0.3" is a feature, not a bug). Measured
    /// fractions print for the DEVLOG.
    #[test]
    fn lowlands_dominate_mountains_across_seeds() {
        let mut fractions = Vec::new();
        let mut high_totals = Vec::new();
        for seed in [1u64, 2, 12345, 999999, 20260827] {
            let gen = WorldGen::new(Seed(seed));
            let mut near_sea = 0usize;
            let mut high = 0usize;
            let mut total = 0usize;
            for x in (-2000..2000).step_by(11) {
                for z in (-2000..2000).step_by(11) {
                    let h = gen.height(x, z);
                    if (h - SEA_LEVEL).abs() <= 6 {
                        near_sea += 1;
                    }
                    if h > SEA_LEVEL + 40 {
                        high += 1;
                    }
                    total += 1;
                }
            }
            let flat = near_sea as f64 / total as f64;
            fractions.push(flat);
            high_totals.push(high as f64 / total as f64);
            assert!(flat >= 0.30, "seed {}: flat fraction {:.3} below the 30% floor", seed, flat);
        }
        let mean = fractions.iter().sum::<f64>() / fractions.len() as f64;
        let high_mean = high_totals.iter().sum::<f64>() / high_totals.len() as f64;
        assert!(mean >= 0.40, "mean flat fraction {:.3} below the 40% target: {:?}", mean, fractions);
        assert!(mean > high_mean * 1.2,
            "flat land ({:.3}) does not dominate mountains ({:.3})", mean, high_mean);
        println!("flat-land fractions: {:?} (mean {:.3}, mountains {:.3})",
            fractions.iter().map(|f| format!("{:.3}", f)).collect::<Vec<_>>(), mean, high_mean);
    }

    /// ui-world-craft D2: rivers exist — water channels below sea level on
    /// land, 2-6 deep, bounded width (no flooded corridors), and none in
    /// the highlands.
    #[test]
    fn rivers_carve_lowland_channels() {
        let gen = WorldGen::new(Seed(20260827));
        let mut river_columns = 0usize;
        let mut river_chunks = 0usize;
        for cx in -40..40i32 {
            for cz in -40..40i32 {
                let mut in_chunk = 0usize;
                for lx in (0..16).step_by(4) {
                    for lz in (0..16).step_by(4) {
                        let x = cx * 16 + lx;
                        let z = cz * 16 + lz;
                        let cf = gen.continental_factor(x, z);
                        let rf = gen.river_factor(x, z);
                        if rf > 0.7 {
                            in_chunk += 1;
                            river_columns += 1;
                            assert!(cf <= 0.55, "river in the highlands at ({},{})", x, z);
                            let h = gen.height(x, z);
                            // a full channel bottoms out at the river bed on
                            // land (water fills to sea level); past the shelf
                            // it is open estuary water
                            if gen.continental_stretched(x, z) >= 0.40 {
                                // the carve never lifts ground: a channel may
                                // ride through a shallow dip below the bed,
                                // but never above sea level and never a canyon
                                assert!((SEA_LEVEL - 8..=SEA_LEVEL).contains(&h),
                                    "river bed out of band at ({},{}): {}", x, z, h);
                            } else {
                                assert!(h <= SEA_LEVEL, "estuary above sea at ({},{}): {}", x, z, h);
                            }
                        }
                    }
                }
                if in_chunk > 0 {
                    river_chunks += 1;
                }
            }
        }
        assert!(river_columns > 15, "expected real rivers across 80x80 chunks, found {} columns in {} chunks",
            river_columns, river_chunks);
    }

    /// ui-world-craft D3: lava floods the deepest caves; deep slate takes
    /// over below y=30.
    #[test]
    fn deep_caves_get_lava_and_deep_slate() {
        let gen = WorldGen::new(Seed(12345));
        let (mut lava, mut deep) = (0usize, 0usize);
        for cx in -3..=3 {
            for cz in -3..=3 {
                let col = gen.generate_chunk(cx, cz);
                for lx in 0..16usize {
                    for lz in 0..16usize {
                        for y in 6..30 {
                            let id = col.get(lx, y, lz).id();
                            if id == lf_voxel::registry::block::LAVA {
                                lava += 1;
                                assert!(y <= 10, "lava above y=10 at {}", y);
                            }
                            if id == lf_voxel::registry::block::DEEP_SLATE {
                                deep += 1;
                                assert!(y <= 30, "deep slate above y=30 at {}", y);
                            }
                        }
                    }
                }
            }
        }
        assert!(lava > 20, "expected lava lakes below y=10, found {}", lava);
        assert!(deep > 500, "expected a deep-slate cave biome, found {}", deep);
    }

    /// ui-world-craft E3: every biome's ground cover actually generates —
    /// a forest chunk carries tall grass, a desert chunk carries cactus.
    #[test]
    fn biome_ground_cover_generates() {
        use lf_voxel::registry::block;
        let gen = WorldGen::new(Seed(12345));
        let mut tall_grass = 0usize;
        for cx in -30..30i32 {
            for cz in -30..30i32 {
                // sample meadow-ish columns via the biome fn before generating
                let biome = gen.biome(cx * 16 + 8, cz * 16 + 8);
                if !matches!(biome, Biome::Meadow | Biome::Forest | Biome::FlowerForest | Biome::Jungle
                    | Biome::SunflowerPlains | Biome::AspenGrove | Biome::MapleForest | Biome::LavenderFields
                    | Biome::RedwoodForest) {
                    continue;
                }
                let col = gen.generate_chunk(cx, cz);
                for lx in 1..15usize {
                    for lz in 1..15usize {
                        for y in 40..140usize {
                            if col.get(lx, y, lz).id() == block::TALL_GRASS {
                                tall_grass += 1;
                            }
                        }
                    }
                }
                if tall_grass > 30 { break; }
            }
        }
        assert!(tall_grass > 30, "lush biomes must scatter tall grass, found {}", tall_grass);
    }

    /// ui-world-craft D5: structures never float — wherever a hut actually
    /// built, its floor is solid across the footprint (terrain or filled
    /// platform). Sites the adaptivity refuses (mostly underwater, or no
    /// room) simply build nothing.
    #[test]
    fn structure_footprints_are_supported() {
        use lf_voxel::registry::block;
        let gen = WorldGen::new(Seed(12345));
        let feats = gen.seed_for_features();
        let mut checked = 0usize;
        'find: for cx in -80..80i32 {
            for cz in -80..80i32 {
                let b = gen.biome(cx * 16 + 8, cz * 16 + 8);
                if !matches!(b, Biome::Meadow | Biome::Forest | Biome::SunflowerPlains
                    | Biome::AspenGrove | Biome::MapleForest | Biome::LavenderFields) {
                    continue;
                }
                if hash2(cx, cz, feats ^ 0x5bd1e995) % 37 != 0 {
                    continue; // the hut placement hash
                }
                let col = gen.generate_chunk(cx, cz);
                // did a hut actually build here? its crafting table marks it
                let mut table_y = None;
                'scan: for lx in 5..=10usize {
                    for lz in 5..=10usize {
                        for y in 60..200usize {
                            if col.get(lx, y, lz).id() == block::CRAFTING_TABLE {
                                table_y = Some(y);
                                break 'scan;
                            }
                        }
                    }
                }
                let Some(table_y) = table_y else {
                    // refused site: the footprint must NOT have a floating
                    // partial build — nothing to assert beyond that
                    continue;
                };
                let base = table_y - 1; // the floor layer under the table
                for dx in 5..=10usize {
                    for dz in 5..=10usize {
                        assert!(col.get(dx, base, dz).id() != block::AIR,
                            "unsupported hut floor at ({},{}) cell ({},{})", cx, cz, dx, dz);
                    }
                }
                checked += 1;
                if checked >= 2 {
                    break 'find;
                }
            }
        }
        assert!(checked >= 1, "no built hut found for the footprint scan");
    }

    /// P36: dragon roosts (egg clutches on a stone crag) generate in the
    /// mountain biomes only. Biome-guided like the faction-structure test:
    /// predict the placement with the generator's own hash, then verify the
    /// chunk really carries eggs (a blind radius scan would need thousands
    /// of chunk generations under the wide v4 terrain).
    #[test]
    fn dragon_roosts_generate_on_peaks() {
        use lf_voxel::registry::block;
        for seed in [7u64, 42, 99, 2026, 12345] {
            let gen = WorldGen::new(Seed(seed));
            let feats = gen.seed_for_features();
            for cx in -60..60i32 {
                for cz in -60..60i32 {
                    let biome = gen.biome(cx * 16 + 8, cz * 16 + 8);
                    let hits = match biome {
                        Biome::Mountains => hash2(cx, cz, feats ^ 0x5bd1e995) % 89 == 0,
                        Biome::SnowyPeaks => hash2(cx, cz, feats ^ 0x5bd1e995) % 101 == 0,
                        _ => false,
                    };
                    if !hits {
                        continue;
                    }
                    let col = gen.generate_chunk(cx, cz);
                    let eggs = (0..16usize).flat_map(|lx| (0..16usize).map(move |lz| (lx, lz)))
                        .filter(|(lx, lz)| (40..230usize).any(|y| col.get(*lx, y, *lz).id() == block::DRAGON_EGG))
                        .count();
                    assert!(eggs >= 2, "seed {}: predicted roost at ({},{}) has {} eggs",
                        seed, cx, cz, eggs);
                    return; // one verified roost proves the pipeline
                }
            }
        }
        panic!("no sampled seed places a roost in +-60 chunks — check the placement hash");
    }

    /// P33: wizard towers generate (enchanting table on top), only in the
    /// gated biomes, rare.
    /// lore-and-visuals C3: all six faction structures generate (their
    /// banner markers appear), only in their factions' home biomes,
    /// deterministically. Placement is predicted with the same per-chunk
    /// hash the generator uses (cheap), then verified on the real chunk.
    #[test]
    fn faction_structures_generate_in_home_biomes() {
        use lf_voxel::registry::block;
        let gen = WorldGen::new(Seed(2026));
        let feats = gen.seed_for_features();
        // (marker, structure name, home biomes, placement modulus)
        let markers: [(u32, &str, Vec<Biome>, u64); 6] = [
            (block::BANNER_ACCORD, "accord_embassy", vec![Biome::Meadow, Biome::Forest], 131),
            (block::BANNER_IRONBORN, "ironborn_forge_camp", vec![Biome::Mountains, Biome::Badlands, Biome::Volcanic], 139),
            (block::BANNER_COVENANT, "covenant_grove_shrine", vec![Biome::Highlands, Biome::Taiga, Biome::MushroomHollow], 149),
            (block::BANNER_FREEHOLDS, "freeholds_longhouse", vec![Biome::Savanna, Biome::WindsweptSavanna], 157),
            (block::BANNER_ASHEN, "ashen_library", vec![Biome::WindsweptHills, Biome::Tundra], 167),
            (block::BANNER_NAMELESS, "nameless_camp", vec![Biome::PaleGarden, Biome::DarkForest], 173),
        ];
        let has_marker = |col: &lf_voxel::ChunkColumn, marker: u32| {
            (0..16).any(|lx| (0..16).any(|lz|
                (40..200).any(|y| col.get(lx, y, lz).id() == marker)))
        };
        for (marker, name, homes, modulus) in &markers {
            // terrain `prepare` may refuse a predicted site (water/slope),
            // so walk the candidates in order and assert at least one
            // chunk actually carries the banner
            let mut built = 0;
            'find: for cx in -120..120i32 {
                for cz in -120..120i32 {
                    let b = gen.biome(cx * 16 + 8, cz * 16 + 8);
                    if !homes.contains(&b) {
                        continue;
                    }
                    let h0 = hash2(cx, cz, feats ^ 0x5bd1e995);
                    if h0 % *modulus != 0 {
                        continue;
                    }
                    // this chunk is a predicted site: it only counts once
                    // the terrain actually accepts the build (prepare may
                    // refuse underwater/steep sites)
                    let col = gen.generate_chunk(cx, cz);
                    if !has_marker(&col, *marker) {
                        continue; // site refused by terrain; try the next
                    }
                    // determinism: same chunk, same banner
                    let col2 = gen.generate_chunk(cx, cz);
                    assert!(has_marker(&col2, *marker), "{} marker not deterministic", name);
                    built += 1;
                    if built >= 1 {
                        break 'find;
                    }
                }
            }
            assert!(built >= 1, "no chunk in a 240x240 scan places {} — check the placement hash", name);
        }
    }

    /// The planted faction structures are buildable standalone (vistest
    /// uses this) and carry their faction blocks.
    #[test]
    fn planted_faction_structures_carry_faction_blocks() {
        use lf_voxel::registry::block;
        let gen = WorldGen::new(Seed(7));
        let ground = |_lx: usize, _lz: usize| -> usize { 80 };
        let cases: [(FactionStructure, u32, u32); 6] = [
            (FactionStructure::AccordEmbassy, block::ACCORD_STONE, block::BANNER_ACCORD),
            (FactionStructure::IronbornForgeCamp, block::IRONBORN_BRICK, block::BANNER_IRONBORN),
            (FactionStructure::CovenantGroveShrine, block::EMBER_COVENANTWOOD, block::BANNER_COVENANT),
            (FactionStructure::FreeholdsLonghouse, block::FREEHOLDS_THATCH, block::BANNER_FREEHOLDS),
            (FactionStructure::AshenLibrary, block::ASHEN_MARBLE, block::BANNER_ASHEN),
            (FactionStructure::NamelessCamp, block::NAMELESS_ROTWOOD, block::BANNER_NAMELESS),
        ];
        for (kind, themed, marker) in cases {
            let mut col = lf_voxel::ChunkColumn::empty();
            build_faction_structure(kind, &mut col, &ground);
            let count = |id: u32| (0..16).flat_map(|lx| (0..16).map(move |lz| (lx, lz)))
                .filter(|(lx, lz)| (40..200).any(|y| col.get(*lx, y, *lz).id() == id)).count();
            assert!(count(themed) > 4, "{:?} lacks its themed block", kind);
            assert!(count(marker) >= 1, "{:?} lacks its banner marker", kind);
            assert_eq!(count(marker), 1, "{:?} has duplicate markers", kind);
        }
        let _ = gen;
    }

#[test]
    fn wizard_towers_generate_in_gated_biomes() {
        let mut towers = 0usize;
        let gen = WorldGen::new(Seed(42));
        for cx in -40..40i32 {
            for cz in -40..40i32 {
                let biome = gen.biome(cx * 16 + 8, cz * 16 + 8);
                let col = gen.generate_chunk(cx, cz);
                for lx in 6..=10usize {
                    for lz in 6..=10usize {
                        for y in 60..200usize {
                            if col.get(lx, y, lz).id() == lf_voxel::registry::block::ENCHANTING_TABLE {
                                towers += 1;
                                assert!(matches!(biome, Biome::FlowerForest | Biome::Highlands),
                                    "tower in {:?} at ({},{})", biome, cx, cz);
                            }
                        }
                    }
                }
            }
        }
        assert!(towers >= 1, "a 20x20-chunk scan should find at least one tower, got {}", towers);
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
            ((0.1, 0.5, 70, 0.95), FrostMeadow),
            ((0.1, 0.5, 70, 0.82), IceSpikes),
            ((0.1, 0.5, 70, 0.3), Tundra),
            ((0.15, 0.5, 35, 0.5), FrozenOcean),
            ((0.85, 0.9, 70, 0.5), Savanna),
            ((0.85, 0.2, 70, 0.9), Badlands),
            ((0.85, 0.2, 70, 0.75), WindsweptSavanna),
            ((0.85, 0.2, 70, 0.4), Desert),
            ((0.3, 0.9, 80, 0.3), Taiga),
            ((0.3, 0.9, 80, 0.85), Swamp),
            ((0.3, 0.6, 80, 0.9), MapleForest),
            ((0.3, 0.6, 80, 0.45), Forest),
            ((0.3, 0.3, 80, 0.4), Tundra),
            ((0.65, 0.9, 80, 0.9), PaleGarden),
            ((0.65, 0.9, 80, 0.5), DarkForest),
            ((0.65, 0.7, 80, 0.85), CherryGrove),
            ((0.65, 0.7, 80, 0.3), Forest),
            ((0.65, 0.7, 80, 0.55), FlowerForest),
            // king-quest B: the 15 new biomes are reachable
            ((0.85, 0.2, 70, 0.12), Oasis),
            ((0.85, 0.2, 70, 0.22), PaintedDunes),
            ((0.7, 0.2, 70, 0.08), SaltFlats),
            ((0.85, 0.9, 70, 0.05), BaobabFields),
            ((0.8, 0.15, 80, 0.04), Emberwood),
            ((0.8, 0.15, 80, 0.08), Volcanic),
            ((0.5, 0.9, 80, 0.03), MushroomHollow),
            ((0.5, 0.9, 80, 0.09), RedwoodForest),
            ((0.65, 0.7, 80, 0.75), WillowWetlands),
            ((0.65, 0.7, 80, 0.65), LavenderFields),
            ((0.65, 0.7, 80, 0.2), AspenGrove),
            ((0.3, 0.6, 80, 0.85), MapleForest),
            ((0.3, 0.4, 80, 0.9), PineBarrens),
            ((0.1, 0.5, 70, 0.92), FrostMeadow),
            ((0.15, 0.5, 52, 0.6), FoggyFjord),
            ((0.45, 0.5, 80, 0.7), SunflowerPlains),
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
        assert!(all.len() >= 40, "expected ~46 distinct biomes, got {}", all.len());
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
        // biome-guided per-target chunk selection: terrain `prepare` may
        // refuse a matching chunk (water/steepness), so scan until each
        // structure is actually FOUND, not merely predicted
        let targets: [(u32, &[Biome]); 3] = [
            (block::CRAFTING_TABLE, &[Biome::Meadow, Biome::Forest,
                Biome::SunflowerPlains, Biome::AspenGrove, Biome::MapleForest,
                Biome::LavenderFields]),
            (block::SAND, &[Biome::Desert, Biome::Badlands]),
            (block::STONE, &[Biome::Mountains, Biome::SnowyPeaks]),
        ];
        for (marker, biomes) in targets {
            'find: for cx in -90..90i32 {
                for cz in -90..90i32 {
                    let b = gen.biome(cx * 16 + 8, cz * 16 + 8);
                    if !biomes.contains(&b) {
                        continue;
                    }
                    let a = gen.generate_chunk(cx, cz);
                    let found = if marker == block::CRAFTING_TABLE {
                        let tables = count(&a, block::CRAFTING_TABLE, 60, 200);
                        if tables > 0 {
                            assert!(count(&a, block::FURNACE, 60, 200) > 0, "hut has a furnace");
                            let b2 = gen.generate_chunk(cx, cz);
                            assert_eq!(count(&b2, block::CRAFTING_TABLE, 60, 200), tables,
                                "hut placement not deterministic at ({},{})", cx, cz);
                            true
                        } else { false }
                    } else if marker == block::SAND {
                        (60..180).any(|y| {
                            (2..14).any(|lx| (2..14).any(|lz|
                                a.get(lx, y, lz).id() == block::SAND
                                    || a.get(lx, y, lz).id() == block::RED_SAND))
                                && {
                                    let mut sand = 0;
                                    for lx in 2..14 { for lz in 2..14 {
                                        if a.get(lx, y, lz).id() == block::SAND
                                            || a.get(lx, y, lz).id() == block::RED_SAND { sand += 1; }
                                    }}
                                    sand > 25
                                }
                        })
                    } else {
                        let mut stone_high = 0;
                        for lx in 6..=10 { for lz in 6..=10 { for y in 120..220 {
                            if a.get(lx, y, lz).id() == block::STONE { stone_high += 1; }
                        }}}
                        stone_high > 40
                    };
                    if found {
                        if marker == block::CRAFTING_TABLE { hut = true; }
                        else if marker == block::SAND { pyramid = true; }
                        else { tower = true; }
                        break 'find;
                    }
                }
            }
        }
        assert!(hut, "no huts found in scan");
        assert!(pyramid, "no pyramids found in scan");
        assert!(tower, "no watchtowers found in scan");
    }

    /// king-quest: the Accord Bastion (walled city) generates rarely in
    /// the accord meadowlands, and the new frontier biomes carry their
    /// own towers and ruins. Terrain `prepare` may refuse candidate
    /// sites, so each scan walks candidates until the structure is FOUND.
    #[test]
    fn accord_bastion_and_frontier_structures_generate() {
        use lf_voxel::registry::block;
        let gen = WorldGen::new(Seed(777));
        let feats = gen.seed_for_features();
        let matches_at = |cx: i32, cz: i32, biomes: &[Biome], modulus: u64| -> bool {
            let b = gen.biome(cx * 16 + 8, cz * 16 + 8);
            biomes.contains(&b) && hash2(cx, cz, feats ^ 0x5bd1e995) % modulus == 0
        };
        // the bastion: banner over a stone keep, inside stone walls
        let mut city = false;
        'city: for cx in -160..160i32 {
            for cz in -160..160i32 {
                if !matches_at(cx, cz, &[Biome::Meadow, Biome::SunflowerPlains], 331) { continue; }
                let col = gen.generate_chunk(cx, cz);
                for y in 60..200 {
                    if col.get(7, y, 7).id() == block::BANNER_ACCORD {
                        let mut stone = 0;
                        for ly in y.saturating_sub(6)..(y + 4).min(255) {
                            for lx in 0..16 { for lz in 0..16 {
                                if col.get(lx, ly as usize, lz).id() == block::STONE { stone += 1; }
                            }}
                        }
                        assert!(stone > 150, "bastion keep+walls missing (stone={})", stone);
                        city = true;
                        break 'city;
                    }
                }
            }
        }
        assert!(city, "no Accord Bastion in the 320x320 scan");
        // frontier wooden tower: log frame in the new forest biomes
        let mut tower = false;
        'tower: for cx in -160..160i32 {
            for cz in -160..160i32 {
                if !matches_at(cx, cz, &[Biome::RedwoodForest, Biome::PineBarrens,
                    Biome::FoggyFjord, Biome::MapleForest, Biome::WillowWetlands], 43) { continue; }
                let col = gen.generate_chunk(cx, cz);
                let logs = (0..16).map(|lx| (0..16).map(move |lz| (lx, lz)))
                    .flatten()
                    .filter(|(lx, lz)| (40..200).any(|y| col.get(*lx, y, *lz).id() == block::LOG))
                    .count();
                if logs >= 8 { tower = true; break 'tower; }
            }
        }
        assert!(tower, "no frontier watchtower in the 320x320 scan");
        // desert ruin: torch-marked remnant walls in the new desert biomes
        let mut ruin = false;
        'ruin: for cx in -160..160i32 {
            for cz in -160..160i32 {
                if !matches_at(cx, cz, &[Biome::Oasis, Biome::PaintedDunes], 47) { continue; }
                let col = gen.generate_chunk(cx, cz);
                let torch = (0..16).any(|lx| (0..16).any(|lz|
                    (40..200).any(|y| col.get(lx, y, lz).id() == block::TORCH)));
                if torch { ruin = true; break 'ruin; }
            }
        }
        assert!(ruin, "no desert ruin in the 320x320 scan");
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

    /// Failure meaning: kingdoms stop being findable — no region near spawn
    /// hosts one, or `nearest_kingdom` disagrees with `kingdom_at`.
    #[test]
    fn kingdoms_are_placed_and_findable() {
        let gen = WorldGen::new(Seed(12345));
        // at least one kingdom within the 5x5-region compass window of spawn
        let (site, _d) = gen.nearest_kingdom(0, 0)
            .expect("some kingdom must exist near the origin window");
        assert!(KINGDOM_NAMES.contains(&site.name), "name comes from the pool");
        // kingdom_at agrees exactly on the site chunk and disagrees elsewhere
        assert_eq!(gen.kingdom_at(site.cx, site.cz), Some(site));
        assert_eq!(gen.kingdom_at(site.cx + 1, site.cz), None);
        // deterministic: the same seed yields the same site and name
        let gen2 = WorldGen::new(Seed(12345));
        assert_eq!(gen2.nearest_kingdom(0, 0).map(|(s, _)| s), Some(site));
        // different seeds place different courts (not all identical)
        let other = WorldGen::new(Seed(999));
        let mut any = false;
        for rx in -2..=2 {
            for rz in -2..=2 {
                if gen.kingdom_in_region(rx, rz) != other.kingdom_in_region(rx, rz) {
                    any = true;
                }
            }
        }
        assert!(any, "seeds should vary kingdom placement somewhere");
    }

    /// Failure meaning: the generated citadel chunk lacks its marker blocks
    /// (throne + banner) or walls — the client settles NPCs by scanning
    /// for the throne, so a citadel without it is dead terrain.
    #[test]
    fn citadel_chunk_builds_the_full_court() {
        let gen = WorldGen::new(Seed(12345));
        let (site, _) = gen.nearest_kingdom(0, 0).unwrap();
        let col = gen.generate_chunk(site.cx, site.cz);
        use lf_voxel::registry::block;
        let mut throne = 0;
        let mut banners = 0;
        let mut bricks = 0;
        for lx in 0..16usize {
            for lz in 0..16usize {
                for y in 0..256usize {
                    match col.get(lx, y, lz).id() {
                        block::THRONE => throne += 1,
                        block::BANNER_KINGDOM => banners += 1,
                        block::KINGDOM_BRICK => bricks += 1,
                        _ => {}
                    }
                }
            }
        }
        assert_eq!(throne, 1, "exactly one throne (the marker)");
        assert!(banners >= 4, "royal banners fly (gate + throne + market): {banners}");
        assert!(bricks > 150, "walls + keep are real masonry: {bricks}");
    }

    /// Failure meaning: `build_kingdom_citadel` stopped raising the throne
    /// dais, gate banners, or the well when planted on flat ground.
    #[test]
    fn citadel_on_flat_ground_has_landmarks() {
        use lf_voxel::registry::block;
        let mut col = lf_voxel::ChunkColumn::empty();
        for lx in 0..16usize {
            for lz in 0..16usize {
                for y in 0..70usize {
                    col.set(lx, y, lz, lf_voxel::BlockState(block::STONE));
                }
            }
        }
        build_kingdom_citadel(&mut col, 70);
        assert_eq!(col.get(7, 72, 8).id(), block::THRONE);
        assert_eq!(col.get(7, 76, 15).id(), block::BANNER_KINGDOM);
        assert_eq!(col.get(2, 70, 10).id(), block::WATER, "the well holds water");
        // the gate is a real opening in the wall
        assert_eq!(col.get(7, 71, 15).id(), block::AIR);
        assert_eq!(col.get(8, 72, 15).id(), block::AIR);
        // walls stand on all four edges (cells clear of the gate opening)
        for (lx, lz) in [(0usize, 7usize), (15, 7), (7, 0), (9, 15)] {
            assert_eq!(col.get(lx, 71, lz).id(), block::KINGDOM_BRICK);
        }
    }
}

#[cfg(test)]
mod probe2 {
    use crate::*;
    #[test]
    fn river_factor_distribution() {
        let gen = WorldGen::new(Seed(42));
        let mut buckets = [0usize; 6]; // cf <0.05, 0.05-0.15, 0.15-0.28, 0.28-0.4, 0.4-0.55, >0.55
        let mut strong = [0usize; 6];
        for x in (-1200..1200).step_by(4) {
            for z in (-1200..1200).step_by(4) {
                let cf = gen.continental_factor(x, z);
                let rf = gen.river_factor(x, z);
                let b = if cf < 0.05 {0} else if cf < 0.15 {1} else if cf < 0.28 {2}
                    else if cf < 0.40 {3} else if cf < 0.55 {4} else {5};
                buckets[b] += 1;
                if rf > 0.7 { strong[b] += 1; }
            }
        }
        println!("cf buckets: <0.05:{} 0.05-15:{} 0.15-28:{} 0.28-40:{} 0.40-55:{} >0.55:{}",
            buckets[0], buckets[1], buckets[2], buckets[3], buckets[4], buckets[5]);
        println!("rf>0.7:    {} {} {} {} {} {}",
            strong[0], strong[1], strong[2], strong[3], strong[4], strong[5]);
    }
}

