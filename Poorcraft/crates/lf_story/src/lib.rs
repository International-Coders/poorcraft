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
    /// Reached a named world tag ("road_marker", "new_biome", ...).
    Reached(String),
    /// Broke a block (by block name, e.g. "accord_pillar").
    Broke(String),
    /// Placed a block (by block name).
    Placed(String),
    /// Interacted with an NPC archetype ("the_unmarked", ...).
    Interacted(String),
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
    Break,
    Place,
}

impl QuestType {
    /// Parse a TOML-friendly name (case-insensitive).
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "collect" => QuestType::Collect,
            "craft" => QuestType::Craft,
            "kill" => QuestType::Kill,
            "reach" => QuestType::Reach,
            "interact" => QuestType::Interact,
            "escort" => QuestType::Escort,
            "defend" => QuestType::Defend,
            "break" => QuestType::Break,
            "place" => QuestType::Place,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Quest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub objectives: Vec<QuestObjective>,
    pub act: u8,
    pub completed: bool,
    /// Issuing faction (faction quest chains; None for the base chain).
    #[serde(default)]
    pub faction: Option<String>,
    /// Standing change with the issuing faction on completion.
    #[serde(default)]
    pub standing_reward: i32,
    /// Standing ripples with other factions on completion (id, delta).
    #[serde(default)]
    pub other_standing: Vec<(String, i32)>,
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
                    (QuestEvent::Reached(tag), QuestType::Reach) => {
                        // "depth" stays exclusive to the ReachedDepth form
                        &obj.target == tag && tag != "depth"
                    }
                    (QuestEvent::Broke(block), QuestType::Break) => &obj.target == block,
                    (QuestEvent::Placed(block), QuestType::Place) => &obj.target == block,
                    (QuestEvent::Interacted(npc), QuestType::Interact) => &obj.target == npc,
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
    let base = |id: &str, title: &str, description: &str, objectives: Vec<QuestObjective>, act: u8| Quest {
        id: id.into(),
        title: title.into(),
        description: description.into(),
        objectives,
        act,
        completed: false,
        faction: None,
        standing_reward: 0,
        other_standing: Vec::new(),
    };
    vec![
        base(
            "q1_timber",
            "Punch a Tree",
            "Gather wood the honest way: 3 oak logs.",
            vec![objective(QuestType::Collect, "log", 3)],
            1,
        ),
        base(
            "q2_basics",
            "Crafting Basics",
            "Turn logs into planks at the inventory craft grid.",
            vec![objective(QuestType::Craft, "planks", 1)],
            1,
        ),
        base(
            "q3_tools",
            "Tools of the Trade",
            "Craft a crafting table and a wooden pickaxe.",
            vec![
                objective(QuestType::Craft, "crafting_table", 1),
                objective(QuestType::Craft, "wooden_pickaxe", 1),
            ],
            1,
        ),
        base(
            "q4_iron_age",
            "The Iron Age",
            "Smelt an iron ingot in a furnace.",
            vec![objective(QuestType::Collect, "iron_ingot", 1)],
            2,
        ),
        base(
            "q5_night_hunter",
            "Night Hunter",
            "The night belongs to glitches. Slay 3 hostiles.",
            vec![objective(QuestType::Kill, "hostile", 3)],
            2,
        ),
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
            faction: None,
            standing_reward: 0,
            other_standing: Vec::new(),
        };
        assert!(!quest.is_complete());
        // Complete all objectives
        for obj in &mut quest.objectives {
            obj.completed = true;
        }
        assert!(quest.is_complete());
    }

    /// The faction-quest event surface: Reach tags, Break/Place blocks,
    /// Interact NPCs.
    #[test]
    fn faction_event_kinds_advance_objectives() {
        let obj = |t: QuestType, target: &str, count: u32| QuestObjective {
            objective_type: t,
            target: target.into(),
            count,
            progress: 0,
            completed: false,
        };
        let mut log = QuestLog::new();
        log.add_quest(Quest {
            id: "f1".into(),
            title: "Mixed".into(),
            description: "d".into(),
            objectives: vec![
                obj(QuestType::Reach, "road_marker", 2),
                obj(QuestType::Break, "accord_pillar", 1),
                obj(QuestType::Place, "freeholds_daub", 1),
                obj(QuestType::Interact, "the_unmarked", 1),
            ],
            act: 1,
            completed: false,
            faction: Some("nameless".into()),
            standing_reward: 20,
            other_standing: vec![("accord".into(), -10)],
        });
        assert!(log.record_event(&QuestEvent::Reached("road_marker".into())).is_empty());
        let mut done = log.record_event(&QuestEvent::Reached("road_marker".into()));
        assert!(done.is_empty(), "not complete until all objectives finish");
        log.record_event(&QuestEvent::Broke("accord_pillar".into()));
        log.record_event(&QuestEvent::Placed("freeholds_daub".into()));
        // wrong targets must not advance
        log.record_event(&QuestEvent::Broke("stone".into()));
        log.record_event(&QuestEvent::Interacted("accord_herald".into()));
        done = log.record_event(&QuestEvent::Interacted("the_unmarked".into()));
        assert_eq!(done, vec!["f1".to_string()]);
        // the depth form of Reach is separate from tagged Reach
        let mut depth = QuestLog::new();
        depth.add_quest(Quest {
            id: "d1".into(),
            title: "Deep".into(),
            description: "d".into(),
            objectives: vec![obj(QuestType::Reach, "depth", 1)],
            act: 1,
            completed: false,
            faction: None,
            standing_reward: 0,
            other_standing: Vec::new(),
        });
        assert!(depth.record_event(&QuestEvent::Reached("depth".into())).is_empty());
        assert!(!depth.record_event(&QuestEvent::ReachedDepth(1)).is_empty());
    }
}