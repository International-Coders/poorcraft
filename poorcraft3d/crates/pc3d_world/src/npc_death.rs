//! P3D-612: permanent named-NPC death, replacement, service loss.
//!
//! Named NPCs have identity (name, role, skill level). Death is permanent.
//! Replacement NPCs are generated with lower skills. Service loss is
//! tracked per building (blacksmith dies = no more crafting there).

/// A named NPC with identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedNpc {
    pub entity: crate::entities::EntityId,
    pub name: String,
    pub role: crate::npc::Role,
    pub skill: u8,
    pub alive: bool,
}

/// Tracks named NPCs and service losses.
#[derive(Clone, Debug, Default)]
pub struct NpcRoster {
    pub npcs: Vec<NamedNpc>,
    next_id: u64,
}

impl NpcRoster {
    pub fn new() -> Self {
        NpcRoster::default()
    }

    /// Add a named NPC.
    pub fn recruit(&mut self, name: &str, role: crate::npc::Role, skill: u8) -> crate::entities::EntityId {
        self.next_id += 1;
        let id = crate::entities::EntityId(self.next_id);
        self.npcs.push(NamedNpc {
            entity: id,
            name: name.to_string(),
            role,
            skill,
            alive: true,
        });
        id
    }

    /// Permanent death: marks NPC as dead. Cannot be undone.
    pub fn kill(&mut self, id: crate::entities::EntityId) -> bool {
        let Some(npc) = self.npcs.iter_mut().find(|n| n.entity == id && n.alive) else {
            return false;
        };
        npc.alive = false;
        true
    }

    /// Replace a dead NPC with a lower-skilled replacement.
    /// Returns the new entity id, or None if the original is still alive.
    pub fn replace(&mut self, dead_id: crate::entities::EntityId, name: &str) -> Option<crate::entities::EntityId> {
        let dead = self.npcs.iter().find(|n| n.entity == dead_id)?;
        if dead.alive {
            return None;
        }
        let skill = dead.skill.saturating_sub(2);
        let id = self.recruit(name, dead.role, skill);
        Some(id)
    }

    /// Service loss: is there a living NPC with this role?
    pub fn has_service(&self, role: crate::npc::Role) -> bool {
        self.npcs.iter().any(|n| n.alive && n.role == role)
    }

    /// Living NPCs of a given role.
    pub fn living_by_role(&self, role: crate::npc::Role) -> Vec<&NamedNpc> {
        self.npcs.iter().filter(|n| n.alive && n.role == role).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npc::Role;

    #[test]
    fn p3d612_death_is_permanent() {
        let mut roster = NpcRoster::new();
        let id = roster.recruit("Grimward", Role::Builder, 5);
        assert!(roster.kill(id));
        assert!(!roster.kill(id), "double death refused");
        assert!(!roster.has_service(Role::Builder), "dead builder provides no service");
    }

    #[test]
    fn p3d612_replacement_has_lower_skills() {
        let mut roster = NpcRoster::new();
        let old_id = roster.recruit("Master Smith", Role::Builder, 8);
        roster.kill(old_id);
        let new_id = roster.replace(old_id, "Apprentice").expect("replacement");
        let new_npc = roster.npcs.iter().find(|n| n.entity == new_id).unwrap();
        assert_eq!(new_npc.role, Role::Builder);
        assert!(new_npc.skill < 8, "replacement has lower skill");
    }

    #[test]
    fn p3d612_service_loss_tracked() {
        let mut roster = NpcRoster::new();
        roster.recruit("Blacksmith A", Role::Builder, 5);
        roster.recruit("Blacksmith B", Role::Builder, 3);
        assert!(roster.has_service(Role::Builder));
        // Kill one: service still exists.
        let alive_ids: Vec<_> = roster.living_by_role(Role::Builder).iter().map(|n| n.entity).collect();
        roster.kill(alive_ids[0]);
        assert!(roster.has_service(Role::Builder), "one builder still alive");
        // Kill the other: service lost.
        let alive_ids: Vec<_> = roster.living_by_role(Role::Builder).iter().map(|n| n.entity).collect();
        for id in alive_ids {
            roster.kill(id);
        }
        assert!(!roster.has_service(Role::Builder), "all builders dead");
    }
}
