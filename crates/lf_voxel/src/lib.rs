pub mod light;
pub mod meshing;
pub mod raycast;
pub mod registry;
pub mod persistence;
pub mod world;

pub use world::{ChunkColumn, World};

use serde::{Serialize, Deserialize};

/// A single voxel block ID packed with state.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockState(pub u32);

impl BlockState {
    pub const AIR: Self = Self(0);
    pub const STONE: Self = Self(1);
    pub const GRASS: Self = Self(2);
    pub const DIRT: Self = Self(3);

    pub fn id(self) -> u32 {
        self.0 & 0x00FFFFFF
    }

    pub fn state_flags(self) -> u8 {
        ((self.0 >> 24) & 0xFF) as u8
    }
}

/// Flow distance of a water block: 0 = source, 1..=7 = how far the cell is
/// from the source feeding it (carried in the state-flags nibble).
pub fn water_level(state: BlockState) -> u8 {
    if state.id() == registry::block::WATER {
        state.state_flags() & 0x0F
    } else {
        0
    }
}

/// A water block with the given flow level (0 = source).
pub fn water_with_level(level: u8) -> BlockState {
    BlockState(registry::block::WATER | (((level as u32) & 0x0F) << 24))
}

/// Flow distance of a crude-oil block (same packing as water).
pub fn oil_level(state: BlockState) -> u8 {
    if state.id() == registry::block::OIL {
        state.state_flags() & 0x0F
    } else {
        0
    }
}

/// A crude-oil block with the given flow level (0 = source).
pub fn oil_with_level(level: u8) -> BlockState {
    BlockState(registry::block::OIL | (((level as u32) & 0x0F) << 24))
}

/// Block shape (P34 construction): lives in the high nibble of the state
/// flags (bits 28..31; water/oil levels own the low nibble). Shape 0 is
/// the plain cube every existing block uses, so old saves are unaffected.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Shape {
    Cube,
    SlabBottom,
    SlabTop,
    /// High half toward -Z (the direction you walk UP is north).
    StairNorth,
    /// High half toward +Z.
    StairSouth,
    /// High half toward -X.
    StairWest,
    /// High half toward +X.
    StairEast,
}

impl Shape {
    pub fn nibble(self) -> u8 {
        match self {
            Shape::Cube => 0,
            Shape::SlabBottom => 1,
            Shape::SlabTop => 2,
            Shape::StairNorth => 3,
            Shape::StairSouth => 4,
            Shape::StairWest => 5,
            Shape::StairEast => 6,
        }
    }

    pub fn from_nibble(n: u8) -> Shape {
        match n {
            1 => Shape::SlabBottom,
            2 => Shape::SlabTop,
            3 => Shape::StairNorth,
            4 => Shape::StairSouth,
            5 => Shape::StairWest,
            6 => Shape::StairEast,
            _ => Shape::Cube,
        }
    }
}

impl BlockState {
    /// This block's shape (P34). Plain blocks are [`Shape::Cube`].
    pub fn shape(self) -> Shape {
        Shape::from_nibble(self.state_flags() >> 4)
    }

    /// The same block with a different shape (fluid levels are preserved).
    pub fn with_shape(self, shape: Shape) -> BlockState {
        BlockState((self.0 & 0x0FFFFFFF) | ((shape.nibble() as u32) << 28))
    }
}

/// A 16x16x16 section of voxels with palette compression.
pub const SECTION_SIZE: usize = 16;
pub const SECTION_VOLUME: usize = SECTION_SIZE * SECTION_SIZE * SECTION_SIZE;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoxelSection {
    pub palette: Vec<BlockState>,
    pub indices: Vec<u16>, // indices into palette, or direct if palette is empty
}

impl VoxelSection {
    pub fn new_empty() -> Self {
        Self {
            palette: vec![BlockState::AIR],
            indices: vec![0; SECTION_VOLUME],
        }
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockState {
        let idx = x + y * SECTION_SIZE + z * SECTION_SIZE * SECTION_SIZE;
        let palette_idx = self.indices[idx] as usize;
        self.palette.get(palette_idx).copied().unwrap_or(BlockState::AIR)
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, block: BlockState) {
        let idx = x + y * SECTION_SIZE + z * SECTION_SIZE * SECTION_SIZE;
        let palette_idx = if let Some(pos) = self.palette.iter().position(|&b| b == block) {
            pos
        } else {
            let pos = self.palette.len();
            self.palette.push(block);
            pos
        };
        self.indices[idx] = palette_idx as u16;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voxel_section_roundtrip() {
        let mut section = VoxelSection::new_empty();
        section.set(0, 0, 0, BlockState::GRASS);
        assert_eq!(section.get(0, 0, 0), BlockState::GRASS);
        assert_eq!(section.get(1, 1, 1), BlockState::AIR);
    }
}
