//! POORCRAFT 3D core — project identity and the save-sharing guard
//! (P3D-001, docs/POORCRAFT-3D/16-IMPLEMENTATION-WORK-BREAKDOWN.md).
//!
//! POORCRAFT is the original game; POORCRAFT 3D is a separate project with a
//! brand-new format (decision register D-001/D-002). Everything in this
//! module exists so the two identities can never blur: the executable name,
//! save root, and save-file magic are declared here exactly once, and the
//! guard refuses any file that does not carry the P3D magic before anything
//! tries to parse it.

/// The one place the new game's identity is declared.
pub mod identity;

/// P3D-002: versioned file headers and the refusal law for unknown versions.
pub mod header;

/// P3D-003: the deterministic simulation spine.
pub mod clock;
pub mod command;
pub mod journal;
pub mod replay;
pub mod seed;

/// P3D-004: counters, frame times, memory counters, baseline records.
pub mod profile;

/// P3D-005: the first empty-world runtime and the headless harness.
pub mod runtime;

pub use clock::{FixedClock, MAX_CATCHUP_TICKS, SIM_HZ, TICK_US};
pub use command::{CommandEnvelope, CommandSequencer};
pub use journal::{fnv1a64, EventJournal, JournalEvent};
pub use replay::ReplayDigest;
pub use seed::{stream, SeedStreams, SplitMix64};
pub use profile::{BaselineRecord, Counters, CounterId, FrameTimes, MemoryCounters};
pub use runtime::{run_headless, RuntimeStats, WorldRuntime, HEARTBEAT_TICKS};

pub use header::{
    open_decision, FormatHeader, OpenDecision, Section, SupportedVersions, HEADER_LEN,
};

pub use identity::{
    identity_block, refuse_foreign_save, ORIGINAL_GAME_EXE, ORIGINAL_SAVE_DIR, P3D_FORMAT_MAGIC,
    P3D_FORMAT_VERSION, P3D_SAVE_DIR, PROJECT_EXE, PROJECT_NAME,
};

/// Guard verdict for a candidate save file's header bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveGuard {
    /// Header carries the P3D magic at the P3D format epoch.
    Accepted,
    /// Header is too short to even contain a magic field.
    TooShort,
    /// Header belongs to some other format (including the original game).
    ForeignFormat,
}
