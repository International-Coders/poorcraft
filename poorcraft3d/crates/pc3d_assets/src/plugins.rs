//! WT-008: local data-extraction plugin contracts + the mod validator.
//!
//! The owner authorizes local plugins/exporters that help inspect the
//! running game — and forbids network upload, public listeners, and
//! save mutation during tests. This module embeds the WT-008 JSON
//! contracts (machine-checkable) and ships the MOD PACK VALIDATOR with
//! twelve sample packs: five GOOD (real affordances) and seven BAD
//! (each breaking one named law — a fake house with no door, a fake
//! NPC with no talk, a fake forge with no output, baked UI key text,
//! NaN asset bounds, a mod missing its material, a perf claim with no
//! before/after). BAD PACKS MUST FAIL.

pub const WT008_PLUGIN_MANIFEST_JSON: &str = include_str!(
    "../../../../docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-008-DATA-EXTRACTION-PLUGIN-LAB/plugin_manifest_contract.json"
);
pub const WT008_EXPORTER_SURFACE_JSON: &str = include_str!(
    "../../../../docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-008-DATA-EXTRACTION-PLUGIN-LAB/exporter_surface_contract.json"
);
pub const WT008_DATA_SAFETY_JSON: &str = include_str!(
    "../../../../docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-008-DATA-EXTRACTION-PLUGIN-LAB/data_safety_contract.json"
);
pub const WT008_MOD_PACK_JSON: &str = include_str!(
    "../../../../docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-008-DATA-EXTRACTION-PLUGIN-LAB/mod_sample_pack_contract.json"
);

/// One sample mod pack: a JSON payload + whether it should validate.
pub struct SamplePack {
    pub id: &'static str,
    pub should_pass: bool,
    pub json: &'static str,
}

/// The twelve sample packs (5 good, 7 bad — the contract's own list).
pub fn sample_packs() -> Vec<SamplePack> {
    let house_good = r#"{
        "id": "sample_house_good", "kind": "building",
        "anchors": [{"id": "door", "kind": "door"}, {"id": "interior", "kind": "interior"}],
        "bounds_m": [6.0, 5.4, 5.0], "materials": ["timber"]
    }"#;
    let npc_good = r#"{
        "id": "sample_npc_good", "kind": "npc",
        "anchors": [{"id": "talk", "kind": "talk"}],
        "state_variants": ["idle", "walking", "working", "sleeping"]
    }"#;
    let forge_good = r#"{
        "id": "sample_forge_good", "kind": "machine",
        "anchors": [{"id": "input", "kind": "input"}, {"id": "output", "kind": "output"}],
        "states": {"working": true, "blocked": "no fuel"}
    }"#;
    let ui_good = r#"{
        "id": "sample_ui_good", "kind": "ui",
        "widgets": [{"id": "btn_play", "label": "PLAY"}],
        "key_text_source": "keymap_table"
    }"#;
    let seed_good = r#"{
        "id": "sample_seed_good", "kind": "seed_preview",
        "deterministic": true, "spawn_safety_checked": ["water", "slope", "solid"]
    }"#;
    let house_bad = r#"{
        "id": "bad_house_no_door", "kind": "building",
        "anchors": [{"id": "interior", "kind": "interior"}],
        "bounds_m": [6.0, 5.4, 5.0], "materials": ["timber"]
    }"#;
    let npc_bad = r#"{
        "id": "bad_npc_no_talk", "kind": "npc",
        "anchors": [{"id": "home", "kind": "interior"}],
        "state_variants": ["idle"]
    }"#;
    let forge_bad = r#"{
        "id": "bad_forge_no_output", "kind": "machine",
        "anchors": [{"id": "input", "kind": "input"}],
        "states": {"working": true}
    }"#;
    let ui_bad = r#"{
        "id": "bad_ui_baked_key_text", "kind": "ui",
        "widgets": [{"id": "btn_play", "label": "PLAY (ENTER)"}],
        "key_text_source": "baked_into_labels"
    }"#;
    let nan_bad = r#"{
        "id": "bad_asset_nan_bounds", "kind": "asset",
        "anchors": [], "bounds_m": [NaN, 5.4, 5.0], "materials": ["timber"]
    }"#;
    let mat_bad = r#"{
        "id": "bad_mod_missing_material", "kind": "asset",
        "anchors": [{"id": "base", "kind": "fx"}],
        "bounds_m": [1.0, 1.0, 1.0], "materials": []
    }"#;
    let perf_bad = r#"{
        "id": "bad_perf_claim_no_before_after", "kind": "perf_claim",
        "claims_percent_faster": 42.0, "evidence": {"before": null, "after": null}
    }"#;
    vec![
        SamplePack { id: "sample_house_good", should_pass: true, json: house_good },
        SamplePack { id: "sample_npc_good", should_pass: true, json: npc_good },
        SamplePack { id: "sample_forge_good", should_pass: true, json: forge_good },
        SamplePack { id: "sample_ui_good", should_pass: true, json: ui_good },
        SamplePack { id: "sample_seed_good", should_pass: true, json: seed_good },
        SamplePack { id: "bad_house_no_door", should_pass: false, json: house_bad },
        SamplePack { id: "bad_npc_no_talk", should_pass: false, json: npc_bad },
        SamplePack { id: "bad_forge_no_output", should_pass: false, json: forge_bad },
        SamplePack { id: "bad_ui_baked_key_text", should_pass: false, json: ui_bad },
        SamplePack { id: "bad_asset_nan_bounds", should_pass: false, json: nan_bad },
        SamplePack { id: "bad_mod_missing_material", should_pass: false, json: mat_bad },
        SamplePack { id: "bad_perf_claim_no_before_after", should_pass: false, json: perf_bad },
    ]
}

/// Validates one mod pack payload. Returns Err with the NAMED broken
/// law — the catch-fake-things gate the owner asked for.
pub fn validate_mod_pack(json: &str) -> Result<(), String> {
    // NaN never survives strict JSON parsing — the pack embeds it via a
    // raw scan for the literal before parsing (serde_json rejects NaN).
    if json.contains("NaN") {
        return Err("non-finite bounds: NaN in numeric fields".into());
    }
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("pack json: {e}"))?;
    let id = v["id"].as_str().unwrap_or("<unnamed>");
    let kind = v["kind"].as_str().unwrap_or("");
    let anchors = v["anchors"].as_array().cloned().unwrap_or_default();
    let has = |k: &str| anchors.iter().any(|a| a["kind"] == k);
    match kind {
        "building" => {
            if !has("door") {
                return Err(format!("{id}: building with no door anchor — fake house"));
            }
            if !has("interior") {
                return Err(format!("{id}: building with no interior anchor"));
            }
        }
        "npc" => {
            if !has("talk") {
                return Err(format!("{id}: npc with no talk anchor — fake NPC"));
            }
            let variants = v["state_variants"].as_array().cloned().unwrap_or_default();
            if variants.len() < 2 {
                return Err(format!(
                    "{id}: npc needs at least idle+working state variants"
                ));
            }
        }
        "machine" => {
            if !has("output") {
                return Err(format!("{id}: machine with no output anchor — fake forge"));
            }
            if v["states"]["blocked"].is_null() {
                return Err(format!("{id}: machine needs a named blocked state"));
            }
        }
        "ui" => {
            if v["key_text_source"] != "keymap_table" {
                return Err(format!(
                    "{id}: UI bakes key text into labels — the keymap is the source"
                ));
            }
        }
        "seed_preview" => {
            if v["deterministic"] != serde_json::json!(true) {
                return Err(format!("{id}: seed preview must be deterministic"));
            }
        }
        "asset" => {
            let mats = v["materials"].as_array().cloned().unwrap_or_default();
            if mats.is_empty() {
                return Err(format!("{id}: asset/mod with no materials declared"));
            }
        }
        "perf_claim" => {
            let before = v["evidence"]["before"].as_f64();
            let after = v["evidence"]["after"].as_f64();
            if before.is_none() || after.is_none() {
                return Err(format!(
                    "{id}: perf claim with no before/after evidence — fake optimization"
                ));
            }
        }
        other => return Err(format!("{id}: unknown mod kind '{other}'")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wt008_contracts_parse() {
        for (name, json) in [
            ("plugin_manifest", WT008_PLUGIN_MANIFEST_JSON),
            ("exporter_surface", WT008_EXPORTER_SURFACE_JSON),
            ("data_safety", WT008_DATA_SAFETY_JSON),
            ("mod_pack", WT008_MOD_PACK_JSON),
        ] {
            let v: serde_json::Value =
                serde_json::from_str(json).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(v["version"], serde_json::json!(1), "{name} version");
        }
        let surface: serde_json::Value =
            serde_json::from_str(WT008_EXPORTER_SURFACE_JSON).unwrap();
        for exporter in [
            "scene", "ui", "asset", "mesh", "materials", "player", "npc",
            "machine", "seed_preview", "worldgen_sample", "perf",
            "evidence_bundle",
        ] {
            assert!(
                surface["exporters"].as_array().unwrap().iter().any(|e| e == exporter),
                "exporter row {exporter} missing"
            );
        }
    }

    #[test]
    fn every_good_pack_passes_and_every_bad_pack_fails_named() {
        let packs = sample_packs();
        assert_eq!(packs.len(), 12, "5 good + 7 bad");
        assert_eq!(packs.iter().filter(|p| p.should_pass).count(), 5);
        for p in packs {
            let verdict = validate_mod_pack(p.json);
            if p.should_pass {
                assert!(verdict.is_ok(), "{} should pass: {:?}", p.id, verdict);
            } else {
                let Err(reason) = verdict else {
                    panic!("{} must FAIL", p.id);
                };
                assert!(
                    reason.len() > 10,
                    "{}: the failure must be named ({reason})",
                        p.id
                );
            }
        }
    }

    #[test]
    fn the_safety_laws_are_explicit() {
        // Local-first: the safety contract forbids the listed exfiltration
        // paths; the exporter surface carries no network row.
        let safety: serde_json::Value =
            serde_json::from_str(WT008_DATA_SAFETY_JSON).unwrap();
        assert_eq!(safety["player_save_policy"], "read_only_unless_owner_requested");
        for law in [
            "public_listener",
            "external_upload_by_default",
            "secret_export",
            "unrelated_file_scan",
            "silent_save_mutation",
        ] {
            assert!(
                safety["forbidden"].as_array().unwrap().iter().any(|f| f == law),
                "forbidden law '{law}' missing"
            );
        }
    }
}
