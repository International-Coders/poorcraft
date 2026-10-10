//! P34 construction: blueprints — two-corner capture, file persistence,
//! material bills, paste. Pure world reads so tests and the client share
//! one path.

use lf_voxel::{BlockState, World};
use serde::{Deserialize, Serialize};

/// Largest capture cube per side (paste cost stays reviewable).
pub const MAX_SIDE: i32 = 16;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Blueprint {
    pub sx: u16,
    pub sy: u16,
    pub sz: u16,
    /// Blocks in y-major order (for y in 0..sy, for z in 0..sz, for x).
    pub blocks: Vec<BlockState>,
}

impl Blueprint {
    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockState {
        if x < 0 || y < 0 || z < 0 || x >= self.sx as i32 || y >= self.sy as i32 || z >= self.sz as i32 {
            return BlockState::AIR;
        }
        let i = (y as usize) * self.sz as usize * self.sx as usize
            + (z as usize) * self.sx as usize
            + x as usize;
        self.blocks.get(i).copied().unwrap_or(BlockState::AIR)
    }

    /// The item bill for pasting: (item id, count) per non-air block,
    /// derived from the drop table so paste consumes real materials.
    pub fn bill(&self) -> Vec<(String, u16)> {
        let mut bill: Vec<(String, u16)> = Vec::new();
        for b in &self.blocks {
            if b.id() == 0 {
                continue;
            }
            if let Some(item) = crate::items::block_drop(b.id()) {
                match bill.iter_mut().find(|(id, _)| *id == item) {
                    Some((_, n)) => *n += 1,
                    None => bill.push((item, 1)),
                }
            }
        }
        bill
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let bytes = bincode::serialize(self).map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    }

    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        bincode::deserialize(&bytes).map_err(|e| e.to_string())
    }
}

/// Capture the axis-aligned box between two corners (inclusive), clamped
/// to MAX_SIDE per axis (the min corner wins when clamping).
pub fn capture(world: &World, a: (i32, i32, i32), b: (i32, i32, i32)) -> Blueprint {
    let min = (a.0.min(b.0), a.1.min(b.1), a.2.min(b.2));
    let max = (a.0.max(b.0), a.1.max(b.1), a.2.max(b.2));
    let sx = (max.0 - min.0 + 1).clamp(1, MAX_SIDE) as u16;
    let sy = (max.1 - min.1 + 1).clamp(1, MAX_SIDE) as u16;
    let sz = (max.2 - min.2 + 1).clamp(1, MAX_SIDE) as u16;
    let mut blocks = Vec::with_capacity(sx as usize * sy as usize * sz as usize);
    for y in 0..sy as i32 {
        for z in 0..sz as i32 {
            for x in 0..sx as i32 {
                blocks.push(world.get_block(min.0 + x, min.1 + y, min.2 + z));
            }
        }
    }
    Blueprint { sx, sy, sz, blocks }
}

/// Paste the blueprint with its min corner at `at`. Returns the blocks
/// actually placed (cells that were air); the caller consumes the bill
/// for exactly these.
pub fn paste_targets(world: &World, bp: &Blueprint, at: (i32, i32, i32)) -> Vec<((i32, i32, i32), BlockState)> {
    let mut out = Vec::new();
    for y in 0..bp.sy as i32 {
        for z in 0..bp.sz as i32 {
            for x in 0..bp.sx as i32 {
                let b = bp.get(x, y, z);
                if b.id() == 0 {
                    continue;
                }
                let cell = (at.0 + x, at.1 + y, at.2 + z);
                if world.get_block(cell.0, cell.1, cell.2) == BlockState::AIR {
                    out.push((cell, b));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use lf_voxel::registry::block;

    fn world_with_box() -> (World, (i32, i32, i32), (i32, i32, i32)) {
        let mut w = World::new();
        w.ensure_chunk(0, 0);
        // a 3x2x1 stone step with a planks slab on top
        for x in 0..3 {
            for y in 0..2 {
                w.set_block(x, y + 64, 4, BlockState(block::STONE));
            }
        }
        w.set_block(1, 66, 4, BlockState(block::STONE).with_shape(lf_voxel::Shape::SlabTop));
        (w, (0, 64, 4), (2, 66, 4))
    }

    #[test]
    fn capture_round_trips_shapes_and_files() {
        let (w, a, b) = world_with_box();
        let bp = capture(&w, a, b);
        assert_eq!((bp.sx, bp.sy, bp.sz), (3, 3, 1), "3x3x1 box");
        assert_eq!(bp.get(1, 2, 0).shape(), lf_voxel::Shape::SlabTop, "shapes survive capture");
        // file round trip in a temp dir
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.bp");
        bp.save(&path).unwrap();
        let loaded = Blueprint::load(&path).unwrap();
        assert_eq!(loaded.blocks, bp.blocks);
        assert_eq!(loaded.get(0, 0, 0).id(), block::STONE);
    }

    #[test]
    fn bill_counts_materials_and_paste_targets_skip_occupied() {
        let (w, a, b) = world_with_box();
        let bp = capture(&w, a, b);
        let bill = bp.bill();
        assert_eq!(bill.iter().find(|(id, _)| id == "stone").map(|(_, n)| *n), Some(7), "6 stone + the slab counts as stone");
        assert_eq!(bill.iter().find(|(id, _)| id == "planks"), None);
        // pasting into the SAME spot: everything is occupied -> nothing placed
        let targets = paste_targets(&w, &bp, a);
        assert!(targets.is_empty(), "occupied cells are skipped");
        // pasting beside it places everything
        let targets = paste_targets(&w, &bp, (10, 64, 4));
        assert_eq!(targets.len(), 7);
    }
}
