//! Multiple save slots: each world lives in `worlds/<slot-name>/` with a
//! `meta.dat` (name, type, seed, updated-at). The pre-P23 `worlds/default`
//! directory migrates to slot "World 1" on first run.

use std::path::{Path, PathBuf};

pub const WORLDS_ROOT: &str = "worlds";
/// The legacy pre-slot world directory.
pub const LEGACY_DIR: &str = "worlds/default";

/// Slot metadata, persisted as `meta.dat` inside the slot directory.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SlotMeta {
    pub name: String,
    pub world_type: lf_worldgen::WorldType,
    pub seed: u64,
    /// Unix seconds of the last save (for ordering in the picker).
    pub updated_secs: u64,
}

/// A fresh OS-entropy seed (time ^ pid, splitmix-mixed).
pub fn random_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E3779B97F4A7C15);
    let pid = std::process::id() as u64;
    let mut z = nanos ^ pid.rotate_left(32);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Keep slot names filesystem-safe.
pub fn sanitize(name: &str) -> String {
    let cleaned: String = name.trim().chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let trimmed = cleaned.trim_matches('_').to_string();
    if trimmed.is_empty() { "World".to_string() } else { trimmed.chars().take(24).collect() }
}

pub fn slot_dir(name: &str) -> PathBuf {
    slot_dir_in(Path::new(WORLDS_ROOT), name)
}

/// Slot directory under an explicit worlds root (tests use tempdirs).
pub fn slot_dir_in(root: &Path, name: &str) -> PathBuf {
    root.join(sanitize(name))
}

pub fn write_meta(dir: &Path, meta: &SlotMeta) {
    let _ = std::fs::create_dir_all(dir);
    if let Ok(bytes) = bincode::serialize(meta) {
        let _ = std::fs::write(dir.join("meta.dat"), bytes);
    }
}

pub fn read_meta(dir: &Path) -> Option<SlotMeta> {
    let bytes = std::fs::read(dir.join("meta.dat")).ok()?;
    bincode::deserialize(&bytes).ok()
}

/// All slots, most recently played first.
pub fn list_slots() -> Vec<SlotMeta> {
    list_slots_in(Path::new(WORLDS_ROOT))
}

pub fn list_slots_in(root: &Path) -> Vec<SlotMeta> {
    let mut out: Vec<SlotMeta> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            if let Some(meta) = read_meta(&entry.path()) {
                out.push(meta);
            }
        }
    }
    out.sort_by(|a, b| b.updated_secs.cmp(&a.updated_secs));
    out
}

pub fn delete_slot(name: &str) {
    let _ = std::fs::remove_dir_all(slot_dir(name));
}

/// One-time migration of the pre-slot `worlds/default` into "World 1".
pub fn migrate_legacy() {
    migrate_legacy_in(Path::new(WORLDS_ROOT));
}

pub fn migrate_legacy_in(root: &Path) {
    let legacy = root.join("default");
    if !legacy.exists() {
        return;
    }
    let target = slot_dir_in(root, "World 1");
    if target.exists() || read_meta(&legacy).is_some() {
        return; // already migrated or already slotted
    }
    let _ = std::fs::create_dir_all(root);
    if std::fs::rename(&legacy, &target).is_ok() {
        let seed = lf_voxel::world::WorldStorage::open(&target)
            .load_seed()
            .unwrap_or(12345); // the pre-P23 constant seed
        write_meta(&target, &SlotMeta {
            name: "World 1".into(),
            world_type: lf_worldgen::WorldType::Normal,
            seed,
            updated_secs: 0,
        });
        tracing::info!("migrated legacy world -> {} (seed {})", target.display(), seed);
    }
}

/// The slot the game boots into: the most recently played, else a fresh
/// "World 1" created with a random seed.
pub fn boot_slot() -> SlotMeta {
    boot_slot_in(Path::new(WORLDS_ROOT))
}

pub fn boot_slot_in(root: &Path) -> SlotMeta {
    migrate_legacy_in(root);
    if let Some(latest) = list_slots_in(root).into_iter().next() {
        return latest;
    }
    let meta = SlotMeta {
        name: "World 1".into(),
        world_type: lf_worldgen::WorldType::Normal,
        seed: random_seed(),
        updated_secs: 0,
    };
    let dir = slot_dir_in(root, &meta.name);
    let _ = std::fs::create_dir_all(&dir);
    let _ = lf_voxel::world::WorldStorage::open(&dir).save_seed(meta.seed);
    write_meta(&dir, &meta);
    meta
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_keeps_filesystem_safe_names() {
        assert_eq!(sanitize("My World!"), "My_World");
        assert_eq!(sanitize("../../../etc"), "etc");
        assert_eq!(sanitize("   "), "World");
        assert_eq!(sanitize("a-really-long-name-that-should-be-cut"), "a-really-long-name-that-");
    }

    #[test]
    fn random_seeds_vary_and_are_huge() {
        let a = random_seed();
        let b = random_seed();
        assert_ne!(a, b);
        assert_ne!(a, 12345);
        assert_ne!(b, 12345);
    }

    #[test]
    fn slot_meta_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let meta = SlotMeta {
            name: "Test".into(),
            world_type: lf_worldgen::WorldType::Amplified,
            seed: 987654321,
            updated_secs: 42,
        };
        write_meta(dir.path(), &meta);
        let back = read_meta(dir.path()).expect("meta readable");
        assert_eq!(back.name, "Test");
        assert_eq!(back.seed, 987654321);
        assert_eq!(back.world_type, lf_worldgen::WorldType::Amplified);
    }

    #[test]
    fn slots_list_orders_by_recency_and_migration_works() {
        let root = tempfile::tempdir().unwrap();
        // fresh boot creates World 1 with a random persisted seed
        let boot = boot_slot_in(root.path());
        assert_eq!(boot.name, "World 1");
        assert_ne!(boot.seed, 12345);
        let slots = list_slots_in(root.path());
        assert_eq!(slots.len(), 1);
        // a second, newer slot sorts first
        let meta2 = SlotMeta { name: "Adventure".into(), world_type: lf_worldgen::WorldType::Amplified,
            seed: 777, updated_secs: boot.updated_secs + 100 };
        let dir2 = slot_dir_in(root.path(), "Adventure");
        std::fs::create_dir_all(&dir2).unwrap();
        write_meta(&dir2, &meta2);
        let slots = list_slots_in(root.path());
        assert_eq!(slots[0].name, "Adventure");
        assert_eq!(slots.len(), 2);
    }

    #[test]
    fn legacy_default_world_migrates_with_its_seed() {
        let root = tempfile::tempdir().unwrap();
        let legacy = root.path().join("default");
        std::fs::create_dir_all(&legacy).unwrap();
        let storage = lf_voxel::world::WorldStorage::open(&legacy);
        storage.save_seed(12345).unwrap();
        let saved = storage.save_chunk(0, 0, &lf_worldgen::WorldGen::new(lf_worldgen::Seed(12345)).generate_chunk(0, 0));
        assert!(saved.is_ok());
        migrate_legacy_in(root.path());
        let target = slot_dir_in(root.path(), "World 1");
        assert!(target.exists(), "migrated dir exists");
        assert!(!legacy.exists(), "legacy dir gone");
        let meta = read_meta(&target).expect("meta written");
        assert_eq!(meta.seed, 12345);
        assert_eq!(meta.name, "World 1");
        // the chunk came along
        let storage2 = lf_voxel::world::WorldStorage::open(&target);
        assert!(storage2.load_chunk(0, 0).is_some(), "chunk survived the migration");
    }

    #[test]
    fn seed_persists_through_storage() {
        let dir = tempfile::tempdir().unwrap();
        let storage = lf_voxel::world::WorldStorage::open(dir.path());
        assert!(storage.load_seed().is_none(), "no seed yet");
        storage.save_seed(777).unwrap();
        assert_eq!(storage.load_seed(), Some(777));
    }
}
