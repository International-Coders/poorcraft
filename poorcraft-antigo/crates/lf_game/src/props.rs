//! GMod-style physics props for ground item stacks.
//!
//! A mined or farmed block becomes a rigid prop: gravity, bounces off the
//! floor AND walls, tumbles while moving, and settles flat when it runs out
//! of energy. The player carries one at range by holding right-click (the
//! spring in the client pins it to the view ray) and picks stacks up by
//! walking into them. Ground stacks of the same item merge up to
//! [`PROP_STACK_MAX`], and the prop's cube grows with its count until a
//! full 5-stack is exactly one block wide.

use glam::Vec3;
use lf_voxel::World;

/// Maximum items one physics stack carries before it must split.
pub const PROP_STACK_MAX: u8 = 5;

/// Floor restitution (vertical bounce keeps 30% of impact speed).
const FLOOR_RESTITUTION: f32 = 0.3;
/// Wall restitution (props ping off walls more crisply than the floor).
const WALL_RESTITUTION: f32 = 0.4;
/// Ground friction per second of floor contact (props slide a couple of
/// blocks, GMod-style, instead of stopping on a dime).
const GROUND_FRICTION_PER_S: f32 = 1.2;
/// Below this impact speed a contact kills the axis instead of bouncing.
const BOUNCE_THRESHOLD: f32 = 1.6;

/// Half-extent of a stack's collision/render cube: one item is a
/// hand-sized chunk, a full 5-stack is exactly a block.
pub fn prop_half(count: u8) -> f32 {
    0.14 + (count.min(PROP_STACK_MAX) as f32) * 0.072
}

/// Edge-to-edge distance under which two resting stacks merge.
pub fn merge_distance(count_a: u8, count_b: u8) -> f32 {
    prop_half(count_a) + prop_half(count_b) + 0.02
}

/// Merge two same-item stack counts. Returns the surviving count and the
/// leftover that must stay behind as its own prop; `None` when either
/// stack is already full (two full stacks never merge).
pub fn merged_counts(a: u8, b: u8) -> Option<(u8, u8)> {
    let (a, b) = (a.min(PROP_STACK_MAX), b.min(PROP_STACK_MAX));
    if a >= PROP_STACK_MAX || b >= PROP_STACK_MAX {
        return None;
    }
    let total = a + b;
    if total <= PROP_STACK_MAX {
        Some((total, 0))
    } else {
        Some((PROP_STACK_MAX, total - PROP_STACK_MAX))
    }
}

/// Rigid state of one ground prop. `angle`/`tumble_axis` are render state
/// driven by the same step; `held` suspends physics (the client pins the
/// prop to the view ray), `rest` is the sleep flag.
#[derive(Clone, Debug)]
pub struct PropBody {
    pub position: Vec3,
    pub velocity: Vec3,
    pub angle: f32,
    pub angvel: f32,
    pub tumble_axis: Vec3,
    pub held: bool,
    pub rest: bool,
}

impl PropBody {
    pub fn new(position: Vec3, velocity: Vec3, tumble_axis: Vec3) -> Self {
        Self {
            position,
            velocity,
            angle: 0.0,
            // rolling speed ties to horizontal motion in step_prop
            angvel: 0.0,
            tumble_axis: tumble_axis.normalize_or_zero(),
            held: false,
            rest: false,
        }
    }
}

/// True when any solid block overlaps the prop's axis-aligned cube.
fn prop_blocked(world: &World, center: Vec3, half: f32) -> bool {
    let min = center - Vec3::splat(half);
    let max = center + Vec3::splat(half);
    for x in min.x.floor() as i32..=max.x.floor() as i32 {
        for y in min.y.floor() as i32..=max.y.floor() as i32 {
            for z in min.z.floor() as i32..=max.z.floor() as i32 {
                if world.is_solid(x, y, z) {
                    return true;
                }
            }
        }
    }
    false
}

/// One physics step. Held props are pinned externally and frozen here;
/// resting props stay asleep until something wakes them (a merge that
/// grows the cube, or being picked up and thrown).
pub fn step_prop(world: &World, p: &mut PropBody, half: f32, dt: f32) {
    if p.held || (p.rest && p.velocity.length_squared() < 0.001) {
        return;
    }
    p.rest = false;
    p.velocity.y -= 20.0 * dt;

    // --- X axis: move, bounce off walls -------------------------------
    let mut next = p.position;
    next.x += p.velocity.x * dt;
    if prop_blocked(world, next, half) {
        next.x = p.position.x;
        if p.velocity.x.abs() > BOUNCE_THRESHOLD {
            p.velocity.x = -p.velocity.x * WALL_RESTITUTION;
        } else {
            p.velocity.x = 0.0;
        }
    }
    // --- Z axis --------------------------------------------------------
    next.z += p.velocity.z * dt;
    if prop_blocked(world, next, half) {
        next.z = p.position.z;
        if p.velocity.z.abs() > BOUNCE_THRESHOLD {
            p.velocity.z = -p.velocity.z * WALL_RESTITUTION;
        } else {
            p.velocity.z = 0.0;
        }
    }
    // --- Y axis: land on the block top, bounce or settle ----------------
    next.y += p.velocity.y * dt;
    let mut grounded = false;
    if prop_blocked(world, next, half) {
        if p.velocity.y < 0.0 {
            // snap the cube's floor face onto the block top
            let feet_cell = (next.y - half).floor() as i32;
            next.y = feet_cell as f32 + 1.0 + half;
            grounded = true;
            if -p.velocity.y > BOUNCE_THRESHOLD {
                p.velocity.y = -p.velocity.y * FLOOR_RESTITUTION;
            } else {
                p.velocity.y = 0.0;
            }
            p.velocity.x *= 1.0 - (dt * GROUND_FRICTION_PER_S).min(0.9);
            p.velocity.z *= 1.0 - (dt * GROUND_FRICTION_PER_S).min(0.9);
        } else {
            // bonked a ceiling
            next.y = p.position.y;
            p.velocity.y = -p.velocity.y * WALL_RESTITUTION * 0.5;
        }
    }
    p.position = next;

    // --- tumble: rolling speed follows horizontal motion -----------------
    let horiz = (p.velocity.x * p.velocity.x + p.velocity.z * p.velocity.z).sqrt();
    if horiz > 0.05 {
        let target = horiz * 1.6;
        p.angvel += (target - p.angvel).clamp(-dt * 8.0, dt * 8.0);
        p.angle += p.angvel * dt;
    } else {
        p.angvel *= 1.0 - (dt * 6.0).min(1.0);
        if grounded || p.rest {
            // settle onto the nearest flat face
            let target = (p.angle / std::f32::consts::FRAC_PI_2).round()
                * std::f32::consts::FRAC_PI_2;
            p.angle += (target - p.angle) * (dt * 10.0).min(1.0);
        }
    }

    if grounded && p.velocity.length_squared() < 0.04 {
        p.velocity = Vec3::ZERO;
        p.rest = true;
        // sleeping props snap flat; the lerp below never runs post-sleep
        p.angle = (p.angle / std::f32::consts::FRAC_PI_2).round()
            * std::f32::consts::FRAC_PI_2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lf_voxel::BlockState;

    fn flat_world() -> World {
        let mut w = World::new();
        for cx in -2..=2 {
            for cz in -2..=2 {
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

    #[test]
    fn prop_size_grows_to_exactly_a_block() {
        let one = prop_half(1);
        let three = prop_half(3);
        let five = prop_half(5);
        assert!(one < three && three < five, "size must grow: {one} {three} {five}");
        assert!((five - 0.5).abs() < 1e-4, "a full stack is block-sized: {five}");
        assert_eq!(prop_half(9), five, "counts past the cap clamp");
        assert!(merge_distance(1, 1) > prop_half(1) * 2.0 - 0.01);
    }

    #[test]
    fn stacks_merge_up_to_five_and_never_past_it() {
        assert_eq!(merged_counts(2, 3), Some((5, 0)));
        assert_eq!(merged_counts(4, 4), Some((5, 3)), "overflow stays behind");
        assert_eq!(merged_counts(5, 2), None, "full stacks never merge");
        assert_eq!(merged_counts(5, 5), None);
    }

    #[test]
    fn props_fall_bounce_and_rest_on_the_floor() {
        let w = flat_world();
        let half = prop_half(3);
        let mut p = PropBody::new(Vec3::new(0.0, 4.0, 0.0), Vec3::new(1.5, -3.0, 0.0), Vec3::Z);
        for _ in 0..400 {
            step_prop(&w, &mut p, half, 0.016);
        }
        assert!(p.rest, "prop must come to rest");
        assert!((p.position.y - (1.0 + half)).abs() < 0.02,
            "cube floor face sits on the block top: {}", p.position.y);
        assert!(p.position.x > 0.3, "kept some sideways travel: {}", p.position.x);
        // settled flat: angle is a multiple of 90 degrees
        let quarter = p.angle / std::f32::consts::FRAC_PI_2;
        assert!((quarter - quarter.round()).abs() < 0.02, "settled flat at {}", p.angle);
    }

    #[test]
    fn props_bounce_off_walls_and_stop_touching_them() {
        let mut w = flat_world();
        // wall one block east of the origin column, 3 tall
        for y in 1..=3 {
            w.set_block(3, y, 0, BlockState::STONE);
            w.set_block(3, y, 1, BlockState::STONE);
            w.set_block(3, y, -1, BlockState::STONE);
        }
        let half = prop_half(1);
        // fast throw: rebounds off the wall, never penetrates it
        let mut p = PropBody::new(Vec3::new(0.0, 1.0 + half, 0.0), Vec3::new(6.0, 0.0, 0.0), Vec3::Y);
        let mut bounced = false;
        for _ in 0..900 {
            step_prop(&w, &mut p, half, 0.016);
            if p.velocity.x < 0.0 {
                bounced = true;
            }
            assert!(p.position.x + half <= 3.0 + 0.01,
                "penetrated the wall: {} + {}", p.position.x, half);
        }
        assert!(bounced, "a fast prop must rebound off the wall");
        assert!(p.rest);
        // slow push: slides into the wall and stops touching it
        let mut q = PropBody::new(Vec3::new(0.0, 1.0 + half, 0.0), Vec3::new(3.5, 0.0, 0.0), Vec3::Y);
        for _ in 0..900 {
            step_prop(&w, &mut q, half, 0.016);
        }
        assert!(q.rest, "slow push settles");
        assert!(q.position.x + half > 2.9,
            "slow push comes to rest against the wall: {}", q.position.x);
    }

    #[test]
    fn held_props_freeze_and_resting_props_sleep() {
        let w = flat_world();
        let half = prop_half(2);
        let mut p = PropBody::new(Vec3::new(0.0, 8.0, 0.0), Vec3::ZERO, Vec3::X);
        p.held = true;
        step_prop(&w, &mut p, half, 0.5);
        assert_eq!(p.position.y, 8.0, "held props do not fall");
        p.held = false;
        step_prop(&w, &mut p, half, 0.5);
        assert!(p.position.y < 8.0, "released props fall again");
        // a resting prop ignores tiny residual velocities
        p.rest = true;
        p.velocity = Vec3::new(0.001, 0.0, 0.0);
        step_prop(&w, &mut p, half, 0.5);
        assert_eq!(p.velocity, Vec3::new(0.001, 0.0, 0.0), "sleeping props skip the step");
    }
}
