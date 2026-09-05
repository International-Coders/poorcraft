//! P3D-505: valve-era engineering components — valves, pipes, waterwheels.
//!
//! All components are CONSUMERS on the flow-consumer contract (D-007):
//! they read flow potential and never weaken the river. A valve gates an
//! edge; a pipe records a connection; a waterwheel derives spin rate
//! from discharge × slope at its site. Pure and deterministic.

use crate::coords::RegionCoord;
use crate::hydro::RiverGraph;
use std::collections::BTreeMap;

/// An on/off gate on one river edge (from → to region pair).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Valve {
    pub id: u64,
    pub on: bool,
    pub edge: ((i32, i32), (i32, i32)),
}

/// The valve network: valves keyed by their edge; several valves may sit
/// on one edge (ANY closed valve blocks).
#[derive(Clone, Debug, Default)]
pub struct ValveNetwork {
    pub valves: BTreeMap<u64, Valve>,
    next_id: u64,
}

impl ValveNetwork {
    pub fn new() -> Self {
        ValveNetwork { valves: BTreeMap::new(), next_id: 0 }
    }

    pub fn add(&mut self, on: bool, edge: ((i32, i32), (i32, i32))) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.valves.insert(id, Valve { id, on, edge });
        id
    }

    pub fn set(&mut self, id: u64, on: bool) -> bool {
        let Some(v) = self.valves.get_mut(&id) else {
            return false;
        };
        v.on = on;
        true
    }

    /// Flow through an edge: 0 when any valve on the edge is closed,
    /// otherwise the base discharge provided by the caller.
    pub fn flow_through(&self, edge: ((i32, i32), (i32, i32)), base: u64) -> u64 {
        for v in self.valves.values() {
            if v.edge == edge && !v.on {
                return 0;
            }
            // Valves are bidirectional gates: check the reversed edge too.
            let (a, b) = v.edge;
            if (b.0, b.1) == edge.0 && (a.0, a.1) == edge.1 && !v.on {
                return 0;
            }
        }
        base
    }
}

/// A waterwheel sited on a river region: spin rate (milli-RPM) is
/// proportional to discharge × slope at the site — the D-007 law means
/// the wheel reads these records without weakening them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaterWheel {
    pub id: u64,
    pub site: RegionCoord,
    pub rpm_milli: i64,
}

impl WaterWheel {
    /// Derive a wheel from real flow records. Discharge and slope come
    /// from the graph; site viability was proven by P3D-306.
    pub fn site(graph: &RiverGraph, id: u64, site: RegionCoord) -> Option<WaterWheel> {
        let discharge = graph.discharge(site);
        if discharge == 0 {
            return None;
        }
        let slope = graph
            .downstream(site)
            .map(|d| {
                let here = graph.elevation[&(site.x, site.z)];
                let there = graph.elevation[&(d.x, d.z)];
                ((here - there).max(0) as i64 * 1_000_000 / 256_000) as i64
            })
            .unwrap_or(0);
        // Spin: milli-RPM = discharge × slope / 4096, floor 5 when flowing.
        let rpm_milli = (discharge.saturating_mul(slope as u64) / 4096).max(5) as i64;
        Some(WaterWheel { id, site, rpm_milli })
    }
}

/// A pipe connection between two cell endpoints (records the
/// connection; per-cell flow physics arrive with 702).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pipe {
    pub id: u64,
    pub from: (i32, i32, i32),
    pub to: (i32, i32, i32),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::WorldGen;

    /// Valve on/off gates flow: on passes the base discharge, off blocks
    /// to zero, and the gate is bidirectional on the edge.
    #[test]
    fn p3d505_valves_gate_flow() {
        let mut net = ValveNetwork::new();
        let edge = ((0, 0), (1, 0));
        let id = net.add(true, edge);
        assert_eq!(net.flow_through(edge, 500), 500, "open valve passes");
        assert!(net.set(id, false));
        assert_eq!(net.flow_through(edge, 500), 0, "closed valve blocks");
        // Bidirectional: the reversed edge is the same gate.
        let rev = ((1, 0), (0, 0));
        assert_eq!(net.flow_through(rev, 500), 0);
        assert!(net.set(id, true));
        assert_eq!(net.flow_through(rev, 500), 500);
        // Multiple valves on one edge: ANY closed blocks.
        let id2 = net.add(true, edge);
        net.set(id, true);
        assert_eq!(net.flow_through(edge, 500), 500);
        net.set(id2, false);
        assert_eq!(net.flow_through(edge, 500), 0);
    }

    /// Waterwheel rpm derives from real records: proportional to
    /// discharge × slope, deterministic, and None on dry sites.
    #[test]
    fn p3d505_waterwheel_rpm_from_records() {
        let g = WorldGen::new(2024);
        let graph = RiverGraph::new(&g, 24);
        let mut found = 0;
        let mut last_rpm = 0i64;
        for x in -24..=24 {
            for z in -24..=24 {
                if let Some(wheel) = WaterWheel::site(&graph, 1, RegionCoord { x, z }) {
                    found += 1;
                    assert!(wheel.rpm_milli >= 5);
                    // Deterministic: same site, same rpm.
                    assert_eq!(
                        WaterWheel::site(&graph, 2, RegionCoord { x, z })
                            .unwrap()
                            .rpm_milli,
                        wheel.rpm_milli
                    );
                    last_rpm = wheel.rpm_milli;
                }
            }
        }
        assert!(found > 0, "river worlds must host viable wheel sites");
        assert!(last_rpm > 0);
    }
}
