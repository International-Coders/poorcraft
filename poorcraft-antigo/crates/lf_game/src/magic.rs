//! Magic foundation (V1REBRAND doc 05 / P33): mana, the bounded four-spell
//! set, and the spellbook. Pure data + gating so the client applies the
//! effects and tests run headless. The set is bounded by design — new
//! spells are a DECISIONS decision, not a content dump.

use serde::{Deserialize, Serialize};

/// Mana pool size and regeneration.
pub const MAX_MANA: f32 = 30.0;
/// Mana per second regenerated passively.
pub const MANA_REGEN: f32 = 1.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Spell {
    Firebolt,
    GaleStep,
    Ward,
    Hearthlight,
}

impl Spell {
    pub const ALL: [Spell; 4] = [Spell::Firebolt, Spell::GaleStep, Spell::Ward, Spell::Hearthlight];

    pub fn name(self) -> &'static str {
        match self {
            Spell::Firebolt => "Firebolt",
            Spell::GaleStep => "Gale-step",
            Spell::Ward => "Ward",
            Spell::Hearthlight => "Hearthlight",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Spell::Firebolt => "hurl a bolt of fire — an arrow that hits harder",
            Spell::GaleStep => "blink forward along your gaze",
            Spell::Ward => "a shield that drinks damage for a few seconds",
            Spell::Hearthlight => "the Smith's trick: light the dark, soften one ore by hand",
        }
    }

    pub fn cost(self) -> f32 {
        match self {
            Spell::Firebolt => 8.0,
            Spell::GaleStep => 12.0,
            Spell::Ward => 20.0,
            Spell::Hearthlight => 15.0,
        }
    }

    /// The scroll item id a wizard sells that teaches this spell.
    pub fn scroll_id(self) -> &'static str {
        match self {
            Spell::Firebolt => "scroll_of_firebolt",
            Spell::GaleStep => "scroll_of_gale_step",
            Spell::Ward => "scroll_of_ward",
            Spell::Hearthlight => "scroll_of_hearthlight",
        }
    }

    pub fn from_scroll(item_id: &str) -> Option<Spell> {
        Spell::ALL.iter().copied().find(|s| s.scroll_id() == item_id)
    }
}

/// What casting produces; the client applies each variant.
#[derive(Clone, Debug, PartialEq)]
pub enum SpellEffect {
    Firebolt,
    /// Blink this many blocks along the look direction.
    Blink { forward: f32 },
    /// Absorb damage for this many seconds.
    Ward { secs: f32 },
    /// Light + hand-smelt one ore; the caller picks the ore with
    /// [`hearthlight_pick`].
    Hearthlight,
}

/// Why a cast was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastError {
    NotLearned,
    NotEnoughMana,
}

/// The player's learned spells + the three cast slots. Slot contents are
/// cast with the spell keys; `learned` is what the book shows.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spellbook {
    #[serde(default)]
    pub learned: Vec<Spell>,
    #[serde(default)]
    pub slots: [Option<Spell>; 3],
}

impl Default for Spellbook {
    fn default() -> Self {
        Self { learned: Vec::new(), slots: [None, None, None] }
    }
}

impl Spellbook {
    pub fn knows(&self, spell: Spell) -> bool {
        self.learned.contains(&spell)
    }

    pub fn learn(&mut self, spell: Spell) -> bool {
        if self.knows(spell) {
            return false;
        }
        self.learned.push(spell);
        // auto-assign to the first empty slot so a fresh spell is castable
        if let Some(slot) = self.slots.iter_mut().find(|s| s.is_none()) {
            *slot = Some(spell);
        }
        true
    }

    pub fn assign(&mut self, slot: usize, spell: Option<Spell>) {
        if let Some(s) = self.slots.get_mut(slot.min(2)) {
            *s = spell;
        }
    }

    /// Gate + cost check. Returns the effect to apply and the mana left.
    pub fn try_cast(
        &self,
        slot: usize,
        mana: f32,
    ) -> Result<(SpellEffect, f32), CastError> {
        let spell = self.slots.get(slot.min(2)).copied().flatten().ok_or(CastError::NotLearned)?;
        if !self.knows(spell) {
            return Err(CastError::NotLearned);
        }
        if mana < spell.cost() {
            return Err(CastError::NotEnoughMana);
        }
        let left = mana - spell.cost();
        let effect = match spell {
            Spell::Firebolt => SpellEffect::Firebolt,
            Spell::GaleStep => SpellEffect::Blink { forward: 8.0 },
            Spell::Ward => SpellEffect::Ward { secs: 5.0 },
            Spell::Hearthlight => SpellEffect::Hearthlight,
        };
        Ok((effect, left))
    }
}

/// The first smeltable item id Hearthlight should soften (caller feeds the
/// inventory's distinct item ids, in slot order).
pub fn hearthlight_pick(item_ids: &[&str]) -> Option<(String, String)> {
    for id in item_ids {
        if let Some(out) = crate::smelting::smelt_result(id) {
            // ores only — the Smith softens ore, not sand
            if id.starts_with("raw_") {
                return Some((id.to_string(), out.to_string()));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spellbook_learning_and_slots() {
        let mut book = Spellbook::default();
        assert!(!book.knows(Spell::Firebolt));
        assert!(book.learn(Spell::Firebolt));
        assert!(!book.learn(Spell::Firebolt), "no double learning");
        assert_eq!(book.slots[0], Some(Spell::Firebolt), "auto-assigned");
        book.learn(Spell::Ward);
        book.learn(Spell::GaleStep);
        book.learn(Spell::Hearthlight);
        // 4 learned, 3 slots: the last one waits for a free slot
        assert_eq!(book.slots, [Some(Spell::Firebolt), Some(Spell::Ward), Some(Spell::GaleStep)]);
        book.assign(2, Some(Spell::Hearthlight));
        assert_eq!(book.slots[2], Some(Spell::Hearthlight));
    }

    #[test]
    fn casting_costs_mana_and_gates_on_learning() {
        let mut book = Spellbook::default();
        book.learn(Spell::Firebolt);
        // not learned (empty slot 1)
        assert_eq!(book.try_cast(1, MAX_MANA), Err(CastError::NotLearned));
        // not enough mana
        assert_eq!(book.try_cast(0, 3.0), Err(CastError::NotEnoughMana));
        // good cast
        let (effect, left) = book.try_cast(0, 10.0).unwrap();
        assert_eq!(effect, SpellEffect::Firebolt);
        assert!((left - 2.0).abs() < 1e-4);
        // hearthlight signals the effect; the ore pick is separate
        let mut hb = Spellbook::default();
        hb.learn(Spell::Hearthlight);
        let (effect, _) = hb.try_cast(0, MAX_MANA).unwrap();
        assert_eq!(effect, SpellEffect::Hearthlight);
        assert_eq!(
            hearthlight_pick(&["stone", "raw_copper", "raw_iron"]),
            Some(("raw_copper".into(), "copper_ingot".into()))
        );
        assert_eq!(hearthlight_pick(&["stone", "sand"]), None, "ores only");
    }

    #[test]
    fn bounded_set_and_costs_are_stable() {
        assert_eq!(Spell::ALL.len(), 4, "the spell set is bounded by design (doc 05)");
        assert!(Spell::Firebolt.cost() < Spell::GaleStep.cost());
        assert!(Spell::GaleStep.cost() < Spell::Hearthlight.cost());
        assert!(Spell::Hearthlight.cost() < Spell::Ward.cost());
        assert!(Spell::Ward.cost() <= MAX_MANA, "every spell is castable on a full pool");
        assert_eq!(Spell::from_scroll("scroll_of_ward"), Some(Spell::Ward));
        assert_eq!(Spell::from_scroll("book"), None);
    }

    /// The imbue minigame mirrors the forge: band-gated pulses, then a
    /// fresh-workpiece reset.
    #[test]
    fn imbue_minigame_binds_in_the_band() {
        let mut g = ImbueMinigame::new(3);
        assert!(!g.pulse(), "50 is outside the 55..75 band");
        g.focus(20.0);
        assert!(!g.pulse(), "70 binds (1/3)");
        g.focus(30.0);
        assert!(!g.pulse(), "100 is outside the band");
        g.focus(-40.0);
        assert!(!g.pulse(), "60 binds (2/3)");
        g.focus(-2.0);
        assert!(g.pulse(), "58 binds (3/3) — rune ready");
        assert!(g.ready());
        g.reset();
        assert!(!g.ready() && g.attunement == 50.0);
    }

    /// Runes fill the pre-cut CustomTool.rune slot (doc 05) and carry
    /// their gameplay multipliers.
    #[test]
    fn runes_fill_the_custom_tool_slot() {
        use crate::smithing::{CustomTool, ToolMaterial};
        let mut tool = CustomTool::assemble("Mythril Pickaxe", ToolMaterial::Mythril, ToolMaterial::Iron, ToolMaterial::Bronze);
        assert!(tool.rune.is_none(), "the pre-cut slot starts empty");
        tool.rune = Some(Rune::Haste.item_id().to_string());
        assert_eq!(tool.rune.as_deref(), Some("rune_of_haste"));
        assert_eq!(Rune::from_item("rune_of_haste"), Some(Rune::Haste));
        assert_eq!(Rune::from_item("book"), None);
        assert!((Rune::Haste.mining_multiplier() - 1.3).abs() < 1e-4);
        assert_eq!(Rune::Warding.armor_bonus(), 2.0);
    }

    /// Old saves (pre-magic) deserialize with an empty book.
    #[test]
    fn pre_magic_spellbook_loads() {
        let bytes = bincode::serialize(&Spellbook::default()).unwrap();
        let book: Spellbook = bincode::deserialize(&bytes).unwrap();
        assert!(book.learned.is_empty());
        assert_eq!(book.slots, [None, None, None]);
    }
}

/// The imbue minigame (P33, mirrors ForgeMinigame): channel the table's
/// attunement into the 55..75 band, then pulse. Three clean pulses bind a
/// rune to a tool.
pub struct ImbueMinigame {
    pub attunement: f32,
    pub pulses: u32,
    pub target_pulses: u32,
}

impl ImbueMinigame {
    pub fn new(target_pulses: u32) -> Self {
        Self { attunement: 50.0, pulses: 0, target_pulses }
    }

    pub fn focus(&mut self, amount: f32) {
        self.attunement = (self.attunement + amount).clamp(0.0, 100.0);
    }

    /// One pulse: binds only inside the attunement band.
    pub fn pulse(&mut self) -> bool {
        if (55.0..=75.0).contains(&self.attunement) {
            self.pulses += 1;
        }
        self.pulses >= self.target_pulses
    }

    pub fn ready(&self) -> bool {
        self.pulses >= self.target_pulses
    }

    /// Fresh rune: the UI grants the result once and calls this (the same
    /// per-frame-mint guard ForgeMinigame.reset exists for).
    pub fn reset(&mut self) {
        self.attunement = 50.0;
        self.pulses = 0;
    }
}

/// Runes (P33): what a bound rune does to the tool it lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rune {
    Haste,
    Warding,
}

impl Rune {
    pub fn name(self) -> &'static str {
        match self {
            Rune::Haste => "Rune of Haste",
            Rune::Warding => "Rune of Warding",
        }
    }

    pub fn item_id(self) -> &'static str {
        match self {
            Rune::Haste => "rune_of_haste",
            Rune::Warding => "rune_of_warding",
        }
    }

    pub fn from_item(item_id: &str) -> Option<Rune> {
        Rune::ALL.iter().copied().find(|r| r.item_id() == item_id)
    }

    pub const ALL: [Rune; 2] = [Rune::Haste, Rune::Warding];

    /// The gameplay effect while the runed tool is held.
    pub fn mining_multiplier(self) -> f32 {
        match self {
            Rune::Haste => 1.3,
            Rune::Warding => 1.0,
        }
    }

    pub fn armor_bonus(self) -> f32 {
        match self {
            Rune::Haste => 0.0,
            Rune::Warding => 2.0,
        }
    }
}
