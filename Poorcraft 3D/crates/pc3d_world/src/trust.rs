//! N01 — the adversarial trust harness (CONTINUACAO-GLM 13, card N01).
//!
//! "Guilty scene" infrastructure: it proves the world gates turn RED for
//! the RIGHT reasons. Four regression classes must not cross the battery:
//!
//! | fixture              | deliberate mutation                    | gate that must fail        |
//! |----------------------|----------------------------------------|----------------------------|
//! | bad_viewmodel_flip   | mirrored X scale                       | viewmodel_winding          |
//! | bad_spawn_tree       | body AABB intersects the trunk         | spawn_capsule_clear        |
//! | bad_prop_float       | contact 0.50 m above the ground        | prop_ground_contact        |
//! | bad_water_air        | water surface with no sampled bed      | water_bed_support          |
//!
//! Every fixture carries seed, generator tag, mutation, expected error and
//! a control. A violation message names subject, measurement, value,
//! tolerance and seed. The mutations live ONLY here (tests + the explicit
//! `--trust-audit` audit command) — never in normal runtime. The verdicts
//! are semantic: computed purely from world data, never from pixels, so a
//! camera swap or a hidden mesh cannot convert a culprit into a valid
//! scene (`meta_checks` proves each gate is load-bearing: disable it and
//! its mutant survives).

use crate::coords::RegionCoord;
use crate::flora::{jitter, plant_at, PlantKind, SlotCoord};
use crate::gen::WorldGen;
use crate::hydro::RiverGraph;
use crate::player::{is_water_at, Player, BODY_HEIGHT, HALF_WIDTH};
use crate::scales::{MM_PER_METER, REGION_MM};

/// Bump when worldgen behavior changes the coordinates a fixture pins.
pub const GEN_ALGO_TAG: u32 = 1;
/// The audit report layout version.
pub const TRUST_SCHEMA_VERSION: u32 = 1;

/// Viewmodel contract constants (camera space: +X right, +Y up, -Z forward).
/// The renderer's near plane is 0.1 m; a held tool must respect it.
pub const VIEWMODEL_NEAR_M: f32 = 0.1;
/// Minimum distance the grip keeps in front of the near plane.
pub const GRIP_MIN_CLEAR_M: f32 = 0.05;
/// The held tool's half extent toward the camera (placeholder until N05
/// ships real tool volumes; the gate law does not change).
pub const TOOL_HALF_EXTENT_M: f32 = 0.15;
/// Screen-space envelope for the grip (NDC-ish fractions).
pub const GRIP_ENVELOPE_X: f32 = 0.6;
pub const GRIP_ENVELOPE_Y_LOW: f32 = -0.6;
pub const GRIP_ENVELOPE_Y_HIGH: f32 = 0.5;

/// Spawn clearance envelope. Trees are not solid cells in `final_solid`,
/// so the capsule law carries its own conservative trunk envelope; the
/// crown sits above BODY_HEIGHT and cannot hit a standing body.
pub const TRUNK_ENVELOPE_R: f32 = 0.45;
pub const TRUNK_ENVELOPE_H: f32 = 2.6;
/// How far a trunk may sit ahead of the eyes before the first frontal
/// view counts as blocked (plus the cone half-angle).
pub const FRONTAL_VIEW_DIST_M: f32 = 1.6;
pub const FRONTAL_VIEW_CONE_DEG: f32 = 35.0;
/// Rigid props: visible contact within 2 cm (N04 refines per-consumer).
pub const PROP_CONTACT_TOL_M: f32 = 0.02;
/// A water surface must sit on a sampled bed: at least this far below the
/// surface, and within a plausible depth window of it.
pub const BED_MIN_SUBMERGENCE_M: f32 = 0.15;
pub const BED_MAX_DEPTH_M: f32 = 3.0;
/// Water strips sample the bed at this many columns across their width.
pub const WATER_BED_SAMPLES: usize = 5;
/// The analytic surface offset the river mesher uses above the centerline.
pub const RIVER_SURFACE_OFFSET_M: f32 = 0.35;

fn m_to_mm(m: f32) -> i64 {
    (m * MM_PER_METER as f32) as i64
}

fn mm_to_m(mm: i64) -> f32 {
    mm as f32 / MM_PER_METER as f32
}

/// Which fixture gates are active. `Default` is all-on; only the audit
/// command and the meta-tests ever turn one off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateSwitches {
    pub viewmodel: bool,
    pub spawn: bool,
    pub prop: bool,
    pub water: bool,
}

impl Default for GateSwitches {
    fn default() -> Self {
        Self { viewmodel: true, spawn: true, prop: true, water: true }
    }
}

/// Whether a fixture is the deliberate mutant or its valid control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FixtureKind {
    Culprit,
    Control,
}

/// One named, measured refusal. The message names subject, measurement,
/// value, tolerance and seed — never a bare "failed".
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrustViolation {
    pub gate: String,
    pub subject: String,
    pub measurement: String,
    pub value: f64,
    pub tolerance: f64,
    pub seed: u64,
}

impl TrustViolation {
    fn new(
        gate: &str,
        subject: impl Into<String>,
        measurement: &str,
        value: f64,
        tolerance: f64,
        seed: u64,
    ) -> Self {
        Self {
            gate: gate.to_string(),
            subject: subject.into(),
            measurement: measurement.to_string(),
            value,
            tolerance,
            seed,
        }
    }

    /// The audit-line form: subject, measurement, tolerance and seed named.
    pub fn message(&self) -> String {
        format!(
            "[{}] {} : {} = {:.4} (tolerance {:.4}, seed {})",
            self.gate, self.subject, self.measurement, self.value, self.tolerance, self.seed
        )
    }
}

/// A fixture's semantic outcome. Camera and mesh visibility are
/// deliberately absent — a verdict is world data, not a screenshot.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrustVerdict {
    pub fixture_id: String,
    pub kind: FixtureKind,
    pub refused: bool,
    pub violations: Vec<TrustViolation>,
}

/// The camera-space pose a first-person tool must satisfy (N05 consumes
/// this same contract with a real mesh; until then the gate guards the
/// math so a bad pose cannot silently enter the battery).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ViewmodelPose {
    pub label: String,
    pub dominant_right: bool,
    /// Axis-aligned scale; a negative component mirrors the mesh.
    pub scale: [f32; 3],
    /// Grip point in camera space (meters).
    pub grip: [f32; 3],
    /// The direction the tool acts in, camera space (unit-ish).
    pub tool_forward: [f32; 3],
}

/// What a gate evaluates. Spawn/water payloads are world coordinates;
/// the others are self-contained measurements.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FixturePayload {
    Viewmodel(ViewmodelPose),
    Spawn { feet: [f32; 3], yaw_deg: f32 },
    Prop { id: String, contact_gap_m: f32 },
    WaterStrip { center_mm: [i64; 2], width_m: f32, surface_y_m: f32 },
}

/// One guilty scene or control, fully described: seed, generator tag,
/// mutation, expected refusal, ROI note and payload.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrustFixture {
    pub id: String,
    pub kind: FixtureKind,
    pub seed: u64,
    pub gen_algo: u32,
    pub mutation: String,
    pub expected_error: String,
    /// Informational only — the verdict never reads it (camera-swap law).
    pub camera_note: String,
    pub payload: FixturePayload,
}

/// The per-gate meta result: a gate is load-bearing when its culprit is
/// refused with the gate on and survives with the gate off.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MetaCheck {
    pub gate: String,
    pub load_bearing: bool,
    pub detail: String,
}

/// The whole audit: 8 verdicts + the meta row.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrustAuditReport {
    pub schema: u32,
    pub seed: u64,
    pub gen_algo: u32,
    pub verdicts: Vec<TrustVerdict>,
    pub meta: Vec<MetaCheck>,
}

impl TrustAuditReport {
    /// The N01 closure law: 4/4 culprits refused, 4/4 controls accepted,
    /// every gate load-bearing.
    pub fn all_clear(&self) -> bool {
        let mut culprits = 0;
        let mut controls = 0;
        for v in &self.verdicts {
            match v.kind {
                FixtureKind::Culprit => {
                    culprits += 1;
                    if !v.refused {
                        return false;
                    }
                }
                FixtureKind::Control => {
                    controls += 1;
                    if v.refused {
                        return false;
                    }
                }
            }
        }
        culprits == 4 && controls == 4 && self.meta.iter().all(|m| m.load_bearing)
    }
}

// ---------------------------------------------------------------- gates --

fn finite3(v: [f32; 3]) -> bool {
    v.iter().all(|c| c.is_finite())
}

/// The viewmodel gate: winding (no mirrored scale), grip tolerance in the
/// dominant hand, near-plane clearance, the action hemisphere, the screen
/// envelope. Non-finite input is its own failure and is never coerced.
pub fn viewmodel_pose_violations(pose: &ViewmodelPose, seed: u64) -> Vec<TrustViolation> {
    let subject = format!("viewmodel pose '{}'", pose.label);
    if !finite3(pose.scale) || !finite3(pose.grip) || !finite3(pose.tool_forward) {
        return vec![TrustViolation::new(
            "viewmodel_non_finite",
            subject,
            "finite_components",
            0.0,
            0.0,
            seed,
        )];
    }
    let mut out = Vec::new();
    let det = pose.scale[0] * pose.scale[1] * pose.scale[2];
    if det <= 0.0 {
        out.push(TrustViolation::new(
            "viewmodel_winding",
            subject.clone(),
            "scale_determinant",
            det as f64,
            0.0,
            seed,
        ));
    }
    let grip_z = pose.grip[2];
    if grip_z > -(VIEWMODEL_NEAR_M + GRIP_MIN_CLEAR_M) {
        out.push(TrustViolation::new(
            "primary_hand_grip_tolerance",
            subject.clone(),
            "grip_forward_distance_m",
            (-grip_z) as f64,
            (VIEWMODEL_NEAR_M + GRIP_MIN_CLEAR_M) as f64,
            seed,
        ));
    }
    if !pose.dominant_right {
        out.push(TrustViolation::new(
            "primary_hand_grip_tolerance",
            subject.clone(),
            "dominant_hand_is_right",
            0.0,
            1.0,
            seed,
        ));
    }
    let nearest_z = grip_z + TOOL_HALF_EXTENT_M;
    if nearest_z > -VIEWMODEL_NEAR_M {
        out.push(TrustViolation::new(
            "near_plane_clearance",
            subject.clone(),
            "nearest_point_to_camera_m",
            (-nearest_z) as f64,
            VIEWMODEL_NEAR_M as f64,
            seed,
        ));
    }
    // Action hemisphere: the tool must act AWAY from the camera (-Z).
    if pose.tool_forward[2] > 0.0 {
        out.push(TrustViolation::new(
            "correct_action_hemisphere",
            subject.clone(),
            "tool_forward_z",
            pose.tool_forward[2] as f64,
            0.0,
            seed,
        ));
    }
    let (gx, gy) = (pose.grip[0], pose.grip[1]);
    if gx.abs() > GRIP_ENVELOPE_X || gy < GRIP_ENVELOPE_Y_LOW || gy > GRIP_ENVELOPE_Y_HIGH {
        out.push(TrustViolation::new(
            "screen_coverage_envelope",
            subject,
            "grip_screen_offset",
            gx.abs().max(gy.abs()) as f64,
            GRIP_ENVELOPE_X as f64,
            seed,
        ));
    }
    out
}

fn tree_kind(kind: PlantKind) -> bool {
    matches!(kind, PlantKind::TreePine | PlantKind::TreeBroadleaf | PlantKind::TreeBirch)
}

/// One nearby tree's trunk (world meters) and its sampled ground height.
/// This mirrors the render flora's placement exactly: pivot at slot
/// center + jitter (dx, dz), grounded on `effective_surface_mm`.
fn trunk_of(gen: &WorldGen, slot: SlotCoord) -> Option<(PlantKind, [f32; 2], f32)> {
    let plant = plant_at(gen, slot)?;
    if !tree_kind(plant.kind) {
        return None;
    }
    let c = slot.center_m();
    let j = jitter(gen, slot);
    let tx = c[0] + j[0];
    let tz = c[1] + j[1];
    let ground = mm_to_m(gen.effective_surface_mm(m_to_mm(tx), m_to_mm(tz)));
    Some((plant.kind, [tx, tz], ground))
}

/// The yaw direction unit vector in world XZ (0° faces -Z, matching the
/// camera convention: yaw rotates around +Y).
fn angle_to_dir(yaw_deg: f32) -> [f32; 2] {
    let r = yaw_deg.to_radians();
    [-r.sin(), -r.cos()]
}

/// The spawn gate: walkable solid support, no water/fall start, the body
/// envelope clear of every trunk, and the first frontal view unblocked.
pub fn spawn_clearance_violations(
    gen: &WorldGen,
    feet: [f32; 3],
    yaw_deg: f32,
    seed: u64,
) -> Vec<TrustViolation> {
    let subject = format!("spawn feet ({:.2}, {:.2}, {:.2})", feet[0], feet[1], feet[2]);
    let mut out = Vec::new();
    let (fx_mm, fz_mm) = (m_to_mm(feet[0]), m_to_mm(feet[2]));

    // The floor law probes the CELL below the feet cell — the same cell
    // spawn_safe stood on — at its cell-index height (final_solid is
    // continuous in mm; an arbitrary offset inside the cell can sit above
    // the noise surface and read as air).
    let floor_cell_y = feet[1].floor() as i64 - 1;
    let floor_solid =
        crate::terrain::final_solid(gen, fx_mm, floor_cell_y * MM_PER_METER, fz_mm).solid;
    if !floor_solid {
        out.push(TrustViolation::new(
            "spawn_walkable_support",
            subject.clone(),
            "floor_solid_below_feet",
            0.0,
            1.0,
            seed,
        ));
    }
    if feet[1] <= 0.0 || is_water_at(gen, feet[0], feet[1], feet[2]) {
        out.push(TrustViolation::new(
            "spawn_safe_from_water_and_fall",
            subject.clone(),
            "feet_above_sea_and_dry_m",
            feet[1] as f64,
            0.0,
            seed,
        ));
    }

    // Trunk envelope scan: the 5x5 slot neighborhood around the body.
    let body_top = feet[1] + BODY_HEIGHT;
    let capsule_r = HALF_WIDTH + TRUNK_ENVELOPE_R;
    let center_slot = SlotCoord::from_world_mm(fx_mm, fz_mm);
    let yaw_dir = angle_to_dir(yaw_deg);
    for dx in -2i32..=2 {
        for dz in -2i32..=2 {
            let slot = SlotCoord { x: center_slot.x + dx, z: center_slot.z + dz };
            let Some((kind, trunk, ground)) = trunk_of(gen, slot) else { continue };
            let dist = ((trunk[0] - feet[0]).powi(2) + (trunk[1] - feet[2]).powi(2)).sqrt();
            let vertical_overlap = ground <= body_top && ground + TRUNK_ENVELOPE_H >= feet[1];
            if dist < capsule_r && vertical_overlap {
                out.push(TrustViolation::new(
                    "spawn_capsule_clear",
                    format!("{subject} vs {kind:?} trunk at ({:.2}, {:.2})", trunk[0], trunk[1]),
                    "capsule_trunk_distance_m",
                    dist as f64,
                    capsule_r as f64,
                    seed,
                ));
                continue; // frontal law keeps its own jurisdiction (see below)
            }
            // Frontal view: a trunk close ahead inside the view cone. A
            // trunk already inside the capsule is the capsule law's catch.
            if dist <= FRONTAL_VIEW_DIST_M {
                let to_trunk = (trunk[1] - feet[2]).atan2(trunk[0] - feet[0]);
                let bearing_yaw = yaw_dir[1].atan2(yaw_dir[0]);
                let mut diff = (to_trunk - bearing_yaw).to_degrees().abs();
                if diff > 180.0 {
                    diff = 360.0 - diff;
                }
                let ahead = yaw_dir[0] * (trunk[0] - feet[0]) + yaw_dir[1] * (trunk[1] - feet[2]);
                if diff <= FRONTAL_VIEW_CONE_DEG && ahead > 0.0 {
                    out.push(TrustViolation::new(
                        "spawn_forward_view_clear",
                        format!("{subject} vs {kind:?} trunk at ({:.2}, {:.2})", trunk[0], trunk[1]),
                        "trunk_ahead_distance_m",
                        ahead as f64,
                        FRONTAL_VIEW_DIST_M as f64,
                        seed,
                    ));
                }
            }
        }
    }
    out
}

/// The grounding law for rigid props: the contact gap to the sampled
/// ground must stay within the tolerance. Unknown ground is not a
/// violation here (callers pass the authority's answer).
pub fn prop_contact_violation(id: &str, contact_gap_m: f32, seed: u64) -> Option<TrustViolation> {
    if !contact_gap_m.is_finite() {
        return Some(TrustViolation::new(
            "prop_ground_contact_non_finite",
            format!("prop '{id}'"),
            "finite_contact_gap_m",
            0.0,
            0.0,
            seed,
        ));
    }
    if contact_gap_m.abs() > PROP_CONTACT_TOL_M {
        Some(TrustViolation::new(
            "prop_ground_contact",
            format!("prop '{id}'"),
            "contact_gap_m",
            contact_gap_m as f64,
            PROP_CONTACT_TOL_M as f64,
            seed,
        ))
    } else {
        None
    }
}

/// The water law: a surface strip exists only where sampled columns
/// confirm a bed below the surface within the depth window. The
/// centerline is mandatory; a majority of sampled columns must keep a
/// supported bed so banks may rise but the channel cannot vanish.
pub fn water_bed_violations(
    gen: &WorldGen,
    center_mm: [i64; 2],
    width_m: f32,
    surface_y_m: f32,
    seed: u64,
) -> Vec<TrustViolation> {
    let subject = format!(
        "water strip at ({:.1}, {:.1}) m wide {:.1}",
        mm_to_m(center_mm[0]),
        mm_to_m(center_mm[1]),
        width_m
    );
    let mut bed_below: Vec<f32> = Vec::with_capacity(WATER_BED_SAMPLES);
    for i in 0..WATER_BED_SAMPLES {
        let t = i as f32 / (WATER_BED_SAMPLES - 1) as f32 - 0.5;
        let x = center_mm[0] + (t * width_m * MM_PER_METER as f32) as i64;
        let bed = mm_to_m(gen.effective_surface_mm(x, center_mm[1]));
        bed_below.push(surface_y_m - bed);
    }
    let mut out = Vec::new();
    let center = bed_below[WATER_BED_SAMPLES / 2];
    if center < BED_MIN_SUBMERGENCE_M {
        out.push(TrustViolation::new(
            "water_bed_support",
            subject.clone(),
            "centerline_bed_below_surface_m",
            center as f64,
            BED_MIN_SUBMERGENCE_M as f64,
            seed,
        ));
    }
    let min_supported = WATER_BED_SAMPLES / 2 + 1;
    let supported = bed_below
        .iter()
        .filter(|b| **b >= BED_MIN_SUBMERGENCE_M && **b <= BED_MAX_DEPTH_M)
        .count();
    if supported < min_supported {
        out.push(TrustViolation::new(
            "water_bed_support",
            subject,
            "supported_bed_samples",
            supported as f64,
            min_supported as f64,
            seed,
        ));
    }
    out
}

// ------------------------------------------------------------- fixtures --

/// The sane right-handed pose every other viewmodel fixture mutates.
pub fn viewmodel_control_pose() -> ViewmodelPose {
    ViewmodelPose {
        label: "control_right_hand".to_string(),
        dominant_right: true,
        scale: [1.0, 1.0, 1.0],
        grip: [0.25, -0.25, -0.5],
        tool_forward: [0.0, 0.0, -1.0],
    }
}

pub fn fixture_viewmodel_control(seed: u64) -> TrustFixture {
    TrustFixture {
        id: "viewmodel_control".to_string(),
        kind: FixtureKind::Control,
        seed,
        gen_algo: GEN_ALGO_TAG,
        mutation: "none — the honest right-hand grip".to_string(),
        expected_error: String::new(),
        camera_note: "camera space; pose math, mesh arrives in N05".to_string(),
        payload: FixturePayload::Viewmodel(viewmodel_control_pose()),
    }
}

pub fn fixture_bad_viewmodel_flip(seed: u64) -> TrustFixture {
    let mut pose = viewmodel_control_pose();
    pose.label = "bad_viewmodel_flip".to_string();
    pose.scale = [-1.0, 1.0, 1.0]; // mirrored mesh: inverted winding
    TrustFixture {
        id: "bad_viewmodel_flip".to_string(),
        kind: FixtureKind::Culprit,
        seed,
        gen_algo: GEN_ALGO_TAG,
        mutation: "scale.x = -1 (mirrored tool)".to_string(),
        expected_error: "viewmodel_winding".to_string(),
        camera_note: "camera space; pose math, mesh arrives in N05".to_string(),
        payload: FixturePayload::Viewmodel(pose),
    }
}

/// The spawn the world's own authority picks — the standing control.
pub fn fixture_spawn_control(gen: &WorldGen, seed: u64) -> TrustFixture {
    let p = Player::spawn_safe(gen);
    // Aim the control at the first of 8 directions with a clear view.
    let mut yaw = 0.0f32;
    for k in 0..8 {
        let candidate = k as f32 * 45.0;
        if spawn_clearance_violations(gen, p.pos, candidate, seed).is_empty() {
            yaw = candidate;
            break;
        }
    }
    TrustFixture {
        id: "spawn_control".to_string(),
        kind: FixtureKind::Control,
        seed,
        gen_algo: GEN_ALGO_TAG,
        mutation: "none — spawn_safe's own answer, aimed at a clear view".to_string(),
        expected_error: String::new(),
        camera_note: "eye at the body's eye height, looking along yaw".to_string(),
        payload: FixturePayload::Spawn { feet: p.pos, yaw_deg: yaw },
    }
}

fn first_tree_slot_near(
    gen: &WorldGen,
    at: SlotCoord,
) -> Option<(SlotCoord, PlantKind, [f32; 2], f32)> {
    // Near rings first (the guilty spawn candidate sits by its own
    // spawn_safe answer), then wider rings — a plains spawn can be
    // treeless for a hundred meters and the fixture must still build.
    for r in 0..=48i32 {
        for dx in -r..=r {
            for dz in -r..=r {
                if dx.abs() != r && dz.abs() != r {
                    continue; // ring sweep only
                }
                let slot = SlotCoord { x: at.x + dx, z: at.z + dz };
                if let Some((kind, trunk, ground)) = trunk_of(gen, slot) {
                    return Some((slot, kind, trunk, ground));
                }
            }
        }
    }
    None
}

/// The guilty spawn: feet inside the nearest tree's trunk envelope.
pub fn fixture_bad_spawn_tree(gen: &WorldGen, seed: u64) -> Option<TrustFixture> {
    let p = Player::spawn_safe(gen);
    let at = SlotCoord::from_world_mm(m_to_mm(p.pos[0]), m_to_mm(p.pos[2]));
    let (_slot, kind, trunk, ground) = first_tree_slot_near(gen, at)?;
    Some(TrustFixture {
        id: "bad_spawn_tree".to_string(),
        kind: FixtureKind::Culprit,
        seed,
        gen_algo: GEN_ALGO_TAG,
        mutation: "feet moved into the trunk envelope".to_string(),
        expected_error: "spawn_capsule_clear".to_string(),
        camera_note: "eye at the body's eye height; culprit and control share the world".to_string(),
        payload: FixturePayload::Spawn {
            feet: [trunk[0], ground + 0.01, trunk[1]],
            yaw_deg: 0.0,
        },
    })
}

pub fn fixture_prop_control(seed: u64) -> TrustFixture {
    TrustFixture {
        id: "prop_control".to_string(),
        kind: FixtureKind::Control,
        seed,
        gen_algo: GEN_ALGO_TAG,
        mutation: "none — seated by the placement authority".to_string(),
        expected_error: String::new(),
        camera_note: "close-up of the contact point".to_string(),
        payload: FixturePayload::Prop { id: "control_rock".to_string(), contact_gap_m: 0.0 },
    }
}

pub fn fixture_bad_prop_float(seed: u64) -> TrustFixture {
    TrustFixture {
        id: "bad_prop_float".to_string(),
        kind: FixtureKind::Culprit,
        seed,
        gen_algo: GEN_ALGO_TAG,
        mutation: "contact lifted 0.50 m above the ground".to_string(),
        expected_error: "prop_ground_contact".to_string(),
        camera_note: "refused before meshing; the sidecar carries the gap".to_string(),
        payload: FixturePayload::Prop { id: "floating_rock".to_string(), contact_gap_m: 0.5 },
    }
}

/// First river region around the origin (±20 regions), its center in mm.
pub fn first_river_center_mm(gen: &WorldGen) -> Option<[i64; 2]> {
    let graph = RiverGraph::new(gen, 20);
    for x in -20..=20 {
        for z in -20..=20 {
            let r = RegionCoord { x, z };
            if graph.is_river(r) {
                let o = r.origin();
                return Some([o.x + REGION_MM / 2, o.z + REGION_MM / 2]);
            }
        }
    }
    None
}

pub fn fixture_water_control(gen: &WorldGen, seed: u64) -> Option<TrustFixture> {
    let center = first_river_center_mm(gen)?;
    let bed = mm_to_m(gen.effective_surface_mm(center[0], center[1]));
    Some(TrustFixture {
        id: "water_control".to_string(),
        kind: FixtureKind::Control,
        seed,
        gen_algo: GEN_ALGO_TAG,
        mutation: "none — a real river section over its own channel".to_string(),
        expected_error: String::new(),
        camera_note: "the same camera the culprit site would use".to_string(),
        payload: FixturePayload::WaterStrip {
            center_mm: center,
            width_m: 4.0,
            surface_y_m: bed + RIVER_SURFACE_OFFSET_M,
        },
    })
}

/// The guilty water: a surface pushed 2 m UNDER a nearby hilltop, so the
/// sampled bed sits ABOVE the "surface" — water in the air.
pub fn fixture_bad_water_air(gen: &WorldGen, seed: u64) -> Option<TrustFixture> {
    let center = first_river_center_mm(gen)?;
    // Walk 96 m east of the river and take the terrain there as the hill.
    let hill = [center[0] + 96 * MM_PER_METER, center[1]];
    let hill_bed = mm_to_m(gen.effective_surface_mm(hill[0], hill[1]));
    Some(TrustFixture {
        id: "bad_water_air".to_string(),
        kind: FixtureKind::Culprit,
        seed,
        gen_algo: GEN_ALGO_TAG,
        mutation: "surface placed 2 m below a hilltop bed (no sampled bed below)".to_string(),
        expected_error: "water_bed_support".to_string(),
        camera_note: "refused before meshing; the sidecar carries the samples".to_string(),
        payload: FixturePayload::WaterStrip {
            center_mm: hill,
            width_m: 4.0,
            surface_y_m: hill_bed - 2.0,
        },
    })
}

/// The full pack: 4 culprits + 4 controls, or None where a fixture
/// cannot be truthfully built for this seed (e.g. no river region).
pub fn build_fixtures(gen: &WorldGen, seed: u64) -> Option<Vec<TrustFixture>> {
    Some(vec![
        fixture_bad_viewmodel_flip(seed),
        fixture_viewmodel_control(seed),
        fixture_bad_spawn_tree(gen, seed)?,
        fixture_spawn_control(gen, seed),
        fixture_bad_prop_float(seed),
        fixture_prop_control(seed),
        fixture_bad_water_air(gen, seed)?,
        fixture_water_control(gen, seed)?,
    ])
}

// ---------------------------------------------------------------- audit --

fn evaluate(fixture: &TrustFixture, gen: &WorldGen, switches: &GateSwitches) -> TrustVerdict {
    let seed = fixture.seed;
    let violations = match &fixture.payload {
        FixturePayload::Viewmodel(pose) => {
            if switches.viewmodel {
                viewmodel_pose_violations(pose, seed)
            } else {
                Vec::new()
            }
        }
        FixturePayload::Spawn { feet, yaw_deg } => {
            if switches.spawn {
                spawn_clearance_violations(gen, *feet, *yaw_deg, seed)
            } else {
                Vec::new()
            }
        }
        FixturePayload::Prop { id, contact_gap_m } => {
            if switches.prop {
                prop_contact_violation(id, *contact_gap_m, seed).into_iter().collect()
            } else {
                Vec::new()
            }
        }
        FixturePayload::WaterStrip { center_mm, width_m, surface_y_m } => {
            if switches.water {
                water_bed_violations(gen, *center_mm, *width_m, *surface_y_m, seed)
            } else {
                Vec::new()
            }
        }
    };
    TrustVerdict {
        fixture_id: fixture.id.clone(),
        kind: fixture.kind,
        refused: !violations.is_empty(),
        violations,
    }
}

fn audit_verdicts(seed: u64, switches: GateSwitches) -> Vec<TrustVerdict> {
    let gen = WorldGen::new(seed);
    let Some(fixtures) = build_fixtures(&gen, seed) else {
        return Vec::new();
    };
    fixtures.iter().map(|f| evaluate(f, &gen, &switches)).collect()
}

/// The meta row: for each gate, its culprit must flip when the gate is
/// disabled — that is what makes the gate load-bearing.
fn culprit_of<'a>(verdicts: &'a [TrustVerdict], gate: &str) -> Option<&'a TrustVerdict> {
    verdicts
        .iter()
        .find(|v| v.kind == FixtureKind::Culprit && v.fixture_id.contains(gate))
}

pub fn meta_checks(seed: u64) -> Vec<MetaCheck> {
    let all = GateSwitches::default();
    let on = audit_verdicts(seed, all);
    let mut out = Vec::new();
    for (gate, off) in [
        ("viewmodel", GateSwitches { viewmodel: false, ..all }),
        ("spawn", GateSwitches { spawn: false, ..all }),
        ("prop", GateSwitches { prop: false, ..all }),
        ("water", GateSwitches { water: false, ..all }),
    ] {
        let disabled = audit_verdicts(seed, off);
        let refused_on = culprit_of(&on, gate).map(|v| v.refused).unwrap_or(false);
        let survived_off = culprit_of(&disabled, gate).map(|v| !v.refused).unwrap_or(false);
        out.push(MetaCheck {
            gate: gate.to_string(),
            load_bearing: refused_on && survived_off,
            detail: format!(
                "culprit refused with gate on: {refused_on}; culprit survives with gate off: {survived_off}"
            ),
        });
    }
    out
}

/// The whole N01 audit for one seed. Deterministic: pure functions of the
/// seed and the world generator.
pub fn run_trust_audit(seed: u64) -> TrustAuditReport {
    TrustAuditReport {
        schema: TRUST_SCHEMA_VERSION,
        seed,
        gen_algo: GEN_ALGO_TAG,
        verdicts: audit_verdicts(seed, GateSwitches::default()),
        meta: meta_checks(seed),
    }
}

// ---------------------------------------------------------------- tests --

#[cfg(test)]
mod tests {
    use super::*;

    fn verdict<'a>(report: &'a TrustAuditReport, id: &str) -> &'a TrustVerdict {
        report
            .verdicts
            .iter()
            .find(|v| v.fixture_id == id)
            .unwrap_or_else(|| panic!("verdict for {id} missing"))
    }

    #[test]
    fn p3dn01_viewmodel_control_is_accepted() {
        let v = viewmodel_pose_violations(&viewmodel_control_pose(), 4242);
        assert!(v.is_empty(), "control pose must be clean: {v:?}");
    }

    #[test]
    fn p3dn01_viewmodel_flip_is_refused_by_winding() {
        let mut flip = viewmodel_control_pose();
        flip.label = "flip".to_string();
        flip.scale = [-1.0, 1.0, 1.0];
        let v = viewmodel_pose_violations(&flip, 4242);
        assert!(v.iter().any(|x| x.gate == "viewmodel_winding"), "{v:?}");
        // and ONLY winding: the mutation is minimal
        assert_eq!(v.len(), 1, "minimal mutation must trip one law: {v:?}");
        assert!(v[0].message().contains("4242"), "message must name the seed");
    }

    #[test]
    fn p3dn01_spawn_control_is_accepted_across_seeds() {
        for seed in [4242u64, 777, 2024] {
            let gen = WorldGen::new(seed);
            let f = fixture_spawn_control(&gen, seed);
            let FixturePayload::Spawn { feet, yaw_deg } = f.payload else { panic!("spawn payload") };
            let v = spawn_clearance_violations(&gen, feet, yaw_deg, seed);
            assert!(v.is_empty(), "seed {seed}: spawn_safe's own answer must be clean: {v:?}");
        }
    }

    #[test]
    fn p3dn01_spawn_in_tree_is_refused() {
        for seed in [4242u64, 777, 2024] {
            let gen = WorldGen::new(seed);
            let Some(f) = fixture_bad_spawn_tree(&gen, seed) else {
                panic!("seed {seed}: no tree near spawn for the guilty fixture");
            };
            let FixturePayload::Spawn { feet, yaw_deg } = f.payload else { panic!("spawn payload") };
            let v = spawn_clearance_violations(&gen, feet, yaw_deg, seed);
            assert!(
                v.iter().any(|x| x.gate == "spawn_capsule_clear"),
                "seed {seed}: trunk intersection must name spawn_capsule_clear: {v:?}"
            );
        }
    }

    #[test]
    fn p3dn01_frontal_law_has_its_own_jurisdiction() {
        // A trunk 1.3 m dead ahead: outside the capsule (0.75 m), inside
        // the frontal window (1.6 m, 35°). ONLY the frontal law fires.
        let seed = 4242u64;
        let gen = WorldGen::new(seed);
        let p = Player::spawn_safe(&gen);
        let at = SlotCoord::from_world_mm(m_to_mm(p.pos[0]), m_to_mm(p.pos[2]));
        let Some((_slot, _kind, trunk, ground)) = first_tree_slot_near(&gen, at) else {
            panic!("no tree near spawn to build the frontal mutant");
        };
        let dir = angle_to_dir(0.0);
        // Stand 1.3 m back from the trunk along the yaw-forward axis, so
        // the trunk is dead ahead but beyond capsule reach.
        let feet = [trunk[0] - dir[0] * 1.3, ground + 0.01, trunk[1] - dir[1] * 1.3];
        let v = spawn_clearance_violations(&gen, feet, 0.0, seed);
        assert!(
            v.iter().any(|x| x.gate == "spawn_forward_view_clear"),
            "frontal block must be named: {v:?}"
        );
        assert!(
            !v.iter().any(|x| x.gate == "spawn_capsule_clear"),
            "1.3 m is outside the capsule; jurisdiction must not overlap: {v:?}"
        );
    }

    #[test]
    fn p3dn01_floating_prop_is_refused_and_control_accepted() {
        let bad = prop_contact_violation("floating_rock", 0.5, 4242).unwrap();
        assert_eq!(bad.gate, "prop_ground_contact");
        assert_eq!(bad.value, 0.5);
        assert_eq!(bad.tolerance, PROP_CONTACT_TOL_M as f64);
        assert!(prop_contact_violation("control_rock", 0.0, 4242).is_none());
        assert!(prop_contact_violation("tilted", 0.015, 4242).is_none(), "within tolerance");
    }

    #[test]
    fn p3dn01_water_without_bed_is_refused_and_river_accepted() {
        for seed in [4242u64, 3] {
            let gen = WorldGen::new(seed);
            let Some(ctrl) = fixture_water_control(&gen, seed) else {
                panic!("seed {seed}: no river region found for the control");
            };
            let FixturePayload::WaterStrip { center_mm, width_m, surface_y_m } = ctrl.payload
            else {
                panic!("water payload")
            };
            let v = water_bed_violations(&gen, center_mm, width_m, surface_y_m, seed);
            assert!(v.is_empty(), "seed {seed}: a real river section must be clean: {v:?}");

            let Some(bad) = fixture_bad_water_air(&gen, seed) else {
                panic!("seed {seed}: no hill site for the guilty fixture");
            };
            let FixturePayload::WaterStrip { center_mm, width_m, surface_y_m } = bad.payload
            else {
                panic!("water payload")
            };
            let v = water_bed_violations(&gen, center_mm, width_m, surface_y_m, seed);
            assert!(
                v.iter().any(|x| x.gate == "water_bed_support"),
                "seed {seed}: suspended water must be refused: {v:?}"
            );
        }
    }

    #[test]
    fn p3dn01_every_gate_is_load_bearing() {
        let meta = meta_checks(4242);
        assert_eq!(meta.len(), 4);
        for m in &meta {
            assert!(m.load_bearing, "gate '{}' is not load-bearing: {}", m.gate, m.detail);
        }
    }

    #[test]
    fn p3dn01_disabling_the_wrong_gate_still_refuses() {
        // Orthogonality: with the prop gate off, the tree mutant is STILL
        // refused — gates must not shadow each other.
        let gen = WorldGen::new(4242);
        let switches = GateSwitches { prop: false, ..GateSwitches::default() };
        let f = fixture_bad_spawn_tree(&gen, 4242).expect("tree fixture");
        let v = evaluate(&f, &gen, &switches);
        assert!(v.refused, "prop switch must not un-refuse the tree culprit");
    }

    #[test]
    fn p3dn01_camera_swap_cannot_validate_a_culprit() {
        // The verdict must be identical no matter what the camera note
        // claims — it is semantic, not photographic.
        let seed = 4242u64;
        let gen = WorldGen::new(seed);
        let mut f1 = fixture_bad_spawn_tree(&gen, seed).expect("tree fixture");
        f1.camera_note = "close-up from the south".to_string();
        let mut f2 = f1.clone();
        f2.camera_note = "wide shot from a drone".to_string();
        let v1 = evaluate(&f1, &gen, &GateSwitches::default());
        let v2 = evaluate(&f2, &gen, &GateSwitches::default());
        assert_eq!(v1, v2, "camera notes must not influence the verdict");
        assert!(v1.refused);
    }

    #[test]
    fn p3dn01_non_finite_pose_is_its_own_failure() {
        let mut pose = viewmodel_control_pose();
        pose.label = "nan_grip".to_string();
        pose.grip = [f32::NAN, 0.0, -1.0];
        let v = viewmodel_pose_violations(&pose, 4242);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].gate, "viewmodel_non_finite");
        assert!(v[0].message().contains("nan_grip"));
    }

    #[test]
    fn p3dn01_full_audit_closes_4x4() {
        let report = run_trust_audit(4242);
        assert_eq!(report.verdicts.len(), 8, "4 culprits + 4 controls");
        for id in ["bad_viewmodel_flip", "bad_spawn_tree", "bad_prop_float", "bad_water_air"] {
            assert!(verdict(&report, id).refused, "{id} must be refused");
        }
        for id in ["viewmodel_control", "spawn_control", "prop_control", "water_control"] {
            assert!(!verdict(&report, id).refused, "{id} must be accepted");
        }
        assert!(report.all_clear(), "the audit must close");
    }

    #[test]
    fn p3dn01_fixtures_carry_their_contract() {
        let gen = WorldGen::new(4242);
        let fixtures = build_fixtures(&gen, 4242).expect("fixtures for seed 4242");
        assert_eq!(fixtures.len(), 8);
        for f in &fixtures {
            assert_eq!(f.gen_algo, GEN_ALGO_TAG);
            assert_eq!(f.seed, 4242);
            if f.kind == FixtureKind::Culprit {
                assert!(!f.expected_error.is_empty(), "{} must name its error", f.id);
                assert!(!f.mutation.is_empty(), "{} must name its mutation", f.id);
            }
        }
    }

    #[test]
    fn p3dn01_report_serializes_for_the_sidecar() {
        let report = run_trust_audit(4242);
        let json = serde_json::to_string(&report).expect("serialize");
        let back: TrustAuditReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, report);
    }
}
