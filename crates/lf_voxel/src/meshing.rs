use crate::{VoxelSection, BlockState, SECTION_SIZE};
use crate::registry;

/// Which face of a block a texture is selected for (per-face materials:
/// grass top/side/bottom, log rings on the ends, ...).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Face {
    Top,
    Bottom,
    Side,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
    pub ao: f32,
    pub light: u32,
    pub tex_index: u32,
    /// Wind-sway weight (1.0 for foliage, 0.0 for everything else). The
    /// vertex shader offsets `position` by a world-position-phased wave
    /// scaled by this, so animation stays stable across chunk borders.
    pub sway: f32,
}

#[derive(Default)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

/// Simple culled face meshing for a voxel section. `tex_of` maps each block
/// state + face to a texture atlas layer index (per-face materials); light is
/// smoothed per vertex by averaging the four cells around each corner, and
/// ambient occlusion darkens corners occluded by neighboring blocks.
pub fn mesh_section(section: &VoxelSection, neighbor_px: Option<&VoxelSection>, neighbor_nx: Option<&VoxelSection>,
                     neighbor_py: Option<&VoxelSection>, neighbor_ny: Option<&VoxelSection>, neighbor_pz: Option<&VoxelSection>, neighbor_nz: Option<&VoxelSection>,
                     tex_of: &dyn Fn(BlockState, Face) -> u32,
                     light_of: &dyn Fn(i32, i32, i32) -> u32) -> MeshData {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let get_block = |x: i32, y: i32, z: i32| -> BlockState {
        let in_range = |v: i32| (0..16).contains(&v);
        let oob = [!in_range(x), !in_range(y), !in_range(z)];
        if !oob[0] && !oob[1] && !oob[2] {
            section.get(x as usize, y as usize, z as usize)
        } else if oob.iter().filter(|o| **o).count() == 1 {
            // exactly one axis outside the section: ask that face neighbor
            if oob[0] {
                if x < 0 { neighbor_nx.map_or(BlockState::AIR, |n| n.get((x + 16) as usize, y as usize, z as usize)) }
                else { neighbor_px.map_or(BlockState::AIR, |n| n.get((x - 16) as usize, y as usize, z as usize)) }
            } else if oob[1] {
                if y < 0 { neighbor_ny.map_or(BlockState::AIR, |n| n.get(x as usize, (y + 16) as usize, z as usize)) }
                else { neighbor_py.map_or(BlockState::AIR, |n| n.get(x as usize, (y - 16) as usize, z as usize)) }
            } else {
                if z < 0 { neighbor_nz.map_or(BlockState::AIR, |n| n.get(x as usize, y as usize, (z + 16) as usize)) }
                else { neighbor_pz.map_or(BlockState::AIR, |n| n.get(x as usize, y as usize, (z - 16) as usize)) }
            }
        } else {
            // diagonal across a section corner: no diagonal neighbor is
            // available, approximate as air (same policy as a missing
            // face neighbor)
            BlockState::AIR
        }
    };

    // UV patterns: X faces and Y/Z faces use different corner orientations.
    const UVS_A: [[f32; 2]; 4] = [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
    const UVS_B: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

    /// Emit one axis-aligned quad with CCW winding seen from outside.
    fn push_face(vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                 corners: [[f32; 3]; 4], uvs: [[f32; 2]; 4], normal: [f32; 3], tex_index: u32,
                 ao: [f32; 4], light: [u32; 4], sway: f32) {
        let base_idx = vertices.len() as u32;
        for (i, (corner, uv)) in corners.iter().zip(uvs.iter()).enumerate() {
            vertices.push(Vertex {
                position: *corner,
                normal,
                tex_coord: *uv,
                ao: ao[i],
                light: light[i],
                tex_index,
                sway,
            });
        }
        indices.extend_from_slice(&[base_idx, base_idx + 2, base_idx + 1, base_idx, base_idx + 3, base_idx + 2]);
    }

    /// Per-vertex AO and smoothed light for one quad. For each corner, the
    /// three cells diagonal to it in the face's neighbor plane occlude (side1,
    /// side2, corner); light is the per-channel average of the four cells
    /// touching the corner, with opaque cells contributing darkness (classic
    /// voxel smooth lighting).
    let corner_shades = |cell: (i32, i32, i32), normal: [i32; 3], corners: &[[f32; 3]], bmin: [f32; 3]| -> ([f32; 4], [u32; 4]) {
        let axis = if normal[0] != 0 { 0 } else if normal[1] != 0 { 1 } else { 2 };
        let (t1, t2) = if axis == 0 { (1, 2) } else if axis == 1 { (0, 2) } else { (0, 1) };
        let base = (cell.0 + normal[0], cell.1 + normal[1], cell.2 + normal[2]);
        let step = |p: (i32, i32, i32), axis: usize, s: i32| -> (i32, i32, i32) {
            let mut q = p;
            if axis == 0 { q.0 += s; } else if axis == 1 { q.1 += s; } else { q.2 += s; }
            q
        };
        let opaque_at = |p: (i32, i32, i32)| registry::is_opaque(get_block(p.0, p.1, p.2));
        let mut aos = [1.0f32; 4];
        let mut lights = [0u32; 4];
        for (i, c) in corners.iter().enumerate() {
            // which side of THIS block the corner is on (min corner -> -1)
            let sgn = |v: f32, m: f32| if v <= m { -1 } else { 1 };
            let s1 = sgn(c[t1], bmin[t1]);
            let s2 = sgn(c[t2], bmin[t2]);
            let side1 = step(base, t1, s1);
            let side2 = step(base, t2, s2);
            let corner = step(step(base, t1, s1), t2, s2);
            let (o1, o2, oc) = (opaque_at(side1), opaque_at(side2), opaque_at(corner));
            let occl = if o1 && o2 { 3.0 } else { (o1 as u8 + o2 as u8 + oc as u8) as f32 };
            aos[i] = 1.0 - 0.2 * occl;
            let (mut sky, mut blk) = (0u32, 0u32);
            for p in [base, side1, side2, corner] {
                if !opaque_at(p) {
                    let l = light_of(p.0, p.1, p.2);
                    sky += (l >> 4) & 15;
                    blk += l & 15;
                }
            }
            lights[i] = ((sky / 4) << 4) | (blk / 4);
        }
        (aos, lights)
    };

    for x in 0..SECTION_SIZE {
        for y in 0..SECTION_SIZE {
            for z in 0..SECTION_SIZE {
                let block = section.get(x, y, z);
                if block == BlockState::AIR {
                    continue;
                }

                let fx = x as f32;
                let fy = y as f32;
                let fz = z as f32;
                let fx1 = fx + 1.0;
                let fy1 = fy + 1.0;
                let fz1 = fz + 1.0;
                let cell = (x as i32, y as i32, z as i32);
                let sway = if registry::is_leaf(block.id()) { 1.0 } else { 0.0 };
                // Faces render when the neighbor does not hide them (air,
                // water, leaves). No faces between two water blocks.
                let face_visible = |nb: BlockState| {
                    if block.id() == crate::registry::block::WATER && nb.id() == crate::registry::block::WATER {
                        return false;
                    }
                    !crate::registry::is_opaque(nb)
                };

                // -X face
                if face_visible(get_block(x as i32 - 1, y as i32, z as i32)) {
                    let corners = [[fx, fy, fz], [fx, fy1, fz], [fx, fy1, fz1], [fx, fy, fz1]];
                    let (ao, light) = corner_shades(cell, [-1, 0, 0], &corners, [fx, fy, fz]);
                    push_face(&mut vertices, &mut indices, corners, UVS_A, [-1.0, 0.0, 0.0], tex_of(block, Face::Side), ao, light, sway);
                }
                // +X face
                if face_visible(get_block(x as i32 + 1, y as i32, z as i32)) {
                    let corners = [[fx1, fy, fz1], [fx1, fy1, fz1], [fx1, fy1, fz], [fx1, fy, fz]];
                    let (ao, light) = corner_shades(cell, [1, 0, 0], &corners, [fx, fy, fz]);
                    push_face(&mut vertices, &mut indices, corners, UVS_A, [1.0, 0.0, 0.0], tex_of(block, Face::Side), ao, light, sway);
                }
                // -Y face (corners wound so the outward normal points down)
                if face_visible(get_block(x as i32, y as i32 - 1, z as i32)) {
                    let corners = [[fx, fy, fz], [fx, fy, fz1], [fx1, fy, fz1], [fx1, fy, fz]];
                    let (ao, light) = corner_shades(cell, [0, -1, 0], &corners, [fx, fy, fz]);
                    push_face(&mut vertices, &mut indices, corners, UVS_B, [0.0, -1.0, 0.0], tex_of(block, Face::Bottom), ao, light, sway);
                }
                // +Y face
                if face_visible(get_block(x as i32, y as i32 + 1, z as i32)) {
                    let corners = [[fx, fy1, fz1], [fx, fy1, fz], [fx1, fy1, fz], [fx1, fy1, fz1]];
                    let (ao, light) = corner_shades(cell, [0, 1, 0], &corners, [fx, fy, fz]);
                    push_face(&mut vertices, &mut indices, corners, UVS_B, [0.0, 1.0, 0.0], tex_of(block, Face::Top), ao, light, sway);
                }
                // -Z face
                if face_visible(get_block(x as i32, y as i32, z as i32 - 1)) {
                    let corners = [[fx1, fy, fz], [fx1, fy1, fz], [fx, fy1, fz], [fx, fy, fz]];
                    let (ao, light) = corner_shades(cell, [0, 0, -1], &corners, [fx, fy, fz]);
                    push_face(&mut vertices, &mut indices, corners, UVS_B, [0.0, 0.0, -1.0], tex_of(block, Face::Side), ao, light, sway);
                }
                // +Z face
                if face_visible(get_block(x as i32, y as i32, z as i32 + 1)) {
                    let corners = [[fx, fy, fz1], [fx, fy1, fz1], [fx1, fy1, fz1], [fx1, fy, fz1]];
                    let (ao, light) = corner_shades(cell, [0, 0, 1], &corners, [fx, fy, fz]);
                    push_face(&mut vertices, &mut indices, corners, UVS_B, [0.0, 0.0, 1.0], tex_of(block, Face::Side), ao, light, sway);
                }
            }
        }
    }

    MeshData { vertices, indices }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tex(_b: BlockState, _f: Face) -> u32 { 0 }

    #[test]
    fn single_block_emits_six_faces() {
        let mut s = VoxelSection::new_empty();
        s.set(8, 8, 8, BlockState::STONE);
        let mesh = mesh_section(&s, None, None, None, None, None, None, &tex, &|_, _, _| 0xFF);
        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn buried_faces_are_culled() {
        let mut s = VoxelSection::new_empty();
        s.set(8, 8, 8, BlockState::STONE);
        s.set(9, 8, 8, BlockState::DIRT);
        let mesh = mesh_section(&s, None, None, None, None, None, None, &tex, &|_, _, _| 0xFF);
        // 10 exposed faces instead of 12
        assert_eq!(mesh.vertices.len(), 40);
    }

    #[test]
    fn every_triangle_faces_outward() {
        // Guards the face winding: with back-face culling, an inward-wound
        // face is invisible from outside (the see-through-terrain bug).
        let mut s = VoxelSection::new_empty();
        s.set(8, 8, 8, BlockState::STONE);
        s.set(4, 4, 4, BlockState(6)); // second block, different corner
        let mesh = mesh_section(&s, None, None, None, None, None, None, &tex, &|_, _, _| 0xFF);
        let centers = [[8.5f32, 8.5, 8.5], [4.5f32, 4.5, 4.5]];
        let mut checked = 0;
        for center in centers {
            let mut tris = 0;
            for tri in mesh.indices.chunks(3) {
                let p: Vec<[f32; 3]> = tri.iter().map(|i| mesh.vertices[*i as usize].position).collect();
                let belongs = p.iter().all(|v| {
                    (v[0] - center[0]).abs() <= 0.5 + 1e-6
                        && (v[1] - center[1]).abs() <= 0.5 + 1e-6
                        && (v[2] - center[2]).abs() <= 0.5 + 1e-6
                });
                if !belongs {
                    continue;
                }
                let u = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
                let v = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
                let n = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                let centroid = [
                    (p[0][0] + p[1][0] + p[2][0]) / 3.0 - center[0],
                    (p[0][1] + p[1][1] + p[2][1]) / 3.0 - center[1],
                    (p[0][2] + p[1][2] + p[2][2]) / 3.0 - center[2],
                ];
                let dot = n[0] * centroid[0] + n[1] * centroid[1] + n[2] * centroid[2];
                assert!(dot > 0.0, "inward-wound triangle at {:?}: n={:?} centroid={:?}", p, n, centroid);
                tris += 1;
            }
            assert_eq!(tris, 12, "block at {:?} should emit 12 triangles", center);
            checked += tris;
        }
        assert_eq!(checked, 24);
    }

    #[test]
    fn tex_index_is_per_block() {
        let mut s = VoxelSection::new_empty();
        s.set(8, 8, 8, BlockState::STONE);
        s.set(8, 8, 9, BlockState(6)); // snow
        let mesh = mesh_section(&s, None, None, None, None, None, None, &|b, _| (b.0 * 2) as u32, &|_, _, _| 0xFF);
        let indices: std::collections::HashSet<u32> = mesh.vertices.iter().map(|v| v.tex_index).collect();
        assert!(indices.contains(&2) && indices.contains(&12), "got {:?}", indices);
    }

    #[test]
    fn per_face_textures_land_on_the_right_faces() {
        let mut s = VoxelSection::new_empty();
        s.set(8, 8, 8, BlockState(2)); // grass
        let mesh = mesh_section(&s, None, None, None, None, None, None,
            &|_b, f| match f { Face::Top => 100, Face::Bottom => 101, Face::Side => 102 },
            &|_, _, _| 0xFF);
        for v in &mesh.vertices {
            let expected = if v.normal[1] > 0.5 {
                100 // top
            } else if v.normal[1] < -0.5 {
                101 // bottom
            } else {
                102 // side
            };
            assert_eq!(v.tex_index, expected, "normal {:?} got tex {}", v.normal, v.tex_index);
        }
    }

    #[test]
    fn corner_occlusion_darkens_ao() {
        let mut s = VoxelSection::new_empty();
        s.set(8, 8, 8, BlockState::STONE);
        // occluder diagonal above, beside the x=7 edge of the top face
        s.set(7, 9, 8, BlockState::STONE);
        let mesh = mesh_section(&s, None, None, None, None, None, None, &tex, &|_, _, _| 0xFF);
        // only the lower block's top face sits at y=9 (the occluder's is at y=10)
        let top: Vec<&Vertex> = mesh.vertices.iter()
            .filter(|v| v.normal[1] > 0.5 && v.position[1] == 9.0).collect();
        assert_eq!(top.len(), 4);
        // the two corners whose touching cells include the occluder darken to 0.8
        for v in &top {
            let expected = if v.position[0] == 8.0 { 0.8 } else { 1.0 };
            assert!((v.ao - expected).abs() < 1e-6, "corner {:?} ao {} expected {}", v.position, v.ao, expected);
        }
    }

    #[test]
    fn smooth_light_averages_corner_cells() {
        let mut s = VoxelSection::new_empty();
        s.set(8, 8, 8, BlockState::STONE);
        // sky light triples with x. The four cells touching a top-face corner
        // are the 2x2 block-adjacent ones: corner x=8 averages cells {7,8}
        // -> (0+3)*2/4 = 1; corner x=9 averages cells {8,9} -> (3+6)*2/4 = 4.
        let mesh = mesh_section(&s, None, None, None, None, None, None, &tex,
            &|x, _y, _z| (((x - 7) * 3).clamp(0, 15) as u32) << 4);
        let sky_at = |pos: [f32; 3]| -> u32 {
            mesh.vertices.iter().find(|v| v.normal[1] > 0.5 && v.position == pos)
                .map(|v| (v.light >> 4) & 15)
                .expect("top-face corner vertex")
        };
        assert_eq!(sky_at([8.0, 9.0, 8.0]), 1, "cells x in {{7,8}}: (0+3+0+3)/4");
        assert_eq!(sky_at([9.0, 9.0, 9.0]), 4, "cells x in {{8,9}}: (3+6+3+6)/4");
    }

    #[test]
    fn leaves_sway_stone_does_not() {
        let mut s = VoxelSection::new_empty();
        s.set(8, 8, 8, BlockState(8)); // leaves
        let leaf_mesh = mesh_section(&s, None, None, None, None, None, None, &tex, &|_, _, _| 0xFF);
        assert!(leaf_mesh.vertices.iter().all(|v| v.sway == 1.0), "leaf vertices sway");
        let mut s2 = VoxelSection::new_empty();
        s2.set(8, 8, 8, BlockState::STONE);
        let stone_mesh = mesh_section(&s2, None, None, None, None, None, None, &tex, &|_, _, _| 0xFF);
        assert!(stone_mesh.vertices.iter().all(|v| v.sway == 0.0), "stone vertices do not");
    }
}
