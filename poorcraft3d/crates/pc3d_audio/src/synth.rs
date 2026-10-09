//! The procedural synth (BETA-0.4 W-B B2): every sound in POORCRAFT 3D
//! is DETERMINISTIC — a pure function of (SoundId, seed). No audio
//! files, no external assets: the same laws that grow the world grow
//! its sounds. A buffer is 48 kHz mono f32; the mixer pans/plays it.

/// Output sample rate the whole crate speaks.
pub const SAMPLE_RATE: u32 = 48_000;

/// One sound the game can play. Stable codes (persist-safe).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SoundId {
    StepGrass,
    StepStone,
    StepWood,
    Jump,
    Land,
    Dig,
    Build,
    Remove,
    ForgeWhoosh,
    OreClink,
    BarReady,
    Eat,
    ToastTick,
    UiOpen,
    UiClose,
    UiClick,
    UiDeny,
    Swing,
    Hit,
    ChestOpen,
    Coin,
    QuestAccept,
    QuestDone,
}

impl SoundId {
    /// (duration_samples, kind, seed) — the recipe table. Deterministic.
    fn recipe(self) -> (usize, Kind, u64) {
        match self {
            SoundId::StepGrass => (2_880, Kind::NoiseBurst { cutoff: 0.35, thump: 0.2 }, 0x51),
            SoundId::StepStone => (1_920, Kind::Click { body: 0.6 }, 0x52),
            SoundId::StepWood => (2_400, Kind::Click { body: 0.35 }, 0x5C),
            SoundId::Jump => (3_600, Kind::Sweep { from: 220.0, to: 330.0 }, 0x53),
            SoundId::Land => (4_800, Kind::NoiseBurst { cutoff: 0.2, thump: 0.9 }, 0x54),
            SoundId::Dig => (4_800, Kind::NoiseBurst { cutoff: 0.25, thump: 0.7 }, 0x55),
            SoundId::Build => (3_600, Kind::Click { body: 0.5 }, 0x56),
            SoundId::Remove => (5_760, Kind::NoiseBurst { cutoff: 0.3, thump: 0.5 }, 0x57),
            SoundId::ForgeWhoosh => (14_400, Kind::NoiseBurst { cutoff: 0.15, thump: 0.3 }, 0x58),
            SoundId::OreClink => (2_880, Kind::Click { body: 0.9 }, 0x59),
            SoundId::BarReady => (9_600, Kind::Chime { base: 880.0 }, 0x5A),
            SoundId::Eat => (7_200, Kind::NoiseBurst { cutoff: 0.4, thump: 0.4 }, 0x5B),
            SoundId::ToastTick => (1_440, Kind::Click { body: 0.3 }, 0x5D),
            SoundId::UiOpen => (2_400, Kind::Sweep { from: 300.0, to: 420.0 }, 0x5E),
            SoundId::UiClose => (2_400, Kind::Sweep { from: 420.0, to: 300.0 }, 0x5F),
            SoundId::UiClick => (1_200, Kind::Click { body: 0.4 }, 0x60),
            SoundId::UiDeny => (4_800, Kind::Sweep { from: 180.0, to: 120.0 }, 0x61),
            SoundId::Swing => (4_800, Kind::Sweep { from: 500.0, to: 260.0 }, 0x62),
            SoundId::Hit => (3_600, Kind::NoiseBurst { cutoff: 0.5, thump: 0.8 }, 0x63),
            SoundId::ChestOpen => (7_200, Kind::Click { body: 0.7 }, 0x64),
            SoundId::Coin => (3_600, Kind::Chime { base: 1_320.0 }, 0x65),
            SoundId::QuestAccept => (7_200, Kind::Chime { base: 660.0 }, 0x66),
            SoundId::QuestDone => (14_400, Kind::Chime { base: 523.25 }, 0x67),
        }
    }

    /// The buffer length in samples.
    pub fn len_samples(self) -> usize {
        self.recipe().0
    }
}

/// The sound-shape families the synth knows.
#[derive(Clone, Copy, Debug)]
enum Kind {
    /// Low-passed noise with a thump body (footsteps, digs, whooshes).
    NoiseBurst { cutoff: f32, thump: f32 },
    /// A sharp resonant click (stone, clinks, UI).
    Click { body: f32 },
    /// A pitch sweep (jump, UI open/close, swings).
    Sweep { from: f32, to: f32 },
    /// A decaying sine + its fifth (chimes, coins, fanfares).
    Chime { base: f32 },
}

/// FNV-1a over the bytes — the repo's own hash, for the noise streams.
fn fnv(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// One-step xorshift — cheap deterministic white noise.
fn next_noise(state: &mut u64) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    ((*state >> 11) as f32 / (1u64 << 53) as f32) * 2.0 - 1.0
}

/// Render a sound to a mono f32 buffer. Pure: the same id replays
/// byte-equal forever (the determinism law the repo runs on).
pub fn render(id: SoundId) -> Vec<f32> {
    let (len, kind, seed) = id.recipe();
    let mut noise_state = fnv(&[seed as u8, (len & 0xff) as u8, 0xA0]);
    let mut out = vec![0.0f32; len];
    match kind {
        Kind::NoiseBurst { cutoff, thump } => {
            // One-pole low-pass at `cutoff` (0..1 of Nyquist), with a
            // decaying thump body and a fast exponential envelope.
            let alpha = cutoff.clamp(0.01, 0.95);
            let mut lp = 0.0f32;
            for (i, o) in out.iter_mut().enumerate() {
                let t = i as f32 / len as f32;
                let env = (1.0 - t) * (1.0 - t);
                let n = next_noise(&mut noise_state);
                lp += alpha * (n - lp);
                let thump_body = (6.283_185_5 * 70.0 * i as f32 / SAMPLE_RATE as f32).sin() * thump;
                *o = (lp * 0.7 + thump_body * 0.3) * env * 0.8;
            }
        }
        Kind::Click { body } => {
            // A damped sine at a seeded mid frequency + a noise transient
            // in the first 2 ms (the "attack").
            let freq = 600.0 + body * 900.0;
            for (i, o) in out.iter_mut().enumerate() {
                let t = i as f32 / len as f32;
                let env = (-t * 7.0).max(0.0).exp();
                let tone = (6.283_185_5 * freq * i as f32 / SAMPLE_RATE as f32).sin();
                let attack = if i < 96 { next_noise(&mut noise_state) * 0.6 } else { 0.0 };
                *o = (tone * body + attack) * env * 0.6;
            }
        }
        Kind::Sweep { from, to } => {
            // A soft sine gliding from..to with a symmetric envelope.
            let mut phase = 0.0f32;
            for (i, o) in out.iter_mut().enumerate() {
                let t = i as f32 / len as f32;
                let f = from + (to - from) * t;
                phase += 6.283_185_5 * f / SAMPLE_RATE as f32;
                let env = (t * 3.141_592_7).sin();
                *o = phase.sin() * env * 0.35;
            }
        }
        Kind::Chime { base } => {
            // base + fifth (1.5x), slow decay, gentle shimmer (2x at 0.2).
            for (i, o) in out.iter_mut().enumerate() {
                let t = i as f32 / len as f32;
                let env = (-t * 4.5).max(0.0).exp();
                let tt = i as f32 / SAMPLE_RATE as f32;
                let a = (6.283_185_5 * base * tt).sin();
                let fifth = (6.283_185_5 * base * 1.5 * tt).sin() * 0.5;
                let shimmer = (6.283_185_5 * base * 2.0 * tt).sin() * 0.2;
                *o = (a + fifth + shimmer) * env * 0.4;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE DETERMINISM LAW: the same id renders byte-equal forever.
    #[test]
    fn beta04_render_is_deterministic() {
        for id in [
            SoundId::StepGrass,
            SoundId::Dig,
            SoundId::BarReady,
            SoundId::UiDeny,
        ] {
            let a = render(id);
            let b = render(id);
            assert_eq!(a, b, "{id:?} must replay byte-equal");
            assert_eq!(a.len(), id.len_samples());
        }
    }

    /// THE SIGNAL LAW: no NaN, bounded amplitude, and every sound has
    /// actual content (peak above 0.05 — a silent buffer is a bug).
    #[test]
    fn beta04_render_is_sane_audio() {
        for id in [
            SoundId::StepGrass,
            SoundId::StepStone,
            SoundId::Jump,
            SoundId::Land,
            SoundId::Dig,
            SoundId::Build,
            SoundId::Remove,
            SoundId::ForgeWhoosh,
            SoundId::OreClink,
            SoundId::BarReady,
            SoundId::Eat,
            SoundId::ToastTick,
            SoundId::UiOpen,
            SoundId::UiClose,
            SoundId::UiClick,
            SoundId::UiDeny,
            SoundId::Swing,
            SoundId::Hit,
            SoundId::ChestOpen,
            SoundId::Coin,
            SoundId::QuestAccept,
            SoundId::QuestDone,
        ] {
            let buf = render(id);
            let peak = buf.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(!buf.iter().any(|x| x.is_nan()), "{id:?} produced NaN");
            assert!(peak <= 1.0, "{id:?} clips: {peak}");
            assert!(peak >= 0.05, "{id:?} is silent (peak {peak})");
        }
    }

    /// THE DISTINCTION LAW: different ids produce different buffers —
    /// a footstep must not sound like a chime.
    #[test]
    fn beta04_sounds_distinct() {
        let a = render(SoundId::StepGrass);
        let b = render(SoundId::BarReady);
        let diff: f32 = a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).abs())
            .sum();
        assert!(diff > 1.0, "distinct sounds must differ");
    }
}
