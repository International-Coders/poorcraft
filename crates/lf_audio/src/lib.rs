//! Procedural audio (build-pack Step 4 / V1REBRAND P38): break/place
//! feedback per material category, synthesized as PCM at boot — no asset
//! files, the same philosophy as the procedural textures. Playback goes
//! through rodio and degrades to silence when no output device exists
//! (headless CI, broken audio), so the game logic never has to care.

use std::time::Instant;

/// Material family a block belongs to for sound purposes. The mapping is
/// pure and unit-tested — the goal's acceptance check is exactly this
/// dispatch being right.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Category {
    Wood,
    Stone,
    Metal,
    Glass,
    Soft,
}

/// What happened to the block.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Break,
    Place,
}

const SAMPLE_RATE: u32 = 22_050;

/// Deterministic LCG noise so synthesized sounds (and the tests that
/// inspect them) are stable across runs.
struct Noise(u64);

impl Noise {
    fn new(seed: u64) -> Self {
        Noise(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1)
    }
    fn next_f32(&mut self) -> f32 {
        // xorshift64 -> [0,1)
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 11) as f32 / (1u64 << 53) as f32
    }
}

/// One synthesized one-shot: a mix of a decaying tone and a filtered noise
/// burst, colored per category. Returns mono f32 samples in [-1, 1].
pub fn synth(category: Category, action: Action) -> Vec<f32> {
    let (dur_ms, tone_hz, tone_mix, noise_bright, punch) = match (category, action) {
        (Category::Wood, Action::Break) => (180.0, 170.0, 0.45, 0.35, 0.9),
        (Category::Wood, Action::Place) => (90.0, 220.0, 0.5, 0.3, 0.7),
        (Category::Stone, Action::Break) => (150.0, 320.0, 0.25, 0.55, 1.0),
        (Category::Stone, Action::Place) => (80.0, 380.0, 0.3, 0.5, 0.75),
        (Category::Metal, Action::Break) => (260.0, 1150.0, 0.6, 0.45, 0.85),
        (Category::Metal, Action::Place) => (120.0, 980.0, 0.55, 0.4, 0.6),
        (Category::Glass, Action::Break) => (220.0, 2600.0, 0.35, 0.95, 1.0),
        (Category::Glass, Action::Place) => (70.0, 2100.0, 0.3, 0.9, 0.5),
        (Category::Soft, Action::Break) => (120.0, 120.0, 0.3, 0.18, 0.7),
        (Category::Soft, Action::Place) => (70.0, 150.0, 0.35, 0.15, 0.55),
    };
    let n = (SAMPLE_RATE as f32 * dur_ms / 1000.0) as usize;
    let mut noise = Noise::new(category as u64 * 7919 + action as u64 * 104729 + 12345);
    let mut out = Vec::with_capacity(n);
    let mut lp = 0.0f32; // one-pole low-pass state for the noise path
    for i in 0..n {
        let t = i as f32 / n as f32;
        let env = (1.0 - t) * (1.0 - t) * punch; // quadratic decay
        let tone = (i as f32 * tone_hz / SAMPLE_RATE as f32 * std::f32::consts::TAU).sin();
        // brightness = how much raw noise passes vs the smoothed version
        let raw = noise.next_f32() * 2.0 - 1.0;
        lp += (raw - lp) * (0.08 + 0.6 * noise_bright);
        let s = (tone * tone_mix + lp * (1.0 - tone_mix)) * env * 0.5;
        out.push(s.clamp(-1.0, 1.0));
    }
    out
}

/// Scale a sample buffer by the effective volume (master x sfx).
pub fn scaled(samples: &[f32], volume: f32) -> Vec<f32> {
    samples.iter().map(|s| s * volume.clamp(0.0, 1.0)).collect()
}

/// Which material family a block sounds like.
pub fn block_category(block_id: u32) -> Category {
    use lf_voxel::registry::block as b;
    match block_id {
        b::LOG | b::PLANKS | b::CRAFTING_TABLE | b::LEAVES | b::BIRCH_LEAVES
        | b::SPRUCE_LEAVES | b::DARK_LEAVES | b::CHERRY_LEAVES | b::PALE_LEAVES => Category::Wood,
        b::GLASS | b::ICE => Category::Glass,
        b::IRON_ORE | b::COPPER_ORE | b::TIN_ORE | b::COAL_GENERATOR | b::ELECTRIC_FURNACE
        | b::CRUSHER | b::ASSEMBLER | b::RESEARCH_BENCH | b::SMITHING_TABLE => Category::Metal,
        b::SAND | b::RED_SAND | b::DIRT | b::GRASS | b::MOSS | b::MYCELIUM | b::SNOW
            => Category::Soft,
        _ => Category::Stone,
    }
}

/// Live player: owns the output stream's mixer; `None` when the machine
/// has no usable audio output (silent fallback — gameplay never blocks).
pub struct Audio {
    mixer: rodio::mixer::Mixer,
    _sink: rodio::MixerDeviceSink,
    last: Instant,
}

impl Audio {
    pub fn new() -> Option<Self> {
        let mut sink = rodio::DeviceSinkBuilder::open_default_sink().ok()?;
        sink.log_on_drop(false);
        let mixer = sink.mixer().clone();
        Some(Self { mixer, _sink: sink, last: Instant::now() - std::time::Duration::from_secs(1) })
    }

    /// Play a one-shot. Rate-limited so a burst of edits (water sim, sand
    /// collapse) cannot machine-gun the mixer.
    pub fn play(&mut self, category: Category, action: Action, volume: f32) {
        if volume <= 0.01 {
            return;
        }
        let now = Instant::now();
        if now.duration_since(self.last).as_millis() < 30 {
            return;
        }
        self.last = now;
        let samples = scaled(&synth(category, action), volume);
        let player = rodio::Player::connect_new(&self.mixer);
        player.append(rodio::buffer::SamplesBuffer::new(
            std::num::NonZero::<u16>::new(1).unwrap(),
            std::num::NonZero::<u32>::new(SAMPLE_RATE).unwrap(),
            samples,
        ));
        player.play();
        // the player handle drops here; the mixer keeps the source playing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lf_voxel::registry::block as b;

    /// The goal's Step-4 acceptance: the right sound category dispatches
    /// per block family on break/place.
    #[test]
    fn block_categories_dispatch_correctly() {
        assert_eq!(block_category(b::LOG), Category::Wood);
        assert_eq!(block_category(b::PLANKS), Category::Wood);
        assert_eq!(block_category(b::STONE), Category::Stone);
        assert_eq!(block_category(b::GLASS), Category::Glass);
        assert_eq!(block_category(b::IRON_ORE), Category::Metal);
        assert_eq!(block_category(b::CRUSHER), Category::Metal);
        assert_eq!(block_category(b::SAND), Category::Soft);
        assert_eq!(block_category(b::GRASS), Category::Soft);
    }

    #[test]
    fn synth_is_bounded_decaying_and_distinct() {
        for (cat, act) in [
            (Category::Wood, Action::Break),
            (Category::Stone, Action::Break),
            (Category::Metal, Action::Break),
            (Category::Glass, Action::Break),
            (Category::Soft, Action::Break),
        ] {
            let s = synth(cat, act);
            assert!(!s.is_empty(), "{:?} produced no samples", cat);
            assert!(s.len() < SAMPLE_RATE as usize / 3, "{:?} too long", cat);
            assert!(s.iter().all(|v| v.abs() <= 1.0), "{:?} clipped", cat);
            let tail: f32 = s[s.len() - 16..].iter().map(|v| v.abs()).sum();
            assert!(tail < 0.05, "{:?} does not decay to silence", cat);
        }
        // different categories sound different (different RMS profile)
        let wood = synth(Category::Wood, Action::Break);
        let glass = synth(Category::Glass, Action::Break);
        let peak = |v: &[f32]| v.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!((peak(&wood) - peak(&glass)).abs() > 0.02 || wood.len() != glass.len(),
            "categories should differ");
    }

    #[test]
    fn place_is_shorter_than_break() {
        for cat in [Category::Wood, Category::Stone, Category::Metal] {
            assert!(synth(cat, Action::Place).len() < synth(cat, Action::Break).len(),
                "{:?} place should be a shorter hit", cat);
        }
    }

    #[test]
    fn volume_scales_samples() {
        let s = synth(Category::Stone, Action::Break);
        let half = scaled(&s, 0.5);
        let peak = |v: &[f32]| v.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!((peak(&half) - peak(&s) * 0.5).abs() < 1e-4);
        assert!(scaled(&s, 0.0).iter().all(|v| *v == 0.0));
    }
}
