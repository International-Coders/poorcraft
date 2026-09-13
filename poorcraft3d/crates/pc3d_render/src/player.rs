//! The walking player body (R3DV-011): first-person movement with terrain
//! collision — read-only `final_solid` queries, the same authority the
//! mesher and future combat use. Gravity snaps the feet to the ground
//! column; horizontal moves are blocked by solid cells at body height AND
//! by THE RISE LAW (a rise the surface doesn't allow as a step or a
//! walkable ramp refuses the move); nothing here can mutate the world.

use crate::camera::{fwd_of, CameraPose};
use pc3d_world::coords::CellCoord;
use pc3d_world::gen::WorldGen;
use pc3d_world::terrain::final_solid;

pub const EYE_ABOVE_FEET: f32 = 1.7;
pub const WALK_SPEED: f32 = 4.0;
/// Sprint speed (Shift while moving, stamina permitting).
pub const SPRINT_SPEED: f32 = 6.6;

/// A support gap beyond one walking step: when the ground answer sits
/// this far below the feet, no surface holds the body — the walk's snap
/// refuses it (THE STEP LAW) and the slice's walk-off commit (this
/// predicate against the surface truth) begins the fall.
pub const SUPPORT_GAP_M: f32 = 1.05;

/// THE WALK-OFF LAW (pure): the body is unsupported when the ground
/// answer sits more than one step below the feet — the frame support
/// vanishes (a ledge walked off, a floor dug out), the fall begins.
pub fn is_unsupported(feet_y: f32, ground_y: f32) -> bool {
    feet_y - ground_y > SUPPORT_GAP_M
}

/// THE RISE LAW (pure): a walk may raise the feet only onto a surface
/// that is itself walkable — a discrete STEP within one step height
/// (`SUPPORT_GAP_M`, the up twin of the down law) or a RAMP within the
/// nav's own walkability contract (`crate::surface::MAX_WALK_SLOPE`,
/// scaled by the distance actually moved this frame, so the verdict is
/// the same at any refresh rate). `slope` is the surface's rise ALONG
/// THE MOVE over a 1 m baseline (the mesh's own node spacing), SIGNED:
/// a descent ahead is not a wall (the down law owns descents), a rise
/// steeper than the walkable contract is. An unanswerable slope
/// refuses.
pub fn rise_accepted(rise: f32, slope: Option<f32>, move_m: f32) -> bool {
    let Some(slope) = slope else {
        return false;
    };
    if slope > crate::surface::MAX_WALK_SLOPE {
        return false;
    }
    rise <= SUPPORT_GAP_M || rise <= crate::surface::MAX_WALK_SLOPE * move_m + 1e-4
}

/// One press of the dig verb lowers the targeted column's ground this
/// far — exactly one walking step (the rise law admits <= 1.05 m), so a
/// single dig is a terrace a body can step down into AND back out of;
/// deeper shafts are dug by repeated presses and then HOLD the body
/// (the wall law) until terraces carry the climber out.
pub const DIG_DEPTH_M: f32 = 1.0;
/// The dig reaches as far as the build verb's placement ray.
pub const DIG_REACH_M: f32 = 8.0;

/// THE DIG TARGET (pure): march the look ray and answer the first
/// column whose LIVE ground the ray enters, within reach. The ground
/// closure is the same streamed answer the walk stands on and the
/// picture draws (delta layer included), so the verb can never target
/// ground the player cannot see or stand on — re-aiming at a dug
/// terrace flies over it to the next surface. A level gaze across flat
/// ground digs nothing; a wall ahead takes the dig at its face.
pub fn dig_target(
    ground: &dyn Fn(f32, f32) -> f32,
    eye: [f32; 3],
    fwd: [f32; 3],
    reach_m: f32,
) -> Option<(i32, i32)> {
    const STEP: f32 = 0.1;
    let mut t = STEP;
    while t <= reach_m {
        let p = [eye[0] + fwd[0] * t, eye[1] + fwd[1] * t, eye[2] + fwd[2] * t];
        if p[1] <= ground(p[0], p[2]) + 0.05 {
            return Some((p[0].floor() as i32, p[2].floor() as i32));
        }
        t += STEP;
    }
    None
}

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
        // Axis-separated: x then z. Each axis is stopped by a solid
        // cell (walls) AND by THE RISE LAW: the streamed surface's
        // cell_solid answers false everywhere ("slopes gate walkability
        // through ground_at stepping"), so the ground answer is the
        // streamed path's only wall — and the old snap accepted ANY
        // rise (`pos[1] - g <= SUPPORT_GAP_M` is trivially true when g
        // is above the feet), making every cliff and dug wall a free
        // elevator.
        if !self.blocked_on(gen, surface, self.pos[0] + dx, self.pos[2])
            && !self.rise_refused_on(gen, surface, self.pos[0] + dx, self.pos[2], dx, 0.0)
        {
            self.pos[0] += dx;
        }
        if !self.blocked_on(gen, surface, self.pos[0], self.pos[2] + dz)
            && !self.rise_refused_on(gen, surface, self.pos[0], self.pos[2] + dz, 0.0, dz)
        {
            self.pos[2] += dz;
        }
        // Gravity/ground: snap to the surface top ONLY within one step
        // of the feet (1 m up / SUPPORT_GAP_M down — THE STEP LAW). A
        // deeper drop belongs to the FALL: refusing the snap leaves the
        // gap the slice's walk-off commit reads (is_unsupported against
        // the same surface truth the arc lands on), so a walked-off body
        // falls and takes the impact law's damage. The unbounded snap
        // used to swallow every drop the streamed surface answered —
        // walk-offs landed soft and the commit was dead code there.
        if let Some(g) = surface.ground_at(gen, self.pos[0], self.pos[2], self.pos[1] + 2.0) {
            if self.pos[1] - g <= SUPPORT_GAP_M {
                self.pos[1] = g;
            }
        }
    }

    /// THE RISE LAW, sampled (pure read): true when the axis move into
    /// (x, z) would raise the feet onto a surface the walk may not
    /// climb. The slope is the ground answer's rise ALONG THE MOVE over
    /// a 1 m baseline (the mesh's own node spacing), SIGNED — a descent
    /// ahead is never a wall (the down law owns descents; a body may
    /// always walk AWAY from a rise), only a rise steeper than the
    /// walkable contract is. A missing answer refuses the rise (the
    /// snap cannot verify walkability); a rise at or below the feet
    /// belongs to the down law, not this one.
    fn rise_refused_on(
        &self,
        gen: &WorldGen,
        surface: &dyn CollisionSurface,
        x: f32,
        z: f32,
        axis_dx: f32,
        axis_dz: f32,
    ) -> bool {
        let Some(g) = surface.ground_at(gen, x, z, self.pos[1] + 2.0) else {
            return false; // no answer here: the snap ignores it too
        };
        let rise = g - self.pos[1];
        if rise <= 0.0 {
            return false; // descending or flat: THE STEP LAW owns it
        }
        let len = (axis_dx * axis_dx + axis_dz * axis_dz).sqrt();
        if len <= 0.0 {
            return false; // no move, nothing to refuse
        }
        let (ux, uz) = (axis_dx / len, axis_dz / len);
        let slope = match (
            surface.ground_at(gen, x - ux * 0.5, z - uz * 0.5, self.pos[1] + 2.0),
            surface.ground_at(gen, x + ux * 0.5, z + uz * 0.5, self.pos[1] + 2.0),
        ) {
            // Signed rise along the move, over exactly 1 m: ahead is
            // FURTHER ALONG the move than behind.
            (Some(behind), Some(ahead)) => ahead - behind,
            _ => f32::INFINITY, // unverifiable: refuse
        };
        !rise_accepted(rise, Some(slope), len)
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

    /// THE STEP LAW (the walk-off's ground half): the walk holds a
    /// step down (0.5 m snaps) and REFUSES a ledge (2.5 m — y stays,
    /// and the gap is_unsupported reads is left standing for the
    /// slice's walk-off commit). The stub answers the true surface
    /// IGNORING from_y — the SurfaceStreamer's exact shape, where the
    /// old unbounded snap swallowed every walked-off drop soft.
    #[test]
    fn the_walk_holds_a_step_and_refuses_a_ledged_drop() {
        struct StreamedLedge {
            edge_z: f32,
            top: f32,
            bottom: f32,
        }
        impl crate::player::CollisionSurface for StreamedLedge {
            fn ground_at(
                &self,
                _g: &pc3d_world::gen::WorldGen,
                _x: f32,
                z: f32,
                _y: f32,
            ) -> Option<f32> {
                Some(if z > self.edge_z { self.top } else { self.bottom })
            }
            fn cell_solid(&self, _g: &pc3d_world::gen::WorldGen, _x: i32, _y: i32, _z: i32) -> bool {
                false
            }
        }
        let gen = pc3d_world::gen::WorldGen::new(22);
        // A STEP down: walked and held (the body follows the surface).
        let mut walker = PlayerBody { pos: [0.5, 10.0, 0.5], yaw: 0.0, pitch: 0.0 };
        for _ in 0..40 {
            walker.walk_on_speed(
                &gen,
                &StreamedLedge { edge_z: 2.0, top: 10.0, bottom: 9.5 },
                1.0,
                0.0,
                1.0 / 60.0,
                WALK_SPEED,
            );
        }
        assert!(
            walker.pos[2] < 2.0,
            "the walk carried the body past the step (z {:.2})",
            walker.pos[2]
        );
        assert!(
            (walker.pos[1] - 9.5).abs() < 0.01,
            "a step down is walked ({:.2})",
            walker.pos[1]
        );
        // A LEDGE: the drop is refused — the feet stay, and the gap the
        // commit reads (the SAME surface truth the arc lands on) says
        // unsupported. The fall, the impact, and the damage belong to
        // the slice's shared airborne machinery, never to the walk.
        let mut lemming = PlayerBody { pos: [0.5, 10.0, 0.5], yaw: 0.0, pitch: 0.0 };
        for _ in 0..40 {
            lemming.walk_on_speed(
                &gen,
                &StreamedLedge { edge_z: 2.0, top: 10.0, bottom: 7.5 },
                1.0,
                0.0,
                1.0 / 60.0,
                WALK_SPEED,
            );
        }
        assert!(
            lemming.pos[2] < 2.0,
            "the walk carried the body past the edge (z {:.2})",
            lemming.pos[2]
        );
        assert!(
            (lemming.pos[1] - 10.0).abs() < 0.01,
            "a ledged drop is NOT walked ({:.2} — the old snap landed it soft)",
            lemming.pos[1]
        );
        assert!(
            is_unsupported(lemming.pos[1], 7.5),
            "the ledge gap commits the fall"
        );
    }
}

#[cfg(test)]
mod rise_law_tests {
    use super::*;

    /// A cell-constant surface: named cells answer their height, all
    /// other ground is 0. The streamed surface ramps within the border
    /// cell (bilinear); the LAW reads the same border algebra either
    /// way — the ±0.5 m slope baseline spans the border in both shapes.
    struct Cells {
        heights: Vec<((i32, i32), f32)>,
    }
    impl Cells {
        fn h(&self, x: f32, z: f32) -> f32 {
            let key = (x.floor() as i32, z.floor() as i32);
            self.heights
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, h)| *h)
                .unwrap_or(0.0)
        }
    }
    impl crate::player::CollisionSurface for Cells {
        fn ground_at(
            &self,
            _g: &pc3d_world::gen::WorldGen,
            x: f32,
            z: f32,
            _y: f32,
        ) -> Option<f32> {
            Some(self.h(x, z))
        }
        fn cell_solid(
            &self,
            _g: &pc3d_world::gen::WorldGen,
            _x: i32,
            _y: i32,
            _z: i32,
        ) -> bool {
            false // the streamed surface's exact shape: no walls but the rise law
        }
    }

    /// W with yaw 0 walks -z (hf = [-sin yaw, -cos yaw]); every body in
    /// these laws faces -z.
    const PIT_FLOOR: f32 = -5.0;

    /// THE UP-STEP LAW (the up twin of the down law): a 0.5 m step is
    /// WALKED (the body crosses and its feet hold the top); a 5 m wall
    /// REFUSES the move — the body stays on its own side at its own
    /// height, where the old snap teleported it to the top.
    #[test]
    fn a_step_up_is_walked_and_a_wall_refuses_the_move() {
        let gen = pc3d_world::gen::WorldGen::new(22);
        // A STEP up (the cell toward -z): walked and held.
        let mut walker = PlayerBody { pos: [0.5, 0.0, 0.5], yaw: 0.0, pitch: 0.0 };
        for _ in 0..20 {
            walker.walk_on_speed(
                &gen,
                &Cells { heights: vec![((0, -1), 0.5)] },
                1.0,
                0.0,
                1.0 / 60.0,
                WALK_SPEED,
            );
        }
        assert!(
            walker.pos[2] < 0.0 && walker.pos[2] > -1.0,
            "the walk carried the body onto the step cell (z {:.2})",
            walker.pos[2]
        );
        assert!(
            (walker.pos[1] - 0.5).abs() < 0.01,
            "a step up is walked ({:.2})",
            walker.pos[1]
        );
        // A WALL: the move is refused — the body never crosses into the
        // wall cell and its feet never lift (the old snap landed it on
        // top).
        let wall = Cells { heights: vec![((0, -1), 5.0)] };
        let mut climber = PlayerBody { pos: [0.5, 0.0, 0.5], yaw: 0.0, pitch: 0.0 };
        for _ in 0..120 {
            climber.walk_on_speed(&gen, &wall, 1.0, 0.0, 1.0 / 60.0, WALK_SPEED);
        }
        assert!(
            climber.pos[2] >= -0.05 && climber.pos[2] <= 0.55,
            "the wall stopped the walk (z {:.2})",
            climber.pos[2]
        );
        assert!(
            climber.pos[1].abs() < 0.01,
            "the feet stayed at the base ({:.2} — the old snap lifted them)",
            climber.pos[1]
        );
        assert!(
            !is_unsupported(climber.pos[1], wall.h(climber.pos[0], climber.pos[2])),
            "standing at a wall is supported (no spurious walk-off)"
        );
    }

    /// THE RISE VERDICT IS GEOMETRY, NOT THE FRAME: a walkable ramp is
    /// climbed to the same top at 30/60/120 fps, and a cliff face
    /// refuses at all three — the old snap's acceptance grew with the
    /// frame step, so steep ground changed character with the refresh
    /// rate.
    #[test]
    fn the_rise_verdict_is_the_slope_not_the_frame() {
        let gen = pc3d_world::gen::WorldGen::new(22);
        // A 1.2 m/m ramp rising toward -z (0 m at z=0, 9.6 m at z=-8) —
        // inside MAX_WALK_SLOPE: climbable at every refresh rate.
        struct Ramp;
        impl crate::player::CollisionSurface for Ramp {
            fn ground_at(
                &self,
                _g: &pc3d_world::gen::WorldGen,
                _x: f32,
                z: f32,
                _y: f32,
            ) -> Option<f32> {
                Some(1.2 * (-z).clamp(0.0, 8.0))
            }
            fn cell_solid(
                &self,
                _g: &pc3d_world::gen::WorldGen,
                _x: i32,
                _y: i32,
                _z: i32,
            ) -> bool {
                false
            }
        }
        for fps in [30.0f32, 60.0, 120.0] {
            let dt = 1.0 / fps;
            let mut body = PlayerBody { pos: [0.5, 0.0, 0.5], yaw: 0.0, pitch: 0.0 };
            for _ in 0..(9.0 / (WALK_SPEED * dt)) as usize {
                body.walk_on_speed(&gen, &Ramp, 1.0, 0.0, dt, WALK_SPEED);
            }
            assert!(
                body.pos[1] > 9.0,
                "the ramp is climbed at {fps} fps (feet {:.2})",
                body.pos[1]
            );
        }
        // A 5 m/m face (the dug pit's ramp slope): refused at every rate.
        struct Face;
        impl crate::player::CollisionSurface for Face {
            fn ground_at(
                &self,
                _g: &pc3d_world::gen::WorldGen,
                _x: f32,
                z: f32,
                _y: f32,
            ) -> Option<f32> {
                Some(if z < -1.0 { 5.0 * (-z - 1.0).min(1.0) } else { 0.0 })
            }
            fn cell_solid(
                &self,
                _g: &pc3d_world::gen::WorldGen,
                _x: i32,
                _y: i32,
                _z: i32,
            ) -> bool {
                false
            }
        }
        for fps in [30.0f32, 60.0, 120.0] {
            let dt = 1.0 / fps;
            let mut body = PlayerBody { pos: [0.5, 0.0, 0.5], yaw: 0.0, pitch: 0.0 };
            for _ in 0..120 {
                body.walk_on_speed(&gen, &Face, 1.0, 0.0, dt, WALK_SPEED);
            }
            assert!(
                body.pos[2] < 0.0 && body.pos[2] >= -1.55,
                "the walk approached the face at {fps} fps and was held (z {:.2})",
                body.pos[2]
            );
            assert!(
                body.pos[1].abs() < 0.05,
                "the face refuses at {fps} fps (feet {:.2} — the old snap climbed it)",
                body.pos[1]
            );
        }
    }

    /// THE DUG PIT HOLDS THE BODY UNTIL THE STEP (the route's shape at
    /// unit level): in a one-cell pit five meters down, walking into
    /// the wall holds the body inside; the ONE cell dug to 4.5 m — a
    /// 0.5 m step — admits the walk, and the body stands on the step,
    /// still held by the wall beyond it. Dig down, and only steps or
    /// ramps bring you out.
    #[test]
    fn the_dug_pit_holds_the_body_until_the_step() {
        let gen = pc3d_world::gen::WorldGen::new(22);
        // Pit cell (0,0) at -5; the step cell (0,-1) at -4.5. The rim
        // (everything else) is 0.
        let pit = Cells {
            heights: vec![((0, 0), PIT_FLOOR), ((0, -1), PIT_FLOOR + 0.5)],
        };
        let mut body = PlayerBody { pos: [0.5, PIT_FLOOR, 0.5], yaw: 0.0, pitch: 0.0 };
        // Face +z (yaw = PI) and hold W: the pit wall refuses.
        body.yaw = std::f32::consts::PI;
        for _ in 0..120 {
            body.walk_on_speed(&gen, &pit, 1.0, 0.0, 1.0 / 60.0, WALK_SPEED);
        }
        assert!(
            body.pos[2] > 0.8 && body.pos[2] <= 1.05,
            "the body walked to the pit wall and was held (z {:.2})",
            body.pos[2]
        );
        assert!(
            (body.pos[1] - PIT_FLOOR).abs() < 0.01,
            "the feet stayed on the pit floor ({:.2})",
            body.pos[1]
        );
        // Face -z (yaw = 0): the 0.5 m step admits the body.
        body.yaw = 0.0;
        for _ in 0..60 {
            body.walk_on_speed(&gen, &pit, 1.0, 0.0, 1.0 / 60.0, WALK_SPEED);
        }
        assert!(
            body.pos[2] < -0.9,
            "the walk carried the body onto the step cell (z {:.2})",
            body.pos[2]
        );
        assert!(
            (body.pos[1] - PIT_FLOOR - 0.5).abs() < 0.01,
            "the feet hold the step ({:.2})",
            body.pos[1]
        );
        // Keep walking -z: the step cell's own outer wall refuses.
        for _ in 0..120 {
            body.walk_on_speed(&gen, &pit, 1.0, 0.0, 1.0 / 60.0, WALK_SPEED);
        }
        assert!(
            body.pos[2] >= -1.55,
            "the outer wall held the body (z {:.2} — escaped the pit)",
            body.pos[2]
        );
        assert!(
            (body.pos[1] - PIT_FLOOR - 0.5).abs() < 0.01,
            "still on the step, not the rim ({:.2})",
            body.pos[1]
        );
    }

    /// THE PROBE'S LAW (a live-route staging bug, promoted): a body
    /// pressed against a wall may always walk AWAY from it — the slope
    /// verdict is the rise ALONG THE MOVE, signed, so the steep ramp
    /// BEHIND the body never refuses a step across flat ground in
    /// front. (The first draft measured |ahead - behind|: the wall
    /// behind poisoned the baseline and the body stood glued to the
    /// wall it had just been refused by — and the stuck state unlocked
    /// only when frame-rate jitter reshuffled the samples.)
    #[test]
    fn the_body_walks_away_from_a_wall_it_was_refused_by() {
        let gen = pc3d_world::gen::WorldGen::new(22);
        // The pit shape: a flat floor cell, a rising wall cell toward
        // -z (5 m/m ramp in the neighbor). The body stands with its
        // back to the -z wall, on the flat floor.
        let pit = Cells {
            heights: vec![((0, 0), PIT_FLOOR), ((0, -1), 0.0)],
        };
        let mut body = PlayerBody { pos: [0.5, PIT_FLOOR, 0.5], yaw: 0.0, pitch: 0.0 };
        // Walk -z into the wall: refused at the border.
        for _ in 0..60 {
            body.walk_on_speed(&gen, &pit, 1.0, 0.0, 1.0 / 60.0, WALK_SPEED);
        }
        assert!(body.pos[2] >= -0.05, "the wall held (z {:.2})", body.pos[2]);
        // Turn around (+z) and walk away: NOT refused by the wall
        // behind — the body crosses its own cell toward +z.
        body.yaw = std::f32::consts::PI;
        for _ in 0..20 {
            body.walk_on_speed(&gen, &pit, 1.0, 0.0, 1.0 / 60.0, WALK_SPEED);
        }
        assert!(
            body.pos[2] > 0.8,
            "the body walked away from the wall (z {:.2} — glued by the baseline)",
            body.pos[2]
        );
        assert!(
            (body.pos[1] - PIT_FLOOR).abs() < 0.01,
            "the feet stayed on the flat floor ({:.2})",
            body.pos[1]
        );
    }
}

/// THE DIG TARGET laws (pure — a synthetic ground closure, no world).
#[cfg(test)]
mod dig_target_tests {
    use super::*;

    const EYE: [f32; 3] = [0.5, 1.7, 0.5];

    #[test]
    fn the_look_ray_digs_the_first_column_it_meets() {
        // Flat ground: a down-forward gaze (pitch -0.6, facing -z)
        // meets the surface ~3 m out — inside column (0, -2), NOT at
        // max reach, NOT underfoot.
        let flat = |_: f32, _: f32| 0.0f32;
        let fwd = fwd_of(0.0, -0.6);
        let (tx, tz) = dig_target(&flat, EYE, fwd, DIG_REACH_M).expect("flat ground in reach");
        assert_eq!((tx, tz), (0, -2), "the first met column takes the dig");
    }

    #[test]
    fn a_dug_terrace_moves_the_target_past_it() {
        // The terrace at (0,-2) sits 1 m lower: the SAME aim flies over
        // it and meets the live surface beyond — the verb targets what
        // the picture now draws, never the original skin.
        let trenched = |x: f32, z: f32| {
            if x.floor() == 0.0 && z.floor() == -2.0 {
                -1.0
            } else {
                0.0
            }
        };
        let fwd = fwd_of(0.0, -0.6);
        let (tx, tz) = dig_target(&trenched, EYE, fwd, DIG_REACH_M).expect("ground beyond the terrace");
        assert_eq!(
            (tx, tz),
            (0, -3),
            "the ray flew over the dug terrace to the next surface"
        );
    }

    #[test]
    fn a_wall_takes_the_dig_at_its_face() {
        // A 2 m wall toward -z: a near-level gaze hits the wall's FACE
        // column (-1), not a cell beyond it and not the ground below.
        let wall = |_: f32, z: f32| if z < 0.0 { 2.0 } else { 0.0 };
        let fwd = fwd_of(0.0, -0.2);
        let (tx, tz) = dig_target(&wall, EYE, fwd, DIG_REACH_M).expect("the wall is in reach");
        assert_eq!((tx, tz), (0, -1), "the wall's face column takes the dig");
    }

    #[test]
    fn a_level_gaze_across_flat_ground_digs_nothing() {
        let flat = |_: f32, _: f32| 0.0f32;
        assert!(
            dig_target(&flat, EYE, fwd_of(0.0, 0.0), DIG_REACH_M).is_none(),
            "nothing in reach to dig"
        );
    }

    #[test]
    fn a_steep_gaze_digs_underfoot() {
        // Nearly straight down: the eye's own column — the walk-off
        // trick is a legal player verb now (the walk-off law owns the
        // fall it causes).
        let flat = |_: f32, _: f32| 0.0f32;
        let (tx, tz) = dig_target(&flat, EYE, fwd_of(0.0, -1.5), DIG_REACH_M)
            .expect("the ground underfoot is in reach");
        assert_eq!((tx, tz), (0, 0), "the body digs its own column");
    }
}
