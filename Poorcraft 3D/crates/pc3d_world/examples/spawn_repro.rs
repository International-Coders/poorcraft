//! N02 probe (throwaway, kept as the self-check demo): builds the
//! world of seed 3, runs the N02 spawn decision, and asserts the
//! three headline laws in one place. Run: cargo run -p pc3d_world
//! --example spawn_repro
use pc3d_world::gen::WorldGen;
use pc3d_world::player::Player;
use pc3d_world::spawn::decide_spawn;
use pc3d_world::trust::spawn_clearance_violations;

fn main() {
    let seed = 3u64;
    let gen = WorldGen::new(seed);
    let d = decide_spawn(&gen, seed);
    let c = d
        .candidate
        .as_ref()
        .expect("seed 3 must produce a tier-1 candidate");
    assert!(!d.used_fallback, "seed 3 must not need the fallback tier");
    assert!(
        spawn_clearance_violations(&gen, c.feet, c.yaw_deg, seed).is_empty(),
        "winner must pass the N01 gate at its own yaw"
    );
    assert!(c.resource.is_some(), "winner must carry a nav-confirmed resource");
    let p = Player::spawn_safe(&gen);
    assert_eq!(p.pos, c.feet, "spawn_safe must agree with the decision");
    println!("N02 self-check OK: feet={:?} yaw={}", c.feet, c.yaw_deg);
}
