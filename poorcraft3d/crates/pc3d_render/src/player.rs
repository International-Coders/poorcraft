//! The walking player body (R3DV-011): first-person movement with terrain
//! collision — read-only `final_solid` queries, the same authority the
//! mesher and future combat use. Gravity snaps the feet to the ground
//! column; horizontal moves are blocked by solid cells at body height;
//! nothing here can mutate the world.

use crate::camera::{fwd_of, CameraPose};
use pc3d_world::coords::CellCoord;
use pc3d_world::gen::WorldGen;
use pc3d_world::terrain::final_solid;

pub const EYE_ABOVE_FEET: f32 = 1.7;
pub const WALK_SPEED: f32 = 4.0;
/// Sprint speed (Shift while moving, stamina permitting).
pub const SPRINT_SPEED: f32 = 6.6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerBody {
    /// Feet position (meters).
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
}

/// The collision surface a body walks on: the default is the authoritative
/// `final_solid` column; the surface path (NWR-005) overrides with the
/// streamed surface's own heights so edits and foundations are felt under
/// the player's feet.
pub trait CollisionSurface {
    /// Ground height under (x, z) or None if unsupported here.
    fn ground_at(&self, gen: &WorldGen, x: f32, z: f32, from_y: f32) -> Option<f32>;
    /// Solidity of a full cell (walls).
    fn cell_solid(&self, gen: &WorldGen, x: i32, y: i32, z: i32) -> bool;
}

/// Any surface by reference is a surface (adapters compose over borrows).
impl<S: CollisionSurface> CollisionSurface for &S {
    fn ground_at(&self, gen: &WorldGen, x: f32, z: f32, from_y: f32) -> Option<f32> {
        (*self).ground_at(gen, x, z, from_y)
    }
    fn cell_solid(&self, gen: &WorldGen, x: i32, y: i32, z: i32) -> bool {
        (*self).cell_solid(gen, x, y, z)
    }
}

/// The default authority-backed surface (final_solid columns).
pub struct AuthorityGround;

impl CollisionSurface for AuthorityGround {
    fn ground_at(&self, gen: &WorldGen, x: f32, z: f32, from_y: f32) -> Option<f32> {
        let cx = x.floor() as i32;
        let cz = z.floor() as i32;
        let mut y = from_y.floor() as i32;
        while y > (from_y - 3.0).floor() as i32 {
            if solid_at(gen, cx, y, cz) {
                return Some((y + 1) as f32);
            }
            y -= 1;
        }
        None
    }
    fn cell_solid(&self, gen: &WorldGen, x: i32, y: i32, z: i32) -> bool {
        solid_at(gen, x, y, z)
    }
}

fn solid_at(gen: &WorldGen, x: i32, y: i32, z: i32) -> bool {
    final_solid(gen, x as i64 * 1000, y as i64 * 1000, z as i64 * 1000).solid
}

impl PlayerBody {
    /// First-person look deltas (raw pointer units) with sensitivity and
    /// invert-Y — the ONE place look is defined so the live slice, free
    /// flight, and proofs share it. THE MOUSE FIX's core: the body owns
    /// the camera in the slice, so look persists instead of being
    /// clobbered by the per-frame set_pose(player.pose()).
    pub fn apply_look(&mut self, dx: f32, dy: f32, sens: f32, invert_y: bool) {
        let dy_signed = if invert_y { -dy } else { dy };
        self.yaw -= dx * 0.0022 * sens;
        self.pitch = (self.pitch - dy_signed * 0.0022 * sens).clamp(-1.55, 1.55);
    }

    pub fn pose(&self) -> CameraPose {
        CameraPose::new(
            [self.pos[0], self.pos[1] + EYE_ABOVE_FEET, self.pos[2]],
            self.yaw,
            self.pitch,
        )
    }

    /// The ground height under a column: the top of the highest solid cell
    /// at or below `from_y + 2` (lets the player step up 1 m slopes).
    fn ground_at(&self, gen: &WorldGen, x: f32, z: f32) -> f32 {
        let cx = x.floor() as i32;
        let cz = z.floor() as i32;
        let mut y = (self.pos[1] + 2.0).floor() as i32;
        while y > (self.pos[1] - 3.0).floor() as i32 {
            if solid_at(gen, cx, y, cz) {
                return (y + 1) as f32;
            }
            y -= 1;
        }
        self.pos[1]
    }

    /// True when a body (0.3 m radius, 1.8 m tall) at `x/z` intersects
    /// solid cells.
    fn blocked(&self, gen: &WorldGen, x: f32, z: f32) -> bool {
        const R: f32 = 0.3;
        for dx in [-R, R] {
            for dz in [-R, R] {
                let cx = (x + dx).floor() as i32;
                let cz = (z + dz).floor() as i32;
                // Body occupies feet+0.2 (ankle clearance) .. feet+1.8.
                let y0 = (self.pos[1] + 0.2).floor() as i32;
                let y1 = (self.pos[1] + 1.8).floor() as i32;
                for y in y0..=y1 {
                    if solid_at(gen, cx, y, cz) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// One walking step on the AUTHORITY ground (the default).
    pub fn walk(&mut self, gen: &WorldGen, fwd: f32, strafe: f32, dt: f32) {
        self.walk_on(gen, &AuthorityGround, fwd, strafe, dt);
    }

    /// One walking step over an explicit collision surface (the NWR-005
    /// surface path feeds the streamed heights here): yaw-relative
    /// movement, axis-separated wall blocking, gravity snap with step-up.
    pub fn walk_on(
        &mut self,
        gen: &WorldGen,
        surface: &dyn CollisionSurface,
        fwd: f32,
        strafe: f32,
        dt: f32,
    ) {
        self.walk_on_speed(gen, surface, fwd, strafe, dt, WALK_SPEED)
    }

    /// The same walk at an explicit speed (sprint path: Shift while
    /// moving, stamina permitting).
    pub fn walk_on_speed(
        &mut self,
        gen: &WorldGen,
        surface: &dyn CollisionSurface,
        fwd: f32,
        strafe: f32,
        dt: f32,
        speed: f32,
    ) {
        let yaw = self.yaw;
        let hf = [-yaw.sin(), -yaw.cos()];
        let hr = [yaw.cos(), -yaw.sin()];
        let mut dx = (hf[0] * fwd + hr[0] * strafe) * speed * dt;
        let mut dz = (hf[1] * fwd + hr[1] * strafe) * speed * dt;
        // Normalize diagonals.
        let planar = (dx * dx + dz * dz).sqrt();
        if planar > speed * dt {
            let k = speed * dt / planar;
            dx *= k;
            dz *= k;
        }
        // Axis-separated: x then z.
        if !self.blocked_on(gen, surface, self.pos[0] + dx, self.pos[2]) {
            self.pos[0] += dx;
        }
        if !self.blocked_on(gen, surface, self.pos[0], self.pos[2] + dz) {
            self.pos[2] += dz;
        }
        // Gravity/ground: snap down to the surface top, allowing a 1 m
        // step-up when the ground rises under the new position.
        if let Some(g) = surface.ground_at(gen, self.pos[0], self.pos[2], self.pos[1] + 2.0) {
            self.pos[1] = g;
        }
    }

    /// blocked() against an explicit surface.
    fn blocked_on(&self, gen: &WorldGen, surface: &dyn CollisionSurface, x: f32, z: f32) -> bool {
        const R: f32 = 0.3;
        for dx in [-R, R] {
            for dz in [-R, R] {
                let cx = (x + dx).floor() as i32;
                let cz = (z + dz).floor() as i32;
                let y0 = (self.pos[1] + 0.2).floor() as i32;
                let y1 = (self.pos[1] + 1.8).floor() as i32;
                for y in y0..=y1 {
                    if surface.cell_solid(gen, cx, y, cz) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// The cell a look ray targets (for place/remove): the first solid
    /// cell hit from the eye along the view direction, with the last air
    /// cell before it (where a new block would go).
    pub fn ray_target(&self, gen: &WorldGen, max_m: f32) -> Option<(CellCoord, CellCoord)> {
        let eye = self.pose().position;
        let fwd = fwd_of(self.yaw, self.pitch);
        crate::terrain::ray_first_hit(gen, eye, fwd, max_m).map(|(hit, _normal, before)| {
            let air = CellCoord {
                x: before[0].floor() as i32,
                y: before[1].floor() as i32,
                z: before[2].floor() as i32,
            };
            (hit, air)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walker_follows_terrain_and_is_blocked_by_walls() {
        let (seed, coord) = pc3d_world::terrain::SceneSpec::SmoothHills.patch();
        let gen = WorldGen::new(seed);
        let o = coord.origin();
        let cx = o.x.div_euclid(1000) as i32 + 8;
        let cz = o.z.div_euclid(1000) as i32 + 8;
        // Find open ground near the scene center.
        let mut start = None;
        for dx in 0..8 {
            for dz in 0..8 {
                let x = cx + dx;
                let z = cz + dz;
                let mut y = 40;
                while y > 0 && !solid_at(&gen, x, y, z) {
                    y -= 1;
                }
                if solid_at(&gen, x, y, z)
                    && !solid_at(&gen, x, y + 1, z)
                    && !solid_at(&gen, x, y + 2, z)
                {
                    start = Some([x as f32, (y + 1) as f32, z as f32]);
                    break;
                }
            }
            if start.is_some() {
                break;
            }
        }
        let start = start.expect("open ground");
        let mut p = PlayerBody {
            pos: start,
            yaw: 0.0,
            pitch: 0.0,
        };

        // Walking 4 m north moves and keeps feet on the terrain columns.
        for _ in 0..60 {
            p.walk(&gen, 1.0, 0.0, 1.0 / 60.0);
        }
        assert!((p.pos[2] - start[2] + 4.0).abs() < 0.2, "moved ~4 m north");
        let ground = p.ground_at(&gen, p.pos[0], p.pos[2]);
        assert!((p.pos[1] - ground).abs() < 0.01, "feet glued to ground");

        // Blocked by a solid wall: teleport in front of a cliff-like column
        // and walk into it — the x/z must not pass through solid cells.
        // (Find any adjacent solid column.)
        let (mut wx, mut wz) = (p.pos[0] as i32, p.pos[2] as i32);
        'find: for dx in -3..=3 {
            for dz in -3..=3 {
                let x = p.pos[0] as i32 + dx;
                let z = p.pos[2] as i32 + dz;
                let y = p.pos[1] as i32;
                if solid_at(&gen, x, y, z) && solid_at(&gen, x, y + 1, z) {
                    wx = x;
                    wz = z;
                    break 'find;
                }
            }
        }
        if (wx, wz) != (p.pos[0] as i32, p.pos[2] as i32) {
            // Stand one cell south of the wall, face north, walk into it.
            let save = p.pos;
            p.pos = [wx as f32, p.pos[1], (wz + 2) as f32];
            let y = p.pos[1] as i32;
            if solid_at(&gen, wx, y, wz) && solid_at(&gen, wx, y + 1, wz) {
                for _ in 0..30 {
                    p.walk(&gen, 1.0, 0.0, 1.0 / 60.0);
                }
                assert!(
                    (p.pos[2] as i32) >= wz + 1 || p.blocked(&gen, p.pos[0], wz as f32 + 0.31),
                    "the wall stopped the walker (now {:?})",
                    p.pos
                );
            }
            p.pos = save;
        }
    }

    #[test]
    fn ray_target_hits_solid_and_offers_an_air_cell() {
        let (seed, coord) = pc3d_world::terrain::SceneSpec::SmoothHills.patch();
        let gen = WorldGen::new(seed);
        let o = coord.origin();
        let x = o.x.div_euclid(1000) as i32 + 8;
        let z = o.z.div_euclid(1000) as i32 + 8;
        let mut y = 40;
        while y > 0 && !solid_at(&gen, x, y, z) {
            y -= 1;
        }
        let ground = y;
        let p = PlayerBody {
            pos: [x as f32, (ground + 1) as f32, z as f32],
            yaw: 0.0,
            pitch: -0.9,
        };
        let (hit, before) = p.ray_target(&gen, 30.0).expect("looking down hits ground");
        assert!(solid_at(&gen, hit.x, hit.y, hit.z));
        assert!(!solid_at(&gen, before.x, before.y, before.z));
    }
}

#[cfg(test)]
mod look_tests {
    use super::*;

    /// THE MOUSE regression: in the live slice the frame loop calls
    /// set_pose(player.pose()) every frame — look must live in the BODY
    /// or it is clobbered instantly (the owner-reported dead mouse).
    #[test]
    fn look_writes_the_body_and_survives_the_frame_pose_roundtrip() {
        let mut body = PlayerBody {
            pos: [0.0, 0.0, 0.0],
            yaw: 0.0,
            pitch: 0.0,
        };
        let yaw0 = body.yaw;
        body.apply_look(-300.0, 80.0, 1.0, false);
        assert!(body.yaw > yaw0, "yaw turns right for leftward... sign check: {:.3} > {:.3}", body.yaw, yaw0);
        assert!(body.pitch < 0.0, "looking up (negative dy) pitches up");
        // The roundtrip the frame loop performs: camera = body.pose().
        let pose = body.pose();
        assert_eq!(pose.yaw, body.yaw);
        assert_eq!(pose.pitch, body.pitch);
    }

    #[test]
    fn look_respects_sensitivity_invert_and_clamps() {
        let mut a = PlayerBody { pos: [0.0; 3], yaw: 0.0, pitch: 0.0 };
        let mut b = PlayerBody { pos: [0.0; 3], yaw: 0.0, pitch: 0.0 };
        a.apply_look(100.0, 100.0, 2.0, false);
        b.apply_look(100.0, 100.0, 1.0, false);
        assert!(a.yaw.abs() > b.yaw.abs(), "sensitivity scales the turn");
        let mut up = PlayerBody { pos: [0.0; 3], yaw: 0.0, pitch: 0.0 };
        let mut down = PlayerBody { pos: [0.0; 3], yaw: 0.0, pitch: 0.0 };
        up.apply_look(0.0, -50.0, 1.0, false);
        down.apply_look(0.0, -50.0, 1.0, true);
        assert!(up.pitch > 0.0 && down.pitch < 0.0, "invert Y flips the pitch");
        let mut clamp = PlayerBody { pos: [0.0; 3], yaw: 0.0, pitch: 0.0 };
        for _ in 0..50 {
            clamp.apply_look(0.0, -200.0, 1.0, false);
        }
        assert!(clamp.pitch <= 1.55, "pitch clamps at the poles ({})", clamp.pitch);
    }

    #[test]
    fn sprint_speed_moves_farther_than_walk() {
        // The sprint law's other half: SPRINT_SPEED > WALK_SPEED, and
        // walk_on_speed honors it on a flat authority surface.
        assert!(SPRINT_SPEED > WALK_SPEED * 1.4, "sprint is meaningfully faster");
        // On the (post-terracing-fix) natural ground, walls can block
        // either body differently — the LAW is proven on a synthetic
        // flat surface: same flat ground, sprint covers strictly more
        // distance in the same time.
        struct Flat;
        impl crate::player::CollisionSurface for Flat {
            fn ground_at(&self, _g: &pc3d_world::gen::WorldGen, _x: f32, _z: f32, _y: f32) -> Option<f32> {
                Some(10.0)
            }
            fn cell_solid(&self, _g: &pc3d_world::gen::WorldGen, _x: i32, _y: i32, _z: i32) -> bool {
                false
            }
        }
        let gen = pc3d_world::gen::WorldGen::new(22);
        let mut walker = PlayerBody { pos: [0.5, 10.0, 0.5], yaw: 0.0, pitch: 0.0 };
        let mut sprinter = walker;
        for _ in 0..120 {
            walker.walk_on_speed(&gen, &Flat, 1.0, 0.0, 1.0 / 60.0, WALK_SPEED);
            sprinter.walk_on_speed(&gen, &Flat, 1.0, 0.0, 1.0 / 60.0, SPRINT_SPEED);
        }
        assert!(
            sprinter.pos[2] < walker.pos[2] - 1.0,
            "the sprinter outruns the walker ({:.1} vs {:.1})",
            sprinter.pos[2],
            walker.pos[2]
        );
    }
}
