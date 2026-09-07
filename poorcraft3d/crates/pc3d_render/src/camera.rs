//! P3D camera and matrix math.
//!
//! World convention (binding, per 02-RENDERER-ARCHITECTURE.md): right-handed
//! coordinates, meters as the unit, +X east, +Y up, +Z south. Yaw 0 looks
//! north (-Z); positive yaw turns counterclockwise seen from above (toward
//! west); pitch is positive looking up. Everything here is in P3D world
//! coordinates — the renderer never invents its own space.

/// A full camera state: where the eye is and where it looks.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraPose {
    /// Eye position in world meters.
    pub position: [f32; 3],
    /// Rotation about +Y, radians. 0 looks -Z (north).
    pub yaw: f32,
    /// Rotation about the camera right axis, radians. Positive looks up.
    pub pitch: f32,
}

impl CameraPose {
    pub const fn new(position: [f32; 3], yaw: f32, pitch: f32) -> Self {
        Self {
            position,
            yaw,
            pitch,
        }
    }
}

pub const DEFAULT_FOV_Y_RAD: f32 = 70.0_f32.to_radians();
pub const DEFAULT_NEAR: f32 = 0.1;
pub const DEFAULT_FAR: f32 = 600.0;

/// Forward view direction for yaw/pitch (unit length).
pub fn fwd_of(yaw: f32, pitch: f32) -> [f32; 3] {
    let cy = yaw.cos();
    let sy = yaw.sin();
    let cp = pitch.cos();
    let sp = pitch.sin();
    // yaw 0/pitch 0 -> (0,0,-1); positive yaw -> west; positive pitch -> up.
    [-sy * cp, sp, -cy * cp]
}

/// Right vector for a forward direction (world-up constrained, unit length).
pub fn right_of(fwd: [f32; 3]) -> [f32; 3] {
    // right = normalize(cross(fwd, world_up)); world_up = (0,1,0)
    let r = [
        fwd[1] * 0.0 - fwd[2] * 1.0,
        fwd[2] * 0.0 - fwd[0] * 0.0,
        fwd[0] * 1.0 - fwd[1] * 0.0,
    ];
    // cross((x,y,z),(0,1,0)) = (y*0 - z*1, z*0 - x*0, x*1 - y*0) = (-z, 0, x)
    let n = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
    if n <= 0.0 {
        [1.0, 0.0, 0.0]
    } else {
        [r[0] / n, r[1] / n, r[2] / n]
    }
}

/// True up vector for a forward direction (unit length).
pub fn up_of(fwd: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    // up = cross(right, fwd)
    let u = [
        right[1] * fwd[2] - right[2] * fwd[1],
        right[2] * fwd[0] - right[0] * fwd[2],
        right[0] * fwd[1] - right[1] * fwd[0],
    ];
    let n = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
    [u[0] / n, u[1] / n, u[2] / n]
}

/// Column-major 4x4 matrix (as WGSL mat4x4f expects from a flat buffer).
pub type Mat4 = [f32; 16];

/// Right-handed view matrix looking along `fwd` from `eye`.
pub fn view_matrix(eye: [f32; 3], fwd: [f32; 3], right: [f32; 3], up: [f32; 3]) -> Mat4 {
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    // Rows are the basis; stored column-major.
    [
        right[0], up[0], -fwd[0], 0.0, //
        right[1], up[1], -fwd[1], 0.0, //
        right[2], up[2], -fwd[2], 0.0, //
        -dot(right, eye), -dot(up, eye), dot(fwd, eye), 1.0,
    ]
}

/// Right-handed perspective projection with NDC depth in [0, 1] (wgpu).
pub fn perspective(fov_y_rad: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fov_y_rad / 2.0).tan();
    [
        f / aspect, 0.0, 0.0, 0.0, //
        0.0, f, 0.0, 0.0, //
        0.0, 0.0, far / (near - far), -1.0, //
        0.0, 0.0, far * near / (near - far), 0.0,
    ]
}

/// Matrix product a * b (both column-major).
pub fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut out = [0.0f32; 16];
    for col in 0..4 {
        for row in 0..4 {
            let mut s = 0.0;
            for k in 0..4 {
                s += a[k * 4 + row] * b[col * 4 + k];
            }
            out[col * 4 + row] = s;
        }
    }
    out
}

/// The full camera: pose + projection parameters.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub pose: CameraPose,
    pub fov_y_rad: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn new(pose: CameraPose) -> Self {
        Self {
            pose,
            fov_y_rad: DEFAULT_FOV_Y_RAD,
            near: DEFAULT_NEAR,
            far: DEFAULT_FAR,
        }
    }

    pub fn fwd(&self) -> [f32; 3] {
        fwd_of(self.pose.yaw, self.pose.pitch)
    }

    /// tan(fov_y / 2) — the sky shader's ray reconstruction uses this.
    pub fn tan_half_fov(&self) -> f32 {
        (self.fov_y_rad / 2.0).tan()
    }

    pub fn right(&self) -> [f32; 3] {
        right_of(self.fwd())
    }

    pub fn up(&self) -> [f32; 3] {
        up_of(self.fwd(), self.right())
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let v = view_matrix(self.pose.position, self.fwd(), self.right(), self.up());
        let p = perspective(self.fov_y_rad, aspect, self.near, self.far);
        mul(&p, &v)
    }

    /// First-person ground movement: forward/back along yaw (pitch ignored,
    /// standard FPS walk), strafe along right, plus explicit vertical.
    /// Returns the world-space displacement for `dt` seconds at `speed`.
    pub fn walk_step(&self, fwd_amt: f32, strafe_amt: f32, vert_amt: f32, dt: f32, speed: f32) -> [f32; 3] {
        let yaw = self.pose.yaw;
        let hf = [-yaw.sin(), 0.0, -yaw.cos()];
        let hr = [yaw.cos(), 0.0, -yaw.sin()];
        let mut d = [
            (hf[0] * fwd_amt + hr[0] * strafe_amt) * speed * dt,
            vert_amt * speed * dt,
            (hf[2] * fwd_amt + hr[2] * strafe_amt) * speed * dt,
        ];
        // Normalize the horizontal plan so diagonals aren't sqrt(2) faster.
        let planar = (d[0] * d[0] + d[2] * d[2]).sqrt();
        if planar > speed * dt && planar > 0.0 {
            let k = speed * dt / planar;
            d[0] *= k;
            d[2] *= k;
        }
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-4;

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < EPS)
    }

    #[test]
    fn basis_vectors_follow_the_documented_convention() {
        // yaw 0, pitch 0: looking north (-Z).
        assert!(close(fwd_of(0.0, 0.0), [0.0, 0.0, -1.0]));
        // Positive yaw turns toward west (-X); right hand then points north.
        assert!(close(fwd_of(std::f32::consts::FRAC_PI_2, 0.0), [-1.0, 0.0, 0.0]));
        assert!(close(right_of(fwd_of(std::f32::consts::FRAC_PI_2, 0.0)), [0.0, 0.0, -1.0]));
        // Facing north, right hand points east (+X).
        assert!(close(right_of(fwd_of(0.0, 0.0)), [1.0, 0.0, 0.0]));
        // Positive pitch looks up.
        assert!(fwd_of(0.0, std::f32::consts::FRAC_PI_2)[1] > 0.99);
        assert!((up_of(fwd_of(0.0, 0.0), right_of(fwd_of(0.0, 0.0)))[1] - 1.0).abs() < EPS);
    }

    #[test]
    fn projection_maps_near_to_zero_and_far_to_one() {
        let p = perspective(DEFAULT_FOV_Y_RAD, 1.0, 0.1, 100.0);
        let z = |view_z: f32| {
            // view-space point (0,0,z), z negative in front of the camera.
            let clip_z = p[2 * 4 + 2] * view_z + p[3 * 4 + 2];
            let w = p[2 * 4 + 3] * view_z;
            clip_z / w
        };
        assert!((z(-0.1) - 0.0).abs() < EPS);
        assert!((z(-100.0) - 1.0).abs() < EPS);
        assert!(z(-1.0) > 0.0 && z(-1.0) < 1.0);
    }

    #[test]
    fn view_proj_projects_a_known_world_point() {
        // Camera at (0,1.7,2) looking north; a point 5 m ahead, 0.7 m below
        // the eye must land slightly below screen center.
        let cam = Camera::new(CameraPose::new([0.0, 1.7, 2.0], 0.0, 0.0));
        let vp = cam.view_proj(1.0);
        let project = |p: [f32; 3]| {
            let x = vp[0] * p[0] + vp[4] * p[1] + vp[8] * p[2] + vp[12];
            let y = vp[1] * p[0] + vp[5] * p[1] + vp[9] * p[2] + vp[13];
            let w = vp[3] * p[0] + vp[7] * p[1] + vp[11] * p[2] + vp[15];
            (x / w, y / w)
        };
        let (nx, ny) = project([0.0, 1.0, -3.0]);
        assert!(nx.abs() < EPS, "on-axis point must be centered, got {nx}");
        assert!(ny < -0.1 && ny > -0.3, "below-eye point must sit below center, got {ny}");

        // A point to the east of the view axis lands on the right half.
        let (nx, _) = project([1.0, 1.0, -3.0]);
        assert!(nx > 0.1);
    }

    #[test]
    fn walk_steps_follow_yaw_and_normalize_diagonals() {
        let cam = Camera::new(CameraPose::new([0.0, 1.7, 2.0], 0.0, 0.0));
        // Walking forward at yaw 0 moves north (-Z).
        let d = cam.walk_step(1.0, 0.0, 0.0, 1.0, 4.0);
        assert!(close(d, [0.0, 0.0, -4.0]));
        // Strafing right at yaw 0 moves east (+X).
        let d = cam.walk_step(0.0, 1.0, 0.0, 1.0, 4.0);
        assert!(close(d, [4.0, 0.0, 0.0]));
        // Vertical movement is explicit, not yaw-dependent.
        let d = cam.walk_step(0.0, 0.0, 1.0, 0.5, 4.0);
        assert!(close(d, [0.0, 2.0, 0.0]));
        // Forward + strafe covers no more ground than either alone.
        let d = cam.walk_step(1.0, 1.0, 0.0, 1.0, 4.0);
        let planar = (d[0] * d[0] + d[2] * d[2]).sqrt();
        assert!((planar - 4.0).abs() < 1e-3);
    }
}
