//! P3D-204: journal and snapshot persistence for terrain edits.
//!
//! Per-patch edit journals live at `edits/j<x>_<y>_<z>.journal` and
//! compacted snapshots at `edits/s<x>_<y>_<z>.snap` — both framed and
//! opened through the P3D-102 law (header → length → checksum).

use crate::framing::FrameError;
use crate::store::{write_atomic, LoadError};
use crate::{frame, unframe};
use pc3d_core::{FormatHeader, SupportedVersions};
use pc3d_world::edit::EditOp;
use pc3d_world::PatchCoord;
use std::fs;
use std::path::Path;

fn journal_rel(coord: PatchCoord) -> std::path::PathBuf {
    std::path::PathBuf::from(format!(
        "edits/j{}_{}_{}.journal",
        coord.x, coord.y, coord.z
    ))
}

fn snapshot_rel(coord: PatchCoord) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("edits/s{}_{}_{}.snap", coord.x, coord.y, coord.z))
}

/// Deterministic op encoding for a whole journal: count u64 LE, then each
/// op's fixed 48-byte record.
fn encode_ops(ops: &[EditOp]) -> Vec<u8> {
    let mut b = Vec::with_capacity(8 + ops.len() * 48);
    b.extend_from_slice(&(ops.len() as u64).to_le_bytes());
    for op in ops {
        b.extend_from_slice(&op.encode());
    }
    b
}

fn decode_ops(bytes: &[u8]) -> Result<Vec<EditOp>, LoadError> {
    if bytes.len() < 8 {
        return Err(LoadError::Framing(crate::framing::FrameError::LengthMismatch {
            declared: 8,
            actual: bytes.len(),
        }));
    }
    let count = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
    if bytes.len() != 8 + count * 48 {
        return Err(LoadError::Framing(crate::framing::FrameError::LengthMismatch {
            declared: (8 + count * 48) as u64,
            actual: bytes.len(),
        }));
    }
    let mut ops = Vec::with_capacity(count);
    for i in 0..count {
        let rec: [u8; 48] = bytes[8 + i * 48..8 + (i + 1) * 48]
            .try_into()
            .unwrap();
        let op = EditOp::decode(&rec).ok_or_else(|| {
            LoadError::Framing(crate::framing::FrameError::ChecksumMismatch {
                expected: 0,
                actual: 0,
            })
        })?;
        ops.push(op);
    }
    Ok(ops)
}

/// Atomically persist a patch's edit journal.
pub fn save_journal(
    save_root: &Path,
    world_name: &str,
    coord: PatchCoord,
    ops: &[EditOp],
    supported: &SupportedVersions,
) -> Result<(), LoadError> {
    let header = FormatHeader { save: supported.save, ..FormatHeader::current() };
    let bytes = frame(&header, &encode_ops(ops));
    let path = crate::paths::world_root(save_root, world_name).join(journal_rel(coord));
    write_atomic(&path, &bytes)?;
    Ok(())
}

/// Load a patch's edit journal through the refusal law.
pub fn load_journal(
    save_root: &Path,
    world_name: &str,
    coord: PatchCoord,
    supported: &SupportedVersions,
) -> Result<Vec<EditOp>, LoadError> {
    let path = crate::paths::world_root(save_root, world_name).join(journal_rel(coord));
    let bytes = fs::read(path)?;
    Ok(decode_ops(&unframe(&bytes, supported)?)?)
}

/// Atomically persist a compacted snapshot (4096 material bytes).
pub fn save_snapshot(
    save_root: &Path,
    world_name: &str,
    coord: PatchCoord,
    cells: &[u8],
    supported: &SupportedVersions,
) -> Result<(), LoadError> {
    let header = FormatHeader { save: supported.save, ..FormatHeader::current() };
    let bytes = frame(&header, cells);
    let path = crate::paths::world_root(save_root, world_name).join(snapshot_rel(coord));
    write_atomic(&path, &bytes)?;
    Ok(())
}

/// Load a compacted snapshot's material bytes.
pub fn load_snapshot(
    save_root: &Path,
    world_name: &str,
    coord: PatchCoord,
    supported: &SupportedVersions,
) -> Result<Vec<u8>, LoadError> {
    let path = crate::paths::world_root(save_root, world_name).join(snapshot_rel(coord));
    let bytes = fs::read(path)?;
    Ok(unframe(&bytes, supported)?)
}



#[cfg(test)]
mod tests {
    use super::*;
    use pc3d_world::edit::{EditKind, Brush};
    use pc3d_world::{CellCoord, CellMaterial};

    const SUP: SupportedVersions = SupportedVersions::epoch1();

    fn op(id: u64, tick: u64, cx: i32) -> EditOp {
        EditOp {
            id,
            tick,
            kind: if id % 2 == 0 { EditKind::Dig } else { EditKind::Fill },
            brush: Brush { center: CellCoord { x: cx, y: 7, z: 3 }, radius: 2 },
            material: CellMaterial::Rock,
        }
    }

    /// Journal round-trip through disk with the header law enforced.
    #[test]
    fn p3d204_journal_round_trips_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let coord = PatchCoord { x: -7, y: 1, z: 9 };
        let ops = vec![op(1, 1, 4), op(2, 1, 10), op(3, 2, -6)];
        save_journal(root, "world", coord, &ops, &SUP).unwrap();
        let loaded = load_journal(root, "world", coord, &SUP).unwrap();
        assert_eq!(loaded, ops);
        // The file sits at the deterministic path.
        assert!(crate::paths::world_root(root, "world")
            .join(journal_rel(coord))
            .exists());
    }

    /// A foreign or wrong-version journal file is refused by the law.
    #[test]
    fn p3d204_journal_refusals_apply_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let coord = PatchCoord { x: 0, y: 0, z: 0 };
        let path = crate::paths::world_root(root, "w").join(journal_rel(coord));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"LOREFORGE journal").unwrap();
        assert!(matches!(
            load_journal(root, "w", coord, &SUP),
            Err(LoadError::Framing(FrameError::ForeignFormat))
        ));
        let mut newer = FormatHeader::current();
        newer.save = 6;
        fs::write(&path, frame(&newer, &encode_ops(&[]))).unwrap();
        assert!(matches!(
            load_journal(root, "w", coord, &SUP),
            Err(LoadError::Framing(FrameError::Newer { section: "save", file: 6, supported: 1 }))
        ));
    }

    /// Snapshots round-trip byte-deterministically.
    #[test]
    fn p3d204_snapshot_round_trip_is_byte_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let coord = PatchCoord { x: 3, y: -2, z: 5 };
        let cells: Vec<u8> = (0..4096u32).map(|i| (i % 7) as u8).collect();
        save_snapshot(root, "snap", coord, &cells, &SUP).unwrap();
        let a = load_snapshot(root, "snap", coord, &SUP).unwrap();
        let b = load_snapshot(root, "snap", coord, &SUP).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 4096);
    }
}
