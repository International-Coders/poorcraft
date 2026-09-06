//! P3D-609: allied, puppet/protectorate, rival, and conquered-city
//! relationship contracts.
//!
//! The player's faction has RELATIONSHIPS with other factions. Each kind
//! carries different autonomy (how much the other faction governs
//! itself), tribute (what the player collects), and growth multiplier
//! (how fast the other faction's population grows). Deterministic.

use std::collections::BTreeMap;

/// The kind of relationship between the player's faction and another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelationshipKind {
    Allied,
    Puppet,
    Protectorate,
    Rival,
    Conquered,
}

impl RelationshipKind {
    /// How much the other faction governs itself (0–100).
    pub fn autonomy(self) -> u8 {
        match self {
            RelationshipKind::Allied => 100,
            RelationshipKind::Puppet => 30,
            RelationshipKind::Protectorate => 60,
            RelationshipKind::Rival => 100,
            RelationshipKind::Conquered => 20,
        }
    }

    /// Tribute rate: fraction of prosperity collected per day (basis points).
    pub fn tribute_rate(self) -> u64 {
        match self {
            RelationshipKind::Allied => 0,
            RelationshipKind::Puppet => 500,
            RelationshipKind::Protectorate => 200,
            RelationshipKind::Rival => 0,
            RelationshipKind::Conquered => 800,
        }
    }

    /// Population growth percentage per day.
    pub fn growth_pct(self) -> u64 {
        match self {
            RelationshipKind::Allied => 5,
            RelationshipKind::Puppet => 2,
            RelationshipKind::Protectorate => 3,
            RelationshipKind::Rival => 1,
            RelationshipKind::Conquered => 1,
        }
    }
}

/// A relationship with another faction's settlement/city.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CityRelationship {
    pub faction_id: u64,
    pub name: String,
    pub kind: RelationshipKind,
    pub population: i64,
    pub prosperity: i64,
    /// Uncollected tribute accumulated over days.
    pub tribute_owed: u64,
}

/// Manages all relationships.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RelationshipSystem {
    pub relationships: BTreeMap<u64, CityRelationship>,
    next_id: u64,
}

impl RelationshipSystem {
    pub fn new() -> Self {
        RelationshipSystem::default()
    }

    /// Establish a relationship with another faction.
    pub fn establish(
        &mut self,
        faction_id: u64,
        name: String,
        kind: RelationshipKind,
        population: i64,
    ) {
        self.next_id += 1;
        self.relationships.insert(
            faction_id,
            CityRelationship {
                faction_id,
                name,
                kind,
                population,
                prosperity: 50,
                tribute_owed: 0,
            },
        );
    }

    /// Release a puppet/protectorate/conquered city to independence (Allied).
    pub fn release(&mut self, faction_id: u64) -> bool {
        let Some(rel) = self.relationships.get_mut(&faction_id) else {
            return false;
        };
        if rel.kind == RelationshipKind::Allied || rel.kind == RelationshipKind::Rival {
            return false; // already independent or hostile
        }
        rel.kind = RelationshipKind::Allied;
        true
    }

    /// Change the relationship kind.
    pub fn change_kind(&mut self, faction_id: u64, kind: RelationshipKind) -> bool {
        let Some(rel) = self.relationships.get_mut(&faction_id) else {
            return false;
        };
        rel.kind = kind;
        true
    }

    /// Simulate one day for all relationships: puppets and conquered
    /// cities grow autonomously (slower than Allied), tribute accumulates.
    pub fn simulate_day(&mut self) {
        for rel in self.relationships.values_mut() {
            let growth_pct = rel.kind.growth_pct() as i64;
            let change = rel.population * growth_pct / 100;
            rel.population += change;
            rel.prosperity = (rel.prosperity + 1).min(100);

            let tribute = rel.kind.tribute_rate();
            if tribute > 0 {
                rel.tribute_owed += rel.population as u64 * tribute / 10_000;
            }
        }
    }

    /// Collect all owed tribute. Returns total collected and clears owed.
    pub fn collect_tribute(&mut self) -> u64 {
        let mut total = 0;
        for rel in self.relationships.values_mut() {
            total += rel.tribute_owed;
            rel.tribute_owed = 0;
        }
        total
    }

    pub fn len(&self) -> usize {
        self.relationships.len()
    }

    pub fn is_empty(&self) -> bool {
        self.relationships.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Relationship kinds have different autonomy, tribute, and growth.
    #[test]
    fn p3d609_kinds_have_distinct_properties() {
        let kinds = [
            RelationshipKind::Allied,
            RelationshipKind::Puppet,
            RelationshipKind::Protectorate,
            RelationshipKind::Rival,
            RelationshipKind::Conquered,
        ];
        // Allied has full autonomy, no tribute.
        assert_eq!(RelationshipKind::Allied.autonomy(), 100);
        assert_eq!(RelationshipKind::Allied.tribute_rate(), 0);
        // Conquered has lowest autonomy, highest tribute.
        assert!(RelationshipKind::Conquered.autonomy() < RelationshipKind::Puppet.autonomy());
        assert!(RelationshipKind::Conquered.tribute_rate() > RelationshipKind::Puppet.tribute_rate());
        // All have nonzero growth.
        for k in kinds {
            assert!(k.growth_pct() > 0);
        }
    }

    /// Establish, grow puppets over days, collect tribute, release.
    #[test]
    fn p3d609_puppet_growth_and_tribute_lifecycle() {
        let mut sys = RelationshipSystem::new();
        sys.establish(1, "Vassal Town".into(), RelationshipKind::Puppet, 100);
        sys.establish(2, "Conquered City".into(), RelationshipKind::Conquered, 200);
        assert_eq!(sys.len(), 2);

        // Simulate 10 days: puppets grow and owe tribute.
        for _ in 0..10 {
            sys.simulate_day();
        }

        let vassal = &sys.relationships[&1];
        assert!(vassal.population > 100, "puppet grew from 100");
        assert!(vassal.tribute_owed > 0, "puppet owes tribute");
        let conquered = &sys.relationships[&2];
        assert!(conquered.tribute_owed > 0, "conquered owes tribute");
        assert!(conquered.population > 200, "conquered grew (slowly)");

        // Collect: clears all owed tribute.
        let total = sys.collect_tribute();
        assert!(total > 0);
        assert!(sys.relationships.values().all(|r| r.tribute_owed == 0));
    }

    /// Release changes puppet to Allied (independence).
    #[test]
    fn p3d609_release_grants_independence() {
        let mut sys = RelationshipSystem::new();
        sys.establish(5, "Satellite".into(), RelationshipKind::Puppet, 50);
        assert!(sys.release(5));
        assert_eq!(sys.relationships[&5].kind, RelationshipKind::Allied);
        // Already allied: cannot re-release.
        assert!(!sys.release(5));
    }

    /// Rivals grow slower than puppets.
    #[test]
    fn p3d609_rivals_grow_slower_than_puppets() {
        let mut sys = RelationshipSystem::new();
        sys.establish(1, "Puppet Town".into(), RelationshipKind::Puppet, 100);
        sys.establish(2, "Rival City".into(), RelationshipKind::Rival, 100);
        for _ in 0..10 {
            sys.simulate_day();
        }
        let puppet_pop = sys.relationships[&1].population;
        let rival_pop = sys.relationships[&2].population;
        assert!(
            puppet_pop > rival_pop,
            "puppet {puppet_pop} should grow faster than rival {rival_pop}"
        );
    }

    /// Determinism: same actions → same state.
    #[test]
    fn p3d609_relationships_are_deterministic() {
        let mut a = RelationshipSystem::new();
        let mut b = RelationshipSystem::new();
        a.establish(1, "Town".into(), RelationshipKind::Protectorate, 50);
        b.establish(1, "Town".into(), RelationshipKind::Protectorate, 50);
        for _ in 0..20 {
            a.simulate_day();
            b.simulate_day();
        }
        assert_eq!(a.relationships[&1].population, b.relationships[&1].population);
        assert_eq!(a.relationships[&1].prosperity, b.relationships[&1].prosperity);
    }
}
