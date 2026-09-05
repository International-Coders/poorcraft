//! P3D-404: NPC identity, roles, needs, schedule, intent, activity.
//!
//! An NPC is a small state machine on the fixed clock: its day schedule
//! (derived from TimeOfDay ticks) demands Sleep/Work/Idle; the brain turns
//! demands into INTENT (walking a nav path, working at a site, sleeping);
//! needs (hunger up, energy down while working) decay deterministically
//! and restore through eating/sleeping.

use crate::coords::CellCoord;
use crate::nav::NavPatch;

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
        let restore = if !working { ENERGY_RESTORE_SLEEPING } else { 0.0 };
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
            needs: Needs { hunger: 0, energy: 100, hunger_f: 0.0, energy_f: 100.0 },
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
                    self.intent = Intent::Working { site: self.work_site };
                    self.needs.decay(true);
                }
            }
            SchedulePhase::Idle => {
                self.intent = Intent::Idle;
                self.needs.decay(false);
            }
        }
        // Advancing a walk consumes one leg per tick.
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

    fn walk_toward(&mut self, nav: &NavPatch, target: CellCoord) {
        if let Intent::Walking { path, leg } = &mut self.intent {
            if *leg < path.len() {
                self.intent = Intent::Walking { path: path.clone(), leg: *leg };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::WorldGen;
    use crate::coords::PatchCoord;

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
        let mut n = Needs { hunger: 0, energy: 100, hunger_f: 0.0, energy_f: 100.0 };
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
}
