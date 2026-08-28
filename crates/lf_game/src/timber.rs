//! Valheim-style tree felling (loop 330, user request): chop a trunk and
//! the whole tree falls. Like everything in lf_game this is a set of pure
//! world transforms — the client calls [`find_tree`] when a log breaks,
//! animates [`tree_parts`] while the faller is airborne, then applies
//! [`fall_plan`] on impact (place horizontal-log blocks, shatter leaves).
//! Tests and vistest proofs run the same code headless.

use lf_voxel::registry::{self, block};
use lf_voxel::{BlockState, World};

/// A trunk can't be taller than this (worldgen caps GiantSpruce at 15).
pub const MAX_TRUNK_HEIGHT: usize = 24;
/// Canopy scan radius around the trunk top.
const CANOPY_RADIUS: i32 = 3;
/// The fall is "landed" at this angle from vertical (impact + placement).
pub const LAND_ANGLE: f32 = 1.42; // ~81°

/// One identifiable standing tree: a same-species trunk column above the
/// broken cell plus the leaf cells around its top.
#[derive(Clone)]
pub struct Tree {
    /// The cell the player broke (stump position; the fall hinge).
    pub base: [i32; 3],
    /// Remaining trunk cells above the break, bottom to top.
    pub trunk: Vec<[i32; 3]>,
    /// Leaf/canopy cells around the trunk top (shatter on landing).
    pub leaves: Vec<[i32; 3]>,
    pub log_id: u32,
    pub leaf_id: u32,
}

impl Tree {
    pub fn height(&self) -> f32 {
        self.trunk.len() as f32
    }
}

/// Cardinal fall direction (the dominant horizontal of the breaker's look).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FallDir {
    NegX,
    PosX,
    NegZ,
    PosZ,
}

impl FallDir {
    /// Unit vector along the fall, on the ground plane.
    pub fn vec(self) -> [f32; 2] {
        match self {
            FallDir::NegX => [-1.0, 0.0],
            FallDir::PosX => [1.0, 0.0],
            FallDir::NegZ => [0.0, -1.0],
            FallDir::PosZ => [0.0, 1.0],
        }
    }

    /// Pick a cardinal from a horizontal look direction (x, z).
    pub fn from_look(x: f32, z: f32) -> FallDir {
        if x.abs() >= z.abs() {
            if x >= 0.0 { FallDir::PosX } else { FallDir::NegX }
        } else if z >= 0.0 {
            FallDir::PosZ
        } else {
            FallDir::NegZ
        }
    }
}

/// The inventory item a species' trunk yields.
fn log_item(log_id: u32) -> &'static str {
    match log_id {
        block::BIRCH_LOG => "birch_log",
        block::SPRUCE_LOG => "spruce_log",
        block::DARK_LOG => "dark_log",
        block::CHERRY_LOG => "cherry_log",
        _ => "log",
    }
}

/// The leaf-equivalent id for a trunk species (mushroom "canopies" are
/// MUSHROOM_CAP on birch trunks).
fn canopy_id_for(log_id: u32) -> u32 {
    match log_id {
        block::BIRCH_LOG => block::BIRCH_LEAVES,
        block::SPRUCE_LOG => block::SPRUCE_LEAVES,
        block::DARK_LOG => block::DARK_LEAVES,
        block::CHERRY_LOG => block::CHERRY_LEAVES,
        _ => block::LEAVES,
    }
}

fn is_canopy(id: u32) -> bool {
    registry::is_leaf(id) || id == block::MUSHROOM_CAP
}

/// Identify the tree that would fall when the trunk cell at `above_base`
/// (the cell directly above the broken block) is part of a trunk. Returns
/// None when that cell is not a log: boulders, stumps with nothing above,
/// and placed single logs never trigger a fall.
pub fn find_tree(world: &World, above_base: [i32; 3]) -> Option<Tree> {
    let log_id = world.get_block(above_base[0], above_base[1], above_base[2]).id();
    if !registry::is_log(log_id) {
        return None;
    }
    let (x0, y0, z0) = (above_base[0], above_base[1], above_base[2]);
    let mut trunk = Vec::new();
    let mut y = y0;
    while trunk.len() < MAX_TRUNK_HEIGHT
        && world.get_block(x0, y, z0).id() == log_id
    {
        trunk.push([x0, y, z0]);
        y += 1;
    }
    // a single placed log is a block, not a tree — felling needs a trunk
    if trunk.len() < 2 {
        return None;
    }
    // canopy: any leaf-family cells in the box around the trunk top
    let top = *trunk.last().unwrap();
    let leaf_id = canopy_id_for(log_id);
    let mut leaves = Vec::new();
    for dx in -CANOPY_RADIUS..=CANOPY_RADIUS {
        for dy in -3..=3 {
            for dz in -CANOPY_RADIUS..=CANOPY_RADIUS {
                let (x, yy, z) = (top[0] + dx, top[1] + dy, top[2] + dz);
                if is_canopy(world.get_block(x, yy, z).id()) {
                    leaves.push([x, yy, z]);
                }
            }
        }
    }
    Some(Tree { base: [x0, y0 - 1, z0], trunk, leaves, log_id, leaf_id })
}

/// What happens on impact: log cells to place as horizontal blocks, and
/// the item ids of trunks that had nowhere to land (occupied by terrain —
/// the client turns those into item drops).
pub struct LandingPlan {
    /// (cell, horizontal log id) to place.
    pub place: Vec<([i32; 3], u32)>,
    /// Species item ids for logs that could not be placed.
    pub drop_items: Vec<String>,
    /// How many canopy cells shatter (drives particles/drops client-side).
    pub shattered_leaves: usize,
}

/// Compute the landing: the trunk lies down along `dir` at the stump's
/// level, one horizontal log per trunk cell, in order. A cell that is not
/// free (terrain in the way) converts that log into a drop instead.
/// `is_free` says whether a cell can take the placed log (air, fluids,
/// plants — the client passes a solid/replaceable check over the world).
pub fn fall_plan(tree: &Tree, dir: FallDir, is_free: impl Fn([i32; 3]) -> bool) -> LandingPlan {
    let [dx, dz] = dir.vec();
    let x_axis = dir == FallDir::PosX || dir == FallDir::NegX;
    let log_h = if x_axis {
        registry::log_horizontal_x(tree.log_id)
    } else {
        registry::log_horizontal_z(tree.log_id)
    }
    .unwrap_or(block::LOG_X);
    let mut plan = LandingPlan { place: Vec::new(), drop_items: Vec::new(), shattered_leaves: tree.leaves.len() };
    // the fall line starts at the stump and steps along the fall direction
    let (mut cx, mut cz) = (tree.base[0], tree.base[2]);
    let y = tree.base[1];
    for _ in 0..tree.trunk.len() {
        cx += dx as i32;
        cz += dz as i32;
        let cell = [cx, y, cz];
        if is_free(cell) {
            plan.place.push((cell, log_h));
        } else {
            plan.drop_items.push(log_item(tree.log_id).to_string());
        }
    }
    plan
}

/// Rigid-body layout of the falling tree at `angle` (0 = upright) rotated
/// around the hinge at the stump's ground point, falling along `dir`.
/// Returns world-space cube centers + per-cube half-size. This is the
/// dragon_parts idiom: one pure layout fn shared by the client renderer
/// and the vistest proofs.
pub fn tree_parts(tree: &Tree, angle: f32, dir: FallDir) -> Vec<([f32; 3], [f32; 3])> {
    let [dx, dz] = dir.vec();
    // the hinge is the stump cell's center: at 90° every trunk cube lands
    // exactly on the cell centers the landing plan places
    let hinge = [tree.base[0] as f32 + 0.5, tree.base[1] as f32 + 0.5, tree.base[2] as f32 + 0.5];
    let (sin, cos) = angle.sin_cos();
    let mut parts = Vec::with_capacity(tree.trunk.len() + tree.leaves.len());
    // trunk: unit cubes stacked along +Y from the hinge
    for (i, cell) in tree.trunk.iter().enumerate() {
        let h = i as f32 + 1.0; // center of the (i+1)-th trunk cube above the hinge
        let local = rot([0.0, h, 0.0], dx, dz, sin, cos);
        parts.push((
            [hinge[0] + local[0], hinge[1] + local[1], hinge[2] + local[2]],
            [0.48, 0.48, 0.48],
        ));
    }
    // canopy: full cubes at their true offsets from the hinge
    for cell in &tree.leaves {
        let p = [
            (cell[0] as f32 + 0.5) - hinge[0],
            (cell[1] as f32 + 0.5) - hinge[1],
            (cell[2] as f32 + 0.5) - hinge[2],
        ];
        let local = rot(p, dx, dz, sin, cos);
        parts.push((
            [hinge[0] + local[0], hinge[1] + local[1], hinge[2] + local[2]],
            [0.45, 0.45, 0.45],
        ));
    }
    parts
}

/// The world-space rotation that matches [`rot`]'s tilt for a fall
/// direction: (unit axis, sign for the fall angle). The renderer feeds
/// (axis, sign * angle) into the rotated-cube geometry so each cube's
/// faces tilt exactly with the trunk line. Pinned against `rot` by test.
pub fn fall_rotation(dir: FallDir) -> ([f32; 3], f32) {
    match dir {
        FallDir::PosX => ([0.0, 0.0, 1.0], -1.0),
        FallDir::NegX => ([0.0, 0.0, 1.0], 1.0),
        FallDir::PosZ => ([1.0, 0.0, 0.0], 1.0),
        FallDir::NegZ => ([1.0, 0.0, 0.0], -1.0),
    }
}

/// Rotate a local offset by `angle` toward the fall direction (rotation
/// around the horizontal axis perpendicular to the fall).
fn rot(p: [f32; 3], dx: f32, dz: f32, sin: f32, cos: f32) -> [f32; 3] {
    let pd = p[0] * dx + p[2] * dz; // component along the fall
    let ps = p[0] * dz - p[2] * dx; // side component
    let y = p[1];
    let d_new = pd * cos + y * sin;
    let y_new = y * cos - pd * sin;
    [dx * d_new + dz * ps, y_new, dz * d_new - dx * ps]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat stone world with a standard oak: 5-tall trunk, blob canopy.
    fn world_with_oak() -> (World, [i32; 3]) {
        let mut w = World::new();
        for cx in -1..=1 {
            for cz in -1..=1 {
                w.chunks.insert((cx, cz), lf_voxel::ChunkColumn::empty());
            }
        }
        for x in -12..12 {
            for z in -12..12 {
                w.set_block(x, 0, z, BlockState::STONE);
            }
        }
        // trunk at (0, 1..=5, 0)
        for y in 1..=5 {
            w.set_block(0, y, 0, BlockState(block::LOG));
        }
        // canopy: plus-shaped leaves around the top
        w.set_block(0, 6, 0, BlockState(block::LEAVES));
        w.set_block(1, 5, 0, BlockState(block::LEAVES));
        w.set_block(-1, 5, 0, BlockState(block::LEAVES));
        w.set_block(0, 5, 1, BlockState(block::LEAVES));
        w.set_block(0, 5, -1, BlockState(block::LEAVES));
        // the player broke the bottom log (y=1): the stump is [0,1,0] and
        // the first standing trunk cell is [0,2,0]
        (w, [0, 2, 0])
    }

    #[test]
    fn find_tree_identifies_trunk_and_canopy() {
        let (w, above) = world_with_oak();
        let tree = find_tree(&w, above).expect("oak found");
        assert_eq!(tree.trunk.len(), 4, "trunk above the break");
        assert_eq!(tree.log_id, block::LOG);
        assert_eq!(tree.leaves.len(), 5, "plus canopy");
        assert_eq!(tree.base, [0, 1, 0], "the stump cell");
    }

    #[test]
    fn find_tree_refuses_non_trunks() {
        let (mut w, _) = world_with_oak();
        // a lone placed log is a block, not a tree
        w.set_block(5, 1, 5, BlockState(block::LOG));
        assert!(find_tree(&w, [5, 1, 5]).is_none(), "single logs never fell");
        // a stone boulder never fells
        assert!(find_tree(&w, [0, 0, 3]).is_none());
    }

    #[test]
    fn find_tree_caps_lanky_trunks() {
        let (mut w, _) = world_with_oak();
        for y in 1..=40 {
            w.set_block(7, y, 7, BlockState(block::LOG));
        }
        let tree = find_tree(&w, [7, 2, 7]).unwrap();
        assert_eq!(tree.trunk.len(), MAX_TRUNK_HEIGHT, "capped");
    }

    #[test]
    fn fall_dir_picks_the_dominant_cardinal() {
        assert_eq!(FallDir::from_look(0.9, 0.2), FallDir::PosX);
        assert_eq!(FallDir::from_look(-0.1, -0.95), FallDir::NegZ);
        assert_eq!(FallDir::from_look(-0.8, 0.1), FallDir::NegX);
    }

    #[test]
    fn fall_plan_places_horizontal_logs_along_the_fall() {
        let (w, above) = world_with_oak();
        let tree = find_tree(&w, above).unwrap();
        let plan = fall_plan(&tree, FallDir::PosX, |_| true);
        assert_eq!(plan.place.len(), 4, "the 4 trunk cells above the break");
        assert_eq!(plan.drop_items.len(), 0);
        for (i, (cell, id)) in plan.place.iter().enumerate() {
            assert_eq!(cell[0], i as i32 + 1, "steps +X from the stump");
            assert_eq!(cell[1], 1, "lies at stump level");
            assert_eq!(*id, block::LOG_X, "X-aligned variant");
        }
        assert_eq!(plan.shattered_leaves, 5);
        // Z fall uses the Z-aligned variant
        let plan_z = fall_plan(&tree, FallDir::PosZ, |_| true);
        assert_eq!(plan_z.place[0].1, block::LOG_Z);
        assert_eq!(plan_z.place[0].0[2], 1);
    }

    #[test]
    fn fall_plan_converts_blocked_cells_to_drops() {
        let (w, above) = world_with_oak();
        let tree = find_tree(&w, above).unwrap();
        // a boulder sits two cells along the fall line
        let plan = fall_plan(&tree, FallDir::PosX, |c| c[0] != 2);
        assert_eq!(plan.place.len(), 3, "four logs minus the blocked one");
        assert_eq!(plan.drop_items.len(), 1, "blocked log becomes a drop");
        assert!(!plan.place.iter().any(|(c, _)| c[0] == 2));
    }

    #[test]
    fn fall_rotation_matches_the_parts_layout() {
        // rotate the trunk-top probe with rot(), then with Rodrigues around
        // fall_rotation's axis — they must agree for every direction
        for dir in [FallDir::PosX, FallDir::NegX, FallDir::PosZ, FallDir::NegZ] {
            let [dx, dz] = dir.vec();
            let (axis, sign) = fall_rotation(dir);
            let angle = 0.9_f32;
            let (sin, cos) = angle.sin_cos();
            let got = rot([0.0, 3.0, 0.0], dx, dz, sin, cos);
            let (ax, ay, az) = (axis[0], axis[1], axis[2]);
            let v = [0.0_f32, 3.0, 0.0];
            let dot = ax * v[0] + ay * v[1] + az * v[2];
            let cross = [ay * v[2] - az * v[1], az * v[0] - ax * v[2], ax * v[1] - ay * v[0]];
            let (s2, c2) = (angle * sign).sin_cos();
            let expected = [
                v[0] * c2 + cross[0] * s2 + ax * dot * (1.0 - c2),
                v[1] * c2 + cross[1] * s2 + ay * dot * (1.0 - c2),
                v[2] * c2 + cross[2] * s2 + az * dot * (1.0 - c2),
            ];
            for k in 0..3 {
                assert!((got[k] - expected[k]).abs() < 1e-4,
                    "{:?} axis {:?} angle {}: got {:?} expected {:?}",
                    dir, axis, sign * angle, got, expected);
            }
        }
    }

    #[test]
    fn tree_parts_pivot_and_preserve_length() {
        let (w, above) = world_with_oak();
        let tree = find_tree(&w, above).unwrap();
        let upright = tree_parts(&tree, 0.0, FallDir::PosX);
        assert_eq!(upright.len(), tree.trunk.len() + tree.leaves.len());
        // upright: trunk centers stack straight above the hinge (stump
        // center y=1.5; first standing cube center at 2.5)
        for (i, (c, _)) in upright.iter().enumerate().take(4) {
            assert!((c[0] - 0.5).abs() < 1e-4, "no sideways drift at angle 0");
            assert!((c[1] - (2.5 + i as f32)).abs() < 1e-4);
        }
        // landed: the trunk tip lies on the placed row's cell centers
        let landed = tree_parts(&tree, std::f32::consts::FRAC_PI_2, FallDir::PosX);
        let top = landed[3].0;
        assert!((top[0] - 4.5).abs() < 1e-4, "tip lies 4 blocks along +X");
        assert!((top[1] - 1.5).abs() < 1e-4, "tip at stump-row level");
        // mid-fall: length preserved (rotation is rigid around the hinge)
        let mid = tree_parts(&tree, 0.7, FallDir::PosX);
        let d = (mid[3].0[0] - 0.5).hypot(mid[3].0[1] - 1.5);
        assert!((d - 4.0).abs() < 1e-4, "rigid rotation keeps radius, got {}", d);
        // monotonic tilt: tip x grows with the angle
        let (a, b) = (tree_parts(&tree, 0.3, FallDir::PosX), tree_parts(&tree, 0.6, FallDir::PosX));
        assert!(a[4].0[0] < b[4].0[0]);
    }
}
