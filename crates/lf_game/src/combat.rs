//! Projectiles (arrows), XP levels, and armor mitigation.

use crate::items::{item_def, ItemKind};
use crate::survival::ItemStack;
use glam::Vec3;

/// A flying arrow.
#[derive(Clone, Debug)]
pub struct Arrow {
    pub position: Vec3,
    pub velocity: Vec3,
    pub age: f32,
}

impl Arrow {
    /// Advance physics; returns Some(()) when it hit something solid
    /// (position snapped to the impact).
    pub fn update(&mut self, dt: f32, solid: impl Fn(i32, i32, i32) -> bool) -> bool {
        self.age += dt;
        self.velocity.y -= 18.0 * dt;
        let next = self.position + self.velocity * dt;
        let hit = solid(next.x as i32, next.y as i32, next.z as i32);
        if !hit {
            self.position = next;
        }
        hit || self.age > 8.0
    }
}

/// XP curve: levels cost 7 + level*3 points.
pub fn xp_for_level(level: u32) -> u32 {
    7 + level * 3
}

/// Grant xp, carrying levels over. Returns the new (level, progress).
pub fn grant_xp(mut level: u32, mut progress: u32, amount: u32) -> (u32, u32) {
    progress += amount;
    while progress >= xp_for_level(level) {
        progress -= xp_for_level(level);
        level += 1;
    }
    (level, progress)
}

/// Damage after armor: flat reduction, min 1.
pub fn mitigate(damage: f32, armor_points: u8) -> f32 {
    (damage - armor_points as f32).max(1.0)
}

/// Total armor points worn from equipped chest slot (simplified: any armor
/// in the hotbar-adjacent "armor" slot index 36 counts).
pub fn worn_armor_points(slots: &[Option<ItemStack>]) -> u8 {
    slots
        .get(36)
        .and_then(|s| s.as_ref())
        .and_then(|s| item_def(&s.item_id))
        .map(|d| match d.kind {
            ItemKind::Armor(p) => p,
            _ => 0,
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_fall_and_hit() {
        let mut a = Arrow { position: Vec3::new(0.0, 20.0, 0.0), velocity: Vec3::new(5.0, 0.0, 0.0), age: 0.0 };
        let mut hit = false;
        for _ in 0..400 {
            if a.update(0.05, |x, y, _| y <= 0 || (x > 6 && y < 18)) {
                hit = true;
                break;
            }
        }
        assert!(hit, "arrow should hit the ground");
        assert!(a.position.y <= 20.0);
    }

    #[test]
    fn xp_levels_accumulate() {
        let (l, p) = grant_xp(0, 0, 10); // level 0 costs 7
        assert_eq!(l, 1);
        assert_eq!(p, 3);
        let (l, _) = grant_xp(1, 0, 10); // level 1 costs 10
        assert_eq!(l, 2);
        assert_eq!(xp_for_level(5), 22);
    }

    #[test]
    fn armor_reduces_but_never_nullifies() {
        assert_eq!(mitigate(10.0, 4), 6.0);
        assert_eq!(mitigate(10.0, 20), 1.0);
        let slots = vec![None; 37];
        assert_eq!(worn_armor_points(&slots), 0);
    }
}
