//! P36 dragons: flight AI (circle / swoop / perch), fire-breath gating,
//! and the mount brain. Pure functions so tests and the client share
//! one path. Rendering is multi-part on the client (body/head/wings/
//! tail with sine offsets).

use serde::{Deserialize, Serialize};

/// Flight radius + band the dragon keeps around its roost.
pub const CIRCLE_RADIUS: f32 = 14.0;
/// Altitude above the roost platform while circling.
pub const CIRCLE_ALT: f32 = 8.0;
/// How close the player must be (horizontally) to provoke a swoop.
pub const AGGRO_RANGE: f32 = 20.0;
/// Seconds a swoop lasts before climbing back.
pub const SWOOP_TIME: f32 = 4.0;
/// Seconds between fire breaths while perched.
pub const BREATH_PERIOD: f32 = 3.0;
/// Breath reach (blocks).
pub const BREATH_RANGE: f32 = 7.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Phase {
    Circling,
    Swooping,
    Perched,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DragonBrain {
    pub phase: Phase,
    /// Sweep angle around the roost center (radians).
    pub angle: f32,
    pub breath_cooldown: f32,
    pub swoop_t: f32,
}

impl Default for DragonBrain {
    fn default() -> Self {
        Self { phase: Phase::Circling, angle: 0.0, breath_cooldown: 0.0, swoop_t: 0.0 }
    }
}

impl DragonBrain {
    /// One AI tick. `center` = the roost platform position, `target` =
    /// the player (or None). Returns (new position, breathing_this_tick,
    /// breath_origin_direction_dummy). The caller owns collisions.
    pub fn tick(
        &mut self,
        dt: f32,
        center: glam::Vec3,
        target: Option<glam::Vec3>,
        perched_home: glam::Vec3,
    ) -> (glam::Vec3, bool) {
        let speed = 0.9f32; // rad/s around the circle
        match self.phase {
            Phase::Circling => {
                self.angle += speed * dt;
                let pos = glam::Vec3::new(
                    center.x + self.angle.cos() * CIRCLE_RADIUS,
                    center.y + CIRCLE_ALT + self.angle.sin() * 1.5,
                    center.z + self.angle.sin() * CIRCLE_RADIUS,
                );
                // provoke: player inside the ring's horizontal reach
                if let Some(t) = target {
                    let flat = ((t.x - center.x).powi(2) + (t.z - center.z).powi(2)).sqrt();
                    if flat < AGGRO_RANGE {
                        self.phase = Phase::Swooping;
                        self.swoop_t = 0.0;
                    }
                }
                (pos, false)
            }
            Phase::Swooping => {
                self.swoop_t += dt;
                let pos = match target {
                    Some(t) => {
                        // dive toward the target, easing back up after the
                        // swoop window
                        let k = (self.swoop_t / SWOOP_TIME).min(1.0);
                        let arc = (k * std::f32::consts::PI).sin();
                        glam::Vec3::new(
                            t.x + (center.x - t.x) * 0.2,
                            t.y + 3.0 + (1.0 - arc) * 4.0,
                            t.z + (center.z - t.z) * 0.2,
                        )
                    }
                    None => perched_home,
                };
                if self.swoop_t >= SWOOP_TIME {
                    self.phase = Phase::Circling;
                }
                (pos, false)
            }
            Phase::Perched => {
                self.breath_cooldown -= dt;
                let mut breathing = false;
                if let Some(t) = target {
                    let d = (t - perched_home).length();
                    if d < BREATH_RANGE && self.breath_cooldown <= 0.0 {
                        breathing = true;
                        self.breath_cooldown = BREATH_PERIOD;
                    }
                    // a threatened perch launches back into the ring
                    if d < AGGRO_RANGE * 0.5 {
                        self.phase = Phase::Circling;
                    }
                }
                (perched_home, breathing)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn circles_the_roost_and_keeps_the_ring() {
        let mut brain = DragonBrain::default();
        let center = Vec3::new(0.0, 100.0, 0.0);
        let mut last = Vec3::ZERO;
        for i in 0..200 {
            let (pos, _) = brain.tick(0.05, center, None, center);
            if i > 0 {
                let flat = ((pos.x - center.x).powi(2) + (pos.z - center.z).powi(2)).sqrt();
                assert!((flat - CIRCLE_RADIUS).abs() < 1.5, "stays on the ring, got {}", flat);
                assert!(pos.y > center.y + CIRCLE_ALT - 2.0);
                assert!((pos - last).length() > 0.0, "keeps moving");
            }
            last = pos;
        }
    }

    #[test]
    fn provocation_swoops_then_returns() {
        let mut brain = DragonBrain::default();
        let center = Vec3::new(0.0, 100.0, 0.0);
        let player = Vec3::new(3.0, 100.0, 3.0);
        let (mut pos, _) = brain.tick(0.05, center, Some(player), center);
        assert_eq!(brain.phase, Phase::Swooping, "close player provokes the dive");
        // the threat stays: the dragon re-dives while the player lingers
        for _ in 0..60 {
            let (p, _) = brain.tick(0.05, center, Some(player), center);
            pos = p;
        }
        assert!(pos.y < 108.0, "still diving at the lingering player");
        // the player retreats: the window closes and the ring resumes
        let far = Vec3::new(60.0, 100.0, 60.0);
        for _ in 0..120 {
            let (p, _) = brain.tick(0.05, center, Some(far), center);
            pos = p;
        }
        assert_eq!(brain.phase, Phase::Circling, "the swoop window closes");
        assert!(pos.y > 105.0, "climbing back out of the dive");
    }

    #[test]
    fn perched_breath_is_periodic_and_finite() {
        let mut brain = DragonBrain { phase: Phase::Perched, ..Default::default() };
        let home = Vec3::new(0.0, 100.0, 0.0);
        let player = Vec3::new(4.0, 100.0, 0.0);
        let (_, b1) = brain.tick(0.05, home, Some(player), home);
        assert!(b1, "first breath fires in range");
        let (_, b2) = brain.tick(0.05, home, Some(player), home);
        assert!(!b2, "breath is gated by the period");
        for _ in 0..80 {
            brain.tick(0.05, home, Some(player), home);
        }
        // a very close player eventually launches the dragon off the perch
        let close = Vec3::new(1.0, 100.0, 0.0);
        let mut launched = false;
        for _ in 0..50 {
            let (_, _) = brain.tick(0.05, home, Some(close), home);
            if brain.phase != Phase::Perched {
                launched = true;
                break;
            }
        }
        assert!(launched, "a threatened perch launches back into the ring");
    }
}

/// Multi-part body layout (P36): every part as (offset from the body
/// center, half-size) — the client renderer and the vistest proofs share
/// this so the proof shows the real assembly. `t` drives the sine
/// animation (wing flap + tail sway); facing yaw rotates the layout.
pub fn dragon_parts(t: f32, yaw: f32) -> Vec<(glam::Vec3, f32)> {
    let flap = (t * 2.4).sin();
    let sway = (t * 1.1).sin();
    let (sy, cy) = yaw.sin_cos();
    let fwd = glam::Vec3::new(sy, 0.0, -cy);
    let right = glam::Vec3::new(cy, 0.0, sy);
    let mut parts = Vec::new();
    // body
    parts.push((glam::Vec3::ZERO, 0.9));
    // head: forward + up, with a small bob
    parts.push((fwd * 1.2 + glam::Vec3::new(0.0, 0.35 + flap * 0.08, 0.0), 0.45));
    // wings: out to the sides, flapping
    parts.push((right * 1.6 + glam::Vec3::new(0.0, 0.2 + flap * 0.5, 0.0), 0.7));
    parts.push((-right * 1.6 + glam::Vec3::new(0.0, 0.2 + flap * 0.5, 0.0), 0.7));
    // tail: three shrinking segments behind, swaying
    for (i, s) in [0.55f32, 0.42, 0.3].iter().enumerate() {
        let sway_off = right * sway * 0.25 * (i + 1) as f32;
        parts.push((-fwd * (1.0 + i as f32 * 0.7) + sway_off, *s));
    }
    parts
}

#[cfg(test)]
mod part_tests {
    use super::*;

    /// The layout animates: wings rise and fall, tail sways — and the
    /// head always stays in front of the body.
    #[test]
    fn dragon_parts_flap_and_face_forward() {
        let up = dragon_parts(0.0, 0.0);
        let half = dragon_parts(std::f32::consts::FRAC_PI_2 / 2.4, 0.0);
        let wing_up = up[2].0.y;
        let wing_mid = half[2].0.y;
        assert!((wing_up - wing_mid).abs() > 0.1, "the wings actually flap");
        assert!(up[1].0.z < 0.0, "the head leads at yaw 0 (-Z forward)");
        // tail segments trail behind
        assert!(up[5].0.z > 0.5 && up[6].0.z > up[5].0.z, "the tail trails and extends");
        // yaw rotates the whole assembly
        let turned = dragon_parts(0.0, std::f32::consts::FRAC_PI_2);
        assert!(turned[1].0.x > 0.5, "at yaw 90 the head faces +X");
    }
}
