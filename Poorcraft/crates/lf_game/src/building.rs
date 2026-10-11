//! P35 smart building: conduit-relayed power distribution, elevator
//! shafts, climate comfort. Pure functions over the world so tests and
//! the client share one path.

use crate::machines::POWER_RANGE;
use lf_voxel::registry::block;
use lf_voxel::World;

/// How many conduit hops a field may travel (relay chains stay finite).
pub const MAX_RELAY_HOPS: usize = 4;

/// Multi-hop power reachability: a machine is reachable from a source if
/// it is within POWER_RANGE of the source, or within POWER_RANGE of any
/// conduit reachable from the source through chains of conduits (each
/// hop <= POWER_RANGE, at most MAX_RELAY_HOPS deep).
pub fn relayed_reachable(
    source: (i32, i32, i32),
    machine: (i32, i32, i32),
    conduits: &[(i32, i32, i32)],
) -> bool {
    let near = |a: (i32, i32, i32), b: (i32, i32, i32)| {
        let d = ((a.0 - b.0).pow(2) + (a.1 - b.1).pow(2) + (a.2 - b.2).pow(2)) as f32;
        d.sqrt() <= POWER_RANGE
    };
    if near(source, machine) {
        return true;
    }
    // BFS outward from the source through conduits
    let mut frontier = vec![source];
    let mut visited = vec![source];
    for _ in 0..MAX_RELAY_HOPS {
        let mut next = Vec::new();
        for &f in &frontier {
            for &c in conduits {
                if near(f, c) && !visited.contains(&c) {
                    if near(c, machine) {
                        return true;
                    }
                    visited.push(c);
                    next.push(c);
                }
            }
        }
        if next.is_empty() {
            return false;
        }
        frontier = next;
    }
    false
}

/// The next elevator block in a column strictly above/below `y` (P35:
/// powered vertical ride between platforms). Returns its y.
pub fn next_elevator_y(world: &World, x: i32, y: i32, z: i32, up: bool) -> Option<i32> {
    let mut ty = if up { y + 1 } else { y - 1 };
    loop {
        if ty < 0 || ty > 255 {
            return None;
        }
        if world.get_block(x, ty, z).id() == block::ELEVATOR {
            return Some(ty + 1); // the platform you stand ON
        }
        ty = if up { ty + 1 } else { ty - 1 };
    }
}

/// Climate comfort (P35): an AC unit within 4 blocks of the player, with
/// a power producer within 4 blocks of the unit, regenerates a little.
pub fn climate_comfort(world: &World, pos: (i32, i32, i32), producers: &[(i32, i32, i32)]) -> bool {
    let near = |a: (i32, i32, i32)| {
        let d = ((a.0 - pos.0).pow(2) + (a.1 - pos.1).pow(2) + (a.2 - pos.2).pow(2)) as f32;
        d.sqrt() <= 4.0
    };
    let mut found_unit = false;
    for dx in -4..=4i32 {
        for dy in -2..=3i32 {
            for dz in -4..=4i32 {
                if world.get_block(pos.0 + dx, pos.1 + dy, pos.2 + dz).id() == block::AC_UNIT {
                    let unit = (pos.0 + dx, pos.1 + dy, pos.2 + dz);
                    if producers.iter().any(|p| {
                        let d = ((p.0 - unit.0).pow(2) + (p.1 - unit.1).pow(2) + (p.2 - unit.2).pow(2)) as f32;
                        d.sqrt() <= POWER_RANGE
                    }) {
                        found_unit = true;
                    }
                }
            }
        }
    }
    found_unit
}

#[cfg(test)]
mod tests {
    use super::*;
    use lf_voxel::BlockState;

    fn flat() -> World {
        let mut w = World::new();
        w.ensure_chunk(0, 0);
        for x in -2..20 {
            for z in -2..4 {
                w.set_block(x, 0, z, BlockState::STONE);
            }
        }
        w
    }

    /// Relays bridge the gap the raw field cannot: 10 blocks is beyond
    /// POWER_RANGE (4), two conduits chain it.
    #[test]
    fn conduits_relay_power_beyond_range() {
        let source = (0, 0, 0);
        let machine = (10, 0, 0);
        assert!(!relayed_reachable(source, machine, &[]), "raw field stops at ~4 blocks");
        let conduits = [(3, 0, 0), (7, 0, 0)];
        assert!(relayed_reachable(source, machine, &conduits), "two hops bridge 10 blocks");
        // a broken chain does not
        assert!(!relayed_reachable(source, machine, &[(3, 0, 0), (20, 0, 0)]));
        // hop cap: 5 conduits of 3 blocks each = 15 blocks of chain is over the cap
        let long = [(3, 0, 0), (6, 0, 0), (9, 0, 0), (12, 0, 0), (15, 0, 0)];
        assert!(!relayed_reachable(source, (18, 0, 0), &long), "relay chains stay finite (4 hops)");
    }

    /// Elevators find the next platform up and down the shaft.
    #[test]
    fn elevators_find_next_platform() {
        let mut w = flat();
        for y in 1..8 {
            w.set_block(0, y, 0, BlockState(block::ELEVATOR));
        }
        // standing on the y=1 elevator (platform y=2): next up = 3
        assert_eq!(next_elevator_y(&w, 0, 1, 0, true), Some(3));
        assert_eq!(next_elevator_y(&w, 0, 5, 0, false), Some(5), "descend to stand on the elevator below (feet at 5)");
        // nothing above the top shaft
        assert_eq!(next_elevator_y(&w, 0, 7, 0, true), None);
    }

    /// Climate comfort needs a powered unit nearby.
    #[test]
    fn climate_needs_a_powered_unit() {
        let mut w = flat();
        w.set_block(3, 1, 0, BlockState(block::AC_UNIT));
        let player = (0, 1, 0);
        assert!(!climate_comfort(&w, player, &[]), "unpowered AC does nothing");
        assert!(climate_comfort(&w, player, &[(5, 1, 0)]), "a producer near the unit comforts");
        // unit out of reach of the player
        w.set_block(3, 1, 0, BlockState::AIR);
        w.set_block(12, 1, 0, BlockState(block::AC_UNIT));
        assert!(!climate_comfort(&w, player, &[(13, 1, 0)]), "too far to feel it");
    }
}
