use std::collections::VecDeque;

use crate::registry;
use crate::world::{ChunkColumn, World};

pub const MAX_LIGHT: u8 = 15;

/// Emission strength of light-emitting blocks.
pub fn emission(block_id: u32) -> u8 {
    match block_id {
        registry::block::TORCH => 14,
        registry::block::LANTERN => 15,
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

/// Compute sky and block light for one chunk column. Sky light pours down
/// from above and floods sideways with -1 falloff; block light spreads from
/// emitters the same way. Opacity comes from the world (cross-chunk reads),
/// but sources are confined to this column (seams at chunk borders are a
/// known P3 simplification).
pub fn compute_column_light(world: &World, cx: i32, cz: i32, col: &ChunkColumn) -> ColumnLight {
    let mut light = ColumnLight::new();
    let wx0 = cx * 16;
    let wz0 = cz * 16;

    let opaque_at = |x: i32, y: i32, z: i32| -> bool {
        if y < 0 || y >= 256 {
            return false;
        }
        let (tcx, lx) = (x.div_euclid(16), x.rem_euclid(16) as usize);
        let (tcz, lz) = (z.div_euclid(16), z.rem_euclid(16) as usize);
        if tcx == cx && tcz == cz {
            registry::is_opaque(col.get(lx, y as usize, lz))
        } else {
            registry::is_opaque(world.get_block(x, y, z))
        }
    };

    // --- Sky light: columns of open sky get 15 down to the first opaque block.
    let mut sky_queue: VecDeque<Cell> = VecDeque::new();
    for lx in 0..16usize {
        for lz in 0..16usize {
            let mut y = 255usize;
            loop {
                if registry::is_opaque(col.get(lx, y, lz)) {
                    break;
                }
                light.sky[ColumnLight::idx(lx, y, lz)] = MAX_LIGHT;
                y -= 1;
                if y == 0 {
                    if !registry::is_opaque(col.get(lx, 0, lz)) {
                        light.sky[ColumnLight::idx(lx, 0, lz)] = MAX_LIGHT;
                    }
                    break;
                }
            }
            // Queue lit cells that border unlit space (spread frontier).
            if y > 1 {
                sky_queue.push_back(Cell { x: lx, y, z: lz, level: MAX_LIGHT });
            }
        }
    }

    // --- Block light: emitters seed the queue.
    let mut block_queue: VecDeque<Cell> = VecDeque::new();
    for lx in 0..16usize {
        for lz in 0..16usize {
            for y in 0..256usize {
                let e = emission(col.get(lx, y, lz).id());
                if e > 0 {
                    light.block[ColumnLight::idx(lx, y, lz)] = e;
                    block_queue.push_back(Cell { x: lx, y, z: lz, level: e });
                }
            }
        }
    }

    // --- BFS flood fill shared by both channels.
    let mut flood = |channel: &mut Vec<u8>, queue: &mut VecDeque<Cell>, is_sky: bool| {
        while let Some(cell) = queue.pop_front() {
            if cell.level <= 1 {
                continue;
            }
            let dirs = [
                (1i32, 0i32, 0i32),
                (-1, 0, 0),
                (0, 1, 0),
                (0, -1, 0),
                (0, 0, 1),
                (0, 0, -1),
            ];
            for (dx, dy, dz) in dirs {
                let nx = cell.x as i32 + dx;
                let ny = cell.y as i32 + dy;
                let nz = cell.z as i32 + dz;
                if nx < 0 || nx > 15 || nz < 0 || nz > 15 || ny < 0 || ny > 255 {
                    continue; // stay in this column (cross-chunk seams accepted)
                }
                let (nxu, nyu, nzu) = (nx as usize, ny as usize, nz as usize);
                if registry::is_opaque(col.get(nxu, nyu, nzu)) {
                    continue;
                }
                let next = cell.level - 1;
                let i = ColumnLight::idx(nxu, nyu, nzu);
                if channel[i] < next {
                    channel[i] = next;
                    queue.push_back(Cell { x: nxu, y: nyu, z: nzu, level: next });
                }
                let _ = is_sky;
            }
        }
    };

    flood(&mut light.sky, &mut sky_queue, true);
    flood(&mut light.block, &mut block_queue, false);

    let _ = (wx0, wz0);
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
