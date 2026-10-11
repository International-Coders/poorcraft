//! VS V6: forge + quest progress beside the player pack (framed).

use crate::framing::{frame, unframe};
use crate::paths::world_root;
use crate::store::{write_atomic, LoadError};
use pc3d_core::{FormatHeader, SupportedVersions};
use pc3d_world::forge::Forge;
use pc3d_world::quest::Quest;
use std::path::Path;

const MAGIC: u8 = 1;

/// Optional forge + quest list for one world slot.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SessionExtras {
    pub forge: Option<Forge>,
    pub quests: Vec<Quest>,
    /// BETA-0.2 W3.1: the chosen career path (None = the fork awaits).
    /// Encoded as a trailing byte — old builds ignore it, new builds
    /// default it to None on old saves (compatible both ways).
    pub career: Option<u8>,
}

impl SessionExtras {
    fn encode(&self) -> Vec<u8> {
        let mut b = vec![MAGIC];
        match &self.forge {
            Some(f) => {
                b.push(1);
                b.extend_from_slice(&f.encode());
            }
            None => b.push(0),
        }
        b.extend_from_slice(&(self.quests.len() as u16).to_le_bytes());
        for q in &self.quests {
            let qb = q.encode();
            b.extend_from_slice(&(qb.len() as u16).to_le_bytes());
            b.extend_from_slice(&qb);
        }
        // The career trailer (W3.1): 0 = unchosen, 1/2 = the path's code.
        b.push(self.career.unwrap_or(0));
        b
    }

    fn decode(bytes: &[u8]) -> Result<SessionExtras, LoadError> {
        if bytes.is_empty() || bytes[0] != MAGIC {
            return Err(LoadError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "session extras magic",
            )));
        }
        let mut off = 1;
        if off >= bytes.len() {
            return Err(LoadError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "session truncated",
            )));
        }
        let forge = if bytes[off] == 1 {
            off += 1;
            if off + 42 > bytes.len() {
                return Err(LoadError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "forge truncated",
                )));
            }
            let f = Forge::decode(&bytes[off..off + 42]).ok_or_else(|| {
                LoadError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "forge decode",
                ))
            })?;
            off += 42;
            Some(f)
        } else {
            off += 1;
            None
        };
        if off + 2 > bytes.len() {
            return Err(LoadError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "quest count truncated",
            )));
        }
        let n = u16::from_le_bytes(bytes[off..off + 2].try_into().unwrap()) as usize;
        off += 2;
        let mut quests = Vec::with_capacity(n);
        for _ in 0..n {
            if off + 2 > bytes.len() {
                return Err(LoadError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "quest len truncated",
                )));
            }
            let len = u16::from_le_bytes(bytes[off..off + 2].try_into().unwrap()) as usize;
            off += 2;
            if off + len > bytes.len() {
                return Err(LoadError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "quest body truncated",
                )));
            }
            let (q, used) = Quest::decode(&bytes[off..off + len]).ok_or_else(|| {
                LoadError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "quest decode",
                ))
            })?;
            if used != len {
                return Err(LoadError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "quest length mismatch",
                )));
            }
            off += len;
            quests.push(q);
        }
        // The career trailer (W3.1): present in new saves, absent in old
        // ones — absence is None, never an error (compatible both ways).
        let career = if off < bytes.len() {
            let c = bytes[off];
            if c == 0 { None } else { Some(c) }
        } else {
            None
        };
        Ok(SessionExtras { forge, quests, career })
    }
}

pub fn save_session(
    save_root: &Path,
    world_name: &str,
    extras: &SessionExtras,
    supported: &SupportedVersions,
) -> Result<(), LoadError> {
    let header = FormatHeader {
        save: supported.save,
        ..FormatHeader::current()
    };
    let bytes = frame(&header, &extras.encode());
    let path = world_root(save_root, world_name).join("session.bin");
    write_atomic(&path, &bytes)?;
    Ok(())
}

/// Missing file → empty extras (pre-VS saves).
pub fn load_session(
    save_root: &Path,
    world_name: &str,
    supported: &SupportedVersions,
) -> Result<SessionExtras, LoadError> {
    let path = world_root(save_root, world_name).join("session.bin");
    if !path.exists() {
        return Ok(SessionExtras::default());
    }
    let bytes = std::fs::read(path)?;
    let payload = unframe(&bytes, supported)?;
    SessionExtras::decode(&payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc3d_world::coords::CellCoord;
    use pc3d_world::npc::Role;
    use pc3d_world::quest::{QuestKind, QuestState};

    #[test]
    fn session_forge_and_quests_round_trip() {
        let root = tempfile::tempdir().expect("tmp");
        let sup = SupportedVersions::epoch1();
        let mut forge = Forge::default();
        forge.load_fuel(1000, 500);
        forge.load_ore(1);
        forge.tick();
        let quests = vec![Quest {
            id: 42,
            title: "ignored".into(),
            giver_role: Role::Farmer,
            giver_cell: CellCoord { x: 1, y: 0, z: 2 },
            kind: QuestKind::Greet { count: 3 },
            state: QuestState::Active,
            progress: 1,
            reward: 15,
        }];
        let extras = SessionExtras {
            forge: Some(forge.clone()),
            quests: quests.clone(),
            career: Some(2),
        };
        save_session(root.path(), "w", &extras, &sup).expect("save");
        let back = load_session(root.path(), "w", &sup).expect("load");
        assert_eq!(back.forge.as_ref().unwrap().ore, forge.ore);
        assert_eq!(back.forge.as_ref().unwrap().fuel_milli, forge.fuel_milli);
        assert_eq!(back.quests.len(), 1);
        assert_eq!(back.quests[0].id, 42);
        assert_eq!(back.quests[0].progress, 1);
        assert_eq!(back.quests[0].state, QuestState::Active);
        assert_eq!(back.career, Some(2), "the career trailer round-trips");
        assert_eq!(back.quests[0].kind, QuestKind::Greet { count: 3 });
    }

    #[test]
    fn missing_session_file_is_empty() {
        let root = tempfile::tempdir().expect("tmp");
        let back = load_session(root.path(), "missing", &SupportedVersions::epoch1()).unwrap();
        assert!(back.forge.is_none());
        assert!(back.quests.is_empty());
    }
}
