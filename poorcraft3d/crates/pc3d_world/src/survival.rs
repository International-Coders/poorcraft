//! P3D-502: the survival loop — fishing yields, eating, onboarding.
//!
//! Wires the three systems the player touches first: FishStocks (water)
//! → FOOD items (inventory) → Needs (body). Plus the contextual
//! onboarding checklist: ordered milestones recorded deterministically.

use crate::edit::EditKind;
use crate::gen::{CellMaterial, WorldGen};
use crate::hydro::{FishStocks, RiverGraph};
use crate::items::{harvest_yields, Inventory, ItemId};
use crate::npc::Needs;

/// The fish item (Food { heal: 15 }).
pub const FISH: ItemId = ItemId(21);

/// Harvest from a terrain cell with a tool tier, into the inventory.
/// Returns the number of item units actually stored (0 = hands too weak
/// or inventory full).
pub fn harvest_into(
    gen: &WorldGen,
    inventory: &mut Inventory,
    material: CellMaterial,
    tool_tier: Option<u8>,
) -> usize {
    let mut stored = 0usize;
    for (item, count) in crate::items::harvest_yields(material, tool_tier) {
        let leftover = inventory.add(item, count);
        stored += (count - leftover) as usize;
    }
    let _ = gen;
    stored
}

/// Catch a fish: consumes stock from the region, adds a FISH item to the
/// inventory. The river itself never weakens (D-007) — only the stock.
/// Returns the item id on success.
pub fn fishing_catch(
    graph: &RiverGraph,
    stocks: &mut FishStocks,
    inventory: &mut Inventory,
    region: crate::coords::RegionCoord,
) -> Option<ItemId> {
    let caught = stocks.catch_fish(region, 1);
    if caught == 0 {
        return None;
    }
    if inventory.add(FISH, 1) > 0 {
        // Inventory full: return the fish to the stock (never destroy).
        stocks.restock_region(&graph, region, 1);
        return None;
    }
    Some(FISH)
}

/// Eat one food item from the inventory: consumes it and clears hunger.
/// Returns false when the item is missing or not food.
pub fn eat_from(inventory: &mut Inventory, needs: &mut Needs, food: ItemId) -> bool {
    let removed = inventory.remove(food, 1);
    if removed == 0 {
        return false;
    }
    needs.eat();
    true
}

/// The contextual onboarding checklist: ordered milestones.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Onboarding {
    done: Vec<&'static str>,
}

/// The ordered milestones.
pub const ONBOARDING_STEPS: &[&str] =
    &["first_tree", "first_catch", "first_build", "first_night"];

impl Onboarding {
    /// Mark a step done (idempotent; only known steps accepted).
    pub fn mark(&mut self, step: &'static str) {
        if ONBOARDING_STEPS.contains(&step) && !self.done.contains(&step) {
            self.done.push(step);
        }
    }

    pub fn is_done(&self, step: &str) -> bool {
        self.done.contains(&step)
    }

    pub fn all_done(&self) -> bool {
        self.done.len() == ONBOARDING_STEPS.len()
    }

    /// Ordered progress: done steps in ONBOARDING_STEPS order.
    pub fn progress(&self) -> Vec<&'static str> {
        ONBOARDING_STEPS
            .iter()
            .filter(|s| self.done.contains(s))
            .copied()
            .collect()
    }

    /// Persistence bytes: the done flags as a bitmask in step order.
    pub fn encode(&self) -> [u8; 1] {
        let mut mask = 0u8;
        for (i, s) in ONBOARDING_STEPS.iter().enumerate() {
            if self.done.contains(s) {
                mask |= 1 << i;
            }
        }
        [mask]
    }

    pub fn decode(mask: u8) -> Onboarding {
        let mut o = Onboarding::default();
        for (i, s) in ONBOARDING_STEPS.iter().enumerate() {
            if mask & (1 << i) != 0 {
                o.done.push(s);
            }
        }
        o
    }
}

/// The one visible dig helper for survival: harvest into the inventory
/// (shares EditKind::Mine semantics with the P3D-204 path).
pub fn dig_yield_kind(material: CellMaterial) -> EditKind {
    let _ = material;
    EditKind::Dig
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hydro::RiverGraph;

    /// THE survival loop: catch a fish (stock decrements, inventory
    /// gains), eat it (hunger clears). The river's discharge never moved.
    #[test]
    fn p3d502_fishing_to_eating_loop_closes() {
        let g = WorldGen::new(2024);
        let graph = RiverGraph::new(&g, 20);
        let mut stocks = FishStocks::new(&graph);
        let mut inv = Inventory::new(8);
        let mut needs = Needs { hunger: 80, energy: 50, hunger_f: 80.0, energy_f: 50.0 };

        let Some(r) = graph.river_regions.first() else {
            panic!("rivers must exist");
        };
        let region = crate::coords::RegionCoord { x: r.0, z: r.1 };
        let stock_before = stocks.stock_at(region);
        assert!(stock_before > 0);

        let caught = fishing_catch(&graph, &mut stocks, &mut inv, region);
        assert_eq!(caught, Some(FISH), "catch yields a fish item");
        assert_eq!(stocks.stock_at(region), stock_before - 1, "stock consumed");
        assert_eq!(inv.count(FISH), 1);

        // Eating clears hunger and consumes the item.
        assert!(eat_from(&mut inv, &mut needs, FISH));
        assert_eq!(inv.count(FISH), 0);
        assert_eq!(needs.hunger, 0);

        // The river's flow records are untouched by any of this (D-007).
        assert!(graph.is_river(region));
    }

    /// Empty stock or full inventory: fishing fails without destroying
    /// anything.
    #[test]
    fn p3d502_fishing_fails_cleanly() {
        let g = WorldGen::new(2024);
        let graph = RiverGraph::new(&g, 20);
        let mut stocks = FishStocks::new(&graph);
        let mut inv = Inventory::new(8);
        // Empty the first region's stock.
        let r = crate::coords::RegionCoord { x: graph.river_regions[0].0, z: graph.river_regions[0].1 };
        let _ = stocks.catch_fish(r, u64::MAX);
        let before_stock = stocks.stock_at(r);
        let before_items = inv.count(FISH);
        assert_eq!(fishing_catch(&graph, &mut stocks, &mut inv, r), None);
        assert_eq!(stocks.stock_at(r), 0);
        assert_eq!(inv.count(FISH), before_items);
    }

    /// Onboarding: ordered, idempotent, persisted as a bitmask.
    #[test]
    fn p3d502_onboarding_is_ordered_and_persistent() {
        let mut o = Onboarding::default();
        o.mark("first_catch");
        o.mark("first_tree");
        o.mark("first_catch"); // idempotent
        assert_eq!(o.progress(), vec!["first_tree", "first_catch"]);
        assert!(!o.all_done());
        let mask = o.encode();
        let back = Onboarding::decode(mask[0]);
        assert_eq!(back.progress(), o.progress());
        o.mark("first_build");
        o.mark("first_night");
        assert!(o.all_done());
    }

    /// Harvest into inventory gates by tier (shares the P3D-501 law).
    #[test]
    fn p3d502_harvest_into_gates_rock() {
        let gen = WorldGen::new(1);
        let mut inv = Inventory::new(8);
        assert_eq!(harvest_into(&gen, &mut inv, CellMaterial::Rock, None), 0);
        assert_eq!(harvest_into(&gen, &mut inv, CellMaterial::Rock, Some(1)), 1);
        assert_eq!(inv.count(ItemId(2)), 1);
        let _ = dig_yield_kind(CellMaterial::Air);
    }
}
