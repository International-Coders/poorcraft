//! POORCRAFT 3D executable — a stub that can answer "who am I" (P3D-001)
//! and state its format law (P3D-002). The first real runtime loop arrives
//! with P3D-005.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--identity") | None => {
            println!("{}", pc3d_core::identity_block());
            if args.len() == 1 {
                println!("\n(no runtime yet — P3D-005 builds the first empty-world loop)");
            }
        }
        Some("--format") => {
            let sup = pc3d_core::SupportedVersions::epoch1();
            let header = pc3d_core::FormatHeader::current();
            println!(
                "file header: {} bytes — magic(4) | epoch u32le | world/save/content/protocol u16le each",
                pc3d_core::HEADER_LEN
            );
            println!(
                "this build: epoch {} · world v{} · save v{} · content v{} · protocol v{}",
                sup.epoch, sup.world, sup.save, sup.content, sup.protocol
            );
            println!("wire bytes: {:02x?}", header.encode());
            println!("law: unknown versions are refused with a reason, never guessed (D-002)");
        }
        Some("--baseline") => {
            // A deterministic synthetic workload exercising the spine: a
            // fixed clock, a stream-driven command schedule, frame times
            // from the same jitter every run. Same machine, same numbers.
            use pc3d_core::{CounterId, Counters, FixedClock, FrameTimes, SeedStreams};
            let mut counters = Counters::default();
            let mut frames = FrameTimes::default();
            let mut clock = FixedClock::new();
            let streams = SeedStreams::new(0xC0FFEE);
            let mut rng = streams.rng(pc3d_core::stream::WEATHER);
            for _ in 0..600 {
                // Deterministic ~30-90 fps jitter.
                let dt_ms = 11.0 + rng.unit_f32() * 22.0;
                frames.push(dt_ms);
                for tick in clock.advance(dt_ms / 1000.0) {
                    counters.inc(CounterId::EntityTicks);
                    let _ = tick;
                }
                counters.add(CounterId::MeshWork, 3);
                counters.add(CounterId::JournalEvents, 1);
            }
            let record = pc3d_core::BaselineRecord {
                profile_name: "p3d000-synthetic".into(),
                arch: std::env::consts::ARCH.into(),
                os: std::env::consts::OS.into(),
                format_epoch: 1,
                counters,
                frames: frames.len(),
                p50_ms: frames.p50(),
                p95_ms: frames.p95(),
                min_ms: frames.min(),
                max_ms: frames.max(),
            };
            println!("{}", record.to_json());
        }
        Some("--run") => {
            // P3D-005: the first runtime. Headless empty world, deterministic
            // jitter frames (~27-90 fps equivalent), real clock/counters/
            // journal. `--run [seconds]` simulates 60 fps-equivalent frames.
            let seconds: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5);
            let frames = seconds * 60;
            let rt = pc3d_core::run_headless(0x00C0FFEE, frames);
            let s = rt.stats();
            println!(
                "ran {} frames ({} s) · {} ticks · {} journal events · frame p50 {:.2} ms p95 {:.2} ms",
                s.frames, seconds, s.ticks, s.journal_events, s.p50_ms, s.p95_ms
            );
            println!("digest {:016x}", s.digest);
            // Liveness: a running world ticks, journals, and measures.
            if s.ticks == 0 || s.journal_events == 0 || s.p50_ms <= 0.0 {
                eprintln!("[FAIL] runtime not alive: {s:?}");
                std::process::exit(1);
            }
        }
        Some("--atlas") => {
            // P3D-104: render a seed's biome atlas to shots/atlas_seed<N>.png
            // and print the disagreement census + patch-hash spot checks.
            let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
            let half: i32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(48);
            let atlas = pc3d_world::proof::render_region_atlas(seed, half);
            let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("shots");
            std::fs::create_dir_all(&out_dir).expect("mkdir shots");
            let out = out_dir.join(format!("atlas_seed{seed}.png"));
            image::save_buffer(
                &out,
                &atlas.rgb,
                atlas.size as u32,
                atlas.size as u32,
                image::ColorType::Rgb8,
            )
            .expect("encode png");
            println!(
                "[ok] atlas seed {seed} -> {} ({}x{} regions)",
                out.display(),
                atlas.size,
                atlas.size
            );
            // Census + spot checks: worlds differ, regeneration replays.
            for other in [seed + 1, seed.wrapping_add(0x9E37)] {
                let d = pc3d_world::proof::cross_seed_disagreement(seed, other, 24);
                println!("     biome disagreement vs seed {other}: {:.1}%", d * 100.0);
            }
            let spot: Vec<bool> = (-2..=2)
                .map(|i| {
                    pc3d_world::proof::verify_patch_hash(
                        seed,
                        pc3d_world::PatchCoord { x: i * 4, y: -1, z: i * 3 },
                    )
                })
                .collect();
            println!(
                "     patch-hash spot checks: {}/{} ok",
                spot.iter().filter(|&&ok| ok).count(),
                spot.len()
            );
            if !spot.iter().all(|&ok| ok) {
                eprintln!("[FAIL] patch hash mismatch");
                std::process::exit(1);
            }
        }
        Some("--terrain-bench") => {
            // P3D-201: the surface-extraction bake-off. Measured, not
            // preferred — the table chooses what P3D-202 promotes.
            let rows = pc3d_world::terrain::run_bakeoff();
            println!(
                "{:<14} {:<18} {:>10} {:>8} {:>11} {:>9} {:>8}",
                "scene", "candidate", "extract_us", "bytes", "rebuild_us", "err_m", "cols"
            );
            for r in &rows {
                println!(
                    "{:<14} {:<18} {:>10} {:>8} {:>11} {:>9.3} {:>8}",
                    r.scene, r.candidate, r.extract_us, r.grid_bytes, r.edit_rebuild_us,
                    r.fidelity_err_m, r.fidelity_columns
                );
            }
        }
        Some("--debug-overlay") => {
            // P3D-207: per-patch debug rows + a visual LOD-ring atlas.
            let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
            let half: i32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(16);
            let gen = pc3d_world::WorldGen::new(seed);
            let viewer = pc3d_world::WorldPos::default();
            let rows = pc3d_world::rows_for(&gen, viewer, half, |_| 0, |_| 0);
            let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("shots");
            std::fs::create_dir_all(&out_dir).expect("mkdir shots");
            let out = out_dir.join(format!("debug_overlay_seed{seed}.png"));
            let atlas = pc3d_world::debug_overlay::render_overlay(&gen, viewer, half);
            image::save_buffer(
                &out,
                &atlas.rgb,
                atlas.size as u32,
                atlas.size as u32,
                image::ColorType::Rgb8,
            )
            .expect("encode png");
            println!(
                "[ok] debug overlay seed {seed} -> {} ({}x{} patches)",
                out.display(),
                atlas.size,
                atlas.size
            );
            let mut counts = std::collections::BTreeMap::new();
            for r in &rows {
                *counts.entry(format!("{:?}", r.lod)).or_insert(0usize) += 1;
            }
            for (lod, n) in counts {
                println!("     {lod}: {n} patches");
            }
        }
        Some(other) => {
            eprintln!(
                "unknown argument: {other}\nusage: poorcraft3d [--identity|--format|--baseline|--run [seconds]|--atlas <seed> [half_regions]|--terrain-bench|--debug-overlay <seed>]"
            );
            std::process::exit(2);
        }
    }
}
