use std::path::PathBuf;

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
            println!("Packaging release artifacts... (not implemented yet; lands in P11)");
        }
        _ => {
            println!("LOREFORGE xtask automation");
            println!("  cargo xtask vistest [out-dir]       render all scenes to PNGs");
            println!("  cargo xtask screenshot <scene> [out] [seed]  render one scene");
            println!("  cargo xtask package                build release artifacts (P11)");
        }
    }
}
