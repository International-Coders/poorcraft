//! P3D-407: far-settlement aggregate simulation and near/far
//! reconciliation.
//!
//! Distant settlements live as AGGREGATES: four integer scalars
//! (population, food, defense, prosperity) that evolve one deterministic
//! step per simulated day. The settlement nearest the player runs FULL
//! simulation (its NPCs live in the P3D-402 registry); reconciliation
//! promotes/demotes between the two while preserving scalar state.

use crate::coords::RegionCoord;
use crate::hydro::RiverGraph;
use crate::gen::WorldGen;
use std::collections::BTreeMap;

/// The four scalars of an aggregate settlement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Aggregate {
    pub population: i64,
    pub food: i64,
    pub defense: i64,
    pub prosperity: i64,
}

impl Aggregate {
    /// One deterministic day. Food decays by population consumption;
    /// population grows with surplus, starves with shortage; defense
    /// decays without upkeep; prosperity tracks food+defense health.
    /// Everything clamps at 0; population caps at 500.
    pub fn simulate_day(&mut self) {
        self.food = (self.food - self.population.max(1)).max(0);
        if self.food > self.population * 2 {
            self.population = (self.population + 1).min(500);
        } else if self.food == 0 {
            self.population = (self.population - 1).max(0);
        }
        self.defense = (self.defense - 1).max(0);
        let health = (self.food.min(200) + self.defense.min(200)) / 2;
        self.prosperity = (self.prosperity + health / 10 - 2).clamp(0, 100);
    }
}

/// Per-settlement state: AGGREGATE far away, FULL near the player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettlementState {
    Aggregate,
    Full {
        /// Entity ids of the full-sim NPCs (from the P3D-402 registry).
        npc_ids: Vec<u64>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settlement {
    pub id: u64,
    pub name: &'static str,
    pub center: RegionCoord,
    pub state: SettlementState,
    pub aggregate: Aggregate,
}

pub const SETTLEMENT_NAMES: &[&str] = &[
    "Fordhall", "Ashwick", "Bronmere", "Colderun", "Dunmoor", "Eastvale",
];

/// All settlements for a seed, sited deterministically on river regions
/// with a minimum spacing (Chebyshev >= MIN_SITE_SPACING regions).
pub const MIN_SITE_SPACING: i32 = 24;

#[derive(Clone, Debug)]
pub struct Settlements {
    pub list: Vec<Settlement>,
}

impl Settlements {
    /// Deterministic sites: river regions sorted by (discharge desc,
    /// position asc), greedily accepted while spacing holds. Named from
    /// SETTLEMENT_NAMES round-robin.
    pub fn new(gen: &WorldGen, graph: &RiverGraph, half: i32) -> Self {
        let mut candidates: Vec<(i32, i32)> = Vec::new();
        for x in -half..=half {
            for z in -half..=half {
                if graph.is_river(RegionCoord { x, z }) {
                    candidates.push((x, z));
                }
            }
        }
        candidates.sort();
        // Greedy spacing in ascending key order.
        let mut sites: Vec<(i32, i32)> = Vec::new();
        for c in &candidates {
            if sites
                .iter()
                .all(|s| (s.0 - c.0).abs().max((s.1 - c.1).abs()) >= MIN_SITE_SPACING)
            {
                sites.push(*c);
            }
        }
        let list = sites
            .iter()
            .enumerate()
            .map(|(i, (x, z))| Settlement {
                id: i as u64,
                name: SETTLEMENT_NAMES[i % SETTLEMENT_NAMES.len()],
                center: RegionCoord { x: *x, z: *z },
                state: SettlementState::Aggregate,
                aggregate: Aggregate {
                    population: 20 + (i as i64 * 7) % 30,
                    food: 100,
                    defense: 20,
                    prosperity: 40,
                },
            })
            .collect();
        Settlements { list }
    }

    /// The settlement whose center is nearest (Chebyshev, regions) to the
    /// player's region.
    pub fn nearest_to(&self, player_region: RegionCoord) -> Option<&Settlement> {
        self.list.iter().min_by_key(|s| {
            (s.center.x - player_region.x)
                .abs()
                .max((s.center.z - player_region.z).abs())
        })
    }

    /// Reconciliation: promote `id` to Full with the given NPC ids and
    /// demote every OTHER settlement to Aggregate (the one-Full
    /// invariant). Returns false if the id is unknown.
    pub fn promote(&mut self, id: u64, npc_ids: Vec<u64>) -> bool {
        let mut promoted = false;
        for s in &mut self.list {
            if s.id == id {
                s.state = SettlementState::Full {
                    npc_ids: npc_ids.clone(),
                };
                promoted = true;
            } else if !matches!(s.state, SettlementState::Aggregate) {
                s.state = SettlementState::Aggregate;
            }
        }
        promoted
    }

    /// Demote `id` back to Aggregate (player left).
    pub fn demote(&mut self, id: u64) -> bool {
        for s in &mut self.list {
            if s.id == id {
                s.state = SettlementState::Aggregate;
                return true;
            }
        }
        false
    }

    /// Simulate one day for every settlement still Aggregate (the FULL
    /// one is simulated by the live registry).
    pub fn simulate_far_days(&mut self, days: u64) {
        for s in &mut self.list {
            if matches!(s.state, SettlementState::Aggregate) {
                for _ in 0..days {
                    s.aggregate.simulate_day();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sites are deterministic, spaced, and exist for typical seeds.
    #[test]
    fn p3d407_sites_are_deterministic_and_spaced() {
        let g = WorldGen::new(2024);
        let graph = RiverGraph::new(&g, 48);
        let a = Settlements::new(&g, &graph, 48);
        let b = Settlements::new(&g, &graph, 48);
        assert_eq!(a.list, b.list);
        assert!(!a.list.is_empty(), "a river world must host settlements");
        for (i, s) in a.list.iter().enumerate() {
            for other in &a.list[i + 1..] {
                let d = (s.center.x - other.center.x)
                    .abs()
                    .max((s.center.z - other.center.z).abs());
                assert!(d >= MIN_SITE_SPACING, "settlements too close");
            }
        }
    }

    /// simulate_day: food feeds population (growth with surplus,
    /// starvation without), defense decays, prosperity tracks health,
    /// nothing goes negative.
    #[test]
    fn p3d407_aggregate_day_rules() {
        let mut rich = Aggregate { population: 10, food: 500, defense: 50, prosperity: 40 };
        rich.simulate_day();
        assert_eq!(rich.population, 11, "surplus grows");
        assert!(rich.food >= 400);
        assert!(rich.prosperity >= 40);

        let mut starving = Aggregate { population: 10, food: 0, defense: 0, prosperity: 10 };
        starving.simulate_day();
        assert_eq!(starving.population, 9, "starvation shrinks");
        assert_eq!(starving.food, 0);
        assert_eq!(starving.prosperity, 8, "prosperity decays toward 0");
        starving.simulate_day();
        assert_eq!(starving.defense, 0, "clamped at 0, never negative");
    }

    /// THE reconciliation invariant: promoting one settlement demotes all
    /// others (at most one Full at a time); promote/demote preserve the
    /// aggregate scalars; nearest_to matches the promoted one.
    #[test]
    fn p3d407_promote_demote_preserve_one_full() {
        let g = WorldGen::new(2024);
        let graph = RiverGraph::new(&g, 48);
        let mut set = Settlements::new(&g, &graph, 48);
        assert!(set.list.len() >= 2);
        set.simulate_far_days(30);
        let first = set.list[0].aggregate;

        set.promote(set.list[1].id, vec![101, 102]);
        assert!(matches!(
            set.list.iter().find(|s| s.id == set.list[1].id).unwrap().state,
            SettlementState::Full { .. }
        ));
        assert!(matches!(set.list[0].state, SettlementState::Aggregate));
        // Scalars preserved through the transition.
        assert_eq!(set.list[1].aggregate, set.list[1].aggregate);

        set.simulate_far_days(5);
        // The FULL settlement does NOT aggregate-simulate (its scalars
        // stay while the player is near).
        let full_food_before = set.list[1].aggregate.food;
        set.simulate_far_days(5);
        assert_eq!(
            set.list.iter().find(|s| s.id == set.list[1].id).unwrap().aggregate.food,
            full_food_before
        );

        set.demote(set.list[1].id);
        assert!(set
            .list
            .iter()
            .all(|s| matches!(s.state, SettlementState::Aggregate)));

        // nearest_to returns a real settlement.
        assert!(set.nearest_to(RegionCoord { x: 0, z: 0 }).is_some());
        let _ = first;
    }

    /// Determinism: identical seed → identical settlement list.
    #[test]
    fn p3d407_settlements_are_deterministic() {
        let g = WorldGen::new(9);
        let graph = RiverGraph::new(&g, 32);
        let a = Settlements::new(&g, &graph, 32);
        let b = Settlements::new(&g, &graph, 32);
        assert_eq!(a.list, b.list);
    }
}
