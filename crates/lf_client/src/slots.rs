//! Multiple save slots: each world lives in `worlds/<slot-name>/` with a
//! `meta.dat` (name, type, seed, updated-at). The pre-P23 `worlds/default`
//! directory migrates to slot "World 1" on first run.

use std::path::{Path, PathBuf};

pub const WORLDS_ROOT: &str = "worlds";
/// The legacy pre-slot world directory.
pub const LEGACY_DIR: &str = "worlds/default";

/// How the world threatens the player (ui-world-craft C1). Saved per
/// world; Peaceful skips hostile spawns, Easy/Normal/Hard scale mob
/// damage and hunger pace.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum Difficulty {
    Peaceful,
    #[default]
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Peaceful => "Peaceful",
            Difficulty::Easy => "Easy",
            Difficulty::Normal => "Normal",
            Difficulty::Hard => "Hard",
        }
    }

    pub const ALL: [Difficulty; 4] = [
        Difficulty::Peaceful,
        Difficulty::Easy,
        Difficulty::Normal,
        Difficulty::Hard,
    ];

    /// Mob melee damage multiplier.
    pub fn mob_damage(self) -> f32 {
        match self {
            Difficulty::Peaceful => 0.0,
            Difficulty::Easy => 0.7,
            Difficulty::Normal => 1.0,
            Difficulty::Hard => 1.5,
        }
    }

    /// Hunger drain multiplier (Hard is stricter).
    pub fn hunger_rate(self) -> f32 {
        match self {
            Difficulty::Peaceful => 0.0,
            Difficulty::Easy => 0.85,
            Difficulty::Normal => 1.0,
            Difficulty::Hard => 1.3,
        }
    }
}

/// Survival vs Creative (ui-world-craft C1). Creative is saved to the
/// world so the toggle is real, but does not gate content yet.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
}

impl GameMode {
    pub fn label(self) -> &'static str {
        match self {
            GameMode::Survival => "Survival",
            GameMode::Creative => "Creative",
        }
    }

    pub const ALL: [GameMode; 2] = [GameMode::Survival, GameMode::Creative];

    /// Loop 329: creative behaviors as pure gates so the wiring sites read
    /// clearly and the mode semantics are testable.
    pub fn takes_damage(self) -> bool {
        self == GameMode::Survival
    }
    pub fn drains_hunger(self) -> bool {
        self == GameMode::Survival
    }
    /// Creative never consumes from the inventory (infinite blocks, scrolls
    /// stay after learning, buckets pour without emptying).
    pub fn consumes_items(self) -> bool {
        self == GameMode::Survival
    }
    pub fn may_fly(self) -> bool {
        self == GameMode::Creative
    }
    /// Creative breaks any block in one hit.
    pub fn instant_mining(self) -> bool {
        self == GameMode::Creative
    }
}

/// Slot metadata, persisted as `meta.dat` inside the slot directory.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SlotMeta {
    pub name: String,
    pub world_type: lf_worldgen::WorldType,
    pub seed: u64,
    /// Unix seconds of the last save (for ordering in the picker).
    pub updated_secs: u64,
    /// Unix seconds of creation (C2 shows it in the picker).
    #[serde(default)]
    pub created_secs: u64,
    #[serde(default)]
    pub difficulty: Difficulty,
    #[serde(default)]
    pub game_mode: GameMode,
    /// LOREFORGE version that created this world.
    #[serde(default)]
    pub version_created: String,
}

/// The pre-ui-world-craft meta shape (name/type/seed/updated only).
/// bincode can't apply serde defaults to a short file, so old metas read
/// through this and gain the new fields with sensible values.
#[derive(serde::Serialize, serde::Deserialize)]
struct LegacySlotMeta {
    name: String,
    world_type: lf_worldgen::WorldType,
    seed: u64,
    updated_secs: u64,
}

/// A fresh OS-entropy seed (time ^ pid, splitmix-mixed). A process-local
/// counter is mixed in so two calls in the same clock tick still differ —
/// the audit run caught random_seeds_vary_and_are_huge failing exactly
/// that way.
pub fn random_seed() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E3779B97F4A7C15);
    let pid = std::process::id() as u64;
    let seq = SEQ.fetch_add(1, Ordering::Relaxed) as u64;
    let mut z = nanos ^ pid.rotate_left(32) ^ seq.rotate_left(48);
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
    match bincode::deserialize::<SlotMeta>(&bytes) {
        Ok(meta) => Some(meta),
        Err(_) => {
            // worlds saved before the creation flow: 4-field metas upgrade
            // in place (created = last played, standard difficulty)
            let legacy: LegacySlotMeta = bincode::deserialize(&bytes).ok()?;
            let meta = SlotMeta {
                created_secs: legacy.updated_secs,
                difficulty: Difficulty::Easy,
                game_mode: GameMode::Survival,
                version_created: String::new(),
                ..legacy_into_meta(legacy)
            };
            Some(meta)
        }
    }
}

fn legacy_into_meta(l: LegacySlotMeta) -> SlotMeta {
    SlotMeta {
        name: l.name,
        world_type: l.world_type,
        seed: l.seed,
        updated_secs: l.updated_secs,
        ..Default::default()
    }
}

/// Stamp `genver.dat` with the current generator version, warning loudly
/// when the world was last played under a different one (unedited chunks
/// regenerate from the seed, so terrain may drift at revisit borders).
/// Returns the previous version when it differed. Pre-P25 worlds have no
/// stamp and are silently upgraded to the current version.
pub fn sync_generator_version(dir: &Path) -> Option<u32> {
    let current = lf_worldgen::GENERATOR_VERSION;
    let previous = lf_worldgen::load_generator_version(dir);
    match previous {
        Some(v) if v != current => {
            tracing::warn!(
                "world '{}' was generated with gen v{}, this build is gen v{}: \
                 revisited unedited chunks may differ from their first visit \
                 (edited chunks are safe on disk)",
                dir.display(), v, current
            );
            let _ = lf_worldgen::save_generator_version(dir, current);
            Some(v)
        }
        _ => {
            if previous.is_none() {
                let _ = lf_worldgen::save_generator_version(dir, current);
            }
            None
        }
    }
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
            created_secs: now_secs(),
            difficulty: Difficulty::Easy,
            game_mode: GameMode::Survival,
            version_created: env!("CARGO_PKG_VERSION").into(),
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
        created_secs: now_secs(),
        difficulty: Difficulty::Easy,
        game_mode: GameMode::Survival,
        version_created: env!("CARGO_PKG_VERSION").into(),
    };
    let dir = slot_dir_in(root, &meta.name);
    let _ = std::fs::create_dir_all(&dir);
    let _ = lf_voxel::world::WorldStorage::open(&dir).save_seed(meta.seed);
    let _ = lf_worldgen::save_generator_version(&dir, lf_worldgen::GENERATOR_VERSION);
    write_meta(&dir, &meta);
    meta
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Hash a non-numeric world-seed string to a u64. Stable across runs and
/// machines (std's DefaultHasher is NOT — its keys are randomized), so a
/// shared seed string always builds the same world.
pub fn hash_seed_string(s: &str) -> u64 {
    // FNV-1a 64 over the UTF-8 bytes, then splitmix-mixed for good spread.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    let mut z = h;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Loop 329: creative mode semantics — the five behavior gates.
    #[test]
    fn creative_mode_gates_behaviors() {
        use GameMode::{Creative, Survival};
        assert!(Survival.takes_damage() && !Creative.takes_damage());
        assert!(Survival.drains_hunger() && !Creative.drains_hunger());
        assert!(Survival.consumes_items() && !Creative.consumes_items());
        assert!(Creative.may_fly() && !Survival.may_fly());
        assert!(Creative.instant_mining() && !Survival.instant_mining());
    }

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
            created_secs: 40,
            difficulty: Difficulty::Hard,
            game_mode: GameMode::Creative,
            version_created: "0.4.2".into(),
        };
        write_meta(dir.path(), &meta);
        let back = read_meta(dir.path()).expect("meta readable");
        assert_eq!(back.name, "Test");
        assert_eq!(back.seed, 987654321);
        assert_eq!(back.world_type, lf_worldgen::WorldType::Amplified);
        assert_eq!(back.difficulty, Difficulty::Hard);
        assert_eq!(back.game_mode, GameMode::Creative);
        assert_eq!(back.version_created, "0.4.2");
    }

    /// Pre-creation-flow worlds carry 4-field bincode metas; reading them
    /// must upgrade in place instead of dropping the slot.
    #[test]
    fn legacy_meta_upgrades_with_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = LegacySlotMeta {
            name: "Old".into(),
            world_type: lf_worldgen::WorldType::Normal,
            seed: 1234,
            updated_secs: 77,
        };
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(dir.path().join("meta.dat"), bincode::serialize(&legacy).unwrap()).unwrap();
        let back = read_meta(dir.path()).expect("legacy meta readable");
        assert_eq!(back.name, "Old");
        assert_eq!(back.seed, 1234);
        assert_eq!(back.created_secs, 77, "created falls back to last played");
        assert_eq!(back.difficulty, Difficulty::Easy);
        assert_eq!(back.game_mode, GameMode::Survival);
    }

    #[test]
    fn difficulty_and_mode_tables() {
        assert_eq!(Difficulty::Peaceful.mob_damage(), 0.0);
        assert!(Difficulty::Hard.mob_damage() > Difficulty::Normal.mob_damage());
        assert!(Difficulty::Hard.hunger_rate() > Difficulty::Easy.hunger_rate());
    }

    #[test]
    fn seed_strings_hash_stably() {
        let a = hash_seed_string("mountains-please");
        assert_eq!(a, hash_seed_string("mountains-please"), "same string, same world");
        assert_ne!(a, hash_seed_string("Mountains-Please"), "case matters");
        assert_ne!(a, hash_seed_string("mountains-pleas"), "every character matters");
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
            seed: 777, updated_secs: boot.updated_secs + 100, ..Default::default() };
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

    #[test]
    fn boot_slot_stamps_generator_version() {
        let root = tempfile::tempdir().unwrap();
        let boot = boot_slot_in(root.path());
        let dir = slot_dir_in(root.path(), &boot.name);
        assert_eq!(
            lf_worldgen::load_generator_version(&dir),
            Some(lf_worldgen::GENERATOR_VERSION),
            "fresh slots carry the current generator version"
        );
    }

    #[test]
    fn generator_version_sync_detects_mismatch_and_upgrades() {
        let dir = tempfile::tempdir().unwrap();
        // pre-P25 world: no stamp at all -> silently upgraded
        assert_eq!(sync_generator_version(dir.path()), None);
        assert_eq!(lf_worldgen::load_generator_version(dir.path()), Some(lf_worldgen::GENERATOR_VERSION));
        // matching stamp -> no report
        assert_eq!(sync_generator_version(dir.path()), None);
        // stale stamp -> reported, then updated to current
        lf_worldgen::save_generator_version(dir.path(), lf_worldgen::GENERATOR_VERSION.wrapping_add(1)).unwrap();
        assert_eq!(sync_generator_version(dir.path()), Some(lf_worldgen::GENERATOR_VERSION.wrapping_add(1)));
        assert_eq!(lf_worldgen::load_generator_version(dir.path()), Some(lf_worldgen::GENERATOR_VERSION));
    }
}
