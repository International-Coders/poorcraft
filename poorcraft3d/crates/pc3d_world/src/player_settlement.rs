//! P3D-606: player-founded settlements — appointments, policies, expansion.
//!
//! The player can found a settlement (choosing a site from the river
//! graph), appoint an NPC as steward, set policies (tax rate, curfew,
//! gate state), and expand by claiming nearby territory. All
//! deterministic and UI-independent.

use crate::coords::RegionCoord;
use crate::hydro::RiverGraph;
use crate::npc::Role;
use std::collections::BTreeMap;

/// A player-founded settlement: name, steward, policies, claimed cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerSettlement {
    pub name: String,
    pub center: RegionCoord,
    pub steward_id: Option<u64>,
    pub tax_rate: u8,
    pub curfew: bool,
    pub gates_open: bool,
    /// Claimed region coordinates (Chebyshev-adjacent expansion).
    pub claimed: Vec<(i32, i32)>,
}

/// Why a founding attempt failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FoundError {
    /// Another settlement already controls this center.
    Occupied,
    /// The site is not on walkable land near water.
    UnviableSite,
}

/// The player's founding and management of settlements.
#[derive(Clone, Debug, Default)]
pub struct PlayerSettlements {
    pub founded: BTreeMap<RegionCoord, PlayerSettlement>,
}

impl PlayerSettlements {
    /// Found a new settlement at a river region center. Fails if already
    /// occupied by another player settlement.
    pub fn found(
        &mut self,
        graph: &RiverGraph,
        center: RegionCoord,
        name: String,
    ) -> Result<(), FoundError> {
        if self.founded.contains_key(&center) {
            return Err(FoundError::Occupied);
        }
        // Site viability: must be a river region (water access).
        if !graph.is_river(center) {
            return Err(FoundError::UnviableSite);
        }
        self.founded.insert(
            center,
            PlayerSettlement {
                name,
                center,
                steward_id: None,
                tax_rate: 10,
                curfew: false,
                gates_open: true,
                claimed: vec![(center.x, center.z)],
            },
        );
        Ok(())
    }

    /// Appoint an NPC as steward (only one steward per settlement).
    pub fn appoint_steward(&mut self, center: &RegionCoord, npc_id: u64) -> bool {
        self.founded
            .get_mut(center)
            .map(|s| {
                s.steward_id = Some(npc_id);
            })
            .is_some()
    }

    /// Set the tax rate (0–25%).
    pub fn set_tax_rate(&mut self, center: &RegionCoord, rate: u8) -> bool {
        self.founded
            .get_mut(center)
            .map(|s| s.tax_rate = rate.min(25))
            .is_some()
    }

    /// Toggle curfew.
    pub fn set_curfew(&mut self, center: &RegionCoord, curfew: bool) -> bool {
        self.founded
            .get_mut(center)
            .map(|s| s.curfew = curfew)
            .is_some()
    }

    /// Toggle gates.
    pub fn set_gates(&mut self, center: &RegionCoord, open: bool) -> bool {
        self.founded
            .get_mut(center)
            .map(|s| s.gates_open = open)
            .is_some()
    }

    /// Expand territory: claim Chebyshev-adjacent regions (distance 1)
    /// around the settlement center that aren't already claimed.
    pub fn expand(&mut self, center: &RegionCoord) -> Vec<(i32, i32)> {
        let Some(s) = self.founded.get_mut(center) else {
            return Vec::new();
        };
        let mut newly_claimed = Vec::new();
        for dx in -1..=1i32 {
            for dz in -1..=1i32 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                let candidate = (s.center.x + dx, s.center.z + dz);
                if !s.claimed.contains(&candidate) {
                    s.claimed.push(candidate);
                    newly_claimed.push(candidate);
                }
            }
        }
        newly_claimed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::WorldGen;

    fn graph() -> RiverGraph {
        RiverGraph::new(&WorldGen::new(2024), 24)
    }

    fn river_center(graph: &RiverGraph) -> RegionCoord {
        for x in -20..=20 {
            for z in -20..=20 {
                if graph.is_river(RegionCoord { x, z }) {
                    return RegionCoord { x, z };
                }
            }
        }
        panic!("no river region found");
    }

    /// Found a settlement, appoint a steward, set policies.
    #[test]
    fn p3d606_found_appoint_policies() {
        let graph = graph();
        let center = river_center(&graph);
        let mut set = PlayerSettlements::default();
        set.found(&graph, center, "Riverton".into()).expect("found");
        assert!(set.appoint_steward(&center, 42));
        assert!(set.set_tax_rate(&center, 15));
        assert!(set.set_curfew(&center, true));
        assert!(set.set_gates(&center, false));
        let s = &set.founded[&center];
        assert_eq!(s.name, "Riverton");
        assert_eq!(s.steward_id, Some(42));
        assert_eq!(s.tax_rate, 15);
        assert!(s.curfew);
        assert!(!s.gates_open);
    }

    /// Founding on the same center twice fails.
    #[test]
    fn p3d606_double_found_fails() {
        let graph = graph();
        let center = river_center(&graph);
        let mut set = PlayerSettlements::default();
        set.found(&graph, center, "first".into()).expect("first");
        assert_eq!(set.found(&graph, center, "second".into()), Err(FoundError::Occupied));
    }

    /// Expansion claims Chebyshev-adjacent regions.
    #[test]
    fn p3d606_expansion_claims_adjacent() {
        let graph = graph();
        let center = river_center(&graph);
        let mut set = PlayerSettlements::default();
        set.found(&graph, center, "expanding".into()).expect("found");
        let claimed = set.expand(&center);
        assert_eq!(claimed.len(), 8, "8 Chebyshev-adjacent regions");
        for (dx, dz) in &claimed {
            let d = (dx.abs() - 1).max(dz.abs() - 1);
            let _ = d;
        }
        assert!(set.founded[&center].claimed.contains(&(center.x + 1, center.z)));
    }
}
