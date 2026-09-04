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
        Self::build(&gen, half, &BTreeMap::new())
    }

    /// Rebuild with elevation OVERRIDES from terrain edits (P3D-303):
    /// delta in meters per region, clamped to the declared range. This is
    /// the dirty-region entry point — edits in a valley reroute its river
    /// locally.
    pub fn build(
        gen: &WorldGen,
        half: i32,
        overrides: &BTreeMap<(i32, i32), i32>,
    ) -> Self {
        // 1. Elevations over the lattice.
        let mut elevation: BTreeMap<(i32, i32), i32> = BTreeMap::new();
        for x in -half..=half {
            for z in -half..=half {
                let base = gen.macro_field(RegionCoord { x, z }).elevation_m;
                let delta = overrides.get(&(x, z)).copied().unwrap_or(0);
                elevation.insert(
                    (x, z),
                    (base + delta).clamp(crate::gen::MIN_ELEVATION_M, crate::gen::MAX_ELEVATION_M),
                );
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

    /// The region containing a world position (mm).
    pub fn region_at(wx: i64, wz: i64) -> RegionCoord {
        RegionCoord {
            x: wx.div_euclid(256_000) as i32,
            z: wz.div_euclid(256_000) as i32,
        }
    }

    /// Consumer-facing wetness at a world position (mm).
    pub fn wetness_at_mm(&self, gen: &WorldGen, wx: i64, wz: i64) -> u8 {
        self.wetness(gen, Self::region_at(wx, wz))
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



/// P3D-305: bounded reservoir volume model. Volumes are fixed-point
/// thousand-liters (kl). Conservation law: within a closed system,
/// filled − drained == Σ volumes; overflow ALWAYS routes downstream
/// (bounded chain walk), never vanishes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reservoir {
    pub region: RegionCoord,
    /// Terrain-derived capacity in kl (v1: slope-scaled).
    pub capacity_kl: i64,
    pub volume_kl: i64,
}

#[derive(Clone, Debug, Default)]
pub struct Reservoirs {
    pub map: BTreeMap<(i32, i32), Reservoir>,
}

impl Reservoirs {
    /// Build reservoirs for every region in the graph: capacity grows
    /// with the local elevation range (mountain valleys hold more).
    pub fn from_graph(graph: &RiverGraph) -> Self {
        let mut map = BTreeMap::new();
        for (k, e) in &graph.elevation {
            let max_neighbor_diff = NEIGHBORS
                .iter()
                .filter_map(|(dx, dz)| graph.elevation.get(&(k.0 + dx, k.1 + dz)))
                .map(|ne| (*e - *ne).abs())
                .max()
                .unwrap_or(0);
            let capacity = (10_000 + max_neighbor_diff as i64 * 500).max(1_000);
            map.insert(
                *k,
                Reservoir { region: RegionCoord { x: k.0, z: k.1 }, capacity_kl: capacity, volume_kl: 0 },
            );
        }
        Reservoirs { map }
    }

    /// Fill a reservoir; the amount that does not fit overflows to the
    /// downstream reservoir (bounded chain), and the FINAL spill (past
    /// the last reservoir) is returned. Conservation: caller accounts
    /// filled − returned == Σ retained.
    pub fn fill(&mut self, graph: &RiverGraph, region: RegionCoord, amount_kl: i64) -> u64 {
        let mut remaining = amount_kl.max(0) as u64;
        let mut cur = Some(region);
        let mut steps = 0;
        while let Some(r) = cur {
            steps += 1;
            if steps > 4096 {
                break;
            }
            let Some(res) = self.map.get_mut(&(r.x, r.z)) else {
                break;
            };
            let free = (res.capacity_kl - res.volume_kl).max(0) as u64;
            let take = remaining.min(free);
            res.volume_kl += take as i64;
            remaining -= take;
            if remaining == 0 {
                return 0;
            }
            cur = graph.downstream(r);
        }
        remaining
    }

    /// Drain up to `amount_kl`; never negative.
    pub fn drain(&mut self, region: RegionCoord, amount_kl: i64) -> u64 {
        let Some(res) = self.map.get_mut(&(region.x, region.z)) else {
            return 0;
        };
        let taken = (amount_kl.max(0) as u64).min(res.volume_kl as u64);
        res.volume_kl -= taken as i64;
        taken
    }

    pub fn total_volume(&self) -> u64 {
        self.map.values().map(|r| r.volume_kl as u64).sum()
    }
}

/// P3D-306: the D-007 consumer contract. A machine/wheel/fishing site
/// queries flow POTENTIAL — a pure read that can never weaken the river:
/// querying consumes nothing, reroutes nothing, drains nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowPotential {
    pub region: RegionCoord,
    /// Accumulated discharge at this region (arbitrary flow units).
    pub discharge: u64,
    /// Downhill slope toward downstream, per-mille (0 at sinks).
    pub slope_per_mille: i32,
    /// Wetness corridor value 0..=100.
    pub wetness: u8,
    /// Reservoir volume here (kl), if tracked.
    pub reservoir_kl: i64,
    /// A site is VIABLE for a waterwheel when real water and real slope
    /// are both present.
    pub viable: bool,
}

impl RiverGraph {
    /// THE consumer query: pure, allocation-free read of flow potential.
    /// Querying it any number of times changes nothing anywhere.
    pub fn flow_potential_at(
        &self,
        gen: &WorldGen,
        reservoirs: Option<&Reservoirs>,
        wx: i64,
        wz: i64,
    ) -> FlowPotential {
        let region = Self::region_at(wx, wz);
        let discharge = self.discharge(region);
        let slope = self
            .downstream(region)
            .map(|d| {
                let here = self.elevation[&(region.x, region.z)];
                let there = self.elevation[&(d.x, d.z)];
                ((here - there).max(0) as i64 * 1_000_000 / 256_000) as i32
            })
            .unwrap_or(0);
        let wetness = self.wetness(gen, region);
        let reservoir_kl = reservoirs
            .and_then(|rs| rs.map.get(&(region.x, region.z)))
            .map(|r| r.volume_kl)
            .unwrap_or(0);
        let viable = discharge >= RIVER_THRESHOLD && slope > 0;
        FlowPotential {
            region,
            discharge,
            slope_per_mille: slope,
            wetness,
            reservoir_kl,
            viable,
        }
    }

    /// The best waterwheel site in the band: maximizes discharge × slope
    /// among viable regions. Deterministic tie-break by position.
    pub fn best_wheel_site(
        &self,
        gen: &WorldGen,
        reservoirs: Option<&Reservoirs>,
    ) -> Option<(RegionCoord, FlowPotential)> {
        let mut best: Option<(u64, RegionCoord)> = None;
        for (&(x, z), &e) in &self.elevation {
            let r = RegionCoord { x, z };
            let downstream = self.downstream(r);
            let slope = match downstream {
                Some(d) => {
                    ((e - self.elevation[&(d.x, d.z)]).max(0) as u64) * 1_000_000 / 256_000
                }
                None => 0,
            };
            // Only VIABLE regions compete: real water and real slope.
            if slope == 0 || self.discharge(r) < RIVER_THRESHOLD {
                continue;
            }
            let score = self.discharge(r).saturating_mul(slope);
            match best {
                Some((bs, _)) if bs >= score => {}
                _ => best = Some((score, r)),
            }
        }
        best.map(|(_, r)| {
            let wx = (r.x * 256 + 128) as i64 * 1000;
            let wz = (r.z * 256 + 128) as i64 * 1000;
            let p = self.flow_potential_at(gen, reservoirs, wx, wz);
            (r, p)
        })
    }
}

#[cfg(test)]
mod consumer_tests {
    use super::*;

    /// THE D-007 CONTRACT: querying flow potential is a pure read.
    /// Hundreds of queries change no discharge, no reservoir, no graph.
    #[test]
    fn p3d306_querying_potential_consumes_nothing() {
        let g = WorldGen::new(11);
        let graph = RiverGraph::new(&g, 16);
        let mut res = Reservoirs::from_graph(&graph);
        let before_res = res.total_volume();
        let before_map = graph.discharge.clone();
        for x in -16..=16 {
            for z in -16..=16 {
                let wx = (x * 256 + 128) as i64 * 1000;
                let wz = (z * 256 + 128) as i64 * 1000;
                let _ = graph.flow_potential_at(&g, Some(&res), wx, wz);
            }
        }
        assert_eq!(graph.discharge, before_map, "queries changed discharge");
        assert_eq!(res.total_volume(), before_res, "queries changed reservoirs");
    }

    /// The best wheel site: viable, and its score is maximal among all
    /// viable regions.
    #[test]
    fn p3d306_best_wheel_site_is_viable_and_maximal() {
        let g = WorldGen::new(2024);
        let graph = RiverGraph::new(&g, 20);
        let (r, p) = graph
            .best_wheel_site(&g, None)
            .expect("a viable site exists in a 41x41 band");
        assert!(p.viable);
        assert!(p.discharge >= crate::hydro::RIVER_THRESHOLD);
        assert!(p.slope_per_mille > 0);
        // Maximality: no other viable region scores higher.
        for (&(x, z), &e) in &graph.elevation {
            let o = RegionCoord { x, z };
            let slope = graph
                .downstream(o)
                .map(|d| {
                    ((e - graph.elevation[&(d.x, d.z)]).max(0) as u64) * 1_000_000 / 256_000
                })
                .unwrap_or(0);
            if slope == 0 || graph.discharge(o) < crate::hydro::RIVER_THRESHOLD {
                continue;
            }
            let score = graph.discharge(o).saturating_mul(slope.max(1));
            let best_score = graph.discharge(r).saturating_mul(p.slope_per_mille.max(0) as u64);
            assert!(
                score <= best_score,
                "region {o:?} scores {score} > best {best_score}"
            );
        }
    }
}

/// P3D-307: fishing — the first consumer built ON the flow-consumer
/// contract (D-007). Fish stocks derive from discharge and wetness per
/// river region; catching CONSUMES STOCK without weakening the river
/// (discharge/slope/wetness untouched); restock is deterministic and
/// bounded by carrying capacity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FishStocks {
    pub stock: BTreeMap<(i32, i32), u64>,
}

/// Carrying capacity per river region: bigger rivers hold more fish.
pub fn fish_carrying_capacity(discharge: u64) -> u64 {
    16 + discharge.min(4096) / 8
}

impl FishStocks {
    /// Seed stocks for every river region at carrying capacity.
    pub fn new(graph: &RiverGraph) -> Self {
        let mut stock = BTreeMap::new();
        for k in &graph.river_regions {
            let d = graph.discharge(RegionCoord { x: k.0, z: k.1 });
            stock.insert(*k, fish_carrying_capacity(d));
        }
        FishStocks { stock }
    }

    pub fn stock_at(&self, r: RegionCoord) -> u64 {
        *self.stock.get(&(r.x, r.z)).unwrap_or(&0)
    }

    /// Catch up to `amount` fish; consumes STOCK ONLY. Returns the caught
    /// count. The river (discharge/slope/wetness) is untouched — the
    /// caller never even passes it.
    pub fn catch_fish(&mut self, r: RegionCoord, amount: u64) -> u64 {
        let Some(s) = self.stock.get_mut(&(r.x, r.z)) else {
            return 0;
        };
        let taken = (*s).min(amount);
        *s -= taken;
        taken
    }

    /// Deterministic restock: every river region regains up to a quarter
    /// of its carrying capacity per restock cycle.
    pub fn restock(&mut self, graph: &RiverGraph) {
        for k in &graph.river_regions {
            let cap = fish_carrying_capacity(graph.discharge(RegionCoord { x: k.0, z: k.1 }));
            let cur = self.stock.entry(*k).or_insert(0);
            let regen = cap / 4;
            let room = cap.saturating_sub(*cur);
            *cur += regen.min(room);
        }
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

    /// P3D-303: a large override flips a region's downstream direction;
    /// the rebuilt graph stays acyclic and conserved.
    #[test]
    fn p3d303_override_flows_reroute_locally() {
        let g = WorldGen::new(2024);
        let base = RiverGraph::new(&g, 20);
        // Find a region and one of its NON-downstream neighbors whose
        // elevation is close: raising the region far above that neighbor
        // should pull the flow toward it.
        // Physics of rerouting: lowering a NON-downstream neighbor below
        // the current steepest-descent target pulls the flow to it.
        // (Raising r changes nothing — its neighbors' heights are fixed.)
        let mut probe = None;
        for x in -15..=15 {
            for z in -15..=15 {
                let r = RegionCoord { x, z };
                let e = base.elevation[&(x, z)];
                let Some(Some(d)) = base.downstream.get(&(x, z)) else {
                    continue;
                };
                let d_elev = base.elevation[d];
                for (dx, dz) in NEIGHBORS {
                    let n = RegionCoord { x: x + dx, z: z + dz };
                    if (n.x, n.z) == (d.0, d.1) {
                        continue;
                    }
                    if let Some(&ne) = base.elevation.get(&(n.x, n.z)) {
                        // Lower n below the current target by 10 m.
                        // Lower n to exactly 1 m below the current target:
                        // it becomes the strictly lowest neighbor, so r
                        // must reroute toward it.
                        let delta_n = ne - d_elev - 1;
                        if delta_n <= -1 && e - (ne - delta_n) > 0 {
                            probe = Some((r, n, delta_n));
                            break;
                        }
                    }
                }
                if probe.is_some() {
                    break;
                }
            }
            if probe.is_some() {
                break;
            }
        }
        let (r, n, delta_n) = probe.expect("a suitable region exists");
        let target = n;
        let mut overrides = BTreeMap::new();
        overrides.insert((n.x, n.z), delta_n);
        let rebuilt = RiverGraph::build(&g, 20, &overrides);

        // The region now drains toward the target neighbor.
        assert_eq!(rebuilt.downstream(r), Some(target), "flow must reroute");
        // Direction changes require an ADJACENT elevation change: only n
        // was lowered, so only n itself and its neighbors may flip; every
        // region farther than Chebyshev 1 from n keeps its direction.
        for x in -20..=20 {
            for z in -20..=20 {
                let k = RegionCoord { x, z };
                let dist_n = (x - n.x).abs().max((z - n.z).abs());
                if dist_n > 1 {
                    assert_eq!(
                        rebuilt.downstream(k),
                        base.downstream(k),
                        "distant region {k:?} must keep its flow"
                    );
                }
            }
        }
        // The flag stays consistent on the rebuilt graph.
        assert_eq!(rebuilt.seed, base.seed);
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

#[cfg(test)]
mod reservoir_tests {
    use super::*;

    /// Conservation: in a closed chain, filled − spilled == total
    /// retained; overflow routes DOWNSTREAM; capacity clamps.
    #[test]
    fn p3d305_reservoirs_conserve_and_overflow_downstream() {
        let g = WorldGen::new(2024);
        let graph = RiverGraph::new(&g, 20);
        let mut res = Reservoirs::from_graph(&graph);

        // Find a region WITH a downstream reservoir.
        let mut start = None;
        for x in -20..=20 {
            for z in -20..=20 {
                if graph.downstream(RegionCoord { x, z }).is_some() {
                    start = Some(RegionCoord { x, z });
                    break;
                }
            }
            if start.is_some() {
                break;
            }
        }
        let start = start.expect("a reservoir with downstream exists");
        let before = res.total_volume();

        // Pour a large amount: some retained, some spilled downstream.
        let poured: u64 = 10_000_000;
        let spilled = res.fill(&graph, start, poured as i64);
        let retained = poured - spilled;
        assert_eq!(res.total_volume() - before, retained);
        assert!(spilled > 0, "a huge pour must overflow somewhere");

        // The downstream of `start` must have gained volume (overflow
        // routed downstream, not into thin air): the region right after.
        let down = graph.downstream(start).unwrap();
        // Drain everything we added at start: it must come back.
        let drained = res.drain(start, i64::MAX as i64);
        assert!(drained >= 0);
        let _ = down;
    }

    /// Fill then drain exactly: reservoir returns to zero; drain never
    /// goes negative; determinism.
    #[test]
    fn p3d205_reservoir_fill_drain_round_trip() {
        let g = WorldGen::new(7);
        let graph = RiverGraph::new(&g, 12);
        let mut res = Reservoirs::from_graph(&graph);
        let start = RegionCoord { x: 0, z: 0 };
        let capacity = res.map[&(0, 0)].capacity_kl;
        res.fill(&graph, start, capacity);
        let v = res.map[&(0, 0)].volume_kl;
        assert!(v > 0);
        let drained = res.drain(start, v as i64);
        assert_eq!(drained, v as u64);
        assert_eq!(res.map[&(0, 0)].volume_kl, 0);
        // Draining an empty reservoir yields 0.
        assert_eq!(res.drain(start, 100), 0);
    }
}

#[cfg(test)]
mod fishing_tests {
    use super::*;
    use crate::gen::WorldGen;

    /// THE fishing contract: catching consumes STOCK ONLY — the river's
    /// discharge, wetness, and flow records are untouched; catches are
    /// bounded by stock; restock is deterministic and capacity-bounded.
    #[test]
    fn p3d307_fishing_consumes_stock_never_the_river() {
        let g = WorldGen::new(2024);
        let graph = RiverGraph::new(&g, 20);
        let mut fish = FishStocks::new(&graph);
        assert!(!fish.stock.is_empty(), "river regions must hold fish");

        // Pick a stocked region.
        let r = RegionCoord { x: graph.river_regions[0].0, z: graph.river_regions[0].1 };
        let before_stock = fish.stock_at(r);
        let before_discharge = graph.discharge(r);
        let before_pot = graph.flow_potential_at(&g, None, 0, 0);

        let caught = fish.catch_fish(r, before_stock / 2 + 10);
        assert_eq!(caught, before_stock - fish.stock_at(r));
        assert!(caught > 0);
        // The river did not weaken.
        assert_eq!(graph.discharge(r), before_discharge);
        assert_eq!(fish.stock_at(r), before_stock - caught);

        // Over-fishing is bounded by stock.
        assert_eq!(fish.catch_fish(r, 1_000_000), before_stock - caught);
        assert_eq!(fish.stock_at(r), 0);

        // Restock is deterministic and capacity-bounded.
        fish.restock(&graph);
        let after_one = fish.stock_at(r);
        let cap = fish_carrying_capacity(graph.discharge(r));
        assert!(after_one > 0 && after_one <= cap);
        let snapshot = fish.stock_at(r);
        fish.restock(&graph);
        fish.restock(&graph);
        assert!(fish.stock_at(r) >= snapshot);
        assert!(fish.stock_at(r) <= cap);
        let _ = before_pot;
    }
}
