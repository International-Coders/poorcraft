use serde::{Deserialize, Serialize};

use glam::Vec3;
use lf_voxel::World;

use crate::mob_pathfind::{self, BlockPos, CachedPath};
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MobType {
    // Passive
    Boar,
    Woolbeast,
    // Hostile
    Glitchling,
    Stalker,
    Crawler,
    // Boss
    NullKnight,
    /// P36: roost boss — never rolls naturally; the client settles them
    /// at mountain roosts. Rendered multi-part on the client.
    Dragon,
    /// lore-and-visuals: The Nameless' raiders — spawn near nameless
    /// camps, pay in food, carry stolen archive pages.
    NamelessRaider,
    // king-quest C: animals (multi-part cube rendering, own spawn rules)
    Chicken,
    Wolf,
    Dog,
    Bear,
}

impl MobType {
    pub fn is_hostile(self) -> bool {
        matches!(self, MobType::Glitchling | MobType::Stalker | MobType::Crawler | MobType::NullKnight
            | MobType::NamelessRaider | MobType::Wolf | MobType::Bear)
    }

    /// Boss-tier mobs own their AI elsewhere (the dragon flies via
    /// `DragonBrain`). The generic behaviour machine never drives them.
    /// The NullKnight keeps the generic machine for now — it has no boss
    /// brain yet, and freezing it solid would be a regression, not a
    /// "boss phase" (full boss AI is its own task).
    pub fn use_boss_ai(self) -> bool {
        matches!(self, MobType::Dragon)
    }

    /// B3: mobs can belong to a lore faction; the player's standing with
    /// that faction widens or calms their aggro radius.
    pub fn faction(self) -> Option<&'static str> {
        match self {
            MobType::NamelessRaider => Some("nameless"),
            _ => None,
        }
    }

    pub fn stats(self) -> MobStats {
        use MobType::*;
        match self {
            Boar => MobStats { max_health: 10.0, damage: 0.0, speed: 2.0, size: 0.7, detect: 0.0 },
            Woolbeast => MobStats { max_health: 15.0, damage: 0.0, speed: 1.8, size: 0.8, detect: 0.0 },
            Glitchling => MobStats { max_health: 20.0, damage: 4.0, speed: 3.2, size: 0.6, detect: 16.0 },
            Stalker => MobStats { max_health: 30.0, damage: 6.0, speed: 2.8, size: 0.8, detect: 20.0 },
            Crawler => MobStats { max_health: 15.0, damage: 3.0, speed: 3.6, size: 0.5, detect: 14.0 },
            NullKnight => MobStats { max_health: 250.0, damage: 15.0, speed: 2.2, size: 1.4, detect: 32.0 },
            Dragon => MobStats { max_health: 400.0, damage: 18.0, speed: 6.0, size: 2.2, detect: 32.0 },
            NamelessRaider => MobStats { max_health: 22.0, damage: 5.0, speed: 3.0, size: 0.45, detect: 18.0 },
            Chicken => MobStats { max_health: 4.0, damage: 0.0, speed: 1.6, size: 0.3, detect: 0.0 },
            Wolf => MobStats { max_health: 14.0, damage: 3.0, speed: 3.4, size: 0.45, detect: 14.0 },
            Dog => MobStats { max_health: 10.0, damage: 0.0, speed: 2.2, size: 0.45, detect: 0.0 },
            Bear => MobStats { max_health: 40.0, damage: 8.0, speed: 2.6, size: 0.9, detect: 10.0 },
        }
    }

    pub fn color(self) -> [f32; 3] {
        use MobType::*;
        match self {
            Boar => [0.55, 0.4, 0.3],
            Woolbeast => [0.9, 0.9, 0.85],
            Glitchling => [0.4, 0.9, 0.5],
            Stalker => [0.25, 0.25, 0.3],
            Crawler => [0.6, 0.3, 0.3],
            NullKnight => [0.15, 0.1, 0.25],
            Dragon => [0.45, 0.12, 0.1],
            NamelessRaider => [0.12, 0.12, 0.13],
            Chicken => [0.94, 0.92, 0.86],
            Wolf => [0.58, 0.58, 0.6],
            Dog => [0.62, 0.46, 0.3],
            Bear => [0.36, 0.26, 0.18],
        }
    }

    /// Items dropped on death.
    pub fn drops(self) -> &'static [(&'static str, u8)] {
        use MobType::*;
        match self {
            Boar => &[("porkchop", 2)],
            Woolbeast => &[("mutton", 1)],
            Glitchling => &[("glitch_dust", 1)],
            Stalker => &[("glitch_dust", 2)],
            Crawler => &[("glitch_dust", 1)],
            NullKnight => &[("iron_ingot", 4), ("null_shard", 1)],
            Dragon => &[("dragon_scale", 3), ("raw_iron", 4)],
            // the Nameless pay in food and carry what they've stolen
            NamelessRaider => &[("porkchop", 1), ("apple", 1), ("torn_archive_page", 1)],
            Chicken => &[("porkchop", 1)],
            Wolf => &[("mutton", 1)],
            Dog => &[],
            Bear => &[("porkchop", 3)],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MobStats {
    pub max_health: f32,
    pub damage: f32,
    pub speed: f32,
    /// Cube half-size for rendering/collision.
    pub size: f32,
    /// Detection range for hostile AI (0 = passive).
    pub detect: f32,
}

impl MobStats {
    /// Physics half-width: a stride-friendly fraction of the render cube
    /// so bodies don't wedge in one-wide gaps between trees (loop 347).
    pub fn collision_half_width(self) -> f32 {
        (self.size * 0.75).max(0.2)
    }

    /// Physics height (feet up); dragons never use this — their flight
    /// brain owns the vertical axis.
    pub fn collision_height(self) -> f32 {
        (self.size * 1.6).max(0.5)
    }
}

/// B1: the mob behaviour state machine. One variant per intention; the
/// update loop reads the variant, acts, and transitions. The sim has a
/// single targetable actor (the player), so Chase/Attack carry no target
/// id — if pets or multi-actor combat arrive, the target rides here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MobBehaviourState {
    /// Default: exploring territory, no target
    Wander { timer: f32, next_pos: Option<BlockPos> },
    /// Target acquired, moving toward it. `aggro_timer` accumulates the
    /// whole engagement (shared with Attack via `combat_time`), `react_delay`
    /// is the group-aggro reaction pause, `unseen_for` counts lost LOS.
    Chase { aggro_timer: f32, react_delay: f32, unseen_for: f32 },
    /// Taking damage, moving away from threat
    Flee { threat_pos: [f32; 3], flee_timer: f32 },
    /// Cannot see target, searching last known position
    Investigate { last_known: BlockPos, search_timer: f32 },
    /// In combat range, executing attack
    Attack { cooldown: f32 },
    /// After long combat with no kill, mob disengages
    Disengage { cooldown: f32 },
    /// Mob is idle (passive mobs, or between behaviours)
    Idle { timer: f32 },
}

impl Default for MobBehaviourState {
    fn default() -> Self {
        MobBehaviourState::Wander { timer: 0.0, next_pos: None }
    }
}

/// B2 (ai-npc-assets): cheap DDA voxel raycast from `from` to `to`.
/// False when a solid block blocks the ray or the target is beyond 32
/// blocks. Callers cache the result per mob per tick.
pub fn has_line_of_sight(from: Vec3, to: Vec3, world: &World) -> bool {
    let delta = to - from;
    let dist = delta.length();
    if dist > 32.0 {
        return false;
    }
    if dist < 0.05 {
        return true;
    }
    // stop just short of the target cell so the target's own body cell
    // never counts as an obstruction
    let hit = lf_voxel::raycast::raycast_voxel(from, delta / dist, dist - 0.25, |cell| {
        world.is_solid(cell.x, cell.y, cell.z)
    });
    hit.is_none()
}

/// B3: standing modulates the aggro radius. +100 standing → 0.0 (mob
/// ignores the player entirely unless attacked), −100 → double radius.
pub fn effective_aggro_radius(base_radius: f32, standing: i32) -> f32 {
    let standing_factor = 1.0 - (standing as f32 / 100.0).clamp(-1.0, 1.0);
    base_radius * standing_factor
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MobEntity {
    pub id: u64,
    pub mob_type: MobType,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub health: f32,
    pub attack_cooldown: f32,
    pub wander_cooldown: f32,
    pub wander_dir: (f32, f32),
    pub hurt_flash: f32,
    pub age: f32,
    /// P36: dragons carry their flight brain + roost (serde-defaulted so
    /// pre-dragon saves load; JSON extras make the default real).
    #[serde(default)]
    pub dragon: Option<crate::dragons::DragonBrain>,
    #[serde(default)]
    pub roost: Option<[f32; 3]>,
    /// B1: current behaviour (serde-defaulted so pre-upgrade saves load).
    #[serde(default)]
    pub behaviour: MobBehaviourState,
    /// B3: lore faction this mob belongs to, if any.
    #[serde(default)]
    pub faction_id: Option<String>,
    /// B2: LOS cache, recomputed at most once per update tick.
    #[serde(default)]
    pub los_to_player: bool,
    /// B1: total time spent in this engagement (Chase + Attack).
    #[serde(default)]
    pub combat_time: f32,
    /// B4: set for one tick when this mob aggroes on its own so the
    /// owner can let first-order neighbours join (never chains).
    #[serde(default)]
    pub group_ping: bool,
    /// B5: cached A* route for Chase/Investigate.
    #[serde(default)]
    pub path: Option<CachedPath>,
    /// Walk-cycle phase (radians), advanced by distance travelled so legs
    /// never moonwalk; `gait_amp` is the smoothed 0..1 stride amplitude.
    #[serde(default)]
    pub gait_phase: f32,
    #[serde(default)]
    pub gait_amp: f32,
    /// Death animation: None = alive; Some(elapsed seconds) topples the
    /// corpse (DEATH_TOPPLE_S) then rests it (DEATH_REST_S) before removal.
    #[serde(default)]
    pub death_t: Option<f32>,
}

impl MobEntity {
    pub fn spawn(id: u64, mob_type: MobType, position: Vec3) -> Self {
        Self {
            id,
            mob_type,
            position,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            health: mob_type.stats().max_health,
            attack_cooldown: 0.0,
            wander_cooldown: 0.0,
            wander_dir: (0.0, 0.0),
            hurt_flash: 0.0,
            age: 0.0,
            dragon: (mob_type == MobType::Dragon).then(crate::dragons::DragonBrain::default),
            roost: None,
            behaviour: MobBehaviourState::Wander { timer: 0.0, next_pos: None },
            faction_id: mob_type.faction().map(str::to_string),
            los_to_player: false,
            combat_time: 0.0,
            group_ping: false,
            path: None,
            gait_phase: 0.0,
            gait_amp: 0.0,
            death_t: None,
        }
    }

    /// One AI + physics step (no faction standing; equivalent to
    /// standing 0). Returns Some(damage_to_player) when this mob lands
    /// an attack this frame.
    pub fn update(&mut self, dt: f32, world: &World, player_pos: Vec3) -> Option<f32> {
        self.update_with_standing(dt, world, player_pos, 0)
    }

    /// Axis-separated AABB move against the world — the same collision
    /// contract the player has (P34 fractional boxes included), so
    /// animals stop at walls instead of gliding through them while they
    /// bounce. A blocked stride while grounded becomes a hop: one-block
    /// walls are meant to be jumped, taller walls are meant to stop you
    /// (loop 347 "animals walk through walls" fix).
    pub fn physics_step(&mut self, dt: f32, world: &World, wish: (f32, f32), speed: f32) {
        let wish_len = (wish.0 * wish.0 + wish.1 * wish.1).sqrt();
        self.velocity.x = wish.0 / wish_len.max(1.0) * speed;
        self.velocity.z = wish.1 / wish_len.max(1.0) * speed;
        self.velocity.y -= 24.0 * dt;

        let stats = self.mob_type.stats();
        let half = stats.collision_half_width();
        let height = stats.collision_height();
        let aabb = |p: Vec3| {
            (
                Vec3::new(p.x - half, p.y, p.z - half),
                Vec3::new(p.x + half, p.y + height, p.z + half),
            )
        };
        let blocked = |p: Vec3| {
            let (min, max) = aabb(p);
            crate::player::box_intersects_solid(world, min, max)
        };

        // wedged inside solid (stale save, shifted terrain): pop up to
        // safety instead of being sealed in forever
        if blocked(self.position) {
            for _ in 0..4 {
                self.position.y += 1.0;
                if !blocked(self.position) {
                    break;
                }
            }
            self.velocity = Vec3::ZERO;
            return;
        }

        // substep so a sprinting mob can't tunnel a thin wall on a
        // frame-time spike
        let travel = (self.velocity * dt).length();
        let steps = ((travel / 0.4).ceil() as usize).clamp(1, 8);
        let sdt = dt / steps as f32;
        let mut blocked_h = false;
        let mut grounded = false;
        for _ in 0..steps {
            // X
            let dx = self.velocity.x * sdt;
            if dx != 0.0 {
                let mut p = self.position;
                p.x += dx;
                if blocked(p) {
                    self.velocity.x = 0.0;
                    blocked_h = true;
                } else {
                    self.position.x = p.x;
                }
            }
            // Z
            let dz = self.velocity.z * sdt;
            if dz != 0.0 {
                let mut p = self.position;
                p.z += dz;
                if blocked(p) {
                    self.velocity.z = 0.0;
                    blocked_h = true;
                } else {
                    self.position.z = p.z;
                }
            }
            // Y
            let dy = self.velocity.y * sdt;
            if dy != 0.0 {
                let mut p = self.position;
                p.y += dy;
                if blocked(p) {
                    if dy < 0.0 {
                        // land on the floor plane (player-style clamp)
                        let (min, _) = aabb(p);
                        self.position.y = min.y.ceil() + 1e-4;
                        grounded = true;
                    }
                    self.velocity.y = 0.0;
                } else {
                    self.position.y = p.y;
                }
            }
        }
        // standing (not just landed): probe a thin slab under the feet
        if !grounded && self.velocity.y.abs() < 0.05 {
            let (min, max) = aabb(self.position);
            if crate::player::box_intersects_solid(
                world,
                Vec3::new(min.x, min.y - 0.06, min.z),
                Vec3::new(max.x, min.y - 0.01, max.z),
            ) {
                grounded = true;
            }
        }
        if blocked_h && grounded && wish_len > 0.1 {
            self.velocity.y = 8.0; // hop over one-block obstacles
        }
        // hard floor: never fall out of the world
        if self.position.y < -10.0 {
            self.health = 0.0;
        }
    }

    /// B4 hook: when a mob aggroes on its own it pings once; the owner
    /// (client) calls this so nearby same-type mobs join the chase with
    /// a 0.5s reaction delay. First-order neighbours only (mobs already
    /// in Chase/Attack never re-propagate), group capped at 5, and a
    /// fleeing passive never drags anyone into a fight.
    pub fn propagate_group_aggro(mobs: &mut [MobEntity], origin: usize, target: Vec3) {
        let Some(origin_mob) = mobs.get(origin) else { return };
        if !origin_mob.mob_type.is_hostile() {
            return;
        }
        let kind = origin_mob.mob_type;
        let opos = origin_mob.position;
        let group: Vec<usize> = (0..mobs.len())
            .filter(|&i| {
                mobs[i].mob_type == kind && mobs[i].position.distance(opos) <= 8.0
            })
            .collect();
        if group.len() > 5 {
            return; // too big a pack: perf and player experience both suffer
        }
        for &i in &group {
            if i == origin {
                continue;
            }
            if matches!(
                mobs[i].behaviour,
                MobBehaviourState::Chase { .. } | MobBehaviourState::Attack { .. }
            ) {
                continue; // already fighting; never chains beyond first-order
            }
            mobs[i].behaviour = MobBehaviourState::Chase {
                aggro_timer: 0.0,
                react_delay: 0.5,
                unseen_for: 0.0,
            };
            mobs[i].combat_time = 0.0;
        }
        let _ = target;
    }

    /// Deterministic per-mob pseudo-random in 0..N (id-hashed, stable).
    fn hash_mod(&self, salt: u64, n: u64) -> u64 {
        (self.id.wrapping_mul(2654435761).wrapping_add(salt)) % n
    }

    fn eye(&self) -> Vec3 {
        Vec3::new(self.position.x, self.position.y + 0.9, self.position.z)
    }

    fn feet_block(&self) -> BlockPos {
        [
            self.position.x.floor() as i32,
            self.position.y.floor() as i32,
            self.position.z.floor() as i32,
        ]
    }

    /// B5: steer toward `goal_block` along a cached A* path, recomputing
    /// when stale (older than 2s, goal drifted >4 blocks, or exhausted).
    /// Falls back to the direct wish when the goal is out of A* range or
    /// no path exists. Returns the movement wish.
    fn path_wish(&mut self, goal_block: BlockPos, world: &World, dt: f32) -> (f32, f32) {
        let start = self.feet_block();
        let in_range = mob_pathfind::path_distance(start, goal_block) <= mob_pathfind::MAX_PATH_RANGE;
        if in_range {
            match &mut self.path {
                Some(p) if p.age < 2.0 && mob_pathfind::path_distance(p.goal, goal_block) <= 4 => {
                    p.age += dt;
                }
                _ => {
                    self.path = mob_pathfind::find_path(start, goal_block, world, mob_pathfind::MAX_PATH_NODES)
                        .map(|nodes| CachedPath { nodes, goal: goal_block, age: 0.0, cursor: 0 });
                }
            }
        } else {
            self.path = None;
        }
        if let Some(p) = &mut self.path {
            // advance the cursor past nodes we are standing on
            while let Some(node) = p.nodes.get(p.cursor) {
                let dx = node[0] as f32 + 0.5 - self.position.x;
                let dz = node[2] as f32 + 0.5 - self.position.z;
                if dx * dx + dz * dz < 0.45 * 0.45 {
                    p.cursor += 1;
                } else {
                    break;
                }
            }
            let target = match p.nodes.get(p.cursor) {
                Some(node) => Vec3::new(node[0] as f32 + 0.5, self.position.y, node[2] as f32 + 0.5),
                None => Vec3::new(
                    goal_block[0] as f32 + 0.5,
                    self.position.y,
                    goal_block[2] as f32 + 0.5,
                ),
            };
            let d = target - self.position;
            let len = (d.x * d.x + d.z * d.z).sqrt();
            if len > 0.05 {
                return (d.x / len, d.z / len);
            }
            return (0.0, 0.0);
        }
        // fallback: direct steering (also covers out-of-range goals)
        let d = Vec3::new(
            goal_block[0] as f32 + 0.5 - self.position.x,
            0.0,
            goal_block[2] as f32 + 0.5 - self.position.z,
        );
        let len = (d.x * d.x + d.z * d.z).sqrt();
        if len > 0.05 {
            (d.x / len, d.z / len)
        } else {
            (0.0, 0.0)
        }
    }

    /// One AI + physics step with the player's faction standing applied
    /// (B3). `standing` is the player's standing with THIS mob's faction
    /// (0 when the mob is unaffiliated).
    pub fn update_with_standing(&mut self, dt: f32, world: &World, player_pos: Vec3, standing: i32) -> Option<f32> {
        self.age += dt;
        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
        self.hurt_flash = (self.hurt_flash - dt * 3.0).max(0.0);
        let stats = self.mob_type.stats();

        // The dead keep no AI — gravity and friction only, so the corpse
        // settles while the topple animation plays out.
        if let Some(t) = &mut self.death_t {
            *t += dt;
            self.velocity.x *= 1.0 - (dt * 6.0).min(1.0);
            self.velocity.z *= 1.0 - (dt * 6.0).min(1.0);
            self.velocity.y -= 24.0 * dt;
            let next = self.position + self.velocity * dt;
            if self.velocity.y < 0.0
                && world.is_solid(next.x as i32, (next.y - 0.1) as i32, next.z as i32)
            {
                self.velocity = Vec3::ZERO;
            } else {
                self.position = next;
            }
            self.gait_amp = (self.gait_amp - dt * 8.0).max(0.0);
            return None;
        }

        // Bosses with their own brain are owned by the client flight loop.
        if self.mob_type.use_boss_ai() && self.dragon.is_some() {
            return None;
        }

        let to_player = player_pos - self.position;
        let dist = to_player.length();
        let player_block: BlockPos = [
            player_pos.x.floor() as i32,
            player_pos.y.floor() as i32,
            player_pos.z.floor() as i32,
        ];
        let aggro_radius = effective_aggro_radius(stats.detect, standing);

        let mut wish: (f32, f32) = (0.0, 0.0);
        let mut speed_mult = 1.0f32;
        let mut strike: Option<f32> = None;

        match self.behaviour.clone() {
            MobBehaviourState::Idle { mut timer } => {
                timer -= dt;
                if timer <= 0.0 {
                    // Idle → Wander after a random 3–8s pause
                    self.behaviour = MobBehaviourState::Wander { timer: 0.0, next_pos: None };
                    self.wander_cooldown = 0.0;
                } else {
                    self.behaviour = MobBehaviourState::Idle { timer };
                }
            }
            MobBehaviourState::Wander { timer, next_pos } => {
                // Wander → Chase: hostile, inside the (standing-modulated)
                // aggro radius, and the player is actually visible.
                if self.mob_type.is_hostile() && aggro_radius > 0.0 && dist < aggro_radius {
                    self.los_to_player = has_line_of_sight(
                        self.eye(),
                        Vec3::new(player_pos.x, player_pos.y + 1.2, player_pos.z),
                        world,
                    );
                    if self.los_to_player {
                        self.combat_time = 0.0;
                        self.group_ping = true;
                        self.behaviour = MobBehaviourState::Chase {
                            aggro_timer: 0.0,
                            react_delay: 0.0,
                            unseen_for: 0.0,
                        };
                    } else {
                        self.behaviour = MobBehaviourState::Wander { timer, next_pos };
                        self.wander_step(dt, timer, next_pos, &mut wish);
                    }
                } else {
                    self.wander_step(dt, timer, next_pos, &mut wish);
                }
            }
            MobBehaviourState::Chase { aggro_timer, mut react_delay, mut unseen_for } => {
                self.combat_time += dt;
                if self.combat_time > 30.0 {
                    // Chase → Disengage: long fight, no kill — break off
                    self.path = None;
                    self.behaviour = MobBehaviourState::Disengage { cooldown: 8.0 };
                } else {
                    self.los_to_player = has_line_of_sight(
                        self.eye(),
                        Vec3::new(player_pos.x, player_pos.y + 1.2, player_pos.z),
                        world,
                    );
                    if self.los_to_player {
                        unseen_for = 0.0;
                    } else {
                        unseen_for += dt;
                    }
                    if unseen_for > 2.0 {
                        // Chase → Investigate: lost sight for >2s
                        self.path = None;
                        let search = 8.0 + self.hash_mod(11, 8) as f32;
                        self.behaviour = MobBehaviourState::Investigate {
                            last_known: player_block,
                            search_timer: search,
                        };
                    } else if dist <= 1.5 && to_player.y.abs() < 2.0 {
                        // Chase → Attack: in melee range
                        self.behaviour = MobBehaviourState::Attack { cooldown: 0.0 };
                    } else {
                        if react_delay > 0.0 {
                            // group-aggro reaction pause: stand, then commit
                            react_delay -= dt;
                            self.behaviour = MobBehaviourState::Chase {
                                aggro_timer: aggro_timer + dt,
                                react_delay,
                                unseen_for,
                            };
                        } else {
                            wish = self.path_wish(player_block, world, dt);
                            self.behaviour = MobBehaviourState::Chase {
                                aggro_timer: aggro_timer + dt,
                                react_delay: 0.0,
                                unseen_for,
                            };
                        }
                    }
                }
            }
            MobBehaviourState::Attack { mut cooldown } => {
                self.combat_time += dt;
                if self.combat_time > 30.0 {
                    // the whole engagement (Chase + Attack) timed out
                    self.path = None;
                    self.behaviour = MobBehaviourState::Disengage { cooldown: 8.0 };
                } else if dist > 1.5 {
                    // Attack → Chase: target slipped out of melee range
                    self.behaviour = MobBehaviourState::Chase {
                        aggro_timer: 0.0,
                        react_delay: 0.0,
                        unseen_for: 0.0,
                    };
                } else {
                    cooldown = cooldown.max(0.0) - dt;
                    if cooldown <= 0.0 && to_player.y.abs() < 2.0 {
                        cooldown = 1.0;
                        strike = Some(stats.damage);
                    }
                    self.behaviour = MobBehaviourState::Attack { cooldown };
                }
            }
            MobBehaviourState::Flee { threat_pos, mut flee_timer } => {
                flee_timer -= dt;
                let threat = Vec3::from(threat_pos);
                let threat_visible = has_line_of_sight(
                    self.eye(),
                    Vec3::new(threat.x, threat.y + 1.2, threat.z),
                    world,
                );
                if !threat_visible {
                    // Flee → Wander: lost sight of the threat (even early)
                    self.behaviour = MobBehaviourState::Wander { timer: 0.0, next_pos: None };
                    self.wander_cooldown = 0.0;
                } else if flee_timer < -5.0 {
                    // threat still visible but we have fled long enough
                    self.behaviour = MobBehaviourState::Wander { timer: 0.0, next_pos: None };
                    self.wander_cooldown = 0.0;
                } else {
                    let away = self.position - threat;
                    let len = (away.x * away.x + away.z * away.z).sqrt().max(0.001);
                    wish = (away.x / len, away.z / len);
                    speed_mult = 1.5;
                    self.behaviour = MobBehaviourState::Flee { threat_pos, flee_timer };
                }
            }
            MobBehaviourState::Investigate { last_known, mut search_timer } => {
                self.los_to_player = self.mob_type.is_hostile()
                    && dist < stats.detect
                    && has_line_of_sight(
                        self.eye(),
                        Vec3::new(player_pos.x, player_pos.y + 1.2, player_pos.z),
                        world,
                    );
                if self.los_to_player {
                    // Investigate → Chase: target spotted again
                    self.combat_time = 0.0;
                    self.group_ping = true;
                    self.behaviour = MobBehaviourState::Chase {
                        aggro_timer: 0.0,
                        react_delay: 0.0,
                        unseen_for: 0.0,
                    };
                } else {
                    search_timer -= dt;
                    if search_timer <= 0.0 {
                        // Investigate → Wander: gave up the search
                        self.behaviour = MobBehaviourState::Wander { timer: 0.0, next_pos: None };
                        self.wander_cooldown = 0.0;
                    } else {
                        let here = self.feet_block();
                        let arrived = (here[0] - last_known[0]).abs() <= 1
                            && (here[2] - last_known[2]).abs() <= 1;
                        if !arrived {
                            wish = self.path_wish(last_known, world, dt);
                        }
                        self.behaviour = MobBehaviourState::Investigate { last_known, search_timer };
                    }
                }
            }
            MobBehaviourState::Disengage { mut cooldown } => {
                cooldown -= dt;
                if cooldown <= 0.0 {
                    let idle = 3.0 + self.hash_mod(7, 5) as f32;
                    self.behaviour = MobBehaviourState::Idle { timer: idle };
                } else {
                    self.behaviour = MobBehaviourState::Disengage { cooldown };
                }
            }
        }

        if let Some(damage) = strike {
            return Some(damage);
        }        let wish_len = (wish.0 * wish.0 + wish.1 * wish.1).sqrt();
        if wish_len > 0.001 {
            // smooth shortest-arc turn (max ~8 rad/s) instead of snapping —
            // bodies swing around, they don't teleport between facings
            let target = wish.0.atan2(wish.1);
            let mut delta = target - self.yaw;
            while delta > std::f32::consts::PI {
                delta -= std::f32::consts::TAU;
            }
            while delta < -std::f32::consts::PI {
                delta += std::f32::consts::TAU;
            }
            self.yaw += delta.clamp(-8.0 * dt, 8.0 * dt);
        }

        // --- physics (loop 347): axis-separated AABB collision with
        // hop-assisted step-up — animals stop at walls like the player
        // does; the old point-probe physics committed every horizontal
        // move and let them glide through anything 2 blocks tall.
        self.physics_step(dt, world, wish, stats.speed * speed_mult);

        // --- gait: phase advances with distance travelled, amplitude eases
        // in/out so legs start and stop instead of freezing mid-stride
        let horiz_speed = (self.velocity.x * self.velocity.x + self.velocity.z * self.velocity.z).sqrt();
        let moving = wish_len > 0.1 && horiz_speed > 0.15;
        let target_amp = if moving { 1.0 } else { 0.0 };
        let amp_rate = if target_amp > self.gait_amp { 10.0 } else { 6.0 };
        self.gait_amp += (target_amp - self.gait_amp).clamp(-amp_rate * dt, amp_rate * dt);
        if moving {
            // ~1.4 blocks per full stride cycle
            self.gait_phase += dt * horiz_speed * 4.5;
        }
        None
    }

    /// Wander steering: pick a territory target within ~6 blocks, walk
    /// there, occasionally pause (the "creature with a home" feel).
    fn wander_step(
        &mut self,
        dt: f32,
        timer: f32,
        next_pos: Option<BlockPos>,
        wish: &mut (f32, f32),
    ) {
        self.wander_cooldown -= dt;
        let here = self.feet_block();
        let arrived = next_pos
            .map(|p| (p[0] - here[0]).abs() + (p[2] - here[2]).abs() <= 0)
            .unwrap_or(true);
        if self.wander_cooldown <= 0.0 || arrived {
            self.wander_cooldown = 2.0 + self.hash_mod(3, 3000) as f32 / 1000.0;
            if self.hash_mod(5, 3) == 0 {
                // stand still and look around for a while (proper Idle so
                // the pause actually lasts a couple of seconds)
                self.behaviour = MobBehaviourState::Idle {
                    timer: 2.0 + self.hash_mod(19, 4) as f32,
                };
                *wish = (0.0, 0.0);
                return;
            }
            let a = (self.hash_mod(13, 360) as f32) / 57.3;
            let r = 2 + self.hash_mod(17, 5) as i32;
            let target: BlockPos = [
                here[0] + (a.sin() * r as f32).round() as i32,
                here[1],
                here[2] + (a.cos() * r as f32).round() as i32,
            ];
            self.behaviour = MobBehaviourState::Wander { timer: timer + dt, next_pos: Some(target) };
            *wish = (a.sin(), a.cos());
        } else if let Some(p) = next_pos {
            let dx = p[0] as f32 + 0.5 - self.position.x;
            let dz = p[2] as f32 + 0.5 - self.position.z;
            let len = (dx * dx + dz * dz).sqrt();
            if len > 0.1 {
                *wish = (dx / len, dz / len);
            }
            self.behaviour = MobBehaviourState::Wander { timer: timer + dt, next_pos };
        }
    }

    /// Player attack: apply damage + knockback. Returns true if this kills.
    pub fn take_hit(&mut self, damage: f32, from: Vec3) -> bool {
        self.health -= damage;
        self.hurt_flash = 1.0;
        let push = (self.position - from).normalize() * 6.0;
        self.velocity.x = push.x;
        self.velocity.z = push.z;
        self.velocity.y = 4.0;
        // B1: being attacked overrides the current intention. Passives
        // bolt; fighters commit even at honored standing (the only way
        // past `effective_aggro_radius == 0`).
        if self.mob_type.is_hostile() {
            self.combat_time = self.combat_time.max(0.0);
            self.group_ping = true;
            self.behaviour = MobBehaviourState::Chase {
                aggro_timer: 0.0,
                react_delay: 0.0,
                unseen_for: 0.0,
            };
        } else {
            self.path = None;
            self.behaviour = MobBehaviourState::Flee {
                threat_pos: [from.x, from.y, from.z],
                flee_timer: 5.0,
            };
        }
        self.health <= 0.0
    }

    /// Start the death animation (idempotent — extra killing blows land on
    /// an already-falling corpse). Loot still pops out immediately; the
    /// body topples, rests, and only then is removed by the owner.
    pub fn begin_death(&mut self) {
        if self.death_t.is_none() {
            self.death_t = Some(0.0);
        }
    }

    /// True once the topple + rest have fully played and the corpse should
    /// be removed.
    pub fn dead_and_gone(&self) -> bool {
        self.death_t
            .map(|t| t >= DEATH_TOPPLE_S + DEATH_REST_S)
            .unwrap_or(false)
    }
}

/// One oriented cuboid of an animal body, in local space (+Z forward, +Y
/// up, feet on y=0). The renderer pitches it around `pivot` and yaws the
/// whole assembly to the mob's facing — same math as the humanoids.
#[derive(Clone, Copy, Debug)]
pub struct AnimalPart {
    pub center: [f32; 3],
    pub half: [f32; 3],
    /// Pitch around `pivot` in radians (leg swing, head bob, grazing).
    pub pitch: f32,
    pub pivot: [f32; 3],
}

/// Death animation timing: the corpse topples over DEATH_TOPPLE_S, rests
/// DEATH_REST_S, then the owner removes it.
pub const DEATH_TOPPLE_S: f32 = 0.5;
pub const DEATH_REST_S: f32 = 1.0;

/// Multi-part articulated layouts for the animals. `phase` is the walk
/// cycle (radians, distance-driven), `amp` the 0..1 stride amplitude, and
/// `hurt` the 0..1 damage flash (bodies flinch-squash). Local +Z is
/// forward; the caller yaws everything by the mob's facing.
pub fn animal_parts(kind: MobType, phase: f32, amp: f32, hurt: f32) -> Vec<AnimalPart> {
    use MobType::*;
    let hurt = hurt.clamp(0.0, 1.0);
    let squash = 1.0 - 0.1 * hurt;
    let mut parts: Vec<AnimalPart> = Vec::new();
    // a leg: column of half-height len/2 hanging from its hip, pitched by
    // the trot cycle; diagonal pairs move together (FL+RR vs FR+RL)
    let mut leg = |parts: &mut Vec<AnimalPart>,
                   cx: f32, hip_y: f32, cz: f32, thick: f32, len: f32,
                   swing: f32, phase_offset: f32| {
        let pitch = (phase + phase_offset).sin() * swing * amp;
        parts.push(AnimalPart {
            center: [cx, (hip_y - len / 2.0) * squash, cz],
            half: [thick, len / 2.0, thick],
            pitch,
            pivot: [cx, hip_y, cz],
        });
    };
    // two steps per stride cycle: the body bobs at double frequency
    let bob = (phase * 2.0).sin() * 0.025 * amp;
    match kind {
        Chicken => {
            let peck = (1.0 - amp) * ((phase * 0.7).sin() * 0.5 + 0.35); // idle pecking
            parts.push(AnimalPart {
                center: [0.0, (0.28 + bob) * squash, 0.0],
                half: [0.16, 0.13, 0.2],
                pitch: 0.0,
                pivot: [0.0, 0.28, 0.0],
            });
            let head_pitch = (phase).sin() * 0.2 * amp - peck;
            let neck = [0.0, 0.42, 0.12];
            parts.push(AnimalPart {
                center: [0.0, 0.5 * squash, 0.16],
                half: [0.09, 0.09, 0.09],
                pitch: head_pitch,
                pivot: neck,
            });
            parts.push(AnimalPart {
                center: [0.0, 0.47 * squash, 0.28],
                half: [0.035, 0.03, 0.05],
                pitch: head_pitch,
                pivot: neck,
            });
            parts.push(AnimalPart {
                center: [0.0, 0.61 * squash, 0.14],
                half: [0.03, 0.035, 0.05],
                pitch: head_pitch,
                pivot: neck,
            });
            leg(&mut parts, 0.05, 0.15, 0.0, 0.025, 0.15, 0.7, 0.0);
            leg(&mut parts, -0.05, 0.15, 0.0, 0.025, 0.15, 0.7, std::f32::consts::PI);
        }
        Wolf | Dog => {
            parts.push(AnimalPart {
                center: [0.0, (0.42 + bob) * squash, 0.0],
                half: [0.17, 0.16, 0.32],
                pitch: 0.0,
                pivot: [0.0, 0.42, 0.0],
            });
            let neck = [0.0, 0.5, 0.3];
            parts.push(AnimalPart {
                center: [0.0, 0.56 * squash, 0.38],
                half: [0.11, 0.1, 0.11],
                pitch: (phase).sin() * 0.12 * amp,
                pivot: neck,
            });
            for sx in [0.06, -0.06] {
                parts.push(AnimalPart {
                    center: [sx, 0.68 * squash, 0.36],
                    half: [0.035, 0.035, 0.035],
                    pitch: 0.0,
                    pivot: neck,
                });
            }
            parts.push(AnimalPart {
                center: [0.0, 0.5 * squash, -0.44],
                half: [0.045, 0.045, 0.14],
                // the tail keeps a lazy wag even at rest
                pitch: -(phase).sin() * 0.35 * (0.3 + 0.7 * amp),
                pivot: [0.0, 0.5, -0.34],
            });
            leg(&mut parts, 0.11, 0.3, 0.2, 0.045, 0.3, 0.55, 0.0);
            leg(&mut parts, -0.11, 0.3, -0.2, 0.045, 0.3, 0.55, 0.0);
            leg(&mut parts, 0.11, 0.3, -0.2, 0.045, 0.3, 0.55, std::f32::consts::PI);
            leg(&mut parts, -0.11, 0.3, 0.2, 0.045, 0.3, 0.55, std::f32::consts::PI);
        }
        Bear => {
            parts.push(AnimalPart {
                center: [0.0, (0.55 + bob) * squash, 0.0],
                half: [0.34, 0.3, 0.44],
                pitch: 0.0,
                pivot: [0.0, 0.55, 0.0],
            });
            let neck = [0.0, 0.62, 0.45];
            parts.push(AnimalPart {
                center: [0.0, 0.68 * squash, 0.55],
                half: [0.18, 0.16, 0.16],
                pitch: (phase).sin() * 0.08 * amp,
                pivot: neck,
            });
            for sx in [0.1, -0.1] {
                parts.push(AnimalPart {
                    center: [sx, 0.84 * squash, 0.5],
                    half: [0.05, 0.05, 0.05],
                    pitch: 0.0,
                    pivot: neck,
                });
            }
            leg(&mut parts, 0.26, 0.28, 0.28, 0.1, 0.28, 0.45, 0.0);
            leg(&mut parts, -0.26, 0.28, -0.28, 0.1, 0.28, 0.45, 0.0);
            leg(&mut parts, 0.26, 0.28, -0.28, 0.1, 0.28, 0.45, std::f32::consts::PI);
            leg(&mut parts, -0.26, 0.28, 0.28, 0.1, 0.28, 0.45, std::f32::consts::PI);
        }
        Boar => {
            parts.push(AnimalPart {
                center: [0.0, (0.42 + bob) * squash, 0.0],
                half: [0.24, 0.2, 0.34],
                pitch: 0.0,
                pivot: [0.0, 0.42, 0.0],
            });
            let neck = [0.0, 0.42, 0.32];
            let head_pitch = (phase).sin() * 0.1 * amp;
            parts.push(AnimalPart {
                center: [0.0, 0.42 * squash, 0.42],
                half: [0.13, 0.12, 0.14],
                pitch: head_pitch,
                pivot: neck,
            });
            parts.push(AnimalPart {
                center: [0.0, 0.38 * squash, 0.56],
                half: [0.05, 0.05, 0.06],
                pitch: head_pitch,
                pivot: neck,
            });
            leg(&mut parts, 0.16, 0.22, 0.22, 0.06, 0.22, 0.5, 0.0);
            leg(&mut parts, -0.16, 0.22, -0.22, 0.06, 0.22, 0.5, 0.0);
            leg(&mut parts, 0.16, 0.22, -0.22, 0.06, 0.22, 0.5, std::f32::consts::PI);
            leg(&mut parts, -0.16, 0.22, 0.22, 0.06, 0.22, 0.5, std::f32::consts::PI);
        }
        Woolbeast => {
            parts.push(AnimalPart {
                center: [0.0, (0.5 + bob) * squash, 0.0],
                half: [0.28, 0.24, 0.36],
                pitch: 0.0,
                pivot: [0.0, 0.5, 0.0],
            });
            // head lowers to graze while idle
            let graze = (1.0 - amp).min(1.0) * (0.4 + (phase * 0.5).sin() * 0.1);
            parts.push(AnimalPart {
                center: [0.0, 0.6 * squash, 0.44],
                half: [0.1, 0.1, 0.12],
                pitch: (phase).sin() * 0.09 * amp + graze,
                pivot: [0.0, 0.56, 0.36],
            });
            leg(&mut parts, 0.18, 0.26, 0.24, 0.06, 0.26, 0.5, 0.0);
            leg(&mut parts, -0.18, 0.26, -0.24, 0.06, 0.26, 0.5, 0.0);
            leg(&mut parts, 0.18, 0.26, -0.24, 0.06, 0.26, 0.5, std::f32::consts::PI);
            leg(&mut parts, -0.18, 0.26, 0.24, 0.06, 0.26, 0.5, std::f32::consts::PI);
        }
        _ => parts.push(AnimalPart {
            center: [0.0, 0.0, 0.0],
            half: [0.0, 0.0, 0.0],
            pitch: 0.0,
            pivot: [0.0, 0.0, 0.0],
        }),
    }
    parts
}

/// Which mob type should spawn given the time of day.
/// Day spawns are biome-appropriate (Step 18): Woolbeasts are cold-biome
/// fauna, Boars temperate; night hostiles are global. `nameless_biome`
/// (lore-and-visuals) rolls Nameless raiders — the camps' garrison.
pub fn roll_spawn(rand: u64, is_day: bool, cold_biome: bool) -> Option<MobType> {
    roll_spawn_full(rand, is_day, cold_biome, false)
}

/// king-quest C: ambient animal spawns. Chickens peck around temperate
/// land by day, wolves hunt the cold night, bears keep to the deep
/// forests, dogs den near settlements. Deterministic in `rand`.
pub fn roll_animal_spawn(rand: u64, is_day: bool, cold: bool, forest: bool, settlement: bool) -> Option<MobType> {
    let v = (rand ^ 0x9E3779B97F4A7C15) % 100;
    if is_day {
        if settlement && v < 18 { return Some(MobType::Dog); }
        if forest && v < 22 { return Some(MobType::Bear); }
        if !cold && v < 40 { return Some(MobType::Chicken); }
        None
    } else if cold && v < 30 {
        Some(MobType::Wolf)
    } else {
        None
    }
}

pub fn roll_spawn_full(rand: u64, is_day: bool, cold_biome: bool, nameless_biome: bool) -> Option<MobType> {
    use MobType::*;
    let v = (rand * 2654435761) % 100;
    if is_day {
        if cold_biome {
            match v {
                0..=59 => Some(Woolbeast),
                _ => None,
            }
        } else {
            match v {
                0..=39 => Some(Boar),
                40..=49 => Some(Woolbeast), // shaggy stragglers roam the edges
                _ => None,
            }
        }
    } else if nameless_biome {
        match v {
            0..=24 => Some(NamelessRaider),
            25..=49 => Some(Glitchling),
            50..=64 => Some(Crawler),
            _ => None,
        }
    } else {
        match v {
            0..=34 => Some(Glitchling),
            35..=54 => Some(Crawler),
            55..=69 => Some(Stalker),
            70..=71 => Some(NullKnight), // rare boss
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lf_voxel::BlockState;

    fn flat_world() -> World {
        let mut w = World::new();
        // chunks for the full -20..19 strip (set_block silently drops
        // edits in chunks that were never ensured)
        for cx in -2..=1 {
            for cz in -2..=1 {
                w.ensure_chunk(cx, cz);
            }
        }
        for x in 0..40 {
            for z in 0..40 {
                w.set_block(x - 20, 0, z - 20, BlockState::STONE);
            }
        }
        w
    }

    /// Wall through the whole playable strip so LOS across z=0 dies.
    fn wall(w: &mut World, x: i32) {
        for z in -20..20 {
            w.set_block(x, 1, z, BlockState::STONE);
            w.set_block(x, 2, z, BlockState::STONE);
            w.set_block(x, 3, z, BlockState::STONE);
        }
    }

    #[test]
    fn mobs_fall_and_land() {
        let w = flat_world();
        let mut m = MobEntity::spawn(1, MobType::Boar, Vec3::new(0.0, 20.0, 0.0));
        for _ in 0..200 {
            m.update(0.05, &w, Vec3::new(100.0, 1.0, 100.0));
        }
        assert!((m.position.y - 1.0).abs() < 0.2, "boar should land on floor, at {}", m.position.y);
    }

    #[test]
    fn hostile_chases_and_attacks_player() {
        let w = flat_world();
        let mut m = MobEntity::spawn(2, MobType::Glitchling, Vec3::new(5.0, 1.0, 0.0));
        let player = Vec3::new(0.0, 1.0, 0.0);
        let mut hit = false;
        for _ in 0..400 {
            if m.update(0.05, &w, player).is_some() {
                hit = true;
            }
        }
        assert!(hit, "glitchling should have attacked");
        assert!((m.position - player).length() < 4.0, "should close distance, at {:?}", m.position);
    }

    #[test]
    fn passive_ignores_player() {
        let w = flat_world();
        let mut m = MobEntity::spawn(3, MobType::Woolbeast, Vec3::new(5.0, 1.0, 0.0));
        let player = Vec3::new(0.0, 1.0, 0.0);
        for _ in 0..100 {
            m.update(0.05, &w, player);
        }
        // no attack ever
        assert!(!m.mob_type.is_hostile());
        assert!(matches!(m.behaviour, MobBehaviourState::Wander { .. } | MobBehaviourState::Idle { .. }));
    }

    #[test]
    fn take_hit_knocks_back_and_kills() {
        let mut m = MobEntity::spawn(4, MobType::Crawler, Vec3::new(0.0, 5.0, 0.0));
        assert!(!m.take_hit(10.0, Vec3::new(1.0, 5.0, 0.0)));
        assert!(m.hurt_flash > 0.0);
        assert!(m.velocity.x < 0.0, "knocked away from attacker");
        assert!(matches!(m.behaviour, MobBehaviourState::Chase { .. }), "hostile commits when struck");
        assert!(m.take_hit(20.0, Vec3::new(1.0, 5.0, 0.0)));
        assert!(m.health <= 0.0);
    }

    #[test]
    fn spawn_rules_respond_to_day_night() {
        let day: std::collections::HashSet<MobType> = (0..1000)
            .filter_map(|i| roll_spawn(i, true, false))
            .collect();
        let night: std::collections::HashSet<MobType> = (0..1000)
            .filter_map(|i| roll_spawn(i, false, false))
            .collect();
        assert!(day.contains(&MobType::Boar) && !night.contains(&MobType::Boar));
        assert!(night.contains(&MobType::Glitchling) && !day.contains(&MobType::Glitchling));
        assert!(night.contains(&MobType::NullKnight));
    }

    /// Failure meaning: the mob state machine has a broken transition.
    /// Walks every edge in the B1 transition table.
    #[test]
    fn mob_ai_state_transitions() {
        let mut w = flat_world();
        let far = Vec3::new(60.0, 1.0, 60.0); // beyond every aggro radius

        // Idle → Wander after the idle timer expires
        let mut m = MobEntity::spawn(21, MobType::Glitchling, Vec3::new(2.0, 1.0, 0.0));
        m.behaviour = MobBehaviourState::Idle { timer: 0.3 };
        m.update(0.4, &w, far);
        assert!(matches!(m.behaviour, MobBehaviourState::Wander { .. }), "{:?}", m.behaviour);

        // Wander → Chase when a hostile sees the player
        m.position = Vec3::new(6.0, 1.0, 0.0);
        m.update(0.05, &w, Vec3::new(0.0, 1.0, 0.0));
        assert!(matches!(m.behaviour, MobBehaviourState::Chase { react_delay: 0.0, .. }), "{:?}", m.behaviour);
        assert!(m.group_ping, "self-aggro pings the group");

        // Chase → Attack at melee range; the strike lands on the next
        // tick (cooldown starts at 0 and expires immediately)
        m.position = Vec3::new(1.2, 1.0, 0.0);
        let hit1 = m.update(0.05, &w, Vec3::new(0.0, 1.0, 0.0));
        assert!(matches!(m.behaviour, MobBehaviourState::Attack { .. }), "{:?}", m.behaviour);
        assert_eq!(hit1, None, "entry tick only transitions");
        let hit2 = m.update(0.05, &w, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(hit2, Some(4.0), "attack lands once the cooldown expires");
        m.position = Vec3::new(4.0, 1.0, 0.0);
        m.update(0.05, &w, Vec3::new(0.0, 1.0, 0.0));
        assert!(matches!(m.behaviour, MobBehaviourState::Chase { .. }), "{:?}", m.behaviour);

        // Chase → Investigate after >2s without line of sight (the mob
        // starts far enough that it cannot reach or hop the wall first)
        wall(&mut w, 3);
        m.position = Vec3::new(12.0, 1.0, 0.0);
        m.behaviour = MobBehaviourState::Chase { aggro_timer: 0.0, react_delay: 0.0, unseen_for: 0.0 };
        let mut saw_investigate = false;
        for _ in 0..60 {
            m.update(0.05, &w, Vec3::new(0.0, 1.0, 0.0));
            if let MobBehaviourState::Investigate { last_known, .. } = &m.behaviour {
                assert_eq!(last_known, &[0, 1, 0], "remembers the last known player block");
                saw_investigate = true;
                break;
            }
        }
        assert!(saw_investigate, "never lost the trail through the wall");

        // Investigate → Chase when the target steps back into view
        w.set_block(3, 1, -15, BlockState::AIR); // breach the wall low at z=-15
        m.position = Vec3::new(4.0, 1.0, -15.0);
        m.update(0.05, &w, Vec3::new(0.0, 1.0, -15.0));
        assert!(matches!(m.behaviour, MobBehaviourState::Chase { .. }), "{:?}", m.behaviour);
        // close the breach again
        w.set_block(3, 1, -15, BlockState::STONE);

        // Investigate → Wander when the search timer runs out
        m.behaviour = MobBehaviourState::Investigate { last_known: [0, 1, 0], search_timer: 0.2 };
        m.update(0.3, &w, Vec3::new(20.0, 1.0, 0.0));
        assert!(matches!(m.behaviour, MobBehaviourState::Wander { .. }), "{:?}", m.behaviour);

        // Wander → Flee when a passive is struck; Flee → Wander when the threat is unseen
        let mut boar = MobEntity::spawn(22, MobType::Boar, Vec3::new(0.0, 1.0, 0.0));
        boar.take_hit(3.0, Vec3::new(2.0, 1.0, 0.0));
        assert!(matches!(boar.behaviour, MobBehaviourState::Flee { .. }), "{:?}", boar.behaviour);
        let before = boar.position;
        boar.update(0.1, &w, Vec3::new(2.0, 1.0, 0.0));
        assert!(boar.position.distance(before) > 0.05, "fleeing boar actually moves");
        // wall the boar off from the threat: LOS dies → back to Wander
        wall(&mut w, 5);
        boar.position = Vec3::new(8.0, 1.0, 0.0);
        boar.behaviour = MobBehaviourState::Flee { threat_pos: [2.0, 1.0, 0.0], flee_timer: 4.0 };
        boar.update(0.05, &w, Vec3::new(2.0, 1.0, 0.0));
        assert!(matches!(boar.behaviour, MobBehaviourState::Wander { .. }), "{:?}", boar.behaviour);
        w.set_block(5, 1, -25, BlockState::AIR);
        w.set_block(5, 2, -25, BlockState::AIR);
        w.set_block(5, 3, -25, BlockState::AIR);

        // Chase → Disengage after 30s of combat without a kill; Disengage → Idle
        // (kept inside the floor strip: outside it the mob falls out of the world)
        let mut stalker = MobEntity::spawn(23, MobType::Stalker, Vec3::new(14.0, 1.0, 0.0));
        stalker.behaviour = MobBehaviourState::Chase { aggro_timer: 0.0, react_delay: 0.0, unseen_for: 0.0 };
        let mut disengaged = false;
        for _ in 0..(33 * 20) {
            stalker.update(0.05, &w, Vec3::new(14.5, 1.0, 0.0));
            if matches!(stalker.behaviour, MobBehaviourState::Disengage { .. }) {
                disengaged = true;
                break;
            }
        }
        assert!(disengaged, "a 30s fight with no kill must break off");
        stalker.behaviour = MobBehaviourState::Disengage { cooldown: 0.1 };
        stalker.update(0.2, &w, Vec3::new(60.0, 1.0, 60.0));
        assert!(matches!(stalker.behaviour, MobBehaviourState::Idle { .. }), "{:?}", stalker.behaviour);
    }

    /// Failure meaning: the LOS raycast sees through (or is blocked by)
    /// blocks it should not.
    #[test]
    fn mob_los_check() {
        let w = flat_world();
        let from = Vec3::new(0.5, 1.9, 0.5);
        let to = Vec3::new(6.5, 1.9, 0.5);
        assert!(has_line_of_sight(from, to, &w), "open flat ground is visible");
        let mut w = flat_world();
        w.set_block(3, 1, 0, BlockState::STONE);
        w.set_block(3, 2, 0, BlockState::STONE);
        w.set_block(3, 3, 0, BlockState::STONE);
        assert!(!has_line_of_sight(from, to, &w), "a wall between breaks LOS");
        let very_far = Vec3::new(0.5, 1.9, 40.5);
        assert!(!has_line_of_sight(from, very_far, &w), "beyond 32 blocks LOS is false");
        assert!(has_line_of_sight(from, from + Vec3::new(0.01, 0.0, 0.0), &w), "trivial case");
    }

    /// Failure meaning: group aggro either fails to recruit nearby
    /// same-type mobs or ignores its caps.
    #[test]
    fn mob_group_aggro() {
        let mut mobs = Vec::new();
        // three glitchlings clustered at the origin, one far away,
        // one passive boar in the cluster
        for i in 0..3 {
            let mut m = MobEntity::spawn(30 + i, MobType::Glitchling, Vec3::new(i as f32 * 2.0, 1.0, 0.0));
            m.behaviour = MobBehaviourState::Wander { timer: 0.0, next_pos: None };
            mobs.push(m);
        }
        let mut far = MobEntity::spawn(40, MobType::Glitchling, Vec3::new(60.0, 1.0, 0.0));
        far.behaviour = MobBehaviourState::Wander { timer: 0.0, next_pos: None };
        let mut boar = MobEntity::spawn(41, MobType::Boar, Vec3::new(1.0, 1.0, 3.0));
        boar.behaviour = MobBehaviourState::Wander { timer: 0.0, next_pos: None };
        mobs.push(far);
        mobs.push(boar);

        mobs[0].group_ping = true;
        MobEntity::propagate_group_aggro(&mut mobs, 0, Vec3::new(0.0, 1.0, 20.0));
        assert!(matches!(mobs[1].behaviour, MobBehaviourState::Chase { react_delay, .. } if react_delay == 0.5), "{:?}", mobs[1].behaviour);
        assert!(matches!(mobs[2].behaviour, MobBehaviourState::Chase { react_delay, .. } if react_delay == 0.5));
        assert!(matches!(mobs[3].behaviour, MobBehaviourState::Wander { .. }), "distant same-type stays calm");
        assert!(matches!(mobs[4].behaviour, MobBehaviourState::Wander { .. }), "passives never join");
        // the origin keeps its own (immediate) chase
        assert!(matches!(mobs[0].behaviour, MobBehaviourState::Wander { .. } | MobBehaviourState::Chase { .. }));

        // pack cap: 6 same-type mobs all within 8 blocks → nobody joins
        let mut pack: Vec<MobEntity> = (0..6)
            .map(|i| {
                let mut m = MobEntity::spawn(50 + i, MobType::Crawler, Vec3::new(i as f32 * 1.5, 1.0, 0.0));
                m.behaviour = MobBehaviourState::Wander { timer: 0.0, next_pos: None };
                m
            })
            .collect();
        MobEntity::propagate_group_aggro(&mut pack, 0, Vec3::ZERO);
        assert!(matches!(pack[1].behaviour, MobBehaviourState::Wander { .. }), "groups >5 never mass-aggro");

        // no chain: recruited mobs never set group_ping (only self-aggro
        // and taking a hit do), so the owner never propagates from them
        let duo_world = flat_world();
        let mut duo: Vec<MobEntity> = (0..2)
            .map(|i| {
                let mut m = MobEntity::spawn(60 + i, MobType::Stalker, Vec3::new(i as f32 * 3.0 + 5.0, 1.0, 0.0));
                m.behaviour = MobBehaviourState::Wander { timer: 0.0, next_pos: None };
                m
            })
            .collect();
        MobEntity::propagate_group_aggro(&mut duo, 0, Vec3::ZERO);
        assert!(matches!(duo[1].behaviour, MobBehaviourState::Chase { .. }));
        for _ in 0..20 {
            duo[1].update(0.05, &duo_world, Vec3::new(50.0, 1.0, 0.0));
        }
        assert!(!duo[1].group_ping, "a recruited mob must not re-ping (no chains)");
    }

    /// Failure meaning: the animal set lost its stats, articulated layout,
    /// walk cycle, or deterministic spawn rules.
    #[test]
    fn animals_spawn_render_and_behave() {
        // stats: chicken/dog passive, wolf/bear hostile
        assert!(!MobType::Chicken.is_hostile() && !MobType::Dog.is_hostile());
        assert!(MobType::Wolf.is_hostile() && MobType::Bear.is_hostile());
        assert_eq!(MobType::Bear.stats().max_health, 40.0);
        // articulated layouts: every part inside the mob's local bounds,
        // every animal has at least four swinging legs (chicken two)
        for kind in [MobType::Chicken, MobType::Wolf, MobType::Dog, MobType::Bear,
                     MobType::Boar, MobType::Woolbeast] {
            let parts = animal_parts(kind, 0.4, 1.0, 0.0);
            assert!(parts.len() >= 5, "{:?} lost parts: {}", kind, parts.len());
            for p in &parts {
                assert!(p.half.iter().all(|&h| h > 0.0), "{:?} degenerate part", kind);
                let mag = p.center.iter().map(|c| c.abs()).sum::<f32>()
                    + p.half.iter().sum::<f32>();
                assert!(mag < 3.0, "{:?} part strays from the body: {:?}", kind, p.center);
            }
        }
        // chicken parts = body+head+beak+comb+2 legs; bear = body+head+2 ears+4 legs
        assert_eq!(animal_parts(MobType::Chicken, 0.0, 0.0, 0.0).len(), 6);
        assert_eq!(animal_parts(MobType::Bear, 0.0, 0.0, 0.0).len(), 8);
        // spawn rules: chickens by temperate day, wolves on cold nights,
        // bears in deep-forest days, dogs near settlements
        let day: Vec<MobType> = (0..200)
            .filter_map(|i| roll_animal_spawn(i, true, false, false, false))
            .collect();
        assert!(day.contains(&MobType::Chicken) && !day.contains(&MobType::Wolf));
        let night_cold: Vec<MobType> = (0..200)
            .filter_map(|i| roll_animal_spawn(i, false, true, false, false))
            .collect();
        assert!(night_cold.contains(&MobType::Wolf));
        let forest_day: Vec<MobType> = (0..200)
            .filter_map(|i| roll_animal_spawn(i, true, false, true, false))
            .collect();
        assert!(forest_day.contains(&MobType::Bear));
        let village: Vec<MobType> = (0..200)
            .filter_map(|i| roll_animal_spawn(i, true, false, false, true))
            .collect();
        assert!(village.contains(&MobType::Dog));
        // a bear chases and lands hits through the behaviour machine
        let w = flat_world();
        let mut bear = MobEntity::spawn(99, MobType::Bear, Vec3::new(6.0, 1.0, 0.0));
        let mut hit = false;
        for _ in 0..600 {
            if bear.update(0.05, &w, Vec3::new(0.0, 1.0, 0.0)).is_some() { hit = true; }
        }
        assert!(hit, "a bear should reach and maul the player");
    }

    /// Failure meaning: walking mobs no longer animate — legs must swing
    /// with the cycle, counter-swing in diagonal pairs, and freeze at
    /// zero amplitude.
    #[test]
    fn animal_gait_swings_legs_in_diagonal_pairs() {
        let legs_of = |phase: f32, amp: f32| -> Vec<f32> {
            // legs are the parts whose pitch is nonzero at amp 1: last 2
            // (chicken) or last 4 (quadrupeds) parts
            let parts = animal_parts(MobType::Wolf, phase, amp, 0.0);
            let n = 4;
            parts[parts.len() - n..]
                .iter()
                .map(|p| p.pitch)
                .collect()
        };
        let forward = legs_of(std::f32::consts::FRAC_PI_2, 1.0);
        let back = legs_of(-std::f32::consts::FRAC_PI_2, 1.0);
        assert!(forward.iter().any(|&p| p > 0.2), "legs swing forward: {:?}", forward);
        assert!(back.iter().any(|&p| p < -0.2), "legs swing back: {:?}", back);
        // trot: diagonal pairs share phase, the other pair is anti-phase
        assert!((forward[0] - forward[1]).abs() < 1e-4, "FL and RR move together");
        assert!((forward[0] + forward[2]).abs() < 1e-3, "FR is anti-phase to FL");
        // standing still: no swing at all
        assert!(legs_of(std::f32::consts::FRAC_PI_2, 0.0).iter().all(|&p| p.abs() < 1e-5));
        // hurt squash pulls the body down without breaking the layout
        let hurt = animal_parts(MobType::Wolf, 0.0, 0.0, 1.0);
        let calm = animal_parts(MobType::Wolf, 0.0, 0.0, 0.0);
        assert!(hurt[0].center[1] < calm[0].center[1], "flinch squashes the body");
        // woolbeast grazes with its head down while idle
        let idle = animal_parts(MobType::Woolbeast, 0.0, 0.0, 0.0);
        let walking = animal_parts(MobType::Woolbeast, 0.0, 1.0, 0.0);
        assert!(idle[1].pitch > walking[1].pitch + 0.2, "idle head lowers to graze");
    }

    /// Failure meaning: the walk cycle is not driven by actual movement,
    /// or faces snap instead of turning.
    #[test]
    fn gait_phase_tracks_speed_and_yaw_turns_smoothly() {
        let w = flat_world();
        let mut m = MobEntity::spawn(31, MobType::Wolf, Vec3::new(5.0, 1.0, 0.0));
        let player = Vec3::new(0.0, 1.0, 0.0);
        let mut peak_amp: f32 = 0.0;
        for _ in 0..30 {
            m.update(0.05, &w, player);
            peak_amp = peak_amp.max(m.gait_amp);
        }
        assert!(m.gait_phase > 1.0, "chasing advances the walk cycle: {}", m.gait_phase);
        assert!(peak_amp > 0.9, "stride amplitude rises while moving");
        // stand still: amplitude eases back to zero
        m.behaviour = MobBehaviourState::Idle { timer: 100.0 };
        m.velocity = Vec3::ZERO;
        for _ in 0..100 {
            m.update(0.05, &w, Vec3::new(500.0, 1.0, 500.0));
        }
        assert!(m.gait_amp < 0.05, "amplitude settles at rest");
        // turning is rate-limited: a wolf facing +Z told to walk -Z cannot
        // spin 180° in one 50ms tick
        let mut m2 = MobEntity::spawn(32, MobType::Wolf, Vec3::new(0.0, 1.0, -5.0));
        m2.yaw = 0.0;
        m2.behaviour = MobBehaviourState::Chase { aggro_timer: 0.0, react_delay: 0.0, unseen_for: 0.0 };
        m2.update(0.05, &w, Vec3::new(0.0, 1.0, -10.0));
        let d = (m2.yaw - std::f32::consts::PI).abs();
        assert!(d > 0.1, "yaw must not snap 180° in one tick (was {})", m2.yaw);
        assert!(d < std::f32::consts::PI, "yaw must turn the short way");
    }

    /// Failure meaning: dying mobs keep fighting, never finish dying, or
    /// fall out of the world without cleanup.
    #[test]
    fn dying_mobs_stop_fighting_and_finish() {
        let w = flat_world();
        let mut m = MobEntity::spawn(33, MobType::Glitchling, Vec3::new(1.2, 1.0, 0.0));
        assert!(m.take_hit(1000.0, Vec3::new(0.0, 1.0, 0.0)), "the hit kills");
        m.begin_death();
        assert!(!m.dead_and_gone(), "corpse has not played out yet");
        let player = Vec3::new(0.0, 1.0, 0.0);
        for _ in 0..20 {
            assert!(m.update(0.05, &w, player).is_none(), "a corpse never attacks");
        }
        // topple (0.5s) + rest (1.0s)
        for _ in 0..21 {
            m.update(0.05, &w, player);
        }
        assert!(m.dead_and_gone(), "corpse must finish: {:?}", m.death_t);
        // extra hits do not restart the animation
        m.begin_death();
        assert!(m.death_t.unwrap() > 1.4, "begin_death is idempotent");
    }

    /// Failure meaning: faction standing does not actually gate aggro.
    #[test]
    /// Loop 347: animals stop at walls. The old point-probe physics
    /// committed every horizontal move — mobs glided through 2-high
    /// walls while bouncing; now the stride is refused at the wall plane.
    #[test]
    fn mobs_stop_at_walls() {
        let mut w = flat_world();
        // a 3-high wall at x = 5 across the corridor
        for z in -20..20 {
            for y in 1..4 {
                w.set_block(5, y, z, BlockState::STONE);
            }
        }
        let mut m = MobEntity::spawn(80, MobType::Boar, Vec3::new(2.5, 1.0, 0.5));
        for _ in 0..240 {
            // 4 seconds at 60fps
            m.physics_step(1.0 / 60.0, &w, (1.0, 0.0), 2.0);
        }
        let half = MobType::Boar.stats().collision_half_width();
        assert!(m.position.x < 5.0 - half + 1e-3,
            "boar must stop before the wall, at x={} (half={})", m.position.x, half);
        assert!(m.position.x > 5.0 - half - 0.6, "and right against it, at x={}", m.position.x);
    }

    /// Loop 347: a one-block step is hopped, not a wall — the boar ends
    /// up ON the platform it walked into (wide enough that 4 seconds of
    /// walking can't cross it and drop off the far side).
    #[test]
    fn mobs_hop_one_block_steps() {
        let mut w = flat_world();
        for z in -20..20 {
            for x in 5..16 {
                w.set_block(x, 1, z, BlockState::STONE);
            }
        }
        let mut m = MobEntity::spawn(81, MobType::Boar, Vec3::new(2.5, 1.0, 0.5));
        for _ in 0..240 {
            m.physics_step(1.0 / 60.0, &w, (1.0, 0.0), 2.0);
        }
        assert!(m.position.x > 5.0, "boar climbed the step, at x={}", m.position.x);
        assert!(m.position.x < 16.0, "boar still on the platform, at x={}", m.position.x);
        assert!((m.position.y - 2.0).abs() < 0.25, "standing on top (y=2), at y={}", m.position.y);
    }

    /// Gravity still lands mobs on the floor and keeps them there.
    #[test]
    fn mobs_fall_and_rest_on_ground() {
        let w = flat_world();
        let mut m = MobEntity::spawn(82, MobType::Chicken, Vec3::new(0.5, 12.0, 0.5));
        for _ in 0..240 {
            m.physics_step(1.0 / 60.0, &w, (0.0, 0.0), 1.6);
        }
        assert!((m.position.y - 1.0).abs() < 0.02, "chicken rests at y=1, at {}", m.position.y);
    }

    fn faction_standing_gates_aggro() {
        let w = flat_world();
        let mut m = MobEntity::spawn(70, MobType::NamelessRaider, Vec3::new(3.0, 1.0, 0.0));
        assert_eq!(m.faction_id.as_deref(), Some("nameless"));
        // honored to the brim (+100): radius 0 → ignores the player entirely
        m.update_with_standing(0.05, &w, Vec3::new(0.0, 1.0, 0.0), 100);
        assert!(!matches!(m.behaviour, MobBehaviourState::Chase { .. } | MobBehaviourState::Attack { .. }),
            "at +100 standing the mob must never aggro, got {:?}", m.behaviour);
        // ...unless attacked
        m.take_hit(2.0, Vec3::new(0.0, 1.0, 0.0));
        assert!(matches!(m.behaviour, MobBehaviourState::Chase { .. }));
        // honored-but-not-max (+75): radius shrinks to a quarter, still aggroes up close
        let mut m2 = MobEntity::spawn(71, MobType::NamelessRaider, Vec3::new(2.0, 1.0, 0.0));
        m2.update_with_standing(0.05, &w, Vec3::new(0.0, 1.0, 0.0), 75);
        assert!(matches!(m2.behaviour, MobBehaviourState::Chase { .. }), "{:?}", m2.behaviour);
        assert_eq!(effective_aggro_radius(18.0, 75), 4.5);
        assert_eq!(effective_aggro_radius(18.0, -100), 36.0);
        assert_eq!(effective_aggro_radius(18.0, 0), 18.0);
    }
}
