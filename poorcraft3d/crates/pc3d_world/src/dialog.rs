//! The NPC talk authority (pure).
//!
//! `talk_with` derives a villager's line from their REAL brain — role,
//! current activity (the intent state machine), and needs — with the
//! speaker's name a deterministic function of their home cell. No UI, no
//! rendering: the same brain always says the same thing at the same
//! moment, so the observatory route and the live game agree by
//! construction. Original POORCRAFT villager flavor only.

use crate::coords::CellCoord;
use crate::npc::{Activity, NpcBrain, Role};

/// One spoken line with its speaker context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialogLine {
    pub speaker: String,
    pub role: &'static str,
    pub activity: &'static str,
    pub text: String,
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Original-flavor villager name parts (the settlement's own people).
const NAME_FIRST: &[&str] = &[
    "Bram", "Edda", "Torvald", "Maren", "Oswin", "Petra", "Harald", "Gilda",
    "Rowan", "Sif", "Dunstan", "Estrid", "Calder", "Ylva", "Merrick", "Thora",
];
const NAME_LAST: &[&str] = &[
    "Stonehand", "Emberworth", "Longbarrow", "Quarrylee", "Fernbrook",
    "Ashdown", "Millgate", "Thatcher", "Oldford", "Brightwell",
];

/// A villager's name, deterministic from their home cell: the same
/// villager is the same person in every save, capture, and route.
pub fn villager_name(home: CellCoord) -> String {
    let h = fnv1a(&[
        (home.x as u32).to_le_bytes(),
        (home.y as u32).to_le_bytes(),
        (home.z as u32).to_le_bytes(),
    ]
    .concat());
    let f = NAME_FIRST[(h % NAME_FIRST.len() as u64) as usize];
    let l = NAME_LAST[((h >> 16) % NAME_LAST.len() as u64) as usize];
    format!("{f} {l}")
}

fn role_str(r: Role) -> &'static str {
    match r {
        Role::Farmer => "FARMER",
        Role::Fisher => "FISHER",
        Role::Builder => "BUILDER",
        Role::Guard => "GUARD",
    }
}

fn activity_str(a: Activity) -> &'static str {
    match a {
        Activity::Idle => "IDLE",
        Activity::Walking => "WALKING",
        Activity::Farming => "FARMING",
        Activity::Fishing => "FISHING",
        Activity::Building => "BUILDING",
        Activity::Guarding => "GUARDING",
        Activity::Sleeping => "SLEEPING",
    }
}

/// Working lines per role — the villager speaks their trade.
fn work_line(role: Role, h: u64) -> &'static str {
    let pick = |v: &'static [&'static str]| -> &'static str {
        v[(h % v.len() as u64) as usize]
    };
    match role {
        Role::Farmer => pick(&[
            "The rows want water before the sun climbs much higher.",
            "Good soil this season. The village will eat well.",
            "Back already? The fields don't weed themselves.",
        ]),
        Role::Fisher => pick(&[
            "River was generous at dawn. Look at the catch.",
            "They bite when the water settles. Patience, then supper.",
            "A quiet bank is a good bank, friend.",
        ]),
        Role::Builder => pick(&[
            "Another course of stone and the wall holds the winter out.",
            "Mind the mortar — it needs a day yet.",
            "We build it once, we build it right.",
        ]),
        Role::Guard => pick(&[
            "All's quiet on my watch. Keep it that way.",
            "The wall's solid and my eyes are open.",
            "Travelers pass by the gate, nothing worse today.",
        ]),
    }
}

/// What the villager says right now — derived from the LIVE brain.
pub fn talk_with(brain: &NpcBrain) -> DialogLine {
    let name = villager_name(brain.home);
    let h = fnv1a(name.as_bytes());
    let activity = brain.activity();
    let text = match activity {
        Activity::Sleeping => {
            let v = [
                "…mm. Five more minutes…",
                "…the turnip cart… no, the OTHER turnip cart…",
            ];
            v[(h % v.len() as u64) as usize].to_string()
        }
        Activity::Walking => {
            let v = [
                "On my way — the day doesn't wait.",
                "Heading over to the worksite. Walk with me.",
                "Errands, always errands. Village runs on them.",
            ];
            v[(h % v.len() as u64) as usize].to_string()
        }
        Activity::Idle => {
            let v = [
                "Fine day for it, whatever it is.",
                "Just taking a breath. The plaza's peaceful.",
                "Ask at the workshop if you need hands — mine are resting.",
            ];
            v[(h % v.len() as u64) as usize].to_string()
        }
        working => work_line(brain.role, h ^ (activity_str(working).len() as u64))
            .to_string(),
    };
    DialogLine {
        speaker: name,
        role: role_str(brain.role),
        activity: activity_str(activity),
        text,
    }
}

// ---------------------------------------------------------------------------
// Tests: determinism, state discrimination, flavor coverage
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn brain(role: Role, intent: crate::npc::Intent, home: (i32, i32, i32)) -> NpcBrain {
        let home = CellCoord { x: home.0, y: home.1, z: home.2 };
        NpcBrain {
            role,
            home,
            work_site: home,
            pos: home,
            needs: crate::npc::Needs {
                hunger: 0,
                energy: 100,
                hunger_f: 0.0,
                energy_f: 100.0,
            },
            intent,
        }
    }

    #[test]
    fn talk_is_deterministic_and_state_aware() {
        use crate::npc::Intent;
        let b = brain(Role::Farmer, Intent::Working { site: CellCoord { x: 1, y: 0, z: 1 } }, (3, 0, 4));
        let a = talk_with(&b);
        let c = talk_with(&b);
        assert_eq!(a, c, "same brain, same moment: same line");
        assert_eq!(a.activity, "FARMING");
        // A different brain state says a different class of thing.
        let sleeping = talk_with(&brain(Role::Farmer, Intent::Sleeping, (3, 0, 4)));
        assert_eq!(sleeping.activity, "SLEEPING");
        assert_ne!(a.activity, sleeping.activity);
    }

    #[test]
    fn every_role_and_activity_speaks() {
        use crate::npc::Intent;
        let home = CellCoord { x: 2, y: 0, z: -3 };
        for role in [Role::Farmer, Role::Fisher, Role::Builder, Role::Guard] {
            for intent in [
                Intent::Idle,
                Intent::Sleeping,
                Intent::Walking { path: vec![], leg: 0 },
                Intent::Working { site: home },
            ] {
                let line = talk_with(&brain(role, intent.clone(), (2, 0, -3)));
                assert!(!line.speaker.is_empty());
                assert!(!line.text.is_empty(), "{role:?}/{intent:?} needs a line");
                assert!(!line.role.is_empty());
            }
        }
    }

    #[test]
    fn names_are_stable_and_varied() {
        let a = villager_name(CellCoord { x: 3, y: 0, z: 4 });
        let b = villager_name(CellCoord { x: 3, y: 0, z: 4 });
        assert_eq!(a, b, "same home = same person");
        let c = villager_name(CellCoord { x: -17, y: 0, z: 9 });
        assert_ne!(a, c, "different homes are different people");
        // A sample of homes yields several distinct people.
        let mut people = std::collections::BTreeSet::new();
        for x in 0..32i32 {
            people.insert(villager_name(CellCoord { x, y: 0, z: x * 3 }));
        }
        assert!(people.len() >= 16, "name variety ({})", people.len());
    }
}
