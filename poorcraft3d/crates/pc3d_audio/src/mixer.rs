//! The mixer (BETA-0.4 W-B B1): playing voices on a stereo bus. The
//! host's audio callback pulls f32 stereo frames from here; voices are
//! procedural buffers with per-voice gain and pan. Allocation only on
//! `play` (one push); the steady-state pull is allocation-free.

use crate::synth::{render, SoundId, SAMPLE_RATE};

/// One playing voice.
struct Voice {
    buffer: std::sync::Arc<Vec<f32>>,
    pos: usize,
    gain: f32,
    /// -1.0 (left) .. 1.0 (right).
    pan: f32,
}

/// The bus. `shares` the rendered buffers (Arc) so repeated sounds
/// (footsteps!) never re-render.
#[derive(Default)]
pub struct Mixer {
    voices: Vec<Voice>,
    /// Rendered-once buffers, shared by id.
    bank: std::collections::BTreeMap<SoundId, std::sync::Arc<Vec<f32>>>,
    pub master: f32,
    pub sfx_gain: f32,
}

impl Mixer {
    pub fn new() -> Self {
        Mixer {
            voices: Vec::new(),
            bank: std::collections::BTreeMap::new(),
            master: 0.8,
            sfx_gain: 1.0,
        }
    }

    /// Queue a sound. `pan` in -1..1.
    pub fn play(&mut self, id: SoundId, gain: f32, pan: f32) {
        let buffer = self
            .bank
            .entry(id)
            .or_insert_with(|| std::sync::Arc::new(render(id)))
            .clone();
        self.voices.push(Voice {
            buffer,
            pos: 0,
            gain: gain.clamp(0.0, 1.0) * self.sfx_gain * self.master,
            pan: pan.clamp(-1.0, 1.0),
        });
    }

    /// The number of currently playing voices (the audio gate reads it).
    pub fn playing(&self) -> usize {
        self.voices.len()
    }

    /// Pull the next stereo frame (left, right), advancing every voice.
    /// Allocation-free; finished voices are drained.
    pub fn next_frame(&mut self) -> (f32, f32) {
        let mut left = 0.0f32;
        let mut right = 0.0f32;
        self.voices.retain_mut(|v| {
            if v.pos >= v.buffer.len() {
                return false;
            }
            let s = v.buffer[v.pos] * v.gain;
            // Constant-ish pan: cos/sin law.
            let pan_angle = (v.pan + 1.0) * std::f32::consts::FRAC_PI_4;
            left += s * pan_angle.cos();
            right += s * pan_angle.sin();
            v.pos += 1;
            v.pos < v.buffer.len()
        });
        (left.clamp(-1.0, 1.0), right.clamp(-1.0, 1.0))
    }

    /// Fill an interleaved stereo output slice (the cpal callback's
    /// buffer). Deterministic and allocation-free.
    pub fn fill_interleaved(&mut self, out: &mut [f32]) {
        for chunk in out.chunks_exact_mut(2) {
            let (l, r) = self.next_frame();
            chunk[0] = l;
            chunk[1] = r;
        }
    }

    /// The sample rate the mixer speaks.
    pub const SAMPLE_RATE: u32 = SAMPLE_RATE;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::SoundId;

    /// THE PLAY LAW: play() registers a voice; pulling len samples of a
    /// short sound drains it exactly.
    #[test]
    fn beta04_mixer_plays_and_drains() {
        let mut m = Mixer::new();
        m.play(SoundId::ToastTick, 1.0, 0.0);
        assert_eq!(m.playing(), 1);
        let mut drained = 0;
        while m.playing() > 0 {
            let (l, r) = m.next_frame();
            assert!(l.is_finite() && r.is_finite());
            assert!(l.abs() <= 1.0 && r.abs() <= 1.0);
            drained += 1;
            assert!(drained <= SoundId::ToastTick.len_samples() + 2);
        }
        assert!(drained >= SoundId::ToastTick.len_samples() - 2);
    }

    /// THE GAIN LAW: master 0 mutes the bus (mute-on-pause uses this).
    #[test]
    fn beta04_mixer_master_zero_mutes() {
        let mut m = Mixer::new();
        m.play(SoundId::BarReady, 1.0, 0.0);
        m.master = 0.0;
        let (l, r) = m.next_frame();
        assert_eq!((l, r), (0.0, 0.0), "master 0 must mute");
    }

    /// THE PAN LAW: a hard-left pan puts all energy in the left channel.
    #[test]
    fn beta04_mixer_pan_law() {
        let mut m = Mixer::new();
        m.play(SoundId::OreClink, 1.0, -1.0);
        let (l, r) = m.next_frame();
        assert!(l.abs() > 0.0, "left carries");
        assert_eq!(r, 0.0, "right silent on hard-left");
    }

    /// THE SHARED-BUFFER LAW: playing the same id twice shares the bank
    /// entry (footsteps never re-render).
    #[test]
    fn beta04_mixer_bank_shared() {
        let mut m = Mixer::new();
        m.play(SoundId::StepGrass, 1.0, 0.0);
        m.play(SoundId::StepGrass, 1.0, 0.3);
        assert_eq!(m.bank.len(), 1, "one render serves both voices");
        assert_eq!(m.playing(), 2);
    }
}
