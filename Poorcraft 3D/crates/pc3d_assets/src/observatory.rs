//! WT-003: the game observatory contracts (embedded, machine-checkable).
//!
//! The observatory is CLI-first: the same command names and JSON shapes
//! a future MCP server would expose. This module owns the CONTRACTS
//! (input routes, evidence bundles, MCP tool names); the runtime lives
//! in pc3d_render::observe and apps/poorcraft3d.

pub const WT003_INPUT_ROUTES_JSON: &str = include_str!(
    "../../../../docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-003-GAME-OBSERVATORY-MCP-LAB/input_route_contract.json"
);
pub const WT003_EVIDENCE_SCHEMA_JSON: &str = include_str!(
    "../../../../docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-003-GAME-OBSERVATORY-MCP-LAB/evidence_bundle.schema.json"
);
pub const WT003_MCP_TOOLS_JSON: &str = include_str!(
    "../../../../docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/WT-003-GAME-OBSERVATORY-MCP-LAB/mcp_tool_contract.json"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wt003_contracts_parse_and_agree_with_the_observatory() {
        let routes: serde_json::Value =
            serde_json::from_str(WT003_INPUT_ROUTES_JSON).expect("routes contract parses");
        assert_eq!(routes["version"], serde_json::json!(1));
        let required: Vec<&str> = routes["required_routes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        // The routes pc3d_render::observe implements must be a subset of
        // the contract's list (checked here against the embedded copy).
        for expect in [
            "route_title_mouse",
            "route_new_world_seed",
            "route_escape_pause",
            "route_asset_inspect",
            "route_house_entry",
            "route_npc_talk",
            "route_forge_use",
            "route_gpu_markers",
        ] {
            assert!(
                required.contains(&expect),
                "contract must require route {expect}"
            );
        }
        let schema: serde_json::Value =
            serde_json::from_str(WT003_EVIDENCE_SCHEMA_JSON).expect("evidence schema parses");
        for field in [
            "bundle_id", "build_hash", "route_id", "scene", "seed", "viewport",
            "quality", "commands", "runtime_state_path", "screenshots", "wireframes",
            "overlays", "asset_dumps", "perf_path", "gpu_marker_path", "diff_paths",
            "verdict_path", "digest",
        ] {
            assert!(
                schema["required_fields"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|f| f.as_str() == Some(field)),
                "evidence schema must require {field}"
            );
        }
        let tools: serde_json::Value =
            serde_json::from_str(WT003_MCP_TOOLS_JSON).expect("mcp tools contract parses");
        assert!(tools["version"].is_number() || tools["version"].is_string());
    }
}
