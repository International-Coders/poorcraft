use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuestObjective {
    pub objective_type: QuestType,
    pub target: String,
    pub count: u32,
    pub progress: u32,
    pub completed: bool,
}

/// A gameplay event that can advance quest objectives.
#[derive(Clone, Debug, PartialEq)]
pub enum QuestEvent {
    Collected(String),
    Crafted(String),
    Killed(String),
    /// Reached a depth in blocks (objective target is a y threshold).
    ReachedDepth(i32),
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum QuestType {
    Collect,
    Craft,
    Kill,
    Reach,
    Interact,
    Escort,
    Defend,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Quest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub objectives: Vec<QuestObjective>,
    pub act: u8,
    pub completed: bool,
}

impl Quest {
    pub fn is_complete(&self) -> bool {
        self.objectives.iter().all(|o| o.completed)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuestLog {
    pub quests: Vec<Quest>,
    pub completed_quests: Vec<String>,
}

impl QuestLog {
    pub fn new() -> Self {
        Self {
            quests: Vec::new(),
            completed_quests: Vec::new(),
        }
    }

    pub fn add_quest(&mut self, quest: Quest) {
        self.quests.push(quest);
    }

    pub fn complete_quest(&mut self, quest_id: &str) {
        if let Some(quest) = self.quests.iter_mut().find(|q| q.id == quest_id) {
            quest.completed = true;
            if !self.completed_quests.contains(&quest_id.to_string()) {
                self.completed_quests.push(quest_id.to_string());
            }
        }
    }

    /// Feed a gameplay event; returns ids of quests that just completed.
    pub fn record_event(&mut self, event: &QuestEvent) -> Vec<String> {
        let mut finished = Vec::new();
        for quest in self.quests.iter_mut() {
            if quest.completed {
                continue;
            }
            for obj in quest.objectives.iter_mut() {
                if obj.completed {
                    continue;
                }
                let matches = match (&event, obj.objective_type) {
                    (QuestEvent::Collected(item), QuestType::Collect) => item == &obj.target,
                    (QuestEvent::Crafted(item), QuestType::Craft) => item == &obj.target,
                    (QuestEvent::Killed(kind), QuestType::Kill) => {
                        obj.target == "hostile" || &obj.target == kind
                    }
                    (QuestEvent::ReachedDepth(y), QuestType::Reach) => {
                        obj.target == "depth" && *y <= obj.count as i32
                    }
                    _ => false,
                };
                if matches {
                    obj.progress += 1;
                    if obj.progress >= obj.count {
                        obj.completed = true;
                    }
                }
            }
            if quest.is_complete() && !quest.completed {
                quest.completed = true;
                self.completed_quests.push(quest.id.clone());
                finished.push(quest.id.clone());
            }
        }
        finished
    }
}

/// The starter quest chain.
pub fn starter_quests() -> Vec<Quest> {
    let objective = |t: QuestType, target: &str, count: u32| QuestObjective {
        objective_type: t,
        target: target.to_string(),
        count,
        progress: 0,
        completed: false,
    };
    vec![
        Quest {
            id: "q1_timber".into(),
            title: "Punch a Tree".into(),
            description: "Gather wood the honest way: 3 oak logs.".into(),
            objectives: vec![objective(QuestType::Collect, "log", 3)],
            act: 1,
            completed: false,
        },
        Quest {
            id: "q2_basics".into(),
            title: "Crafting Basics".into(),
            description: "Turn logs into planks at the inventory craft grid.".into(),
            objectives: vec![objective(QuestType::Craft, "planks", 1)],
            act: 1,
            completed: false,
        },
        Quest {
            id: "q3_tools".into(),
            title: "Tools of the Trade".into(),
            description: "Craft a crafting table and a wooden pickaxe.".into(),
            objectives: vec![
                objective(QuestType::Craft, "crafting_table", 1),
                objective(QuestType::Craft, "wooden_pickaxe", 1),
            ],
            act: 1,
            completed: false,
        },
        Quest {
            id: "q4_iron_age".into(),
            title: "The Iron Age".into(),
            description: "Smelt an iron ingot in a furnace.".into(),
            objectives: vec![objective(QuestType::Collect, "iron_ingot", 1)],
            act: 2,
            completed: false,
        },
        Quest {
            id: "q5_night_hunter".into(),
            title: "Night Hunter".into(),
            description: "The night belongs to glitches. Slay 3 hostiles.".into(),
            objectives: vec![objective(QuestType::Kill, "hostile", 3)],
            act: 2,
            completed: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quest_objective() {
        let mut obj = QuestObjective {
            objective_type: QuestType::Kill,
            target: "Glitchling".into(),
            count: 5,
            progress: 0,
            completed: false,
        };
        obj.completed = true;
        assert!(obj.completed);
    }

    #[test]
    fn events_advance_and_complete_quests() {
        let mut log = QuestLog::new();
        for q in starter_quests() {
            log.add_quest(q);
        }
        // gather 3 logs
        for _ in 0..2 {
            assert!(log.record_event(&QuestEvent::Collected("log".into())).is_empty());
        }
        let done = log.record_event(&QuestEvent::Collected("log".into()));
        assert_eq!(done, vec!["q1_timber".to_string()]);
        // hostile kills count via the "hostile" wildcard
        for _ in 0..2 {
            log.record_event(&QuestEvent::Killed("Glitchling".into()));
        }
        let done = log.record_event(&QuestEvent::Killed("Crawler".into()));
        assert_eq!(done, vec!["q5_night_hunter".to_string()]);
        // crafting completes q2/q3 progressively
        log.record_event(&QuestEvent::Crafted("planks".into()));
        assert!(log.completed_quests.contains(&"q2_basics".to_string()));
        log.record_event(&QuestEvent::Crafted("crafting_table".into()));
        log.record_event(&QuestEvent::Crafted("wooden_pickaxe".into()));
        assert!(log.completed_quests.contains(&"q3_tools".to_string()));
        // unrelated events don't advance
        assert!(log.record_event(&QuestEvent::Collected("sand".into())).is_empty());
    }

    #[test]
    fn test_quest_completeness() {
        let mut quest = Quest {
            id: "act1_first_blocks".into(),
            title: "First Blocks".into(),
            description: "Punch, craft, build shelter".into(),
            objectives: vec![
                QuestObjective { objective_type: QuestType::Collect, target: "Wood".into(), count: 10, progress: 0, completed: false },
                QuestObjective { objective_type: QuestType::Craft, target: "Wooden Pickaxe".into(), count: 1, progress: 0, completed: false },
            ],
            act: 1,
            completed: false,
        };
        assert!(!quest.is_complete());
        // Complete all objectives
        for obj in &mut quest.objectives {
            obj.completed = true;
        }
        assert!(quest.is_complete());
    }
}