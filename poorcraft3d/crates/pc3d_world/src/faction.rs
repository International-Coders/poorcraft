//! P3D-605: faction relations — trust, diplomacy, quests, territory.
//!
//! Factions have trust levels toward each other (Allied → Hostile),
//! diplomacy actions shift trust, quests connect factions to the player,
//! and territory tracks which faction controls which regions.

use std::collections::BTreeMap;

/// A faction identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FactionId(pub u64);

/// Trust level between factions (ordinal, higher = better).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustLevel {
    Hostile,
    Wary,
    Neutral,
    Friendly,
    Allied,
}

impl TrustLevel {
    pub fn from_score(score: i32) -> Self {
        match score {
            s if s >= 80 => TrustLevel::Allied,
            s if s >= 60 => TrustLevel::Friendly,
            s if s >= 30 => TrustLevel::Neutral,
            s if s >= 10 => TrustLevel::Wary,
            _ => TrustLevel::Hostile,
        }
    }

    /// Can factions at this trust level trade?
    pub fn can_trade(self) -> bool {
        self >= TrustLevel::Neutral
    }

    /// Are factions at this trust level allied?
    pub fn is_allied(self) -> bool {
        self == TrustLevel::Allied
    }

    /// Are factions at this trust level at war?
    pub fn is_hostile(self) -> bool {
        self == TrustLevel::Hostile
    }
}

/// A diplomacy action that shifts trust.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiplomacyAction {
    Alliance,
    TradeAgreement,
    Insult,
    BorderSkirmish,
    DeclareWar,
}

impl DiplomacyAction {
    /// Trust delta from this action.
    pub fn trust_delta(self) -> i32 {
        match self {
            DiplomacyAction::Alliance => 30,
            DiplomacyAction::TradeAgreement => 15,
            DiplomacyAction::Insult => -10,
            DiplomacyAction::BorderSkirmish => -15,
            DiplomacyAction::DeclareWar => -100,
        }
    }
}

/// A quest offered by a faction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quest {
    pub id: u64,
    pub faction: FactionId,
    pub target_cell: crate::coords::CellCoord,
    pub reward_reputation: i32,
    pub accepted: bool,
    pub completed: bool,
}

/// Territory claims per faction.
pub type Territory = BTreeMap<FactionId, Vec<(i32, i32)>>;

/// The faction relations system.
#[derive(Clone, Debug, Default)]
pub struct FactionRelations {
    /// Trust scores between faction pairs, keyed by (min_id, max_id).
    trust: BTreeMap<(u64, u64), i32>,
    /// Active quests.
    pub quests: BTreeMap<u64, Quest>,
    next_quest_id: u64,
    /// Territory claims.
    pub territory: Territory,
}

impl FactionRelations {
    pub fn new() -> Self {
        FactionRelations::default()
    }

    fn key(a: FactionId, b: FactionId) -> (u64, u64) {
        if a.0 <= b.0 {
            (a.0, b.0)
        } else {
            (b.0, a.0)
        }
    }

    /// Get the trust score between two factions (default 50 = Neutral).
    pub fn trust(&self, a: FactionId, b: FactionId) -> i32 {
        *self.trust.get(&Self::key(a, b)).unwrap_or(&50)
    }

    /// Set trust directly (bounded 0–100).
    pub fn set_trust(&mut self, a: FactionId, b: FactionId, score: i32) {
        self.trust.insert(Self::key(a, b), score.clamp(0, 100));
    }

    /// Apply a diplomacy action: shifts trust between two factions.
    pub fn diplomacy(&mut self, a: FactionId, b: FactionId, action: DiplomacyAction) -> TrustLevel {
        let key = Self::key(a, b);
        let current = *self.trust.get(&key).unwrap_or(&50);
        let new = (current + action.trust_delta()).clamp(0, 100);
        self.trust.insert(key, new);
        TrustLevel::from_score(new)
    }

    /// Offer a quest from a faction. Returns the quest id.
    pub fn offer_quest(
        &mut self,
        faction: FactionId,
        target_cell: crate::coords::CellCoord,
        reward_reputation: i32,
    ) -> u64 {
        self.next_quest_id += 1;
        let id = self.next_quest_id;
        self.quests.insert(
            id,
            Quest { id, faction, target_cell, reward_reputation, accepted: false, completed: false },
        );
        id
    }

    /// Accept a quest.
    pub fn accept_quest(&mut self, quest_id: u64) -> bool {
        self.quests.get_mut(&quest_id).map(|q| q.accepted = true).is_some()
    }

    /// Complete a quest (must be accepted first).
    pub fn complete_quest(&mut self, quest_id: u64) -> Option<i32> {
        let q = self.quests.get_mut(&quest_id)?;
        if q.accepted && !q.completed {
            q.completed = true;
            Some(q.reward_reputation)
        } else {
            None
        }
    }

    /// Claim territory for a faction.
    pub fn claim_territory(&mut self, faction: FactionId, regions: Vec<(i32, i32)>) {
        self.territory.entry(faction).or_default().extend(regions);
    }

    /// Which faction controls a region?
    pub fn controller_of(&self, region: (i32, i32)) -> Option<FactionId> {
        self.territory
            .iter()
            .find(|(_, regions)| regions.contains(&region))
            .map(|(f, _)| *f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::CellCoord;

    const A: FactionId = FactionId(1);
    const B: FactionId = FactionId(2);

    /// Trust starts neutral, shifts with diplomacy, and maps to levels.
    #[test]
    fn p3d605_trust_shifts_with_diplomacy() {
        let mut f = FactionRelations::new();
        assert_eq!(f.trust(A, B), 50, "default neutral");

        assert_eq!(
            f.diplomacy(A, B, DiplomacyAction::TradeAgreement),
            TrustLevel::Friendly,
            "trade agreement shifts to friendly"
        );
        assert_eq!(
            f.diplomacy(A, B, DiplomacyAction::Insult),
            TrustLevel::Neutral,
            "insult drops back toward neutral"
        );
        assert_eq!(
            f.diplomacy(A, B, DiplomacyAction::DeclareWar),
            TrustLevel::Hostile,
            "war declaration drops to hostile"
        );
    }

    /// Alliance requires high trust; hostile factions cannot trade.
    #[test]
    fn p3d605_trust_levels_gate_actions() {
        let mut f = FactionRelations::new();
        // Default trust 50 = Neutral, which CAN trade. Start hostile.
        f.set_trust(A, B, 5);
        assert!(!TrustLevel::from_score(f.trust(A, B)).can_trade(), "hostile cannot trade");

        // Build trust to Friendly via trade agreements.
        for _ in 0..2 {
            f.diplomacy(A, B, DiplomacyAction::TradeAgreement);
        }
        assert!(TrustLevel::from_score(f.trust(A, B)).can_trade());

        // Push trust to 75, then alliance action +30 reaches >= 80.
        f.set_trust(A, B, 75);
        f.diplomacy(A, B, DiplomacyAction::Alliance);
        assert!(
            TrustLevel::from_score(f.trust(A, B)).is_allied(),
            "alliance pushes to allied level"
        );
    }

    /// Quest lifecycle: offer → accept → complete → reward.
    #[test]
    fn p3d605_quest_lifecycle() {
        let mut f = FactionRelations::new();
        let id = f.offer_quest(A, CellCoord { x: 1, y: 2, z: 3 }, 25);
        assert!(f.accept_quest(id));
        assert_eq!(f.complete_quest(id), Some(25), "completing returns reward");
        // Already completed: cannot re-complete.
        assert_eq!(f.complete_quest(id), None);
    }

    /// Territory: factions claim regions, controller is queryable.
    #[test]
    fn p3d605_territory_claims() {
        let mut f = FactionRelations::new();
        f.claim_territory(A, vec![(0, 0), (1, 0)]);
        f.claim_territory(B, vec![(2, 0)]);
        assert_eq!(f.controller_of((0, 0)), Some(A));
        assert_eq!(f.controller_of((2, 0)), Some(B));
        assert_eq!(f.controller_of((10, 10)), None);
    }

    /// Determinism: same actions → same trust.
    #[test]
    fn p3d605_diplomacy_is_deterministic() {
        let mut a = FactionRelations::new();
        let mut b = FactionRelations::new();
        for _ in 0..5 {
            a.diplomacy(A, B, DiplomacyAction::TradeAgreement);
            b.diplomacy(A, B, DiplomacyAction::TradeAgreement);
        }
        assert_eq!(a.trust(A, B), b.trust(A, B));
    }
}
