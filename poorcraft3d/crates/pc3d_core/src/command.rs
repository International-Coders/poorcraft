//! P3D-003: sequenced command envelopes.
//!
//! Every player/system intent enters the simulation as a `CommandEnvelope`:
//! which tick it belongs to and a globally monotonic id. Order by (tick, id)
//! is total, so delivery grouping, reordering, and duplicates are inert —
//! the host applies batches in canonical order and duplicate ids keep their
//! EARLIEST occurrence.

/// Monotonic command identity source. Persist the high-water mark with the
/// world (`restore`) so ids never repeat across saves.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub fn restore(high_water_mark: u64) -> Self {
        CommandSequencer { next: high_water_mark }
    }
    /// The next id that would be assigned — the persistence value.
    pub fn high_water_mark(&self) -> u64 {
        self.next
    }
}

/// A command wrapped with its simulation identity.
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

    /// Canonical application order: (tick, id), duplicate ids dropped
    /// keeping the EARLIEST occurrence. After this, delivering N commands
    /// in one batch versus one-by-one cannot change the result.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Echo(u32);

    /// (tick, id) is the total order; duplicates keep their earliest side.
    #[test]
    fn p3d003_canonical_order_is_total_and_dedups() {
        let mk = |id: u64, tick: u64, v: u32| CommandEnvelope::new(id, tick, Echo(v));
        let batch = vec![mk(3, 1, 30), mk(1, 2, 10), mk(3, 0, 99), mk(2, 1, 20), mk(1, 0, 5)];
        let canon = CommandEnvelope::canonical_batch(batch);
        let order: Vec<(u64, u64)> = canon.iter().map(|e| (e.tick, e.id)).collect();
        assert_eq!(order, vec![(0, 1), (0, 3), (1, 2)]);
        let values: Vec<u32> = canon.into_iter().map(|e| e.command.0).collect();
        assert_eq!(values, vec![5, 99, 20], "earliest duplicate wins");
    }

    /// Ids are dense and monotone; the high-water mark restores without reuse.
    #[test]
    fn p3d003_sequencer_is_dense_and_restorable() {
        let mut seq = CommandSequencer::new();
        for i in 1..=10 {
            assert_eq!(seq.assign(), i);
        }
        assert_eq!(seq.high_water_mark(), 10);
        let mut restored = CommandSequencer::restore(10);
        assert_eq!(restored.assign(), 11, "ids must never repeat across a save");
    }
}
