use crate::BlockState;

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

    pub fn name(id: u32) -> &'static str {
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
            _ => "Unknown",
        }
    }
}

/// Blocks entities collide with. Water is not solid; leaves are.
pub fn is_solid(b: BlockState) -> bool {
    let id = b.id();
    id != block::AIR && id != block::WATER && id != block::TORCH && id != block::LANTERN
}

/// Blocks that hide the neighboring face when meshing. Air, water and leaves
/// let faces behind them render.
pub fn is_opaque(b: BlockState) -> bool {
    let id = b.id();
    id != block::AIR && id != block::WATER && id != block::LEAVES && id != block::TORCH
        && id != block::LANTERN && id != block::GLASS
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
    fn all_blocks_named() {
        for id in 0..=18u32 {
            assert_ne!(block::name(id), "Unknown", "id {} unnamed", id);
        }
    }
}
