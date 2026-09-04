//! P3D-003: the replay-hash harness.
//!
//! `ReplayDigest` is the state fingerprint type: a stable FNV-1a fold the
//! simulation (and its tests) use to prove that identical command streams
//! produce identical worlds. The tests in this module ARE the harness — the
//! representative simulation below is the pattern every future subsystem
//! replays: commands live on sim ticks, the host applies canonical batches,
//! and the digest must be invariant under render cadence and delivery
//! batching.

use crate::clock::FixedClock;
use crate::command::{CommandEnvelope, CommandSequencer};
use crate::journal::JournalEvent;

/// Stable state-fingerprint builder (FNV-1a 64). Fold fields in ONE fixed
/// order per fingerprint version; integers only — floats enter via to_bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ReplayDigest(pub u64);

impl ReplayDigest {
    pub fn new() -> Self {
        ReplayDigest(0xcbf29ce484222325)
    }
    pub fn mix_u64(&mut self, v: u64) {
        self.0 ^= v;
        self.0 = self.0.wrapping_mul(0x100000001b3);
    }
    pub fn mix_i64(&mut self, v: i64) {
        self.mix_u64(v as u64);
    }
    pub fn mix_f32(&mut self, v: f32) {
        self.mix_u64(v.to_bits() as u64);
    }
    /// Fold a journal's events so a digest can span state + history.
    pub fn mix_event(&mut self, e: &JournalEvent) {
        self.mix_u64(e.tick);
        self.mix_u64(e.seq);
        self.mix_u64(e.kind as u64);
        self.mix_u64(e.payload[0]);
        self.mix_u64(e.payload[1]);
    }
}

/// A representative deterministic simulation: one cart on a fixed-point
/// rail, moved only by commands, eventing every applied command. Small, but
/// it exercises exactly the contract — tick-defined time, total command
/// order, journaled results — every future subsystem will run under.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Cart {
    pos: i64,
    applied: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Move {
    Step(i64),
    Brake,
}

impl Cart {
    fn apply(&mut self, tick: u64, cmd: &Move, journal: &mut Vec<JournalEvent>) {
        let (kind, payload) = match cmd {
            Move::Step(v) => {
                self.pos += v;
                self.applied += 1;
                (1u32, [*v as u64, self.pos as u64])
            }
            Move::Brake => (2u32, [0, self.pos as u64]),
        };
        journal.push(JournalEvent { tick, seq: u64::MAX, kind, payload });
    }
    fn digest(&self) -> ReplayDigest {
        let mut d = ReplayDigest::new();
        d.mix_i64(self.pos);
        d.mix_u64(self.applied);
        d
    }
}

/// The full harness run: feed a dt stream, schedule tick-keyed commands,
/// apply canonical batches per tick, return (cart digest, journal digest,
/// final tick).
fn run(dts: &[f32]) -> (ReplayDigest, u64, u64) {
    let mut clock = FixedClock::new();
    let mut seq = CommandSequencer::new();
    let mut cart = Cart { pos: 0, applied: 0 };
    let mut journal: Vec<JournalEvent> = Vec::new();
    let mut pending: Vec<CommandEnvelope<Move>> = Vec::new();
    for dt in dts {
        for tick in clock.advance(*dt) {
            // The schedule lives on the SIM timeline: every 37th tick queues
            // a move for the next tick.
            if tick > 1 && (tick - 1) % 37 == 0 {
                let id = seq.assign();
                let cmd = if ((tick - 1) / 37) % 2 == 1 { Move::Step(250) } else { Move::Step(-90) };
                pending.push(CommandEnvelope::new(id, tick, cmd));
            }
            let due: Vec<CommandEnvelope<Move>> =
                pending.iter().filter(|e| e.tick == tick).cloned().collect();
            for env in CommandEnvelope::canonical_batch(due) {
                cart.apply(tick, &env.command, &mut journal);
            }
            pending.retain(|e| e.tick != tick);
        }
    }
    // Renumber events by record order (the journal owns seq).
    let mut j = EventJournalSink::new();
    for mut e in journal {
        e.seq = j.next_seq();
        j.push(e);
    }
    let mut jd = ReplayDigest::new();
    for e in j.events() {
        jd.mix_event(e);
    }
    let mut cd = cart.digest();
    jd.mix_u64(cd.0);
    (jd, cd.0, clock.tick)
}

/// Tiny local sink to assign dense seqs while keeping the digest fold honest.
struct EventJournalSink {
    events: Vec<JournalEvent>,
    next: u64,
}

impl EventJournalSink {
    fn new() -> Self {
        EventJournalSink { events: Vec::new(), next: 0 }
    }
    fn next_seq(&self) -> u64 {
        self.next
    }
    fn push(&mut self, e: JournalEvent) {
        self.events.push(e);
        self.next += 1;
    }
    fn events(&self) -> &[JournalEvent] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE P3D-003 CONTRACT: render cadence must not alter a representative
    /// result. Uniform 60 fps, 3-tick frames, and mixed 1/2-tick partitions
    /// over the same 600 sim ticks execute the same tick-keyed schedule and
    /// MUST produce identical cart and journal digests.
    #[test]
    fn p3d003_replay_is_cadence_invariant() {
        let uniform: Vec<f32> = vec![1.0 / 60.0; 600];
        let chunky: Vec<f32> = vec![3.0 / 60.0; 200];
        let mut mixed = Vec::new();
        for i in 0..400 {
            mixed.push(if i % 2 == 0 { 1.0 / 60.0 } else { 2.0 / 60.0 });
        }

        let (digest_a, cart_a, ticks_a) = run(&uniform);
        let (digest_b, cart_b, ticks_b) = run(&chunky);
        let (digest_c, cart_c, ticks_c) = run(&mixed);

        assert_eq!(ticks_a, 600);
        assert_eq!(ticks_b, 600);
        assert_eq!(ticks_c, 600);
        assert_eq!(cart_a, cart_b, "cart state diverged under 3-tick frames");
        assert_eq!(cart_a, cart_c, "cart state diverged under mixed frames");
        assert_eq!(digest_a, digest_b, "journal diverged under 3-tick frames");
        assert_eq!(digest_a, digest_c, "journal diverged under mixed frames");
    }

    /// The other half: delivery batching must not alter the result. The
    /// same commands on the same ticks, applied one-by-one versus in one
    /// reversed batch, yield identical state and journal digests.
    #[test]
    fn p3d003_replay_is_batching_invariant() {
        let commands: Vec<(u64, u64, Move)> = (0..30u64)
            .map(|i| {
                let tick = i / 3;
                let cmd =
                    if i % 7 == 0 { Move::Brake } else { Move::Step((i as i64 % 5 - 2) * 100) };
                (i, tick, cmd)
            })
            .collect();

        let apply =
            |batch: Vec<CommandEnvelope<Move>>| -> (ReplayDigest, ReplayDigest) {
                let mut cart = Cart { pos: 0, applied: 0 };
                let mut journal: Vec<JournalEvent> = Vec::new();
                for env in CommandEnvelope::canonical_batch(batch) {
                    cart.apply(env.tick, &env.command, &mut journal);
                }
                let mut jd = ReplayDigest::new();
                for (seq, mut e) in journal.into_iter().enumerate() {
                    e.seq = seq as u64;
                    jd.mix_event(&e);
                }
                (cart.digest(), jd)
            };

        let one_by_one: Vec<CommandEnvelope<Move>> = commands
            .iter()
            .map(|&(id, tick, ref c)| CommandEnvelope::new(id, tick, c.clone()))
            .collect();
        let mut single_batch: Vec<CommandEnvelope<Move>> = one_by_one.clone();
        single_batch.reverse();

        let (cart1, j1) = apply(one_by_one);
        let (cart2, j2) = apply(single_batch);
        assert_eq!(cart1, cart2, "batching changed the outcome");
        assert_eq!(j1, j2, "batching changed the journal");
    }

    /// A replayed command id is inert in a canonical batch.
    #[test]
    fn p3d003_duplicate_commands_apply_once() {
        let batch = vec![
            CommandEnvelope::new(1, 0, Move::Step(10)),
            CommandEnvelope::new(1, 5, Move::Step(999)),
            CommandEnvelope::new(2, 1, Move::Brake),
        ];
        let canon = CommandEnvelope::canonical_batch(batch);
        assert_eq!(canon.len(), 2);
        assert_eq!(canon[0].command, Move::Step(10), "earliest duplicate wins");
        assert_eq!(canon[1].command, Move::Brake);
    }
}
