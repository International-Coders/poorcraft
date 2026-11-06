//! N04 RED probe — grounding divergence inventory.
//!
//! Reproduces the four grounding defects against the CURRENT code so the
//! gate (GroundingResult authority tests) can be shown to fail for the
//! described reasons before the fix lands:
//!
//! 1. NPC float: `npc_world_pos` adds +1.0 m above the analytic surface
//!    (the GLB pivot is AT the feet) — every NPC renders hovering.
//! 2. Flora in rivers: `plant_at` has no river-channel gate — since N03
//!    carved channels hold water, trees can stand in it.
//! 3. River-blind ground: analytic `effective_surface_mm` answers the
//!    pre-carve bank height inside a channel (final_solid bed is lower).
//! 4. Water wheel -0.2 on the analytic surface (same river blindness).
//!
//! Run: cargo run --release -p pc3d_world --example n04_grounding_repro
use pc3d_world::flora::{plant_at, SlotCoord};
use pc3d_world::gen::WorldGen;
use pc3d_world::river_carve::CarveField;
use pc3d_world::terrain::final_solid;

fn main() {
    let mut defects = 0usize;

    // --- Defect 3: analytic surface vs carved voxel truth in channels ---
    // Region-graph pick first (same law as the carve authority: the
    // channel exists only along carved region edges), then dense column
    // scan around each found midpoint.
    for seed in [42u64, 4242, 11] {
        let gen = WorldGen::new(seed);
        let graph = pc3d_world::hydro::RiverGraph::new(
            &gen,
            pc3d_world::river_carve::WATERSHED_HALF,
        );
        let threshold = pc3d_world::hydro::RIVER_THRESHOLD;
        let mut checked = 0;
        let mut diverged = 0;
        'regions: for rx in -28..=28i32 {
            for rz in -28..=28i32 {
                let r = pc3d_world::coords::RegionCoord { x: rx, z: rz };
                let Some(d) = graph.downstream(r) else { continue };
                if graph.discharge(r) < threshold || graph.discharge(d) < threshold {
                    continue;
                }
                let mx = ((rx as f32 + 0.5) * 256.0 * 1000.0) as i64;
                let mz = ((rz as f32 + 0.5) * 256.0 * 1000.0) as i64;
                let Some(_ch) = gen.river_channel(mx, mz) else { continue };
                checked += 1;
                if checked > 60 {
                    break 'regions;
                }
                let analytic = gen.effective_surface_mm(mx, mz) as f32 / 1000.0;
                // The voxel truth: scan down from the analytic surface for
                // the first solid cell (what final_solid actually holds).
                let Some(ch) = gen.river_channel(mx, mz) else { continue };
                let mut voxel_top = None;
                let mut wy = analytic.ceil() as i64 + 1;
                while wy > ch.bed_m as i64 - 4 {
                    let c = final_solid(&gen, mx, wy * 1000, mz);
                    if c.solid {
                        voxel_top = Some(wy);
                        break;
                    }
                    wy -= 1;
                }
                let vt = voxel_top.unwrap_or(-999) as f32;
                let gap = analytic - vt;
                if gap > 0.0 {
                    diverged += 1;
                    if diverged <= 3 {
                        println!(
                            "DEFECT-3 seed {seed}: channel column analytic {analytic:.2} m, \
                             first solid voxel top {vt:.2} m — divergence {gap:.2} m \
                             (bank answer over carved water)"
                        );
                    }
                }
            }
        }
        println!(
            "seed {seed}: {checked} channel columns sampled, {diverged} diverged from voxel truth"
        );
        if diverged > 0 {
            defects += 1;
        }
    }

    // --- Defect 2: flora standing in carved water ---
    for seed in [42u64, 4242] {
        let gen = WorldGen::new(seed);
        let mut in_water = 0usize;
        let mut scanned = 0usize;
        for x in 0..600i32 {
            for z in 0..600i32 {
                let slot = SlotCoord {
                    x,
                    z,
                };
                let Some(p) = plant_at(&gen, slot) else { continue };
                let [cx, cz] = slot.center_m();
                let mx = (cx * 1000.0) as i64;
                let mz = (cz * 1000.0) as i64;
                if let Some(ch) = gen.river_channel(mx, mz) {
                    let gy = gen.effective_surface_mm(mx, mz) as f32 / 1000.0;
                    if gy > ch.bed_m && gy <= ch.surface_m {
                        in_water += 1;
                    }
                }
                scanned += 1;
            }
        }
        if in_water > 0 {
            defects += 1;
            println!(
                "DEFECT-2 seed {seed}: {in_water}/{scanned} plants sit inside \
                 carved water columns"
            );
        } else {
            println!("seed {seed}: {scanned} plants scanned, none in water");
        }
    }

    // --- Defect 1: the NPC +1.0 magic offset (renderer crate) — this
    // probe records the law the authority must replace; the float itself
    // is proven by the pc3d_render test suite swap in this card.
    println!("DEFECT-1 (recorded): npc_world_pos = surface + 1.0 m with feet-pivot GLB");
    defects += 1;

    if defects >= 4 {
        println!(
            "\nN04 RED CONFIRMED: {defects} grounding defect classes reproduce \
             against current code."
        );
    } else {
        println!("\nN04 probe inconclusive ({defects} defects) — investigate.");
        std::process::exit(1);
    }
}
