use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuestObjective {
    pub objective_type: QuestType,
    pub target: String,
    pub count: u32,
    pub completed: bool,
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
            self.completed_quests.push(quest_id.to_string());
        }
    }
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
            completed: false,
        };
        obj.completed = true;
        assert!(obj.completed);
    }

    #[test]
    fn test_quest_completeness() {
        let mut quest = Quest {
            id: "act1_first_blocks".into(),
            title: "First Blocks".into(),
            description: "Punch, craft, build shelter".into(),
            objectives: vec![
                QuestObjective { objective_type: QuestType::Collect, target: "Wood".into(), count: 10, completed: false },
                QuestObjective { objective_type: QuestType::Craft, target: "Wooden Pickaxe".into(), count: 1, completed: false },
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