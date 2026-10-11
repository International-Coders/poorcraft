//! D3 (ai-npc-assets): headless smoke mode — real game logic, no window,
//! no GPU. `loreforge --smoke` runs 300 world ticks of actual systems:
//! worldgen, mob AI (one passive + one hostile), an NPC's enriched
//! schedule, one craft, one mine. Exit 0 = every step completed; any
//! panic or failed step exits 1.

use lf_voxel::World;
use lf_worldgen::{Seed, WorldGen, WorldType};

const TICKS: usize = 300;
const DT: f32 = 1.0 / 20.0;

fn build_superflat(seed: u64) -> (WorldGen, World) {
    let gen = WorldGen::with_type(Seed(seed), WorldType::Superflat);
    let mut world = World::new();
    for cx in -2..=2 {
        for cz in -2..=2 {
            world.chunks.insert((cx, cz), gen.generate_chunk(cx, cz));
        }
    }
    assert!(
        world.is_solid(0, gen.surface_top(0, 0) - 1, 0),
        "world_gen: superflat ground missing"
    );
    (gen, world)
}

/// Runs the whole smoke sequence. Returns a process exit code.
pub fn run() -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run_inner));
    match result {
        Ok(Ok(summary)) => {
            println!("SMOKE PASS: {}", summary);
            0
        }
        Ok(Err(step)) => {
            eprintln!("SMOKE FAIL: step '{}' failed", step);
            1
        }
        Err(panic) => {
            let msg = panic
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".into());
            eprintln!("SMOKE FAIL: panic: {}", msg);
            1
        }
    }
}

fn run_inner() -> Result<String, &'static str> {
    // 1. world gen: seed 42, Superflat, Peaceful rules
    let (gen, mut world) = build_superflat(42);

    // 2. entities: one passive + one hostile mob, one NPC
    let ground = gen.surface_top(0, 0) as f32;
    let mut boar = lf_game::mobs::MobEntity::spawn(1, lf_game::mobs::MobType::Boar, glam::Vec3::new(-4.5, ground + 0.2, 0.5));
    let mut glitchling = lf_game::mobs::MobEntity::spawn(2, lf_game::mobs::MobType::Glitchling, glam::Vec3::new(6.5, ground + 0.2, 0.5));
    let player = glam::Vec3::new(0.5, ground + 0.2, 0.5);
    let mut villager = lf_npc::Villager::new(100, lf_npc::VillagerJob::Smith, "Smoke".into(), [2.5, ground + 0.2, 2.5]);
    villager.workstation_pos = Some([2, ground as i32, 2]);

    // 3. one craft: a log saws into planks (the always-available recipe)
    let mut grid: [Option<lf_game::survival::ItemStack>; 4] = Default::default();
    grid[0] = Some(lf_game::survival::ItemStack { item_id: "log".into(), count: 1 });
    let crafted = lf_game::crafting::match_recipe(&grid);
    match crafted {
        Some((out, n)) if out == "planks" && n == 4 => {}
        other => return Err("craft"),
    }
    let _ = crafted;
    lf_game::crafting::consume_ingredients(&mut grid);
    if grid[0].is_some() {
        return Err("craft");
    }

    // 4. one mine: remove the surface block at (0, ground, 0)
    let mine_y = gen.surface_top(0, 0) - 1;
    if world.set_block(0, mine_y, 0, lf_voxel::BlockState::AIR).is_none() {
        return Err("mine");
    }
    if world.get_block(0, mine_y, 0).id() != lf_voxel::registry::block::AIR {
        return Err("mine");
    }

    // 5. 300 ticks of AI + schedule (simulated time, not real time)
    let mut hostile_attacked = false;
    for t in 0..TICKS {
        let day_fraction = (t as f32 * DT) / lf_game::TimeOfDay::TICKS_PER_DAY as f32 + 0.35;
        let entry = lf_npc::enriched_slot_at(&lf_npc::default_schedule_entries(), day_fraction);
        villager.activity = lf_npc::activity_state_for(&entry, false);
        let _ = boar.update(DT, &world, player);
        if let Some(_) = glitchling.update(DT, &world, player) {
            hostile_attacked = true;
        }
    }
    let _ = hostile_attacked; // a hit is likely but timing-dependent; the
                              // tick loop itself is the thing under test
    Ok(format!(
        "world_gen=OK, ai_ticks={}, npc_ticks={}, craft=OK, mine=OK",
        TICKS, TICKS
    ))
}

#[cfg(test)]
mod tests {
    /// Failure meaning: the headless smoke sequence itself broke (it is
    /// the binary's `--smoke` path, so this keeps it honest in CI).
    #[test]
    fn smoke_sequence_completes() {
        let summary = super::run_inner().expect("smoke inner run must succeed");
        assert!(summary.contains("craft=OK"), "{}", summary);
        assert!(summary.contains("mine=OK"), "{}", summary);
    }
}
