fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--headless") {
        let mut scene = "spawn_plains_dawn".to_string();
        let mut seed: Option<u64> = None;
        let mut out = std::path::PathBuf::from("shots/headless.png");
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--scene" if i + 1 < args.len() => { scene = args[i + 1].clone(); i += 1; }
                "--seed" if i + 1 < args.len() => { seed = args[i + 1].parse().ok(); i += 1; }
                "--out" if i + 1 < args.len() => { out = std::path::PathBuf::from(&args[i + 1]); i += 1; }
                _ => {}
            }
            i += 1;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        match lf_vistest::run_scene(&scene, seed, &out) {
            Ok(()) => {
                println!("wrote {}", out.display());
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("headless render failed: {}", e);
                std::process::exit(1);
            }
        }
    }
    lf_engine::run();
}
