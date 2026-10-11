//! POORCRAFT 3D audio (BETA-0.4 W-B): every sound is a deterministic
//! procedural synth — pure functions of (SoundId, seed), played by a
//! cpal-hosted mixer. THE GRACEFUL LAW: audio can always degrade to
//! silent; it can never crash the game.

pub mod host;
pub mod mixer;
pub mod synth;

pub use host::{AudioHost, HostState};
pub use mixer::Mixer;
pub use synth::{render, SoundId, SAMPLE_RATE};
