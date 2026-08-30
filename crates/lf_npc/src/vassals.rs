//! king-quest: the Vassal system — the player's path from wanderer to
//! liege. A villager whose faction stands at Honored (+75) can be sworn
//! into the player's service (sneak-use on them). Vassals keep their
//! home and post, work their trade for the player each in-game day, and
//! stack the yield for the liege to collect (sneak-use on the vassal).
//!
//! Everything here is pure and deterministic (day-seeded yields), so the
//! economy is testable and save/reload honest.

use serde::{Deserialize, Serialize};

use crate::VillagerJob;

/// What a vassal does for the liege, derived from their old trade.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerKind {
    Miner,
    Lumberjack,
    Farmer,
}

/// The sworn vassal's working state, persisted on the villager.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VassalState {
    pub worker: WorkerKind,
    pub sworn_day: u32,
    /// Yield stacked for the liege, not yet collected.
    pub stock: Vec<(String, u32)>,
}

/// The trade each job drills its vassals in.
pub fn worker_kind_for_job(job: VillagerJob) -> WorkerKind {
    use VillagerJob as J;
    match job {
        J::Smith => WorkerKind::Miner,
        J::Farmer => WorkerKind::Farmer,
        J::Lorekeeper | J::Wizard => WorkerKind::Miner,
        J::Guard | J::Trader | J::Bard => WorkerKind::Lumberjack,
    }
}

/// One day's honest work: (item, base count). A d10-style hash roll adds
/// +0..2 and occasionally a rare find, keyed by (villager id, day) so the
/// same vassal/day always yields the same haul.
fn yield_table(worker: WorkerKind) -> &'static [(&'static str, u32)] {
    match worker {
        WorkerKind::Miner => &[("stone", 3), ("coal", 2), ("iron_ingot", 1)],
        WorkerKind::Lumberjack => &[("log", 4), ("stick", 2)],
        WorkerKind::Farmer => &[("apple", 3)],
    }
}

/// Swear a villager in. `day` is the in-game day of the oath.
pub fn recruit(job: VillagerJob, day: u32) -> VassalState {
    VassalState {
        worker: worker_kind_for_job(job),
        sworn_day: day,
        stock: Vec::new(),
    }
}

/// Whether the player may swear this villager in: the lore gate is
/// Honored standing (+75), enforced by the caller with the real standing.
pub fn can_recruit(standing: i32, already_vassal: bool) -> bool {
    standing >= 75 && !already_vassal
}

/// Run one day of work for a vassal. Deterministic per (id, day).
/// Returns the day's haul; the caller also finds it appended to `stock`.
pub fn work_day(state: &mut VassalState, id: u64, day: u32) -> Vec<(String, u32)> {
    let table = yield_table(state.worker);
    let mut haul = Vec::new();
    for (i, (item, base)) in table.iter().enumerate() {
        // hash-roll: +0..2 of each line, and every third line only on a
        // "good day" (id+day hash) — sparse enough to feel earned
        let roll = (id
            .wrapping_mul(0x9E3779B97F4A7C15)
            .wrapping_add((day as u64).wrapping_mul(0xBF58476D1CE4E5B9))
            .wrapping_add(i as u64 * 0x2545F4914F6CDD1D))
            % 100;
        let mut count = *base + (roll % 3) as u32;
        if i > 0 && roll < 35 {
            continue; // the rare line only pays sometimes
        }
        if roll > 92 {
            count += 2; // an unusually good shift
        }
        haul.push((item.to_string(), count));
    }
    for (item, count) in &haul {
        push_stock(&mut state.stock, item, *count);
    }
    haul
}

/// The liege collects the stacked yield. Returns everything and empties.
pub fn collect(state: &mut VassalState) -> Vec<(String, u32)> {
    std::mem::take(&mut state.stock)
}

fn push_stock(stock: &mut Vec<(String, u32)>, item: &str, count: u32) {
    if let Some(entry) = stock.iter_mut().find(|(i, _)| i == item) {
        entry.1 += count;
    } else {
        stock.push((item.to_string(), count));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Villager, VillagerJob};

    /// Failure meaning: the oath gate or the job->trade mapping broke.
    #[test]
    fn vassal_recruitment_gates_and_trades() {
        assert!(!can_recruit(50, false), "below Honored: no oath");
        assert!(can_recruit(75, false), "at Honored the oath opens");
        assert!(!can_recruit(100, true), "already sworn");
        let smith = recruit(VillagerJob::Smith, 3);
        assert_eq!(smith.worker, WorkerKind::Miner);
        assert_eq!(smith.sworn_day, 3);
        assert!(smith.stock.is_empty());
        assert_eq!(worker_kind_for_job(VillagerJob::Farmer), WorkerKind::Farmer);
        assert_eq!(worker_kind_for_job(VillagerJob::Guard), WorkerKind::Lumberjack);
    }

    /// Failure meaning: a vassal's daily work is empty, non-deterministic,
    /// or stacks wrong.
    #[test]
    fn vassals_work_and_the_liege_collects() {
        let mut state = recruit(VillagerJob::Guard, 1); // guards swing axes for the liege
        let day1 = work_day(&mut state, 42, 5);
        assert!(!day1.is_empty(), "a vassal's day must produce something");
        assert!(day1.iter().any(|(i, _)| i == "log"), "lumberjacks bring wood");
        let day1_again = work_day(&mut VassalState { stock: Vec::new(), ..state.clone() }, 42, 5);
        assert_eq!(day1, day1_again, "same vassal+day must yield the same haul");
        // stock accumulates across days until collected
        let before: u32 = state.stock.iter().map(|(_, c)| c).sum();
        assert!(before > 0);
        let stacked = state.stock.len();
        let collected = collect(&mut state);
        assert_eq!(collected.len(), stacked, "collect empties the stock");
        assert!(state.stock.is_empty());
        assert!(collected.iter().all(|(i, c)| *c > 0));
    }

    /// Failure meaning: vassal state does not ride the villager JSON save.
    #[test]
    fn vassal_state_survives_the_save() {
        let mut v = Villager::new(11, VillagerJob::Smith, "Harl".into(), [4.0, 64.0, 4.0]);
        v.vassal = Some(recruit(VillagerJob::Smith, 2));
        work_day(v.vassal.as_mut().unwrap(), 11, 4);
        let bytes = serde_json::to_vec(&v).unwrap();
        let loaded: Villager = serde_json::from_slice(&bytes).unwrap();
        let vs = loaded.vassal.expect("vassal state survives the save");
        assert_eq!(vs.worker, WorkerKind::Miner);
        assert!(!vs.stock.is_empty(), "worked stock survives too");
    }
}
