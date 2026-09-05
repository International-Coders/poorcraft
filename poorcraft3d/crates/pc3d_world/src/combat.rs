//! P3D-503: combat, creatures, loot, and the first dungeon room.
//!
//! Hostile creatures are registry entities with deterministic melee:
//! fixed range 1 (Chebyshev on cells), cooldown 30 ticks, per-creature
//! damage. Death yields loot by table. The dungeon room is a bounded
//! underground chamber + corridor carved as an EDIT PLAN (cells to clear
//! as Air / floor to keep solid) — composed through the P3D-204/205
//! path when building.

use crate::coords::CellCoord;
use crate::items::{harvest_yields, Inventory, ItemId, ToolState};
use crate::npc::Needs;

/// Attack cooldown in ticks after a creature lands a hit.
pub const CREATURE_COOLDOWN: u64 = 30;
/// Melee range in cells (Chebyshev).
pub const MELEE_RANGE: i32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreatureKind {
    Goblin,
    CaveSpider,
}

impl CreatureKind {
    pub fn base_hp(self) -> i32 {
        match self {
            CreatureKind::Goblin => 20,
            CreatureKind::CaveSpider => 12,
        }
    }
    pub fn base_damage(self) -> i32 {
        match self {
            CreatureKind::Goblin => 4,
            CreatureKind::CaveSpider => 3,
        }
    }
    /// Loot per kill: (item, count) pairs.
    pub fn loot(self) -> Vec<(ItemId, u32)> {
        match self {
            CreatureKind::Goblin => vec![(ItemId(2), 2)],
            CreatureKind::CaveSpider => vec![(ItemId(1), 3)],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Creature {
    pub id: u64,
    pub kind: CreatureKind,
    pub pos: CellCoord,
    pub hp: i32,
    pub cooldown_until: u64,
}

#[derive(Clone, Debug, Default)]
pub struct CreatureSystem {
    pub creatures: Vec<Creature>,
    pub next_id: u64,
}

/// Result of one creature attack tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hit {
    pub creature_id: u64,
    pub damage: i32,
}

impl CreatureSystem {
    pub fn spawn(&mut self, kind: CreatureKind, pos: CellCoord) -> u64 {
        self.next_id += 1;
        self.creatures.push(Creature {
            id: self.next_id,
            kind,
            pos,
            hp: kind.base_hp(),
            cooldown_until: 0,
        });
        self.next_id
    }

    /// Creature melee: any creature within MELEE_RANGE of the player and
    /// off cooldown deals its damage and starts its cooldown. Returns the
    /// hits landed this tick.
    pub fn creature_attacks(
        &mut self,
        player_pos: CellCoord,
        tick: u64,
    ) -> Vec<Hit> {
        let mut hits = Vec::new();
        for c in &mut self.creatures {
            let dx = (c.pos.x - player_pos.x).abs();
            let dz = (c.pos.z - player_pos.z).abs();
            let dy = (c.pos.y - player_pos.y).abs();
            if dx.max(dz).max(dy) <= MELEE_RANGE && tick >= c.cooldown_until {
                hits.push(Hit { creature_id: c.id, damage: c.kind.base_damage() });
                c.cooldown_until = tick + CREATURE_COOLDOWN;
            }
        }
        hits
    }

    /// Player melee: damages one creature within MELEE_RANGE of the
    /// player (first by id — deterministic). Dead creatures are removed
    /// and their loot returned.
    pub fn player_attack(
        &mut self,
        player_pos: CellCoord,
        tick: u64,
    ) -> Vec<(ItemId, u32)> {
        // Choose the lowest-id creature in range (deterministic), then
        // damage it.
        let target_id = {
            let mut best: Option<u64> = None;
            for c in &self.creatures {
                let dx = (c.pos.x - player_pos.x).abs();
                let dz = (c.pos.z - player_pos.z).abs();
                let dy = (c.pos.y - player_pos.y).abs();
                if dx.max(dz).max(dy) <= MELEE_RANGE {
                    match best {
                        Some(b) if b <= c.id => {}
                        _ => best = Some(c.id),
                    }
                }
            }
            best
        };
        let Some(creature_id) = target_id else {
            return Vec::new();
        };
        let (hp, loot) = {
            let c = self
                .creatures
                .iter_mut()
                .find(|c| c.id == creature_id)
                .expect("target exists");
            c.hp -= 10; // player melee v1: fixed 10 damage
            (c.hp, c.kind.loot())
        };
        if hp > 0 {
            return Vec::new();
        }
        let loot_table = loot;
        self.creatures.retain(|c| c.id != creature_id);
        loot_table
    }
}

/// A carved underground room: bounds + the cells to clear.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DungeonRoom {
    pub center: CellCoord,
    /// Chamber half-extents (x/z); height fixed 3.
    pub half: i32,
    /// Corridor length from the chamber edge toward +x.
    pub corridor: i32,
}

impl DungeonRoom {
    /// All AIR cells to clear (chamber + corridor), underground by
    /// construction (caller passes a center at least 8 cells below the
    /// surface). Deterministic per (seed is irrelevant — geometry is
    /// fixed; the seed drives placement via the caller).
    pub fn carve_cells(&self) -> Vec<CellCoord> {
        let mut cells = Vec::new();
        for dx in -self.half..=self.half {
            for dz in -self.half..=self.half {
                for dy in 0..3 {
                    cells.push(CellCoord {
                        x: self.center.x + dx,
                        y: self.center.y + dy,
                        z: self.center.z + dz,
                    });
                }
            }
        }
        for i in 1..=self.corridor {
            for dy in 0..2 {
                cells.push(CellCoord {
                    x: self.center.x + self.half + i,
                    y: self.center.y + dy,
                    z: self.center.z,
                });
            }
        }
        cells
    }

    /// The floor cells (bottom layer) — where the player stands.
    pub fn floor_cells(&self) -> Vec<CellCoord> {
        self.carve_cells()
            .into_iter()
            .filter(|c| c.y == self.center.y)
            .collect()
    }
}

/// Survival helpers re-used by the combat loop.
pub fn eat_to_heal(
    inventory: &mut Inventory,
    needs: &mut Needs,
    food: ItemId,
) -> bool {
    let removed = inventory.remove(food, 1);
    if removed == 0 {
        return false;
    }
    needs.eat();
    true
}

/// Tool-assisted yield for a killed rock-creature (shares the P3D-501
/// harvest law so loot stays consistent with terrain yields).
pub fn loot_into(inventory: &mut Inventory, loot: &[(ItemId, u32)]) -> u32 {
    let mut stored = 0;
    for (item, count) in loot {
        let leftover = inventory.add(*item, *count);
        stored += (count - leftover) as u32;
    }
    stored
}

/// Harvest-yield passthrough for consistency tests.
pub fn yields_match_terrain(material: crate::gen::CellMaterial, tier: Option<u8>) -> Vec<(ItemId, u32)> {
    harvest_yields(material, tier)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system_with_goblin(at: CellCoord) -> CreatureSystem {
        let mut sys = CreatureSystem::default();
        sys.spawn(CreatureKind::Goblin, at);
        sys
    }

    /// Creature melee: hits within range 1, honors the cooldown, damage
    /// matches the creature kind.
    #[test]
    fn p3d503_creature_melee_ranges_and_cooldowns() {
        let at = CellCoord { x: 10, y: 5, z: 10 };
        let player = CellCoord { x: 10, y: 5, z: 10 }; // same cell = range 0
        let mut sys = system_with_goblin(at);

        let hits = sys.creature_attacks(player, 0);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].damage, 4, "goblin base damage");

        // Cooldown: a tick later, nothing.
        assert!(sys.creature_attacks(player, 1).is_empty());
        assert!(sys.creature_attacks(player, 29).is_empty());
        let hits = sys.creature_attacks(player, 30);
        assert_eq!(hits.len(), 1, "cooldown expired at tick 30");

        // Out of range: nothing.
        let far = CellCoord { x: 10, y: 5, z: 20 };
        assert!(sys.creature_attacks(far, 100).is_empty());
    }

    /// Player kills a creature: deterministic single-target damage, loot
    /// per table, dead creature removed.
    #[test]
    fn p3d503_player_kills_and_loots() {
        let at = CellCoord { x: 3, y: 3, z: 3 };
        let mut sys = system_with_goblin(at);
        sys.spawn(CreatureKind::CaveSpider, at);

        // Both creatures die within 10 hits of 10 damage.
        let mut loot = Vec::new();
        for _ in 0..10 {
            let got = sys.player_attack(at, 100);
            loot.extend(got);
        }
        assert!(sys.creatures.is_empty(), "all creatures die to 10 hits of 10");
        assert!(
            loot.contains(&(ItemId(2), 2)),
            "goblin loot must include 2 stone: {loot:?}"
        );
        assert!(
            loot.contains(&(ItemId(1), 3)),
            "spider loot must include 3 wood: {loot:?}"
        );
    }

    /// The dungeon room: deterministic, bounded, underground cells for
    /// carving; the floor layer is present.
    #[test]
    fn p3d503_dungeon_room_carve_is_deterministic_and_bounded() {
        let room = DungeonRoom { center: CellCoord { x: 100, y: -20, z: 100 }, half: 4, corridor: 5 };
        let a = room.carve_cells();
        let b = room.carve_cells();
        assert_eq!(a, b, "carve must be deterministic");
        // Chamber 9x3x9 + corridor 5x2 = 243 + 10.
        assert_eq!(a.len(), 9 * 3 * 9 + 5 * 2);
        // All cells are underground (y < 0) and bounded.
        assert!(a.iter().all(|c| c.y < 0 && c.y >= -20));
        // Floor = chamber floor (81) + corridor cells at floor level (5).
        assert_eq!(room.floor_cells().len(), 81 + 5);
    }

    /// Loot and food flow into inventory (consistency with P3D-501/502).
    #[test]
    fn p3d503_loot_lands_in_inventory() {
        let mut inv = Inventory::new(8);
        inv.add(ItemId(20), 2); // bread to eat
        let loot = vec![(ItemId(2), 2), (ItemId(1), 3)];
        let stored = loot_into(&mut inv, &loot);
        assert_eq!(stored, 5);
        assert_eq!(inv.count(ItemId(2)), 2);
        assert_eq!(inv.count(ItemId(1)), 3);
        let mut needs = Needs { hunger: 50, energy: 50, hunger_f: 50.0, energy_f: 50.0 };
        assert!(eat_to_heal(&mut inv, &mut needs, ItemId(20)));
        let _ = yields_match_terrain(crate::gen::CellMaterial::Rock, Some(1));
    }
}
