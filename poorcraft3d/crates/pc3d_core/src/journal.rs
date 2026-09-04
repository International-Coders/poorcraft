//! P3D-003: the append-only event journal.
//!
//! What happened, on which tick, under which globally monotonic sequence
//! number. Append-only by law — consumers (persistence, replication, NPC
//! knowledge) index by seq and may never mutate history. The `digest` is
//! FNV-1a 64 over the fixed field order; it is the journal's fingerprint.

/// One immutable domain event. Fixed-width payload (two u64 words) keeps the
/// digest trivially stable across saves; richer payloads hash into a word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalEvent {
    pub tick: u64,
    pub seq: u64,
    pub kind: u32,
    pub payload: [u64; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventJournal {
    events: Vec<JournalEvent>,
    next_seq: u64,
}

impl Default for EventJournal {
    fn default() -> Self {
        Self::new()
    }
}

/// FNV-1a 64 over bytes — the P3D house digest, used by the journal,
/// `ReplayDigest`, and the seed streams.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

impl EventJournal {
    pub fn new() -> Self {
        EventJournal { events: Vec::new(), next_seq: 0 }
    }

    /// Record one event at `tick`; returns its sequence number (dense,
    /// monotone, starting at 0).
    pub fn record(&mut self, tick: u64, kind: u32, payload: [u64; 2]) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.events.push(JournalEvent { tick, seq, kind, payload });
        seq
    }

    /// Persistence seam: the high-water mark plus the full history.
    pub fn high_water_mark(&self) -> u64 {
        self.next_seq
    }

    pub fn restore(next_seq: u64, events: Vec<JournalEvent>) -> Self {
        EventJournal { events, next_seq }
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

    /// FNV-1a over (tick, seq, kind, payload[0], payload[1]) in record
    /// order. Identical journals digest identically; any perturbation of
    /// content OR order changes the digest.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Sequence numbers are dense, monotone, and survive restore exactly.
    #[test]
    fn p3d003_journal_sequence_is_dense_and_restorable() {
        let mut j = EventJournal::new();
        for i in 0..25u64 {
            assert_eq!(j.record(i / 2, 7, [i, i * 3]), i);
        }
        assert_eq!(j.high_water_mark(), 25);
        let restored = EventJournal::restore(j.high_water_mark(), j.iter().cloned().collect());
        assert_eq!(restored.digest(), j.digest());
        assert_eq!(restored.len(), 25);
    }

    /// Identical journals digest identically; content OR order changes move
    /// the digest.
    #[test]
    fn p3d003_digest_is_stable_and_perturbation_sensitive() {
        let mut j = EventJournal::new();
        for i in 0..50u64 {
            j.record(i / 2, (i % 3) as u32 + 1, [i, i.wrapping_mul(7919)]);
        }
        let d = j.digest();

        let same = EventJournal::restore(j.high_water_mark(), j.iter().cloned().collect());
        assert_eq!(same.digest(), d);

        let mut flip: Vec<JournalEvent> = j.iter().cloned().collect();
        flip.last_mut().unwrap().payload[0] ^= 1;
        assert_ne!(EventJournal::restore(j.high_water_mark(), flip).digest(), d);

        let mut swapped: Vec<JournalEvent> = j.iter().cloned().collect();
        swapped.swap(10, 11);
        assert_ne!(EventJournal::restore(j.high_water_mark(), swapped).digest(), d);

        // Deterministic across repeats.
        assert_eq!(j.digest(), d);
    }
}
