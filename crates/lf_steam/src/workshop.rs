//! Step 37: Workshop UGC. Items are mod folders; on Steam they arrive
//! via ISteamUGC subscriptions (the `steam` feature arm), and everywhere
//! else — the default build, dev, and tests — they live in a directory
//! (`ugc/` by default) this module scans. One model, both worlds.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkshopItem {
    /// The workshop id (Steam) or the folder name (dev/UGC dir).
    pub id: String,
    pub title: String,
    /// Path to the mod folder the loader can consume directly.
    pub path: String,
}

/// Scan a UGC directory for installed items: every subfolder with a
/// mod.toml is an item. Steam subscriptions land in the same shape after
/// download, so the loader treats them identically.
pub fn scan_installed(dir: &std::path::Path) -> Vec<WorkshopItem> {
    let mut items = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return items;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || !path.join("mod.toml").is_file() {
            continue;
        }
        // pull the title from the manifest (best-effort; folder name wins)
        let title = std::fs::read_to_string(path.join("mod.toml"))
            .ok()
            .and_then(|t| t.lines().find(|l| l.starts_with("name")).and_then(|l| l.split('"').nth(1).map(String::from)))
            .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());
        items.push(WorkshopItem {
            id: entry.file_name().to_string_lossy().to_string(),
            title,
            path: path.to_string_lossy().to_string(),
        });
    }
    items.sort_by(|a, b| a.id.cmp(&b.id));
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The dev/UGC path is real: a folder with a manifest scans in as an
    /// item the loader can consume.
    #[test]
    fn scan_finds_installed_items() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("castle_pack/data")).unwrap();
        std::fs::write(dir.path().join("castle_pack/mod.toml"),
            "id = \"castle_pack\"\nname = \"Castle Pack\"\nversion = \"1.0.0\"\n").unwrap();
        std::fs::create_dir_all(dir.path().join("not_a_mod")).unwrap();
        let items = scan_installed(dir.path());
        assert_eq!(items.len(), 1, "only folders with mod.toml count");
        assert_eq!(items[0].id, "castle_pack");
        assert_eq!(items[0].title, "Castle Pack");
        assert!(items[0].path.ends_with("castle_pack"));
        // missing dir -> empty, not an error
        assert!(scan_installed(std::path::Path::new("/nonexistent/ugc")).is_empty());
    }
}
