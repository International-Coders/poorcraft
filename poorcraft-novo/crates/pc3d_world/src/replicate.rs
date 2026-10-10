//! P3D-802: replication — reliable channels, acks, interest snapshots.
//!
//! The pure protocol layer under the dedicated server: every message is
//! sequenced, delivery is in-order (out-of-order arrivals buffer until
//! the gap fills), acks are cumulative plus a 32-range bitfield, and
//! unacked payloads stay queued for resend. State replication uses
//! versioned interest snapshots — only what a player can perceive —
//! and mirrors apply them monotonically: an old snapshot is stale and
//! rejected. No sockets here; the transport adapter is P3D-803's job.

use crate::coords::RegionCoord;
use crate::settlement::Settlements;
use std::collections::{BTreeMap, VecDeque};

/// One replicated scalar bundle per settlement (population, food,
/// defense, prosperity) — the Aggregate, flattened for the wire.
pub type ReplicatedValues = [i64; 4];

/// A versioned, interest-filtered piece of world state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepSnapshot {
    /// Host tick the snapshot was taken at — the staleness gate.
    pub tick: u64,
    pub origin: RegionCoord,
    pub radius: i32,
    /// settlement id → values.
    pub values: BTreeMap<u64, ReplicatedValues>,
}

/// Build a snapshot of every settlement within `radius` (Chebyshev) of
/// the player's region — interest management keeps replication bounded.
pub fn interest_snapshot(
    settlements: &Settlements,
    player_region: RegionCoord,
    radius: i32,
    tick: u64,
) -> RepSnapshot {
    let mut values = BTreeMap::new();
    for s in &settlements.list {
        let d = (s.center.x - player_region.x)
            .abs()
            .max((s.center.z - player_region.z).abs());
        if d > radius {
            continue;
        }
        let a = &s.aggregate;
        values.insert(s.id, [a.population, a.food, a.defense, a.prosperity]);
    }
    RepSnapshot {
        tick,
        origin: player_region,
        radius,
        values,
    }
}

/// The client-side mirror of replicated state.
#[derive(Clone, Debug, Default)]
pub struct Mirror {
    pub last_tick: u64,
    pub values: BTreeMap<u64, ReplicatedValues>,
}

impl Mirror {
    /// Apply a snapshot; stale (older-tick) snapshots are rejected.
    /// Returns true when applied.
    pub fn apply(&mut self, snap: &RepSnapshot) -> bool {
        if snap.tick <= self.last_tick && self.last_tick != 0 {
            return false;
        }
        // Interest contract: drop entries that left the snapshot's
        // radius only when the snapshot covers this player's area —
        // every in-radius id is present in `values`.
        self.values.retain(|id, _| snap.values.contains_key(id));
        for (id, v) in &snap.values {
            self.values.insert(*id, *v);
        }
        self.last_tick = snap.tick;
        true
    }
}

/// A reliable, ordered channel: the server side keeps unacked payloads
/// for resend; the receive side applies in order and acks.
#[derive(Clone, Debug, Default)]
pub struct ReliableChannel {
    /// Next sequence number to assign on send.
    send_seq: u64,
    /// Highest in-order applied sequence on receive.
    recv_high: u64,
    /// Bitfield of received-but-not-yet-in-order seqs (recv_high+1 ..).
    recv_bits: u32,
    /// Payloads awaiting an ack, by sequence.
    unacked: BTreeMap<u64, Vec<u64>>,
    /// Received-but-not-in-order payloads waiting for the gap.
    buffered: BTreeMap<u64, Vec<u64>>,
    /// Applied payloads in arrival order (the receive log).
    pub applied: VecDeque<Vec<u64>>,
}

/// An ack frame: highest contiguous sequence + 32-range bitfield.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ack {
    pub high: u64,
    pub bits: u32,
}

impl ReliableChannel {
    pub fn new() -> ReliableChannel {
        ReliableChannel::default()
    }

    /// Queue a payload; returns its sequence.
    pub fn send(&mut self, payload: Vec<u64>) -> u64 {
        self.send_seq += 1;
        self.unacked.insert(self.send_seq, payload);
        self.send_seq
    }

    /// The peer received `seq`: mark acked. Payloads acked by the
    /// cumulative high or the bitfield leave the resend queue.
    pub fn ack(&mut self, ack: Ack) {
        self.unacked.retain(|seq, _| {
            *seq > ack.high && (seq - ack.high > 32 || ack.bits & (1 << (seq - ack.high - 1)) == 0)
        });
    }

    /// Payloads the peer has not acked yet — resend these.
    pub fn needs_resend(&self) -> Vec<(u64, Vec<u64>)> {
        self.unacked.iter().map(|(s, p)| (*s, p.clone())).collect()
    }

    /// Deliver a sequence+payload off the wire. Out-of-order arrivals
    /// buffer; everything buffered applies the moment the gap fills.
    /// Bit semantics: bit i of the ack bitfield means sequence
    /// `high + 1 + i` has been RECEIVED (buffered past a hole).
    /// Returns the ack the receiver should send back.
    pub fn receive(&mut self, seq: u64, payload: Vec<u64>) -> Ack {
        if seq <= self.recv_high {
            // Duplicate of an already-applied frame: re-ack it.
            return self.current_ack();
        }
        if seq == self.recv_high + 1 {
            self.applied.push_back(payload);
            self.recv_high += 1;
            self.recv_bits >>= 1;
            // Everything now contiguous applies, in order.
            while self.recv_bits & 1 == 1 {
                if let Some(next) = self.buffered.remove(&(self.recv_high + 1)) {
                    self.applied.push_back(next);
                }
                self.recv_high += 1;
                self.recv_bits >>= 1;
            }
        } else {
            // Future frame: buffer it and note the hole.
            self.buffered.insert(seq, payload);
            let gap = seq - self.recv_high - 1; // ≥ 1
            if gap <= 31 {
                self.recv_bits |= 1 << gap;
            }
        }
        self.current_ack()
    }

    fn current_ack(&self) -> Ack {
        Ack {
            high: self.recv_high,
            bits: self.recv_bits,
        }
    }

    pub fn unacked_count(&self) -> usize {
        self.unacked.len()
    }

    pub fn applied_count(&self) -> usize {
        self.applied.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::RegionCoord;
    use crate::settlement::{Aggregate, Settlement, SettlementState};

    fn wire_settlements() -> Settlements {
        let mk = |id: u64, x: i32, z: i32| Settlement {
            id,
            name: "Rep",
            center: RegionCoord { x, z },
            state: SettlementState::Aggregate,
            aggregate: Aggregate {
                population: 40,
                food: 90,
                defense: 25,
                prosperity: 55,
            },
        };
        Settlements {
            list: vec![mk(1, 0, 0), mk(2, 3, 0), mk(3, 50, 50)],
        }
    }

    /// Out-of-order and lost frames: 1,2,4 delivered — 4 buffers; when
    /// 3 finally arrives, 3 then 4 apply in order; duplicates re-ack.
    #[test]
    fn p3d802_gap_buffered_in_order_delivery() {
        let mut rx = ReliableChannel::new();
        let a = rx.receive(1, vec![1]);
        assert_eq!(a, Ack { high: 1, bits: 0 });
        assert_eq!(rx.applied_count(), 1);

        let a = rx.receive(4, vec![4]); // 3 missing
        assert_eq!(a.high, 1);
        assert_eq!(a.bits, 0b100, "hole at 3 flagged (bit 2)");
        assert_eq!(rx.applied_count(), 1, "4 must not apply before 3");

        let a = rx.receive(2, vec![2]); // still blocked at 3
        assert_eq!(a.high, 2);
        assert_eq!(rx.applied_count(), 2);

        let a = rx.receive(3, vec![3]); // gap filled: 3 AND 4 apply
        assert_eq!(a.high, 4, "contiguous again");
        assert_eq!(rx.applied_count(), 4);

        // Duplicate 3: applied count unchanged, ack unchanged.
        let a = rx.receive(3, vec![3]);
        assert_eq!(a.high, 4);
        assert_eq!(rx.applied_count(), 4);
        // Order is real: applied log reads 1,2,3,4.
        let seqs: Vec<u64> = rx.applied.iter().map(|p| p[0]).collect();
        assert_eq!(seqs, vec![1, 2, 3, 4]);
    }

    /// The sender keeps payloads until acked; a cumulative+bitfield ack
    /// clears exactly the right entries; the rest are resent.
    #[test]
    fn p3d802_acks_clear_resend_queue() {
        let mut tx = ReliableChannel::new();
        for i in 1..=6 {
            tx.send(vec![i]);
        }
        assert_eq!(tx.unacked_count(), 6);

        // Peer got 1,2,3,4 contiguous, plus 6 (5 lost): ack high=4;
        // bit i=1 (0b10) means high+1+1 = 6 was received.
        tx.ack(Ack {
            high: 4,
            bits: 0b10,
        });
        assert_eq!(tx.unacked_count(), 1, "only 5 remains unacked");
        let resend: Vec<u64> = tx.needs_resend().iter().map(|(s, _)| *s).collect();
        assert_eq!(resend, vec![5]);

        // Peer recovers 5 and 6: high=6 clears everything.
        tx.ack(Ack { high: 6, bits: 0 });
        assert_eq!(tx.unacked_count(), 0);
        assert!(tx.needs_resend().is_empty());
    }

    /// Snapshots replicate only in-radius settlements; mirrors apply
    /// them monotonically (stale rejected) and converge to host values.
    #[test]
    fn p3d802_interest_snapshots_converge_the_mirror() {
        let mut s = wire_settlements();
        let player = RegionCoord { x: 0, z: 0 };

        let snap1 = interest_snapshot(&s, player, 10, 100);
        assert_eq!(snap1.values.len(), 2, "settlement 3 is out of interest");
        assert!(snap1.values.contains_key(&1) && snap1.values.contains_key(&2));

        let mut mirror = Mirror::default();
        assert!(mirror.apply(&snap1));
        assert_eq!(mirror.values[&1], [40, 90, 25, 55]);

        // World advances; prosperity rises; a fresh snapshot converges.
        s.list[0].aggregate.prosperity = 61;
        let snap2 = interest_snapshot(&s, player, 10, 200);
        assert!(mirror.apply(&snap2));
        assert_eq!(mirror.values[&1][3], 61, "mirror follows the host");
        assert_eq!(mirror.last_tick, 200);

        // Stale snapshot (old tick) rejected.
        let stale = interest_snapshot(&s, player, 10, 150);
        assert!(!mirror.apply(&stale), "older snapshot must not regress");
        assert_eq!(mirror.values[&1][3], 61);

        // Player moved out of range of everything: the mirror follows
        // the new interest set and drops what left it.
        s.list[1].aggregate.defense = 30;
        let moved = interest_snapshot(&s, RegionCoord { x: 0, z: 30 }, 10, 300);
        assert_eq!(moved.values.len(), 0, "nothing in the new radius");
        assert!(mirror.apply(&moved));
        assert!(mirror.values.is_empty(), "mirror tracks the interest set");
    }
}
