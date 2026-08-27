//! Cellular water physics and block gravity (build-pack request; also the
//! P30 Steam-Age fluid groundwork). All functions are pure world transforms
//! so the client can drive them per tick and tests/vistest can run them
//! headless.
//!
//! Water model (Minecraft-flavored, event-driven):
//! - level 0 = source (worldgen oceans/lakes; bucket-placed), levels 1..=7
//!   = flow distance from the feeding source, carried in the block state.
//! - A water cell evaluates: unsupported flow dries up; water falls first
//!   (below-air becomes falling level 1), otherwise spreads horizontally
//!   with level+1 up to [`MAX_SPREAD`].
//! - The client enqueues a cell and its 6 neighbors after every block edit;
//!   each edit re-enqueues its own neighborhood, so floods and recessions
//!   propagate in waves.

use std::collections::VecDeque;

use lf_voxel::registry;
use lf_voxel::{water_level, water_with_level, BlockState, World};

/// Maximum horizontal flow distance from a source.
pub const MAX_SPREAD: u8 = 7;

pub type Cell = (i32, i32, i32);

/// Evaluate one cell of the water simulation, applying edits directly to
/// the world and returning them so the caller can remesh/broadcast and
/// enqueue the affected neighborhoods.
pub fn step_cell(world: &mut World, x: i32, y: i32, z: i32) -> Vec<(Cell, BlockState)> {
    let mut edits = Vec::new();
    let b = world.get_block(x, y, z);
    if b.id() != registry::block::WATER {
        return edits;
    }
    let level = water_level(b);

    // A source is always supported; flowing water needs a shorter-path
    // neighbor or water above it.
    let supported = level == 0
        || world.get_block(x, y + 1, z).id() == registry::block::WATER
        || [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)].iter().any(|&(nx, nz)| {
            let nb = world.get_block(nx, y, nz);
            nb.id() == registry::block::WATER && water_level(nb) < level
        });
    if !supported {
        world.set_block(x, y, z, BlockState::AIR);
        edits.push(((x, y, z), BlockState::AIR));
        return edits;
    }

    // Fall first: water pouring into air (or onto far-flow) goes down,
    // becoming falling water at level 1.
    let below = world.get_block(x, y - 1, z);
    if below.id() == registry::block::AIR {
        world.set_block(x, y - 1, z, water_with_level(1));
        edits.push(((x, y - 1, z), water_with_level(1)));
        return edits;
    }
    if below.id() == registry::block::WATER && water_level(below) > 1 {
        // normalize the column: anything directly under water is fed from
        // above, not from a long horizontal path
        world.set_block(x, y - 1, z, water_with_level(1));
        edits.push(((x, y - 1, z), water_with_level(1)));
        return edits;
    }

    // Blocked below (solid or a source): spread sideways with decay.
    if level < MAX_SPREAD {
        for (nx, nz) in [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)] {
            if world.get_block(nx, y, nz).id() == registry::block::AIR {
                let flowed = water_with_level(level + 1);
                world.set_block(nx, y, nz, flowed);
                edits.push(((nx, y, nz), flowed));
            }
        }
    }
    edits
}

/// Enqueue a cell and its 6 neighbors after an edit at `pos`.
pub fn enqueue_around(queue: &mut VecDeque<Cell>, pos: Cell) {
    let (x, y, z) = pos;
    for p in [
        (x, y, z),
        (x + 1, y, z),
        (x - 1, y, z),
        (x, y + 1, z),
        (x, y - 1, z),
        (x, y, z + 1),
        (x, y, z - 1),
    ] {
        queue.push_back(p);
    }
}

/// Run the event-driven water simulation until the queue drains or
/// `max_cells` evaluations happened (the client calls this with a small
/// budget every tick; tests and vistest call it with a generous one).
/// Returns the number of cells evaluated.
pub fn settle(world: &mut World, queue: &mut VecDeque<Cell>, max_cells: usize) -> usize {
    let mut evaluated = 0;
    while let Some((x, y, z)) = queue.pop_front() {
        if evaluated >= max_cells {
            queue.push_front((x, y, z));
            return evaluated;
        }
        evaluated += 1;
        let edits = step_cell(world, x, y, z);
        for (pos, _) in &edits {
            enqueue_around(queue, *pos);
        }
    }
    evaluated
}

/// Instantly collapse unsupported gravity blocks (sand/dirt-family) in the
/// column at/above `(x, y, z)` — the headless twin of the client's animated
/// [`crate`] FallingBlock entities. Returns how many blocks moved.
pub fn settle_gravity(world: &mut World, x: i32, z: i32) -> usize {
    let mut moved = 0;
    loop {
        let mut any = false;
        for y in (1..(lf_voxel::world::SECTION_COUNT * 16) as i32).rev() {
            let b = world.get_block(x, y, z);
            if !registry::has_gravity(b.id()) {
                continue;
            }
            let below = world.get_block(x, y - 1, z);
            let can_fall = below.id() == registry::block::AIR
                || below.id() == registry::block::WATER
                || !registry::is_solid(below) && !registry::is_opaque(below);
            if can_fall {
                // falling displaces water (and crushes nothing else v1)
                world.set_block(x, y - 1, z, b);
                world.set_block(x, y, z, BlockState::AIR);
                moved += 1;
                any = true;
            }
        }
        if !any {
            return moved;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lf_voxel::registry::block;

    fn world_with_ground() -> World {
        let mut w = World::new();
        for cx in -1..=1 {
            for cz in -1..=1 {
                w.chunks.insert((cx, cz), lf_voxel::ChunkColumn::empty());
            }
        }
        for x in -8..8 {
            for z in -8..8 {
                w.set_block(x, 0, z, BlockState::STONE);
            }
        }
        w
    }

    fn dig(w: &mut World, x: i32, y: i32, z: i32) {
        w.set_block(x, y, z, BlockState::AIR);
    }

    /// Water spreads out from a source and stops at the flow limit.
    #[test]
    fn water_spreads_from_source_and_decays() {
        let mut w = world_with_ground();
        // wall the world so spread is 1-D along +x
        for x in -8..8 {
            w.set_block(x, 1, -1, BlockState::STONE);
            w.set_block(x, 1, 0 - 2 + 1, BlockState::STONE); // z = -1 mirrored
        }
        for z in -1..=1 {
            w.set_block(-1, 1, z, BlockState::STONE);
        }
        w.set_block(0, 1, 0, water_with_level(0));
        let mut q = VecDeque::new();
        enqueue_around(&mut q, (0, 1, 0));
        settle(&mut w, &mut q, 10_000);
        // every cell up the +x line within MAX_SPREAD is water with the
        // expected level; one past the limit is dry
        for lvl in 1..=MAX_SPREAD as i32 {
            let b = w.get_block(lvl, 1, 0);
            assert_eq!(b.id(), block::WATER, "cell at +{} should be water", lvl);
            assert_eq!(water_level(b) as i32, lvl, "level should decay with distance");
        }
        assert_eq!(w.get_block(MAX_SPREAD as i32 + 1, 1, 0), BlockState::AIR,
            "flow must stop at MAX_SPREAD");
    }

    /// Removing the source dries the flow back up.
    #[test]
    fn scooping_the_source_recedes_the_flow() {
        let mut w = world_with_ground();
        for x in -1..=8 {
            w.set_block(x, 1, -1, BlockState::STONE);
            w.set_block(x, 1, 1, BlockState::STONE);
        }
        w.set_block(-1, 1, 0, BlockState::STONE);
        w.set_block(0, 1, 0, water_with_level(0));
        let mut q = VecDeque::new();
        enqueue_around(&mut q, (0, 1, 0));
        settle(&mut w, &mut q, 10_000);
        assert_eq!(w.get_block(5, 1, 0).id(), block::WATER);
        // scoop the source (what a bucket does)
        dig(&mut w, 0, 1, 0);
        let mut q = VecDeque::new();
        enqueue_around(&mut q, (0, 1, 0));
        settle(&mut w, &mut q, 10_000);
        for x in 0..=8 {
            assert_eq!(w.get_block(x, 1, 0), BlockState::AIR,
                "cell {} must dry up without a source", x);
        }
    }

    /// Water on a pillar falls first, then pools on the floor.
    #[test]
    fn water_falls_then_pools() {
        let mut w = World::new();
        w.chunks.insert((0, 0), lf_voxel::ChunkColumn::empty());
        for x in -3..3 {
            for z in -3..3 {
                w.set_block(x, 0, z, BlockState::STONE);
            }
        }
        // pillar top with a source, one cell of floor free around it
        w.set_block(0, 4, 0, BlockState::STONE);
        w.set_block(0, 5, 0, water_with_level(0));
        let mut q = VecDeque::new();
        enqueue_around(&mut q, (0, 5, 0));
        settle(&mut w, &mut q, 10_000);
        assert_eq!(w.get_block(0, 4, 0).id(), block::STONE, "pillar stays");
        assert!(w.get_block(0, 3, 0).id() == block::WATER || w.get_block(0, 1, 0).id() == block::WATER,
            "water must fall off the pillar");
        assert_eq!(w.get_block(0, 1, 0).id(), block::WATER, "water pools on the floor");
    }

    /// Unsupported sand columns collapse; stone does not. The collapse
    /// trigger is the block break under the column (what the client sees).
    #[test]
    fn gravity_blocks_fall_and_stone_does_not() {
        let mut w = world_with_ground();
        for y in 1..=4 {
            w.set_block(0, y, 0, BlockState(block::SAND));
        }
        w.set_block(2, 1, 0, BlockState::STONE); // floater
        dig(&mut w, 0, 0, 0); // break the ground under the column
        let moved = settle_gravity(&mut w, 0, 0);
        assert!(moved >= 4, "the sand column should collapse, moved {}", moved);
        for y in 0..=3 {
            assert_eq!(w.get_block(0, y, 0).id(), block::SAND, "sand at y={}", y);
        }
        assert_eq!(w.get_block(0, 4, 0), BlockState::AIR);
        assert_eq!(w.get_block(2, 1, 0).id(), block::STONE, "stone floats (it is not granular)");
    }

    /// Falling sand displaces water and lands on the pool floor.
    #[test]
    fn falling_sand_displaces_water() {
        let mut w = world_with_ground();
        for y in 1..=3 {
            for x in 0..2 {
                w.set_block(x, y, 0, water_with_level(0));
            }
        }
        w.set_block(0, 4, 0, BlockState(block::SAND));
        w.set_block(0, 5, 0, BlockState(block::SAND));
        let moved = settle_gravity(&mut w, 0, 0);
        assert!(moved >= 2, "sand should sink through the pool, moved {}", moved);
        assert_eq!(w.get_block(0, 1, 0).id(), block::SAND, "first sand sinks to the pool floor");
        assert_eq!(w.get_block(0, 2, 0).id(), block::SAND, "second sand lands on the first");
        assert_ne!(w.get_block(0, 3, 0).id(), block::WATER, "no water trapped inside the sand column");
    }
}
