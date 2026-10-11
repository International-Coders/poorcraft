//! P3D-603: gates, guards, laws, alarms, and faction access.
//!
//! The castle's defensive and legal systems: a gate that opens/closes,
//! laws defining what's forbidden, alarms that summon guards, and
//! faction-standing-gated access. All pure and deterministic.

use crate::coords::CellCoord;
use std::collections::BTreeMap;

/// Gate open/closed state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateState {
    pub open: bool,
}

impl Default for GateState {
    fn default() -> Self {
        GateState { open: true }
    }
}

/// What kind of action the law forbids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LawKind {
    Theft,
    Assault,
    Trespass,
}

impl LawKind {
    pub fn name(self) -> &'static str {
        match self {
            LawKind::Theft => "theft",
            LawKind::Assault => "assault",
            LawKind::Trespass => "trespass",
        }
    }
}

/// What happens when a law is broken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Punishment {
    Fine,
    Exile,
    Attack,
}

/// A law: forbids an action kind with a minimum standing threshold for
/// entry. Characters below the threshold cannot pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Law {
    pub kind: LawKind,
    /// Minimum disposition required to be exempt from this law's
    /// punishment (0 = everyone is punished).
    pub standing_threshold: i32,
    pub punishment: Punishment,
}

/// An active alarm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Alarm {
    pub raised_at: CellCoord,
    pub tick: u64,
    pub active: bool,
}

/// The castle's legal and defensive state.
#[derive(Clone, Debug, Default)]
pub struct CastleLaw {
    pub gate: GateState,
    pub laws: Vec<Law>,
    pub alarm: Option<Alarm>,
    /// Guard response radius in cells.
    pub guard_response_radius: i32,
}

impl CastleLaw {
    pub fn new(laws: Vec<Law>) -> Self {
        CastleLaw {
            gate: GateState { open: true },
            laws,
            alarm: None,
            guard_response_radius: 48,
        }
    }

    /// Toggle the gate.
    pub fn toggle_gate(&mut self) {
        self.gate.open = !self.gate.open;
    }

    /// Raise an alarm at a position. Only one active alarm at a time.
    pub fn raise_alarm(&mut self, at: CellCoord, tick: u64) {
        self.alarm = Some(Alarm {
            raised_at: at,
            tick,
            active: true,
        });
    }

    /// Clear the alarm (guards arrived).
    pub fn clear_alarm(&mut self) {
        self.alarm = None;
    }

    /// Is there an active alarm within the guard response radius of
    /// `guard_pos`?
    pub fn alarm_near(&self, guard_pos: CellCoord) -> bool {
        match &self.alarm {
            Some(a) if a.active => {
                let dx = (a.raised_at.x - guard_pos.x).abs();
                let dz = (a.raised_at.z - guard_pos.z).abs();
                dx <= self.guard_response_radius && dz <= self.guard_response_radius
            }
            _ => false,
        }
    }

    /// Faction access: is a character with `standing` allowed to enter?
    /// The gate must be open AND standing must meet all law thresholds.
    pub fn access_allowed(&self, standing: i32) -> bool {
        self.gate.open && self.laws.iter().all(|l| standing >= l.standing_threshold)
    }

    /// Which law (if any) does this standing violate?
    pub fn violated_law(&self, standing: i32) -> Option<&Law> {
        self.laws.iter().find(|l| standing < l.standing_threshold)
    }

    /// The punishment for a violated law.
    pub fn punishment_for(&self, standing: i32) -> Option<Punishment> {
        self.violated_law(standing).map(|l| l.punishment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn laws() -> Vec<Law> {
        vec![
            Law {
                kind: LawKind::Theft,
                standing_threshold: -20,
                punishment: Punishment::Fine,
            },
            Law {
                kind: LawKind::Assault,
                standing_threshold: 0,
                punishment: Punishment::Attack,
            },
            Law {
                kind: LawKind::Trespass,
                standing_threshold: -50,
                punishment: Punishment::Exile,
            },
        ]
    }

    fn castle() -> CastleLaw {
        CastleLaw::new(laws())
    }

    /// Gate toggles between open and closed.
    #[test]
    fn p3d603_gate_toggles() {
        let mut c = castle();
        assert!(c.gate.open, "gate starts open");
        c.toggle_gate();
        assert!(!c.gate.open);
        c.toggle_gate();
        assert!(c.gate.open);
    }

    /// Faction access: high standing enters, low standing is denied.
    #[test]
    fn p3d603_access_gated_by_standing() {
        let c = castle();
        assert!(c.access_allowed(50), "high standing enters");
        assert!(c.access_allowed(0), "zero standing meets assault threshold");
        assert!(!c.access_allowed(-30), "below assault threshold denied");
        assert!(!c.access_allowed(-60), "below trespass threshold denied");
    }

    /// Closed gate denies everyone regardless of standing.
    #[test]
    fn p3d603_closed_gate_denies_all() {
        let mut c = castle();
        c.toggle_gate();
        assert!(
            !c.access_allowed(100),
            "even max standing denied by closed gate"
        );
    }

    /// The violated law is identified correctly.
    #[test]
    fn p3d603_violated_law_identified() {
        let c = castle();
        assert!(
            c.violated_law(50).is_none(),
            "high standing violates nothing"
        );
        // Standing -25 violates Theft (threshold -20) first in law order.
        let v = c
            .violated_law(-25)
            .expect("standing -25 violates something");
        assert_eq!(v.kind, LawKind::Theft, "theft threshold -20 hit first");
        assert_eq!(v.punishment, Punishment::Fine);
    }

    /// Alarms: raise, check proximity, clear.
    #[test]
    fn p3d603_alarms_raise_check_and_clear() {
        let mut c = castle();
        let alarm_pos = CellCoord { x: 10, y: 0, z: 10 };
        c.raise_alarm(alarm_pos, 100);
        assert!(
            c.alarm_near(CellCoord { x: 15, y: 0, z: 15 }),
            "guard nearby sees alarm"
        );
        assert!(
            !c.alarm_near(CellCoord {
                x: 100,
                y: 0,
                z: 100
            }),
            "far guard doesn't see alarm"
        );
        c.clear_alarm();
        assert!(
            !c.alarm_near(CellCoord { x: 10, y: 0, z: 10 }),
            "cleared alarm invisible"
        );
    }
}
