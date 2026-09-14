//! P3D-501: item authority — catalog, inventory, tools, durability,
//! harvest rules. The ONE place items are defined; UI and world both
//! read from here.

/// Stable item identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Tool { tier: u8 },
    Material,
    Food { heal: u8 },
}

/// The item catalog. Codes are stable; appending is allowed, reordering
/// breaks saves.
pub const ITEMS: &[(u16, &str, ItemKind)] = &[
    (1, "wood", ItemKind::Material),
    (2, "stone", ItemKind::Material),
    (3, "sand", ItemKind::Material),
    (4, "snow", ItemKind::Material),
    (5, "soil", ItemKind::Material),
    (6, "iron_ore", ItemKind::Material),
    (7, "iron_bar", ItemKind::Material),
    (10, "stone_pick", ItemKind::Tool { tier: 1 }),
    (11, "iron_pick", ItemKind::Tool { tier: 2 }),
    (12, "wood_pick", ItemKind::Tool { tier: 1 }),
    (20, "bread", ItemKind::Food { heal: 30 }),
    (21, "fish", ItemKind::Food { heal: 15 }),
];

pub fn item_name(id: ItemId) -> &'static str {
    ITEMS
        .iter()
        .find(|(code, _, _)| *code == id.0)
        .map(|(_, n, _)| *n)
        .expect("unknown item code — catalog broken")
}

pub fn item_kind(id: ItemId) -> ItemKind {
    ITEMS
        .iter()
        .find(|(code, _, _)| *code == id.0)
        .map(|(_, _, k)| *k)
        .expect("unknown item code — catalog broken")
}

/// A stack of identical items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemStack {
    pub item: ItemId,
    pub count: u32,
}

/// Bounded inventory: `slots` Some = occupied. Add stacks first, then
/// fills empty slots; returns the LEFTOVER that didn't fit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
    pub stack_max: u32,
}

impl Inventory {
    pub fn new(slot_count: usize) -> Self {
        Inventory {
            slots: vec![None; slot_count.max(1)],
            stack_max: 64,
        }
    }

    /// Add a stack; returns the leftover count that did not fit.
    pub fn add(&mut self, item: ItemId, mut count: u32) -> u32 {
        // 1. Top up existing stacks of the same item.
        for slot in &mut self.slots {
            if count == 0 {
                break;
            }
            match slot {
                Some(s) if s.item == item && (s.count as u32) < self.stack_max => {
                    let room = self.stack_max - s.count;
                    let take = room.min(count);
                    s.count += take;
                    count -= take;
                }
                _ => {}
            }
        }
        // 2. Fill empty slots.
        for slot in &mut self.slots {
            if count == 0 {
                break;
            }
            if slot.is_none() {
                let take = count.min(self.stack_max);
                *slot = Some(ItemStack { item, count: take });
                count -= take;
            }
        }
        count
    }

    /// Remove up to `count` of `item`; returns how many were removed.
    pub fn remove(&mut self, item: ItemId, mut count: u32) -> u32 {
        let mut removed = 0;
        for slot in &mut self.slots {
            if count == 0 {
                break;
            }
            if let Some(s) = slot {
                if s.item == item {
                    let take = s.count.min(count);
                    s.count -= take;
                    count -= take;
                    removed += take;
                    if s.count == 0 {
                        *slot = None;
                    }
                }
            }
        }
        removed
    }

    pub fn count(&self, item: ItemId) -> u32 {
        self.slots
            .iter()
            .filter_map(|s| s.as_ref())
            .filter(|s| s.item == item)
            .map(|s| s.count)
            .sum()
    }

    /// Whether `count` of `item` would fit WITHOUT displacing anything
    /// (stack tops first, then empty slots). The dig verb asks BEFORE it
    /// breaks ground: a take that can't be carried never edits the world.
    pub fn can_fit(&self, item: ItemId, count: u32) -> bool {
        let mut room = 0u32;
        for slot in &self.slots {
            match slot {
                Some(s) if s.item == item && s.count < self.stack_max => {
                    room += self.stack_max - s.count;
                }
                None => room += self.stack_max,
                _ => {}
            }
            if room >= count {
                return true;
            }
        }
        room >= count
    }
}

/// A tool in use: durability decrements per use, breaks at 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolState {
    pub item: ItemId,
    pub durability: u32,
    pub durability_max: u32,
}

impl ToolState {
    pub fn new(item: ItemId, durability_max: u32) -> Self {
        ToolState {
            item,
            durability: durability_max,
            durability_max,
        }
    }

    /// Use the tool; returns false when it just BROKE (durability hit 0).
    pub fn use_once(&mut self) -> bool {
        if self.durability == 0 {
            return false;
        }
        self.durability -= 1;
        self.durability > 0
    }
}

/// The carried-stock line: the inventory's echo as ONE stable string —
/// nonzero kinds in catalog order, "NAME COUNT", prefixed "PACK"; an
/// empty pack reads "PACK EMPTY". The HUD renders it verbatim, so the
/// dig's take (and the forge's spend, the eaten bread) is visible as a
/// number — the same answer routes and the inspector read.
pub fn stock_line(inv: &Inventory) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (code, name, _) in ITEMS {
        let n = inv.count(ItemId(*code));
        if n > 0 {
            parts.push(format!("{} {}", name.to_uppercase(), n));
        }
    }
    if parts.is_empty() {
        "PACK EMPTY".to_string()
    } else {
        format!("PACK {}", parts.join(" · "))
    }
}

/// Harvest yields for digging a terrain material with a tool tier.
/// Bare hands (None) gather Soil/Sand/Snow but not Stone; any pick takes
/// Stone. Wood yields from Soil/Grass surfaces (trees).
pub fn harvest_yields(
    material: crate::gen::CellMaterial,
    tool_tier: Option<u8>,
) -> Vec<(ItemId, u32)> {
    use crate::gen::CellMaterial as CM;
    match material {
        CM::Grass => vec![(ItemId(5), 1), (ItemId(1), 1)], // soil + wood
        CM::Soil => vec![(ItemId(5), 1)],
        CM::Sand => vec![(ItemId(3), 1)],
        CM::Snow => vec![(ItemId(4), 1)],
        CM::Rock => {
            let tier = tool_tier.unwrap_or(0);
            if tier >= 1 {
                vec![(ItemId(2), 1)]
            } else {
                Vec::new() // bare hands cannot take stone
            }
        }
        CM::Air | CM::Water => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WOOD: ItemId = ItemId(1);
    const STONE: ItemId = ItemId(2);

    /// Add stacks then fills, returning the exact leftover.
    #[test]
    fn p3d501_inventory_stacks_fills_and_reports_leftover() {
        let mut inv = Inventory::new(3);
        // 3 slots of 64 = 192 capacity: 100 wood fits entirely.
        assert_eq!(inv.add(WOOD, 100), 0);
        assert_eq!(inv.count(WOOD), 100);
        // The partial stack tops up: 36 + 5 = 41.
        assert_eq!(inv.add(WOOD, 5), 0);
        assert_eq!(inv.count(WOOD), 105);
        // Second item type fills the free slot.
        assert_eq!(inv.add(STONE, 10), 0);
        assert_eq!(inv.count(STONE), 10);
    }

    /// Remove drains across stacks in order and empties emptied slots.
    #[test]
    fn p3d501_remove_drains_across_stacks() {
        let mut inv = Inventory::new(4);
        inv.add(WOOD, 30);
        inv.add(WOOD, 30);
        assert_eq!(inv.remove(WOOD, 45), 45);
        assert_eq!(inv.count(WOOD), 15);
        assert_eq!(inv.remove(WOOD, 100), 15, "bounded by what exists");
        assert_eq!(inv.count(WOOD), 0);
    }

    /// Tools: durability decrements per use and breaks at 0.
    #[test]
    fn p3d501_tool_durability_breaks() {
        let mut tool = ToolState::new(ItemId(10), 3);
        assert!(tool.use_once());
        assert!(tool.use_once());
        assert_eq!(tool.durability, 1);
        assert!(!tool.use_once(), "the last use BREAKS the tool");
        assert_eq!(tool.durability, 0);
        assert!(!tool.use_once(), "a broken tool stays broken");
    }

    /// Harvest gating: bare hands take soil-like materials but not rock;
    /// a pick takes rock. Yields land in an inventory.
    #[test]
    fn p3d501_harvest_gates_by_tier_and_yields() {
        use crate::gen::CellMaterial;
        // Bare hands: no rock.
        assert!(harvest_yields(CellMaterial::Rock, None).is_empty());
        // Tier 1 pick: stone.
        assert_eq!(
            harvest_yields(CellMaterial::Rock, Some(1)),
            vec![(STONE, 1)]
        );
        // Hands take soil.
        assert_eq!(
            harvest_yields(CellMaterial::Soil, None),
            vec![(ItemId(5), 1)]
        );

        // Yields fit an inventory through add (leftover 0).
        let mut inv = Inventory::new(8);
        for (item, count) in harvest_yields(CellMaterial::Grass, None) {
            assert_eq!(inv.add(item, count), 0);
        }
        assert!(inv.count(ItemId(5)) > 0);
        assert!(inv.count(WOOD) > 0);
    }

    /// THE PACK LINE (the inventory's echo): nonzero kinds in catalog
    /// order as "NAME COUNT" under the "PACK" prefix, counts summed
    /// across stacks, an empty pack reading "PACK EMPTY". The HUD
    /// renders this verbatim — the dig's take visible as a number.
    #[test]
    fn p3d501_stock_line_names_nonzero_kinds_in_catalog_order() {
        const SOIL: ItemId = ItemId(5);
        let mut inv = Inventory::new(8);
        assert_eq!(stock_line(&inv), "PACK EMPTY");
        // Catalog order, not insertion order: wood (code 1) precedes
        // soil (code 5) even though soil was added first.
        inv.add(SOIL, 1);
        assert_eq!(stock_line(&inv), "PACK SOIL 1");
        inv.add(WOOD, 3);
        assert_eq!(stock_line(&inv), "PACK WOOD 3 · SOIL 1");
        inv.add(STONE, 12);
        assert_eq!(stock_line(&inv), "PACK WOOD 3 · STONE 12 · SOIL 1");
        // Counts sum ACROSS stacks: two wood slots read as one number.
        inv.add(WOOD, 2);
        assert_eq!(stock_line(&inv), "PACK WOOD 5 · STONE 12 · SOIL 1");
        // Draining a kind to zero removes its words entirely.
        inv.remove(SOIL, 1);
        assert_eq!(stock_line(&inv), "PACK WOOD 5 · STONE 12");
        inv.remove(WOOD, 5);
        inv.remove(STONE, 12);
        assert_eq!(stock_line(&inv), "PACK EMPTY");
    }

    /// The dig's take lands in the pack line: the grass yield (soil +
    /// wood) through add + stock_line is exactly what the route's HUD
    /// must show after one G press on a grass cell.
    #[test]
    fn p3d501_stock_line_carries_the_dig_take() {
        use crate::gen::CellMaterial;
        let mut inv = Inventory::new(12);
        for (item, count) in harvest_yields(CellMaterial::Grass, None) {
            assert_eq!(inv.add(item, count), 0);
        }
        assert_eq!(stock_line(&inv), "PACK WOOD 1 · SOIL 1");
    }

    /// THE DIG VERB's gate: can_fit answers BEFORE the world is edited —
    /// a take that can't be carried never breaks ground.
    #[test]
    fn p3d501_can_fit_answers_before_the_edit() {
        const SOIL: ItemId = ItemId(5);
        let mut inv = Inventory::new(2);
        assert!(inv.can_fit(SOIL, 1), "an empty pack takes a dig's yield");
        inv.add(SOIL, 64);
        inv.add(STONE, 64);
        // The pack is full of soil+stone: one more soil has no room —
        // the soil stacks are full and the other slot holds stone.
        assert!(!inv.can_fit(SOIL, 1));
        // Drain a partial stack: the freed room answers exactly.
        inv.remove(SOIL, 1);
        assert!(inv.can_fit(SOIL, 1));
        assert!(!inv.can_fit(SOIL, 2));
        // A fresh slot counts whole for a DIFFERENT item, never the
        // stone stack's room.
        assert!(!inv.can_fit(STONE, 1));
    }
}
