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
        Some("--flow-map") => {
            // P3D-304: rivers rendered from flow records — width by
            // discharge, brightness by slope. No particles.
            let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
            let half: i32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(24);
            let atlas = pc3d_world::render_flow_map(seed, half);
            let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("shots");
            std::fs::create_dir_all(&out_dir).expect("mkdir shots");
            let out = out_dir.join(format!("flow_map_seed{seed}.png"));
            image::save_buffer(
                &out,
                &atlas.rgb,
                atlas.size as u32,
                atlas.size as u32,
                image::ColorType::Rgb8,
            )
            .expect("encode png");
            println!("[ok] flow map seed {seed} -> {}", out.display());
        }
        Some("--diagnose") => {
            // P3D-506: the player-diagnosis walk — every shipped system
            // exercised in one deterministic pass, with proof PNGs.
            let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2024);
            let (diagnosis, images) = pc3d_world::run_full_diagnosis(seed);
            for c in &diagnosis.checks {
                let mark = if c.pass { "PASS" } else { "FAIL" };
                println!("  {mark}  {}", c.name);
            }
            let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("shots");
            std::fs::create_dir_all(&out_dir).expect("mkdir shots");
            for (name, atlas) in &images {
                let path = out_dir.join(format!("diagnose_{name}_seed{seed}.png"));
                image::save_buffer(
                    &path,
                    &atlas.rgb,
                    atlas.size as u32,
                    atlas.size as u32,
                    image::ColorType::Rgb8,
                )
                .expect("encode png");
                println!("  PNG: {}", path.display());
            }
            if diagnosis.passed() {
                println!("DIAGNOSIS PASS ({} checks)", diagnosis.checks.len());
            } else {
                let failed: Vec<&str> = diagnosis
                    .checks
                    .iter()
                    .filter(|c| !c.pass)
                    .map(|c| c.name)
                    .collect();
                println!("DIAGNOSIS FAIL: {}", failed.join(", "));
                std::process::exit(1);
            }
        }
        Some("--journey") => {
            // P3D-805: the complete beta player journey, automated.
            let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4242);
            let t0 = std::time::Instant::now();
            let report = pc3d_world::run_journey(seed);
            for s in &report.steps {
                let mark = if s.pass { "PASS" } else { "FAIL" };
                println!("  {mark}  {:<18} {}", s.name, s.detail);
            }
            if report.passed() {
                println!(
                    "JOURNEY PASS ({} steps, digest {:016x}, {:.1?})",
                    report.steps.len(),
                    report.digest,
                    t0.elapsed()
                );
            } else {
                println!("JOURNEY FAIL");
                std::process::exit(1);
            }
        }
        Some("--soak") => {
            // P3D-804: long-running world soak — the integrated host
            // under a continuous command stream, audited at the end.
            let days: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(365);
            let seed: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(80808);
            let t0 = std::time::Instant::now();
            let report = pc3d_world::run_soak(seed, days);
            println!(
                "soak seed {seed} days {days} ticks {} journal {} max_events/tick {} digest {:016x}",
                report.ticks, report.journal_len, report.max_events_per_tick, report.digest
            );
            if report.bounds_ok {
                println!("SOAK PASS ({:.1?})", t0.elapsed());
            } else {
                for v in &report.violations {
                    println!("  VIOLATION: {v}");
                }
                println!("SOAK FAIL");
                std::process::exit(1);
            }
        }
        Some("--play") => {
            // R3DV-002: the interactive 3D renderer. Click to grab the mouse,
            // WASD + Space/Shift to move, Esc quits.
            let cfg = pc3d_render::WindowConfig {
                resize_to: None,
                ..Default::default()
            };
            match pc3d_render::run_windowed(cfg) {
                Ok(report) => print_window_report(&report),
                Err(e) => {
                    eprintln!("[FAIL] windowed renderer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--play-shot") => {
            // R3DV-002 3D proof: open the real window, render at pose A
            // (crimson face), live-resize the window, capture A, walk the
            // camera to pose B (gold face), capture B — then assert the
            // FACE FLIP, the PARALLAX between the two frames, and the resize
            // recovery. A 2D renderer cannot pass this.
            let out = args.get(2).cloned().unwrap_or_else(|| {
                format!("{}/shots/windowed_3d.png", env!("CARGO_MANIFEST_DIR"))
            });
            let out_b = out.replace(".png", "_poseb.png");
            let cfg = pc3d_render::WindowConfig {
                max_frames: Some(45),
                shots: vec![
                    pc3d_render::Shot::new(20, std::path::PathBuf::from(&out)),
                    pc3d_render::Shot::new(40, std::path::PathBuf::from(&out_b.clone())),
                ],
                camera_script: vec![(25, pc3d_render::scene::pose_b())],
                resize_to: Some((800.0, 500.0)),
                ..Default::default()
            };
            match pc3d_render::run_windowed(cfg) {
                Ok(report) => {
                    print_window_report(&report);
                    if report.captures.len() != 2 {
                        eprintln!("[FAIL] expected 2 captures, got {}", report.captures.len());
                        std::process::exit(1);
                    }
                    for cap in &report.captures {
                        if !cap.report.passes() {
                            eprintln!(
                                "[FAIL] capture {} failed verification: {:?}",
                                cap.path.display(),
                                cap.report.failed_probes()
                            );
                            std::process::exit(1);
                        }
                    }
                    // The mid-run resize must have been observed by the
                    // surface and the captures must be at the resized size.
                    let (lw, lh) = (800.0f64, 500.0f64);
                    let expect = (
                        (lw * report.scale_factor).round() as u32,
                        (lh * report.scale_factor).round() as u32,
                    );
                    if report.resizes_observed == 0 {
                        eprintln!("[FAIL] no Resized events reached the surface");
                        std::process::exit(1);
                    }
                    for cap in &report.captures {
                        if cap.report.width != expect.0 || cap.report.height != expect.1 {
                            eprintln!(
                                "[FAIL] capture {}x{} is not the resized surface {}x{}",
                                cap.report.width, cap.report.height, expect.0, expect.1
                            );
                            std::process::exit(1);
                        }
                    }
                    // PARALLAX: pose A and pose B frames must differ widely
                    // (compared over decoded RGBA pixels, not PNG bytes).
                    let diff = pc3d_render::scene::pixel_difference_fraction(
                        &report.captures[0].rgba,
                        &report.captures[1].rgba,
                    );
                    if diff < 0.15 {
                        eprintln!(
                            "[FAIL] camera movement barely changed the image ({:.1}%) — not a 3D render",
                            diff * 100.0
                        );
                        std::process::exit(1);
                    }
                    println!(
                        "RESIZE OK: surface followed {}x{} logical -> {}x{} physical (scale {:.2}, {} events)",
                        lw, lh, expect.0, expect.1, report.scale_factor, report.resizes_observed
                    );
                    println!(
                        "PARALLAX OK: {:.1}% of pixels differ between pose A and pose B",
                        diff * 100.0
                    );
                    println!(
                        "WINDOWED 3D SHOT PASS -> {} + {} ({}x{}, {} distinct colors)",
                        out,
                        out_b,
                        report.captures[0].report.width,
                        report.captures[0].report.height,
                        report.captures[0].report.distinct_colors
                    );
                }
                Err(e) => {
                    eprintln!("[FAIL] windowed renderer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--validate-assets") => {
            // R3DV-003: validate the canonical beta-critical asset manifest
            // (and, if a path is given, that file instead).
            let target = args.get(2).map(std::path::PathBuf::from);
            let result = match &target {
                Some(path) => pc3d_assets::validate_path(path),
                None => pc3d_assets::beta_critical(),
            };
            match result {
                Ok(m) => {
                    let mut counts = std::collections::BTreeMap::new();
                    for a in &m.assets {
                        *counts.entry(a.category.key()).or_insert(0usize) += 1;
                    }
                    let source = target
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "embedded beta_critical_assets.json".into());
                    println!("ASSET MANIFEST PASS: {} rows in {source}", m.assets.len());
                    for (cat, n) in counts {
                        println!("  {cat}: {n}");
                    }
                }
                Err(errs) => {
                    for e in &errs {
                        eprintln!("  ASSET FAIL: {e}");
                    }
                    eprintln!("ASSET MANIFEST FAIL ({} problems)", errs.len());
                    std::process::exit(1);
                }
            }
        }
        Some("--play-build") => {
            // R3DV-004: construction mesh from HOST-owned state.
            //   --play-build live [seed]            interactive: walk, F places
            //                                       a rock block 6 m ahead, R
            //                                       removes it — through the
            //                                       host command path.
            //   --play-build [png] [seed]           automated before/after
            //                                       proof: wall renders,
            //                                       host swaps rock->sand,
            //                                       only that patch remeshes.
            let seed: u64 = args
                .iter()
                .skip(2)
                .filter_map(|s| s.parse().ok())
                .next()
                .unwrap_or(4242);
            let live = args.get(2).map(String::as_str) == Some("live");

            use pc3d_render::{ProbeSet, Shot};
            use pc3d_world::coords::CellCoord;
            use pc3d_world::gen::CellMaterial;
            use pc3d_world::host::{HostCommand, SoloHost};
            use std::cell::RefCell;
            use std::rc::Rc;

            let host = Rc::new(RefCell::new(SoloHost::new(seed)));
            {
                // The starter wall: 5 rock blocks + 3 tops, one patch.
                let mut h = host.borrow_mut();
                for x in 0..=4 {
                    h.submit(HostCommand::Build {
                        cell: CellCoord { x, y: 0, z: -8 },
                        material: CellMaterial::Rock,
                        owner: 7,
                    });
                }
                for x in [0, 2, 4] {
                    h.submit(HostCommand::Build {
                        cell: CellCoord { x, y: 1, z: -8 },
                        material: CellMaterial::Rock,
                        owner: 7,
                    });
                }
                h.run_ticks(1);
            }
            let wall_pose = pc3d_render::CameraPose::new([2.0, 2.2, -2.0], 0.0, -0.18);

            if live {
                println!("live construction mode: click to look, WASD move, F place, R remove, Esc quits");
                let cfg = pc3d_render::WindowConfig {
                    resize_to: None,
                    camera_script: vec![(0, wall_pose)],
                    interactive_host: Some(pc3d_render::InteractiveHost(host)),
                    ..Default::default()
                };
                match pc3d_render::run_windowed(cfg) {
                    Ok(report) => print_window_report(&report),
                    Err(e) => {
                        eprintln!("[FAIL] windowed renderer: {e}");
                        std::process::exit(1);
                    }
                }
                return;
            }

            let base = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| format!("{}/shots/windowed_build.png", env!("CARGO_MANIFEST_DIR")));
            let after_path = base.replace(".png", "_after.png");

            let host_edit = host.clone();
            let edit_stats = Rc::new(RefCell::new(None::<pc3d_render::UpdateStats>));
            let es = edit_stats.clone();
            let host_init = host.clone();
            let init_stats = Rc::new(RefCell::new(None::<pc3d_render::UpdateStats>));
            let is = init_stats.clone();

            let cfg = pc3d_render::WindowConfig {
                max_frames: Some(50),
                probe_set: ProbeSet::SkyOnly,
                resize_to: Some((800.0, 500.0)),
                camera_script: vec![(0, wall_pose)],
                shots: vec![
                    Shot::new(20, std::path::PathBuf::from(&base)),
                    Shot::new(40, std::path::PathBuf::from(&after_path.clone())),
                ],
                frame_hooks: vec![
                    (
                        0,
                        Box::new(move |r: &mut pc3d_render::renderer::Renderer| {
                            r.set_placeholder_scene(false);
                            r.attach_construction();
                            let st = r.update_construction(&host_init.borrow().construction);
                            *is.borrow_mut() = Some(st);
                        }),
                    ),
                    (
                        30,
                        Box::new(move |r: &mut pc3d_render::renderer::Renderer| {
                            let mut h = host_edit.borrow_mut();
                            // Swap the wall's center block rock -> sand
                            // THROUGH THE HOST COMMAND PATH.
                            h.submit(HostCommand::RemoveBuild {
                                cell: CellCoord { x: 2, y: 1, z: -8 },
                                owner: 7,
                            });
                            h.submit(HostCommand::Build {
                                cell: CellCoord { x: 2, y: 1, z: -8 },
                                material: CellMaterial::Sand,
                                owner: 7,
                            });
                            h.run_ticks(1);
                            let st = r.update_construction(&h.construction);
                            *es.borrow_mut() = Some(st);
                        }),
                    ),
                ],
                ..Default::default()
            };
            match pc3d_render::run_windowed(cfg) {
                Ok(report) => {
                    print_window_report(&report);
                    if report.captures.len() != 2 {
                        eprintln!("[FAIL] expected 2 captures, got {}", report.captures.len());
                        std::process::exit(1);
                    }
                    for cap in &report.captures {
                        if !cap.report.passes() {
                            eprintln!(
                                "[FAIL] capture {} failed: {:?}",
                                cap.path.display(),
                                cap.report.failed_probes()
                            );
                            std::process::exit(1);
                        }
                    }
                    let init = init_stats.borrow().expect("init stats");
                    let edit = edit_stats.borrow().expect("edit stats");
                    println!(
                        "REMESH: init added {} patch(es); edit remeshed {} of {} inspected, {} vertices, {} us",
                        init.added, edit.remeshed, edit.inspected, edit.uploaded_vertices, edit.mesh_us
                    );
                    if edit.remeshed != 1 || edit.added != 0 || edit.inspected != 1 {
                        eprintln!("[FAIL] edit not bounded to one patch: {edit:?}");
                        std::process::exit(1);
                    }

                    // The edited cell's pixels must flip rock -> sand while a
                    // control sky pixel stays identical.
                    let (w, h) = (report.captures[0].report.width, report.captures[0].report.height);
                    let aspect = w as f32 / h as f32;
                    let target = pc3d_render::scene::project_ndc(wall_pose, aspect, [2.5, 1.5, -7.0]);
                    let before_px = pc3d_render::scene::sample_ndc(&report.captures[0].rgba, w, h, target);
                    let after_px = pc3d_render::scene::sample_ndc(&report.captures[1].rgba, w, h, target);
                    let delta = (before_px[0] - after_px[0]).abs() + (before_px[1] - after_px[1]).abs();
                    if delta < 0.15 {
                        eprintln!(
                            "[FAIL] edited block barely changed on screen: {before_px:?} vs {after_px:?}"
                        );
                        std::process::exit(1);
                    }
                    let sky = (-0.75f32, 0.55f32);
                    let sb = pc3d_render::scene::sample_ndc(&report.captures[0].rgba, w, h, sky);
                    let sa = pc3d_render::scene::sample_ndc(&report.captures[1].rgba, w, h, sky);
                    for i in 0..3 {
                        if (sb[i] - sa[i]).abs() > 0.02 {
                            eprintln!("[FAIL] control sky pixel changed: {sb:?} vs {sa:?}");
                            std::process::exit(1);
                        }
                    }
                    println!(
                        "VISUAL EDIT OK: block at cell (2,1,-8) flipped rock->sand on screen (delta {delta:.2}); sky control unchanged"
                    );
                    println!(
                        "WINDOWED BUILD PROOF PASS -> {base} + {after_path} ({}x{})",
                        w, h
                    );
                }
                Err(e) => {
                    eprintln!("[FAIL] windowed renderer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--play-terrain") => {
            // R3DV-005: natural terrain from the authoritative query. One
            // windowed run, three scenes (hill, cliff, cave+overhang), each
            // meshed from final_solid over a 3x3 patch neighborhood, captured
            // from the live window and verified with query-derived probes.
            let out_dir = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| format!("{}/shots", env!("CARGO_MANIFEST_DIR")));
            std::fs::create_dir_all(&out_dir).expect("mkdir shots");

            use pc3d_render::terrain::{
                cave_pose, cave_probes, find_cave_pocket_near, neighborhood3, overview_pose,
                vista_probes,
            };
            use pc3d_render::{ProbeSet, Shot};
            use pc3d_world::coords::PatchCoord;
            use pc3d_world::gen::WorldGen;
            use pc3d_world::terrain::SceneSpec;

            let (hill_seed, hill_coord) = SceneSpec::SmoothHills.patch();
            let (cliff_seed, cliff_coord) = SceneSpec::Cliff.patch();
            let (cave_seed, near_coord) = SceneSpec::Highlands.patch();
            let hill_gen = std::rc::Rc::new(WorldGen::new(hill_seed));
            let cliff_gen = std::rc::Rc::new(WorldGen::new(cliff_seed));
            // The cave pocket generator: the highlands seed first, then the
            // hills seed as a wider-ring fallback.
            let cave_gen_h = WorldGen::new(cave_seed);
            let pocket = find_cave_pocket_near(&cave_gen_h, near_coord, 1)
                .or_else(|| find_cave_pocket_near(&WorldGen::new(hill_seed), hill_coord, 2));
            let (cave_air, cave_wall, cave_dir) =
                pocket.expect("a cave pocket near a scene patch");
            let cave_used_hills = find_cave_pocket_near(&cave_gen_h, near_coord, 1).is_none();
            let cave_gen = if cave_used_hills {
                hill_gen.clone()
            } else {
                std::rc::Rc::new(cave_gen_h)
            };

            let hill_pose = overview_pose(&hill_gen, hill_coord);
            let cliff_pose = overview_pose(&cliff_gen, cliff_coord);
            let cave_pose_used = cave_pose(cave_air, cave_wall);

            let hg = hill_gen.clone();
            let cg = cliff_gen.clone();
            let kg = cave_gen.clone();
            let stats_rc =
                std::rc::Rc::new(std::cell::RefCell::new(Vec::<(String, pc3d_render::TerrainStats)>::new()));
            let st0 = stats_rc.clone();
            let st1 = stats_rc.clone();
            let st2 = stats_rc.clone();

            let cfg = pc3d_render::WindowConfig {
                title: "POORCRAFT 3D — terrain".into(),
                max_frames: Some(115),
                probe_set: ProbeSet::SkyOnly,
                resize_to: Some((800.0, 500.0)),
                camera_script: vec![(0, hill_pose)],
                shots: vec![
                    Shot::new(30, format!("{out_dir}/windowed_terrain_hills.png")),
                    Shot::new(70, format!("{out_dir}/windowed_terrain_cliff.png")),
                    Shot::new(110, format!("{out_dir}/windowed_terrain_cave.png")),
                ],
                frame_hooks: vec![
                    (
                        0,
                        Box::new(move |r: &mut pc3d_render::renderer::Renderer| {
                            r.set_placeholder_scene(false);
                            let st = r.load_terrain(&hg, &neighborhood3(hill_coord));
                            r.set_pose(hill_pose);
                            st0.borrow_mut().push(("hills".into(), st));
                        }),
                    ),
                    (
                        40,
                        Box::new(move |r: &mut pc3d_render::renderer::Renderer| {
                            let st = r.load_terrain(&cg, &neighborhood3(cliff_coord));
                            r.set_pose(cliff_pose);
                            st1.borrow_mut().push(("cliff".into(), st));
                        }),
                    ),
                    (
                        80,
                        Box::new(move |r: &mut pc3d_render::renderer::Renderer| {
                            let cave_patch = PatchCoord {
                                x: cave_air.x.div_euclid(16),
                                y: cave_air.y.div_euclid(16),
                                z: cave_air.z.div_euclid(16),
                            };
                            let st = r.load_terrain(&kg, &neighborhood3(cave_patch));
                            r.set_pose(cave_pose_used);
                            st2.borrow_mut().push(("cave".into(), st));
                        }),
                    ),
                ],
                ..Default::default()
            };
            match pc3d_render::run_windowed(cfg) {
                Ok(report) => {
                    print_window_report(&report);
                    if report.captures.len() != 3 {
                        eprintln!("[FAIL] expected 3 captures, got {}", report.captures.len());
                        std::process::exit(1);
                    }
                    for (name, st) in stats_rc.borrow().iter() {
                        println!(
                            "TERRAIN {name}: {} patches, {} verts, {} tris, {} ms mesh",
                            st.patches, st.vertices, st.triangles, st.mesh_us / 1000
                        );
                    }
                    let (w, h) = (
                        report.captures[0].report.width,
                        report.captures[0].report.height,
                    );
                    let aspect = w as f32 / h as f32;
                    let hill_probes = vista_probes(&hill_gen, hill_coord, hill_pose, aspect);
                    let cliff_probes = vista_probes(&cliff_gen, cliff_coord, cliff_pose, aspect);
                    let cave_probes = cave_probes(
                        &cave_gen,
                        cave_air,
                        cave_wall,
                        cave_dir,
                        cave_pose_used,
                        aspect,
                    );
                    let checks: [(String, usize, Vec<pc3d_render::scene::Probe>); 3] = [
                        ("hills".into(), 12, hill_probes),
                        ("cliff".into(), 12, cliff_probes),
                        ("cave".into(), 4, cave_probes),
                    ];
                    for (i, (name, min_distinct, probes)) in checks.iter().enumerate() {
                        let rgba = &report.captures[i].rgba;
                        let verified = pc3d_render::scene::verify_frame_rgba(rgba, w, h, probes);
                        if !verified.passes_with(*min_distinct) {
                            eprintln!(
                                "[FAIL] {name} terrain probes: {:?} (report {verified:?})",
                                verified.failed_probes()
                            );
                            std::process::exit(1);
                        }
                        println!(
                            "TERRAIN {name} PROBES PASS ({} checks, {} distinct colors) -> {}",
                            verified.probes.len(),
                            verified.distinct_colors,
                            report.captures[i].path.display()
                        );
                    }
                    println!("WINDOWED TERRAIN PROOF PASS ({w}x{h})");
                }
                Err(e) => {
                    eprintln!("[FAIL] windowed renderer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--play-stream") => {
            // R3DV-006: streamed terrain LOD walk. The world's own interest
            // rings + LOD bands drive bounded per-frame meshing/uploads with
            // a GPU budget, eviction, and frustum culling; three waypoints
            // walk across the rings with live-window captures.
            let out_dir = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| format!("{}/shots", env!("CARGO_MANIFEST_DIR")));
            std::fs::create_dir_all(&out_dir).expect("mkdir shots");

            use pc3d_render::{ProbeSet, Shot, StreamConfig};
            use pc3d_world::gen::WorldGen;
            use pc3d_world::stream::Tier;
            use pc3d_world::terrain::SceneSpec;

            let (seed, hill_coord) = SceneSpec::SmoothHills.patch();
            let gen = std::rc::Rc::new(WorldGen::new(seed));
            let o = hill_coord.origin();
            let cx = o.x as f32 / 1000.0 + 8.0;
            let cz = o.z as f32 / 1000.0 + 8.0;
            let surface_at = |x: f32| {
                gen.effective_surface_mm((x * 1000.0) as i64, (cz * 1000.0) as i64) as f32
                    / 1000.0
            };
            let pose_at = |x: f32| {
                let s = surface_at(x);
                pc3d_render::CameraPose::new([x, s + 1.7, cz], 0.0, -0.08)
            };
            let waypoints = [cx, cx + 96.0, cx + 192.0];
            let poses: Vec<_> = waypoints.iter().map(|x| pose_at(*x)).collect();

            let gen_hook = gen.clone();
            let cfg = pc3d_render::WindowConfig {
                title: "POORCRAFT 3D — streamed terrain".into(),
                max_frames: Some(360),
                probe_set: ProbeSet::SkyOnly,
                resize_to: Some((800.0, 500.0)),
                camera_script: vec![(0, poses[0]), (120, poses[1]), (240, poses[2])],
                shots: vec![
                    Shot::new(110, format!("{out_dir}/windowed_stream_walk1.png")),
                    Shot::new(230, format!("{out_dir}/windowed_stream_walk2.png")),
                    Shot::new(350, format!("{out_dir}/windowed_stream_walk3.png")),
                ],
                frame_hooks: vec![(
                    0,
                    Box::new(move |r: &mut pc3d_render::renderer::Renderer| {
                        r.set_placeholder_scene(false);
                        r.attach_streaming(
                            gen_hook.clone(),
                            StreamConfig {
                                max_mesh_per_frame: 3,
                                max_uploads_per_frame: 6,
                                gpu_byte_budget: 24 * 1024 * 1024,
                                tiers: &[Tier::Full, Tier::Lod],
                            },
                            1,
                        );
                    }),
                )],
                ..Default::default()
            };
            match pc3d_render::run_windowed(cfg) {
                Ok(report) => {
                    print_window_report(&report);
                    if report.captures.len() != 3 {
                        eprintln!("[FAIL] expected 3 captures, got {}", report.captures.len());
                        std::process::exit(1);
                    }
                    let c = report
                        .final_stream_counters
                        .expect("streaming counters");
                    println!(
                        "STREAM: loaded {} (full {} / mid {}), meshed {} in {} ms total, gpu {} KB of {} KB budget",
                        c.loaded,
                        c.loaded_full,
                        c.loaded_mid,
                        c.meshed,
                        c.mesh_us_total / 1000,
                        c.gpu_bytes / 1024,
                        24 * 1024
                    );
                    println!(
                        "STREAM bounds: max mesh/frame {} (cap 3), max uploads/frame {} (cap 6), deferred held {}, queue rejected {}",
                        c.max_mesh_per_frame_seen,
                        c.max_uploads_per_frame_seen,
                        c.deferred,
                        c.queue_rejected
                    );
                    println!(
                        "STREAM culling: {} draws, {} frustum-culled patches",
                        c.drawn_patches, c.frustum_culled
                    );
                    if c.max_mesh_per_frame_seen > 3 || c.max_uploads_per_frame_seen > 6 {
                        eprintln!("[FAIL] per-frame budget violated");
                        std::process::exit(1);
                    }
                    if c.gpu_bytes > 24 * 1024 * 1024 {
                        eprintln!("[FAIL] gpu budget violated");
                        std::process::exit(1);
                    }
                    if c.loaded_full == 0 || c.loaded_mid == 0 {
                        eprintln!("[FAIL] LOD rings not populated");
                        std::process::exit(1);
                    }
                    if c.frustum_culled == 0 {
                        eprintln!("[FAIL] frustum culling never culled");
                        std::process::exit(1);
                    }
                    // Query-derived ground probe per waypoint capture.
                    let (w, h) = (
                        report.captures[0].report.width,
                        report.captures[0].report.height,
                    );
                    let aspect = w as f32 / h as f32;
                    for (i, pose) in poses.iter().enumerate() {
                        // Probe what the view-center ray actually hits
                        // (occlusion-proof: expectation from the query).
                        let probe = pc3d_render::terrain::probe_view_center(&gen, *pose, aspect)
                            .expect("the view must hit terrain within 300 m");
                        let rgba = &report.captures[i].rgba;
                        let verified = pc3d_render::scene::verify_frame_rgba(rgba, w, h, &[probe]);
                        if !verified.passes_with(8) {
                            eprintln!(
                                "[FAIL] waypoint {} ground probe: {:?} ({verified:?})",
                                i + 1,
                                verified.failed_probes()
                            );
                            std::process::exit(1);
                        }
                        println!(
                            "STREAM waypoint {} PROBE PASS -> {}",
                            i + 1,
                            report.captures[i].path.display()
                        );
                    }
                    println!("WINDOWED STREAM WALK PASS ({w}x{h})");
                }
                Err(e) => {
                    eprintln!("[FAIL] windowed renderer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--play-water") => {
            // R3DV-007: river water from flow records. Windowed before/
            // after the dam edit: transparent water in perspective with the
            // record-driven current, then a local channel edit that
            // remeshes only the dirty sections.
            let out_dir = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| format!("{}/shots", env!("CARGO_MANIFEST_DIR")));
            let seed: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3);
            std::fs::create_dir_all(&out_dir).expect("mkdir shots");

            use pc3d_render::water::{proof_scene, river_pose};
            use pc3d_render::{ProbeSet, Shot};
            use pc3d_world::flow::FlowTable;
            use pc3d_world::hydro::RiverGraph;

            let (gen, graph, t0, (a, b)) = proof_scene(seed);
            let graph = std::rc::Rc::new(graph);
            let t0 = std::rc::Rc::new(t0);
            let gen = std::rc::Rc::new(gen);
            let mid_x = ((a.0 as f32 + 0.5) + (b.0 as f32 + 0.5)) / 2.0 * 256.0;
            let mid_z = ((a.1 as f32 + 0.5) + (b.1 as f32 + 0.5)) / 2.0 * 256.0;
            let mid_y = gen
                .effective_surface_mm((mid_x * 1000.0) as i64, (mid_z * 1000.0) as i64)
                as f32
                / 1000.0;
            let mid_patch = pc3d_world::coords::PatchCoord {
                x: (mid_x as i32).div_euclid(16),
                y: (mid_y as i32).div_euclid(16),
                z: (mid_z as i32).div_euclid(16),
            };
            let pose = river_pose(&graph, &gen, a, b);
            let graph0 = graph.clone();
            let table0 = t0.clone();
            let graph_edit = graph.clone();
            let table_edit = t0.clone();

            let g0 = gen.clone();
            let g_edit = gen.clone();
            let stats_rc = std::rc::Rc::new(std::cell::RefCell::new(Vec::<(
                String,
                pc3d_render::WaterStats,
            )>::new()));
            let st0 = stats_rc.clone();
            let st1 = stats_rc.clone();

            let cfg = pc3d_render::WindowConfig {
                title: "POORCRAFT 3D — river water".into(),
                max_frames: Some(60),
                probe_set: ProbeSet::SkyOnly,
                resize_to: Some((800.0, 500.0)),
                camera_script: vec![(0, pose)],
                shots: vec![
                    Shot::new(25, format!("{out_dir}/windowed_water_before.png")),
                    Shot::new(55, format!("{out_dir}/windowed_water_after.png")),
                ],
                frame_hooks: vec![
                    (
                        0,
                        Box::new(move |r: &mut pc3d_render::renderer::Renderer| {
                            r.set_placeholder_scene(false);
                            r.load_terrain(&g0, &pc3d_render::terrain::neighborhood3(mid_patch));
                            r.attach_water();
                            let st = r.update_water(&g0, &graph0, &table0);
                            st0.borrow_mut().push(("before".into(), st));
                            r.set_water_time(Some(0.0));
                            r.set_pose(pose);
                        }),
                    ),
                    (
                        35,
                        Box::new(move |r: &mut pc3d_render::renderer::Renderer| {
                            // DAM the downstream region: elevation override
                            // reroutes the river locally (P3D-303), the
                            // dirty-region table bumps only changed records,
                            // and only those sections remesh.
                            let mut overrides = std::collections::BTreeMap::new();
                            overrides.insert(b, 200);
                            let g1 = RiverGraph::build(&g_edit, graph_edit.half, &overrides);
                            let t1 = FlowTable::from_graph_with_revisions(Some(&table_edit), &g1);
                            let st = r.update_water(&g_edit, &g1, &t1);
                            st1.borrow_mut().push(("after_dam".into(), st));
                        }),
                    ),
                ],
                ..Default::default()
            };
            match pc3d_render::run_windowed(cfg) {
                Ok(report) => {
                    print_window_report(&report);
                    if report.captures.len() != 2 {
                        eprintln!("[FAIL] expected 2 captures, got {}", report.captures.len());
                        std::process::exit(1);
                    }
                    for cap in &report.captures {
                        if !cap.report.passes_with(8) {
                            eprintln!(
                                "[FAIL] capture {}: {:?}",
                                cap.path.display(),
                                cap.report.failed_probes()
                            );
                            std::process::exit(1);
                        }
                    }
                    for (name, st) in stats_rc.borrow().iter() {
                        println!(
                            "WATER {name}: {} sections, {} added, {} remeshed, {} verts, {} us",
                            st.sections, st.added, st.remeshed, st.vertices, st.mesh_us
                        );
                    }
                    let before = &stats_rc.borrow()[0].1;
                    let after = &stats_rc.borrow()[1].1;
                    if after.remeshed == 0 || after.remeshed >= before.sections {
                        eprintln!("[FAIL] dam edit did not stay local: {after:?}");
                        std::process::exit(1);
                    }
                    let diff = pc3d_render::scene::pixel_difference_fraction(
                        &report.captures[0].rgba,
                        &report.captures[1].rgba,
                    );
                    if diff < 0.005 {
                        eprintln!("[FAIL] the dam must change the image ({diff})");
                        std::process::exit(1);
                    }
                    println!(
                        "DAM EDIT OK: {}/{} sections remeshed, image changed {:.2}%",
                        after.remeshed,
                        before.sections,
                        diff * 100.0
                    );
                    println!(
                        "WINDOWED WATER PROOF PASS -> {} + {}",
                        report.captures[0].path.display(),
                        report.captures[1].path.display()
                    );
                }
                Err(e) => {
                    eprintln!("[FAIL] windowed renderer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--play-city") => {
            // R3DV-008: castle/city modules from the placement authorities.
            // Windowed captures: the capital + town overview, then the gate
            // close-up (a real opening between the pillars).
            let out_dir = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| format!("{}/shots", env!("CARGO_MANIFEST_DIR")));
            let seed: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3);
            std::fs::create_dir_all(&out_dir).expect("mkdir shots");

            use pc3d_render::city::{city_scene, mesh_city};
            use pc3d_render::{ProbeSet, Shot};
            use pc3d_world::coords::RegionCoord;

            let (gen, _center, layout, plan) = city_scene(seed, RegionCoord { x: 0, z: 0 });
            let gen = std::rc::Rc::new(gen);
            let (cverts, cidx, info) = mesh_city(&gen, &layout, &plan);
            for kind in ["gatehouse", "wall", "tower", "home", "workshop"] {
                assert!(
                    info.kind_present(kind),
                    "silhouette {kind} missing before launch"
                );
            }
            println!(
                "CITY: {} verts / {} tris across {} kinds; {} collision cells, {} nav anchors",
                info.vertices,
                info.triangles,
                info.tris_by_kind.len(),
                info.collision_cells.len(),
                info.nav_anchors.len()
            );

            // Terrain under everything (city bounds + margin, 3 y levels).
            let mut patches = Vec::new();
            let pmin = (
                (info.bounds_min[0] as i32).div_euclid(16) - 1,
                (info.bounds_min[2] as i32).div_euclid(16) - 1,
            );
            let pmax = (
                (info.bounds_max[0] as i32).div_euclid(16) + 1,
                (info.bounds_max[2] as i32).div_euclid(16) + 1,
            );
            let y_level = ((info.bounds_min[1] + 2.0) as i32).div_euclid(16).max(0);
            for px in pmin.0..=pmax.0 {
                for pz in pmin.1..=pmax.1 {
                    for py in (y_level - 1)..=(y_level + 1) {
                        patches.push(pc3d_world::coords::PatchCoord { x: px, y: py, z: pz });
                    }
                }
            }
            let span = (info.bounds_max[0] - info.bounds_min[0])
                .max(info.bounds_max[2] - info.bounds_min[2])
                .max(16.0);
            let overview = pc3d_render::CameraPose::new(
                [
                    (info.bounds_min[0] + info.bounds_max[0]) / 2.0,
                    (info.bounds_min[1] + info.bounds_max[1]) / 2.0 + span * 0.9,
                    (info.bounds_min[2] + info.bounds_max[2]) / 2.0 + span * 0.8,
                ],
                0.0,
                (-0.75f32).atan2(1.3),
            );
            // Gate close-up (eye between the gate and its market).
            let gate = layout
                .modules
                .iter()
                .find(|m| m.kind == pc3d_world::castle::ModuleKind::GateHouse)
                .expect("gatehouse");
            let gate_base = (0..2)
                .flat_map(|dz| (0..3).map(move |dx| (dx, dz)))
                .map(|(dx, dz)| {
                    gen.effective_surface_mm(
                        (gate.origin.x + dx) as i64 * 1000,
                        (gate.origin.z + dz) as i64 * 1000,
                    ) as f32
                        / 1000.0
                })
                .fold(f32::MAX, f32::min);
            let mut gate_eye = [
                gate.origin.x as f32 + 1.5,
                gate_base + 1.9,
                gate.origin.z as f32 + 2.4,
            ];
            let gate_surf = gen
                .effective_surface_mm((gate_eye[0] * 1000.0) as i64, (gate_eye[2] * 1000.0) as i64)
                as f32
                / 1000.0;
            gate_eye[1] = gate_eye[1].max(gate_surf + 0.5);
            let arch_mid = [
                gate.origin.x as f32 + 1.5,
                gate_base + 2.0,
                gate.origin.z as f32 + 1.0,
            ];
            let gd = [
                arch_mid[0] - gate_eye[0],
                arch_mid[1] - gate_eye[1],
                arch_mid[2] - gate_eye[2],
            ];
            let gate_pose = pc3d_render::CameraPose::new(
                gate_eye,
                (-gd[0]).atan2(-gd[2]),
                (gd[1] / (gd[0] * gd[0] + gd[1] * gd[1] + gd[2] * gd[2]).sqrt()).asin(),
            );

            let g = gen.clone();
            let cv = cverts.clone();
            let ci = cidx.clone();
            let patches_hook = patches.clone();
            let cfg = pc3d_render::WindowConfig {
                title: "POORCRAFT 3D — city".into(),
                max_frames: Some(70),
                probe_set: ProbeSet::SkyOnly,
                resize_to: Some((800.0, 500.0)),
                camera_script: vec![(0, overview)],
                shots: vec![
                    Shot::new(25, format!("{out_dir}/windowed_city.png")),
                    Shot::new(55, format!("{out_dir}/windowed_city_gate.png"))
                        .sky(false),
                ],
                frame_hooks: vec![
                    (
                        0,
                        Box::new(move |r: &mut pc3d_render::renderer::Renderer| {
                            r.set_placeholder_scene(false);
                            let t0 = std::time::Instant::now();
                            r.load_terrain(&g, &patches_hook);
                            println!(
                                "TERRAIN under city: {} patches in {} ms",
                                patches_hook.len(),
                                t0.elapsed().as_millis()
                            );
                            r.load_city(&cv, &ci);
                            r.set_pose(overview);
                        }),
                    ),
                    (30, Box::new(move |r| r.set_pose(gate_pose))),
                ],
                ..Default::default()
            };
            match pc3d_render::run_windowed(cfg) {
                Ok(report) => {
                    print_window_report(&report);
                    if report.captures.len() != 2 {
                        eprintln!("[FAIL] expected 2 captures, got {}", report.captures.len());
                        std::process::exit(1);
                    }
                    for cap in &report.captures {
                        if !cap.report.passes_with(4) {
                            eprintln!(
                                "[FAIL] capture {}: {:?}",
                                cap.path.display(),
                                cap.report.failed_probes()
                            );
                            std::process::exit(1);
                        }
                    }
                    // The gate close-up: opening differs from pillar stone.
                    let (w, h) = (
                        report.captures[1].report.width,
                        report.captures[1].report.height,
                    );
                    let aspect = w as f32 / h as f32;
                    let arch_ndc =
                        pc3d_render::scene::project_ndc(gate_pose, aspect, arch_mid);
                    let pillar = [
                        gate.origin.x as f32 + 0.95,
                        gate_base + 2.0,
                        gate.origin.z as f32 + 1.0,
                    ];
                    let pillar_ndc = pc3d_render::scene::project_ndc(gate_pose, aspect, pillar);
                    let arch_px =
                        pc3d_render::scene::sample_ndc(&report.captures[1].rgba, w, h, arch_ndc);
                    let pillar_px =
                        pc3d_render::scene::sample_ndc(&report.captures[1].rgba, w, h, pillar_ndc);
                    let delta: f32 = (0..3).map(|i| (arch_px[i] - pillar_px[i]).abs()).sum();
                    if delta < 0.08 {
                        eprintln!(
                            "[FAIL] gate opening not open: {arch_px:?} vs {pillar_px:?}"
                        );
                        std::process::exit(1);
                    }
                    println!("GATE OK: opening/pillar delta {delta:.2}");
                    println!(
                        "WINDOWED CITY PROOF PASS -> {} + {}",
                        report.captures[0].path.display(),
                        report.captures[1].path.display()
                    );
                }
                Err(e) => {
                    eprintln!("[FAIL] windowed renderer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--play-npcs") => {
            // R3DV-009: NPCs at their simulated positions + Bed/Work/Idle
            // inspect boxes. Captures: town overview with the cast, one
            // close-up per NPC (resident/worker/guard), then the inspect
            // overview with anchor boxes.
            let out_dir = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| format!("{}/shots", env!("CARGO_MANIFEST_DIR")));
            let seed: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3);
            std::fs::create_dir_all(&out_dir).expect("mkdir shots");

            use pc3d_render::city::{city_scene, mesh_city};
            use pc3d_render::npcs::{
                advance, cast_for, mesh_anchor_boxes, mesh_npc, npc_world_pos,
            };
            use pc3d_render::{ProbeSet, Shot};
            use pc3d_world::coords::RegionCoord;
            use pc3d_world::nav::NavPatch;

            let (gen, _c, layout, plan) = city_scene(seed, RegionCoord { x: 0, z: 0 });
            let gen = std::rc::Rc::new(gen);
            let (cverts, cidx, info) = mesh_city(&gen, &layout, &plan);
            let plaza_patch = pc3d_world::coords::PatchCoord {
                x: plan.plaza.x.div_euclid(16),
                y: 0,
                z: plan.plaza.z.div_euclid(16),
            };
            let nav = NavPatch::from_gen(&gen, plaza_patch);
            let mut cast = cast_for(&plan, &info);
            advance(&mut cast, &nav, 0.5, 200);
            for c in &cast {
                println!(
                    "NPC {:<8} pos {:?} intent {:?}",
                    c.label, c.brain.pos, c.brain.intent
                );
            }

            let (mut nverts, mut nidx) = (Vec::new(), Vec::new());
            for c in &cast {
                mesh_npc(&gen, c, &mut nverts, &mut nidx);
            }
            let (mut bverts, mut bidx) = (nverts.clone(), nidx.clone());
            mesh_anchor_boxes(&gen, &plan, &mut bverts, &mut bidx);

            let mut patches = Vec::new();
            let pmin = (
                (info.bounds_min[0] as i32).div_euclid(16) - 1,
                (info.bounds_min[2] as i32).div_euclid(16) - 1,
            );
            let pmax = (
                (info.bounds_max[0] as i32).div_euclid(16) + 1,
                (info.bounds_max[2] as i32).div_euclid(16) + 1,
            );
            let y_level = ((info.bounds_min[1] + 2.0) as i32).div_euclid(16).max(0);
            for px in pmin.0..=pmax.0 {
                for pz in pmin.1..=pmax.1 {
                    for py in (y_level - 1)..=(y_level + 1) {
                        patches.push(pc3d_world::coords::PatchCoord { x: px, y: py, z: pz });
                    }
                }
            }
            let town_c = plan.plaza;
            let town_surf = gen
                .effective_surface_mm(town_c.x as i64 * 1000, town_c.z as i64 * 1000)
                as f32
                / 1000.0;
            let overview = pc3d_render::CameraPose::new(
                [town_c.x as f32 + 0.5, town_surf + 26.0, town_c.z as f32 + 22.0],
                0.0,
                (-0.75f32).atan2(1.3),
            );

            // Per-NPC close-up poses (eye due south of the sim position).
            let closeups: Vec<(usize, pc3d_render::CameraPose, [f32; 3])> = cast
                .iter()
                .map(|c| {
                    let base = npc_world_pos(&gen, &c.brain);
                    let face = [base[0], base[1] + 1.05, base[2] + 0.14];
                    let eye = [base[0], base[1] + 1.25, base[2] + 3.0];
                    let d = [
                        face[0] - eye[0],
                        face[1] - eye[1],
                        face[2] - eye[2],
                    ];
                    (
                        0usize,
                        pc3d_render::CameraPose::new(
                            eye,
                            (-d[0]).atan2(-d[2]),
                            (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin(),
                        ),
                        base,
                    )
                })
                .collect();

            let g = gen.clone();
            let cv = cverts.clone();
            let ci = cidx.clone();
            let nv = nverts.clone();
            let ni = nidx.clone();
            let bv = bverts.clone();
            let bi = bidx.clone();
            let cu = closeups.clone();
            let cu2 = closeups.clone();
            let cu3 = closeups.clone();

            let cfg = pc3d_render::WindowConfig {
                title: "POORCRAFT 3D — npcs".into(),
                max_frames: Some(90),
                probe_set: ProbeSet::SkyOnly,
                resize_to: Some((800.0, 500.0)),
                camera_script: vec![(0, overview)],
                shots: vec![
                    Shot::new(20, format!("{out_dir}/windowed_npcs_town.png")),
                    Shot::new(35, format!("{out_dir}/windowed_npcs_resident.png")),
                    Shot::new(50, format!("{out_dir}/windowed_npcs_worker.png")),
                    Shot::new(65, format!("{out_dir}/windowed_npcs_guard.png")),
                    Shot::new(85, format!("{out_dir}/windowed_npcs_inspect.png")),
                ],
                frame_hooks: vec![
                    (
                        0,
                        Box::new(move |r: &mut pc3d_render::renderer::Renderer| {
                            r.set_placeholder_scene(false);
                            r.load_terrain(&g, &patches);
                            r.load_city(&cv, &ci);
                            r.load_npcs(&nv, &ni);
                            r.set_pose(overview);
                        }),
                    ),
                    (25, Box::new(move |r| r.set_pose(cu[0].1))),
                    (40, Box::new(move |r| r.set_pose(cu2[1].1))),
                    (55, Box::new(move |r| r.set_pose(cu3[2].1))),
                    (
                        70,
                        Box::new(move |r: &mut pc3d_render::renderer::Renderer| {
                            // Inspect mode: anchor boxes join the render.
                            r.load_npcs(&bv, &bi);
                            r.set_pose(overview);
                        }),
                    ),
                ],
                ..Default::default()
            };
            match pc3d_render::run_windowed(cfg) {
                Ok(report) => {
                    print_window_report(&report);
                    if report.captures.len() != 5 {
                        eprintln!("[FAIL] expected 5 captures, got {}", report.captures.len());
                        std::process::exit(1);
                    }
                    for cap in &report.captures {
                        if !cap.report.passes_with(4) {
                            eprintln!(
                                "[FAIL] capture {}: {:?}",
                                cap.path.display(),
                                cap.report.failed_probes()
                            );
                            std::process::exit(1);
                        }
                    }
                    // Each close-up must show SOMETHING at the torso point
                    // (not the dawn sky behind an empty spot).
                    let (w, h) = (
                        report.captures[1].report.width,
                        report.captures[1].report.height,
                    );
                    let aspect = w as f32 / h as f32;
                    let sky = |ndc: (f32, f32)| {
                        pc3d_render::scene::to_srgb4(pc3d_render::scene::sky_color_linear(
                            pc3d_render::scene::dir_from_ndc(
                                pc3d_render::CameraPose::new([0.0; 3], 0.0, 0.0),
                                ndc,
                                aspect,
                            ),
                            pc3d_render::scene::SUN_DIR,
                        ))
                    };
                    for (i, cap) in report.captures[1..4].iter().enumerate() {
                        let ndc = pc3d_render::scene::project_ndc(
                            closeups[i].1,
                            aspect,
                            [closeups[i].2[0], closeups[i].2[1] + 1.05, closeups[i].2[2] + 0.14],
                        );
                        let px = pc3d_render::scene::sample_ndc(&cap.rgba, w, h, ndc);
                        let s = sky(ndc);
                        let delta: f32 = (0..3).map(|k| (px[k] - s[k]).abs()).sum();
                        if delta < 0.05 {
                            eprintln!(
                                "[FAIL] NPC {i} close-up shows sky at the torso point ({px:?})"
                            );
                            std::process::exit(1);
                        }
                        println!("NPC {i} close-up: torso point occupied (sky-delta {delta:.2})");
                    }
                    println!(
                        "WINDOWED NPC PROOF PASS -> {} (+4 more)",
                        report.captures[0].path.display()
                    );
                }
                Err(e) => {
                    eprintln!("[FAIL] windowed renderer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--play-quality") => {
            // R3DV-010: the SAME scene at Low/Mid/High tiers — three
            // windowed captures + the memory/frame record per tier.
            let out_dir = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| format!("{}/shots", env!("CARGO_MANIFEST_DIR")));
            let seed: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3);
            std::fs::create_dir_all(&out_dir).expect("mkdir shots");

            use pc3d_render::water::proof_scene;
            use pc3d_render::{QualityTier, Shot, StreamConfig};

            // Scene: the water proof river + the sim-sited water wheel.
            let (gen, graph, t0, _edge) = proof_scene(seed);
            let gen = std::rc::Rc::new(gen);
            let (mut wverts, mut widx) = (Vec::new(), Vec::new());
            let hub = pc3d_render::mesh_water_wheel(&gen, &graph, &mut wverts, &mut widx)
                .expect("a wheel site");
            let eye = [hub[0] + 14.0, hub[1] + 9.0, hub[2] + 18.0];
            let d = [hub[0] - eye[0], hub[1] - eye[1], hub[2] - eye[2]];
            let pose = pc3d_render::CameraPose::new(
                eye,
                (-d[0]).atan2(-d[2]),
                (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin(),
            );
            let y_level = (hub[1] as i32).div_euclid(16).max(0);

            let tiers: [(QualityTier, &'static str); 3] = [
                (QualityTier::Low, "low"),
                (QualityTier::Mid, "mid"),
                (QualityTier::High, "high"),
            ];
            let setup = move |r: &mut pc3d_render::renderer::Renderer,
                              tier: QualityTier,
                              gen: &std::rc::Rc<pc3d_world::gen::WorldGen>,
                              graph: &pc3d_world::hydro::RiverGraph,
                              t0: &pc3d_world::flow::FlowTable,
                              wv: &[pc3d_render::scene::SceneVertex],
                              wi: &[u16]| {
                r.set_placeholder_scene(false);
                r.set_quality(tier);
                r.attach_streaming(
                    gen.clone(),
                    StreamConfig {
                        max_mesh_per_frame: tier.max_mesh_per_frame(),
                        max_uploads_per_frame: tier.max_mesh_per_frame() * 2,
                        gpu_byte_budget: tier.gpu_byte_budget(),
                        tiers: tier.tiers(),
                    },
                    y_level,
                );
                r.attach_water();
                let _ = r.update_water(gen, graph, t0);
                r.load_npcs(wv, wi);
                r.set_pose(pose);
            };

            let shots: Vec<Shot> = tiers
                .iter()
                .enumerate()
                .map(|(i, (_, name))| {
                    Shot::new(35 + (i as u64) * 40, format!("{out_dir}/windowed_quality_{name}.png"))
                })
                .collect();

            let g0 = gen.clone();
            let g1 = gen.clone();
            let g2 = gen.clone();
            let gr0 = graph.clone();
            let gr1 = graph.clone();
            let gr2 = graph.clone();
            let t00 = t0.clone();
            let t01 = t0.clone();
            let t02 = t0.clone();
            let wv0 = wverts.clone();
            let wv1 = wverts.clone();
            let wv2 = wverts.clone();
            let wi0 = widx.clone();
            let wi1 = widx.clone();
            let wi2 = widx.clone();
            let records = std::rc::Rc::new(std::cell::RefCell::new(Vec::<(
                &'static str,
                pc3d_render::StreamCounters,
            )>::new()));
            let rec_low = records.clone();
            let rec_mid = records.clone();

            let cfg = pc3d_render::WindowConfig {
                title: "POORCRAFT 3D — quality tiers".into(),
                max_frames: Some(125),
                probe_set: pc3d_render::ProbeSet::SkyOnly,
                resize_to: None,
                camera_script: vec![],
                shots,
                frame_hooks: vec![
                    (
                        0,
                        Box::new(move |r: &mut pc3d_render::renderer::Renderer| {
                            setup(r, QualityTier::Low, &g0, &gr0, &t00, &wv0, &wi0);
                        }),
                    ),
                    (
                        42,
                        Box::new(move |r: &mut pc3d_render::renderer::Renderer| {
                            if let Some(c) = r.stream_counters() {
                                rec_low.borrow_mut().push(("low", c));
                            }
                            setup(r, QualityTier::Mid, &g1, &gr1, &t01, &wv1, &wi1);
                        }),
                    ),
                    (
                        82,
                        Box::new(move |r: &mut pc3d_render::renderer::Renderer| {
                            if let Some(c) = r.stream_counters() {
                                rec_mid.borrow_mut().push(("mid", c));
                            }
                            setup(r, QualityTier::High, &g2, &gr2, &t02, &wv2, &wi2);
                        }),
                    ),
                ],
                ..Default::default()
            };
            match pc3d_render::run_windowed(cfg) {
                Ok(report) => {
                    print_window_report(&report);
                    if report.captures.len() != 3 {
                        eprintln!("[FAIL] expected 3 captures, got {}", report.captures.len());
                        std::process::exit(1);
                    }
                    for (i, cap) in report.captures.iter().enumerate() {
                        if !cap.report.passes_with(8) {
                            eprintln!(
                                "[FAIL] {} capture: {:?}",
                                tiers[i].1,
                                cap.report.failed_probes()
                            );
                            std::process::exit(1);
                        }
                    }
                    let high_counters = report.final_stream_counters.unwrap_or_default();
                    let mut rows: Vec<(u64, String, String, String)> = Vec::new();
                    for (name, c) in records.borrow().iter() {
                        rows.push((
                            c.loaded as u64,
                            format!("{}", c.gpu_bytes / 1024),
                            format!("{}", c.loaded_full),
                            format!("{}", c.loaded_mid + c.loaded_far),
                        ));
                        println!(
                            "QUALITY {name}: loaded {} patches (full {} / far {}) gpu {} KB meshed {}",
                            c.loaded, c.loaded_full, c.loaded_mid + c.loaded_far, c.gpu_bytes / 1024, c.meshed
                        );
                    }
                    println!(
                        "QUALITY high: loaded {} patches gpu {} KB",
                        high_counters.loaded,
                        high_counters.gpu_bytes / 1024
                    );
                    // The tier law: low loads FEWER patches than high.
                    if records.borrow().is_empty() {
                        eprintln!("[FAIL] no tier records captured");
                        std::process::exit(1);
                    }
                    let low_loaded = records.borrow()[0].1.loaded as u64;
                    if low_loaded >= high_counters.loaded as u64 {
                        eprintln!(
                            "[FAIL] low tier loaded {} >= high {} — tiers do not scale",
                            low_loaded, high_counters.loaded
                        );
                        std::process::exit(1);
                    }
                    println!(
                        "WINDOWED QUALITY PROOF PASS -> low/mid/high PNGs in {out_dir} (frames p50 {:.2} ms p95 {:.2} ms)",
                        report.p50_ms(),
                        report.p95_ms()
                    );
                }
                Err(e) => {
                    eprintln!("[FAIL] windowed renderer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--play-slice") => {
            // R3DV-011: THE VERTICAL SLICE. `--play-slice` = automated
            // showcase capture; `--play-slice live` = the walkable world.
            let live = args.get(2).map(String::as_str) == Some("live");
            let (seed_arg, out_dir): (Option<u64>, String) = if live {
                (args.get(3).and_then(|s| s.parse().ok()), "shots".into())
            } else {
                (
                    args.get(3).and_then(|s| s.parse().ok()),
                    args.get(2)
                        .cloned()
                        .unwrap_or_else(|| format!("{}/shots", env!("CARGO_MANIFEST_DIR"))),
                )
            };
            if !live {
                std::fs::create_dir_all(&out_dir).expect("mkdir shots");
            }

            use pc3d_render::slice::{assemble, find_showcase};
            use pc3d_render::{Shot, SliceHost};

            let t0 = std::time::Instant::now();
            let (seed, scene) = find_showcase(seed_arg.unwrap_or(3));
            println!(
                "SLICE: showcase seed {seed} found in {:.1}s (gate {:?}, cave {:?}, river edge {:?})",
                t0.elapsed().as_secs_f32(),
                scene.gate,
                scene.cave.0,
                scene.river_edge
            );
            let scene = std::rc::Rc::new(scene);

            if live {
                println!("live slice: WASD walk, click to look, F place / R remove (ray target), B save, L reload, I inspect boxes, Esc quits");
                let save_root = std::rc::Rc::new(
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("saves3d"),
                );
                let cfg = pc3d_render::WindowConfig {
                    title: "POORCRAFT 3D — the vertical slice".into(),
                    resize_to: None,
                    slice_setup: Some(pc3d_render::app::SliceSetup {
                        seed,
                        scene: scene.clone(),
                        save_root,
                        world_name: "slice".into(),
                    }),
                    ..Default::default()
                };
                match pc3d_render::run_windowed(cfg) {
                    Ok(report) => print_window_report(&report),
                    Err(e) => {
                        eprintln!("[FAIL] windowed renderer: {e}");
                        std::process::exit(1);
                    }
                }
                return;
            }

            // Automated showcase: assemble + captures (overview with the
            // cast+wheel+water, then the cave interior, then a build proof).
            let scene_cap = scene.clone();
            let scene_cave = scene.clone();
            let save_root = std::rc::Rc::new(
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("saves3d"),
            );
            let spawn = pc3d_render::spawn_player(&scene);
            let overview = pc3d_render::CameraPose::new(
                [spawn.pos[0] + 18.0, spawn.pos[1] + 24.0, spawn.pos[2] + 30.0],
                0.0,
                (-0.75f32).atan2(1.4),
            );
            let (air, wall, dir) = scene.cave;
            let cave_pose = pc3d_render::terrain::cave_pose(air, wall);
            let _ = dir;
            let built_spot = pc3d_world::coords::CellCoord {
                x: spawn.pos[0] as i32 + 2,
                y: spawn.pos[1] as i32,
                z: spawn.pos[2] as i32 - 2,
            };

            let save_root_hook = save_root.clone();
            let save_root_hook2 = save_root_hook.clone();
            let cfg = pc3d_render::WindowConfig {
                title: "POORCRAFT 3D — slice showcase".into(),
                max_frames: Some(95),
                probe_set: pc3d_render::ProbeSet::SkyOnly,
                resize_to: Some((800.0, 500.0)),
                shots: vec![
                    Shot::new(25, format!("{out_dir}/windowed_slice_showcase.png")),
                    Shot::new(55, format!("{out_dir}/windowed_slice_cave.png")),
                    Shot::new(85, format!("{out_dir}/windowed_slice_built.png")),
                ],
                frame_hooks: vec![
                    (
                        0,
                        Box::new(move |r: &mut pc3d_render::Renderer| {
                            assemble(r, &scene_cap, seed, save_root_hook2.clone(), "slice");
                            r.set_pose(overview);
                        }),
                    ),
                    (30, Box::new(move |r: &mut pc3d_render::Renderer| r.set_pose(cave_pose))),
                    (
                        60,
                        Box::new(move |r: &mut pc3d_render::Renderer| {
                            // Build through a host command, sync read-only.
                            let mut h = pc3d_world::host::SoloHost::new(seed);
                            h.submit(pc3d_world::host::HostCommand::Build {
                                cell: built_spot,
                                material: pc3d_world::gen::CellMaterial::Sand,
                                owner: 7,
                            });
                            h.run_ticks(1);
                            r.update_construction(&h.construction);
                            // Back at the spawn eye the block is visible.
                            let eye = [
                                spawn.pos[0],
                                spawn.pos[1] + 1.7,
                                spawn.pos[2],
                            ];
                            let face = [
                                built_spot.x as f32 + 0.5,
                                built_spot.y as f32 + 0.5,
                                built_spot.z as f32 + 1.05,
                            ];
                            let d = [
                                face[0] - eye[0],
                                face[1] - eye[1],
                                face[2] - eye[2],
                            ];
                            r.set_pose(pc3d_render::CameraPose::new(
                                eye,
                                (-d[0]).atan2(-d[2]),
                                (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin(),
                            ));
                            let _ = &scene_cave;
                        }),
                    ),
                ],
                ..Default::default()
            };
            match pc3d_render::run_windowed(cfg) {
                Ok(report) => {
                    print_window_report(&report);
                    if report.captures.len() != 3 {
                        eprintln!("[FAIL] expected 3 captures, got {}", report.captures.len());
                        std::process::exit(1);
                    }
                    for cap in &report.captures {
                        if !cap.report.passes_with(4) {
                            eprintln!(
                                "[FAIL] capture {}: {:?}",
                                cap.path.display(),
                                cap.report.failed_probes()
                            );
                            std::process::exit(1);
                        }
                    }
                    println!(
                        "WINDOWED SLICE PROOF PASS -> showcase + cave + built in {out_dir}"
                    );
                }
                Err(e) => {
                    eprintln!("[FAIL] windowed renderer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--play-assets") => {
            // NWR-002: the original GLB asset factory proof — tree, rock,
            // and house rendered in a live window at correct scale with
            // materials, depth, and LOD variety; prints the draw/triangle
            // and asset-memory record.
            let out_dir = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| format!("{}/shots", env!("CARGO_MANIFEST_DIR")));
            std::fs::create_dir_all(&out_dir).expect("mkdir shots");

            use pc3d_render::glb::{load_asset_file, Asset};
            use pc3d_render::{ProbeSet, Shot};

            let (seed, coord) = pc3d_world::terrain::SceneSpec::SmoothHills.patch();
            let gen = std::rc::Rc::new(pc3d_world::gen::WorldGen::new(seed));
            let assets_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../assets/compiled");
            let tree = load_asset_file(&assets_root.join("prop/tree_ash.glb")).expect("tree glb");
            let rock = load_asset_file(&assets_root.join("prop/rock_granite.glb")).expect("rock glb");
            let house = load_asset_file(&assets_root.join("module/house_croft.glb")).expect("house glb");
            // Asset memory record: parsed vertex/index bytes per asset.
            let mem = |a: &Asset| -> usize {
                a.lods
                    .iter()
                    .map(|l| l.vertices.len() * 36 + l.indices.len() * 4)
                    .sum()
            };
            println!(
                "ASSETS: tree {} B, rock {} B, house {} B (parsed mesh bytes)",
                mem(&tree),
                mem(&rock),
                mem(&house)
            );

            let t_base = [
                coord.origin().x as f32 / 1000.0 + 6.0,
                0.0,
                coord.origin().z as f32 / 1000.0 + 6.0,
            ];
            let g = std::rc::Rc::clone(&gen);
            let ground = move |x: f32, z: f32| {
                g.effective_surface_mm((x * 1000.0) as i64, (z * 1000.0) as i64) as f32 / 1000.0
            };
            let tp = [t_base[0], ground(t_base[0], t_base[2]), t_base[2]];
            let far_p = [tp[0] + 12.0, ground(tp[0] + 12.0, tp[2] - 8.0), tp[2] - 8.0];
            let rp = [tp[0] + 5.0, ground(tp[0] + 5.0, tp[2] + 2.0), tp[2] + 2.0];
            let hp = [tp[0] - 2.0, ground(tp[0] - 2.0, tp[2] + 10.0), tp[2] + 10.0];

            let eye = [tp[0] - 2.0, tp[1] + 3.2, tp[2] + 14.0];
            let aim = [tp[0] + 1.0, tp[1] + 1.8, tp[2]];
            let d = [aim[0] - eye[0], aim[1] - eye[1], aim[2] - eye[2]];
            let pose = pc3d_render::CameraPose::new(
                eye,
                (-d[0]).atan2(-d[2]),
                (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin(),
            );

            let g2 = std::rc::Rc::clone(&gen);
            let cfg = pc3d_render::WindowConfig {
                title: "POORCRAFT 3D — asset factory".into(),
                max_frames: Some(40),
                probe_set: ProbeSet::SkyOnly,
                resize_to: Some((800.0, 500.0)),
                shots: vec![Shot::new(25, format!("{out_dir}/windowed_assets.png"))],
                frame_hooks: vec![(
                    0,
                    Box::new(move |r: &mut pc3d_render::Renderer| {
                        r.set_placeholder_scene(false);
                        let patch = pc3d_world::coords::PatchCoord {
                            x: (tp[0] as i32).div_euclid(16),
                            y: 1,
                            z: (tp[2] as i32).div_euclid(16),
                        };
                        let mut patches = Vec::new();
                        for dx in -1..=1i32 {
                            for dz in -1..=1i32 {
                                patches.push(pc3d_world::coords::PatchCoord {
                                    x: patch.x + dx,
                                    y: patch.y,
                                    z: patch.z + dz,
                                });
                            }
                        }
                        r.load_terrain(&g2, &patches);
                        // Near tree lod0, far tree lod1 (LOD variety),
                        // rock lod0, house lod0.
                        let tris = [
                            r.load_asset(&tree, "lod0", tp),
                            r.load_asset(&tree, "lod1", far_p),
                            r.load_asset(&rock, "lod0", rp),
                            r.load_asset(&house, "lod0", hp),
                        ];
                        println!(
                            "ASSET SCENE: {} draws, {} tris ({} lod0 + {} lod1 + rock {} + house {})",
                            4,
                            tris.iter().sum::<usize>(),
                            tris[0], tris[1], tris[2], tris[3]
                        );
                        r.set_pose(pose);
                    }) as Box<dyn FnMut(&mut pc3d_render::Renderer)>,
                )],
                ..Default::default()
            };
            match pc3d_render::run_windowed(cfg) {
                Ok(report) => {
                    print_window_report(&report);
                    if report.captures.len() != 1 || !report.captures[0].report.passes_with(12) {
                        eprintln!("[FAIL] asset capture failed");
                        std::process::exit(1);
                    }
                    println!(
                        "WINDOWED ASSET PROOF PASS -> {}",
                        report.captures[0].path.display()
                    );
                }
                Err(e) => {
                    eprintln!("[FAIL] windowed renderer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--play-surface") => {
            // NWR-003: the natural-terrain SPIKE — an isolated 3x3 patch
            // region of low-poly surface terrain (NOT the migrated world):
            // before/after captures around a raised plateau, with the
            // sloped-ground discriminator and the mesh record.
            let out_dir = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| format!("{}/shots", env!("CARGO_MANIFEST_DIR")));
            std::fs::create_dir_all(&out_dir).expect("mkdir shots");

            use pc3d_render::surface::{SurfaceEdit, SurfaceRegion, PATCH_M};
            use pc3d_render::{ProbeSet, Shot};

            let (seed, coord) = pc3d_world::terrain::SceneSpec::SmoothHills.patch();
            let gen = std::rc::Rc::new(pc3d_world::gen::WorldGen::new(seed));
            let mut region = SurfaceRegion::new(
                pc3d_world::gen::WorldGen::new(seed),
                coord,
            );
            let t0mesh = std::time::Instant::now();
            let (verts, idx, versions) = region.mesh_region();
            let mesh_us = t0mesh.elapsed().as_micros();
            println!(
                "SURFACE SPIKE: 9 patches, {} verts / {} tris, mesh {mesh_us} us, versions {versions:?}",
                verts.len(),
                idx.len() / 3
            );

            let ox = coord.x as f32 * PATCH_M - PATCH_M;
            let oz = coord.z as f32 * PATCH_M - PATCH_M;
            let eye = [
                ox + 24.0,
                region.height_at(ox + 24.0, oz + 34.0) + 12.0,
                oz + 34.0,
            ];
            let aim = [ox + 24.0, region.height_at(ox + 24.0, oz + 10.0), oz + 10.0];
            let d = [aim[0] - eye[0], aim[1] - eye[1], aim[2] - eye[2]];
            let pose = pc3d_render::CameraPose::new(
                eye,
                (-d[0]).atan2(-d[2]),
                (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin(),
            );

            let v1 = verts.clone();
            let i1 = idx.clone();
            let g1 = gen.clone();
            let cfg = pc3d_render::WindowConfig {
                title: "POORCRAFT 3D — surface spike".into(),
                max_frames: Some(80),
                probe_set: ProbeSet::SkyOnly,
                resize_to: Some((800.0, 500.0)),
                shots: vec![
                    Shot::new(25, format!("{out_dir}/windowed_surface_before.png")),
                    Shot::new(65, format!("{out_dir}/windowed_surface_after.png"))
                        .sky(false), // the raised plateau can fill the frame
                ],
                frame_hooks: vec![
                    (
                        0,
                        Box::new(move |r: &mut pc3d_render::Renderer| {
                            r.set_placeholder_scene(false);
                            r.load_surface(&v1, &i1);
                            r.set_pose(pose);
                            let _ = &g1;
                        }) as Box<dyn FnMut(&mut pc3d_render::Renderer)>,
                    ),
                    (
                        35,
                        Box::new(move |r: &mut pc3d_render::Renderer| {
                            // Raise the plateau through the EDIT COMMAND path
                            // (3x3, one patch) and remesh from the edited
                            // region.
                            let mut region2 =
                                SurfaceRegion::new(pc3d_world::gen::WorldGen::new(seed), coord);
                            let mut dirty = std::collections::BTreeSet::new();
                            for dx in 0..3i32 {
                                for dz in 0..3i32 {
                                    dirty.extend(region2.edit(SurfaceEdit::Raise {
                                        cell: pc3d_world::coords::CellCoord {
                                            x: (ox + 23.0) as i32 + dx,
                                            y: 0,
                                            z: (oz + 11.0) as i32 + dz,
                                        },
                                        meters: 4.0,
                                    }));
                                }
                            }
                            let (v2, i2, _) = region2.mesh_region();
                            println!(
                                "SURFACE EDIT: dirty {dirty:?} -> remesh {} verts (bounded)",
                                v2.len()
                            );
                            r.load_surface(&v2, &i2);
                        }) as Box<dyn FnMut(&mut pc3d_render::Renderer)>,
                    ),
                ],
                ..Default::default()
            };
            match pc3d_render::run_windowed(cfg) {
                Ok(report) => {
                    print_window_report(&report);
                    if report.captures.len() != 2 {
                        eprintln!("[FAIL] expected 2 captures, got {}", report.captures.len());
                        std::process::exit(1);
                    }
                    for cap in &report.captures {
                        if !cap.report.passes_with(12) {
                            eprintln!(
                                "[FAIL] capture {}: {:?}",
                                cap.path.display(),
                                cap.report.failed_probes()
                            );
                            std::process::exit(1);
                        }
                    }
                    let diff = pc3d_render::scene::pixel_difference_fraction(
                        &report.captures[0].rgba,
                        &report.captures[1].rgba,
                    );
                    if diff < 0.005 {
                        eprintln!("[FAIL] the plateau edit must change the view ({diff})");
                        std::process::exit(1);
                    }
                    println!("SURFACE EDIT VISIBLE: image diff {diff:.2}%");
                    println!(
                        "WINDOWED SURFACE SPIKE PASS -> before/after in {out_dir}"
                    );
                }
                Err(e) => {
                    eprintln!("[FAIL] windowed renderer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--play-caves") => {
            // NWR-005: caves + conforming water + local edits, one live
            // window. Stand INSIDE a carved cave aimed at its wall, fly
            // to a river overlook with conforming water drawn at the
            // strip water-line, then raise a berm ON a water strip
            // through the surface-edit path: only nearby sections
            // refresh (the P3D-303 local-remesh law) and the berm is
            // visibly there before/after.
            let out_dir = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| format!("{}/shots", env!("CARGO_MANIFEST_DIR")));
            std::fs::create_dir_all(&out_dir).expect("mkdir shots");

            use pc3d_render::surface::{SurfaceEdit, SurfaceRegion, PATCH_M};
            use pc3d_render::world_features::{CaveRegion, ConformingWater, FoundationCheck};
            use pc3d_render::{ProbeSet, Shot};
            use pc3d_world::flow::FlowTable;
            use pc3d_world::hydro::RiverGraph;

            // --- Cave scene on the hills region (same scene as --play-surface).
            let (seed, coord) = pc3d_world::terrain::SceneSpec::SmoothHills.patch();
            let gen = std::rc::Rc::new(pc3d_world::gen::WorldGen::new(seed));
            let hills = SurfaceRegion::new(pc3d_world::gen::WorldGen::new(seed), coord);
            let (hverts, hidx, _) = hills.mesh_region();
            let pocket = pc3d_render::terrain::find_cave_pocket_near(&gen, coord, 1)
                .map(|(air, _, _)| air)
                .or_else(|| {
                    let mut found = None;
                    for dx in -3..=3i32 {
                        for dz in -3..=3i32 {
                            let p = pc3d_world::coords::PatchCoord {
                                x: coord.x + dx,
                                y: coord.y,
                                z: coord.z + dz,
                            };
                            if let Some((air, _, _)) =
                                pc3d_render::terrain::find_cave_pocket_near(&gen, p, 0)
                            {
                                found = Some(air);
                            }
                        }
                    }
                    found
                })
                .expect("a carved pocket near the hills region");
            let cave = CaveRegion::around(&gen, pocket).expect("a cave region");
            let (cverts, cidx) = cave.mesh(&gen);
            println!(
                "CAVE: pocket {pocket:?} spans {}x{}x{} cells, {} verts / {} tris",
                cave.max[0] - cave.min[0] + 1,
                cave.max[1] - cave.min[1] + 1,
                cave.max[2] - cave.min[2] + 1,
                cverts.len(),
                cidx.len() / 3
            );

            // Camera inside the pocket aimed at the NEAREST enclosing
            // wall (aiming down a corridor exits the mouth — the GPU
            // proof run caught that).
            let eye = [
                pocket.x as f32 + 0.5,
                pocket.y as f32 + 0.9,
                pocket.z as f32 + 0.5,
            ];
            let dirs = [
                [1.0f32, 0.0, 0.0],
                [-1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, -1.0],
            ];
            let mut look = dirs[0];
            let mut wall_dist = f32::MAX;
            for d in dirs {
                for step in 1..=6i32 {
                    let probe = pc3d_world::coords::CellCoord {
                        x: pocket.x + (d[0] * step as f32) as i32,
                        y: pocket.y,
                        z: pocket.z + (d[2] * step as f32) as i32,
                    };
                    if pc3d_world::terrain::final_solid(
                        &gen,
                        probe.x as i64 * 1000,
                        probe.y as i64 * 1000,
                        probe.z as i64 * 1000,
                    )
                    .solid
                    {
                        if (step as f32) < wall_dist {
                            wall_dist = step as f32;
                            look = d;
                        }
                        break;
                    }
                }
            }
            assert!(wall_dist < f32::MAX, "an enclosing wall exists near the pocket");
            let aim = [
                eye[0] + look[0] * (wall_dist - 0.6),
                eye[1] - 0.15,
                eye[2] + look[2] * (wall_dist - 0.6),
            ];
            let dv = [aim[0] - eye[0], aim[1] - eye[1], aim[2] - eye[2]];
            let pose_cave = pc3d_render::CameraPose::new(
                eye,
                (-dv[0]).atan2(-dv[2]),
                (dv[1] / (dv[0] * dv[0] + dv[1] * dv[1] + dv[2] * dv[2]).sqrt()).asin(),
            );

            // --- River scene: nearest river region (graph band covers
            // the hills scene — half=70 like the GPU proof).
            let graph = RiverGraph::new(&gen, 70);
            let flow = FlowTable::from_graph(&graph);
            let cr = pc3d_world::coords::RegionCoord {
                x: coord.x.div_euclid(16),
                z: coord.z.div_euclid(16),
            };
            let mut best: Option<(pc3d_world::coords::RegionCoord, i32)> = None;
            for dx in -8..=8i32 {
                for dz in -8..=8i32 {
                    let reg = pc3d_world::coords::RegionCoord { x: cr.x + dx, z: cr.z + dz };
                    if let Some(d) = graph.downstream(reg) {
                        if graph.discharge(d) >= pc3d_world::hydro::RIVER_THRESHOLD {
                            let dist = dx.abs() + dz.abs();
                            if best.as_ref().map(|(_, b)| dist < *b).unwrap_or(true) {
                                best = Some((reg, dist));
                            }
                        }
                    }
                }
            }
            let (center, _) = best.expect("a river region within +/-8");
            // Two-pass (the GPU proof found this first): a probe build
            // names section 0's strip start; the REAL patch window is
            // rebuilt AT that start — a SurfaceRegion is a 3x3 PATCH
            // (48 m) window, not a 3x3 region, so the edit must land
            // inside those 48 m.
            let prov_patch = pc3d_world::coords::PatchCoord {
                x: center.x * 16 + 8,
                y: coord.y,
                z: center.z * 16 + 8,
            };
            let prov = SurfaceRegion::new(pc3d_world::gen::WorldGen::new(seed), prov_patch);
            let probe = ConformingWater::build(&gen, &graph, &flow, center, &prov);
            let s0r = probe.sections[0].region;
            let ox = (s0r.0 as f32 + 0.5) * 256.0;
            let oz = (s0r.1 as f32 + 0.5) * 256.0;
            let river_patch = pc3d_world::coords::PatchCoord {
                x: (ox / PATCH_M).floor() as i32,
                y: coord.y,
                z: (oz / PATCH_M).floor() as i32,
            };
            let river_region = SurfaceRegion::new(pc3d_world::gen::WorldGen::new(seed), river_patch);
            let water = ConformingWater::build(&gen, &graph, &flow, center, &river_region);
            println!(
                "RIVER: section 0 at region {s0r:?}, {} conforming sections",
                water.sections.len()
            );

            // The strip's own direction (for the corridor windows, the
            // camera, and the berm placement).
            let s0 = &water.sections[0];
            let sdir = match s0.direction {
                0 => [1.0f32, 0.0],
                1 => [1.0, 1.0],
                2 => [0.0, 1.0],
                3 => [-1.0, 1.0],
                4 => [-1.0, 0.0],
                5 => [-1.0, -1.0],
                6 => [0.0, -1.0],
                _ => [1.0, -1.0],
            };
            let sl = sdir[0].hypot(sdir[1]);
            let dir = [sdir[0] / sl, sdir[1] / sl];
            let slen = 256.0 * if sl > 1.4 { std::f32::consts::SQRT_2 } else { 1.0 };

            // Corridor surface windows along the strip so the vantage
            // sees a valley, not one floating 48 m tile.
            let along = |t: f32| (ox + dir[0] * slen * t, oz + dir[1] * slen * t);
            let corridor = |t: f32| {
                let (cx, cz) = along(t);
                pc3d_world::coords::PatchCoord {
                    x: (cx / PATCH_M).floor() as i32,
                    y: coord.y,
                    z: (cz / PATCH_M).floor() as i32,
                }
            };
            let region1 = SurfaceRegion::new(pc3d_world::gen::WorldGen::new(seed), corridor(0.3));
            let region2 = SurfaceRegion::new(pc3d_world::gen::WorldGen::new(seed), corridor(0.6));

            // Camera: standing on the bank BESIDE the berm site (the
            // t=1/16 sample point), looking at it down the corridor —
            // the berm must dominate its share of the after-capture.
            let ground = |x: f32, z: f32| {
                gen.effective_surface_mm((x * 1000.0) as i64, (z * 1000.0) as i64) as f32 / 1000.0
            };
            let (sx, sz) = along(1.0 / 16.0);
            let perp = [-dir[1], dir[0]];
            let mut eye2 = [
                ox + dir[0] * 6.0 + perp[0] * 18.0,
                0.0,
                oz + dir[1] * 6.0 + perp[1] * 18.0,
            ];
            eye2[1] = ground(eye2[0], eye2[2]) + 6.0;
            let aim2 = [sx, ground(sx, sz) + 1.0, sz];
            let d2 = [aim2[0] - eye2[0], aim2[1] - eye2[1], aim2[2] - eye2[2]];
            let pose_river = pc3d_render::CameraPose::new(
                eye2,
                (-d2[0]).atan2(-d2[2]),
                (d2[1] / (d2[0] * d2[0] + d2[1] * d2[1] + d2[2] * d2[2]).sqrt()).asin(),
            );

            // Foundation evidence: scan pads inside the window — real
            // ground near the river should offer a buildable spot (the
            // accept/reject law is proven in the unit tests).
            let mut valid_pad = None;
            for d in [4.0f32, 8.0, 12.0, -8.0] {
                let pad = pc3d_world::coords::CellCoord {
                    x: (ox + d) as i32,
                    y: 0,
                    z: (oz + d) as i32,
                };
                if let FoundationCheck::Valid { leveled_by } =
                    pc3d_render::world_features::check_foundation(
                        &gen,
                        &river_region,
                        pad,
                        (2, 2),
                    )
                {
                    valid_pad = Some((d, leveled_by));
                    break;
                }
            }
            let (fd, lv) =
                valid_pad.unwrap_or_else(|| {
                    eprintln!("[FAIL] no buildable pad near the river strip");
                    std::process::exit(1);
                });
            println!("FOUNDATION near river: Valid {{ leveled_by: {lv} }} at +{fd} m");

            // --- Merged terrain mesh: hills + the three corridor windows
            // coexist in one world (one surface slot); the cave rides the
            // u32 mesh slot.
            let (r0v, r0i, _) = river_region.mesh_region();
            let (r1v, r1i, _) = region1.mesh_region();
            let (r2v, r2i, _) = region2.mesh_region();
            let mut mverts = hverts.clone();
            let mut midx = hidx.clone();
            for (v, i) in [(r0v, r0i), (r1v, r1i), (r2v, r2i)] {
                let base = mverts.len() as u32;
                mverts.extend(v.iter().cloned());
                midx.extend(i.iter().map(|k| k + base));
            }
            let (wverts, widx) = water.mesh(&gen);

            // The berm: a 2x2 cell block centered on the t=1/16 sample
            // point. Raises STACK on shared nodes (Raise is additive per
            // node), so 2x2 at 6 m is a ~12 m dam wall — a 4x4 block
            // piled 24 m into the sky and the proof run caught it.
            let bx = (sx - 1.0).floor() as i32;
            let bz = (sz - 1.0).floor() as i32;

            let water_rc = std::rc::Rc::new(std::cell::RefCell::new(water));
            let w0 = water_rc.clone();
            let g_edit = gen.clone();
            let hv = hverts.clone();
            let hi = hidx.clone();
            let cp1 = corridor(0.3);
            let cp2 = corridor(0.6);

            let cfg = pc3d_render::WindowConfig {
                title: "POORCRAFT 3D — caves and water".into(),
                max_frames: Some(170),
                probe_set: ProbeSet::SkyOnly,
                resize_to: Some((800.0, 500.0)),
                camera_script: vec![
                    (0, pose_cave),
                    (60, pose_river),
                    (140, pose_river),
                ],
                shots: vec![
                    Shot::new(25, format!("{out_dir}/windowed_cave_interior.png"))
                        .sky(false), // the frame is cave wall, no sky
                    Shot::new(85, format!("{out_dir}/windowed_river_water.png")),
                    Shot::new(145, format!("{out_dir}/windowed_water_after_edit.png"))
                        .sky(false), // the dam wall in the foreground can fill the top
                ],
                frame_hooks: vec![
                    (
                        0,
                        Box::new(move |r: &mut pc3d_render::Renderer| {
                            r.set_placeholder_scene(false);
                            r.load_surface(&mverts, &midx);
                            r.load_u32_mesh(&cverts, &cidx);
                            r.load_water_vertices(&wverts, &widx);
                            r.set_water_time(Some(0.0));
                        }) as Box<dyn FnMut(&mut pc3d_render::Renderer)>,
                    ),
                    (
                        120,
                        Box::new(move |r: &mut pc3d_render::Renderer| {
                            // Raise the berm ON the strip through the
                            // surface-edit path, refresh the conforming
                            // water, remesh — all LOCAL work.
                            let mut r0 = SurfaceRegion::new(
                                pc3d_world::gen::WorldGen::new(seed),
                                river_patch,
                            );
                            let mut dirty = std::collections::BTreeSet::new();
                            for i in 0..2i32 {
                                for j in 0..2i32 {
                                    dirty.extend(r0.edit(SurfaceEdit::Raise {
                                        cell: pc3d_world::coords::CellCoord {
                                            x: bx + i,
                                            y: 0,
                                            z: bz + j,
                                        },
                                        meters: 6.0,
                                    }));
                                }
                            }
                            if dirty.is_empty() {
                                eprintln!("[FAIL] the berm missed the patch window");
                                std::process::exit(1);
                            }
                            // edit() reports (x,z) patch keys; the refresh
                            // takes full PatchCoords at the region's Y.
                            let edited: std::collections::BTreeSet<
                                pc3d_world::coords::PatchCoord,
                            > = dirty
                                .iter()
                                .map(|(x, z)| pc3d_world::coords::PatchCoord {
                                    x: *x,
                                    y: river_patch.y,
                                    z: *z,
                                })
                                .collect();
                            let total = w0.borrow().sections.len();
                            let before: Vec<(Vec<f32>, f32)> = w0
                                .borrow()
                                .sections
                                .iter()
                                .map(|s| (s.heights.clone(), s.water_line))
                                .collect();
                            let t0 = std::time::Instant::now();
                            let touched =
                                w0.borrow_mut().refresh_after_edit(&g_edit, &r0, &edited);
                            let refresh_us = t0.elapsed().as_micros();
                            let changed = w0
                                .borrow()
                                .sections
                                .iter()
                                .zip(before.iter())
                                .filter(|(s, (h, b))| {
                                    (s.water_line - *b).abs() > 1e-6
                                        || s.heights
                                            .iter()
                                            .zip(h.iter())
                                            .any(|(a, c)| (a - c).abs() > 1e-6)
                                })
                                .count();
                            let t1 = std::time::Instant::now();
                            let (wv2, wi2) = w0.borrow().mesh(&g_edit);
                            let remesh_us = t1.elapsed().as_micros();
                            // Rebuild the corridor: the edited window plus
                            // the two unedited ones (deterministic rebuild).
                            let (rv0, ri0, _) = r0.mesh_region();
                            let c1 = SurfaceRegion::new(pc3d_world::gen::WorldGen::new(seed), cp1);
                            let c2 = SurfaceRegion::new(pc3d_world::gen::WorldGen::new(seed), cp2);
                            let (rv1, ri1, _) = c1.mesh_region();
                            let (rv2, ri2, _) = c2.mesh_region();
                            println!(
                                "LOCAL EDIT: dirty patches {dirty:?}; {touched}/{total} sections refreshed, {changed} changed data; refresh {refresh_us} us, water remesh {remesh_us} us, terrain remesh {} verts",
                                rv0.len() + rv1.len() + rv2.len()
                            );
                            if touched != changed {
                                eprintln!("[FAIL] exactly the refreshed sections may change");
                                std::process::exit(1);
                            }
                            if touched == 0 || touched >= total {
                                eprintln!("[FAIL] the edit must stay local ({touched}/{total})");
                                std::process::exit(1);
                            }
                            let mut mv2 = hv.clone();
                            let mut mi2 = hi.clone();
                            for (v, i) in [(rv0, ri0), (rv1, ri1), (rv2, ri2)] {
                                let base = mv2.len() as u32;
                                mv2.extend(v.iter().cloned());
                                mi2.extend(i.iter().map(|k| k + base));
                            }
                            r.load_surface(&mv2, &mi2);
                            r.load_water_vertices(&wv2, &wi2);
                        }) as Box<dyn FnMut(&mut pc3d_render::Renderer)>,
                    ),
                ],
                ..Default::default()
            };
            match pc3d_render::run_windowed(cfg) {
                Ok(report) => {
                    print_window_report(&report);
                    if report.captures.len() != 3 {
                        eprintln!("[FAIL] expected 3 captures, got {}", report.captures.len());
                        std::process::exit(1);
                    }
                    // The cave interior is a close-up of a few flat faces:
                    // the relaxed nonuniformity floor (3) applies there;
                    // the open river views keep the 8 floor.
                    for (i, cap) in report.captures.iter().enumerate() {
                        let floor = if i == 0 { 3 } else { 8 };
                        if !cap.report.passes_with(floor) {
                            eprintln!(
                                "[FAIL] capture {}: {:?} (distinct {}, floor {floor})",
                                cap.path.display(),
                                cap.report.failed_probes(),
                                cap.report.distinct_colors
                            );
                            std::process::exit(1);
                        }
                    }
                    let diff = pc3d_render::scene::pixel_difference_fraction(
                        &report.captures[1].rgba,
                        &report.captures[2].rgba,
                    );
                    if diff < 0.005 {
                        eprintln!("[FAIL] the berm edit must change the view ({diff})");
                        std::process::exit(1);
                    }
                    println!("WATER EDIT VISIBLE: image diff {diff:.2}%");
                    println!(
                        "WINDOWED CAVES+WATER PROOF PASS -> cave interior, river before/after in {out_dir}"
                    );
                }
                Err(e) => {
                    eprintln!("[FAIL] windowed renderer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--play-materials") => {
            // NWR-006: materials + atmosphere in one live window — the
            // Mid tier vista (shadows + fog + material grain + glint),
            // the LEGACY control (same scene, every term off), the High
            // tier, and a walk into the cutout foliage.
            let out_dir = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| format!("{}/shots", env!("CARGO_MANIFEST_DIR")));
            std::fs::create_dir_all(&out_dir).expect("mkdir shots");

            use pc3d_render::atmosphere::{
                foliage_quads, AtmosphereTier, CutoutMask, LEGACY,
            };
            use pc3d_render::surface::SurfaceRegion;
            use pc3d_render::{ProbeSet, Shot};

            let (seed, coord) = pc3d_world::terrain::SceneSpec::SmoothHills.patch();
            let gen = std::rc::Rc::new(pc3d_world::gen::WorldGen::new(seed));
            let region = SurfaceRegion::new(pc3d_world::gen::WorldGen::new(seed), coord);
            let (verts, idx, _) = region.mesh_region();
            let ox = coord.x as f32 * 16.0 - 16.0;
            let oz = coord.z as f32 * 16.0 - 16.0;
            let ground = |x: f32, z: f32| {
                gen.effective_surface_mm((x * 1000.0) as i64, (z * 1000.0) as i64) as f32 / 1000.0
            };
            // Foliage field on the near hill + water strips in the dip.
            let (fverts, fidx) = foliage_quads([ox + 6.0, ground(ox + 6.0, oz + 6.0), oz + 6.0], 3, 3, 2.2);
            let mut wverts = Vec::new();
            let mut widx = Vec::new();
            for k in 0..10i32 {
                let z0 = oz + 2.0 + k as f32 * 2.0;
                let y = ground(ox + 14.0, z0) + 0.35;
                let base = wverts.len() as u16;
                for p in [
                    [ox + 9.0, y, z0],
                    [ox + 19.0, y, z0],
                    [ox + 19.0, y, z0 + 2.0],
                    [ox + 9.0, y, z0 + 2.0],
                ] {
                    wverts.push(pc3d_render::water::WaterVertex {
                        pos: p,
                        dir: [1.0, 0.0],
                        speed: 2.0,
                        alpha: 0.66,
                    });
                }
                widx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            }
            // Vista pose: above the hill looking across the water toward
            // the sun (the glint) with the foliage in the mid-ground.
            let eye = [
                ox + 1.0,
                ground(ox + 1.0, oz + 1.0) + 9.0,
                oz + 1.0,
            ];
            let aim = [ox + 16.0, ground(ox + 16.0, oz + 10.0) + 1.0, oz + 10.0];
            let d = [aim[0] - eye[0], aim[1] - eye[1], aim[2] - eye[2]];
            let pose = pc3d_render::CameraPose::new(
                eye,
                (-d[0]).atan2(-d[2]),
                (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin(),
            );
            // Cutout close-up: inside the foliage field.
            let eye2 = [
                ox + 4.0,
                ground(ox + 4.0, oz + 5.0) + 1.7,
                oz + 5.0,
            ];
            let aim2 = [ox + 9.0, ground(ox + 9.0, oz + 7.0) + 1.4, oz + 7.0];
            let d2 = [aim2[0] - eye2[0], aim2[1] - eye2[1], aim2[2] - eye2[2]];
            let pose2 = pc3d_render::CameraPose::new(
                eye2,
                (-d2[0]).atan2(-d2[2]),
                (d2[1] / (d2[0] * d2[0] + d2[1] * d2[1] + d2[2] * d2[2]).sqrt()).asin(),
            );

            let v1 = verts.clone();
            let i1 = idx.clone();
            let fv = fverts.clone();
            let fi = fidx.clone();
            let wv = wverts.clone();
            let wi = widx.clone();
            let cfg = pc3d_render::WindowConfig {
                title: "POORCRAFT 3D — materials and atmosphere".into(),
                max_frames: Some(190),
                probe_set: ProbeSet::SkyOnly,
                resize_to: Some((800.0, 500.0)),
                camera_script: vec![
                    (0, pose),
                    (130, pose2),
                ],
                shots: vec![
                    Shot::new(30, format!("{out_dir}/windowed_materials_mid.png")),
                    Shot::new(70, format!("{out_dir}/windowed_materials_legacy.png")),
                    Shot::new(110, format!("{out_dir}/windowed_materials_high.png")),
                    Shot::new(170, format!("{out_dir}/windowed_materials_cutout.png"))
                        .sky(false), // inside the leaf cards
                ],
                frame_hooks: vec![
                    (
                        0,
                        Box::new(move |r: &mut pc3d_render::Renderer| {
                            r.set_placeholder_scene(false);
                            r.load_surface(&v1, &i1);
                            let tris = r.load_cutout(&fv, &fi, CutoutMask::Leaf);
                            r.load_water_vertices(&wv, &wi);
                            r.set_water_time(Some(0.0));
                            r.set_atmosphere_tier(AtmosphereTier::Mid);
                            println!(
                                "ATMOSPHERE: mid tier (shadow {}, fog {}, detail {}, glint), {} foliage tris",
                                AtmosphereTier::Mid.params().shadow_res,
                                AtmosphereTier::Mid.params().fog_density,
                                AtmosphereTier::Mid.params().detail_strength,
                                tris
                            );
                        }) as Box<dyn FnMut(&mut pc3d_render::Renderer)>,
                    ),
                    (
                        60,
                        Box::new(move |r: &mut pc3d_render::Renderer| {
                            // The control: EXACTLY the legacy look.
                            r.set_atmosphere(LEGACY);
                        }) as Box<dyn FnMut(&mut pc3d_render::Renderer)>,
                    ),
                    (
                        100,
                        Box::new(move |r: &mut pc3d_render::Renderer| {
                            let p = AtmosphereTier::High.params();
                            r.set_atmosphere(p);
                            println!(
                                "ATMOSPHERE: high tier (shadow {}, fog {}, detail {})",
                                p.shadow_res, p.fog_density, p.detail_strength
                            );
                        }) as Box<dyn FnMut(&mut pc3d_render::Renderer)>,
                    ),
                ],
                ..Default::default()
            };
            match pc3d_render::run_windowed(cfg) {
                Ok(report) => {
                    print_window_report(&report);
                    if report.captures.len() != 4 {
                        eprintln!("[FAIL] expected 4 captures, got {}", report.captures.len());
                        std::process::exit(1);
                    }
                    for cap in &report.captures {
                        if !cap.report.passes_with(3) {
                            eprintln!(
                                "[FAIL] capture {}: {:?}",
                                cap.path.display(),
                                cap.report.failed_probes()
                            );
                            std::process::exit(1);
                        }
                    }
                    let mid = &report.captures[0].rgba;
                    let legacy = &report.captures[1].rgba;
                    let high = &report.captures[2].rgba;
                    let d_mid = pc3d_render::scene::pixel_difference_fraction(mid, legacy);
                    let d_high = pc3d_render::scene::pixel_difference_fraction(high, legacy);
                    if d_mid < 0.02 {
                        eprintln!("[FAIL] the atmosphere must visibly change the scene ({d_mid})");
                        std::process::exit(1);
                    }
                    if d_high < 0.02 {
                        eprintln!("[FAIL] the high tier must differ from legacy ({d_high})");
                        std::process::exit(1);
                    }
                    println!(
                        "ATMOSPHERE DIFFS: mid vs legacy {d_mid:.2}%, high vs legacy {d_high:.2}%"
                    );
                    println!(
                        "WINDOWED MATERIALS PROOF PASS -> mid/legacy/high/cutout in {out_dir}"
                    );
                }
                Err(e) => {
                    eprintln!("[FAIL] windowed renderer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("--play-surface-stream") => {
            // NWR-004: the MIGRATED ordinary terrain — streamed SURFACE
            // patches with LOD rings + skirts, a real vista to the horizon.
            let out_dir = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| format!("{}/shots", env!("CARGO_MANIFEST_DIR")));
            std::fs::create_dir_all(&out_dir).expect("mkdir shots");

            use pc3d_render::surface_stream::SurfaceStreamer;
            use pc3d_render::{ProbeSet, Shot};
            use pc3d_world::stream::Tier;

            let (seed, coord) = pc3d_world::terrain::SceneSpec::SmoothHills.patch();
            let gen = std::rc::Rc::new(pc3d_world::gen::WorldGen::new(seed));
            let cx = coord.x as f32 * 16.0 + 8.0;
            let cz = coord.z as f32 * 16.0 + 8.0;
            let ground = gen
                .effective_surface_mm((cx * 1000.0) as i64, ((cz + 30.0) * 1000.0) as i64)
                as f32
                / 1000.0;
            let aim_h = gen.effective_surface_mm((cx * 1000.0) as i64, (cz * 1000.0) as i64)
                as f32
                / 1000.0;
            let eye = [cx, ground + 9.0, cz + 30.0];
            let d = [cx - eye[0], aim_h - eye[1], cz - eye[2]];
            let pose = pc3d_render::CameraPose::new(
                eye,
                (-d[0]).atan2(-d[2]),
                (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin(),
            );

            let g1 = gen.clone();
            let cfg = pc3d_render::WindowConfig {
                title: "POORCRAFT 3D — streamed surface terrain".into(),
                max_frames: Some(260),
                probe_set: ProbeSet::SkyOnly,
                resize_to: Some((800.0, 500.0)),
                camera_script: vec![(0, pose)],
                shots: vec![Shot::new(250, format!("{out_dir}/windowed_surface_stream.png"))],
                frame_hooks: vec![(
                    0,
                    Box::new(move |r: &mut pc3d_render::Renderer| {
                        r.set_placeholder_scene(false);
                        let mut s = SurfaceStreamer::new(g1.clone(), &[Tier::Full, Tier::Lod], coord.y);
                        s.set_budgets(3, 24 * 1024 * 1024);
                        r.attach_surface_stream(s);
                        r.set_pose(pose);
                    }) as Box<dyn FnMut(&mut pc3d_render::Renderer)>,
                )],
                ..Default::default()
            };
            match pc3d_render::run_windowed(cfg) {
                Ok(report) => {
                    print_window_report(&report);
                    if report.captures.len() != 1
                        || !report.captures[0].report.passes_with(12)
                    {
                        eprintln!("[FAIL] surface-stream capture failed");
                        std::process::exit(1);
                    }
                    println!(
                        "WINDOWED SURFACE STREAM PASS -> {}",
                        report.captures[0].path.display()
                    );
                }
                Err(e) => {
                    eprintln!("[FAIL] windowed renderer: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some(other) => {
            eprintln!(
                "unknown argument: {other}\nusage: poorcraft3d [--identity|--format|--baseline|--run [seconds]|--atlas <seed> [half_regions]|--terrain-bench|--debug-overlay <seed>|--flow-map <seed]|--diagnose <seed]|--soak <days> [seed]|--journey [seed]|--play|--play-shot [png]|--play-build [png|live] [seed]|--play-terrain [outdir]|--play-stream [outdir]|--play-water [outdir] [seed]|--play-city [outdir] [seed]|--play-npcs [outdir] [seed]|--play-quality [outdir] [seed]|--play-slice [outdir|live] [seed]|--play-assets [outdir]|--play-surface [outdir]|--play-caves [outdir]|--play-surface-stream [outdir]|--validate-assets [path]]"
            );
            std::process::exit(2);
        }
    }
}

fn print_window_report(report: &pc3d_render::WindowReport) {
    println!(
        "windowed run: {} frames · frame p50 {:.2} ms · p95 {:.2} ms · avg {:.1} fps",
        report.frames,
        report.p50_ms(),
        report.p95_ms(),
        report.avg_fps()
    );
}
