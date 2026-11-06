//! N04 static-offset gate: reads this crate's own source files and
//! fails if any FORBIDDEN_OFFSETS pattern survives in production code.
//! Registry lives in pc3d_world::grounding; this is its renderer-side
//! enforcement (the consumer side of the contract).

use std::path::Path;

/// Production source files where grounding math is allowed ONLY through
/// the authority (ground_at) — never the historic magic offsets.
const AUDITED_FILES: &[&str] = &[
    "npcs.rs",
    "settlement.rs",
    "flora.rs",
    "city.rs",
    "slice.rs",
    "water.rs",
    "world_features.rs",
    "renderer.rs",
    "surface_stream.rs",
    "glb.rs",
    "machines.rs",
];

#[test]
fn p3dn04_no_forbidden_offsets_survive_in_render_sources() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders: Vec<String> = Vec::new();
    for file in AUDITED_FILES {
        let path = src_dir.join(file);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let test_start = source.find("#[cfg(test)]").unwrap_or(source.len());
        let prod = &source[..test_start];
        for (line_no, line) in prod.lines().enumerate() {
            // The historic offsets, stated as the registry describes them:
            // a ground/surface answer shifted by a magic constant. The
            // defect SHAPE is a grounding identifier on the line — grid
            // half-steps ((gx + 0.5) * step) and trig components
            // (1.0 * a.sin()) are geometry, not grounding.
            let grounding_id = ["ground", "surf", "surface", "_y", "gy", "base_y", "feet"];
            let line_has_grounding = grounding_id.iter().any(|g| line.contains(g));
            if !line_has_grounding {
                continue;
            }
            let forbidden = [
                ("+ 1.0", "npc +1.0 m float"),
                ("+1.0", "npc +1.0 m float"),
                ("+ 0.5", "anchor +0.5 m center"),
                ("+0.5", "anchor +0.5 m center"),
                ("- 0.2", "wheel -0.2 m by eye"),
                ("-0.2", "wheel -0.2 m by eye"),
            ];
            // Split into comma-separated components: the defect class is a
            // Y-channel (grounding) component shifted by a magic constant.
            // X/Z half-steps on cell coords are voxel-center geometry.
            let components: Vec<&str> = line.split(',').collect();
            let y_comp = |c: &str| -> bool {
                let lc = c.to_lowercase();
                lc.contains("ground") || lc.contains("surf") || lc.contains("_y")
                    || lc.contains("gy") || lc.contains("feet") || lc.contains("base")
            };
            for (pat, label) in forbidden {
                for c in &components {
                    if c.contains(pat) && y_comp(c) {
                        let trimmed = line.trim();
                        offenders.push(format!("{file}:{line_no}: {label} -> {trimmed}"));
                        break;
                    }
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "FORBIDDEN_OFFSETS survivors in production grounding math:\n{}",
        offenders.join("\n")
    );
}
