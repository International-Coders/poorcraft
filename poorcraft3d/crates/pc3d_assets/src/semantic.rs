//! WT-002: the semantic asset factory contracts and the starter batch.
//!
//! The owner's law: no asset is accepted unless it has gameplay metadata
//! and proof — a beautiful house with no doorway is rejected, a forge
//! with no forge action is rejected. This module owns the CONTRACTS
//! (catalog, affordance schema, factory queue — embedded at compile
//! time, re-read from disk by tests so drift fails the suite) and the
//! registry of the first semantic batch binding each required asset to
//! its real repo artifact (GLB with anchors, sim authority, or painter).
//! Validation is pure metadata; cross-crate proofs (GLB really contains
//! the anchors, the sim really has the talk state) live in pc3d_render.

use serde::Deserialize;

pub const WT002_CATALOG_JSON: &str = include_str!(
    "../../../../docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-002-SEMANTIC-ASSET-FACTORY-LAB/gameplay_asset_catalog.json"
);
pub const WT002_AFFORDANCE_SCHEMA_JSON: &str = include_str!(
    "../../../../docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-002-SEMANTIC-ASSET-FACTORY-LAB/asset_affordance_schema.json"
);
pub const WT002_FACTORY_QUEUE_JSON: &str = include_str!(
    "../../../../docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-002-SEMANTIC-ASSET-FACTORY-LAB/asset_factory_queue.json"
);

/// One category row of the gameplay catalog.
#[derive(Debug, Clone, Deserialize)]
pub struct CatalogCategory {
    pub id: String,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub required_affordances: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Catalog {
    pub version: u32,
    pub categories: Vec<CatalogCategory>,
}

/// One anchor of a semantic asset (the affordance surface).
#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    pub id: &'static str,
    /// The affordance kind this anchor provides (door, talk, harvest...).
    pub kind: &'static str,
    pub position_m: [f32; 3],
    pub rotation_y_deg: f32,
    pub radius_m: f32,
    pub requires_clearance: bool,
    pub runtime_action: &'static str,
}

/// Where the asset physically lives.
#[derive(Debug, Clone, PartialEq)]
pub enum Provenance {
    /// A compiled GLB on disk (anchors verified by pc3d_render tests).
    Glb(&'static str),
    /// A live simulation authority (npc brains, machine chain, items).
    SimAuthority(&'static str),
    /// Drawn by the UI painter (icons, swatches, markers).
    Painter(&'static str),
}

/// One registered semantic asset: metadata + affordances + proofs.
#[derive(Debug, Clone)]
pub struct SemanticAsset {
    pub id: &'static str,
    pub category: &'static str,
    pub provenance: Provenance,
    pub dimensions_m: [f32; 3],
    pub materials: &'static [&'static str],
    pub lods: &'static [&'static str],
    pub collision: &'static str,
    pub nav: &'static str,
    pub anchors: &'static [Anchor],
    /// (key, value) gameplay facts the runtime can act on.
    pub gameplay: &'static [(&'static str, &'static str)],
    /// Named proofs that must exist for this asset to be accepted.
    pub proofs: &'static [&'static str],
}

impl SemanticAsset {
    /// The WT-002 rejection law: every affordance the asset's category
    /// requires (catalog list + schema category law) must be provided —
    /// by an anchor of that kind, a gameplay key of that name, or the
    /// collision/nav fields for those two. A category no contract covers
    /// is never silently accepted: it must still carry the minimal
    /// gameplay-metadata surface (anchor + gameplay + proof).
    pub fn missing_affordances(
        &self,
        catalog: &Catalog,
        schema: &AffordanceSchema,
    ) -> Vec<String> {
        let mut required: Vec<String> = Vec::new();
        if let Some(cat) = catalog.categories.iter().find(|c| c.id == self.category) {
            required.extend(cat.required_affordances.iter().cloned());
        }
        if let Some(laws) = schema.category_laws.get(self.category) {
            required.extend(laws.iter().cloned());
        }
        if required.is_empty() {
            if self.anchors.is_empty() || self.gameplay.is_empty() || self.proofs.is_empty() {
                return vec!["<no-category-law-and-no-gameplay-metadata>".into()];
            }
            return vec![];
        }
        required.sort();
        required.dedup();
        required
            .into_iter()
            .filter(|req| {
                let r = req.as_str();
                !(self.anchors.iter().any(|a| a.kind == r)
                    || self.gameplay.iter().any(|(k, _)| *k == r)
                    || (r == "collision" && self.collision != "none")
                    || (r == "nav_entry" && self.nav != "none"))
            })
            .collect()
    }
}

/// The affordance schema's category laws (extra per-category anchor
/// requirements beyond the catalog).
#[derive(Debug, Clone, Deserialize)]
pub struct AffordanceSchema {
    pub version: u32,
    #[serde(default)]
    pub category_laws: std::collections::BTreeMap<String, Vec<String>>,
}

/// The starter_semantic_batch registry: every required asset bound to its
/// real artifact. `asset_factory_queue.json` names these ids; the
/// acceptance tests refuse a batch row that is not registered.
pub fn starter_batch() -> Vec<SemanticAsset> {
    use Anchor as A;
    vec![
        SemanticAsset {
            id: "starter_house",
            category: "building",
            provenance: Provenance::Glb("module.house_croft"),
            dimensions_m: [6.0, 5.4, 5.0],
            materials: &["fieldstone", "timber", "thatch"],
            lods: &["lod0"],
            collision: "solid_box",
            nav: "door_front",
            anchors: &[
                A { id: "door_front", kind: "door", position_m: [0.45, 0.0, -2.5], rotation_y_deg: 0.0, radius_m: 1.0, requires_clearance: true, runtime_action: "enter" },
                A { id: "road_front", kind: "nav_entry", position_m: [0.45, 0.0, -5.0], rotation_y_deg: 0.0, radius_m: 1.5, requires_clearance: true, runtime_action: "path_to_door" },
                A { id: "interior", kind: "interior", position_m: [0.45, 0.0, 0.4], rotation_y_deg: 0.0, radius_m: 1.5, requires_clearance: false, runtime_action: "indoor_spawn" },
                A { id: "roof_smoke", kind: "fx", position_m: [1.7, 4.9, 0.55], rotation_y_deg: 0.0, radius_m: 0.4, requires_clearance: false, runtime_action: "chimney_smoke" },
            ],
            gameplay: &[
                ("enter", "door_front"),
                ("interior_spawn", "interior"),
            ],
            proofs: &["play_assets_windowed", "asset_sidecar"],
        },
        SemanticAsset {
            id: "talkable_villager",
            category: "npc",
            provenance: Provenance::SimAuthority("pc3d_world::npcs (brain: identity + schedule + intent; crowd rig renders it)"),
            dimensions_m: [0.6, 1.75, 0.6],
            materials: &["tunic", "skin", "role_gear"],
            lods: &["rig", "impostor_box_64m"],
            collision: "capsule_chest_high",
            nav: "walks_paths",
            anchors: &[
                A { id: "talk", kind: "talk", position_m: [0.0, 1.4, 0.0], rotation_y_deg: 0.0, radius_m: 2.0, requires_clearance: false, runtime_action: "open_dialog" },
            ],
            gameplay: &[
                ("identity", "npc brain name + role"),
                ("schedule", "npc daily schedule"),
                ("state", "Idle/Walking/Working/Sleeping intent"),
            ],
            proofs: &["play_people_windowed", "npc_cast_gate", "asset_sidecar"],
        },
        SemanticAsset {
            id: "working_forge",
            category: "machine",
            provenance: Provenance::SimAuthority("pc3d_world machines (boiler->engine->generator->battery chain proven by --journey)"),
            dimensions_m: [2.0, 1.6, 1.2],
            materials: &["stone", "iron", "ember"],
            lods: &["lod0"],
            collision: "solid_box",
            nav: "workspot",
            anchors: &[
                A { id: "forge_input", kind: "input", position_m: [-0.9, 0.9, -0.5], rotation_y_deg: 0.0, radius_m: 0.8, requires_clearance: true, runtime_action: "place_fuel" },
                A { id: "forge_output", kind: "output", position_m: [0.9, 0.9, -0.5], rotation_y_deg: 0.0, radius_m: 0.8, requires_clearance: true, runtime_action: "take_result" },
                A { id: "workspot", kind: "workspot", position_m: [0.0, 0.0, -1.4], rotation_y_deg: 0.0, radius_m: 1.0, requires_clearance: true, runtime_action: "forge_work" },
            ],
            gameplay: &[
                ("heat", "boiler thermal state"),
                ("work_state", "machine running/stalled"),
                ("blocked_state", "no fuel / contaminated"),
            ],
            proofs: &["journey_battery_chain", "asset_sidecar"],
        },
        SemanticAsset {
            id: "openable_chest",
            category: "chest",
            provenance: Provenance::Glb("prop.chest"),
            dimensions_m: [1.05, 0.9, 0.63],
            materials: &["timber", "iron_band"],
            lods: &["lod0", "lod1"],
            collision: "solid_box",
            nav: "approach_front",
            anchors: &[
                A { id: "open", kind: "open", position_m: [0.0, 0.4, -0.75], rotation_y_deg: 0.0, radius_m: 1.2, requires_clearance: true, runtime_action: "open_chest" },
                A { id: "lid", kind: "fx", position_m: [0.0, 0.62, 0.28], rotation_y_deg: 0.0, radius_m: 0.5, requires_clearance: false, runtime_action: "lid_hinge" },
            ],
            gameplay: &[
                ("inventory", "items::Inventory"),
            ],
            proofs: &["asset_sidecar", "budget_checked_regeneration"],
        },
        SemanticAsset {
            id: "harvestable_ore_node",
            category: "resource",
            provenance: Provenance::Glb("prop.ore_node"),
            dimensions_m: [1.6, 1.1, 1.3],
            materials: &["granite", "ember_quartz"],
            lods: &["lod0", "lod1"],
            collision: "solid_blob",
            nav: "approach_front",
            anchors: &[
                A { id: "harvest", kind: "harvest", position_m: [0.0, 0.5, -0.85], rotation_y_deg: 0.0, radius_m: 1.2, requires_clearance: true, runtime_action: "harvest_ore" },
            ],
            gameplay: &[
                ("yield", "items::harvest_yields"),
                ("depleted_state", "harvest clears the node until regrow"),
                ("rarity", "ore rarity keyed by depth/biome"),
            ],
            proofs: &["asset_sidecar", "budget_checked_regeneration"],
        },
        SemanticAsset {
            id: "map_marker_set",
            category: "marker",
            provenance: Provenance::Glb("module.banner_sign"),
            dimensions_m: [0.8, 2.4, 0.8],
            materials: &["timber", "banner_cloth"],
            lods: &["lod0"],
            collision: "solid_post",
            nav: "none",
            anchors: &[
                A { id: "inspect", kind: "inspect", position_m: [0.0, 1.2, -0.6], rotation_y_deg: 0.0, radius_m: 1.5, requires_clearance: false, runtime_action: "read_marker" },
            ],
            gameplay: &[
                ("spawn_marker", "seed preview map"),
                ("world_marker", "banner_sign GLB"),
            ],
            proofs: &["play_settlement_windowed", "asset_sidecar"],
        },
        SemanticAsset {
            id: "hud_icon_set",
            category: "ui",
            provenance: Provenance::Painter("pc3d_render::ui painter (hotbar material swatches, bar icons, toasts)"),
            dimensions_m: [0.0, 0.0, 0.0],
            materials: &["flat_ui_palette"],
            lods: &["vector"],
            collision: "none",
            nav: "none",
            anchors: &[],
            gameplay: &[
                ("semantic_id", "hud_icon_set: fixed icon ids"),
                ("alpha_policy", "alpha-blended quad over the world"),
                ("contrast", "ember-on-dark forged theme"),
                ("state_variants", "hotbar normal/selected/hover + bar fills"),
            ],
            proofs: &["ui_shots_13", "quad_coverage_law"],
        },
    ]
}

// ---------------------------------------------------------------------------
// Tests: contracts parse; the rejection law bites; the batch is complete
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Catalog {
        serde_json::from_str(WT002_CATALOG_JSON).expect("gameplay catalog parses")
    }

    fn schema() -> AffordanceSchema {
        serde_json::from_str(WT002_AFFORDANCE_SCHEMA_JSON).expect("affordance schema parses")
    }

    #[test]
    fn wt002_contracts_parse_from_the_pack() {
        let cat: Catalog = catalog();
        assert_eq!(cat.version, 1);
        assert!(
            cat.categories.iter().any(|c| c.id == "building"),
            "building category required"
        );
        let schema: serde_json::Value =
            serde_json::from_str(WT002_AFFORDANCE_SCHEMA_JSON).expect("affordance schema parses");
        assert_eq!(schema["version"], serde_json::json!(1));
        assert!(
            schema["required_anchor_fields"]
                .as_array()
                .unwrap()
                .iter()
                .all(|f| f.is_string()),
            "anchor fields are strings"
        );
        let queue: serde_json::Value =
            serde_json::from_str(WT002_FACTORY_QUEUE_JSON).expect("factory queue parses");
        assert_eq!(queue["version"], serde_json::json!(1));
        assert!(
            queue["batches"][0]["id"] == "starter_semantic_batch",
            "starter batch declared"
        );
    }

    #[test]
    fn starter_batch_is_complete_per_the_factory_queue() {
        let queue: serde_json::Value = serde_json::from_str(WT002_FACTORY_QUEUE_JSON).unwrap();
        let required = queue["batches"][0]["required_assets"].as_array().unwrap();
        let batch = starter_batch();
        for id in required {
            let id = id.as_str().unwrap();
            assert!(
                batch.iter().any(|a| a.id == id),
                "starter batch asset {id} not registered"
            );
        }
        // Every registered id is unique.
        let n = batch.len();
        let mut ids: Vec<_> = batch.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "duplicate semantic ids");
    }

    #[test]
    fn every_starter_asset_satisfies_its_category_laws() {
        // THE WT-002 LAW: no asset without gameplay metadata is accepted.
        let cat = catalog();
        let sch = schema();
        for a in starter_batch() {
            let missing = a.missing_affordances(&cat, &sch);
            assert!(
                missing.is_empty(),
                "{} rejected — missing affordances: {:?}",
                a.id,
                missing
            );
        }
    }

    #[test]
    fn the_rejection_law_bites() {
        // A beautiful house with NO doorway is rejected.
        let cat = catalog();
        let sch = schema();
        let mut house = starter_batch()[0].clone();
        house.anchors = &[]; // strip the affordance surface
        let missing = house.missing_affordances(&cat, &sch);
        assert!(
            missing.iter().any(|m| m.contains("door")),
            "doorless house must be rejected, got {missing:?}"
        );

        // An NPC mesh with no talk state is rejected.
        let mut npc = starter_batch()[1].clone();
        npc.anchors = &[];
        npc.gameplay = &[];
        assert!(
            npc.missing_affordances(&cat, &sch)
                .iter()
                .any(|m| m.contains("talk")),
            "talkless NPC must be rejected"
        );

        // A forge with no forge action is rejected.
        let mut forge = starter_batch()[2].clone();
        forge.anchors = &[];
        let missing = forge.missing_affordances(&cat, &sch);
        assert!(
            missing.iter().any(|m| m.contains("input")),
            "forge without input anchor must be rejected, got {missing:?}"
        );
    }

    #[test]
    fn anchors_carry_the_full_contract_fields() {
        for a in starter_batch() {
            for an in a.anchors {
                assert!(an.radius_m > 0.0, "{} anchor {} radius", a.id, an.id);
                assert!(
                    !an.runtime_action.is_empty(),
                    "{} anchor {} runtime action",
                    a.id,
                    an.id
                );
            }
        }
    }
}
