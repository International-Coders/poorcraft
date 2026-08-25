use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VillagerJob {
    Farmer,
    Smith,
    Guard,
    Trader,
    Bard,
    Lorekeeper,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ScheduleSlot {
    Sleep,
    Eat,
    Work,
    Socialize,
    Patrol,
    Rest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VillagerSchedule {
    pub slots: Vec<(ScheduleSlot, i32, i32)>,
    pub location: [f32; 3],
}

impl Default for VillagerSchedule {
    fn default() -> Self {
        Self {
            slots: vec![
                (ScheduleSlot::Sleep, 0, 5),
                (ScheduleSlot::Eat, 7, 8),
                (ScheduleSlot::Work, 9, 17),
                (ScheduleSlot::Socialize, 18, 20),
                (ScheduleSlot::Rest, 22, 23),
            ],
            location: [8.0, 64.0, 8.0],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Villager {
    pub id: u64,
    pub job: VillagerJob,
    pub name: String,
    pub schedule: VillagerSchedule,
    pub position: [f32; 3],
    pub reputation: f32,
}

impl Villager {
    pub fn new(id: u64, job: VillagerJob, name: String, position: [f32; 3]) -> Self {
        Self {
            id,
            job,
            name,
            schedule: Default::default(),
            position,
            reputation: 0.0,
        }
    }

    pub fn should_rest(&self, current_hour: i32) -> bool {
        for (slot, start, end) in &self.schedule.slots {
            if *slot == ScheduleSlot::Sleep && current_hour >= *start && current_hour < *end {
                return true;
            }
        }
        false
    }
}

/// Hostile mob: Geode Guardian - protects crystal grove biome
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeodeGuardian {
    pub id: u64,
    pub position: [f32; 3],
    pub health: f32,
    pub max_health: f32,
    pub damage: f32,
    pub crystal_armor: u8,
}

impl GeodeGuardian {
    pub fn new(id: u64, position: [f32; 3]) -> Self {
        Self {
            id,
            position,
            health: 80.0,
            max_health: 80.0,
            damage: 12.0,
            crystal_armor: 3,
        }
    }

    pub fn take_damage(&mut self, amount: f32) -> bool {
        let effective = amount - (self.crystal_armor as f32 * 0.5);
        self.health = (self.health - effective.max(1.0)).max(0.0);
        self.health == 0.0
    }
}

/// Hostile mob: Cinder Crawler - found in obsidian desert biome
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CinderCrawler {
    pub id: u64,
    pub position: [f32; 3],
    pub health: f32,
    pub max_health: f32,
    pub damage: f32,
    pub ash_trail: bool,
}

impl CinderCrawler {
    pub fn new(id: u64, position: [f32; 3]) -> Self {
        Self {
            id,
            position,
            health: 45.0,
            max_health: 45.0,
            damage: 8.0,
            ash_trail: false,
        }
    }

    pub fn take_damage(&mut self, amount: f32) -> bool {
        self.health = (self.health - amount).max(0.0);
        self.health == 0.0
    }

    pub fn leave_ash_trail(&mut self) -> bool {
        self.ash_trail = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_villager_schedule() {
        let v = Villager::new(1, VillagerJob::Farmer, "Test".into(), [8.0, 64.0, 8.0]);
        assert!(v.should_rest(3));
        assert!(!v.should_rest(10));
    }

    #[test]
    fn test_geode_guardian() {
        let mut g = GeodeGuardian::new(1, [100.0, 64.0, 100.0]);
        assert_eq!(g.health, 80.0);
        let dead = g.take_damage(100.0);
        assert!(dead);
        assert_eq!(g.health, 0.0);
    }

    #[test]
    fn test_geode_guardian_armor() {
        let mut g = GeodeGuardian::new(1, [100.0, 64.0, 100.0]);
        g.take_damage(5.0);
        assert!(g.health < 80.0 && g.health > 75.0);
    }

    #[test]
    fn test_cinder_crawler() {
        let mut c = CinderCrawler::new(1, [200.0, 64.0, 200.0]);
        assert_eq!(c.health, 45.0);
        assert_eq!(c.damage, 8.0);
        assert!(!c.ash_trail);
        assert!(c.leave_ash_trail());
        let dead = c.take_damage(50.0);
        assert!(dead);
    }
}