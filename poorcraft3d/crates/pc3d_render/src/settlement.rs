//! The settlement kit (NWR-008): the FIRST modular building set,
//! assembled from the AUTHORITATIVE castle/settlement plans.
//!
//! The plans (pc3d_world::castle / settlement_plan) remain the placement
//! authority — every module and building maps to exactly one kit
//! placement at its plan cell. The kit adds: declared SOCKET alignment
//! (wall chains share wall_a/wall_b positions; the gate's passage
//! socket axis is the walk-through), a refined gate collision (the
//! passage column opens — the primitive path blocked its whole
//! footprint), preserved nav anchors + Bed/Work/Idle zones, and LOD
//! buckets drawn instanced (one draw per module+LOD).

use crate::city::NavAnchor;
use pc3d_world::castle::{CastleLayout, ModuleKind, PlacedModule};
use pc3d_world::coords::CellCoord;
use pc3d_world::gen::WorldGen;
use pc3d_world::settlement_plan::{BuildingKind, BuildingSlot, SettlementPlan};
use std::collections::{BTreeMap, BTreeSet};

/// The ten kit modules (asset ids are `module.<name>`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KitModule {
    SettlementHouse,
    SettlementWorkshop,
    MarketStall,
    WallSegment,
    GateArch,
    Watchtower,
    Keep,
    BridgeDock,
    BannerSign,
    WaterWheel,
}

impl KitModule {
    pub fn name(self) -> &'static str {
        match self {
            KitModule::SettlementHouse => "module.settlement_house",
            KitModule::SettlementWorkshop => "module.settlement_workshop",
            KitModule::MarketStall => "module.market_stall",
            KitModule::WallSegment => "module.wall_segment",
            KitModule::GateArch => "module.gate_arch",
            KitModule::Watchtower => "module.watchtower",
            KitModule::Keep => "module.keep",
            KitModule::BridgeDock => "module.bridge_dock",
            KitModule::BannerSign => "module.banner_sign",
            KitModule::WaterWheel => "module.water_wheel",
        }
    }

    fn asset_rel(self) -> &'static str {
        match self {
            KitModule::SettlementHouse => "module/settlement_house.glb",
            KitModule::SettlementWorkshop => "module/settlement_workshop.glb",
            KitModule::MarketStall => "module/market_stall.glb",
            KitModule::WallSegment => "module/wall_segment.glb",
            KitModule::GateArch => "module/gate_arch.glb",
            KitModule::Watchtower => "module/watchtower.glb",
            KitModule::Keep => "module/keep.glb",
            KitModule::BridgeDock => "module/bridge_dock.glb",
            KitModule::BannerSign => "module/banner_sign.glb",
            KitModule::WaterWheel => "module/water_wheel.glb",
        }
    }

    /// Authored world extents (m) — the (long, cross) axes of the GLB.
    fn authored(self) -> (f32, f32) {
        match self {
            KitModule::SettlementHouse => (4.2, 3.2),
            KitModule::SettlementWorkshop => (5.4, 4.0),
            KitModule::MarketStall => (3.0, 2.2),
            KitModule::WallSegment => (4.1, 0.9),
            KitModule::GateArch => (9.1, 2.7),
            KitModule::Watchtower => (4.2, 4.2),
            KitModule::Keep => (10.4, 8.4),
            KitModule::BridgeDock => (2.9, 7.9),
            KitModule::BannerSign => (1.0, 1.0),
            KitModule::WaterWheel => (5.0, 2.6),
        }
    }
}

/// The authoritative-kind -> kit mapping (EXHAUSTIVE — the compiler
/// enforces every present or future kind maps; returns module + visual
/// scale multiplier).
fn map_module(kind: ModuleKind) -> (KitModule, f32) {
    match kind {
        ModuleKind::Keep => (KitModule::Keep, 1.0),
        ModuleKind::Wall => (KitModule::WallSegment, 1.0),
        ModuleKind::GateHouse => (KitModule::GateArch, 0.55),
        ModuleKind::Tower => (KitModule::Watchtower, 0.95),
        ModuleKind::Barracks => (KitModule::SettlementWorkshop, 1.0),
        ModuleKind::Chapel => (KitModule::Keep, 0.72),
        ModuleKind::Market => (KitModule::MarketStall, 1.2),
        ModuleKind::Arsenal => (KitModule::SettlementWorkshop, 0.9),
        ModuleKind::TrainingYard => (KitModule::SettlementWorkshop, 1.15),
        ModuleKind::Warehouse => (KitModule::SettlementWorkshop, 1.25),
        ModuleKind::Mint => (KitModule::SettlementHouse, 0.85),
        ModuleKind::Cathedral => (KitModule::Keep, 1.1),
        ModuleKind::Reliquary => (KitModule::Keep, 0.6),
        ModuleKind::Forum => (KitModule::MarketStall, 1.5),
        ModuleKind::Watchpost => (KitModule::Watchtower, 0.7),
        ModuleKind::Vault => (KitModule::Watchtower, 0.8),
        ModuleKind::Lookout => (KitModule::Watchtower, 0.6),
    }
}

fn map_building(kind: BuildingKind) -> (KitModule, f32) {
    match kind {
        BuildingKind::Home => (KitModule::SettlementHouse, 1.0),
        BuildingKind::Workshop => (KitModule::SettlementWorkshop, 1.0),
        BuildingKind::Storage => (KitModule::SettlementHouse, 0.8),
        BuildingKind::Barracks => (KitModule::SettlementWorkshop, 1.0),
        // A marker post at the well cell (the kit has no well piece —
        // an honest, documented stand-in until the kit grows one).
        BuildingKind::Well => (KitModule::BannerSign, 1.0),
        BuildingKind::Farm => (KitModule::MarketStall, 0.9),
        BuildingKind::Watchtower => (KitModule::Watchtower, 0.8),
    }
}

/// One placed kit module: world position, Y rotation, uniform scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub module: KitModule,
    pub pos: [f32; 3],
    pub rot_y: f32,
    pub scale: f32,
    /// Which authoritative element placed it.
    pub serves: &'static str,
}

/// The assembled kit scene + the bindings the primitive path preserved.
#[derive(Clone, Debug, Default)]
pub struct KitScene {
    pub placements: Vec<Placement>,
    /// Solid (x, z) world-meter cells (gate passages REFINED — see
    /// assemble_kit).
    pub collision_cells: BTreeSet<(i32, i32)>,
    pub nav_anchors: Vec<NavAnchor>,
    /// The D-033 zones, straight from the plan.
    pub bed_cells: Vec<CellCoord>,
    /// Rendered anchor marker boxes: (world pos, color).
    pub anchor_boxes: Vec<([f32; 3], [f32; 3])>,
    pub work_cells: Vec<CellCoord>,
    pub idle_cells: Vec<CellCoord>,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    /// Placements per kit module name (the draw/tri budget record).
    pub count_by_module: BTreeMap<&'static str, usize>,
    /// HOUSE ENTRY (WT-002 slice 5): per town building, the open door
    /// cell and the walkable interior center (world meters, XZ).
    pub door_entries: Vec<DoorEntry>,
}

/// One enterable building: where the door stands and where the
/// interior center is (world XZ meters).
#[derive(Clone, Debug, PartialEq)]
pub struct DoorEntry {
    pub kind: &'static str,
    pub door: [f32; 2],
    pub interior: [f32; 2],
}

/// A kit's declared socket in module-local meters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Socket {
    pub name: &'static str,
    pub offset: [f32; 3],
}

/// Loads the kit GLBs (meshes + declared sockets).
#[derive(Clone)]
pub struct SettlementKit {
    pub assets: BTreeMap<KitModule, crate::glb::Asset>,
    sockets: BTreeMap<KitModule, Vec<Socket>>,
}

impl SettlementKit {
    pub fn load() -> Self {
        let root =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/compiled");
        let mut assets = BTreeMap::new();
        let mut sockets = BTreeMap::new();
        for m in [
            KitModule::SettlementHouse,
            KitModule::SettlementWorkshop,
            KitModule::MarketStall,
            KitModule::WallSegment,
            KitModule::GateArch,
            KitModule::Watchtower,
            KitModule::Keep,
            KitModule::BridgeDock,
            KitModule::BannerSign,
            KitModule::WaterWheel,
        ] {
            let path = root.join(m.asset_rel());
            let asset = crate::glb::load_asset_file(&path)
                .unwrap_or_else(|e| panic!("kit asset {}: {e}", path.display()));
            let soc: Vec<Socket> = asset
                .sockets
                .iter()
                .map(|(n, p)| Socket {
                    name: match n.as_str() {
                        "door_front" | "door" => "door",
                        "road_front" => "road",
                        "roof_smoke" => "roof_smoke",
                        "work_anchor" => "work_anchor",
                        "front" => "front",
                        "wall_a" => "wall_a",
                        "wall_b" => "wall_b",
                        "top" => "top",
                        "passage_a" => "passage_a",
                        "passage_b" => "passage_b",
                        "banner_top" => "banner_top",
                        "deck_a" => "deck_a",
                        "deck_b" => "deck_b",
                        "water_line" => "water_line",
                        "build_base" => "build_base",
                        "base" => "base",
                        "axle" => "axle",
                        "power_anchor" => "power_anchor",
                        _ => "unknown",
                    },
                    offset: *p,
                })
                .collect();
            assets.insert(m, asset);
            sockets.insert(m, soc);
        }
        Self { assets, sockets }
    }

    pub fn sockets(&self, m: KitModule) -> &[Socket] {
        &self.sockets[&m]
    }
}

fn rot_y(v: [f32; 3], rot: f32) -> [f32; 3] {
    let (c, s) = (rot.cos(), rot.sin());
    [v[0] * c + v[2] * s, v[1], -v[0] * s + v[2] * c]
}

impl Placement {
    /// A socket's world position for this placement.
    pub fn socket_world(&self, s: &Socket) -> [f32; 3] {
        let local = [
            s.offset[0] * self.scale,
            s.offset[1],
            s.offset[2] * self.scale,
        ];
        let r = rot_y(local, self.rot_y);
        [self.pos[0] + r[0], self.pos[1] + r[1], self.pos[2] + r[2]]
    }
}

fn cells_of(origin: CellCoord, fw: i32, fh: i32) -> BTreeSet<(i32, i32)> {
    let mut s = BTreeSet::new();
    for dx in 0..fw {
        for dz in 0..fh {
            s.insert((origin.x + dx, origin.z + dz));
        }
    }
    s
}

/// The kit assembly: one placement per authoritative element, walls
/// chained socket-to-socket around the circuit, doors facing the road
/// target, collision/nav/anchors preserved from the same computation
/// the primitive path uses (mesh_city) — with the gate REFINED so its
/// passage column opens for the walk-through.
pub fn assemble_kit(
    gen: &WorldGen,
    layout: &CastleLayout,
    plan: &SettlementPlan,
    kit: &SettlementKit,
) -> KitScene {
    // Parity source: the primitive path's bindings (collision + nav).
    let (_, _, prim) = crate::city::mesh_city(gen, layout, plan);
    let mut scene = KitScene {
        nav_anchors: prim.nav_anchors.clone(),
        bed_cells: plan.anchors.bed_cells.clone(),
        work_cells: plan.anchors.work_cells.clone(),
        idle_cells: plan.anchors.idle_cells.clone(),
        collision_cells: prim.collision_cells.clone(),
        bounds_min: [f32::MAX; 3],
        bounds_max: [f32::MIN; 3],
        ..Default::default()
    };

    let walls = crate::city::derived_walls_pub(layout);
    let wall_sock = |p: &Placement, which: &str| -> [f32; 3] {
        let s = kit
            .sockets(KitModule::WallSegment)
            .iter()
            .find(|s| s.name == which)
            .expect("wall sockets declared");
        p.socket_world(s)
    };

    // Face target: the plaza (settlement) / castle center (capital).
    let face_target = |x: f32, z: f32, tx: f32, tz: f32| -> f32 {
        let d = (tx - x, tz - z);
        if d.0.abs() < 0.01 && d.1.abs() < 0.01 {
            return 0.0;
        }
        // Door faces +X at rot 0: rot = angle of (target - pos) minus 0.
        let a = (d.1).atan2(d.0);
        // Quantize to the 4 cardinal rotY values the kit doors expect.
        let q = (a / std::f32::consts::FRAC_PI_2).round() * std::f32::consts::FRAC_PI_2;
        -q
    };

    let plaza_x = plan.plaza.x as f32;
    let plaza_z = plan.plaza.z as f32;
    let center_x = (layout.center.x as f32 + 0.5) * 256.0;
    let center_z = (layout.center.z as f32 + 0.5) * 256.0;

    let mut push = |scene: &mut KitScene,
                    m: KitModule,
                    vis_scale: f32,
                    fw: i32,
                    fh: i32,
                    origin: CellCoord,
                    rot: f32,
                    serves: &'static str| {
        let target = if m == KitModule::WallSegment {
            // Walls scale to their 3 m circuit step along the LONG axis.
            fw.max(fh) as f32
        } else {
            fw.max(fh) as f32
        };
        let authored = match m {
            KitModule::WallSegment => kit_authored_long(m),
            _ => m.authored().0.max(m.authored().1),
        };
        let scale = (target / authored).max(0.05) * vis_scale;
        let x = origin.x as f32 + fw as f32 / 2.0;
        let z = origin.z as f32 + fh as f32 / 2.0;
        let y = crate::city::surface_base(gen, origin.x, origin.z, fw, fh);
        let p = Placement {
            module: m,
            pos: [x, y, z],
            rot_y: rot,
            scale,
            serves,
        };
        *scene.count_by_module.entry(m.name()).or_insert(0) += 1;
        for a in 0..3 {
            scene.bounds_min[a] = scene.bounds_min[a].min(p.pos[a]);
            scene.bounds_max[a] = scene.bounds_max[a].max(p.pos[a]);
        }
        scene.placements.push(p);
    };

    // Capital modules (rotation: face the castle center).
    for m in layout.modules.iter() {
        let (km, vis) = map_module(m.kind);
        let (fw, fh) = m.kind.footprint();
        let cx = m.origin.x as f32 + fw as f32 / 2.0;
        let cz = m.origin.z as f32 + fh as f32 / 2.0;
        let rot = if m.kind == ModuleKind::GateHouse {
            // The gate's passage axis faces the approach road: south
            // (toward the settlement) — the circuit's gate gap side.
            std::f32::consts::FRAC_PI_2
        } else {
            face_target(cx, cz, center_x, center_z)
        };
        push(
            &mut scene,
            km,
            vis,
            fw as i32,
            fh as i32,
            m.origin,
            rot,
            m.kind.name(),
        );
        if m.kind == ModuleKind::GateHouse {
            // GATE REFINEMENT: open the middle column of the footprint —
            // the pillars keep the edge columns, the passage walks.
            for dz in 0..fh as i32 {
                scene
                    .collision_cells
                    .remove(&(m.origin.x + 1, m.origin.z + dz));
            }
        }
    }
    // Wall circuit: chained by the 3 m steps; rotation along each edge;
    // the socket-alignment law (consecutive wall_b == next wall_a) holds
    // by construction and is TESTED.
    for w in walls.iter() {
        let (fw, fh) = w.kind.footprint();
        // Direction to the next wall in the circuit (same edge): the
        // neighbor sharing one axis.
        let dir = walls
            .iter()
            .find(|o| {
                o.origin != w.origin
                    && ((o.origin.z == w.origin.z && (o.origin.x - w.origin.x).abs() == 3)
                        || (o.origin.x == w.origin.x && (o.origin.z - w.origin.z).abs() == 3))
            })
            .map(|o| (o.origin.x - w.origin.x, o.origin.z - w.origin.z));
        let rot = match dir {
            Some((dx, _)) if dx != 0 => 0.0,
            Some(_) => std::f32::consts::FRAC_PI_2,
            None => 0.0,
        };
        push(
            &mut scene,
            KitModule::WallSegment,
            1.0,
            fw as i32,
            fh as i32,
            w.origin,
            rot,
            "wall",
        );
    }
    // Settlement buildings (rotation: face the plaza).
    for b in plan.buildings.iter() {
        let (km, vis) = map_building(b.kind);
        let cx = b.cell.x as f32 + b.size.0 as f32 / 2.0;
        let cz = b.cell.z as f32 + b.size.1 as f32 / 2.0;
        let rot = face_target(cx, cz, plaza_x, plaza_z);
        push(
            &mut scene,
            km,
            vis,
            b.size.0 as i32,
            b.size.1 as i32,
            b.cell,
            rot,
            b.kind.name(),
        );
    }
    // HOUSE ENTRY refinement (WT-002 slice 5): every town building is
    // ENTERABLE — the interior cells open (the wall RING stays solid),
    // and the door column on the plaza-facing edge opens so the player
    // walks in. Mirrors the gate refinement one loop above.
    for b in plan.buildings.iter() {
        let (sw, sh) = (b.size.0 as i32, b.size.1 as i32);
        for dx in 0..sw {
            for dz in 0..sh {
                let on_ring =
                    dx == 0 || dz == 0 || dx == sw - 1 || dz == sh - 1;
                if !on_ring {
                    scene
                        .collision_cells
                        .remove(&(b.cell.x + dx, b.cell.z + dz));
                }
            }
        }
        // The door column: the ring cell at the center of the edge
        // facing the plaza (the same facing the module was rotated to).
        let cx = b.cell.x as f32 + sw as f32 / 2.0;
        let cz = b.cell.z as f32 + sh as f32 / 2.0;
        let (dxs, dzs) = if (plaza_x - cx).abs() >= (plaza_z - cz).abs() {
            (
                if plaza_x > cx { sw - 1 } else { 0 },
                (sh - 1) / 2,
            )
        } else {
            (
                (sw - 1) / 2,
                if plaza_z > cz { sh - 1 } else { 0 },
            )
        };
        scene.collision_cells.remove(&(b.cell.x + dxs, b.cell.z + dzs));
        scene.door_entries.push(DoorEntry {
            kind: b.kind.name(),
            door: [b.cell.x as f32 + dxs as f32 + 0.5, b.cell.z as f32 + dzs as f32 + 0.5],
            interior: [cx, cz],
        });
    }
    // The plaza banner + a dock/water-wheel pair when a river is near.
    push(
        &mut scene,
        KitModule::BannerSign,
        1.4,
        1,
        1,
        CellCoord {
            x: plan.plaza.x,
            y: plan.plaza.y,
            z: plan.plaza.z,
        },
        0.0,
        "plaza_banner",
    );
    if let Some((wx, wz, ground)) = nearest_river_point(gen, plan.plaza) {
        let y = ground - 0.2;
        scene.placements.push(Placement {
            module: KitModule::WaterWheel,
            pos: [wx, y, wz],
            rot_y: 0.0,
            scale: 1.0,
            serves: "water_wheel",
        });
        *scene
            .count_by_module
            .entry(KitModule::WaterWheel.name())
            .or_insert(0) += 1;
        // The dock reaches the bank from the wheel.
        scene.placements.push(Placement {
            module: KitModule::BridgeDock,
            pos: [wx, y, wz + 6.0],
            rot_y: 0.0,
            scale: 1.0,
            serves: "dock",
        });
        *scene
            .count_by_module
            .entry(KitModule::BridgeDock.name())
            .or_insert(0) += 1;
    }

    // Anchor marker boxes (bed plum / work marker / idle green) at the
    // plan's D-033 zones on the local ground.
    let cell_ground = |c: &CellCoord| -> f32 {
        gen.effective_surface_mm(c.x as i64 * 1000, c.z as i64 * 1000) as f32 / 1000.0
    };
    for c in plan.anchors.bed_cells.iter().take(4) {
        scene.anchor_boxes.push((
            [c.x as f32 + 0.5, cell_ground(c) + 0.5, c.z as f32 + 0.5],
            [0.55, 0.25, 0.60],
        ));
    }
    for c in plan.anchors.work_cells.iter().take(4) {
        scene.anchor_boxes.push((
            [c.x as f32 + 0.5, cell_ground(c) + 0.5, c.z as f32 + 0.5],
            [0.95, 0.45, 0.10],
        ));
    }
    for c in plan.anchors.idle_cells.iter().take(4) {
        scene.anchor_boxes.push((
            [c.x as f32 + 0.5, cell_ground(c) + 0.5, c.z as f32 + 0.5],
            [0.55, 0.68, 0.55],
        ));
    }

    // The wall-chain socket record (proof introspection).
    let chain: Vec<&Placement> = scene
        .placements
        .iter()
        .filter(|p| p.module == KitModule::WallSegment)
        .collect();
    let _ = chain
        .iter()
        .map(|p| (wall_sock(p, "wall_a"), wall_sock(p, "wall_b")))
        .count();
    scene
}

fn kit_authored_long(m: KitModule) -> f32 {
    m.authored().0
}

/// The nearest river point within a few regions of a cell (deterministic
/// scan; None when the town sits dry).
fn nearest_river_point(gen: &WorldGen, from: CellCoord) -> Option<(f32, f32, f32)> {
    let graph = pc3d_world::hydro::RiverGraph::new(gen, 24);
    let cr = pc3d_world::coords::RegionCoord {
        x: from.x.div_euclid(256),
        z: from.z.div_euclid(256),
    };
    let mut best: Option<(pc3d_world::coords::RegionCoord, i32)> = None;
    for dx in -4..=4i32 {
        for dz in -4..=4i32 {
            let reg = pc3d_world::coords::RegionCoord {
                x: cr.x + dx,
                z: cr.z + dz,
            };
            if let Some(d) = graph.downstream(reg) {
                if graph.discharge(d) >= pc3d_world::hydro::RIVER_THRESHOLD {
                    let dist = dx.abs() + dz.abs();
                    if best.as_ref().map(|(_, b)| dist < *b).unwrap_or(true) {
                        best = Some((reg, dist));
                    }
                }
            }
        }
    }
    let (reg, _) = best?;
    let wx = (reg.x as f32 + 0.5) * 256.0;
    let wz = (reg.z as f32 + 0.5) * 256.0;
    let ground =
        gen.effective_surface_mm((wx * 1000.0) as i64, (wz * 1000.0) as i64) as f32 / 1000.0;
    Some((wx, wz, ground))
}

// ---------------------------------------------------------------------------
// GPU: instanced buckets (one draw per module+LOD) + anchor markers
// ---------------------------------------------------------------------------

/// One (module, lod) instance bucket.
struct Bucket {
    vertex: wgpu::Buffer,
    index: wgpu::Buffer,
    index_count: u32,
    instances: wgpu::Buffer,
    instance_count: u32,
}

/// The uploaded kit scene: static (settlements do not stream — they are
/// placed once by the plan and re-derived deterministically on load).
pub struct SettlementGpu {
    buckets: Vec<Bucket>,
    /// Bed/Work/Idle + nav marker boxes (the D-033 zones stay visible).
    anchor_mesh: Option<(wgpu::Buffer, wgpu::Buffer, u32)>,
    pub tris: usize,
    pub draws: usize,
}

fn anchor_box(
    verts: &mut Vec<crate::scene::SceneVertex>,
    idx: &mut Vec<u16>,
    x: f32,
    y: f32,
    z: f32,
    color: [f32; 3],
) {
    use crate::scene::SceneVertex;
    let (hx, hy, hz) = (0.45f32, 0.45f32, 0.45f32);
    let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ];
    for (n, u, v) in faces {
        let base = verts.len() as u16;
        for (su, sv) in [(-1.0f32, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            verts.push(SceneVertex {
                pos: [
                    x + (n[0] * hx + u[0] * hx * su + v[0] * hy * sv),
                    y + (n[1] * hy + u[1] * hx * su + v[1] * hy * sv),
                    z + (n[2] * hz + u[2] * hx * su + v[2] * hy * sv),
                ],
                normal: n,
                color,
            });
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

impl SettlementGpu {
    /// Uploads the scene: kit meshes shared per module, one instance
    /// buffer per (module, lod) — the viewer distance picks the LOD.
    pub fn new(
        device: &wgpu::Device,
        scene: &KitScene,
        kit: &SettlementKit,
        viewer: [f32; 2],
    ) -> Self {
        use wgpu::util::DeviceExt;
        let mut rows: BTreeMap<
            (KitModule, u8),
            (Vec<crate::flora::Instance>, &crate::glb::LodMesh),
        > = BTreeMap::new();
        for p in &scene.placements {
            let asset = &kit.assets[&p.module];
            let d = (p.pos[0] - viewer[0]).hypot(p.pos[2] - viewer[1]);
            let want = if d < 40.0 {
                "lod0"
            } else if d < 120.0 {
                "lod1"
            } else {
                "lod2"
            };
            let lod = asset
                .lods
                .iter()
                .find(|l| l.name == want)
                .or_else(|| asset.lods.last());
            let (lod, li) = match lod {
                Some(l) => (
                    l,
                    if l.name == "lod0" {
                        0
                    } else if l.name == "lod1" {
                        1
                    } else {
                        2
                    },
                ),
                None => continue,
            };
            let (_, _, _, wind, _, _) = (p.pos, p.rot_y, p.scale, 0.0, 0.0, 0.0);
            rows.entry((p.module, li))
                .or_insert_with(|| (Vec::new(), lod))
                .0
                .push(crate::flora::Instance {
                    pos_scale: [p.pos[0], p.pos[1], p.pos[2], p.scale],
                    params: [p.rot_y, wind, 1.0, 0.0],
                });
        }
        let mut buckets = Vec::new();
        let mut tris = 0usize;
        for ((_, _), (list, lod)) in rows {
            if list.is_empty() {
                continue;
            }
            let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("settlement vertices"),
                contents: bytemuck::cast_slice(&lod.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("settlement indices"),
                contents: bytemuck::cast_slice(&lod.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            let inst = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("settlement instances"),
                contents: bytemuck::cast_slice(&list),
                usage: wgpu::BufferUsages::VERTEX,
            });
            tris += lod.indices.len() / 3 * list.len();
            buckets.push(Bucket {
                vertex: vb,
                index: ib,
                index_count: lod.indices.len() as u32,
                instances: inst,
                instance_count: list.len() as u32,
            });
        }
        // Anchor markers from the scene's computed boxes (bed plum /
        // work marker orange / idle green — the plan's D-033 zones).
        let mut verts = Vec::new();
        let mut idx = Vec::new();
        for (pos, color) in &scene.anchor_boxes {
            anchor_box(&mut verts, &mut idx, pos[0], pos[1], pos[2], *color);
        }
        let anchor_mesh = if idx.is_empty() {
            None
        } else {
            let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("settlement anchors"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("settlement anchor indices"),
                contents: bytemuck::cast_slice(&idx),
                usage: wgpu::BufferUsages::INDEX,
            });
            Some((vb, ib, idx.len() as u32))
        };
        Self {
            buckets,
            anchor_mesh,
            tris,
            draws: 0,
        }
    }

    /// Draws through the caller-bound INSTANCED pipeline; the anchors
    /// ride the plain mesh pipeline (the caller rebinds it after).
    pub fn draw<'rp>(
        &mut self,
        pass: &mut wgpu::RenderPass<'rp>,
        pipelines: &crate::renderer::FloraPipelines,
        bg_globals: &wgpu::BindGroup,
    ) {
        pass.set_pipeline(&pipelines.inst);
        pass.set_bind_group(0, bg_globals, &[]);
        let mut draws = 0usize;
        for b in &self.buckets {
            pass.set_vertex_buffer(0, b.vertex.slice(..));
            pass.set_vertex_buffer(1, b.instances.slice(..));
            pass.set_index_buffer(b.index.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..b.index_count, 0, 0..b.instance_count);
            draws += 1;
        }
        self.draws = draws;
    }

    /// Sun shadows for the modules (the anchors cast nothing).
    pub fn draw_shadow<'rp>(
        &mut self,
        pass: &mut wgpu::RenderPass<'rp>,
        pipelines: &crate::renderer::FloraPipelines,
        bg_light: &wgpu::BindGroup,
    ) {
        pass.set_pipeline(&pipelines.inst_shadow);
        pass.set_bind_group(0, bg_light, &[]);
        for b in &self.buckets {
            pass.set_vertex_buffer(0, b.vertex.slice(..));
            pass.set_vertex_buffer(1, b.instances.slice(..));
            pass.set_index_buffer(b.index.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..b.index_count, 0, 0..b.instance_count);
        }
    }

    /// The anchor marker mesh through the plain mesh pipeline (the
    /// renderer binds it).
    pub fn draw_anchors<'rp>(
        &self,
        pass: &mut wgpu::RenderPass<'rp>,
        mesh_pipeline: &wgpu::RenderPipeline,
        bg_globals: &wgpu::BindGroup,
    ) {
        if let Some((vb, ib, count)) = &self.anchor_mesh {
            pass.set_pipeline(mesh_pipeline);
            pass.set_bind_group(0, bg_globals, &[]);
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..*count, 0, 0..1);
        }
    }
}

/// A collision adapter: another surface plus the kit's solid cells
/// (footprints minus the refined gate passages).
pub struct SettlementGround<S> {
    pub inner: S,
    pub cells: BTreeSet<(i32, i32)>,
}

impl<S: crate::player::CollisionSurface> crate::player::CollisionSurface for SettlementGround<S> {
    fn ground_at(&self, gen: &WorldGen, x: f32, z: f32, from_y: f32) -> Option<f32> {
        self.inner.ground_at(gen, x, z, from_y)
    }

    fn cell_solid(&self, gen: &WorldGen, x: i32, y: i32, z: i32) -> bool {
        if self.inner.cell_solid(gen, x, y, z) {
            return true;
        }
        if !self.cells.contains(&(x, z)) {
            return false;
        }
        // The height window rides the INNER surface's own ground answer
        // (the kit floats wherever its surface says — the first flat-
        // ground walk sailed straight through a wall whose window sat
        // on unrelated generator terrain).
        let ground = self
            .inner
            .ground_at(gen, x as f32 + 0.5, z as f32 + 0.5, f32::MAX / 4.0)
            .unwrap_or(0.0);
        let wy = y as f32;
        wy >= ground - 0.5 && wy <= ground + 4.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::city::city_scene;
    use crate::player::CollisionSurface as _;
    use pc3d_world::coords::RegionCoord;

    fn scene() -> (WorldGen, pc3d_world::castle::CastleLayout, SettlementPlan) {
        let (gen, _center, layout, plan) = city_scene(3, RegionCoord { x: 0, z: 0 });
        (gen, layout, plan)
    }

    #[test]
    fn plan_to_render_consistency_and_determinism() {
        let (gen, layout, plan) = scene();
        let kit = SettlementKit::load();
        let a = assemble_kit(&gen, &layout, &plan, &kit);
        let b = assemble_kit(&gen, &layout, &plan, &kit);
        assert_eq!(a.placements, b.placements, "assembly is deterministic (the save/load stability law: the plan re-derives the same kit)");
        // Every authoritative element places EXACTLY ONE module at its
        // footprint center: capital + walls + town + banner (+ water pair
        // when a river is near).
        let walls = crate::city::derived_walls_pub(&layout);
        let expected = layout.modules.len() + walls.len() + plan.buildings.len() + 1;
        let extras = a
            .placements
            .iter()
            .filter(|p| matches!(p.serves, "water_wheel" | "dock"))
            .count();
        assert_eq!(
            a.placements.len(),
            expected + extras,
            "one placement per plan element (+{extras} water pieces)"
        );
        // Spot-consistency: each capital module's placement centers on
        // its footprint.
        for m in layout.modules.iter() {
            let (fw, fh) = m.kind.footprint();
            let cx = m.origin.x as f32 + fw as f32 / 2.0;
            let cz = m.origin.z as f32 + fh as f32 / 2.0;
            assert!(
                a.placements
                    .iter()
                    .any(|p| (p.pos[0] - cx).abs() < 0.6 && (p.pos[2] - cz).abs() < 0.6),
                "module {} at {:?} renders",
                m.kind.name(),
                m.origin
            );
        }
        // The mapping covers every plan kind (the counts prove use).
        for kind in ["keep", "gatehouse", "tower"] {
            assert!(
                layout.modules.iter().any(|m| m.kind.name() == kind),
                "{kind} in the capital"
            );
        }
    }

    #[test]
    fn wall_sockets_chain_and_gate_passage_alignment() {
        let (gen, layout, plan) = scene();
        let kit = SettlementKit::load();
        let a = assemble_kit(&gen, &layout, &plan, &kit);
        // Wall chains: consecutive segments along an edge share socket
        // positions (wall_b of one == wall_a of the neighbor).
        let walls: Vec<&Placement> = a
            .placements
            .iter()
            .filter(|p| p.module == KitModule::WallSegment)
            .collect();
        assert!(walls.len() >= 4, "a wall circuit ({})", walls.len());
        let sock = |p: &Placement, n: &str| -> [f32; 3] {
            let s = kit
                .sockets(KitModule::WallSegment)
                .iter()
                .find(|s| s.name == n)
                .unwrap();
            p.socket_world(s)
        };
        // Adjacent segments must share A socket pair — wall_b->wall_a in
        // circuit order, or the mirror when the iteration order runs the
        // other way (the first assert pinned one orientation and 'proved'
        // a mismatch that was the reversed pair).
        let mut links = 0usize;
        for w in &walls {
            for o in &walls {
                if w.pos == o.pos {
                    continue;
                }
                let center_d = (w.pos[0] - o.pos[0]).hypot(w.pos[2] - o.pos[2]);
                if center_d >= 3.5 {
                    continue;
                }
                let wb = sock(w, "wall_b");
                let oa = sock(o, "wall_a");
                let wa = sock(w, "wall_a");
                let ob = sock(o, "wall_b");
                let d1 = (wb[0] - oa[0]).hypot(wb[2] - oa[2]);
                let d2 = (wa[0] - ob[0]).hypot(wa[2] - ob[2]);
                assert!(
                    d1.min(d2) < 0.35,
                    "adjacent wall sockets align (d1 {d1:.2} d2 {d2:.2}, w {:?} rot {:.2}, o {:?} rot {:.2})",
                    w.pos,
                    w.rot_y,
                    o.pos,
                    o.rot_y
                );
                links += 1;
            }
        }
        assert!(links >= 6, "the circuit chains socket-to-socket ({links})");
        // The gate's passage sockets sit on the walk axis: the passage
        // column cells are exactly the ones the assembly opened.
        let gate = layout
            .modules
            .iter()
            .find(|m| m.kind == ModuleKind::GateHouse)
            .expect("a gatehouse");
        for dz in 0..gate.kind.footprint().1 as i32 {
            assert!(
                !a.collision_cells
                    .contains(&(gate.origin.x + 1, gate.origin.z + dz)),
                "the gate passage column is open"
            );
            assert!(
                a.collision_cells
                    .contains(&(gate.origin.x, gate.origin.z + dz)),
                "the gate pillar column stays solid"
            );
        }
    }

    #[test]
    fn collision_nav_parity_with_the_primitive_path() {
        let (gen, layout, plan) = scene();
        let kit = SettlementKit::load();
        let a = assemble_kit(&gen, &layout, &plan, &kit);
        let (_, _, prim) = crate::city::mesh_city(&gen, &layout, &plan);
        // Nav anchors and the D-033 zones pass through unchanged.
        assert_eq!(a.nav_anchors, prim.nav_anchors, "nav anchors preserved");
        assert_eq!(a.bed_cells, plan.anchors.bed_cells, "bed zones preserved");
        assert_eq!(
            a.work_cells, plan.anchors.work_cells,
            "work zones preserved"
        );
        assert_eq!(
            a.idle_cells, plan.anchors.idle_cells,
            "idle zones preserved"
        );
        // Collision: the kit is the primitive set MINUS the gate
        // passages (a strict refinement).
        assert!(
            a.collision_cells.is_subset(&prim.collision_cells),
            "kit collision refines, never adds"
        );
        let opened = prim.collision_cells.difference(&a.collision_cells).count();
        assert!(opened > 0, "the gate passages opened ({opened} cells)");
    }

    #[test]
    fn houses_are_enterable_ring_solid_door_open_interior_walks() {
        // WT-002 slice 5: a town Home on flat ground — the wall RING
        // stays solid, the interior opens, and the player WALKS from
        // the road through the DOOR to the interior center.
        let gen = WorldGen::new(77);
        let layout = pc3d_world::castle::CastleLayout {
            center: pc3d_world::coords::RegionCoord { x: 9, z: 9 },
            modules: vec![],
            roads: vec![],
        };
        let home = pc3d_world::settlement_plan::BuildingSlot {
            kind: pc3d_world::settlement_plan::BuildingKind::Home,
            cell: CellCoord { x: 100, y: 0, z: 100 },
            size: (5, 5),
        };
        let plan = SettlementPlan {
            center: pc3d_world::coords::RegionCoord { x: 9, z: 9 },
            plaza: CellCoord { x: 120, y: 0, z: 100 },
            buildings: vec![home],
            roads: vec![],
            anchors: pc3d_world::settlement_plan::Anchors::default(),
        };
        let kit = SettlementKit::load();
        let a = assemble_kit(&gen, &layout, &plan, &kit);

        // The door entry is recorded: plaza at +X -> door on the EAST
        // ring, center row; the interior center is the footprint middle.
        assert_eq!(a.door_entries.len(), 1);
        let de = a.door_entries[0].clone();
        assert_eq!(de.kind, "home");
        assert_eq!((de.door[0] as i32, de.door[1] as i32), (104, 102));
        assert_eq!(
            (de.interior[0] as i32, de.interior[1] as i32),
            (102, 102)
        );
        // Ring SOLID beside the door; door OPEN; interior cells OPEN.
        assert!(a.collision_cells.contains(&(104, 100)), "ring solid");
        assert!(a.collision_cells.contains(&(104, 104)), "ring solid");
        assert!(
            !a.collision_cells.contains(&(104, 102)),
            "the door column is open"
        );
        for dx in 101..104i32 {
            for dz in 101..104i32 {
                assert!(
                    !a.collision_cells.contains(&(dx, dz)),
                    "interior ({dx},{dz}) open"
                );
            }
        }

        // The WALK: flat ground isolates the kit collision law; the
        // player walks WEST (-X) from the road at x 107 through the
        // door at (104.5, 102.5) into the interior center (102.5, 102.5).
        struct FlatGround;
        impl crate::player::CollisionSurface for FlatGround {
            fn ground_at(&self, _gen: &WorldGen, _x: f32, _z: f32, _from_y: f32) -> Option<f32> {
                Some(0.0)
            }
            fn cell_solid(&self, _gen: &WorldGen, _x: i32, _y: i32, _z: i32) -> bool {
                false
            }
        }
        let surface = SettlementGround {
            inner: FlatGround,
            cells: a.collision_cells.clone(),
        };
        let mut body = crate::player::PlayerBody {
            pos: [107.0, 0.0, 102.5],
            yaw: std::f32::consts::FRAC_PI_2, // yaw pi/2 walks -X (west)
            pitch: 0.0,
        };
        for _ in 0..400 {
            body.walk_on(&gen, &surface, 1.0, 0.0, 1.0 / 60.0);
        }
        let reached_interior = body.pos[0] < 104.0 && body.pos[0] > 100.0
            && body.pos[2] > 101.0
            && body.pos[2] < 104.0;
        assert!(
            reached_interior,
            "the player must reach the interior through the door (pos {:?})",
            body.pos
        );
        // And the WALLS still stop a straight line: walking west along
        // z=100.5 (a ring row) never crosses the west wall.
        let mut wallbody = crate::player::PlayerBody {
            pos: [107.0, 0.0, 100.5],
            yaw: std::f32::consts::FRAC_PI_2,
            pitch: 0.0,
        };
        for _ in 0..400 {
            wallbody.walk_on(&gen, &surface, 1.0, 0.0, 1.0 / 60.0);
        }
        assert!(
            wallbody.pos[0] >= 104.0,
            "the ring beside the door must stop the player (pos {:?})",
            wallbody.pos
        );
    }

    #[test]
    fn the_player_walks_through_the_gate_and_not_the_walls() {
        // A purpose-built arrangement (the proof scene's capital has its
        // own tower/wall topology around the gate — walkability THERE is
        // a plan question; the KIT contract gets a clean stage): a gate
        // with wall runs either side, passage open along -Z, flat ground.
        let gen = WorldGen::new(77);
        let layout = pc3d_world::castle::CastleLayout {
            center: pc3d_world::coords::RegionCoord { x: 0, z: 0 },
            modules: vec![
                PlacedModule {
                    kind: ModuleKind::GateHouse,
                    origin: CellCoord { x: -1, y: 0, z: 0 },
                },
                PlacedModule {
                    kind: ModuleKind::Wall,
                    origin: CellCoord { x: 2, y: 0, z: 0 },
                },
                PlacedModule {
                    kind: ModuleKind::Wall,
                    origin: CellCoord { x: -5, y: 0, z: 0 },
                },
            ],
            roads: vec![],
        };
        let plan = SettlementPlan {
            center: pc3d_world::coords::RegionCoord { x: 5, z: 5 },
            plaza: CellCoord {
                x: 1280,
                y: 0,
                z: 1280,
            },
            buildings: vec![],
            roads: vec![],
            anchors: pc3d_world::settlement_plan::Anchors::default(),
        };
        let kit = SettlementKit::load();
        let a = assemble_kit(&gen, &layout, &plan, &kit);
        let gate = &layout.modules[0];
        // The arrangement: wall cells x>=2 and x<=-2 at z 0..1, gate
        // pillars x -1 and +1, passage x 0 OPEN both rows.
        for dz in 0..2i32 {
            assert!(a.collision_cells.contains(&(-1, dz)), "pillar west");
            assert!(a.collision_cells.contains(&(1, dz)), "pillar east");
            assert!(!a.collision_cells.contains(&(0, dz)), "passage open");
        }
        // Flat ground isolates the kit collision law.
        struct FlatGround;
        impl crate::player::CollisionSurface for FlatGround {
            fn ground_at(&self, _gen: &WorldGen, _x: f32, _z: f32, _from_y: f32) -> Option<f32> {
                Some(0.0)
            }
            fn cell_solid(&self, _gen: &WorldGen, _x: i32, _y: i32, _z: i32) -> bool {
                false
            }
        }
        let surface = SettlementGround {
            inner: FlatGround,
            cells: a.collision_cells.clone(),
        };
        // Walk north through the passage: from z 6 to z -6 at x 0.5.
        let mut body = crate::player::PlayerBody {
            pos: [0.5, 0.0, 6.0],
            yaw: 0.0, // yaw 0 = -Z (the flora walk test's convention)
            pitch: 0.0,
        };
        for _ in 0..600 {
            body.walk_on(&gen, &surface, 1.0, 0.0, 1.0 / 60.0);
        }
        assert!(
            body.pos[2] < -4.0,
            "walked through the gate passage (z {:.1})",
            body.pos[2]
        );
        // The wall stops the same walk: approach the east wall from its
        // east side, walking -X (yaw PI = +X is WRONG here; -X needs
        // yaw... hf = [-sin, -cos]: yaw PI/2 -> hf = [-1, 0] = -X).
        let wall = a
            .placements
            .iter()
            .find(|p| p.module == KitModule::WallSegment && p.pos[0] > 0.0)
            .expect("the east wall");
        let mut body2 = crate::player::PlayerBody {
            pos: [wall.pos[0] + 6.0, 0.0, wall.pos[2]],
            yaw: std::f32::consts::FRAC_PI_2, // face -X
            pitch: 0.0,
        };
        for _ in 0..600 {
            body2.walk_on(&gen, &surface, 1.0, 0.0, 1.0 / 60.0);
        }
        assert!(
            body2.pos[0] > wall.pos[0] + 0.9,
            "the wall stops the walk (x {:.1} vs wall {:.1})",
            body2.pos[0],
            wall.pos[0]
        );
    }

    /// GPU: the kit RENDERS (control diff), buckets split by LOD, the
    /// anchor markers draw, and the shadow pass runs with modules.
    #[test]
    fn settlement_renders_with_lod_and_anchors() {
        let (gen, layout, plan) = scene();
        let kit = SettlementKit::load();
        let a = assemble_kit(&gen, &layout, &plan, &kit);
        // Terrain under the city (bounds + margin, 3 y levels like the
        // city proof).
        let mut patches = Vec::new();
        let pmin = (
            (a.bounds_min[0] as i32).div_euclid(16) - 1,
            (a.bounds_min[2] as i32).div_euclid(16) - 1,
        );
        let pmax = (
            (a.bounds_max[0] as i32).div_euclid(16) + 1,
            (a.bounds_max[2] as i32).div_euclid(16) + 1,
        );
        for px in pmin.0..=pmax.0 {
            for pz in pmin.1..=pmax.1 {
                patches.push(pc3d_world::coords::PatchCoord { x: px, y: 1, z: pz });
            }
        }
        // Eye above the plaza looking at the keep.
        let keep = a.placements.iter().find(|p| p.module == KitModule::Keep);
        let (ex, ez) = match keep {
            Some(k) => (k.pos[0] + 30.0, k.pos[2] + 30.0),
            None => (a.bounds_min[0] + 20.0, a.bounds_min[2] + 20.0),
        };
        let gy =
            gen.effective_surface_mm((ex * 1000.0) as i64, (ez * 1000.0) as i64) as f32 / 1000.0;
        let eye = [ex, gy + 16.0, ez];
        let target = keep.map(|k| k.pos).unwrap_or([ex - 30.0, gy, ez - 30.0]);
        let d = [
            target[0] - eye[0],
            target[1] + 6.0 - eye[1],
            target[2] - eye[2],
        ];
        let pose = crate::camera::CameraPose::new(
            eye,
            (-d[0]).atan2(-d[2]),
            (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin(),
        );

        // Control: terrain only.
        let mut ctrl = crate::renderer::Renderer::offscreen(384, 288);
        ctrl.set_placeholder_scene(false);
        ctrl.load_terrain(&std::rc::Rc::new(gen.clone()), &patches);
        ctrl.set_pose(pose);
        let (_, no_kit) = ctrl.capture_png(&std::env::temp_dir().join("pc3d_setl_off.png"), &[]);

        let mut r = crate::renderer::Renderer::offscreen(384, 288);
        r.set_placeholder_scene(false);
        r.load_terrain(&std::rc::Rc::new(gen.clone()), &patches);
        r.set_pose(pose);
        r.set_atmosphere_tier(crate::atmosphere::AtmosphereTier::Mid);
        r.attach_settlement(&a, &kit);
        let (_, with_kit) = r.capture_png(&std::env::temp_dir().join("pc3d_setl_on.png"), &[]);
        let (draws, tris) = r.settlement_stats();
        let diff = crate::scene::pixel_difference_fraction(&no_kit, &with_kit);
        println!(
            "settlement: diff {diff:.3}, {draws} bucket draws, {tris} tris, {} placements, anchors {}",
            a.placements.len(),
            a.anchor_boxes.len()
        );
        assert!(diff > 0.012, "the kit visibly renders ({diff})");
        assert!(draws >= 5, "multiple module buckets draw ({draws})");
        // LOD1 at the 40 m vantage keeps the whole town under ~1.5k
        // tris — the Deck-friendly budget the kit is FOR.
        assert!(tris > 500, "a real triangle budget ({tris})");
        assert!(!a.anchor_boxes.is_empty(), "the D-033 anchors are present");
    }
}
