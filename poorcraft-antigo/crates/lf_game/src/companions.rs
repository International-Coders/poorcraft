//! Hireable companions (lore-and-visuals Section B): the relationship
//! economy of trust, morale, and daily wages. The data model + state
//! machine live here; the client drives stepping, rendering, and payment.

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// What a companion is doing right now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CompanionState {
    Idle,
    Following,
    Guarding { pos: [f32; 3] },
    Working,
    Resting,
}

/// A work order (the B3 command set). Targets are block positions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CompanionTask {
    /// Mine the targeted block.
    Mine { target: [i32; 3] },
    /// Chop the nearest tree around a center point.
    Chop { center: [i32; 3] },
    /// Move the companion's cargo to a chest.
    Haul { src: [i32; 3], dst: [i32; 3] },
    /// Craft a recipe from the companion's cargo.
    Craft { recipe_id: String },
    /// Stand watch over an area.
    Guard { area: [i32; 3] },
}

impl CompanionTask {
    pub fn label(&self) -> &'static str {
        match self {
            CompanionTask::Mine { .. } => "MINE",
            CompanionTask::Chop { .. } => "CHOP",
            CompanionTask::Haul { .. } => "HAUL",
            CompanionTask::Craft { .. } => "CRAFT",
            CompanionTask::Guard { .. } => "GUARD",
        }
    }
}

/// Commands from the B3 wheel.
#[derive(Clone, Debug, PartialEq)]
pub enum CompanionCommand {
    FollowMe,
    StayHere { pos: [f32; 3] },
    Rest,
    MineThis { target: [i32; 3] },
    ChopNearby { center: [i32; 3] },
    HaulToChest { src: [i32; 3], dst: [i32; 3] },
    Craft { recipe_id: String },
    GuardArea { area: [i32; 3] },
}

pub const MAX_ACTIVE_COMPANIONS: usize = 3;

/// Morale at/below this refuses Working commands ("I need rest").
pub const MORALE_REFUSE: i32 = 20;
/// Trust where the loyalty badge appears and recipes get shared.
pub const TRUST_BADGE: i32 = 50;
/// Trust where the second task slot unlocks.
pub const TRUST_SECOND_SLOT: i32 = 75;

/// One hired companion. Persisted in the world save alongside mobs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Companion {
    pub id: u64,
    /// Roster archetype (lore/npcs.toml companion entry).
    pub npc_archetype_id: String,
    pub display_name: String,
    /// The faction this companion belongs to (dialogue posture, hire gate).
    pub faction_id: Option<String>,
    /// 0..100 — long-term belief in the player.
    pub trust: i32,
    /// 0..100 — how they feel right now; 0 triggers a quit.
    pub morale: i32,
    /// Items owed per in-game day (checked against the player inventory).
    pub daily_wage: Vec<(String, u8)>,
    pub state: CompanionState,
    pub assigned_task: Option<CompanionTask>,
    /// Second rotation slot, unlocked at trust >= 75.
    #[serde(default)]
    pub second_task: Option<CompanionTask>,
    pub position: Vec3,
    #[serde(default)]
    pub velocity: Vec3,
    #[serde(default)]
    pub yaw: f32,
    pub health: f32,
    /// Pre-hire schedule origin (where they return on dismiss/quit).
    pub home: [f32; 3],
    /// Cargo mined/gathered, hauled to chests or handed to the player.
    #[serde(default)]
    pub cargo: Vec<(String, u8)>,
    /// Progress on the current work action (seconds accumulated).
    #[serde(default)]
    pub work_progress: f32,
    /// Consecutive in-game days without pay.
    #[serde(default)]
    pub days_unpaid: u32,
    /// Fractional morale banked from resting (+5/minute, applied when a
    /// whole point accumulates — per-frame rounding would round to zero).
    #[serde(default)]
    rest_bank: f32,
}

impl Companion {
    pub fn new(
        id: u64,
        archetype_id: &str,
        display_name: String,
        faction_id: Option<String>,
        daily_wage: Vec<(String, u8)>,
        position: Vec3,
    ) -> Self {
        Self {
            id,
            npc_archetype_id: archetype_id.to_string(),
            display_name,
            faction_id,
            trust: 0,
            morale: 50,
            daily_wage,
            state: CompanionState::Following,
            assigned_task: None,
            second_task: None,
            position,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            health: 30.0,
            home: position.to_array(),
            cargo: Vec::new(),
            work_progress: 0.0,
            days_unpaid: 0,
            rest_bank: 0.0,
        }
    }

    pub fn apply_trust(&mut self, delta: i32) {
        self.trust = (self.trust + delta).clamp(0, 100);
    }

    pub fn apply_morale(&mut self, delta: i32) {
        self.morale = (self.morale + delta).clamp(0, 100);
    }

    /// The B3 command set as state transitions. Returns Err with the
    /// companion's refusal line when the command can't be taken (low
    /// morale refuses work).
    pub fn command(&mut self, cmd: &CompanionCommand) -> Result<(), &'static str> {
        match cmd {
            CompanionCommand::FollowMe => {
                self.state = CompanionState::Following;
                self.assigned_task = None;
                Ok(())
            }
            CompanionCommand::StayHere { pos } => {
                self.state = CompanionState::Guarding { pos: *pos };
                self.assigned_task = None;
                Ok(())
            }
            CompanionCommand::Rest => {
                self.state = CompanionState::Resting;
                self.assigned_task = None;
                Ok(())
            }
            CompanionCommand::MineThis { target } => {
                self.work_cmd(CompanionTask::Mine { target: *target })
            }
            CompanionCommand::ChopNearby { center } => {
                self.work_cmd(CompanionTask::Chop { center: *center })
            }
            CompanionCommand::HaulToChest { src, dst } => {
                self.work_cmd(CompanionTask::Haul { src: *src, dst: *dst })
            }
            CompanionCommand::Craft { recipe_id } => {
                self.work_cmd(CompanionTask::Craft { recipe_id: recipe_id.clone() })
            }
            CompanionCommand::GuardArea { area } => {
                self.work_cmd(CompanionTask::Guard { area: *area })
            }
        }
    }

    fn work_cmd(&mut self, task: CompanionTask) -> Result<(), &'static str> {
        if self.morale <= MORALE_REFUSE {
            // refused: they move themselves to Resting instead (B5)
            self.state = CompanionState::Resting;
            return Err("I need rest.");
        }
        self.work_progress = 0.0;
        if self.state == CompanionState::Working && self.trust >= TRUST_SECOND_SLOT {
            // high trust: the new order rotates into the second slot
            if let Some(current) = &self.assigned_task {
                if current != &task && self.second_task.is_none() {
                    self.second_task = Some(task.clone());
                    return Ok(());
                }
            }
        }
        self.assigned_task = Some(task);
        self.state = CompanionState::Working;
        Ok(())
    }

    /// One day passed. `paid` = the player covered today's wage.
    /// Returns the day's consequence for chronicle/chat handling.
    pub fn tick_day(&mut self, paid: bool) -> DayOutcome {
        if paid {
            self.days_unpaid = 0;
            self.apply_trust(1);
            return DayOutcome::Paid;
        }
        self.days_unpaid += 1;
        self.apply_morale(-10);
        if self.morale <= 0 {
            return DayOutcome::Quit;
        }
        DayOutcome::Unpaid
    }

    /// "Pay now": early manual payment.
    pub fn pay_now(&mut self) {
        self.apply_trust(2);
        self.days_unpaid = 0;
    }

    /// Seconds of work needed to finish one unit of the current task.
    pub fn work_duration(&self) -> f32 {
        match &self.assigned_task {
            Some(CompanionTask::Mine { .. }) => 2.5,
            Some(CompanionTask::Chop { .. }) => 2.0,
            Some(CompanionTask::Craft { .. }) => 4.0,
            _ => 1.0,
        }
    }

    /// A single AI step. `solid(x,y,z)` is the world collision query.
    /// Returns an action for the client to apply (damage, mined block,
    /// chat). Physics follows the mob rules: gravity + 1-block hop.
    pub fn step(
        &mut self,
        dt: f32,
        player_pos: Vec3,
        solid: &dyn Fn(i32, i32, i32) -> bool,
        stats: CompanionStats,
    ) -> CompanionAction {
        let mut action = CompanionAction::None;
        // gravity + ground (mirrors MobEntity::update)
        self.velocity.y -= 24.0 * dt;
        let next = self.position + self.velocity * dt;
        if solid(next.x as i32, next.y as i32, next.z as i32) {
            // hop up a single block like every other walker
            if solid(self.position.x as i32, (self.position.y + 0.5) as i32, self.position.z as i32)
                || self.velocity.y > 0.0
            {
                self.velocity.y = 0.0;
            } else {
                self.velocity.y = 8.0;
            }
        } else {
            self.position = next;
        }
        // ground snap: if standing in solid, push up
        if solid(self.position.x as i32, (self.position.y - 0.2) as i32, self.position.z as i32) {
            self.position.y = (self.position.y).ceil();
            self.velocity.y = self.velocity.y.max(0.0);
        }

        match self.state {
            CompanionState::Idle | CompanionState::Resting => {
                if self.state == CompanionState::Resting {
                    // +5 morale per minute of rest (fractional bank)
                    self.rest_bank += 5.0 * dt / 60.0;
                    if self.rest_bank >= 1.0 {
                        let whole = self.rest_bank as i32;
                        self.rest_bank -= whole as f32;
                        self.apply_morale(whole);
                    }
                }
                self.velocity.x *= 0.8;
                self.velocity.z *= 0.8;
            }
            CompanionState::Following => {
                // 2-4 block follow distance: approach past 4, hold in the
                // band, gently back off inside 2 — never cling at 0.
                let to_player = player_pos - self.position;
                let d = to_player.length();
                let wish = if d > 4.0 {
                    to_player.normalize() * stats.speed
                } else if d < 2.0 {
                    -to_player.normalize() * stats.speed * 0.5
                } else {
                    Vec3::ZERO
                };
                self.velocity.x = wish.x;
                self.velocity.z = wish.z;
                if wish.length_squared() > 0.0 {
                    self.yaw = wish.x.atan2(wish.z);
                }
            }
            CompanionState::Guarding { pos } => {
                let anchor = Vec3::from(pos);
                let to_anchor = anchor - self.position;
                let d = to_anchor.length();
                let wish = if d > 3.0 {
                    to_anchor.normalize() * stats.speed
                } else {
                    Vec3::ZERO
                };
                self.velocity.x = wish.x;
                self.velocity.z = wish.z;
            }
            CompanionState::Working => {
                let Some(task) = self.assigned_task.clone() else {
                    self.state = CompanionState::Idle;
                    return action;
                };
                let target_pos = match &task {
                    CompanionTask::Mine { target } => Vec3::new(target[0] as f32, target[1] as f32, target[2] as f32),
                    CompanionTask::Chop { center } => Vec3::new(center[0] as f32, center[1] as f32, center[2] as f32),
                    CompanionTask::Haul { dst, .. } => Vec3::new(dst[0] as f32, dst[1] as f32, dst[2] as f32),
                    CompanionTask::Craft { .. } => self.position,
                    CompanionTask::Guard { area } => Vec3::new(area[0] as f32, area[1] as f32, area[2] as f32),
                };
                let to_target = target_pos - self.position;
                let d = to_target.length();
                if d > 2.0 && !matches!(task, CompanionTask::Craft { .. }) {
                    let wish = to_target.normalize() * stats.speed;
                    self.velocity.x = wish.x;
                    self.velocity.z = wish.z;
                    self.yaw = wish.x.atan2(wish.z);
                } else {
                    self.velocity.x = 0.0;
                    self.velocity.z = 0.0;
                    // at the target: accumulate work
                    self.work_progress += dt;
                    if self.work_progress >= self.work_duration() {
                        self.work_progress = 0.0;
                        match task {
                            CompanionTask::Mine { target } => {
                                action = CompanionAction::Mined(target);
                                self.apply_morale(5); // task completed
                                self.apply_trust(1);
                                // rotate to the second slot if trusted
                                if let Some(second) = self.second_task.take() {
                                    self.assigned_task = Some(second);
                                } else {
                                    self.assigned_task = None;
                                    self.state = CompanionState::Idle;
                                }
                            }
                            CompanionTask::Chop { .. } => {
                                action = CompanionAction::Chopped;
                                self.apply_morale(5);
                                self.apply_trust(1);
                                if let Some(second) = self.second_task.take() {
                                    self.assigned_task = Some(second);
                                } else {
                                    self.assigned_task = None;
                                    self.state = CompanionState::Idle;
                                }
                            }
                            CompanionTask::Haul { .. } => {
                                action = CompanionAction::Hauled;
                                if let Some(second) = self.second_task.take() {
                                    self.assigned_task = Some(second);
                                } else {
                                    self.assigned_task = None;
                                    self.state = CompanionState::Idle;
                                }
                            }
                            CompanionTask::Craft { recipe_id } => {
                                action = CompanionAction::Crafted(recipe_id);
                                if let Some(second) = self.second_task.take() {
                                    self.assigned_task = Some(second);
                                } else {
                                    self.assigned_task = None;
                                    self.state = CompanionState::Idle;
                                }
                            }
                            CompanionTask::Guard { .. } => {}
                        }
                    }
                }
            }
        }
        action
    }

    /// Melee against whatever hurt the player: shared cooldown rule with
    /// mob combat (1s), returns damage when in range.
    pub fn try_attack(&mut self, target: Vec3, stats: CompanionStats, cooldown: &mut f32) -> Option<f32> {
        let d = (target - self.position).length();
        if d < 2.2 && *cooldown <= 0.0 {
            *cooldown = 1.0;
            Some(stats.damage)
        } else {
            None
        }
    }
}

/// Combat/movement profile resolved from the roster archetype.
#[derive(Copy, Clone, Debug)]
pub struct CompanionStats {
    pub speed: f32,
    pub damage: f32,
}

impl Default for CompanionStats {
    fn default() -> Self {
        Self { speed: 3.0, damage: 4.0 }
    }
}

/// What the client should do after a step.
#[derive(Clone, Debug, PartialEq)]
pub enum CompanionAction {
    None,
    Mined([i32; 3]),
    Chopped,
    Hauled,
    Crafted(String),
}

/// Daily wage-day outcome.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DayOutcome {
    Paid,
    Unpaid,
    /// Morale hit zero — the companion quits (B5).
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_companion() -> Companion {
        Companion::new(
            7,
            "accord_warden",
            "Herald Aldis".into(),
            Some("accord".into()),
            vec![("iron_ingot".into(), 8)],
            Vec3::new(0.0, 64.0, 0.0),
        )
    }

    /// Solid floor at y < 64 (grounded world; without a floor the gravity
    /// accumulator drifts the companion out of work range in the mock).
    fn floor_world(_x: i32, y: i32, _z: i32) -> bool {
        y < 64
    }

    /// B1 verify: every field survives a serialize/deserialize round-trip.
    #[test]
    fn companion_round_trips_through_serde() {
        let mut c = test_companion();
        c.trust = 62;
        c.morale = 31;
        c.state = CompanionState::Working;
        c.assigned_task = Some(CompanionTask::Mine { target: [10, 64, -4] });
        c.second_task = Some(CompanionTask::Chop { center: [0, 64, 0] });
        c.position = Vec3::new(12.5, 70.0, -3.0);
        c.velocity = Vec3::new(0.4, -1.0, 0.2);
        c.yaw = 2.25;
        c.health = 28.0;
        c.home = [1.0, 65.0, 2.0];
        c.cargo = vec![("stone".to_string(), 12), ("iron_ore".to_string(), 3)];
        c.work_progress = 1.75;
        c.days_unpaid = 2;

        let bytes = bincode::serialize(&c).unwrap();
        let back: Companion = bincode::deserialize(&bytes).unwrap();
        assert_eq!(back.id, c.id);
        assert_eq!(back.npc_archetype_id, "accord_warden");
        assert_eq!(back.display_name, "Herald Aldis");
        assert_eq!(back.faction_id.as_deref(), Some("accord"));
        assert_eq!(back.trust, 62);
        assert_eq!(back.morale, 31);
        assert_eq!(back.state, CompanionState::Working);
        assert_eq!(back.assigned_task, Some(CompanionTask::Mine { target: [10, 64, -4] }));
        assert_eq!(back.second_task, Some(CompanionTask::Chop { center: [0, 64, 0] }));
        assert_eq!(back.position, c.position);
        assert_eq!(back.velocity, c.velocity);
        assert_eq!(back.yaw, 2.25);
        assert_eq!(back.health, 28.0);
        assert_eq!(back.home, [1.0, 65.0, 2.0]);
        assert_eq!(back.cargo, c.cargo);
        assert_eq!(back.work_progress, 1.75);
        assert_eq!(back.days_unpaid, 2);
        assert_eq!(back.daily_wage, vec![("iron_ingot".to_string(), 8)]);
    }

    #[test]
    fn fresh_companion_starts_following_with_baseline_relationship() {
        let c = test_companion();
        assert_eq!(c.trust, 0);
        assert_eq!(c.morale, 50);
        assert_eq!(c.state, CompanionState::Following);
    }

    /// B3 verify: each command transitions companion state correctly, and
    /// low morale refuses work.
    #[test]
    fn commands_transition_states() {
        let mut c = test_companion();
        assert!(c.command(&CompanionCommand::StayHere { pos: [4.0, 64.0, 4.0] }).is_ok());
        assert_eq!(c.state, CompanionState::Guarding { pos: [4.0, 64.0, 4.0] });
        assert!(c.command(&CompanionCommand::Rest).is_ok());
        assert_eq!(c.state, CompanionState::Resting);
        assert!(c.command(&CompanionCommand::MineThis { target: [8, 64, 8] }).is_ok());
        assert_eq!(c.state, CompanionState::Working);
        assert_eq!(c.assigned_task, Some(CompanionTask::Mine { target: [8, 64, 8] }));
        assert!(c.command(&CompanionCommand::FollowMe).is_ok());
        assert_eq!(c.state, CompanionState::Following);

        // low morale: work refused with the refusal line, rest instead
        c.morale = MORALE_REFUSE;
        let err = c.command(&CompanionCommand::ChopNearby { center: [0, 64, 0] }).unwrap_err();
        assert_eq!(err, "I need rest.");
        assert_eq!(c.state, CompanionState::Resting);
        assert!(c.assigned_task.is_none());
    }

    /// High trust unlocks the second task slot (rotation instead of
    /// replacement).
    #[test]
    fn high_trust_unlocks_second_task_slot() {
        let mut c = test_companion();
        c.trust = TRUST_SECOND_SLOT;
        assert!(c.command(&CompanionCommand::MineThis { target: [8, 64, 8] }).is_ok());
        // a second order while working rotates into the free slot
        assert!(c.command(&CompanionCommand::ChopNearby { center: [9, 64, 9] }).is_ok());
        assert_eq!(c.assigned_task, Some(CompanionTask::Mine { target: [8, 64, 8] }));
        assert_eq!(c.second_task, Some(CompanionTask::Chop { center: [9, 64, 9] }));
        // finishing the first rotates the second in
        let mut stepped = CompanionAction::None;
        while stepped == CompanionAction::None {
            stepped = c.step(0.1, Vec3::new(0.0, 64.0, 0.0), &floor_world, CompanionStats::default());
            c.position = Vec3::new(8.0, 64.0, 8.0); // keep at the mine target
        }
        assert_eq!(stepped, CompanionAction::Mined([8, 64, 8]));
        assert_eq!(c.assigned_task, Some(CompanionTask::Chop { center: [9, 64, 9] }));
        assert_eq!(c.state, CompanionState::Working);
        assert!(c.second_task.is_none());
    }

    /// B4 verify: following keeps a 2-4 block distance — approaches the
    /// player but never overshoots to 0.
    #[test]
    fn following_maintains_standoff_distance() {
        let stats = CompanionStats { speed: 3.4, damage: 5.0 };
        let player = Vec3::new(0.0, 64.0, 0.0);
        let mut c = test_companion();
        c.position = Vec3::new(10.0, 64.0, 0.0);
        for _ in 0..60 {
            c.step(1.0 / 20.0, player, &floor_world, stats);
        }
        let d1 = (player - c.position).length();
        assert!(d1 > 2.0 && d1 <= 4.2, "approached into the band, got {d1}");
        // holding in the band: no further closing
        for _ in 0..60 {
            c.step(1.0 / 20.0, player, &floor_world, stats);
        }
        let d2 = (player - c.position).length();
        assert!((d2 - d1).abs() < 0.6, "steady in the band: {d1} -> {d2}");
        // teleported too close: backs off gently, never negative-through
        c.position = Vec3::new(0.4, 64.0, 0.0);
        for _ in 0..30 {
            c.step(1.0 / 20.0, player, &floor_world, stats);
        }
        let d3 = (player - c.position).length();
        assert!(d3 >= 1.5, "backed off to {d3}");
    }

    /// B5 verify: the morale-zero quit path (state exit + outcome), and
    /// the wage-day trust/morale economy.
    #[test]
    fn wage_days_and_the_quit_path() {
        let mut c = test_companion();
        // paid day: trust grows
        assert_eq!(c.tick_day(true), DayOutcome::Paid);
        assert_eq!(c.trust, 1);
        // unpaid day: morale drops
        assert_eq!(c.tick_day(false), DayOutcome::Unpaid);
        assert_eq!(c.morale, 40);
        assert_eq!(c.days_unpaid, 1);
        // grinding morale to zero quits
        let mut outcome = DayOutcome::Unpaid;
        while outcome == DayOutcome::Unpaid {
            outcome = c.tick_day(false);
        }
        assert_eq!(outcome, DayOutcome::Quit);
        assert_eq!(c.morale, 0);
        // pay now clears the unpaid streak and adds trust
        let mut p = test_companion();
        p.tick_day(false);
        p.pay_now();
        assert_eq!(p.days_unpaid, 0);
        assert_eq!(p.trust, 2);
    }

    /// Resting recovers morale; attacking shares the 1s cooldown rule.
    #[test]
    fn resting_recovers_and_attacks_share_cooldown() {
        let mut c = test_companion();
        c.morale = 10;
        c.state = CompanionState::Resting;
        for _ in 0..20 * 60 {
            c.step(1.0 / 20.0, Vec3::new(50.0, 64.0, 50.0), &floor_world, CompanionStats::default());
        }
        assert!(c.morale > 10, "rest recovered morale: {}", c.morale);

        let mut c2 = test_companion();
        c2.position = Vec3::new(0.0, 64.0, 1.0);
        let foe = Vec3::new(0.0, 64.0, 0.0); // ground level, 1 block away
        let stats = CompanionStats { speed: 3.0, damage: 6.0 };
        let mut cd = 0.0;
        assert!(c2.try_attack(foe, stats, &mut cd).is_some());
        assert!(c2.try_attack(foe, stats, &mut cd).is_none(),
            "cooldown blocks the second swing");
        cd = 0.0;
        assert!(c2.try_attack(foe, stats, &mut cd).is_some());
        // out of range: no attack
        c2.position = Vec3::new(10.0, 64.0, 10.0);
        cd = 0.0;
        assert!(c2.try_attack(foe, stats, &mut cd).is_none());
    }
}
