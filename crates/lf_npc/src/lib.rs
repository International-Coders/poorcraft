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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_villager_schedule() {
        let v = Villager::new(1, VillagerJob::Farmer, "Test".into(), [8.0, 64.0, 8.0]);
        assert!(v.should_rest(3));
        assert!(!v.should_rest(10));
    }
}
