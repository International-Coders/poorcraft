use std::collections::VecDeque;

use crate::registry;
use crate::world::{ChunkColumn, World};

pub const MAX_LIGHT: u8 = 15;

/// Emission strength of light-emitting blocks.
pub fn emission(block_id: u32) -> u8 {
    match block_id {
        registry::block::TORCH => 14,
        registry::block::LANTERN => 15,
        // meltdown residue glows an unhealthy green (P32)
        registry::block::RADIATION => 7,
        // the crossover light block: fuelless, full-bright (P33)
        registry::block::LUMEN_BLOCK => 15,
        _ => 0,
    }
}

/// Per-column light: two 16x256x16 channels (sky, block).
#[derive(Clone, Debug)]
pub struct ColumnLight {
    pub sky: Vec<u8>,
    pub block: Vec<u8>,
}

impl ColumnLight {
    fn new() -> Self {
        Self {
            sky: vec![0; 16 * 256 * 16],
            block: vec![0; 16 * 256 * 16],
        }
    }

    fn idx(x: usize, y: usize, z: usize) -> usize {
        // y must own the largest stride: the column is 256 tall, so a
        // y*16 stride would collide with z cells.
        x + z * 16 + y * 256
    }

    pub fn sky_at(&self, x: usize, y: usize, z: usize) -> u8 {
        self.sky[Self::idx(x, y, z)]
    }

    pub fn block_at(&self, x: usize, y: usize, z: usize) -> u8 {
        self.block[Self::idx(x, y, z)]
    }
}

#[derive(Copy, Clone)]
struct Cell {
    x: usize,
    y: usize,
    z: usize,
    level: u8,
}

/// Side length of the lighting neighborhood: 3 chunk columns (48 blocks).
const V: usize = 48;

#[inline]
fn vidx(x: usize, y: usize, z: usize) -> usize {
    x + z * V + y * V * V
}

/// Compute sky and block light for one chunk column via a 3x3-column
/// neighborhood (P28: cross-chunk seams eliminated). Torch light and sky
/// spill flood across chunk borders inside the volume; only the center
/// column's slice is kept. `col` is retained for signature compatibility —
/// all block data comes from the world (identical for the center column).
pub fn compute_column_light(world: &World, cx: i32, cz: i32, col: &ChunkColumn) -> ColumnLight {
    let _ = col;
    let wx0 = cx * 16 - 16;
    let wz0 = cz * 16 - 16;

    // One pass over the 48x256x48 volume: opacity bitset + emitters.
    let vol = V * V * 256;
    let mut opaque = vec![false; vol];
    let mut sky = vec![0u8; vol];
    let mut block = vec![0u8; vol];
    let mut block_queue: VecDeque<Cell> = VecDeque::new();
    let mut sky_queue: VecDeque<Cell> = VecDeque::new();

    for x in 0..V {
        for z in 0..V {
            let wx = wx0 + x as i32;
            let wz = wz0 + z as i32;
            // sky pour down the volume column
            let mut y = 255usize;
            loop {
                let i = vidx(x, y, z);
                let b = world.get_block(wx, y as i32, wz);
                opaque[i] = registry::is_opaque(b);
                let e = emission(b.id());
                if e > 0 {
                    block[i] = e;
                    block_queue.push_back(Cell { x, y, z, level: e });
                }
                if opaque[i] {
                    break;
                }
                sky[i] = MAX_LIGHT;
                if y == 0 {
                    break;
                }
                y -= 1;
            }
            // frontier: the lowest lit cell spills sideways into shade
            if y > 1 {
                sky_queue.push_back(Cell { x, y, z, level: MAX_LIGHT });
            }
        }
    }

    // BFS flood shared by both channels, free to cross chunk borders.
    let mut flood = |channel: &mut Vec<u8>, queue: &mut VecDeque<Cell>| {
        while let Some(cell) = queue.pop_front() {
            if cell.level <= 1 {
                continue;
            }
            for (dx, dy, dz) in [
                (1i32, 0i32, 0i32), (-1, 0, 0),
                (0, 1, 0), (0, -1, 0),
                (0, 0, 1), (0, 0, -1),
            ] {
                let nx = cell.x as i32 + dx;
                let ny = cell.y as i32 + dy;
                let nz = cell.z as i32 + dz;
                if nx < 0 || nx >= V as i32 || nz < 0 || nz >= V as i32 || ny < 0 || ny > 255 {
                    continue;
                }
                let (nxu, nyu, nzu) = (nx as usize, ny as usize, nz as usize);
                let i = vidx(nxu, nyu, nzu);
                if opaque[i] {
                    continue;
                }
                let next = cell.level - 1;
                if channel[i] < next {
                    channel[i] = next;
                    queue.push_back(Cell { x: nxu, y: nyu, z: nzu, level: next });
                }
            }
        }
    };

    flood(&mut sky, &mut sky_queue);
    flood(&mut block, &mut block_queue);

    // Extract the center column's slice.
    let mut light = ColumnLight::new();
    for lx in 0..16usize {
        for lz in 0..16usize {
            for y in 0..256usize {
                let src = vidx(lx + 16, y, lz + 16);
                let dst = ColumnLight::idx(lx, y, lz);
                light.sky[dst] = sky[src];
                light.block[dst] = block[src];
            }
        }
    }
    light
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockState;

    fn flat_world_with_floor() -> World {
        let mut w = World::new();
        w.ensure_chunk(0, 0);
        for lx in 0..16 {
            for lz in 0..16 {
                w.set_block(lx, 0, lz, BlockState::STONE);
            }
        }
        w
    }

    #[test]
    fn sky_light_fills_open_air_and_stops_at_ground() {
        let w = flat_world_with_floor();
        let col = w.chunk(0, 0).unwrap();
        let light = compute_column_light(&w, 0, 0, col);
        assert_eq!(light.sky_at(8, 200, 8), 15);
        assert_eq!(light.sky_at(8, 1, 8), 15);
        // inside the floor there is no light
        assert_eq!(light.sky_at(8, 0, 8), 0);
    }

    #[test]
    fn torch_emits_and_falls_off() {
        let mut w = flat_world_with_floor();
        w.set_block(8, 1, 8, BlockState(registry::block::TORCH));
        let col = w.chunk(0, 0).unwrap().clone();
        let light = compute_column_light(&w, 0, 0, &col);
        assert_eq!(light.block_at(8, 1, 8), 14);
        assert_eq!(light.block_at(10, 1, 8), 12, "2 blocks away should lose 2 levels");
        assert_eq!(light.block_at(15, 1, 8), 7, "7 blocks away should lose 7 levels");
        assert_eq!(light.block_at(8, 1, 7), 13);
        // stone blocks the spread (below floor)
        assert_eq!(light.block_at(8, 0, 8), 0, "opaque floor should not receive light");
    }

    /// P28 regression: light must cross chunk borders — a torch on one
    /// column's edge lights the neighboring column (the old per-column BFS
    /// left a hard seam).
    #[test]
    fn torch_light_crosses_chunk_borders() {
        let mut w = World::new();
        w.ensure_chunk(0, 0);
        w.ensure_chunk(1, 0);
        w.ensure_chunk(-1, 0);
        for cx in [-1, 0, 1] {
            for lx in 0..16 {
                for lz in 0..16 {
                    w.set_block(cx * 16 + lx, 0, lz, BlockState::STONE);
                }
            }
        }
        // torch on the east edge of column (0,0), block 15
        w.set_block(15, 1, 8, BlockState(registry::block::TORCH));
        let col1 = w.chunk(1, 0).unwrap().clone();
        let light1 = compute_column_light(&w, 1, 0, &col1);
        assert_eq!(light1.block_at(0, 1, 8), 13,
            "light spills one block into the eastern neighbor across the border");
        assert_eq!(light1.block_at(3, 1, 8), 10, "and falls off with distance");
        // symmetric: the western column also receives nothing (it's dark)
        let colw = w.chunk(-1, 0).unwrap().clone();
        let lightw = compute_column_light(&w, -1, 0, &colw);
        assert_eq!(lightw.block_at(15, 1, 8), 0, "no light on the far side of the torch column");
    }

    #[test]
    fn sky_floods_into_overhangs() {
        // roof at y=10 with a gap at (8,z), open below: light should spill
        let mut w = World::new();
        w.ensure_chunk(0, 0);
        for lx in 0..16 {
            for lz in 0..16 {
                w.set_block(lx, 10, lz, BlockState::STONE);
                w.set_block(lx, 0, lz, BlockState::STONE);
            }
        }
        w.set_block(8, 10, 8, BlockState::AIR); // skylight shaft
        let col = w.chunk(0, 0).unwrap().clone();
        let light = compute_column_light(&w, 0, 0, &col);
        assert_eq!(light.sky_at(8, 9, 8), 15, "directly under the shaft is lit");
        assert_eq!(light.sky_at(8, 5, 8), 15);
        assert!(light.sky_at(10, 5, 8) > 0 && light.sky_at(10, 5, 8) < 15,
            "sideways spill loses levels, got {}", light.sky_at(10, 5, 8));
        assert_eq!(light.sky_at(8, 0, 8), 0, "floor stays unlit");
    }
}
