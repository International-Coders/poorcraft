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
}

/// Mod blocks start here, far above the vanilla range.
pub const MOD_BLOCK_BASE: u32 = 100;

/// Highest contiguous vanilla block id (machine/ore ids included).
pub const MAX_VANILLA_BLOCK: u32 = 41;

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
    // industrial ores
    pub const COPPER_ORE: u32 = 32;
    pub const TIN_ORE: u32 = 33;
    pub const BAUXITE_ORE: u32 = 34;
    pub const SULFUR_ORE: u32 = 35;

    pub fn name(id: u32) -> &'static str {
        if let Some(def) = crate::registry::mod_block(id) {
            // names of registered mods outlive the lookup via leak (bounded by the mod set)
            return Box::leak(def.name.into_boxed_str());
        }
        match id {
            AIR => "Air",
            STONE => "Stone",
            GRASS => "Grass",
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
    id != block::AIR && id != block::WATER && id != block::TORCH && id != block::LANTERN
}

/// Blocks that hide the neighboring face when meshing. Air, water and leaves
/// let faces behind them render.
pub fn is_opaque(b: BlockState) -> bool {
    let id = b.id();
    if id >= MOD_BLOCK_BASE {
        return mod_block(id).map(|d| d.opaque).unwrap_or(true);
    }
    id != block::AIR && id != block::WATER && id != block::LEAVES && id != block::TORCH
        && id != block::LANTERN && id != block::GLASS && id != block::ICE
}

/// Blocks the crosshair can target (mining/placing raycast hits).
pub fn is_targetable(b: BlockState) -> bool {
    b.id() != block::AIR && b.id() != block::WATER
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
        assert!(register_mod_block(150, ModBlockDef {
            name: "ember_ores:ember_ore".into(),
            solid: true,
            opaque: true,
            drop: Some("ember_ores:ember_ingot".into()),
        }));
        assert!(register_mod_block(150, ModBlockDef {
            name: "ember_ores:ember_ore".into(),
            solid: true,
            opaque: true,
            drop: Some("ember_ores:ember_ingot".into()),
        }), "idempotent re-register");
        assert!(!register_mod_block(150, ModBlockDef {
            name: "other:clash".into(), solid: true, opaque: true, drop: None,
        }), "id collision with a different mod rejected");
        assert!(!register_mod_block(5, ModBlockDef {
            name: "low:id".into(), solid: true, opaque: true, drop: None,
        }), "vanilla id range rejected");
        assert!(is_solid(BlockState(150)));
        assert!(is_opaque(BlockState(150)));
        assert_eq!(block::name(150), "ember_ores:ember_ore");
        assert_eq!(mod_block(150).unwrap().drop.as_deref(), Some("ember_ores:ember_ingot"));
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
            name: "server_test:probe".into(), solid: true, opaque: true, drop: None,
        }));
        assert!(is_known_block(9100), "registered mod id");
    }
}
