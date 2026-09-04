//! P3D-102: deterministic save paths under the P3D save root.

use pc3d_core::P3D_SAVE_DIR;
use pc3d_world::PatchCoord;
use std::path::{Path, PathBuf};

/// The world's root directory: `<saves3d>/<world_name>`.
pub fn world_root(save_root: &Path, world_name: &str) -> PathBuf {
    save_root.join(P3D_SAVE_DIR).join(world_name)
}

/// Relative path of the world meta file: `world.p3d`.
pub fn world_file_rel_path() -> PathBuf {
    PathBuf::from("world.p3d")
}

/// Relative path of a patch file: `patches/p<x>_<y>_<z>.patch`. Deterministic
/// pure function of the coordinate — signed decimal, no padding, so the same
/// patch always maps to the same name on every host.
pub fn patch_rel_path(coord: PatchCoord) -> PathBuf {
    PathBuf::from(format!(
        "patches/p{}_{}_{}.patch",
        coord.x, coord.y, coord.z
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The layout lives under saves3d (never the original game's worlds/),
    /// and patch keys are pure deterministic functions of the coordinate.
    #[test]
    fn p3d102_paths_are_deterministic_and_separated() {
        let root = world_root(Path::new("/data"), "alpha");
        let rendered = root.to_str().expect("utf8 path");
        assert!(rendered.contains("saves3d"), "must live under the P3D save root");
        assert!(!rendered.contains("worlds/"), "must never touch the original game's dir");
        assert_eq!(root, Path::new("/data/saves3d/alpha"));

        assert_eq!(world_file_rel_path().to_str(), Some("world.p3d"));
        assert_eq!(
            patch_rel_path(PatchCoord { x: -1, y: 0, z: 16 }).to_str(),
            Some("patches/p-1_0_16.patch")
        );
        // Same coordinate, same name — a different one would fork saves.
        assert_eq!(
            patch_rel_path(PatchCoord { x: -1, y: 0, z: 16 }),
            patch_rel_path(PatchCoord { x: -1, y: 0, z: 16 })
        );
        assert_ne!(
            patch_rel_path(PatchCoord { x: -1, y: 0, z: 16 }),
            patch_rel_path(PatchCoord { x: 1, y: 0, z: 16 })
        );
    }
}
