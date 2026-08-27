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
pub const MAX_VANILLA_BLOCK: u32 = 105;

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
            _ => "Unknown",
        }
    }
}

/// Foliage blocks: non-opaque, sway in the wind, alpha-cutout rendering.
pub fn is_leaf(id: u32) -> bool {
    use block as b;
    id == b::LEAVES || id == b::BIRCH_LEAVES || id == b::SPRUCE_LEAVES
        || id == b::DARK_LEAVES || id == b::CHERRY_LEAVES || id == b::PALE_LEAVES
}

/// Blocks entities collide with. Water is not solid; leaves are.
pub fn is_solid(b: BlockState) -> bool {
    let id = b.id();
    if id >= MOD_BLOCK_BASE {
        return mod_block(id).map(|d| d.solid).unwrap_or(true);
    }
    id != block::AIR && id != block::WATER && id != block::OIL && id != block::TORCH
        && id != block::LANTERN && id != block::FLOWER && id != block::LANTERN_HANGING
        && !is_banner(id)
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
    id != block::AIR && id != block::WATER && id != block::OIL && id != block::LEAVES
        && id != block::TORCH && id != block::LANTERN && id != block::GLASS
        && id != block::ICE && id != block::FLOWER
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
        | b::BANNER_FREEHOLDS | b::BANNER_ASHEN | b::BANNER_NAMELESS)
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

/// Blocks the crosshair can target (mining/placing raycast hits). Fluids
/// are scooped with the bucket via the face-adjacent cell instead.
pub fn is_targetable(b: BlockState) -> bool {
    b.id() != block::AIR && b.id() != block::WATER && b.id() != block::OIL
}

/// Cross-plants: non-solid cutout blocks that sit on the ground. Banners
/// render through the same flat-quad path (sign-style, per SKIN_MANIFEST).
pub fn is_plant(id: u32) -> bool {
    id == block::FLOWER || is_banner(id)
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

    #[test]
    fn mod_blocks_register_and_behave() {
        assert!(register_mod_block(250, ModBlockDef {
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
