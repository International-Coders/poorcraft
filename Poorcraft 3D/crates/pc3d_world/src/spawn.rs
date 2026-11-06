//! N02 — the safe-spawn multi-seed authority (CONTINUACAO-GLM 13, card N02).
//!
//! Spawn is a VALIDATED SEARCH with proof, never an approximate point.
//! [`decide_spawn`] enumerates candidate cells in deterministic rings
//! around the origin, applies the doctrine's laws in cheap-first order,
//! scores only the survivors, and records every rejection with its
//! reason. The N01 trust gate [`crate::trust::spawn_clearance_violations`]
//! is load-bearing HERE in production: a candidate must pass it at the
//! chosen yaw before it can win.
//!
//! The resource law is a PATH law, not a distance law: the nearest
//! starter resource (choppable tree, loose log, or minable boulder) must
//! be reachable through the nav authority — the same A* an NPC walks —
//! within the search budget. Euclidean proximity alone does not count.
//!
//! [`Player::spawn_safe`](crate::player::Player::spawn_safe) delegates
//! to this module; there is no second spawn path in the app.

use crate::coords::{PatchCoord, RegionCoord};
use crate::flora::{plant_at, PlantKind, SlotCoord, SLOT_M};
use crate::gen::{Biome, WorldGen, SEA_LEVEL_M};
use crate::nav::NavPatch;
use crate::scales::{PATCH_CELL_AXIS, REGION_MM};
use crate::trust::{spawn_clearance_violations, TrustViolation};

/// The algorithm identity persisted with the choice (doc 13, step 8).
pub const SPAWN_ALGO_VERSION: u32 = 1;
/// Candidate ring sweep: 4 m slots per ring, up to this radius.
/// 384 slots ≈ 1.5 km — the deep-ocean world (seed 0's nearest land is
/// region -16, 4 km out) still exhausts this and falls back honestly;
/// the fallback tier exists exactly for that shape of world.
pub const SLOT_RING_MAX: i32 = 384;

/// Walkability: max |Δh| across the 4 m slot footprint (m per slot).
pub const MAX_STEP_PER_SLOT_M: f32 = 1.0;
/// Fall margin: ground within this distance must not drop deeper than a
/// step below the candidate (cliff/river-edge guard).
pub const MARGIN_RING_SLOTS: i32 = 2;
pub const MAX_MARGIN_DROP_M: f32 = 3.0;
/// Water margin: no water cell within this ring of slots.
pub const WATER_MARGIN_SLOTS: i32 = 3;
/// Starter resource must be reachable within this many slots.
pub const RESOURCE_RING_MAX: i32 = 10;
/// A confirmed resource path may not exceed this many A* cells — the
/// first errand is short by law (the A* hard node cap stays the safety
/// ceiling inside the nav authority).
pub const RESOURCE_PATH_MAX_CELLS: usize = 256;
/// Score weights — only survivors are scored; the order never gates.
const W_FLAT: f32 = 1.0;
const W_RESOURCE: f32 = 0.5;
const W_ORIGIN: f32 = 0.02;

/// One candidate the search evaluated.
#[derive(Clone, Debug)]
pub struct SpawnCandidate {
    /// Feet position (meters, on the sampled surface).
    pub feet: [f32; 3],
    /// Yaw that passed the clearance gate (faces the clearest quadrant).
    pub yaw_deg: f32,
    /// Flora slot (4 m) this candidate sits on.
    pub slot: SlotCoord,
    /// The macro biome under the candidate.
    pub biome: Biome,
    /// Nearest nav-reachable starter resource (slot, kind, path length).
    pub resource: Option<Resource>,
    /// Lower is better; only already-valid candidates are compared.
    pub score: f32,
}

/// A confirmed starter resource: what, where, and the nav path length.
#[derive(Clone, Debug)]
pub struct Resource {
    pub kind: PlantKind,
    pub slot: SlotCoord,
    pub path_cells: usize,
}

/// Why a candidate (or a whole search) was rejected. The sidecar keeps
/// every named refusal — a silent timeout is forbidden (doc 05).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SpawnRejection {
    /// The rejected flora slot (4 m) as `[x, z]` (sidecar-friendly).
    pub slot: [i32; 2],
    pub law: String,
    pub detail: String,
}

/// The full decision: winner (or none), every rejection, counts.
#[derive(Clone, Debug)]
pub struct SpawnDecision {
    pub algo_version: u32,
    pub seed: u64,
    pub candidate: Option<SpawnCandidate>,
    pub evaluated: u32,
    pub rejections: Vec<SpawnRejection>,
    /// True when the tier-2 legacy region fallback made the pick.
    pub used_fallback: bool,
}

impl SpawnDecision {
    /// The spawn feet position (None when the search honestly failed).
    pub fn feet(&self) -> Option<[f32; 3]> {
        self.candidate.as_ref().map(|c| c.feet)
    }

    /// JSON sidecar (doc 13 step 8: persist choice + algo version; the
    /// ledger keeps every named rejection, capped to keep files sane).
    pub fn to_sidecar_json(&self) -> serde_json::Value {
        serde_json::json!({
            "schema": 1,
            "algo_version": self.algo_version,
            "seed": self.seed,
            "used_fallback": self.used_fallback,
            "evaluated": self.evaluated,
            "candidate": self.candidate.as_ref().map(|c| serde_json::json!({
                "feet_m": c.feet,
                "yaw_deg": c.yaw_deg,
                "slot": [c.slot.x, c.slot.z],
                "biome": format!("{:?}", c.biome),
                "resource": c.resource.as_ref().map(|r| serde_json::json!({
                    "kind": r.kind.name(),
                    "slot": [r.slot.x, r.slot.z],
                    "path_cells": r.path_cells,
                })),
                "score": c.score,
            })),
            // The full ledger is the DAILY matrix's business; a per-decision
            // sidecar keeps the last 64 named refusals.
            "rejections_tail": self.rejections.iter().rev().take(64).map(|r| {
                serde_json::json!({ "slot": r.slot, "law": r.law, "detail": r.detail })
            }).collect::<Vec<_>>(),
        })
    }
}

/// Slot key for a world meter position (flora's 4 m grid).
fn slot_at(x_m: f32, z_m: f32) -> SlotCoord {
    SlotCoord::from_world_mm((x_m * 1000.0) as i64, (z_m * 1000.0) as i64)
}

fn slot_center(slot: SlotCoord) -> [f32; 2] {
    slot.center_m()
}

/// Surface height (m) at a world point — the final-surface authority
/// every other subsystem samples.
fn surface_m(gen: &WorldGen, x_m: f32, z_m: f32) -> f32 {
    gen.effective_surface_mm((x_m * 1000.0) as i64, (z_m * 1000.0) as i64) as f32 / 1000.0
}

/// Is this flora a starter resource the first hour needs (doc 13 N02)?
fn is_starter_resource(kind: PlantKind) -> bool {
    kind.is_tree() || matches!(kind, PlantKind::Log | PlantKind::RockBoulder)
}

/// The walkable-support probe at a slot: floor solid with headroom,
/// gentle slope across the footprint, dry, and above sea level.
/// Feet sit at the TOP of the surface cell (floor_cell + 1) — the same
/// cell-index height the cell probe (`final_solid` at cell y * 1000)
/// and the N01 floor law read, so all three agree by construction.
/// (The analytic mm offset `surface + ε` disagrees on terraced ground:
/// the surface mm can fall INSIDE a solid cell's span, while the cell
/// probe is defined at the cell base — cell bases are the shared law.)
fn slot_support(gen: &WorldGen, slot: SlotCoord) -> Option<[f32; 3]> {
    let [cx, cz] = slot_center(slot);
    let (cx_mm, cz_mm) = ((cx * 1000.0) as i64, (cz * 1000.0) as i64);
    // THE SLOT's center CELL: slot index * 4 m + half a slot, in cells.
    // (Center meters * 1000 div 1000 was the bug: 6.0 m → cell 26 when
    // SLOT_M got multiplied in twice.)
    let cell_x = (slot.x as i64 * SLOT_M * 1000 + SLOT_M * 500).div_euclid(1000) as i32;
    let cell_z = (slot.z as i64 * SLOT_M * 1000 + SLOT_M * 500).div_euclid(1000) as i32;
    let surface_cell = gen.effective_surface_mm(cx_mm, cz_mm).div_euclid(1000) as i32;
    // Find the standing cell: the surface cell itself when passable
    // there (feet inside it), else the first passable cell above.
    let mut feet_cell = None;
    for y in surface_cell..=(surface_cell + 3) {
        if passable_cell(gen, cell_x, y, cell_z) && passable_cell(gen, cell_x, y + 1, cell_z) {
            feet_cell = Some(y);
            break;
        }
    }
    let Some(fc) = feet_cell else { return None };
    let gy = fc as f32;
    let feet = gy + 0.01;
    // Sea/dry law: the standing cell must start at or above sea level.
    if fc < SEA_LEVEL_M {
        return None;
    }
    // Slope law: the 4 corners of the slot may not step more than the
    // controller's 1 m step inside the footprint.
    let h00 = gy;
    let hx = gen.effective_surface_mm(cx_mm + SLOT_M as i64 * 1000, cz_mm) as f32 / 1000.0;
    let hz = gen.effective_surface_mm(cx_mm, cz_mm + SLOT_M as i64 * 1000) as f32 / 1000.0;
    let hxx = gen.effective_surface_mm(cx_mm + SLOT_M as i64 * 1000, cz_mm + SLOT_M as i64 * 1000)
        as f32
        / 1000.0;
    let max_step = (hx - h00).abs().max((hz - h00).abs()).max((hxx - h00).abs());
    if max_step > MAX_STEP_PER_SLOT_M {
        return None;
    }
    Some([cx, feet, cz])
}

/// Cell passability at cell-index coordinates (air or water) — the
/// same question the player controller and the p3d401 contract ask.
fn passable_cell(gen: &WorldGen, cx: i32, cy: i32, cz: i32) -> bool {
    let a = crate::terrain::final_solid(gen, cx as i64 * 1000, cy as i64 * 1000, cz as i64 * 1000);
    matches!(a.material, crate::gen::CellMaterial::Air | crate::gen::CellMaterial::Water)
}

/// Margin laws around a candidate: no cliff/river-edge drop inside the
/// margin ring, no water inside the water ring. Water ring first with
/// an early exit — a coast slot must fail in ~4 samples, not 49.
fn margin_violations(gen: &WorldGen, slot: SlotCoord, feet_y: f32) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    // Water ring (the cheap, most-rejecting law on coastlines).
    'water: for dz in -WATER_MARGIN_SLOTS..=WATER_MARGIN_SLOTS {
        for dx in -WATER_MARGIN_SLOTS..=WATER_MARGIN_SLOTS {
            let s = SlotCoord { x: slot.x + dx, z: slot.z + dz };
            let [sx, sz] = slot_center(s);
            let gy = surface_m(gen, sx, sz);
            if gy <= SEA_LEVEL_M as f32 {
                out.push(("water_margin", format!("slot ({:+},{:+}) is water", dx, dz)));
                break 'water; // one named refusal is enough for the ledger
            }
        }
    }
    if !out.is_empty() {
        return out;
    }
    // Drop ring (only for slots that already look dry).
    for dz in -MARGIN_RING_SLOTS..=MARGIN_RING_SLOTS {
        for dx in -MARGIN_RING_SLOTS..=MARGIN_RING_SLOTS {
            if dx == 0 && dz == 0 {
                continue;
            }
            let s = SlotCoord { x: slot.x + dx, z: slot.z + dz };
            let [sx, sz] = slot_center(s);
            let gy = surface_m(gen, sx, sz);
            if gy < feet_y - MAX_MARGIN_DROP_M - MAX_STEP_PER_SLOT_M {
                out.push((
                    "margin_drop",
                    format!("slot ({:+},{:+}) drops to {gy:.1} m", dx, dz),
                ));
                break; // one named refusal is enough for the ledger
            }
        }
    }
    out
}

/// The nav-path confirmation exists in [`path_exists_cached`] (patch
/// cache); the winner-revalidation test re-runs that same authority.

/// Nearest nav-CONFIRMED starter resource within `RESOURCE_RING_MAX`
/// slots. Cheap-first: collect euclidean hits across the rings (hash
/// probes only), then path-test nearest-first with a per-call patch
/// cache — an unconfirmed candidate costs at most `PATH_ATTEMPTS`
/// bounded A* runs, and a resourceless neighborhood costs no nav at all.
fn nearest_reachable_resource(
    gen: &WorldGen,
    from: [f32; 3],
    at: SlotCoord,
) -> Option<Resource> {
    const PATH_ATTEMPTS: usize = 8;
    let mut hits: Vec<(SlotCoord, PlantKind, f32)> = Vec::new();
    for r in 1..=RESOURCE_RING_MAX {
        for dz in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dz.abs() != r {
                    continue; // ring perimeter only
                }
                let s = SlotCoord { x: at.x + dx, z: at.z + dz };
                let Some(plant) = plant_at(gen, s) else { continue };
                if !is_starter_resource(plant.kind) {
                    continue;
                }
                let [sx, sz] = slot_center(s);
                let d = ((sx - from[0]).powi(2) + (sz - from[2]).powi(2)).sqrt();
                hits.push((s, plant.kind, d));
            }
        }
        // Nearest ring with any hit stops the euclidean sweep.
        if !hits.is_empty() {
            break;
        }
    }
    if hits.is_empty() {
        return None;
    }
    hits.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
    let mut cache: std::collections::HashMap<PatchCoord, NavPatch> = Default::default();
    for (s, kind, _) in hits.iter().take(PATH_ATTEMPTS) {
        let Some(path_cells) = path_exists_cached(gen, from, *s, &mut cache) else {
            continue;
        };
        return Some(Resource { kind: *kind, slot: *s, path_cells });
    }
    None
}

/// `path_exists` with a patch cache so one ring scan builds each
/// NavPatch once (a patch build is 256 final_solid column probes).
fn path_exists_cached(
    gen: &WorldGen,
    from: [f32; 3],
    to_slot: SlotCoord,
    cache: &mut std::collections::HashMap<PatchCoord, NavPatch>,
) -> Option<usize> {
    let [rx, rz] = slot_center(to_slot);
    let from_cell = crate::coords::CellCoord {
        x: from[0] as i32,
        y: from[1] as i32,
        z: from[2] as i32,
    };
    let to_cell = crate::coords::CellCoord { x: rx as i32, y: from[1] as i32, z: rz as i32 };
    let pa = PatchCoord {
        x: from_cell.x.div_euclid(PATCH_CELL_AXIS as i32),
        y: 0,
        z: from_cell.z.div_euclid(PATCH_CELL_AXIS as i32),
    };
    let pb = PatchCoord {
        x: to_cell.x.div_euclid(PATCH_CELL_AXIS as i32),
        y: 0,
        z: to_cell.z.div_euclid(PATCH_CELL_AXIS as i32),
    };
    let len = if pa == pb {
        cached_patch(gen, pa, cache).path(from_cell, to_cell)?.len()
    } else {
        let a = cached_patch(gen, pa, cache);
        let b = cached_patch(gen, pb, cache);
        let portals = a.portals_to(gen, pb);
        let portal = portals.first().copied()?;
        let head = a.path(from_cell, portal)?;
        let entry = if pb.x > pa.x {
            crate::coords::CellCoord { x: portal.x + 1, y: portal.y, z: portal.z }
        } else {
            crate::coords::CellCoord { x: portal.x, y: portal.y, z: portal.z + 1 }
        };
        let tail = b.path(entry, to_cell)?;
        head.len() + tail.len()
    };
    // A first errand is SHORT: a path longer than the cap is no proof of
    // a reachable next step (doc P4 — recipes name the next ingredient
    // reachable, not across the map).
    if len <= RESOURCE_PATH_MAX_CELLS {
        Some(len)
    } else {
        None
    }
}

fn cached_patch(
    gen: &WorldGen,
    coord: PatchCoord,
    cache: &mut std::collections::HashMap<PatchCoord, NavPatch>,
) -> NavPatch {
    // ponytail: clone-on-cache (NavPatch is a 256-Option<i32> vec — a
    // RefCell/Rc would be premature; upgrade if profiling disagrees).
    if let Some(p) = cache.get(&coord) {
        return p.clone();
    }
    let p = NavPatch::from_gen(gen, coord);
    cache.insert(coord, p.clone());
    p
}

/// The best yaw for one candidate: the first of the 8 compass
/// directions whose clearance gate comes back clean (deterministic
/// order; ties keep the earliest).
fn best_yaw(gen: &WorldGen, feet: [f32; 3], seed: u64) -> Option<f32> {
    for k in 0..8 {
        let yaw = k as f32 * 45.0;
        if spawn_clearance_violations(gen, feet, yaw, seed).is_empty() {
            return Some(yaw);
        }
    }
    None
}

/// The N02 search: slot rings around the origin, laws in cheap-first
/// order, N01 gate load-bearing, nav-confirmed resource, scored
/// survivors, every rejection recorded. Deterministic per seed.
pub fn decide_spawn(gen: &WorldGen, seed: u64) -> SpawnDecision {
    let origin = slot_at(0.0, 0.0);
    let mut evaluated = 0u32;
    let mut rejections: Vec<SpawnRejection> = Vec::new();
    let mut survivors: Vec<SpawnCandidate> = Vec::new();
    // The tier-1 budget: law probes across the whole sweep (the ring
    // sweep is perimeter-bounded; this caps the expensive tail). Ocean
    // slots cost only the biome hash and never consume budget. 8192
    // probes ≈ 0.6 s worst case in release — spawn happens once per
    // world creation, and the honest hand-off beats a bigger silent
    // radius (the fallback tier exists exactly for this shape of world).
    let mut budget = 8192u32;
    'rings: for r in 0..=SLOT_RING_MAX {
        for dz in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dz.abs() != r {
                    continue; // ring perimeter
                }
                if budget == 0 {
                    break 'rings;
                }
                let slot = SlotCoord { x: origin.x + dx, z: origin.z + dz };
                // Cheap prefilter: ocean biome never hosts land spawn.
                let wx = slot.x as i64 * SLOT_M * 1000;
                let wz = slot.z as i64 * SLOT_M * 1000;
                let region = RegionCoord {
                    x: wx.div_euclid(REGION_MM) as i32,
                    z: wz.div_euclid(REGION_MM) as i32,
                };
                let biome = gen.biome(region);
                if biome == Biome::Ocean {
                    continue; // not a rejection — the ocean is not a candidate
                }
                evaluated += 1;
                budget -= 1;
                let Some(feet) = slot_support(gen, slot) else {
                    rejections.push(SpawnRejection {
                        slot: [slot.x, slot.z],
                        law: "walkable_support".into(),
                        detail: "steep, underwater, or below sea".into(),
                    });
                    continue;
                };
                let margins = margin_violations(gen, slot, feet[1]);
                if !margins.is_empty() {
                    let (law, detail) = margins[0].clone();
                    rejections.push(SpawnRejection {
                        slot: [slot.x, slot.z],
                        law: law.into(),
                        detail,
                    });
                    continue;
                }
                let Some(yaw) = best_yaw(gen, feet, seed) else {
                    let v = spawn_clearance_violations(gen, feet, 0.0, seed);
                    let detail = v
                        .first()
                        .map(|x: &TrustViolation| x.message())
                        .unwrap_or_else(|| "clearance refused at all 8 yaws".into());
                    rejections.push(SpawnRejection {
                        slot: [slot.x, slot.z],
                        law: "trust_clearance".into(),
                        detail,
                    });
                    continue;
                };
                let Some(resource) = nearest_reachable_resource(gen, feet, slot) else {
                    rejections.push(SpawnRejection {
                        slot: [slot.x, slot.z],
                        law: "resource_reachable".into(),
                        detail: format!(
                            "no nav-confirmed starter resource within {RESOURCE_RING_MAX} slots"
                        ),
                    });
                    continue;
                };
                // Score survivors only: flatter first, then a shorter
                // first errand, then nearer the origin (stable tiebreak
                // is the slot coordinate itself via the ring order).
                let step = surface_m(gen, feet[0] + SLOT_M as f32, feet[2]) - feet[1];
                let dist = ((feet[0] * feet[0]) + (feet[2] * feet[2])).sqrt();
                let score = step.abs() * W_FLAT
                    + resource.path_cells as f32 * W_RESOURCE
                    + dist * W_ORIGIN;
                survivors.push(SpawnCandidate {
                    feet,
                    yaw_deg: yaw,
                    slot,
                    biome,
                    resource: Some(resource),
                    score,
                });
            }
        }
        // Early exit: a few survivors near the origin are plenty — the
        // player spawns NEAR the origin and depth beats breadth.
        if survivors.len() >= 4 {
            break;
        }
    }
    let best = survivors.iter().min_by(|a, b| {
        a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal)
    });
    if let Some(c) = best {
        return SpawnDecision {
            algo_version: SPAWN_ALGO_VERSION,
            seed,
            candidate: Some(c.clone()),
            evaluated,
            rejections,
            used_fallback: false,
        };
    }
    // Tier 2: the legacy region-ring expansion — still deterministic,
    // still recorded as a fallback decision.
    fallback_spawn(gen, seed, evaluated, rejections)
}

/// The legacy region-ring fallback (formerly `Player::spawn_safe`'s
/// body): progressive 2/4/8… patch rings of region-center cells. It
/// keeps the promise that every land seed spawns somewhere; the
/// decision honestly reports `used_fallback` so tests can demand tier-1
/// quality where it matters.
fn fallback_spawn(
    gen: &WorldGen,
    seed: u64,
    evaluated: u32,
    mut rejections: Vec<SpawnRejection>,
) -> SpawnDecision {
    let mut r = 2i32;
    loop {
        // ponytail: iterate REGIONS, not cells — the old cell loop
        // re-tested the same region's macro-field 256x per region
        // (16x16 cells share one RegionCoord), which made deep-ocean
        // fallbacks O(cells) instead of O(regions). Upgrade path: none
        // needed; region granularity is what the macro-field offers.
        for gx in -r..=r {
            for gz in -r..=r {
                let region = RegionCoord { x: gx, z: gz };
                if gen.macro_field(region).elevation_m < 1 {
                    continue;
                }
                let cx = region.x * 256 + 128;
                let cz = region.z * 256 + 128;
                if let Some(feet) = try_region_cell(gen, cx, cz) {
                    let win_slot = slot_at(feet[0], feet[2]);
                    rejections.push(SpawnRejection {
                        slot: [win_slot.x, win_slot.z],
                        law: "fallback_used".into(),
                        detail: "tier-1 slot search exhausted; legacy region cell won".into(),
                    });
                    return SpawnDecision {
                        algo_version: SPAWN_ALGO_VERSION,
                        seed,
                        candidate: Some(SpawnCandidate {
                            feet,
                            yaw_deg: 0.0,
                            slot: slot_at(feet[0], feet[2]),
                            biome: gen.biome(RegionCoord {
                                x: (cx as i64 * 1000).div_euclid(REGION_MM) as i32,
                                z: (cz as i64 * 1000).div_euclid(REGION_MM) as i32,
                            }),
                            resource: None,
                            score: f32::INFINITY,
                        }),
                        evaluated,
                        rejections,
                        used_fallback: true,
                    };
                }
            }
        }
        r *= 2;
        if r > 4096 {
            panic!("no safe spawn found within +-64 km — generator produced an ocean world");
        }
    }
}

/// The old cell probe, verbatim semantics: topmost solid in 0..=48
/// whose two cells above are passable, feet above sea level.
fn try_region_cell(gen: &WorldGen, cx: i32, cz: i32) -> Option<[f32; 3]> {
    for surface_y in (0..=48).rev() {
        let floor_solid = solid_at(gen, cx, surface_y, cz);
        if !floor_solid {
            continue;
        }
        let feet = surface_y + 1;
        if feet <= 0 {
            continue; // below sea level: not land spawn
        }
        if passable_at(gen, cx, feet, cz) && passable_at(gen, cx, feet + 1, cz) {
            return Some([cx as f32, feet as f32 + 0.01, cz as f32]);
        }
        break; // surface found but blocked above
    }
    None
}

/// Terrain solidity at a world CELL (the legacy probe's authority).
fn solid_at(gen: &WorldGen, cx: i32, cy: i32, cz: i32) -> bool {
    crate::terrain::final_solid(gen, cx as i64 * 1000, cy as i64 * 1000, cz as i64 * 1000).solid
}

/// Passable = air or water (swimmable) at a world CELL.
fn passable_at(gen: &WorldGen, cx: i32, cy: i32, cz: i32) -> bool {
    let a = crate::terrain::final_solid(gen, cx as i64 * 1000, cy as i64 * 1000, cz as i64 * 1000);
    matches!(a.material, crate::gen::CellMaterial::Air | crate::gen::CellMaterial::Water)
}

// ---------------------------------------------------------------------------
// The N02 matrix: the daily/PR/nightly seed sweep the doctrine demands.
// Pure and windowless — the CLI route wraps this with a JSON sidecar and
// the windowed captures live in the app.
// ---------------------------------------------------------------------------

/// One seed's matrix row.
#[derive(Clone, Debug, serde::Serialize)]
pub struct MatrixRow {
    pub seed: u64,
    pub used_fallback: bool,
    pub feet: Option<[f32; 3]>,
    pub yaw_deg: f32,
    pub biome: Option<&'static str>,
    pub has_resource: bool,
    pub gate_violations_at_own_yaw: usize,
    pub deterministic_repeat: bool,
    pub evaluated: u32,
}

/// One line of law: every row is gate-clean at its own yaw (or honestly a
/// fallback whose clearance still closes at SOME compass yaw), carries a
/// nav-confirmed resource on tier-1, and decides identically twice.
pub fn spawn_matrix_row(seed: u64) -> MatrixRow {
    let gen = WorldGen::new(seed);
    let d = decide_spawn(&gen, seed);
    let d2 = decide_spawn(&gen, seed);
    let deterministic = d
        .candidate
        .as_ref()
        .zip(d2.candidate.as_ref())
        .map(|(a, b)| {
            a.feet == b.feet
                && a.yaw_deg == b.yaw_deg
                && a.slot.x == b.slot.x
                && a.slot.z == b.slot.z
                && a.resource.as_ref().map(|r| (r.slot.x, r.slot.z, r.path_cells))
                    == b.resource.as_ref().map(|r| (r.slot.x, r.slot.z, r.path_cells))
        })
        .unwrap_or(true);
    let (feet, yaw, biome, has_resource) = d
        .candidate
        .as_ref()
        .map(|c| {
            (
                Some(c.feet),
                c.yaw_deg,
                Some(match c.biome {
                    Biome::Ocean => "Ocean",
                    Biome::Coast => "Coast",
                    Biome::Plains => "Plains",
                    Biome::Forest => "Forest",
                    Biome::Wetland => "Wetland",
                    Biome::Highlands => "Highlands",
                    Biome::Mountains => "Mountains",
                    Biome::SnowPeaks => "SnowPeaks",
                }),
                c.resource.is_some(),
            )
        })
        .unwrap_or((None, 0.0, None, false));
    let gate = d
        .candidate
        .as_ref()
        .map(|c| spawn_clearance_violations(&gen, c.feet, c.yaw_deg, seed).len())
        .unwrap_or(0);
    MatrixRow {
        seed,
        used_fallback: d.used_fallback,
        feet,
        yaw_deg: yaw,
        biome,
        has_resource,
        gate_violations_at_own_yaw: gate,
        deterministic_repeat: deterministic,
        evaluated: d.evaluated,
    }
}

/// The matrix law across a seed range: zero blocked spawns, zero
/// gate-clean failures on tier-1, all deterministic.
pub fn spawn_matrix_ok(seeds: std::ops::Range<u64>) -> Result<(), String> {
    let mut fallbacks = 0usize;
    let mut tier1 = 0usize;
    for seed in seeds {
        let row = spawn_matrix_row(seed);
        if !row.deterministic_repeat {
            return Err(format!("seed {}: decision is not deterministic", row.seed));
        }
        if row.used_fallback {
            fallbacks += 1;
        } else {
            tier1 += 1;
            if !row.has_resource {
                return Err(format!("seed {}: tier-1 without nav-confirmed resource", row.seed));
            }
            if row.gate_violations_at_own_yaw != 0 {
                return Err(format!(
                    "seed {}: tier-1 spawn violates the clearance gate at its own yaw ({} violations)",
                    row.seed, row.gate_violations_at_own_yaw
                ));
            }
            let feet = row.feet.expect("tier-1 has feet");
            if !feet.iter().all(|f| f.is_finite()) {
                return Err(format!("seed {}: non-finite feet", row.seed));
            }
        }
        if row.feet.is_none() {
            return Err(format!("seed {}: no candidate at all", row.seed));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests: the N02 laws
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    /// THE N02 contract: across seeds the spawn passes the N01 clearance
    /// gate at its OWN yaw, on every seed tested. Tier-2 fallbacks keep
    /// the OLD laws (yaw 0 + clearance, as the legacy path always did) —
    /// those seeds are named honestly by `used_fallback`.
    #[test]
    fn p3dn02_spawn_passes_trust_gate_at_own_yaw() {
        for seed in [3u64, 2024, 777, 4242] {
            let gen = WorldGen::new(seed);
            let d = decide_spawn(&gen, seed);
            let c = d
                .candidate
                .as_ref()
                .expect("every land seed must produce a candidate");
            let v = spawn_clearance_violations(&gen, c.feet, c.yaw_deg, seed);
            if d.used_fallback {
                // Fallback seeds: the legacy pick still refuses a blocked
                // trunk ahead — rotate the yaw like the control fixture
                // does instead of asserting the tier-1 contract.
                let mut ok = v.is_empty();
                if !ok {
                    ok = (0..8).any(|k| {
                        spawn_clearance_violations(&gen, c.feet, k as f32 * 45.0, seed)
                            .is_empty()
                    });
                }
                assert!(ok, "seed {seed}: even the fallback spawn has no clear yaw");
                continue;
            }
            assert!(
                v.is_empty(),
                "seed {seed}: spawn violates the N01 gate at its own yaw: {:?}",
                v.first().map(|x| x.message())
            );
            assert!(!c.feet.iter().any(|f| !f.is_finite()));
        }
    }

    /// THE resource law: the spawn's first errand is nav-CONFIRMED, not
    /// euclidean. Across a seed matrix, every tier-1 spawn carries a
    /// reachable resource — the RED probe measured 60/256 FAILING this
    /// before the fix.
    #[test]
    fn p3dn02_resource_is_nav_confirmed_across_seeds() {
        // Debug profile: 8 seeds (NavPatch builds dominate debug cost).
        let n = if cfg!(debug_assertions) { 8 } else { 24 };
        let mut tier1 = 0;
        let mut with_resource = 0;
        for seed in 0..n {
            let gen = WorldGen::new(seed);
            let d = decide_spawn(&gen, seed);
            let Some(c) = d.candidate.as_ref() else {
                continue;
            };
            if !d.used_fallback {
                tier1 += 1;
                assert!(
                    c.resource.is_some(),
                    "seed {seed}: tier-1 spawn without a nav-confirmed resource"
                );
                with_resource += 1;
            }
        }
        assert!(tier1 > 0, "the slot search should win on normal seeds");
        assert_eq!(tier1, with_resource);
    }

    /// The decision is deterministic: same seed, same decision.
    #[test]
    fn p3dn02_decision_is_deterministic() {
        for seed in [5u64, 615] {
            let gen = WorldGen::new(seed);
            let a = decide_spawn(&gen, seed);
            let b = decide_spawn(&gen, seed);
            let cand_eq = match (&a.candidate, &b.candidate) {
                (None, None) => true,
                (Some(x), Some(y)) => {
                    x.feet == y.feet
                        && x.yaw_deg == y.yaw_deg
                        && x.slot == y.slot
                        && x.biome == y.biome
                        && x.resource.as_ref().map(|r| r.slot) == y.resource.as_ref().map(|r| r.slot)
                }
                _ => false,
            };
            let res_eq = match (&a.candidate.as_ref().and_then(|c| c.resource.as_ref()),
                                &b.candidate.as_ref().and_then(|c| c.resource.as_ref())) {
                (None, None) => true,
                (Some(x), Some(y)) => x.slot == y.slot && x.kind == y.kind && x.path_cells == y.path_cells,
                _ => false,
            };
            assert!(
                cand_eq && res_eq && a.evaluated == b.evaluated && a.used_fallback == b.used_fallback
                    && a.rejections == b.rejections,
                "same seed must decide identically"
            );
        }
    }

    /// Score only orders valid candidates: a candidate in the decision
    /// must clear EVERY law on its own merits (support, margins, gate,
    /// resource) — re-testing the laws independently catches a scoring
    /// shortcut admitting an invalid point.
    #[test]
    fn p3dn02_winner_revalidates_every_law() {
        for seed in [3u64, 9] {
            let gen = WorldGen::new(seed);
            let d = decide_spawn(&gen, seed);
            let c = d.candidate.as_ref().expect("candidate");
            if d.used_fallback {
                continue; // the fallback tier keeps the OLD laws only
            }
            // support
            let again = slot_support(&gen, c.slot).expect("winner must re-pass support");
            assert!((again[1] - c.feet[1]).abs() < 0.5, "support height drifted");
            // margins
            assert!(
                margin_violations(&gen, c.slot, c.feet[1]).is_empty(),
                "winner must re-pass margins"
            );
            // gate at the recorded yaw
            assert!(spawn_clearance_violations(&gen, c.feet, c.yaw_deg, seed).is_empty());
            // resource by path
            let res = c.resource.as_ref().expect("tier-1 resource");
            let mut cache = std::collections::HashMap::new();
            assert!(
                path_exists_cached(&gen, c.feet, res.slot, &mut cache).is_some(),
                "resource path must re-confirm"
            );
        }
    }

    /// Yaw contract: the winner's yaw is one of the 8 compass directions.
    #[test]
    fn p3dn02_yaw_is_compass_quantized() {
        let gen = WorldGen::new(3);
        let d = decide_spawn(&gen, 3);
        let c = d.candidate.as_ref().expect("candidate");
        let k = (c.yaw_deg / 45.0).round() as i32;
        assert!((0..8).contains(&k), "yaw {} is not one of the 8 directions", c.yaw_deg);
        assert!((c.yaw_deg - k as f32 * 45.0).abs() < 1e-4);
    }

    /// The rejection ledger names laws, never silent timeouts: a seeded
    /// decision that used candidates records WHY losers lost, and a
    /// fallback decision names the fallback.
    #[test]
    fn p3dn02_rejections_are_named() {
        // Seed 2024 exercises the fallback tier; seed 9 exercises the
        // resource + clearance rejections (measured by the probe).
        for seed in [2024u64, 9] {
            let gen = WorldGen::new(seed);
            let d = decide_spawn(&gen, seed);
            assert!(
                !d.rejections.is_empty() || d.evaluated == 0 && d.used_fallback,
                "seed {seed}: a decision must carry evidence"
            );
            for r in &d.rejections {
                assert!(!r.law.is_empty(), "rejection must name its law");
                assert!(!r.detail.is_empty(), "rejection must carry a detail");
            }
        }
        // The normal-tier seed must have evaluated real candidates.
        let gen = WorldGen::new(9);
        let d = decide_spawn(&gen, 9);
        assert!(d.evaluated > 0, "the search must evaluate real candidates");
    }

    /// THE PR MATRIX (doc 13: PR 32 seeds): the whole-sweep law in one
    /// assert — deterministic, tier-1 gate-clean, resource-confirmed.
    /// The full 32-seed range is enforced in RELEASE by
    /// `make p3d-spawn-matrix` (and the p3d-beta stage); debug profile
    /// runs an 8-seed spot check of the same laws so ordinary
    /// `cargo test` stays minutes, not hours (NavPatch builds dominate
    /// debug cost — ~30x slower than release).
    #[test]
    fn p3dn02_matrix_pr_32_seeds_is_clean() {
        let n = if cfg!(debug_assertions) { 8 } else { 32 };
        if let Err(e) = spawn_matrix_ok(0..n) {
            panic!("PR {n}-seed matrix: {e}");
        }
    }

    /// THE NIGHTLY MATRIX (doc 13: nightly 256 seeds) — ignored by the
    /// default suite (release still costs ~4.5 min; debug would cost
    /// hours); run it explicitly with
    /// `cargo test -p pc3d_world --release p3dn02_matrix_nightly -- --ignored`
    /// or `make p3d-spawn-matrix-nightly`. Release-only range (256);
    /// `cargo test --ignored` in debug keeps the 96-seed spot check.
    #[test]
    #[ignore = "nightly: 256-seed spawn matrix, minutes in release — run via make p3d-spawn-matrix-nightly"]
    fn p3dn02_matrix_nightly_256_seeds_is_clean() {
        let n = if cfg!(debug_assertions) { 96 } else { 256 };
        if let Err(e) = spawn_matrix_ok(0..n) {
            panic!("nightly matrix ({n} seeds): {e}");
        }
    }

    /// The sidecar: choice + algo version persisted, rejections named.
    #[test]
    fn p3dn02_sidecar_carries_choice_and_version() {
        let gen = WorldGen::new(3);
        let d = decide_spawn(&gen, 3);
        let j = d.to_sidecar_json();
        assert_eq!(j["algo_version"], SPAWN_ALGO_VERSION, "sidecar version");
        assert_eq!(j["seed"], 3, "sidecar seed");
        assert!(j["candidate"].is_object(), "sidecar choice");
        assert!(j["candidate"]["feet_m"].is_array(), "sidecar feet");
        assert!(j["candidate"]["resource"].is_object(), "sidecar resource");
        assert!(j["rejections_tail"].is_array(), "sidecar ledger");
    }
}
