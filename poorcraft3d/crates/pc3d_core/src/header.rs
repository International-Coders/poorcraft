//! P3D-002: the versioned file header and its refusal law
//! (docs/POORCRAFT-3D/16-IMPLEMENTATION-WORK-BREAKDOWN.md, P3D-000).
//!
//! Every POORCRAFT 3D file begins with one fixed little-endian layout:
//! magic (4) | format epoch (u32) | world, save, content, protocol (u16 each)
//! = 18 bytes. The layout IS the contract — encode/decode are hand-rolled,
//! byte-exact, and dependency-free so no serialization choice can silently
//! move a field.
//!
//! [`open_decision`] is the only way anything may open a P3D file: it layers
//! on top of the P3D-001 magic guard and refuses — with a machine-readable
//! reason and a human line — anything whose epoch or section versions are
//! unknown. Policy at epoch 1: mismatches refuse; never guess, never
//! migrate silently (decision register D-002).

use crate::identity::{refuse_foreign_save, P3D_FORMAT_MAGIC};
use crate::SaveGuard;

/// Byte length of a [`FormatHeader`] on disk.
pub const HEADER_LEN: usize = 4 + 4 + 4 * 2;

/// Independent version axes. A file is openable only when EVERY axis matches
/// the supported set — block content, saves, and network peers move at
/// different speeds, so they version separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatHeader {
    /// Format epoch. A mismatch here means "not this format family at all".
    pub epoch: u32,
    pub world: u16,
    pub save: u16,
    pub content: u16,
    pub protocol: u16,
}

/// What this build supports. Epoch 1 is the first format; sections start at 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SupportedVersions {
    pub epoch: u32,
    pub world: u16,
    pub save: u16,
    pub content: u16,
    pub protocol: u16,
}

impl SupportedVersions {
    pub const fn epoch1() -> Self {
        SupportedVersions { epoch: 1, world: 1, save: 1, content: 1, protocol: 1 }
    }
}

impl FormatHeader {
    /// The current build's header, as written into every new file.
    pub const fn current() -> Self {
        let s = SupportedVersions::epoch1();
        FormatHeader { epoch: s.epoch, world: s.world, save: s.save, content: s.content, protocol: s.protocol }
    }

    pub fn encode(&self) -> [u8; HEADER_LEN] {
        let mut b = [0u8; HEADER_LEN];
        b[0..4].copy_from_slice(P3D_FORMAT_MAGIC);
        b[4..8].copy_from_slice(&self.epoch.to_le_bytes());
        b[8..10].copy_from_slice(&self.world.to_le_bytes());
        b[10..12].copy_from_slice(&self.save.to_le_bytes());
        b[12..14].copy_from_slice(&self.content.to_le_bytes());
        b[14..16].copy_from_slice(&self.protocol.to_le_bytes());
        b
    }

    /// Decode a header from the front of `bytes`. `None` when too short.
    pub fn decode(bytes: &[u8]) -> Option<FormatHeader> {
        if bytes.len() < HEADER_LEN {
            return None;
        }
        let u16_at = |off: usize| u16::from_le_bytes([bytes[off], bytes[off + 1]]);
        Some(FormatHeader {
            epoch: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            world: u16_at(8),
            save: u16_at(10),
            content: u16_at(12),
            protocol: u16_at(14),
        })
    }
}

/// Which section a refusal is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    World,
    Save,
    Content,
    Protocol,
}

impl Section {
    pub fn name(self) -> &'static str {
        match self {
            Section::World => "world",
            Section::Save => "save",
            Section::Content => "content",
            Section::Protocol => "protocol",
        }
    }
    pub const ALL: [Section; 4] =
        [Section::World, Section::Save, Section::Content, Section::Protocol];
}

/// The loader's verdict on a candidate file. `Accepted` is the only verdict
/// that may proceed to a parser; every other variant carries both the exact
/// numbers and a human line, because "save is newer than the game" and
/// "save is older than the game" demand different player actions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenDecision {
    Accepted,
    /// Fewer bytes than a header — nothing to decide from.
    TooShort,
    /// The P3D-001 magic guard's verdicts, passed through.
    ForeignFormat,
    UnknownEpoch { file_epoch: u32, supported_epoch: u32 },
    Newer { section: Section, file: u16, supported: u16 },
    Older { section: Section, file: u16, supported: u16 },
}

impl OpenDecision {
    /// The human line for this verdict. Refusals explain the action:
    /// newer file -> update the game; older file -> the build is too new.
    pub fn explanation(&self) -> String {
        match self {
            OpenDecision::Accepted => "accepted".into(),
            OpenDecision::TooShort => {
                format!("file is shorter than a {HEADER_LEN}-byte header — refused")
            }
            OpenDecision::ForeignFormat => {
                "not a POORCRAFT 3D file (magic mismatch) — refused".into()
            }
            OpenDecision::UnknownEpoch { file_epoch, supported_epoch } => format!(
                "unknown format epoch {file_epoch} (this build speaks epoch {supported_epoch}) — \
                 the file comes from a different format family; refused"
            ),
            OpenDecision::Newer { section, file, supported } => format!(
                "{} version {} is NEWER than this build supports ({}) — \
                 update the game to open this file",
                section.name(),
                file,
                supported
            ),
            OpenDecision::Older { section, file, supported } => format!(
                "{} version {} is OLDER than this build supports ({}) — \
                 this build cannot downgrade; refused",
                section.name(),
                file,
                supported
            ),
        }
    }
}

/// The only way anything may open a P3D file. Layered outside the P3D-001
/// magic guard: the guard answers "is this even our format", this answers
/// "is this build allowed to parse it". Pure — bytes in, verdict out.
pub fn open_decision(bytes: &[u8], supported: &SupportedVersions) -> OpenDecision {
    // Outermost layer: the P3D-001 guard owns magic rejection.
    if refuse_foreign_save(bytes) == SaveGuard::TooShort {
        return OpenDecision::TooShort;
    }
    if refuse_foreign_save(bytes) == SaveGuard::ForeignFormat {
        return OpenDecision::ForeignFormat;
    }
    let header = FormatHeader::decode(bytes).expect("length proven above");
    if header.epoch != supported.epoch {
        return OpenDecision::UnknownEpoch {
            file_epoch: header.epoch,
            supported_epoch: supported.epoch,
        };
    }
    for (section, file_v, sup) in [
        (Section::World, header.world, supported.world),
        (Section::Save, header.save, supported.save),
        (Section::Content, header.content, supported.content),
        (Section::Protocol, header.protocol, supported.protocol),
    ] {
        if file_v > sup {
            return OpenDecision::Newer { section, file: file_v, supported: sup };
        }
        if file_v < sup {
            return OpenDecision::Older { section, file: file_v, supported: sup };
        }
    }
    OpenDecision::Accepted
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The byte layout is the contract: this exact 16-byte vector must never
    /// move. If it changes, every file ever written changes meaning.
    #[test]
    fn p3d002_layout_is_byte_stable() {
        let h = FormatHeader { epoch: 1, world: 1, save: 2, content: 3, protocol: 4 };
        let bytes = h.encode();
        assert_eq!(bytes.len(), HEADER_LEN);
        assert_eq!(HEADER_LEN, 16);
        let expected: [u8; 16] = [
            b'P', b'C', b'3', b'D', //
            1, 0, 0, 0, // epoch 1 LE
            1, 0, // world
            2, 0, // save
            3, 0, // content
            4, 0, // protocol
        ];
        assert_eq!(bytes, expected, "header layout moved — that is a format break");
    }

    #[test]
    fn p3d002_round_trips() {
        let h = FormatHeader { epoch: 7, world: 11, save: 12, content: 13, protocol: 14 };
        let bytes = h.encode();
        assert_eq!(FormatHeader::decode(&bytes), Some(h));
        // Extra payload after the header does not disturb decoding.
        let mut with_payload = bytes.to_vec();
        with_payload.extend_from_slice(b"payload");
        assert_eq!(FormatHeader::decode(&with_payload), Some(h));
    }

    /// Every refusal branch, per section: newer refuses with "update the
    /// game", older refuses with "cannot downgrade", and the first offending
    /// section (world, save, content, protocol order) is the one named.
    #[test]
    fn p3d002_refuses_unknown_versions_per_section() {
        let sup = SupportedVersions::epoch1();
        let ok = FormatHeader::current().encode();
        assert_eq!(open_decision(&ok, &sup), OpenDecision::Accepted);

        let bump = |mut h: FormatHeader, s: Section, v: u16| {
            match s {
                Section::World => h.world = v,
                Section::Save => h.save = v,
                Section::Content => h.content = v,
                Section::Protocol => h.protocol = v,
            }
            h.encode()
        };
        for section in Section::ALL {
            let newer = open_decision(&bump(FormatHeader::current(), section, 2), &sup);
            assert_eq!(
                newer,
                OpenDecision::Newer { section, file: 2, supported: 1 },
                "newer {} must refuse",
                section.name()
            );
            assert!(newer.explanation().contains("update the game"));

            // v0 is OLDER than supported 1 (versions never drop to 0 in practice,
            // but the law must hold for any smaller number).
            let older = open_decision(&bump(FormatHeader::current(), section, 0), &sup);
            assert_eq!(
                older,
                OpenDecision::Older { section, file: 0, supported: 1 },
                "older {} must refuse",
                section.name()
            );
            assert!(older.explanation().contains("cannot downgrade"));
        }

        // Only the FIRST offending section is reported (deterministic order).
        let mut multi = FormatHeader::current();
        multi.save = 9;
        multi.protocol = 9;
        let d = open_decision(&multi.encode(), &sup);
        assert_eq!(d, OpenDecision::Newer { section: Section::Save, file: 9, supported: 1 });
    }

    #[test]
    fn p3d002_refuses_unknown_epochs() {
        let sup = SupportedVersions::epoch1();
        let mut h = FormatHeader::current();
        h.epoch = 2;
        let d = open_decision(&h.encode(), &sup);
        assert_eq!(d, OpenDecision::UnknownEpoch { file_epoch: 2, supported_epoch: 1 });
        assert!(d.explanation().contains("different format family"));
        h.epoch = 0;
        assert!(matches!(
            open_decision(&h.encode(), &sup),
            OpenDecision::UnknownEpoch { file_epoch: 0, .. }
        ));
    }

    /// Layering: the P3D-001 guard's verdicts pass through unchanged, and a
    /// LOREFORGE-style file is refused as foreign BEFORE version logic runs.
    #[test]
    fn p3d002_layers_on_the_p3d001_guard() {
        let sup = SupportedVersions::epoch1();
        assert_eq!(open_decision(&[], &sup), OpenDecision::TooShort);
        assert_eq!(open_decision(b"PC3", &sup), OpenDecision::TooShort);
        assert_eq!(open_decision(b"LOREFORGE save data", &sup), OpenDecision::ForeignFormat);
        assert_eq!(open_decision(b"pc3d too lowercase", &sup), OpenDecision::ForeignFormat);
    }

    /// Deciding is pure: the same bytes always produce the same verdict.
    #[test]
    fn p3d002_decision_is_deterministic() {
        let sup = SupportedVersions::epoch1();
        let bytes = FormatHeader::current().encode();
        let first = open_decision(&bytes, &sup);
        for _ in 0..10 {
            assert_eq!(open_decision(&bytes, &sup), first);
        }
        assert_eq!(first.explanation(), "accepted");
    }
}
