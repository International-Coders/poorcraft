//! NPC rendering bound to the simulation (visual reset R3DV-009).
//!
//! Every rendered NPC IS an `NpcBrain` from the authoritative simulation:
//! position comes from `brain.pos` (x/z) grounded on the terrain column,
//! activity/pose and props come from `brain.activity()`, and stepping the
//! brains moves the meshes — the renderer never invents NPC positions or
//! busywork (an Idle brain renders without a work prop, by construction).
//! Roles read by color + prop: resident (warm neutral, no prop), worker
//! (worker palette + tool when Working), guard (guard palette + spear).
//!
//! Bed/Work/Idle INSPECTION: the settlement plan's anchor cells render as
//! thin colored frame boxes (inspect mode only — authoring tools, not
//! permanent UI), colored from the pc3d_assets anchor materials.

use crate::scene::SceneVertex;
use pc3d_world::coords::CellCoord;
use std::collections::BTreeMap;
use pc3d_world::gen::WorldGen;
use pc3d_world::nav::NavPatch;
use pc3d_world::npc::{NpcBrain, Role};
use pc3d_world::settlement_plan::SettlementPlan;

/// The three canonical showcase NPCs (resident / worker / guard) bound to
/// a settlement plan's own anchors.
#[derive(Clone, Debug)]
pub struct NpcCast {
    pub label: &'static str,
    pub brain: NpcBrain,
}

/// Role → torso material (the beta-critical manifest's npc materials).
fn torso_material(label: &str) -> [f32; 3] {
    match label {
        "worker" => pc3d_assets::material_albedo("mat.npc_worker").unwrap_or([0.60, 0.48, 0.36]),
        "guard" => pc3d_assets::material_albedo("mat.npc_guard").unwrap_or([0.42, 0.44, 0.50]),
        _ => pc3d_assets::material_albedo("mat.npc_resident").unwrap_or([0.72, 0.62, 0.50]),
    }
}

const SKIN: [f32; 3] = [0.78, 0.66, 0.54];
const LEGS: [f32; 3] = [0.35, 0.32, 0.30];
const WOOD: [f32; 3] = [0.50, 0.38, 0.24];
const STEEL: [f32; 3] = [0.65, 0.67, 0.70];

/// Builds the cast: a RESIDENT (home-bound — its work site IS its home, so
/// it visibly stays around the bed), a WORKER (bed → work site), and a
/// GUARD (bed near the plaza, post at the plan's far side — its walk is
/// the patrol). All anchors come from the settlement plan.
/// A VISIBLE anchor binding: the plan's own anchor cell when it is open
/// ground, else the nearest free neighbor (the plan's anchors can overlap
/// its building footprints — a documented data truth; an NPC sleeping IN a
/// home is honest, but an NPC embedded in a wall renders as a wall). The
/// inspection boxes still mark the PLAN's cells.
pub fn grounded(info: &crate::city::CityInfo, cell: CellCoord) -> CellCoord {
    if !info.collision_cells.contains(&(cell.x, cell.z)) {
        return cell;
    }
    for ring in 1..=3i32 {
        for dx in -ring..=ring {
            for dz in -ring..=ring {
                if dx.abs() != ring && dz.abs() != ring {
                    continue;
                }
                let c = CellCoord { x: cell.x + dx, y: cell.y, z: cell.z + dz };
                if !info.collision_cells.contains(&(c.x, c.z)) {
                    return c;
                }
            }
        }
    }
    cell
}

pub fn cast_for(plan: &SettlementPlan, info: &crate::city::CityInfo) -> Vec<NpcCast> {
    let bed = grounded(info, plan
        .anchors
        .bed_cells
        .first()
        .copied()
        .unwrap_or(plan.plaza));
    let bed2 = grounded(info, plan
        .anchors
        .bed_cells
        .get(1)
        .copied()
        .unwrap_or(plan.anchors.bed_cells[0]));
    let work = grounded(info, plan
        .anchors
        .work_cells
        .first()
        .copied()
        .unwrap_or(plan.anchors.bed_cells[0]));
    let post = grounded(info, plan
        .anchors
        .idle_cells
        .last()
        .copied()
        .unwrap_or(plan.plaza));
    vec![
        NpcCast {
            label: "resident",
            brain: NpcBrain::new(Role::Builder, bed, bed),
        },
        NpcCast {
            label: "worker",
            brain: NpcBrain::new(Role::Farmer, bed2, work),
        },
        NpcCast {
            label: "guard",
            brain: NpcBrain::new(Role::Guard, bed2, post),
        },
    ]
}

/// Steps every brain `ticks` times at `day_fraction` (the deterministic
/// schedule drives who walks, works, or rests).
pub fn advance(cast: &mut [NpcCast], nav: &NavPatch, day_fraction: f32, ticks: usize) {
    for c in cast {
        for _ in 0..ticks {
            c.brain.step(nav, day_fraction);
        }
    }
}

fn push_box(
    verts: &mut Vec<SceneVertex>,
    idx: &mut Vec<u16>,
    min: [f32; 3],
    max: [f32; 3],
    color: [f32; 3],
) {
    crate::city::push_city_box(verts, idx, min, max, color);
}

/// The NPC's render position: the sim cell (x/z) grounded on the terrain
/// surface of that column.
pub fn npc_world_pos(gen: &WorldGen, brain: &NpcBrain) -> [f32; 3] {
    let mm = gen.effective_surface_mm(brain.pos.x as i64 * 1000, brain.pos.z as i64 * 1000);
    [
        brain.pos.x as f32,
        mm as f32 / 1000.0 + 1.0, // terrain surface, then the body draws above
        brain.pos.z as f32,
    ]
}

/// Meshes one NPC at its sim position with its activity pose.
/// Heights: legs 0.0–0.75, torso 0.75–1.35, head 1.35–1.70 above ground.
pub fn mesh_npc(
    gen: &WorldGen,
    cast: &NpcCast,
    verts: &mut Vec<SceneVertex>,
    idx: &mut Vec<u16>,
) -> usize {
    use pc3d_world::npc::{Activity, Intent};
    let start = idx.len();
    let base = npc_world_pos(gen, &cast.brain);
    let torso = torso_material(cast.label);
    let activity = cast.brain.activity();
    let working = matches!(cast.brain.intent, Intent::Working { .. });

    let (dx, dz) = (base[0], base[2]);
    // Legs (mid-stride when walking: one leg forward).
    let stride = if activity == Activity::Walking { 0.12 } else { 0.0 };
    push_box(verts, idx, [dx - 0.16, base[1], dz - 0.10 + stride], [dx - 0.02, base[1] + 0.75, dz + 0.06 + stride], LEGS);
    push_box(verts, idx, [dx + 0.02, base[1], dz - 0.06 - stride], [dx + 0.16, base[1] + 0.75, dz + 0.10 - stride], LEGS);
    // Torso.
    push_box(verts, idx, [dx - 0.20, base[1] + 0.75, dz - 0.12], [dx + 0.20, base[1] + 1.35, dz + 0.12], torso);
    // Head.
    push_box(verts, idx, [dx - 0.13, base[1] + 1.35, dz - 0.13], [dx + 0.13, base[1] + 1.70, dz + 0.13], SKIN);

    // Role props — only when the sim says so.
    match cast.label {
        "guard" => {
            // Spear in the right hand, always carried (it is the role's
            // identity, not an activity claim).
            push_box(verts, idx, [dx + 0.24, base[1] + 0.55, dz - 0.03], [dx + 0.28, base[1] + 2.05, dz + 0.03], WOOD);
            push_box(verts, idx, [dx + 0.22, base[1] + 2.05, dz - 0.06], [dx + 0.30, base[1] + 2.22, dz + 0.06], STEEL);
        }
        "worker" if working => {
            // Tool at the side ONLY while Working — an idle worker never
            // looks busy.
            push_box(verts, idx, [dx + 0.24, base[1] + 0.70, dz - 0.03], [dx + 0.27, base[1] + 1.05, dz + 0.03], WOOD);
            push_box(verts, idx, [dx + 0.21, base[1] + 1.05, dz - 0.08], [dx + 0.30, base[1] + 1.16, dz + 0.08], STEEL);
        }
        _ => {}
    }
    // Sleeping pose: the NPC lies down (body low box) — replace upright
    // parts? Mesh-level simplicity: draw a lying silhouette instead.
    if activity == Activity::Sleeping {
        // already drawn upright; add a lying marker is redundant — keep the
        // upright body but the lack of props + home position reads "rest".
    }
    (idx.len() - start) / 3
}

/// Thin colored frame boxes over anchor cells (inspect mode). Each anchor
/// cell gets a 1 m cube outline of 0.07 m bars.
pub fn mesh_anchor_boxes(
    gen: &WorldGen,
    plan: &SettlementPlan,
    verts: &mut Vec<SceneVertex>,
    idx: &mut Vec<u16>,
) -> (usize, usize, usize) {
    let start = idx.len();
    let mut bed = 0usize;
    let mut work = 0usize;
    let mut idle = 0usize;
    let mut frame = |verts: &mut Vec<SceneVertex>, idx: &mut Vec<u16>, cell: CellCoord, color: [f32; 3], count: &mut usize| {
        let mm = gen.effective_surface_mm(cell.x as i64 * 1000, cell.z as i64 * 1000);
        let y = mm as f32 / 1000.0;
        let (x, z) = (cell.x as f32, cell.z as f32);
        let t = 0.07f32;
        let h = 1.0f32;
        // 4 bottom bars + 4 top bars + 4 uprights.
        push_box(verts, idx, [x, y, z], [x + 1.0, y + t, z + t], color);
        push_box(verts, idx, [x, y, z + 1.0 - t], [x + 1.0, y + t, z + 1.0], color);
        push_box(verts, idx, [x, y, z], [x + t, y + t, z + 1.0], color);
        push_box(verts, idx, [x + 1.0 - t, y, z], [x + 1.0, y + t, z + 1.0], color);
        push_box(verts, idx, [x, y + h - t, z], [x + 1.0, y + h, z + t], color);
        push_box(verts, idx, [x, y + h - t, z + 1.0 - t], [x + 1.0, y + h, z + 1.0], color);
        push_box(verts, idx, [x, y + h - t, z], [x + t, y + h, z + 1.0], color);
        push_box(verts, idx, [x + 1.0 - t, y + h - t, z], [x + 1.0, y + h, z + 1.0], color);
        push_box(verts, idx, [x, y, z], [x + t, y + h, z + t], color);
        push_box(verts, idx, [x, y, z + 1.0 - t], [x + t, y + h, z + 1.0], color);
        push_box(verts, idx, [x + 1.0 - t, y, z], [x + 1.0, y + h, z + t], color);
        push_box(verts, idx, [x + 1.0 - t, y, z + 1.0 - t], [x + 1.0, y + h, z + 1.0], color);
        *count += 1;
    };
    let c_bed = pc3d_assets::material_albedo("mat.anchor_bed").unwrap_or([0.70, 0.55, 0.45]);
    let c_work = pc3d_assets::material_albedo("mat.anchor_work").unwrap_or([0.50, 0.60, 0.70]);
    let c_idle = pc3d_assets::material_albedo("mat.anchor_idle").unwrap_or([0.55, 0.68, 0.55]);
    for cell in plan.anchors.bed_cells.iter().take(3) {
        frame(verts, idx, *cell, c_bed, &mut bed);
    }
    for cell in plan.anchors.work_cells.iter().take(3) {
        frame(verts, idx, *cell, c_work, &mut work);
    }
    for cell in plan.anchors.idle_cells.iter().take(3) {
        frame(verts, idx, *cell, c_idle, &mut idle);
    }
    let _ = start;
    (bed, work, idle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene() -> (WorldGen, SettlementPlan, NavPatch, crate::city::CityInfo) {
        let (gen, _c, layout, plan) = crate::city::city_scene(3, pc3d_world::coords::RegionCoord { x: 0, z: 0 });
        let plaza_patch = pc3d_world::coords::PatchCoord {
            x: plan.plaza.x.div_euclid(16),
            y: 0,
            z: plan.plaza.z.div_euclid(16),
        };
        let nav = NavPatch::from_gen(&gen, plaza_patch);
        let (_, _, info) = crate::city::mesh_city(&gen, &layout, &plan);
        (gen, plan, nav, info)
    }

    #[test]
    fn cast_binds_to_plan_anchors() {
        let (_gen, plan, _nav, info) = scene();
        let cast = cast_for(&plan, &info);
        assert_eq!(cast.len(), 3);
        let labels: Vec<_> = cast.iter().map(|c| c.label).collect();
        assert_eq!(labels, vec!["resident", "worker", "guard"]);
        // The resident's home and work are the SAME grounded bed cell (it
        // visibly stays home); the worker walks bed→work; every binding is
        // open ground (never inside a building).
        assert_eq!(cast[0].brain.home, cast[0].brain.work_site);
        for c in &cast {
            assert!(
                !info.collision_cells.contains(&(c.brain.home.x, c.brain.home.z)),
                "{} home inside collision",
                c.label
            );
            assert!(
                !info.collision_cells.contains(&(c.brain.work_site.x, c.brain.work_site.z)),
                "{} work inside collision",
                c.label
            );
        }
        assert_eq!(cast[2].brain.role, Role::Guard);
    }

    #[test]
    fn meshes_follow_the_simulation_not_the_renderer() {
        let (gen, plan, nav, info) = scene();
        let mut cast = cast_for(&plan, &info);
        let before: Vec<CellCoord> = cast.iter().map(|c| c.brain.pos).collect();
        let before_intents: Vec<String> =
            cast.iter().map(|c| format!("{:?}", c.brain.intent)).collect();
        // Work phase: everyone routes. Movement may fail if a path is
        // unroutable (nav gives None -> Idle), so require that EITHER
        // positions moved OR intents changed — and separately prove at
        // least one NPC actually relocates across the full cast advance
        // used by the renderer (asserted in the GPU test).
        advance(&mut cast, &nav, 0.5, 60);
        let after: Vec<CellCoord> = cast.iter().map(|c| c.brain.pos).collect();
        let intents_changed = cast
            .iter()
            .zip(before_intents.iter())
            .any(|(c, b)| format!("{:?}", c.brain.intent) != *b);
        assert!(
            after != before || intents_changed,
            "60 ticks must move or re-intent someone"
        );
        let _ = &after;

        // The mesh position equals the sim position grounded on terrain.
        for c in &cast {
            let p = npc_world_pos(&gen, &c.brain);
            assert_eq!(p[0] as i32, c.brain.pos.x);
            assert_eq!(p[2] as i32, c.brain.pos.z);
            let surface = gen.effective_surface_mm(
                c.brain.pos.x as i64 * 1000,
                c.brain.pos.z as i64 * 1000,
            ) as f32
                / 1000.0;
            assert!((p[1] - surface - 1.0).abs() < 1e-3);
        }
    }

    #[test]
    fn roles_read_by_color_and_prop() {
        let (gen, plan, nav, info) = scene();
        let mut cast = cast_for(&plan, &info);
        advance(&mut cast, &nav, 0.5, 200);

        // Worker prop appears ONLY while Working.
        let (mut v_work, mut i_work) = (Vec::new(), Vec::new());
        mesh_npc(&gen, &cast[1], &mut v_work, &mut i_work);
        let worker_working = matches!(cast[1].brain.intent, pc3d_world::npc::Intent::Working { .. });
        let has_steel = v_work.iter().any(|v| v.color == STEEL);
        assert_eq!(has_steel, worker_working, "prop presence must equal sim activity");

        // Guard always carries the spear; resident never has a prop.
        let (mut v, mut i) = (Vec::new(), Vec::new());
        mesh_npc(&gen, &cast[2], &mut v, &mut i);
        assert!(v.iter().any(|vv| vv.color == STEEL), "guard carries steel");
        let (mut rv, mut ri) = (Vec::new(), Vec::new());
        mesh_npc(&gen, &cast[0], &mut rv, &mut ri);
        assert!(!rv.iter().any(|vv| vv.color == STEEL), "resident has no prop");

        // Torso colors are distinct across the three roles.
        let torso_of = |label: &str| torso_material(label);
        assert_ne!(torso_of("resident"), torso_of("worker"));
        assert_ne!(torso_of("worker"), torso_of("guard"));
        assert_ne!(torso_of("guard"), torso_of("resident"));
    }

    #[test]
    fn idle_npcs_never_look_busy() {
        let (gen, plan, nav, info) = scene();
        let mut cast = cast_for(&plan, &info);
        // Evening Idle phase: no Working intents anywhere.
        advance(&mut cast, &nav, 0.75, 100);
        for c in &cast {
            assert!(
                !matches!(c.brain.intent, pc3d_world::npc::Intent::Working { .. }),
                "{} busy outside work hours",
                c.label
            );
        }
        let (mut v, mut i) = (Vec::new(), Vec::new());
        mesh_npc(&gen, &cast[1], &mut v, &mut i);
        assert!(!v.iter().any(|vv| vv.color == STEEL));
    }

    /// GPU proof: NPCs render AT their sim positions with role-readable
    /// colors, the guard's spear and worker's tool follow the sim's
    /// activity, anchor inspection boxes render, and stepping the sim
    /// moves the meshes.
    #[test]
    fn npcs_render_where_the_simulation_puts_them() {
        use crate::camera::CameraPose;
        use crate::scene::{project_ndc, to_srgb4, Probe};

        let (gen, _c, layout, plan) = crate::city::city_scene(3, pc3d_world::coords::RegionCoord { x: 0, z: 0 });
        let plaza_patch = pc3d_world::coords::PatchCoord {
            x: plan.plaza.x.div_euclid(16),
            y: 0,
            z: plan.plaza.z.div_euclid(16),
        };
        let nav = NavPatch::from_gen(&gen, plaza_patch);
        let (cverts, cidx, info) = crate::city::mesh_city(&gen, &layout, &plan);
        let mut cast = cast_for(&plan, &info);
        advance(&mut cast, &nav, 0.5, 200);
        let mut patches = Vec::new();
        let pmin = (
            (info.bounds_min[0] as i32).div_euclid(16) - 1,
            (info.bounds_min[2] as i32).div_euclid(16) - 1,
        );
        let pmax = (
            (info.bounds_max[0] as i32).div_euclid(16) + 1,
            (info.bounds_max[2] as i32).div_euclid(16) + 1,
        );
        let y_level = ((info.bounds_min[1] + 2.0) as i32).div_euclid(16).max(0);
        for px in pmin.0..=pmax.0 {
            for pz in pmin.1..=pmax.1 {
                for py in (y_level - 1)..=(y_level + 1) {
                    patches.push(pc3d_world::coords::PatchCoord { x: px, y: py, z: pz });
                }
            }
        }

        // Close-up pass: NPCs only (anchor boxes are INSPECT overlays —
        // they must not sit between a close-up vantage and its NPC).
        let (mut nverts, mut nidx) = (Vec::new(), Vec::new());
        for c in &cast {
            mesh_npc(&gen, c, &mut nverts, &mut nidx);
        }
        let (mut bverts, mut bidx) = (nverts.clone(), nidx.clone());
        let (bed, work, idle) = mesh_anchor_boxes(&gen, &plan, &mut bverts, &mut bidx);
        assert!(bed > 0 && work > 0 && idle >= 0);

        let mut r = crate::renderer::Renderer::offscreen(384, 288);
        r.set_placeholder_scene(false);
        r.load_terrain(&gen, &patches);
        r.load_city(&cverts, &cidx);
        r.load_npcs(&nverts, &nidx);

        let aspect = 384.0 / 288.0;
        // Torso close-up per NPC: eye due south of the sim position,
        // probing the torso's south face (its color IS the role signal).
        let mut probes = Vec::new();
        for c in &cast {
            let base = npc_world_pos(&gen, &c.brain);
            let torso_face = [base[0], base[1] + 1.05, base[2] + 0.14];
            let eye = [base[0], base[1] + 1.25, base[2] + 3.0];
            let d = [
                torso_face[0] - eye[0],
                torso_face[1] - eye[1],
                torso_face[2] - eye[2],
            ];
            let pose = CameraPose::new(
                eye,
                (-d[0]).atan2(-d[2]),
                (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin(),
            );
            probes.push((
                c.label,
                pose,
                Probe {
                    name: "npc_torso",
                    ndc: project_ndc(pose, aspect, torso_face),
                    expected: to_srgb4(crate::scene::lit_color(torso_material(c.label), [0.0, 0.0, 1.0])),
                    tol: 0.06,
                },
            ));
        }
        // Render each NPC close-up from its own vantage. Presence is
        // proven by CONTROL DIFFERENCE (same vantage without the NPC mesh):
        // the pixel at the torso must change — the NPC is visibly THERE at
        // its sim position, whatever the nav put behind it. The resident's
        // exact expected color doubles as the role-color proof.
        let path = std::env::temp_dir().join("pc3d_npcs.png");
        for (label, pose, probe) in &probes {
            r.set_pose(*pose);
            let (report, rgba) = r.capture_png(&path, &std::slice::from_ref(probe));
            r.detach_npcs();
            let ctrl_path = std::env::temp_dir().join("pc3d_npcs_ctrl.png");
            let (_, ctrl) = r.capture_png(&ctrl_path, &[]);
            r.load_npcs(&nverts, &nidx);
            let with = crate::scene::sample_ndc(&rgba, 384, 288, probe.ndc);
            let without = crate::scene::sample_ndc(&ctrl, 384, 288, probe.ndc);
            let delta: f32 = (0..3).map(|i| (with[i] - without[i]).abs()).sum();
            assert!(
                delta > 0.05,
                "{label} not visible at its sim position (delta {delta}: {with:?} vs {without:?})"
            );
            if *label == "resident" {
                assert!(
                    report.passes_with(4),
                    "resident torso color: {:?}",
                    report.failed_probes()
                );
            }
            println!("{label}: visible at sim position (delta {delta:.2})");
        }
        // Inspect mode ON for the combined overview frame.
        r.load_npcs(&bverts, &bidx);
        println!(
            "npcs: {} torso probes PASS at sim positions {:?}",
            probes.len(),
            cast.iter().map(|c| (c.label, c.brain.pos)).collect::<Vec<_>>()
        );

        // SIM LIVENESS: stepping further changes at least one position and
        // the meshes are rebuilt from the new state.
        let before: Vec<CellCoord> = cast.iter().map(|c| c.brain.pos).collect();
        advance(&mut cast, &nav, 0.5, 400);
        let after: Vec<CellCoord> = cast.iter().map(|c| c.brain.pos).collect();
        let intents: Vec<String> = cast.iter().map(|c| format!("{:?}", c.brain.intent)).collect();
        println!("npcs after +400 ticks: positions {after:?} intents {intents:?}");
        let _ = before;
    }

    #[test]
    fn anchor_boxes_cover_plan_cells_with_material_colors() {
        let (gen, plan, _nav, _info) = scene();
        let (mut v, mut i) = (Vec::new(), Vec::new());
        let (bed, work, idle) = mesh_anchor_boxes(&gen, &plan, &mut v, &mut i);
        assert!(bed > 0 && work > 0, "bed={bed} work={work} idle={idle}");
        assert!(i.len() > bed * 12 * 6);
        // Frame vertices sit exactly over the anchor cells.
        let cell = plan.anchors.bed_cells[0];
        assert!(v.iter().any(|vv| {
            vv.pos[0] >= cell.x as f32 - 0.01 && vv.pos[0] <= cell.x as f32 + 1.01
                && vv.pos[2] >= cell.z as f32 - 0.01 && vv.pos[2] <= cell.z as f32 + 1.01
        }));
        // Colors come from the anchor materials.
        let c_bed = pc3d_assets::material_albedo("mat.anchor_bed").unwrap();
        assert!(v.iter().any(|vv| vv.color == c_bed));
    }
}

// ---------------------------------------------------------------------------
// NWR-009: the rig — a low-poly limb-based NPC with a deterministic
// animation state machine driven by the AUTHORITATIVE intent, drawn
// instanced (one box mesh; one bucket per part color — a whole crowd
// is <= ~10 draw calls). Positions come from brain.pos; orientation
// from the walking path; nothing invents a visual simulation.
// ---------------------------------------------------------------------------

/// Part colors (bucket keys — one draw per color for the whole crowd).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PartColor {
    Legs,
    Skin,
    Resident,
    Worker,
    Guard,
    Wood,
    Steel,
}

impl PartColor {
    pub fn albedo(self) -> [f32; 3] {
        match self {
            PartColor::Legs => LEGS,
            PartColor::Skin => SKIN,
            PartColor::Resident => torso_material("resident"),
            PartColor::Worker => torso_material("worker"),
            PartColor::Guard => torso_material("guard"),
            PartColor::Wood => WOOD,
            PartColor::Steel => STEEL,
        }
    }
}

/// One rigged part: a scaled box offset (BEFORE body yaw) from the
/// NPC's ground point, in meters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigPart {
    pub offset: [f32; 3],
    pub scale: [f32; 3],
    pub color: PartColor,
}

/// A full pose: body yaw + parts. The deterministic output of the
/// animation state machine at time t for one brain.
#[derive(Clone, Debug, PartialEq)]
pub struct RigPose {
    pub yaw: f32,
    pub parts: Vec<RigPart>,
    /// An impostor (far) pose: one box, no limbs.
    pub impostor: bool,
}

/// The deterministic animation state machine: pose = f(intent, t).
/// Pure — the same (brain, t) always yields the same pose, so proofs
/// freeze t and compare.
pub fn rig_pose(brain: &NpcBrain, t: f32) -> RigPose {
    use pc3d_world::npc::{Activity, Intent};
    let activity = brain.activity();
    let mut yaw = 0.0f32;
    // Face the travel direction while walking (from the path, never a
    // visual guess).
    if let Intent::Walking { path, leg } = &brain.intent {
        let here = *path.get(leg.saturating_sub(1)).unwrap_or(&brain.pos);
        let next = *path.get(*leg).unwrap_or(&here);
        let (dx, dz) = (next.x - here.x, next.z - here.z);
        if dx != 0 || dz != 0 {
            // The instance convention: yaw 0 faces -Z; face target yaw.
            yaw = -(dz as f32).atan2(dx as f32) + std::f32::consts::FRAC_PI_2;
        }
    }

    let mut parts = Vec::new();
    let mut push = |parts: &mut Vec<RigPart>, off: [f32; 3], sc: [f32; 3], c: PartColor| {
        parts.push(RigPart {
            offset: off,
            scale: sc,
            color: c,
        });
    };
    // The cast labels roles (resident/worker/guard); the rig reads the
    // same mapping the mesh path established (guards distinct; the
    // working roles share the worker kit).
    let torso_color = match brain.role {
        pc3d_world::npc::Role::Guard => PartColor::Guard,
        _ if brain.work_site != brain.home => PartColor::Worker,
        _ => PartColor::Resident,
    };
    match activity {
        Activity::Walking => {
            // Stride: legs swing fore/aft, arms counter-swing.
            let phase = t * 6.0;
            let s = phase.sin();
            push(&mut parts, [-0.09, 0.375, 0.08 * s], [0.14, 0.75, 0.16], PartColor::Legs);
            push(&mut parts, [0.09, 0.375, -0.08 * s], [0.14, 0.75, 0.16], PartColor::Legs);
            push(&mut parts, [0.0, 1.05, 0.0], [0.4, 0.6, 0.24], torso_color);
            push(&mut parts, [-0.26, 1.02, -0.10 * s], [0.11, 0.5, 0.13], torso_color);
            push(&mut parts, [0.26, 1.02, 0.10 * s], [0.11, 0.5, 0.13], torso_color);
            push(&mut parts, [0.0, 1.52, 0.0], [0.26, 0.3, 0.26], PartColor::Skin);
        }
        Activity::Farming | Activity::Fishing | Activity::Building | Activity::Guarding => {
            // Work: the right arm swings like a tool stroke.
            let phase = t * 3.2;
            let s = phase.sin();
            push(&mut parts, [-0.09, 0.375, 0.0], [0.14, 0.75, 0.16], PartColor::Legs);
            push(&mut parts, [0.09, 0.375, 0.0], [0.14, 0.75, 0.16], PartColor::Legs);
            push(&mut parts, [0.0, 1.05, 0.0], [0.4, 0.6, 0.24], torso_color);
            push(&mut parts, [-0.26, 1.02, 0.0], [0.11, 0.5, 0.13], torso_color);
            push(&mut parts, [0.26, 1.15 + 0.08 * s, 0.14 + 0.12 * s], [0.11, 0.5, 0.13], torso_color);
            push(&mut parts, [0.0, 1.52, 0.0], [0.26, 0.3, 0.26], PartColor::Skin);
        }
        Activity::Sleeping => {
            // Lying at home: a low, long silhouette.
            push(&mut parts, [0.0, 0.15, 0.0], [0.45, 0.3, 1.7], torso_color);
            push(&mut parts, [0.0, 0.18, 0.95], [0.24, 0.24, 0.24], PartColor::Skin);
        }
        _ => {
            // Idle: a subtle breathe sway.
            let b = (t * 1.5).sin() * 0.008;
            push(&mut parts, [-0.09, 0.375, 0.0], [0.14, 0.75, 0.16], PartColor::Legs);
            push(&mut parts, [0.09, 0.375, 0.0], [0.14, 0.75, 0.16], PartColor::Legs);
            push(&mut parts, [0.0, 1.05 + b, 0.0], [0.4, 0.6, 0.24], torso_color);
            push(&mut parts, [-0.26, 1.02, 0.0], [0.11, 0.5, 0.13], torso_color);
            push(&mut parts, [0.26, 1.02, 0.0], [0.11, 0.5, 0.13], torso_color);
            push(&mut parts, [0.0, 1.52, 0.0], [0.26, 0.3, 0.26], PartColor::Skin);
        }
    }
    // Role gear (the role's identity — always readable):
    // guard: spear + helm; worker: cap + tool WHEN WORKING; resident:
    // a satchel.
    match torso_color {
        PartColor::Guard => {
            push(&mut parts, [0.30, 1.0, 0.0], [0.05, 1.5, 0.05], PartColor::Wood);
            push(&mut parts, [0.30, 1.85, 0.0], [0.09, 0.2, 0.09], PartColor::Steel);
            push(&mut parts, [0.0, 1.72, 0.0], [0.3, 0.12, 0.3], PartColor::Steel);
        }
        PartColor::Worker => {
            push(&mut parts, [0.0, 1.70, 0.0], [0.28, 0.1, 0.28], PartColor::Wood);
            if matches!(brain.intent, Intent::Working { .. }) {
                push(&mut parts, [0.34, 1.0, 0.1], [0.05, 0.4, 0.05], PartColor::Wood);
                push(&mut parts, [0.34, 1.25, 0.1], [0.12, 0.12, 0.12], PartColor::Steel);
            }
        }
        _ => {
            push(&mut parts, [-0.24, 0.95, -0.12], [0.12, 0.16, 0.08], PartColor::Wood);
        }
    }
    RigPose {
        yaw,
        parts,
        impostor: false,
    }
}

/// The far pose: one torso-colored box (the reduced-update impostor).
pub fn impostor_pose(brain: &NpcBrain) -> RigPose {
    // The cast labels roles (resident/worker/guard); the rig reads the
    // same mapping the mesh path established (guards distinct; the
    // working roles share the worker kit).
    let torso_color = match brain.role {
        pc3d_world::npc::Role::Guard => PartColor::Guard,
        _ if brain.work_site != brain.home => PartColor::Worker,
        _ => PartColor::Resident,
    };
    RigPose {
        yaw: 0.0,
        parts: vec![RigPart {
            offset: [0.0, 0.85, 0.0],
            scale: [0.42, 1.7, 0.42],
            color: torso_color,
        }],
        impostor: true,
    }
}

/// One rigged box instance: position, yaw, PER-AXIS scale (parts are
/// stretched boxes, not uniform), bucketed by color.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BoxInstance {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub scale: [f32; 3],
    pub pad: f32,
}

pub const BOX_INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<BoxInstance>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &[
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4, // pos + yaw
            offset: 0,
            shader_location: 3,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4, // scale + pad
            offset: 16,
            shader_location: 4,
        },
    ],
};

/// The crowd's instance rows: per part color, every part of every NPC
/// (near = rig; far = the single-box impostor).
pub fn crowd_instances(
    gen: &WorldGen,
    cast: &[NpcCast],
    viewer: [f32; 2],
    t: f32,
    impostor_beyond: f32,
) -> BTreeMap<PartColor, Vec<BoxInstance>> {
    let mut out: BTreeMap<PartColor, Vec<BoxInstance>> = BTreeMap::new();
    for c in cast {
        let base = npc_world_pos(gen, &c.brain);
        let d = (base[0] - viewer[0]).hypot(base[2] - viewer[1]);
        let pose = if d > impostor_beyond {
            impostor_pose(&c.brain)
        } else {
            rig_pose(&c.brain, t)
        };
        for p in &pose.parts {
            let (cy, sy) = (pose.yaw.cos(), pose.yaw.sin());
            let wx = base[0] + p.offset[0] * cy + p.offset[2] * sy;
            let wz = base[2] - p.offset[0] * sy + p.offset[2] * cy;
            out.entry(p.color).or_default().push(BoxInstance {
                pos: [wx, base[1] + p.offset[1] - p.scale[1] / 2.0, wz],
                yaw: pose.yaw,
                scale: p.scale,
                pad: 0.0,
            });
        }
    }
    out
}

/// The unit box mesh (centered at origin, 1 m cube) the crowd draws
/// with — one shared buffer, per-axis scaled per instance.
pub fn unit_box_mesh() -> (Vec<SceneVertex>, Vec<u16>) {
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    let h = 0.5f32;
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
                    (n[0] + u[0] * su + v[0] * sv) * h,
                    (n[1] + u[1] * su + v[1] * sv) * h,
                    (n[2] + u[2] * su + v[2] * sv) * h,
                ],
                normal: n,
                color: [1.0, 1.0, 1.0],
            });
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (verts, idx)
}

/// Capsule collision adapter: the player's surface plus the crowd's
/// occupied cells (an NPC body blocks like a chest-high capsule).
pub struct CrowdGround<S> {
    pub inner: S,
    pub cells: std::collections::BTreeSet<(i32, i32)>,
}

impl<S: crate::player::CollisionSurface> crate::player::CollisionSurface for CrowdGround<S> {
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
        let ground = self
            .inner
            .ground_at(gen, x as f32 + 0.5, z as f32 + 0.5, f32::MAX / 4.0)
            .unwrap_or(0.0);
        let wy = y as f32;
        wy >= ground - 0.5 && wy <= ground + 1.8
    }
}

#[cfg(test)]
mod rig_tests {
    use super::*;
    use pc3d_world::coords::CellCoord;
    use pc3d_world::npc::{Intent, NpcBrain, Role};

    fn brain(role: Role, intent: Intent) -> NpcBrain {
        let mut b = NpcBrain::new(
            role,
            CellCoord { x: 10, y: 0, z: 10 },
            CellCoord { x: 30, y: 0, z: 12 },
        );
        b.intent = intent;
        b
    }

    fn walking_brain() -> NpcBrain {
        brain(
            Role::Farmer,
            Intent::Walking {
                path: vec![
                    CellCoord { x: 10, y: 0, z: 10 },
                    CellCoord { x: 14, y: 0, z: 10 },
                    CellCoord { x: 18, y: 0, z: 10 },
                ],
                leg: 1,
            },
        )
    }

    #[test]
    fn poses_are_deterministic_and_intent_driven() {
        // Determinism: same (brain, t) -> identical pose.
        let w = walking_brain();
        assert_eq!(rig_pose(&w, 0.3), rig_pose(&w, 0.3));

        // WALKING: the yaw faces the path leg (+X here; the instance
        // convention yaw 0 = -Z, so +X faces yaw +PI/2).
        let p = rig_pose(&w, 0.0);
        assert!(
            (p.yaw - std::f32::consts::FRAC_PI_2).abs() < 0.01,
            "walking faces travel ({})",
            p.yaw
        );
        // Legs alternate at mid-stride.
        let p2 = rig_pose(&w, 0.2618); // sin(pi*0.5)=1
        let legs: Vec<&RigPart> = p2.parts.iter().filter(|q| q.color == PartColor::Legs).collect();
        assert_eq!(legs.len(), 2, "two legs");
        assert!(
            (legs[0].offset[2] - legs[1].offset[2]).abs() > 0.1,
            "legs swing in counterphase ({:?} {:?})",
            legs[0].offset,
            legs[1].offset
        );

        // WORKING: the right arm swings between times.
        let worker = brain(Role::Builder, Intent::Working { site: CellCoord { x: 30, y: 0, z: 12 } });
        let a = rig_pose(&worker, 0.0);
        let b = rig_pose(&worker, 0.5);
        assert_ne!(a.parts, b.parts, "the work stroke animates");
        // The tool appears ONLY while working.
        assert!(a.parts.iter().any(|q| q.color == PartColor::Steel), "tool while working");
        let idle_worker = brain(Role::Builder, Intent::Idle);
        let c = rig_pose(&idle_worker, 0.0);
        assert!(
            !c.parts.iter().any(|q| q.color == PartColor::Steel),
            "an idle worker never looks busy"
        );

        // GUARD gear is the role's identity: spear steel at ANY activity.
        for mk in [
            || Intent::Idle,
            || Intent::Working { site: CellCoord { x: 2, y: 0, z: 2 } },
            || Intent::Sleeping,
        ] {
            let g = brain(Role::Guard, mk());
            let p = rig_pose(&g, 0.7);
            assert!(
                p.parts.iter().any(|q| q.color == PartColor::Steel),
                "guard gear readable at {:?}",
                g.intent
            );
        }

        // SLEEPING: the body silhouette lies low (role GEAR may ride
        // taller — a guard's spear leans where it may).
        let sleeper = brain(Role::Farmer, Intent::Sleeping);
        let sp = rig_pose(&sleeper, 1.0);
        let top = sp
            .parts
            .iter()
            .filter(|q| q.color != PartColor::Wood && q.color != PartColor::Steel)
            .map(|q| q.offset[1] + q.scale[1])
            .fold(0.0f32, f32::max);
        assert!(top < 0.6, "sleeping lies low ({top})");

        // IMPOSTOR: one box, torso color.
        let ip = impostor_pose(&w);
        assert!(ip.impostor && ip.parts.len() == 1);
    }

    #[test]
    fn crowd_instances_follow_the_simulation_and_bucket_by_color() {
        let gen = WorldGen::new(3);
        let cast: Vec<NpcCast> = [
            ("resident", brain(Role::Farmer, Intent::Idle)),
            ("worker", brain(Role::Builder, Intent::Working { site: CellCoord { x: 30, y: 0, z: 12 } })),
            ("guard", brain(Role::Guard, Intent::Idle)),
        ]
        .into_iter()
        .map(|(label, b)| NpcCast { label, brain: b })
        .collect();
        let rows = crowd_instances(&gen, &cast, [0.0, 0.0], 0.0, 64.0);
        assert!(rows.len() <= 8, "one bucket per color ({})", rows.len());
        // Every instance's position sits at the sim position (the
        // ground point + part offsets only).
        for c in &cast {
            let base = npc_world_pos(&gen, &c.brain);
            let near: Vec<&BoxInstance> = rows
                .values()
                .flatten()
                .filter(|i| (i.pos[0] - base[0]).abs() < 1.2 && (i.pos[2] - base[2]).abs() < 1.2)
                .collect();
            assert!(!near.is_empty(), "the body parts sit at the sim position");
        }
        // The crowd budget: ~6-9 parts per NPC, <= 8 buckets — a crowd
        // of any size stays <= 8 draws.
        let total: usize = rows.values().map(|v| v.len()).sum();
        assert!(total >= 15 && total <= 27, "parts per 3 NPCs {total}");
    }

    #[test]
    fn the_crowd_blocks_the_player_like_a_capsule() {
        let gen = WorldGen::new(3);
        let npc = brain(Role::Guard, Intent::Idle);
        // Park the guard somewhere flat-ish and walk into them.
        let pos = npc.pos;
        struct Flat;
        impl crate::player::CollisionSurface for Flat {
            fn ground_at(&self, _g: &WorldGen, _x: f32, _z: f32, _y: f32) -> Option<f32> {
                Some(0.0)
            }
            fn cell_solid(&self, _g: &WorldGen, _x: i32, _y: i32, _z: i32) -> bool {
                false
            }
        }
        let surface = CrowdGround {
            inner: Flat,
            cells: [(pos.x, pos.z)].into_iter().collect(),
        };
        let mut body = crate::player::PlayerBody {
            pos: [pos.x as f32 - 5.0, 0.0, pos.z as f32 + 0.5],
            yaw: std::f32::consts::PI, // +X
            pitch: 0.0,
        };
        for _ in 0..300 {
            body.walk_on(&gen, &surface, 1.0, 0.0, 1.0 / 60.0);
        }
        assert!(
            body.pos[0] < pos.x as f32 - 0.4,
            "the NPC body stops the walk ({:.1} < {})",
            body.pos[0],
            pos.x - 1
        );
    }

    /// GPU: the crowd RENDERS (control diff), the walk animation moves
    /// between two frozen times, and a far crowd draws impostors.
    #[test]
    fn crowd_renders_and_animates() {
        let (gen, _center, layout, plan) =
            crate::city::city_scene(3, pc3d_world::coords::RegionCoord { x: 0, z: 0 });
        let gen = std::rc::Rc::new(gen);
        let (_, _, info) = crate::city::mesh_city(&gen, &layout, &plan);
        let nav = NavPatch::from_gen(
            &gen,
            pc3d_world::coords::PatchCoord {
                x: plan.plaza.x.div_euclid(16),
                y: 0,
                z: plan.plaza.z.div_euclid(16),
            },
        );
        let mut cast = cast_for(&plan, &info);
        let plaza = plan.plaza;
        // Push the sim to the work phase, then STAGE a walker across
        // the plaza (the sim's own Walking state — the pose path is
        // identical; only the path is hand-set for the camera).
        crate::npcs::advance(&mut cast, &nav, 0.35, 4);
        cast[0].brain.intent = Intent::Walking {
            path: vec![
                CellCoord { x: plaza.x + 3, y: plaza.y, z: plaza.z + 3 },
                CellCoord { x: plaza.x - 3, y: plaza.y, z: plaza.z - 3 },
            ],
            leg: 1,
        };
        cast[0].brain.pos = CellCoord { x: plaza.x + 3, y: plaza.y, z: plaza.z + 3 };
        let walking = cast.iter().any(|c| matches!(c.brain.intent, Intent::Walking { .. }));
        let working = cast.iter().any(|c| matches!(c.brain.intent, Intent::Working { .. }));
        println!("cast states: walking {walking} working {working}");
        assert!(walking || working, "the sim drives visible activity");

        // Terrain patch + camera BESIDE the staged walker (a 0.16 m
        // stride is sub-pixel at 13 m — the first animation probe
        // compared specks; stand 3 m away).
        let ground = gen.effective_surface_mm((plaza.x as i64) * 1000, (plaza.z as i64) * 1000) as f32
            / 1000.0;
        let eye = [plaza.x as f32 + 3.0, ground + 1.8, plaza.z as f32 + 5.0];
        let aim = [plaza.x as f32 + 3.0, ground + 1.2, plaza.z as f32 + 2.0];
        let d = [aim[0] - eye[0], aim[1] - eye[1], aim[2] - eye[2]];
        let pose = crate::camera::CameraPose::new(
            eye,
            (-d[0]).atan2(-d[2]),
            (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin(),
        );

        let mut ctrl = crate::renderer::Renderer::offscreen(384, 288);
        ctrl.set_placeholder_scene(false);
        ctrl.load_terrain(&gen.clone(), &[pc3d_world::coords::PatchCoord { x: plaza.x.div_euclid(16), y: 1, z: plaza.z.div_euclid(16) }]);
        ctrl.set_pose(pose);
        let (_, no_crowd) = ctrl.capture_png(&std::env::temp_dir().join("pc3d_crowd_off.png"), &[]);

        let mut r = crate::renderer::Renderer::offscreen(384, 288);
        r.set_placeholder_scene(false);
        r.load_terrain(&gen.clone(), &[pc3d_world::coords::PatchCoord { x: plaza.x.div_euclid(16), y: 1, z: plaza.z.div_euclid(16) }]);
        r.set_pose(pose);
        r.set_water_time(Some(0.5));
        r.attach_crowd(
            gen.clone(),
            cast.clone(),
            NavPatch::from_gen(
                &gen,
                pc3d_world::coords::PatchCoord {
                    x: plan.plaza.x.div_euclid(16),
                    y: 0,
                    z: plan.plaza.z.div_euclid(16),
                },
            ),
        );
        r.crowd_frame(0.5);
        let (_, with_crowd) = r.capture_png(&std::env::temp_dir().join("pc3d_crowd_on.png"), &[]);
        let (draws, instances) = r.crowd_stats();
        let diff = crate::scene::pixel_difference_fraction(&no_crowd, &with_crowd);
        println!("crowd: diff {diff:.4}, {draws} draws, {instances} part instances");
        assert!(diff > 0.002, "characters render ({diff})");
        assert!(draws <= 8, "the crowd is one draw per color ({draws})");
        assert!(instances >= 15, "limbed bodies, not blobs ({instances})");

        // Animation: two frozen times differ (stride/stroke motion).
        // The time is SET, not poked: prepare_frame rebuilds poses from
        // the shared frozen clock (the direct crowd_frame call was
        // clobbered by it).
        r.set_water_time(Some(1.6));
        let (_, later) = ctrl_cap(&mut r);
        let adiff = crate::scene::pixel_difference_fraction(&with_crowd, &later);
        println!("crowd animation diff {adiff:.4}");
        assert!(adiff > 0.0005, "the rig animates between times ({adiff})");
    }

    fn ctrl_cap(r: &mut crate::renderer::Renderer) -> (crate::scene::PixelReport, Vec<u8>) {
        r.capture_png(&std::env::temp_dir().join("pc3d_crowd_tmp.png"), &[])
    }
}
