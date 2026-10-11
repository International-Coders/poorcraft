//! B5 (ai-npc-assets): short-range A* pathfinding for mob and NPC
//! ground movement. Cardinal moves only (diagonals produce diagonal
//! floating that reads wrong in a voxel world), plus 1-block up/down
//! steps so a single obstacle block or a single step is traversable.
//!
//! Budget discipline (ai/MOB_AI_UPGRADE.md): the explorer is hard-capped
//! at `max_nodes` (callers pass 256; anything larger is clamped), and
//! callers only ask for paths when the goal is within 16 blocks — beyond
//! that, direct steering is good enough and much cheaper.

use std::collections::{BinaryHeap, HashMap};

use serde::{Deserialize, Serialize};

use lf_voxel::World;

pub const MAX_PATH_NODES: usize = 256;
/// Longest intended path; callers should not request goals farther than
/// this (in Manhattan distance) from the start.
pub const MAX_PATH_RANGE: i32 = 16;

pub type BlockPos = [i32; 3];

/// B5: a mob's cached A* route. Valid for 2 seconds or until the goal
/// moves more than 4 blocks — whichever comes first (the owner checks).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPath {
    pub nodes: Vec<BlockPos>,
    pub goal: BlockPos,
    pub age: f32,
    pub cursor: usize,
}

/// A cell a ground mob can occupy: body air, headroom air, floor solid.
fn passable(cell: BlockPos, world: &World) -> bool {
    let [x, y, z] = cell;
    !world.is_solid(x, y, z) && !world.is_solid(x, y + 1, z) && world.is_solid(x, y - 1, z)
}

fn manhattan(a: BlockPos, b: BlockPos) -> i32 {
    (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs()
}

/// Manhattan distance, exposed for callers that gate path requests.
pub fn path_distance(a: BlockPos, b: BlockPos) -> i32 {
    manhattan(a, b)
}

/// Returns a path of block positions from start to goal (start excluded,
/// goal included), or None if no path found within `max_nodes` explored.
pub fn find_path(start: BlockPos, goal: BlockPos, world: &World, max_nodes: usize) -> Option<Vec<BlockPos>> {
    let max_nodes = max_nodes.min(MAX_PATH_NODES).max(1);
    if start == goal {
        return Some(Vec::new());
    }
    if !passable(goal, world) {
        return None;
    }
    // A* over uniform steps; jump-ups cost a little more so flat routes win.
    #[derive(PartialEq, Eq)]
    struct Node {
        f: i32,
        g: i32,
        pos: BlockPos,
    }
    impl Ord for Node {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            other.f.cmp(&self.f).then(other.g.cmp(&self.g)) // min-heap via BinaryHeap
        }
    }
    impl PartialOrd for Node {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    let mut open = BinaryHeap::new();
    let mut g_score: HashMap<BlockPos, i32> = HashMap::new();
    let mut came_from: HashMap<BlockPos, BlockPos> = HashMap::new();
    g_score.insert(start, 0);
    open.push(Node { f: manhattan(start, goal), g: 0, pos: start });
    let mut explored = 0usize;

    while let Some(Node { g, pos, .. }) = open.pop() {
        if pos == goal {
            let mut path = Vec::new();
            let mut cur = goal;
            while cur != start {
                path.push(cur);
                cur = came_from[&cur];
            }
            path.reverse();
            return Some(path);
        }
        // stale heap entry (a better route already handled this cell)
        if g_score.get(&pos).copied().unwrap_or(i32::MAX) < g {
            continue;
        }
        explored += 1;
        if explored > max_nodes {
            return None;
        }
        let [x, y, z] = pos;
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            for dy in [-1, 0, 1] {
                let next: BlockPos = [x + dx, y + dy, z + dz];
                if !passable(next, world) {
                    continue;
                }
                // only pay for the step-up when it actually climbs
                let step_cost = 1 + (dy.max(0) * 5); // flat/lower 1, jump-up 6
                let ng = g + step_cost;
                if ng < g_score.get(&next).copied().unwrap_or(i32::MAX) {
                    g_score.insert(next, ng);
                    came_from.insert(next, pos);
                    open.push(Node { f: ng + manhattan(next, goal), g: ng, pos: next });
                }
            }
        }
    }
    None

}

#[cfg(test)]
mod tests {
    use super::*;
    use lf_voxel::BlockState;

    fn floor_world() -> World {
        let mut w = World::new();
        // chunks for the full -20..19 strip (set_block silently drops
        // edits in chunks that were never ensured)
        for cx in -2..=1 {
            for cz in -2..=1 {
                w.ensure_chunk(cx, cz);
            }
        }
        for x in -20..20 {
            for z in -20..20 {
                w.set_block(x, 0, z, BlockState::STONE);
            }
        }
        w
    }

    /// Failure meaning: A* must route around (or over) a single-block
    /// obstacle instead of reporting "no path" — this is the exact case
    /// the old 1-block-hop movement got stuck on.
    #[test]
    fn mob_pathfinding_basic() {
        let mut w = floor_world();
        // one-block wall on the straight line from (-2,1,0) to (2,1,0)
        w.set_block(0, 1, 0, BlockState::STONE);
        let start = [-2, 1, 0];
        let goal = [2, 1, 0];
        let path = find_path(start, goal, &w, MAX_PATH_NODES)
            .expect("a path around a single block must exist");
        assert_eq!(*path.last().unwrap(), goal, "path ends at the goal");
        assert_eq!(path[0], [-1, 1, 0], "first step is adjacent to start");
        // every step is a cardinal move with at most 1 block of climb
        let mut cur = start;
        for step in &path {
            let d = manhattan(cur, *step);
            assert!(d == 1 || d == 3, "cardinal step or 1-up jump, got {} {:?}", d, step);
            assert!(passable(*step, &w), "every step is standable");
            cur = *step;
        }
        assert!(path.len() <= 6, "route stays efficient ({} steps)", path.len());
        // straight open ground: path is the straight line
        let w2 = floor_world();
        let straight = find_path([-2, 1, 0], [2, 1, 0], &w2, MAX_PATH_NODES).unwrap();
        assert_eq!(straight.len(), 4, "open ground = 4 straight steps");
    }

    #[test]
    fn unreachable_goal_returns_none() {
        let mut w = floor_world();
        // sealed box: goal inside, no opening
        for x in -3..=3 {
            for z in -3..=3 {
                for y in 1..=4 {
                    let wall = x == -3 || x == 3 || z == -3 || z == 3 || y == 4;
                    if wall {
                        w.set_block(x, y, z, BlockState::STONE);
                    }
                }
            }
        }
        w.set_block(0, 3, 0, BlockState::STONE); // not standable inside anyway
        assert_eq!(find_path([0, 1, 6], [0, 1, 0], &w, MAX_PATH_NODES), None);
        // goal cell itself not standable (body + head filled)
        let mut w3 = floor_world();
        w3.set_block(5, 1, 0, BlockState::STONE);
        w3.set_block(5, 2, 0, BlockState::STONE);
        assert_eq!(find_path([0, 1, 0], [5, 1, 0], &w3, MAX_PATH_NODES), None);
    }

    #[test]
    fn node_budget_is_respected() {
        let w = floor_world();
        // a distant goal cannot be reached within a tiny node budget
        assert_eq!(find_path([0, 1, 0], [15, 1, 15], &w, 8), None);
        assert!(find_path([0, 1, 0], [15, 1, 15], &w, MAX_PATH_NODES).is_some());
    }
}
