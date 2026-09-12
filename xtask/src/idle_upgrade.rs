use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub const REQUIRED_MARKDOWN: &[(&str, usize)] = &[
    ("README.md", 2_000),
    ("ZCODE-IDLE-TASK-PROMPT.md", 15_000),
    ("LOREFORGE-CANON-BIBLE.md", 35_000),
    ("UPGRADE-DECISION-FRAMEWORK.md", 7_000),
];

pub const MANIFEST: &str = "autonomous_upgrade_contract.json";

const FACTION_IDS: &[&str] = &[
    "accord",
    "ironborn",
    "ember_covenant",
    "free_holds",
    "ashen_order",
    "nameless",
];

#[derive(Debug, PartialEq, Eq)]
pub struct PackStats {
    pub markdown_documents: usize,
    pub bytes: usize,
    pub links_checked: usize,
    pub canon_locks: usize,
    pub proof_gates: usize,
}

fn markdown_links(text: &str) -> impl Iterator<Item = &str> {
    text.split("](")
        .skip(1)
        .filter_map(|tail| tail.split(')').next())
}

fn local_target(doc: &Path, target: &str) -> Option<PathBuf> {
    let clean = target.split('#').next().unwrap_or(target);
    if clean.is_empty()
        || clean.starts_with("http://")
        || clean.starts_with("https://")
        || clean.starts_with('/')
    {
        return None;
    }
    Some(doc.parent().unwrap_or_else(|| Path::new(".")).join(clean))
}

fn require_markers(label: &str, text: &str, markers: &[&str]) -> Result<(), String> {
    let missing: Vec<_> = markers
        .iter()
        .filter(|marker| !text.contains(**marker))
        .copied()
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("{label} is missing required markers {missing:?}"))
    }
}

fn string_array<'a>(value: &'a serde_json::Value, key: &str) -> Result<Vec<&'a str>, String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("manifest field {key:?} must be an array"))?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .ok_or_else(|| format!("manifest field {key:?} must contain only strings"))
        })
        .collect()
}

/// Validate the idle upgrade prompt, canon, decision framework, and machine
/// contract as one durable package. Gameplay truth still comes from runtime
/// proofs; this prevents the autonomous instruction itself from silently
/// losing its lore, verification, portability, or shipping obligations.
pub fn validate(root: &Path) -> Result<PackStats, String> {
    if !root.is_dir() {
        return Err(format!(
            "idle upgrade directory does not exist: {}",
            root.display()
        ));
    }

    let mut contents = std::collections::BTreeMap::new();
    let mut bytes = 0usize;
    let mut links_checked = 0usize;
    for (name, min_bytes) in REQUIRED_MARKDOWN {
        let path = root.join(name);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        if !text.starts_with("# ") {
            return Err(format!("{} must start with one H1 heading", path.display()));
        }
        if text.len() < *min_bytes {
            return Err(format!(
                "{} is unexpectedly thin ({} bytes, want at least {})",
                path.display(),
                text.len(),
                min_bytes
            ));
        }
        for target in markdown_links(&text) {
            if let Some(local) = local_target(&path, target) {
                links_checked += 1;
                if !local.is_file() {
                    return Err(format!("broken local link in {}: {target}", path.display()));
                }
            }
        }
        bytes += text.len();
        contents.insert(*name, text);
    }

    let readme = contents.get("README.md").expect("required README loaded");
    for name in [
        "ZCODE-IDLE-TASK-PROMPT.md",
        "LOREFORGE-CANON-BIBLE.md",
        "UPGRADE-DECISION-FRAMEWORK.md",
        MANIFEST,
    ] {
        if !readme.contains(name) {
            return Err(format!("README.md does not name {name}"));
        }
    }

    let prompt = contents
        .get("ZCODE-IDLE-TASK-PROMPT.md")
        .expect("required prompt loaded");
    require_markers(
        "ZCODE-IDLE-TASK-PROMPT.md",
        prompt,
        &[
            "There is no terminal design point",
            "make idle-upgrade-check",
            "LOREFORGE-CANON-BIBLE.md",
            "UPGRADE-DECISION-FRAMEWORK.md",
            "one valuable shippable job",
            "PERFORMANCE AND LIGHTWEIGHT ENGINE LAW",
            "RUST LIBRARY AND NEW-CRATE LAW",
            "PORTABILITY AND CONSOLE-READINESS LAW",
            "git push github HEAD",
            "REQUIRED END-OF-JOB REPORT",
        ],
    )?;

    let canon = contents
        .get("LOREFORGE-CANON-BIBLE.md")
        .expect("required canon loaded");
    require_markers(
        "LOREFORGE-CANON-BIBLE.md",
        canon,
        &[
            "Valdenmoor",
            "Era IV, Year 1",
            "Anima",
            "locked ambiguity",
            "The Ashenmoor Compact",
            "The Sundering",
            "The Accord of Ashenmoor",
            "The Coal Compact",
            "The Unmarked",
            "Archivist Maren Voss",
            "Foreman Dag Holtz",
            "The Smith",
            "The Great Smelter",
            "The Ember Sanctum",
            "The Ashen Archive",
            "The Sundering Scar",
            "The Nameless Warrens",
            "The player has no fixed origin",
            "LORE IMPACT",
        ],
    )?;
    for faction in [
        "The Accord",
        "The Ironborn",
        "The Ember Covenant",
        "The Free Holds",
        "The Ashen Order",
        "The Nameless",
    ] {
        if !canon.contains(faction) {
            return Err(format!("canon bible lost foundational faction {faction}"));
        }
    }

    let framework = contents
        .get("UPGRADE-DECISION-FRAMEWORK.md")
        .expect("required framework loaded");
    require_markers(
        "UPGRADE-DECISION-FRAMEWORK.md",
        framework,
        &[
            "Rust dependency gate",
            "Creating a new Rust crate",
            "Portability and console-readiness",
            "Steam Deck",
            "before-and-after",
            "There is no final design milestone",
        ],
    )?;

    let manifest_path = root.join(MANIFEST);
    let manifest_text = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("cannot read {}: {e}", manifest_path.display()))?;
    bytes += manifest_text.len();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_text)
        .map_err(|e| format!("cannot parse {}: {e}", manifest_path.display()))?;

    if manifest.get("schema_version").and_then(|v| v.as_u64()) != Some(1) {
        return Err("manifest schema_version must be 1".into());
    }
    if manifest.get("project").and_then(|v| v.as_str()) != Some("LOREFORGE / POORCRAFT 3D") {
        return Err("manifest project identity drifted".into());
    }

    let required = string_array(&manifest, "required_documents")?;
    let expected: BTreeSet<_> = REQUIRED_MARKDOWN.iter().map(|(name, _)| *name).collect();
    let actual: BTreeSet<_> = required.into_iter().collect();
    if actual != expected {
        return Err(format!(
            "manifest required_documents drifted; found {actual:?}, want {expected:?}"
        ));
    }

    let canon_locks = manifest
        .get("canon_locks")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "manifest canon_locks must be an object".to_string())?;
    let factions: BTreeSet<_> = canon_locks
        .get("factions")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "manifest canon_locks.factions must be an array".to_string())?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    let expected_factions: BTreeSet<_> = FACTION_IDS.iter().copied().collect();
    if factions != expected_factions {
        return Err(format!(
            "manifest foundational factions drifted; found {factions:?}, want {expected_factions:?}"
        ));
    }

    let eras = canon_locks
        .get("eras")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "manifest canon_locks.eras must be an array".to_string())?;
    if eras.len() != 4 {
        return Err(format!(
            "manifest must lock all four eras, found {}",
            eras.len()
        ));
    }

    let proof_gates = string_array(&manifest, "proof_gates")?;
    if proof_gates.len() < 8 {
        return Err(format!(
            "manifest needs a complete proof ladder, found {} gates",
            proof_gates.len()
        ));
    }
    let forbidden = string_array(&manifest, "forbidden")?;
    for marker in [
        "declare_the_game_permanently_finished",
        "silent_canon_change",
        "touch_unrelated_dirty_files",
        "claim_console_support_without_authorized_hardware_toolchain_and_proof",
    ] {
        if !forbidden.contains(&marker) {
            return Err(format!("manifest forbidden list lost {marker}"));
        }
    }

    Ok(PackStats {
        markdown_documents: REQUIRED_MARKDOWN.len(),
        bytes,
        links_checked,
        canon_locks: canon_locks.len(),
        proof_gates: proof_gates.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_idle_upgrade_pack_is_complete_and_lore_locked() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("docs/POORCRAFT-3D/ZCODE-IDLE-UPGRADE");
        let stats = validate(&root).expect("checked-in idle upgrade pack must stay valid");
        assert_eq!(stats.markdown_documents, 4);
        assert!(stats.bytes > 65_000, "idle pack lost substantial content");
        assert!(stats.links_checked >= 3, "README should link the pack");
        assert!(
            stats.canon_locks >= 6,
            "canon lock set is unexpectedly thin"
        );
        assert!(stats.proof_gates >= 8, "proof ladder is unexpectedly thin");
    }

    #[test]
    fn marker_guard_names_every_missing_obligation() {
        let error = require_markers(
            "prompt",
            "has performance only",
            &["performance", "lore", "push"],
        )
        .expect_err("missing markers must fail");
        assert!(error.contains("lore"));
        assert!(error.contains("push"));
    }
}
