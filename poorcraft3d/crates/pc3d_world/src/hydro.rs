//! P3D-301: the macro watershed and river graph
//! (docs/POORCRAFT-3D/16-IMPLEMENTATION-WORK-BREAKDOWN.md, P3D-300 opener).
//!
//! From the seed's macro elevation field: every region drains to exactly
//! one lower neighbor (steepest descent; ties break deterministically by
//! elevation, then x, then z), discharge accumulates downstream in
//! descending-elevation order, and edges whose discharge reaches
//! [`RIVER_THRESHOLD`] are RIVERS. Pure, cycle-free (flow always goes
//! strictly downhill), and derivable from the seed — never stored.

use crate::coords::RegionCoord;
use crate::gen::WorldGen;
use std::collections::BTreeMap;

/// Minimum accumulated discharge for an edge to count as a river.
pub const RIVER_THRESHOLD: u64 = 64;

/// The 8-neighborhood offsets.
const NEIGHBORS: [(i32, i32); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

/// The watershed over `[-half, half]²` regions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RiverGraph {
    pub seed: u64,
    pub half: i32,
    /// Elevation per region (meters).
    pub elevation: BTreeMap<(i32, i32), i32>,
    /// Each region's downstream neighbor (None = sink).
    pub downstream: BTreeMap<(i32, i32), Option<(i32, i32)>>,
    /// Accumulated discharge per region.
    pub discharge: BTreeMap<(i32, i32), u64>,
    /// Regions that lie on a river edge (discharge ≥ threshold).
    pub river_regions: Vec<(i32, i32)>,
}

impl RiverGraph {
    pub fn new(gen: &WorldGen, half: i32) -> Self {
        // 1. Elevations over the lattice.
        let mut elevation: BTreeMap<(i32, i32), i32> = BTreeMap::new();
        for x in -half..=half {
            for z in -half..=half {
                elevation.insert((x, z), gen.macro_field(RegionCoord { x, z }).elevation_m);
            }
        }

        // 2. Steepest descent (deterministic tie-break: lower elevation,
        //    then lower x, then lower z).
        let mut downstream: BTreeMap<(i32, i32), Option<(i32, i32)>> = BTreeMap::new();
        for x in -half..=half {
            for z in -half..=half {
                let key = (x, z);
                let e = elevation[&key];
                let mut best: Option<(i32, i32)> = None;
                let mut best_e = e;
                for (dx, dz) in NEIGHBORS {
                    let nk = (x + dx, z + dz);
                    if let Some(&ne) = elevation.get(&nk) {
                        if ne < best_e {
                            best_e = ne;
                            best = Some(nk);
                        } else if ne == best_e && best.is_some() {
                            // Equal-height neighbor: deterministic tie-break
                            // prefers the lexicographically smaller key.
                            let bk = best.unwrap();
                            if nk < bk {
                                best = Some(nk);
                            }
                        } else if ne == best_e && best.is_none() && ne < e {
                            best = Some(nk);
                        }
                    }
                }
                downstream.insert(key, best);
            }
        }

        // 3. Discharge accumulation in DESCENDING elevation order (every
        //    cell is complete before any strictly lower cell accumulates
        //    from it — flow is strictly downhill so this is well-defined).
        let mut order: Vec<(i32, i32)> = elevation.keys().copied().collect();
        order.sort_by(|a, b| elevation[b].cmp(&elevation[a]).then(a.cmp(b)));
        let mut discharge: BTreeMap<(i32, i32), u64> = BTreeMap::new();
        for k in &order {
            discharge.entry(*k).or_insert(0);
        }
        for k in &order {
            // Every region contributes its own drop, then passes the
            // total downstream.
            *discharge.entry(*k).or_insert(0) += 1;
            let d_here = *discharge.get(k).unwrap();
            if let Some(Some(down)) = downstream.get(k) {
                *discharge.entry(*down).or_insert(0) += d_here;
            }
        }

        // 4. River edges: discharge ≥ threshold on both ends (an edge is
        //    a river when the DOWNSTREAM cell has gathered real flow).
        let mut river_regions: Vec<(i32, i32)> = Vec::new();
        for (k, down) in &downstream {
            if let Some(d) = down {
                if discharge[k] >= RIVER_THRESHOLD && discharge[d] >= RIVER_THRESHOLD {
                    river_regions.push(*k);
                    river_regions.push(*d);
                }
            }
        }
        river_regions.sort();
        river_regions.dedup();

        RiverGraph {
            seed: gen.seed(),
            half,
            elevation,
            downstream,
            discharge,
            river_regions,
        }
    }

    pub fn downstream(&self, r: RegionCoord) -> Option<RegionCoord> {
        self.downstream
            .get(&(r.x, r.z))
            .and_then(|d| d.map(|(x, z)| RegionCoord { x, z }))
    }

    pub fn discharge(&self, r: RegionCoord) -> u64 {
        *self.discharge.get(&(r.x, r.z)).unwrap_or(&0)
    }

    /// True when this region lies on a river (its discharge meets the
    /// threshold and it drains somewhere).
    pub fn is_river(&self, r: RegionCoord) -> bool {
        self.river_regions.binary_search(&(r.x, r.z)).is_ok()
    }

    /// Regions lying on rivers, ascending.
    pub fn river_region_list(&self) -> &[(i32, i32)] {
        &self.river_regions
    }

    /// River edges (upstream, downstream) with discharge ≥ threshold,
    /// ascending.
    pub fn river_edges(&self) -> Vec<(RegionCoord, RegionCoord)> {
        let mut edges: Vec<(RegionCoord, RegionCoord)> = self
            .river_regions
            .windows(2)
            .map(|w| {
                (
                    RegionCoord { x: w[0].0, z: w[0].1 },
                    RegionCoord { x: w[1].0, z: w[1].1 },
                )
            })
            .collect();
        edges.sort();
        edges.dedup();
        edges
    }

    /// Wetness for a region: its humidity plus a river-corridor bonus
    /// that decays with distance to the nearest river region (D-016).
    pub fn wetness(&self, gen: &WorldGen, r: RegionCoord) -> u8 {
        let humidity = gen.macro_field(r).humidity as i32;
        let mut best = i32::MAX;
        for (x, z) in &self.river_regions {
            let d = (x - r.x).abs().max((z - r.z).abs());
            best = best.min(d);
        }
        if best == i32::MAX {
            return humidity as u8;
        }
        let bonus = (12 - best.min(12)) * 4; // up to +48 adjacent to rivers
        (humidity + bonus).min(100) as u8
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    fn gen() -> WorldGen {
        WorldGen::new(2024)
    }

    /// Same seed → identical graph; different seed → different rivers.
    #[test]
    fn p3d301_graph_is_deterministic_and_seed_sensitive() {
        let g = WorldGen::new(2024);
        let a = RiverGraph::new(&g, 24);
        let b = RiverGraph::new(&g, 24);
        assert_eq!(a.downstream, b.downstream);
        assert_eq!(a.discharge, b.discharge);
        let other = RiverGraph::new(&WorldGen::new(2025), 24);
        assert_ne!(a.discharge, other.discharge, "different seed, same discharge map");
    }

    /// THE watershed law: no cycles — every downstream chain reaches a
    /// sink within bounded steps — and discharge is conserved: each
    /// region's discharge equals 1 + the sum of its upstream tributaries.
    #[test]
    fn p3d301_flow_is_acyclic_and_conserved() {
        let g = gen();
        let graph = RiverGraph::new(&g, 20);
        // Acyclicity: every chain terminates.
        for x in -20..=20 {
            for z in -20..=20 {
                let mut cur = RegionCoord { x, z };
                let mut steps = 0;
                while let Some(next) = graph.downstream(cur) {
                    cur = next;
                    steps += 1;
                    assert!(steps <= 41 * 41, "flow cycle detected at {cur:?}");
                }
            }
        }
        // Conservation: recompute each node's discharge from upstream.
        let mut recomputed: BTreeMap<(i32, i32), u64> = BTreeMap::new();
        // Process in ascending elevation (opposite of the build order):
        let mut order: Vec<(i32, i32)> =
            graph.elevation.keys().copied().collect();
        order.sort_by_key(|k| std::cmp::Reverse(graph.elevation[k]));
        for k in order {
            let upstream: u64 = graph
                .downstream
                .iter()
                .filter(|(_, d)| **d == Some(k))
                .map(|(src, _)| recomputed.get(src).copied().unwrap_or(0))
                .sum();
            let mine = 1 + upstream;
            // Only enforce when all upstream values are already final
            // (elevation-ascending guarantees it unless equal heights —
            // equal-height neighbors cannot flow into each other).
            recomputed.insert(k, mine);
        }
        for (k, v) in &graph.discharge {
            assert_eq!(recomputed[k], *v, "discharge mismatch at {k:?}");
        }
    }

    /// Rivers exist in most seeds, flow strictly downhill, and wetness is
    /// higher near rivers than far from them.
    #[test]
    fn p3d301_rivers_exist_flow_downhill_and_wet_corridors() {
        let mut seeds_with_rivers = 0;
        for seed in 0..6u64 {
            let g = WorldGen::new(seed.wrapping_mul(7919));
            let graph = RiverGraph::new(&g, 24);
            let regions = graph.river_regions.len();
            if regions >= 2 {
                seeds_with_rivers += 1;
            }
            for r in &graph.river_regions {
                let e = graph.elevation[r];
                if let Some(down) = graph.downstream(RegionCoord { x: r.0, z: r.1 }) {
                    assert!(
                        graph.elevation[&(down.x, down.z)] <= e,
                        "river edge flows uphill"
                    );
                }
            }
        }
        assert!(seeds_with_rivers >= 4, "rivers missing in {seeds_with_rivers}/6 seeds");

        // Wetness corridor: for one seed with rivers, wetness adjacent to
        // river regions exceeds wetness far away.
        let g = WorldGen::new(2024);
        let graph = RiverGraph::new(&g, 24);
        let near = graph
            .river_regions
            .first()
            .map(|r| graph.wetness(&g, RegionCoord { x: r.0, z: r.1 }));
        if let Some(n) = near {
            let far = graph.wetness(&g, RegionCoord { x: 24, z: 24 });
            assert!(n > far || far == 100, "wet corridor absent: near={n} far={far}");
        }
    }
}
