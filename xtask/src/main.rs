use std::path::PathBuf;

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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("help");
    match cmd {
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
        _ => {
            println!("LOREFORGE xtask automation");
            println!("  cargo xtask vistest [out-dir]       render all scenes to PNGs");
            println!("  cargo xtask screenshot <scene> [out] [seed]  render one scene");
            println!("  cargo xtask perf [scene] [n]        frame-time benchmark (p50/p95)");
            println!("  cargo xtask package                build release artifacts (P11)");
        }
    }
}
