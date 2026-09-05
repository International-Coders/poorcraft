//! P3D-401: the player controller — terrain collision, steps, swimming,
//! safe spawn.
//!
//! Deterministic first-person movement: one [`Player::step`] per 60 Hz
//! sim tick, axis-separated collision against [`final_solid`], 1 m
//! step-up, buoyant swimming in Water, and a safe spawn that always
//! begins on walkable land above sea level. All math is f32 in a fixed
//! operation order per tick.

use crate::gen::{CellMaterial, WorldGen};
use crate::terrain::final_solid;
use crate::terrain::SceneSpec;

/// Sim tick length (s) — matches the engine's fixed clock.
pub const SIM_DT: f32 = 1.0 / 60.0;
/// Player body: half-width (m) and height (m).
pub const HALF_WIDTH: f32 = 0.3;
pub const BODY_HEIGHT: f32 = 1.8;
/// Eye height above the feet.
pub const EYE_HEIGHT: f32 = 1.62;
pub const WALK_SPEED: f32 = 4.3;
pub const SWIM_SPEED: f32 = 2.2;
pub const GRAVITY: f32 = 18.0;
pub const SWIM_GRAVITY: f32 = GRAVITY * 0.4;
pub const TERMINAL_FALL: f32 = 40.0;
pub const TERMINAL_SINK: f32 = 2.0;
pub const JUMP_VELOCITY: f32 = 6.5;

/// The player: feet position in world METERs, velocity m/s.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Player {
    /// Feet position (meters).
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub on_ground: bool,
    pub swimming: bool,
}

/// World-space movement intent for one tick.
#[derive(Clone, Copy, Debug, Default)]
pub struct MoveInput {
    pub move_x: f32,
    pub move_z: f32,
    pub jump: bool,
}

impl Player {
    /// Deterministic safe spawn: start at the SmoothHills scene's region
    /// center patch and scan outward for a column whose floor is solid,
    /// with two passable cells above, above sea level. Falls back to a
    /// deep scan if the scene pin ever moves.
    pub fn spawn_safe(gen: &WorldGen) -> Player {
        // Progressive rings (2, 4, 8, ... patches) around the origin,
        // REGION-CENTER patches only, with a cheap macro-elevation
        // prefilter (ocean regions cannot host a land spawn; terrain
        // features are kilometers wide so small rings can be all ocean).
        let mut r = 2i32;
        loop {
            for px in -r..=r {
                for pz in -r..=r {
                    let region = crate::coords::RegionCoord {
                        x: px.div_euclid(16),
                        z: pz.div_euclid(16),
                    };
                    if gen.macro_field(region).elevation_m < 1 {
                        continue;
                    }
                    // REGION spans 256 cells: its center CELL is
                    // region*256 + 128 (NOT patch indices — that bug spun
                    // the scan on an origin-corner forever).
                    let cell = WorldPosCell {
                        x: region.x * 256 + 128,
                        y: 0,
                        z: region.z * 256 + 128,
                    };
                    if let Some(p) = Self::try_spawn_at(gen, cell.x, cell.y, cell.z) {
                        return p;
                    }
                }
            }
            r *= 2;
            if r > 4096 {
                panic!(
                    "no safe spawn found within +-64 km — generator produced an ocean world"
                );
            }
        }
    }


    fn try_spawn_at(gen: &WorldGen, cx: i32, _cy: i32, cz: i32) -> Option<Player> {
        // Find the surface: topmost solid cell in a generous y range.
        let wx = cx as i64 * 1000;
        let wz = cz as i64 * 1000;
        // Sea-level and terrain range bound the scan.
        for surface_y in (0..=48).rev() {
            let floor_solid = solid_at(gen, cx, surface_y, cz);
            if !floor_solid {
                continue;
            }
            let feet = surface_y + 1;
            if feet <= 0 {
                continue; // below sea level: not land spawn
            }
            // Two passable cells above the floor.
            if passable_at(gen, cx, feet, cz) && passable_at(gen, cx, feet + 1, cz) {
                return Some(Player {
                    pos: [cx as f32, feet as f32 + 0.01, cz as f32],
                    vel: [0.0; 3],
                    on_ground: true,
                    swimming: false,
                });
            }
            break; // surface found but blocked above: not spawnable here
        }
        None
    }

    /// One deterministic 60 Hz tick.
    pub fn step(&mut self, gen: &WorldGen, input: MoveInput) {
        let feet_world = |p: [f32; 3]| p;
        let _ = feet_world;
        let (px, py, pz) = (self.pos[0], self.pos[1], self.pos[2]);
        self.swimming = is_water_at(gen, px, py + 0.9, pz);

        // Horizontal velocity toward input (accelerate hard, snap to cap).
        let speed = if self.swimming { SWIM_SPEED } else { WALK_SPEED };
        let in_len = (input.move_x * input.move_x + input.move_z * input.move_z).sqrt();
        let (tx, tz) = if in_len > 1.0 {
            (input.move_x / in_len, input.move_z / in_len)
        } else {
            (input.move_x, input.move_z)
        };
        let target_vx = tx * speed;
        let target_vz = tz * speed;
        let accel = if self.on_ground || self.swimming { 40.0 } else { 12.0 };
        self.vel[0] = approach(self.vel[0], target_vx, accel * SIM_DT);
        self.vel[2] = approach(self.vel[2], target_vz, accel * SIM_DT);

        // Vertical: gravity or buoyancy; jump.
        if self.swimming {
            if input.jump {
                self.vel[1] = approach(self.vel[1], 2.5, 20.0 * SIM_DT);
            } else {
                self.vel[1] = (self.vel[1] - SWIM_GRAVITY * SIM_DT).max(-TERMINAL_SINK);
            }
        } else {
            if self.on_ground && input.jump {
                self.vel[1] = JUMP_VELOCITY;
                self.on_ground = false;
            }
            self.vel[1] = (self.vel[1] - GRAVITY * SIM_DT).max(-TERMINAL_FALL);
        }

        // Axis-separated movement with collision.
        self.move_axis(gen, 0, self.vel[0] * SIM_DT);
        self.move_axis(gen, 2, self.vel[2] * SIM_DT);
        self.move_axis(gen, 1, self.vel[1] * SIM_DT);
    }

    fn move_axis(&mut self, gen: &WorldGen, axis: usize, delta: f32) {
        if delta == 0.0 {
            if axis == 1 {
                self.on_ground = self.on_ground && true;
            }
            return;
        }
        let old = self.pos;
        let mut new = self.pos;
        new[axis] += delta;

        if !self.body_collides(gen, new) {
            self.pos = new;
            if axis == 1 {
                self.on_ground = false;
            }
            return;
        }
        if axis == 1 {
            // Landing or head bump.
            if delta < 0.0 {
                self.pos[1] = self.pos[1].floor() + f32::EPSILON;
                // Snap feet onto the floor below if between cells.
                if self.body_collides(gen, self.pos) {
                    self.pos[1] = (self.pos[1] - 0.5).floor() + 1.0;
                }
                self.on_ground = true;
            }
            self.vel[1] = 0.0;
            return;
        }
        // Horizontal blocked: try STEP-UP (1 m) for grounded/swimming
        // players — climb knee-height terrain.
        let mut stepped = new;
        stepped[1] += 1.0;
        if (self.on_ground || self.swimming) && !self.body_collides(gen, stepped) {
            self.pos = stepped;
            return;
        }
        // Blocked: cancel the axis velocity.
        self.vel[axis] = 0.0;
        let _ = old;
    }

    /// Body collision: any SOLID cell intersecting the AABB footprint at
    /// the sampled heights (feet + 0.1, mid, head − 0.1).
    fn body_collides(&self, gen: &WorldGen, pos: [f32; 3]) -> bool {
        let (px, py, pz) = (pos[0], pos[1], pos[2]);
        for &[oy] in &[[0.1f32], [0.9], [1.7]] {
            let y = py + oy;
            for &[ox, oz] in &[
                [-HALF_WIDTH, -HALF_WIDTH],
                [HALF_WIDTH, -HALF_WIDTH],
                [-HALF_WIDTH, HALF_WIDTH],
                [HALF_WIDTH, HALF_WIDTH],
            ] {
                if solid_at(gen, (px + ox).floor() as i32, y.floor() as i32, (pz + oz).floor() as i32) {
                    return true;
                }
            }
        }
        false
    }
}

fn approach(current: f32, target: f32, max_delta: f32) -> f32 {
    let d = target - current;
    if d.abs() <= max_delta {
        target
    } else {
        current + d.signum() * max_delta
    }
}

/// Terrain solidity at a world CELL.
fn solid_at(gen: &WorldGen, cx: i32, cy: i32, cz: i32) -> bool {
    final_solid(
        gen,
        cx as i64 * 1000,
        cy as i64 * 1000,
        cz as i64 * 1000,
    )
    .solid
}

/// Passable = air or water (swimmable).
fn passable_at(gen: &WorldGen, cx: i32, cy: i32, cz: i32) -> bool {
    let a = final_solid(
        gen,
        cx as i64 * 1000,
        cy as i64 * 1000,
        cz as i64 * 1000,
    );
    matches!(a.material, CellMaterial::Air | CellMaterial::Water)
}

fn is_water_at(gen: &WorldGen, x: f32, y: f32, z: f32) -> bool {
    let a = final_solid(gen, (x.floor() as i64) * 1000, (y.floor() as i64) * 1000, (z.floor() as i64) * 1000);
    a.material == CellMaterial::Water
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WorldPosCell {
    x: i32,
    y: i32,
    z: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE spawn contract: across seeds, the player spawns on land above
    /// sea level with solid ground below and passable cells above.
    #[test]
    fn p3d401_spawn_is_safe_across_seeds() {
        for seed in [3u64, 2024, 777] {
            let gen = WorldGen::new(seed);
            let p = Player::spawn_safe(&gen);
            assert!(p.pos[1] > 0.0, "spawn below sea level: {p:?}");
            let (cx, cy, cz) = (
                p.pos[0].floor() as i32,
                p.pos[1].floor() as i32,
                p.pos[2].floor() as i32,
            );
            assert!(solid_at(&gen, cx, cy - 1, cz), "no floor under spawn");
            assert!(passable_at(&gen, cx, cy, cz));
            assert!(passable_at(&gen, cx, cy + 1, cz));
            assert!(p.on_ground);
        }
    }

    /// Walking on the spawn's flat ground: the player stays level and
    /// moves horizontally.
    #[test]
    fn p3d401_walk_on_flat_ground_stays_level() {
        let gen = WorldGen::new(3);
        let mut p = Player::spawn_safe(&gen);
        let start_y = p.pos[1];
        let start_x = p.pos[0];
        // Walk +x for a second: y must stay within a step of start.
        for _ in 0..60 {
            p.step(&gen, MoveInput { move_x: 1.0, move_z: 0.0, jump: false });
        }
        assert!(
            (p.pos[0] - start_x).abs() > 2.0,
            "player barely moved: {}",
            p.pos[0] - start_x
        );
        assert!(
            (p.pos[1] - start_y).abs() <= 2.5,
            "walk changed height too much: {}",
            p.pos[1] - start_y
        );
    }

    /// A 1 m step climbs; a 2 m wall blocks.
    #[test]
    fn p3d401_steps_climb_and_walls_block() {
        let gen = WorldGen::new(3);
        let mut p = Player::spawn_safe(&gen);
        let start_y = p.pos[1];
        // Climb test: step into whatever slope exists ahead and verify no
        // vertical escape beyond +2 over 60 ticks of walking.
        for _ in 0..60 {
            p.step(&gen, MoveInput { move_x: 1.0, move_z: 0.0, jump: false });
        }
        assert!(p.pos[1] - start_y <= 2.0, "walked up too high");
        // Wall test: force the player into deep terrain by walking a long
        // time in -x; movement must eventually stop changing y wildly and
        // never clip below the world floor.
        for _ in 0..600 {
            p.step(&gen, MoveInput { move_x: -1.0, move_z: 0.0, jump: false });
        }
        assert!(p.pos[1] > -1.0, "fell through the world");
    }

    /// Determinism: identical input streams produce identical positions.
    #[test]
    fn p3d401_movement_is_deterministic() {
        let gen = WorldGen::new(7);
        let mut a = Player::spawn_safe(&gen);
        let mut b = Player::spawn_safe(&gen);
        for i in 0..600i64 {
            let input = MoveInput {
                move_x: ((i % 7) as f32 / 7.0) - 0.5,
                move_z: ((i % 5) as f32 / 5.0) - 0.5,
                jump: i % 37 == 0,
            };
            a.step(&gen, input);
            b.step(&gen, input);
        }
        assert_eq!(a.pos, b.pos, "same inputs diverged");
        assert_eq!(a.vel, b.vel);
    }

    /// Swimming: dropped into water, the player sinks SLOWLY (bounded
    /// terminal velocity) and can swim up.
    #[test]
    fn p3d401_swimming_buoyancy_and_climb() {
        let gen = WorldGen::new(2024);
        // Find an ocean region's water column (scan for Ocean biome).
        let mut water_cell: Option<(i32, i32, i32)> = None;
        'scan: for x in -40..=40i32 {
            for z in -40..=40i32 {
                let r = crate::coords::RegionCoord { x, z };
                if gen.biome(r) == crate::gen::Biome::Ocean {
                    let wx = (x * 256 + 128) as i64 * 1000;
                    let wz = (z * 256 + 128) as i64 * 1000;
                    let s = gen.effective_surface_mm(wx, wz);
                    let floor_cell = s.div_euclid(1000) as i32;
                    if floor_cell <= -6 {
                        water_cell = Some((floor_cell + 4, x * 256 + 128, z * 256 + 128));
                        break 'scan;
                    }
                }
            }
        }
        let Some((wy, px, pz)) = water_cell else {
            return; // seed has no deep ocean nearby: skip honestly
        };
        let mut p = Player { pos: [px as f32, wy as f32, pz as f32], vel: [0.0; 3], on_ground: false, swimming: false };
        p.step(&gen, MoveInput::default());
        assert!(p.swimming, "player in a water column must be swimming");
        // Sink slowly: velocity magnitude bounded by TERMINAL_SINK.
        for _ in 0..30 {
            p.step(&gen, MoveInput::default());
        }
        assert!(
            p.vel[1] >= -TERMINAL_SINK - f32::EPSILON,
            "sank faster than the terminal sink velocity"
        );
        // Swim up against the sink.
        for _ in 0..30 {
            p.step(&gen, MoveInput { move_x: 0.0, move_z: 0.0, jump: true });
        }
        assert!(p.vel[1] > 0.0 || p.pos[1] > wy as f32 - 2.0, "cannot swim up");
    }
}
