//! Sky and weather visuals: drifting cloud layer, sun/moon/stars, weather
//! particles (rain/snow), and underwater tint parameters.

use glam::Vec3;

use crate::scene::GpuVertex;

pub const CLOUD_LEVEL: f32 = 160.0;

/// Build the translucent cloud quad layer around `center`, drifting with
/// `time`. Cells of 12 blocks sampled from a cheap value-noise.
pub fn cloud_mesh(center: Vec3, time: f32) -> (Vec<GpuVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let cell = 12.0f32;
    let drift = time * 1.5; // blocks/sec of wind
    let base_x = (center.x / cell).floor() * cell - drift;
    let base_z = (center.z / cell).floor() * cell;
    let half_extent = 24; // cells each way
    let tex = lf_assets::texture_index_for_block(lf_voxel::registry::block::SNOW);
    for cx in -half_extent..=half_extent {
        for cz in -half_extent..=half_extent {
            let wx = base_x + cx as f32 * cell;
            let wz = base_z + cz as f32 * cell;
            let n = value_noise(wx * 0.012 + 3.7, wz * 0.012 - 1.3);
            if n < 0.58 {
                continue;
            }
            let base = vertices.len() as u32;
            let (x0, z0) = (wx, wz);
            let (x1, z1) = (wx + cell, wz + cell);
            let y = CLOUD_LEVEL + (n - 0.58) * 30.0;
            let corners = [[x0, y, z1], [x0, y, z0], [x1, y, z0], [x1, y, z1]];
            let uvs = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
            for (c, uv) in corners.iter().zip(uvs.iter()) {
                vertices.push(GpuVertex {
                    position: *c,
                    normal: [0.0, -1.0, 0.0],
                    tex_coord: *uv,
                    tex_index: tex,
                    ao: 0.9,
                    light: 0xF0,
                    sway: 0.0,
                });
            }
            indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }
    (vertices, indices)
}

/// Sun and moon quads (plus stars at night) along the celestial circle.
/// `time_frac` is the day fraction [0..1).
pub fn sky_bodies(eye: Vec3, time_frac: f32) -> (Vec<GpuVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let angle = time_frac * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
    let sun_dir = Vec3::new(angle.cos(), angle.sin(), 0.25).normalize();
    let dist = 420.0;

    let mut push_billboard = |dir: Vec3, size: f32, tex: u32, brightness: u32| {
        let center = eye + dir * dist;
        // build a camera-independent up/right basis around the direction
        let up = if dir.y.abs() > 0.95 { Vec3::X } else { Vec3::Y };
        let right = dir.cross(up).normalize() * size;
        let real_up = right.cross(dir).normalize() * size;
        let base = vertices.len() as u32;
        let corners = [center - right - real_up, center - right + real_up, center + right + real_up, center + right - real_up];
        let uvs = [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
        for (c, uv) in corners.iter().zip(uvs.iter()) {
            vertices.push(GpuVertex {
                position: [c.x, c.y, c.z],
                normal: [-dir.x, -dir.y, -dir.z],
                tex_coord: *uv,
                tex_index: tex,
                ao: 1.0,
                light: brightness,
                    sway: 0.0,
            });
        }
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    };

    let moon_tex = lf_assets::texture_index_for_block(lf_voxel::registry::block::SNOW);
    push_billboard(sun_dir, 30.0, moon_tex, 0xFF); // full-bright white sun
    push_billboard(-sun_dir, 18.0, moon_tex, 0xF0); // moon

    // stars fade in at night (time outside [0.25, 0.75] is dark)
    let darkness = (time_frac - 0.5).abs().min(0.25);
    let star_alpha = ((0.25 - darkness) / 0.25).clamp(0.0, 1.0);
    if star_alpha > 0.05 {
        let mut rng = 0x853c49e6748fea9bu64;
        let mut next = || {
            rng ^= rng >> 12;
            rng ^= rng << 25;
            rng ^= rng >> 27;
            rng.wrapping_mul(0x2545F4914F6CDD1D)
        };
        for _ in 0..90 {
            // random direction on the upper hemisphere
            let a = (next() % 3600) as f32 / 3600.0 * std::f32::consts::TAU;
            let y = 0.15 + (next() % 1000) as f32 / 1000.0 * 0.8;
            let r = (1.0 - y * y).sqrt();
            let dir = Vec3::new(r * a.cos(), y, r * a.sin());
            let _ = star_alpha;
            push_billboard(dir, 1.4, moon_tex, 0xD0);
        }
    }
    (vertices, indices)
}

/// Falling weather particles around the player. `snow` switches to slow,
/// swaying flakes. Returns vertices in the transparent pass.
pub fn weather_particles(center: Vec3, time: f32, snow: bool) -> (Vec<GpuVertex>, Vec<u32>) {
    let count = if snow { 220 } else { 380 };
    let mut rng = 0x9e3779b97f4a7c15u64;
    let mut next = || {
        rng ^= rng >> 30;
        rng ^= rng << 27;
        rng ^= rng >> 15;
        rng.wrapping_mul(0x8cb92ba72f3d8dd7)
    };
    let rain_tex = lf_assets::texture_index_for_block(lf_voxel::registry::block::WATER);
    let snow_tex = lf_assets::texture_index_for_block(lf_voxel::registry::block::SNOW);
    let tex = if snow { snow_tex } else { rain_tex };
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let speed = if snow { 4.0 } else { 22.0 };
    let sway = if snow { 1.5 } else { 0.0 };
    for i in 0..count {
        let sx = ((next() % 4000) as f32 / 4000.0 - 0.5) * 40.0;
        let sz = ((next() % 4000) as f32 / 4000.0 - 0.5) * 40.0;
        let span = 26.0;
        let y = center.y + 18.0 - ((time * speed + i as f32 * 0.37) % span);
        let x = center.x + sx + if sway > 0.0 { (time * 2.0 + i as f32).sin() * sway } else { 0.0 };
        let z = center.z + sz;
        let size = if snow { 0.12 } else { 0.06 };
        let base = vertices.len() as u32;
        let corners = [
            [x - size, y, z - size],
            [x - size, y + size * 3.0, z - size],
            [x + size, y + size * 3.0, z + size],
            [x + size, y, z + size],
        ];
        let uvs = [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
        for (c, uv) in corners.iter().zip(uvs.iter()) {
            vertices.push(GpuVertex {
                position: *c,
                normal: [0.0, 1.0, 0.0],
                tex_coord: *uv,
                tex_index: tex,
                ao: 1.0,
                light: 0xE0,
                    sway: 0.0,
            });
        }
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    (vertices, indices)
}

/// Cheap deterministic 2D value noise for cloud coverage.
fn value_noise(x: f32, y: f32) -> f32 {
    let xi = x.floor();
    let yi = y.floor();
    let xf = x - xi;
    let yf = y - yi;
    let hash = |ix: i32, iy: i32| -> f32 {
        let h = (ix as i64 * 374761393 + iy as i64 * 668265263) as u64;
        let h = h ^ (h >> 13);
        (h & 0xFFFF) as f32 / 65535.0
    };
    let (sx, sy) = (xf * xf * (3.0 - 2.0 * xf), yf * yf * (3.0 - 2.0 * yf));
    let a = hash(xi as i32, yi as i32);
    let b = hash(xi as i32 + 1, yi as i32);
    let c = hash(xi as i32, yi as i32 + 1);
    let d = hash(xi as i32 + 1, yi as i32 + 1);
    a + (b - a) * sx + (c - a) * sy + (a - b - c + d) * sx * sy
}

/// Underwater fog parameters when the eye is submerged.
pub fn underwater_env(eye_block: u32) -> Option<([f32; 3], f32)> {
    use lf_voxel::registry::block;
    if eye_block == block::WATER {
        Some(([0.05, 0.20, 0.35], 14.0))
    } else {
        None
    }
}
