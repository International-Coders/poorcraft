//! P3D-402: entity registry persistence through the header law.

use crate::framing::{frame, unframe, FrameError};
use crate::store::{write_atomic, LoadError};
use pc3d_core::{FormatHeader, SupportedVersions};
use pc3d_world::entities::EntityRegistry;
use std::fs;
use std::path::Path;

/// Atomically persist the entity registry.
pub fn save_entities(
    save_root: &Path,
    world_name: &str,
    registry: &EntityRegistry,
    supported: &SupportedVersions,
) -> Result<(), LoadError> {
    let header = FormatHeader { save: supported.save, ..FormatHeader::current() };
    let bytes = frame(&header, &registry.encode());
    let path = crate::paths::world_root(save_root, world_name)
        .join(std::path::PathBuf::from("entities/registry.p3d"));
    write_atomic(&path, &bytes)?;
    Ok(())
}

/// Load the entity registry through the refusal law.
pub fn load_entities(
    save_root: &Path,
    world_name: &str,
    supported: &SupportedVersions,
) -> Result<EntityRegistry, LoadError> {
    let path = crate::paths::world_root(save_root, world_name)
        .join(std::path::PathBuf::from("entities/registry.p3d"));
    let bytes = fs::read(path)?;
    let payload = unframe(&bytes, supported)?;
    EntityRegistry::decode(&payload)
        .ok_or(LoadError::Framing(FrameError::ChecksumMismatch { expected: 0, actual: 0 }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc3d_world::{CellCoord, EntityKind};

    const SUP: SupportedVersions = SupportedVersions::epoch1();

    /// Registry round-trips through disk with ids, kinds, cells, and the
    /// id high-water mark intact; foreign files refuse.
    #[test]
    fn p3d402_entities_persist_and_refuse() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut reg = EntityRegistry::new();
        reg.spawn(EntityKind::Villager, CellCoord { x: 3, y: 1, z: 4 }, 99);
        reg.spawn(EntityKind::Animal, CellCoord { x: -6, y: 0, z: 8 }, 1);
        save_entities(root, "w", &reg, &SUP).unwrap();
        let loaded = load_entities(root, "w", &SUP).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.encode(), reg.encode(), "exact round-trip");

        let path = crate::paths::world_root(root, "w")
            .join(std::path::PathBuf::from("entities/registry.p3d"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"LOREFORGE entity data, long enough").unwrap();
        assert!(matches!(
            load_entities(root, "w", &SUP),
            Err(LoadError::Framing(FrameError::ForeignFormat))
        ));
    }
}
