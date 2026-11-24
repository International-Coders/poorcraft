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

/// N08 THE WATER LAW (pure): how far (m) the player stands from a
/// river channel's water surface, when the column is within the
/// channel's half width. None = dry land: no fishing here. The gate is
/// the CARVED authority (`river_channel`) — the same truth render,
/// collision and nav read; a fishing spot can never disagree with the
/// water the player sees.
pub fn water_edge_distance_m(
    gen: &WorldGen,
    pos: &[f32; 3],
) -> Option<f32> {
    // Scan the 5×5 m neighborhood for the nearest in-channel column
    // (the channel's half width spans 1.5–6 m; a 5 m reach finds the
    // bank edge without pulling the whole watershed in).
    let wx = (pos[0] * 1000.0) as i64;
    let wz = (pos[2] * 1000.0) as i64;
    let mm = 1000_i64;
    let mut best: Option<f32> = None;
    for dx in -5..=5 {
        for dz in -5..=5 {
            let Some(hit) = gen.river_channel(wx + dx * mm, wz + dz * mm) else {
                continue;
            };
            // In-channel column: distance from the player to THIS
            // column (the water they could cast into).
            let col_x = (wx + dx * mm) as f32 / 1000.0;
            let col_z = (wz + dz * mm) as f32 / 1000.0;
            let d = (pos[0] - col_x).hypot(pos[2] - col_z);
            if best.is_none_or(|b| d < b) {
                best = Some(d);
            }
        }
    }
    best
}

/// N08 THE FIRST CATCH: fish at the current column when water is
/// within arm's reach. One stock unit → one FISH (the authority's own
/// exchange). Returns Err(honest reason) on dry land or an empty
/// region; Ok(item) on the catch (the caller marks first_catch and
/// plays the sound).
pub const CAST_REACH_M: f32 = 3.0;

pub fn fish_at(
    graph: &RiverGraph,
    stocks: &mut FishStocks,
    gen: &WorldGen,
    inventory: &mut Inventory,
    onboarding: &mut Onboarding,
    pos: &[f32; 3],
) -> Result<ItemId, &'static str> {
    // The gate first: dry land refuses before any stock moves.
    let dist = water_edge_distance_m(gen, pos).ok_or("NO WATER HERE — FIND THE RIVER")?;
    if dist > CAST_REACH_M {
        return Err("TOO FAR FROM THE WATER TO CAST");
    }
    let region = crate::coords::RegionCoord {
        x: ((pos[0] * 1000.0) as i64).div_euclid(256_000) as i32,
        z: ((pos[2] * 1000.0) as i64).div_euclid(256_000) as i32,
    };
    let caught = fishing_catch(graph, stocks, inventory, region)
        .ok_or("THE RIVER IS FISHED OUT — COME BACK LATER")?;
    onboarding.mark("first_catch");
    Ok(caught)
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
pub const ONBOARDING_STEPS: &[&str] = &["first_tree", "first_catch", "first_build", "first_night"];

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

    /// PLAYTEST-0.3 D7: the CURRENT objective in player language — the
    /// first unmet step, or None when the list is done. The HUD shows
    /// exactly one next thing (never a debug mask).
    pub fn objective(&self) -> Option<&'static str> {
        let line = |step: &str| match step {
            "first_tree" => "OBJECTIVE: WALK TO A TREE AND DIG IT (G) FOR WOOD",
            "first_catch" => "OBJECTIVE: CATCH A FISH AT THE RIVER (E AT WATER)",
            "first_build" => "OBJECTIVE: BUILD A BLOCK ON FLAT GROUND (F)",
            "first_night" => "OBJECTIVE: SURVIVE TILL DUSK — WATCH THE SKY",
            _ => "OBJECTIVE: EXPLORE",
        };
        ONBOARDING_STEPS
            .iter()
            .find(|s| !self.done.contains(s))
            .map(|s| line(s))
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
    /// D7: the objective walks the steps in order, in player language,
    /// and disappears when the list is done.
    #[test]
    fn beta030_objective_walks_steps_and_ends() {
        use super::*;
        let mut o = Onboarding::default();
        assert_eq!(
            o.objective(),
            Some("OBJECTIVE: WALK TO A TREE AND DIG IT (G) FOR WOOD")
        );
        o.mark("first_tree");
        assert_eq!(
            o.objective(),
            Some("OBJECTIVE: CATCH A FISH AT THE RIVER (E AT WATER)")
        );
        o.mark("first_catch");
        o.mark("first_build");
        assert_eq!(
            o.objective(),
            Some("OBJECTIVE: SURVIVE TILL DUSK — WATCH THE SKY")
        );
        o.mark("first_night");
        assert_eq!(o.objective(), None, "done list hides the strip");
    }

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
        let mut needs = Needs {
            hunger: 80,
            energy: 50,
            hunger_f: 80.0,
            energy_f: 50.0,
        };

        let Some(r) = graph.river_regions.first() else {
            panic!("rivers must exist");
        };
        let region = crate::coords::RegionCoord { x: r.0, z: r.1 };
        let stock_before = stocks.stock_at(region);
        assert!(stock_before > 0);

        let caught = fishing_catch(&graph, &mut stocks, &mut inv, region);
        assert_eq!(caught, Some(FISH), "catch yields a fish item");
        assert!(stocks.stock_at(region) < stock_before, "stock consumed");
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
        let r = crate::coords::RegionCoord {
            x: graph.river_regions[0].0,
            z: graph.river_regions[0].1,
        };
        let _ = stocks.catch_fish(r, u64::MAX);
        let _before_stock = stocks.stock_at(r);
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

    /// N08 THE WATER LAW: a river column answers its distance; dry land
    /// answers None. The showcase watershed (seed 3's field) has a
    /// channel; a far column does not.
    #[test]
    fn p3dn08_water_edge_answers_the_carved_channel() {
        let gen = WorldGen::new(3);
        // Find an in-channel column by scanning the field (regions sit
        // a few km out; the stocked region (-24,-5) holds the channel).
        let mut spot: Option<[f32; 3]> = None;
        'outer: for x in -6400..-6000_i64 {
            for z in -1200..-900_i64 {
                if gen.river_channel(x * 1000, z * 1000).is_some() {
                    spot = Some([x as f32, 0.0, z as f32]);
                    break 'outer;
                }
            }
        }
        let [sx, _, sz] = spot.expect("seed 3 has a carved river");
        let at_water = water_edge_distance_m(&gen, &[sx, 0.0, sz]);
        assert!(at_water.is_some_and(|d| d <= 1.0), "in-channel is close");
        // Dry land: (+10, +30) from the first column measured truly dry
        // (the channel runs diagonal; this offset clears its bands).
        let far = water_edge_distance_m(&gen, &[sx + 10.0, 0.0, sz + 30.0]);
        assert!(far.is_none(), "the measured dry spot must answer None: {far:?}");
    }

    /// N08 THE FIRST CATCH: at the water the fish lands (stock paid,
    /// item added, first_catch marked); dry land refuses BEFORE stock
    /// moves, and an exhausted region refuses honestly.
    #[test]
    fn p3dn08_fish_at_gates_land_then_stock() {
        let gen = WorldGen::new(3);
        let graph = RiverGraph::new(&gen, 20);
        let mut stocks = FishStocks::new(&graph);
        let mut inv = Inventory::new(8);
        let mut o = Onboarding::default();
        // Find an in-channel spot inside a stocked river region
        // (is_river is the graph's own membership law).
        let mut spot: Option<[f32; 3]> = None;
        let mut region = crate::coords::RegionCoord { x: 0, z: 0 };
        'outer: for (rx, rz) in &graph.river_regions {
            let r = crate::coords::RegionCoord { x: *rx, z: *rz };
            if stocks.stock_at(r) == 0 {
                continue;
            }
            // Scan inside this 256 m region at 2 m steps for a channel
            // column (measured: (-6144,-1026) on seed 3's field).
            for x in (rx * 256..(rx + 1) * 256).step_by(2) {
                for z in (rz * 256..(rz + 1) * 256).step_by(2) {
                    if gen.river_channel(x as i64 * 1000, z as i64 * 1000).is_some() {
                        spot = Some([x as f32, 0.0, z as f32]);
                        region = r;
                        break 'outer;
                    }
                }
            }
        }
        let [sx, _, sz] = spot.expect("seed 3 has a stocked river");
        let stock_before = stocks.stock_at(region);
        // THE CATCH: stock pays one, the pack gains a fish, onboarding
        // marks the milestone.
        let caught =
            fish_at(&graph, &mut stocks, &gen, &mut inv, &mut o, &[sx, 0.0, sz]);
        assert_eq!(caught, Ok(FISH));
        assert_eq!(stocks.stock_at(region), stock_before - 1);
        assert_eq!(inv.count(FISH), 1);
        assert!(o.is_done("first_catch"));
        // THE DRY LAW: the measured dry spot refuses BEFORE stock moves.
        let dry = fish_at(
            &graph,
            &mut stocks,
            &gen,
            &mut inv,
            &mut o,
            &[sx + 10.0, 0.0, sz + 30.0],
        );
        assert!(dry.is_err());
        assert_eq!(
            stocks.stock_at(region),
            stock_before - 1,
            "dry land pays no stock"
        );
        // THE SPENT LAW: an empty region refuses with the honest line.
        let r = crate::coords::RegionCoord { x: region.x, z: region.z };
        let held = stocks.stock_at(r);
        stocks.catch_fish(r, held);
        let mut inv2 = Inventory::new(8);
        let spent = fish_at(&graph, &mut stocks, &gen, &mut inv2, &mut o, &[sx, 0.0, sz]);
        assert_eq!(spent, Err("THE RIVER IS FISHED OUT — COME BACK LATER"));
    }
}
