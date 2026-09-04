//! P3D-102: the patch store — atomic saves, guarded loads.

use crate::framing::{unframe, FrameError};
use crate::paths::{patch_rel_path, world_file_rel_path, world_root};
use pc3d_core::{FormatHeader, SupportedVersions};
use pc3d_world::PatchCoord;
use std::fs;
use std::io;
use std::path::Path;

/// Minimal world identity saved as `world.p3d` (grows when generation lands).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldMeta {
    pub seed: u64,
    pub name: String,
}

impl WorldMeta {
    fn encode(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(8 + self.name.len());
        b.extend_from_slice(&self.seed.to_le_bytes());
        b.extend_from_slice(self.name.as_bytes());
        b
    }
    fn decode(bytes: &[u8]) -> Result<WorldMeta, FrameError> {
        if bytes.len() < 8 {
            return Err(FrameError::LengthMismatch { declared: 8, actual: bytes.len() });
        }
        let seed = u64::from_le_bytes(bytes[..8].try_into().expect("8 bytes"));
        String::from_utf8(bytes[8..].to_vec())
            .map(|name| WorldMeta { seed, name })
            .map_err(|_| FrameError::ChecksumMismatch { expected: 0, actual: 0 })
    }
}

/// Why a disk load refused. `Io` wraps the filesystem error; every other
/// variant mirrors the framing law with the numbers attached.
#[derive(Debug)]
pub enum LoadError {
    Framing(FrameError),
    Io(io::Error),
}

impl From<FrameError> for LoadError {
    fn from(e: FrameError) -> Self {
        LoadError::Framing(e)
    }
}

impl From<io::Error> for LoadError {
    fn from(e: io::Error) -> Self {
        LoadError::Io(e)
    }
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Framing(e) => write!(f, "{}", e.explanation()),
            LoadError::Io(e) => write!(f, "io: {e}"),
        }
    }
}

fn write_atomic(target: &Path, bytes: &[u8]) -> Result<(), io::Error> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut tmp = target.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = Path::new(&tmp);
    fs::write(tmp, bytes)?;
    fs::rename(tmp, target)?;
    Ok(())
}

fn read_guarded(path: &Path, supported: &SupportedVersions) -> Result<Vec<u8>, LoadError> {
    let bytes = fs::read(path)?;
    Ok(unframe(&bytes, supported)?)
}

/// Atomically save one patch's opaque payload under its deterministic key.
pub fn save_patch(
    save_root: &Path,
    world_name: &str,
    coord: PatchCoord,
    payload: &[u8],
    supported: &SupportedVersions,
) -> Result<(), LoadError> {
    let header = FormatHeader { save: supported.save, ..FormatHeader::current() };
    let bytes = crate::framing::frame(&header, payload);
    let path = world_root(save_root, world_name).join(patch_rel_path(coord));
    write_atomic(&path, &bytes)?;
    Ok(())
}

/// Load one patch. The version law runs before anything trusts the bytes.
pub fn load_patch(
    save_root: &Path,
    world_name: &str,
    coord: PatchCoord,
    supported: &SupportedVersions,
) -> Result<Vec<u8>, LoadError> {
    let path = world_root(save_root, world_name).join(patch_rel_path(coord));
    read_guarded(&path, supported)
}

/// Atomically save the world meta (seed + name).
pub fn save_world_meta(
    save_root: &Path,
    meta: &WorldMeta,
    supported: &SupportedVersions,
) -> Result<(), LoadError> {
    let header = FormatHeader { save: supported.save, ..FormatHeader::current() };
    let bytes = crate::framing::frame(&header, &meta.encode());
    let path = world_root(save_root, &meta.name).join(world_file_rel_path());
    write_atomic(&path, &bytes)?;
    Ok(())
}

/// Load the world meta.
pub fn load_world_meta(
    save_root: &Path,
    world_name: &str,
    supported: &SupportedVersions,
) -> Result<WorldMeta, LoadError> {
    let path = world_root(save_root, world_name).join(world_file_rel_path());
    let payload = read_guarded(&path, supported)?;
    Ok(WorldMeta::decode(&payload)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUP: SupportedVersions = SupportedVersions::epoch1();

    fn temp_root() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    /// Round-trip a patch through the real filesystem, then prove the
    /// bytes on disk are exactly the framed form.
    #[test]
    fn p3d102_patch_round_trips_and_frames_exactly() {
        let dir = temp_root();
        let root = dir.path();
        let coord = PatchCoord { x: -2, y: 1, z: 3 };
        save_patch(root, "alpha", coord, b"terrain-bytes", &SUP).expect("save");
        let loaded = load_patch(root, "alpha", coord, &SUP).expect("load");
        assert_eq!(loaded, b"terrain-bytes");

        // The on-disk file is the deterministic framed form at the
        // deterministic path.
        let path = world_root(root, "alpha").join(patch_rel_path(coord));
        let raw = fs::read(&path).expect("file exists");
        assert_eq!(raw, crate::framing::frame(&FormatHeader::current(), b"terrain-bytes"));
        // No tmp residue after a clean save.
        assert!(!path.with_extension("patch.tmp").exists());
        let _ = coord;
    }

    /// The full refusal matrix at the disk layer: a foreign file where a
    /// patch should be, a wrong-version file, and a corrupted payload each
    /// refuse with the precise reason. The world meta round-trips too.
    #[test]
    fn p3d102_disk_loads_refuse_foreign_wrong_and_corrupt() {
        let dir = temp_root();
        let root = dir.path();
        let coord = PatchCoord { x: 0, y: 0, z: 0 };

        // Foreign (LOREFORGE-style) bytes at a patch path.
        let foreign_path =
            world_root(root, "beta").join(patch_rel_path(coord));
        fs::create_dir_all(foreign_path.parent().unwrap()).unwrap();
        fs::write(&foreign_path, b"LOREFORGE save data").unwrap();
        assert!(matches!(
            load_patch(root, "beta", coord, &SUP),
            Err(LoadError::Framing(FrameError::ForeignFormat))
        ));

        // A newer save-section file refuses with the numbers.
        let mut newer = FormatHeader::current();
        newer.save = 7;
        fs::write(&foreign_path, crate::framing::frame(&newer, b"x")).unwrap();
        assert!(matches!(
            load_patch(root, "beta", coord, &SUP),
            Err(LoadError::Framing(FrameError::Newer { section: "save", file: 7, supported: 1 }))
        ));

        // Corrupt payload: save clean, flip a payload byte on disk.
        save_patch(root, "beta", coord, b"careful bytes", &SUP).expect("save");
        let path = world_root(root, "beta").join(patch_rel_path(coord));
        let mut raw = fs::read(&path).unwrap();
        let last_payload = raw.len() - 9;
        raw[last_payload] ^= 0x80;
        fs::write(&path, &raw).unwrap();
        assert!(matches!(
            load_patch(root, "beta", coord, &SUP),
            Err(LoadError::Framing(FrameError::ChecksumMismatch { .. }))
        ));

        // World meta round-trips with the same guard path.
        let meta = WorldMeta { seed: 0xFEED_FACE, name: "beta".into() };
        save_world_meta(root, &meta, &SUP).expect("meta save");
        assert_eq!(load_world_meta(root, "beta", &SUP).expect("meta load"), meta);
    }

    /// Atomic-by-construction: a leftover .tmp file (crash between write
    /// and rename) is invisible to loaders, and re-saving replaces cleanly.
    #[test]
    fn p3d102_tmp_residue_is_invisible_and_saves_replace() {
        let dir = temp_root();
        let root = dir.path();
        let coord = PatchCoord { x: 5, y: -5, z: 5 };
        save_patch(root, "gamma", coord, b"v1", &SUP).expect("save v1");

        // Simulate a crash: a stray tmp with garbage.
        let path = world_root(root, "gamma").join(patch_rel_path(coord));
        let mut tmp = path.clone().into_os_string();
        tmp.push(".tmp");
        fs::write(&tmp, b"garbage from a crashed save").unwrap();

        // The loader reads the clean file, never the tmp.
        assert_eq!(load_patch(root, "gamma", coord, &SUP).expect("load"), b"v1");

        // A new save atomically replaces v1 and clears the residue story.
        save_patch(root, "gamma", coord, b"v2-longer", &SUP).expect("save v2");
        assert_eq!(load_patch(root, "gamma", coord, &SUP).expect("load"), b"v2-longer");
        assert!(!Path::new(&tmp).exists(), "rename consumed the tmp name");
    }

    /// The store never writes outside its world directory.
    #[test]
    fn p3d102_store_stays_inside_the_world_dir() {
        let dir = temp_root();
        let root = dir.path();
        save_patch(root, "delta", PatchCoord { x: 0, y: 0, z: 0 }, b"x", &SUP).expect("save");
        save_world_meta(root, &WorldMeta { seed: 1, name: "delta".into() }, &SUP).expect("meta");
        // Everything written lives under saves3d/delta.
        fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
            for e in fs::read_dir(dir).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    collect(&p, out);
                } else {
                    out.push(p);
                }
            }
        }
        let mut files = Vec::new();
        collect(root, &mut files);
        assert!(!files.is_empty());
        for f in files {
            let s = f.to_str().expect("utf8");
            assert!(s.contains("saves3d"), "stray write outside save root: {s}");
            assert!(s.contains("delta"), "stray write outside world dir: {s}");
        }
    }

    use std::path::PathBuf;
}
