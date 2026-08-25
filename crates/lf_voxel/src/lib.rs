pub mod meshing;

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
