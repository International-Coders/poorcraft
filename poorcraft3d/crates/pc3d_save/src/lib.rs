//! POORCRAFT 3D persistence — the patch store (P3D-102).
//!
//! The only crate that touches `saves3d/`. Every file on disk is framed as:
//!
//! ```text
//! [16-byte P3D header | payload length u64 LE | payload | FNV-1a-64 of payload]
//! ```
//!
//! Every open runs `pc3d_core::open_decision` FIRST (the P3D-002 refusal
//! law), then verifies length and checksum. Saves are atomic by
//! construction: write `<name>.tmp`, rename over the target — a reader
//! never observes a half-written file, and a crash leaves only a `.tmp`
//! that no loader ever reads.

pub mod build_journal;
pub mod framing;
pub mod journal;
pub mod paths;
pub mod store;

pub use framing::{frame, unframe, FrameError, FRAME_OVERHEAD};
pub use paths::{patch_rel_path, world_file_rel_path, world_root};
pub use store::{load_patch, load_world_meta, save_patch, save_world_meta, LoadError, WorldMeta};
pub use build_journal::{load_build_journal, load_build_snapshot, save_build_journal, save_build_snapshot};
pub use journal::{load_journal, load_snapshot, save_journal, save_snapshot};
