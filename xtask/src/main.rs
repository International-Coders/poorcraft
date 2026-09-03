use std::path::PathBuf;

mod gen;
mod night_plan;
mod truth;

fn copy_dir(src: &str, dst: std::path::PathBuf) {
    let root = PathBuf::from(src);
    let _ = std::fs::create_dir_all(&dst);
    if let Ok(entries) = std::fs::read_dir(&root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let target = dst.join(entry.file_name());
            if path.is_dir() {
                copy_dir(path.to_str().unwrap_or(""), target);
            } else {
                let _ = std::fs::copy(&path, &target);
            }
        }
    }
}

/// Step 39: scaffold a new mod folder (manifest + data files), refusing
/// to overwrite an existing one. Returns the created path.
fn scaffold_mod(root: &std::path::Path, id: &str, name: &str) -> Result<std::path::PathBuf, String> {
    if id.is_empty() || !id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        return Err(format!("mod id must be [a-z0-9_], got {:?}", id));
    }
    let dir = root.join(id);
    if dir.exists() {
        return Err(format!("{} already exists", dir.display()));
    }
    let data = dir.join("data");
    std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("mod.toml"), format!(
"# {} mod manifest — see mods/README.md for the full guide
id = \"{}\"
name = \"{}\"
version = \"0.1.0\"
api_version = \"1\"
side = \"both\"
dependencies = [\"core\"]
permissions = [\"world.read\", \"world.write\"]
", name, id, name)).map_err(|e| e.to_string())?;
    std::fs::write(data.join("blocks.toml"), format!(
"# One example block to start from — delete or duplicate at will.
[[blocks]]
id = \"{}:example_block\"
name = \"Example Block\"
texture = \"example_block.png\"
hardness = 1.0
harvest_level = 0
light = 0
", id)).map_err(|e| e.to_string())?;
    std::fs::write(data.join("items.toml"), format!(
"[[items]]
id = \"{}:example_token\"
name = \"Example Token\"
", id)).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("help");
    match cmd {
        "seedlab" => {
            // N05: the 64-seed diversity laboratory — machine-readable
            // report at target/seedlab_report.json (transient by design;
            // the tracked evidence is the test suite's reduced corpus)
            let seeds = lf_worldgen::seedlab::corpus_64();
            let report = lf_worldgen::seedlab::diversity_report(
                &seeds, lf_worldgen::WorldType::Normal, 384, 8);
            let path = std::path::Path::new("target/seedlab_report.json");
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match serde_json::to_vec_pretty(&report) {
                Ok(bytes) => {
                    let _ = std::fs::write(path, &bytes);
                    println!("[ok] seedlab: {} seeds -> {} ({} bytes)",
                        report.corpus_size, path.display(), bytes.len());
                }
                Err(e) => eprintln!("[FAIL] seedlab serialize: {e}"),
            }
            println!("  generator v{} · pairs {} · height L1 mean {:.4} p05 {:.4} · biome JS mean {:.4} p05 {:.4}",
                report.generator_version, report.pairwise.pairs,
                report.pairwise.mean_height_l1, report.pairwise.p05_height_l1,
                report.pairwise.mean_biome_js, report.pairwise.p05_biome_js);
            if report.failures.is_empty() {
                println!("  diversity: PASS");
            } else {
                for f in &report.failures {
                    println!("  FAIL: {f}");
                }
                std::process::exit(1);
            }
        }
        "truth" => {
            // B01: runtime truth dashboard — machine-readable ownership,
            // versions, and counts at target/truth_report.json. Optional live
            // perf via --bench <scene> [frames].
            let mut perf = None;
            let mut rest = args.iter().skip(2);
            while let Some(flag) = rest.next().cloned() {
                if flag == "--bench" {
                    let scene = rest.next().cloned().unwrap_or_default();
                    let frames: usize = rest
                        .next()
                        .and_then(|n| n.parse().ok())
                        .unwrap_or(120);
                    match lf_vistest::bench(&scene, frames) {
                        Ok(b) => {
                            perf = Some(truth::PerfLine {
                                scene,
                                frames,
                                p50_ms: b.p50_ms as f64,
                                p95_ms: b.p95_ms as f64,
                                min_ms: b.min_ms as f64,
                            });
                        }
                        Err(e) => eprintln!("[warn] bench {scene}: {e}"),
                    }
                }
            }
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .to_path_buf();
            if let Err(e) = truth::run(&root, perf) {
                eprintln!("[FAIL] truth: {e}");
                std::process::exit(1);
            }
        }
        "night-plan-check" => {
            let root = args
                .get(2)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("docs/NIGHTLY-BETA"));
            match night_plan::validate(&root) {
                Ok(stats) => println!(
                    "[ok] nightly beta plan: {} documents, {} ordered jobs, {} bytes, {} local links",
                    stats.documents, stats.jobs, stats.bytes, stats.links_checked
                ),
                Err(e) => {
                    eprintln!("[FAIL] nightly beta plan: {e}");
                    std::process::exit(1);
                }
            }
        }
        "new-mod" => {
            // Step 39: `xtask new-mod <id> [--name "Pretty Name"]`
            let id = args.get(2).cloned().unwrap_or_default();
            let name = args.iter().position(|a| a == "--name")
                .and_then(|i| args.get(i + 1).cloned())
                .unwrap_or_else(|| id.clone());
            if id.is_empty() {
                eprintln!("usage: xtask new-mod <id> [--name \"Pretty Name\"]");
                std::process::exit(2);
            }
            match scaffold_mod(std::path::Path::new("mods"), &id, &name) {
                Ok(dir) => println!("[ok] scaffolded {} — edit {}/data/*.toml (see mods/README.md)",
                    dir.display(), dir.display()),
                Err(e) => {
                    eprintln!("[FAIL] {}", e);
                    std::process::exit(1);
                }
            }
        }
        "vistest" => {
            // Renders every registered scene to shots/vistest_<name>.png.
            let out_dir = args.get(2).cloned().unwrap_or_else(|| "shots".into());
            std::fs::create_dir_all(&out_dir).expect("create shots dir");
            let mut failed = false;
            for scene in lf_vistest::scenes() {
                let out = PathBuf::from(&out_dir).join(format!("vistest_{}.png", scene.name));
                match lf_vistest::run_scene(scene.name, None, &out) {
                    Ok(()) => println!("[ok] {} -> {} ({})", scene.name, out.display(), scene.desc),
                    Err(e) => {
                        println!("[FAIL] {}: {}", scene.name, e);
                        failed = true;
                    }
                }
            }
            if failed {
                std::process::exit(1);
            }
        }
        "screenshot" => {
            // cargo xtask screenshot <scene> [out.png] [seed]
            let scene = args.get(2).map(String::as_str).unwrap_or("spawn_plains_dawn");
            let out = args.get(3).cloned().unwrap_or_else(|| format!("shots/{}.png", scene));
            let seed = args.get(4).and_then(|s| s.parse().ok());
            match lf_vistest::run_scene(scene, seed, std::path::Path::new(&out)) {
                Ok(()) => println!("wrote {}", out),
                Err(e) => {
                    eprintln!("failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "package" => {
            // Build release binaries and assemble a portable distribution
            // directory + zip. Cross-platform by design: whatever host runs
            // this produces that host's artifact.
            let out_dir = PathBuf::from("dist/loreforge");
            let _ = std::fs::remove_dir_all(&out_dir);
            std::fs::create_dir_all(out_dir.join("bin")).expect("create dist dir");
            let bins = [("loreforge", "loreforge"), ("loreforge-server", "loreforge-server")];
            for (package, bin) in bins {
                let status = std::process::Command::new("cargo")
                    .args(["build", "--release", "-p", package])
                    .status()
                    .expect("run cargo");
                if !status.success() {
                    eprintln!("build of {} failed", package);
                    std::process::exit(1);
                }
                let src = PathBuf::from(format!("target/release/{}", bin));
                let dst = out_dir.join("bin").join(bin);
                std::fs::copy(&src, &dst).expect("copy binary");
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(0o755));
                }
            }
            // ship the mods and docs
            if PathBuf::from("mods").exists() {
                copy_dir("mods", out_dir.join("mods"));
            }
            for f in ["mods/README.md", "RELEASE.md"] {
                if PathBuf::from(f).exists() {
                    let name = PathBuf::from(f).file_name().unwrap().to_str().unwrap().to_string();
                    let _ = std::fs::copy(f, out_dir.join(name));
                }
            }
            // zip it
            let os = std::env::consts::OS;
            let zip_name = format!("dist/loreforge-{}-{}.zip", env!("CARGO_PKG_VERSION"), os);
            let _ = std::fs::remove_file(&zip_name);
            let ok = std::process::Command::new("zip")
                .args(["-r", &zip_name, "dist/loreforge"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            println!("packaged {} -> {}", out_dir.display(), if ok { zip_name } else { "(zip tool missing; directory only)".to_string() });
        }
        "perf" => {
            // Frame-time benchmark (goal Section 5 / Step 9): persistent
            // renderer, scene built once — measures per-frame cost (mesh
            // upload + GPU + readback + PNG encode), not setup. Caveat
            // recorded in DECISIONS.md.
            let scene = args.get(2).map(String::as_str).unwrap_or("terrain_vista");
            let n: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(30);
            match lf_vistest::bench(scene, n) {
                Ok(b) => println!(
                    "perf {} x{} (warm): p50 {:.1} ms  p95 {:.1} ms  min {:.1} ms  (=> ~{:.0} fps at p50)",
                    scene, n - 1, b.p50_ms, b.p95_ms, b.min_ms, 1000.0 / b.p50_ms.max(0.001)
                ),
                Err(e) => {
                    eprintln!("perf failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        "gen-texture" => {
            // F1: gen-texture <type> <output.png> [--seed N] [--faction id]
            //                    [--base-color RRGGBB] [--variation 0..50]
            let ty = args.get(2).map(String::as_str).unwrap_or("");
            let out = args.get(3).cloned().unwrap_or_default();
            let mut seed = 0u64;
            let mut faction = String::new();
            let mut base = [106u8, 106, 106];
            let mut variation = 12i32;
            let mut i = 4;
            while i < args.len() {
                match args[i].as_str() {
                    "--seed" if i + 1 < args.len() => { seed = args[i + 1].parse().unwrap_or(0); i += 1; }
                    "--faction" if i + 1 < args.len() => { faction = args[i + 1].clone(); i += 1; }
                    "--base-color" if i + 1 < args.len() => {
                        if let Ok(c) = gen::parse_hex(&args[i + 1]) { base = c; }
                        i += 1;
                    }
                    "--variation" if i + 1 < args.len() => { variation = args[i + 1].parse().unwrap_or(12); i += 1; }
                    _ => {}
                }
                i += 1;
            }
            let result = match ty {
                "grass-ctm-strip" if !out.is_empty() => gen::grass_ctm_strip(seed).save(&out).map_err(|e| e.to_string()),
                "stone-ctm-strip" if !out.is_empty() => gen::stone_ctm_strip(seed).save(&out).map_err(|e| e.to_string()),
                "entity-skin" if !out.is_empty() => {
                    match gen::FACTIONS.iter().position(|f| f.id == faction) {
                        Some(idx) => gen::entity_skin(idx, seed).save(&out).map_err(|e| e.to_string()),
                        None => Err(format!("unknown faction {:?} (want one of: {:?})", faction,
                            gen::FACTIONS.iter().map(|f| f.id).collect::<Vec<_>>())),
                    }
                }
                "block-noise" if !out.is_empty() => gen::block_noise(base, variation, 16, seed).save(&out).map_err(|e| e.to_string()),
                _ => Err("usage: gen-texture <grass-ctm-strip|stone-ctm-strip|entity-skin|block-noise> <out.png> [--seed N] [--faction id] [--base-color RRGGBB] [--variation 0..50]".into()),
            };
            if let Err(e) = result {
                eprintln!("gen-texture failed: {}", e);
                std::process::exit(1);
            }
            println!("wrote {}", out);
        }
        "gen-ctm" => {
            // E6: bootstrap a block's CTM strip PNG from its in-game art
            let block = args.get(2).map(String::as_str).unwrap_or("");
            let out = format!("assets/ctm/{}.png", block);
            match lf_assets::CTM_BLOCKS.iter().find(|b| b.art == block) {
                Some(_) => {
                    if std::path::Path::new(&out).exists() {
                        println!("SKIP (exists): {}", out);
                    } else {
                        let _ = std::fs::create_dir_all("assets/ctm");
                        match gen::grass_ctm_strip(0).save(&out) {
                            Ok(()) => println!("wrote {} (seed 0 = the exact in-game strip)", out),
                            Err(e) => { eprintln!("gen-ctm failed: {}", e); std::process::exit(1); }
                        }
                    }
                }
                None => {
                    eprintln!("gen-ctm: unknown block {:?}; CTM blocks: {:?}", block,
                        lf_assets::CTM_BLOCKS.iter().map(|b| b.art).collect::<Vec<_>>());
                    std::process::exit(1);
                }
            }
        }
        "gen-all-textures" => {
            // F2: every CTM strip + every faction skin; existing files are
            // skipped (never overwrite hand-crafted assets)
            for line in gen::gen_all(std::path::Path::new(".")) {
                println!("{}", line);
            }
        }
        _ => {
            println!("LOREFORGE xtask automation");
            println!("  cargo xtask vistest [out-dir]       render all scenes to PNGs");
            println!("  cargo xtask screenshot <scene> [out] [seed]  render one scene");
            println!("  cargo xtask perf [scene] [n]        frame-time benchmark (p50/p95)");
            println!("  cargo xtask package                build release artifacts (P11)");
            println!("  cargo xtask gen-texture <type> <out.png> [--seed N] [--faction id] [--base-color RRGGBB] [--variation N]");
            println!("  cargo xtask gen-ctm <block>         write a block's CTM strip to assets/ctm/");
            println!("  cargo xtask gen-all-textures        all CTM strips + faction skins (skips existing)");
            println!("  cargo xtask night-plan-check [dir]  validate the ZCode nightly beta goal pack");
            println!("  cargo xtask seedlab                 64-seed diversity report -> target/seedlab_report.json");
        }
    }
}

#[cfg(test)]
mod tests {
    /// Failure meaning: the asset generator is not deterministic — the
    /// same seed must produce bit-identical output on any machine (F3).
    #[test]
    fn asset_generator_grass_output() {
        let a = crate::gen::grass_ctm_strip(42);
        let b = crate::gen::grass_ctm_strip(42);
        assert_eq!(a.as_raw(), b.as_raw(), "grass strip (seed 42) must be bit-identical");
        assert_eq!(a.dimensions(), (192, 512), "full 8-block strip layout");
        // the strip is not blank and not absurd: it has colour variety and
        // no pure-white / pure-black pixels (pixel-art Rule 1)
        let mut colors = std::collections::HashSet::new();
        for p in a.pixels() {
            assert!((p[0] as u32) + (p[1] as u32) + (p[2] as u32) > 0, "pure-black pixel in strip");
            assert!((p[0] as u32) + (p[1] as u32) + (p[2] as u32) < 765, "pure-white pixel");
            colors.insert(p.0);
        }
        assert!(colors.len() > 32, "strip has only {} distinct colors", colors.len());
    }

    /// Failure meaning: the other generator paths lost determinism or the
    /// output guard stopped protecting hand-crafted files.
    #[test]
    fn generator_outputs_are_deterministic_and_guarded() {
        let s1 = crate::gen::stone_ctm_strip(7);
        let s2 = crate::gen::stone_ctm_strip(7);
        assert_eq!(s1.as_raw(), s2.as_raw());
        let e1 = crate::gen::entity_skin(0, 42);
        let e2 = crate::gen::entity_skin(0, 42);
        assert_eq!(e1.as_raw(), e2.as_raw(), "entity skin must be deterministic");
        assert_eq!(e1.dimensions(), (64, 32));
        let n1 = crate::gen::block_noise([90, 138, 42], 12, 16, 5);
        let n2 = crate::gen::block_noise([90, 138, 42], 12, 16, 5);
        assert_eq!(n1.as_raw(), n2.as_raw());
        // gen_all only writes NEW files: seed the temp tree, rerun, verify
        // the SKIP paths report (never overwrite)
        let dir = std::env::temp_dir().join(format!("lf_gen_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let first = crate::gen::gen_all(&dir);
        assert_eq!(first.iter().filter(|l| l.starts_with("wrote")).count(), 14);
        let second = crate::gen::gen_all(&dir);
        assert_eq!(second.iter().filter(|l| l.starts_with("SKIP")).count(), 14,
            "second run must skip everything: {:?}", second);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
