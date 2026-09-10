//! The rebuild asset manifest (schema v2) validator — NWR-002.
//!
//! Mirrors docs/POORCRAFT-VALHEIM-STYLE-REBUILD/contracts/asset_manifest_v2
//! .schema.json in typed Rust (a JSON-schema engine is not worth the dep):
//! field presence, enums, id/source/compiled/material patterns, provenance
//! depth, LOD rules (named levels, budgets >= 1), collision shape, socket
//! names — plus pack-level laws the schema implies: unique ids, categories
//! matching the id prefix, at least one LOD, and (when asked) compiled
//! files that exist on disk.

use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct V2Manifest {
    pub schema_version: u32,
    pub coordinate_system: String,
    pub assets: Vec<V2Asset>,
}

#[derive(Debug, Deserialize)]
pub struct V2Asset {
    pub id: String,
    pub category: String,
    pub status: String,
    pub provenance: V2Provenance,
    pub source: Vec<String>,
    pub compiled: String,
    pub materials: Vec<String>,
    pub lods: Vec<V2Lod>,
    pub collision: V2Collision,
    #[serde(default)]
    pub sockets: Vec<String>,
    pub runtime_consumer: String,
    pub proof_scene: String,
}

#[derive(Debug, Deserialize)]
pub struct V2Provenance {
    pub kind: String,
    pub license_or_originality: String,
    pub reviewed: bool,
}

#[derive(Debug, Deserialize)]
pub struct V2Lod {
    pub name: String,
    pub max_triangles: u32,
}

#[derive(Debug, Deserialize)]
pub struct V2Collision {
    pub kind: String,
    pub blocks_navigation: bool,
}

pub const CATEGORIES: &[&str] = &[
    "prop",
    "module",
    "character",
    "material",
    "effect",
    "flora",
    "landmark",
];
pub const STATUSES: &[&str] = &[
    "planned",
    "source_ready",
    "compiled",
    "integrated",
    "proven",
];
pub const PROVENANCE_KINDS: &[&str] = &[
    "original_manual",
    "original_procedural",
    "compatible_license",
];
pub const LOD_NAMES: &[&str] = &["lod0", "lod1", "lod2", "impostor"];
pub const COLLISION_KINDS: &[&str] = &["none", "box", "capsule", "convex", "mesh"];

/// Full validation. `compiled_root` (when given) checks that non-planned
/// rows have their compiled file on disk — the factory honesty check.
pub fn validate_v2(json: &str, compiled_root: Option<&Path>) -> Result<V2Manifest, Vec<String>> {
    let mut errors = Vec::new();
    let m: V2Manifest = match serde_json::from_str(json) {
        Ok(m) => m,
        Err(e) => return Err(vec![format!("parse error: {e}")]),
    };

    if m.schema_version != 2 {
        errors.push(format!(
            "schema_version must be 2, got {}",
            m.schema_version
        ));
    }
    if m.coordinate_system != "meters,+Y-up,-Z-forward" {
        errors.push(format!(
            "coordinate_system must be \"meters,+Y-up,-Z-forward\", got {:?}",
            m.coordinate_system
        ));
    }
    if m.assets.is_empty() {
        errors.push("assets must have at least one row".into());
    }

    let mut ids = BTreeSet::new();
    for a in &m.assets {
        if !ids.insert(a.id.clone()) {
            errors.push(format!("{}: duplicate id", a.id));
        }
        // id pattern: <category>.<lowercase snake>
        let prefix = a.id.split('.').next().unwrap_or("");
        let rest = a.id.split('.').nth(1).unwrap_or("");
        if !CATEGORIES.contains(&prefix) || rest.is_empty() || rest != rest.to_lowercase() {
            errors.push(format!("{}: id must be <category>.<lowercase_id>", a.id));
        }
        if a.category != prefix {
            errors.push(format!("{}: category {:?} != id prefix", a.id, a.category));
        }
        if !STATUSES.contains(&a.status.as_str()) {
            errors.push(format!("{}: bad status {:?}", a.id, a.status));
        }
        if !PROVENANCE_KINDS.contains(&a.provenance.kind.as_str()) {
            errors.push(format!("{}: bad provenance kind", a.id));
        }
        if a.provenance.license_or_originality.len() < 12 {
            errors.push(format!("{}: provenance statement too short (<12)", a.id));
        }
        if a.source.is_empty()
            || !a
                .source
                .iter()
                .all(|s| s.starts_with("assets-src/") || s.starts_with("tools/"))
        {
            errors.push(format!(
                "{}: every source must be under assets-src/ or tools/",
                a.id
            ));
        }
        if !(a.compiled.starts_with("assets/compiled/") && a.compiled.ends_with(".glb")) {
            errors.push(format!("{}: compiled must be assets/compiled/**.glb", a.id));
        }
        if a.materials.is_empty() || !a.materials.iter().all(|s| s.starts_with("mat.")) {
            errors.push(format!("{}: materials must be mat.* and non-empty", a.id));
        }
        if a.lods.is_empty() {
            errors.push(format!("{}: at least one LOD required", a.id));
        }
        for l in &a.lods {
            if !LOD_NAMES.contains(&l.name.as_str()) {
                errors.push(format!("{}: bad lod name {:?}", a.id, l.name));
            }
            if l.max_triangles == 0 {
                errors.push(format!("{}: lod {} budget must be >= 1", a.id, l.name));
            }
        }
        if !COLLISION_KINDS.contains(&a.collision.kind.as_str()) {
            errors.push(format!("{}: bad collision kind", a.id));
        }
        if !a.sockets.iter().all(|s| {
            s.chars()
                .next()
                .map(|c| c.is_ascii_lowercase())
                .unwrap_or(false)
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        }) {
            errors.push(format!("{}: socket names must be lower snake_case", a.id));
        }
        if a.runtime_consumer.len() < 4 {
            errors.push(format!("{}: runtime_consumer too short", a.id));
        }
        if a.proof_scene.len() < 4 {
            errors.push(format!("{}: proof_scene too short", a.id));
        }
        // Factory honesty: compiled/integrated/proven rows need the file.
        if let Some(root) = compiled_root {
            if ["compiled", "integrated", "proven"].contains(&a.status.as_str()) {
                let p = root
                    .join("assets/compiled")
                    .join(a.compiled.trim_start_matches("assets/compiled/"));
                if !p.exists() {
                    errors.push(format!(
                        "{}: status {} but compiled file missing: {}",
                        a.id,
                        a.status,
                        p.display()
                    ));
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(m)
    } else {
        Err(errors)
    }
}

pub fn load_pack(path: &Path, compiled_root: Option<&Path>) -> Result<V2Manifest, Vec<String>> {
    let json =
        std::fs::read_to_string(path).map_err(|e| vec![format!("read {}: {e}", path.display())])?;
    validate_v2(&json, compiled_root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_row() -> String {
        r#"{
            "id": "prop.tree_test",
            "category": "prop",
            "status": "planned",
            "provenance": {"kind": "original_procedural", "license_or_originality": "Repository-owned original Rust generator.", "reviewed": true},
            "source": ["tools/assetgen/src/main.rs"],
            "compiled": "assets/compiled/prop/tree_test.glb",
            "materials": ["mat.bark"],
            "lods": [{"name": "lod0", "max_triangles": 100}],
            "collision": {"kind": "capsule", "blocks_navigation": true},
            "runtime_consumer": "pc3d_render::glb",
            "proof_scene": "asset_factory_tree"
        }"#
        .into()
    }

    fn wrap(asset: &str) -> String {
        format!(
            "{{\"schema_version\":2,\"coordinate_system\":\"meters,+Y-up,-Z-forward\",\"assets\":[{asset}]}}"
        )
    }

    #[test]
    fn real_pack_validates() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap();
        let pack = root.join("docs/POORCRAFT-VALHEIM-STYLE-REBUILD/assets/first_asset_batch.json");
        let m = load_pack(&pack, Some(&root.join("poorcraft3d"))).expect("pack validates");
        assert_eq!(m.assets.len(), 3);
    }

    #[test]
    fn variant_pack_validates_300_integrated_rows_on_disk() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap();
        let pack = root.join("docs/POORCRAFT-VALHEIM-STYLE-REBUILD/assets/variant_batch.json");
        let m = load_pack(&pack, Some(&root.join("poorcraft3d"))).expect("pack validates");
        assert!(m.assets.len() >= 300, "the owner-ordered batch ({})", m.assets.len());
        for a in &m.assets {
            assert!(a.id.contains("_v"), "{} is a variant", a.id);
        }
    }

    #[test]
    fn settlement_pack_validates_with_files_on_disk() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap();
        let pack = root.join("docs/POORCRAFT-VALHEIM-STYLE-REBUILD/assets/settlement_batch.json");
        let m = load_pack(&pack, Some(&root.join("poorcraft3d"))).expect("pack validates");
        assert_eq!(m.assets.len(), 10);
        for a in &m.assets {
            assert!(!a.sockets.is_empty(), "{} declares sockets", a.id);
        }
    }

    #[test]
    fn wilderness_pack_validates_with_files_on_disk() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap();
        let pack = root.join("docs/POORCRAFT-VALHEIM-STYLE-REBUILD/assets/wilderness_batch.json");
        let m = load_pack(&pack, Some(&root.join("poorcraft3d"))).expect("pack validates");
        assert_eq!(m.assets.len(), 9);
        // The honesty law runs against the real tree: every integrated
        // row's GLB exists (assetgen wrote them this session).
        for a in &m.assets {
            assert_eq!(a.status, "integrated", "{} integrated", a.id);
        }
    }

    #[test]
    fn minimal_row_passes() {
        assert!(validate_v2(&wrap(&good_row()), None).is_ok());
    }

    #[test]
    fn rejects_each_named_violation() {
        // Build a matrix of single-field corruptions; each must fail with
        // a message naming the problem.
        let cases: Vec<(&str, String, &str)> = vec![
            (
                "bad id",
                wrap(&good_row().replace("prop.tree_test", "PROP.tree_test")),
                "id must be",
            ),
            (
                "category mismatch",
                wrap(&good_row().replace("\"category\": \"prop\"", "\"category\": \"module\"")),
                "category",
            ),
            (
                "bad status",
                wrap(&good_row().replace("\"planned\"", "\"shipped\"")),
                "status",
            ),
            (
                "bad provenance kind",
                wrap(&good_row().replace("original_procedural", "ripped_from_a_game")),
                "provenance kind",
            ),
            (
                "short license",
                wrap(&good_row().replace("Repository-owned original Rust generator.", "mine")),
                "too short",
            ),
            (
                "bad source prefix",
                wrap(&good_row().replace("tools/assetgen/src/main.rs", "downloads/model.glb")),
                "source",
            ),
            (
                "bad compiled path",
                wrap(&good_row().replace("assets/compiled/prop/tree_test.glb", "somewhere/x.gltf")),
                "compiled",
            ),
            (
                "bad material",
                wrap(&good_row().replace("mat.bark", "bark")),
                "materials",
            ),
            (
                "no lods",
                wrap(&good_row().replace("[{\"name\": \"lod0\", \"max_triangles\": 100}]", "[]")),
                "LOD",
            ),
            (
                "bad lod name",
                wrap(&good_row().replace("\"lod0\"", "\"high\"")),
                "lod name",
            ),
            (
                "zero budget",
                wrap(&good_row().replace("\"max_triangles\": 100", "\"max_triangles\": 0")),
                "budget",
            ),
            (
                "bad collision",
                wrap(&good_row().replace("\"capsule\"", "\"forcefield\"")),
                "collision kind",
            ),
            (
                "bad socket",
                wrap(&good_row().replace(
                    "\"collision\"",
                    "\"sockets\": [\"Front Door\"], \"collision\"",
                )),
                "socket",
            ),
            (
                "bad coordinate system",
                wrap(&good_row()).replace("meters,+Y-up,-Z-forward", "feet"),
                "coordinate_system",
            ),
            (
                "bad schema version",
                wrap(&good_row()).replace("\"schema_version\":2", "\"schema_version\":1"),
                "schema_version",
            ),
        ];
        for (name, json, expect) in cases {
            let errs = validate_v2(&json, None).unwrap_err();
            assert!(
                errs.iter().any(|e| e.contains(expect)),
                "{name}: expected {expect:?} in {errs:?}"
            );
        }
    }

    #[test]
    fn factory_honesty_missing_compiled_file_fails() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap();
        let json = wrap(&good_row().replace("\"planned\"", "\"integrated\""));
        let errs = validate_v2(&json, Some(&root.join("poorcraft3d"))).unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("compiled file missing")),
            "{errs:?}"
        );
    }

    #[test]
    fn duplicate_ids_rejected() {
        let two = format!("{},{}", good_row(), good_row());
        let errs = validate_v2(&wrap(&two), None).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("duplicate")));
    }
}
