use fastnoise_lite::{FastNoiseLite, NoiseType, FractalType};

/// Deterministic world generation seed across all platforms.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Seed(pub u64);

/// Biomes (v1) from the spec.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

    /// Height at chunk column (x,z) in blocks 16..256+.
    pub fn height(&self, cx: i32, cz: i32) -> i32 {
        let n = self.noise_base.get_noise_2d(cx as f32, cz as f32);
        let scale = (n + 1.0) * 0.5; // 0..1
        let base = 64;
        let amp = 128;
        (base + (scale * amp as f32).round() as i32).max(16)
    }

    /// Temperature [0..1].
    pub fn temperature(&self, cx: i32, cz: i32) -> f32 {
        (self.noise_temp.get_noise_2d(cx as f32, cz as f32) + 1.0) * 0.5
    }

    /// Humidity [0..1].
    pub fn humidity(&self, cx: i32, cz: i32) -> f32 {
        (self.noise_humid.get_noise_2d(cx as f32, cz as f32) + 1.0) * 0.5
    }

    /// Biome from temperature/humidity. Deterministic across platforms.
    pub fn biome(&self, cx: i32, cz: i32) -> Biome {
        let t = self.temperature(cx, cz);
        let h = self.humidity(cx, cz);
        if t < 0.25 {
            Biome::Tundra
        } else if t > 0.7 && h < 0.4 {
            Biome::Desert
        } else if h > 0.85 {
            Biome::MushroomHollow
        } else {
            Biome::Meadow
        }
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
            Biome::Ocean | Biome::DeepOcean => BlockId::STONE,
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
}
