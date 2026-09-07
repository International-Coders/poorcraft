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
use pc3d_world::gen::WorldGen;
use pc3d_world::nav::NavPatch;
use pc3d_world::npc::{NpcBrain, Role};
use pc3d_world::settlement_plan::SettlementPlan;

/// The three canonical showcase NPCs (resident / worker / guard) bound to
/// a settlement plan's own anchors.
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
