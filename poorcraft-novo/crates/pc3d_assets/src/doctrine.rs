//! Machine-enforced continuation doctrine for POORCRAFT 3D.
//!
//! The Markdown pack is written for humans and coding agents. This module
//! keeps its minimum proof surface executable: weakening the 30-phase ladder,
//! asset/viewmodel/worldgen gates, or platform matrix fails the workspace.

use serde::Deserialize;
use std::collections::BTreeSet;

pub const CONTINUATION_DOCTRINE_JSON: &str =
    include_str!("../../../docs/CONTINUACAO-GLM/doctrine.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Doctrine {
    pub version: u32,
    pub project: String,
    pub status: String,
    pub required_documents: Vec<String>,
    pub non_negotiable_laws: Vec<String>,
    pub phase_ladder: Vec<Phase>,
    pub asset_gates: Vec<String>,
    pub viewmodel_gates: Vec<String>,
    pub worldgen_gates: Vec<String>,
    pub platforms: Vec<Platform>,
    pub tool_candidates: Vec<ToolCandidate>,
    pub release_gate: ReleaseGate,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Phase {
    pub id: u8,
    pub name: String,
    pub kind: String,
    pub required_evidence: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Platform {
    pub id: String,
    pub artifact: String,
    pub steam_depot: String,
    pub smoke: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCandidate {
    pub name: String,
    pub url: String,
    pub status: String,
    pub adoption_gate: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseGate {
    pub min_phase_count: usize,
    pub min_asset_gates: usize,
    pub min_viewmodel_gates: usize,
    pub min_worldgen_gates: usize,
    pub required_platform_count: usize,
    pub required_next_job: String,
}

fn unique_nonempty(values: &[String], label: &str, errors: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() {
            errors.push(format!("{label} contains an empty value"));
        } else if !seen.insert(value) {
            errors.push(format!("{label} contains duplicate {value:?}"));
        }
    }
}

/// Parses and enforces the minimum owner contract. All problems are returned
/// together so an agent can repair a bad edit in one pass.
pub fn validate(json: &str) -> Result<Doctrine, Vec<String>> {
    let doctrine: Doctrine = match serde_json::from_str(json) {
        Ok(value) => value,
        Err(error) => return Err(vec![format!("parse error: {error}")]),
    };
    let mut errors = Vec::new();

    if doctrine.version != 1 {
        errors.push(format!("version must be 1, got {}", doctrine.version));
    }
    if doctrine.project != "POORCRAFT 3D" {
        errors.push(format!(
            "project must be POORCRAFT 3D, got {:?}",
            doctrine.project
        ));
    }
    if doctrine.status != "mandatory" {
        errors.push("status must remain mandatory".into());
    }

    unique_nonempty(
        &doctrine.required_documents,
        "required_documents",
        &mut errors,
    );
    unique_nonempty(
        &doctrine.non_negotiable_laws,
        "non_negotiable_laws",
        &mut errors,
    );
    unique_nonempty(&doctrine.asset_gates, "asset_gates", &mut errors);
    unique_nonempty(&doctrine.viewmodel_gates, "viewmodel_gates", &mut errors);
    unique_nonempty(&doctrine.worldgen_gates, "worldgen_gates", &mut errors);

    for law in [
        "fail_before_fix",
        "asset_requires_runtime_consumer",
        "viewmodel_is_not_world_model",
        "spawn_is_validated_search",
        "water_requires_supported_bed",
        "visual_proof_requires_semantic_state",
        "human_inspection_is_required",
        "three_platforms_are_planned_now",
        "external_tools_require_license_and_security_review",
    ] {
        if !doctrine
            .non_negotiable_laws
            .iter()
            .any(|value| value == law)
        {
            errors.push(format!("missing non-negotiable law {law}"));
        }
    }

    if doctrine.phase_ladder.len() != doctrine.release_gate.min_phase_count
        || doctrine.release_gate.min_phase_count != 30
    {
        errors.push(format!(
            "phase ladder must contain exactly 30 phases, found {} (gate says {})",
            doctrine.phase_ladder.len(),
            doctrine.release_gate.min_phase_count
        ));
    }
    let mut phase_names = BTreeSet::new();
    for (index, phase) in doctrine.phase_ladder.iter().enumerate() {
        let expected = index as u8 + 1;
        if phase.id != expected {
            errors.push(format!(
                "phase at index {index} must have id {expected}, got {}",
                phase.id
            ));
        }
        if phase.name.trim().is_empty() || !phase_names.insert(phase.name.as_str()) {
            errors.push(format!("phase {} has an empty or duplicate name", phase.id));
        }
        if phase.kind.trim().is_empty() {
            errors.push(format!("phase {} has no kind", phase.id));
        }
        if phase.required_evidence.is_empty()
            || phase
                .required_evidence
                .iter()
                .any(|value| value.trim().is_empty())
        {
            errors.push(format!("phase {} requires named evidence", phase.id));
        }
    }
    for required_kind in [
        "unit",
        "property",
        "fuzz",
        "mutation",
        "render",
        "journey",
        "performance",
        "package",
        "human",
    ] {
        if !doctrine
            .phase_ladder
            .iter()
            .any(|phase| phase.kind == required_kind)
        {
            errors.push(format!("phase ladder must include kind {required_kind}"));
        }
    }

    for (actual, minimum, label) in [
        (
            doctrine.asset_gates.len(),
            doctrine.release_gate.min_asset_gates,
            "asset",
        ),
        (
            doctrine.viewmodel_gates.len(),
            doctrine.release_gate.min_viewmodel_gates,
            "viewmodel",
        ),
        (
            doctrine.worldgen_gates.len(),
            doctrine.release_gate.min_worldgen_gates,
            "worldgen",
        ),
    ] {
        if actual < minimum {
            errors.push(format!(
                "{label} gates weakened: need {minimum}, found {actual}"
            ));
        }
    }

    if doctrine.platforms.len() != doctrine.release_gate.required_platform_count
        || doctrine.release_gate.required_platform_count != 3
    {
        errors.push(format!(
            "platform matrix must contain exactly 3 rows, found {}",
            doctrine.platforms.len()
        ));
    }
    for required in ["windows", "linux", "macos"] {
        if !doctrine.platforms.iter().any(|platform| {
            platform.id.contains(required)
                && !platform.artifact.trim().is_empty()
                && !platform.steam_depot.trim().is_empty()
                && !platform.smoke.trim().is_empty()
        }) {
            errors.push(format!("platform matrix lacks a complete {required} row"));
        }
    }

    if doctrine.release_gate.required_next_job != "N01" {
        errors.push("the first unclosed job must remain N01 until bookkeeping promotes it".into());
    }
    if doctrine.tool_candidates.len() < 5 {
        errors.push("tool research must retain at least five evaluated candidates".into());
    }
    for tool in &doctrine.tool_candidates {
        if tool.name.trim().is_empty()
            || !tool.url.starts_with("https://")
            || !matches!(
                tool.status.as_str(),
                "evaluate" | "approved" | "hold" | "rejected"
            )
            || tool.adoption_gate.trim().is_empty()
        {
            errors.push(format!("tool candidate {:?} is incomplete", tool.name));
        }
    }

    if errors.is_empty() {
        Ok(doctrine)
    } else {
        Err(errors)
    }
}

pub fn checked_in() -> Result<Doctrine, Vec<String>> {
    validate(CONTINUATION_DOCTRINE_JSON)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn continuation_doctrine_is_complete_and_files_exist() {
        let doctrine = checked_in().expect("checked-in continuation doctrine must validate");
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/CONTINUACAO-GLM");
        for document in &doctrine.required_documents {
            assert!(
                root.join(document).is_file(),
                "missing doctrine document {document}"
            );
        }
        assert_eq!(doctrine.phase_ladder.len(), 30);
        assert_eq!(doctrine.asset_gates.len(), 16);
        assert_eq!(doctrine.viewmodel_gates.len(), 12);
        assert_eq!(doctrine.worldgen_gates.len(), 20);
        assert_eq!(doctrine.platforms.len(), 3);
    }

    #[test]
    fn weakening_the_ladder_or_owner_laws_fails_named() {
        let mut value: serde_json::Value =
            serde_json::from_str(CONTINUATION_DOCTRINE_JSON).expect("doctrine JSON");
        value["phase_ladder"].as_array_mut().unwrap().pop();
        value["non_negotiable_laws"] = serde_json::json!(["no_docs_only_completion"]);
        let errors = validate(&value.to_string()).expect_err("weakened doctrine must fail");
        let joined = errors.join("\n");
        assert!(joined.contains("exactly 30 phases"));
        assert!(joined.contains("fail_before_fix"));
        assert!(joined.contains("viewmodel_is_not_world_model"));
    }
}
