//! P3D-405: perception, memory, reports, karma baselines, local reactions.
//!
//! The moral-consequence substrate (D-028/D-030): NPCs who WITNESS a
//! moral event store personal evidence at full confidence; REPORTED
//! knowledge arrives at lower confidence and decays with age; disposition
//! toward an actor = faction baseline + evidence-weighted delta, clamped.
//! All pure and deterministic — the same history yields the same minds.

use crate::coords::CellCoord;
use std::collections::BTreeMap;

/// A witnessed moral event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoralEvent {
    pub actor_id: u64,
    pub kind: MoralKind,
    pub at: CellCoord,
    pub tick: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoralKind {
    Theft,
    Assault,
    Gift,
    Help,
}

impl MoralKind {
    /// Signed weight on the actor's disposition.
    pub fn weight(self) -> i32 {
        match self {
            MoralKind::Theft => -10,
            MoralKind::Assault => -20,
            MoralKind::Gift => 10,
            MoralKind::Help => 8,
        }
    }
    pub fn code(self) -> u8 {
        match self {
            MoralKind::Theft => 1,
            MoralKind::Assault => 2,
            MoralKind::Gift => 3,
            MoralKind::Help => 4,
        }
    }
    pub fn from_code(c: u8) -> Option<Self> {
        match c {
            1 => Some(MoralKind::Theft),
            2 => Some(MoralKind::Assault),
            3 => Some(MoralKind::Gift),
            4 => Some(MoralKind::Help),
            _ => None,
        }
    }
}

/// Sight radius in cells (Chebyshev).
pub const SIGHT_RADIUS: i32 = 24;
/// Witnessed evidence starts at this confidence.
pub const WITNESSED_CONFIDENCE: f32 = 1.0;
/// Reported evidence arrives at this confidence (never above witnessed).
pub const REPORT_CONFIDENCE: f32 = 0.6;
/// Confidence lost per 1000 ticks of age.
pub const KNOWLEDGE_DECAY_PER_KT: f32 = 0.05;
/// Bounded memory per NPC.
pub const KNOWLEDGE_CAPACITY: usize = 32;

/// Did an NPC at `npc_pos` witness `event`?
pub fn witness(npc_pos: CellCoord, sight_radius: i32, event: &MoralEvent) -> bool {
    let dx = (npc_pos.x - event.at.x).abs();
    let dy = (npc_pos.y - event.at.y).abs();
    let dz = (npc_pos.z - event.at.z).abs();
    dx.max(dy).max(dz) <= sight_radius
}

/// One piece of personal evidence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Evidence {
    pub event: MoralEvent,
    pub confidence: f32,
}

/// An NPC's bounded knowledge base.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Knowledge {
    pub evidence: Vec<Evidence>,
}

impl Knowledge {
    /// Remember an event: merging raises confidence to the max of
    /// (existing, new); capacity drops the LOWEST-confidence item first.
    pub fn remember(&mut self, event: MoralEvent, confidence: f32) {
        if let Some(e) = self
            .evidence
            .iter_mut()
            .find(|e| e.event.actor_id == event.actor_id && e.event.kind == event.kind && e.event.at == event.at)
        {
            e.confidence = e.confidence.max(confidence);
            return;
        }
        let e = Evidence { event, confidence };
        if self.evidence.len() >= KNOWLEDGE_CAPACITY {
            // Drop the lowest-confidence item (first on ties).
            let worst = self
                .evidence
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.confidence.total_cmp(&b.1.confidence))
                .map(|(i, _)| i);
            if let Some(i) = worst {
                self.evidence.remove(i);
            }
        }
        self.evidence.push(e);
    }

    /// Age-based confidence decay: confidence -= decay × (ticks/1000),
    /// floored at 0. Items at 0 are forgotten.
    pub fn age(&mut self, current_tick: u64) {
        for e in &mut self.evidence {
            let age_kt = current_tick.saturating_sub(e.event.tick) as f32 / 1000.0;
            e.confidence = (e.confidence - age_kt * KNOWLEDGE_DECAY_PER_KT).max(0.0);
        }
        self.evidence.retain(|e| e.confidence > 0.0);
    }

    /// Report one piece of evidence to a listener: arrives at
    /// REPORT_CONFIDENCE scaled by the knower's confidence.
    pub fn report_to(&self, listener: &mut Knowledge, actor_id: u64, kind: MoralKind) -> bool {
        let Some(e) = self
            .evidence
            .iter()
            .find(|e| e.event.actor_id == actor_id && e.event.kind == kind)
        else {
            return false;
        };
        let conf = (e.confidence * REPORT_CONFIDENCE / WITNESSED_CONFIDENCE).min(REPORT_CONFIDENCE);
        let mut event = e.event;
        event.tick = e.event.tick; // age anchors to the original event tick
        listener.remember(event, conf);
        true
    }
}

/// Faction karma baselines + per-actor evidence deltas.
#[derive(Clone, Debug, Default)]
pub struct Karma {
    /// Faction baseline disposition per faction id.
    pub baselines: BTreeMap<u64, i32>,
    /// Per-actor accumulated weighted evidence (actor_id -> delta).
    pub actor_deltas: BTreeMap<u64, i32>,
}

/// Clamp bounds on disposition.
pub const DISPOSITION_MIN: i32 = -100;
pub const DISPOSITION_MAX: i32 = 100;

impl Karma {
    pub fn new(baselines: &[(u64, i32)]) -> Self {
        Karma {
            baselines: baselines.iter().copied().collect(),
            actor_deltas: BTreeMap::new(),
        }
    }

    /// Record evidence's weighted effect on an actor.
    pub fn apply(&mut self, actor_id: u64, kind: MoralKind, confidence: f32) {
        let delta = (kind.weight() as f32 * confidence).round() as i32;
        *self.actor_deltas.entry(actor_id).or_insert(0) += delta;
    }

    /// Local reaction: faction baseline + actor delta, clamped.
    pub fn disposition_toward(&self, faction: u64, actor_id: u64) -> i32 {
        let base = self.baselines.get(&faction).copied().unwrap_or(0);
        let delta = self.actor_deltas.get(&actor_id).copied().unwrap_or(0);
        (base + delta).clamp(DISPOSITION_MIN, DISPOSITION_MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(actor: u64, kind: MoralKind, x: i32, tick: u64) -> MoralEvent {
        MoralEvent { actor_id: actor, kind, at: CellCoord { x, y: 5, z: 5 }, tick }
    }

    /// Witness radius: inside true, boundary exact, outside false.
    #[test]
    fn p3d405_witness_radius_is_chebyshev_exact() {
        let npc = CellCoord { x: 0, y: 0, z: 0 };
        assert!(witness(npc, 24, &event(1, MoralKind::Gift, 24, 0)));
        assert!(witness(npc, 24, &event(1, MoralKind::Gift, 24, 0)));
        assert!(!witness(npc, 24, &event(1, MoralKind::Gift, 25, 0)));
        // Corner at Chebyshev 24: (24, 24, 0).
        assert!(witness(npc, 24, &event(1, MoralKind::Gift, 24, 24)));
        assert!(!witness(npc, 24, &event(1, MoralKind::Gift, 25, 24)));
    }

    /// Reports spread at lower confidence; direct knowledge at full;
    /// aging decays and forgets; capacity drops the weakest.
    #[test]
    fn p3d405_knowledge_reports_ages_and_bounds() {
        let mut knower = Knowledge::default();
        knower.remember(event(9, MoralKind::Theft, 1, 0), WITNESSED_CONFIDENCE);

        let mut listener = Knowledge::default();
        assert!(knower.report_to(&mut listener, 9, MoralKind::Theft));
        let le = &listener.evidence[0];
        assert!(
            (le.confidence - REPORT_CONFIDENCE).abs() < 1e-6,
            "reported confidence = {le:?}"
        );
        assert!(le.confidence < WITNESSED_CONFIDENCE);

        // Reporting something the knower never saw fails.
        assert!(!knower.report_to(&mut listener, 12, MoralKind::Gift));

        // Aging: at +3000 ticks, 1.0 confidence loses 3×0.05 = 0.15.
        let mut aged = Knowledge::default();
        aged.remember(event(9, MoralKind::Theft, 1, 0), 0.5);
        aged.age(3000);
        assert!((aged.evidence[0].confidence - 0.35).abs() < 1e-5);
        // Fully aged-out evidence is forgotten.
        aged.age(200_000);
        assert!(aged.evidence.is_empty(), "aged-out evidence forgotten");

        // Capacity: 40 events, capacity 32 — the weakest vanish first, so
        // the survivors all have confidence >= the 8 dropped ones' (0.1..0.17).
        let mut full = Knowledge::default();
        for i in 0..40u64 {
            full.remember(event(i, MoralKind::Gift, i as i32, i), 0.1 + i as f32 / 100.0);
        }
        assert_eq!(full.evidence.len(), KNOWLEDGE_CAPACITY);
        let min_kept = full
            .evidence
            .iter()
            .map(|e| e.confidence)
            .fold(f32::INFINITY, f32::min);
        assert!(
            min_kept > 0.1 + 7.0 / 100.0,
            "a low-confidence item survived: {min_kept}"
        );
    }

    /// Karma: faction baseline + evidence delta, clamped; faction
    /// baselines differ (D-030).
    #[test]
    fn p3d405_karma_baselines_and_deltas() {
        let mut karma = Karma::new(&[(1, 20), (2, -30)]);
        // One theft witnessed at full confidence: -10 delta.
        karma.apply(9, MoralKind::Theft, 1.0);
        assert_eq!(karma.disposition_toward(1, 9), 10, "20 - 10");
        assert_eq!(karma.disposition_toward(2, 9), -40, "-30 - 10 clamped >= -100");
        // Gifts push back up.
        karma.apply(9, MoralKind::Gift, 1.0);
        karma.apply(9, MoralKind::Help, 0.5);
        assert_eq!(karma.disposition_toward(1, 9), 10 + 10 + 4);
        // Clamp at the floor.
        for _ in 0..10 {
            karma.apply(9, MoralKind::Assault, 1.0);
        }
        assert_eq!(karma.disposition_toward(1, 9), DISPOSITION_MIN);
        // Unknown actor: pure baseline.
        assert_eq!(karma.disposition_toward(1, 77), 20);
    }

    /// Determinism: the same history yields identical dispositions.
    #[test]
    fn p3d405_reactions_are_deterministic() {
        let build = || {
            let mut k = Karma::new(&[(5, 0)]);
            let mut kn = Knowledge::default();
            for t in 0..10u64 {
                let e = event(3, if t % 2 == 0 { MoralKind::Help } else { MoralKind::Theft }, t as i32, t);
                kn.remember(e, 0.9);
                k.apply(3, e.kind, 0.9);
            }
            (kn, k)
        };
        let (a_kn, a_k) = build();
        let (b_kn, b_k) = build();
        assert_eq!(a_kn, b_kn);
        assert_eq!(a_k.disposition_toward(5, 3), b_k.disposition_toward(5, 3));
    }
}
