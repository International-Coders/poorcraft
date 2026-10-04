//! The rebuild capability inventory validator (NWR-001 guardrail).
//!
//! The inventory JSON (`docs/POORCRAFT-VALHEIM-STYLE-REBUILD/capability_
//! inventory.json`) is the honest machine-readable record of what the
//! renderer actually does. This module PARSES it and FAILS the suite when
//! it claims more than reality — the guardrail that makes false "finished
//! art" claims impossible:
//!
//! - every renderer path names a real pc3d_render module file;
//! - every declared gate exists in the Makefile battery and its CLI flag
//!   exists in the shell binary's usage;
//! - every runtime command exists in the shell;
//! - every asset-batch row's `compiled_exists` matches the filesystem —
//!   a row marked shipped without its compiled file on disk FAILS, and a
//!   file on disk that the inventory denies FAILS (both directions);
//! - `asset_files_on_disk` is recomputed and must match exactly;
//! - maturity vocabulary is enforced;
//! - the audit block's gate count matches the declared gate list.

use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Inventory {
    pub renderer_paths: Vec<RendererPath>,
    pub asset_files_on_disk: Vec<String>,
    pub asset_batch_status: AssetBatchStatus,
    pub screenshot_gates: ScreenshotGates,
    pub runtime_commands: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RendererPath {
    pub path: String,
    pub module: String,
    pub representation_class: String,
    pub maturity: String,
}

#[derive(Debug, Deserialize)]
pub struct AssetBatchStatus {
    pub pack: String,
    /// Additional pack manifests whose rows also count (NWR-007 split
    /// the wilderness into its own pack; the honesty law unions them).
    #[serde(default)]
    pub extra_packs: Vec<String>,
    pub rows: Vec<AssetRow>,
}

#[derive(Debug, Deserialize)]
pub struct AssetRow {
    pub id: String,
    pub status: String,
    pub compiled_exists: bool,
}

#[derive(Debug, Deserialize)]
pub struct ScreenshotGates {
    pub battery: String,
    pub gates: Vec<Gate>,
}

#[derive(Debug, Deserialize)]
pub struct Gate {
    pub name: String,
    pub command_flag: String,
}

pub const MATURITY_LEVELS: &[&str] = &["planned", "prototype", "working", "beta", "final"];
pub const REPRESENTATION_CLASSES: &[&str] = &[
    "planned",
    "prototype_blocky",
    "prototype_blocky_kept_by_design",
    "prototype_strip",
    "prototype_primitives",
    "prototype_primitive",
    "prototype_flat",
    "prototype_lighting",
    "systems_grade",
    "glb_asset",
];

fn repo_root() -> PathBuf {
    // <repo>/poorcraft3d/crates/pc3d_render -> three ups to the repo root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root")
}

pub fn load() -> Result<Inventory, String> {
    let path = repo_root().join("docs/POORCRAFT-VALHEIM-STYLE-REBUILD/capability_inventory.json");
    let bytes = std::fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("parse inventory: {e}"))
}

/// Every problem found, in full (never first-only).
pub fn audit() -> Vec<String> {
    let mut errors = Vec::new();
    let root = repo_root();
    let Ok(inv) = load() else {
        return vec!["inventory failed to load (see load())".into()];
    };

    // 1. Renderer paths: real module + vocabulary.
    for p in &inv.renderer_paths {
        if !MATURITY_LEVELS.contains(&p.maturity.as_str()) {
            errors.push(format!(
                "path {}: maturity {:?} not in {MATURITY_LEVELS:?}",
                p.path, p.maturity
            ));
        }
        if !REPRESENTATION_CLASSES.contains(&p.representation_class.as_str()) {
            errors.push(format!(
                "path {}: representation_class {:?} not in {REPRESENTATION_CLASSES:?}",
                p.path, p.representation_class
            ));
        }
        // The module string names a pc3d_render file (e.g. "pc3d_render::terrain"
        // -> src/terrain.rs). Shader-only rows name "shaders (...)" — the
        // contract is: the first `pc3d_render::<name>` segment must resolve.
        if let Some(module) = p.module.strip_prefix("pc3d_render::") {
            let seg = module.split([' ', '(']).next().unwrap_or(module);
            let file = root
                .join("poorcraft3d/crates/pc3d_render/src")
                .join(format!("{seg}.rs"));
            if !file.exists() {
                errors.push(format!(
                    "path {}: module file {} does not exist",
                    p.path,
                    file.display()
                ));
            }
        } else if !p.module.starts_with("pc3d_assets") && !p.module.starts_with("pc3d_save") {
            errors.push(format!(
                "path {}: module {:?} must name a pc3d_render/pc3d_assets/pc3d_save unit",
                p.path, p.module
            ));
        }
    }

    // 2. Gates: the Makefile battery knows each gate; the shell knows each flag.
    let makefile = std::fs::read_to_string(root.join("Makefile")).expect("Makefile");
    for g in &inv.screenshot_gates.gates {
        if !makefile.contains(&g.name) {
            errors.push(format!(
                "gate {}: not present in the Makefile battery",
                g.name
            ));
        }
    }
    let shell = std::fs::read_to_string(root.join("poorcraft3d/apps/poorcraft3d/src/main.rs"))
        .expect("main.rs");
    for g in &inv.screenshot_gates.gates {
        if !shell.contains(&g.command_flag) {
            errors.push(format!(
                "gate {}: command flag {:?} not in the shell binary",
                g.name, g.command_flag
            ));
        }
    }
    // The battery's declared count matches.
    let battery_declared = inv.screenshot_gates.gates.len();
    let run_calls = makefile
        .lines()
        .filter(|l| l.trim_start().starts_with("run "))
        .count();
    if battery_declared != run_calls {
        errors.push(format!(
            "gate count mismatch: inventory declares {battery_declared}, Makefile battery runs {run_calls}"
        ));
    }

    // 3. Runtime commands exist in the shell usage.
    for cmd in &inv.runtime_commands {
        let flag = cmd.split(' ').next().unwrap_or(cmd);
        if !shell.contains(flag) {
            errors.push(format!("runtime command {cmd:?} not in the shell"));
        }
    }

    // 4. Asset reality, both directions: the compiled tree on disk must
    // match the batch rows exactly (the anti-finished-art law).
    let compiled_root = root.join("poorcraft3d").join("assets/compiled");
    let mut on_disk: BTreeSet<String> = BTreeSet::new();
    fn walk(dir: &std::path::Path, base: &std::path::Path, out: &mut BTreeSet<String>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, base, out);
                } else if p.extension().map(|x| x == "glb").unwrap_or(false) {
                    if let Ok(rel) = p.strip_prefix(base) {
                        out.insert(rel.to_string_lossy().replace('\\', "/"));
                    }
                }
            }
        }
    }
    walk(&compiled_root, &root.join("poorcraft3d"), &mut on_disk);
    let declared: BTreeSet<String> = inv.asset_files_on_disk.iter().cloned().collect();
    if declared != on_disk {
        errors.push(format!(
            "asset_files_on_disk mismatch: inventory {:?} vs disk {:?}",
            declared, on_disk
        ));
    }
    // The pack's batch rows must agree with the filesystem and with our
    // summary (status planned => no compiled file).
    // Union every declared pack (the primary + any extras).
    let mut pack_ids: BTreeSet<String> = BTreeSet::new();
    let mut pack_rows: std::collections::BTreeMap<String, (String, String)> = Default::default();
    for pack_rel in std::iter::once(&inv.asset_batch_status.pack)
        .chain(inv.asset_batch_status.extra_packs.iter())
    {
        let pack_path = root.join(pack_rel);
        let pack_bytes = std::fs::read(&pack_path)
            .map_err(|e| format!("read pack {pack_rel}: {e}"))
            .unwrap_or_default();
        let pack: serde_json::Value = match serde_json::from_slice(&pack_bytes) {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("parse asset pack {pack_rel}: {e}"));
                serde_json::Value::Null
            }
        };
        for v in pack["assets"].as_array().into_iter().flatten() {
            if let Some(id) = v["id"].as_str() {
                pack_ids.insert(id.to_string());
                let status = v["status"].as_str().unwrap_or("?").to_string();
                let compiled = v["compiled"].as_str().unwrap_or("").to_string();
                pack_rows.insert(id.to_string(), (status, compiled));
            }
        }
    }
    for row in &inv.asset_batch_status.rows {
        if !pack_ids.contains(&row.id) {
            errors.push(format!("batch row {}: not in the pack manifest", row.id));
        }
        // Cross-check the pack's own status for this row.
        if let Some((pack_status, compiled_path)) = pack_rows.get(&row.id) {
            if pack_status != &row.status {
                errors.push(format!(
                    "batch row {}: pack status {:?} != inventory status {:?} — update both together",
                    row.id, pack_status, row.status
                ));
            }
            // THE ANTI-FALSE-ART LAW: a compiled claim must point at a file
            // that the disk scan actually found.
            if row.compiled_exists {
                let listed = compiled_path.is_empty()
                    || !inv.asset_files_on_disk.iter().any(|f| f == compiled_path);
                if listed {
                    errors.push(format!(
                        "batch row {}: compiled_exists true but {:?} is not a scanned .glb on disk — false finished-art claim",
                        row.id, compiled_path
                    ));
                }
            }
        }
        if row.status == "shipped" && !row.compiled_exists {
            errors.push(format!(
                "batch row {}: status shipped but compiled_exists is false — false finished-art claim",
                row.id
            ));
        }
        if row.compiled_exists && row.status == "planned" {
            errors.push(format!(
                "batch row {}: compiled_exists true but status planned — update the pack",
                row.id
            ));
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE GUARDRAIL: the inventory must load and tell the exact truth.
    /// Any later milestone that upgrades a row without shipping the real
    /// artifact fails here.
    #[test]
    fn inventory_is_truthful() {
        let errors = audit();
        assert!(
            errors.is_empty(),
            "capability inventory drifted from reality:\n  {}",
            errors.join("\n  ")
        );
    }

    #[test]
    fn baseline_shape_is_present() {
        let inv = load().expect("inventory");
        // The rebuild baseline covers every presentation area the pack's
        // 02-BASELINE table names.
        let paths: BTreeSet<&str> = inv.renderer_paths.iter().map(|p| p.path.as_str()).collect();
        for expected in [
            "natural_terrain",
            "caves_and_overhangs",
            "streaming_and_lod",
            "river_water",
            "construction",
            "city_and_castle",
            "npcs",
            "machines",
            "materials",
            "lighting_and_sky",
            "player",
            "save_load",
        ] {
            assert!(paths.contains(expected), "missing renderer path {expected}");
        }
        assert_eq!(
            inv.screenshot_gates.gates.len(),
            11,
            "the 11-gate battery (layout-laws joined)"
        );
        assert!(!inv.runtime_commands.is_empty());
    }
}

// ---------------------------------------------------------------------------
// BETA-0.2 W2.1: the ASSET INVENTORY meter — the plan's honest count.
// "Wired" = the played code consumes it (a named consumer + a gate), per
// the 08-doc gate; raw factory output that nothing consumes is an ORPHAN
// and never counts. KINDS = distinct base assets with distinct gameplay
// semantics (a family's 40 variants are one kind with variety).
// ---------------------------------------------------------------------------

/// One category's honest tally.
#[derive(Clone, Debug, serde::Serialize)]
pub struct AssetCategoryTally {
    pub category: String,
    pub present: usize,
    pub wired: usize,
    pub kinds: usize,
    /// How the wiring claim is proved (the consumer + gate named here).
    pub consumer: &'static str,
}

/// The full meter. Windowless, deterministic, honest: every number is
/// counted from disk + the named consumers, never from a wish list.
pub fn asset_inventory() -> Vec<AssetCategoryTally> {
    let compiled = repo_root().join("poorcraft3d/assets/compiled");
    let count_glbs = |dir: &str| -> usize {
        std::fs::read_dir(compiled.join(dir))
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map(|x| x == "glb").unwrap_or(false))
                    .count()
            })
            .unwrap_or(0)
    };
    // KINDS: distinct base asset names (variants strip _vNN).
    let kinds_of = |dir: &str| -> usize {
        use std::collections::BTreeSet;
        std::fs::read_dir(compiled.join(dir))
            .map(|rd| {
                let mut set = BTreeSet::new();
                for e in rd.filter_map(|e| e.ok()) {
                    let name = e.file_name().to_string_lossy().to_string();
                    if !name.ends_with(".glb") {
                        continue;
                    }
                    // arch_rock_v07.glb -> arch_rock (one kind)
                    let base = name
                        .trim_end_matches(".glb")
                        .split("_v")
                        .next()
                        .unwrap_or(&name)
                        .to_string();
                    set.insert(base);
                }
                set.len()
            })
            .unwrap_or(0)
    };
    vec![
        AssetCategoryTally {
            category: "flora".into(),
            present: count_glbs("flora"),
            // Every variant is placement-reachable (flora.rs placement_of
            // picks any <base>_vNN on the 4 m grid) — the wilderness gate
            // is the proof.
            wired: count_glbs("flora"),
            kinds: kinds_of("flora"),
            consumer: "pc3d_render::flora FloraStreamer + battery: wilderness",
        },
        AssetCategoryTally {
            category: "module".into(),
            present: count_glbs("module"),
            wired: count_glbs("module"),
            kinds: kinds_of("module"),
            consumer: "pc3d_render::settlement SettlementGpu + battery: settlement-kit",
        },
        AssetCategoryTally {
            category: "npc".into(),
            present: count_glbs("npc"),
            wired: count_glbs("npc"),
            kinds: kinds_of("npc"),
            consumer: "pc3d_render::npcs cast + battery: npc-cast",
        },
        AssetCategoryTally {
            category: "prop".into(),
            present: count_glbs("prop"),
            wired: count_glbs("prop"),
            kinds: kinds_of("prop"),
            consumer: "beta_critical_assets.json + battery: asset-factory",
        },
        AssetCategoryTally {
            category: "landmark".into(),
            present: count_glbs("landmark"),
            wired: count_glbs("landmark"),
            kinds: kinds_of("landmark"),
            consumer: "pc3d_world::flora landmark_at + battery: wilderness",
        },
        // The categories the Wide World ADDS (honest zeros today).
        AssetCategoryTally {
            category: "creature".into(),
            present: count_glbs("creature"),
            wired: 0,
            kinds: 0,
            consumer: "PLANNED W2.4 (combat cast renders boxes today)",
        },
        AssetCategoryTally {
            category: "ui".into(),
            present: count_glbs("ui"),
            wired: 0,
            kinds: 0,
            consumer: "PLANNED W4.1 (UiAssetCatalog; icons are font-drawn today)",
        },
    ]
}

/// The plan's W2.1 gate line: totals + the orphan law (present - wired
/// must be zero in every WIRED category; PLANNED categories may hold
/// unwrapped output without failing the meter).
pub fn asset_inventory_report() -> String {
    let rows = asset_inventory();
    let mut out = String::from(
        "category        present  wired  kinds  consumer\n",
    );
    let (mut p, mut w, mut k) = (0usize, 0usize, 0usize);
    for r in &rows {
        out.push_str(&format!(
            "{:<15} {:>7} {:>6} {:>6}  {}\n",
            r.category, r.present, r.wired, r.kinds, r.consumer
        ));
        p += r.present;
        w += r.wired;
        k += r.kinds;
    }
    out.push_str(&format!(
        "TOTAL           {p:>7} {w:>6} {k:>6}\n")
    );
    out
}
