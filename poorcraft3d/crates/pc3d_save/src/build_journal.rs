//! P3D-205: construction journal and snapshot persistence.
//!
//! Build journals live at `edits/b<x>_<y>_<z>.build`, construction
//! snapshots (built cell records) at `edits/s<x>_<y>_<z>.bsnap` — framed
//! and opened through the P3D-102 law like every other file.

use crate::framing::{frame, unframe, FrameError};
use crate::store::{write_atomic, LoadError};
use pc3d_core::{FormatHeader, SupportedVersions};
use pc3d_world::build::{BuildOp, Construction};
use pc3d_world::{CellCoord, PatchCoord};
use std::fs;
use std::path::Path;

fn build_journal_rel(coord: PatchCoord) -> std::path::PathBuf {
    std::path::PathBuf::from(format!(
        "edits/b{}_{}_{}.build",
        coord.x, coord.y, coord.z
    ))
}

fn build_snap_rel(coord: PatchCoord) -> std::path::PathBuf {
    std::path::PathBuf::from(format!(
        "edits/s{}_{}_{}.bsnap",
        coord.x, coord.y, coord.z
    ))
}

/// Journal encoding: count u64 LE, then each op's fixed 48-byte record.
fn encode_ops(ops: &[BuildOp]) -> Vec<u8> {
    let mut b = Vec::with_capacity(8 + ops.len() * 48);
    b.extend_from_slice(&(ops.len() as u64).to_le_bytes());
    for op in ops {
        b.extend_from_slice(&op.encode());
    }
    b
}

fn decode_ops(bytes: &[u8]) -> Result<Vec<BuildOp>, LoadError> {
    if bytes.len() < 8 {
        return Err(LoadError::Framing(FrameError::LengthMismatch {
            declared: 8,
            actual: bytes.len(),
        }));
    }
    let count = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
    if bytes.len() != 8 + count * 48 {
        return Err(LoadError::Framing(FrameError::LengthMismatch {
            declared: (8 + count * 48) as u64,
            actual: bytes.len(),
        }));
    }
    let mut ops = Vec::with_capacity(count);
    for i in 0..count {
        let rec: [u8; 48] = bytes[8 + i * 48..8 + (i + 1) * 48].try_into().unwrap();
        let op =
            BuildOp::decode(&rec).ok_or(LoadError::Framing(FrameError::ChecksumMismatch {
                expected: 0,
                actual: 0,
            }))?;
        ops.push(op);
    }
    Ok(ops)
}

/// Snapshot encoding: count u64 LE, then per built cell:
/// lx,ly,lz i32 | material u8 | pad 3 | owner u64 (24 bytes each).
fn encode_construction(c: &Construction) -> Vec<u8> {
    let mut built = Vec::new();
    let o = c.coord.origin();
    let ax = o.x.div_euclid(1000) as i32;
    let ay = o.y.div_euclid(1000) as i32;
    let az = o.z.div_euclid(1000) as i32;
    let n = PATCH_AXIS_I64;
    for (i, slot) in c.cells.iter().enumerate() {
        let Some(b) = slot else { continue };
        let l = i as i64;
        let lz = l % n;
        let ly = (l / n) % n;
        let lx = l / (n * n);
        let wx = (ax as i64 + lx) as i32;
        let wy = (ay as i64 + ly) as i32;
        let wz = (az as i64 + lz) as i32;
        built.push((wx, wy, wz, *b));
    }
    let mut bytes = Vec::with_capacity(8 + built.len() * 24);
    bytes.extend_from_slice(&(built.len() as u64).to_le_bytes());
    for (wx, wy, wz, b) in built {
        bytes.extend_from_slice(&wx.to_le_bytes());
        bytes.extend_from_slice(&wy.to_le_bytes());
        bytes.extend_from_slice(&wz.to_le_bytes());
        bytes.push(b.material as u8);
        bytes.extend_from_slice(&[0, 0, 0]);
        bytes.extend_from_slice(&b.owner.to_le_bytes());
    }
    bytes
}

fn decode_construction(coord: PatchCoord, bytes: &[u8]) -> Result<Construction, LoadError> {
    if bytes.len() < 8 {
        return Err(LoadError::Framing(FrameError::LengthMismatch {
            declared: 8,
            actual: bytes.len(),
        }));
    }
    let count = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
    if bytes.len() != 8 + count * 24 {
        return Err(LoadError::Framing(FrameError::LengthMismatch {
            declared: (8 + count * 24) as u64,
            actual: bytes.len(),
        }));
    }
    let mut c = Construction::new(coord);
    for i in 0..count {
        let rec = &bytes[8 + i * 24..8 + (i + 1) * 24];
        let cell = CellCoord {
            x: i32::from_le_bytes(rec[0..4].try_into().unwrap()),
            y: i32::from_le_bytes(rec[4..8].try_into().unwrap()),
            z: i32::from_le_bytes(rec[8..12].try_into().unwrap()),
        };
        let material = pc3d_world::CellMaterial::from_code(rec[12])
            .ok_or(LoadError::Framing(FrameError::ChecksumMismatch {
                expected: 0,
                actual: 0,
            }))?;
        let owner = u64::from_le_bytes(rec[16..24].try_into().unwrap());
        c.place(cell, pc3d_world::BuildBlock { material, owner })
            .map_err(|_| {
                LoadError::Framing(FrameError::ChecksumMismatch { expected: 0, actual: 0 })
            })?;
    }
    Ok(c)
}

/// Persist a build journal atomically.
pub fn save_build_journal(
    save_root: &Path,
    world_name: &str,
    coord: PatchCoord,
    ops: &[BuildOp],
    supported: &SupportedVersions,
) -> Result<(), LoadError> {
    let header = FormatHeader { save: supported.save, ..FormatHeader::current() };
    let bytes = frame(&header, &encode_ops(ops));
    let path = crate::paths::world_root(save_root, world_name).join(build_journal_rel(coord));
    write_atomic(&path, &bytes)?;
    Ok(())
}

/// Load a build journal through the refusal law.
pub fn load_build_journal(
    save_root: &Path,
    world_name: &str,
    coord: PatchCoord,
    supported: &SupportedVersions,
) -> Result<Vec<BuildOp>, LoadError> {
    let path = crate::paths::world_root(save_root, world_name).join(build_journal_rel(coord));
    let bytes = fs::read(path)?;
    Ok(decode_ops(&unframe(&bytes, supported)?)?)
}

/// Persist a construction snapshot atomically.
pub fn save_build_snapshot(
    save_root: &Path,
    world_name: &str,
    coord: PatchCoord,
    c: &Construction,
    supported: &SupportedVersions,
) -> Result<(), LoadError> {
    let header = FormatHeader { save: supported.save, ..FormatHeader::current() };
    let bytes = frame(&header, &encode_construction(c));
    let path = crate::paths::world_root(save_root, world_name).join(build_snap_rel(coord));
    write_atomic(&path, &bytes)?;
    Ok(())
}

/// Load a construction snapshot through the refusal law.
pub fn load_build_snapshot(
    save_root: &Path,
    world_name: &str,
    coord: PatchCoord,
    supported: &SupportedVersions,
) -> Result<Construction, LoadError> {
    let path = crate::paths::world_root(save_root, world_name).join(build_snap_rel(coord));
    let bytes = fs::read(path)?;
    Ok(decode_construction(coord, &unframe(&bytes, supported)?)?)
}

const PATCH_AXIS_I64: i64 = 16;

#[cfg(test)]
mod tests {
    use super::*;
    use pc3d_world::{BuildKind, CellCoord, CellMaterial};

    const SUP: SupportedVersions = SupportedVersions::epoch1();

    /// Build journals round-trip on disk at deterministic paths.
    #[test]
    fn p3d205_build_journal_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let coord = PatchCoord { x: 2, y: -1, z: 3 };
        let ops = vec![
            BuildOp {
                id: 1,
                tick: 1,
                kind: BuildKind::Place,
                cell: CellCoord { x: 9, y: 12, z: -4 },
                material: CellMaterial::Rock,
                owner: 7,
            },
            BuildOp {
                id: 2,
                tick: 2,
                kind: BuildKind::RemoveBuild,
                cell: CellCoord { x: 9, y: 12, z: -4 },
                material: CellMaterial::Rock,
                owner: 7,
            },
        ];
        save_build_journal(root, "w", coord, &ops, &SUP).unwrap();
        assert_eq!(load_build_journal(root, "w", coord, &SUP).unwrap(), ops);
    }

    /// Construction snapshots round-trip: place, persist, reload —
    /// ownership and cells intact; foreign files refuse.
    #[test]
    fn p3d205_construction_snapshot_round_trips_and_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let coord = PatchCoord { x: 0, y: 0, z: 0 };
        let mut c = Construction::new(coord);
        c.place(
            CellCoord { x: 1, y: 2, z: 3 },
            pc3d_world::BuildBlock { material: CellMaterial::Grass, owner: 5 },
        )
        .unwrap();
        c.place(
            CellCoord { x: 14, y: 8, z: 6 },
            pc3d_world::BuildBlock { material: CellMaterial::Snow, owner: 6 },
        )
        .unwrap();
        save_build_snapshot(root, "w", coord, &c, &SUP).unwrap();

        let loaded = load_build_snapshot(root, "w", coord, &SUP).unwrap();
        assert_eq!(loaded.built_count(), 2);
        assert_eq!(
            loaded.at(CellCoord { x: 1, y: 2, z: 3 }),
            Some(pc3d_world::BuildBlock { material: CellMaterial::Grass, owner: 5 })
        );

        let path = crate::paths::world_root(root, "w").join(build_snap_rel(coord));
        fs::write(&path, b"not a build snapshot").unwrap();
        assert!(matches!(
            load_build_snapshot(root, "w", coord, &SUP),
            Err(LoadError::Framing(FrameError::ForeignFormat))
        ));
    }
}
