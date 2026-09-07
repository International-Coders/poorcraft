//! P3D slice: the player state store — position/look saved beside the
//! world meta and build snapshots, through the same framing law.

use crate::framing::{frame, unframe};
use crate::paths::world_root;
use crate::store::{write_atomic, LoadError};
use pc3d_core::{FormatHeader, SupportedVersions};
use std::path::Path;

/// The first-person player: feet position (meters), look angles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerState {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
}

impl PlayerState {
    fn encode(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(20);
        for v in self.pos {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&self.yaw.to_le_bytes());
        b.extend_from_slice(&self.pitch.to_le_bytes());
        b
    }
    fn decode(bytes: &[u8]) -> Result<PlayerState, LoadError> {
        if bytes.len() != 20 {
            return Err(LoadError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "player payload must be 20 bytes",
            )));
        }
        let f = |i: usize| f32::from_le_bytes(bytes[i..i + 4].try_into().expect("4"));
        Ok(PlayerState {
            pos: [f(0), f(4), f(8)],
            yaw: f(12),
            pitch: f(16),
        })
    }
}

pub fn save_player(
    save_root: &Path,
    world_name: &str,
    state: &PlayerState,
    supported: &SupportedVersions,
) -> Result<(), LoadError> {
    let header = FormatHeader { save: supported.save, ..FormatHeader::current() };
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
        let s = PlayerState { pos: [12.5, -3.25, 88.0], yaw: 1.25, pitch: -0.1 };
        save_player(root.path(), "slice", &s, &sup).expect("save");
        let back = load_player(root.path(), "slice", &sup).expect("load");
        assert_eq!(s, back);
        // Wrong-length payload refuses.
        assert!(PlayerState::decode(&[0u8; 19]).is_err());
    }
}
