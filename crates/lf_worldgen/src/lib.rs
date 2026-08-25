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

        Self {
            noise_base: base,
            noise_temp: temp,
            noise_humid: humid,
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
}

#[cfg(test)]
mod window_probe {
    use super::*;
    #[test]
    fn window_heights() {
        for s in [1u64, 2, 3, 12345] {
            let g = WorldGen::new(Seed(s));
            let mut min = i32::MAX; let mut max = i32::MIN; let mut sum = 0i64; let n = (2*16+16)*(2*16+16);
            for x in -16..32 { for z in -16..32 { let h = g.height(x, z); min=min.min(h); max=max.max(h); sum += h as i64; } }
            println!("SEED {} min {} max {} avg {:.1}", s, min, max, sum as f64 / n as f64);
        }
    }
}
