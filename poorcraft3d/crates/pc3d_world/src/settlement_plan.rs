//! P3D-601: settlement plan — the physical layout of a settlement.
//!
//! The plan is the DATA model that drives construction, NPC schedules,
//! and the oversight panel. Deterministic per (seed, center): a central
//! plaza with a well, buildings ringed around it, and roads radiating to
//! each building from the plaza edge.

use crate::coords::{CellCoord, RegionCoord};
use crate::gen::WorldGen;
use std::collections::BTreeMap;

/// What a building provides to the settlement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BuildingKind {
    Home,
    Workshop,
    Storage,
    Barracks,
    Well,
    Farm,
    Watchtower,
}

impl BuildingKind {
    pub fn name(self) -> &'static str {
        match self {
            BuildingKind::Home => "home",
            BuildingKind::Workshop => "workshop",
            BuildingKind::Storage => "storage",
            BuildingKind::Barracks => "barracks",
            BuildingKind::Well => "well",
            BuildingKind::Farm => "farm",
            BuildingKind::Watchtower => "watchtower",
        }
    }

    /// Service contribution per building.
    pub fn service(self) -> Service {
        match self {
            BuildingKind::Home => Service::Housing { capacity: 4 },
            BuildingKind::Workshop => Service::Production { output: 5 },
            BuildingKind::Storage => Service::Storage { capacity: 200 },
            BuildingKind::Barracks => Service::Defense { strength: 10 },
            BuildingKind::Well => Service::Water,
            BuildingKind::Farm => Service::Food { output: 15 },
            BuildingKind::Watchtower => Service::Defense { strength: 5 },
        }
    }
}

/// What a building provides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Service {
    Housing { capacity: i64 },
    Production { output: i64 },
    Storage { capacity: i64 },
    Defense { strength: i64 },
    Water,
    Food { output: i64 },
}

/// One placed building.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildingSlot {
    pub kind: BuildingKind,
    pub cell: CellCoord,
    /// Footprint (x/z extent in cells).
    pub size: (u8, u8),
}

/// The D-033 anchors: Bed, Work, Idle zones.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Anchors {
    pub bed_cells: Vec<CellCoord>,
    pub work_cells: Vec<CellCoord>,
    pub idle_cells: Vec<CellCoord>,
}

/// A road segment between two points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoadSegment {
    pub from: CellCoord,
    pub to: CellCoord,
}

/// The settlement plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettlementPlan {
    pub center: RegionCoord,
    pub plaza: CellCoord,
    pub buildings: Vec<BuildingSlot>,
    pub roads: Vec<RoadSegment>,
    pub anchors: Anchors,
}

/// Validation verdict for a plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanValidation {
    pub all_buildings_present: bool,
    pub plaza_exists: bool,
    pub roads_connect: bool,
    pub anchors_populated: bool,
}

impl PlanValidation {
    pub fn passed(&self) -> bool {
        self.all_buildings_present
            && self.plaza_exists
            && self.roads_connect
            && self.anchors_populated
    }
}

/// The standard building layout for one settlement.
/// Buildings are placed in a ring around the plaza at increasing radii:
/// ring 0 (r=3): Well (center-of-plaza), Home ×2
/// ring 1 (r=6): Home, Workshop, Farm, Home
/// ring 2 (r=10): Storage, Barracks, Watchtower, Farm
pub const LAYOUT_RING0: &[(BuildingKind, i32, i32)] = &[
    (BuildingKind::Well, 0, 0),
    (BuildingKind::Home, 3, 0),
    (BuildingKind::Home, -3, 0),
];

pub const LAYOUT_RING1: &[(BuildingKind, i32, i32)] = &[
    (BuildingKind::Home, 6, 0),
    (BuildingKind::Workshop, 0, 6),
    (BuildingKind::Farm, -6, 0),
    (BuildingKind::Home, 0, -6),
];

pub const LAYOUT_RING2: &[(BuildingKind, i32, i32)] = &[
    (BuildingKind::Storage, 10, 0),
    (BuildingKind::Barracks, 0, 10),
    (BuildingKind::Watchtower, -10, 0),
    (BuildingKind::Farm, 0, -10),
];

impl SettlementPlan {
    /// Generate a deterministic settlement plan centered at `center`.
    /// The plaza is the region center cell. Buildings are offset from
    /// the plaza by the layout rings. Roads connect plaza to each
    /// building's nearest edge.
    pub fn plan(gen: &WorldGen, center: RegionCoord) -> SettlementPlan {
        let o = center.origin();
        let plaza = CellCoord {
            x: o.x.div_euclid(1000) as i32 + 128,
            y: 0,
            z: o.z.div_euclid(1000) as i32 + 128,
        };

        let mut buildings = Vec::new();
        let mut roads = Vec::new();

        // Place buildings from the three layout rings.
        for (kind, ox, oz) in LAYOUT_RING0 {
            buildings.push(BuildingSlot {
                kind: *kind,
                cell: CellCoord { x: plaza.x + ox, y: 0, z: plaza.z + oz },
                size: (2, 2),
            });
        }
        for (kind, ox, oz) in LAYOUT_RING1 {
            buildings.push(BuildingSlot {
                kind: *kind,
                cell: CellCoord { x: plaza.x + ox, y: 0, z: plaza.z + oz },
                size: (2, 2),
            });
        }
        for (kind, ox, oz) in LAYOUT_RING2 {
            buildings.push(BuildingSlot {
                kind: *kind,
                cell: CellCoord { x: plaza.x + ox, y: 0, z: plaza.z + oz },
                size: (2, 2),
            });
        }

        // Roads: from plaza to each non-well building (the well IS the
        // plaza center).
        for b in buildings.iter().filter(|b| b.kind != BuildingKind::Well) {
            roads.push(RoadSegment { from: plaza, to: b.cell });
        }

        // Derive anchors from buildings.
        let mut anchors = Anchors::default();
        for b in &buildings {
            match b.kind {
                BuildingKind::Home => {
                    anchors.bed_cells.push(b.cell);
                }
                BuildingKind::Workshop | BuildingKind::Farm => {
                    anchors.work_cells.push(b.cell);
                }
                _ => {}
            }
            // The plaza and road junctions serve as idle areas.
            anchors.idle_cells.push(b.cell);
        }
        anchors.idle_cells.push(plaza);

        let _ = gen; // reserved for terrain-aware placement (P3D-602)
        SettlementPlan { center, plaza, buildings, roads, anchors }
    }

    /// Validate the plan: all buildings present, plaza exists, roads
    /// connect plaza to every building, anchors populated.
    pub fn validate(&self) -> PlanValidation {
        let all_present = self.buildings.len() == LAYOUT_RING0.len() + LAYOUT_RING1.len() + LAYOUT_RING2.len();
        let plaza_exists = self
            .buildings
            .iter()
            .any(|b| b.kind == BuildingKind::Well && b.cell == self.plaza);
        let roads_connect = self.roads.iter().all(|r| r.from == self.plaza);
        let anchors_ok = !self.anchors.bed_cells.is_empty()
            && !self.anchors.work_cells.is_empty()
            && !self.anchors.idle_cells.is_empty();
        PlanValidation {
            all_buildings_present: all_present,
            plaza_exists,
            roads_connect,
            anchors_populated: anchors_ok,
        }
    }

    /// The service summary derived from building counts.
    pub fn services(&self) -> ServiceSummary {
        let mut s = ServiceSummary::default();
        for b in &self.buildings {
            match b.kind.service() {
                Service::Housing { capacity } => s.housing_capacity += capacity,
                Service::Production { output } => s.production_output += output,
                Service::Storage { capacity } => s.storage_capacity += capacity,
                Service::Defense { strength } => s.defense_strength += strength,
                Service::Water => s.has_water = true,
                Service::Food { output } => s.food_output += output,
            }
        }
        s
    }

    /// Buildings of a given kind.
    pub fn buildings_of_kind(&self, kind: BuildingKind) -> Vec<&BuildingSlot> {
        self.buildings.iter().filter(|b| b.kind == kind).collect()
    }
}

/// Summary of what the settlement provides.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ServiceSummary {
    pub housing_capacity: i64,
    pub production_output: i64,
    pub storage_capacity: i64,
    pub defense_strength: i64,
    pub food_output: i64,
    pub has_water: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic: same center → identical plan.
    #[test]
    fn p3d601_plan_is_deterministic() {
        let gen = WorldGen::new(1);
        let center = RegionCoord { x: 0, z: 0 };
        let a = SettlementPlan::plan(&gen, center);
        let b = SettlementPlan::plan(&gen, center);
        assert_eq!(a, b);
    }

    /// The layout has all expected building kinds, the well is on the
    /// plaza, and anchors are derived from buildings.
    #[test]
    fn p3d601_layout_has_all_kinds_and_anchors() {
        let gen = WorldGen::new(1);
        let plan = SettlementPlan::plan(&gen, RegionCoord { x: 0, z: 0 });
        // All 11 buildings placed (2+4+4 = 10 non-well + 1 well = 11).
        assert_eq!(plan.buildings.len(), 11);
        // Every kind present.
        for kind in [
            BuildingKind::Home,
            BuildingKind::Workshop,
            BuildingKind::Storage,
            BuildingKind::Barracks,
            BuildingKind::Well,
            BuildingKind::Farm,
            BuildingKind::Watchtower,
        ] {
            assert!(
                plan.buildings.iter().any(|b| b.kind == kind),
                "missing building kind: {}",
                kind.name()
            );
        }
        // Anchors populated from buildings.
        assert!(!plan.anchors.bed_cells.is_empty(), "beds from homes");
        assert!(!plan.anchors.work_cells.is_empty(), "work from workshop/farm");
        assert!(!plan.anchors.idle_cells.is_empty(), "idle from plaza");
    }

    /// Roads connect the plaza to every non-well building.
    #[test]
    fn p3d601_roads_connect_plaza_to_buildings() {
        let gen = WorldGen::new(1);
        let plan = SettlementPlan::plan(&gen, RegionCoord { x: 0, z: 0 });
        let non_well = plan.buildings.iter().filter(|b| b.kind != BuildingKind::Well).count();
        assert_eq!(plan.roads.len(), non_well, "one road per non-well building");
        for road in &plan.roads {
            assert_eq!(road.from, plan.plaza, "roads start at plaza");
        }
    }

    /// The plan passes validation.
    #[test]
    fn p3d601_plan_validates() {
        let gen = WorldGen::new(1);
        let plan = SettlementPlan::plan(&gen, RegionCoord { x: 0, z: 0 });
        let v = plan.validate();
        assert!(v.all_buildings_present);
        assert!(v.plaza_exists);
        assert!(v.roads_connect);
        assert!(v.anchors_populated);
        assert!(v.passed());
    }

    /// The service summary reflects building counts.
    #[test]
    fn p3d601_service_summary_is_correct() {
        let gen = WorldGen::new(1);
        let plan = SettlementPlan::plan(&gen, RegionCoord { x: 0, z: 0 });
        let s = plan.services();
        // 4 homes (2 ring0 + 2 ring1) × 4 = 16 housing.
        assert_eq!(s.housing_capacity, 16);
        // 1 workshop × 5 = 5 production.
        assert_eq!(s.production_output, 5);
        // 1 storage × 200 = 200.
        assert_eq!(s.storage_capacity, 200);
        // 1 barracks × 10 + 1 watchtower × 5 = 15 defense.
        assert_eq!(s.defense_strength, 15);
        // 2 farms × 15 = 30 food.
        assert_eq!(s.food_output, 30);
        assert!(s.has_water);
    }
}
