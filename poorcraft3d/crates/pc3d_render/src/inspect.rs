//! WT-002 slice 2: windowless inspection sidecars.
//!
//! For every starter-batch asset: load the REAL artifact (GLB with its
//! sockets, or the declared sim/painter authority), measure it (bounds,
//! triangles, LODs, materials), and emit the JSON sidecar exactly per
//! `inspection_export_contract.json`. The sidecar is the machine's
//! answer to "is this asset real and gameplay-semantic?" — the anchor
//! cross-check FAILS if the GLB on disk lacks an anchor the registry
//! promises (the doorway law, enforced against bytes on disk).

use pc3d_assets::semantic::{Provenance, SemanticAsset};

fn compiled_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/compiled")
}

fn glb_rel_path(glb_id: &str) -> String {
    let (dir, name) = glb_id.split_once('.').unwrap_or((glb_id, glb_id));
    format!("{dir}/{name}.glb")
}

/// The inspection sidecar for one semantic asset (windowless; scene/seed/
/// viewport describe the capture context and are filled by the screenshot
/// slice when it exists — here they honestly say windowless).
pub fn asset_sidecar(asset: &SemanticAsset, command: &str) -> serde_json::Value {
    let mut bounds = serde_json::json!([0.0, 0.0, 0.0]);
    let mut triangles: u64 = 0;
    let mut materials: Vec<String> = Vec::new();
    let mut lods: Vec<serde_json::Value> = Vec::new();
    let mut anchors: Vec<serde_json::Value> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let mut failures: Vec<String> = Vec::new();

    match &asset.provenance {
        Provenance::Glb(glb_id) => {
            let path = compiled_root().join(glb_rel_path(glb_id));
            match crate::glb::load_asset_file(&path) {
                Ok(g) => {
                    let mut ext = [f32::MAX; 3];
                    let mut min = [f32::MIN; 3];
                    for lod in &g.lods {
                        let tris = lod.indices.len() as u64 / 3;
                        lods.push(serde_json::json!({
                            "name": lod.name,
                            "triangles": tris,
                        }));
                        if lod.name == "lod0" {
                            triangles = tris;
                            for v in &lod.vertices {
                                for i in 0..3 {
                                    ext[i] = ext[i].max(v.pos[i]);
                                    min[i] = min[i].min(v.pos[i]);
                                }
                            }
                            let mut colors: Vec<[u8; 3]> = Vec::new();
                            for v in &lod.vertices {
                                let c = [
                                    (v.color[0].clamp(0.0, 1.0) * 255.0) as u8,
                                    (v.color[1].clamp(0.0, 1.0) * 255.0) as u8,
                                    (v.color[2].clamp(0.0, 1.0) * 255.0) as u8,
                                ];
                                if !colors.contains(&c) {
                                    colors.push(c);
                                }
                            }
                            materials = colors
                                .iter()
                                .map(|c| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]))
                                .collect();
                        }
                    }
                    if min[0] > ext[0] {
                        min = [0.0; 3];
                        ext = [0.0; 3];
                    } else {
                        for i in 0..3 {
                            ext[i] -= min[i];
                        }
                    }
                    bounds = serde_json::json!([ext[0], ext[1], ext[2]]);
                    // The doorway law, enforced against bytes on disk:
                    // every registry anchor must exist as a GLB socket.
                    for a in asset.anchors {
                        let socket = g
                            .sockets
                            .iter()
                            .find(|(name, _)| name == a.id)
                            .map(|(_, p)| serde_json::json!([p[0], p[1], p[2]]));
                        let Some(at) = socket else {
                            failures.push(format!(
                                "ANCHOR {} ({}) MISSING FROM GLB {}",
                                a.id, a.kind, glb_id
                            ));
                            anchors.push(serde_json::json!({
                                "id": a.id, "kind": a.kind,
                                "position_m": a.position_m,
                                "rotation_y_deg": a.rotation_y_deg,
                                "radius_m": a.radius_m,
                                "requires_clearance": a.requires_clearance,
                                "runtime_action": a.runtime_action,
                                "in_glb": false,
                            }));
                            continue;
                        };
                        anchors.push(serde_json::json!({
                            "id": a.id, "kind": a.kind,
                            "position_m": at,
                            "rotation_y_deg": a.rotation_y_deg,
                            "radius_m": a.radius_m,
                            "requires_clearance": a.requires_clearance,
                            "runtime_action": a.runtime_action,
                            "in_glb": true,
                        }));
                    }
                }
                Err(e) => failures.push(format!("GLB LOAD FAILED {}: {e}", glb_id)),
            }
        }
        Provenance::SimAuthority(a) | Provenance::Painter(a) => {
            notes.push(format!("non-GLB provenance: {a}"));
            for a_ in asset.anchors {
                anchors.push(serde_json::json!({
                    "id": a_.id, "kind": a_.kind,
                    "position_m": a_.position_m,
                    "rotation_y_deg": a_.rotation_y_deg,
                    "radius_m": a_.radius_m,
                    "requires_clearance": a_.requires_clearance,
                    "runtime_action": a_.runtime_action,
                    "in_glb": false,
                }));
            }
        }
    }

    serde_json::json!({
        "version": 1,
        "build_hash": crate::ui::build_stamp(),
        "command": command,
        "asset_id": asset.id,
        "category": asset.category,
        "provenance": format!("{:?}", asset.provenance),
        "scene": "windowless",
        "seed": "",
        "viewport": [0, 0],
        "bounds": bounds,
        "triangles": triangles,
        "materials": materials,
        "declared_materials": asset.materials,
        "lods": lods,
        "collision": asset.collision,
        "nav": asset.nav,
        "anchors": anchors,
        "gameplay": asset.gameplay.iter().map(|(k, v)| serde_json::json!({
            "key": k, "value": v,
        })).collect::<Vec<_>>(),
        "proof_paths": asset.proofs,
        "notes": notes,
        "failures": failures,
    })
}

/// All starter-batch sidecars (id, json) in registry order.
pub fn starter_batch_sidecars(command: &str) -> Vec<(String, serde_json::Value)> {
    pc3d_assets::semantic::starter_batch()
        .into_iter()
        .map(|a| (a.id.to_string(), asset_sidecar(&a, command)))
        .collect()
}

/// Whether a sidecar passed inspection. A missing anchor or a failed GLB
/// load lands in `failures` and must be refused; `notes` stays
/// informational (non-GLB provenance is legal, not a failure).
pub fn sidecar_passes(sidecar: &serde_json::Value) -> bool {
    sidecar["failures"]
        .as_array()
        .map(|n| n.is_empty())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Tests: the contract fields; the doorway law against real GLB bytes
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn find(sidecars: &[(String, serde_json::Value)], id: &str) -> serde_json::Value {
        sidecars
            .iter()
            .find(|(sid, _)| sid == id)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| panic!("sidecar {id} missing"))
    }

    #[test]
    fn every_starter_sidecar_has_the_contract_fields() {
        for (id, v) in starter_batch_sidecars("test") {
            for field in [
                "version", "build_hash", "command", "asset_id", "scene", "seed",
                "viewport", "bounds", "triangles", "materials", "lods", "collision",
                "nav", "anchors", "gameplay", "proof_paths",
            ] {
                assert!(v.get(field).is_some(), "{id}: field {field} missing");
            }
            assert!(sidecar_passes(&v), "{id}: inspection failures: {:?}", v["failures"]);
        }
    }

    #[test]
    fn house_sidecar_proves_door_nav_interior_from_the_glb_bytes() {
        // The acceptance law: the house has an enterable door and an
        // interior anchor — checked against the GLB on disk, not the
        // registry's word.
        let v = find(&starter_batch_sidecars("test"), "starter_house");
        let kinds: Vec<&str> = v["anchors"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|a| a["kind"].as_str())
            .collect();
        assert!(kinds.contains(&"door"), "door anchor missing: {kinds:?}");
        assert!(kinds.contains(&"nav_entry"), "nav anchor missing: {kinds:?}");
        assert!(kinds.contains(&"interior"), "interior anchor missing: {kinds:?}");
        assert!(
            v["anchors"].as_array().unwrap().iter().all(|a| a["in_glb"] == true),
            "every house anchor must exist in the GLB"
        );
        assert!(v["triangles"].as_u64().unwrap() > 0, "house has geometry");
    }

    #[test]
    fn chest_and_ore_node_sidecars_carry_their_law_anchors() {
        let all = starter_batch_sidecars("test");
        let chest = find(&all, "openable_chest");
        let kinds: Vec<&str> = chest["anchors"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|a| a["kind"].as_str())
            .collect();
        assert!(kinds.contains(&"open"), "chest open anchor: {kinds:?}");
        let ore = find(&all, "harvestable_ore_node");
        let kinds: Vec<&str> = ore["anchors"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|a| a["kind"].as_str())
            .collect();
        assert!(kinds.contains(&"harvest"), "ore harvest anchor: {kinds:?}");
        assert!(ore["triangles"].as_u64().unwrap() > 0);
    }

    #[test]
    fn a_promised_anchor_missing_from_the_glb_fails_inspection() {
        // Sabotage: promise an anchor the GLB does not carry — the
        // sidecar must flag it and sidecar_passes must refuse.
        let house = pc3d_assets::semantic::starter_batch()[0].clone();
        let mut anchors: Vec<pc3d_assets::semantic::Anchor> = house.anchors.to_vec();
        anchors.push(pc3d_assets::semantic::Anchor {
            id: "ghost_door",
            kind: "door",
            position_m: [9.9, 0.0, 9.9],
            rotation_y_deg: 0.0,
            radius_m: 1.0,
            requires_clearance: true,
            runtime_action: "enter",
        });
        let doctored = SemanticAsset {
            anchors: Box::leak(anchors.into_boxed_slice()),
            ..house
        };
        let v = asset_sidecar(&doctored, "test");
        assert!(!sidecar_passes(&v), "ghost anchor must fail inspection");
        assert!(
            v["failures"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n.as_str().unwrap().contains("ghost_door")),
            "the failure must name the ghost anchor"
        );
    }
}
