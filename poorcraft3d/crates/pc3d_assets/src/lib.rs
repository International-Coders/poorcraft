//! pc3d_assets — manifest validation, asset lookup, material metadata
//! (visual reset R3DV-003).
//!
//! Architecture law (02-RENDERER-ARCHITECTURE.md): this crate owns manifest
//! validation, asset lookup, material metadata, and compiled asset paths.
//! It is a pure data layer — no wgpu, no winit, no world state. Nothing may
//! be imported as a game asset before it has a manifest row that passes
//! [`validate_str`].
//!
//! The contract enforced here mirrors
//! `docs/POORCRAFT-3D-VISUAL-RESET/assets/asset_manifest.schema.json`
//! (structure: required fields, enums, id pattern, additionalProperties
//! rejection) plus the semantic asset gate from
//! `docs/POORCRAFT-3D-VISUAL-RESET/04-3D-ASSET-PIPELINE.md`: duplicate ids,
//! missing consumers, absent proof scenes, forbidden source policy (no other
//! game's branded names anywhere in a row), invalid LODs, and `final` assets
//! without a runtime geometry path are all REJECTED with named errors.

use serde::{Deserialize, Serialize};

/// The canonical beta-critical manifest supplied by the visual-reset pack,
/// embedded at compile time so the validator always has the real data (a
/// test re-reads the file from disk and fails if the pack drifts).
pub const BETA_CRITICAL_JSON: &str =
    include_str!("../../../../docs/POORCRAFT-3D-VISUAL-RESET/assets/beta_critical_assets.json");

/// Owner-facing UI concept manifest for the generated alpha rescue asset pack.
/// It is intentionally a concept-source manifest until the renderer has a
/// sliced runtime atlas; tests still keep the docs, names, and PNG files in
/// sync.
pub const UI_ASSET_MANIFEST_JSON: &str =
    include_str!("../../../../docs/POORCRAFT-3D/assets/ui_asset_manifest.json");

/// GLM/Z-code owner UI rework handoff pack. This keeps the machine-readable
/// prompt pack parseable and present beside the generated assets.
pub const GLM_UI_REWORK_PACK_JSON: &str =
    include_str!("../../../../docs/POORCRAFT-3D/GLM-UI-REWORK-PACK/glm_ui_rework_manifest.json");

/// GLM/Z-code world tools and semantic asset handoff pack. This pack keeps
/// start-menu seed generation, asset semantics, GPU tooling, and wireframe
/// extraction rules machine-checkable.
pub const GLM_WORLD_TOOLS_PACK_JSON: &str =
    include_str!("../../../../docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/world_tools_manifest.json");

// ---------------------------------------------------------------------------
// Typed manifest (serde mirrors the JSON schema; enums reject out of contract)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    TerrainMaterial,
    ConstructionBlock,
    CastleModule,
    Npc,
    Machine,
    WorldProp,
    CityAnchor,
    Effect,
}

impl Category {
    pub fn key(self) -> &'static str {
        match self {
            Category::TerrainMaterial => "terrain_material",
            Category::ConstructionBlock => "construction_block",
            Category::CastleModule => "castle_module",
            Category::Npc => "npc",
            Category::Machine => "machine",
            Category::WorldProp => "world_prop",
            Category::CityAnchor => "city_anchor",
            Category::Effect => "effect",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Placeholder,
    Prototype,
    Beta,
    Final,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceKind {
    ProceduralOriginal,
    AuthoredOriginal,
    Licensed,
    AiGeneratedOriginal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GeometryKind {
    ProceduralMesh,
    GeneratedVoxelMesh,
    Gltf,
    Billboard,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LodPolicy {
    None,
    Generated,
    Lod0Lod1,
    Instanced,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub kind: ProvenanceKind,
    pub originality_reviewed: bool,
    pub license_note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Geometry {
    pub kind: GeometryKind,
    pub source_or_generator: String,
    pub triangle_budget: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Lod {
    pub policy: LodPolicy,
}

/// One manifest row. `deny_unknown_fields` enforces the schema's
/// `additionalProperties: false` on assets.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssetRow {
    pub id: String,
    pub category: Category,
    pub status: Status,
    pub provenance: Provenance,
    pub geometry: Geometry,
    pub material: String,
    pub lod: Lod,
    pub collision: Option<String>,
    pub navigation: Option<String>,
    pub runtime_consumers: Vec<String>,
    pub proof_scene: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CoordinateSystem {
    pub units: String,
    pub up_axis: UpAxis,
    pub forward_axis: ForwardAxis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum UpAxis {
    #[serde(rename = "Y")]
    Y,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum ForwardAxis {
    #[serde(rename = "-Z")]
    NegZ,
    #[serde(rename = "+Z")]
    PosZ,
}

/// A validated manifest: the typed file plus the coordinate convention the
/// renderer is bound to (meters, +Y up — matching pc3d_render's documented
/// world convention).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub project: String,
    pub coordinate_system: CoordinateSystem,
    pub assets: Vec<AssetRow>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Brand tokens forbidden by the source policy — original POORCRAFT
/// expression only (04-3D-ASSET-PIPELINE.md, zai_visual_gate.json).
const FORBIDDEN_TOKENS: &[&str] = &[
    "warcraft",
    "minecraft",
    "skyrim",
    "heroes of might and magic",
    "all the mods",
    "homm",
];

fn id_matches_pattern(id: &str) -> bool {
    let mut chars = id.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }
    let rest_ok = chars
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.' || c == '-');
    rest_ok && id.len() >= 2
}

/// Full validation: JSON parse, schema shape, and the semantic asset gate.
/// Returns every problem found (not just the first) so a bad manifest can be
/// fixed in one pass; `Ok` only when the row set is completely clean.
pub fn validate_str(json: &str) -> Result<Manifest, Vec<String>> {
    let manifest: Manifest = match serde_json::from_str(json) {
        Ok(m) => m,
        Err(e) => return Err(vec![format!("parse error: {e}")]),
    };

    let mut errors = Vec::new();

    if manifest.schema_version != 1 {
        errors.push(format!(
            "schema_version must be 1, got {}",
            manifest.schema_version
        ));
    }
    if manifest.project != "POORCRAFT 3D" {
        errors.push(format!(
            "project must be \"POORCRAFT 3D\", got {:?}",
            manifest.project
        ));
    }
    if manifest.coordinate_system.units != "meters" {
        errors.push(format!(
            "coordinate_system.units must be \"meters\", got {:?}",
            manifest.coordinate_system.units
        ));
    }
    if manifest.assets.is_empty() {
        errors.push("assets must contain at least one row".into());
    }

    // Duplicate ids.
    let mut seen = std::collections::BTreeSet::new();
    for a in &manifest.assets {
        if !seen.insert(a.id.clone()) {
            errors.push(format!("duplicate asset id {:?}", a.id));
        }
    }

    for a in &manifest.assets {
        let id = &a.id;
        let mut field_err = |msg: String| errors.push(format!("asset {id}: {msg}"));

        if !id_matches_pattern(id) {
            field_err("id must match ^[a-z0-9][a-z0-9_.-]+$".into());
        }
        if !a.provenance.originality_reviewed {
            field_err("provenance.originality_reviewed must be true".into());
        }
        if matches!(a.provenance.kind, ProvenanceKind::Licensed)
            && a.provenance
                .license_note
                .as_deref()
                .map(str::is_empty)
                .unwrap_or(true)
        {
            field_err("licensed assets require a non-empty provenance.license_note".into());
        }
        if a.geometry.source_or_generator.trim().is_empty() {
            field_err("geometry.source_or_generator must be a non-empty path/generator".into());
        }
        if matches!(a.status, Status::Final) && matches!(a.geometry.kind, GeometryKind::None) {
            field_err("a final asset must have real geometry, not kind \"none\"".into());
        }
        if a.material.trim().is_empty() {
            field_err("material must be a non-empty name".into());
        }
        if a.runtime_consumers.is_empty() {
            field_err("runtime_consumers must list at least one consumer".into());
        }
        if a.runtime_consumers.iter().any(|c| c.trim().is_empty()) {
            field_err("runtime_consumers must not contain empty strings".into());
        }
        if a.proof_scene.trim().is_empty() {
            field_err("proof_scene must name a visual proof scene".into());
        }

        // Source policy: no other game's branded expression anywhere in the
        // row's text fields.
        let haystacks: [(&str, &str); 8] = [
            ("id", id),
            (
                "geometry.source_or_generator",
                &a.geometry.source_or_generator,
            ),
            ("material", &a.material),
            (
                "license_note",
                a.provenance.license_note.as_deref().unwrap_or(""),
            ),
            ("collision", a.collision.as_deref().unwrap_or("")),
            ("navigation", a.navigation.as_deref().unwrap_or("")),
            ("runtime_consumers", &a.runtime_consumers.join(" ")),
            ("proof_scene", &a.proof_scene),
        ];
        for (field, text) in haystacks {
            // Normalize separators so "all_the_mods" reads as "all the mods".
            let lower = text.to_lowercase().replace(['_', '-'], " ");
            if let Some(token) = FORBIDDEN_TOKENS.iter().find(|t| lower.contains(*t)) {
                field_err(format!(
                    "forbidden source policy: {field} mentions {token:?} (original POORCRAFT expression only)"
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(manifest)
    } else {
        Err(errors)
    }
}

/// Validates a manifest file from disk.
pub fn validate_path(path: &std::path::Path) -> Result<Manifest, Vec<String>> {
    let bytes = std::fs::read(path).map_err(|e| vec![format!("read {}: {e}", path.display())])?;
    let json = String::from_utf8(bytes)
        .map_err(|e| vec![format!("{} is not valid UTF-8: {e}", path.display())])?;
    validate_str(&json)
}

/// The canonical beta-critical manifest, validated.
pub fn beta_critical() -> Result<Manifest, Vec<String>> {
    validate_str(BETA_CRITICAL_JSON)
}

/// Placeholder material albedos (linear RGB) keyed by the manifest's
/// material names — pc3d_assets owns material metadata (the architecture
/// law); R3DV-010 replaces these flat colors with the real atlas. Only
/// materials declared by beta-critical rows belong here.
pub fn material_albedo(name: &str) -> Option<[f32; 3]> {
    Some(match name {
        "mat.castle_stone" => [0.62, 0.60, 0.56],
        "mat.timber_roof" => [0.55, 0.30, 0.18],
        "mat.timber_metal" => [0.48, 0.42, 0.38],
        "mat.block_stone" => [0.55, 0.54, 0.50],
        "mat.block_wood" => [0.45, 0.32, 0.20],
        "mat.grass" => [0.30, 0.55, 0.22],
        "mat.soil" => [0.45, 0.32, 0.20],
        "mat.rock" => [0.55, 0.54, 0.50],
        "mat.sand" => [0.80, 0.72, 0.48],
        "mat.snow" => [0.92, 0.94, 0.97],
        "mat.water_flow" => [0.24, 0.52, 0.85],
        "mat.wood_metal" => [0.52, 0.44, 0.36],
        "mat.npc_resident" => [0.72, 0.62, 0.50],
        "mat.npc_worker" => [0.60, 0.48, 0.36],
        "mat.npc_guard" => [0.42, 0.44, 0.50],
        "mat.anchor_bed" => [0.70, 0.55, 0.45],
        "mat.anchor_work" => [0.50, 0.60, 0.70],
        "mat.anchor_idle" => [0.55, 0.68, 0.55],
        _ => return None,
    })
}

/// NWR-006 material-detail metadata. Every beta-critical material names
/// the DETAIL FAMILY it renders as (the atlas tile index the renderer
/// blends from vertex albedo weights: 0 grass / 1 rock / 2 sand / 3 snow)
/// plus its own grain seed. pc3d_assets owns the metadata; pc3d_render
/// generates the atlas deterministically from [`DETAIL_ATLAS_SPECS`].
pub fn material_detail(name: &str) -> Option<DetailSpec> {
    Some(match name {
        "mat.grass" | "mat.anchor_idle" => DetailSpec { family: 0 },
        "mat.castle_stone" | "mat.block_stone" | "mat.rock" | "mat.timber_metal"
        | "mat.wood_metal" | "mat.npc_guard" => DetailSpec { family: 1 },
        "mat.timber_roof" | "mat.block_wood" | "mat.soil" | "mat.sand" | "mat.npc_resident"
        | "mat.npc_worker" | "mat.anchor_bed" => DetailSpec { family: 2 },
        "mat.snow" | "mat.anchor_work" => DetailSpec { family: 3 },
        "mat.water_flow" => DetailSpec { family: 2 }, // riverbed grain under water
        _ => return None,
    })
}

/// One material's detail family (atlas tile).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetailSpec {
    pub family: u8,
}

/// The four atlas tiles' generator parameters [noise scale (px), seed] —
/// grass, rock, sand, snow. The renderer's procedural atlas is a pure
/// function of this table (no image files ship).
pub const DETAIL_ATLAS_SPECS: [[u32; 2]; 4] = [[8, 11], [10, 23], [6, 37], [7, 53]];

#[cfg(test)]
mod material_tests {
    use super::*;

    #[test]
    fn detail_registry_covers_every_albedo_material() {
        for name in [
            "mat.castle_stone",
            "mat.timber_roof",
            "mat.timber_metal",
            "mat.block_stone",
            "mat.block_wood",
            "mat.grass",
            "mat.soil",
            "mat.rock",
            "mat.sand",
            "mat.snow",
            "mat.water_flow",
            "mat.wood_metal",
            "mat.npc_resident",
            "mat.npc_worker",
            "mat.npc_guard",
            "mat.anchor_bed",
            "mat.anchor_work",
            "mat.anchor_idle",
        ] {
            let spec =
                material_detail(name).unwrap_or_else(|| panic!("{name} missing detail spec"));
            assert!(spec.family < 4, "{name} family out of atlas range");
        }
        assert!(material_detail("mat.plasma_gun").is_none());
        assert_eq!(DETAIL_ATLAS_SPECS.len(), 4);
    }

    #[test]
    fn registry_covers_every_beta_critical_material() {
        let beta = beta_critical().expect("manifest");
        for a in &beta.assets {
            assert!(
                material_albedo(&a.material).is_some(),
                "material {} ({}) missing from the registry",
                a.material,
                a.id
            );
        }
    }

    #[test]
    fn registry_refuses_unknown_materials() {
        assert!(material_albedo("mat.plasma_gun").is_none());
    }
}

pub mod observatory;
pub mod semantic;
pub mod v2;

impl Manifest {
    pub fn get(&self, id: &str) -> Option<&AssetRow> {
        self.assets.iter().find(|a| a.id == id)
    }

    pub fn ids(&self) -> Vec<&str> {
        self.assets.iter().map(|a| a.id.as_str()).collect()
    }

    pub fn of_category(&self, category: Category) -> Vec<&AssetRow> {
        self.assets
            .iter()
            .filter(|a| a.category == category)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN_GOOD: &str = r#"{
        "schema_version": 1,
        "project": "POORCRAFT 3D",
        "coordinate_system": {"units": "meters", "up_axis": "Y", "forward_axis": "-Z"},
        "assets": [
            {"id": "block.stone", "category": "construction_block", "status": "placeholder",
             "provenance": {"kind": "procedural_original", "originality_reviewed": true},
             "geometry": {"kind": "generated_voxel_mesh", "source_or_generator": "construction overlay"},
             "material": "mat.block_stone", "lod": {"policy": "generated"},
             "runtime_consumers": ["construction_renderer"], "proof_scene": "build_scene"}
        ]
    }"#;

    fn row_overriding(json: &str, field: &str, value: serde_json::Value) -> String {
        let mut v: serde_json::Value = serde_json::from_str(json).unwrap();
        v["assets"][0][field] = value;
        v.to_string()
    }

    #[test]
    fn beta_critical_manifest_validates() {
        let m = beta_critical().expect("beta_critical_assets.json must validate");
        assert_eq!(m.assets.len(), 19);
        // Every beta-critical category is covered by the pack.
        for cat in [
            Category::TerrainMaterial,
            Category::ConstructionBlock,
            Category::CastleModule,
            Category::Npc,
            Category::Machine,
            Category::CityAnchor,
            Category::Effect,
        ] {
            assert!(
                !m.of_category(cat).is_empty(),
                "beta-critical category {} has no rows",
                cat.key()
            );
        }
        // Spot lookup: the guard NPC's row is queryable by id.
        let guard = m.get("npc.guard").expect("npc.guard row");
        assert_eq!(guard.category, Category::Npc);
        assert!(guard
            .runtime_consumers
            .contains(&"npc_renderer".to_string()));
        assert_eq!(guard.proof_scene, "vertical_slice_city");
    }

    #[test]
    fn embedded_copy_matches_the_docs_pack_on_disk() {
        // If someone edits the pack without rebuilding, or edits the embedded
        // copy, this fails — the canonical file is the one in docs/.
        let disk_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/POORCRAFT-3D-VISUAL-RESET/assets/beta_critical_assets.json");
        let disk = std::fs::read_to_string(&disk_path).expect("pack file readable");
        assert_eq!(
            disk.trim_end(),
            BETA_CRITICAL_JSON.trim_end(),
            "embedded beta-critical manifest drifted from docs/"
        );
    }

    #[test]
    fn generated_ui_asset_manifest_names_existing_png_sheets() {
        let manifest: serde_json::Value =
            serde_json::from_str(UI_ASSET_MANIFEST_JSON).expect("ui asset manifest json");
        assert_eq!(manifest["version"], serde_json::json!(1));
        assert_eq!(manifest["status"], serde_json::json!("concept-source"));

        let rules = manifest["rules"].as_array().expect("rules array");
        assert!(
            rules.iter().any(|rule| rule
                .as_str()
                .is_some_and(|s| s.contains("Runtime key labels"))),
            "manifest must keep generated key text out of the binding source"
        );
        assert!(
            rules.iter().any(|rule| rule
                .as_str()
                .is_some_and(|s| s.contains("preserve RGBA alpha"))),
            "manifest must keep alpha proof as a runtime import rule"
        );

        let sheets = manifest["sheets"].as_object().expect("sheets object");
        for required in [
            "brand.logo",
            "hud.composition",
            "hud.bars",
            "controls.keys",
            "icons.actions",
            "icons.resources",
            "ui.frames",
            "strategy.markers",
        ] {
            let sheet = sheets.get(required).expect("required UI sheet");
            let rel = sheet["path"].as_str().expect("sheet path");
            assert!(
                rel.ends_with(".png"),
                "{required} should point at a PNG concept sheet"
            );
            let disk_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../docs/POORCRAFT-3D/assets")
                .join(rel);
            assert!(
                disk_path.is_file(),
                "{required} missing concept sheet at {}",
                disk_path.display()
            );
        }
    }

    #[test]
    fn glm_ui_rework_pack_is_parseable_and_complete() {
        let pack: serde_json::Value =
            serde_json::from_str(GLM_UI_REWORK_PACK_JSON).expect("glm ui rework manifest json");
        assert_eq!(pack["version"], serde_json::json!(1));

        let auth = pack["authorizations"]
            .as_array()
            .expect("authorizations array");
        assert!(
            auth.iter().any(|value| value
                .as_str()
                .is_some_and(|s| s.contains("MCP-style inspector"))),
            "pack must preserve the owner's inspector/MCP authorization"
        );
        assert!(
            auth.iter().any(|value| value
                .as_str()
                .is_some_and(|s| s.contains("wireframe"))),
            "pack must preserve the owner's wireframe/data export authorization"
        );

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/POORCRAFT-3D/GLM-UI-REWORK-PACK");
        for rel in pack["required_files"]
            .as_array()
            .expect("required files array")
        {
            let rel = rel.as_str().expect("required file path");
            let path = root.join(rel);
            assert!(path.is_file(), "missing GLM UI rework pack file: {rel}");
        }

        for rel in [
            "ui_acceptance_gates.json",
            "mcp_game_inspector.schema.json",
            "telemetry_contract.json",
            "screenshot_scenes.json",
            "ui_strings.en.json",
            "zcode_task_queue.json",
            "data_exports.json",
        ] {
            let json = std::fs::read_to_string(root.join(rel)).expect("pack json readable");
            let parsed: serde_json::Value =
                serde_json::from_str(&json).unwrap_or_else(|e| panic!("{rel}: {e}"));
            assert_eq!(parsed["version"], serde_json::json!(1), "{rel} version");
        }
    }

    #[test]
    fn glm_world_tools_pack_is_parseable_and_complete() {
        let pack: serde_json::Value =
            serde_json::from_str(GLM_WORLD_TOOLS_PACK_JSON).expect("glm world tools manifest json");
        assert_eq!(pack["version"], serde_json::json!(1));

        let preserved = pack["must_preserve"]
            .as_array()
            .expect("must_preserve array");
        for law in [
            "house needs door",
            "npc needs talk",
            "forge needs forging",
            "seed preview must be deterministic",
            "gpu vendor work starts with markers and data",
            "screenshots and wireframes are required evidence",
            "asset factory outputs must be gameplay-semantic",
            "GLM must inspect real game evidence before claiming success",
            "broad features must be sliced into proofable gameplay",
            "asset prompts must produce importable playable assets",
            "Z-code must continue through proofable green checkpoints",
            "vendor GPU claims require before-after capture evidence",
            "data extraction must be local safe and proof-oriented",
        ] {
            assert!(
                preserved.iter().any(|value| value
                    .as_str()
                    .is_some_and(|s| s == law)),
                "world tools pack must preserve law: {law}"
            );
        }

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK");
        for rel in pack["required_files"]
            .as_array()
            .expect("required files array")
        {
            let rel = rel.as_str().expect("required file path");
            let path = root.join(rel);
            assert!(path.is_file(), "missing GLM world tools pack file: {rel}");
        }

        for rel in [
            "world_tools_manifest.json",
            "zcode_world_tools_task_queue.json",
            "asset_generation_backlog.json",
            "asset_prompt_matrix.json",
            "interactive_object_contracts.json",
            "start_menu_worldgen_contract.json",
            "gpu_vendor_tooling_contract.json",
            "screenshot_wiremesh_capture_contract.json",
            "semantic_asset_tags.schema.json",
            "tooling_backlog.json",
            "WT-001-SEED-PREVIEW-HARNESS/seed_preview_execution_plan.json",
            "WT-001-SEED-PREVIEW-HARNESS/seed_preview_ui_wireframe.json",
            "WT-001-SEED-PREVIEW-HARNESS/seed_preview_outputs.schema.json",
            "WT-001-SEED-PREVIEW-HARNESS/seed_preview_test_matrix.json",
            "WT-002-SEMANTIC-ASSET-FACTORY-LAB/semantic_asset_factory_manifest.json",
            "WT-002-SEMANTIC-ASSET-FACTORY-LAB/asset_factory_queue.json",
            "WT-002-SEMANTIC-ASSET-FACTORY-LAB/gameplay_asset_catalog.json",
            "WT-002-SEMANTIC-ASSET-FACTORY-LAB/asset_affordance_schema.json",
            "WT-002-SEMANTIC-ASSET-FACTORY-LAB/inspection_export_contract.json",
            "WT-002-SEMANTIC-ASSET-FACTORY-LAB/gpu_capture_contract.json",
            "WT-002-SEMANTIC-ASSET-FACTORY-LAB/tool_commands_manifest.json",
            "WT-002-SEMANTIC-ASSET-FACTORY-LAB/playtest_evidence_matrix.json",
            "WT-002-SEMANTIC-ASSET-FACTORY-LAB/asset_iteration_budgets.json",
            "WT-003-GAME-OBSERVATORY-MCP-LAB/observatory_manifest.json",
            "WT-003-GAME-OBSERVATORY-MCP-LAB/mcp_tool_contract.json",
            "WT-003-GAME-OBSERVATORY-MCP-LAB/runtime_state_export.schema.json",
            "WT-003-GAME-OBSERVATORY-MCP-LAB/screenshot_scene_matrix.json",
            "WT-003-GAME-OBSERVATORY-MCP-LAB/wireframe_overlay_contract.json",
            "WT-003-GAME-OBSERVATORY-MCP-LAB/asset_dump_contract.json",
            "WT-003-GAME-OBSERVATORY-MCP-LAB/input_route_contract.json",
            "WT-003-GAME-OBSERVATORY-MCP-LAB/gpu_marker_contract.json",
            "WT-003-GAME-OBSERVATORY-MCP-LAB/regression_gate_contract.json",
            "WT-003-GAME-OBSERVATORY-MCP-LAB/evidence_bundle.schema.json",
            "WT-004-FEATURE-EXPANSION-MATRIX/feature_expansion_manifest.json",
            "WT-004-FEATURE-EXPANSION-MATRIX/feature_backlog_matrix.json",
            "WT-004-FEATURE-EXPANSION-MATRIX/worldgen_system_contract.json",
            "WT-004-FEATURE-EXPANSION-MATRIX/npc_faction_system_contract.json",
            "WT-004-FEATURE-EXPANSION-MATRIX/industry_magic_progression_contract.json",
            "WT-004-FEATURE-EXPANSION-MATRIX/ui_hud_feature_contract.json",
            "WT-004-FEATURE-EXPANSION-MATRIX/survival_combat_contract.json",
            "WT-004-FEATURE-EXPANSION-MATRIX/quest_story_contract.json",
            "WT-004-FEATURE-EXPANSION-MATRIX/multiplayer_steam_contract.json",
            "WT-004-FEATURE-EXPANSION-MATRIX/modding_tools_contract.json",
            "WT-004-FEATURE-EXPANSION-MATRIX/proof_gate_matrix.json",
            "WT-005-ASSET-PROMPT-ATLAS/asset_prompt_atlas_manifest.json",
            "WT-005-ASSET-PROMPT-ATLAS/prompt_batches.json",
            "WT-005-ASSET-PROMPT-ATLAS/asset_naming_taxonomy.json",
            "WT-005-ASSET-PROMPT-ATLAS/ui_sprite_contract.json",
            "WT-005-ASSET-PROMPT-ATLAS/gltf_import_contract.json",
            "WT-005-ASSET-PROMPT-ATLAS/material_palette_contract.json",
            "WT-005-ASSET-PROMPT-ATLAS/animation_rig_contract.json",
            "WT-005-ASSET-PROMPT-ATLAS/texture_compression_contract.json",
            "WT-005-ASSET-PROMPT-ATLAS/batch_acceptance_matrix.json",
            "WT-006-ZCODE-CONTINUOUS-RUNBOOK/zcode_runbook_manifest.json",
            "WT-006-ZCODE-CONTINUOUS-RUNBOOK/operating_loop_contract.json",
            "WT-006-ZCODE-CONTINUOUS-RUNBOOK/task_priority_contract.json",
            "WT-006-ZCODE-CONTINUOUS-RUNBOOK/proof_gate_contract.json",
            "WT-006-ZCODE-CONTINUOUS-RUNBOOK/failure_recovery_contract.json",
            "WT-006-ZCODE-CONTINUOUS-RUNBOOK/sprint_cards.json",
            "WT-006-ZCODE-CONTINUOUS-RUNBOOK/evidence_bundle_contract.json",
            "WT-006-ZCODE-CONTINUOUS-RUNBOOK/completion_audit_contract.json",
            "WT-007-GPU-VENDOR-COOKBOOK/gpu_vendor_cookbook_manifest.json",
            "WT-007-GPU-VENDOR-COOKBOOK/vendor_toolchain_contract.json",
            "WT-007-GPU-VENDOR-COOKBOOK/marker_matrix.json",
            "WT-007-GPU-VENDOR-COOKBOOK/capture_scene_matrix.json",
            "WT-007-GPU-VENDOR-COOKBOOK/upscaler_readiness_contract.json",
            "WT-007-GPU-VENDOR-COOKBOOK/before_after_evidence_contract.json",
            "WT-007-GPU-VENDOR-COOKBOOK/shader_debug_contract.json",
            "WT-008-DATA-EXTRACTION-PLUGIN-LAB/data_extraction_plugin_manifest.json",
            "WT-008-DATA-EXTRACTION-PLUGIN-LAB/plugin_manifest_contract.json",
            "WT-008-DATA-EXTRACTION-PLUGIN-LAB/exporter_surface_contract.json",
            "WT-008-DATA-EXTRACTION-PLUGIN-LAB/inspector_endpoint_contract.json",
            "WT-008-DATA-EXTRACTION-PLUGIN-LAB/telemetry_signal_contract.json",
            "WT-008-DATA-EXTRACTION-PLUGIN-LAB/mod_sample_pack_contract.json",
            "WT-008-DATA-EXTRACTION-PLUGIN-LAB/data_safety_contract.json",
            "WT-008-DATA-EXTRACTION-PLUGIN-LAB/extraction_evidence_contract.json",
        ] {
            let json = std::fs::read_to_string(root.join(rel)).expect("pack json readable");
            let parsed: serde_json::Value =
                serde_json::from_str(&json).unwrap_or_else(|e| panic!("{rel}: {e}"));
            assert_eq!(parsed["version"], serde_json::json!(1), "{rel} version");
        }

        let wt008_safety: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join(
                "WT-008-DATA-EXTRACTION-PLUGIN-LAB/data_safety_contract.json",
            ))
            .expect("WT-008 data safety readable"),
        )
        .expect("WT-008 data safety json");
        let forbidden = wt008_safety["forbidden"]
            .as_array()
            .expect("WT-008 forbidden list");
        for forbidden_claim in [
            "public_listener",
            "external_upload_by_default",
            "secret_export",
            "silent_save_mutation",
        ] {
            assert!(
                forbidden.iter().any(|value| value
                    .as_str()
                    .is_some_and(|s| s == forbidden_claim)),
                "WT-008 extraction safety must forbid {forbidden_claim}"
            );
        }

        let wt008_exporters: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join(
                "WT-008-DATA-EXTRACTION-PLUGIN-LAB/exporter_surface_contract.json",
            ))
            .expect("WT-008 exporter surface readable"),
        )
        .expect("WT-008 exporter surface json");
        let exporters = wt008_exporters["exporters"]
            .as_array()
            .expect("WT-008 exporters list");
        for exporter in ["scene", "ui", "mesh", "npc", "machine", "evidence_bundle"] {
            assert!(
                exporters
                    .iter()
                    .any(|value| value.as_str().is_some_and(|s| s == exporter)),
                "WT-008 exporter surface must include {exporter}"
            );
        }
    }

    #[test]
    fn minimal_valid_manifest_passes() {
        assert!(validate_str(MIN_GOOD).is_ok());
    }

    #[test]
    fn rejects_unparseable_json() {
        let errs = validate_str("{ not json").unwrap_err();
        assert!(errs.iter().any(|e| e.contains("parse error")));
    }

    #[test]
    fn rejects_wrong_header_fields() {
        for (field, value, expect) in [
            (
                "schema_version",
                serde_json::json!(2),
                "schema_version must be 1",
            ),
            (
                "project",
                serde_json::json!("Some Other Game"),
                "project must be",
            ),
        ] {
            let mut v: serde_json::Value = serde_json::from_str(MIN_GOOD).unwrap();
            v[field] = value;
            let errs = validate_str(&v.to_string()).unwrap_err();
            assert!(
                errs.iter().any(|e| e.contains(expect)),
                "{field} rejection: got {errs:?}"
            );
        }
        let mut v: serde_json::Value = serde_json::from_str(MIN_GOOD).unwrap();
        v["coordinate_system"]["units"] = serde_json::json!("feet");
        let errs = validate_str(&v.to_string()).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("units")));
    }

    #[test]
    fn rejects_out_of_contract_enums_and_unknown_fields() {
        // Unknown category, bad status, invalid LOD policy.
        for (field, value) in [
            ("category", serde_json::json!("spaceship")),
            ("status", serde_json::json!("shipped")),
            ("lod", serde_json::json!({"policy": "ultra"})),
            (
                "provenance",
                serde_json::json!({"kind": "ripped_from_cd", "originality_reviewed": true}),
            ),
        ] {
            let errs = validate_str(&row_overriding(MIN_GOOD, field, value)).unwrap_err();
            assert!(
                errs.iter().any(|e| e.contains("parse error")),
                "{field} enum rejection: got {errs:?}"
            );
        }
        // additionalProperties: false on asset rows.
        let mut v: serde_json::Value = serde_json::from_str(MIN_GOOD).unwrap();
        v["assets"][0]["texture_pack"] = serde_json::json!("whatever");
        let errs = validate_str(&v.to_string()).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("unknown field")));
        // Missing a required field entirely.
        let mut v: serde_json::Value = serde_json::from_str(MIN_GOOD).unwrap();
        v["assets"][0]
            .as_object_mut()
            .unwrap()
            .remove("proof_scene");
        let errs = validate_str(&v.to_string()).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.contains("missing field `proof_scene`")));
    }

    #[test]
    fn rejects_bad_ids_and_duplicates() {
        let errs = validate_str(&row_overriding(
            MIN_GOOD,
            "id",
            serde_json::json!("Block.Stone"),
        ))
        .unwrap_err();
        assert!(errs.iter().any(|e| e.contains("id must match")));

        let mut m: Manifest = serde_json::from_str(MIN_GOOD).unwrap();
        m.assets.push(m.assets[0].clone());
        let errs = validate_str(&serde_json::to_string(&m).unwrap()).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("duplicate asset id")));
    }

    #[test]
    fn rejects_missing_consumers_and_proof_scene() {
        let errs = validate_str(&row_overriding(
            MIN_GOOD,
            "runtime_consumers",
            serde_json::json!([]),
        ))
        .unwrap_err();
        assert!(errs.iter().any(|e| e.contains("runtime_consumers")));

        let errs = validate_str(&row_overriding(
            MIN_GOOD,
            "proof_scene",
            serde_json::json!(""),
        ))
        .unwrap_err();
        assert!(errs.iter().any(|e| e.contains("proof_scene")));
    }

    #[test]
    fn rejects_unreviewed_provenance_and_licensed_without_note() {
        let errs = validate_str(&row_overriding(
            MIN_GOOD,
            "provenance",
            serde_json::json!({"kind": "procedural_original", "originality_reviewed": false}),
        ))
        .unwrap_err();
        assert!(errs.iter().any(|e| e.contains("originality_reviewed")));

        let errs = validate_str(&row_overriding(
            MIN_GOOD,
            "provenance",
            serde_json::json!({"kind": "licensed", "originality_reviewed": true}),
        ))
        .unwrap_err();
        assert!(errs.iter().any(|e| e.contains("licensed assets require")));
    }

    #[test]
    fn rejects_forbidden_source_policy_anywhere_in_the_row() {
        for (field, value) in [
            (
                "geometry",
                serde_json::json!({"kind": "gltf", "source_or_generator": "minecraft_texture_pack.glb"}),
            ),
            ("material", serde_json::json!("mat.skyrim_stone")),
            (
                "runtime_consumers",
                serde_json::json!(["warcraft_renderer"]),
            ),
            ("proof_scene", serde_json::json!("all_the_mods_showcase")),
        ] {
            let errs = validate_str(&row_overriding(MIN_GOOD, field, value)).unwrap_err();
            assert!(
                errs.iter().any(|e| e.contains("forbidden source policy")),
                "{field} brand scan: got {errs:?}"
            );
        }
    }

    #[test]
    fn rejects_final_asset_without_geometry_path() {
        // Final + geometry kind none: rejected even with a non-empty string.
        let mut v: serde_json::Value = serde_json::from_str(MIN_GOOD).unwrap();
        v["assets"][0]["status"] = serde_json::json!("final");
        v["assets"][0]["geometry"]["kind"] = serde_json::json!("none");
        let errs = validate_str(&v.to_string()).unwrap_err();
        assert!(errs
            .iter()
            .any(|e| e.contains("final asset must have real geometry")));

        // Empty generator string is rejected for every status.
        let errs = validate_str(&row_overriding(
            MIN_GOOD,
            "geometry",
            serde_json::json!({"kind": "procedural_mesh", "source_or_generator": "  "}),
        ))
        .unwrap_err();
        assert!(errs.iter().any(|e| e.contains("source_or_generator")));
    }

    #[test]
    fn collects_all_errors_not_just_the_first() {
        let mut v: serde_json::Value = serde_json::from_str(MIN_GOOD).unwrap();
        v["assets"][0]["material"] = serde_json::json!("");
        v["assets"][0]["proof_scene"] = serde_json::json!("");
        let errs = validate_str(&v.to_string()).unwrap_err();
        assert!(errs.len() >= 2, "expected multiple errors, got {errs:?}");
    }

    #[test]
    fn id_pattern_is_enforced_precisely() {
        assert!(id_matches_pattern("a1"));
        assert!(id_matches_pattern("terrain.grass"));
        assert!(id_matches_pattern("npc.guard-2_x"));
        assert!(!id_matches_pattern("A")); // uppercase
        assert!(!id_matches_pattern("a")); // too short (needs 2+ chars)
        assert!(!id_matches_pattern(".block")); // leading dot
        assert!(!id_matches_pattern("block stone")); // space
        assert!(!id_matches_pattern("block!stone")); // punctuation
    }
}
