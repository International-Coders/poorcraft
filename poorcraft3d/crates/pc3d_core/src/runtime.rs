//! P3D-005: the first empty-world runtime.
//!
//! `WorldRuntime` is the skeleton every later host (solo in-process or
//! dedicated) must reuse: one owner for the fixed clock, the counters, the
//! frame times, the journal, the command sequencer, and the world's seed
//! streams. Today the world is empty — a tick is honest work only in that
//! it is counted and journaled — but the LOOP is real, and the digest
//! proves it runs deterministically.

use crate::clock::FixedClock;
use crate::command::CommandSequencer;
use crate::journal::JournalEvent;
use crate::profile::{Counters, CounterId, FrameTimes};
use crate::replay::ReplayDigest;
use crate::seed::SeedStreams;

/// Journal heartbeat cadence: one heartbeat event every 600 ticks (10
/// simulated seconds at 60 Hz).
pub const HEARTBEAT_TICKS: u64 = 600;

/// Journal event kinds emitted by the empty-world runtime.
pub const EV_HEARTBEAT: u32 = 1;
pub const EV_FRAME_BATCH: u32 = 2;

/// The single owner of the simulation's measurement and time state.
pub struct WorldRuntime {
    pub profile_name: String,
    pub world_seed: u64,
    pub clock: FixedClock,
    pub counters: Counters,
    pub frames: FrameTimes,
    pub journal: EventJournalOwner,
    pub sequencer: CommandSequencer,
    pub streams: SeedStreams,
}

/// A thin wrapper so the journal records through one path and the digest
/// sees the same events.
#[derive(Default)]
pub struct EventJournalOwner {
    events: Vec<JournalEvent>,
    next_seq: u64,
}

impl EventJournalOwner {
    pub fn record(&mut self, tick: u64, kind: u32, payload: [u64; 2]) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.events.push(JournalEvent { tick, seq, kind, payload });
        seq
    }
    pub fn len(&self) -> usize {
        self.events.len()
    }
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = &JournalEvent> {
        self.events.iter()
    }
    pub fn digest(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for e in &self.events {
            for word in [e.tick, e.seq, e.kind as u64, e.payload[0], e.payload[1]] {
                for b in word.to_le_bytes() {
                    h ^= b as u64;
                    h = h.wrapping_mul(0x100000001b3);
                }
            }
        }
        h
    }
}

/// Numbers one run produced — the smoke line and the tests both read these.
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeStats {
    pub frames: usize,
    pub ticks: u64,
    pub journal_events: usize,
    pub p50_ms: f32,
    pub p95_ms: f32,
    pub digest: u64,
}

impl WorldRuntime {
    pub fn new(world_seed: u64, profile_name: &str) -> Self {
        WorldRuntime {
            profile_name: profile_name.to_string(),
            world_seed,
            clock: FixedClock::new(),
            counters: Counters::default(),
            frames: FrameTimes::default(),
            journal: EventJournalOwner::default(),
            sequencer: CommandSequencer::new(),
            streams: SeedStreams::new(world_seed),
        }
    }

    /// One real frame: feed frame time, execute every fired tick, record
    /// the measurement. Returns how many ticks fired.
    pub fn frame(&mut self, frame_ms: f32) -> u32 {
        let mut fired: u32 = 0;
        for tick in self.clock.advance(frame_ms / 1000.0) {
            fired += 1;
            self.counters.inc(CounterId::EntityTicks);
            if tick % HEARTBEAT_TICKS == 0 {
                // Heartbeat payload mixes the seed so two worlds' journals
                // differ even when their tick counts match.
                let mut seed_mixin = self.streams.stream_seed("heartbeat");
                seed_mixin ^= tick;
                self.journal.record(tick, EV_HEARTBEAT, [tick, seed_mixin]);
            }
        }
        if fired > 0 {
            self.journal
                .record(self.clock.tick, EV_FRAME_BATCH, [fired as u64, self.clock.tick]);
        }
        self.frames.push(frame_ms);
        self.counters.add(CounterId::JournalEvents, 1);
        fired
    }

    /// The run's fingerprint: clock, counters, journal — everything the
    /// deterministic spine owns.
    pub fn digest(&self) -> u64 {
        let mut d = ReplayDigest::new();
        d.mix_u64(self.clock.tick);
        for (id, v) in self.counters.snapshot() {
            d.mix_u64(id as u64);
            d.mix_u64(v);
        }
        d.mix_u64(self.journal.digest());
        d.0
    }

    pub fn stats(&self) -> RuntimeStats {
        RuntimeStats {
            frames: self.frames.len(),
            ticks: self.clock.tick,
            journal_events: self.journal.len(),
            p50_ms: self.frames.p50(),
            p95_ms: self.frames.p95(),
            digest: self.digest(),
        }
    }
}

/// Drive `frames` frames of deterministic jitter (the same construction the
/// baseline workload uses) and return the runtime.
pub fn run_headless(world_seed: u64, frames: usize) -> WorldRuntime {
    let mut rt = WorldRuntime::new(world_seed, "headless");
    let mut jitter = rt.streams.rng(crate::seed::stream::WEATHER);
    for _ in 0..frames {
        let frame_ms = 11.0 + jitter.unit_f32() * 22.0; // ~27..90 fps
        rt.frame(frame_ms);
    }
    rt
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE stage capstone contract: the runtime is deterministic. Same seed,
    /// same frame count → identical fingerprint, tick for tick.
    #[test]
    fn p3d005_runtime_is_deterministic() {
        let a = run_headless(0xA11CE, 600);
        let b = run_headless(0xA11CE, 600);
        assert_eq!(a.digest(), b.digest(), "same seed must replay identically");
        assert_eq!(a.stats(), b.stats());
        assert!(a.clock.tick > 0);
    }

    /// Different seeds diverge — the jitter stream itself derives from the
    /// world seed, so even the headless frame stream is world-specific.
    #[test]
    fn p3d005_seed_changes_the_fingerprint() {
        let a = run_headless(1, 700);
        let b = run_headless(2, 700);
        assert_ne!(a.digest(), b.digest());
        // Same frame COUNT, but each world jitters its own way; the tick
        // counts therefore need not match — only same-seed runs must replay.
        assert!(a.clock.tick > 0 && b.clock.tick > 0);
        let again = run_headless(2, 700);
        assert_eq!(again.digest(), b.digest(), "the second world must replay itself");
    }

    /// Liveness + heartbeat cadence: ticks fire, the journal beats on the
    /// 600-tick mark, and batches only journal on firing frames.
    #[test]
    fn p3d005_liveness_and_heartbeat_cadence() {
        // 700 frames of ~11-33 ms ≈ 11.6 sim-seconds ≈ 700 ticks: crosses
        // the 600 heartbeat but not 1200.
        let rt = run_headless(9, 700);
        assert!(rt.clock.tick >= 600 && rt.clock.tick < 1200, "tick={}", rt.clock.tick);
        let heartbeats =
            rt.journal.iter().filter(|e| e.kind == EV_HEARTBEAT).count();
        assert_eq!(heartbeats, 1, "exactly one heartbeat at tick 600");
        assert!(rt.journal.iter().any(|e| e.kind == EV_FRAME_BATCH));
        let s = rt.stats();
        assert_eq!(s.frames, 700);
        assert!(s.p50_ms > 0.0);
        assert!(s.p50_ms <= s.p95_ms || s.p95_ms == 0.0);
        assert!(s.journal_events >= heartbeats);
        // Batches fire only on frames that fired >= 1 tick: sub-tick frames
        // (11-16.6 ms jitter) record the frame but journal nothing.
        let batches = rt.journal.iter().filter(|e| e.kind == EV_FRAME_BATCH).count();
        assert!(batches > 0 && batches <= 700, "batches={batches}");
        // Every batch's fired-count is positive and within the shed cap.
        for e in rt.journal.iter().filter(|e| e.kind == EV_FRAME_BATCH) {
            let fired = e.payload[0];
            assert!(fired >= 1 && fired <= 8, "batch fired {fired}");
        }
    }

    /// A frame that fires no ticks (dt below one tick) still records the
    /// frame but journals no batch — the clock's honesty at runtime level.
    #[test]
    fn p3d005_sub_tick_frames_record_without_firing() {
        let mut rt = WorldRuntime::new(5, "test");
        let fired = rt.frame(1.0); // 1 ms < 16.67 ms tick
        assert_eq!(fired, 0);
        assert_eq!(rt.frames.len(), 1);
        assert_eq!(rt.journal.len(), 0, "no ticks, no batch event");
        assert_eq!(rt.counters.get(CounterId::EntityTicks), 0);
    }
}
