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
        let mut remaining = count;
        // First pass: fill existing stacks
        for slot in self.slots.iter_mut().take(36) {
            if remaining == 0 { break; }
            if let Some(stack) = slot {
                if stack.item_id == item_id && stack.count < 64 {
                    let space = 64 - stack.count;
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
                    let add = remaining.min(64);
                    *slot = Some(ItemStack { item_id: item_id.to_string(), count: add });
                    remaining -= add;
                }
            }
        }
        remaining
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CraftingRecipe {
    pub input_pattern: Vec<String>, // simplified flat list or recipe key
    pub output_item: String,
    pub output_count: u8,
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
}
