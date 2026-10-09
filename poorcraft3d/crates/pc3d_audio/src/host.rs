//! The AudioHost (BETA-0.4 W-B B1): a cpal output stream pulling from
//! the Mixer. THE GRACEFUL LAW: no device, no permission, or any stream
//! build failure leaves the host ALIVE and silent (the game runs
//! headless/muted; audio is never a crash).

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::mixer::Mixer;
use std::sync::Arc;

/// What the host managed to build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostState {
    /// A live output stream at the given sample rate.
    Live { sample_rate: u32 },
    /// No usable output device — the game runs muted.
    Silent,
}

/// The host: owns the stream (dropping it stops audio) and shares the
/// mixer with the game (the game calls `play`).
pub struct AudioHost {
    mixer: Arc<std::sync::Mutex<Mixer>>,
    #[allow(dead_code)]
    stream: Option<cpal::Stream>,
    pub state: HostState,
}

impl AudioHost {
    /// Build the host against the default output device. NEVER panics:
    /// every failure degrades to `HostState::Silent`.
    pub fn connect() -> Self {
        let mixer = Arc::new(std::sync::Mutex::new(Mixer::new()));
        let config = || {
            let device = cpal::default_host()
                .devices()
                .ok()?
                .find(|d| d.default_output_config().is_ok())?;
            let configs: Vec<_> = device.supported_output_configs().ok()?.collect();
            // Prefer stereo f32 at (or near) the mixer rate; fall back to
            // whatever stereo config exists.
            // F32 stereo only — the mixer speaks f32; anything else is
            // the Silent path (graceful, by the host law).
            let wanted = configs
                .iter()
                .find(|c| {
                    c.channels() == 2
                        && c.sample_format() == cpal::SampleFormat::F32
                        && c.min_sample_rate().0 <= 48_000
                        && 48_000 <= c.max_sample_rate().0
                })
                .or_else(|| {
                    configs.iter().find(|c| {
                        c.channels() == 2 && c.sample_format() == cpal::SampleFormat::F32
                    })
                })?;
            let sample_rate = wanted.min_sample_rate().0.max(48_000.min(wanted.max_sample_rate().0));
            let cfg = wanted
                .clone()
                .with_sample_rate(cpal::SampleRate(sample_rate))
                .into();
            device.build_output_stream(
                &cfg,
                {
                    let mixer = mixer.clone();
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        if let Ok(mut m) = mixer.lock() {
                            m.fill_interleaved(data);
                        } else {
                            for s in data.iter_mut() {
                                *s = 0.0;
                            }
                        }
                    }
                },
                |err| {
                    // Callback errors: drain and continue (a device hiccup
                    // must never take the game down).
                    let _ = err;
                },
                None,
            )
            .ok()
        };
        match config() {
            Some(stream) => match stream.play() {
                Ok(()) => AudioHost {
                    mixer,
                    stream: Some(stream),
                    state: HostState::Live { sample_rate: 48_000 },
                },
                Err(_) => AudioHost {
                    mixer,
                    stream: None,
                    state: HostState::Silent,
                },
            },
            None => AudioHost {
                mixer,
                stream: None,
                state: HostState::Silent,
            },
        }
    }

    /// Queue a sound on the bus (the game's only audio verb).
    pub fn play(&self, id: crate::synth::SoundId, gain: f32, pan: f32) {
        if let Ok(mut m) = self.mixer.lock() {
            m.play(id, gain, pan);
        }
    }

    /// Master gain 0..1 (mute-on-pause + the settings slider drive this).
    pub fn set_master(&self, gain: f32) {
        if let Ok(mut m) = self.mixer.lock() {
            m.master = gain.clamp(0.0, 1.0);
        }
    }

    /// SFX gain 0..1 (the settings slider).
    pub fn set_sfx_gain(&self, gain: f32) {
        if let Ok(mut m) = self.mixer.lock() {
            m.sfx_gain = gain.clamp(0.0, 1.0);
        }
    }

    /// Voices currently playing (the audio gate reads it).
    pub fn playing(&self) -> usize {
        self.mixer.lock().map(|m| m.playing()).unwrap_or(0)
    }

    /// The shared mixer for law tests that need direct pulls.
    pub fn mixer(&self) -> &Arc<std::sync::Mutex<Mixer>> {
        &self.mixer
    }
}
