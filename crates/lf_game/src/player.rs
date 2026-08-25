use glam::Vec3;
use lf_voxel::World;

/// Player AABB: 0.6 wide, 1.8 tall. Position is the feet center.
pub const PLAYER_HALF_WIDTH: f32 = 0.3;
pub const PLAYER_HEIGHT: f32 = 1.8;
pub const EYE_HEIGHT: f32 = 1.62;

const GRAVITY: f32 = 32.0; // blocks / s^2
const JUMP_VELOCITY: f32 = 9.2; // ~1.3 block jump apex
const WALK_SPEED: f32 = 4.3;
const SPRINT_SPEED: f32 = 5.6;
const FLY_SPEED: f32 = 11.0;
const TERMINAL_VELOCITY: f32 = -60.0;
const MAX_STEP: f32 = 0.05; // physics substep, blocks

/// Per-frame input from the client.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerInput {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sneak: bool,
    pub sprint: bool,
    pub fly_up: bool,
    pub fly_down: bool,
    /// Radians this frame (already sensitivity-scaled).
    pub yaw_delta: f32,
    pub pitch_delta: f32,
}

#[derive(Clone, Debug)]
pub struct Player {
    /// Feet center position in world space.
    pub position: Vec3,
    pub velocity: Vec3,
    /// Yaw in radians: 0 looks toward -Z, positive turns right (+X).
    pub yaw: f32,
    /// Pitch in radians, clamped to +-89 degrees.
    pub pitch: f32,
    pub on_ground: bool,
    pub flying: bool,
    /// True during the frame the player landed (for landing effects later).
    pub just_landed: bool,
}

impl Player {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            flying: false,
            just_landed: false,
        }
    }

    pub fn eye_position(&self) -> Vec3 {
        Vec3::new(self.position.x, self.position.y + EYE_HEIGHT, self.position.z)
    }

    /// Unit look direction from yaw/pitch.
    pub fn look_dir(&self) -> Vec3 {
        let (sin_p, cos_p) = self.pitch.sin_cos();
        let (sin_y, cos_y) = self.yaw.sin_cos();
        Vec3::new(sin_y * cos_p, sin_p, -cos_y * cos_p)
    }

    pub fn apply_look(&mut self, yaw_delta: f32, pitch_delta: f32) {
        self.yaw += yaw_delta;
        self.pitch = (self.pitch + pitch_delta).clamp(-89f32.to_radians(), 89f32.to_radians());
        // keep yaw in -pi..pi for stable serialization
        if self.yaw > std::f32::consts::PI {
            self.yaw -= std::f32::consts::TAU;
        } else if self.yaw < -std::f32::consts::PI {
            self.yaw += std::f32::consts::TAU;
        }
    }

    /// Advance physics by dt seconds with the given input.
    pub fn update(&mut self, dt: f32, input: &PlayerInput, world: &World) {
        self.just_landed = false;
        self.apply_look(input.yaw_delta, input.pitch_delta);

        // Substep so fast falls never tunnel through blocks.
        let mut remaining = dt.min(0.25);
        while remaining > 0.0 {
            let step = remaining.min(MAX_STEP);
            self.substep(step, input, world);
            remaining -= step;
        }
    }

    fn substep(&mut self, dt: f32, input: &PlayerInput, world: &World) {
        // Horizontal wish direction in world space from yaw.
        let (sin_y, cos_y) = self.yaw.sin_cos();
        let forward = Vec3::new(sin_y, 0.0, -cos_y);
        let right = Vec3::new(cos_y, 0.0, sin_y);
        let mut wish = Vec3::ZERO;
        if input.forward { wish += forward; }
        if input.back { wish -= forward; }
        if input.right { wish += right; }
        if input.left { wish -= right; }
        if wish.length_squared() > 0.0 {
            wish = wish.normalize();
        }

        if self.flying {
            let speed = FLY_SPEED;
            self.velocity = wish * speed;
            self.velocity.y = if input.fly_up { speed } else if input.fly_down { -speed } else { 0.0 };
        } else {
            let speed = if input.sprint { SPRINT_SPEED } else { WALK_SPEED };
            self.velocity.x = wish.x * speed;
            self.velocity.z = wish.z * speed;
            if input.jump && self.on_ground {
                self.velocity.y = JUMP_VELOCITY;
                self.on_ground = false;
            }
            self.velocity.y = (self.velocity.y - GRAVITY * dt).max(TERMINAL_VELOCITY);
        }

        // Axis-separated integrate + collide.
        self.move_axis(world, Axis::X, self.velocity.x * dt);
        self.move_axis(world, Axis::Z, self.velocity.z * dt);
        let was_falling = self.velocity.y < 0.0;
        self.move_axis(world, Axis::Y, self.velocity.y * dt);
        let _ = was_falling;
    }

    fn move_axis(&mut self, world: &World, axis: Axis, delta: f32) {
        if delta == 0.0 {
            return;
        }
        let mut pos = self.position;
        match axis {
            Axis::X => pos.x += delta,
            Axis::Y => pos.y += delta,
            Axis::Z => pos.z += delta,
        }

        let aabb = aabb_of(pos);
        if !intersects_solid(world, &aabb) {
            self.position = pos;
            if axis == Axis::Y && delta < 0.0 {
                self.on_ground = false;
            }
            return;
        }

        // Collision: clamp to the touching block plane.
        match axis {
            Axis::X => {
                if delta > 0.0 {
                    self.position.x = aabb.max.x.floor() - PLAYER_HALF_WIDTH - 1e-4;
                } else {
                    self.position.x = aabb.min.x.ceil() + PLAYER_HALF_WIDTH + 1e-4;
                }
                self.velocity.x = 0.0;
            }
            Axis::Z => {
                if delta > 0.0 {
                    self.position.z = aabb.max.z.floor() - PLAYER_HALF_WIDTH - 1e-4;
                } else {
                    self.position.z = aabb.min.z.ceil() + PLAYER_HALF_WIDTH + 1e-4;
                }
                self.velocity.z = 0.0;
            }
            Axis::Y => {
                if delta > 0.0 {
                    // hit ceiling
                    self.position.y = aabb.max.y.floor() - PLAYER_HEIGHT - 1e-4;
                } else {
                    // land on floor
                    self.position.y = aabb.min.y.ceil() + 1e-4;
                    self.on_ground = true;
                    self.just_landed = true;
                }
                self.velocity.y = 0.0;
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum Axis { X, Y, Z }

#[derive(Copy, Clone, Debug)]
struct Aabb {
    min: Vec3,
    max: Vec3,
}

fn aabb_of(pos: Vec3) -> Aabb {
    Aabb {
        min: Vec3::new(pos.x - PLAYER_HALF_WIDTH, pos.y, pos.z - PLAYER_HALF_WIDTH),
        max: Vec3::new(pos.x + PLAYER_HALF_WIDTH, pos.y + PLAYER_HEIGHT, pos.z + PLAYER_HALF_WIDTH),
    }
}

fn intersects_solid(world: &World, aabb: &Aabb) -> bool {
    let min = aabb.min.floor().as_ivec3();
    let max = (aabb.max - Vec3::splat(1e-6)).floor().as_ivec3();
    for x in min.x..=max.x {
        for y in min.y..=max.y {
            for z in min.z..=max.z {
                if world.is_solid(x, y, z) {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use lf_voxel::BlockState;

    /// Flat world: solid stone floor at y=0..=0, optionally a wall or ceiling.
    struct Fixture {
        world: World,
    }

    impl Fixture {
        fn flat() -> Self {
            let mut world = World::new();
            for cx in -2..=2 {
                for cz in -2..=2 {
                    world.ensure_chunk(cx, cz);
                    for lx in 0..16 {
                        for lz in 0..16 {
                            world.set_block(cx * 16 + lx, 0, cz * 16 + lz, BlockState::STONE);
                        }
                    }
                }
            }
            Self { world }
        }

        fn wall_x(&mut self, x: i32) {
            for z in -32..32 {
                for y in 1..4 {
                    self.world.set_block(x, y, z, BlockState::STONE);
                }
            }
        }

        fn ceiling(&mut self, y: i32) {
            for x in -32..32 {
                for z in -32..32 {
                    self.world.set_block(x, y, z, BlockState::STONE);
                }
            }
        }
    }

    fn no_input() -> PlayerInput {
        PlayerInput::default()
    }

    fn run_frames(player: &mut Player, input: &PlayerInput, world: &World, secs: f32) {
        let mut t = 0.0;
        while t < secs {
            player.update(1.0 / 60.0, input, world);
            t += 1.0 / 60.0;
        }
    }

    #[test]
    fn gravity_lands_on_floor() {
        let f = Fixture::flat();
        let mut p = Player::new(Vec3::new(0.5, 10.0, 0.5));
        run_frames(&mut p, &no_input(), &f.world, 2.0);
        assert!(p.on_ground, "should be on ground");
        assert!((p.position.y - 1.0).abs() < 0.01, "y={} want 1.0 (floor top)", p.position.y);
    }

    #[test]
    fn walking_moves_and_wall_stops() {
        let mut f = Fixture::flat();
        f.wall_x(5);
        let mut p = Player::new(Vec3::new(0.5, 1.0, 0.5));
        p.yaw = std::f32::consts::FRAC_PI_2; // face +X
        let input = PlayerInput { forward: true, ..Default::default() };
        run_frames(&mut p, &input, &f.world, 1.0);
        assert!(p.position.x > 1.0, "should have moved +X, at {}", p.position.x);
        run_frames(&mut p, &input, &f.world, 5.0);
        assert!(p.position.x < 5.0 - PLAYER_HALF_WIDTH + 1e-3,
            "wall should stop player before x=4.7, at {}", p.position.x);
        assert!((p.position.x - (5.0 - PLAYER_HALF_WIDTH)).abs() < 0.05);
    }

    #[test]
    fn jump_rises_then_lands() {
        let f = Fixture::flat();
        let mut p = Player::new(Vec3::new(0.5, 1.0, 0.5));
        p.on_ground = true;
        let mut input = PlayerInput { jump: true, ..Default::default() };
        // hold jump for a few frames then release
        run_frames(&mut p, &input, &f.world, 0.1);
        assert!(p.position.y > 1.2, "should be rising, y={}", p.position.y);
        input.jump = false;
        run_frames(&mut p, &input, &f.world, 3.0);
        assert!(p.on_ground);
        assert!((p.position.y - 1.0).abs() < 0.01);
    }

    #[test]
    fn ceiling_blocks_rise() {
        let mut f = Fixture::flat();
        f.ceiling(3); // only 1 block of headroom above standing height... floor top=1, ceiling bottom=3 => 2 blocks space
        let mut p = Player::new(Vec3::new(0.5, 1.0, 0.5));
        p.on_ground = true;
        let input = PlayerInput { jump: true, ..Default::default() };
        run_frames(&mut p, &input, &f.world, 2.0);
        assert!(p.position.y + PLAYER_HEIGHT <= 3.0 + 1e-3,
            "head should not pass ceiling, y={}", p.position.y);
    }

    #[test]
    fn fly_ignores_gravity() {
        let f = Fixture::flat();
        let mut p = Player::new(Vec3::new(0.5, 1.0, 0.5));
        p.flying = true;
        let input = PlayerInput { fly_up: true, ..Default::default() };
        run_frames(&mut p, &input, &f.world, 1.0);
        assert!(p.position.y > 5.0, "should fly up, y={}", p.position.y);
    }

    #[test]
    fn no_tunneling_on_long_fall() {
        let mut world = World::new();
        world.ensure_chunk(0, 0);
        for lx in 0..16 {
            for lz in 0..16 {
                world.set_block(lx, 0, lz, BlockState::STONE);
            }
        }
        let mut p = Player::new(Vec3::new(8.0, 250.0, 8.0));
        run_frames(&mut p, &no_input(), &world, 30.0);
        assert!(p.on_ground, "must land despite big dt");
        assert!((p.position.y - 1.0).abs() < 0.01);
    }

    #[test]
    fn look_dir_points_forward() {
        let mut p = Player::new(Vec3::ZERO);
        assert!((p.look_dir().x - 0.0).abs() < 1e-5 && (p.look_dir().z + 1.0).abs() < 1e-5);
        p.apply_look(std::f32::consts::FRAC_PI_2, 0.0);
        assert!((p.look_dir().x - 1.0).abs() < 1e-5, "yaw +90deg looks +X, got {:?}", p.look_dir());
        p.apply_look(0.0, std::f32::consts::FRAC_PI_2);
        // pitch clamps at 89 degrees, so look nearly straight up
        assert!(p.look_dir().y > 0.99, "pitch +90 clamps to near-up, got {:?}", p.look_dir());
    }
}
