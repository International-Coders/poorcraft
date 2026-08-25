use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChronicleEvent {
    pub id: String,
    pub event_type: EventType,
    pub in_game_date: u64,
    pub location: [f32; 3],
    pub actors: Vec<String>,
    pub payload: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    FirstCraft,
    FirstBlood,
    StructureCompleted,
    BossSlain,
    Death,
    Discovery,
    GreatTrade,
    VillageFounded,
    ActCompleted,
    RuneApplied,
    ItemCrafted,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct SagaGenerator;

impl SagaGenerator {
    pub fn generate_title(events: &[ChronicleEvent]) -> String {
        if events.is_empty() {
            return "The Chronicle Begins".into();
        }
        let first = &events[0];
        match first.event_type {
            EventType::FirstCraft => "First Steps in the Forge".into(),
            EventType::FirstBlood => "Blood in the Early Dawn".into(),
            EventType::StructureCompleted => "A New Hall Rises".into(),
            EventType::BossSlain => "Victory Over the Corrupted".into(),
            EventType::Death => "The Smith's Final Rest".into(),
            EventType::Discovery => "New Lands Found".into(),
            EventType::GreatTrade => "Wealth Beyond the Hills".into(),
            EventType::VillageFounded => "A Village is Born".into(),
            EventType::ActCompleted => "Act Concluded".into(),
            _ => "A Chapter of the Chronicle".into(),
        }
    }

    pub fn generate_chapter(events: &[ChronicleEvent]) -> String {
        if events.is_empty() {
            return "The world was still and unfinished.".into();
        }

        let default_actor = "the Smith".to_string();
        let first_actor = events.first().and_then(|e| e.actors.first()).unwrap_or(&default_actor);
        let location_desc = Self::describe_location(events);
        let mut chapter = format!("In the realm of {}, {}", location_desc, first_actor);

        if events.len() >= 2 {
            let second = &events[1];
            let default_other = "another".to_string();
            let second_actor = second.actors.first().unwrap_or(&default_other);
            chapter.push_str(&format!(" and {} did ", second_actor));
            match second.event_type {
                EventType::FirstCraft => chapter.push_str("craft their first tools"),
                EventType::BossSlain => chapter.push_str("defeated the boss"),
                _ => chapter.push_str("performed a great deed"),
            }
        }
        chapter.push_str("...");
        chapter
    }

    fn describe_location(events: &[ChronicleEvent]) -> String {
        if events.is_empty() { return "the world".into(); }
        let pos = events[0].location;
        if pos[1] < 32.0 { "deep underground".into() }
        else if pos[1] > 128.0 { "the sky lands".into() }
        else { "the surface".into() }
    }

    pub fn export_markdown(events: &[ChronicleEvent]) -> String {
        let mut md = String::new();
        md.push_str("# Loreforge Chronicle\n\n");
        md.push_str(&Self::generate_title(events));
        md.push_str("\n\n");
        md.push_str(&Self::generate_chapter(events));
        md.push_str("\n\n---\n\n");
        md.push_str("## Events Logged\n\n");
        for event in events {
            md.push_str(&format!("- **{}**: {} — {}\n", event.event_type, event.payload, event.actors.join(", ")));
        }
        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saga_title_generation() {
        let events = vec![
            ChronicleEvent {
                id: "1".into(),
                event_type: EventType::FirstCraft,
                in_game_date: 1,
                location: [8.0, 64.0, 8.0],
                actors: vec!["Player1".into()],
                payload: "Crafted a wooden pickaxe".into(),
            }
        ];
        let title = SagaGenerator::generate_title(&events);
        assert!(!title.is_empty());
    }

    #[test]
    fn test_saga_chapter_generation() {
        let events = vec![
            ChronicleEvent {
                id: "1".into(),
                event_type: EventType::FirstCraft,
                in_game_date: 1,
                location: [8.0, 64.0, 8.0],
                actors: vec!["Player1".into()],
                payload: "Crafted a wooden pickaxe".into(),
            }
        ];
        let chapter = SagaGenerator::generate_chapter(&events);
        assert!(!chapter.is_empty());
        assert!(chapter.contains("Player1"));
    }

    #[test]
    fn test_export_markdown() {
        let events = vec![
            ChronicleEvent {
                id: "1".into(),
                event_type: EventType::FirstCraft,
                in_game_date: 1,
                location: [8.0, 64.0, 8.0],
                actors: vec!["Player1".into()],
                payload: "Crafted a wooden pickaxe".into(),
            }
        ];
        let md = SagaGenerator::export_markdown(&events);
        assert!(md.contains("Loreforge Chronicle"));
        assert!(md.contains("First Steps in the Forge"));
    }
}
