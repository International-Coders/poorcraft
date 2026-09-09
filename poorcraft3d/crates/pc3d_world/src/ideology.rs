//! P3D-614: ideology-founded player factions.
//!
//! A player creates a faction with an ideology that shapes diplomacy,
//! recruitment, and law. The ideology can evolve through play.

/// Ideology archetypes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ideology {
    Conquest,
    Commerce,
    Faith,
    Liberty,
    Isolation,
}

impl Ideology {
    pub fn name(self) -> &'static str {
        match self {
            Ideology::Conquest => "conquest",
            Ideology::Commerce => "commerce",
            Ideology::Faith => "faith",
            Ideology::Liberty => "liberty",
            Ideology::Isolation => "isolation",
        }
    }

    /// Diplomacy bonus with factions of the same ideology.
    pub fn same_ideology_diplomacy_bonus(self) -> i32 {
        match self {
            Ideology::Conquest => 0,
            Ideology::Commerce => 10,
            Ideology::Faith => 15,
            Ideology::Liberty => 10,
            Ideology::Isolation => 0,
        }
    }

    /// Recruitment bonus (how appealing to potential recruits).
    pub fn recruitment_appeal(self) -> i32 {
        match self {
            Ideology::Conquest => 15,
            Ideology::Commerce => 5,
            Ideology::Faith => 10,
            Ideology::Liberty => 12,
            Ideology::Isolation => 0,
        }
    }

    /// Law strictness modifier (how strict laws are).
    pub fn law_strictness(self) -> i32 {
        match self {
            Ideology::Conquest => 30,
            Ideology::Commerce => 10,
            Ideology::Faith => 25,
            Ideology::Liberty => -20,
            Ideology::Isolation => 15,
        }
    }
}

/// A player-founded faction with an ideology.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerFaction {
    pub faction: crate::faction::FactionId,
    pub name: String,
    pub ideology: Ideology,
    /// Ideology drift: how far the faction has moved from its founding.
    pub ideology_drift: i32,
}

impl PlayerFaction {
    pub fn new(faction: crate::faction::FactionId, name: String, ideology: Ideology) -> Self {
        PlayerFaction {
            faction,
            name,
            ideology,
            ideology_drift: 0,
        }
    }

    /// Ideology can evolve through play (D-031).
    pub fn shift_ideology(&mut self, direction: Ideology) {
        if direction != self.ideology {
            self.ideology_drift += 10;
            self.ideology = direction;
        }
    }

    /// Diplomacy bonus with same-ideology factions, adjusted by drift.
    pub fn diplomacy_bonus_with(&self, other: Ideology) -> i32 {
        if other == self.ideology && self.ideology_drift < 30 {
            self.ideology.same_ideology_diplomacy_bonus()
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ideologies have distinct bonuses.
    #[test]
    fn p3d614_ideologies_shape_diplomacy_and_law() {
        assert!(Ideology::Faith.same_ideology_diplomacy_bonus() > 0);
        assert_eq!(Ideology::Isolation.same_ideology_diplomacy_bonus(), 0);
        assert!(Ideology::Conquest.law_strictness() > Ideology::Liberty.law_strictness());
        assert!(Ideology::Conquest.recruitment_appeal() > Ideology::Isolation.recruitment_appeal());
    }

    /// Player faction: founding, ideology shift, diplomacy bonus.
    #[test]
    fn p3d614_player_faction_ideology_evolution() {
        let fid = crate::faction::FactionId(99);
        let mut pf = PlayerFaction::new(fid, "Iron Pact".into(), Ideology::Commerce);
        assert_eq!(pf.ideology, Ideology::Commerce);
        assert_eq!(pf.ideology_drift, 0);
        assert!(
            pf.diplomacy_bonus_with(Ideology::Commerce) > 0,
            "same ideology bonus active"
        );
        // Shift to Faith: drift increases, bonus shifts.
        pf.shift_ideology(Ideology::Faith);
        assert_eq!(pf.ideology, Ideology::Faith);
        assert!(pf.ideology_drift > 0);
        assert_eq!(
            pf.diplomacy_bonus_with(Ideology::Faith),
            15,
            "new ideology bonus active"
        );
        assert_eq!(
            pf.diplomacy_bonus_with(Ideology::Commerce),
            0,
            "old ideology bonus gone"
        );
    }
}
