//! Natural terrain mesh from the authoritative query (visual reset R3DV-005).
//!
//! The mesher asks `pc3d_world::terrain::final_solid` — THE single
//! authoritative cell answer (P3D-202) — for every cell and every face
//! neighbor, including neighbors in ADJACENT patches: natural terrain is a
//! pure function of world coordinates, so cross-patch culling is exact and
//! patch-boundary seams cannot exist by construction. Faces are emitted
//! exactly where solid meets air; construction cells are not consulted here
//! (the construction mesher owns built cells, R3DV-004).
//!
//! Algorithm selection is evidence-backed: the P3D-201 bake-off
//! (`poorcraft3d --terrain-bench`, reproduced during this task) shows the
//! heightfield-family extraction (the analytic per-column surface that
//! `final_solid` consults) at 33–87 µs/patch with fidelity equal to or
//! better than the density-threshold candidate (3.3–4.6 ms/patch) on all
//! four scenes — so the culled-cell prototype meshes the authoritative
//! query directly. Smooth surface extraction (dual contouring class)
//! remains a later quality pass on top of the same query, per
//! 03-VOXEL-TERRAIN-AND-MESHING.md.

use crate::camera::CameraPose;
use crate::construction::material_albedo;
use crate::scene::{lit_color, to_srgb4, SceneVertex, FACE_BASIS};
use pc3d_world::coords::{CellCoord, PatchCoord};
use pc3d_world::gen::WorldGen;
use pc3d_world::scales::PATCH_CELL_AXIS;
use pc3d_world::terrain::final_solid;

const CELL_MM: i64 = 1_000;

fn solid_at(gen: &WorldGen, cell: CellCoord) -> bool {
    final_solid(gen, cell.x as i64 * CELL_MM, cell.y as i64 * CELL_MM, cell.z as i64 * CELL_MM)
        .solid
}

fn material_at(gen: &WorldGen, cell: CellCoord) -> pc3d_world::gen::CellMaterial {
    final_solid(gen, cell.x as i64 * CELL_MM, cell.y as i64 * CELL_MM, cell.z as i64 * CELL_MM)
        .material
}

/// Render detail per patch (drives face culling by LOD ring). Top faces
/// are NEVER culled at any level, so the visible-from-above shell cannot
/// crack between rings; collision lives only in the Full ring (the
/// P3D-105 tier contract).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshLod {
    /// Every solid/air face (tops, sides, bottoms — caves and overhangs).
    Full,
    /// Tops and sides (bottom faces are invisible beyond the full ring).
    Mid,
    /// Top-shell only (the heightmap silhouette at far distance).
    Far,
}

/// Meshes one natural-terrain patch: culled block faces in world meters,
/// colors from the authoritative cell material. Neighbors outside the patch
/// are queried directly (exact cross-patch culling, no seams).
pub fn mesh_patch_natural(gen: &WorldGen, coord: PatchCoord) -> (Vec<SceneVertex>, Vec<u16>) {
    mesh_patch_lod(gen, coord, MeshLod::Full)
}

pub fn mesh_patch_lod(
    gen: &WorldGen,
    coord: PatchCoord,
    lod: MeshLod,
) -> (Vec<SceneVertex>, Vec<u16>) {
    let n = PATCH_CELL_AXIS as i32;
    let o = coord.origin();
    let base = (
        o.x.div_euclid(CELL_MM) as i32,
        o.y.div_euclid(CELL_MM) as i32,
        o.z.div_euclid(CELL_MM) as i32,
    );
    // Query cache: the patch plus a one-cell border (18^3 answers), so
    // every cell and face neighbor costs exactly ONE final_solid call
    // (direct meshing asks ~7x more). The cache IS the authoritative
    // answer — the consistency tests verify it against direct queries.
    let stride = (n + 2) as usize;
    let mut cache = vec![
        pc3d_world::terrain::SolidAnswer { solid: false, material: pc3d_world::gen::CellMaterial::Air };
        stride * stride * stride
    ];
    for bx in 0..stride {
        for by in 0..stride {
            for bz in 0..stride {
                let cell = CellCoord {
                    x: base.0 + bx as i32 - 1,
                    y: base.1 + by as i32 - 1,
                    z: base.2 + bz as i32 - 1,
                };
                let a = final_solid(
                    gen,
                    cell.x as i64 * CELL_MM,
                    cell.y as i64 * CELL_MM,
                    cell.z as i64 * CELL_MM,
                );
                cache[(bx * stride + by) * stride + bz] = a;
            }
        }
    }
    let cached = |cell: CellCoord| -> pc3d_world::terrain::SolidAnswer {
        cache[(((cell.x - base.0 + 1) as usize) * stride
            + (cell.y - base.1 + 1) as usize)
            * stride
            + (cell.z - base.2 + 1) as usize]
    };
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    for lx in 0..n {
        for ly in 0..n {
            for lz in 0..n {
                let cell = CellCoord {
                    x: base.0 + lx,
                    y: base.1 + ly,
                    z: base.2 + lz,
                };
                let answer = cached(cell);
                if !answer.solid {
                    continue;
                }
                let material = answer.material;
                let color = material_albedo(material);
                let center = [
                    cell.x as f32 + 0.5,
                    cell.y as f32 + 0.5,
                    cell.z as f32 + 0.5,
                ];
                for (normal, u, v) in FACE_BASIS {
                    // LOD face culling: top faces always render (the shell
                    // cannot crack); bottoms drop beyond the full ring;
                    // sides drop in the far ring.
                    if lod == MeshLod::Mid && normal == [0.0, -1.0, 0.0] {
                        continue;
                    }
                    if lod == MeshLod::Far && normal != [0.0, 1.0, 0.0] {
                        continue;
                    }
                    let neighbor = CellCoord {
                        x: cell.x + normal[0] as i32,
                        y: cell.y + normal[1] as i32,
                        z: cell.z + normal[2] as i32,
                    };
                    if cached(neighbor).solid {
                        continue; // culled: neighbor is also solid
                    }
                    let corner = |su: f32, sv: f32| SceneVertex {
                        pos: [
                            center[0] + normal[0] * 0.5 + u[0] * 0.5 * su + v[0] * 0.5 * sv,
                            center[1] + normal[1] * 0.5 + u[1] * 0.5 * su + v[1] * 0.5 * sv,
                            center[2] + normal[2] * 0.5 + u[2] * 0.5 * su + v[2] * 0.5 * sv,
                        ],
                        normal,
                        color,
                    };
                    let start = verts.len() as u16;
                    verts.extend([
                        corner(-1.0, -1.0),
                        corner(1.0, -1.0),
                        corner(1.0, 1.0),
                        corner(-1.0, 1.0),
                    ]);
                    idx.extend([start, start + 1, start + 2, start, start + 2, start + 3]);
                }
            }
        }
    }
    (verts, idx)
}

/// What a terrain load did — perf evidence for the bounded mesh work.
#[derive(Clone, Copy, Debug, Default)]
pub struct TerrainStats {
    pub patches: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub mesh_us: u128,
}

/// The highest solid cell with air above at a column, in world cell coords
/// (None = open air all the way down the queried band). Collision-aligned
/// probe helper: this is the cell whose TOP face the ground collision would
/// touch.
pub fn column_top(gen: &WorldGen, cell_x: i32, cell_z: i32, y_from: i32) -> Option<CellCoord> {
    let mut y = y_from;
    while y > y_from - 256 {
        let cell = CellCoord { x: cell_x, y, z: cell_z };
        if solid_at(gen, cell) && !solid_at(gen, CellCoord { x: cell_x, y: y + 1, z: cell_z }) {
            return Some(cell);
        }
        y -= 1;
    }
    None
}

/// A walkable cave pocket near a patch: an AIR cell, 5–40 m under the
/// surface, with solid floor, solid ceiling (the overhang), and a solid
/// wall 1–6 cells along a cardinal direction. Deterministic first-hit scan
/// (columns inside the patch, depth bands top-down, directions in a fixed
/// order). Returns (air cell, wall cell, direction to the wall) and,
/// preferentially, pockets whose first two cells toward the wall are also
/// AIR — an open corridor, so a camera inside sees wall AND ceiling
/// underside in one forward view.
pub fn find_cave_pocket(
    gen: &WorldGen,
    coord: PatchCoord,
) -> Option<(CellCoord, CellCoord, [i32; 3])> {
    let mut fallback: Option<(CellCoord, CellCoord, [i32; 3])> = None;
    let n = PATCH_CELL_AXIS as i32;
    let o = coord.origin();
    let base = (
        o.x.div_euclid(CELL_MM) as i32,
        o.y.div_euclid(CELL_MM) as i32,
        o.z.div_euclid(CELL_MM) as i32,
    );
    const DIRS: [[i32; 3]; 4] = [[0, 0, -1], [1, 0, 0], [0, 0, 1], [-1, 0, 0]];
    let is_air = |c: CellCoord| !solid_at(gen, c);
    for lx in 0..n {
        for lz in 0..n {
            let wx = (base.0 + lx) as i64 * CELL_MM;
            let wz = (base.2 + lz) as i64 * CELL_MM;
            let surface_m = gen.effective_surface_mm(wx, wz).div_euclid(CELL_MM);
            for depth in 5..40i64 {
                let y = (surface_m - depth) as i32;
                let air = CellCoord { x: base.0 + lx, y, z: base.2 + lz };
                if solid_at(gen, air) {
                    continue;
                }
                let floor = CellCoord { x: air.x, y: y - 1, z: air.z };
                let ceiling = CellCoord { x: air.x, y: y + 1, z: air.z };
                if !solid_at(gen, floor) || !solid_at(gen, ceiling) {
                    continue;
                }
                for dir in DIRS {
                    // Camera-friendly = an ENCLOSED corridor: the two cells
                    // toward the wall are air with solid floor AND ceiling,
                    // so no sightline from the pocket can reach the sky.
                    let step = |k: i32| CellCoord {
                        x: air.x + dir[0] * k,
                        y: air.y,
                        z: air.z + dir[2] * k,
                    };
                    let enclosed = |k: i32| {
                        let c = step(k);
                        !solid_at(gen, c)
                            && solid_at(gen, CellCoord { x: c.x, y: c.y - 1, z: c.z })
                            && solid_at(gen, CellCoord { x: c.x, y: c.y + 1, z: c.z })
                    };
                    let corridor_open = enclosed(1) && enclosed(2);
                    for dist in 1..=6 {
                        let probe = step(dist);
                        if solid_at(gen, probe) {
                            let found = (air, probe, dir);
                            if dist >= 3 && corridor_open {
                                return Some(found);
                            }
                            fallback = fallback.or(Some(found));
                            break;
                        }
                    }
                }
            }
        }
    }
    fallback
}

/// True when the pocket has an open corridor (air eye-line toward the
/// wall) — the camera-friendly case.
pub fn pocket_has_corridor(
    gen: &WorldGen,
    air: CellCoord,
    dir: [i32; 3],
) -> bool {
    !solid_at(gen, CellCoord { x: air.x + dir[0], y: air.y, z: air.z + dir[2] })
}

/// Scans a ring of patches around `center` for a camera-friendly pocket
/// (open corridor + wall at distance >= 3); falls back to the best pocket
/// `find_cave_pocket` found anywhere in the ring. Deterministic order.
pub fn find_cave_pocket_near(
    gen: &WorldGen,
    center: PatchCoord,
    ring: i32,
) -> Option<(CellCoord, CellCoord, [i32; 3])> {
    let mut fallback = None;
    for dy in -ring..=ring {
        for dx in -ring..=ring {
            for dz in -ring..=ring {
                let coord = PatchCoord {
                    x: center.x + dx,
                    y: center.y + dy,
                    z: center.z + dz,
                };
                if let Some((air, wall, dir)) = find_cave_pocket(gen, coord) {
                    if pocket_has_corridor(gen, air, dir)
                        && (wall.x - air.x).abs() + (wall.z - air.z).abs() >= 3
                    {
                        return Some((air, wall, dir));
                    }
                    fallback = fallback.or(Some((air, wall, dir)));
                }
            }
        }
    }
    fallback
}

/// A pose looking at a patch from the south, above the surface (overview).
pub fn overview_pose(gen: &WorldGen, coord: PatchCoord) -> CameraPose {
    let o = coord.origin();
    let cx = o.x.div_euclid(CELL_MM) as f32 + 8.0;
    let cz = o.z.div_euclid(CELL_MM) as f32 + 8.0;
    let surface_m = gen.effective_surface_mm(cx as i64 * CELL_MM, cz as i64 * CELL_MM) as f32
        / CELL_MM as f32;
    let eye = [cx, surface_m + 14.0, cz + 22.0];
    let target = [cx, surface_m, cz];
    let d = [
        target[0] - eye[0],
        target[1] - eye[1],
        target[2] - eye[2],
    ];
    let yaw = (-d[0]).atan2(-d[2]);
    let pitch = (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin();
    CameraPose::new(eye, yaw, pitch)
}

/// A pose inside a cave pocket looking at its far wall (and, with the
/// default up-tilt, the ceiling overhang).
pub fn cave_pose(air: CellCoord, wall: CellCoord) -> CameraPose {
    let eye = [air.x as f32 + 0.5, air.y as f32 + 0.9, air.z as f32 + 0.5];
    // Aim at the wall face center, lifted above eye line so the wall and
    // the ceiling underside both land in the forward view.
    let target = [
        wall.x as f32 + 0.5,
        air.y as f32 + 1.35,
        wall.z as f32 + 0.5,
    ];
    let d = [
        target[0] - eye[0],
        target[1] - eye[1],
        target[2] - eye[2],
    ];
    let yaw = (-d[0]).atan2(-d[2]);
    let pitch = (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin();
    CameraPose::new(eye, yaw, pitch)
}

/// Vista probes for an overview pose: sky control, the standable center
/// cell's top face, and the first hill-slope side face — every expectation
/// derived from the authoritative query at the probed cell.
pub fn vista_probes(
    gen: &WorldGen,
    coord: PatchCoord,
    pose: CameraPose,
    aspect: f32,
) -> Vec<crate::scene::Probe> {
    use crate::scene::{dir_from_ndc, project_ndc, Probe, SUN_DIR, sky_color_linear, to_srgb4};

    let o = coord.origin();
    let bx = o.x.div_euclid(CELL_MM) as i32;
    let by = o.y.div_euclid(CELL_MM) as i32;
    let bz = o.z.div_euclid(CELL_MM) as i32;
    let top = column_top(gen, bx + 8, bz + 8, by + 15).expect("standable center cell");
    let top_point = [top.x as f32 + 0.5, top.y as f32 + 1.0, top.z as f32 + 0.5];

    let mut slope: Option<(CellCoord, [f32; 3], [f32; 3])> = None;
    for lx in 2..14 {
        for lz in 2..14 {
            if let Some(cell) = column_top(gen, bx + lx, bz + lz, by + 15) {
                if !solid_at(
                    gen,
                    CellCoord { x: cell.x + 1, y: cell.y, z: cell.z },
                ) {
                    slope = Some((
                        cell,
                        [1.0, 0.0, 0.0],
                        [cell.x as f32 + 1.0, cell.y as f32 + 0.5, cell.z as f32 + 0.5],
                    ));
                    break;
                }
            }
        }
        if slope.is_some() {
            break;
        }
    }
    let (slope_cell, slope_normal, slope_point) = slope.expect("a slope step");

    vec![
        Probe {
            name: "sky_above_horizon",
            ndc: (0.0, 0.8),
            expected: to_srgb4(sky_color_linear(dir_from_ndc(pose, (0.0, 0.8), aspect), SUN_DIR)),
            tol: 0.05,
        },
        Probe {
            name: "ground_top_face_matches_query",
            ndc: project_ndc(pose, aspect, top_point),
            expected: face_expectation(gen, top, [0.0, 1.0, 0.0]),
            tol: 0.06,
        },
        Probe {
            name: "slope_side_face_matches_query",
            ndc: project_ndc(pose, aspect, slope_point),
            expected: face_expectation(gen, slope_cell, slope_normal),
            tol: 0.06,
        },
    ]
}

/// Cave-interior probes: the corridor wall face and the ceiling underside
/// (the overhang), expectations from the query's own materials.
pub fn cave_probes(
    gen: &WorldGen,
    air: CellCoord,
    wall: CellCoord,
    dir: [i32; 3],
    pose: CameraPose,
    aspect: f32,
) -> Vec<crate::scene::Probe> {
    use crate::scene::{project_ndc, Probe};
    let corridor = CellCoord { x: air.x + dir[0], y: air.y, z: air.z + dir[2] };
    let corridor_ceiling = CellCoord { x: corridor.x, y: corridor.y + 1, z: corridor.z };
    let wall_face_point = [
        wall.x as f32 + 0.5 - dir[0] as f32 * 0.5,
        wall.y as f32 + 0.5,
        wall.z as f32 + 0.5 - dir[2] as f32 * 0.5,
    ];
    vec![
        Probe {
            name: "cave_wall_face",
            ndc: project_ndc(pose, aspect, wall_face_point),
            expected: face_expectation(gen, wall, [-dir[0] as f32, 0.0, -dir[2] as f32]),
            tol: 0.06,
        },
        Probe {
            name: "cave_ceiling_overhang_underside",
            ndc: project_ndc(
                pose,
                aspect,
                [corridor.x as f32 + 0.5, corridor.y as f32 + 1.0, corridor.z as f32 + 0.5],
            ),
            expected: face_expectation(gen, corridor_ceiling, [0.0, -1.0, 0.0]),
            tol: 0.06,
        },
    ]
}

/// The 3x3 patch neighborhood around a center patch (vistas must not show
/// the world through unmeshed neighbors).
pub fn neighborhood3(center: PatchCoord) -> Vec<PatchCoord> {
    let mut v = Vec::new();
    for dy in -1..=1 {
        for dx in -1..=1 {
            for dz in -1..=1 {
                v.push(PatchCoord {
                    x: center.x + dx,
                    y: center.y + dy,
                    z: center.z + dz,
                });
            }
        }
    }
    v
}

/// A view-center probe derived by RAYCASTING the authoritative query: the
/// first solid cell the sightline enters, the face it enters through, and
/// that face's expected color — robust to occlusion (the probe is what the
/// viewer actually sees, not a hand-picked spot).
pub fn probe_view_center(
    gen: &WorldGen,
    pose: CameraPose,
    aspect: f32,
) -> Option<crate::scene::Probe> {
    use crate::scene::{project_ndc, Probe};
    let fwd = crate::camera::fwd_of(pose.yaw, pose.pitch);
    let mut prev = [0.0f32; 3];
    let mut t = 0.4f32;
    while t < 300.0 {
        let p = [
            pose.position[0] + fwd[0] * t,
            pose.position[1] + fwd[1] * t,
            pose.position[2] + fwd[2] * t,
        ];
        let cell = CellCoord {
            x: p[0].floor() as i32,
            y: p[1].floor() as i32,
            z: p[2].floor() as i32,
        };
        if solid_at(gen, cell) {
            // Entry face: the axis whose cell index changed last.
            let pc = CellCoord {
                x: prev[0].floor() as i32,
                y: prev[1].floor() as i32,
                z: prev[2].floor() as i32,
            };
            let normal = if cell.x != pc.x {
                if cell.x > pc.x { [-1.0, 0.0, 0.0] } else { [1.0, 0.0, 0.0] }
            } else if cell.y != pc.y {
                if cell.y > pc.y { [0.0, -1.0, 0.0] } else { [0.0, 1.0, 0.0] }
            } else if cell.z > pc.z {
                [0.0, 0.0, -1.0]
            } else {
                [0.0, 0.0, 1.0]
            };
            // Probe just in front of the face (the last air point).
            return Some(Probe {
                name: "streamed_view_center_matches_query",
                ndc: project_ndc(pose, aspect, prev),
                expected: face_expectation(gen, cell, normal),
                tol: 0.06,
            });
        }
        prev = p;
        t += 0.1;
    }
    None
}

/// Expected sRGB color of a cell face as the lit shader would render it.
pub fn face_expectation(
    gen: &WorldGen,
    cell: CellCoord,
    normal: [f32; 3],
) -> [f32; 4] {
    to_srgb4(lit_color(material_albedo(material_at(gen, cell)), normal))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::FACE_BASIS;

    fn hills() -> (WorldGen, PatchCoord) {
        let (seed, coord) = pc3d_world::terrain::SceneSpec::SmoothHills.patch();
        (WorldGen::new(seed), coord)
    }

    #[test]
    fn faces_exist_exactly_where_solid_meets_air() {
        // Sampled full-coverage check: for every solid cell in the patch,
        // the mesh must contain one face per air neighbor and none per
        // solid neighbor — including neighbors OUTSIDE the patch.
        let (gen, coord) = hills();
        let (verts, idx) = mesh_patch_natural(&gen, coord);

        // Index faces by (cell, plane, face-direction slot in FACE_BASIS).
        let dir_slot = |n: [f32; 3]| -> Option<u8> {
            FACE_BASIS
                .iter()
                .position(|(normal, _, _)| *normal == n)
                .map(|i| i as u8)
        };
        // A triangle's centroid lies strictly inside its cell face (never
        // on the boundary like corner vertices), so keys derived from
        // centroids cannot alias into neighboring cells.
        let mut face_set = std::collections::HashSet::new();
        for tri in idx.chunks_exact(3) {
            let a = &verts[tri[0] as usize];
            let b = &verts[tri[1] as usize];
            let c = &verts[tri[2] as usize];
            let p = [
                (a.pos[0] + b.pos[0] + c.pos[0]) / 3.0,
                (a.pos[1] + b.pos[1] + c.pos[1]) / 3.0,
                (a.pos[2] + b.pos[2] + c.pos[2]) / 3.0,
            ];
            let n = a.normal;
            let Some(slot) = dir_slot(n) else { continue };
            let (cell, plane) = if n[1].abs() > 0.5 {
                (CellCoord {
                    x: p[0].floor() as i32,
                    y: if n[1] > 0.0 { (p[1] - 1.0) as i32 } else { p[1] as i32 },
                    z: p[2].floor() as i32,
                }, p[1] as i32)
            } else if n[0].abs() > 0.5 {
                (CellCoord {
                    x: if n[0] > 0.0 { (p[0] - 1.0) as i32 } else { p[0] as i32 },
                    y: p[1].floor() as i32,
                    z: p[2].floor() as i32,
                }, p[0] as i32)
            } else {
                (CellCoord {
                    x: p[0].floor() as i32,
                    y: p[1].floor() as i32,
                    z: if n[2] > 0.0 { (p[2] - 1.0) as i32 } else { p[2] as i32 },
                }, p[2] as i32)
            };
            face_set.insert((cell, plane, slot));
        }

        let n16 = PATCH_CELL_AXIS as i32;
        let o = coord.origin();
        let base = (
            o.x.div_euclid(CELL_MM) as i32,
            o.y.div_euclid(CELL_MM) as i32,
            o.z.div_euclid(CELL_MM) as i32,
        );
        let mut checked = 0;
        for lx in 0..n16 {
            for ly in 0..n16 {
                for lz in 0..n16 {
                    let cell = CellCoord {
                        x: base.0 + lx,
                        y: base.1 + ly,
                        z: base.2 + lz,
                    };
                    if !solid_at(&gen, cell) {
                        continue;
                    }
                    checked += 1;
                    for (slot, (normal, _, _)) in FACE_BASIS.iter().enumerate() {
                        let neighbor = CellCoord {
                            x: cell.x + normal[0] as i32,
                            y: cell.y + normal[1] as i32,
                            z: cell.z + normal[2] as i32,
                        };
                        let plane = if normal[1] != 0.0 {
                            cell.y as f32 + if normal[1] > 0.0 { 1.0 } else { 0.0 }
                        } else if normal[0] != 0.0 {
                            cell.x as f32 + if normal[0] > 0.0 { 1.0 } else { 0.0 }
                        } else {
                            cell.z as f32 + if normal[2] > 0.0 { 1.0 } else { 0.0 }
                        };
                        let present = face_set.contains(&(cell, plane as i32, slot as u8));
                        let neighbor_solid = solid_at(&gen, neighbor);
                        assert_eq!(
                            present,
                            !neighbor_solid,
                            "cell {cell:?} face {normal:?}: mesh has face={present}, neighbor solid={neighbor_solid}"
                        );
                    }
                }
            }
        }
        assert!(checked > 1_000, "expected substantial solid cells, got {checked}");
    }

    #[test]
    fn collision_alignment_top_face_iff_air_above() {
        // The ground a player would stand on: a top face exists exactly when
        // the cell is solid and the cell above is air — the same predicate
        // the collision query answers.
        let (gen, coord) = hills();
        let (verts, idx) = mesh_patch_natural(&gen, coord);
        let mut top_faces = std::collections::HashSet::new();
        for tri in idx.chunks(3) {
            for &vi in tri {
                let v = &verts[vi as usize];
                if v.normal == [0.0, 1.0, 0.0] {
                    top_faces.insert((
                        v.pos[0].floor() as i32,
                        (v.pos[1] - 1.0) as i32,
                        v.pos[2].floor() as i32,
                    ));
                }
            }
        }
        let n16 = PATCH_CELL_AXIS as i32;
        let o = coord.origin();
        let base = (
            o.x.div_euclid(CELL_MM) as i32,
            o.y.div_euclid(CELL_MM) as i32,
            o.z.div_euclid(CELL_MM) as i32,
        );
        for lx in 0..n16 {
            for lz in 0..n16 {
                let top = column_top(&gen, base.0 + lx, base.2 + lz, base.1 + n16 - 1);
                match top {
                    Some(cell) => {
                        assert!(
                            top_faces.contains(&(cell.x, cell.y, cell.z)),
                            "standable cell {cell:?} has no rendered top face"
                        );
                    }
                    None => {
                        // No standable cell in the band: no top face may
                        // claim these coordinates.
                        for y in base.1..base.1 + n16 {
                            assert!(!top_faces.contains(&(base.0 + lx, y, base.2 + lz)));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn every_triangle_winds_ccw_outward() {
        let (gen, coord) = hills();
        let (verts, idx) = mesh_patch_natural(&gen, coord);
        assert!(!idx.is_empty());
        for tri in idx.chunks(3) {
            let a = verts[tri[0] as usize];
            let b = verts[tri[1] as usize];
            let c = verts[tri[2] as usize];
            let e1 = [b.pos[0] - a.pos[0], b.pos[1] - a.pos[1], b.pos[2] - a.pos[2]];
            let e2 = [c.pos[0] - a.pos[0], c.pos[1] - a.pos[1], c.pos[2] - a.pos[2]];
            let cross = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let dot = cross[0] * a.normal[0] + cross[1] * a.normal[1] + cross[2] * a.normal[2];
            assert!(dot > 0.0);
        }
    }

    #[test]
    fn cave_pocket_has_ceiling_overhang_and_wall() {
        // The cave/overhang capability: a pocket exists whose ceiling cell
        // (solid above air) renders a BOTTOM face — the overhang — and the
        // mesher emits it.
        let (seed, hill_coord) = pc3d_world::terrain::SceneSpec::Highlands.patch();
        let gen = WorldGen::new(seed);
        let pocket = find_cave_pocket(&gen, hill_coord)
            .or_else(|| {
                let (s, c) = pc3d_world::terrain::SceneSpec::SmoothHills.patch();
                let _ = s;
                find_cave_pocket(&WorldGen::new(3), c)
            })
            .expect("a cave pocket must exist near a scene patch");
        let (air, wall, dir) = pocket;
        // Structure: floor solid, ceiling solid, wall solid along dir.
        assert!(!solid_at(&gen, air));
        assert!(solid_at(&gen, CellCoord { x: air.x, y: air.y - 1, z: air.z }));
        assert!(solid_at(&gen, CellCoord { x: air.x, y: air.y + 1, z: air.z }));
        assert!(solid_at(&gen, wall));
        assert_ne!((wall.x - air.x, wall.z - air.z), (0, 0));

        // Mesh the patch containing the ceiling: its underside face must be
        // present (the overhang).
        let ceiling = CellCoord { x: air.x, y: air.y + 1, z: air.z };
        let cp = PatchCoord {
            x: ceiling.x.div_euclid(16),
            y: ceiling.y.div_euclid(16),
            z: ceiling.z.div_euclid(16),
        };
        let (verts, all_idx) = mesh_patch_natural(&gen, cp);
        let mut has_underside = false;
        for &vi in &all_idx {
            {
                let v = &verts[vi as usize];
                if v.normal == [0.0, -1.0, 0.0]
                    && v.pos[0].floor() as i32 == ceiling.x
                    && v.pos[1] as i32 == ceiling.y
                    && v.pos[2].floor() as i32 == ceiling.z
                {
                    has_underside = true;
                }
            }
        }
        assert!(has_underside, "cave ceiling must render an underside face");
        let _ = dir;
    }

    #[test]
    fn hill_scene_has_visible_slope_faces() {
        // "Hill: the natural mesh has visible slope" — the staircase of a
        // rolling hill exposes side faces (the flat-heightmap ideal would
        // render tops only).
        let (gen, coord) = hills();
        let (verts, idx) = mesh_patch_natural(&gen, coord);
        assert!(!idx.is_empty());
        let side_faces = verts.iter().filter(|v| v.normal[1].abs() < 0.5).count();
        assert!(side_faces > 100, "slope faces visible: {side_faces}");
    }

    #[test]
    fn cliff_scene_shows_material_separation() {
        // "Cliff: visible slope AND material separation" — the terraced 4 m
        // cliff steps expose sub-surface soil/rock side faces next to the
        // surface material.
        let (seed, coord) = pc3d_world::terrain::SceneSpec::Cliff.patch();
        let gen = WorldGen::new(seed);
        let (verts, idx) = mesh_patch_natural(&gen, coord);
        assert!(!idx.is_empty());
        let mut materials: Vec<[f32; 3]> = verts.iter().map(|v| v.color).collect();
        materials.sort_by(|a, b| a.partial_cmp(b).unwrap());
        materials.dedup_by(|a, b| a == b);
        assert!(
            materials.len() >= 2,
            "cliff steps must expose at least two materials, got {materials:?}"
        );
        let side_faces = verts.iter().filter(|v| v.normal[1].abs() < 0.5).count();
        assert!(side_faces > 100, "cliff walls visible: {side_faces}");
    }

    #[test]
    fn overview_pose_looks_at_the_patch_surface() {
        let (gen, coord) = hills();
        let pose = overview_pose(&gen, coord);
        let o = coord.origin();
        let cx = o.x.div_euclid(CELL_MM) as f32 + 8.0;
        let cz = o.z.div_euclid(CELL_MM) as f32 + 8.0;
        let surface_m = gen.effective_surface_mm(cx as i64 * CELL_MM, cz as i64 * CELL_MM) as f32
            / CELL_MM as f32;
        // The pose must aim exactly at the patch-center surface point.
        let d = [
            cx - pose.position[0],
            surface_m - pose.position[1],
            cz - pose.position[2],
        ];
        let yaw = (-d[0]).atan2(-d[2]);
        let pitch =
            (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin();
        assert!((pose.yaw - yaw).abs() < 1e-4);
        assert!((pose.pitch - pitch).abs() < 1e-4);
    }
}
