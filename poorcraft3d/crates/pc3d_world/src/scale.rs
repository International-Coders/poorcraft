//! P3D-806: the player-scale proof — 4, 16, 32, 64, 128.
//!
//! D-029 says a 128-player world is an allowed TARGET only when the
//! server can sustain it, and scale claims must be staged and MEASURED.
//! This harness measures: N simulated players, each with deterministic
//! placement, their own interest snapshot per tick, and reliable-channel
//! traffic for what they would receive. The laws under test:
//!
//! 1. PER-PLAYER cost is independent of N (interest management bounds
//!    what any one player replicates — that is the whole point).
//! 2. TOTAL tick cost grows linearly (or better) in N.
//! 3. The measured rows are reported verbatim: "128 sustains" is a
//!    conclusion the rows must earn, never an assertion.

use crate::coords::RegionCoord;
use crate::host::{HostCommand, SoloHost, TICKS_PER_DAY};
use crate::machines::MachineKind;
use crate::replicate::{interest_snapshot, ReliableChannel};

/// One measured row of the scale proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScaleRow {
    pub players: usize,
    /// Average host-tick cost in microseconds (wall clock — the one
    /// number here that is measured, not derived).
    pub avg_tick_micros: u64,
    /// Interest snapshots built per tick (== player count).
    pub snapshots_per_tick: usize,
    /// Snapshot map-entries shipped per tick across all players.
    pub bytes_per_tick: usize,
    /// Largest interest set any ONE player held — must not grow with N.
    pub max_entries_per_player: usize,
}

/// Deterministic player placement: a spread over the world that never
/// depends on N (player i sits at the same region at every scale).
fn player_region(i: usize) -> RegionCoord {
    let x = ((i * 37 + 11) % 61) as i32 - 30;
    let z = ((i * 23 + 5) % 61) as i32 - 30;
    RegionCoord { x, z }
}

/// Measure one scale: N players, each tick builds every player's
/// interest snapshot (radius 12) and queues its bytes on a reliable
/// channel. Runs `ticks` host ticks (with a little world life in them).
pub fn measure_scale(seed: u64, players: usize, ticks: u64) -> ScaleRow {
    let mut host = SoloHost::new(seed);
    // A little industry so snapshots watch a living world.
    let boiler = host.machines.add_machine(MachineKind::Boiler);
    let engine = host.machines.add_machine(MachineKind::SteamEngine);
    host.machines.connect(boiler, engine).expect("typed wire");
    host.submit(HostCommand::FeedBoiler { machine: boiler, fuel_milli: i64::MAX / 8, water_milli: i64::MAX / 8 });

    let regions: Vec<RegionCoord> = (0..players).map(player_region).collect();
    let mut channels: Vec<ReliableChannel> =
        (0..players).map(|_| ReliableChannel::new()).collect();

    let mut total_entries: usize = 0;
    let mut max_entries: usize = 0;
    let mut micros: u128 = 0;

    for t in 0..ticks {
        let t0 = std::time::Instant::now();
        host.run_ticks(1);
        for (i, r) in regions.iter().enumerate() {
            let snap = interest_snapshot(&host.settlements, *r, 12, host.tick);
            let entries = snap.values.len();
            max_entries = max_entries.max(entries);
            total_entries += entries;
            // The wire cost: 8 bytes per entry + 8 tick stamp (the
            // values are i64 bundles).
            let mut frame: Vec<u64> = vec![snap.tick];
            for (id, v) in &snap.values {
                frame.push(*id);
                frame.extend(v.iter().map(|x| *x as u64));
            }
            let _ = channels[i].send(frame);
        }
        micros += t0.elapsed().as_micros();
    }

    ScaleRow {
        players,
        avg_tick_micros: (micros / ticks.max(1) as u128) as u64,
        snapshots_per_tick: players,
        bytes_per_tick: total_entries * 8 / ticks.max(1) as usize,
        max_entries_per_player: max_entries,
    }
}

/// The staged proof: measure each count in order.
pub fn scale_proof(seed: u64, counts: &[usize]) -> Vec<ScaleRow> {
    let ticks = 5u64;
    counts.iter().map(|n| measure_scale(seed, *n, ticks)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The interest-management law: the largest interest set any ONE
    /// player holds is a constant of the WORLD, not of N — at 128
    /// players nobody replicates more than at 4.
    #[test]
    fn p3d806_per_player_cost_is_independent_of_scale() {
        let rows = scale_proof(4242, &[4, 16, 32, 64, 128]);
        let per_player_at_4 = rows[0].max_entries_per_player;
        for row in &rows {
            assert!(
                row.max_entries_per_player <= per_player_at_4 + 1,
                "player {} replicates {} entries vs {} at scale 4 — interest leaked",
                row.players, row.max_entries_per_player, per_player_at_4
            );
        }
        // And the structure is honest: snapshots per tick == players.
        for row in &rows {
            assert_eq!(row.snapshots_per_tick, row.players);
        }
    }

    /// D-029, measured: total tick cost grows at most linearly in N
    /// (generous 4× slack over perfect scaling for wall-clock noise),
    /// and 128 players sustain under an explicit per-tick budget. The
    /// rows are printed so the claim travels with its evidence.
    #[test]
    fn p3d806_total_cost_grows_linearly_and_128_sustains() {
        let rows = scale_proof(4242, &[4, 16, 32, 64, 128]);
        for row in &rows {
            println!(
                "scale {:>3} players: tick {:>4} µs, bytes/tick {:>3}, max/player {}",
                row.players, row.avg_tick_micros, row.bytes_per_tick, row.max_entries_per_player
            );
        }
        let base = rows[0].avg_tick_micros.max(1);
        let perfect = rows[rows.len() - 1].players as f64 / rows[0].players as f64;
        let worst = rows[rows.len() - 1].avg_tick_micros as f64;
        assert!(
            worst <= base as f64 * perfect * 4.0,
            "128-player tick cost {worst} µs exceeded linear budget {} µs",
            base as f64 * perfect * 4.0
        );
        // The explicit D-029 budget: the 128-player host tick stays
        // under 20 ms (one 60 Hz frame).
        assert!(
            rows[rows.len() - 1].avg_tick_micros < 20_000,
            "128 players: {} µs per tick breaks the frame budget",
            rows[rows.len() - 1].avg_tick_micros
        );
    }

    /// Scale runs are structurally reproducible: same seed, same
    /// player placement, same replication loads — only the measured
    /// wall-clock column is allowed to breathe.
    #[test]
    fn p3d806_rows_reproducible() {
        let a = scale_proof(777, &[4, 128]);
        let b = scale_proof(777, &[4, 128]);
        assert_eq!(a.len(), b.len());
        for (ra, rb) in a.iter().zip(b.iter()) {
            assert_eq!(ra.players, rb.players);
            assert_eq!(ra.snapshots_per_tick, rb.snapshots_per_tick);
            assert_eq!(ra.bytes_per_tick, rb.bytes_per_tick);
            assert_eq!(ra.max_entries_per_player, rb.max_entries_per_player);
            assert!(ra.avg_tick_micros > 0 && rb.avg_tick_micros > 0);
        }
    }
}
