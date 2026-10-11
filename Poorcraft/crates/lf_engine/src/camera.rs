use glam::{Mat4, Vec3};

/// First-person style camera used by both the windowed app and headless shots.
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn new(eye: Vec3, target: Vec3) -> Self {
        Self {
            eye,
            target,
            up: Vec3::Y,
            aspect: 4.0 / 3.0,
            fovy: 45f32.to_radians(),
            near: 0.1,
            far: 1000.0,
        }
    }

    pub fn set_aspect(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.aspect = width as f32 / height as f32;
        }
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.eye, self.target, self.up);
        let proj = Mat4::perspective_rh(self.fovy, self.aspect, self.near, self.far);
        proj * view
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Goal Section 5 / Step 7: verify the FOV-to-projection math against
    /// hand-computed reference values at two FOV settings — the same class
    /// of bug the P25 audit found in the path tracer (double to_radians)
    /// must not exist on the raster path. For a camera at the origin
    /// looking down -Z the view matrix is identity, so the view-projection
    /// IS the projection matrix: m00 = (1/tan(fovy/2))/aspect, m11 =
    /// 1/tan(fovy/2), m22 = far/(near-far), m23 = near*far/(near-far).
    #[test]
    fn projection_matches_reference_values_at_two_fovs() {
        for (fov_deg, want_m00, want_m11) in [
            // fov 90: 1/tan(45deg) = 1.0; aspect 4/3 -> m00 = 0.75
            (90.0f32, 0.75f32, 1.0f32),
            // fov 60: 1/tan(30deg) = 1.7320508; aspect 4/3 -> m00 = 1.2990381
            (60.0, 1.299_038_1, 1.732_050_8),
        ] {
            let mut cam = Camera::new(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
            cam.fovy = fov_deg.to_radians();
            let m = cam.build_view_projection_matrix().to_cols_array();
            assert!((m[0] - want_m00).abs() < 1e-4,
                "fov {fov_deg}: m00 {}, want {want_m00}", m[0]);
            assert!((m[5] - want_m11).abs() < 1e-4,
                "fov {fov_deg}: m11 {}, want {want_m11}", m[5]);
            // near 0.1 / far 1000: m22 = 1000/(0.1-1000) = -1.0001000
            assert!((m[10] - -1.000_100_05).abs() < 1e-4, "m22 {}", m[10]);
            // m23 = near*far/(near-far) = 100/(-999.9) = -0.10001
            assert!((m[14] - -0.100_010).abs() < 1e-4, "m23 {}", m[14]);
            // a FOV in degrees must NOT be converted twice (the P25 bug):
            // at fov_deg=90 a double conversion would collapse m00 to ~0.0037
        }

        // sanity: changing FOV actually changes the projection
        let mut wide = Camera::new(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        wide.fovy = 100f32.to_radians();
        let mut narrow = Camera::new(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        narrow.fovy = 30f32.to_radians();
        assert!(wide.build_view_projection_matrix().to_cols_array()[5]
            < narrow.build_view_projection_matrix().to_cols_array()[5],
            "wider FOV = smaller m11");
    }
}
