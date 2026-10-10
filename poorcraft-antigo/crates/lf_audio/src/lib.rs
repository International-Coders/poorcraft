//! Game audio: a bank of real sound effects (generated once via the
//! ElevenLabs Sound Effects API — see `tools/gen_sounds.py`, the MP3s in
//! `assets/sounds/` are the committed artifacts) embedded into the binary
//! and decoded to mono PCM at boot, with the original procedural
//! synthesizer kept as a deterministic per-event fallback when a sample
//! is missing or fails to decode. Playback goes through rodio and
//! degrades to silence when no output device exists (headless CI, broken
//! audio), so the game logic never has to care.

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
    /// Loop 349 — the sampled bank widens the event set: entering deep
    /// water, combat, world interactions.
    /// Body plunging into water.
    Splash,
    /// Bow string release.
    BowShoot,
    /// Arrow sticking into something solid.
    ArrowHit,
    /// Melee swing whoosh (whiff).
    MeleeSwing,
    /// Fleshy hit on a creature.
    MobHit,
    /// A creature collapsing.
    MobDeath,
    /// The mount dragon announcing itself.
    DragonRoar,
    /// Collected drops popping into inventory.
    ItemPickup,
    /// A recipe successfully crafted.
    CraftDone,
    /// Chest lid creaking open.
    ChestOpen,
    /// Hammer on the forge/anvil.
    SmithClang,
    /// Player death sting.
    PlayerDeath,
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
        // Loop 349 fallbacks for the widened sampled set (each also has a
        // bank sample; these keep the enum total and the tests synthetic).
        Sfx::Splash => {
            // ~0.3s: bright noise wash with a falling body tone
            let n = (SAMPLE_RATE as f32 * 0.3) as usize;
            let mut noise = Noise::new(0x51A1);
            let mut lp = 0.0f32;
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let env = (1.0 - t) * (1.0 - t);
                    let hz = 300.0 - 220.0 * t;
                    let tone = (i as f32 * hz / SAMPLE_RATE as f32 * std::f32::consts::TAU).sin();
                    let raw = noise.next_f32() * 2.0 - 1.0;
                    lp += (raw - lp) * 0.45;
                    ((lp * 0.7 + tone * 0.3) * env * 0.8).clamp(-1.0, 1.0)
                })
                .collect()
        }
        Sfx::BowShoot => {
            // string pluck (rising then snap): bright 900 Hz twang, fast
            tone_burst(140.0, 900.0, 0.8, 0.2, 0.8, 101, 2.5)
        }
        Sfx::ArrowHit => {
            // dry stick: sharp mid knock with a quick tail
            tone_burst(110.0, 720.0, 0.55, 0.45, 0.9, 103, 3.0)
        }
        Sfx::MeleeSwing => {
            // air whoosh: bandpassed noise swell, no body tone
            noise_burst(150.0, 260.0, 0.08, 0.5, 107)
        }
        Sfx::MobHit => {
            // fleshy thud: dull low knock
            tone_burst(110.0, 160.0, 0.5, 0.5, 0.95, 109, 2.0)
        }
        Sfx::MobDeath => {
            // collapsing body: low knock into a short noise settle
            let mut a = tone_burst(130.0, 130.0, 0.55, 0.45, 0.95, 113, 2.0);
            let b = noise_burst(160.0, 90.0, 0.3, 0.15, 127);
            a.extend(b);
            a
        }
        Sfx::DragonRoar => {
            // ~0.8s: big wobbling growl around 80 Hz with a rasp top
            let n = (SAMPLE_RATE as f32 * 0.8) as usize;
            let mut noise = Noise::new(0xD406);
            let mut lp = 0.0f32;
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let env = (t * 4.0).min(1.0) * (1.0 - t) * 0.95;
                    let wobble = (t * 21.0).sin() * 14.0;
                    let tone = (i as f32 * (80.0 + wobble) / SAMPLE_RATE as f32
                        * std::f32::consts::TAU).sin();
                    let raw = noise.next_f32() * 2.0 - 1.0;
                    lp += (raw - lp) * 0.2;
                    ((tone * 0.6 + lp * 0.4) * env).clamp(-1.0, 1.0)
                })
                .collect()
        }
        Sfx::ItemPickup => {
            // soft pop: tiny rising blip
            tone_burst(60.0, 950.0, 0.9, 0.1, 0.7, 131, 1.5)
        }
        Sfx::CraftDone => {
            // two confident taps then a swish
            let mut a = tone_burst(60.0, 540.0, 0.5, 0.5, 0.8, 137, 2.0);
            let b = tone_burst(70.0, 660.0, 0.5, 0.5, 0.85, 139, 2.0);
            let c = noise_burst(110.0, 220.0, 0.15, 0.3, 149);
            a.extend(b);
            a.extend(c);
            a
        }
        Sfx::ChestOpen => {
            // creak: slow wobbling mid tone with grain
            let n = (SAMPLE_RATE as f32 * 0.4) as usize;
            let mut noise = Noise::new(0xCE5);
            let mut lp = 0.0f32;
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let env = (t * 2.0).min(1.0) * (1.0 - t) * 0.8;
                    let wobble = (t * 26.0).sin() * 25.0;
                    let tone = (i as f32 * (240.0 + wobble) / SAMPLE_RATE as f32
                        * std::f32::consts::TAU).sin();
                    let raw = noise.next_f32() * 2.0 - 1.0;
                    lp += (raw - lp) * 0.25;
                    ((tone * 0.55 + lp * 0.45) * env).clamp(-1.0, 1.0)
                })
                .collect()
        }
        Sfx::SmithClang => {
            // anvil ring: bright high partial, long ringing tail
            tone_burst(280.0, 1500.0, 0.85, 0.15, 0.9, 151, 4.0)
        }
        Sfx::PlayerDeath => {
            // ~0.6s: descending farewell tone
            let n = (SAMPLE_RATE as f32 * 0.6) as usize;
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let env = (1.0 - t) * (1.0 - t);
                    let hz = 330.0 - 200.0 * t;
                    let tone = (i as f32 * hz / SAMPLE_RATE as f32 * std::f32::consts::TAU).sin();
                    (tone * env * 0.55).clamp(-1.0, 1.0)
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

// ---------------------------------------------------------------------------
// Sampled bank (loop 349): real effects generated once with the ElevenLabs
// Sound Effects API (`tools/gen_sounds.py` holds the prompts), embedded at
// compile time so the binary is self-contained and the catalog cannot
// drift from the code — a missing file is a build error, a file that fails
// to decode falls back to the synthesizer for that one event.
// ---------------------------------------------------------------------------

/// Every sound the bank knows. Consumer side of the gen_sounds.py manifest.
static BANK_FILES: &[(&str, &[u8])] = &[
    ("break_wood", include_bytes!("../../../assets/sounds/break_wood.mp3")),
    ("break_stone", include_bytes!("../../../assets/sounds/break_stone.mp3")),
    ("break_metal", include_bytes!("../../../assets/sounds/break_metal.mp3")),
    ("break_glass", include_bytes!("../../../assets/sounds/break_glass.mp3")),
    ("break_soft", include_bytes!("../../../assets/sounds/break_soft.mp3")),
    ("place_wood", include_bytes!("../../../assets/sounds/place_wood.mp3")),
    ("place_stone", include_bytes!("../../../assets/sounds/place_stone.mp3")),
    ("place_metal", include_bytes!("../../../assets/sounds/place_metal.mp3")),
    ("place_glass", include_bytes!("../../../assets/sounds/place_glass.mp3")),
    ("place_soft", include_bytes!("../../../assets/sounds/place_soft.mp3")),
    ("step_wood", include_bytes!("../../../assets/sounds/step_wood.mp3")),
    ("step_stone", include_bytes!("../../../assets/sounds/step_stone.mp3")),
    ("step_metal", include_bytes!("../../../assets/sounds/step_metal.mp3")),
    ("step_glass", include_bytes!("../../../assets/sounds/step_glass.mp3")),
    ("step_soft", include_bytes!("../../../assets/sounds/step_soft.mp3")),
    ("ui_click", include_bytes!("../../../assets/sounds/ui_click.mp3")),
    ("eat", include_bytes!("../../../assets/sounds/eat.mp3")),
    ("hurt", include_bytes!("../../../assets/sounds/hurt.mp3")),
    ("xp", include_bytes!("../../../assets/sounds/xp.mp3")),
    ("tree_creak", include_bytes!("../../../assets/sounds/tree_creak.mp3")),
    ("tree_crash", include_bytes!("../../../assets/sounds/tree_crash.mp3")),
    ("splash", include_bytes!("../../../assets/sounds/splash.mp3")),
    ("bow_shoot", include_bytes!("../../../assets/sounds/bow_shoot.mp3")),
    ("arrow_hit", include_bytes!("../../../assets/sounds/arrow_hit.mp3")),
    ("melee_swing", include_bytes!("../../../assets/sounds/melee_swing.mp3")),
    ("mob_hit", include_bytes!("../../../assets/sounds/mob_hit.mp3")),
    ("mob_death", include_bytes!("../../../assets/sounds/mob_death.mp3")),
    ("dragon_roar", include_bytes!("../../../assets/sounds/dragon_roar.mp3")),
    ("item_pickup", include_bytes!("../../../assets/sounds/item_pickup.mp3")),
    ("craft_done", include_bytes!("../../../assets/sounds/craft_done.mp3")),
    ("chest_open", include_bytes!("../../../assets/sounds/chest_open.mp3")),
    ("smith_clang", include_bytes!("../../../assets/sounds/smith_clang.mp3")),
    ("player_death", include_bytes!("../../../assets/sounds/player_death.mp3")),
];

/// One decoded bank entry: mono f32 at the file's own sample rate.
struct Sample {
    rate: u32,
    data: Vec<f32>,
}

/// The decoded sound bank; entries that fail to decode are simply absent
/// (playback falls back to the synthesizer for that event).
pub struct Bank(std::collections::HashMap<&'static str, Sample>);

impl Bank {
    /// Decode every embedded MP3. Cheap — a few seconds of audio total.
    pub fn load() -> Self {
        let mut map = std::collections::HashMap::new();
        for (name, bytes) in BANK_FILES {
            if let Some(s) = decode_mp3_mono(bytes) {
                map.insert(*name, s);
            }
        }
        Bank(map)
    }

    fn get(&self, name: &str) -> Option<&Sample> {
        self.0.get(name)
    }

    /// Sorted names of all successfully decoded entries (tests/diagnostics).
    pub fn loaded_names(&self) -> Vec<&'static str> {
        let mut v: Vec<_> = self.0.keys().copied().collect();
        v.sort_unstable();
        v
    }
}

/// Bank key for a block event.
fn block_key(category: Category, action: Action) -> &'static str {
    match (category, action) {
        (Category::Wood, Action::Break) => "break_wood",
        (Category::Stone, Action::Break) => "break_stone",
        (Category::Metal, Action::Break) => "break_metal",
        (Category::Glass, Action::Break) => "break_glass",
        (Category::Soft, Action::Break) => "break_soft",
        (Category::Wood, Action::Place) => "place_wood",
        (Category::Stone, Action::Place) => "place_stone",
        (Category::Metal, Action::Place) => "place_metal",
        (Category::Glass, Action::Place) => "place_glass",
        (Category::Soft, Action::Place) => "place_soft",
    }
}

/// Bank key for a one-shot event.
pub fn sfx_key(sfx: Sfx) -> &'static str {
    match sfx {
        Sfx::UiClick => "ui_click",
        Sfx::Eat => "eat",
        Sfx::Hurt => "hurt",
        Sfx::Xp => "xp",
        Sfx::TreeCreak => "tree_creak",
        Sfx::TreeCrash => "tree_crash",
        Sfx::Footstep(Category::Wood) => "step_wood",
        Sfx::Footstep(Category::Stone) => "step_stone",
        Sfx::Footstep(Category::Metal) => "step_metal",
        Sfx::Footstep(Category::Glass) => "step_glass",
        Sfx::Footstep(Category::Soft) => "step_soft",
        Sfx::Splash => "splash",
        Sfx::BowShoot => "bow_shoot",
        Sfx::ArrowHit => "arrow_hit",
        Sfx::MeleeSwing => "melee_swing",
        Sfx::MobHit => "mob_hit",
        Sfx::MobDeath => "mob_death",
        Sfx::DragonRoar => "dragon_roar",
        Sfx::ItemPickup => "item_pickup",
        Sfx::CraftDone => "craft_done",
        Sfx::ChestOpen => "chest_open",
        Sfx::SmithClang => "smith_clang",
        Sfx::PlayerDeath => "player_death",
    }
}

/// Decode an embedded MP3 to trimmed, normalized mono f32 PCM.
fn decode_mp3_mono(bytes: &[u8]) -> Option<Sample> {
    use rodio::Source;
    // Decoder requires an owned reader (symphonia probes the stream)
    let cursor = std::io::Cursor::new(bytes.to_vec());
    let decoder = rodio::Decoder::new(cursor).ok()?;
    let channels = (decoder.channels().get() as usize).max(1);
    let rate = decoder.sample_rate().get();
    if rate == 0 {
        return None;
    }
    let raw: Vec<f32> = decoder.collect();
    if raw.len() < channels {
        return None;
    }
    // downmix interleaved channels to mono
    let mono: Vec<f32> = if channels == 1 {
        raw
    } else {
        raw.chunks(chunks_max(channels)).map(|c| c.iter().sum::<f32>() / c.len() as f32).collect()
    };
    let peak = mono.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    // the generator sometimes returns near-silent audio; reject it outright
    // (playback falls back to the synthesizer for that event) instead of
    // boosting garbage
    if peak < 0.05 {
        return None;
    }
    // Trim leading/trailing near-silence (MP3 encoder padding plus the
    // generator's own head/tail room) with a small guard so percussive
    // onsets — footsteps, clicks — still breathe. The threshold is
    // relative to the file's own peak because generations master at very
    // different levels (measured peaks range 0.15..1.0).
    let threshold = (peak * 0.08).clamp(0.004, 0.05);
    let first = mono.iter().position(|s| s.abs() > threshold)?;
    let last = mono.iter().rposition(|s| s.abs() > threshold)?;
    let guard = (rate as usize / 125).max(1); // ~8 ms
    let start = first.saturating_sub(guard);
    let end = (last + guard + 1).min(mono.len());
    let mut data = mono[start..end].to_vec();
    // Normalize to a common playing level so a quietly-mastered generation
    // doesn't vanish in the mix. The ceiling of 12x is generous on
    // purpose: the generator's output is clean (no noise floor to boost).
    let trimmed_peak = data.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if trimmed_peak > 1e-4 {
        let gain = (0.85 / trimmed_peak).clamp(0.25, 12.0);
        for s in &mut data {
            *s = (*s * gain).clamp(-1.0, 1.0);
        }
    }
    Some(Sample { rate, data })
}

/// chunks() panics on 0; the channel count is already >= 1 here.
fn chunks_max(channels: usize) -> usize {
    channels.max(1)
}

/// Live player: owns the output stream's mixer; `None` when the machine
/// has no usable audio output (silent fallback — gameplay never blocks).
pub struct Audio {
    mixer: rodio::mixer::Mixer,
    _sink: rodio::MixerDeviceSink,
    bank: Bank,
    last: Instant,
}

impl Audio {
    pub fn new() -> Option<Self> {
        let mut sink = rodio::DeviceSinkBuilder::open_default_sink().ok()?;
        sink.log_on_drop(false);
        let mixer = sink.mixer().clone();
        Some(Self {
            mixer,
            _sink: sink,
            bank: Bank::load(),
            last: Instant::now() - std::time::Duration::from_secs(1),
        })
    }

    /// Play a one-shot. Rate-limited so a burst of edits (water sim, sand
    /// collapse) cannot machine-gun the mixer. The sampled bank is the
    /// primary source; the synthesizer covers any event without a sample.
    pub fn play(&mut self, category: Category, action: Action, volume: f32) {
        if volume <= 0.01 {
            return;
        }
        let now = Instant::now();
        if now.duration_since(self.last).as_millis() < 30 {
            return;
        }
        self.last = now;
        match self.bank.get(block_key(category, action)) {
            Some(sample) => self.append(&sample.data, sample.rate, volume),
            None => self.append(&synth(category, action), SAMPLE_RATE, volume),
        }
    }

    /// Play a non-block one-shot (ui/body/movement/combat). Same rate limit
    /// as block sounds — footsteps at sprint cadence must not stack.
    pub fn play_sfx(&mut self, sfx: Sfx, volume: f32) {
        if volume <= 0.01 {
            return;
        }
        let now = Instant::now();
        if now.duration_since(self.last).as_millis() < 30 {
            return;
        }
        self.last = now;
        match self.bank.get(sfx_key(sfx)) {
            Some(sample) => self.append(&sample.data, sample.rate, volume),
            None => self.append(&synth_sfx(sfx), SAMPLE_RATE, volume),
        }
    }

    fn append(&self, samples: &[f32], rate: u32, volume: f32) {
        let rate = if rate == 0 { SAMPLE_RATE } else { rate };
        let samples = scaled(samples, volume);
        let player = rodio::Player::connect_new(&self.mixer);
        player.append(rodio::buffer::SamplesBuffer::new(
            std::num::NonZero::<u16>::new(1).unwrap(),
            std::num::NonZero::<u32>::new(rate).unwrap(),
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
            Sfx::Splash,
            Sfx::BowShoot,
            Sfx::ArrowHit,
            Sfx::MeleeSwing,
            Sfx::MobHit,
            Sfx::MobDeath,
            Sfx::DragonRoar,
            Sfx::ItemPickup,
            Sfx::CraftDone,
            Sfx::ChestOpen,
            Sfx::SmithClang,
            Sfx::PlayerDeath,
        ];
        for sfx in all {
            let s = synth_sfx(sfx);
            assert!(!s.is_empty(), "{:?} produced no samples", sfx);
            // one-shots stay under a third of a second; the timber pair and
            // the long dramatic beats (roar, death) may breathe for up to 1s
            let cap = matches!(
                sfx,
                Sfx::TreeCreak
                    | Sfx::TreeCrash
                    | Sfx::DragonRoar
                    | Sfx::PlayerDeath
                    | Sfx::ChestOpen
            )
            .then(|| SAMPLE_RATE as usize)
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

    /// Loop 349: every embedded bank file decodes to usable mono PCM, and
    /// every Sfx/block event maps to a bank entry — the catalog cannot
    /// drift from the generator manifest.
    #[test]
    fn bank_decodes_and_covers_every_event() {
        let bank = Bank::load();
        let names = bank.loaded_names();
        assert_eq!(names.len(), BANK_FILES.len(), "some embedded files failed to decode");
        for (name, _) in BANK_FILES {
            assert!(names.contains(name), "{} missing from decoded bank", name);
        }
        // every non-block event resolves to a decoded sample
        let every_sfx = [
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
            Sfx::Splash,
            Sfx::BowShoot,
            Sfx::ArrowHit,
            Sfx::MeleeSwing,
            Sfx::MobHit,
            Sfx::MobDeath,
            Sfx::DragonRoar,
            Sfx::ItemPickup,
            Sfx::CraftDone,
            Sfx::ChestOpen,
            Sfx::SmithClang,
            Sfx::PlayerDeath,
        ];
        for sfx in every_sfx {
            let sample = bank
                .get(sfx_key(sfx))
                .unwrap_or_else(|| panic!("{:?} has no bank sample", sfx));
            assert!(sample.rate >= 8_000, "{:?} implausible rate {}", sfx, sample.rate);
            assert!(!sample.data.is_empty(), "{:?} decoded to nothing", sfx);
            assert!(sample.data.iter().all(|v| v.abs() <= 1.0), "{:?} clipped", sfx);
            // the trim keeps at least some audible energy in every sample
            let peak = sample.data.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.1, "{:?} is effectively silent (peak {})", sfx, peak);
        }
        // every block event resolves too
        for cat in [
            Category::Wood,
            Category::Stone,
            Category::Metal,
            Category::Glass,
            Category::Soft,
        ] {
            for action in [Action::Break, Action::Place] {
                assert!(
                    bank.get(block_key(cat, action)).is_some(),
                    "{:?}/{:?} has no bank sample",
                    cat,
                    action
                );
            }
        }
    }

    /// The decoder trims MP3 padding: samples start near their energy and
    /// end decaying, so short percussive events don't feel laggy.
    #[test]
    fn bank_trim_and_normalize_shape() {
        let bank = Bank::load();
        let step = bank.get("step_stone").expect("step_stone sample");
        // leading padding trimmed: audible energy within the first quarter
        let head = (step.data.len() / 4).max(1);
        let lead = step.data[..head].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(lead > 0.05, "step_stone still has a long silent head ({})", lead);
        // normalization brings everyone near the common playing level
        let peak = step.data.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!((0.5..=1.0).contains(&peak), "step_stone not normalized (peak {})", peak);
    }
}
