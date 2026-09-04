//! P3D-102 framing: header | length | payload | checksum.

use pc3d_core::{fnv1a64, FormatHeader, OpenDecision, SupportedVersions, HEADER_LEN};

/// header (16) + length (8) + checksum (8).
pub const FRAME_OVERHEAD: usize = HEADER_LEN + 8 + 8;

/// What an open refused, at file granularity. Carries the numbers so a UI
/// can say exactly what is wrong (P3D-002's law, persisted).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameError {
    TooShort,
    ForeignFormat,
    UnknownEpoch { file_epoch: u32, supported_epoch: u32 },
    Newer { section: &'static str, file: u16, supported: u16 },
    Older { section: &'static str, file: u16, supported: u16 },
    LengthMismatch { declared: u64, actual: usize },
    ChecksumMismatch { expected: u64, actual: u64 },
}

impl FrameError {
    /// The human line — refusals explain the action, never guess.
    pub fn explanation(&self) -> String {
        match self {
            FrameError::TooShort => "file shorter than a P3D frame — refused".into(),
            FrameError::ForeignFormat => {
                "not a POORCRAFT 3D file (magic mismatch) — refused".into()
            }
            FrameError::UnknownEpoch { file_epoch, supported_epoch } => format!(
                "unknown format epoch {file_epoch} (build speaks {supported_epoch}) — refused"
            ),
            FrameError::Newer { section, file, supported } => format!(
                "{} version {file} is NEWER than supported {supported} — update the game",
                section
            ),
            FrameError::Older { section, file, supported } => format!(
                "{} version {file} is OLDER than supported {supported} — cannot downgrade",
                section
            ),
            FrameError::LengthMismatch { declared, actual } => format!(
                "declared payload {declared} bytes but file holds {actual} — corrupt or truncated"
            ),
            FrameError::ChecksumMismatch { expected, actual } => format!(
                "payload checksum {actual:016x} != expected {expected:016x} — corrupt"
            ),
        }
    }
}

/// Encode: header | length LE | payload | FNV-1a-64(payload) LE.
pub fn frame(header: &FormatHeader, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(FRAME_OVERHEAD + payload.len());
    out.extend_from_slice(&header.encode());
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(payload);
    out.extend_from_slice(&fnv1a64(payload).to_le_bytes());
    out
}

/// Decode against the supported set. Runs the version law FIRST, then
/// length, then checksum. Returns the payload bytes on `Ok`.
pub fn unframe(bytes: &[u8], supported: &SupportedVersions) -> Result<Vec<u8>, FrameError> {
    // Layer 1: the P3D-002 version law (includes magic + length floor).
    match pc3d_core::open_decision(bytes, supported) {
        OpenDecision::Accepted => {}
        OpenDecision::TooShort => return Err(FrameError::TooShort),
        OpenDecision::ForeignFormat => return Err(FrameError::ForeignFormat),
        OpenDecision::UnknownEpoch { file_epoch, supported_epoch } => {
            return Err(FrameError::UnknownEpoch { file_epoch, supported_epoch })
        }
        OpenDecision::Newer { section, file, supported } => {
            return Err(FrameError::Newer {
                section: section.name(),
                file,
                supported,
            })
        }
        OpenDecision::Older { section, file, supported } => {
            return Err(FrameError::Older {
                section: section.name(),
                file,
                supported,
            })
        }
    }
    // Layer 1.5: to read the DECLARED length we need header + length field.
    // Below that the frame is unreadable (TooShort); above it, a missing
    // tail is a LengthMismatch — the precise refusal naming both numbers.
    if bytes.len() < HEADER_LEN + 8 {
        return Err(FrameError::TooShort);
    }
    // Layer 2: declared length vs actual bytes in the file.
    let declared = u64::from_le_bytes([
        bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
    ]);
    let available = bytes.len().saturating_sub(FRAME_OVERHEAD);
    if declared != available as u64 {
        return Err(FrameError::LengthMismatch { declared, actual: available });
    }
    // Layer 3: payload checksum.
    let payload = &bytes[HEADER_LEN + 8..HEADER_LEN + 8 + available];
    let sum_bytes: [u8; 8] = bytes[HEADER_LEN + 8 + available..HEADER_LEN + 8 + available + 8]
        .try_into()
        .expect("length proven above");
    let expected = u64::from_le_bytes(sum_bytes);
    let actual = fnv1a64(payload);
    if actual != expected {
        return Err(FrameError::ChecksumMismatch { expected, actual });
    }
    Ok(payload.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc3d_core::Section;

    const SUP: SupportedVersions = SupportedVersions::epoch1();

    /// The framing byte layout is the contract: 16-byte header, 8-byte LE
    /// length, payload, 8-byte LE FNV checksum — pinned against exact bytes.
    #[test]
    fn p3d102_framing_is_byte_stable() {
        let h = FormatHeader::current();
        let bytes = frame(&h, b"AB");
        assert_eq!(bytes.len(), FRAME_OVERHEAD + 2);
        assert_eq!(FRAME_OVERHEAD, 32);
        // Header block equals the header's own encoding.
        assert_eq!(&bytes[..16], &h.encode()[..]);
        // Declared length 2 LE at [16..24].
        assert_eq!(&bytes[16..24], &[2, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&bytes[24..26], b"AB");
        let expected_sum = fnv1a64(b"AB");
        assert_eq!(&bytes[26..34], &expected_sum.to_le_bytes());
    }

    /// Round-trip through the full guard stack.
    #[test]
    fn p3d102_frame_round_trips_through_all_guards() {
        let payload: Vec<u8> = (0..1000u32).map(|i| (i * 7 % 251) as u8).collect();
        let bytes = frame(&FormatHeader::current(), &payload);
        assert_eq!(unframe(&bytes, &SUP).expect("clean open"), payload);
    }

    /// The refusal matrix fires per branch, in law order: version law
    /// before length, length before checksum.
    #[test]
    fn p3d102_refusals_fire_in_law_order() {
        let h = FormatHeader::current();
        let good = frame(&h, b"payload");

        // Foreign magic beats everything.
        let mut foreign = good.clone();
        foreign[0] = b'X';
        assert_eq!(unframe(&foreign, &SUP), Err(FrameError::ForeignFormat));

        // Truncated to nothing / partial header.
        assert_eq!(unframe(&[], &SUP), Err(FrameError::TooShort));
        assert_eq!(unframe(&good[..10], &SUP), Err(FrameError::TooShort));

        // A full, valid-header frame cut short BEFORE the length field
        // reports TooShort (the header law owns the floor).
        assert_eq!(unframe(&good[..16], &SUP), Err(FrameError::TooShort));

        // Length mismatch: declared 2 but no payload bytes follow.
        let mut cut = good.clone();
        cut.truncate(HEADER_LEN + 8); // length says payload.len(), none present
        match unframe(&cut, &SUP) {
            Err(FrameError::LengthMismatch { declared, actual }) => {
                assert_eq!(declared, 7);
                assert_eq!(actual, 0);
            }
            other => panic!("expected LengthMismatch, got {other:?}"),
        }

        // Checksum mismatch: flip one payload bit in a complete frame.
        let mut corrupt = frame(&h, b"payload");
        let last = corrupt.len() - 9; // last payload byte
        corrupt[last] ^= 0x01;
        match unframe(&corrupt, &SUP) {
            Err(FrameError::ChecksumMismatch { .. }) => {}
            other => panic!("expected ChecksumMismatch, got {other:?}"),
        }

        // Wrong versions refuse BEFORE length/checksum even if those are fine.
        let mut newer = FormatHeader::current();
        newer.save = 9;
        let framed = frame(&newer, b"payload");
        assert!(matches!(
            unframe(&framed, &SUP),
            Err(FrameError::Newer { section: "save", file: 9, supported: 1 })
        ));
        let mut older = FormatHeader::current();
        older.content = 0;
        assert!(matches!(
            unframe(&frame(&older, b"x"), &SUP),
            Err(FrameError::Older { section: "content", file: 0, supported: 1 })
        ));
        let mut epoch2 = FormatHeader::current();
        epoch2.epoch = 2;
        assert!(matches!(
            unframe(&frame(&epoch2, b"x"), &SUP),
            Err(FrameError::UnknownEpoch { file_epoch: 2, supported_epoch: 1 })
        ));
        let _ = Section::World; // keep the section type visible in this scope
    }

    /// Explanations name the action.
    #[test]
    fn p3d102_explanations_name_the_action() {
        assert!(FrameError::Newer { section: "save", file: 9, supported: 1 }
            .explanation()
            .contains("update the game"));
        assert!(FrameError::Older { section: "save", file: 0, supported: 1 }
            .explanation()
            .contains("cannot downgrade"));
        assert!(FrameError::ChecksumMismatch { expected: 1, actual: 2 }
            .explanation()
            .contains("corrupt"));
    }
}
