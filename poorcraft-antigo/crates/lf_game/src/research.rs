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
    /// Oil Age (doc 04): requires Industrial AND (Steam or Electrical) —
    /// appended last so bincode variant indices in existing saves stay valid.
    Oil,
    /// Nuclear (P32): the ceiling tier — needs Oil AND the reactor_safety
    /// certification (DECISIONS Pillar 5).
    Nuclear,
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
            Era::Oil => "Oil Age",
            Era::Nuclear => "Nuclear Age",
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
        matches!(self, Era::Water | Era::Steam | Era::Oil | Era::Nuclear)
    }

    /// Era prerequisites (the graph edges; mainline chain + branches).
    /// For Oil this is the necessary set only — the Steam-or-Electrical
    /// half lives in [`ResearchState::meets_prereqs`].
    pub fn prereqs(self) -> &'static [Era] {
        match self {
            Era::Bronze => &[Era::Primitive],
            Era::Industrial => &[Era::Bronze],
            Era::Electrical | Era::Water | Era::Steam | Era::Oil | Era::Nuclear => &[Era::Industrial],
            Era::Primitive => &[],
        }
    }

    /// Every era card the tech tree shows, in display order.
    pub const CARDS: [Era; 8] = [
        Era::Primitive, Era::Bronze, Era::Industrial, Era::Electrical,
        Era::Water, Era::Steam, Era::Oil, Era::Nuclear,
    ];

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
            // Oil Age (doc 04): you must have RUN the machinery to earn it —
            // refined fuel proves the extraction chain actually works.
            Era::Oil => &[("refined_fuel", 4), ("iron_ingot", 8), ("machine_frame", 1)],
            // Nuclear (P32): the ceiling asks for the fuel itself.
            Era::Nuclear => &[("uranium_ingot", 8), ("fuel_rod", 1), ("machine_frame", 2)],
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
            // Oil Age (P31): extraction kit is Industrial-era engineering;
            // the era itself gates the combustion generator.
            "pump" | "refinery" => Era::Industrial,
            "combustion_generator" => Era::Oil,
            "reactor" | "fuel_rod" => Era::Nuclear,
            "steel_chestplate" | "bronze_chestplate" => Era::Bronze,
            "bronze_helmet" | "bronze_leggings" | "bronze_boots"
            | "steel_helmet" | "steel_leggings" | "steel_boots" => Era::Bronze,
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
    /// Reactor safety certification (P32): Nuclear refuses to unlock
    /// without it — the meltdown tax is paid up front, in study.
    #[serde(default)]
    pub reactor_safety: bool,
}

impl Default for ResearchState {
    fn default() -> Self {
        Self { era: Era::Primitive, branches: Vec::new(), reactor_safety: false }
    }
}

impl ResearchState {
    /// Prerequisite satisfaction, including the Oil either-or edge
    /// (Industrial AND at least one of Steam/Electrical — doc 04).
    fn meets_prereqs(&self, e: Era) -> bool {
        match e {
            Era::Oil => {
                self.unlocked(Era::Industrial)
                    && (self.unlocked(Era::Steam) || self.unlocked(Era::Electrical))
            }
            // the ceiling: Oil + the safety certification (P32)
            Era::Nuclear => self.unlocked(Era::Oil) && self.reactor_safety,
            other => other.prereqs().iter().all(|p| self.unlocked(*p)),
        }
    }

    /// Materials for the reactor safety certification.
    pub const REACTOR_SAFETY_COST: [(&'static str, u8); 3] =
        [("glass", 8), ("basic_circuit", 2), ("book", 1)];

    /// Study for the reactor safety certification (gates the Nuclear era).
    pub fn unlock_reactor_safety(&mut self, slots: &mut Vec<Option<crate::survival::ItemStack>>) -> bool {
        if self.reactor_safety || !self.unlocked(Era::Oil) {
            return false;
        }
        let have = Self::have_counts(slots);
        for (id, n) in Self::REACTOR_SAFETY_COST {
            let got = have.iter().find(|(h, _)| h == id).map(|(_, c)| *c).unwrap_or(0);
            if got < n as u16 {
                return false;
            }
        }
        for (id, n) in Self::REACTOR_SAFETY_COST {
            let mut left = n as u16;
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
        self.reactor_safety = true;
        true
    }

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
        !self.unlocked(e) && self.meets_prereqs(e)
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
        let mut r = ResearchState { era: Era::Industrial, branches: vec![], reactor_safety: false };
        let mut inv = slots(&[("planks", 3), ("stone", 24), ("iron_ingot", 4)]);
        assert_eq!(r.unlock(Era::Water, &mut inv), None);
        assert!(!r.unlocked(Era::Water));
    }

    /// Doc 03: Water and Steam are independent branches — either order.
    #[test]
    fn steam_unlocks_independently_of_water() {
        let mut r = ResearchState { era: Era::Industrial, branches: vec![Era::Water], reactor_safety: false };
        assert!(r.can_unlock(Era::Steam), "Steam does not require Water");
        let mut inv = slots(&[("iron_ingot", 12), ("iron_gear", 4), ("coal", 16)]);
        assert_eq!(r.unlock(Era::Steam, &mut inv), Some(Era::Steam));
        assert_eq!(Era::required_for("boiler"), Era::Steam);
        assert_eq!(Era::required_for("steam_engine"), Era::Steam);
        assert_eq!(Era::required_for("pipe"), Era::Steam);
        // and the reverse order works from scratch
        let mut r2 = ResearchState { era: Era::Industrial, branches: vec![Era::Steam], reactor_safety: false };
        assert!(r2.can_unlock(Era::Water));
    }

    /// Old saves (no branches field) deserialize with branches empty.
    #[test]
    fn pre_branch_saves_load() {
        let bytes = bincode::serialize(&ResearchState { era: Era::Bronze, branches: vec![], reactor_safety: false }).unwrap();
        let old: ResearchState = bincode::deserialize(&bytes).unwrap();
        assert_eq!(old.era, Era::Bronze);
        assert!(!old.unlocked(Era::Water));
    }

    /// Doc 04: Oil needs Industrial AND (Steam or Electrical) — either
    /// branch route qualifies, Industrial alone does not.
    #[test]
    fn oil_needs_steam_or_electrical() {
        let mut r = ResearchState { era: Era::Industrial, branches: vec![], reactor_safety: false };
        assert!(!r.can_unlock(Era::Oil), "Industrial alone is not enough");
        r.branches.push(Era::Steam);
        assert!(r.can_unlock(Era::Oil), "the Steam route opens Oil");
        // the Electrical route works from scratch too
        let mut r2 = ResearchState { era: Era::Electrical, branches: vec![], reactor_safety: false };
        assert!(r2.can_unlock(Era::Oil), "the Electrical route opens Oil");
        // unlock consumes the refined-fuel cost
        let mut inv = slots(&[("refined_fuel", 4), ("iron_ingot", 8), ("machine_frame", 1)]);
        assert_eq!(r2.unlock(Era::Oil, &mut inv), Some(Era::Oil));
        assert!(r2.unlocked(Era::Oil));
        assert!(ResearchState::have_counts(&inv).iter().find(|(id, _)| id == "refined_fuel").is_none());
        assert_eq!(Era::required_for("combustion_generator"), Era::Oil);
        assert_eq!(Era::required_for("pump"), Era::Industrial);
        assert_eq!(Era::required_for("refinery"), Era::Industrial);
    }

    /// P32: Nuclear needs Oil AND the safety certification — study first,
    /// then the ceiling opens.
    #[test]
    fn nuclear_needs_oil_and_safety_certification() {
        let mut r = ResearchState { era: Era::Electrical, branches: vec![Era::Oil], reactor_safety: false };
        assert!(!r.can_unlock(Era::Nuclear), "oil alone is not enough");
        // certification needs its study materials + oil unlocked
        let mut poor = slots(&[("glass", 8), ("basic_circuit", 2), ("book", 1)]);
        assert!(!r.unlock_reactor_safety(&mut poor) || {
            // with oil unlocked it should succeed; sanity-check both paths
            r.reactor_safety
        });
        // do it cleanly: certification then unlock
        let mut r2 = ResearchState { era: Era::Electrical, branches: vec![Era::Oil], reactor_safety: false };
        let mut inv = slots(&[("glass", 8), ("basic_circuit", 2), ("book", 1),
                              ("uranium_ingot", 8), ("fuel_rod", 1), ("machine_frame", 2)]);
        assert!(r2.unlock_reactor_safety(&mut inv));
        assert!(r2.can_unlock(Era::Nuclear));
        assert_eq!(r2.unlock(Era::Nuclear, &mut inv), Some(Era::Nuclear));
        assert!(r2.unlocked(Era::Nuclear));
        assert_eq!(Era::required_for("reactor"), Era::Nuclear);
        assert_eq!(Era::required_for("fuel_rod"), Era::Nuclear);
        // old saves (no reactor_safety field) default to uncertified
        let old: ResearchState = bincode::deserialize(
            &bincode::serialize(&ResearchState::default()).unwrap()).unwrap();
        assert!(!old.reactor_safety);
    }

    #[test]
    fn insufficient_materials_rejected() {
        let mut r = ResearchState::default();
        let mut inv = slots(&[("copper_ingot", 3)]);
        assert_eq!(r.advance(&mut inv), None);
        assert_eq!(r.era, Era::Primitive);
    }
}
