//! The POORCRAFT 3D project identity — declared once, enforced everywhere.
//!
//! The separation constants are the P3D-001 deliverable: if any of these
//! matched the original game, saves could silently cross projects. The tests
//! in this module pin every separation invariant.

use crate::SaveGuard;

/// The original game's identity (LOREFORGE build of POORCRAFT). Declared
/// here only so tests can prove P3D differs from it — never for loading.
pub const ORIGINAL_GAME_EXE: &str = "loreforge";
/// The original game's save directory at the repository root.
pub const ORIGINAL_SAVE_DIR: &str = "worlds";

/// The new game's name.
pub const PROJECT_NAME: &str = "POORCRAFT 3D";
/// The new game's executable name.
pub const PROJECT_EXE: &str = "poorcraft3d";
/// The new game's save directory at the repository root. Deliberately not
/// `worlds` and not a prefix/suffix of it.
pub const P3D_SAVE_DIR: &str = "saves3d";
/// Save-file magic: every P3D save file begins with these bytes.
pub const P3D_FORMAT_MAGIC: &[u8; 4] = b"PC3D";
/// The format epoch. P3D-002 turns this into full versioned headers with
/// refusal behavior; P3D-001 only needs the concept to exist for the guard.
pub const P3D_FORMAT_VERSION: u32 = 1;

/// How many header bytes the guard needs to see.
pub const MAGIC_LEN: usize = P3D_FORMAT_MAGIC.len();

/// Decide whether a save file may be opened by POORCRAFT 3D, from its first
/// bytes alone. Pure: no IO, no filesystem, deterministic. Refusal happens
/// here — before any parser runs — so a LOREFORGE save can never be
/// misinterpreted, and a truncated file is refused as too short, not guessed.
pub fn refuse_foreign_save(header: &[u8]) -> SaveGuard {
    use crate::SaveGuard::*;
    if header.len() < MAGIC_LEN {
        return TooShort;
    }
    if &header[..MAGIC_LEN] == P3D_FORMAT_MAGIC {
        Accepted
    } else {
        ForeignFormat
    }
}

/// The identity block the executable prints for `--identity`.
pub fn identity_block() -> String {
    format!(
        "{}\n  executable: {}\n  save dir:   {} (the original game keeps its own at `{}`)\n  format:     {} v{} — new format, no POORCRAFT compatibility (D-002)",
        PROJECT_NAME,
        PROJECT_EXE,
        P3D_SAVE_DIR,
        ORIGINAL_SAVE_DIR,
        String::from_utf8_lossy(P3D_FORMAT_MAGIC),
        P3D_FORMAT_VERSION,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SaveGuard::{Accepted, ForeignFormat, TooShort};

    /// THE P3D-001 GUARD: the two projects' identities cannot blur. Every
    /// separation axis is pinned — if someone renames one side into the
    /// other, this fails before a save can cross projects.
    #[test]
    fn p3d_identity_is_separate_from_the_original_game() {
        assert_ne!(PROJECT_EXE, ORIGINAL_GAME_EXE, "executable names must differ");
        assert_ne!(PROJECT_NAME, "LOREFORGE");
        assert_ne!(P3D_SAVE_DIR, ORIGINAL_SAVE_DIR, "save roots must differ");
        assert!(
            !ORIGINAL_SAVE_DIR.contains(P3D_SAVE_DIR) && !P3D_SAVE_DIR.contains(ORIGINAL_SAVE_DIR),
            "neither save dir may be a substring of the other"
        );
        assert_ne!(P3D_FORMAT_MAGIC, b"LORE" , "save magic must not echo the original game's identity");
    }

    /// A LOREFORGE-style save file is refused before parsing.
    #[test]
    fn guard_refuses_original_game_saves() {
        // A fabricated file in the original game's binary era (its saves are
        // bincode; some begin with a 8-byte length prefix). What matters is
        // that the first four bytes are NOT the P3D magic.
        let loreforge_style: &[u8] = &[0x2A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xDE, 0xAD];
        assert_eq!(refuse_foreign_save(loreforge_style), ForeignFormat);
        // Even a file that literally begins with the original project's name.
        let named: &[u8] = b"LOREFORGE save data ...";
        assert_eq!(refuse_foreign_save(named), ForeignFormat);
    }

    /// P3D saves are accepted exactly when the magic leads the file.
    #[test]
    fn guard_accepts_only_p3d_magic() {
        let mut ok = Vec::new();
        ok.extend_from_slice(P3D_FORMAT_MAGIC);
        ok.extend_from_slice(&P3D_FORMAT_VERSION.to_le_bytes());
        assert_eq!(refuse_foreign_save(&ok), Accepted);
        // Magic is case- and content-strict.
        assert_eq!(refuse_foreign_save(b"pc3d"), ForeignFormat);
        assert_eq!(refuse_foreign_save(b"PC3Dx"), Accepted, "extra bytes after the magic are payload");
    }

    /// Truncated headers are refused as too short, never guessed.
    #[test]
    fn guard_refuses_truncated_headers() {
        assert_eq!(refuse_foreign_save(b""), TooShort);
        assert_eq!(refuse_foreign_save(b"PC"), TooShort);
        assert_eq!(refuse_foreign_save(b"PC3"), TooShort);
    }

    /// The printed identity block names the project, the executable, and
    /// both save roots — the player-visible "who am I" answer.
    #[test]
    fn identity_block_is_complete() {
        let block = identity_block();
        assert!(block.contains(PROJECT_NAME));
        assert!(block.contains(PROJECT_EXE));
        assert!(block.contains(P3D_SAVE_DIR));
        assert!(block.contains(ORIGINAL_SAVE_DIR));
        assert!(block.contains("PC3D"));
    }
}
