//! Item registry: every obtainable item, its kind, stack size, and tool stats.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolKind {
    Pickaxe,
    Axe,
    Shovel,
    Sword,
    Bow,
}

impl ToolKind {
    /// Blocks this tool speeds up (see lf_voxel::registry for block ids).
    pub fn effective_on(self) -> &'static [u32] {
        use lf_voxel::registry::block;
        match self {
            ToolKind::Pickaxe => &[block::STONE, block::COAL_ORE, block::IRON_ORE],
            ToolKind::Axe => &[block::LOG, block::PLANKS, block::CRAFTING_TABLE, block::CHEST],
            ToolKind::Shovel => &[block::DIRT, block::GRASS, block::SAND, block::SNOW, block::MYCELIUM],
            ToolKind::Sword => &[block::LEAVES],
            ToolKind::Bow => &[],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    /// Places the given block id.
    Block(u32),
    /// Armor with flat damage-reduction points.
    Armor(u8),
    /// Tool with a kind and tier (0 wood, 1 stone, 2 iron).
    Tool(ToolKind, u8),
    /// Restores hunger points when eaten.
    Food(u8),
    Material,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemDef {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: ItemKind,
    pub max_stack: u8,
}

/// Mining speed multiplier and durability by tool tier.
pub fn tier_speed(tier: u8) -> f32 {
    match tier {
        0 => 2.0,  // wood
        1 => 4.0,  // stone
        2 => 6.0,  // iron
        _ => 8.0,
    }
}

pub fn tier_durability(tier: u8) -> u32 {
    match tier {
        0 => 60,
        1 => 132,
        2 => 250,
        _ => 500,
    }
}

/// The full item table. Block items map 1:1 to placeable blocks.
pub fn items() -> &'static [ItemDef] {
    use lf_voxel::registry::block;
    static ITEMS: &[ItemDef] = &[
        // block items
        ItemDef { id: "grass", name: "Grass Block", kind: ItemKind::Block(block::GRASS), max_stack: 64 },
        ItemDef { id: "dirt", name: "Dirt", kind: ItemKind::Block(block::DIRT), max_stack: 64 },
        ItemDef { id: "stone", name: "Stone", kind: ItemKind::Block(block::STONE), max_stack: 64 },
        ItemDef { id: "sand", name: "Sand", kind: ItemKind::Block(block::SAND), max_stack: 64 },
        ItemDef { id: "mycelium", name: "Mycelium", kind: ItemKind::Block(block::MYCELIUM), max_stack: 64 },
        ItemDef { id: "snow", name: "Snow", kind: ItemKind::Block(block::SNOW), max_stack: 64 },
        ItemDef { id: "log", name: "Oak Log", kind: ItemKind::Block(block::LOG), max_stack: 64 },
        ItemDef { id: "leaves", name: "Oak Leaves", kind: ItemKind::Block(block::LEAVES), max_stack: 64 },
        ItemDef { id: "torch", name: "Torch", kind: ItemKind::Block(block::TORCH), max_stack: 64 },
        ItemDef { id: "crafting_table", name: "Crafting Table", kind: ItemKind::Block(block::CRAFTING_TABLE), max_stack: 64 },
        ItemDef { id: "furnace", name: "Furnace", kind: ItemKind::Block(block::FURNACE), max_stack: 64 },
        ItemDef { id: "chest", name: "Chest", kind: ItemKind::Block(block::CHEST), max_stack: 64 },
        ItemDef { id: "planks", name: "Oak Planks", kind: ItemKind::Block(block::PLANKS), max_stack: 64 },
        ItemDef { id: "glass", name: "Glass", kind: ItemKind::Block(block::GLASS), max_stack: 64 },
        // materials
        ItemDef { id: "stick", name: "Stick", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "coal", name: "Coal", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "raw_iron", name: "Raw Iron", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "iron_ingot", name: "Iron Ingot", kind: ItemKind::Material, max_stack: 64 },
        // tools (tier 0 wood, 1 stone, 2 iron)
        ItemDef { id: "wooden_pickaxe", name: "Wooden Pickaxe", kind: ItemKind::Tool(ToolKind::Pickaxe, 0), max_stack: 1 },
        ItemDef { id: "stone_pickaxe", name: "Stone Pickaxe", kind: ItemKind::Tool(ToolKind::Pickaxe, 1), max_stack: 1 },
        ItemDef { id: "iron_pickaxe", name: "Iron Pickaxe", kind: ItemKind::Tool(ToolKind::Pickaxe, 2), max_stack: 1 },
        ItemDef { id: "wooden_axe", name: "Wooden Axe", kind: ItemKind::Tool(ToolKind::Axe, 0), max_stack: 1 },
        ItemDef { id: "stone_axe", name: "Stone Axe", kind: ItemKind::Tool(ToolKind::Axe, 1), max_stack: 1 },
        ItemDef { id: "iron_axe", name: "Iron Axe", kind: ItemKind::Tool(ToolKind::Axe, 2), max_stack: 1 },
        ItemDef { id: "wooden_shovel", name: "Wooden Shovel", kind: ItemKind::Tool(ToolKind::Shovel, 0), max_stack: 1 },
        ItemDef { id: "stone_shovel", name: "Stone Shovel", kind: ItemKind::Tool(ToolKind::Shovel, 1), max_stack: 1 },
        ItemDef { id: "iron_shovel", name: "Iron Shovel", kind: ItemKind::Tool(ToolKind::Shovel, 2), max_stack: 1 },
        ItemDef { id: "wooden_sword", name: "Wooden Sword", kind: ItemKind::Tool(ToolKind::Sword, 0), max_stack: 1 },
        ItemDef { id: "stone_sword", name: "Stone Sword", kind: ItemKind::Tool(ToolKind::Sword, 1), max_stack: 1 },
        ItemDef { id: "iron_sword", name: "Iron Sword", kind: ItemKind::Tool(ToolKind::Sword, 2), max_stack: 1 },
        // food
        ItemDef { id: "apple", name: "Apple", kind: ItemKind::Food(4), max_stack: 64 },
        ItemDef { id: "porkchop", name: "Porkchop", kind: ItemKind::Food(8), max_stack: 64 },
        ItemDef { id: "mutton", name: "Mutton", kind: ItemKind::Food(6), max_stack: 64 },
        ItemDef { id: "book", name: "Lore Book", kind: ItemKind::Material, max_stack: 16 },
        ItemDef { id: "bow", name: "Bow", kind: ItemKind::Tool(ToolKind::Bow, 1), max_stack: 1 },
        ItemDef { id: "arrow", name: "Arrow", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "smithing_table", name: "Smithing Table", kind: ItemKind::Block(36), max_stack: 8 },
        ItemDef { id: "bronze_chestplate", name: "Bronze Chestplate", kind: ItemKind::Armor(4), max_stack: 1 },
        ItemDef { id: "steel_chestplate", name: "Steel Chestplate", kind: ItemKind::Armor(7), max_stack: 1 },
        // industrial materials (ores land with P15 worldgen)
        ItemDef { id: "raw_copper", name: "Raw Copper", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "copper_ingot", name: "Copper Ingot", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "raw_tin", name: "Raw Tin", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "tin_ingot", name: "Tin Ingot", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "aluminum_ingot", name: "Aluminum Ingot", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "sulfur", name: "Sulfur", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "bronze_ingot", name: "Bronze Ingot", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "steel_ingot", name: "Steel Ingot", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "copper_wire", name: "Copper Wire", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "iron_gear", name: "Iron Gear", kind: ItemKind::Material, max_stack: 16 },
        ItemDef { id: "machine_frame", name: "Machine Frame", kind: ItemKind::Material, max_stack: 16 },
        ItemDef { id: "basic_circuit", name: "Basic Circuit", kind: ItemKind::Material, max_stack: 32 },
        // mob materials (P8 quest hooks)
        ItemDef { id: "glitch_dust", name: "Glitch Dust", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "null_shard", name: "Null Shard", kind: ItemKind::Material, max_stack: 16 },
    ];
    ITEMS
}

/// Attack damage for a held tool (hearts); hands do 1.
pub fn tool_damage(kind: ToolKind, tier: u8) -> f32 {
    let base = match kind {
        ToolKind::Sword => 4.0,
        ToolKind::Bow => 5.0,
        ToolKind::Axe => 3.0,
        ToolKind::Pickaxe => 2.0,
        ToolKind::Shovel => 1.5,
    };
    base + tier as f32
}

pub fn item_def(id: &str) -> Option<ItemDef> {
    if let Some(def) = items().iter().find(|i| i.id == id) {
        return Some(*def);
    }
    mod_item(id)
}

// --- runtime mod items ---
fn mod_items() -> &'static std::sync::RwLock<Vec<ItemDef>> {
    static ITEMS: std::sync::OnceLock<std::sync::RwLock<Vec<ItemDef>>> = std::sync::OnceLock::new();
    ITEMS.get_or_init(|| std::sync::RwLock::new(Vec::new()))
}

/// Register a mod item (namespaced ids like "ember_ores:ember_ingot").
/// Ids are leaked into the bounded mod namespace set.
pub fn register_mod_item(id: String, name: String, kind: ItemKind, max_stack: u8) -> bool {
    let mut items = mod_items().write().unwrap();
    if items.iter().any(|i| i.id == id.as_str()) {
        return true; // idempotent reload
    }
    items.push(ItemDef {
        id: Box::leak(id.into_boxed_str()),
        name: Box::leak(name.into_boxed_str()),
        kind,
        max_stack,
    });
    true
}

fn mod_item(id: &str) -> Option<ItemDef> {
    mod_items().read().unwrap().iter().find(|i| i.id == id).copied()
}

pub fn registered_mod_items() -> Vec<ItemDef> {
    mod_items().read().unwrap().clone()
}

/// The item id a broken block drops (None = nothing).
pub fn block_drop(block_id: u32) -> Option<String> {
    use lf_voxel::registry::block;
    match block_id {
        block::GRASS => Some("dirt".into()),
        block::DIRT => Some("dirt".into()),
        block::STONE => Some("stone".into()),
        block::SAND => Some("sand".into()),
        block::MYCELIUM => Some("mycelium".into()),
        block::SNOW => Some("snow".into()),
        block::LOG => Some("log".into()),
        block::LEAVES => Some("leaves".into()), // apples are a rare bonus handled by the caller
        block::TORCH => Some("torch".into()),
        block::CRAFTING_TABLE => Some("crafting_table".into()),
        block::FURNACE => Some("furnace".into()),
        block::CHEST => Some("chest".into()),
        block::PLANKS => Some("planks".into()),
        block::GLASS => None, // glass shatters
        block::COAL_ORE => Some("coal".into()),
        block::IRON_ORE => Some("raw_iron".into()),
        block::COPPER_ORE => Some("raw_copper".into()),
        block::TIN_ORE => Some("raw_tin".into()),
        block::BAUXITE_ORE => Some("sulfur".into()), // placeholder drop until P15 bauxite item
        block::SULFUR_ORE => Some("sulfur".into()),
        block::WATER | block::AIR => None,
        id if id >= lf_voxel::registry::MOD_BLOCK_BASE => {
            lf_voxel::registry::mod_block(id).and_then(|d| d.drop)
        }
        _ => Some("stone".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_unique_and_named() {
        let mut ids: Vec<&str> = items().iter().map(|i| i.id).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n, "duplicate item ids");
        assert!(items().iter().all(|i| !i.name.is_empty()));
    }

    #[test]
    fn lookups_work() {
        assert_eq!(item_def("stone").unwrap().name, "Stone");
        assert!(item_def("nonexistent").is_none());
        let pick = item_def("wooden_pickaxe").unwrap();
        match pick.kind {
            ItemKind::Tool(ToolKind::Pickaxe, 0) => {}
            other => panic!("wrong kind: {:?}", other),
        }
    }

    #[test]
    fn tools_stack_to_one_and_have_durability() {
        for id in ["wooden_pickaxe", "stone_axe", "wooden_shovel"] {
            let def = item_def(id).unwrap();
            assert_eq!(def.max_stack, 1);
            if let ItemKind::Tool(_, tier) = def.kind {
                assert!(tier_durability(tier) > 0);
            }
        }
    }

    #[test]
    fn catalog_consistency() {
        use crate::crafting::recipes;
        use crate::smelting::smelt_result;
        let valid = |id: &str| item_def(id).is_some();
        // every recipe output and ingredient is a real item
        for r in recipes() {
            assert!(valid(&r.output), "recipe output '{}' is not an item", r.output);
            for row in &r.pattern {
                for cell in row.iter().flatten() {
                    assert!(valid(cell), "recipe ingredient '{}' is not an item", cell);
                }
            }
        }
        // every smelt output is a real item
        for input in ["raw_iron", "sand", "stone", "log"] {
            if let Some(out) = smelt_result(input) {
                assert!(valid(out), "smelt output '{}' is not an item", out);
            }
        }
        // every block drop is a real item
        for block_id in 0..=18u32 {
            if let Some(drop) = block_drop(block_id) {
                assert!(valid(&drop), "drop '{}' for block {} is not an item", drop, block_id);
            }
        }
        // every placeable block item maps to a named block
        for def in items() {
            if let ItemKind::Block(b) = def.kind {
                assert_ne!(lf_voxel::registry::block::name(b), "Unknown",
                    "item '{}' places unknown block {}", def.id, b);
            }
        }
        // tool tiers have stats
        for def in items() {
            if let ItemKind::Tool(_, tier) = def.kind {
                assert!(tier_durability(tier) > 0);
                assert!(super::tier_speed(tier) > 1.0);
            }
        }
    }

    #[test]
    fn mod_items_register_and_resolve() {
        assert!(register_mod_item("ember_ores:ember_ingot".into(), "Ember Ingot".into(), ItemKind::Material, 64));
        assert!(register_mod_item("ember_ores:ember_ingot".into(), "Ember Ingot".into(), ItemKind::Material, 64), "idempotent");
        assert_eq!(item_def("ember_ores:ember_ingot").unwrap().name, "Ember Ingot");
        assert!(item_def("ember_ores:missing").is_none());
    }

    #[test]
    fn ores_drop_materials() {
        use lf_voxel::registry::block;
        assert_eq!(block_drop(block::COAL_ORE).as_deref(), Some("coal"));
        assert_eq!(block_drop(block::IRON_ORE).as_deref(), Some("raw_iron"));
        assert_eq!(block_drop(block::GRASS).as_deref(), Some("dirt"));
        assert_eq!(block_drop(block::WATER), None);
    }
}
