//! P3D-302: persistent flow records — direction, slope, discharge,
//! capacity, revision, and boundary ports.
//!
//! Every river region carries one record derived deterministically from
//! the river graph. Slope is fixed-point (per-mille, no floats persisted).
//! Boundary PORTS are the patch/region water contract: an exit toward a
//! downstream neighbor is that neighbor's entry at the SAME world
//! position with the SAME discharge — neighbors can never disagree about
//! water crossing between them.

use crate::coords::RegionCoord;
use crate::gen::WorldGen;
use crate::hydro::RiverGraph;
use std::collections::BTreeMap;

/// 8-compass direction code toward downstream (0 = +x, going clockwise:
/// 1 = +x+z, 2 = +z, ... 7 = +x−z). 8 = sink (no downstream).
pub const DIR_SINK: u8 = 8;

/// Compass direction code from region `from` toward region `to`
/// (8-neighborhood).
pub fn direction_code(from: RegionCoord, to: RegionCoord) -> Option<u8> {
    let dx = to.x - from.x;
    let dz = to.z - from.z;
    let v = (dx, dz);
    Some(match v {
        (1, 0) => 0,
        (1, 1) => 1,
        (0, 1) => 2,
        (-1, 1) => 3,
        (-1, 0) => 4,
        (-1, -1) => 5,
        (0, -1) => 6,
        (1, -1) => 7,
        _ => return None,
    })
}

/// One river region's flow record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowRecord {
    pub region_x: i32,
    pub region_z: i32,
    /// Compass code toward downstream (DIR_SINK at sinks).
    pub direction: u8,
    /// Elevation drop toward downstream, in per-mille of the edge
    /// distance (fixed-point; never negative toward downstream).
    pub slope_per_mille: i32,
    pub discharge: u64,
    /// Deterministic channel capacity: discharge scaled by slope.
    pub capacity: u64,
    /// Version counter — bumped when the record's source graph rebuilds.
    pub revision: u64,
}

/// The flow table for a whole watershed band.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlowTable {
    pub revision: u64,
    pub records: BTreeMap<(i32, i32), FlowRecord>,
}

impl FlowTable {
    /// Derive from the river graph with DIRTY-REGION revisions
    /// (P3D-303): a region whose record is semantically equal to its
    /// previous record KEEPS the previous revision; a changed region's
    /// revision increments (or starts at 1 when there was none). The
    /// table revision increments by one per rebuild.
    pub fn from_graph_with_revisions(
        previous: Option<&FlowTable>,
        graph: &RiverGraph,
    ) -> Self {
        let mut table = Self::from_graph(graph);
        table.revision = previous.map(|p| p.revision + 1).unwrap_or(1);
        for rec in table.records.values_mut() {
            rec.revision = table.revision;
            if let Some(prev) = previous.and_then(|p| p.records.get(&(rec.region_x, rec.region_z))) {
                let same = prev.direction == rec.direction
                    && prev.slope_per_mille == rec.slope_per_mille
                    && prev.discharge == rec.discharge
                    && prev.capacity == rec.capacity;
                if same {
                    rec.revision = prev.revision;
                }
            }
        }
        table
    }

    /// Derive from the river graph. Revision starts at 1.
    pub fn from_graph(graph: &RiverGraph) -> Self {
        let mut records = BTreeMap::new();
        for x in -graph.half..=graph.half {
            for z in -graph.half..=graph.half {
                let r = RegionCoord { x, z };
                let downstream = graph.downstream(r);
                let (direction, slope_per_mille) = match downstream {
                    None => (DIR_SINK, 0),
                    Some(d) => {
                        let here = graph.elevation[&(r.x, r.z)];
                        let there = graph.elevation[&(d.x, d.z)];
                        let drop_mm = (here - there).max(0) as i64 * 1000;
                        let dist_mm = 256_000i64
                            * if dx_dz_diagonal(r, d) { 2 } else { 1 };
                        (
                            direction_code(r, d).unwrap_or(DIR_SINK),
                            (drop_mm * 1000 / dist_mm.max(1)) as i32,
                        )
                    }
                };
                let discharge = graph.discharge(r);
                let slope = slope_per_mille.max(0) as u64;
                let capacity = discharge * (1 + slope / 50);
                records.insert(
                    (x, z),
                    FlowRecord {
                        region_x: x,
                        region_z: z,
                        direction,
                        slope_per_mille,
                        discharge,
                        capacity,
                        revision: 1,
                    },
                );
            }
        }
        FlowTable { revision: 1, records }
    }

    pub fn get(&self, r: RegionCoord) -> Option<&FlowRecord> {
        self.records.get(&(r.x, r.z))
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// A rebuild of the underlying graph bumps every record's revision.
    pub fn bump_revision(&mut self) {
        self.revision += 1;
        for rec in self.records.values_mut() {
            rec.revision = self.revision;
        }
    }

    /// Boundary ports for one region: an OUT port toward the downstream
    /// neighbor (if any) and an IN port from every upstream neighbor.
    /// A port's world position is the midpoint of the shared border.
    pub fn ports(&self, r: RegionCoord) -> Vec<Port> {
        let mut out = Vec::new();
        let rec = self.get(r);
        if let Some(rec) = rec {
            if rec.direction != DIR_SINK {
                if let Some(to) = compass_target(r, rec.direction) {
                    out.push(Port {
                        side: rec.direction,
                        position_mm: shared_border_midpoint(r, rec.direction),
                        out: true,
                        discharge: rec.discharge,
                    });
                    let _ = to;
                }
            }
        }
        // In ports: neighbors whose direction points at us.
        for (dx, dz) in [
            (1i32, 0i32),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ] {
            let n = RegionCoord { x: r.x + dx, z: r.z + dz };
            if let Some(nrec) = self.get(n) {
                if nrec.direction != DIR_SINK {
                    if let Some(target) = compass_target(n, nrec.direction) {
                        if target == r {
                            out.push(Port {
                                side: (nrec.direction + 4) % 8,
                                position_mm: shared_border_midpoint(n, nrec.direction),
                                out: false,
                                discharge: nrec.discharge,
                            });
                        }
                    }
                }
            }
        }
        out
    }
}

/// A water crossing on a region/patch boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Port {
    /// Compass side of THIS region the port sits on.
    pub side: u8,
    /// World midpoint of the shared border (millimeters; y = 0).
    pub position_mm: (i64, i64),
    pub out: bool,
    pub discharge: u64,
}

fn dx_dz_diagonal(a: RegionCoord, b: RegionCoord) -> bool {
    a.x != b.x && a.z != b.z
}

fn compass_target(r: RegionCoord, dir: u8) -> Option<RegionCoord> {
    let (dx, dz) = match dir {
        0 => (1, 0),
        1 => (1, 1),
        2 => (0, 1),
        3 => (-1, 1),
        4 => (-1, 0),
        5 => (-1, -1),
        6 => (0, -1),
        7 => (1, -1),
        _ => return None,
    };
    Some(RegionCoord { x: r.x + dx, z: r.z + dz })
}

/// World midpoint (mm) of the border between `r` and its neighbor in
/// compass direction `dir`.
fn shared_border_midpoint(r: RegionCoord, dir: u8) -> (i64, i64) {
    let (dx, dz) = match dir {
        0 => (1, 0),
        1 => (1, 1),
        2 => (0, 1),
        3 => (-1, 1),
        4 => (-1, 0),
        5 => (-1, -1),
        6 => (0, -1),
        7 => (1, -1),
        _ => (0, 0),
    };
    let region_mm = 256_000i64;
    let cx = r.x as i64 * region_mm + region_mm / 2 + dx * region_mm / 2;
    let cz = r.z as i64 * region_mm + region_mm / 2 + dz * region_mm / 2;
    (cx, cz)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> FlowTable {
        FlowTable::from_graph(&RiverGraph::new(&WorldGen::new(2024), 24))
    }

    /// Directions agree with the downstream map; slope is never negative
    /// toward downstream; sinks carry DIR_SINK.
    #[test]
    fn p3d302_records_agree_with_the_graph() {
        let g = RiverGraph::new(&WorldGen::new(2024), 24);
        let t = table();
        for x in -24..=24 {
            for z in -24..=24 {
                let r = RegionCoord { x, z };
                let rec = t.get(r).expect("record per region");
                match g.downstream(r) {
                    None => {
                        assert_eq!(rec.direction, DIR_SINK);
                        assert_eq!(rec.slope_per_mille, 0);
                    }
                    Some(d) => {
                        let code = direction_code(r, d).unwrap();
                        assert_eq!(rec.direction, code);
                        assert!(rec.slope_per_mille >= 0, "downhill is non-negative");
                        assert_eq!(rec.discharge, g.discharge(r));
                    }
                }
            }
        }
    }

    /// PORT MATCHING LAW: A's exit toward B is B's entry from A — same
    /// world position, same discharge, opposite sides.
    #[test]
    fn p3d302_ports_match_across_borders() {
        let t = table();
        let mut checked = 0;
        for x in -24..=24 {
            for z in -24..=24 {
                let a = RegionCoord { x, z };
                let rec = t.get(a).expect("record");
                if rec.direction == DIR_SINK {
                    continue;
                }
                let b = compass_target(a, rec.direction).unwrap();
                let a_ports = t.ports(a);
                let b_ports = t.ports(b);
                let exit = a_ports
                    .iter()
                    .find(|p| p.out && p.discharge == rec.discharge)
                    .expect("exit port exists");
                // Match by POSITION (discharges can tie between neighbors;
                // positions identify the shared border uniquely).
                let entry = b_ports
                    .iter()
                    .find(|p| !p.out && p.position_mm == exit.position_mm)
                    .expect("matching entry exists on the shared border");
                assert_eq!(entry.discharge, rec.discharge);
                checked += 1;
                if checked >= 50 {
                    return;
                }
            }
        }
        assert!(checked > 0, "no river ports checked");
    }

    /// Revision: starts at 1; a bump moves every record together.
    #[test]
    fn p3d302_revisions_bump() {
        let mut t = table();
        assert_eq!(t.revision(), 1);
        let before = t.get(RegionCoord { x: 0, z: 0 }).unwrap().revision;
        assert_eq!(before, 1);
        t.bump_revision();
        assert_eq!(t.revision(), 2);
        assert!(t
            .records
            .values()
            .all(|r| r.revision == 2));
    }

    /// P3D-303: dirty-region revisions — a reroute bumps ONLY the regions
    /// whose records actually changed; unchanged regions keep theirs.
    #[test]
    fn p3d303_revisions_change_only_where_flow_changed() {
        let g = WorldGen::new(2024);
        let base = RiverGraph::new(&g, 20);
        let first = FlowTable::from_graph(&base);

        // Find a region whose raising flips its downstream (as in the
        // hydro reroute test).
        // Lower a non-downstream neighbor below the current target so the
        // region reroutes toward it (see the hydro reroute test).
        let mut probe = None;
        for x in -15..=15 {
            for z in -15..=15 {
                let r = RegionCoord { x, z };
                let e = base.elevation[&(x, z)];
                let Some(Some(d)) = base.downstream.get(&(x, z)) else {
                    continue;
                };
                let d_elev = base.elevation[d];
                for (dx, dz) in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
                    let n = RegionCoord { x: x + dx, z: z + dz };
                    if (n.x, n.z) == (d.0, d.1) {
                        continue;
                    }
                    if let Some(&ne) = base.elevation.get(&(n.x, n.z)) {
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
        let mut overrides = std::collections::BTreeMap::new();
        overrides.insert((n.x, n.z), delta_n);
        let rebuilt = RiverGraph::build(&g, 20, &overrides);
        let second = FlowTable::from_graph_with_revisions(Some(&first), &rebuilt);

        // Table revision advanced by exactly one rebuild.
        assert_eq!(second.revision(), first.revision() + 1);

        // Unchanged records keep their revision; the rerouted region's
        // record bumped.
        let mut changed = 0;
        let mut kept = 0;
        for (k, rec) in &second.records {
            match first.records.get(k) {
                Some(prev) if prev.revision == rec.revision => kept += 1,
                _ => changed += 1,
            }
        }
        assert!(changed > 0, "the reroute must bump the changed region");
        assert!(kept > 0, "unchanged regions must keep their revision");
    }
}
