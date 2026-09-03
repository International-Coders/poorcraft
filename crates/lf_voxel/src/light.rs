use std::collections::VecDeque;

use crate::registry;
use crate::world::{ChunkColumn, World};

pub const MAX_LIGHT: u8 = 15;

/// RGB emission of light-emitting blocks, one 0..15 channel each. The
/// palette is material-led: flame is warm, Lumen is cool, radiation is
/// green, while ordinary mod light remains neutral for compatibility.
pub fn emission_rgb(block_id: u32) -> [u8; 3] {
    // mod blocks carry their own emission (P34: the modapi `light`
    // field finally reaches the light engine)
    if block_id >= registry::MOD_BLOCK_BASE {
        if let Some(def) = registry::mod_block(block_id) {
            let light = def.light.min(15);
            return [light; 3];
        }
    }
    match block_id {
        registry::block::TORCH => [14, 11, 7],
        registry::block::EMBER_TORCH => [15, 7, 2],
        registry::block::LUMEN_TORCH => [7, 13, 15],
        registry::block::FIREPLACE => [15, 8, 3],
        registry::block::LANTERN => [15, 13, 9],
        // meltdown residue glows an unhealthy green (P32)
        registry::block::RADIATION => [3, 7, 2],
        // the crossover light block: fuelless, full-bright (P33)
        registry::block::LUMEN_BLOCK => [9, 14, 15],
        // Covenant altar stone: warm amber, muted per SKIN_MANIFEST (C1)
        registry::block::EMBER_GLOWSTONE => [8, 5, 2],
        // ui-world-craft D3: lava lakes light the deep caves
        registry::block::LAVA => [12, 5, 2],
        registry::block::LANTERN_HANGING => [15, 13, 9],
        _ => [0; 3],
    }
}

/// Brightest emission channel, retained for callers that only need to know
/// whether a block glows or its approximate radius.
pub fn emission(block_id: u32) -> u8 {
    emission_rgb(block_id).into_iter().max().unwrap_or(0)
}

/// Vertex-light packing keeps the legacy sky and scalar-block nibbles in
/// place: R=bits 0..3, sky=4..7, G=8..11, B=12..15. Existing `0xF0`
/// full-sky vertices therefore remain valid while colored light consumes
/// previously unused bits without growing the vertex format.
pub fn pack_light(sky: u8, block: [u8; 3]) -> u32 {
    (block[0].min(15) as u32)
        | ((sky.min(15) as u32) << 4)
        | ((block[1].min(15) as u32) << 8)
        | ((block[2].min(15) as u32) << 12)
}

pub fn unpack_sky(light: u32) -> u8 {
    ((light >> 4) & 15) as u8
}

pub fn unpack_block_rgb(light: u32) -> [u8; 3] {
    [
        (light & 15) as u8,
        ((light >> 8) & 15) as u8,
        ((light >> 12) & 15) as u8,
    ]
}

/// Per-column light: scalar sky plus RGB block light over 16x256x16 cells.
#[derive(Clone, Debug)]
pub struct ColumnLight {
    pub sky: Vec<u8>,
    pub block_rgb: Vec<[u8; 3]>,
}

impl ColumnLight {
    fn new() -> Self {
        Self {
            sky: vec![0; 16 * 256 * 16],
            block_rgb: vec![[0; 3]; 16 * 256 * 16],
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
        self.block_rgb_at(x, y, z).into_iter().max().unwrap_or(0)
    }

    pub fn block_rgb_at(&self, x: usize, y: usize, z: usize) -> [u8; 3] {
        self.block_rgb[Self::idx(x, y, z)]
    }
}

#[derive(Copy, Clone)]
struct Cell {
    x: usize,
    y: usize,
    z: usize,
    level: u8,
}

#[derive(Copy, Clone)]
struct ColorCell {
    x: usize,
    y: usize,
    z: usize,
    level: [u8; 3],
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

    // One pass over loaded sections in the 48x256x48 volume: opacity bitset
    // + emitters. Reading sections directly avoids a hash lookup and world-
    // coordinate division per voxel; empty/transparent sections are skipped.
    let vol = V * V * 256;
    let mut opaque = vec![false; vol];
    let mut sky = vec![0u8; vol];
    let mut block = vec![[0u8; 3]; vol];
    let mut block_queue: VecDeque<ColorCell> = VecDeque::new();
    let mut sky_queue: VecDeque<Cell> = VecDeque::new();

    for dcx in -1..=1 {
        for dcz in -1..=1 {
            let Some(column) = world.chunk(cx + dcx, cz + dcz) else { continue };
            let x0 = ((dcx + 1) as usize) * 16;
            let z0 = ((dcz + 1) as usize) * 16;
            for (section_y, section) in column.sections.iter().enumerate() {
                if !section.palette.iter().any(|b| {
                    registry::is_opaque(*b) || emission(b.id()) > 0
                }) {
                    continue;
                }
                for ly in 0..16usize {
                    let y = section_y * 16 + ly;
                    for lz in 0..16usize {
                        for lx in 0..16usize {
                            let (x, z) = (x0 + lx, z0 + lz);
                            let i = vidx(x, y, z);
                            let b = section.get(lx, ly, lz);
                            opaque[i] = registry::is_opaque(b);
                            let e = emission_rgb(b.id());
                            if e != [0; 3] {
                                block[i] = e;
                                block_queue.push_back(ColorCell { x, y, z, level: e });
                            }
                        }
                    }
                }
            }
        }
    }

    for x in 0..V {
        for z in 0..V {
            // Sky pours down only until the first opaque cell.
            let mut y = 255usize;
            loop {
                let i = vidx(x, y, z);
                if opaque[i] {
                    break;
                }
                sky[i] = MAX_LIGHT;
                if y == 0 {
                    break;
                }
                y -= 1;
            }
            // Frontier: spill from the lowest transparent sky-lit cell, never
            // from the opaque blocker itself (which leaked light through roofs).
            let frontier_y = if opaque[vidx(x, y, z)] { y + 1 } else { y };
            if frontier_y < 256 {
                sky_queue.push_back(Cell { x, y: frontier_y, z, level: MAX_LIGHT });
            }
        }
    }

    // Scalar skylight flood, free to cross chunk borders.
    let flood = |channel: &mut Vec<u8>, queue: &mut VecDeque<Cell>| {
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

    // RGB block light uses max-compositing per channel. Different source
    // colors therefore blend where their volumes overlap, while every
    // channel still loses one level per transparent cell.
    while let Some(cell) = block_queue.pop_front() {
        let next = [
            cell.level[0].saturating_sub(1),
            cell.level[1].saturating_sub(1),
            cell.level[2].saturating_sub(1),
        ];
        if next == [0; 3] {
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
            let mut changed = false;
            for channel in 0..3 {
                if block[i][channel] < next[channel] {
                    block[i][channel] = next[channel];
                    changed = true;
                }
            }
            if changed {
                block_queue.push_back(ColorCell { x: nxu, y: nyu, z: nzu, level: block[i] });
            }
        }
    }

    // Extract the center column's slice.
    let mut light = ColumnLight::new();
    for lx in 0..16usize {
        for lz in 0..16usize {
            for y in 0..256usize {
                let src = vidx(lx + 16, y, lz + 16);
                let dst = ColumnLight::idx(lx, y, lz);
                light.sky[dst] = sky[src];
                light.block_rgb[dst] = block[src];
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

    #[test]
    fn packed_rgb_light_preserves_the_legacy_sky_nibble() {
        assert_eq!(pack_light(15, [0; 3]), 0xF0,
            "existing full-sky vertices must keep their exact encoding");
        let packed = pack_light(12, [15, 8, 3]);
        assert_eq!(unpack_sky(packed), 12);
        assert_eq!(unpack_block_rgb(packed), [15, 8, 3]);
    }

    #[test]
    fn source_materials_have_distinct_light_palettes() {
        let ordinary = emission_rgb(registry::block::TORCH);
        let ember = emission_rgb(registry::block::EMBER_TORCH);
        let lumen = emission_rgb(registry::block::LUMEN_TORCH);
        let radiation = emission_rgb(registry::block::RADIATION);
        assert!(ordinary[0] > ordinary[2], "ordinary flame is warm");
        assert!(ember[0] > ordinary[0] && ember[2] < ordinary[2], "ember burns hotter/redder");
        assert!(lumen[2] > lumen[0] && lumen[1] > lumen[0], "lumen is cool cyan");
        assert!(radiation[1] > radiation[0] && radiation[1] > radiation[2], "radiation is green");
        assert_eq!(emission_rgb(registry::block::FIREPLACE), [15, 8, 3]);
    }

    #[test]
    fn colored_sources_fall_off_and_blend_per_channel() {
        let mut w = flat_world_with_floor();
        w.set_block(4, 1, 8, BlockState(registry::block::EMBER_TORCH));
        w.set_block(12, 1, 8, BlockState(registry::block::LUMEN_TORCH));
        let col = w.chunk(0, 0).unwrap().clone();
        let light = compute_column_light(&w, 0, 0, &col);
        assert_eq!(light.block_rgb_at(6, 1, 8), [13, 7, 9],
            "red ember and distant cyan light max-blend by channel");
        assert_eq!(light.block_rgb_at(8, 1, 8), [11, 9, 11],
            "the overlap carries both source colors");
        assert_eq!(light.block_rgb_at(8, 0, 8), [0; 3], "opaque floor blocks every channel");
    }

    #[test]
    fn roofed_fireplace_is_discovered_and_lights_the_room() {
        let mut w = flat_world_with_floor();
        for x in 0..16 {
            for z in 0..16 {
                w.set_block(x, 8, z, BlockState::STONE);
            }
        }
        // Seal the room as well as roofing it. A roof with open sides should
        // correctly receive attenuated skylight around its edges.
        for y in 1..8 {
            for p in 0..16 {
                w.set_block(0, y, p, BlockState::STONE);
                w.set_block(15, y, p, BlockState::STONE);
                w.set_block(p, y, 0, BlockState::STONE);
                w.set_block(p, y, 15, BlockState::STONE);
            }
        }
        w.set_block(8, 1, 8, BlockState(registry::block::FIREPLACE));
        let col = w.chunk(0, 0).unwrap().clone();
        let light = compute_column_light(&w, 0, 0, &col);
        assert_eq!(light.sky_at(8, 2, 8), 0, "the stone roof blocks sky");
        assert_eq!(light.block_rgb_at(9, 1, 8), [14, 7, 2],
            "an indoor fireplace must be found below the roof and propagate");
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
