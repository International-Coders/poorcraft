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
    /// N06: tool wear (uses left per carried tool). Trailing blob after
    /// the career byte — pre-N06 saves decode to fresh tools.
    pub tool_wear: pc3d_world::items::ToolWear,
    /// N08: the river's fish stocks. Trailing blob after the wear blob —
    /// pre-N08 saves decode to FULL stocks (a living river, not an
    /// empty one: fish are not player-made).
    pub fish_stocks: Option<pc3d_world::hydro::FishStocks>,
    /// N11: the campfire (fuel/embers/lifetime) + its world position.
    /// Trailing blob after the fish blob — pre-N11 saves decode to None
    /// (no fire lit yet).
    pub campfire: Option<(pc3d_world::campfire::Campfire, [f32; 2])>,
}

impl SessionExtras {
    fn encode(&self) -> Vec<u8> {
        // The N11 campfire rides FIRST (marker 0xF0 none / 0xF1 some) —
        // a byte no older field ever writes (forge is 0/1, quest counts
        // are u16 LE), so pre-N11 saves' forge byte can never read as a
        // campfire and the fish blob can never swallow one.
        let mut b = vec![MAGIC];
        match &self.campfire {
            Some((c, pos)) => {
                b.push(0xF1);
                b.extend_from_slice(&c.encode());
                b.extend_from_slice(&pos[0].to_le_bytes());
                b.extend_from_slice(&pos[1].to_le_bytes());
            }
            None => b.push(0xF0),
        }
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
        // The N06 wear trailer: the blob after the career byte.
        b.extend_from_slice(&self.tool_wear.encode());
        // The N08 fish trailer: the blob after the wear blob.
        if let Some(f) = &self.fish_stocks {
            b.extend_from_slice(&f.encode());
        }
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
        // The N11 campfire header rides FIRST (0xF0 none / 0xF1 some) —
        // a byte range nothing older writes. A pre-N11 save's forge byte
        // (0/1) fails both markers and the campfire decodes to None
        // WITHOUT consuming the byte (the forge branch below reads it).
        let campfire = if off < bytes.len() && bytes[off] == 0xF1 {
            off += 1;
            if off + 24 + 8 > bytes.len() {
                return Err(LoadError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "campfire truncated",
                )));
            }
            let c = pc3d_world::campfire::Campfire::decode(&bytes[off..off + 24])
                .ok_or_else(|| {
                    LoadError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "campfire decode",
                    ))
                })?;
            off += 24;
            let x = f32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
            let z = f32::from_le_bytes(bytes[off + 4..off + 8].try_into().unwrap());
            off += 8;
            Some((c, [x, z]))
        } else {
            if off < bytes.len() && bytes[off] == 0xF0 {
                off += 1; // N11 save with no fire yet: consume the marker
            }
            None
        };
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
        if off < bytes.len() {
            off += 1;
        }
        // The N06 wear trailer: after the career byte; absent in pre-N06
        // saves = fresh tools (compatible both ways).
        let tool_wear = pc3d_world::items::ToolWear::decode(&bytes[off.min(bytes.len())..])
            .unwrap_or_default();
        // The wear blob is 1 + n*6 bytes (mirrors encode) — advance the
        // cursor past it or the fish trailer reads the wear map's bytes.
        off = (off + 1 + tool_wear.uses_left.len() * 6).min(bytes.len());
        // The N08 fish trailer: after the wear blob; absent in pre-N08
        // saves = None (the caller seeds FULL stocks — a living river).
        let fish_stocks =
            pc3d_world::hydro::FishStocks::decode(&bytes[off.min(bytes.len())..]);
        Ok(SessionExtras { forge, quests, career, tool_wear, fish_stocks, campfire })
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
        let mut wear = pc3d_world::items::ToolWear::default();
        wear.wear_use(pc3d_world::items::ItemId(12)); // wood_pick: 11 left
        let mut stocks = pc3d_world::hydro::FishStocks::default();
        stocks.stock.insert((-24, -5), 17);
        let extras = SessionExtras {
            forge: Some(forge.clone()),
            quests: quests.clone(),
            career: Some(2),
            tool_wear: wear.clone(),
            fish_stocks: Some(stocks.clone()),
            campfire: None,
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
        assert_eq!(back.tool_wear, wear, "the wear trailer round-trips");
        assert_eq!(
            back.tool_wear.uses_left(pc3d_world::items::ItemId(12)),
            Some(11)
        );
        assert_eq!(back.quests[0].kind, QuestKind::Greet { count: 3 });
        assert_eq!(
            back.fish_stocks,
            Some(stocks),
            "the fish trailer round-trips"
        );
    }

    /// N11: the campfire trailer round-trips (state + position), and a
    /// pre-N11 save (no trailer) loads with None — no fire lit yet.
    #[test]
    fn p3dn11_campfire_trailer_round_trips() {
        let tmp = tempfile::tempdir().expect("tmp");
        let sup = SupportedVersions::epoch1();
        let mut fire = pc3d_world::campfire::Campfire::default();
        assert!(fire.add_wood());
        fire.tick();
        let extras = SessionExtras {
            forge: None,
            quests: vec![],
            career: None,
            tool_wear: Default::default(),
            fish_stocks: None,
            campfire: Some((fire.clone(), [140.5, 96.25])),
        };
        save_session(tmp.path(), "w", &extras, &sup).expect("save");
        let back = load_session(tmp.path(), "w", &sup).expect("load");
        assert_eq!(
            back.campfire,
            Some((fire, [140.5, 96.25])),
            "campfire state + position round-trip"
        );
    }

    #[test]
    fn p3dn11_old_save_without_campfire_loads_none() {
        let tmp = tempfile::tempdir().expect("tmp");
        let sup = SupportedVersions::epoch1();
        // A pre-N11 payload: MAGIC + forge byte 0 + 0 quests + career 0
        // + fresh wear + no fish + no campfire byte.
        let old = vec![1u8, 0, 0, 0, 0];
        let old = crate::framing::frame(&FormatHeader::current(), &old);
        crate::store::write_atomic(
            &crate::paths::world_root(tmp.path(), "w").join("session.bin"),
            &old,
        )
        .expect("write");
        let back = load_session(tmp.path(), "w", &sup).expect("load");
        assert!(back.campfire.is_none(), "absence = no fire, never an error");
    }

    /// N08: a pre-N08 save (no fish trailer) loads with None — the
    /// caller seeds FULL stocks (a living river, never an empty one).
    #[test]
    fn p3dn08_old_save_without_fish_loads_none() {
        let tmp = tempfile::tempdir().expect("tmp");
        let sup = SupportedVersions::epoch1();
        // Hand-encode: MAGIC + no forge + 0 quests + career 0 (no blobs).
        let old = vec![1u8, 0, 0, 0, 0];
        // load_session unframes, so the legacy payload ships inside a real
        // P3D frame (a naked payload would be refused before decode).
        let old = crate::framing::frame(&FormatHeader::current(), &old);
        crate::store::write_atomic(
            &crate::paths::world_root(tmp.path(), "w").join("session.bin"),
            &old,
        )
        .expect("write");
        let back = load_session(tmp.path(), "w", &sup).expect("load");
        assert!(back.fish_stocks.is_none(), "absence = None, never an error");
    }

    /// N06: a pre-N06 save (no wear trailer) loads with FRESH tools —
    /// absence is honest default, never an error.
    #[test]
    fn p3dn06_old_save_without_wear_loads_fresh_tools() {
        let tmp = tempfile::tempdir().expect("tmp");
        let sup = SupportedVersions::epoch1();
        // Hand-encode a pre-N06 payload: MAGIC + no forge + 0 quests + career 0.
        let old = vec![1u8, 0, 0, 0, 0];
        let old = crate::framing::frame(&FormatHeader::current(), &old);
        crate::store::write_atomic(&crate::paths::world_root(tmp.path(), "w").join("session.bin"), &old)
            .expect("write");
        let back = load_session(tmp.path(), "w", &sup).expect("load");
        assert_eq!(back.tool_wear, pc3d_world::items::ToolWear::default());
    }

    #[test]
    fn missing_session_file_is_empty() {
        let root = tempfile::tempdir().expect("tmp");
        let back = load_session(root.path(), "missing", &SupportedVersions::epoch1()).unwrap();
        assert!(back.forge.is_none());
        assert!(back.quests.is_empty());
    }
}
