//! The campfire's drawn body (N11): a stone ring, crossed logs, and the
//! animated flame boxes — pure box instances through the crowd's box
//! pipeline (no new GLB asset: the fire is CODE, deterministic per tick
//! and phase, cheap enough for the Deck tiers).
//!
//! THE GROUND LAW: the caller answers the fire's base Y through the SAME
//! authoritative `ground_y_at` the body stands on — the fire stands on
//! the world's soil, never floating, never buried.

use crate::npcs::{BoxInstance, PartColor};
use std::collections::BTreeMap;

/// The stone ring's radius (m).
pub const RING_RADIUS_M: f32 = 0.7;
/// Flame column height at full burn (m).
pub const FLAME_HEIGHT_M: f32 = 0.55;

/// One frame of campfire instances. `t` animates the flame (flicker
/// heights + sway); `fuel_frac` [0,1] scales the flame down as the pile
/// burns low; embers-only draws the glow bed without tall flames.
pub fn campfire_instances(
    xz: [f32; 2],
    base_y: f32,
    burning: bool,
    embers: bool,
    fuel_frac: f32,
    t: f32,
) -> BTreeMap<PartColor, Vec<BoxInstance>> {
    let mut out: BTreeMap<PartColor, Vec<BoxInstance>> = BTreeMap::new();
    let mut stones: Vec<BoxInstance> = Vec::new();
    let mut wood: Vec<BoxInstance> = Vec::new();
    let mut flame: Vec<BoxInstance> = Vec::new();

    // THE STONE RING: 8 stones around the center, one box each.
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::TAU / 8.0;
        stones.push(BoxInstance {
            pos: [xz[0] + a.cos() * RING_RADIUS_M, base_y + 0.10, xz[1] + a.sin() * RING_RADIUS_M],
            yaw: a,
            scale: [0.28, 0.20, 0.28],
            pad: 0.0,
        });
    }

    // THE LOGS: two crossed logs under the flame.
    for (i, yaw) in [0.0f32, std::f32::consts::FRAC_PI_2].iter().enumerate() {
        wood.push(BoxInstance {
            pos: [xz[0], base_y + 0.08 + i as f32 * 0.06, xz[1]],
            yaw: *yaw + 0.3,
            scale: [0.55, 0.12, 0.12],
            pad: 0.0,
        });
    }

    // THE FLAME: three boxes whose heights flicker deterministically
    // (sin of the phase — same t, same flame; a paused game freezes it).
    if burning {
        let f = fuel_frac.clamp(0.15, 1.0);
        for (i, (off, sway)) in [(0.0f32, 0.0f32), (0.12, 0.9), (-0.12, -0.8)].iter().enumerate() {
            let ph = t * 6.0 + i as f32 * 2.1;
            let h = FLAME_HEIGHT_M * f * (0.75 + 0.25 * ph.sin());
            flame.push(BoxInstance {
                pos: [xz[0] + sway * 0.08 * ph.cos(), base_y + 0.10 + h * 0.5, xz[1] + *off * 0.5],
                yaw: ph * 0.2,
                scale: [0.30 - i as f32 * 0.06, h, 0.30 - i as f32 * 0.06],
                pad: 0.0,
            });
        }
    } else if embers {
        // The ember bed: one low glow box, no tall flame.
        flame.push(BoxInstance {
            pos: [xz[0], base_y + 0.07, xz[1]],
            yaw: 0.0,
            scale: [0.5, 0.09, 0.5],
            pad: 0.0,
        });
    }

    out.insert(PartColor::Stone, stones);
    out.insert(PartColor::Wood, wood);
    out.insert(PartColor::Flame, flame);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p3dn11_fire_boxes_are_lawful() {
        let xz = [140.5, 96.25];
        let y = 10.7;
        // Burning: 8 stones + 2 logs + 3 flame boxes.
        let full = campfire_instances(xz, y, true, false, 1.0, 1.0);
        assert_eq!(full[&PartColor::Stone].len(), 8);
        assert_eq!(full[&PartColor::Wood].len(), 2);
        assert_eq!(full[&PartColor::Flame].len(), 3);
        // The stones ring the center at the ring radius.
        for s in &full[&PartColor::Stone] {
            let d = ((s.pos[0] - xz[0]).powi(2) + (s.pos[2] - xz[1]).powi(2)).sqrt();
            assert!((d - RING_RADIUS_M).abs() < 1e-3, "stone on the ring");
            assert!(s.pos[1] > y, "nothing below the soil line");
        }
        // Cold: no flame boxes at all.
        let cold = campfire_instances(xz, y, false, false, 0.0, 1.0);
        assert!(cold[&PartColor::Flame].is_empty());
        // Embers: the low bed only.
        let embers = campfire_instances(xz, y, false, true, 0.0, 1.0);
        assert_eq!(embers[&PartColor::Flame].len(), 1);
        assert!(embers[&PartColor::Flame][0].scale[1] < 0.2, "embers are a bed, not a flame");
        // Deterministic: same t, same flame.
        assert_eq!(
            campfire_instances(xz, y, true, false, 1.0, 3.5),
            campfire_instances(xz, y, true, false, 1.0, 3.5)
        );
        // Low fuel scales the flame down (the fire visibly dies).
        let low = campfire_instances(xz, y, true, false, 0.2, 3.5);
        let hi = campfire_instances(xz, y, true, false, 1.0, 3.5);
        assert!(low[&PartColor::Flame][0].scale[1] < hi[&PartColor::Flame][0].scale[1]);
    }
}
