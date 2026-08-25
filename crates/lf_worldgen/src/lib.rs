use fastnoise_lite::{FastNoiseLite, NoiseType, FractalType};

/// Deterministic world generation seed across all platforms.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Seed(pub u64);

/// Biomes (v1) from the spec.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Biome {
    Ocean,
    DeepOcean,
    Meadow,
    Desert,
    Tundra,
    MushroomHollow,
    Highlands,
    Mountains,
}

/// A block position in world space.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Water surface height used by terrain generation.
pub const SEA_LEVEL: i32 = 62;

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
    noise_base: FastNoiseLite,
    noise_temp: FastNoiseLite,
    noise_humid: FastNoiseLite,
    noise_cave: FastNoiseLite,
    noise_ore: FastNoiseLite,
}

impl WorldGen {
    pub fn new(seed: Seed) -> Self {
        let mut base = FastNoiseLite::new();
        base.set_seed(Some(seed.0 as i32));
        base.set_noise_type(Some(NoiseType::Perlin));
        base.set_fractal_type(Some(FractalType::FBm));
        base.set_frequency(Some(0.01));

        let mut temp = FastNoiseLite::new();
        temp.set_seed(Some(seed.0.wrapping_add(7) as i32));
        temp.set_noise_type(Some(NoiseType::Perlin));
        temp.set_frequency(Some(0.005));

        let mut humid = FastNoiseLite::new();
        humid.set_seed(Some(seed.0.wrapping_add(13) as i32));
        humid.set_noise_type(Some(NoiseType::Perlin));
        humid.set_frequency(Some(0.006));

        let mut cave = FastNoiseLite::new();
        cave.set_seed(Some(seed.0.wrapping_add(101) as i32));
        cave.set_noise_type(Some(NoiseType::Perlin));
        cave.set_fractal_type(Some(FractalType::FBm));
        cave.set_frequency(Some(0.03));

        let mut ore = FastNoiseLite::new();
        ore.set_seed(Some(seed.0.wrapping_add(211) as i32));
        ore.set_noise_type(Some(NoiseType::Perlin));
        ore.set_frequency(Some(0.09));

        Self {
            noise_base: base,
            noise_temp: temp,
            noise_humid: humid,
            noise_cave: cave,
            noise_ore: ore,
        }
    }

    /// Height at chunk column (x,z) in blocks. Range spans below and above
    /// sea level so ocean and mountain biomes are both reachable.
    pub fn height(&self, cx: i32, cz: i32) -> i32 {
        let n = self.noise_base.get_noise_2d(cx as f32, cz as f32);
        let scale = (n + 1.0) * 0.5; // 0..1, but FBM stays near the middle
        // Stretch the occupied band so real oceans and peaks exist.
        let stretched = (scale * 1.43 - 0.21).clamp(0.0, 1.0);
        let base = 24;
        let amp = 152;
        (base + (stretched * amp as f32).round() as i32).max(8)
    }

    /// Temperature [0..1].
    pub fn temperature(&self, cx: i32, cz: i32) -> f32 {
        (self.noise_temp.get_noise_2d(cx as f32, cz as f32) + 1.0) * 0.5
    }

    /// Humidity [0..1].
    pub fn humidity(&self, cx: i32, cz: i32) -> f32 {
        (self.noise_humid.get_noise_2d(cx as f32, cz as f32) + 1.0) * 0.5
    }

    /// Biome at column, combining elevation with temperature/humidity.
    /// Deterministic across platforms.
    pub fn biome(&self, cx: i32, cz: i32) -> Biome {
        biome_from(self.temperature(cx, cz), self.humidity(cx, cz), self.height(cx, cz))
    }

    /// Surface block at column position.
    pub fn surface_block(&self, cx: i32, cz: i32) -> BlockId {
        let b = self.biome(cx, cz);
        match b {
            Biome::Tundra => BlockId::STONE,
            Biome::Desert => BlockId::SAND,
            Biome::Meadow => BlockId::GRASS,
            Biome::MushroomHollow => BlockId::MYCELIUM,
            Biome::Highlands => BlockId::GRASS,
            Biome::Mountains => BlockId::STONE,
            Biome::Ocean | Biome::DeepOcean => BlockId::SAND,
        }
    }

    /// Column of blocks from surface down to bedrock.
    pub fn column(&self, cx: i32, cz: i32) -> Vec<(i32, BlockId)> {
        let height = self.height(cx, cz);
        let surf_block = self.surface_block(cx, cz);
        let mut col = Vec::new();
        for y in (0..=height + 16).rev() {
            if y <= height + 3 && y >= height - 2 {
                col.push((y, surf_block));
            } else if y == height - 3 {
                col.push((y, BlockId::DIRT));
            } else if y < height - 3 {
                col.push((y, BlockId::STONE));
            } else {
                col.push((y, BlockId::AIR));
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

/// Pure biome classification from temperature t [0..1], humidity h [0..1],
/// and terrain height in blocks. Exposed for tests and map preview tools.
pub fn biome_from(t: f32, h: f32, height: i32) -> Biome {
    if height >= 140 {
        Biome::Mountains
    } else if height >= 110 {
        Biome::Highlands
    } else if height < 42 {
        Biome::DeepOcean
    } else if height < SEA_LEVEL - 6 {
        Biome::Ocean
    } else if t < 0.25 {
        Biome::Tundra
    } else if t > 0.7 && h < 0.4 {
        Biome::Desert
    } else if h > 0.85 {
        Biome::MushroomHollow
    } else {
        Biome::Meadow
    }
}

/// Voxel BlockState ids matching this crate's BlockId mapping.
fn block_state_of(b: BlockId) -> lf_voxel::BlockState {
    use lf_voxel::BlockState;
    match b {
        BlockId::AIR => BlockState::AIR,
        BlockId::STONE => BlockState::STONE,
        BlockId::DIRT => BlockState::DIRT,
        BlockId::GRASS => BlockState::GRASS,
        BlockId::SAND => BlockState(4),
        BlockId::MYCELIUM => BlockState(5),
        BlockId::SNOW => BlockState(6),
        BlockId(_) => BlockState::AIR,
    }
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
                    if b != BlockId::AIR {
                        col.set(lx, wy as usize, lz, block_state_of(b));
                    }
                }
                surface_tops[lx][lz] = self.surface_top(wx, wz);
            }
        }

        // 2. Caves: 3D noise carving. Never below y=6 (bedrock-ish floor) and
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
                }
            }
        }

        // 4. Water fills open space up to sea level (oceans and lakes).
        for lx in 0..16usize {
            for lz in 0..16usize {
                let top = surface_tops[lx][lz];
                if top <= SEA_LEVEL {
                    for y in top..=SEA_LEVEL {
                        if col.get(lx, y as usize, lz) == BlockState::AIR {
                            col.set(lx, y as usize, lz, BlockState(block::WATER));
                        }
                    }
                }
            }
        }

        // 5. Trees on grass meadow columns, canopy kept inside the chunk.
        for lx in 2..14usize {
            for lz in 2..14usize {
                let wx = cx * 16 + lx as i32;
                let wz = cz * 16 + lz as i32;
                let top = surface_tops[lx][lz];
                if top <= SEA_LEVEL + 1
                    || self.biome(wx, wz) != Biome::Meadow
                    || col.get(lx, (top - 1) as usize, lz) != BlockState::GRASS
                {
                    continue;
                }
                let h = hash2(wx, wz, self.seed_for_features());
                if h % 48 != 0 {
                    continue;
                }
                let trunk = 4 + ((h / 48) % 3) as i32;
                // Re-check the surface was not carved into a cave mouth.
                if col.get(lx, (top - 1) as usize, lz) != BlockState::GRASS {
                    continue;
                }
                for y in top..top + trunk {
                    col.set(lx, y as usize, lz, BlockState(block::LOG));
                }
                let base = top + trunk - 1;
                for dy in -2i32..=0i32 {
                    let r: i32 = if dy < 0 { 2 } else { 1 };
                    for dx in -r..=r {
                        for dz in -r..=r {
                            if dy == 0 && dx == 0 && dz == 0 {
                                continue;
                            }
                            // trim corners for a rounder canopy
                            if dx.abs() == 2 && dz.abs() == 2 {
                                continue;
                            }
                            let px = (lx as i32 + dx) as usize;
                            let pz = (lz as i32 + dz) as usize;
                            let py = (base + dy) as usize;
                            if col.get(px, py, pz) == BlockState::AIR {
                                col.set(px, py, pz, BlockState(block::LEAVES));
                            }
                        }
                    }
                }
                col.set(lx, (base + 1) as usize, lz, BlockState(block::LEAVES));
            }
        }

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

    #[test]
    fn deterministic() {
        let a = WorldGen::new(Seed(42));
        let b = WorldGen::new(Seed(42));
        assert_eq!(a.height(10, 20), b.height(10, 20));
        assert_eq!(a.temperature(10, 20), b.temperature(10, 20));
        assert_eq!(a.humidity(10, 20), b.humidity(10, 20));
        assert_eq!(a.biome(10, 20), b.biome(10, 20));
    }

    #[test]
    fn biome_consistency() {
        let a = WorldGen::new(Seed(100));
        let b = WorldGen::new(Seed(100));
        assert_eq!(a.biome(0, 0), b.biome(0, 0));
    }

    #[test]
    fn every_biome_variant_reachable() {
        use Biome::*;
        assert_eq!(biome_from(0.5, 0.5, 150), Mountains);
        assert_eq!(biome_from(0.5, 0.5, 120), Highlands);
        assert_eq!(biome_from(0.5, 0.5, 30), DeepOcean);
        assert_eq!(biome_from(0.5, 0.5, 50), Ocean);
        assert_eq!(biome_from(0.1, 0.5, 70), Tundra);
        assert_eq!(biome_from(0.9, 0.2, 70), Desert);
        assert_eq!(biome_from(0.5, 0.9, 70), MushroomHollow);
        assert_eq!(biome_from(0.5, 0.5, 70), Meadow);
    }

    #[test]
    fn all_biomes_appear_across_sampled_world() {
        use std::collections::HashSet;
        let gen = WorldGen::new(Seed(42));
        let mut seen = HashSet::new();
        for x in (-1024..1024).step_by(16) {
            for z in (-1024..1024).step_by(16) {
                seen.insert(gen.biome(x, z));
            }
        }
        assert_eq!(seen.len(), 8, "expected all 8 biomes, saw {:?}", seen);
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
                        for y in 40..200 {
                            let b = col.get(lx, y, lz).id();
                            if b == block::LOG { logs += 1; }
                            if b == block::LEAVES { leaves += 1; }
                        }
                    }
                }
            }
        }
        assert!(logs > 20, "expected trees, found {} logs", logs);
        assert!(leaves > logs, "canopy should outnumber trunks: {} leaves vs {} logs", leaves, logs);
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
    fn water_fills_oceans() {
        use lf_voxel::registry::block;
        let gen = WorldGen::new(Seed(12345));
        let mut found_water = 0usize;
        'outer: for cx in -16..=16 {
            for cz in -16..=16 {
                if gen.surface_top(cx * 16, cz * 16) >= SEA_LEVEL {
                    continue;
                }
                let col = gen.generate_chunk(cx, cz);
                for lx in 0..16 {
                    for lz in 0..16 {
                        for y in 30..=SEA_LEVEL as usize {
                            if col.get(lx, y, lz).id() == block::WATER {
                                found_water += 1;
                            }
                        }
                    }
                }
                break 'outer;
            }
        }
        assert!(found_water > 100, "expected ocean water, found {} blocks", found_water);
    }
}
