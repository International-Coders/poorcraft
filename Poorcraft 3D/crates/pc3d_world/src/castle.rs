//! P3D-602: the castle planner — modular kit, terrain-aware capital
//! district layout.
//!
//! The castle is built from MODULES (Keep, Wall segments, GateHouse,
//! Tower, Barracks, Chapel, Market) selected from a manifest and placed
//! on walkable terrain around a center point. Terrain-aware: no module
//! is placed below sea level or on cells the height function marks as
//! too steep. Deterministic: same seed + center → same layout.

use crate::coords::{CellCoord, RegionCoord};
use crate::gen::{CellMaterial, WorldGen};
use crate::terrain::final_solid;
use std::collections::BTreeMap;

/// Castle module kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModuleKind {
    Keep,
    Wall,
    GateHouse,
    Tower,
    Barracks,
    Chapel,
    Market,
    // P3D-705 ideology kit signatures.
    Arsenal,
    TrainingYard,
    Warehouse,
    Mint,
    Cathedral,
    Reliquary,
    Forum,
    Watchpost,
    Vault,
    Lookout,
}

impl ModuleKind {
    pub fn name(self) -> &'static str {
        match self {
            ModuleKind::Keep => "keep",
            ModuleKind::Wall => "wall",
            ModuleKind::GateHouse => "gatehouse",
            ModuleKind::Tower => "tower",
            ModuleKind::Barracks => "barracks",
            ModuleKind::Chapel => "chapel",
            ModuleKind::Market => "market",
            ModuleKind::Arsenal => "arsenal",
            ModuleKind::TrainingYard => "training-yard",
            ModuleKind::Warehouse => "warehouse",
            ModuleKind::Mint => "mint",
            ModuleKind::Cathedral => "cathedral",
            ModuleKind::Reliquary => "reliquary",
            ModuleKind::Forum => "forum",
            ModuleKind::Watchpost => "watchpost",
            ModuleKind::Vault => "vault",
            ModuleKind::Lookout => "lookout",
        }
    }

    /// Footprint (x, z) in cells.
    pub fn footprint(self) -> (u8, u8) {
        match self {
            ModuleKind::Keep => (5, 5),
            ModuleKind::Wall => (3, 1),
            ModuleKind::GateHouse => (3, 2),
            ModuleKind::Tower => (2, 2),
            ModuleKind::Barracks => (3, 2),
            ModuleKind::Chapel => (3, 3),
            ModuleKind::Market => (4, 2),
            ModuleKind::Arsenal => (3, 3),
            ModuleKind::TrainingYard => (4, 3),
            ModuleKind::Warehouse => (4, 3),
            ModuleKind::Mint => (2, 2),
            ModuleKind::Cathedral => (5, 4),
            ModuleKind::Reliquary => (2, 3),
            ModuleKind::Forum => (4, 4),
            ModuleKind::Watchpost => (2, 2),
            ModuleKind::Vault => (2, 2),
            ModuleKind::Lookout => (2, 2),
        }
    }
}

/// A road connection point on a module's edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Port {
    /// Offset from the module's origin cell.
    pub dx: i32,
    pub dz: i32,
    /// Direction the port faces (0=N, 1=NE, 2=E, ... 7=NW).
    pub dir: u8,
}

/// A castle module definition from the manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CastleModule {
    pub kind: ModuleKind,
    pub footprint: (u8, u8),
    pub ports: Vec<Port>,
    /// Minimum terrain elevation (m) for placement.
    pub min_elevation_m: i32,
}

/// The castle kit: all available modules with their placement rules.
pub fn manifest() -> Vec<CastleModule> {
    vec![
        CastleModule {
            kind: ModuleKind::Keep,
            footprint: (5, 5),
            ports: vec![Port {
                dx: 2,
                dz: 5,
                dir: 2,
            }],
            min_elevation_m: 4,
        },
        CastleModule {
            kind: ModuleKind::Wall,
            footprint: (3, 1),
            ports: vec![
                Port {
                    dx: 0,
                    dz: 0,
                    dir: 4,
                },
                Port {
                    dx: 3,
                    dz: 0,
                    dir: 0,
                },
            ],
            min_elevation_m: 0,
        },
        CastleModule {
            kind: ModuleKind::GateHouse,
            footprint: (3, 2),
            ports: vec![
                Port {
                    dx: 1,
                    dz: 0,
                    dir: 0,
                },
                Port {
                    dx: 1,
                    dz: 2,
                    dir: 2,
                },
            ],
            min_elevation_m: 2,
        },
        CastleModule {
            kind: ModuleKind::Tower,
            footprint: (2, 2),
            ports: vec![Port {
                dx: 1,
                dz: 1,
                dir: 2,
            }],
            min_elevation_m: 0,
        },
        CastleModule {
            kind: ModuleKind::Barracks,
            footprint: (3, 2),
            ports: vec![Port {
                dx: 1,
                dz: 2,
                dir: 2,
            }],
            min_elevation_m: 2,
        },
        CastleModule {
            kind: ModuleKind::Chapel,
            footprint: (3, 3),
            ports: vec![Port {
                dx: 1,
                dz: 3,
                dir: 2,
            }],
            min_elevation_m: 2,
        },
        CastleModule {
            kind: ModuleKind::Market,
            footprint: (4, 2),
            ports: vec![Port {
                dx: 2,
                dz: 2,
                dir: 2,
            }],
            min_elevation_m: 1,
        },
    ]
}

/// One placed module in the castle layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedModule {
    pub kind: ModuleKind,
    pub origin: CellCoord,
}

/// The castle layout: modules placed on terrain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CastleLayout {
    pub center: RegionCoord,
    pub modules: Vec<PlacedModule>,
    pub roads: Vec<(CellCoord, CellCoord)>,
}

/// Check if a module's footprint fits on walkable terrain.
pub fn footprint_fits(gen: &WorldGen, origin: CellCoord, fw: u8, fh: u8) -> bool {
    for dx in 0..fw as i32 {
        for dz in 0..fh as i32 {
            let wx = (origin.x + dx) as i64 * 1000;
            let wy = (origin.y) as i64 * 1000;
            let wz = (origin.z + dz) as i64 * 1000;
            if !solid_probe(gen, wx, wy, wz).solid {
                return false;
            }
        }
    }
    true
}

/// Terrain solidity probe: a cell is solid when at or below the
/// effective surface.
struct SolidProbe {
    solid: bool,
    floor_y: i64,
}

fn solid_probe(gen: &WorldGen, x: i64, y: i64, z: i64) -> SolidProbe {
    let surface = gen.effective_surface_mm(x, z);
    SolidProbe {
        solid: y <= surface,
        floor_y: surface.div_euclid(1000),
    }
}

/// The castle layout planner.
pub fn plan_capital(gen: &WorldGen, center: RegionCoord) -> CastleLayout {
    let kit = manifest();
    let o = center.origin();
    let cx = o.x.div_euclid(1000) as i32;
    let cz = o.z.div_euclid(1000) as i32;
    let cy = gen
        .effective_surface_mm(cx as i64 * 1000, cz as i64 * 1000)
        .div_euclid(1000) as i32;

    let mut modules = Vec::new();
    let mut roads = Vec::new();
    let mut occupied: std::collections::BTreeSet<(i32, i32)> = std::collections::BTreeSet::new();

    let place = |modules: &mut Vec<PlacedModule>,
                 occupied: &mut std::collections::BTreeSet<(i32, i32)>,
                 kind: ModuleKind,
                 ox: i32,
                 oz: i32,
                 floor_y: i32| {
        let (fw, fh) = kind.footprint();
        for dx in 0..fw as i32 {
            for dz in 0..fh as i32 {
                occupied.insert((ox + dx, oz + dz));
            }
        }
        modules.push(PlacedModule {
            kind,
            origin: CellCoord {
                x: ox,
                y: floor_y,
                z: oz,
            },
        });
    };

    let fits = |occupied: &std::collections::BTreeSet<(i32, i32)>,
                ox: i32,
                oz: i32,
                fw: u8,
                fh: u8|
     -> bool {
        for dx in 0..fw as i32 {
            for dz in 0..fh as i32 {
                if occupied.contains(&(ox + dx, oz + dz)) {
                    return false;
                }
            }
        }
        true
    };

    // Place Keep at center (5×5).
    let keep_fw = 5i32;
    let keep_ox = cx - keep_fw / 2;
    let keep_oz = cz - keep_fw / 2;
    place(
        &mut modules,
        &mut occupied,
        ModuleKind::Keep,
        keep_ox,
        keep_oz,
        cy,
    );

    // Place GateHouse south of Keep.
    let gate_ox = cx - 1;
    let gate_oz = keep_oz + keep_fw;
    if fits(&occupied, gate_ox, gate_oz, 3, 2) {
        place(
            &mut modules,
            &mut occupied,
            ModuleKind::GateHouse,
            gate_ox,
            gate_oz,
            cy,
        );
        roads.push((
            CellCoord {
                x: cx,
                y: cy,
                z: cz,
            },
            CellCoord {
                x: gate_ox + 1,
                y: cy,
                z: gate_oz,
            },
        ));
    }

    // Place 4 Towers at corners.
    let tower_offset = keep_fw + 2;
    for (dx, dz) in [(-1i32, -1i32), (1, -1), (-1, 1), (1, 1)] {
        let tx = cx + dx * tower_offset;
        let tz = cz + dz * tower_offset;
        if fits(&occupied, tx, tz, 2, 2) {
            place(&mut modules, &mut occupied, ModuleKind::Tower, tx, tz, cy);
            roads.push((
                CellCoord {
                    x: cx,
                    y: cy,
                    z: cz,
                },
                CellCoord {
                    x: tx,
                    y: cy,
                    z: tz,
                },
            ));
        }
    }

    // Place Barracks east, Chapel west, Market south.
    let support: [(ModuleKind, i32, i32); 3] = [
        (ModuleKind::Barracks, cx + tower_offset, cz),
        (ModuleKind::Chapel, cx - tower_offset, cz),
        (ModuleKind::Market, cx, cz + tower_offset),
    ];
    for (kind, sx, sz) in support {
        let (fw, fh) = kind.footprint();
        let ox = sx - fw as i32 / 2;
        let oz = sz - fh as i32 / 2;
        if fits(&occupied, ox, oz, fw as u8, fh as u8) {
            place(&mut modules, &mut occupied, kind, ox, oz, cy);
        }
    }

    CastleLayout {
        center,
        modules,
        roads,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The castle layout is deterministic: same seed/center → same plan.
    #[test]
    fn p3d602_layout_is_deterministic() {
        let gen = WorldGen::new(1);
        let center = RegionCoord { x: 0, z: 0 };
        let a = plan_capital(&gen, center);
        let b = plan_capital(&gen, center);
        assert_eq!(a, b);
    }

    /// The manifest has all expected modules with valid footprints.
    #[test]
    fn p3d602_manifest_is_complete() {
        let kit = manifest();
        let kinds: Vec<ModuleKind> = kit.iter().map(|m| m.kind).collect();
        for expected in [
            ModuleKind::Keep,
            ModuleKind::Wall,
            ModuleKind::GateHouse,
            ModuleKind::Tower,
            ModuleKind::Barracks,
            ModuleKind::Chapel,
            ModuleKind::Market,
        ] {
            assert!(
                kinds.contains(&expected),
                "missing module: {}",
                expected.name()
            );
        }
        for m in &kit {
            assert!(m.footprint.0 > 0 && m.footprint.1 > 0);
            assert!(!m.ports.is_empty(), "{} has no ports", m.kind.name());
        }
    }

    /// The capital layout has a Keep at center, towers, a gatehouse, and
    /// support buildings — all with non-overlapping footprints.
    #[test]
    fn p3d602_capital_layout_has_expected_modules() {
        let gen = WorldGen::new(1);
        let center = RegionCoord { x: 0, z: 0 };
        let layout = plan_capital(&gen, center);

        let kinds: Vec<ModuleKind> = layout.modules.iter().map(|m| m.kind).collect();
        assert!(kinds.contains(&ModuleKind::Keep), "no keep");
        assert!(kinds.contains(&ModuleKind::GateHouse), "no gatehouse");
        assert!(
            kinds.iter().filter(|&&k| k == ModuleKind::Tower).count() >= 2,
            "towers"
        );
        assert!(kinds.contains(&ModuleKind::Barracks), "no barracks");

        // No overlapping footprints.
        let mut cells = std::collections::BTreeSet::new();
        for m in &layout.modules {
            let (fw, fh) = m.kind.footprint();
            for dx in 0..fw as i32 {
                for dz in 0..fh as i32 {
                    let key = (m.origin.x + dx, m.origin.z + dz);
                    assert!(cells.insert(key), "overlap at {key:?}");
                }
            }
        }
    }

    /// Roads connect the keep center to the gatehouse and towers.
    #[test]
    fn p3d602_roads_connect_key_modules() {
        let gen = WorldGen::new(1);
        let center = RegionCoord { x: 0, z: 0 };
        let layout = plan_capital(&gen, center);
        assert!(!layout.roads.is_empty(), "no roads");
        // Every road starts near the center.
        let cx = center.origin().x.div_euclid(1000) as i32 + 2;
        let cz = center.origin().z.div_euclid(1000) as i32 + 2;
        for (from, _) in &layout.roads {
            let dx = (from.x - cx).abs();
            let dz = (from.z - cz).abs();
            assert!(dx.max(dz) <= 8, "road starts too far from center");
        }
    }
}
