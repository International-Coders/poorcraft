//! NPC locomotion: how a villager actually crosses terrain.
//!
//! loop 345 (kingdoms-and-walkers): the old client movement loop only
//! committed a step when the next cell was air AND the cell below it was
//! solid — a one-block bump or a one-block dip froze the NPC forever, and
//! NPCs whose workstation sat behind any obstacle vibrated against it
//! looking "not walking". This module owns the real rules and is pure
//! (terrain arrives as a `solid(x, y, z)` closure) so it is unit-testable
//! without a renderer:
//!
//! - step up one block (head clearance required),
//! - descend up to [`MAX_DROP`] blocks (walk downhill),
//! - refuse cliffs taller than [`MAX_DROP`],
//! - fall with gravity when the ground is gone,
//! - after [`STUCK_TICKS`] refused steps, sidestep perpendicular for a
//!   while so walls and trees get rounded instead of head-butted.

use serde::{Deserialize, Serialize};

/// Ground may drop at most this many blocks under a walking step.
pub const MAX_DROP: i32 = 3;
/// Refused steps before the NPC tries to sidestep around the obstacle.
pub const STUCK_TICKS: u32 = 20;
/// How long a sidestep lasts before re-approaching the target.
pub const SIDESTEP_TICKS: u32 = 40;

/// Per-NPC locomotion scratch state (persisted with the villager so a
/// save/reload does not reset fall speed or an in-progress sidestep).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Loco {
    /// Refused-step counter; cleared by any committed step.
    pub blocked: u32,
    /// Tick timestamp (handed in by the caller) the current sidestep ends.
    pub sidestep_until: u64,
    /// Heading of the current sidestep (radians, atan2(x, z) convention).
    pub sidestep_yaw: f32,
    /// Downward speed accumulator for gravity (blocks/tick).
    pub fall_speed: f32,
    /// Which way this NPC sidesteps (+1 right / -1 left of the bearing).
    /// Fixed per NPC (set from the villager id) so a wall is rounded in
    /// one direction instead of ping-ponging.
    pub side_bias: f32,
}

impl Default for Loco {
    fn default() -> Self {
        Loco {
            blocked: 0,
            sidestep_until: 0,
            sidestep_yaw: 0.0,
            fall_speed: 0.0,
            side_bias: 1.0, // a 0 bias would sidestep straight into the wall
        }
    }
}

impl Loco {
    /// The heading the NPC should walk this tick: the bearing to the
    /// target, or the sidestep heading while one is active.
    pub fn heading(&self, target_yaw: f32, now_ticks: u64) -> f32 {
        if now_ticks < self.sidestep_until {
            self.sidestep_yaw
        } else {
            target_yaw
        }
    }

    fn note_blocked(&mut self, target_yaw: f32, now_ticks: u64) {
        self.blocked += 1;
        if self.blocked >= STUCK_TICKS {
            self.blocked = 0;
            self.sidestep_until = now_ticks + SIDESTEP_TICKS as u64;
            self.sidestep_yaw = target_yaw + self.side_bias * std::f32::consts::FRAC_PI_2;
        }
    }
}

/// What one locomotion tick did.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Move {
    /// Position and ground level advanced (`y` = feet resting on ground).
    Stepped,
    /// Falling; `y` decreased but x/z held.
    Fell,
    /// Refused: obstacle too tall or a cliff beyond [`MAX_DROP`].
    Blocked,
}

/// The standing height (feet y) at `(x, z)` when currently at `from_y`:
/// the first solid block with two air blocks above it, scanning
/// `from_y + 1` down to `from_y - MAX_DROP`. `None` = cliff / no footing.
pub fn footing(solid: &dyn Fn(i32, i32, i32) -> bool, x: i32, from_y: i32, z: i32) -> Option<i32> {
    // already standing inside terrain? the caller keeps its current y
    for y in (from_y - MAX_DROP)..=(from_y + 1) {
        if solid(x, y, z) && !solid(x, y + 1, z) && !solid(x, y + 2, z) {
            return Some(y + 1); // feet rest on top of the solid block
        }
    }
    None
}

/// One locomotion tick. `wish_yaw` is the desired heading (atan2(x, z)),
/// `speed` blocks/second, `dt` seconds. Mutates `pos` (feet position) and
/// returns what happened. Collision is a 1-wide box tested at the cell the
/// step lands in (the NPC's own cell is assumed clear).
pub fn step(
    loco: &mut Loco,
    pos: &mut [f32; 3],
    wish_yaw: f32,
    speed: f32,
    dt: f32,
    now_ticks: u64,
    solid: &dyn Fn(i32, i32, i32) -> bool,
) -> Move {
    let yaw = loco.heading(wish_yaw, now_ticks);
    let dist = speed * dt;
    let nx = pos[0] + yaw.sin() * dist;
    let nz = pos[2] + yaw.cos() * dist;
    let feet = pos[1].floor() as i32;
    let (cx, cz) = (nx.floor() as i32, nz.floor() as i32);
    let (ox, oz) = (pos[0].floor() as i32, pos[2].floor() as i32);
    if cx != ox || cz != oz {
        // crossing into a new cell: check footing + headroom there
        match footing(solid, cx, feet, cz) {
            Some(ground) => {
                if ground > feet + 1 {
                    loco.note_blocked(wish_yaw, now_ticks);
                    return Move::Blocked; // wall taller than a step
                }
                pos[0] = nx;
                pos[2] = nz;
                pos[1] = ground as f32;
                loco.blocked = 0;
                loco.sidestep_until = 0;
                Move::Stepped
            }
            None => {
                loco.note_blocked(wish_yaw, now_ticks);
                Move::Blocked // cliff: no footing within MAX_DROP
            }
        }
    } else {
        pos[0] = nx;
        pos[2] = nz;
        Move::Stepped // same cell: always fine
    }
}

/// Gravity: when nothing supports the current cell, fall (accelerating,
/// capped) until footing returns. Call every tick regardless of wish.
pub fn fall(
    loco: &mut Loco,
    pos: &mut [f32; 3],
    dt: f32,
    solid: &dyn Fn(i32, i32, i32) -> bool,
) -> Move {
    let feet = pos[1].floor() as i32;
    let (cx, cz) = (pos[0].floor() as i32, pos[2].floor() as i32);
    if solid(cx, feet - 1, cz) && !solid(cx, feet, cz) && !solid(cx, feet + 1, cz) {
        loco.fall_speed = 0.0;
        return Move::Stepped; // supported: nothing to do
    }
    if solid(cx, feet, cz) {
        // wedged inside a block (terrain rose under us): pop to its top
        let mut top = feet;
        while solid(cx, top, cz) && top < feet + 2 {
            top += 1;
        }
        pos[1] = top as f32;
        loco.fall_speed = 0.0;
        return Move::Stepped;
    }
    loco.fall_speed = (loco.fall_speed + 32.0 * dt).min(1.8);
    let old = pos[1];
    pos[1] -= loco.fall_speed * dt;
    // land on the highest surface crossed this tick (no tunnelling at
    // terminal speed)
    let mut y = old.floor() as i32;
    let floor_now = pos[1].floor() as i32;
    while y >= floor_now {
        if solid(cx, y - 1, cz) && !solid(cx, y, cz) {
            pos[1] = y as f32;
            loco.fall_speed = 0.0;
            break;
        }
        y -= 1;
    }
    Move::Fell
}

/// Full villager tick used by the client: fall first (ground truth), then
/// walk if asked. Returns the move so callers can drive walk animation.
pub fn tick(
    loco: &mut Loco,
    pos: &mut [f32; 3],
    wish: Option<(f32, f32)>, // (yaw, speed)
    dt: f32,
    now_ticks: u64,
    solid: &dyn Fn(i32, i32, i32) -> bool,
) -> Move {
    let f = fall(loco, pos, dt, solid);
    match wish {
        Some((yaw, speed)) => {
            match step(loco, pos, yaw, speed, dt, now_ticks, solid) {
                Move::Blocked => Move::Blocked,
                Move::Stepped => {
                    if f == Move::Fell {
                        Move::Fell // stepped but also airborne this tick
                    } else {
                        Move::Stepped
                    }
                }
                Move::Fell => f,
            }
        }
        None => f,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Flat world helper: floor at y=64 unless a cell is raised.
    struct Grid(HashSet<(i32, i32, i32)>);
    impl Grid {
        fn flat() -> Self {
            let mut s = HashSet::new();
            for x in -20..20 {
                for z in -20..20 {
                    s.insert((x, 64, z));
                }
            }
            Grid(s)
        }
        fn solid(&self) -> impl Fn(i32, i32, i32) -> bool + '_ {
            move |x, y, z| self.0.contains(&(x, y, z))
        }
        fn raise(&mut self, x: i32, z: i32, h: i32) {
            for y in 0..h {
                self.0.insert((x, 64 + y + 1, z));
            }
        }
    }

    const DT: f32 = 1.0 / 20.0;

    #[test]
    fn walks_flat_ground() {
        let g = Grid::flat();
        let mut loco = Loco::default();
        let mut pos = [0.5, 65.0, 0.5];
        for _ in 0..40 {
            tick(&mut loco, &mut pos, Some((0.0, 1.2)), DT, 0, &g.solid());
        }
        assert!(pos[2] > 1.5, "walked +z, got z={}", pos[2]);
        assert_eq!(pos[1], 65.0, "stays on the floor");
    }

    #[test]
    fn steps_up_a_one_block_bump() {
        let mut g = Grid::flat();
        for z in 2..20 {
            g.raise(0, z, 1); // z>=2 is one block higher (feet at 66)
        }
        let mut loco = Loco::default();
        let mut pos = [0.5, 65.0, 0.5];
        let mut steps = 0;
        for t in 0..80 {
            if tick(&mut loco, &mut pos, Some((0.0, 1.2)), DT, t, &g.solid()) == Move::Stepped {
                steps += 1;
            }
        }
        assert!(pos[2] >= 4.0, "crossed the bump, z={}", pos[2]);
        assert!(pos[1] >= 66.0, "climbed to the raised shelf, y={}", pos[1]);
        assert!(steps > 20, "most ticks commit: {} steps", steps);
    }

    #[test]
    fn walks_down_a_two_block_slope() {
        let mut g = Grid::flat();
        for z in 2..20 {
            g.0.insert((0, 65, z));
            g.0.insert((0, 66, z)); // shelf two higher, feet at 67
        }
        // walk the other way: start on the shelf, target downhill
        let mut loco = Loco::default();
        let mut pos = [0.5, 67.0, 3.5];
        for t in 0..120 {
            tick(&mut loco, &mut pos, Some((std::f32::consts::PI, 1.2)), DT, t, &g.solid());
        }
        assert!(pos[2] <= 1.0, "descended the slope, z={}", pos[2]);
        assert_eq!(pos[1], 65.0, "back on the floor");
    }

    #[test]
    fn refuses_a_cliff() {
        let mut g = Grid::flat();
        for z in 2..20 {
            for y in 50..65 {
                g.0.remove(&(0, y, z)); // pit: bottomless column
            }
        }
        let mut loco = Loco::default();
        let mut pos = [0.5, 65.0, 0.5];
        let mut blocked_early = 0;
        for t in 0..200 {
            let m = tick(&mut loco, &mut pos, Some((0.0, 1.2)), DT, t, &g.solid());
            if t < 60 && m == Move::Blocked {
                blocked_early += 1;
            }
            // the invariant that matters: the pit is never entered
            assert!(pos[1] >= 64.99, "fell into the pit at tick {} (y={})", t, pos[1]);
        }
        assert!(blocked_early > 0, "the pit edge must refuse the step");
        // it either still stands refused at the rim or routed around it
        assert!(pos[2] < 2.0 || pos[0].floor() as i32 != 0,
            "past the pit only by going around: z={} x={}", pos[2], pos[0]);
    }

    #[test]
    fn sidesteps_around_a_wall() {
        // wall ahead with a gap at x=4; walking straight +z must eventually
        // slip around it via the sidestep reflex.
        let mut g = Grid::flat();
        for x in -6..4 {
            g.raise(x, 3, 2); // two-high wall: cannot be stepped
        }
        let mut loco = Loco::default();
        let mut pos = [0.5, 65.0, 0.5];
        for t in 0..600 {
            tick(&mut loco, &mut pos, Some((0.0, 1.2)), DT, t, &g.solid());
        }
        assert!(pos[2] > 6.0, "got past the wall, z={}", pos[2]);
    }

    #[test]
    fn falls_when_floor_removed() {
        let mut g = Grid::flat();
        let mut loco = Loco::default();
        let mut pos = [0.5, 65.0, 0.5];
        g.0.remove(&(0, 64, 0)); // hole under the NPC...
        g.0.insert((0, 62, 0)); // ...with a ledge two below to land on
        let mut fell = false;
        for t in 0..60 {
            if tick(&mut loco, &mut pos, None, DT, t, &g.solid()) == Move::Fell {
                fell = true;
            }
        }
        assert!(fell, "gravity must engage");
        assert_eq!(pos[1], 63.0, "settled on the ledge");
    }

    #[test]
    fn loco_state_survives_serialization() {
        let loco = Loco { blocked: 3, sidestep_until: 90, sidestep_yaw: 1.2, fall_speed: 0.4, side_bias: -1.0 };
        let bytes = serde_json::to_vec(&loco).unwrap();
        let back: Loco = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(loco, back);
    }
}
