//! B01 runtime truth dashboard (docs/BETA-FOUNDATION/08-BETA-DELIVERY-ROADMAP.md).
//!
//! A machine-readable statement of what the engine actually is right now:
//! active systems, schema versions, simulation ownership, scene/test counts,
//! and measured performance. It adds no features. Its contract is enforced by
//! tests in this module: a system may only be labeled `ServerAuthoritative`
//! when the *live* dedicated-server source contains the handling evidence, and
//! the systems the audit names client-simulated are pinned `ClientLocal` — so
//! the dashboard cannot overclaim authority without a real implementation and
//! an explicit, reviewable change to the audit list.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Live dedicated-server source. Server-authority claims are checked against
/// this text at compile time, so the dashboard tracks the code, not memory.
pub const SERVER_SOURCE: &str = include_str!("../../crates/lf_server/src/lib.rs");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimOwner {
    /// The dedicated server owns canonical state and broadcasts results.
    ServerAuthoritative,
    /// The server relays peer-authored data without owning or validating it.
    RelayOnly,
    /// Simulated only inside the client; no canonical server state exists.
    ClientLocal,
    /// Produced by the shared deterministic generator crate on both sides.
    DeterministicGenerator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemRow {
    pub system: &'static str,
    pub owner: SimOwner,
    /// Workspace-relative paths that must exist: evidence the system is real.
    pub evidence: &'static [&'static str],
    /// For `ServerAuthoritative` rows, strings that must appear in the live
    /// `lf_server` source today. If the handling is removed, the dashboard
    /// fails its contract test instead of quietly overclaiming.
    pub server_markers: &'static [&'static str],
    pub note: &'static str,
}

/// The audit-grounded inventory (docs/BETA-FOUNDATION/02-CURRENT-ENGINE-AUDIT.md,
/// loop 356). Server authority today: session/peers, block edits (+history).
/// Everything else is relay or client-local — that honesty is the point.
pub const SYSTEMS: &[SystemRow] = &[
    SystemRow {
        system: "world_generation",
        owner: SimOwner::DeterministicGenerator,
        evidence: &["crates/lf_worldgen/src/lib.rs", "crates/lf_worldgen/src/biome.rs"],
        server_markers: &[],
        note: "same deterministic code on client and server; GENERATOR_VERSION gates saves",
    },
    SystemRow {
        system: "world_identity",
        owner: SimOwner::DeterministicGenerator,
        evidence: &["crates/lf_worldgen/src/identity.rs"],
        server_markers: &[],
        note: "identity.dat stamped before generation; multiplayer clients adopt the server seed via Welcome",
    },
    SystemRow {
        system: "session_and_peers",
        owner: SimOwner::ServerAuthoritative,
        evidence: &["crates/lf_server/src/lib.rs"],
        server_markers: &["ClientMessage::Hello", "ServerMessage::Welcome"],
        note: "server assigns peer ids and owns the join/leave list",
    },
    SystemRow {
        system: "block_edits",
        owner: SimOwner::ServerAuthoritative,
        evidence: &["crates/lf_server/src/lib.rs"],
        server_markers: &["ClientMessage::SetBlock", "ServerMessage::BlockUpdate"],
        note: "the only world-mutating authority: SetBlock accepted, applied, re-broadcast, kept in edit history",
    },
    SystemRow {
        system: "player_presence",
        owner: SimOwner::RelayOnly,
        evidence: &["crates/lf_server/src/lib.rs"],
        server_markers: &["ClientMessage::Position", "ServerMessage::PlayerStates"],
        note: "positions are client-authored and relayed; no server-side physics",
    },
    SystemRow {
        system: "chat",
        owner: SimOwner::RelayOnly,
        evidence: &["crates/lf_server/src/lib.rs"],
        server_markers: &["ClientMessage::Chat"],
        note: "text relay",
    },
    SystemRow {
        system: "peer_trade",
        owner: SimOwner::RelayOnly,
        evidence: &["crates/lf_server/src/lib.rs"],
        server_markers: &["ClientMessage::TradeOffer"],
        note: "offers are relayed but item removal/grant is still finalized client-side; escrow is not server-owned",
    },
    SystemRow {
        system: "transport_udp",
        owner: SimOwner::RelayOnly,
        evidence: &["crates/lf_client/src/net.rs", "crates/lf_server/src/lib.rs"],
        server_markers: &[],
        note: "unreliable datagrams; no application-level sequencing/ack/resync yet (B24)",
    },
    SystemRow {
        system: "transport_steam",
        owner: SimOwner::ClientLocal,
        evidence: &["crates/lf_steam/src/lib.rs", "crates/lf_steam/src/net_steam.rs"],
        server_markers: &[],
        note: "lobby + ISteamNetworkingSockets adapters and probes exist; not wired into the default game path (B25)",
    },
    SystemRow {
        system: "survival_stats",
        owner: SimOwner::ClientLocal,
        evidence: &["crates/lf_game/src/survival.rs", "crates/lf_game/src/player.rs"],
        server_markers: &[],
        note: "health/hunger/air simulated in the client loop",
    },
    SystemRow {
        system: "inventory_and_crafting",
        owner: SimOwner::ClientLocal,
        evidence: &["crates/lf_game/src/items.rs", "crates/lf_game/src/crafting.rs"],
        server_markers: &[],
        note: "transactions run in the client; peer-trade grants are client-finalized too",
    },
    SystemRow {
        system: "fluids",
        owner: SimOwner::ClientLocal,
        evidence: &["crates/lf_game/src/fluids.rs"],
        server_markers: &[],
        note: "client processes 64 queued cells/tick; source-distance levels only, no conserved volume (B04-B06)",
    },
    SystemRow {
        system: "machines_and_power",
        owner: SimOwner::ClientLocal,
        evidence: &["crates/lf_game/src/machines.rs"],
        server_markers: &[],
        note: "water wheel takes a Boolean has_water and emits a fixed 12 EU/s (B07 replaces with torque)",
    },
    SystemRow {
        system: "mobs_and_combat",
        owner: SimOwner::ClientLocal,
        evidence: &["crates/lf_game/src/mobs.rs", "crates/lf_game/src/combat.rs"],
        server_markers: &[],
        note: "mob AI, damage, drops simulated in the client",
    },
    SystemRow {
        system: "npc_life",
        owner: SimOwner::ClientLocal,
        evidence: &["crates/lf_npc/src/lib.rs", "crates/lf_npc/src/locomotion.rs"],
        server_markers: &[],
        note: "schedules and direct steering run in the client; no castle nav graph yet (B13)",
    },
    SystemRow {
        system: "settlement_residents",
        owner: SimOwner::ClientLocal,
        evidence: &["crates/lf_npc/src/vassals.rs", "crates/lf_game/src/mobs.rs"],
        server_markers: &[],
        note: "kingdom court residents share one home anchor; spawner caps the active list at 12",
    },
    SystemRow {
        system: "companions",
        owner: SimOwner::ClientLocal,
        evidence: &["crates/lf_game/src/companions.rs"],
        server_markers: &[],
        note: "trust/morale/commands exist; follow/work/co-op sync incomplete (B17)",
    },
    SystemRow {
        system: "quests_and_story",
        owner: SimOwner::ClientLocal,
        evidence: &["crates/lf_story/src/lib.rs"],
        server_markers: &[],
        note: "quest state advances in the client",
    },
    SystemRow {
        system: "reputation_and_factions",
        owner: SimOwner::ClientLocal,
        evidence: &["crates/lf_client/src/factions.rs"],
        server_markers: &[],
        note: "standings and reasons live in the client; no server canonical knowledge (B15/B16)",
    },
    SystemRow {
        system: "research_eras",
        owner: SimOwner::ClientLocal,
        evidence: &["crates/lf_game/src/research.rs"],
        server_markers: &[],
        note: "era progression simulated in the client",
    },
    SystemRow {
        system: "world_saves",
        owner: SimOwner::ClientLocal,
        evidence: &["crates/lf_client/src/lib.rs"],
        server_markers: &[],
        note: "ClientSave in the world dir; the server keeps generated world + edit history separately",
    },
];

/// Systems the loop-356 audit explicitly names as NOT server-owned. Pinning
/// them here means relabeling one authoritative requires deleting a line from
/// this list — a reviewable contract change, never a silent one.
pub const KNOWN_CLIENT_ONLY: &[&str] = &[
    "survival_stats",
    "inventory_and_crafting",
    "fluids",
    "machines_and_power",
    "mobs_and_combat",
    "npc_life",
    "settlement_residents",
    "companions",
    "quests_and_story",
    "reputation_and_factions",
    "research_eras",
    "world_saves",
    "transport_steam",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfLine {
    pub scene: String,
    pub frames: usize,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub min_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruthReport {
    pub schema: u32,
    pub generated_by: String,
    pub versions: BTreeMap<String, u32>,
    pub counts: BTreeMap<String, usize>,
    pub ownership_summary: BTreeMap<String, usize>,
    pub ownership: Vec<OwnedRow>,
    pub server_authoritative_systems: Vec<String>,
    pub seedlab: Option<serde_json::Value>,
    pub perf: Option<PerfLine>,
    pub honesty: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnedRow {
    pub system: String,
    pub owner: String,
    pub note: String,
}

fn count_rust_stats(root: &Path) -> (usize, usize) {
    let mut files = 0usize;
    let mut tests = 0usize;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if matches!(name, "target" | ".git" | "dist" | "worlds" | "shots" | "node_modules") {
                    continue;
                }
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                files += 1;
                if let Ok(text) = std::fs::read_to_string(&path) {
                    tests += text.matches("#[test]").count();
                }
            }
        }
    }
    (files, tests)
}

/// Build the dashboard. `root` is the workspace root; `perf` is an optional
/// live measurement supplied by the caller (`xtask truth --bench ...`).
pub fn build_report(root: &Path, perf: Option<PerfLine>) -> Result<TruthReport, String> {
    for name in KNOWN_CLIENT_ONLY {
        if !SYSTEMS.iter().any(|r| r.system == *name) {
            return Err(format!(
                "KNOWN_CLIENT_ONLY names an untracked system: {name}"
            ));
        }
    }
    for row in SYSTEMS {
        for path in row.evidence {
            if !root.join(path).exists() {
                return Err(format!(
                    "evidence missing for {}: {}",
                    row.system, path
                ));
            }
        }
        if row.owner == SimOwner::ServerAuthoritative {
            let cited_server = row
                .evidence
                .iter()
                .any(|p| p.starts_with("crates/lf_server") || p.starts_with("apps/loreforge-server"));
            if !cited_server {
                return Err(format!(
                    "{} claims server authority without citing the server crate",
                    row.system
                ));
            }
            for marker in row.server_markers {
                if !SERVER_SOURCE.contains(marker) {
                    return Err(format!(
                        "{} claims server authority but live lf_server source no longer contains {:?}",
                        row.system, marker
                    ));
                }
            }
        }
    }

    let (rs_files, test_attrs) = count_rust_stats(root);

    let mut versions = BTreeMap::new();
    versions.insert("protocol_version".into(), lf_protocol::PROTOCOL_VERSION);
    versions.insert("generator_version".into(), lf_worldgen::GENERATOR_VERSION);
    versions.insert("truth_schema".into(), 1);

    let mut counts = BTreeMap::new();
    counts.insert("vistest_scenes".into(), lf_vistest::scenes().len());
    counts.insert("systems_tracked".into(), SYSTEMS.len());
    counts.insert("rust_source_files".into(), rs_files);
    counts.insert("test_attributes".into(), test_attrs);

    let mut ownership_summary: BTreeMap<String, usize> = BTreeMap::new();
    let mut ownership = Vec::new();
    for row in SYSTEMS {
        *ownership_summary
            .entry(format!("{:?}", row.owner))
            .or_insert(0) += 1;
        ownership.push(OwnedRow {
            system: row.system.into(),
            owner: format!("{:?}", row.owner),
            note: row.note.into(),
        });
    }

    let server_authoritative_systems: Vec<String> = SYSTEMS
        .iter()
        .filter(|r| r.owner == SimOwner::ServerAuthoritative)
        .map(|r| r.system.to_string())
        .collect();

    let seedlab = std::fs::read(root.join("target/seedlab_report.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());

    let mut honesty = BTreeMap::new();
    honesty.insert(
        "label".into(),
        "ALPHA — beta gates live in docs/BETA-FOUNDATION/08-BETA-DELIVERY-ROADMAP.md".into(),
    );
    honesty.insert(
        "server_authority_basis".into(),
        "session/peers + block edits only; every other row is relay or client-local by audit".into(),
    );
    honesty.insert(
        "next_job".into(),
        "B02 fixed tick, command IDs, and domain events".into(),
    );

    Ok(TruthReport {
        schema: 1,
        generated_by: "xtask truth (B01 runtime truth dashboard)".into(),
        versions,
        counts,
        ownership_summary,
        ownership,
        server_authoritative_systems,
        seedlab,
        perf,
        honesty,
    })
}

/// Build, write `target/truth_report.json`, and print a console summary.
pub fn run(root: &Path, perf: Option<PerfLine>) -> Result<(), String> {
    let report = build_report(root, perf)?;
    let out = root.join("target/truth_report.json");
    let bytes = serde_json::to_vec_pretty(&report)
        .map_err(|e| format!("serialize: {e}"))?;
    std::fs::create_dir_all(out.parent().unwrap())
        .map_err(|e| format!("mkdir target/: {e}"))?;
    std::fs::write(&out, bytes).map_err(|e| format!("write {}: {e}", out.display()))?;

    println!("[ok] truth dashboard -> {}", out.display());
    println!(
        "     versions: protocol v{}, generator v{}",
        report.versions["protocol_version"], report.versions["generator_version"]
    );
    println!(
        "     counts: {} vistest scenes, {} #[test] attrs in {} rs files, {} systems tracked",
        report.counts["vistest_scenes"],
        report.counts["test_attributes"],
        report.counts["rust_source_files"],
        report.counts["systems_tracked"]
    );
    for (owner, n) in &report.ownership_summary {
        println!("     ownership: {n:2} {owner}");
    }
    println!(
        "     server-authoritative: {}",
        report.server_authoritative_systems.join(", ")
    );
    if let Some(p) = &report.perf {
        println!(
            "     perf: {} x{} p50 {:.1} ms p95 {:.1} ms",
            p.scene, p.frames, p.p50_ms, p.p95_ms
        );
    }
    if report.seedlab.is_some() {
        println!("     seedlab: folded from target/seedlab_report.json");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn row(name: &str) -> &'static SystemRow {
        SYSTEMS
            .iter()
            .find(|r| r.system == name)
            .unwrap_or_else(|| panic!("missing system row: {name}"))
    }

    /// THE B01 CONTRACT: the dashboard cannot label a client-only system
    /// authoritative. Every audited client-only system must be present and
    /// pinned `ClientLocal`, and every `ServerAuthoritative` claim must be
    /// backed by live `lf_server` source — so a claim without an
    /// implementation fails here, and losing an implementation fails here too.
    #[test]
    fn b01_no_client_only_system_is_labeled_server_authoritative() {
        for name in KNOWN_CLIENT_ONLY {
            let r = row(name);
            assert_eq!(
                r.owner,
                SimOwner::ClientLocal,
                "{name} is audited client-only but the dashboard claims {:?}",
                r.owner
            );
            assert!(
                r.server_markers.is_empty(),
                "{name} is client-only but cites server markers"
            );
        }
        for r in SYSTEMS {
            if r.owner != SimOwner::ServerAuthoritative {
                continue;
            }
            let cites_server = r
                .evidence
                .iter()
                .any(|p| p.starts_with("crates/lf_server") || p.starts_with("apps/loreforge-server"));
            assert!(
                cites_server,
                "{} claims server authority without citing the server crate",
                r.system
            );
            assert!(
                !r.server_markers.is_empty(),
                "{} claims server authority with no live-source marker",
                r.system
            );
            for marker in r.server_markers {
                assert!(
                    SERVER_SOURCE.contains(marker),
                    "{} claims server authority but live lf_server source lacks {:?}",
                    r.system,
                    marker
                );
            }
        }
    }

    #[test]
    fn every_evidence_path_exists_in_the_workspace() {
        let root = workspace_root();
        for r in SYSTEMS {
            for path in r.evidence {
                assert!(
                    root.join(path).exists(),
                    "{} cites missing evidence: {}",
                    r.system,
                    path
                );
            }
        }
    }

    #[test]
    fn system_names_are_unique_and_audit_list_is_covered() {
        let mut names: Vec<_> = SYSTEMS.iter().map(|r| r.system).collect();
        names.sort_unstable();
        let len = names.len();
        names.dedup();
        assert_eq!(names.len(), len, "duplicate system rows");
        for name in KNOWN_CLIENT_ONLY {
            assert!(
                SYSTEMS.iter().any(|r| r.system == *name),
                "KNOWN_CLIENT_ONLY names an untracked system: {name}"
            );
        }
    }

    #[test]
    fn report_versions_match_live_crate_constants() {
        let report = build_report(&workspace_root(), None).expect("build truth report");
        assert_eq!(
            report.versions["protocol_version"],
            lf_protocol::PROTOCOL_VERSION
        );
        assert_eq!(
            report.versions["generator_version"],
            lf_worldgen::GENERATOR_VERSION
        );
    }

    #[test]
    fn report_counts_are_sane_and_ownership_sums_to_the_inventory() {
        let report = build_report(&workspace_root(), None).expect("build truth report");
        assert_eq!(report.counts["vistest_scenes"], lf_vistest::scenes().len());
        assert!(report.counts["vistest_scenes"] >= 100, "scene registry shrank");
        assert!(report.counts["test_attributes"] >= 400, "test attr scan implausibly low");
        let summed: usize = report.ownership_summary.values().sum();
        assert_eq!(summed, SYSTEMS.len());
        assert!(report.ownership_summary.contains_key("ServerAuthoritative"));
        assert!(report.ownership_summary.contains_key("ClientLocal"));
        assert_eq!(
            report.server_authoritative_systems.len(),
            report.ownership_summary["ServerAuthoritative"]
        );
    }

    #[test]
    fn report_round_trips_through_json() {
        let report = build_report(&workspace_root(), None).expect("build truth report");
        let bytes = serde_json::to_vec(&report).expect("serialize");
        let back: TruthReport = serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(back.ownership.len(), report.ownership.len());
        assert_eq!(back.honesty["label"], report.honesty["label"]);
    }
}
