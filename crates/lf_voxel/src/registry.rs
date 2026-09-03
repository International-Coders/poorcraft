use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use crate::BlockState;

/// Runtime-registered mod block (ids >= MOD_BLOCK_BASE).
#[derive(Clone, Debug)]
pub struct ModBlockDef {
    pub name: String,
    pub solid: bool,
    pub opaque: bool,
    /// Item id this block drops (None = nothing).
    pub drop: Option<String>,
    /// Light emission 0..15 (P34: the modapi `light` field finally
    /// reaches the light engine — it was parsed and dropped before).
    pub light: u8,
}

/// Mod blocks start here, far above the vanilla range. Bumped 100 -> 200
/// by the lore-and-visuals block expansion: 38 new vanilla blocks occupy
/// 68..=105 (DECISIONS.md "MOD_BLOCK_BASE 200"). Old worlds keep loading;
/// previously-placed mod blocks re-register at their new hash ids.
pub const MOD_BLOCK_BASE: u32 = 200;

/// Highest contiguous vanilla block id (machine/ore ids included).
pub const MAX_VANILLA_BLOCK: u32 = 141;

/// True when `id` is a placeable block: air, a vanilla id, or a block
/// registered by a loaded mod. The server uses this to validate SetBlock.
pub fn is_known_block(id: u32) -> bool {
    if id >= MOD_BLOCK_BASE {
        return mod_block(id).is_some();
    }
    id <= MAX_VANILLA_BLOCK
}

fn mod_blocks() -> &'static RwLock<HashMap<u32, ModBlockDef>> {
    static BLOCKS: OnceLock<RwLock<HashMap<u32, ModBlockDef>>> = OnceLock::new();
    BLOCKS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register a mod block (id must be >= MOD_BLOCK_BASE). Returns false on a
/// id collision with a different name.
pub fn register_mod_block(id: u32, def: ModBlockDef) -> bool {
    if id < MOD_BLOCK_BASE {
        return false;
    }
    let mut blocks = mod_blocks().write().unwrap();
    match blocks.get(&id) {
        Some(existing) if existing.name == def.name => true,
        Some(_) => false,
        None => {
            blocks.insert(id, def);
            true
        }
    }
}

pub fn mod_block(id: u32) -> Option<ModBlockDef> {
    mod_blocks().read().unwrap().get(&id).cloned()
}

pub fn registered_mod_blocks() -> Vec<(u32, ModBlockDef)> {
    let blocks = mod_blocks().read().unwrap();
    let mut out: Vec<_> = blocks.iter().map(|(k, v)| (*k, v.clone())).collect();
    out.sort_by_key(|(id, _)| *id);
    out
}

/// Canonical block ids. P5 makes this a full data-driven registry; for now a
/// single source of truth for gameplay properties.
pub mod block {
    pub const AIR: u32 = 0;
    pub const STONE: u32 = 1;
    pub const GRASS: u32 = 2;
    pub const DIRT: u32 = 3;
    pub const SAND: u32 = 4;
    pub const MYCELIUM: u32 = 5;
    pub const SNOW: u32 = 6;
    pub const LOG: u32 = 7;
    pub const LEAVES: u32 = 8;
    pub const COAL_ORE: u32 = 9;
    pub const IRON_ORE: u32 = 10;
    pub const WATER: u32 = 11;
    pub const TORCH: u32 = 12;
    pub const LANTERN: u32 = 13;
    pub const CRAFTING_TABLE: u32 = 14;
    pub const FURNACE: u32 = 15;
    pub const CHEST: u32 = 16;
    pub const PLANKS: u32 = 17;
    pub const GLASS: u32 = 18;
    // biome/wood variants
    pub const BIRCH_LOG: u32 = 19;
    pub const SPRUCE_LOG: u32 = 20;
    pub const DARK_LOG: u32 = 21;
    pub const CHERRY_LOG: u32 = 22;
    pub const BIRCH_LEAVES: u32 = 23;
    pub const SPRUCE_LEAVES: u32 = 24;
    pub const DARK_LEAVES: u32 = 25;
    pub const CHERRY_LEAVES: u32 = 26;
    pub const PALE_LEAVES: u32 = 27;
    pub const RED_SAND: u32 = 28;
    pub const TERRACOTTA: u32 = 29;
    pub const MOSS: u32 = 30;
    pub const ICE: u32 = 31;
    pub const SMITHING_TABLE: u32 = 36;
    pub const COAL_GENERATOR: u32 = 37;
    pub const ELECTRIC_FURNACE: u32 = 38;
    pub const CRUSHER: u32 = 39;
    pub const ASSEMBLER: u32 = 40;
    pub const RESEARCH_BENCH: u32 = 41;
    // biome-identity surfaces (build-pack Step 16-17)
    pub const JUNGLE_GRASS: u32 = 42;
    pub const SAVANNA_GRASS: u32 = 43;
    pub const FLOWER: u32 = 44;
    // Water Age machines (P29)
    pub const WATER_WHEEL: u32 = 45;
    pub const BATTERY: u32 = 46;
    // Steam Age machines (P30)
    pub const PIPE: u32 = 47;
    pub const BOILER: u32 = 48;
    pub const STEAM_ENGINE: u32 = 49;
    // Oil Age (P31): crude fluid + extraction/power machines
    pub const OIL: u32 = 50;
    pub const PUMP: u32 = 51;
    pub const REFINERY: u32 = 52;
    pub const COMBUSTION_GENERATOR: u32 = 53;
    // Nuclear tier (P32, the ceiling — DECISIONS Pillar 5)
    pub const URANIUM_ORE: u32 = 54;
    pub const REACTOR: u32 = 55;
    pub const RADIATION: u32 = 56;
    // Magic foundation (P33)
    pub const ENCHANTING_TABLE: u32 = 57;
    pub const LUMEN_BLOCK: u32 = 58;
    pub const WARDING_PYLON: u32 = 59;
    // Construction (P34)
    pub const SCAFFOLD: u32 = 60;
    pub const STATUE: u32 = 61;
    // Smart building (P35)
    pub const CONDUIT: u32 = 62;
    pub const ELEVATOR: u32 = 63;
    pub const AC_UNIT: u32 = 64;
    pub const COMPUTER: u32 = 65;
    // P36: the roost marker + rare crafting material
    pub const DRAGON_EGG: u32 = 66;
    // Step 27: the item belt backbone
    pub const BELT: u32 = 67;
    // industrial ores
    pub const COPPER_ORE: u32 = 32;
    pub const TIN_ORE: u32 = 33;
    pub const BAUXITE_ORE: u32 = 34;
    pub const SULFUR_ORE: u32 = 35;
    // Faction-themed blocks (lore-and-visuals C1) — used by faction
    // structures and territory building.
    pub const ACCORD_STONE: u32 = 68;
    pub const ACCORD_PILLAR: u32 = 69;
    pub const IRONBORN_BRICK: u32 = 70;
    pub const IRONBORN_GRATE: u32 = 71;
    pub const EMBER_COVENANTWOOD: u32 = 72;
    pub const EMBER_GLOWSTONE: u32 = 73;
    pub const FREEHOLDS_THATCH: u32 = 74;
    pub const FREEHOLDS_DAUB: u32 = 75;
    pub const ASHEN_MARBLE: u32 = 76;
    pub const ASHEN_BOOKSHELF: u32 = 77;
    pub const NAMELESS_ROTWOOD: u32 = 78;
    pub const NAMELESS_SCORCHED: u32 = 79;
    // Biome-exclusive blocks (one per biome group filling the 30-biome gaps)
    pub const MUSHROOM_CAP: u32 = 80;
    pub const CORAL_BLOCK: u32 = 81;
    pub const PERMAFROST: u32 = 82;
    pub const VOLCANIC_BASALT: u32 = 83;
    pub const DEEP_SLATE: u32 = 84;
    pub const MESA_TERRACOTTA: u32 = 85;
    pub const GILDED_GRASS: u32 = 86;
    pub const BOG_PEAT: u32 = 87;
    // Decoration blocks
    pub const CARVED_OAK: u32 = 88;
    pub const CARVED_STONE: u32 = 89;
    pub const CARVED_IRON: u32 = 90;
    pub const STAINED_GLASS_RED: u32 = 91;
    pub const STAINED_GLASS_ORANGE: u32 = 92;
    pub const STAINED_GLASS_YELLOW: u32 = 93;
    pub const STAINED_GLASS_GREEN: u32 = 94;
    pub const STAINED_GLASS_BLUE: u32 = 95;
    pub const STAINED_GLASS_PURPLE: u32 = 96;
    pub const STAINED_GLASS_BLACK: u32 = 97;
    pub const STAINED_GLASS_WHITE: u32 = 98;
    pub const BANNER_ACCORD: u32 = 99;
    pub const BANNER_IRONBORN: u32 = 100;
    pub const BANNER_COVENANT: u32 = 101;
    pub const BANNER_FREEHOLDS: u32 = 102;
    pub const BANNER_ASHEN: u32 = 103;
    pub const BANNER_NAMELESS: u32 = 104;
    /// Ceiling/chain-mounted lantern (shares lantern art; different
    /// placement, per SKIN_MANIFEST).
    pub const LANTERN_HANGING: u32 = 105;
    // ui-world-craft D3/E3: cave lava + biome surface decoration
    pub const TALL_GRASS: u32 = 106;
    pub const DRY_GRASS: u32 = 107;
    pub const CACTUS: u32 = 108;
    pub const DEAD_SHRUB: u32 = 109;
    pub const LAVA: u32 = 110;
    // Loop 330 timber: felled trees land as horizontal logs — one X- and
    // one Z-aligned variant per species (the atlas reuses each species'
    // existing bark/log_top layers through the per-face mapping).
    pub const LOG_X: u32 = 111;
    pub const LOG_Z: u32 = 112;
    pub const BIRCH_LOG_X: u32 = 113;
    pub const BIRCH_LOG_Z: u32 = 114;
    pub const SPRUCE_LOG_X: u32 = 115;
    pub const SPRUCE_LOG_Z: u32 = 116;
    pub const DARK_LOG_X: u32 = 117;
    pub const DARK_LOG_Z: u32 = 118;
    pub const CHERRY_LOG_X: u32 = 119;
    pub const CHERRY_LOG_Z: u32 = 120;
    // king-quest B: 15 new biomes need their own trees/ground (ids 121-138)
    pub const PALM_LOG: u32 = 121;
    pub const PALM_LEAVES: u32 = 122;
    pub const ACACIA_LOG: u32 = 123;
    pub const ACACIA_LEAVES: u32 = 124;
    pub const MANGROVE_LOG: u32 = 125;
    pub const MANGROVE_LEAVES: u32 = 126;
    pub const REDWOOD_LOG: u32 = 127;
    pub const REDWOOD_LEAVES: u32 = 128;
    pub const MAPLE_LOG: u32 = 129;
    pub const MAPLE_LEAVES: u32 = 130;
    pub const ASPEN_LEAVES: u32 = 131;
    pub const WILLOW_LEAVES: u32 = 132;
    pub const BAOBAB_LOG: u32 = 133;
    pub const EMBER_LOG: u32 = 134;
    pub const EMBER_LEAVES: u32 = 135;
    pub const LAVENDER: u32 = 136;
    pub const SUNFLOWER: u32 = 137;
    pub const SALT: u32 = 138;

    /// loop 345 kingdoms: the throne is the kingdom-settle marker the
    /// client scans (like faction banners / enchanting-table towers).
    pub const THRONE: u32 = 139;
    /// The royal banner flying over the citadel gate and market stalls.
    pub const BANNER_KINGDOM: u32 = 140;
    /// Pale ashlar masonry of the kingdom walls and keep.
    pub const KINGDOM_BRICK: u32 = 141;

    pub fn name(id: u32) -> &'static str {
        if let Some(def) = crate::registry::mod_block(id) {
            // names of registered mods outlive the lookup via leak (bounded by the mod set)
            return Box::leak(def.name.into_boxed_str());
        }
        match id {
            AIR => "Air",
            STONE => "Stone",
            GRASS => "Grass",
            JUNGLE_GRASS => "Jungle Grass",
            SAVANNA_GRASS => "Savanna Grass",
            FLOWER => "Wildflower",
            WATER_WHEEL => "Water Wheel",
            BATTERY => "Battery",
            PIPE => "Pipe",
            BOILER => "Boiler",
            STEAM_ENGINE => "Steam Engine",
            OIL => "Crude Oil",
            PUMP => "Pumpjack",
            REFINERY => "Refinery",
            COMBUSTION_GENERATOR => "Combustion Generator",
            URANIUM_ORE => "Uranium Ore",
            REACTOR => "Reactor",
            RADIATION => "Radiation Residue",
            ENCHANTING_TABLE => "Enchanting Table",
            LUMEN_BLOCK => "Lumen Block",
            WARDING_PYLON => "Warding Pylon",
            SCAFFOLD => "Scaffolding",
            STATUE => "Chiseled Statue",
            CONDUIT => "Power Conduit",
            ELEVATOR => "Elevator",
            AC_UNIT => "Climate Unit",
            COMPUTER => "Computer Screen",
            DRAGON_EGG => "Dragon Egg",
            BELT => "Item Belt",
            DIRT => "Dirt",
            SAND => "Sand",
            MYCELIUM => "Mycelium",
            SNOW => "Snow",
            LOG => "Log",
            LEAVES => "Leaves",
            COAL_ORE => "Coal Ore",
            IRON_ORE => "Iron Ore",
            WATER => "Water",
            TORCH => "Torch",
            LANTERN => "Lantern",
            CRAFTING_TABLE => "Crafting Table",
            FURNACE => "Furnace",
            CHEST => "Chest",
            PLANKS => "Planks",
            GLASS => "Glass",
            BIRCH_LOG => "Birch Log",
            SPRUCE_LOG => "Spruce Log",
            DARK_LOG => "Dark Oak Log",
            CHERRY_LOG => "Cherry Log",
            LOG_X | LOG_Z => "Log",
            BIRCH_LOG_X | BIRCH_LOG_Z => "Birch Log",
            SPRUCE_LOG_X | SPRUCE_LOG_Z => "Spruce Log",
            DARK_LOG_X | DARK_LOG_Z => "Dark Oak Log",
            CHERRY_LOG_X | CHERRY_LOG_Z => "Cherry Log",
            PALM_LOG => "Palm Log",
            PALM_LEAVES => "Palm Leaves",
            ACACIA_LOG => "Acacia Log",
            ACACIA_LEAVES => "Acacia Leaves",
            MANGROVE_LOG => "Mangrove Log",
            MANGROVE_LEAVES => "Mangrove Leaves",
            REDWOOD_LOG => "Redwood Log",
            REDWOOD_LEAVES => "Redwood Leaves",
            MAPLE_LOG => "Maple Log",
            MAPLE_LEAVES => "Maple Leaves",
            ASPEN_LEAVES => "Aspen Leaves",
            WILLOW_LEAVES => "Willow Leaves",
            BAOBAB_LOG => "Baobab Log",
            EMBER_LOG => "Ember Log",
            EMBER_LEAVES => "Ember Leaves",
            LAVENDER => "Lavender",
            SUNFLOWER => "Sunflower",
            SALT => "Salt Flat",
            THRONE => "Throne",
            BANNER_KINGDOM => "Kingdom Banner",
            KINGDOM_BRICK => "Kingdom Brick",

            BIRCH_LEAVES => "Birch Leaves",
            SPRUCE_LEAVES => "Spruce Leaves",
            DARK_LEAVES => "Dark Oak Leaves",
            CHERRY_LEAVES => "Cherry Leaves",
            PALE_LEAVES => "Pale Leaves",
            RED_SAND => "Red Sand",
            TERRACOTTA => "Terracotta",
            MOSS => "Moss",
            ICE => "Ice",
            COPPER_ORE => "Copper Ore",
            TIN_ORE => "Tin Ore",
            BAUXITE_ORE => "Bauxite Ore",
            SULFUR_ORE => "Sulfur Ore",
            SMITHING_TABLE => "Smithing Table",
            COAL_GENERATOR => "Coal Generator",
            ELECTRIC_FURNACE => "Electric Furnace",
            CRUSHER => "Crusher",
            ASSEMBLER => "Assembler",
            RESEARCH_BENCH => "Research Bench",
            ACCORD_STONE => "Accord Stone",
            ACCORD_PILLAR => "Accord Pillar",
            IRONBORN_BRICK => "Ironborn Brick",
            IRONBORN_GRATE => "Ironborn Grate",
            EMBER_COVENANTWOOD => "Covenantwood",
            EMBER_GLOWSTONE => "Ember Glowstone",
            FREEHOLDS_THATCH => "Free Holds Thatch",
            FREEHOLDS_DAUB => "Free Holds Daub",
            ASHEN_MARBLE => "Ashen Marble",
            ASHEN_BOOKSHELF => "Ashen Bookshelf",
            NAMELESS_ROTWOOD => "Rotwood",
            NAMELESS_SCORCHED => "Scorched Stone",
            MUSHROOM_CAP => "Mushroom Cap",
            CORAL_BLOCK => "Coral Block",
            PERMAFROST => "Permafrost",
            VOLCANIC_BASALT => "Volcanic Basalt",
            DEEP_SLATE => "Deep Slate",
            MESA_TERRACOTTA => "Mesa Terracotta",
            GILDED_GRASS => "Gilded Grass",
            BOG_PEAT => "Bog Peat",
            CARVED_OAK => "Carved Oak",
            CARVED_STONE => "Carved Stone",
            CARVED_IRON => "Carved Iron",
            STAINED_GLASS_RED => "Red Stained Glass",
            STAINED_GLASS_ORANGE => "Orange Stained Glass",
            STAINED_GLASS_YELLOW => "Yellow Stained Glass",
            STAINED_GLASS_GREEN => "Green Stained Glass",
            STAINED_GLASS_BLUE => "Blue Stained Glass",
            STAINED_GLASS_PURPLE => "Purple Stained Glass",
            STAINED_GLASS_BLACK => "Black Stained Glass",
            STAINED_GLASS_WHITE => "White Stained Glass",
            BANNER_ACCORD => "Accord Banner",
            BANNER_IRONBORN => "Ironborn Banner",
            BANNER_COVENANT => "Covenant Banner",
            BANNER_FREEHOLDS => "Free Holds Banner",
            BANNER_ASHEN => "Ashen Banner",
            BANNER_NAMELESS => "Nameless Banner",
            LANTERN_HANGING => "Hanging Lantern",
            TALL_GRASS => "Tall Grass",
            DRY_GRASS => "Dry Grass",
            CACTUS => "Cactus",
            DEAD_SHRUB => "Dead Shrub",
            LAVA => "Lava",
            _ => "Unknown",
        }
    }
}

/// Foliage blocks: non-opaque, sway in the wind, alpha-cutout rendering.
pub fn is_leaf(id: u32) -> bool {
    use block as b;
    id == b::LEAVES || id == b::BIRCH_LEAVES || id == b::SPRUCE_LEAVES
        || id == b::DARK_LEAVES || id == b::CHERRY_LEAVES || id == b::PALE_LEAVES
        || id == b::PALM_LEAVES || id == b::ACACIA_LEAVES || id == b::MANGROVE_LEAVES
        || id == b::REDWOOD_LEAVES || id == b::MAPLE_LEAVES || id == b::ASPEN_LEAVES
        || id == b::WILLOW_LEAVES || id == b::EMBER_LEAVES
}

/// The five trunk species that form trees (mushroom "trunks" are birch).
pub fn is_log(id: u32) -> bool {
    log_axis(id).is_some() || matches!(id,
        block::LOG | block::BIRCH_LOG | block::SPRUCE_LOG | block::DARK_LOG | block::CHERRY_LOG
        | block::PALM_LOG | block::ACACIA_LOG | block::MANGROVE_LOG | block::REDWOOD_LOG
        | block::MAPLE_LOG | block::BAOBAB_LOG | block::EMBER_LOG)
}

/// Alignment of a horizontal log variant: Some(Axis::X) for `LOG_X`-style
/// ids, Some(Axis::Z) for `LOG_Z`-style, None for vertical (or non-log)
/// blocks. Vertical trunks are the un-marked default so old saves and
/// worldgen need no change.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

pub fn log_axis(id: u32) -> Option<Axis> {
    use block as b;
    match id {
        b::LOG_X | b::BIRCH_LOG_X | b::SPRUCE_LOG_X | b::DARK_LOG_X | b::CHERRY_LOG_X => Some(Axis::X),
        b::LOG_Z | b::BIRCH_LOG_Z | b::SPRUCE_LOG_Z | b::DARK_LOG_Z | b::CHERRY_LOG_Z => Some(Axis::Z),
        _ => None,
    }
}

/// The vertical trunk block a species' horizontal variants come from (used
/// for drops and for texture-layer reuse).
pub fn horizontal_log_base(id: u32) -> Option<u32> {
    use block as b;
    match id {
        b::LOG_X | b::LOG_Z => Some(b::LOG),
        b::BIRCH_LOG_X | b::BIRCH_LOG_Z => Some(b::BIRCH_LOG),
        b::SPRUCE_LOG_X | b::SPRUCE_LOG_Z => Some(b::SPRUCE_LOG),
        b::DARK_LOG_X | b::DARK_LOG_Z => Some(b::DARK_LOG),
        b::CHERRY_LOG_X | b::CHERRY_LOG_Z => Some(b::CHERRY_LOG),
        _ => None,
    }
}

/// The X-aligned horizontal variant of a vertical trunk species.
pub fn log_horizontal_x(vertical_id: u32) -> Option<u32> {
    use block as b;
    match vertical_id {
        b::LOG => Some(b::LOG_X),
        b::BIRCH_LOG => Some(b::BIRCH_LOG_X),
        b::SPRUCE_LOG => Some(b::SPRUCE_LOG_X),
        b::DARK_LOG => Some(b::DARK_LOG_X),
        b::CHERRY_LOG => Some(b::CHERRY_LOG_X),
        _ => None,
    }
}

/// The Z-aligned horizontal variant of a vertical trunk species.
pub fn log_horizontal_z(vertical_id: u32) -> Option<u32> {
    use block as b;
    match vertical_id {
        b::LOG => Some(b::LOG_Z),
        b::BIRCH_LOG => Some(b::BIRCH_LOG_Z),
        b::SPRUCE_LOG => Some(b::SPRUCE_LOG_Z),
        b::DARK_LOG => Some(b::DARK_LOG_Z),
        b::CHERRY_LOG => Some(b::CHERRY_LOG_Z),
        _ => None,
    }
}

/// Blocks entities collide with. Water is not solid; leaves are. Every
/// cross-plant (`is_plant`) is walk-through decor — the per-id list once
/// missed lavender/sunflower, which made whole flower biomes invisible
/// solid walls (loop 347).
pub fn is_solid(b: BlockState) -> bool {
    let id = b.id();
    if id >= MOD_BLOCK_BASE {
        return mod_block(id).map(|d| d.solid).unwrap_or(true);
    }
    id != block::AIR && id != block::WATER && id != block::OIL && id != block::TORCH
        && id != block::LANTERN && id != block::LANTERN_HANGING && !is_plant(id)
}

/// Blocks that hide the neighboring face when meshing. Air, water and leaves
/// let faces behind them render.
pub fn is_opaque(b: BlockState) -> bool {
    let id = b.id();
    if id >= MOD_BLOCK_BASE {
        return mod_block(id).map(|d| d.opaque).unwrap_or(true);
    }
    if is_stained_glass(id) || is_banner(id) || id == block::IRONBORN_GRATE {
        return false;
    }
    id != block::AIR && id != block::WATER && id != block::OIL && !is_leaf(id)
        && id != block::TORCH && id != block::LANTERN && id != block::GLASS
        && id != block::ICE && !is_plant(id)
}

/// The eight stained-glass tint variants (translucent pane like glass).
pub fn is_stained_glass(id: u32) -> bool {
    use block as b;
    matches!(id, b::STAINED_GLASS_RED | b::STAINED_GLASS_ORANGE | b::STAINED_GLASS_YELLOW
        | b::STAINED_GLASS_GREEN | b::STAINED_GLASS_BLUE | b::STAINED_GLASS_PURPLE
        | b::STAINED_GLASS_BLACK | b::STAINED_GLASS_WHITE)
}

/// Faction banners: non-solid cutout blocks rendered as flat sign quads.
pub fn is_banner(id: u32) -> bool {
    use block as b;
    matches!(id, b::BANNER_ACCORD | b::BANNER_IRONBORN | b::BANNER_COVENANT
        | b::BANNER_FREEHOLDS | b::BANNER_ASHEN | b::BANNER_NAMELESS
        | b::BANNER_KINGDOM)
}

/// Collision boxes (block-local 0..1 coordinates) for shaped blocks —
/// the physics resolves the player AABB against these (P34). Full cubes
/// return the whole cell.
pub fn collision_boxes(state: BlockState) -> &'static [[f32; 6]] {
    use crate::Shape;
    const FULL: [[f32; 6]; 1] = [[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]];
    const SLAB_BOTTOM: [[f32; 6]; 1] = [[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]];
    const SLAB_TOP: [[f32; 6]; 1] = [[0.0, 0.5, 0.0, 1.0, 1.0, 1.0]];
    const STAIR_N: [[f32; 6]; 2] = [[0.0, 0.0, 0.0, 1.0, 0.5, 1.0], [0.0, 0.5, 0.0, 1.0, 1.0, 0.5]];
    const STAIR_S: [[f32; 6]; 2] = [[0.0, 0.0, 0.0, 1.0, 0.5, 1.0], [0.0, 0.5, 0.5, 1.0, 1.0, 1.0]];
    const STAIR_W: [[f32; 6]; 2] = [[0.0, 0.0, 0.0, 1.0, 0.5, 1.0], [0.0, 0.5, 0.0, 0.5, 1.0, 1.0]];
    const STAIR_E: [[f32; 6]; 2] = [[0.0, 0.0, 0.0, 1.0, 0.5, 1.0], [0.5, 0.5, 0.0, 1.0, 1.0, 1.0]];
    match state.shape() {
        Shape::Cube => &FULL,
        Shape::SlabBottom => &SLAB_BOTTOM,
        Shape::SlabTop => &SLAB_TOP,
        Shape::StairNorth => &STAIR_N,
        Shape::StairSouth => &STAIR_S,
        Shape::StairWest => &STAIR_W,
        Shape::StairEast => &STAIR_E,
    }
}

/// Targeting boxes (block-local 0..1 coordinates): what the crosshair
/// raycast tests and the selection wireframe draws. Distinct from
/// `collision_boxes` because non-solid decor still needs a pick shape —
/// a torch is a thin stick, a flower a small inset box, while physics
/// ignores them entirely (loop 347 hitbox fix).
pub fn pick_boxes(state: BlockState) -> &'static [[f32; 6]] {
    use block as b;
    const FULL: [[f32; 6]; 1] = [[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]];
    /// cross-plants: the meshed diagonals live in the inner ~0.7 cell
    const PLANT: [[f32; 6]; 1] = [[0.15, 0.0, 0.15, 0.85, 0.8, 0.85]];
    const TORCH: [[f32; 6]; 1] = [[0.4375, 0.0, 0.4375, 0.5625, 0.625, 0.5625]];
    const LANTERN: [[f32; 6]; 1] = [[0.3125, 0.0, 0.3125, 0.6875, 0.5625, 0.6875]];
    const LANTERN_HANG: [[f32; 6]; 1] = [[0.3125, 0.4375, 0.3125, 0.6875, 1.0, 0.6875]];
    let id = state.id();
    if id >= MOD_BLOCK_BASE {
        return &FULL;
    }
    if is_plant(id) {
        return &PLANT;
    }
    match id {
        b::TORCH => &TORCH,
        b::LANTERN => &LANTERN,
        b::LANTERN_HANGING => &LANTERN_HANG,
        _ => collision_boxes(state), // cubes, slabs and stairs keep their shape
    }
}

/// Blocks the crosshair can target (mining/placing raycast hits). Fluids
/// are scooped with the bucket via the face-adjacent cell instead.
pub fn is_targetable(b: BlockState) -> bool {
    b.id() != block::AIR && b.id() != block::WATER && b.id() != block::OIL
}

/// Cross-plants: non-solid cutout blocks that sit on the ground. Banners
/// render through the same flat-quad path (sign-style, per SKIN_MANIFEST).
pub fn is_plant(id: u32) -> bool {
    id == block::FLOWER || id == block::TALL_GRASS || id == block::DRY_GRASS
        || id == block::DEAD_SHRUB || id == block::LAVENDER || id == block::SUNFLOWER
        || is_banner(id)
}

/// Granular blocks that fall when the block under them is removed (they do
/// not float). Ores are deliberately excluded — they are embedded in the
/// stone matrix, not loose (same rule Minecraft uses for sand/gravel).
pub fn has_gravity(id: u32) -> bool {
    use block as b;
    id == b::SAND || id == b::RED_SAND || id == b::SNOW
        || id == b::DIRT || id == b::GRASS || id == b::JUNGLE_GRASS
        || id == b::SAVANNA_GRASS || id == b::MOSS || id == b::MYCELIUM
        || id == b::GILDED_GRASS || id == b::PERMAFROST || id == b::BOG_PEAT
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Loop 330 timber: the horizontal log variants know their axis and
    /// their species, drop-path helpers round-trip, and they are plain
    /// opaque solid cubes like the vertical trunks.
    #[test]
    fn horizontal_logs_map_axis_and_species() {
        use block as b;
        assert_eq!(log_axis(b::LOG), None);
        assert_eq!(log_axis(b::LOG_X), Some(Axis::X));
        assert_eq!(log_axis(b::LOG_Z), Some(Axis::Z));
        assert_eq!(log_axis(b::CHERRY_LOG_X), Some(Axis::X));
        assert_eq!(log_axis(b::BIRCH_LOG_Z), Some(Axis::Z));
        // every horizontal variant maps back to its species trunk
        for (vertical, x_id, z_id) in [
            (b::LOG, b::LOG_X, b::LOG_Z),
            (b::BIRCH_LOG, b::BIRCH_LOG_X, b::BIRCH_LOG_Z),
            (b::SPRUCE_LOG, b::SPRUCE_LOG_X, b::SPRUCE_LOG_Z),
            (b::DARK_LOG, b::DARK_LOG_X, b::DARK_LOG_Z),
            (b::CHERRY_LOG, b::CHERRY_LOG_X, b::CHERRY_LOG_Z),
        ] {
            assert_eq!(horizontal_log_base(x_id), Some(vertical));
            assert_eq!(horizontal_log_base(z_id), Some(vertical));
            assert_eq!(log_horizontal_x(vertical), Some(x_id));
            assert_eq!(log_horizontal_z(vertical), Some(z_id));
            assert!(is_log(x_id) && is_log(z_id) && is_log(vertical));
            // plain cubes: solid + opaque like the vertical trunk
            assert!(is_solid(BlockState(x_id)) && is_opaque(BlockState(x_id)));
            assert!(is_targetable(BlockState(z_id)));
        }
    }

    #[test]
    fn registry_properties() {
        assert!(!is_solid(BlockState::AIR));
        assert!(!is_solid(BlockState(block::WATER)));
        assert!(is_solid(BlockState(block::LEAVES)));
        assert!(is_solid(BlockState(block::STONE)));
        assert!(!is_opaque(BlockState(block::WATER)));
        assert!(!is_opaque(BlockState(block::LEAVES)));
        assert!(is_opaque(BlockState(block::GRASS)));
        assert!(!is_targetable(BlockState(block::WATER)));
        assert!(is_targetable(BlockState(block::LOG)));
    }

    /// Loop 331: ground plants are cross-plant decor — walked through,
    /// targeted for breaking, not gravity fallers, and banners excluded.
    /// Loop 347: the list-driven exclusion once missed lavender/sunflower
    /// (invisible solid walls + culled ground faces under whole biomes),
    /// so the contract is now stated for EVERY is_plant id.
    #[test]
    fn plants_are_walk_through_decor() {
        use block as b;
        for plant in [b::FLOWER, b::TALL_GRASS, b::DRY_GRASS, b::DEAD_SHRUB,
                      b::LAVENDER, b::SUNFLOWER, b::BANNER_ACCORD, b::BANNER_KINGDOM] {
            assert!(is_plant(plant), "{} is a plant", plant);
            assert!(!is_solid(BlockState(plant)), "{} is walked through", plant);
            assert!(!is_opaque(BlockState(plant)), "{} never culls the ground under it", plant);
            assert!(is_targetable(BlockState(plant)), "plants breakable");
            assert!(!has_gravity(plant), "plants pop, not fall");
        }
        // banners share is_plant but stay wall decor
        assert!(is_plant(b::BANNER_ACCORD));
        // trunks and stone are unaffected
        assert!(is_solid(BlockState(b::LOG)));
        assert!(!is_plant(b::LOG));
    }

    /// Loop 347 hitbox fix: the pick shape mirrors what you see. Full
    /// cubes outline the whole cell, slabs/stairs reuse their collision
    /// shape, and thin decor (torch/lantern/plants) gets a small box so
    /// the wireframe and crosshair stop claiming empty air.
    #[test]
    fn pick_boxes_match_the_visible_shape() {
        use block as b;
        let full = pick_boxes(BlockState(b::STONE));
        assert_eq!(full, &[[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]);
        // torch: a 2/16-wide stick, 10/16 tall
        let torch = pick_boxes(BlockState(b::TORCH))[0];
        assert!(torch[3] - torch[0] < 0.2, "torch is thin: {:?}", torch);
        assert!(torch[4] < 0.7, "torch is short: {:?}", torch);
        // hanging lantern hangs from the ceiling, not the floor
        let hang = pick_boxes(BlockState(b::LANTERN_HANGING))[0];
        assert!(hang[1] > 0.3 && hang[4] > 0.9, "hanging lantern top-anchored: {:?}", hang);
        // plants: inset, sub-cell height
        let plant = pick_boxes(BlockState(b::LAVENDER))[0];
        assert!(plant[0] > 0.05 && plant[4] < 1.0, "plant box inset: {:?}", plant);
        // slabs pick as half cells (no grabbing the empty top half)
        let slab = pick_boxes(BlockState(b::PLANKS).with_shape(crate::Shape::SlabBottom));
        assert_eq!(slab, &[[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]]);
        // stairs carry both boxes through
        assert_eq!(pick_boxes(BlockState(b::PLANKS).with_shape(crate::Shape::StairNorth)).len(), 2);
    }

    #[test]
    fn mod_blocks_register_and_behave() {        assert!(register_mod_block(250, ModBlockDef {
            name: "ember_ores:ember_ore".into(),
            solid: true,
            opaque: true,
            drop: Some("ember_ores:ember_ingot".into()), light: 0 }));
        assert!(register_mod_block(250, ModBlockDef {
            name: "ember_ores:ember_ore".into(),
            solid: true,
            opaque: true,
            drop: Some("ember_ores:ember_ingot".into()), light: 0 }), "idempotent re-register");
        assert!(!register_mod_block(250, ModBlockDef {
            name: "other:clash".into(), solid: true, opaque: true, drop: None, light: 0 }), "id collision with a different mod rejected");
        assert!(!register_mod_block(5, ModBlockDef {
            name: "low:id".into(), solid: true, opaque: true, drop: None, light: 0 }), "vanilla id range rejected");
        assert!(is_solid(BlockState(250)));
        assert!(is_opaque(BlockState(250)));
        assert_eq!(block::name(250), "ember_ores:ember_ore");
        assert_eq!(mod_block(250).unwrap().drop.as_deref(), Some("ember_ores:ember_ingot"));
    }

    #[test]
    fn all_blocks_named() {
        for id in 0..=MAX_VANILLA_BLOCK {
            assert_ne!(block::name(id), "Unknown", "id {} unnamed", id);
        }
        assert_eq!(block::name(MAX_VANILLA_BLOCK + 1), "Unknown");
    }

    #[test]
    fn known_block_validation() {
        assert!(is_known_block(block::AIR));
        assert!(is_known_block(block::STONE));
        assert!(is_known_block(MAX_VANILLA_BLOCK));
        assert!(!is_known_block(MAX_VANILLA_BLOCK + 1));
        assert!(!is_known_block(MOD_BLOCK_BASE - 1), "vanilla/mod gap is unknown");
        assert!(!is_known_block(MOD_BLOCK_BASE + 500_000), "unregistered mod id");
        assert!(register_mod_block(9100, ModBlockDef {
            name: "server_test:probe".into(), solid: true, opaque: true, drop: None, light: 0 }));
        assert!(is_known_block(9100), "registered mod id");
    }
}
