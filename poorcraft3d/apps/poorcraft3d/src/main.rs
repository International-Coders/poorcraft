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
                    pc3d_render::Shot {
                        frame: 20,
                        path: std::path::PathBuf::from(&out),
                    },
                    pc3d_render::Shot {
                        frame: 40,
                        path: std::path::PathBuf::from(&out_b.clone()),
                    },
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
                    Shot { frame: 20, path: std::path::PathBuf::from(&base) },
                    Shot { frame: 40, path: std::path::PathBuf::from(&after_path.clone()) },
                ],
                frame_hooks: vec![
                    (
                        0,
                        Box::new(move |r| {
                            r.set_placeholder_scene(false);
                            r.attach_construction();
                            let st = r.update_construction(&host_init.borrow().construction);
                            *is.borrow_mut() = Some(st);
                        }),
                    ),
                    (
                        30,
                        Box::new(move |r| {
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
                    Shot {
                        frame: 30,
                        path: std::path::PathBuf::from(format!("{out_dir}/windowed_terrain_hills.png")),
                    },
                    Shot {
                        frame: 70,
                        path: std::path::PathBuf::from(format!("{out_dir}/windowed_terrain_cliff.png")),
                    },
                    Shot {
                        frame: 110,
                        path: std::path::PathBuf::from(format!("{out_dir}/windowed_terrain_cave.png")),
                    },
                ],
                frame_hooks: vec![
                    (
                        0,
                        Box::new(move |r| {
                            r.set_placeholder_scene(false);
                            let st = r.load_terrain(&hg, &neighborhood3(hill_coord));
                            r.set_pose(hill_pose);
                            st0.borrow_mut().push(("hills".into(), st));
                        }),
                    ),
                    (
                        40,
                        Box::new(move |r| {
                            let st = r.load_terrain(&cg, &neighborhood3(cliff_coord));
                            r.set_pose(cliff_pose);
                            st1.borrow_mut().push(("cliff".into(), st));
                        }),
                    ),
                    (
                        80,
                        Box::new(move |r| {
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
                    Shot {
                        frame: 110,
                        path: std::path::PathBuf::from(format!(
                            "{out_dir}/windowed_stream_walk1.png"
                        )),
                    },
                    Shot {
                        frame: 230,
                        path: std::path::PathBuf::from(format!(
                            "{out_dir}/windowed_stream_walk2.png"
                        )),
                    },
                    Shot {
                        frame: 350,
                        path: std::path::PathBuf::from(format!(
                            "{out_dir}/windowed_stream_walk3.png"
                        )),
                    },
                ],
                frame_hooks: vec![(
                    0,
                    Box::new(move |r| {
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
        Some(other) => {
            eprintln!(
                "unknown argument: {other}\nusage: poorcraft3d [--identity|--format|--baseline|--run [seconds]|--atlas <seed> [half_regions]|--terrain-bench|--debug-overlay <seed>|--flow-map <seed>|--diagnose <seed>|--soak <days> [seed]|--journey [seed]|--play|--play-shot [png]|--play-build [png|live] [seed]|--play-terrain [outdir]|--play-stream [outdir]|--validate-assets [path]]"
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
