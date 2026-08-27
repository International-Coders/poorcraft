use serde::{Serialize, Deserialize};

pub mod survival;
pub mod smithing;
pub mod mobs;
pub mod player;
pub mod items;
pub mod mining;
pub mod crafting;
pub mod smelting;
pub mod combat;
pub mod machines;
pub mod research;
pub mod fluids;
pub mod magic;
pub mod construction;
pub mod building;
pub mod dragons;
pub mod paths;
pub mod companions;

/// Game time with a 20-minute day/night cycle as per spec.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeOfDay {
    pub ticks: u64, // increments every 1/20th second = 72000 ticks per 60 min
}

impl TimeOfDay {
    pub const TICKS_PER_DAY: u64 = 72000; // 20 minutes real time at 20 ticks/sec
    pub const TICKS_PER_SECOND: u64 = 20;

    pub fn new(ticks: u64) -> Self {
        Self { ticks: ticks % Self::TICKS_PER_DAY }
    }

    pub fn from_fraction(frac: f32) -> Self {
        Self {
            ticks: ((frac * Self::TICKS_PER_DAY as f32).floor() as u64) % Self::TICKS_PER_DAY,
        }
    }

    pub fn fraction(&self) -> f32 {
        self.ticks as f32 / Self::TICKS_PER_DAY as f32
    }

    pub fn is_day(&self) -> bool {
        let f = self.fraction();
        f > 0.2 && f < 0.8
    }

    pub fn is_night(&self) -> bool {
        !self.is_day()
    }

    pub fn sun_angle(&self) -> f32 {
        (self.fraction() * 360.0 - 90.0).to_radians().sin()
    }

    /// Sky light level [0.0..1.0] - brighter during day, darker at night.
    pub fn sky_light_level(&self) -> f32 {
        let day_brightness = if self.is_day() {
            // Peak at noon (0.5)
            let noon_dist = (self.fraction() - 0.5).abs();
            1.0 - noon_dist * 0.4 // ranges 0.8..1.0 during day
        } else {
            // Night: starlight level
            0.12
        };
        day_brightness
    }

    /// Sky color RGB at current time (day=sky blue, night=darker blue).
    pub fn sky_color(&self) -> [f32; 3] {
        let t = self.sky_light_level();
        // Day: sky blue (0.53, 0.81, 0.98)
        // Night: dark blue (0.05, 0.07, 0.15)
        let day_color = [0.53, 0.81, 0.98];
        let night_color = [0.05, 0.07, 0.15];
        if self.is_day() {
            [
                day_color[0] * t + night_color[0] * (1.0 - t),
                day_color[1] * t + night_color[1] * (1.0 - t),
                day_color[2] * t + night_color[2] * (1.0 - t),
            ]
        } else {
            [
                day_color[0] * 0.15 + night_color[0] * 0.85,
                day_color[1] * 0.15 + night_color[1] * 0.85,
                day_color[2] * 0.15 + night_color[2] * 0.85,
            ]
        }
    }
}

/// Block light emitter levels.
pub const TORCH_LIGHT: u8 = 14;
pub const LANTERN_LIGHT: u8 = 15;
pub const LAVA_LIGHT: u8 = 15;
pub const GLOW_CRYSTAL_LIGHT: u8 = 12;

#[derive(Clone, Debug)]
pub struct LightEngine {
    pub sky_light: Vec<u8>,
    pub block_light: Vec<u8>,
    width: usize,
    height: usize,
    depth: usize,
}

impl LightEngine {
    pub fn new(width: usize, height: usize, depth: usize) -> Self {
        Self {
            sky_light: vec![15u8; width * height * depth],
            block_light: vec![0u8; width * height * depth],
            width,
            height,
            depth,
        }
    }

    pub fn set_block_light(&mut self, x: usize, y: usize, z: usize, level: u8) {
        let idx = x + y * self.width + z * self.width * self.height;
        if idx < self.block_light.len() {
            self.block_light[idx] = level;
        }
    }

    pub fn get_block_light(&self, x: usize, y: usize, z: usize) -> u8 {
        let idx = x + y * self.width + z * self.width * self.height;
        self.block_light.get(idx).copied().unwrap_or(0)
    }

    pub fn apply_sky(&mut self, time: &TimeOfDay) {
        let sky = (time.sky_light_level() * 15.0) as u8;
        for v in &mut self.sky_light {
            *v = sky;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_night_cycle() {
        let day = TimeOfDay::new(36000); // noon-ish
        assert!(day.is_day());
        let night = TimeOfDay::new(65000); // midnight-ish
        assert!(night.is_night());
        assert!(day.sky_light_level() > night.sky_light_level());
    }

    #[test]
    fn test_sky_color_day_night() {
        let day = TimeOfDay::new(36000);
        let night = TimeOfDay::new(65000);
        let day_color = day.sky_color();
        let night_color = night.sky_color();
        assert!(day_color[2] > night_color[2]); // blue component
    }

    #[test]
    fn test_light_engine() {
        let mut le = LightEngine::new(4, 4, 4);
        le.set_block_light(0, 0, 0, 14);
        assert_eq!(le.get_block_light(0, 0, 0), 14);
        assert_eq!(le.get_block_light(1, 0, 0), 0);
    }
}
