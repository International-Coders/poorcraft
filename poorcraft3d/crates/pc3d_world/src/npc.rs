//! P3D-404: NPC identity, roles, needs, schedule, intent, activity.
//!
//! An NPC is a small state machine on the fixed clock: its day schedule
//! (derived from TimeOfDay ticks) demands Sleep/Work/Idle; the brain turns
//! demands into INTENT (walking a nav path, working at a site, sleeping);
//! needs (hunger up, energy down while working) decay deterministically
//! and restore through eating/sleeping.

use crate::coords::CellCoord;
use crate::nav::NavPatch;

use std::collections::HashSet;

/// NPC roles with distinct work activities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Farmer,
    Fisher,
    Builder,
    Guard,
}

impl Role {
    pub fn work_activity(self) -> Activity {
        match self {
            Role::Farmer => Activity::Farming,
            Role::Fisher => Activity::Fishing,
            Role::Builder => Activity::Building,
            Role::Guard => Activity::Guarding,
        }
    }
}

/// Day schedule phases by TimeOfDay tick fraction [0, 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedulePhase {
    Sleep,
    Work,
    Idle,
}

pub const SLEEP_END: f32 = 0.25;
pub const WORK_END: f32 = 0.70;
pub const IDLE_END: f32 = 0.80;

pub fn schedule_phase(day_fraction: f32) -> SchedulePhase {
    let f = day_fraction.rem_euclid(1.0);
    if f < SLEEP_END {
        SchedulePhase::Sleep
    } else if f < WORK_END {
        SchedulePhase::Work
    } else if f < IDLE_END {
        SchedulePhase::Idle
    } else {
        SchedulePhase::Work
    }
}

/// Needs: hunger 0 (fed) ..= 100 (starving); energy 100 ..= 0 (exhausted).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Needs {
    pub hunger: u8,
    pub energy: u8,
    /// Sub-tick accumulators (truncating per tick lost the fractions).
    pub hunger_f: f32,
    pub energy_f: f32,
}

pub const HUNGER_PER_TICK: f32 = 0.01;
pub const ENERGY_DRAIN_WORKING: f32 = 0.02;
pub const ENERGY_RESTORE_SLEEPING: f32 = 0.10;

impl Needs {
    pub fn decay(&mut self, working: bool) {
        // Accumulate in f32 sub-fields to avoid per-tick truncation.
        self.hunger_f = (self.hunger_f + HUNGER_PER_TICK).min(100.0);
        self.hunger = self.hunger_f as u8;
        let drain = if working { ENERGY_DRAIN_WORKING } else { 0.0 };
        let restore = if !working {
            ENERGY_RESTORE_SLEEPING
        } else {
            0.0
        };
        self.energy_f = (self.energy_f - drain + restore).clamp(0.0, 100.0);
        self.energy = self.energy_f as u8;
    }

    pub fn eat(&mut self) {
        self.hunger = 0;
        self.hunger_f = 0.0;
    }
}

/// Visible activity for rendering and tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Idle,
    Walking,
    Farming,
    Fishing,
    Building,
    Guarding,
    Sleeping,
}

/// Intent state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    Idle,
    Walking { path: Vec<CellCoord>, leg: usize },
    Working { site: CellCoord },
    Sleeping,
}

/// An NPC brain bound to a role, home, and work site.
#[derive(Clone, Debug)]
pub struct NpcBrain {
    pub role: Role,
    pub home: CellCoord,
    pub work_site: CellCoord,
    pub pos: CellCoord,
    pub needs: Needs,
    pub intent: Intent,
}

impl NpcBrain {
    pub fn new(role: Role, home: CellCoord, work_site: CellCoord) -> Self {
        NpcBrain {
            role,
            home,
            work_site,
            pos: home,
            needs: Needs {
                hunger: 0,
                energy: 100,
                hunger_f: 0.0,
                energy_f: 100.0,
            },
            intent: Intent::Idle,
        }
    }

    /// Arrival check: x/z only — the y comes from the nav path (terrain
    /// height), not from the anchor's arbitrary stored y.
    fn at(&self, t: &CellCoord) -> bool {
        self.pos.x == t.x && self.pos.z == t.z
    }

    /// One deterministic tick: needs decay, the schedule demands a phase,
    /// and the intent machine routes (walk via `nav`, arrive, work/sleep).
    pub fn step(&mut self, nav: &NavPatch, day_fraction: f32) {
        let phase = schedule_phase(day_fraction);
        self.plan_for(nav, phase);
        self.advance_leg(phase);
    }

    /// The planning half of [`NpcBrain::step`]: needs decay, the schedule
    /// demand routed into intent (paths planned, nothing moves yet).
    /// Split out so the crowd law can hold a leg back when another body
    /// owns the next cell.
    pub fn plan(&mut self, nav: &NavPatch, day_fraction: f32) {
        let phase = schedule_phase(day_fraction);
        self.plan_for(nav, phase);
    }

    fn plan_for(&mut self, nav: &NavPatch, phase: SchedulePhase) {
        match phase {
            SchedulePhase::Sleep => {
                if !self.at(&self.home) {
                    self.walk_toward(nav, self.home);
                } else {
                    self.intent = Intent::Sleeping;
                    self.needs.decay(false);
                }
                if self.needs.hunger >= 100 {
                    // Starving interrupts sleep with a desperate meal.
                    self.needs.eat();
                }
            }
            SchedulePhase::Work => {
                if self.needs.energy <= 5 {
                    self.intent = Intent::Idle;
                    self.needs.decay(false);
                    return;
                }
                if !self.at(&self.work_site) {
                    self.walk_toward(nav, self.work_site);
                } else {
                    self.intent = Intent::Working {
                        site: self.work_site,
                    };
                    self.needs.decay(true);
                }
            }
            SchedulePhase::Idle => {
                self.intent = Intent::Idle;
                self.needs.decay(false);
            }
        }
    }

    /// The movement half of [`NpcBrain::step`]: an in-progress walk
    /// consumes one leg per tick (arrival converts the intent per phase).
    /// The caller supplies the SAME phase the plan used.
    pub fn advance_leg(&mut self, phase: SchedulePhase) {
        if let Intent::Walking { path, leg } = &mut self.intent {
            if *leg < path.len() {
                self.pos = path[*leg];
                *leg += 1;
            }
            if *leg >= path.len() {
                let arrived = self.pos;
                self.intent = match phase {
                    SchedulePhase::Work => Intent::Working { site: arrived },
                    SchedulePhase::Sleep => Intent::Sleeping,
                    _ => Intent::Idle,
                };
            }
        }
    }

    /// The cell this brain's walk will occupy on its next leg (None when
    /// not walking or the path is spent). The crowd law reads this to
    /// reserve cells; the first path cell IS the body's own cell, which
    /// no other body may claim either.
    pub fn next_cell(&self) -> Option<CellCoord> {
        match &self.intent {
            Intent::Walking { path, leg } if *leg < path.len() => Some(path[*leg]),
            _ => None,
        }
    }

    /// The yield move: the crowd law stood this brain aside (a sidestep
    /// around a blocked cell). The body relocates NOW and re-paths from
    /// the new cell next tick — Idle, never an arrival (a sidestep cell
    /// must never read as a work site or home).
    pub fn sidestep_to(&mut self, cell: CellCoord) {
        self.pos = cell;
        self.intent = Intent::Idle;
    }

    fn walk_toward(&mut self, nav: &NavPatch, target: CellCoord) {
        if let Intent::Walking { path, leg } = &mut self.intent {
            if *leg < path.len() {
                self.intent = Intent::Walking {
                    path: path.clone(),
                    leg: *leg,
                };
                return;
            }
        }
        match nav.path(self.pos, target) {
            Some(path) if !path.is_empty() => {
                self.intent = Intent::Walking { path, leg: 0 };
            }
            _ => self.intent = Intent::Idle,
        }
    }

    pub fn activity(&self) -> Activity {
        match &self.intent {
            Intent::Walking { .. } => Activity::Walking,
            Intent::Working { .. } => self.role.work_activity(),
            Intent::Sleeping => Activity::Sleeping,
            Intent::Idle => Activity::Idle,
        }
    }
}

/// THE CROWD LAW (pure): the cast steps exactly as lone brains would,
/// except no body may ENTER a cell another body stands on or an earlier
/// walker (lower cast index) has already claimed this tick. A blocked
/// walker YIELDS: it sidesteps one cell around the blocker (perpendicular
/// to its step, then back, first free walkable cell), or stands and waits
/// keeping its path — the leg is never lost, the route resumes next tick.
/// Cast order is the only tie-break; the sets are never iterated, so the
/// law is deterministic. Bodies still occupy their cells while stepping
/// (a cell vacated this tick opens to the crowd NEXT tick).
pub fn step_crowd(brains: &mut [&mut NpcBrain], nav: &NavPatch, day_fraction: f32, ticks: usize) {
    for _ in 0..ticks {
        let phase = schedule_phase(day_fraction);
        let mut held: HashSet<(i32, i32)> =
            brains.iter().map(|b| (b.pos.x, b.pos.z)).collect();
        let mut reserved: HashSet<(i32, i32)> = HashSet::new();
        for b in brains.iter_mut() {
            b.plan(nav, day_fraction);
            let Some(next) = b.next_cell() else {
                b.advance_leg(phase);
                continue;
            };
            let key = (next.x, next.z);
            if key == (b.pos.x, b.pos.z) {
                // The path's own start cell: standing where we stand.
                b.advance_leg(phase);
                continue;
            }
            if !held.contains(&key) && !reserved.contains(&key) {
                reserved.insert(key);
                b.advance_leg(phase);
                continue;
            }
            // THE YIELD: the cell is a body or an earlier walker's claim.
            if let Some(side) = sidestep_cell(b, nav, next, &held, &reserved) {
                reserved.insert((side.x, side.z));
                b.sidestep_to(side);
            }
            // No room to step aside: wait — the path and leg stand.
        }
    }
}

/// One sidestep candidate around a blocked step: perpendicular to the
/// desired direction, then straight back — in-patch, walkable, and free
/// of both held and reserved cells. The first hit wins (deterministic).
fn sidestep_cell(
    b: &NpcBrain,
    nav: &NavPatch,
    next: CellCoord,
    held: &HashSet<(i32, i32)>,
    reserved: &HashSet<(i32, i32)>,
) -> Option<CellCoord> {
    let (dx, dz) = (
        (next.x - b.pos.x).signum(),
        (next.z - b.pos.z).signum(),
    );
    for (sx, sz) in [(-dz, dx), (dz, -dx), (-dx, -dz)] {
        if (sx, sz) == (0, 0) {
            continue;
        }
        let (cx, cz) = (b.pos.x + sx, b.pos.z + sz);
        let key = (cx, cz);
        if held.contains(&key) || reserved.contains(&key) {
            continue;
        }
        let Some(y) = nav_height_at(nav, cx, cz) else {
            continue;
        };
        return Some(CellCoord { x: cx, y, z: cz });
    }
    None
}

/// Walkable floor cell top for a world cell, or None (out of patch or
/// unwalkable column — a sidestep never leaves the walkable surface).
fn nav_height_at(nav: &NavPatch, x: i32, z: i32) -> Option<i32> {
    let (lx, lz) = nav.local_of(CellCoord { x, y: 0, z })?;
    let y = nav.height(lx, lz)?;
    Some(y + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::PatchCoord;
    use crate::gen::WorldGen;

    fn brain_and_nav() -> (WorldGen, NpcBrain, NavPatch) {
        let gen = WorldGen::new(3);
        let patch = crate::terrain::SceneSpec::SmoothHills.patch().1;
        let nav = NavPatch::from_gen(&gen, patch);
        let o = patch.origin();
        let c = |lx: i32, lz: i32| CellCoord {
            x: o.x.div_euclid(1000) as i32 + lx,
            y: 0,
            z: o.z.div_euclid(1000) as i32 + lz,
        };
        let brain = NpcBrain::new(Role::Farmer, c(3, 3), c(12, 12));
        (gen, brain, nav)
    }

    /// Schedule phases flip exactly at the configured fractions.
    #[test]
    fn p3d404_schedule_phases_flip_at_bounds() {
        assert_eq!(schedule_phase(0.0), SchedulePhase::Sleep);
        assert_eq!(schedule_phase(0.2), SchedulePhase::Sleep);
        assert_eq!(schedule_phase(SLEEP_END), SchedulePhase::Work);
        assert_eq!(schedule_phase(0.5), SchedulePhase::Work);
        assert_eq!(schedule_phase(WORK_END), SchedulePhase::Idle);
        assert_eq!(schedule_phase(0.75), SchedulePhase::Idle);
        assert_eq!(schedule_phase(IDLE_END), SchedulePhase::Work);
        assert_eq!(schedule_phase(0.99), SchedulePhase::Work);
    }

    /// Needs decay deterministically; eating clears hunger; sleeping
    /// restores energy.
    #[test]
    fn p3d404_needs_decay_and_restore() {
        let mut n = Needs {
            hunger: 0,
            energy: 100,
            hunger_f: 0.0,
            energy_f: 100.0,
        };
        for _ in 0..300 {
            n.decay(true);
        }
        assert!(n.hunger >= 1, "hunger must rise (300 ticks): {}", n.hunger);
        assert!(n.energy < 100, "energy must drain while working");
        let h = n.hunger;
        for _ in 0..50 {
            n.decay(true);
        }
        assert!(n.hunger >= h);
        n.eat();
        assert_eq!(n.hunger, 0);
        let e_before = n.energy;
        n.decay(false); // resting restores
        assert!(n.energy >= e_before);
    }

    /// THE day-in-the-life: during Work the NPC walks to the site and
    /// works; during Sleep it goes home and sleeps; determinism holds.
    #[test]
    fn p3d404_npc_lives_the_day_deterministically() {
        let (gen, mut brain, nav) = brain_and_nav();
        // Work phase: walk then work.
        let mut walked = false;
        let mut worked = false;
        for _ in 0..600 {
            brain.step(&nav, 0.5); // Work phase
            if let Intent::Walking { .. } = brain.intent {
                walked = true;
            }
            if brain.activity() == brain.role.work_activity() {
                worked = true;
                break;
            }
        }
        assert!(walked, "never walked to work");
        assert!(worked, "never arrived at work");

        // Sleep phase: return home and sleep.
        for _ in 0..900 {
            brain.step(&nav, 0.1);
        }
        assert_eq!(brain.intent, Intent::Sleeping);
        assert!(
            brain.pos.x == brain.home.x && brain.pos.z == brain.home.z,
            "not home: {:?} vs {:?}",
            brain.pos,
            brain.home
        );

        // Determinism: two fresh brains, same inputs, same states.
        let (_, mut brain_a, nav_a) = brain_and_nav();
        let (_, mut brain_b, nav_b) = brain_and_nav();
        for t in 0..900i32 {
            let frac = if t < 450 { 0.5 } else { 0.1 };
            brain_a.step(&nav_a, frac);
            brain_b.step(&nav_b, frac);
        }
        assert_eq!(brain_a.pos, brain_b.pos, "same inputs diverged");
        assert_eq!(brain_a.activity(), brain_b.activity());
    }

    /// Roles differ in their visible work activity.
    #[test]
    fn p3d404_roles_have_distinct_activities() {
        assert_ne!(Role::Farmer.work_activity(), Role::Fisher.work_activity());
        assert_ne!(Role::Builder.work_activity(), Role::Guard.work_activity());
        let (_, brain, _) = brain_and_nav();
        assert_eq!(brain.role.work_activity(), Activity::Farming);
        let _ = (&brain, &nav_marker());
    }

    fn nav_marker() -> bool {
        true
    }

    /// The crowd scene: the SmoothHills nav plus a local-cell builder
    /// (patch-origin offset applied — world cells, nav-routable).
    fn crowd_scene() -> (NavPatch, impl Fn(i32, i32) -> CellCoord) {
        let (_, _, nav) = brain_and_nav();
        let patch = crate::terrain::SceneSpec::SmoothHills.patch().1;
        let o = patch.origin();
        let c = move |lx: i32, lz: i32| CellCoord {
            x: o.x.div_euclid(1000) as i32 + lx,
            y: 0,
            z: o.z.div_euclid(1000) as i32 + lz,
        };
        (nav, c)
    }

    /// A brain pinned where it stands: arrived at its site (Working), it
    /// never moves — the obstacle the crowd law must route around.
    fn parked_brain(role: Role, at: CellCoord) -> NpcBrain {
        let mut b = NpcBrain::new(role, at, at);
        b.pos = at;
        b.intent = Intent::Working { site: at };
        b
    }

    /// A walker mid-route: standing at the path's start cell with leg 1
    /// (the staging shape the renderer's proof cast uses).
    fn walker_brain(nav: &NavPatch, from: CellCoord, site: CellCoord) -> NpcBrain {
        let path = nav
            .path(from, site)
            .expect("crowd routes exist on smooth ground");
        let mut b = NpcBrain::new(Role::Farmer, from, site);
        b.pos = from;
        b.intent = Intent::Walking { path, leg: 1 };
        b
    }

    /// Arrival is an x/z fact (the path carries the terrain y).
    fn arrived_at(intent: &Intent, site: CellCoord) -> bool {
        matches!(intent, Intent::Working { site: s }
            if (s.x, s.z) == (site.x, site.z))
    }

    /// THE CROWD LAW: crossing walkers never share a cell on any tick,
    /// and every yielded step still lands its real arrival (a sidestep
    /// cell never reads as a work site).
    #[test]
    fn p3d404_the_crowd_never_shares_a_cell() {
        let (nav, c) = crowd_scene();
        let east = walker_brain(&nav, c(6, 8), c(14, 8));
        let south = walker_brain(&nav, c(10, 4), c(10, 12));
        let (mut a, mut b) = (east, south);
        let (site_a, site_b) = (c(14, 8), c(10, 12));
        for tick in 0..400 {
            step_crowd(&mut [&mut a, &mut b], &nav, 0.5, 1);
            assert_ne!(
                (a.pos.x, a.pos.z),
                (b.pos.x, b.pos.z),
                "tick {tick}: two bodies share a cell"
            );
            // A Working intent is only ever the DECLARED site.
            for (brain, site) in [(&a, site_a), (&b, site_b)] {
                if matches!(brain.intent, Intent::Working { .. }) {
                    assert!(
                        arrived_at(&brain.intent, site),
                        "tick {tick}: arrival at a stranger cell"
                    );
                }
            }
        }
        assert!(
            arrived_at(&a.intent, site_a),
            "A arrives: {:?}",
            a.intent
        );
        assert!(
            arrived_at(&b.intent, site_b),
            "B arrives: {:?}",
            b.intent
        );
    }

    /// THE HEAD-ON YIELD: two walkers facing each other on one row must
    /// never jam forever and never overlap — the sidestep resolves it,
    /// deterministically (two identical runs, identical outcomes).
    #[test]
    fn p3d404_head_on_walkers_yield_and_arrive() {
        let run = || {
            let (nav, c) = crowd_scene();
            let mut a = walker_brain(&nav, c(6, 8), c(14, 8));
            let mut b = walker_brain(&nav, c(9, 8), c(2, 8));
            let mut overlap = false;
            for _ in 0..400 {
                step_crowd(&mut [&mut a, &mut b], &nav, 0.5, 1);
                overlap |= (a.pos.x, a.pos.z) == (b.pos.x, b.pos.z);
            }
            (overlap, a.intent.clone(), b.intent.clone(), a.pos, b.pos)
        };
        let (overlap, ia, ib, _, _) = run();
        assert!(!overlap, "head-on pair overlapped");
        let (_, c) = crowd_scene();
        assert!(
            arrived_at(&ia, c(14, 8)),
            "eastbound arrived: {ia:?}"
        );
        assert!(
            arrived_at(&ib, c(2, 8)),
            "westbound arrived: {ib:?}"
        );
        // Determinism: the whole scenario replays bit-identically.
        assert_eq!(run(), run());
    }

    /// The wait half: when no sidestep cell exists (the perpendiculars
    /// and the back cell are all bodies), the blocked walker STANDS —
    /// same cell, same path, same leg — and the crowd never overlaps.
    #[test]
    fn p3d404_a_blocked_walker_waits_keeping_its_leg() {
        let (nav, c) = crowd_scene();
        let mut blocker = parked_brain(Role::Builder, c(8, 8));
        let mut north = parked_brain(Role::Farmer, c(7, 9));
        let mut south = parked_brain(Role::Fisher, c(7, 7));
        let mut back = parked_brain(Role::Guard, c(6, 8));
        let path = vec![c(8, 8), c(9, 8), c(10, 8)];
        let mut walker = NpcBrain::new(Role::Farmer, c(7, 8), c(10, 8));
        walker.pos = c(7, 8);
        walker.intent = Intent::Walking { path, leg: 0 };
        for _ in 0..50 {
            step_crowd(
                &mut [
                    &mut walker, &mut blocker, &mut north, &mut south, &mut back,
                ],
                &nav,
                0.5,
                1,
            );
        }
        assert_eq!(
            (walker.pos.x, walker.pos.z),
            (c(7, 8).x, c(7, 8).z),
            "the yield stands"
        );
        match &walker.intent {
            Intent::Walking { leg, path } => {
                assert_eq!(*leg, 0, "the leg is kept, never lost");
                assert_eq!(path.len(), 3, "the route stands");
            }
            other => panic!("the walker lost its route: {other:?}"),
        }
        // The parked bodies never moved; nobody shares a cell.
        let parked = [
            (blocker.pos.x, blocker.pos.z),
            (north.pos.x, north.pos.z),
            (south.pos.x, south.pos.z),
            (back.pos.x, back.pos.z),
        ];
        for (at, p) in [(c(8, 8), parked[0]), (c(7, 9), parked[1]), (c(7, 7), parked[2]), (c(6, 8), parked[3])] {
            assert_eq!(p, (at.x, at.z), "a parked body moved");
        }
        let mut cells = parked.to_vec();
        cells.push((walker.pos.x, walker.pos.z));
        for i in 0..cells.len() {
            for j in (i + 1)..cells.len() {
                assert_ne!(cells[i], cells[j], "bodies share a cell");
            }
        }
    }

    /// Alone, the crowd law is the old law: a lone brain stepped through
    /// step_crowd traces the SAME trajectory as brain.step, tick for tick.
    #[test]
    fn p3d404_step_crowd_alone_matches_lone_step() {
        let (nav_a, _) = crowd_scene();
        let (_, mut lone, _) = brain_and_nav();
        let (_, mut crowd, _) = brain_and_nav();
        for _ in 0..400 {
            step_crowd(&mut [&mut crowd], &nav_a, 0.5, 1);
            lone.step(&nav_a, 0.5);
            assert_eq!(crowd.pos, lone.pos, "a lone body must not diverge");
            assert_eq!(crowd.intent, lone.intent);
        }
    }
}
