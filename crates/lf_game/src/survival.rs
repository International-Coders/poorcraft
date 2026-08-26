use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerStats {
    pub health: f32,
    pub max_health: f32,
    pub hunger: f32,
    pub max_hunger: f32,
    pub saturation: f32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            health: 20.0,
            max_health: 20.0,
            hunger: 20.0,
            max_hunger: 20.0,
            saturation: 5.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ItemStack {
    pub item_id: String,
    pub count: u8,
}

#[derive(Clone, Debug)]
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>, // 36 main/hotbar + 4 armor + 1 offhand = 41
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            slots: vec![None; 41],
        }
    }

    pub fn add_item(&mut self, item_id: &str, count: u8) -> u8 {
        let cap = crate::items::item_def(item_id).map(|d| d.max_stack).unwrap_or(64);
        let mut remaining = count;
        // First pass: fill existing stacks
        for slot in self.slots.iter_mut().take(36) {
            if remaining == 0 { break; }
            if let Some(stack) = slot {
                if stack.item_id == item_id && stack.count < cap {
                    let space = cap - stack.count;
                    let add = remaining.min(space);
                    stack.count += add;
                    remaining -= add;
                }
            }
        }
        // Second pass: fill empty slots
        if remaining > 0 {
            for slot in self.slots.iter_mut().take(36) {
                if remaining == 0 { break; }
                if slot.is_none() {
                    let add = remaining.min(cap);
                    *slot = Some(ItemStack { item_id: item_id.to_string(), count: add });
                    remaining -= add;
                }
            }
        }
        remaining
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_stats() {
        let stats = PlayerStats::default();
        assert_eq!(stats.health, 20.0);
        assert_eq!(stats.hunger, 20.0);
    }

    #[test]
    fn test_inventory_stacking() {
        let mut inv = Inventory::new();
        let rem = inv.add_item("wood", 70);
        assert_eq!(rem, 0); // 64 + 6 = 70, fully placed
        assert_eq!(inv.slots[0].as_ref().unwrap().count, 64);
        assert_eq!(inv.slots[1].as_ref().unwrap().count, 6);
    }

    #[test]
    fn add_item_respects_max_stack() {
        let mut inv = Inventory::new();
        // iron_gear stacks to 16, not 64
        let rem = inv.add_item("iron_gear", 40);
        assert_eq!(rem, 0, "40 gears = 16 + 16 + 8 across three slots");
        assert_eq!(inv.slots[0].as_ref().unwrap().count, 16);
        assert_eq!(inv.slots[2].as_ref().unwrap().count, 8);
        // tools never stack
        let rem = inv.add_item("iron_pickaxe", 2);
        assert_eq!(rem, 0);
        assert_eq!(inv.slots[3].as_ref().unwrap().count, 1);
        assert_eq!(inv.slots[4].as_ref().unwrap().count, 1);
    }
}
