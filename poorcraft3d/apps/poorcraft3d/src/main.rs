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
        Some(other) => {
            eprintln!(
                "unknown argument: {other}\nusage: poorcraft3d [--identity|--format|--baseline|--run [seconds]]"
            );
            std::process::exit(2);
        }
    }
}
