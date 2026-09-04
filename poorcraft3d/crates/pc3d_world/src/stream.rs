//! P3D-105: interest management and bounded streaming queues.
//!
//! The blueprint's streaming contract: concentric interest rings around
//! every viewer (tier radii are proposal constants), bounded work queues
//! with a visible backlog — a fast flight or teleport must not create
//! unlimited meshing work or freeze the frame. Everything here is pure
//! bookkeeping; the mesh stage (P3D-201+) fills the queues with real jobs.

use crate::coords::{PatchCoord, WorldPos};
use crate::scales::{MAX_QUERY_PATCHES, PATCH_MM};

/// Interest tier radii in meters (blueprint proposals; configurable by
/// editing here, benchmarked in P3D-201+).
pub const TIER_FULL_M: f32 = 96.0;
pub const TIER_LOD_M: f32 = 320.0;
pub const TIER_MACRO_M: f32 = 1024.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Full detail: data, collision, edits, detailed mesh.
    Full,
    /// Lower-resolution mesh + interaction summaries.
    Lod,
    /// Macro mesh + landmarks, no detailed collision.
    Macro,
}

impl Tier {
    pub fn radius_mm(self) -> i64 {
        let m = match self {
            Tier::Full => TIER_FULL_M,
            Tier::Lod => TIER_LOD_M,
            Tier::Macro => TIER_MACRO_M,
        };
        (m * 1000.0) as i64
    }
}

/// Patches whose CENTERS lie within `radius` (millimeters) of the viewer's
/// x/z position — strictly ascending, no duplicates, capped like every
/// query. Y is included (a viewer cares about the vertical column they can
/// touch), so the set is a disc × full column... no: patch centers are
/// checked in x/z only, and every y level of a matching column is listed.
/// That is bounded (columns × 1 y per call site convention: callers pass
/// the tiers they want; this returns y=0 columns and callers expand).
/// We return y = 0 columns; vertical expansion is the consumer's job.
pub fn interest_patches(
    viewer: WorldPos,
    tier: Tier,
) -> Result<Vec<PatchCoord>, crate::query::QueryError> {
    let radius = tier.radius_mm();
    let patch_span = PATCH_MM;
    // Bounding box of the disc in patch coordinates, then exact distance
    // filter on patch centers.
    let min = WorldPos::from_mm(viewer.x - radius, 0, viewer.z - radius).patch();
    let max = WorldPos::from_mm(viewer.x + radius, 0, viewer.z + radius).patch();
    let nx = (max.x - min.x + 1) as u64;
    let nz = (max.z - min.z + 1) as u64;
    if nx.saturating_mul(nz) > MAX_QUERY_PATCHES as u64 {
        return Err(crate::query::QueryError::TooManyPatches {
            requested: nx.saturating_mul(nz),
            cap: MAX_QUERY_PATCHES,
        });
    }
    let r2 = i64::from(radius.checked_mul(radius).ok_or(crate::query::QueryError::TooLarge)?);
    let mut out = Vec::new();
    for px in min.x..=max.x {
        for pz in min.z..=max.z {
            let center = WorldPos::from_mm(
                px as i64 * patch_span + patch_span / 2,
                0,
                pz as i64 * patch_span + patch_span / 2,
            );
            let dx = center.x - viewer.x;
            let dz = center.z - viewer.z;
            if dx * dx + dz * dz <= r2 {
                out.push(PatchCoord { x: px, y: 0, z: pz });
            }
        }
    }
    Ok(out)
}

/// The load/unload plan moving from one interest set to another. Both
/// inputs must be sorted ascending (as `interest_patches` produces);
/// outputs are sorted and disjoint.
pub fn interest_diff(
    previous: &[PatchCoord],
    current: &[PatchCoord],
) -> (Vec<PatchCoord>, Vec<PatchCoord>) {
    let mut to_load = Vec::new();
    let mut to_unload = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < previous.len() && j < current.len() {
        match previous[i].cmp(&current[j]) {
            std::cmp::Ordering::Less => {
                to_unload.push(previous[i]);
                i += 1;
            }
            std::cmp::Ordering::Greater => {
                to_load.push(current[j]);
                j += 1;
            }
            std::cmp::Ordering::Equal => {
                i += 1;
                j += 1;
            }
        }
    }
    to_unload.extend_from_slice(&previous[i..]);
    to_load.extend_from_slice(&current[j..]);
    (to_load, to_unload)
}

/// Fixed-capacity work queue. Overflow REJECTS with the item handed back —
/// work is never silently dropped and never grows unbounded; the counters
/// keep the backlog honest.
#[derive(Debug)]
pub struct BoundedQueue<T> {
    cap: usize,
    items: std::collections::VecDeque<T>,
    pub pushed: u64,
    pub admitted: u64,
    pub rejected: u64,
    pub popped: u64,
}

/// Result of a bounded push.
#[derive(Debug, PartialEq, Eq)]
pub enum Admit<T> {
    Admitted,
    RejectedFull(T),
}

impl<T> BoundedQueue<T> {
    pub fn new(cap: usize) -> Self {
        BoundedQueue {
            cap: cap.max(1),
            items: std::collections::VecDeque::new(),
            pushed: 0,
            admitted: 0,
            rejected: 0,
            popped: 0,
        }
    }

    pub fn push(&mut self, item: T) -> Admit<T> {
        self.pushed += 1;
        if self.items.len() >= self.cap {
            self.rejected += 1;
            Admit::RejectedFull(item)
        } else {
            self.items.push_back(item);
            self.admitted += 1;
            Admit::Admitted
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        let item = self.items.pop_front();
        if item.is_some() {
            self.popped += 1;
        }
        item
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewer_at_m(x: i64, z: i64) -> WorldPos {
        WorldPos::from_mm(x * 1000, 0, z * 1000)
    }

    /// Ring membership: a patch center inside the radius is in, just
    /// outside is out; the set is ascending without duplicates; negative
    /// coordinates behave.
    #[test]
    fn p3d105_interest_rings_membership_and_order() {
        let viewer = viewer_at_m(0, 0);
        let full = interest_patches(viewer, Tier::Full).expect("full tier");
        assert!(!full.is_empty());
        // Full tier radius 96 m: patch centers within 96 m of origin.
        // Patch centers sit at (16k + 8) m; centers at ±8, ±24, ... ±88 m
        // qualify (88² = 7744 < 96²), ±104 do not.
        let radius = TIER_FULL_M * 1000.0;
        for p in &full {
            let cx = p.x as i64 * PATCH_MM + PATCH_MM / 2;
            let cz = p.z as i64 * PATCH_MM + PATCH_MM / 2;
            assert!(cx * cx + cz * cz <= radius as i64 * radius as i64);
        }
        // Ascending, no duplicates.
        let mut sorted = full.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted, full);
        // Far-away viewer still works (no origin bias).
        let far = interest_patches(viewer_at_m(-100_000, 250_000), Tier::Full)
            .expect("far viewer");
        assert!(!far.is_empty());
        assert!(far.iter().all(|p| p.x < 0 || p.z > 0));
        // Macro tier is strictly larger than the full tier.
        let macro_set = interest_patches(viewer, Tier::Macro).expect("macro");
        assert!(macro_set.len() > full.len() * 10);
    }

    /// The queue never exceeds capacity: overflow returns the item, counters
    /// stay honest, pops drain in order.
    #[test]
    fn p3d105_bounded_queue_rejects_and_counts() {
        let mut q: BoundedQueue<u32> = BoundedQueue::new(3);
        for i in 0..3 {
            assert_eq!(q.push(i), Admit::Admitted);
        }
        assert_eq!(q.push(99), Admit::RejectedFull(99), "overflow returns the item");
        assert_eq!(q.len(), 3);
        assert_eq!((q.pushed, q.admitted, q.rejected), (4, 3, 1));
        assert_eq!(q.pop(), Some(0), "FIFO order");
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.popped, 2);
        assert_eq!(q.push(5), Admit::Admitted);
        assert_eq!(q.len(), 2);
        // Drain: 2 more pops (total 4 — popped counts every successful pop).
        while q.pop().is_some() {}
        assert!(q.is_empty());
        assert_eq!(q.popped, 4);
    }

    /// THE teleport scenario: the interest set jumps across the map, the
    /// diff is large, the queue admits only its capacity — and the
    /// remainder is handed BACK to the caller (never lost), with counters
    /// showing the backlog. A later drain re-admits everything.
    #[test]
    fn p3d105_teleport_creates_bounded_not_lost_work() {
        let old = interest_patches(viewer_at_m(0, 0), Tier::Full).expect("old");
        let new_set = interest_patches(viewer_at_m(80_000, 0), Tier::Full).expect("new");
        let (to_load, to_unload) = interest_diff(&old, &new_set);
        assert!(!to_load.is_empty());
        assert_eq!(to_load.len(), new_set.len(), "jump leaves nothing loaded");
        assert_eq!(to_unload.len(), old.len(), "everything old unloads");

        // The mesh queue admits a bounded slice per frame.
        let mut mesh: BoundedQueue<PatchCoord> = BoundedQueue::new(8);
        let mut deferred: Vec<PatchCoord> = Vec::new();
        for p in &to_load {
            match mesh.push(*p) {
                Admit::Admitted => {}
                Admit::RejectedFull(p) => deferred.push(p),
            }
        }
        assert_eq!(mesh.len(), 8, "capacity respected");
        assert_eq!(deferred.len(), to_load.len() - 8);
        assert_eq!(mesh.rejected as usize, deferred.len());
        // Nothing was lost: admitted + deferred == the whole plan. Re-push
        // is GRADUAL (drain a frame, admit the next slice) — the second
        // wave fills the queue again and still overflow-deferred items
        // remain held by the caller, counted, never dropped.
        while mesh.pop().is_some() {}
        let mut still_deferred = Vec::new();
        for p in &deferred {
            if let Admit::RejectedFull(p) = mesh.push(*p) {
                still_deferred.push(p);
            }
        }
        assert_eq!(mesh.len(), 8, "second wave fills to capacity again");
        assert_eq!(still_deferred.len(), deferred.len() - 8);
        // The queue's counters account for every push attempt across both
        // waves: the full plan plus the deferred re-pushes.
        assert_eq!(
            mesh.pushed as usize,
            to_load.len() + deferred.len(),
            "every planned patch was pushed exactly once per wave"
        );
        assert_eq!(mesh.admitted as usize, 8 + (deferred.len() - still_deferred.len()));
        // Rejections accumulate across BOTH waves: wave 1's deferrals plus
        // wave 2's overflow.
        assert_eq!(mesh.rejected as usize, deferred.len() + still_deferred.len());
        assert!(mesh.rejected > 0, "the backlog stayed visible in counters");
    }

    /// Diffs are deterministic and symmetric-safe: same inputs, same plan;
    /// identical sets produce empty diffs.
    #[test]
    fn p3d105_diffs_are_deterministic() {
        let a = interest_patches(viewer_at_m(10, 20), Tier::Lod).expect("a");
        let b = interest_patches(viewer_at_m(30, 20), Tier::Lod).expect("b");
        let (l1, u1) = interest_diff(&a, &b);
        let (l2, u2) = interest_diff(&a, &b);
        assert_eq!(l1, l2);
        assert_eq!(u1, u2);
        let (l0, u0) = interest_diff(&a, &a);
        assert!(l0.is_empty() && u0.is_empty());
        // Loads and unloads never share an element.
        for p in &l1 {
            assert!(!u1.contains(p));
        }
    }
}
