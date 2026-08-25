//! Research eras: material costs unlock recipe/machine tiers.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Era {
    Primitive,
    Bronze,
    Industrial,
    Electrical,
}

impl Era {
    pub fn name(self) -> &'static str {
        match self {
            Era::Primitive => "Primitive",
            Era::Bronze => "Bronze Age",
            Era::Industrial => "Industrial Age",
            Era::Electrical => "Electrical Age",
        }
    }

    pub fn next(self) -> Option<Era> {
        match self {
            Era::Primitive => Some(Era::Bronze),
            Era::Bronze => Some(Era::Industrial),
            Era::Industrial => Some(Era::Electrical),
            Era::Electrical => None,
        }
    }

    /// Materials to advance TO this era (item, count).
    pub fn cost(self) -> &'static [(&'static str, u8)] {
        match self {
            Era::Bronze => &[("copper_ingot", 10), ("tin_ingot", 5)],
            Era::Industrial => &[("steel_ingot", 5), ("iron_gear", 3), ("coal", 20)],
            Era::Electrical => &[("basic_circuit", 5), ("aluminum_ingot", 5), ("machine_frame", 2)],
            Era::Primitive => &[],
        }
    }

    /// The minimum era required to craft an item (gating table).
    pub fn required_for(item_id: &str) -> Era {
        match item_id {
            "coal_generator" | "crusher" | "assembler" | "research_bench" => Era::Industrial,
            "electric_furnace" => Era::Electrical,
            "steel_chestplate" | "bronze_chestplate" => Era::Bronze,
            "basic_circuit" | "machine_frame" => Era::Industrial,
            _ => Era::Primitive,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResearchState {
    pub era: Era,
}

impl Default for ResearchState {
    fn default() -> Self {
        Self { era: Era::Primitive }
    }
}

impl ResearchState {
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

    #[test]
    fn insufficient_materials_rejected() {
        let mut r = ResearchState::default();
        let mut inv = slots(&[("copper_ingot", 3)]);
        assert_eq!(r.advance(&mut inv), None);
        assert_eq!(r.era, Era::Primitive);
    }
}
