use serde::{Serialize, Deserialize};

pub mod vassals;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VillagerJob {
    Farmer,
    Smith,
    Guard,
    Trader,
    Bard,
    Lorekeeper,
    /// P33: dwells the tower, sells spell scrolls + reagents.
    Wizard,
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

/// One trade offer: (payment item, count, received item, count).
pub fn trade_offers(job: VillagerJob) -> &'static [(&'static str, u8, &'static str, u8)] {
    match job {
        VillagerJob::Farmer => &[
            ("coal", 2, "apple", 4),
            ("log", 2, "porkchop", 3),
        ],
        VillagerJob::Smith => &[
            ("raw_iron", 4, "iron_pickaxe", 1),
            ("iron_ingot", 3, "stone_sword", 1),
            ("coal", 6, "furnace", 1),
        ],
        VillagerJob::Trader => &[
            ("sand", 8, "glass", 4),
            ("glitch_dust", 4, "iron_ingot", 1),
            ("coal", 3, "book", 1),
        ],
        VillagerJob::Guard => &[
            ("iron_ingot", 2, "stone_axe", 1),
            ("log", 4, "chest", 1),
        ],
        VillagerJob::Bard => &[
            ("coal", 1, "book", 1),
            ("apple", 2, "book", 1),
        ],
        VillagerJob::Lorekeeper => &[
            ("book", 1, "iron_ingot", 2),
            ("null_shard", 1, "iron_sword", 1),
            // lore tomes (Step 20): the Lorekeeper keeps the valley's books
            ("tome_of_the_forge", 1, "iron_ingot", 4),
            ("tome_of_the_null", 1, "null_shard", 1),
            ("wardens_ledger", 1, "iron_ingot", 6),
        ],
        // P33: the wizard teaches the bounded set — every scroll is here,
        // and reagents flow both ways.
        VillagerJob::Wizard => &[
            ("glitch_dust", 6, "scroll_of_firebolt", 1),
            ("null_shard", 1, "scroll_of_gale_step", 1),
            ("glitch_dust", 12, "scroll_of_ward", 1),
            ("book", 2, "scroll_of_hearthlight", 1),
            ("iron_ingot", 2, "glitch_dust", 3),
        ],
    }
}


/// C1 (ai-npc-assets): where an activity happens. The client resolves the
/// kind to the nearest matching block around the NPC's home structure.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotLocation {
    Bed,
    Table,
    Workstation,
    Gather,
    Door,
}

/// One enriched schedule entry. Times are day fractions (0.0 = midnight,
/// 1.0 = the next midnight), matching `lf_game::TimeOfDay::fraction()`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub activity: ScheduleSlot,
    pub location: SlotLocation,
    pub time_start: f32,
    pub time_end: f32,
}

/// The canonical enriched day every NPC follows (C1). TOML overrides in
/// `lore/npcs.toml` replace entries per archetype; this is the fallback.
pub fn default_schedule_entries() -> Vec<ScheduleEntry> {
    use ScheduleSlot as S;
    use SlotLocation as L;
    vec![
        ScheduleEntry { activity: S::Sleep, location: L::Bed, time_start: 0.0, time_end: 0.25 },
        ScheduleEntry { activity: S::Eat, location: L::Table, time_start: 0.25, time_end: 0.35 },
        ScheduleEntry { activity: S::Work, location: L::Workstation, time_start: 0.35, time_end: 0.75 },
        ScheduleEntry { activity: S::Socialize, location: L::Gather, time_start: 0.75, time_end: 0.85 },
        ScheduleEntry { activity: ScheduleSlot::Patrol, location: SlotLocation::Door, time_start: 0.85, time_end: 1.0 },
    ]
}

/// The enriched entry covering `day_fraction` (defaults fill the whole day).
pub fn enriched_slot_at(entries: &[ScheduleEntry], day_fraction: f32) -> ScheduleEntry {
    let t = day_fraction.rem_euclid(1.0);
    for e in entries {
        let (a, b) = if e.time_start <= e.time_end {
            (e.time_start, e.time_end)
        } else {
            (e.time_start, e.time_end + 1.0) // wraps midnight
        };
        let tt = if t < a && b > 1.0 { t + 1.0 } else { t };
        if tt >= a && tt < b {
            return e.clone();
        }
    }
    // the default table covers 0.0..1.0; a custom table with a hole falls
    // back to wandering near home
    ScheduleEntry { activity: ScheduleSlot::Rest, location: SlotLocation::Gather, time_start: 0.0, time_end: 1.0 }
}

/// C2: what the NPC is visibly doing right now. Drives the model offset /
/// rotation applied during rendering and the dialogue posture.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NpcActivityState {
    Walking,
    Idle,
    Working,
    Eating,
    Sleeping,
    Socializing,
}

impl Default for NpcActivityState {
    fn default() -> Self {
        NpcActivityState::Idle
    }
}

/// The activity state for a schedule slot, given whether the NPC is still
/// en route to the slot's location.
pub fn activity_state_for(entry: &ScheduleEntry, en_route: bool) -> NpcActivityState {
    use ScheduleSlot as S;
    if en_route {
        return NpcActivityState::Walking;
    }
    match entry.activity {
        S::Sleep => NpcActivityState::Sleeping,
        S::Eat => NpcActivityState::Eating,
        S::Work => NpcActivityState::Working,
        S::Socialize => NpcActivityState::Socializing,
        S::Patrol => NpcActivityState::Walking,
        S::Rest => NpcActivityState::Idle,
    }
}

/// C2 dialogue posture: the opening line an NPC gives per activity state.
pub fn activity_opening(activity: NpcActivityState) -> &'static str {
    match activity {
        NpcActivityState::Sleeping => "Mmh...? It's the middle of the night. Come back when the sun's up.",
        NpcActivityState::Working => "I'm busy. What do you need?",
        NpcActivityState::Eating => "Can't it wait until after I've eaten? ...Fine.",
        NpcActivityState::Socializing => "Ah, good timing. I was just thinking about home.",
        NpcActivityState::Walking => "Hold on — walking here.",
        NpcActivityState::Idle => "Hello.",
    }
}

/// Sleep-state NPCs only talk; trade and quests wait for daylight.
pub fn activity_allows_trade(activity: NpcActivityState) -> bool {
    activity != NpcActivityState::Sleeping
}

/// C4: what happened between this NPC and the player.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NpcEvent {
    Gifted,
    Traded,
    QuestGiven,
    QuestCompleted,
    Attacked,
    Dismissed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InteractionRecord {
    pub event: NpcEvent,
    pub day: u32,
}

/// C4: the last two significant interactions, persisted with the world.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NpcMemory {
    pub last_interaction: Option<InteractionRecord>,
    pub prior_interaction: Option<InteractionRecord>,
}

impl NpcMemory {
    /// Push a new record; `prior` becomes the older one. Nothing is
    /// recorded for the very first interaction's `prior`.
    pub fn record(&mut self, event: NpcEvent, day: u32) {
        self.prior_interaction = self.last_interaction.take();
        self.last_interaction = Some(InteractionRecord { event, day });
    }

    /// Memory older than 5 in-game days is forgotten (stranger again).
    pub fn recall(&self, current_day: u32) -> Option<NpcEvent> {
        let last = self.last_interaction.as_ref()?;
        if current_day.saturating_sub(last.day) > 5 {
            return None;
        }
        Some(last.event)
    }
}

/// C4: the memory-referencing opening line, if the memory is fresh
/// (within 5 days) and the event is one NPCs comment on.
pub fn memory_greeting(event: NpcEvent) -> Option<&'static str> {
    match event {
        NpcEvent::QuestCompleted => Some("Back again? Good. I have more work."),
        NpcEvent::Traded => Some("Still carrying those items I gave you?"),
        NpcEvent::Attacked => Some("I remember what you did to us. Be careful."),
        _ => None,
    }
}

/// C3: one-line reactions to nearby world events (delivered through the
/// existing chat system; standing economics are handled elsewhere).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NpcReactionEvent {
    /// The player broke a block inside this NPC's faction structure.
    BlockBrokenInStructure,
    /// Combat started within 24 blocks; the NPC runs.
    CombatStarted,
    /// The player gifted an item.
    GiftedItem { item_id: String },
    /// A same-faction companion hit morale 0 nearby.
    CompanionMoraleZero { companion_name: String },
    /// The player crossed +75 standing with this NPC's faction.
    FactionHonored { title: String },
}

pub fn reaction_line(name: &str, event: &NpcReactionEvent) -> String {
    match event {
        NpcReactionEvent::BlockBrokenInStructure => {
            format!("[{}]: Hey! Watch what you're doing.", name)
        }
        NpcReactionEvent::CombatStarted => {
            format!("[{}]: Fight! Everyone inside!", name)
        }
        NpcReactionEvent::GiftedItem { item_id } => {
            format!("[{}]: A {}? That's thoughtful of you. Thank you.", name, item_id.replace('_', " "))
        }
        NpcReactionEvent::CompanionMoraleZero { companion_name } => {
            format!("[{}]: I see {} has had enough. Can't say I'm surprised.", name, companion_name)
        }
        NpcReactionEvent::FactionHonored { title } => {
            format!("[{}]: The {} walks among us.", name, title)
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
    /// Faction id (lore-and-visuals A): drives the faction skin + standing
    /// gates in trade.
    #[serde(default)]
    pub faction: Option<String>,
    /// NPC roster archetype id (hireable villagers carry their companion
    /// form + hire fee in lore/npcs.toml).
    #[serde(default)]
    pub archetype: Option<String>,
    /// C2: what the NPC is visibly doing (drives render offset + posture).
    #[serde(default)]
    pub activity: NpcActivityState,
    /// C3: combat panic marker — flee until this world tick.
    #[serde(default)]
    pub flee_until_ticks: u64,
    /// C1: workstation anchor resolved at spawn (nearest faction block).
    #[serde(default)]
    pub workstation_pos: Option<[i32; 3]>,
    /// C4: the last two significant interactions with the player.
    #[serde(default)]
    pub memory: NpcMemory,
    /// king-quest: sworn vassal state — Some once the player (at Honored
    /// standing) has pressed this villager into service.
    #[serde(default)]
    pub vassal: Option<vassals::VassalState>,
    /// Facing (radians, atan2(x, z) convention) — written by the client
    /// movement loop so walkers face where they go.
    #[serde(default)]
    pub yaw: f32,
    /// Walk-cycle phase (radians) + amplitude, advanced while moving.
    #[serde(default)]
    pub walk_phase: f32,
    #[serde(default)]
    pub walk_amp: f32,
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
            faction: None,
            archetype: None,
            activity: NpcActivityState::Idle,
            flee_until_ticks: 0,
            workstation_pos: None,
            memory: NpcMemory::default(),
            vassal: None,
            yaw: 0.0,
            walk_phase: 0.0,
            walk_amp: 0.0,
        }
    }

    /// C4: record an interaction (last shifts to prior).
    pub fn record_interaction(&mut self, event: NpcEvent, day: u32) {
        self.memory.record(event, day);
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
    fn trade_offers_are_coherent() {
        for job in [VillagerJob::Farmer, VillagerJob::Smith, VillagerJob::Trader,
                    VillagerJob::Guard, VillagerJob::Bard, VillagerJob::Lorekeeper] {
            let offers = trade_offers(job);
            assert!(!offers.is_empty(), "{:?} has no offers", job);
            for (give, gn, get, rn) in offers {
                assert!(*gn > 0 && *rn > 0, "zero-count trade");
                assert_ne!(give, get, "self-trade {:?} {}", job, give);
            }
        }
        // smith sells better picks than raw material cost
        let smith = trade_offers(VillagerJob::Smith);
        assert!(smith.iter().any(|(g, _, r, _)| *g == "raw_iron" && *r == "iron_pickaxe"));
    }

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

    /// Failure meaning: the enriched schedule does not move the NPC
    /// through the C1 day at the right boundaries.
    #[test]
    fn npc_schedule_activity() {
        use ScheduleSlot as S;
        use SlotLocation as L;
        let table = default_schedule_entries();
        let at = |t: f32| {
            let e = enriched_slot_at(&table, t);
            (e.activity, e.location)
        };
        // the C1 boundary table: sleep → eat → work → socialize → patrol
        assert_eq!(at(0.0), (S::Sleep, L::Bed), "midnight = bed");
        assert_eq!(at(0.1), (S::Sleep, L::Bed), "3am-ish still asleep");
        assert_eq!(at(0.249), (S::Sleep, L::Bed));
        assert_eq!(at(0.25), (S::Eat, L::Table), "6am = breakfast");
        assert_eq!(at(0.349), (S::Eat, L::Table));
        assert_eq!(at(0.35), (S::Work, L::Workstation), "8:24am = work");
        assert_eq!(at(0.5), (S::Work, L::Workstation), "noon = work");
        assert_eq!(at(0.749), (S::Work, L::Workstation));
        assert_eq!(at(0.75), (S::Socialize, L::Gather), "6pm = socialize");
        assert_eq!(at(0.849), (S::Socialize, L::Gather));
        assert_eq!(at(0.85), (S::Patrol, L::Door), "20:24 = head home");
        assert_eq!(at(0.999), (S::Patrol, L::Door));
        // activity states follow the slot, walking while en route
        let work = enriched_slot_at(&table, 0.5);
        assert_eq!(activity_state_for(&work, true), NpcActivityState::Walking);
        assert_eq!(activity_state_for(&work, false), NpcActivityState::Working);
        let sleep = enriched_slot_at(&table, 0.1);
        assert_eq!(activity_state_for(&sleep, false), NpcActivityState::Sleeping);
        // sleeping NPCs do not open the trade menu
        assert!(!activity_allows_trade(NpcActivityState::Sleeping));
        assert!(activity_allows_trade(NpcActivityState::Working));
    }

    /// Failure meaning: NPC memory does not survive the save round-trip
    /// or the 5-day forgetting window is wrong.
    #[test]
    fn npc_memory_persistence() {
        let mut v = Villager::new(7, VillagerJob::Trader, "Maren".into(), [8.0, 64.0, 8.0]);
        v.record_interaction(NpcEvent::Traded, 12);
        v.record_interaction(NpcEvent::QuestCompleted, 14);
        assert_eq!(v.memory.last_interaction.as_ref().unwrap().event, NpcEvent::QuestCompleted);
        assert_eq!(v.memory.prior_interaction.as_ref().unwrap().event, NpcEvent::Traded);
        // serialize exactly the way the world save does (ClientSave JSON)
        let bytes = serde_json::to_vec(&v).unwrap();
        let loaded: Villager = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(loaded.memory, v.memory, "memory survives the save");
        // fresh within the window, forgotten after 5 days
        assert_eq!(loaded.memory.recall(14), Some(NpcEvent::QuestCompleted));
        assert_eq!(loaded.memory.recall(19), Some(NpcEvent::QuestCompleted));
        assert_eq!(loaded.memory.recall(20), None, "5+ days: stranger again");
        // only the cited events produce greetings
        assert_eq!(memory_greeting(NpcEvent::QuestCompleted), Some("Back again? Good. I have more work."));
        assert_eq!(memory_greeting(NpcEvent::Gifted), None);
        // an old save without the memory field still loads (serde default)
        let legacy = serde_json::json!({
            "id": 9, "job": "Trader", "name": "Old", "schedule": {"slots": [], "location": [0.0, 64.0, 0.0]},
            "position": [0.0, 64.0, 0.0], "reputation": 0.0
        });
        let old: Villager = serde_json::from_value(legacy).unwrap();
        assert_eq!(old.memory, NpcMemory::default());
        assert_eq!(old.activity, NpcActivityState::Idle);
    }

    /// Failure meaning: reaction lines drift from the C3 wording table.
    #[test]
    fn npc_reaction_lines() {
        assert_eq!(
            reaction_line("Bran", &NpcReactionEvent::BlockBrokenInStructure),
            "[Bran]: Hey! Watch what you're doing."
        );
        assert_eq!(
            reaction_line("Bran", &NpcReactionEvent::CompanionMoraleZero { companion_name: "Ysolde".into() }),
            "[Bran]: I see Ysolde has had enough. Can't say I'm surprised."
        );
        assert_eq!(
            reaction_line("Bran", &NpcReactionEvent::FactionHonored { title: "Warden".into() }),
            "[Bran]: The Warden walks among us."
        );
        match reaction_line("Bran", &NpcReactionEvent::GiftedItem { item_id: "iron_ingot".into() }) {
            line => assert!(line.contains("iron ingot") && line.contains("Thank you"), "{}", line),
        }
    }
}
