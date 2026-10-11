//! Audit tool (build-pack Step 1): is the title-screen orbit camera buried
//! in terrain for a given world seed? Prints the ring terrain heights the
//! camera sweeps through vs the camera eye height.
use lf_worldgen::{Seed, WorldGen};

fn main() {
    let seed: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok())
        .expect("usage: audit_title_camera <seed u64>");
    let gen = WorldGen::new(Seed(seed));
    let spawn = (0.5f32, 0.5f32);
    let ground0 = gen.surface_top(0, 0);
    let eye_y = ground0 as f32 + 14.0;
    println!("spawn ground y={} -> title camera eye y={eye_y:.0}", ground0);
    let mut buried = 0;
    let mut total = 0;
    for i in 0..64 {
        let a = i as f32 * std::f32::consts::TAU / 64.0;
        let x = spawn.0 + a.cos() * 34.0;
        let z = spawn.1 + a.sin() * 34.0;
        let h = gen.surface_top(x as i32, z as i32);
        total += 1;
        if (h as f32) > eye_y {
            buried += 1;
        }
        if i % 8 == 0 {
            println!("  angle {:5.1} deg: ring terrain y={h} (eye {eye_y:.0})", a.to_degrees());
        }
    }
    println!("ring points with terrain above camera eye: {buried}/{total}");
}
