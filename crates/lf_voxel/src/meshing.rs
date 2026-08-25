use crate::{VoxelSection, BlockState, SECTION_SIZE};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
    pub ao: f32,
    pub light: u32,
    pub tex_index: u32,
}

pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

/// Simple culled face meshing for a voxel section. `tex_of` maps each block
/// state to a texture atlas layer index.
pub fn mesh_section(section: &VoxelSection, neighbor_px: Option<&VoxelSection>, neighbor_nx: Option<&VoxelSection>,
                     neighbor_py: Option<&VoxelSection>, neighbor_ny: Option<&VoxelSection>,
                     neighbor_pz: Option<&VoxelSection>, neighbor_nz: Option<&VoxelSection>,
                     tex_of: &dyn Fn(BlockState) -> u32) -> MeshData {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let get_block = |x: i32, y: i32, z: i32| -> BlockState {
        if x >= 0 && x < 16 && y >= 0 && y < 16 && z >= 0 && z < 16 {
            section.get(x as usize, y as usize, z as usize)
        } else {
            // Check neighbors if available, else AIR
            if x < 0 { neighbor_nx.map_or(BlockState::AIR, |n| n.get((x + 16) as usize, y as usize, z as usize)) }
            else if x >= 16 { neighbor_px.map_or(BlockState::AIR, |n| n.get((x - 16) as usize, y as usize, z as usize)) }
            else if y < 0 { neighbor_ny.map_or(BlockState::AIR, |n| n.get(x as usize, (y + 16) as usize, z as usize)) }
            else if y >= 16 { neighbor_py.map_or(BlockState::AIR, |n| n.get(x as usize, (y - 16) as usize, z as usize)) }
            else if z < 0 { neighbor_nz.map_or(BlockState::AIR, |n| n.get(x as usize, y as usize, (z + 16) as usize)) }
            else if z >= 16 { neighbor_pz.map_or(BlockState::AIR, |n| n.get(x as usize, y as usize, (z - 16) as usize)) }
            else { BlockState::AIR }
        }
    };

    // UV patterns: X faces and Y/Z faces use different corner orientations.
    const UVS_A: [[f32; 2]; 4] = [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
    const UVS_B: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

    /// Emit one axis-aligned quad with CCW winding seen from outside.
    fn push_face(vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                 corners: [[f32; 3]; 4], uvs: [[f32; 2]; 4], normal: [f32; 3], tex_index: u32) {
        let base_idx = vertices.len() as u32;
        for (corner, uv) in corners.iter().zip(uvs.iter()) {
            vertices.push(Vertex {
                position: *corner,
                normal,
                tex_coord: *uv,
                ao: 1.0,
                light: 15,
                tex_index,
            });
        }
        indices.extend_from_slice(&[base_idx, base_idx + 2, base_idx + 1, base_idx, base_idx + 3, base_idx + 2]);
    }

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
                let ti = tex_of(block);

                // -X face
                if get_block(x as i32 - 1, y as i32, z as i32) == BlockState::AIR {
                    push_face(&mut vertices, &mut indices,
                        [[fx, fy, fz], [fx, fy1, fz], [fx, fy1, fz1], [fx, fy, fz1]], UVS_A, [-1.0, 0.0, 0.0], ti);
                }
                // +X face
                if get_block(x as i32 + 1, y as i32, z as i32) == BlockState::AIR {
                    push_face(&mut vertices, &mut indices,
                        [[fx1, fy, fz1], [fx1, fy1, fz1], [fx1, fy1, fz], [fx1, fy, fz]], UVS_A, [1.0, 0.0, 0.0], ti);
                }
                // -Y face
                if get_block(x as i32, y as i32 - 1, z as i32) == BlockState::AIR {
                    push_face(&mut vertices, &mut indices,
                        [[fx, fy, fz], [fx1, fy, fz], [fx1, fy, fz1], [fx, fy, fz1]], UVS_B, [0.0, -1.0, 0.0], ti);
                }
                // +Y face
                if get_block(x as i32, y as i32 + 1, z as i32) == BlockState::AIR {
                    push_face(&mut vertices, &mut indices,
                        [[fx, fy1, fz1], [fx1, fy1, fz1], [fx1, fy1, fz], [fx, fy1, fz]], UVS_B, [0.0, 1.0, 0.0], ti);
                }
                // -Z face
                if get_block(x as i32, y as i32, z as i32 - 1) == BlockState::AIR {
                    push_face(&mut vertices, &mut indices,
                        [[fx1, fy, fz], [fx, fy, fz], [fx, fy1, fz], [fx1, fy1, fz]], UVS_B, [0.0, 0.0, -1.0], ti);
                }
                // +Z face
                if get_block(x as i32, y as i32, z as i32 + 1) == BlockState::AIR {
                    push_face(&mut vertices, &mut indices,
                        [[fx, fy, fz1], [fx1, fy, fz1], [fx1, fy1, fz1], [fx, fy1, fz1]], UVS_B, [0.0, 0.0, 1.0], ti);
                }
            }
        }
    }

    MeshData { vertices, indices }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tex(_b: BlockState) -> u32 { 0 }

    #[test]
    fn single_block_emits_six_faces() {
        let mut s = VoxelSection::new_empty();
        s.set(8, 8, 8, BlockState::STONE);
        let mesh = mesh_section(&s, None, None, None, None, None, None, &tex);
        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn buried_faces_are_culled() {
        let mut s = VoxelSection::new_empty();
        s.set(8, 8, 8, BlockState::STONE);
        s.set(9, 8, 8, BlockState::DIRT);
        let mesh = mesh_section(&s, None, None, None, None, None, None, &tex);
        // 10 exposed faces instead of 12
        assert_eq!(mesh.vertices.len(), 40);
    }

    #[test]
    fn tex_index_is_per_block() {
        let mut s = VoxelSection::new_empty();
        s.set(8, 8, 8, BlockState::STONE);
        s.set(8, 8, 9, BlockState(6)); // snow
        let mesh = mesh_section(&s, None, None, None, None, None, None, &|b| (b.0 * 2) as u32);
        let indices: std::collections::HashSet<u32> = mesh.vertices.iter().map(|v| v.tex_index).collect();
        assert!(indices.contains(&2) && indices.contains(&12), "got {:?}", indices);
    }
}
