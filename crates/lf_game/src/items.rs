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
            ToolKind::Shovel => &[block::DIRT, block::GRASS, block::JUNGLE_GRASS, block::SAVANNA_GRASS, block::SAND, block::SNOW, block::MYCELIUM],
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
        ItemDef { id: "lantern", name: "Lantern", kind: ItemKind::Block(block::LANTERN), max_stack: 64 },
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
        // fluids
        ItemDef { id: "water_wheel", name: "Water Wheel", kind: ItemKind::Block(block::WATER_WHEEL), max_stack: 1 },
        ItemDef { id: "battery", name: "Battery", kind: ItemKind::Block(block::BATTERY), max_stack: 1 },
        // lore tomes (Step 20): readable via lore/books.toml
        ItemDef { id: "tome_of_the_forge", name: "Tome of the First Forge", kind: ItemKind::Material, max_stack: 1 },
        ItemDef { id: "tome_of_the_null", name: "Tome of the Null", kind: ItemKind::Material, max_stack: 1 },
        ItemDef { id: "wardens_ledger", name: "The River Wardens' Ledger", kind: ItemKind::Material, max_stack: 1 },
        ItemDef { id: "pipe", name: "Pipe", kind: ItemKind::Block(block::PIPE), max_stack: 16 },
        ItemDef { id: "boiler", name: "Boiler", kind: ItemKind::Block(block::BOILER), max_stack: 1 },
        ItemDef { id: "steam_engine", name: "Steam Engine", kind: ItemKind::Block(block::STEAM_ENGINE), max_stack: 1 },
        // Oil Age (P31)
        ItemDef { id: "pump", name: "Pumpjack", kind: ItemKind::Block(block::PUMP), max_stack: 1 },
        ItemDef { id: "refinery", name: "Refinery", kind: ItemKind::Block(block::REFINERY), max_stack: 1 },
        ItemDef { id: "combustion_generator", name: "Combustion Generator", kind: ItemKind::Block(block::COMBUSTION_GENERATOR), max_stack: 1 },
        ItemDef { id: "oil_bucket", name: "Bucket of Crude", kind: ItemKind::Material, max_stack: 1 },
        ItemDef { id: "refined_fuel", name: "Refined Fuel", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "tar", name: "Tar", kind: ItemKind::Material, max_stack: 64 },
        // Magic foundation (P33): scrolls teach spells (right-click)
        ItemDef { id: "scroll_of_firebolt", name: "Scroll of Firebolt", kind: ItemKind::Material, max_stack: 4 },
        ItemDef { id: "scroll_of_gale_step", name: "Scroll of Gale-step", kind: ItemKind::Material, max_stack: 4 },
        ItemDef { id: "scroll_of_ward", name: "Scroll of Ward", kind: ItemKind::Material, max_stack: 4 },
        ItemDef { id: "scroll_of_hearthlight", name: "Scroll of Hearthlight", kind: ItemKind::Material, max_stack: 4 },
        ItemDef { id: "rune_of_haste", name: "Rune of Haste", kind: ItemKind::Material, max_stack: 8 },
        ItemDef { id: "rune_of_warding", name: "Rune of Warding", kind: ItemKind::Material, max_stack: 8 },
        // Construction (P34): shaped placement via shaped_placement()
        ItemDef { id: "stone_slab", name: "Stone Slab", kind: ItemKind::Material, max_stack: 32 },
        ItemDef { id: "planks_slab", name: "Planks Slab", kind: ItemKind::Material, max_stack: 32 },
        ItemDef { id: "stone_stairs", name: "Stone Stairs", kind: ItemKind::Material, max_stack: 16 },
        ItemDef { id: "scaffold", name: "Scaffolding", kind: ItemKind::Block(block::SCAFFOLD), max_stack: 32 },
        ItemDef { id: "statue", name: "Chiseled Statue", kind: ItemKind::Block(block::STATUE), max_stack: 8 },
        ItemDef { id: "chisel", name: "Chisel", kind: ItemKind::Tool(ToolKind::Pickaxe, 1), max_stack: 1 },
        ItemDef { id: "blueprint", name: "Blueprint", kind: ItemKind::Material, max_stack: 1 },
        // Smart building (P35)
        ItemDef { id: "conduit", name: "Power Conduit", kind: ItemKind::Block(block::CONDUIT), max_stack: 32 },
        ItemDef { id: "elevator", name: "Elevator", kind: ItemKind::Block(block::ELEVATOR), max_stack: 16 },
        ItemDef { id: "ac_unit", name: "Climate Unit", kind: ItemKind::Block(block::AC_UNIT), max_stack: 8 },
        ItemDef { id: "computer", name: "Computer Screen", kind: ItemKind::Block(block::COMPUTER), max_stack: 8 },
        ItemDef { id: "dragon_scale", name: "Dragon Scale", kind: ItemKind::Material, max_stack: 16 },
        ItemDef { id: "belt", name: "Item Belt", kind: ItemKind::Block(block::BELT), max_stack: 16 },
        // P37 ornate professional-tier items (path-gated)
        ItemDef { id: "precision_gear", name: "Precision Gear", kind: ItemKind::Material, max_stack: 8 },
        ItemDef { id: "master_blueprint", name: "Master Blueprint", kind: ItemKind::Material, max_stack: 4 },
        ItemDef { id: "battlestaff", name: "Battlestaff", kind: ItemKind::Tool(ToolKind::Sword, 3), max_stack: 1 },
        ItemDef { id: "master_chisel", name: "Master Chisel", kind: ItemKind::Tool(ToolKind::Pickaxe, 3), max_stack: 1 },
        ItemDef { id: "enchanting_table", name: "Enchanting Table", kind: ItemKind::Block(block::ENCHANTING_TABLE), max_stack: 1 },
        ItemDef { id: "lumen_block", name: "Lumen Block", kind: ItemKind::Block(block::LUMEN_BLOCK), max_stack: 16 },
        ItemDef { id: "warding_pylon", name: "Warding Pylon", kind: ItemKind::Block(block::WARDING_PYLON), max_stack: 4 },
        // Nuclear tier (P32)
        ItemDef { id: "raw_uranium", name: "Raw Uranium", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "uranium_ingot", name: "Uranium Ingot", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "fuel_rod", name: "Fuel Rod", kind: ItemKind::Material, max_stack: 16 },
        ItemDef { id: "reactor", name: "Reactor", kind: ItemKind::Block(block::REACTOR), max_stack: 1 },
        ItemDef { id: "bucket", name: "Bucket", kind: ItemKind::Material, max_stack: 1 },
        ItemDef { id: "water_bucket", name: "Water Bucket", kind: ItemKind::Material, max_stack: 1 },
        ItemDef { id: "mutton", name: "Mutton", kind: ItemKind::Food(6), max_stack: 64 },
        ItemDef { id: "book", name: "Lore Book", kind: ItemKind::Material, max_stack: 16 },
        ItemDef { id: "bow", name: "Bow", kind: ItemKind::Tool(ToolKind::Bow, 1), max_stack: 1 },
        ItemDef { id: "arrow", name: "Arrow", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "smithing_table", name: "Smithing Table", kind: ItemKind::Block(36), max_stack: 8 },
        ItemDef { id: "coal_generator", name: "Coal Generator", kind: ItemKind::Block(37), max_stack: 8 },
        ItemDef { id: "electric_furnace", name: "Electric Furnace", kind: ItemKind::Block(38), max_stack: 8 },
        ItemDef { id: "crusher", name: "Crusher", kind: ItemKind::Block(39), max_stack: 8 },
        ItemDef { id: "assembler", name: "Assembler", kind: ItemKind::Block(40), max_stack: 8 },
        ItemDef { id: "research_bench", name: "Research Bench", kind: ItemKind::Block(41), max_stack: 8 },
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
        // Faction blocks (lore-and-visuals C1)
        ItemDef { id: "accord_stone", name: "Accord Stone", kind: ItemKind::Block(block::ACCORD_STONE), max_stack: 64 },
        ItemDef { id: "accord_pillar", name: "Accord Pillar", kind: ItemKind::Block(block::ACCORD_PILLAR), max_stack: 64 },
        ItemDef { id: "ironborn_brick", name: "Ironborn Brick", kind: ItemKind::Block(block::IRONBORN_BRICK), max_stack: 64 },
        ItemDef { id: "ironborn_grate", name: "Ironborn Grate", kind: ItemKind::Block(block::IRONBORN_GRATE), max_stack: 64 },
        ItemDef { id: "ember_covenantwood", name: "Covenantwood", kind: ItemKind::Block(block::EMBER_COVENANTWOOD), max_stack: 64 },
        ItemDef { id: "ember_glowstone", name: "Ember Glowstone", kind: ItemKind::Block(block::EMBER_GLOWSTONE), max_stack: 16 },
        ItemDef { id: "freeholds_thatch", name: "Free Holds Thatch", kind: ItemKind::Block(block::FREEHOLDS_THATCH), max_stack: 64 },
        ItemDef { id: "freeholds_daub", name: "Free Holds Daub", kind: ItemKind::Block(block::FREEHOLDS_DAUB), max_stack: 64 },
        ItemDef { id: "ashen_marble", name: "Ashen Marble", kind: ItemKind::Block(block::ASHEN_MARBLE), max_stack: 64 },
        ItemDef { id: "ashen_bookshelf", name: "Ashen Bookshelf", kind: ItemKind::Block(block::ASHEN_BOOKSHELF), max_stack: 16 },
        ItemDef { id: "nameless_rotwood", name: "Rotwood", kind: ItemKind::Block(block::NAMELESS_ROTWOOD), max_stack: 64 },
        ItemDef { id: "nameless_scorched", name: "Scorched Stone", kind: ItemKind::Block(block::NAMELESS_SCORCHED), max_stack: 64 },
        // Biome-exclusive blocks
        ItemDef { id: "mushroom_cap", name: "Mushroom Cap", kind: ItemKind::Block(block::MUSHROOM_CAP), max_stack: 64 },
        ItemDef { id: "coral_block", name: "Coral Block", kind: ItemKind::Block(block::CORAL_BLOCK), max_stack: 64 },
        ItemDef { id: "permafrost", name: "Permafrost", kind: ItemKind::Block(block::PERMAFROST), max_stack: 64 },
        ItemDef { id: "volcanic_basalt", name: "Volcanic Basalt", kind: ItemKind::Block(block::VOLCANIC_BASALT), max_stack: 64 },
        ItemDef { id: "deep_slate", name: "Deep Slate", kind: ItemKind::Block(block::DEEP_SLATE), max_stack: 64 },
        ItemDef { id: "mesa_terracotta", name: "Mesa Terracotta", kind: ItemKind::Block(block::MESA_TERRACOTTA), max_stack: 64 },
        ItemDef { id: "gilded_grass", name: "Gilded Grass", kind: ItemKind::Block(block::GILDED_GRASS), max_stack: 64 },
        ItemDef { id: "bog_peat", name: "Bog Peat", kind: ItemKind::Block(block::BOG_PEAT), max_stack: 64 },
        // Decoration blocks
        ItemDef { id: "carved_oak", name: "Carved Oak", kind: ItemKind::Block(block::CARVED_OAK), max_stack: 64 },
        ItemDef { id: "carved_stone", name: "Carved Stone", kind: ItemKind::Block(block::CARVED_STONE), max_stack: 64 },
        ItemDef { id: "carved_iron", name: "Carved Iron", kind: ItemKind::Block(block::CARVED_IRON), max_stack: 64 },
        ItemDef { id: "stained_glass_red", name: "Red Stained Glass", kind: ItemKind::Block(block::STAINED_GLASS_RED), max_stack: 64 },
        ItemDef { id: "stained_glass_orange", name: "Orange Stained Glass", kind: ItemKind::Block(block::STAINED_GLASS_ORANGE), max_stack: 64 },
        ItemDef { id: "stained_glass_yellow", name: "Yellow Stained Glass", kind: ItemKind::Block(block::STAINED_GLASS_YELLOW), max_stack: 64 },
        ItemDef { id: "stained_glass_green", name: "Green Stained Glass", kind: ItemKind::Block(block::STAINED_GLASS_GREEN), max_stack: 64 },
        ItemDef { id: "stained_glass_blue", name: "Blue Stained Glass", kind: ItemKind::Block(block::STAINED_GLASS_BLUE), max_stack: 64 },
        ItemDef { id: "stained_glass_purple", name: "Purple Stained Glass", kind: ItemKind::Block(block::STAINED_GLASS_PURPLE), max_stack: 64 },
        ItemDef { id: "stained_glass_black", name: "Black Stained Glass", kind: ItemKind::Block(block::STAINED_GLASS_BLACK), max_stack: 64 },
        ItemDef { id: "stained_glass_white", name: "White Stained Glass", kind: ItemKind::Block(block::STAINED_GLASS_WHITE), max_stack: 64 },
        ItemDef { id: "banner_accord", name: "Accord Banner", kind: ItemKind::Block(block::BANNER_ACCORD), max_stack: 16 },
        ItemDef { id: "banner_ironborn", name: "Ironborn Banner", kind: ItemKind::Block(block::BANNER_IRONBORN), max_stack: 16 },
        ItemDef { id: "banner_covenant", name: "Covenant Banner", kind: ItemKind::Block(block::BANNER_COVENANT), max_stack: 16 },
        ItemDef { id: "banner_freeholds", name: "Free Holds Banner", kind: ItemKind::Block(block::BANNER_FREEHOLDS), max_stack: 16 },
        ItemDef { id: "banner_ashen", name: "Ashen Banner", kind: ItemKind::Block(block::BANNER_ASHEN), max_stack: 16 },
        ItemDef { id: "banner_nameless", name: "Nameless Banner", kind: ItemKind::Block(block::BANNER_NAMELESS), max_stack: 16 },
        ItemDef { id: "lantern_hanging", name: "Hanging Lantern", kind: ItemKind::Block(block::LANTERN_HANGING), max_stack: 16 },
        // Faction-economy materials
        ItemDef { id: "iron_plate", name: "Iron Plate", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "bog_grass", name: "Bog Grass Bundle", kind: ItemKind::Material, max_stack: 64 },
        ItemDef { id: "torn_archive_page", name: "Torn Archive Page", kind: ItemKind::Material, max_stack: 1 },
        ItemDef { id: "anima_crystal", name: "Anima Crystal", kind: ItemKind::Material, max_stack: 16 },
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
        block::GRASS | block::JUNGLE_GRASS | block::SAVANNA_GRASS => Some("dirt".into()),
        block::DIRT => Some("dirt".into()),
        block::FLOWER => None, // petals shatter (no item yet — note in BACKLOG)
        block::WATER_WHEEL => Some("water_wheel".into()),
        block::BATTERY => Some("battery".into()),
        block::PIPE => Some("pipe".into()),
        block::BOILER => Some("boiler".into()),
        block::STEAM_ENGINE => Some("steam_engine".into()),
        block::PUMP => Some("pump".into()),
        block::REFINERY => Some("refinery".into()),
        block::COMBUSTION_GENERATOR => Some("combustion_generator".into()),
        block::DRAGON_EGG => Some("dragon_scale".into()),
        block::BELT => Some("belt".into()),
        block::CONDUIT => Some("conduit".into()),
        block::ELEVATOR => Some("elevator".into()),
        block::AC_UNIT => Some("ac_unit".into()),
        block::COMPUTER => Some("computer".into()),
        block::SCAFFOLD => Some("scaffold".into()),
        block::STATUE => Some("statue".into()),
        block::ENCHANTING_TABLE => Some("enchanting_table".into()),
        block::LUMEN_BLOCK => Some("lumen_block".into()),
        block::WARDING_PYLON => Some("warding_pylon".into()),
        block::URANIUM_ORE => Some("raw_uranium".into()),
        block::REACTOR => Some("reactor".into()),
        block::RADIATION => None, // residue is scrubbed, not collected
        block::OIL => None, // scooped with a bucket, not mined
        block::STONE => Some("stone".into()),
        block::SAND => Some("sand".into()),
        block::MYCELIUM => Some("mycelium".into()),
        block::SNOW => Some("snow".into()),
        block::LOG => Some("log".into()),
        block::LEAVES => Some("leaves".into()), // apples are a rare bonus handled by the caller
        block::TORCH => Some("torch".into()),
        block::LANTERN => Some("lantern".into()),
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
        // lore-and-visuals blocks
        block::GILDED_GRASS => Some("dirt".into()),
        block::BOG_PEAT => Some("bog_grass".into()), // the bog gives grass bundles
        block::ACCORD_STONE => Some("accord_stone".into()),
        block::ACCORD_PILLAR => Some("accord_pillar".into()),
        block::IRONBORN_BRICK => Some("ironborn_brick".into()),
        block::IRONBORN_GRATE => Some("ironborn_grate".into()),
        block::EMBER_COVENANTWOOD => Some("ember_covenantwood".into()),
        block::EMBER_GLOWSTONE => Some("ember_glowstone".into()),
        block::FREEHOLDS_THATCH => Some("freeholds_thatch".into()),
        block::FREEHOLDS_DAUB => Some("freeholds_daub".into()),
        block::ASHEN_MARBLE => Some("ashen_marble".into()),
        block::ASHEN_BOOKSHELF => Some("ashen_bookshelf".into()),
        block::NAMELESS_ROTWOOD => Some("nameless_rotwood".into()),
        block::NAMELESS_SCORCHED => Some("nameless_scorched".into()),
        block::MUSHROOM_CAP => Some("mushroom_cap".into()),
        block::CORAL_BLOCK => Some("coral_block".into()),
        block::PERMAFROST => Some("permafrost".into()),
        block::VOLCANIC_BASALT => Some("volcanic_basalt".into()),
        block::DEEP_SLATE => Some("deep_slate".into()),
        block::MESA_TERRACOTTA => Some("mesa_terracotta".into()),
        block::CARVED_OAK => Some("carved_oak".into()),
        block::CARVED_STONE => Some("carved_stone".into()),
        block::CARVED_IRON => Some("carved_iron".into()),
        block::BANNER_ACCORD => Some("banner_accord".into()),
        block::BANNER_IRONBORN => Some("banner_ironborn".into()),
        block::BANNER_COVENANT => Some("banner_covenant".into()),
        block::BANNER_FREEHOLDS => Some("banner_freeholds".into()),
        block::BANNER_ASHEN => Some("banner_ashen".into()),
        block::BANNER_NAMELESS => Some("banner_nameless".into()),
        block::LANTERN_HANGING => Some("lantern_hanging".into()),
        id if lf_voxel::registry::is_stained_glass(id) => None, // stained glass shatters like glass
        id if id >= lf_voxel::registry::MOD_BLOCK_BASE => {
            lf_voxel::registry::mod_block(id).and_then(|d| d.drop)
        }
        _ => Some("stone".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::shaped_placement;
    use lf_voxel::Shape;

    #[test]
    fn shaped_placement_orients_stairs_by_yaw() {
        assert_eq!(shaped_placement("stone_slab", 0.0).unwrap().shape(), Shape::SlabBottom);
        assert_eq!(shaped_placement("planks_slab", 3.0).unwrap().id(), lf_voxel::registry::block::PLANKS);
        assert_eq!(shaped_placement("stone_stairs", 0.0).unwrap().shape(), Shape::StairNorth);
        assert_eq!(shaped_placement("stone_stairs", 1.6).unwrap().shape(), Shape::StairEast);
        assert_eq!(shaped_placement("stone_stairs", 3.2).unwrap().shape(), Shape::StairSouth);
        assert_eq!(shaped_placement("stone_stairs", 4.7).unwrap().shape(), Shape::StairWest);
        assert_eq!(shaped_placement("stone", 0.0), None, "plain blocks are not shaped");
        // two bottom slabs of the same material merge into a cube
        let a = shaped_placement("stone_slab", 0.0).unwrap();
        assert_eq!(super::slab_merge(a, a).unwrap(), lf_voxel::BlockState(lf_voxel::registry::block::STONE));
        let p = shaped_placement("planks_slab", 0.0).unwrap();
        assert!(super::slab_merge(a, p).is_none(), "different materials do not merge");
    }

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

/// P34 construction: shaped-block placement. Slabs place bottom-half
/// (a slab onto a matching bottom slab merges into a full cube — handled
/// by the caller via [`slab_merge`]); stairs orient by the player's yaw
/// so you always walk UP the way you face.
pub fn shaped_placement(item_id: &str, yaw: f32) -> Option<lf_voxel::BlockState> {
    use lf_voxel::Shape;
    use lf_voxel::registry::block;
    let base = match item_id {
        "stone_slab" => lf_voxel::BlockState(block::STONE),
        "planks_slab" => lf_voxel::BlockState(block::PLANKS),
        "stone_stairs" => lf_voxel::BlockState(block::STONE),
        _ => return None,
    };
    let shape = match item_id {
        "stone_slab" | "planks_slab" => Shape::SlabBottom,
        // yaw 0 faces -Z; walking "forward" is the way you look, so the
        // high half goes opposite the look direction (you step UP toward it)
        _ => {
            let deg = yaw.to_degrees().rem_euclid(360.0);
            match deg {
                d if d < 45.0 || d >= 315.0 => Shape::StairNorth,
                d if d < 135.0 => Shape::StairEast,
                d if d < 225.0 => Shape::StairSouth,
                _ => Shape::StairWest,
            }
        }
    };
    Some(base.with_shape(shape))
}

/// Placing a bottom slab onto a matching bottom slab makes a full cube.
/// Returns the state the cell should become (None = no merge).
pub fn slab_merge(existing: lf_voxel::BlockState, incoming: lf_voxel::BlockState) -> Option<lf_voxel::BlockState> {
    use lf_voxel::Shape;
    if existing.id() == incoming.id()
        && existing.shape() == Shape::SlabBottom
        && incoming.shape() == Shape::SlabBottom
    {
        Some(lf_voxel::BlockState(existing.id()))
    } else {
        None
    }
}
