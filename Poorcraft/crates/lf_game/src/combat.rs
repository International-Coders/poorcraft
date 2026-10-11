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

/// Total armor points worn across the four armor slots (36=head, 37=chest,
/// 38=legs, 39=feet — loop 329: the full bronze/steel kit sums to 10/17).
/// Armor in a "wrong" slot still counts; the slot row exists so the player
/// can see what is worn.
pub fn worn_armor_points(slots: &[Option<ItemStack>]) -> u8 {
    slots
        .iter()
        .skip(36)
        .take(4)
        .filter_map(|s| s.as_ref())
        .filter_map(|s| item_def(&s.item_id))
        .map(|d| match d.kind {
            ItemKind::Armor(p) => p,
            _ => 0,
        })
        .sum()
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

    /// Loop 329: the four armor slots sum — a full bronze kit reads 10
    /// points, steel 17, and a 36-slot legacy save (no armor slots) is 0.
    #[test]
    fn armor_sums_across_all_four_slots() {
        let stack = |id: &str| Some(ItemStack { item_id: id.into(), count: 1 });
        let mut slots = vec![None; 41];
        slots[36] = stack("bronze_helmet");
        slots[37] = stack("bronze_chestplate");
        slots[38] = stack("bronze_leggings");
        slots[39] = stack("bronze_boots");
        assert_eq!(worn_armor_points(&slots), 10, "full bronze kit");
        slots[36] = stack("steel_helmet");
        slots[37] = stack("steel_chestplate");
        slots[38] = stack("steel_leggings");
        slots[39] = stack("steel_boots");
        assert_eq!(worn_armor_points(&slots), 17, "full steel kit");
        // mixed pieces count individually
        let mut mixed = vec![None; 41];
        mixed[36] = stack("steel_helmet");
        mixed[39] = stack("bronze_boots");
        assert_eq!(worn_armor_points(&mixed), 4);
        // legacy 36-slot inventory (pre-armor-row save) is safe
        assert_eq!(worn_armor_points(&vec![None; 36]), 0);
        // non-armor in an armor slot contributes nothing
        let mut junk = vec![None; 41];
        junk[36] = stack("stone");
        assert_eq!(worn_armor_points(&junk), 0);
    }
}
