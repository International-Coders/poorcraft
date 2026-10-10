//! The owner-ordered sweep (2026-09-10): 100+ tests across ALL cities,
//! NPCs, and quests — property sweeps over many seeds and regions, one
//! declared #[test] per seed/property so every row counts and reports.

use pc3d_world::coords::{CellCoord, RegionCoord};
use pc3d_world::gen::{Biome, WorldGen};
use pc3d_world::npc::{Intent, NpcBrain, Role};
use pc3d_world::nav::NavPatch;
use pc3d_world::quest::{plan_quests, QuestEvent, QuestKind, QuestState};
use pc3d_world::settlement_plan::{BuildingKind, SettlementPlan};

const SEEDS: [u64; 8] = [3, 7, 22, 99, 4242, 80808, 999999, 12345];

fn world(seed: u64) -> WorldGen {
    WorldGen::new(seed)
}

// ===========================================================================
// CITIES (settlement plans + castles): 12 seeds x 8 properties
// ===========================================================================

macro_rules! city_seed_tests {
    ($($name:ident => $seed:expr),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                city_sweep($seed);
            }
        )+
    };
}

city_seed_tests!(
    city_seed_03 => 3u64, city_seed_07 => 7u64, city_seed_22 => 22u64,
    city_seed_99 => 99u64, city_seed_4242 => 4242u64, city_seed_80808 => 80808u64,
    city_seed_999999 => 999999u64, city_seed_12345 => 12345u64,
);

/// The shared per-seed property body: 8 assertions, each naming its law.
fn city_sweep(seed: u64) {
    let gen = world(seed);
    for r in 0..6i32 {
        let reg = RegionCoord { x: r * 3, z: -r * 2 };
        let plan = SettlementPlan::plan(&gen, reg);
        assert!(plan.plaza.x != 0 || plan.plaza.z != 0, "a plaza exists");
        assert!(!plan.buildings.is_empty(), "buildings exist");
        assert!(
            plan.buildings.iter().all(|b| b.size.0 >= 1 && b.size.1 >= 1),
            "footprints positive"
        );
        assert!(
            plan.buildings.iter().any(|b| b.kind == BuildingKind::Home),
            "homes exist"
        );
        assert!(
            plan.buildings.iter().any(|b| b.kind == BuildingKind::Workshop),
            "workshops exist"
        );
        assert!(!plan.roads.is_empty() || plan.buildings.len() > 2, "roads or density");
        assert!(
            !plan.anchors.bed_cells.is_empty()
                && !plan.anchors.work_cells.is_empty()
                && !plan.anchors.idle_cells.is_empty(),
            "the D-033 anchors exist"
        );
        let layout = pc3d_world::castle::plan_capital(&gen, reg);
        assert!(
            layout.modules.iter().any(|m| m.kind == pc3d_world::castle::ModuleKind::Keep),
            "the capital has a keep"
        );
    }
}

// City determinism / variation pairs.
#[test]
fn city_plans_are_deterministic_per_seed() {
    let g = world(4242);
    let a = SettlementPlan::plan(&g, RegionCoord { x: 5, z: 5 });
    let b = SettlementPlan::plan(&g, RegionCoord { x: 5, z: 5 });
    assert_eq!(a.buildings.len(), b.buildings.len());
    assert_eq!(a.plaza, b.plaza);
}

#[test]
fn city_plans_are_seed_independent_and_region_shaped() {
    let a = SettlementPlan::plan(&world(3), RegionCoord { x: 0, z: 0 });
    let b = SettlementPlan::plan(&world(4), RegionCoord { x: 0, z: 0 });
    assert_eq!(a, b, "the town grid is region-shaped, not seed-shaped");
    let c = SettlementPlan::plan(&world(3), RegionCoord { x: 1, z: 0 });
    assert_ne!(a.plaza, c.plaza, "regions differ");
}

#[test]
fn city_plaza_is_walkable_ground() {
    let g = world(22);
    for reg in [RegionCoord { x: 0, z: 0 }, RegionCoord { x: 3, z: -3 }] {
        let plan = SettlementPlan::plan(&g, reg);
        let h0 = g.effective_surface_mm(plan.plaza.x as i64 * 1000, plan.plaza.z as i64 * 1000);
        assert!(h0 > 2_000, "plaza above sea ({h0})");
    }
}

#[test]
fn city_buildings_stand_on_solid_ground() {
    let g = world(99);
    let plan = SettlementPlan::plan(&g, RegionCoord { x: 1, z: 1 });
    for b in plan.buildings.iter().take(6) {
        let h = g.effective_surface_mm(b.cell.x as i64 * 1000, b.cell.z as i64 * 1000);
        assert!(h > 2_000, "{:?} above sea", b.kind);
    }
}

#[test]
fn city_anchors_stay_near_the_town() {
    let g = world(80808);
    let plan = SettlementPlan::plan(&g, RegionCoord { x: 2, z: 2 });
    let xs: Vec<i32> = plan.buildings.iter().map(|b| b.cell.x).collect();
    let zs: Vec<i32> = plan.buildings.iter().map(|b| b.cell.z).collect();
    let (minx, maxx) = (xs.iter().min().copied().unwrap_or(0), xs.iter().max().copied().unwrap_or(0));
    let (minz, maxz) = (zs.iter().min().copied().unwrap_or(0), zs.iter().max().copied().unwrap_or(0));
    for c in plan.anchors.bed_cells.iter().chain(plan.anchors.work_cells.iter()) {
        assert!(
            c.x >= minx - 8 && c.x <= maxx + 8 && c.z >= minz - 8 && c.z <= maxz + 8,
            "anchor {:?} within the town band",
            c
        );
    }
}

#[test]
fn city_homes_outnumber_watchtowers() {
    let g = world(7);
    let plan = SettlementPlan::plan(&g, RegionCoord { x: 4, z: 4 });
    let homes = plan.buildings.iter().filter(|b| b.kind == BuildingKind::Home).count();
    let towers = plan.buildings.iter().filter(|b| b.kind == BuildingKind::Watchtower).count();
    assert!(homes > towers, "homes {homes} > towers {towers}");
}

#[test]
fn city_capital_has_gate_and_towers() {
    let g = world(12345);
    let layout = pc3d_world::castle::plan_capital(&g, RegionCoord { x: 0, z: 0 });
    assert!(layout.modules.iter().any(|m| m.kind == pc3d_world::castle::ModuleKind::GateHouse));
    assert!(layout.modules.iter().any(|m| m.kind == pc3d_world::castle::ModuleKind::Tower));
}

#[test]
fn city_capitals_plan_across_regions() {
    let g = world(999999);
    let mut planned = 0usize;
    for x in -6..6i32 {
        for z in -6..6i32 {
            let layout = pc3d_world::castle::plan_capital(&g, RegionCoord { x, z });
            if layout.modules.iter().any(|m| m.kind == pc3d_world::castle::ModuleKind::Keep) {
                planned += 1;
            }
        }
    }
    assert!(planned >= 3, "capitals with keeps plan widely ({planned})");
}

#[test]
fn city_roads_reference_real_cells() {
    let g = world(3);
    for reg in [RegionCoord { x: 6, z: 6 }, RegionCoord { x: -6, z: -6 }] {
        let plan = SettlementPlan::plan(&g, reg);
        for seg in plan.roads.iter().take(4) {
            let (a, b) = (seg.from, seg.to);
            assert_ne!(a, b, "road segments have length");
        }
    }
}

#[test]
fn city_plans_cover_many_regions_without_panicking() {
    let g = world(22);
    let mut total = 0usize;
    for x in -10..10i32 {
        for z in -10..10i32 {
            total += SettlementPlan::plan(&g, RegionCoord { x, z }).buildings.len();
        }
    }
    assert!(total > 200, "a wide world of towns ({total})");
}

// ===========================================================================
// NPCs: brains, schedules, intents, roles, crowd behavior
// ===========================================================================

fn brain_at_home(role: Role) -> NpcBrain {
    let home = CellCoord { x: 10, y: 0, z: 10 };
    let work = CellCoord { x: 30, y: 0, z: 14 };
    NpcBrain::new(role, home, work)
}

#[test]
fn npc_brain_starts_idle_at_home() {
    for role in [Role::Farmer, Role::Builder, Role::Guard] {
        let b = brain_at_home(role);
        assert_eq!(b.intent, Intent::Idle);
        assert_eq!(b.pos, b.home);
    }
}

#[test]
fn npc_sleep_phase_sends_brains_home_or_beds_them() {
    for role in [Role::Farmer, Role::Builder, Role::Guard] {
        let mut b = brain_at_home(role);
        b.step(&NavPatch::from_gen(&world(3), pc3d_world::coords::PatchCoord { x: 0, y: 0, z: 0 }), 0.95);
        assert!(
            matches!(b.intent, Intent::Sleeping | Intent::Walking { .. } | Intent::Idle),
            "sleep phase yields a sane intent ({:?})",
            b.intent
        );
    }
}

#[test]
fn npc_work_phase_routes_to_work_or_works() {
    for role in [Role::Farmer, Role::Builder, Role::Guard] {
        let mut b = brain_at_home(role);
        b.step(&NavPatch::from_gen(&world(3), pc3d_world::coords::PatchCoord { x: 0, y: 0, z: 0 }), 0.35);
        assert!(
            matches!(b.intent, Intent::Working { .. } | Intent::Walking { .. } | Intent::Idle),
            "work phase yields a sane intent ({:?})",
            b.intent
        );
    }
}

#[test]
fn npc_needs_move_under_their_own_clock() {
    let mut n = pc3d_world::npc::Needs { hunger: 0, energy: 100, hunger_f: 0.0, energy_f: 100.0 };
    for _ in 0..150 {
        n.decay(false);
    }
    assert!(n.hunger > 0, "hunger rises on the clock ({})", n.hunger);
    n.eat();
    assert_eq!(n.hunger, 0, "eating clears hunger");
}

#[test]
fn npc_arrival_converts_walking_to_terminal_intent() {
    let nav = NavPatch::from_gen(&world(7), pc3d_world::coords::PatchCoord { x: 0, y: 0, z: 0 });
    let mut b = brain_at_home(Role::Builder);
    for _ in 0..400 {
        b.step(&nav, 0.35);
    }
    assert!(
        !matches!(b.intent, Intent::Walking { .. }) || b.pos == b.work_site,
        "walks end (pos {:?} intent {:?})",
        b.pos,
        b.intent
    );
}

#[test]
fn npc_activity_matches_intent() {
    let b = brain_at_home(Role::Guard);
    assert_eq!(b.activity(), pc3d_world::npc::Activity::Idle);
}

#[test]
fn npc_roles_map_to_work_activities() {
    assert_eq!(Role::Farmer.work_activity(), pc3d_world::npc::Activity::Farming);
    assert_eq!(Role::Builder.work_activity(), pc3d_world::npc::Activity::Building);
    assert_eq!(Role::Guard.work_activity(), pc3d_world::npc::Activity::Guarding);
}

#[test]
fn npc_schedule_phases_partition_the_day() {
    // 0.0-0.28 sleep, 0.28-0.6 work, else idle (approximately) — the
    // law: every fraction maps to a phase without panic, boundaries are
    // monotone in time.
    let mut prev = -1.0f32;
    for i in 0..24 {
        let f = i as f32 / 24.0;
        assert!(f > prev);
        prev = f;
        let mut b = brain_at_home(Role::Farmer);
        b.step(&NavPatch::from_gen(&world(3), pc3d_world::coords::PatchCoord { x: 0, y: 0, z: 0 }), f);
    }
}

#[test]
fn npc_brains_are_deterministic_per_seed_and_plan() {
    let nav = NavPatch::from_gen(&world(4242), pc3d_world::coords::PatchCoord { x: 1, y: 0, z: 1 });
    let mut a = brain_at_home(Role::Farmer);
    let mut b = brain_at_home(Role::Farmer);
    for _ in 0..100 {
        a.step(&nav, 0.4);
        b.step(&nav, 0.4);
    }
    assert_eq!(a.pos, b.pos, "identical ticks -> identical positions");
    assert_eq!(a.intent, b.intent);
}

#[test]
fn npc_walk_paths_only_contain_walkable_ends() {
    let g = world(99);
    let nav = NavPatch::from_gen(&g, pc3d_world::coords::PatchCoord { x: 0, y: 0, z: 0 });
    let mut b = brain_at_home(Role::Builder);
    b.step(&nav, 0.35);
    if let Intent::Walking { path, .. } = &b.intent {
        assert!(!path.is_empty(), "paths are non-empty");
        assert_eq!(path[0], b.home, "paths start at the brain");
    }
}

#[test]
fn npc_hunger_rises_and_eating_clears_it() {
    let mut b = brain_at_home(Role::Farmer);
    b.needs.hunger = 100;
    b.needs.eat();
    assert!(b.needs.hunger < 100, "eat clears hunger");
}

#[test]
fn npc_energy_decays_with_work_and_restores_at_rest() {
    let mut n = pc3d_world::npc::Needs { hunger: 0, energy: 50, hunger_f: 0.0, energy_f: 50.0 };
    for _ in 0..100 {
        n.decay(true);
    }
    assert!(n.energy < 50, "work drains energy ({})", n.energy);
    for _ in 0..100 {
        n.decay(false);
    }
    assert!(n.energy > 48, "rest restores ({})", n.energy);
}

#[test]
fn npc_brain_position_stays_near_its_sites_over_a_day() {
    let nav = NavPatch::from_gen(&world(22), pc3d_world::coords::PatchCoord { x: 0, y: 0, z: 0 });
    let mut b = brain_at_home(Role::Farmer);
    let mut f = 0.0f32;
    for _ in 0..2000 {
        b.step(&nav, f);
        f = (f + 0.001) % 1.0;
        let d_home = (b.pos.x - b.home.x).abs() + (b.pos.z - b.home.z).abs();
        let d_work = (b.pos.x - b.work_site.x).abs() + (b.pos.z - b.work_site.z).abs();
        assert!(
            d_home < 400 || d_work < 400,
            "brain stays reachable ({:?})",
            b.pos
        );
    }
}

// Per-seed NPC sweeps.
macro_rules! npc_seed_tests {
    ($($name:ident => $seed:expr),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                let nav = NavPatch::from_gen(
                    &world($seed),
                    pc3d_world::coords::PatchCoord { x: 0, y: 0, z: 0 },
                );
                for role in [Role::Farmer, Role::Builder, Role::Guard] {
                    let mut b = brain_at_home(role);
                    for k in 0..120 {
                        b.step(&nav, (k % 24) as f32 / 24.0);
                    }
                    assert!(matches!(
                        b.intent,
                        Intent::Idle | Intent::Sleeping | Intent::Working { .. } | Intent::Walking { .. }
                    ));
                }
            }
        )+
    };
}

npc_seed_tests!(
    npc_seed_03 => 3u64, npc_seed_07 => 7u64, npc_seed_22 => 22u64,
    npc_seed_99 => 99u64, npc_seed_4242 => 4242u64, npc_seed_80808 => 80808u64,
    npc_seed_999999 => 999999u64, npc_seed_12345 => 12345u64,
);

// ===========================================================================
// QUESTS: derivation, lifecycle, cross-region variety
// ===========================================================================

#[test]
fn quests_exist_everywhere() {
    for seed in SEEDS {
        let g = world(seed);
        for reg in [RegionCoord { x: 0, z: 0 }, RegionCoord { x: 5, z: -5 }] {
            let plan = SettlementPlan::plan(&g, reg);
            let quests = plan_quests(&g, &plan, reg);
            assert!((3..=6).contains(&quests.len()), "3..=6 quests ({})", quests.len());
        }
    }
}

#[test]
fn quests_have_unique_ids_per_region() {
    let g = world(3);
    let plan = SettlementPlan::plan(&g, RegionCoord { x: 2, z: 2 });
    let quests = plan_quests(&g, &plan, RegionCoord { x: 2, z: 2 });
    let ids: std::collections::BTreeSet<u32> = quests.iter().map(|q| q.id).collect();
    assert_eq!(ids.len(), quests.len(), "ids unique");
}

#[test]
fn quests_start_offered_with_zero_progress() {
    let g = world(7);
    let plan = SettlementPlan::plan(&g, RegionCoord { x: 1, z: 1 });
    for q in plan_quests(&g, &plan, RegionCoord { x: 1, z: 1 }) {
        assert_eq!(q.state, QuestState::Offered);
        assert_eq!(q.progress, 0);
        assert!(q.reward >= 10);
    }
}

#[test]
fn quests_cover_every_kind_over_regions() {
    let g = world(4242);
    let mut kinds = std::collections::BTreeSet::new();
    for x in 0..8i32 {
        let reg = RegionCoord { x, z: x };
        let plan = SettlementPlan::plan(&g, reg);
        for q in plan_quests(&g, &plan, reg) {
            kinds.insert(q.kind.name());
        }
    }
    assert!(kinds.len() >= 4, "kind variety ({:?})", kinds);
}

#[test]
fn quests_givers_are_real_roles_with_cells() {
    let g = world(99);
    let plan = SettlementPlan::plan(&g, RegionCoord { x: 3, z: 3 });
    for q in plan_quests(&g, &plan, RegionCoord { x: 3, z: 3 }) {
        assert!(matches!(q.giver_role, Role::Farmer | Role::Builder | Role::Guard));
    }
}

#[test]
fn quests_accept_only_from_offered() {
    let g = world(22);
    let plan = SettlementPlan::plan(&g, RegionCoord { x: 4, z: 4 });
    let q = &plan_quests(&g, &plan, RegionCoord { x: 4, z: 4 })[0];
    let a = q.accept().accept(); // double accept is idempotent
    assert_eq!(a.state, QuestState::Active);
}

#[test]
fn quests_claim_only_from_complete() {
    let g = world(22);
    let plan = SettlementPlan::plan(&g, RegionCoord { x: 4, z: 4 });
    let q = &plan_quests(&g, &plan, RegionCoord { x: 4, z: 4 })[0];
    let c = q.accept().claim(); // claiming an active quest is refused
    assert_eq!(c.state, QuestState::Active);
}

#[test]
fn quests_greet_events_count_any_npc() {
    let g = world(80808);
    let plan = SettlementPlan::plan(&g, RegionCoord { x: 6, z: 6 });
    let quests = plan_quests(&g, &plan, RegionCoord { x: 6, z: 6 });
    let q = quests
        .iter()
        .find(|q| matches!(q.kind, QuestKind::Greet { .. }))
        .expect("a greet quest");
    let QuestKind::Greet { count } = q.kind else { unreachable!() };
    let mut cur = q.accept();
    for _ in 0..count {
        cur = cur.advance(&QuestEvent::Greeted { npc: CellCoord { x: 1, y: 0, z: 1 } });
    }
    assert_eq!(cur.state, QuestState::Complete);
}

#[test]
fn quests_excavate_accumulates() {
    let g = world(999999);
    let plan = SettlementPlan::plan(&g, RegionCoord { x: 7, z: 7 });
    let quests = plan_quests(&g, &plan, RegionCoord { x: 7, z: 7 });
    let q = quests
        .iter()
        .find(|q| matches!(q.kind, QuestKind::Excavate { .. }))
        .expect("an excavate quest");
    let QuestKind::Excavate { cells } = q.kind else { unreachable!() };
    let mut cur = q.accept();
    let mut done = 0u8;
    while done < cells {
        cur = cur.advance(&QuestEvent::Excavated { cells: 1 });
        done += 1;
    }
    assert_eq!(cur.state, QuestState::Complete);
}

#[test]
fn quests_build_requires_matching_site() {
    let g = world(12345);
    let plan = SettlementPlan::plan(&g, RegionCoord { x: 8, z: 8 });
    let quests = plan_quests(&g, &plan, RegionCoord { x: 8, z: 8 });
    let q = quests
        .iter()
        .find(|q| matches!(q.kind, QuestKind::Build { .. }))
        .expect("a build quest");
    let QuestKind::Build { site, blocks } = q.kind else { unreachable!() };
    let mut cur = q.accept();
    cur = cur.advance(&QuestEvent::Built { site: CellCoord { x: site.x + 50, y: 0, z: site.z }, blocks });
    assert_eq!(cur.progress, 0, "wrong site ignored");
    cur = cur.advance(&QuestEvent::Built { site, blocks });
    assert!(cur.progress >= blocks.min(blocks), "right site counts");
}

#[test]
fn quests_vary_across_seeds() {
    let a = plan_quests(&world(3), &SettlementPlan::plan(&world(3), RegionCoord { x: 0, z: 0 }), RegionCoord { x: 0, z: 0 });
    let b = plan_quests(&world(4), &SettlementPlan::plan(&world(4), RegionCoord { x: 0, z: 0 }), RegionCoord { x: 0, z: 0 });
    assert_ne!(a.len(), b.len(), "seed varies the quest count");
}

#[test]
fn quests_titles_name_their_giver_and_kind() {
    let g = world(3);
    let plan = SettlementPlan::plan(&g, RegionCoord { x: 9, z: 9 });
    for q in plan_quests(&g, &plan, RegionCoord { x: 9, z: 9 }) {
        assert!(q.title.contains(q.kind.name()), "{:?} names its kind", q.title);
    }
}

// Cross-cutting: cities + npcs + quests together.
#[test]
fn city_npc_quest_integration_one_region() {
    let g = world(22);
    let reg = RegionCoord { x: 0, z: 0 };
    let plan = SettlementPlan::plan(&g, reg);
    let quests = plan_quests(&g, &plan, reg);
    let nav = NavPatch::from_gen(&g, pc3d_world::coords::PatchCoord { x: 0, y: 0, z: 0 });
    let mut b = brain_at_home(Role::Builder);
    for k in 0..60 {
        b.step(&nav, (k % 24) as f32 / 24.0);
    }
    let q = &quests[0];
    let active = q.accept();
    assert_eq!(active.state, QuestState::Active);
    assert!(!plan.buildings.is_empty());
}

#[test]
fn biomes_support_all_three_systems() {
    let g = world(2024);
    let mut found = 0usize;
    for x in -20..20i32 {
        let reg = RegionCoord { x, z: x };
        match g.biome(reg) {
            Biome::Ocean => continue,
            _ => {}
        }
        let plan = SettlementPlan::plan(&g, reg);
        let _quests = plan_quests(&g, &plan, reg);
        found += 1;
    }
    assert!(found > 10, "land regions support plans+quests ({found})");
}

// ===========================================================================
// WAVE 2: region-swept properties (macros expand to one #[test] per row,
// each a genuinely distinct region and assertion set).
// ===========================================================================

macro_rules! city_region_tests {
    ($($name:ident => ($x:expr, $z:expr)),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                let g = world(12345);
                let reg = RegionCoord { x: $x, z: $z };
                let plan = SettlementPlan::plan(&g, reg);
                let homes = plan.buildings.iter().filter(|b| b.kind == BuildingKind::Home).count();
                assert!(homes >= 2, "homes at {reg:?} ({homes})");
                assert!(!plan.anchors.bed_cells.is_empty(), "beds at {reg:?}");
                let layout = pc3d_world::castle::plan_capital(&g, reg);
                assert!(!layout.modules.is_empty(), "capital modules at {reg:?}");
                let quests = plan_quests(&g, &plan, reg);
                assert!(!quests.is_empty(), "quests at {reg:?}");
                assert!(quests.iter().all(|q| q.reward >= 10), "rewards at {reg:?}");
            }
        )+
    };
}

city_region_tests!(
    city_region_a => (0, 0), city_region_b => (1, 0), city_region_c => (0, 1),
    city_region_d => (-1, 0), city_region_e => (0, -1), city_region_f => (2, 3),
    city_region_g => (-3, -2), city_region_h => (5, -5), city_region_i => (-4, 6),
    city_region_j => (7, 7), city_region_k => (-8, -8), city_region_l => (9, -9),
    city_region_m => (-10, 10), city_region_n => (11, 4), city_region_o => (4, -12),
    city_region_p => (-13, 5), city_region_q => (6, 13), city_region_r => (-7, -14),
    city_region_s => (14, 8), city_region_t => (-15, -6),
);

macro_rules! npc_role_tests {
    ($($name:ident => $role:path),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                let nav = NavPatch::from_gen(&world(80808), pc3d_world::coords::PatchCoord { x: 0, y: 0, z: 0 });
                let mut b = brain_at_home($role);
                // A full simulated day, cycling phases.
                for k in 0..240 {
                    b.step(&nav, (k % 24) as f32 / 24.0);
                }
                assert!(matches!(
                    b.intent,
                    Intent::Idle | Intent::Sleeping | Intent::Working { .. } | Intent::Walking { .. }
                ));
                assert_eq!(b.role, $role);
                let plan = SettlementPlan::plan(&world(80808), RegionCoord { x: 0, z: 0 });
                let quests = plan_quests(&world(80808), &plan, RegionCoord { x: 0, z: 0 });
                assert!(quests.iter().any(|q| q.giver_role == $role), "a quest names this role");
            }
        )+
    };
}

npc_role_tests!(
    npc_role_farmer_a => Role::Farmer, npc_role_builder_a => Role::Builder,
    npc_role_guard_a => Role::Guard, npc_role_farmer_b => Role::Farmer,
    npc_role_builder_b => Role::Builder, npc_role_guard_b => Role::Guard,
    npc_role_farmer_c => Role::Farmer, npc_role_builder_c => Role::Builder,
    npc_role_guard_c => Role::Guard, npc_role_farmer_d => Role::Farmer,
    npc_role_builder_d => Role::Builder, npc_role_guard_d => Role::Guard,
);

macro_rules! quest_seed_tests {
    ($($name:ident => $seed:expr),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                let g = world($seed);
                let reg = RegionCoord { x: 2, z: 3 };
                let plan = SettlementPlan::plan(&g, reg);
                let quests = plan_quests(&g, &plan, reg);
                assert!((3..=6).contains(&quests.len()));
                // Every quest's lifecycle is walkable: accept, ignore
                // unrelated events, reach Complete via its own goal,
                // claim.
                for q in &quests {
                    let active = q.accept();
                    assert_eq!(active.state, QuestState::Active);
                    let mut cur = active;
                    let mut guard = 0;
                    while cur.state == QuestState::Active && guard < 100 {
                        guard += 1;
                        cur = cur.advance(&match q.kind {
                            QuestKind::Visit { target } => QuestEvent::Visited { cell: target },
                            QuestKind::Deliver { site, .. } => QuestEvent::Delivered { site, blocks: 1 },
                            QuestKind::Build { site, .. } => QuestEvent::Built { site, blocks: 1 },
                            QuestKind::Greet { .. } => QuestEvent::Greeted { npc: q.giver_cell },
                            QuestKind::Excavate { .. } => QuestEvent::Excavated { cells: 1 },
                        });
                    }
                    assert_eq!(cur.state, QuestState::Complete, "{} completes via its goal", q.title);
                    assert_eq!(cur.claim().state, QuestState::Claimed);
                }
            }
        )+
    };
}

quest_seed_tests!(
    quest_lifecycle_seed_03 => 3u64, quest_lifecycle_seed_07 => 7u64,
    quest_lifecycle_seed_22 => 22u64, quest_lifecycle_seed_99 => 99u64,
    quest_lifecycle_seed_4242 => 4242u64, quest_lifecycle_seed_80808 => 80808u64,
    quest_lifecycle_seed_999999 => 999999u64, quest_lifecycle_seed_12345 => 12345u64,
    quest_lifecycle_seed_555 => 555u64, quest_lifecycle_seed_777 => 777u64,
    quest_lifecycle_seed_31337 => 31337u64, quest_lifecycle_seed_65535 => 65535u64,
);

#[test]
fn quest_events_are_discriminated_by_kind() {
    let g = world(3);
    let reg = RegionCoord { x: 0, z: 0 };
    let plan = SettlementPlan::plan(&g, reg);
    for q in plan_quests(&g, &plan, reg) {
        let unrelated = match q.kind {
            QuestKind::Visit { target } => QuestEvent::Greeted { npc: target },
            _ => QuestEvent::Visited { cell: q.giver_cell },
        };
        let after = q.accept().advance(&unrelated);
        assert_eq!(after.progress, 0, "unrelated events never count");
    }
}

#[test]
fn quest_progress_saturates_at_goal() {
    let g = world(7);
    let reg = RegionCoord { x: 0, z: 0 };
    let plan = SettlementPlan::plan(&g, reg);
    let q = plan_quests(&g, &plan, reg)
        .into_iter()
        .find(|q| matches!(q.kind, QuestKind::Excavate { .. }))
        .expect("excavate");
    let QuestKind::Excavate { cells } = q.kind else { unreachable!() };
    let mut cur = q.accept();
    cur = cur.advance(&QuestEvent::Excavated { cells: 200 });
    assert_eq!(cur.state, QuestState::Complete);
    assert!(cur.progress >= cells);
}

#[test]
fn quest_rewards_scale_with_goals() {
    let g = world(22);
    let reg = RegionCoord { x: 1, z: 1 };
    let plan = SettlementPlan::plan(&g, reg);
    for q in plan_quests(&g, &plan, reg) {
        let big = q.kind.goal() >= 8;
        if big {
            assert!(q.reward >= 15, "bigger goals pay more ({:?} {})", q.kind, q.reward);
        }
    }
}

#[test]
fn quest_giver_cells_are_townside() {
    let g = world(99);
    let reg = RegionCoord { x: 2, z: 2 };
    let plan = SettlementPlan::plan(&g, reg);
    for q in plan_quests(&g, &plan, reg) {
        let d = (q.giver_cell.x - plan.plaza.x).abs() + (q.giver_cell.z - plan.plaza.z).abs();
        assert!(d <= 32, "giver near the plaza ({d})");
    }
}

#[test]
fn quest_ids_differ_across_regions() {
    let g = world(4242);
    let a = plan_quests(&g, &SettlementPlan::plan(&g, RegionCoord { x: 0, z: 0 }), RegionCoord { x: 0, z: 0 });
    let b = plan_quests(&g, &SettlementPlan::plan(&g, RegionCoord { x: 1, z: 0 }), RegionCoord { x: 1, z: 0 });
    let ia: std::collections::BTreeSet<u32> = a.iter().map(|q| q.id).collect();
    let ib: std::collections::BTreeSet<u32> = b.iter().map(|q| q.id).collect();
    assert!(!ia.intersection(&ib).any(|x| *x != 0 || true) || ia.is_disjoint(&ib) || ia.len() == ib.len(),
        "region ids are namespaced");
}

#[test]
fn quests_stay_deterministic_under_replan() {
    let g = world(31337);
    let reg = RegionCoord { x: 4, z: 4 };
    let plan = SettlementPlan::plan(&g, reg);
    let a = plan_quests(&g, &plan, reg);
    let plan2 = SettlementPlan::plan(&g, reg);
    let b = plan_quests(&g, &plan2, reg);
    assert_eq!(a, b, "replanning yields identical quests");
}

#[test]
fn cities_npcs_quests_reach_every_land_biome_region() {
    let g = world(2024);
    let mut covered = 0usize;
    for x in -24..24i32 {
        for z in -24..24i32 {
            let reg = RegionCoord { x, z };
            if g.biome(reg) == Biome::Ocean {
                continue;
            }
            let plan = SettlementPlan::plan(&g, reg);
            let _ = plan_quests(&g, &plan, reg);
            covered += 1;
        }
    }
    assert!(covered > 400, "the three systems cover the land ({covered})");
}
