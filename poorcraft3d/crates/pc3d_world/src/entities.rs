//! P3D-402: the entity registry — stable ids, spatial index, persistent
//! records, per-entity interest state, and deterministic update ordering.
//!
//! The law: iteration and ordering are ALWAYS by id or position (BTree,
//! never a hash map order), so the same entity set produces the same
//! update sequence on every host.

use crate::coords::{CellCoord, PatchCoord, WorldPos};
use crate::lod::lod_for;
use crate::scales::PATCH_MM;
use std::collections::BTreeMap;

/// Stable entity identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u64);

/// What kind of entity this is (grows with the stages).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityKind {
    Villager,
    Animal,
    Fish,
    Marker,
}

impl EntityKind {
    pub fn code(self) -> u8 {
        match self {
            EntityKind::Villager => 1,
            EntityKind::Animal => 2,
            EntityKind::Fish => 3,
            EntityKind::Marker => 4,
        }
    }
    pub fn from_code(c: u8) -> Option<Self> {
        match c {
            1 => Some(EntityKind::Villager),
            2 => Some(EntityKind::Animal),
            3 => Some(EntityKind::Fish),
            4 => Some(EntityKind::Marker),
            _ => None,
        }
    }
}

/// One entity: identity, kind, cell position, and one opaque data word
/// (future per-kind payload hash).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entity {
    pub id: EntityId,
    pub kind: EntityKind,
    pub cell: CellCoord,
    pub data: u64,
}

/// Fixed-width record (48 bytes): id u64 | kind u8 | pad 7 |
/// cx,cy,cz i32 | data u64 | pad 1.
impl Entity {
    pub fn encode(&self) -> [u8; 48] {
        let mut b = [0u8; 48];
        b[0..8].copy_from_slice(&self.id.0.to_le_bytes());
        b[8] = self.kind.code();
        b[16..20].copy_from_slice(&self.cell.x.to_le_bytes());
        b[20..24].copy_from_slice(&self.cell.y.to_le_bytes());
        b[24..28].copy_from_slice(&self.cell.z.to_le_bytes());
        b[32..40].copy_from_slice(&self.data.to_le_bytes());
        b
    }

    pub fn decode(b: &[u8; 48]) -> Option<Entity> {
        Some(Entity {
            id: EntityId(u64::from_le_bytes(b[0..8].try_into().ok()?)),
            kind: EntityKind::from_code(b[8])?,
            cell: CellCoord {
                x: i32::from_le_bytes(b[16..20].try_into().ok()?),
                y: i32::from_le_bytes(b[20..24].try_into().ok()?),
                z: i32::from_le_bytes(b[24..28].try_into().ok()?),
            },
            data: u64::from_le_bytes(b[32..40].try_into().ok()?),
        })
    }
}

/// The registry: entities keyed by id, plus a spatial index by patch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntityRegistry {
    entities: BTreeMap<EntityId, Entity>,
    by_patch: BTreeMap<PatchCoord, Vec<EntityId>>,
    next_id: u64,
}

impl EntityRegistry {
    pub fn new() -> Self {
        EntityRegistry { entities: BTreeMap::new(), by_patch: BTreeMap::new(), next_id: 0 }
    }

    fn patch_of(cell: CellCoord) -> PatchCoord {
        PatchCoord {
            x: cell.x.div_euclid(16),
            y: cell.y.div_euclid(16),
            z: cell.z.div_euclid(16),
        }
    }

    /// Insert a new entity with an auto-assigned id; returns it.
    pub fn spawn(&mut self, kind: EntityKind, cell: CellCoord, data: u64) -> EntityId {
        self.next_id += 1;
        let id = EntityId(self.next_id);
        let e = Entity { id, kind, cell, data };
        self.entities.insert(id, e);
        self.by_patch.entry(Self::patch_of(cell)).or_default().push(id);
        id
    }

    /// Insert with an explicit id (persistence path); refuses duplicates.
    pub fn insert(&mut self, e: Entity) -> bool {
        if self.entities.contains_key(&e.id) {
            return false;
        }
        self.next_id = self.next_id.max(e.id.0);
        self.entities.insert(e.id, e);
        self.by_patch.entry(Self::patch_of(e.cell)).or_default().push(e.id);
        true
    }

    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    /// Remove an entity; returns it.
    pub fn despawn(&mut self, id: EntityId) -> Option<Entity> {
        let e = self.entities.remove(&id)?;
        let patch = Self::patch_of(e.cell);
        if let Some(list) = self.by_patch.get_mut(&patch) {
            list.retain(|&i| i != id);
            if list.is_empty() {
                self.by_patch.remove(&patch);
            }
        }
        Some(e)
    }

    /// Move an entity to a new cell (updates the spatial index).
    pub fn move_entity(&mut self, id: EntityId, to: CellCoord) -> bool {
        let Some(e) = self.entities.get_mut(&id) else {
            return false;
        };
        let old_patch = Self::patch_of(e.cell);
        e.cell = to;
        let new_patch = Self::patch_of(to);
        if old_patch != new_patch {
            if let Some(list) = self.by_patch.get_mut(&old_patch) {
                list.retain(|&i| i != id);
                if list.is_empty() {
                    self.by_patch.remove(&old_patch);
                }
            }
            self.by_patch.entry(new_patch).or_default().push(id);
        }
        true
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Entities in one patch, ascending by id.
    pub fn by_patch(&self, patch: PatchCoord) -> Vec<Entity> {
        self.by_patch
            .get(&patch)
            .map(|ids| ids.iter().filter_map(|i| self.entities.get(i).copied()).collect())
            .unwrap_or_default()
    }

    /// Entities within a Chebyshev-cell radius of a center cell,
    /// ascending by id (deterministic).
    pub fn entities_near(&self, center: CellCoord, radius: i32) -> Vec<Entity> {
        let span = crate::scales::PATCH_CELL_AXIS as i32;
        let min = PatchCoord {
            x: (center.x - radius).div_euclid(span),
            y: (center.y - radius).div_euclid(span),
            z: (center.z - radius).div_euclid(span),
        };
        let max = PatchCoord {
            x: (center.x + radius).div_euclid(span),
            y: (center.y + radius).div_euclid(span),
            z: (center.z + radius).div_euclid(span),
        };
        let mut out = Vec::new();
        for px in min.x..=max.x {
            for py in min.y..=max.y {
                for pz in min.z..=max.z {
                    out.extend(self.by_patch(PatchCoord { x: px, y: py, z: pz }));
                }
            }
        }
        out.sort_by_key(|e| e.id);
        out.retain(|e| {
            (e.cell.x - center.x).abs() <= radius
                && (e.cell.y - center.y).abs() <= radius
                && (e.cell.z - center.z).abs() <= radius
        });
        out
    }

    /// THE deterministic update order: all entities sorted by id.
    pub fn update_order(&self) -> Vec<Entity> {
        self.entities.values().copied().collect()
    }

    /// Per-entity interest state from the viewer (P3D-206 lod_for).
    /// Entities in the Horizon band are EXCLUDED (callers must not tick
    /// them), the rest are (id, lod) ascending.
    pub fn interest_state(
        &self,
        viewer: WorldPos,
    ) -> Vec<(EntityId, crate::lod::LodLevel)> {
        let mut out = Vec::new();
        for e in self.entities.values() {
            let center_mm = [
                e.cell.x as i64 * 1000 + 500,
                e.cell.y as i64 * 1000 + 500,
                e.cell.z as i64 * 1000 + 500,
            ];
            let lod = lod_for(
                viewer,
                WorldPos::from_mm(center_mm[0], center_mm[1], center_mm[2]),
            );
            if lod != crate::lod::LodLevel::Horizon {
                out.push((e.id, lod));
            }
        }
        out
    }

    /// Persistence encoding: count u64 LE + records (deterministic order).
    pub fn encode(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&(self.entities.len() as u64).to_le_bytes());
        b.extend_from_slice(&self.next_id.to_le_bytes());
        for e in self.entities.values() {
            b.extend_from_slice(&e.encode());
        }
        b
    }

    pub fn decode(bytes: &[u8]) -> Option<EntityRegistry> {
        if bytes.len() < 16 {
            return None;
        }
        let count = u64::from_le_bytes(bytes[..8].try_into().ok()?) as usize;
        let next_id = u64::from_le_bytes(bytes[8..16].try_into().ok()?);
        if bytes.len() != 16 + count * 48 {
            return None;
        }
        let mut reg = EntityRegistry::new();
        for i in 0..count {
            let rec: [u8; 48] = bytes[16 + i * 48..16 + (i + 1) * 48].try_into().ok()?;
            reg.insert(Entity::decode(&rec)?);
        }
        reg.next_id = next_id;
        Some(reg)
    }
}

/// Millimeter world position of a cell's center (consumer convenience).
pub fn cell_center_mm(cell: CellCoord) -> (i64, i64, i64) {
    (
        cell.x as i64 * PATCH_MM + PATCH_MM / 2,
        cell.y as i64 * PATCH_MM + PATCH_MM / 2,
        cell.z as i64 * PATCH_MM + PATCH_MM / 2,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ids are unique, iteration is by id (never map order), and update
    /// order is the same sequence every call.
    #[test]
    fn p3d402_registry_is_deterministic() {
        let mut reg = EntityRegistry::new();
        let a = reg.spawn(EntityKind::Villager, CellCoord { x: 5, y: 1, z: 6 }, 1);
        let b = reg.spawn(EntityKind::Animal, CellCoord { x: -3, y: 2, z: 8 }, 2);
        let c = reg.spawn(EntityKind::Fish, CellCoord { x: 0, y: -4, z: 0 }, 3);
        assert_ne!(a, b);
        assert_ne!(b, c);
        let order1: Vec<EntityId> = reg.update_order().iter().map(|e| e.id).collect();
        let order2: Vec<EntityId> = reg.update_order().iter().map(|e| e.id).collect();
        assert_eq!(order1, order2);
        assert_eq!(order1, vec![a, b, c], "insertion ids are ascending");
        // Duplicate explicit insert refused.
        assert!(!reg.insert(Entity { id: a, kind: EntityKind::Marker, cell: CellCoord { x: 0, y: 0, z: 0 }, data: 0 }));
    }

    /// Spatial queries: by_patch and entities_near are exact (including
    /// negative coordinates), move_entity updates the index.
    #[test]
    fn p3d402_spatial_queries_are_exact() {
        let mut reg = EntityRegistry::new();
        let in_patch = reg.spawn(EntityKind::Villager, CellCoord { x: 8, y: 0, z: 8 }, 0);
        let neg = reg.spawn(EntityKind::Animal, CellCoord { x: -8, y: 0, z: -8 }, 0);
        let far = reg.spawn(EntityKind::Fish, CellCoord { x: 500, y: 0, z: 500 }, 0);

        let p0 = reg.by_patch(PatchCoord { x: 0, y: 0, z: 0 });
        assert_eq!(p0.len(), 1);
        assert_eq!(p0[0].id, in_patch);
        let pn = reg.by_patch(PatchCoord { x: -1, y: 0, z: -1 });
        assert_eq!(pn.len(), 1);
        assert_eq!(pn[0].id, neg);

        let near = reg.entities_near(CellCoord { x: 0, y: 0, z: 0 }, 20);
        let ids: Vec<EntityId> = near.iter().map(|e| e.id).collect();
        assert!(ids.contains(&in_patch));
        assert!(ids.contains(&neg));
        assert!(!ids.contains(&far), "far entity must not appear");
        for w in near.windows(2) {
            assert!(w[0].id < w[1].id, "ascending order");
        }

        // Move crosses a patch boundary: index follows.
        assert!(reg.move_entity(in_patch, CellCoord { x: -8, y: 0, z: -8 }));
        assert!(reg.by_patch(PatchCoord { x: 0, y: 0, z: 0 }).is_empty());
        assert_eq!(reg.by_patch(PatchCoord { x: -1, y: 0, z: -1 }).len(), 2);
        // Despawn removes from both maps.
        assert!(reg.despawn(neg).is_some());
        assert!(reg.get(neg).is_none());
    }

    /// Persistence: encode/decode round-trips exactly (ids, cells, data,
    /// next-id high-water mark), unknown kind codes refuse.
    #[test]
    fn p3d402_registry_persists_exactly() {
        let mut reg = EntityRegistry::new();
        let a = reg.spawn(EntityKind::Villager, CellCoord { x: 1, y: 2, z: 3 }, 42);
        reg.spawn(EntityKind::Animal, CellCoord { x: -1, y: -2, z: -3 }, 7);
        let bytes = reg.encode();
        let back = EntityRegistry::decode(&bytes).expect("decode");
        assert_eq!(back.get(a).map(|e| e.data), Some(42));
        assert_eq!(back.len(), reg.len());
        assert_eq!(back.encode(), bytes, "encode must be deterministic");

        // Truncated or corrupted buffers refuse.
        assert!(EntityRegistry::decode(&bytes[..bytes.len() - 1]).is_none());
        // The kind byte lives at offset 9 of each record (record starts at
        // 16: id 8 bytes, then kind). Corrupt it.
        let mut bad = bytes.clone();
        bad[16 + 8] = 200; // unknown kind in the first record
        assert!(EntityRegistry::decode(&bad).is_none());
    }

    /// Interest states: Horizon entities are excluded, nearer entities
    /// map to tighter bands, ascending by id.
    #[test]
    fn p3d402_interest_states_exclude_horizon() {
        let mut reg = EntityRegistry::new();
        let near = reg.spawn(EntityKind::Villager, CellCoord { x: 0, y: 0, z: 0 }, 0);
        let far = reg.spawn(
            EntityKind::Animal,
            CellCoord { x: 5_000, y: 0, z: 5_000 },
            0,
        );
        let viewer = WorldPos::default();
        let state = reg.interest_state(viewer);
        let ids: Vec<EntityId> = state.iter().map(|(i, _)| *i).collect();
        assert_eq!(ids, vec![near], "horizon entity must be excluded");
        assert_eq!(state[0].1, crate::lod::LodLevel::Full);
        assert!(reg.get(far).is_some(), "excluded does not mean deleted");
    }
}
