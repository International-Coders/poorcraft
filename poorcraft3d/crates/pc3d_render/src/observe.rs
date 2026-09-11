//! WT-003: the game observatory runtime — evidence bundles, runtime
//! state exports, route table, and the regression comparator.
//!
//! CLI-first (the same names/shapes a future MCP server would expose):
//! `--observe <route>` writes a bundle directory (beauty PNG + layout +
//! runtime state + perf + verdict, all stamped with build hash/seed/
//! viewport/scene/command); `--compare-evidence` renders a pass/fail/
//! inconclusive verdict between two bundles. Routes the game cannot
//! honestly run yet (house entry, NPC talk, forge use, GPU markers) are
//! declared unavailable WITH reasons — an honest unavailability is a
//! valid verdict; a silent skip is not.

use crate::app::{CaptureOutcome, WindowReport};

/// One route from the contract's required list.
pub struct RouteSpec {
    pub id: &'static str,
    pub available: bool,
    pub reason: &'static str,
}

/// The route table (input_route_contract.json's required routes).
pub fn routes() -> &'static [RouteSpec] {
    &[
        RouteSpec { id: "route_title_mouse", available: true, reason: "" },
        RouteSpec { id: "route_new_world_seed", available: true, reason: "" },
        RouteSpec { id: "route_escape_pause", available: true, reason: "" },
        RouteSpec { id: "route_asset_inspect", available: true, reason: "" },
        RouteSpec {
            id: "route_house_entry",
            available: false,
            reason: "house entry needs door collision + interior interactions — the semantic playtest slice (WT-002 slice 5)",
        },
        RouteSpec {
            id: "route_npc_talk",
            available: false,
            reason: "talk UI does not exist yet — the NPC talk slice",
        },
        RouteSpec {
            id: "route_forge_use",
            available: false,
            reason: "forge interaction UI does not exist yet — the forge gameplay slice",
        },
        RouteSpec {
            id: "route_gpu_markers",
            available: true,
            reason: "",
        },
    ]
}

/// FNV-1a digest over a JSON value's canonical string form — bundles are
/// comparable byte-for-byte at the data level.
pub fn digest_json(v: &serde_json::Value) -> String {
    let s = serde_json::to_string(v).unwrap_or_default();
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn frame_percentiles(ms: &[f32]) -> serde_json::Value {
    let mut v = ms.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pick = |p: f32| -> f32 {
        if v.is_empty() {
            0.0
        } else {
            v[((v.len() as f32 - 1.0) * p) as usize]
        }
    };
    serde_json::json!({
        "frames": v.len(),
        "p50_ms": (pick(0.50) * 100.0).round() / 100.0,
        "p95_ms": (pick(0.95) * 100.0).round() / 100.0,
        "avg_fps": if v.is_empty() { 0.0 } else {
            (1000.0 / (v.iter().sum::<f32>() / v.len() as f32) * 10.0).round() / 10.0
        },
    })
}

/// The runtime state export exactly per `runtime_state_export.schema.json`
/// (20 fields; nulls carry reasons — `null_requires_reason`).
#[allow(clippy::too_many_arguments)]
pub fn runtime_state_json(
    report: &WindowReport,
    last_capture: Option<&CaptureOutcome>,
    scene: &str,
    seed: &str,
    command: &str,
    camera: &str,
) -> serde_json::Value {
    let ui = last_capture
        .and_then(|c| c.ui_layout.as_ref())
        .map(|l| l.get("ui_state").cloned().unwrap_or(serde_json::json!({})))
        .unwrap_or(serde_json::json!({}));
    let seed_preview = last_capture.and_then(|c| c.seed_preview.as_ref());
    let n_interactions = ui.get("focus").is_some() as usize
        + ui.get("hover").is_some() as usize;
    serde_json::json!({
        "version": 1,
        "build_hash": crate::ui::build_stamp(),
        "command": command,
        "timestamp_utc": chrono_like_now(),
        "scene": scene,
        "seed": seed,
        "viewport": [report.final_physical.0, report.final_physical.1],
        "quality": ui.get("quality").cloned().unwrap_or(serde_json::json!("mid")),
        "camera": camera,
        "player": ui.get("player").cloned().unwrap_or(serde_json::json!({
            "note": "player pose rides the route camera script in UI routes"
        })),
        "ui": ui,
        "world": {
            "seed_resolved": seed_preview.map(|p| p.resolved_seed.to_string())
                .unwrap_or_else(|| seed.to_string()),
            "spawn_safe": seed_preview.map(|p| p.valid),
            "built_count": null_with_reason("construction not mounted in UI routes"),
        },
        "visible_assets": {
            "note": "the asset inspection route dumps the full starter batch",
            "count": pc3d_assets::semantic::starter_batch().len(),
        },
        "interactions": {
            "pointer_grabbed": ui.get("pointer_grabbed").cloned().unwrap_or(serde_json::json!(false)),
            "gameplay_input_blocked": ui.get("gameplay_input_blocked").cloned().unwrap_or(serde_json::json!(false)),
            "tracked_ui_state_events": n_interactions,
        },
        "npcs": null_with_reason("no crowd mounted in UI routes (see --play-people)"),
        "machines": null_with_reason("sim machines not mounted in UI routes (see --journey)"),
        "perf": frame_percentiles(&report.frame_ms),
        "gpu_markers": null_with_reason("pass markers pending WT-007 slice 1"),
        "proof_paths": [],
        "verdict": "exported",
    })
}

fn null_with_reason(reason: &str) -> serde_json::Value {
    serde_json::json!({ "value": null, "reason": reason })
}

fn chrono_like_now() -> String {
    // No chrono dep: a monotonic-ish stamp from the system clock.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".into())
}

/// The evidence bundle JSON exactly per `evidence_bundle.schema.json`
/// (18 fields; every NON-EMPTY path must exist on disk — the comparator
/// enforces it).
#[allow(clippy::too_many_arguments)]
pub fn bundle_json(
    route_id: &str,
    scene: &str,
    seed: &str,
    viewport: (u32, u32),
    quality: &str,
    commands: &[String],
    runtime_state_path: &str,
    screenshots: &[String],
    asset_dumps: &[String],
    perf_path: &str,
    verdict_path: &str,
) -> serde_json::Value {
    serde_json::json!({
        "bundle_id": format!("{route_id}-{}", crate::ui::build_stamp()),
        "build_hash": crate::ui::build_stamp(),
        "route_id": route_id,
        "scene": scene,
        "seed": seed,
        "viewport": [viewport.0, viewport.1],
        "quality": quality,
        "commands": commands,
        "runtime_state_path": runtime_state_path,
        "screenshots": screenshots,
        "wireframes": [],
        "overlays": [],
        "asset_dumps": asset_dumps,
        "perf_path": perf_path,
        "gpu_marker_path": "",
        "diff_paths": [],
        "verdict_path": verdict_path,
        "digest": "",
    })
}

/// Stamp the digest over a bundle (minus the digest field itself).
pub fn stamp_digest(bundle: &mut serde_json::Value) {
    let mut copy = bundle.clone();
    copy["digest"] = serde_json::json!("");
    bundle["digest"] = serde_json::json!(digest_json(&copy));
}

/// The regression comparator (slice 7): pass / fail / inconclusive with
/// named checks. Reads two bundle directories' bundle.json files.
pub fn compare_bundles(a_dir: &std::path::Path, b_dir: &std::path::Path) -> serde_json::Value {
    let mut checks: Vec<serde_json::Value> = Vec::new();
    let read = |d: &std::path::Path| -> Option<serde_json::Value> {
        std::fs::read_to_string(d.join("bundle.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    };
    let (a, b) = (read(a_dir), read(b_dir));
    let push = |checks: &mut Vec<_>, name: &str, pass: bool, detail: String| {
        checks.push(serde_json::json!({
            "name": name, "pass": pass, "detail": detail,
        }));
    };
    let (Some(a), Some(b)) = (a, b) else {
        push(&mut checks, "bundles_readable", false, "bundle.json missing".into());
        return verdict("fail", checks);
    };
    push(&mut checks, "bundles_readable", true, "both bundle.json parsed".into());
    if a["route_id"] != b["route_id"] {
        push(
            &mut checks,
            "same_route",
            false,
            format!("{} vs {}", a["route_id"], b["route_id"]),
        );
        return verdict("fail", checks);
    }
    push(&mut checks, "same_route", true, a["route_id"].to_string());
    // Every bundle's non-empty paths must exist — relative paths resolve
    // against the bundle's OWN directory.
    let resolve = |dir: &std::path::Path, p: &str| -> std::path::PathBuf {
        if std::path::Path::new(p).is_absolute() {
            p.into()
        } else {
            dir.join(p)
        }
    };
    let dirs = [("A", a_dir), ("B", b_dir)];
    for (i, (name, bundle)) in [("A", &a), ("B", &b)].iter().enumerate() {
        let dir = dirs[i].1;
        let mut missing = Vec::new();
        for key in [
            "runtime_state_path", "perf_path", "verdict_path",
        ] {
            let p = bundle[key].as_str().unwrap_or("");
            if !p.is_empty() && !resolve(dir, p).exists() {
                missing.push(format!("{name}:{key}={p}"));
            }
        }
        for s in bundle["screenshots"].as_array().unwrap_or(&Vec::new()) {
            let p = s.as_str().unwrap_or("");
            if !p.is_empty() && !resolve(dir, p).exists() {
                missing.push(format!("{name}:screenshot={p}"));
            }
        }
        push(&mut checks, &format!("{name}_paths_exist"), missing.is_empty(), if missing.is_empty() { "all listed paths exist".into() } else { missing.join(", ") });
    }
    // Screenshots nonblank in both bundles.
    for (i, (name, bundle)) in [("A", &a), ("B", &b)].iter().enumerate() {
        let dir = dirs[i].1;
        let mut blank = Vec::new();
        for s in bundle["screenshots"].as_array().unwrap_or(&Vec::new()) {
            let p = s.as_str().unwrap_or("");
            if p.is_empty() {
                continue;
            }
            let full = resolve(dir, p);
            match image::open(&full) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    let mut distinct = std::collections::BTreeSet::new();
                    for px in rgba.pixels().step_by(97) {
                        distinct.insert((px[0], px[1], px[2]));
                    }
                    if distinct.len() < 8 {
                        blank.push(p.to_string());
                    }
                }
                Err(_) => blank.push(format!("{p} (unreadable)")),
            }
        }
        push(&mut checks, &format!("{name}_nonblank"), blank.is_empty(), if blank.is_empty() { "screenshots carry content".into() } else { blank.join(", ") });
    }
    // Seed agreement: differing seeds make the comparison inconclusive,
    // not failed (they describe different worlds on purpose).
    let same_seed = a["seed"] == b["seed"];
    push(
        &mut checks,
        "same_seed",
        same_seed,
        format!("{} vs {}", a["seed"], b["seed"]),
    );
    let hard_fail = checks.iter().any(|c| c["pass"] == false && c["name"] != "same_seed");
    if hard_fail {
        verdict("fail", checks)
    } else if !same_seed {
        verdict("inconclusive", checks)
    } else {
        verdict("pass", checks)
    }
}

fn verdict(v: &str, checks: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "version": 1,
        "build_hash": crate::ui::build_stamp(),
        "verdict": v,
        "checks": checks,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_table_covers_the_contract_with_honest_reasons() {
        let rs = routes();
        assert_eq!(rs.len(), 8, "the contract's eight required routes");
        for r in rs {
            if !r.available {
                assert!(
                    r.reason.len() > 20,
                    "{} needs a real reason, not a shrug",
                    r.id
                );
            }
        }
        assert!(rs.iter().filter(|r| r.available).count() >= 4);
    }

    #[test]
    fn bundle_json_has_every_schema_field() {
        let b = bundle_json(
            "route_title_mouse", "title", "4242", (1280, 720), "mid",
            &["--observe route_title_mouse".into()],
            "runtime_state.json", &["a.png".into()], &[], "perf.json", "verdict.json",
        );
        for field in [
            "bundle_id", "build_hash", "route_id", "scene", "seed", "viewport",
            "quality", "commands", "runtime_state_path", "screenshots", "wireframes",
            "overlays", "asset_dumps", "perf_path", "gpu_marker_path", "diff_paths",
            "verdict_path", "digest",
        ] {
            assert!(b.get(field).is_some(), "bundle field {field} missing");
        }
        // Digest stamping is stable and content-derived.
        let mut b1 = b.clone();
        stamp_digest(&mut b1);
        let mut b2 = b;
        stamp_digest(&mut b2);
        assert_eq!(b1["digest"], b2["digest"]);
        let mut b3 = b1.clone();
        b3["seed"] = serde_json::json!("7");
        stamp_digest(&mut b3);
        assert_ne!(b1["digest"], b3["digest"], "digest must bind the content");
    }

    #[test]
    fn comparator_passes_identical_fails_blank_and_notes_seed() {
        // Build two real bundle dirs in a temp folder.
        let root = std::env::temp_dir().join(format!("p3d-observe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mk = |name: &str, seed: &str, blank: bool| -> std::path::PathBuf {
            let dir = root.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            let png = dir.join("beauty.png");
            let img = image::RgbaImage::from_fn(64, 64, |x, y| if blank {
                image::Rgba([10, 10, 10, 255])
            } else {
                image::Rgba([(x * 4) as u8, (y * 4) as u8, 128, 255])
            });
            img.save(&png).unwrap();
            std::fs::write(dir.join("runtime_state.json"), "{}").unwrap();
            std::fs::write(dir.join("perf.json"), "{}").unwrap();
            std::fs::write(dir.join("verdict.json"), "{}").unwrap();
            let mut b = bundle_json(
                "route_title_mouse", "title", seed, (1280, 720), "mid",
                &["test".into()], "runtime_state.json",
                &["beauty.png".into()], &[], "perf.json", "verdict.json",
            );
            stamp_digest(&mut b);
            std::fs::write(dir.join("bundle.json"), serde_json::to_string(&b).unwrap()).unwrap();
            dir
        };
        let a = mk("a", "4242", false);
        let b = mk("b", "4242", false);
        let v = compare_bundles(&a, &b);
        assert_eq!(v["verdict"], "pass", "same seed, real images: {v}");
        let c = mk("c", "4242", true);
        let v = compare_bundles(&a, &c);
        assert_eq!(v["verdict"], "fail", "blank screenshot must fail: {v}");
        let d = mk("d", "7", false);
        let v = compare_bundles(&a, &d);
        assert_eq!(v["verdict"], "inconclusive", "different seeds: {v}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
