//! The title-screen preview world (ui-world-craft Section B): a seed
//! derived from the game version, and the scenic camera path that orbits
//! it. Pure math — no rendering — so the client and the vistest harness
//! both drive the exact same orbit.

/// Derive the preview world seed from a "MAJOR.MINOR.PATCH" version string.
///
/// The mix is deliberately non-trivial so adjacent versions produce
/// visually different worlds: v0.4.1 and v0.4.2 must not look like
/// siblings. Stable across machines — nothing but the string feeds it.
pub fn preview_world_seed_from_version(version: &str) -> u64 {
    let parts: Vec<u64> = version
        .split('.')
        .map(|s| s.parse::<u64>().unwrap_or(0))
        .collect();
    let major = parts.first().copied().unwrap_or(0);
    let minor = parts.get(1).copied().unwrap_or(0);
    let patch = parts.get(2).copied().unwrap_or(0);
    major.wrapping_mul(1_000_000)
        .wrapping_add(minor.wrapping_mul(1_000))
        .wrapping_add(patch)
        .wrapping_mul(0x9e3779b97f4a7c15) // Fibonacci hashing constant
}

/// The compiled game's preview seed (`CARGO_PKG_VERSION` at build time).
pub fn version_preview_seed() -> u64 {
    preview_world_seed_from_version(env!("CARGO_PKG_VERSION"))
}

// ------------------------------------------------------------------
// Orbit parameters — tunable in one place, by name.

/// One full scenic orbit every 90 seconds: slow enough to watch, fast
/// enough to notice.
pub const PREVIEW_ORBIT_PERIOD_SECS: f64 = 90.0;
/// Elliptical, not circular — a perfect circle reads as mechanical.
pub const PREVIEW_ORBIT_X_RADIUS: f32 = 80.0;
pub const PREVIEW_ORBIT_Z_RADIUS: f32 = 60.0; // 1.33:1 vs X
/// The camera looks at a point offset from the world center, so the
/// framing is never perfectly symmetric.
pub const PREVIEW_ORBIT_LOOK_OFFSET_X: f32 = 20.0;
/// Eye altitude above the spawn column, before the oscillation.
pub const PREVIEW_BASE_ALTITUDE: f32 = 40.0;
/// The eye rises and falls ±8 blocks on a 57.3s period — prime-ish, so it
/// never syncs with the 90s orbit and the path never exactly repeats.
pub const PREVIEW_ALT_OSCILLATION_AMPLITUDE: f32 = 8.0;
pub const PREVIEW_ALT_OSCILLATION_PERIOD_SECS: f64 = 57.3;

/// Where the preview camera sits and looks at time `t` (seconds).
/// `center` is the spawn point of the preview world.
pub fn preview_camera(t: f64, center: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let angle = t * std::f64::consts::TAU / PREVIEW_ORBIT_PERIOD_SECS;
    let (sin, cos) = angle.sin_cos();
    let eye_x = center[0] + PREVIEW_ORBIT_X_RADIUS * cos as f32;
    let eye_z = center[2] + PREVIEW_ORBIT_Z_RADIUS * sin as f32;
    let alt_angle = t * std::f64::consts::TAU / PREVIEW_ALT_OSCILLATION_PERIOD_SECS;
    let eye_y = center[1] + PREVIEW_BASE_ALTITUDE
        + PREVIEW_ALT_OSCILLATION_AMPLITUDE * alt_angle.sin() as f32;
    let look = [center[0] + PREVIEW_ORBIT_LOOK_OFFSET_X, center[1] + 2.0, center[2]];
    ([eye_x, eye_y, eye_z], look)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_is_stable_and_version_sensitive() {
        let a = preview_world_seed_from_version("0.4.2");
        assert_eq!(a, preview_world_seed_from_version("0.4.2"), "same version, same seed");
        assert_ne!(a, preview_world_seed_from_version("0.4.1"), "a patch bump changes the world");
        assert_ne!(a, preview_world_seed_from_version("0.3.2"), "a minor bump changes the world");
        assert_ne!(preview_world_seed_from_version("0.4.2"), 0);
        // partial / garbage strings still produce a usable seed
        assert_eq!(preview_world_seed_from_version("1.0"), preview_world_seed_from_version("1.0.0"));
    }

    #[test]
    fn orbit_is_elliptical_oscillating_and_offset() {
        let center = [0.5, 70.0, 0.5];
        let (e0, look) = preview_camera(0.0, center);
        // the look target sits off the world center
        assert!((look[0] - (center[0] + PREVIEW_ORBIT_LOOK_OFFSET_X)).abs() < 0.01);
        assert!((look[2] - center[2]).abs() < 0.01);
        // over one orbit, the X and Z swings differ (never a circle)
        let mut max_dx = 0.0f32;
        let mut max_dz = 0.0f32;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for step in 0..1800 {
            let (eye, _) = preview_camera(step as f64 * 0.1, center);
            max_dx = max_dx.max((eye[0] - center[0]).abs());
            max_dz = max_dz.max((eye[2] - center[2]).abs());
            min_y = min_y.min(eye[1]);
            max_y = max_y.max(eye[1]);
        }
        assert!((max_dx - PREVIEW_ORBIT_X_RADIUS).abs() < 0.05);
        assert!((max_dz - PREVIEW_ORBIT_Z_RADIUS).abs() < 0.05);
        assert!((max_dx - max_dz).abs() > 1.0, "radii must differ (elliptical, not circular)");
        // altitude oscillates around the base by ~±8 within one orbit
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for step in 0..1800 {
            let (eye, _) = preview_camera(step as f64 * 0.1, center);
            min_y = min_y.min(eye[1]);
            max_y = max_y.max(eye[1]);
        }
        let span = max_y - min_y;
        assert!(span > 2.0 * PREVIEW_ALT_OSCILLATION_AMPLITUDE * 0.9, "altitude must swing ±8, span={}", span);
        assert!(span < 2.0 * PREVIEW_ALT_OSCILLATION_AMPLITUDE * 1.05 + 0.5);
        // 30 seconds later the framing is visibly different
        let (e30, _) = preview_camera(30.0, center);
        let dist = ((e30[0] - e0[0]).powi(2) + (e30[2] - e0[2]).powi(2)).sqrt();
        assert!(dist > 20.0, "half a period apart the camera must have moved, got {}", dist);
    }
}
