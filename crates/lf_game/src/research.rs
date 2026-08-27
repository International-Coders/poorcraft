//! Research eras: material costs unlock recipe/machine tiers.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Era {
    Primitive,
    Bronze,
    Industrial,
    Electrical,
    /// Branch eras (V1REBRAND doc 03): unlockable in any order relative to
    /// each other once Industrial is reached — a player can rush coal
    /// generators exactly as before, or bootstrap on rivers/boilers instead.
    Water,
    Steam,
}

impl Era {
    pub fn name(self) -> &'static str {
        match self {
            Era::Primitive => "Primitive",
            Era::Bronze => "Bronze Age",
            Era::Industrial => "Industrial Age",
            Era::Electrical => "Electrical Age",
            Era::Water => "Water Age",
            Era::Steam => "Steam Age",
        }
    }

    pub fn next(self) -> Option<Era> {
        match self {
            Era::Primitive => Some(Era::Bronze),
            Era::Bronze => Some(Era::Industrial),
            Era::Industrial => Some(Era::Electrical),
            _ => None,
        }
    }

    /// True when this is a branch era (unlocked via the graph, not the
    /// mainline chain).
    pub fn is_branch(self) -> bool {
        matches!(self, Era::Water | Era::Steam)
    }

    /// Era prerequisites (the graph edges; mainline chain + branches).
    pub fn prereqs(self) -> &'static [Era] {
        match self {
            Era::Bronze => &[Era::Primitive],
            Era::Industrial => &[Era::Bronze],
            Era::Electrical | Era::Water | Era::Steam => &[Era::Industrial],
            Era::Primitive => &[],
        }
    }

    /// Every era card the tech tree shows, in display order.
    pub const CARDS: [Era; 6] = [Era::Primitive, Era::Bronze, Era::Industrial, Era::Electrical, Era::Water, Era::Steam];

    /// Materials to advance TO this era (item, count).
    pub fn cost(self) -> &'static [(&'static str, u8)] {
        match self {
            Era::Bronze => &[("copper_ingot", 10), ("tin_ingot", 5)],
            Era::Industrial => &[("steel_ingot", 5), ("iron_gear", 3), ("coal", 20)],
            Era::Electrical => &[("basic_circuit", 5), ("aluminum_ingot", 5), ("machine_frame", 2)],
            // Water Age (doc 04): cheap, early, buildable — planks + stone +
            // a little iron for axles; the river does the rest for free.
            Era::Water => &[("planks", 16), ("stone", 24), ("iron_ingot", 4)],
            // Steam Age (doc 04): the boiler-room commitment — iron + gears
            Era::Steam => &[("iron_ingot", 12), ("iron_gear", 4), ("coal", 16)],
            Era::Primitive => &[],
        }
    }

    /// The minimum era required to craft an item (gating table).
    pub fn required_for(item_id: &str) -> Era {
        match item_id {
            "coal_generator" | "crusher" | "assembler" | "research_bench" => Era::Industrial,
            "electric_furnace" => Era::Electrical,
            "water_wheel" | "battery" => Era::Water,
            "pipe" | "boiler" | "steam_engine" => Era::Steam,
            "steel_chestplate" | "bronze_chestplate" => Era::Bronze,
            "basic_circuit" | "machine_frame" => Era::Industrial,
            _ => Era::Primitive,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResearchState {
    /// Highest MAINLINE era (Primitive..Electrical) — kept for save
    /// compatibility; branch eras live in `branches`.
    pub era: Era,
    /// Unlocked branch eras (Water, later Steam/…) — serde-defaulted so
    /// pre-branch saves load unchanged.
    #[serde(default)]
    pub branches: Vec<Era>,
}

impl Default for ResearchState {
    fn default() -> Self {
        Self { era: Era::Primitive, branches: Vec::new() }
    }
}

impl ResearchState {
    /// Is this era's content available?
    pub fn unlocked(&self, e: Era) -> bool {
        if e.is_branch() {
            self.branches.contains(&e)
        } else {
            e <= self.era
        }
    }

    /// Can the player unlock this era right now (prereqs met, not owned)?
    pub fn can_unlock(&self, e: Era) -> bool {
        !self.unlocked(e) && e.prereqs().iter().all(|p| self.unlocked(*p))
    }

    /// Count how many of each cost item the inventory holds.
    pub fn have_counts(slots: &[Option<crate::survival::ItemStack>]) -> Vec<(String, u16)> {
        let mut out: Vec<(String, u16)> = Vec::new();
        for slot in slots.iter().flatten() {
            match out.iter_mut().find(|(id, _)| *id == slot.item_id) {
                Some((_, n)) => *n += slot.count as u16,
                None => out.push((slot.item_id.clone(), slot.count as u16)),
            }
        }
        out
    }

    /// Try to advance to the next era, consuming materials. Returns the new
    /// era on success.
    pub fn advance(&mut self, slots: &mut Vec<Option<crate::survival::ItemStack>>) -> Option<Era> {
        let next = self.era.next()?;
        let mut need: Vec<(String, u16)> = next.cost().iter()
            .map(|(id, n)| (id.to_string(), *n as u16))
            .collect();
        // check availability
        let have = Self::have_counts(slots);
        for (id, n) in &need {
            let got = have.iter().find(|(h, _)| h == id).map(|(_, c)| *c).unwrap_or(0);
            if got < *n {
                return None;
            }
        }
        // consume
        for (id, n) in need.drain(..) {
            let mut left = n;
            for slot in slots.iter_mut() {
                if let Some(stack) = slot {
                    if stack.item_id == id {
                        let take = (stack.count as u16).min(left);
                        stack.count -= take as u8;
                        left -= take;
                        if stack.count == 0 {
                            *slot = None;
                        }
                        if left == 0 { break; }
                    }
                }
            }
        }
        self.era = next;
        Some(next)
    }

    /// Unlock a branch era (Water, later Steam), consuming its cost.
    /// Mainline eras use advance().
    pub fn unlock(&mut self, target: Era, slots: &mut Vec<Option<crate::survival::ItemStack>>) -> Option<Era> {
        if !target.is_branch() || !self.can_unlock(target) {
            return None;
        }
        let have = Self::have_counts(slots);
        for (id, n) in target.cost() {
            let got = have.iter().find(|(h, _)| h == id).map(|(_, c)| *c).unwrap_or(0);
            if got < *n as u16 {
                return None;
            }
        }
        for (id, n) in target.cost() {
            let mut left = *n as u16;
            for slot in slots.iter_mut() {
                if let Some(stack) = slot {
                    if stack.item_id == *id {
                        let take = (stack.count as u16).min(left);
                        stack.count -= take as u8;
                        left -= take;
                        if stack.count == 0 {
                            *slot = None;
                        }
                        if left == 0 { break; }
                    }
                }
            }
        }
        self.branches.push(target);
        Some(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::survival::ItemStack;

    fn slots(pairs: &[(&str, u8)]) -> Vec<Option<ItemStack>> {
        pairs.iter().map(|(id, n)| Some(ItemStack { item_id: id.to_string(), count: *n })).collect()
    }

    #[test]
    fn era_chain_gates_items() {
        assert!(Era::Primitive < Era::Bronze);
        assert_eq!(Era::required_for("assembler"), Era::Industrial);
        assert_eq!(Era::required_for("wooden_pickaxe"), Era::Primitive);
        assert_eq!(Era::Electrical.next(), None);
    }

    #[test]
    fn advance_consumes_materials() {
        let mut r = ResearchState::default();
        let mut inv = slots(&[("copper_ingot", 12), ("tin_ingot", 5), ("stone", 30)]);
        assert_eq!(r.advance(&mut inv), Some(Era::Bronze));
        assert_eq!(r.era, Era::Bronze);
        // copper 12-10=2 left, tin 0
        let have = ResearchState::have_counts(&inv);
        assert_eq!(have.iter().find(|(id, _)| id == "copper_ingot").map(|(_, n)| *n), Some(2));
        assert!(have.iter().find(|(id, _)| id == "tin_ingot").is_none());
        // industrial needs steel we don't have
        assert_eq!(r.advance(&mut inv), None);
        assert_eq!(r.era, Era::Bronze);
    }

    /// Doc 03: Water is a branch — it must require Industrial (not the
    /// mainline chain) and be independent of Electrical.
    #[test]
    fn branch_eras_unlock_via_the_graph() {
        let mut r = ResearchState::default();
        assert!(!r.can_unlock(Era::Water), "no water wheels before Industrial");
        assert!(!r.unlocked(Era::Water));
        r.era = Era::Industrial;
        assert!(r.can_unlock(Era::Water), "Industrial opens the branch");
        assert_eq!(Era::required_for("water_wheel"), Era::Water);
        assert_eq!(Era::required_for("battery"), Era::Water);
        let mut inv = slots(&[("planks", 16), ("stone", 24), ("iron_ingot", 4)]);
        assert_eq!(r.unlock(Era::Water, &mut inv), Some(Era::Water));
        assert!(r.unlocked(Era::Water));
        assert!(!r.unlocked(Era::Electrical), "the branch does NOT grant mainline eras");
        // double unlock is a no-op
        let mut inv2 = slots(&[("planks", 16), ("stone", 24), ("iron_ingot", 4)]);
        assert_eq!(r.unlock(Era::Water, &mut inv2), None);
        // materials consumed
        assert!(ResearchState::have_counts(&inv).iter().find(|(id, _)| id == "planks").is_none());
    }

    #[test]
    fn branch_unlock_needs_its_materials() {
        let mut r = ResearchState { era: Era::Industrial, branches: vec![] };
        let mut inv = slots(&[("planks", 3), ("stone", 24), ("iron_ingot", 4)]);
        assert_eq!(r.unlock(Era::Water, &mut inv), None);
        assert!(!r.unlocked(Era::Water));
    }

    /// Doc 03: Water and Steam are independent branches — either order.
    #[test]
    fn steam_unlocks_independently_of_water() {
        let mut r = ResearchState { era: Era::Industrial, branches: vec![Era::Water] };
        assert!(r.can_unlock(Era::Steam), "Steam does not require Water");
        let mut inv = slots(&[("iron_ingot", 12), ("iron_gear", 4), ("coal", 16)]);
        assert_eq!(r.unlock(Era::Steam, &mut inv), Some(Era::Steam));
        assert_eq!(Era::required_for("boiler"), Era::Steam);
        assert_eq!(Era::required_for("steam_engine"), Era::Steam);
        assert_eq!(Era::required_for("pipe"), Era::Steam);
        // and the reverse order works from scratch
        let mut r2 = ResearchState { era: Era::Industrial, branches: vec![Era::Steam] };
        assert!(r2.can_unlock(Era::Water));
    }

    /// Old saves (no branches field) deserialize with branches empty.
    #[test]
    fn pre_branch_saves_load() {
        let bytes = bincode::serialize(&ResearchState { era: Era::Bronze, branches: vec![] }).unwrap();
        let old: ResearchState = bincode::deserialize(&bytes).unwrap();
        assert_eq!(old.era, Era::Bronze);
        assert!(!old.unlocked(Era::Water));
    }

    #[test]
    fn insufficient_materials_rejected() {
        let mut r = ResearchState::default();
        let mut inv = slots(&[("copper_ingot", 3)]);
        assert_eq!(r.advance(&mut inv), None);
        assert_eq!(r.era, Era::Primitive);
    }
}
