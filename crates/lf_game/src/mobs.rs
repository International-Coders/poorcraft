use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MobEntity {
    pub id: u64,
    pub mob_type: MobType,
    pub position: [f32; 3],
    pub health: f32,
    pub max_health: f32,
    pub damage: f32,
    pub is_hostile: bool,
}

impl MobEntity {
    pub fn spawn(id: u64, mob_type: MobType, position: [f32; 3]) -> Self {
        let (max_health, damage, is_hostile) = match mob_type {
            MobType::Boar => (10.0, 0.0, false),
            MobType::Woolbeast => (15.0, 0.0, false),
            MobType::Glitchling => (20.0, 4.0, true),
            MobType::Stalker => (30.0, 6.0, true),
            MobType::Crawler => (15.0, 3.0, true),
            MobType::NullKnight => (250.0, 15.0, true), // Boss
        };
        Self {
            id,
            mob_type,
            position,
            health: max_health,
            max_health,
            damage,
            is_hostile,
        }
    }

    pub fn take_damage(&mut self, amount: f32) -> bool {
        self.health = (self.health - amount).max(0.0);
        self.health == 0.0
    }
}

pub struct MobManager {
    pub mobs: Vec<MobEntity>,
    next_id: u64,
}

impl MobManager {
    pub fn new() -> Self {
        Self { mobs: Vec::new(), next_id: 1 }
    }

    pub fn spawn_mob(&mut self, mob_type: MobType, position: [f32; 3]) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.mobs.push(MobEntity::spawn(id, mob_type, position));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mob_spawn_and_combat() {
        let mut mgr = MobManager::new();
        let id = mgr.spawn_mob(MobType::Glitchling, [0.0, 0.0, 0.0]);
        assert_eq!(mgr.mobs.len(), 1);
        let mob = &mut mgr.mobs[0];
        assert!(mob.is_hostile);
        let dead = mob.take_damage(25.0);
        assert!(dead);
        assert_eq!(mob.health, 0.0);
    }

    #[test]
    fn test_boss_null_knight() {
        let mut mgr = MobManager::new();
        mgr.spawn_mob(MobType::NullKnight, [10.0, 64.0, 10.0]);
        let boss = &mut mgr.mobs[0];
        assert_eq!(boss.mob_type, MobType::NullKnight);
        assert_eq!(boss.max_health, 250.0);
    }
}
