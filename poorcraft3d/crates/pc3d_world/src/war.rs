//! P3D-611: war and defense objectives — high-level NPC intents.
//!
//! Wars are experienced in first person (D-024). The player gives a
//! high-level objective; NPCs path to it via nav and hold. No remote
//! RTS micro-control.

use crate::coords::CellCoord;
use crate::nav::NavPatch;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarObjective {
    DefendGate,
    HoldWall,
    AttackTarget,
    Retreat,
}

/// One NPC's war assignment.
#[derive(Clone, Debug, PartialEq)]
pub struct WarAssignment {
    pub entity: crate::entities::EntityId,
    pub objective: WarObjective,
    pub path: Vec<CellCoord>,
    pub leg: usize,
    pub arrived: bool,
}

/// Assign NPCs to an objective with nav paths.
pub fn assign_npcs(
    nav: &NavPatch,
    npcs: &[crate::entities::Entity],
    objective_cell: CellCoord,
    objective: WarObjective,
) -> Vec<WarAssignment> {
    npcs
        .iter()
        .filter_map(|e| {
            let path = nav.path(e.cell, objective_cell)?;
            Some(WarAssignment {
                entity: e.id,
                objective,
                path,
                leg: 0,
                arrived: false,
            })
        })
        .collect()
}

/// Advance one war assignment: consume one path leg per tick.
pub fn advance(assignments: &mut [WarAssignment]) {
    for a in assignments.iter_mut() {
        if a.arrived || a.leg >= a.path.len() {
            a.arrived = true;
            continue;
        }
        a.leg += 1;
        if a.leg >= a.path.len() {
            a.arrived = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::WorldGen;

    #[test]
    fn p3d611_assign_and_advance_to_objective() {
        let gen = WorldGen::new(3);
        let patch = crate::terrain::SceneSpec::SmoothHills.patch().1;
        let nav = NavPatch::from_gen(&gen, patch);
        let o = patch.origin();
        let cell = |lx: i32, lz: i32| crate::coords::CellCoord {
            x: o.x.div_euclid(1000) as i32 + lx,
            y: 0,
            z: o.z.div_euclid(1000) as i32 + lz,
        };
        let npcs = vec![
            crate::entities::Entity { id: crate::entities::EntityId(1), kind: crate::entities::EntityKind::Villager, cell: cell(2, 2), data: 0 },
            crate::entities::Entity { id: crate::entities::EntityId(2), kind: crate::entities::EntityKind::Villager, cell: cell(3, 3), data: 0 },
        ];
        let target = cell(10, 10);
        let mut assignments = assign_npcs(&nav, &npcs, target, WarObjective::DefendGate);
        assert_eq!(assignments.len(), 2);
        // Advance until arrived.
        for _ in 0..100 {
            advance(&mut assignments);
        }
        assert!(assignments.iter().all(|a| a.arrived), "all NPCs must arrive");
    }
}
