//! Lore data layer (lore-and-visuals Section A1): TOML-driven factions,
//! world history, the NPC roster, and dialogue, parsed at boot following the
//! same pattern as the mod registry. No hardcoded lore strings live in the
//! engine crates — everything reads from `lore/*.toml`.
//!
//! Data files (repo root `lore/`):
//! - `factions.toml` — the six factions, relationships, standing-event table
//! - `world_events.toml` — canonical Era/Year history that the chronicle
//!   references by name
//! - `npcs.toml` — villager archetypes + companion forms (wages, skills)
//! - `dialogue.toml` — state-conditional NPC lines + companion chat lines
//! - `quests_factions.toml` — the 12 faction quests (parsed into
//!   `lf_story::Quest`)

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use lf_worldgen::Biome;

// ---------------------------------------------------------------------------
// Factions
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ideology {
    Coalition,
    Industrial,
    Arcane,
    Traditionalist,
    Scholar,
    Outlaw,
}

impl Ideology {
    pub fn from_name(s: &str) -> Option<Self> {
        Some(match s {
            "coalition" => Ideology::Coalition,
            "industrial" => Ideology::Industrial,
            "arcane" => Ideology::Arcane,
            "traditionalist" => Ideology::Traditionalist,
            "scholar" => Ideology::Scholar,
            "outlaw" => Ideology::Outlaw,
            _ => return None,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Alignment {
    Lawful,
    Neutral,
    Hostile,
}

impl Alignment {
    pub fn from_name(s: &str) -> Option<Self> {
        Some(match s {
            "lawful" => Alignment::Lawful,
            "neutral" => Alignment::Neutral,
            "hostile" => Alignment::Hostile,
            _ => return None,
        })
    }
}

/// Threshold titles (DIALOGUE_FRAMEWORK table) used in chronicle entries
/// when a standing band is crossed.
#[derive(Clone, Debug, Deserialize)]
pub struct FactionTitles {
    pub honored: String,
    pub friendly: String,
    pub known: String,
    pub cold: String,
    pub enemy: String,
}

#[derive(Clone, Debug)]
pub struct FactionDef {
    pub id: String,
    pub full_name: String,
    pub short_name: String,
    pub ideology: Ideology,
    pub alignment: Alignment,
    pub home_biomes: Vec<Biome>,
    pub color: [u8; 3],
    pub symbol: String,
    /// Starting standing for fresh players (0; The Nameless: -50).
    pub starting_standing: i32,
    pub titles: FactionTitles,
}

#[derive(Clone, Debug, Deserialize)]
struct FactionRaw {
    id: String,
    full_name: String,
    short_name: String,
    ideology: String,
    alignment: String,
    home_biomes: Vec<String>,
    color: Vec<u8>,
    symbol: String,
    #[serde(default)]
    starting_standing: i32,
    titles: FactionTitles,
}

impl FactionRaw {
    fn parse(self) -> Option<FactionDef> {
        let ideology = Ideology::from_name(&self.ideology)?;
        let alignment = Alignment::from_name(&self.alignment)?;
        let mut home_biomes = Vec::new();
        for name in &self.home_biomes {
            home_biomes.push(biome_from_key(name)?);
        }
        if self.color.len() != 3 {
            return None;
        }
        Some(FactionDef {
            id: self.id,
            full_name: self.full_name,
            short_name: self.short_name,
            ideology,
            alignment,
            home_biomes,
            color: [self.color[0], self.color[1], self.color[2]],
            symbol: self.symbol,
            starting_standing: self.starting_standing,
            titles: self.titles,
        })
    }
}

/// Biome variant-name lookup ("Meadow", "Volcanic" ...).
pub fn biome_from_key(name: &str) -> Option<Biome> {
    Biome::ALL.into_iter().find(|b| format!("{:?}", b) == name)
}

/// Directed faction-to-faction stance (FACTIONS_OVERVIEW matrix).
#[derive(Clone, Debug, Deserialize)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub relation: String,
}

impl Relationship {
    /// Relations that treat the other faction as an opponent: when the
    /// player becomes honored with one side, these drift colder.
    pub fn is_rival(&self) -> bool {
        matches!(self.relation.as_str(), "enemy" | "tense" | "cold" | "wary")
    }
}

/// Standard standing-change magnitudes (FACTIONS_OVERVIEW table).
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct StandingEvents {
    pub quest_complete: i32,
    pub quest_fail: i32,
    pub trade_ten_items: i32,
    pub attack_npc: i32,
    pub kill_npc: i32,
    pub destroy_structure_block: i32,
    pub build_faction_blocks: i32,
    pub discover_structure: i32,
    pub rival_honored: i32,
}

impl Default for StandingEvents {
    fn default() -> Self {
        Self {
            quest_complete: 15,
            quest_fail: -10,
            trade_ten_items: 2,
            attack_npc: -20,
            kill_npc: -35,
            destroy_structure_block: -5,
            build_faction_blocks: 3,
            discover_structure: 5,
            rival_honored: -10,
        }
    }
}

#[derive(Default, Deserialize)]
struct FactionFile {
    #[serde(default)]
    faction: Vec<FactionRaw>,
    #[serde(default)]
    relationship: Vec<Relationship>,
    #[serde(default)]
    standing_events: Option<StandingEvents>,
}

// ---------------------------------------------------------------------------
// Standing state
// ---------------------------------------------------------------------------

/// Standing bands with gameplay meaning (threshold titles in the docs).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StandingBand {
    Honored,
    Friendly,
    Known,
    Cold,
    Enemy,
}

impl StandingBand {
    pub fn of(value: i32) -> Option<Self> {
        Some(match value {
            v if v >= 75 => StandingBand::Honored,
            v if v >= 50 => StandingBand::Friendly,
            v if v >= 30 => StandingBand::Known,
            v if v <= -75 => StandingBand::Enemy,
            v if v <= -30 => StandingBand::Cold,
            _ => return None,
        })
    }

    pub fn title<'a>(&self, faction: &'a FactionDef) -> &'a str {
        match self {
            StandingBand::Honored => &faction.titles.honored,
            StandingBand::Friendly => &faction.titles.friendly,
            StandingBand::Known => &faction.titles.known,
            StandingBand::Cold => &faction.titles.cold,
            StandingBand::Enemy => &faction.titles.enemy,
        }
    }
}

/// The result of one standing change: what it was, what it is, and the new
/// band if a threshold was crossed (drives chronicle entries + HUD pulse).
#[derive(Clone, Debug)]
pub struct StandingChange {
    pub faction: String,
    pub old: i32,
    pub new: i32,
    pub band: Option<StandingBand>,
}

/// Per-player, per-faction standing (-100..+100). Persisted in ClientSave.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StandingState {
    #[serde(default)]
    pub values: BTreeMap<String, i32>,
}

impl StandingState {
    /// Fresh save: every faction at its documented starting standing
    /// (0 neutral factions, -50 for The Nameless).
    pub fn starting(registry: &LoreRegistry) -> Self {
        let mut values = BTreeMap::new();
        for f in &registry.factions {
            values.insert(f.id.clone(), f.starting_standing);
        }
        Self { values }
    }

    pub fn get(&self, faction: &str) -> i32 {
        self.values.get(faction).copied().unwrap_or(0)
    }

    pub fn add(&mut self, faction: &str, delta: i32) -> StandingChange {
        let old = self.get(faction);
        let new = (old + delta).clamp(-100, 100);
        self.values.insert(faction.to_string(), new);
        StandingChange {
            faction: faction.to_string(),
            old,
            new,
            band: StandingBand::of(new).filter(|b| StandingBand::of(old) != Some(*b)),
        }
    }

    /// Can the player hire from `faction`? (docs: companions at >= +75)
    pub fn can_hire_from(&self, faction: &str) -> bool {
        self.get(faction) >= 75
    }

    /// D1 gates: hostile dialogue / refuse trade at <= -30, bonus trade at
    /// >= 50.
    pub fn refuses_trade(&self, faction: &str) -> bool {
        self.get(faction) <= -30
    }

    pub fn offers_bonus_trade(&self, faction: &str) -> bool {
        self.get(faction) >= 50
    }
}

// ---------------------------------------------------------------------------
// World events (canonical history the chronicle references by name)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WorldEventTrigger {
    QuestCompleted,
    StructureDiscovered,
    StandingHonored,
}

impl WorldEventTrigger {
    fn from_name(s: &str) -> Option<Self> {
        Some(match s {
            "quest_completed" => WorldEventTrigger::QuestCompleted,
            "structure_discovered" => WorldEventTrigger::StructureDiscovered,
            "standing_honored" => WorldEventTrigger::StandingHonored,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct WorldEvent {
    pub id: String,
    pub era: u8,
    pub year: u32,
    pub name: String,
    pub description: String,
    pub factions: Vec<String>,
    /// Which happenings can surface this event in the chronicle.
    pub triggers: Vec<WorldEventTrigger>,
}

impl WorldEvent {
    /// "Era I, Year 214" — the in-world date format from WORLD_HISTORY.md.
    pub fn date(&self) -> String {
        let era = match self.era {
            1 => "I",
            2 => "II",
            3 => "III",
            _ => "IV",
        };
        format!("Era {}, Year {}", era, self.year)
    }

    pub fn era_name(era: u8) -> &'static str {
        match era {
            1 => "the Age of First Flame",
            2 => "the Age of Ruin",
            3 => "the Age of Rebuild",
            _ => "the Age of Reckoning",
        }
    }
}

#[derive(Deserialize)]
struct WorldEventRaw {
    id: String,
    era: u8,
    year: u32,
    name: String,
    description: String,
    factions: Vec<String>,
    /// Which happenings surface this event in the chronicle
    /// ("quest_completed", "standing_honored", "structure_discovered").
    #[serde(default)]
    triggers: Vec<String>,
}

#[derive(Default, Deserialize)]
struct WorldEventFile {
    #[serde(default)]
    world_event: Vec<WorldEventRaw>,
}

// ---------------------------------------------------------------------------
// NPC roster
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NpcKind {
    Villager,
    Companion,
}

/// One NPC archetype from lore/npcs.toml. Villager entries spawn at faction
/// structures and may be hireable (becoming their `companion_form`);
/// companion entries carry wages, skills, and combat stats.
#[derive(Clone, Debug, Deserialize)]
pub struct NpcArchetype {
    pub id: String,
    pub kind: NpcKind,
    #[serde(default)]
    pub faction: Option<String>,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub name_pool: Vec<String>,
    #[serde(default)]
    pub structure: Option<String>,
    #[serde(default = "default_count")]
    pub spawn_min: u8,
    #[serde(default = "default_count")]
    pub spawn_max: u8,
    #[serde(default)]
    pub hireable: bool,
    #[serde(default)]
    pub companion_form: Option<String>,
    #[serde(default = "default_hire_standing")]
    pub hire_standing: i32,
    #[serde(default)]
    pub hire_fee: Vec<(String, u8)>,
    #[serde(default)]
    pub opening_hire_dialogue: String,
    #[serde(default)]
    pub named: bool,
    // companion-form fields
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub health: f32,
    #[serde(default)]
    pub damage: f32,
    #[serde(default = "default_speed")]
    pub speed: f32,
    #[serde(default)]
    pub starter_tools: Vec<(String, u8)>,
    #[serde(default)]
    pub daily_wage: Vec<(String, u8)>,
}

fn default_count() -> u8 {
    1
}
fn default_hire_standing() -> i32 {
    75
}
fn default_speed() -> f32 {
    3.0
}

#[derive(Default, Deserialize)]
struct NpcFile {
    #[serde(default)]
    npc: Vec<NpcArchetype>,
}

// ---------------------------------------------------------------------------
// Dialogue
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DialogueAction {
    Close,
    OpenMenu,
    OpenMenuPlus,
}

impl DialogueAction {
    fn from_name(s: &str) -> Self {
        match s {
            "close" => DialogueAction::Close,
            "open_menu_plus" => DialogueAction::OpenMenuPlus,
            _ => DialogueAction::OpenMenu,
        }
    }
}

fn default_action() -> String {
    "open_menu".into()
}

#[derive(Clone, Debug, Deserialize)]
pub struct DialogueNode {
    pub npc_archetype: String,
    pub condition: String,
    pub text: String,
    #[serde(rename = "action", default = "default_action")]
    raw_action: String,
}

impl DialogueNode {
    pub fn action(&self) -> DialogueAction {
        DialogueAction::from_name(&self.raw_action)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompanionLine {
    /// "any" applies to every companion.
    pub archetype: String,
    pub condition: String,
    pub text: String,
}

#[derive(Default, Deserialize)]
struct DialogueFile {
    #[serde(default)]
    dialogue_node: Vec<DialogueNode>,
    #[serde(default)]
    companion_line: Vec<CompanionLine>,
}

/// Context the condition evaluator sees. Everything optional — a condition
/// referencing missing context evaluates false.
#[derive(Default, Clone, Copy)]
pub struct ConditionCtx<'a> {
    pub standings: Option<&'a StandingState>,
    pub biome: Option<&'a str>,
    pub morale: Option<i32>,
    pub trust: Option<i32>,
    pub structure_discovered: Option<&'a str>,
    pub lore_book_found: bool,
    pub near_machine: bool,
}

fn eval_comparison(lhs: &str, op: &str, rhs: &str, ctx: &ConditionCtx) -> bool {
    let lhs_trim = lhs.trim();
    let rhs_trim = rhs.trim();
    if let Some(stripped) = lhs_trim.strip_prefix("standing_") {
        let standings = match ctx.standings {
            Some(s) => s,
            None => return false,
        };
        let value = standings.get(stripped);
        let Ok(target) = rhs_trim.parse::<i32>() else { return false };
        return compare_i32(Some(value), op, target);
    }
    match lhs_trim {
        "morale" | "trust" => {
            let value = if lhs_trim == "morale" { ctx.morale } else { ctx.trust };
            let Some(value) = value else { return false };
            let Ok(target) = rhs_trim.parse::<i32>() else { return false };
            compare_i32(Some(value), op, target)
        }        "biome" => ctx.biome == Some(rhs_trim) && op == "=",
        "structure_discovered" => {
            ctx.structure_discovered == Some(rhs_trim) && op == "="
        }
        _ => false,
    }
}

fn compare_i32(value: Option<i32>, op: &str, target: i32) -> bool {
    let v = match value {
        Some(v) => v,
        None => return false,
    };
    match op {
        "<" => v < target,
        "<=" => v <= target,
        ">" => v > target,
        ">=" => v >= target,
        "=" => v == target,
        _ => false,
    }
}

/// Evaluate a dialogue condition: comparisons joined by "and", plus the
/// bare flags `lore_book_found` and `near_machine`.
pub fn eval_condition(condition: &str, ctx: &ConditionCtx) -> bool {
    condition.split(" and ").all(|clause| {
        let clause = clause.trim();
        if clause == "lore_book_found" {
            return ctx.lore_book_found;
        }
        if clause == "near_machine" {
            return ctx.near_machine;
        }
        for op in ["<=", ">=", "<", ">", "="] {
            if let Some((lhs, rhs)) = clause.split_once(op) {
                return eval_comparison(lhs, op, rhs, ctx);
            }
        }
        false
    })
}

// ---------------------------------------------------------------------------
// Faction quests (parsed into lf_story quests)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ObjectiveDef {
    #[serde(rename = "type")]
    kind: String,
    target: String,
    count: u32,
}

#[derive(Deserialize)]
struct QuestDef {
    id: String,
    title: String,
    description: String,
    act: u8,
    #[serde(default)]
    faction: Option<String>,
    #[serde(default)]
    standing_reward: i32,
    #[serde(default)]
    other_standing: Option<BTreeMap<String, i32>>,
    objectives: Vec<ObjectiveDef>,
}

#[derive(Default, Deserialize)]
struct QuestFile {
    #[serde(default)]
    quest: Vec<QuestDef>,
}

impl QuestDef {
    fn parse(self) -> Option<lf_story::Quest> {
        let mut objectives = Vec::new();
        for o in self.objectives {
            objectives.push(lf_story::QuestObjective {
                objective_type: lf_story::QuestType::from_name(&o.kind)?,
                target: o.target,
                count: o.count,
                progress: 0,
                completed: false,
            });
        }
        Some(lf_story::Quest {
            id: self.id,
            title: self.title,
            description: self.description,
            objectives,
            act: self.act,
            completed: false,
            faction: self.faction,
            standing_reward: self.standing_reward,
            other_standing: self
                .other_standing
                .map(|m| m.into_iter().collect())
                .unwrap_or_default(),
        })
    }
}

// ---------------------------------------------------------------------------
// The registry
// ---------------------------------------------------------------------------

/// Everything the lore layer knows, loaded from the `lore/` directory.
#[derive(Default, Clone)]
pub struct LoreRegistry {
    pub factions: Vec<FactionDef>,
    pub relationships: Vec<Relationship>,
    pub standing_events: StandingEvents,
    pub world_events: Vec<WorldEvent>,
    pub npcs: Vec<NpcArchetype>,
    pub dialogue_nodes: Vec<DialogueNode>,
    pub companion_lines: Vec<CompanionLine>,
    pub faction_quests: Vec<lf_story::Quest>,
    /// Biome -> owning faction id (first faction in file order wins).
    territory: HashMap<Biome, String>,
}

impl LoreRegistry {
    pub fn faction(&self, id: &str) -> Option<&FactionDef> {
        self.factions.iter().find(|f| f.id == id)
    }

    /// The faction controlling a biome's territory (map tint, HUD).
    pub fn territory_owner(&self, biome: Biome) -> Option<&FactionDef> {
        let id = self.territory.get(&biome)?;
        self.faction(id)
    }

    /// Factions that consider `faction` a rival (drift on honored
    /// crossing). The matrix is symmetric; only the upper triangle is
    /// encoded in the data file.
    pub fn rivals_of(&self, faction: &str) -> Vec<String> {
        self.relationships
            .iter()
            .filter(|r| r.is_rival() && (r.from == faction || r.to == faction))
            .map(|r| if r.from == faction { r.to.clone() } else { r.from.clone() })
            .collect()
    }

    pub fn villager_archetype(&self, id: &str) -> Option<&NpcArchetype> {
        self.npcs
            .iter()
            .find(|n| n.id == id && n.kind == NpcKind::Villager)
    }

    pub fn companion_archetype(&self, id: &str) -> Option<&NpcArchetype> {
        self.npcs
            .iter()
            .find(|n| n.id == id && n.kind == NpcKind::Companion)
    }

    /// Villager archetypes that settle at a given structure kind.
    pub fn archetypes_for_structure(&self, structure: &str) -> Vec<&NpcArchetype> {
        self.npcs
            .iter()
            .filter(|n| n.kind == NpcKind::Villager && n.structure.as_deref() == Some(structure))
            .collect()
    }

    /// First dialogue node whose archetype matches and condition holds
    /// (file order = priority; list the most hostile bands first).
    pub fn dialogue_for(
        &self,
        archetype: &str,
        ctx: &ConditionCtx,
    ) -> Option<&DialogueNode> {
        self.dialogue_nodes
            .iter()
            .find(|n| n.npc_archetype == archetype && eval_condition(&n.condition, ctx))
    }

    /// First companion line matching the archetype (or "any") and condition.
    pub fn companion_line_for(
        &self,
        archetype: &str,
        ctx: &ConditionCtx,
    ) -> Option<&CompanionLine> {
        self.companion_lines.iter().find(|l| {
            (l.archetype == archetype || l.archetype == "any")
                && eval_condition(&l.condition, ctx)
        })
    }

    /// Canonical world events that can be referenced for a happening.
    pub fn world_events_for(
        &self,
        trigger: WorldEventTrigger,
        faction: &str,
    ) -> Vec<&WorldEvent> {
        self.world_events
            .iter()
            .filter(|e| e.triggers.contains(&trigger) && e.factions.iter().any(|f| f == faction))
            .collect()
    }

    /// Load every lore data file from `dir`; missing files contribute
    /// nothing (the game must still run headless/test without them).
    pub fn load(dir: &Path) -> Self {
        let mut reg = LoreRegistry::default();
        if let Ok(text) = std::fs::read_to_string(dir.join("factions.toml")) {
            if let Ok(file) = toml::from_str::<FactionFile>(&text) {
                reg.factions = file.faction.into_iter().filter_map(|f| f.parse()).collect();
                reg.relationships = file.relationship;
                reg.standing_events = file.standing_events.unwrap_or(StandingEvents {
                    quest_complete: 15,
                    quest_fail: -10,
                    trade_ten_items: 2,
                    attack_npc: -20,
                    kill_npc: -35,
                    destroy_structure_block: -5,
                    build_faction_blocks: 3,
                    discover_structure: 5,
                    rival_honored: -10,
                });
            }
        }
        if let Ok(text) = std::fs::read_to_string(dir.join("world_events.toml")) {
            if let Ok(file) = toml::from_str::<WorldEventFile>(&text) {
                reg.world_events = file
                    .world_event
                    .into_iter()
                    .filter_map(|e| {
                        Some(WorldEvent {
                            id: e.id,
                            era: e.era,
                            year: e.year,
                            name: e.name,
                            description: e.description,
                            factions: e.factions,
                            triggers: e
                                .triggers
                                .iter()
                                .filter_map(|n| WorldEventTrigger::from_name(n))
                                .collect(),
                        })
                    })
                    .collect();
            }
        }
        if let Ok(text) = std::fs::read_to_string(dir.join("npcs.toml")) {
            if let Ok(file) = toml::from_str::<NpcFile>(&text) {
                reg.npcs = file.npc;
            }
        }
        if let Ok(text) = std::fs::read_to_string(dir.join("dialogue.toml")) {
            if let Ok(file) = toml::from_str::<DialogueFile>(&text) {
                reg.dialogue_nodes = file.dialogue_node;
                reg.companion_lines = file.companion_line;
            }
        }
        if let Ok(text) = std::fs::read_to_string(dir.join("quests_factions.toml")) {
            if let Ok(file) = toml::from_str::<QuestFile>(&text) {
                reg.faction_quests = file.quest.into_iter().filter_map(|q| q.parse()).collect();
            }
        }
        // Territory: first faction in file order claims each biome.
        for faction in &reg.factions {
            for biome in &faction.home_biomes {
                reg.territory.entry(*biome).or_insert_with(|| faction.id.clone());
            }
        }
        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real() -> LoreRegistry {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join("lore");
        LoreRegistry::load(&dir)
    }

    /// A1 verify: all six factions load with ids, alignments, and non-empty
    /// home biome lists, and standing starts at the documented values.
    #[test]
    fn six_factions_load_with_canonical_data() {
        let reg = real();
        let ids: Vec<&str> = reg.factions.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["accord", "ironborn", "ember_covenant", "free_holds", "ashen_order", "nameless"],
            "faction ids/order"
        );
        for f in &reg.factions {
            assert!(!f.full_name.is_empty() && !f.short_name.is_empty(), "{} names", f.id);
            assert!(!f.home_biomes.is_empty(), "{} has no home biomes", f.id);
            assert_eq!(f.color.len(), 3);
            assert!(!f.symbol.is_empty());
        }
        let nameless = reg.faction("nameless").unwrap();
        assert_eq!(nameless.alignment, Alignment::Hostile);
        assert_eq!(nameless.starting_standing, -50);
        assert_eq!(nameless.ideology, Ideology::Outlaw);
        let accord = reg.faction("accord").unwrap();
        assert_eq!(accord.alignment, Alignment::Lawful);
        assert_eq!(accord.color, [74, 122, 181]);
        assert!(accord.home_biomes.contains(&Biome::Meadow));
        let ironborn = reg.faction("ironborn").unwrap();
        assert_eq!(ironborn.alignment, Alignment::Lawful);
        assert!(ironborn.home_biomes.contains(&Biome::Mountains));
        let covenant = reg.faction("ember_covenant").unwrap();
        assert_eq!(covenant.alignment, Alignment::Neutral);
        let free_holds = reg.faction("free_holds").unwrap();
        assert_eq!(free_holds.alignment, Alignment::Neutral);
        let ashen = reg.faction("ashen_order").unwrap();
        assert_eq!(ashen.alignment, Alignment::Neutral);
    }

    #[test]
    fn standing_starts_correct_in_fresh_state() {
        let reg = real();
        let standing = StandingState::starting(&reg);
        assert_eq!(standing.get("accord"), 0);
        assert_eq!(standing.get("ironborn"), 0);
        assert_eq!(standing.get("ember_covenant"), 0);
        assert_eq!(standing.get("free_holds"), 0);
        assert_eq!(standing.get("ashen_order"), 0);
        assert_eq!(standing.get("nameless"), -50, "The Nameless start hostile");
    }

    #[test]
    fn standing_adds_clamp_and_report_threshold_crossings() {
        let mut s = StandingState::default();
        let c = s.add("ironborn", 15);
        assert_eq!((c.old, c.new), (0, 15));
        assert!(c.band.is_none());
        for _ in 0..3 {
            s.add("ironborn", 15);
        }
        // 60 -> 75 crosses the honored threshold
        let c = s.add("ironborn", 15);
        assert_eq!(c.new, 75);
        assert_eq!(c.band, Some(StandingBand::Honored));
        // clamp at +100
        let c = s.add("ironborn", 50);
        assert_eq!(c.new, 100);
        // drop across the hostile threshold
        let mut n = StandingState::starting(&real());
        let c = n.add("nameless", -25);
        assert_eq!(c.new, -75);
        assert_eq!(c.band, Some(StandingBand::Enemy));
    }

    /// The FACTIONS_OVERVIEW tension rule: honoring one faction drifts its
    /// rivals -10 (becoming an Accord champion angers the Nameless and
    /// nudges the Free Holds and Covenant colder).
    #[test]
    fn rival_drift_follows_the_relationship_matrix() {
        let reg = real();
        let rivals = reg.rivals_of("accord");
        assert!(rivals.contains(&"nameless".to_string()));
        assert!(rivals.contains(&"free_holds".to_string()));
        assert!(rivals.contains(&"ember_covenant".to_string()));
        assert!(!rivals.contains(&"ironborn".to_string()), "allied, not rival");
        assert!(reg.rivals_of("ironborn").contains(&"nameless".to_string()));
    }

    /// Territory: home biomes map to their faction; unlisted biomes are
    /// unclaimed (no tint).
    #[test]
    fn territory_owns_home_biomes_only() {
        let reg = real();
        assert_eq!(reg.territory_owner(Biome::Meadow).unwrap().id, "accord");
        assert_eq!(reg.territory_owner(Biome::Mountains).unwrap().id, "ironborn");
        assert_eq!(reg.territory_owner(Biome::MushroomHollow).unwrap().id, "ember_covenant");
        assert_eq!(reg.territory_owner(Biome::Savanna).unwrap().id, "free_holds");
        assert!(reg.territory_owner(Biome::Desert).is_none());
        assert!(reg.territory_owner(Biome::Ocean).is_none());
    }

    /// World events carry their canonical era/year dates.
    #[test]
    fn world_events_are_canonical_history() {
        let reg = real();
        assert!(reg.world_events.len() >= 13, "expected the full timeline, got {}", reg.world_events.len());
        let smelter = reg.world_events.iter().find(|e| e.id == "great_smelter").unwrap();
        assert_eq!((smelter.era, smelter.year), (1, 214));
        assert_eq!(smelter.date(), "Era I, Year 214");
        assert!(smelter.factions.contains(&"ironborn".to_string()));
        let sundering = reg.world_events.iter().find(|e| e.id == "the_sundering").unwrap();
        assert_eq!((sundering.era, sundering.year), (2, 1));
        // quest-completed events exist for every faction
        for f in ["accord", "ironborn", "ember_covenant", "free_holds", "ashen_order", "nameless"] {
            assert!(
                !reg.world_events_for(WorldEventTrigger::QuestCompleted, f).is_empty(),
                "no quest-completed world event for {f}"
            );
        }
    }

    /// NPC roster: hireable archetypes with companion forms, wages, and the
    /// three named NPCs.
    #[test]
    fn npc_roster_loads() {
        let reg = real();
        for id in ["accord_herald", "ironborn_artisan", "covenant_herbalist",
                   "freeholds_elder", "freeholds_scout", "ashen_archivist", "nameless_drifter"] {
            let arch = reg.villager_archetype(id).unwrap_or_else(|| panic!("{id} missing"));
            assert!(!arch.name_pool.is_empty(), "{id} needs names");
        }
        for id in ["the_unmarked", "maren_voss", "dag_holtz"] {
            assert!(reg.villager_archetype(id).unwrap().named, "{id} must be the named one");
        }
        // every hireable villager resolves to a companion form with a wage
        for n in &reg.npcs {
            if n.kind == NpcKind::Villager && n.hireable {
                let form = n.companion_form.clone().unwrap_or_else(|| panic!("{} hireable but no form", n.id));
                let comp = reg.companion_archetype(&form).unwrap_or_else(|| panic!("form {form} missing"));
                assert!(!comp.daily_wage.is_empty(), "{form} has no wage");
                assert!(comp.health > 0.0 && comp.damage > 0.0);
            }
        }
        let warden = reg.companion_archetype("accord_warden").unwrap();
        assert_eq!(warden.daily_wage, vec![("iron_ingot".to_string(), 8)]);
    }

    /// Dialogue conditions evaluate: standing bands pick the right node,
    /// hostile action closes, honored opens the plus-menu.
    #[test]
    fn dialogue_nodes_select_by_standing() {
        let reg = real();
        let mut s = StandingState::default();
        let ctx = ConditionCtx { standings: Some(&s), ..Default::default() };
        let node = reg.dialogue_for("accord_herald", &ctx).unwrap();
        assert!(node.text.contains("What can the Accord do for you"));
        assert_eq!(node.action(), DialogueAction::OpenMenu);

        s.add("accord", -50);
        let hostile = reg.dialogue_for("accord_herald", &ConditionCtx { standings: Some(&s), ..Default::default() }).unwrap();
        assert_eq!(hostile.action(), DialogueAction::Close);

        let mut champ = StandingState::default();
        champ.add("accord", 100);
        let honored = reg.dialogue_for("accord_herald", &ConditionCtx { standings: Some(&champ), ..Default::default() }).unwrap();
        assert!(honored.text.contains("Champion"));
        assert_eq!(honored.action(), DialogueAction::OpenMenuPlus);
    }

    /// Companion contextual lines: biome/morale/trust conditions and the
    /// "any" fallback archetype.
    #[test]
    fn companion_lines_select_by_context() {
        let reg = real();
        let s = StandingState::default();
        let line = reg.companion_line_for(
            "accord_warden",
            &ConditionCtx { standings: Some(&s), biome: Some("Volcanic"), ..Default::default() },
        );
        assert!(line.unwrap().text.contains("Ironborn"));

        let low = reg.companion_line_for(
            "covenant_channeler",
            &ConditionCtx { standings: Some(&s), morale: Some(20), ..Default::default() },
        );
        assert!(low.unwrap().text.contains("rest"));

        let quit = reg.companion_line_for(
            "ironborn_artisan",
            &ConditionCtx { standings: Some(&s), morale: Some(0), ..Default::default() },
        );
        assert!(quit.unwrap().text.contains("I've had enough"));
    }

    #[test]
    fn condition_grammar() {
        let mut s = StandingState::default();
        s.add("ironborn", 40);
        let ctx = ConditionCtx { standings: Some(&s), biome: Some("Swamp"), morale: Some(55), ..Default::default() };
        assert!(eval_condition("standing_ironborn >= 40", &ctx));
        assert!(eval_condition("standing_ironborn >= -30 and standing_ironborn < 75", &ctx));
        assert!(!eval_condition("standing_ironborn >= 75", &ctx));
        assert!(eval_condition("biome = Swamp", &ctx));
        assert!(!eval_condition("biome = Volcanic", &ctx));
        assert!(eval_condition("morale < 60", &ctx));
        assert!(!eval_condition("morale < 30", &ctx));
        assert!(!eval_condition("standing_ironborn >= -30 and morale < 30", &ctx));
        assert!(!eval_condition("standing_unknown_thing > 3", &ctx));
        let flags = ConditionCtx { lore_book_found: true, near_machine: true, ..Default::default() };
        assert!(eval_condition("lore_book_found", &flags));
        assert!(eval_condition("near_machine", &flags));
        assert!(!eval_condition("lore_book_found", &ConditionCtx::default()));
    }

    /// A4 verify (part 1): all 12 faction quests load, parse, and fire
    /// their correct objective types.
    #[test]
    fn twelve_faction_quests_load_and_fire() {
        let reg = real();
        assert_eq!(reg.faction_quests.len(), 12, "2 per faction x 6");
        for faction in ["accord", "ironborn", "ember_covenant", "free_holds", "ashen_order", "nameless"] {
            let n = reg.faction_quests.iter().filter(|q| q.faction.as_deref() == Some(faction)).count();
            assert_eq!(n, 2, "{faction} needs exactly 2 quests");
        }
        // every quest is playable end-to-end: feeding its own events completes it
        let events_for = |q: &lf_story::Quest| -> Vec<lf_story::QuestEvent> {
            let mut ev = Vec::new();
            for _ in 0..32 {
                for o in &q.objectives {
                    let e = match o.objective_type {
                        lf_story::QuestType::Collect => lf_story::QuestEvent::Collected(o.target.clone()),
                        lf_story::QuestType::Craft => lf_story::QuestEvent::Crafted(o.target.clone()),
                        lf_story::QuestType::Kill => lf_story::QuestEvent::Killed(o.target.clone()),
                        lf_story::QuestType::Reach => lf_story::QuestEvent::Reached(o.target.clone()),
                        lf_story::QuestType::Interact => lf_story::QuestEvent::Interacted(o.target.clone()),
                        lf_story::QuestType::Break => lf_story::QuestEvent::Broke(o.target.clone()),
                        lf_story::QuestType::Place => lf_story::QuestEvent::Placed(o.target.clone()),
                        _ => lf_story::QuestEvent::Reached(o.target.clone()),
                    };
                    ev.push(e);
                }
            }
            ev
        };
        for quest in &reg.faction_quests {
            assert!(quest.standing_reward > 0, "{} has no standing reward", quest.id);
            let mut log = lf_story::QuestLog::new();
            log.add_quest(quest.clone());
            for e in events_for(quest) {
                log.record_event(&e);
            }
            assert!(log.completed_quests.contains(&quest.id), "{} never completes", quest.id);
        }
        // the canon examples survive verbatim enough to recognize
        let unmarked = reg.faction_quests.iter().find(|q| q.id == "nameless_q2_the_philosophy").unwrap();
        assert!(unmarked.description.contains("The Unmarked"));
        let road = reg.faction_quests.iter().find(|q| q.id == "accord_q1_road_survey").unwrap();
        assert!(road.description.contains("Ashenmoor"));
    }
}
