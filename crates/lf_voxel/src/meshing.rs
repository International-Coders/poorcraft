use crate::{VoxelSection, BlockState, SECTION_SIZE};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
    pub ao: f32,
    pub light: u32,
}

pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

/// Simple culled face meshing for a voxel section.
pub fn mesh_section(section: &VoxelSection, neighbor_px: Option<&VoxelSection>, neighbor_nx: Option<&VoxelSection>,
                     neighbor_py: Option<&VoxelSection>, neighbor_ny: Option<&VoxelSection>,
                     neighbor_pz: Option<&VoxelSection>, neighbor_nz: Option<&VoxelSection>) -> MeshData {
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

                // -X face
                if get_block(x as i32 - 1, y as i32, z as i32) == BlockState::AIR {
                    let base_idx = vertices.len() as u32;
                    vertices.push(Vertex { position: [fx, fy, fz], normal: [-1.0, 0.0, 0.0], tex_coord: [0.0, 1.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx, fy + 1.0, fz], normal: [-1.0, 0.0, 0.0], tex_coord: [0.0, 0.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx, fy + 1.0, fz + 1.0], normal: [-1.0, 0.0, 0.0], tex_coord: [1.0, 0.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx, fy, fz + 1.0], normal: [-1.0, 0.0, 0.0], tex_coord: [1.0, 1.0], ao: 1.0, light: 15 });
                    indices.extend_from_slice(&[base_idx, base_idx + 2, base_idx + 1, base_idx, base_idx + 3, base_idx + 2]);
                }
                // +X face
                if get_block(x as i32 + 1, y as i32, z as i32) == BlockState::AIR {
                    let base_idx = vertices.len() as u32;
                    let fx1 = fx + 1.0;
                    vertices.push(Vertex { position: [fx1, fy, fz + 1.0], normal: [1.0, 0.0, 0.0], tex_coord: [0.0, 1.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx1, fy + 1.0, fz + 1.0], normal: [1.0, 0.0, 0.0], tex_coord: [0.0, 0.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx1, fy + 1.0, fz], normal: [1.0, 0.0, 0.0], tex_coord: [1.0, 0.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx1, fy, fz], normal: [1.0, 0.0, 0.0], tex_coord: [1.0, 1.0], ao: 1.0, light: 15 });
                    indices.extend_from_slice(&[base_idx, base_idx + 2, base_idx + 1, base_idx, base_idx + 3, base_idx + 2]);
                }
                // -Y face
                if get_block(x as i32, y as i32 - 1, z as i32) == BlockState::AIR {
                    let base_idx = vertices.len() as u32;
                    vertices.push(Vertex { position: [fx, fy, fz], normal: [0.0, -1.0, 0.0], tex_coord: [0.0, 1.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx + 1.0, fy, fz], normal: [0.0, -1.0, 0.0], tex_coord: [1.0, 1.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx + 1.0, fy, fz + 1.0], normal: [0.0, -1.0, 0.0], tex_coord: [1.0, 0.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx, fy, fz + 1.0], normal: [0.0, -1.0, 0.0], tex_coord: [0.0, 0.0], ao: 1.0, light: 15 });
                    indices.extend_from_slice(&[base_idx, base_idx + 2, base_idx + 1, base_idx, base_idx + 3, base_idx + 2]);
                }
                // +Y face
                if get_block(x as i32, y as i32 + 1, z as i32) == BlockState::AIR {
                    let base_idx = vertices.len() as u32;
                    let fy1 = fy + 1.0;
                    vertices.push(Vertex { position: [fx, fy1, fz + 1.0], normal: [0.0, 1.0, 0.0], tex_coord: [0.0, 1.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx + 1.0, fy1, fz + 1.0], normal: [0.0, 1.0, 0.0], tex_coord: [1.0, 1.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx + 1.0, fy1, fz], normal: [0.0, 1.0, 0.0], tex_coord: [1.0, 0.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx, fy1, fz], normal: [0.0, 1.0, 0.0], tex_coord: [0.0, 0.0], ao: 1.0, light: 15 });
                    indices.extend_from_slice(&[base_idx, base_idx + 2, base_idx + 1, base_idx, base_idx + 3, base_idx + 2]);
                }
                // -Z face
                if get_block(x as i32, y as i32, z as i32 - 1) == BlockState::AIR {
                    let base_idx = vertices.len() as u32;
                    vertices.push(Vertex { position: [fx + 1.0, fy, fz], normal: [0.0, 0.0, -1.0], tex_coord: [0.0, 1.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx, fy, fz], normal: [0.0, 0.0, -1.0], tex_coord: [1.0, 1.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx, fy + 1.0, fz], normal: [0.0, 0.0, -1.0], tex_coord: [1.0, 0.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx + 1.0, fy + 1.0, fz], normal: [0.0, 0.0, -1.0], tex_coord: [0.0, 0.0], ao: 1.0, light: 15 });
                    indices.extend_from_slice(&[base_idx, base_idx + 2, base_idx + 1, base_idx, base_idx + 3, base_idx + 2]);
                }
                // +Z face
                if get_block(x as i32, y as i32, z as i32 + 1) == BlockState::AIR {
                    let base_idx = vertices.len() as u32;
                    let fz1 = fz + 1.0;
                    vertices.push(Vertex { position: [fx, fy, fz1], normal: [0.0, 0.0, 1.0], tex_coord: [0.0, 1.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx + 1.0, fy, fz1], normal: [0.0, 0.0, 1.0], tex_coord: [1.0, 1.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx + 1.0, fy + 1.0, fz1], normal: [0.0, 0.0, 1.0], tex_coord: [1.0, 0.0], ao: 1.0, light: 15 });
                    vertices.push(Vertex { position: [fx, fy + 1.0, fz1], normal: [0.0, 0.0, 1.0], tex_coord: [0.0, 0.0], ao: 1.0, light: 15 });
                    indices.extend_from_slice(&[base_idx, base_idx + 2, base_idx + 1, base_idx, base_idx + 3, base_idx + 2]);
                }
            }
        }
    }

    MeshData { vertices, indices }
}
