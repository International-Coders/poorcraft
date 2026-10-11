//! P3D slice: the player state store — position/look + pack + onboarding
//! saved beside the world meta and build snapshots, through the same framing law.

use crate::framing::{frame, unframe};
use crate::paths::world_root;
use crate::store::{write_atomic, LoadError};
use pc3d_core::{FormatHeader, SupportedVersions};
use pc3d_world::items::{Inventory, ItemId, ItemStack};
use std::path::Path;

/// The first-person player: feet position (meters), look angles, pack, onboarding.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerState {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub inventory: Inventory,
    /// Bitmask from `Onboarding::encode` (VS V2 milestones).
    pub onboarding_mask: u8,
}

impl PlayerState {
    fn encode(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(20 + 2 + self.inventory.slots.len() * 7);
        for v in self.pos {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&self.yaw.to_le_bytes());
        b.extend_from_slice(&self.pitch.to_le_bytes());
        // Trailer (absent in epoch pose-only saves): mask + stacks.
        b.push(self.onboarding_mask);
        let stacks: Vec<&ItemStack> = self
            .inventory
            .slots
            .iter()
            .filter_map(|s| s.as_ref())
            .collect();
        b.push(stacks.len() as u8);
        for s in stacks {
            b.extend_from_slice(&s.item.0.to_le_bytes());
            b.extend_from_slice(&s.count.to_le_bytes());
        }
        b
    }

    fn decode(bytes: &[u8]) -> Result<PlayerState, LoadError> {
        if bytes.len() < 20 {
            return Err(LoadError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "player payload too short",
            )));
        }
        let f = |i: usize| f32::from_le_bytes(bytes[i..i + 4].try_into().expect("4"));
        let mut inventory = Inventory::new(12);
        let mut onboarding_mask = 0u8;
        if bytes.len() > 20 {
            onboarding_mask = bytes[20];
            if bytes.len() < 22 {
                return Err(LoadError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "player pack trailer truncated",
                )));
            }
            let n = bytes[21] as usize;
            let mut off = 22;
            for _ in 0..n {
                if off + 6 > bytes.len() {
                    return Err(LoadError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "player pack stack truncated",
                    )));
                }
                let id = u16::from_le_bytes(bytes[off..off + 2].try_into().expect("2"));
                let count = u32::from_le_bytes(bytes[off + 2..off + 6].try_into().expect("4"));
                let _ = inventory.add(ItemId(id), count);
                off += 6;
            }
        }
        Ok(PlayerState {
            pos: [f(0), f(4), f(8)],
            yaw: f(12),
            pitch: f(16),
            inventory,
            onboarding_mask,
        })
    }
}

pub fn save_player(
    save_root: &Path,
    world_name: &str,
    state: &PlayerState,
    supported: &SupportedVersions,
) -> Result<(), LoadError> {
    let header = FormatHeader {
        save: supported.save,
        ..FormatHeader::current()
    };
    let bytes = frame(&header, &state.encode());
    let path = world_root(save_root, world_name).join("player.bin");
    write_atomic(&path, &bytes)?;
    Ok(())
}

pub fn load_player(
    save_root: &Path,
    world_name: &str,
    supported: &SupportedVersions,
) -> Result<PlayerState, LoadError> {
    let path = world_root(save_root, world_name).join("player.bin");
    let bytes = std::fs::read(path)?;
    let payload = unframe(&bytes, supported)?;
    PlayerState::decode(&payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_state_round_trips_on_disk() {
        let root = tempfile::tempdir().expect("tmp");
        let sup = SupportedVersions::epoch1();
        let mut inv = Inventory::new(12);
        inv.add(ItemId(1), 3);
        inv.add(ItemId(10), 1);
        let s = PlayerState {
            pos: [12.5, -3.25, 88.0],
            yaw: 1.25,
            pitch: -0.1,
            inventory: inv,
            onboarding_mask: 0b0101,
        };
        save_player(root.path(), "slice", &s, &sup).expect("save");
        let back = load_player(root.path(), "slice", &sup).expect("load");
        assert_eq!(s.pos, back.pos);
        assert_eq!(s.yaw, back.yaw);
        assert_eq!(s.pitch, back.pitch);
        assert_eq!(s.onboarding_mask, back.onboarding_mask);
        assert_eq!(back.inventory.count(ItemId(1)), 3);
        assert_eq!(back.inventory.count(ItemId(10)), 1);
        assert!(PlayerState::decode(&[0u8; 19]).is_err());
    }

    #[test]
    fn pose_only_payload_still_loads() {
        // Pre-VS saves were exactly 20 bytes of pose — must not refuse.
        let mut bare = Vec::new();
        for v in [1.0f32, 2.0, 3.0, 0.5, -0.25] {
            bare.extend_from_slice(&v.to_le_bytes());
        }
        let p = PlayerState::decode(&bare).expect("legacy");
        assert_eq!(p.pos, [1.0, 2.0, 3.0]);
        assert_eq!(p.inventory.count(ItemId(1)), 0);
        assert_eq!(p.onboarding_mask, 0);
    }
}
