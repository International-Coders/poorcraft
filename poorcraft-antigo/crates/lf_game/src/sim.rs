//! B02 deterministic simulation primitives
//! (docs/BETA-FOUNDATION/08-BETA-DELIVERY-ROADMAP.md, Stage A).
//!
//! These are the order/tick primitives the authoritative host (B03+) and the
//! fluid, machine, and NPC systems will share. They wrap existing behavior
//! rather than redesigning it:
//!
//! - [`TickClock`] turns the client's variable real-frame `dt` stream into
//!   whole fixed sim ticks, so a simulation result is defined by tick count,
//!   never by render cadence.
//! - [`CommandEnvelope`] gives every player/system command a monotonic id and
//!   an issuing tick; `canonical_batch` makes batching and reordering inert —
//!   application order is always (tick, id).
//! - [`EventLog`] records immutable domain events under a monotonic sequence
//!   and hashes them with FNV-1a (the house hash — std's DefaultHasher is
//!   randomized and was already banned for identity work in lf_worldgen).
//!
//! Hashing uses only integers (`f32` enters hashes via `to_bits`), so the
//! same operation sequence hashes identically everywhere.

/// Nominal simulation rate. 60 Hz matches the client's per-frame systems
/// being fed near-60 Hz dt today; B03 migrates real systems onto it.
pub const SIM_HZ: u32 = 60;

/// One sim tick in whole microseconds (1_000_000 / 60, truncated). Integer
/// accumulation keeps the clock deterministic; the 4 ppm rate offset from
/// truncation is irrelevant to gameplay.
pub const TICK_US: u64 = 1_000_000 / SIM_HZ as u64;

/// Spiral-of-death guard: the most backlog a single `advance` may fire before
/// the remainder is shed. A shed keeps the sim live at the cost of sim/wall
/// skew — the documented overload policy, tested to be deterministic.
pub const MAX_CATCHUP_TICKS: u32 = 8;

/// FNV-1a 64 state for simulation snapshots. Fold state fields in ONE fixed
/// order per snapshot version; never fold map iteration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimHash(pub u64);

impl SimHash {
    pub const fn new() -> Self {
        SimHash(0xcbf29ce484222325)
    }
    pub fn mix_bytes(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= *b as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
    pub fn mix_u64(&mut self, v: u64) {
        self.mix_bytes(&v.to_le_bytes());
    }
    pub fn mix_i64(&mut self, v: i64) {
        self.mix_u64(v as u64);
    }
    pub fn mix_f32(&mut self, v: f32) {
        self.mix_u64(v.to_bits() as u64);
    }
}

/// Fixed-step clock: real-frame seconds in, whole sim ticks out.
///
/// The caller feeds the same clamped real dt it feeds its render systems
/// (the client clamps to 0.25 s today). Input is converted to integer
/// microseconds immediately, so an identical dt stream always yields an
/// identical tick sequence.
#[derive(Clone, Debug)]
pub struct TickClock {
    accumulated_us: u64,
    /// Total sim ticks fired since construction — the simulation's clock.
    pub tick: u64,
}

impl Default for TickClock {
    fn default() -> Self {
        Self::new()
    }
}

impl TickClock {
    pub fn new() -> Self {
        TickClock { accumulated_us: 0, tick: 0 }
    }

    /// Feed real elapsed seconds; returns the INCLUSIVE range of sim ticks
    /// that must execute NOW, in order (an empty range when none fire). The
    /// caller must run every tick in the range exactly once — looping over
    /// the range, not over a count, is what keeps multi-tick frames honest
    /// (each fired tick executes under its own number).
    pub fn advance(&mut self, real_dt: f32) -> std::ops::Range<u64> {
        let add_us = if real_dt.is_finite() && real_dt > 0.0 {
            (real_dt * 1_000_000.0) as u64
        } else {
            0
        };
        self.accumulated_us = self.accumulated_us.saturating_add(add_us);
        let first = self.tick + 1;
        let mut fired: u32 = 0;
        while self.accumulated_us >= TICK_US && fired < MAX_CATCHUP_TICKS {
            self.accumulated_us -= TICK_US;
            fired += 1;
        }
        if fired == MAX_CATCHUP_TICKS {
            // Shed remaining backlog: drop the partial accumulator rather
            // than letting it grow unbounded during hitches.
            self.accumulated_us = 0;
        }
        self.tick += fired as u64;
        first..first + fired as u64
    }
}

/// Monotonic command identity source. Ids never repeat within a sequencer's
/// lifetime, so (tick, id) is a total order over commands.
#[derive(Clone, Debug)]
pub struct CommandSequencer {
    next: u64,
}

impl Default for CommandSequencer {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandSequencer {
    pub fn new() -> Self {
        CommandSequencer { next: 0 }
    }
    pub fn assign(&mut self) -> u64 {
        self.next += 1;
        self.next
    }
    /// Restore from a save: the host persists the high-water mark.
    pub fn restore(next: u64) -> Self {
        CommandSequencer { next }
    }
}

/// A command wrapped with the identity the authoritative simulation needs:
/// which tick it belongs to and its unique id. Generic over the command type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandEnvelope<C> {
    pub id: u64,
    pub tick: u64,
    pub command: C,
}

impl<C> CommandEnvelope<C> {
    pub fn new(id: u64, tick: u64, command: C) -> Self {
        CommandEnvelope { id, tick, command }
    }

    /// Canonical application order: (tick, id), duplicates (same id) dropped
    /// keeping the EARLIEST occurrence. After this, batching N commands per
    /// delivery versus delivering them one-by-one cannot change the result —
    /// the order is total and ids are unique.
    pub fn canonical_batch(mut batch: Vec<Self>) -> Vec<Self>
    where
        C: Clone,
    {
        batch.sort_by(|a, b| (a.tick, a.id).cmp(&(b.tick, b.id)));
        let mut seen = std::collections::BTreeSet::new();
        batch.retain(|env| seen.insert(env.id));
        batch
    }
}

/// An immutable domain event: what happened, on which tick, under which
/// globally monotonic sequence number. Events are append-only; consumers
/// (reputation, NPC knowledge, replication — B15/B16/B24) index by seq.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainEvent {
    pub tick: u64,
    pub seq: u64,
    pub kind: u32,
    /// Two generic u64 payload words (ids, positions, amounts, or a hash of
    /// richer payloads). Kept fixed-width so event hashing is trivially
    /// stable across saves.
    pub payload: [u64; 2],
}

#[derive(Clone, Debug, Default)]
pub struct EventLog {
    events: Vec<DomainEvent>,
    next_seq: u64,
}

impl EventLog {
    pub fn new() -> Self {
        EventLog { events: Vec::new(), next_seq: 0 }
    }

    /// Record one event at `tick`; returns its sequence number.
    pub fn record(&mut self, tick: u64, kind: u32, payload: [u64; 2]) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.events.push(DomainEvent { tick, seq, kind, payload });
        seq
    }

    /// High-water mark for saves; paired with `restore`.
    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }

    pub fn restore(next_seq: u64, events: Vec<DomainEvent>) -> Self {
        EventLog { events, next_seq }
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &DomainEvent> {
        self.events.iter()
    }

    /// FNV-1a over (tick, seq, kind, payload) in record order. A consumed
    /// simulation's event log hash IS its snapshot fingerprint's spine.
    pub fn hash(&self) -> SimHash {
        let mut h = SimHash::new();
        for e in &self.events {
            h.mix_u64(e.tick);
            h.mix_u64(e.seq);
            h.mix_u64(e.kind as u64);
            h.mix_u64(e.payload[0]);
            h.mix_u64(e.payload[1]);
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A representative deterministic simulation: one rider on a 1-D line in
    /// milliblock fixed point, moved only by commands, emitting one domain
    /// event per applied command. Small, but it exercises exactly the B02
    /// contract — tick-defined integration, total command order, evented
    /// results — the same way block edits and inventory transactions will be
    /// migrated in B03.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Rider {
        pos_mb: i64,
        applied: u64,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Ride {
        Step(i64),
        Halt,
    }

    impl Rider {
        fn step(&mut self, tick: u64, cmd: &Ride, log: &mut EventLog) {
            match cmd {
                Ride::Step(v) => {
                    self.pos_mb += *v;
                    self.applied += 1;
                    log.record(tick, 1, [*v as u64, self.pos_mb as u64]);
                }
                Ride::Halt => {
                    log.record(tick, 2, [0, self.pos_mb as u64]);
                }
            }
        }
        fn hash(&self) -> SimHash {
            let mut h = SimHash::new();
            h.mix_i64(self.pos_mb);
            h.mix_u64(self.applied);
            h
        }
    }

    /// Drive the rider through a TickClock with the given per-frame dt
    /// stream. Commands are keyed to SIM TICKS (tick t issues a command that
    /// applies at t+1), which is the contract under test: a command schedule
    /// lives on the simulation timeline, never on render frames.
    fn run_stream(dts: &[f32]) -> (Rider, EventLog, u64) {
        let mut clock = TickClock::new();
        let mut seq = CommandSequencer::new();
        let mut rider = Rider { pos_mb: 0, applied: 0 };
        let mut log = EventLog::new();
        let mut pending: Vec<CommandEnvelope<Ride>> = Vec::new();
        for dt in dts {
            for tick in clock.advance(*dt) {
                // Commands issued on the previous tick arrive for this one.
                if let Some(cmd) = scheduled_command(tick - 1) {
                    let id = seq.assign();
                    pending.push(CommandEnvelope::new(id, tick, cmd));
                }
                let due: Vec<CommandEnvelope<Ride>> = pending
                    .iter()
                    .filter(|e| e.tick == tick)
                    .cloned()
                    .collect();
                for env in CommandEnvelope::canonical_batch(due) {
                    rider.step(tick, &env.command, &mut log);
                }
                pending.retain(|e| e.tick != tick);
            }
        }
        (rider, log, clock.tick)
    }

    /// The deterministic command schedule: every 37th tick issues a step
    /// (alternating +250 / -90 milliblocks) for the next tick.
    fn scheduled_command(tick: u64) -> Option<Ride> {
        if tick % 37 != 0 || tick == 0 {
            return None;
        }
        Some(Ride::Step(if (tick / 37) % 2 == 1 { 250 } else { -90 }))
    }

    fn uniform_60hz(frames: usize) -> Vec<f32> {
        vec![1.0 / 60.0; frames]
    }

    /// Deterministic jitter around ~60 Hz: frames between 20 and 90 fps.
    fn jitter_stream(frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|i| {
                let mut h = SimHash::new();
                h.mix_u64(i as u64);
                let r = (h.0 % 1000) as f32 / 1000.0;
                0.011 + r * 0.038
            })
            .collect()
    }

    /// The done-when: render cadence must not alter a representative
    /// simulation result. Three dt streams — uniform 60 fps, chunky 3-tick
    /// frames, and mixed 1/2-tick frames — each covering exactly the same
    /// 600 sim ticks execute the SAME tick-keyed command schedule and must
    /// produce the SAME final tick, state, and event-log hash. The schedule
    /// lives on the sim timeline; the frame partitioning is invisible to it.
    #[test]
    fn b02_render_cadence_does_not_alter_representative_results() {
        // Stream A: uniform 60 fps, 600 frames of one tick.
        let (a_rider, a_log, a_ticks) = run_stream(&uniform_60hz(600));

        // Stream B: 200 frames of 3 ticks each.
        let stream_b = vec![3.0 / 60.0; 200];
        let (b_rider, b_log, b_ticks) = run_stream(&stream_b);

        // Stream C: alternating 1/2-tick frames, still 600 total.
        let mut stream_c = Vec::new();
        for i in 0..400 {
            stream_c.push(if i % 2 == 0 { 1.0 / 60.0 } else { 2.0 / 60.0 });
        }
        let (c_rider, c_log, c_ticks) = run_stream(&stream_c);

        assert_eq!(a_ticks, 600);
        assert_eq!(b_ticks, 600, "stream B must cover the same 600 ticks");
        assert_eq!(c_ticks, 600, "stream C must cover the same 600 ticks");
        assert!(a_rider.applied > 0, "the schedule must have issued commands");
        assert_eq!(a_rider, b_rider, "state diverged under 3-tick frames");
        assert_eq!(a_rider, c_rider, "state diverged under mixed frames");
        assert_eq!(a_log.hash(), b_log.hash(), "event log diverged (B)");
        assert_eq!(a_log.hash(), c_log.hash(), "event log diverged (C)");
    }

    /// Real-valued jitter must also stay within a tick or two of wall time
    /// (the clock neither drifts nor races), and an identical stream must be
    /// perfectly reproducible.
    #[test]
    fn b02_clock_tracks_wall_time_and_replays_identically() {
        let frames = 600;
        let dts = jitter_stream(frames);
        let total: f32 = dts.iter().sum();
        let expected = (total * SIM_HZ as f32) as u64;

        let mut c1 = TickClock::new();
        let mut c2 = TickClock::new();
        for dt in &dts {
            c1.advance(*dt);
            c2.advance(*dt);
        }
        assert_eq!(c1.tick, c2.tick, "same stream must replay identically");
        let drift = c1.tick.abs_diff(expected);
        assert!(drift <= 2, "clock drifted {drift} ticks from wall time");
    }

    /// A hitch longer than the catch-up budget sheds backlog deterministically
    /// instead of spiraling: the same overload stream twice yields the same
    /// tick count, and a single advance never fires more than the cap.
    #[test]
    fn b02_overload_sheds_backlog_deterministically() {
        let mut clock = TickClock::new();
        let fired = clock.advance(1.0); // a one-second freeze in one frame
        assert_eq!((fired.end - fired.start) as u32, MAX_CATCHUP_TICKS);
        assert_eq!(fired, 1..1 + MAX_CATCHUP_TICKS as u64, "ticks must carry their own numbers");
        assert_eq!(clock.accumulated_us, 0, "backlog must be shed, not banked");
        let mut a = TickClock::new();
        let mut b = TickClock::new();
        for _ in 0..10 {
            a.advance(1.0);
            b.advance(1.0);
        }
        assert_eq!(a.tick, b.tick);
        assert_eq!(a.tick, 10 * MAX_CATCHUP_TICKS as u64);
        // Bad input is inert.
        let mut c = TickClock::new();
        assert!(c.advance(f32::NAN).is_empty());
        assert!(c.advance(-0.5).is_empty());
    }

    /// The other half of the done-when: command batching must not alter
    /// results. The same commands on the same ticks, delivered one-by-one
    /// versus in one scrambled batch, produce the same state and the same
    /// event log — because application order is the total (tick, id) order.
    #[test]
    fn b02_command_batching_does_not_alter_representative_results() {
        // 30 commands across 10 ticks, three per tick.
        let commands: Vec<(u64, u64, Ride)> = (0..30u64)
            .map(|i| {
                let tick = i / 3;
                let cmd = if i % 7 == 0 { Ride::Halt } else { Ride::Step((i as i64 % 5 - 2) * 100) };
                (i, tick, cmd)
            })
            .collect();

        // Delivery 1: commands arrive one-by-one, applied immediately.
        let mut rider1 = Rider { pos_mb: 0, applied: 0 };
        let mut log1 = EventLog::new();
        for &(id, tick, ref cmd) in &commands {
            let _ = id;
            rider1.step(tick, cmd, &mut log1);
        }

        // Delivery 2: everything arrives in ONE batch, reversed, then
        // canonicalized before application.
        let batch: Vec<CommandEnvelope<Ride>> = commands
            .iter()
            .rev()
            .map(|&(id, tick, ref cmd)| CommandEnvelope::new(id, tick, cmd.clone()))
            .collect();
        let mut rider2 = Rider { pos_mb: 0, applied: 0 };
        let mut log2 = EventLog::new();
        for env in CommandEnvelope::canonical_batch(batch) {
            rider2.step(env.tick, &env.command, &mut log2);
        }

        assert!(rider1.applied > 0);
        assert_eq!(rider1, rider2, "batching changed the outcome");
        assert_eq!(log1.hash(), log2.hash(), "batching changed the event log");
    }

    #[test]
    fn b02_canonical_order_is_total_and_duplicates_are_suppressed() {
        let mk = |id: u64, tick: u64, v: i64| CommandEnvelope::new(id, tick, Ride::Step(v));
        let batch = vec![mk(3, 1, 300), mk(1, 2, 100), mk(3, 0, 999), mk(2, 1, 200), mk(1, 0, -50)];
        let canon = CommandEnvelope::canonical_batch(batch);
        let order: Vec<(u64, u64)> = canon.iter().map(|e| (e.tick, e.id)).collect();
        assert_eq!(order, vec![(0, 1), (0, 3), (1, 2)], "order must be (tick, id)");
        // Duplicate ids keep their EARLIEST (tick, id) occurrence: id 1 at
        // tick 0 (-50, not the later tick-2 100) and id 3 at tick 0 (999,
        // not the later tick-1 300).
        let values: Vec<i64> = canon
            .iter()
            .map(|e| match &e.command {
                Ride::Step(v) => *v,
                Ride::Halt => 0,
            })
            .collect();
        assert_eq!(values, vec![-50, 999, 200]);
    }

    #[test]
    fn b02_event_log_is_monotone_and_hash_is_perturbation_sensitive() {
        let mut log = EventLog::new();
        for i in 0..50u64 {
            let seq = log.record(i / 2, (i % 3) as u32 + 1, [i, i.wrapping_mul(7919)]);
            assert_eq!(seq, i, "sequence numbers are dense and monotonic");
        }
        assert_eq!(log.len(), 50);
        let h = log.hash();

        let copy = EventLog::restore(log.next_seq(), log.iter().cloned().collect());
        assert_eq!(copy.hash(), h, "identical logs must hash identically");

        // Flip one payload bit in the last event: hash must change.
        let mut events: Vec<DomainEvent> = log.iter().cloned().collect();
        let last = events.last_mut().unwrap();
        last.payload[0] ^= 1;
        let perturbed = EventLog::restore(log.next_seq(), events);
        assert_ne!(perturbed.hash(), h);

        // Reordering two events must change the hash too (order is content).
        let mut swapped: Vec<DomainEvent> = log.iter().cloned().collect();
        swapped.swap(10, 11);
        assert_ne!(EventLog::restore(log.next_seq(), swapped).hash(), h);

        // Sequencer restore continues the high-water mark without reuse.
        let mut seq = CommandSequencer::restore(41);
        assert_eq!(seq.assign(), 42);
    }
}
