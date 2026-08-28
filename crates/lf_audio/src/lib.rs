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

/// Loop 329: the rest of the game's sound set — interface, body, and
/// movement feedback beyond block break/place. All synthesized like the
/// block sounds (no asset files, deterministic for the tests).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Sfx {
    /// Menu/screen transitions (also hover-confirm on links).
    UiClick,
    /// Eating: short double crunch.
    Eat,
    /// Player takes damage: descending thud.
    Hurt,
    /// XP level up: two-note ascending chime.
    Xp,
    /// A felled trunk starts to lean (loop 330 timber): low wavering creak.
    TreeCreak,
    /// A felled tree hits the ground: heavy crash with a low thud.
    TreeCrash,
    /// Footstep on the given material family.
    Footstep(Category),
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

/// One synthesized non-block one-shot. Same envelope/noise machinery as
/// `synth`, distinct parameter sets per event.
pub fn synth_sfx(sfx: Sfx) -> Vec<f32> {
    match sfx {
        Sfx::UiClick => {
            // 28ms tick: bright short tone, minimal noise
            tone_burst(28.0, 1400.0, 0.85, 0.15, 0.5, 3, 1.0)
        }
        Sfx::Eat => {
            // double crunch: two noise bursts separated by a dip
            let mut a = noise_burst(70.0, 420.0, 0.2, 0.9, 1);
            let b = noise_burst(70.0, 380.0, 0.2, 0.9, 7);
            let gap = vec![0.0; SAMPLE_RATE as usize / 200]; // 5ms
            a.extend(gap);
            a.extend(b);
            a
        }
        Sfx::Hurt => {
            // 160ms descending tone (400 -> 170 Hz) + dull noise
            let n = (SAMPLE_RATE as f32 * 0.16) as usize;
            let mut noise = Noise::new(0xA11C5);
            let mut lp = 0.0f32;
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let env = (1.0 - t) * (1.0 - t);
                    let hz = 400.0 - 230.0 * t;
                    let tone = (i as f32 * hz / SAMPLE_RATE as f32 * std::f32::consts::TAU).sin();
                    let raw = noise.next_f32() * 2.0 - 1.0;
                    lp += (raw - lp) * 0.15;
                    ((tone * 0.7 + lp * 0.3) * env * 0.6).clamp(-1.0, 1.0)
                })
                .collect()
        }
        Sfx::Xp => {
            // two-note ascending chime (660, 990), each 90ms
            let mut a = tone_burst(90.0, 660.0, 0.95, 0.05, 0.55, 11, 0.5);
            let b = tone_burst(110.0, 990.0, 0.95, 0.05, 0.6, 13, 0.5);
            a.extend(b);
            a
        }
        Sfx::Footstep(cat) => {
            // short soft noise step, colored by the material family
            let (dur, lp_k, seed) = match cat {
                Category::Wood => (75.0, 0.12, 21),
                Category::Stone => (70.0, 0.2, 23),
                Category::Metal => (65.0, 0.3, 29),
                Category::Glass => (60.0, 0.35, 31),
                Category::Soft => (85.0, 0.07, 37),
            };
            noise_burst(dur, 90.0, 0.12, lp_k, seed)
        }
        Sfx::TreeCreak => {
            // ~0.5s: low tone with a slow wobble (frequency wobbles around
            // 110 Hz like stressed wood fibers), light noise
            let n = (SAMPLE_RATE as f32 * 0.5) as usize;
            let mut noise = Noise::new(0xC7EA);
            let mut lp = 0.0f32;
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let env = (t * 3.0).min(1.0) * (1.0 - t) * 0.8;
                    let wobble = (t * 34.0).sin() * 18.0;
                    let tone = (i as f32 * (110.0 + wobble) / SAMPLE_RATE as f32
                        * std::f32::consts::TAU).sin();
                    let raw = noise.next_f32() * 2.0 - 1.0;
                    lp += (raw - lp) * 0.1;
                    ((tone * 0.75 + lp * 0.25) * env).clamp(-1.0, 1.0)
                })
                .collect()
        }
        Sfx::TreeCrash => {
            // ~0.45s: two-layer impact — a big bright noise burst over a
            // low 70 Hz thud, both decaying quadratically
            let n = (SAMPLE_RATE as f32 * 0.45) as usize;
            let mut noise = Noise::new(0xC1A5);
            let mut lp = 0.0f32;
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let env = (1.0 - t) * (1.0 - t);
                    let thud = (i as f32 * 70.0 / SAMPLE_RATE as f32 * std::f32::consts::TAU).sin();
                    let raw = noise.next_f32() * 2.0 - 1.0;
                    lp += (raw - lp) * 0.5;
                    ((lp * 0.65 + thud * 0.35) * env * 1.1).clamp(-1.0, 1.0)
                })
                .collect()
        }
    }
}

/// Shared tone+noise burst used by the sfx arms.
fn tone_burst(dur_ms: f32, hz: f32, tone_mix: f32, noise_mix: f32, punch: f32, seed: u64, tail: f32) -> Vec<f32> {
    let n = (SAMPLE_RATE as f32 * dur_ms / 1000.0) as usize;
    let mut noise = Noise::new(seed);
    let mut lp = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let env = (1.0 - t).powf(1.0 + tail) * punch;
            let tone = (i as f32 * hz / SAMPLE_RATE as f32 * std::f32::consts::TAU).sin();
            let raw = noise.next_f32() * 2.0 - 1.0;
            lp += (raw - lp) * 0.3;
            ((tone * tone_mix + lp * noise_mix) * env * 0.5).clamp(-1.0, 1.0)
        })
        .collect()
}

/// Noise-dominant burst with a faint body tone.
fn noise_burst(dur_ms: f32, body_hz: f32, body_mix: f32, lp_k: f32, seed: u64) -> Vec<f32> {
    let n = (SAMPLE_RATE as f32 * dur_ms / 1000.0) as usize;
    let mut noise = Noise::new(seed);
    let mut lp = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let env = (1.0 - t) * (1.0 - t) * 0.55;
            let body = (i as f32 * body_hz / SAMPLE_RATE as f32 * std::f32::consts::TAU).sin();
            let raw = noise.next_f32() * 2.0 - 1.0;
            lp += (raw - lp) * lp_k.clamp(0.01, 0.95);
            ((body * body_mix + lp * (1.0 - body_mix)) * env).clamp(-1.0, 1.0)
        })
        .collect()
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
        self.append(&synth(category, action), volume);
    }

    /// Play a non-block one-shot (ui/body/movement). Same rate limit as
    /// block sounds — footsteps at sprint cadence must not stack.
    pub fn play_sfx(&mut self, sfx: Sfx, volume: f32) {
        if volume <= 0.01 {
            return;
        }
        let now = Instant::now();
        if now.duration_since(self.last).as_millis() < 30 {
            return;
        }
        self.last = now;
        self.append(&synth_sfx(sfx), volume);
    }

    fn append(&self, samples: &[f32], volume: f32) {
        let samples = scaled(samples, volume);
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

    /// Loop 329: the non-block sound set is bounded, decaying, and distinct.
    #[test]
    fn sfx_set_is_bounded_decaying_and_distinct() {
        let all = [
            Sfx::UiClick,
            Sfx::Eat,
            Sfx::Hurt,
            Sfx::Xp,
            Sfx::TreeCreak,
            Sfx::TreeCrash,
            Sfx::Footstep(Category::Wood),
            Sfx::Footstep(Category::Stone),
            Sfx::Footstep(Category::Metal),
            Sfx::Footstep(Category::Glass),
            Sfx::Footstep(Category::Soft),
        ];
        for sfx in all {
            let s = synth_sfx(sfx);
            assert!(!s.is_empty(), "{:?} produced no samples", sfx);
            // one-shots stay under a third of a second; the timber pair
            // (creak/crash) may breathe for up to 0.6s
            let cap = matches!(sfx, Sfx::TreeCreak | Sfx::TreeCrash)
                .then(|| SAMPLE_RATE as usize * 2 / 3)
                .unwrap_or(SAMPLE_RATE as usize / 3);
            assert!(s.len() < cap, "{:?} too long ({} >= {})", sfx, s.len(), cap);
            assert!(s.iter().all(|v| v.abs() <= 1.0), "{:?} clipped", sfx);
            let tail: f32 = s[s.len() - 16..].iter().map(|v| v.abs()).sum();
            assert!(tail < 0.08, "{:?} does not decay to silence (tail {})", sfx, tail);
        }
        // distinct events have distinct waveforms
        let click = synth_sfx(Sfx::UiClick);
        let hurt = synth_sfx(Sfx::Hurt);
        assert_ne!(click, hurt);
        // footsteps are the shortest family member (fast cadence)
        let step = synth_sfx(Sfx::Footstep(Category::Wood));
        assert!(step.len() < hurt.len(), "footstep should be shorter than hurt");
        // material families differ (soft vs stone step)
        assert_ne!(synth_sfx(Sfx::Footstep(Category::Soft)), synth_sfx(Sfx::Footstep(Category::Stone)));
    }
}
