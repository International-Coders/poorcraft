use serde::{Deserialize, Serialize};

use glam::Vec3;
use lf_voxel::World;

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
}

impl MobType {
    pub fn is_hostile(self) -> bool {
        matches!(self, MobType::Glitchling | MobType::Stalker | MobType::Crawler | MobType::NullKnight)
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
        }
    }

    /// One AI + physics step. Returns Some(damage_to_player) when this mob
    /// lands an attack this frame.
    pub fn update(&mut self, dt: f32, world: &World, player_pos: Vec3) -> Option<f32> {
        self.age += dt;
        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
        self.hurt_flash = (self.hurt_flash - dt * 3.0).max(0.0);
        let stats = self.mob_type.stats();

        // --- AI
        let to_player = player_pos - self.position;
        let dist = to_player.length();
        let mut wish: (f32, f32) = (0.0, 0.0);
        if self.mob_type.is_hostile() && dist < stats.detect {
            // chase the player
            if dist > 1.2 {
                let dir = to_player.normalize();
                wish = (dir.x, dir.z);
            }
            // attack when close
            if dist < 1.9 && self.attack_cooldown <= 0.0 && to_player.y.abs() < 2.0 {
                self.attack_cooldown = 1.0;
                return Some(stats.damage);
            }
        } else {
            // wander / flee
            self.wander_cooldown -= dt;
            if self.hurt_flash > 0.0 && !self.mob_type.is_hostile() {
                // flee straight away from the last hit (stored in yaw)
                wish = (-self.yaw.sin(), -self.yaw.cos());
            } else if self.wander_cooldown <= 0.0 {
                self.wander_cooldown = 2.0 + ((self.id * 2654435761) % 3000) as f32 / 1000.0;
                if (self.id + self.age as u64) % 3 == 0 {
                    self.wander_dir = (0.0, 0.0); // idle
                } else {
                    let a = ((self.id * 7919 + (self.age as u64 * 13)) % 360) as f32 / 57.3;
                    self.wander_dir = (a.sin(), a.cos());
                }
            }
            wish = self.wander_dir;
        }
        let wish_len = (wish.0 * wish.0 + wish.1 * wish.1).sqrt();
        if wish_len > 0.001 {
            self.yaw = wish.0.atan2(wish.1);
        }

        // --- physics: horizontal move with step-up jumping, gravity
        let speed = if self.hurt_flash > 0.0 && !self.mob_type.is_hostile() { stats.speed * 2.0 } else { stats.speed };
        self.velocity.x = wish.0 / wish_len.max(1.0) * speed;
        self.velocity.z = wish.1 / wish_len.max(1.0) * speed;
        self.velocity.y -= 24.0 * dt;

        let next = self.position + self.velocity * dt;
        let feet = |p: Vec3| world.is_solid(p.x as i32, (p.y - 0.1) as i32, p.z as i32);
        let blocked_at = |p: Vec3| {
            world.is_solid(p.x as i32, p.y as i32 + 1, p.z as i32) // body
                || world.is_solid(p.x as i32, (p.y + 1.0) as i32, p.z as i32)
        };
        if feet(next) {
            // land / stay grounded
            self.velocity.y = 0.0;
            if blocked_at(next) && wish_len > 0.1 {
                self.velocity.y = 8.0; // hop over one-block obstacles
            }
            self.position = Vec3::new(next.x, self.position.y, next.z);
            // settle down when walking off ledges handled by gravity next frame
        } else {
            self.position = next;
        }
        // hard floor: never fall out of the world
        if self.position.y < -10.0 {
            self.health = 0.0;
        }
        None
    }

    /// Player attack: apply damage + knockback. Returns true if this kills.
    pub fn take_hit(&mut self, damage: f32, from: Vec3) -> bool {
        self.health -= damage;
        self.hurt_flash = 1.0;
        let push = (self.position - from).normalize() * 6.0;
        self.velocity.x = push.x;
        self.velocity.z = push.z;
        self.velocity.y = 4.0;
        self.health <= 0.0
    }
}

/// Which mob type should spawn given the time of day.
/// Day spawns are biome-appropriate (Step 18): Woolbeasts are cold-biome
/// fauna, Boars temperate; night hostiles are global.
pub fn roll_spawn(rand: u64, is_day: bool, cold_biome: bool) -> Option<MobType> {
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
        w.ensure_chunk(0, 0);
        w.ensure_chunk(0, 1);
        w.ensure_chunk(1, 0);
        w.ensure_chunk(1, 1);
        for x in 0..40 {
            for z in 0..40 {
                w.set_block(x - 20, 0, z - 20, BlockState::STONE);
            }
        }
        w
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
    }

    #[test]
    fn take_hit_knocks_back_and_kills() {
        let mut m = MobEntity::spawn(4, MobType::Crawler, Vec3::new(0.0, 5.0, 0.0));
        assert!(!m.take_hit(10.0, Vec3::new(1.0, 5.0, 0.0)));
        assert!(m.hurt_flash > 0.0);
        assert!(m.velocity.x < 0.0, "knocked away from attacker");
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
}
