use crate::{VoxelSection, BlockState, SECTION_SIZE};
use crate::registry;

/// Which face of a block a texture is selected for (per-face materials:
/// grass top/side/bottom, log rings on the ends, ...).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Face {
    Top,
    Bottom,
    /// Any of the four lateral faces (shape meshes and water use this —
    /// neither is ever direction-dependent).
    Side,
    /// Directional lateral faces (loop 330): only horizontal logs care —
    /// their ring-end faces are the pair along their axis.
    West,
    East,
    North,
    South,
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

/// Surface height (0..1 within the cell) of a water block by flow level:
/// sources fill the cell, each level step sinks the surface by 1/8.
pub fn water_surface_height(block: BlockState) -> f32 {
    1.0 - (crate::water_level(block) as f32) * 0.125
}

/// Simple culled face meshing for a voxel section. `tex_of` maps each block
/// state + face to a texture atlas layer index (per-face materials); light is
/// smoothed per vertex by averaging the four cells around each corner, and
/// ambient occlusion darkens corners occluded by neighboring blocks.
/// Step 11: connected-variant table (mirrors lf_assets::connected_
/// variant — lf_voxel cannot depend on lf_assets).
fn lf_assets_conn(layer: u32) -> Option<u32> {
    match layer {
        0 => Some(83),  // stone_conn
        15 => Some(84), // planks_conn
        _ => None,
    }
}

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

    /// Step 11 (connected surfaces): when the block a face is drawn
    /// against is the SAME block (full cube), sample the edgeless
    /// variant so large stone/plank surfaces read as continuous.
    let conn_tex = |block: BlockState, nb: BlockState, face_tex: u32| -> u32 {
        if nb.id() == block.id()
            && nb.shape() == crate::Shape::Cube
            && block.id() != crate::registry::block::WATER
        {
            if let Some(conn) = lf_assets_conn(face_tex) {
                return conn;
            }
        }
        face_tex
    };

    /// P34 shapes: emit a slab or stair as 1-2 boxes with exactly the
    /// exterior faces (no coincident interior quads). Faces flush against
    /// an opaque full-cube neighbor are culled. AO/light reuse
    /// corner_shades, so shaped blocks blend with the smoothed lighting.
    let push_shaped = |vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                       cell: (i32, i32, i32), fx: f32, fy: f32, fz: f32,
                       block: BlockState, shape: crate::Shape,
                       get_block: &dyn Fn(i32, i32, i32) -> BlockState| {
        let opaque_cube = |p: (i32, i32, i32)| {
            let nb = get_block(p.0, p.1, p.2);
            registry::is_opaque(nb) && nb.shape() == crate::Shape::Cube
        };
        let side_tex = tex_of(block, Face::Side);
        let top_tex = tex_of(block, Face::Top);
        let bot_tex = tex_of(block, Face::Bottom);
        // one quad: corners CCW from outside, with AO/light
        let mut quad = |corners: [[f32; 3]; 4], normal: [f32; 3], tex: u32, uvs: &[[f32; 2]; 4]| {
            let (ao, light) = corner_shades(cell, [normal[0] as i32, normal[1] as i32, normal[2] as i32], &corners, [fx, fy, fz]);
            push_face(vertices, indices, corners, *uvs, normal, tex, ao, light, 0.0);
        };
        match shape {
            crate::Shape::Cube => unreachable!(),
            crate::Shape::SlabBottom | crate::Shape::SlabTop => {
                let y0 = if shape == crate::Shape::SlabTop { fy + 0.5 } else { fy };
                let y1 = if shape == crate::Shape::SlabTop { fy + 1.0 } else { fy + 0.5 };
                let below = opaque_cube((cell.0, cell.1 - 1, cell.2));
                let above = opaque_cube((cell.0, cell.1 + 1, cell.2));
                let cull = [opaque_cube((cell.0 - 1, cell.1, cell.2)), opaque_cube((cell.0 + 1, cell.1, cell.2)),
                            opaque_cube((cell.0, cell.1, cell.2 - 1)), opaque_cube((cell.0, cell.1, cell.2 + 1))];
                if !cull[0] { quad([[fx, y0, fz], [fx, y1, fz], [fx, y1, fz + 1.0], [fx, y0, fz + 1.0]], [-1.0, 0.0, 0.0], side_tex, &UVS_A); }
                if !cull[1] { quad([[fx + 1.0, y0, fz + 1.0], [fx + 1.0, y1, fz + 1.0], [fx + 1.0, y1, fz], [fx + 1.0, y0, fz]], [1.0, 0.0, 0.0], side_tex, &UVS_A); }
                if !cull[2] { quad([[fx + 1.0, y0, fz], [fx + 1.0, y1, fz], [fx, y1, fz], [fx, y0, fz]], [0.0, 0.0, -1.0], side_tex, &UVS_B); }
                if !cull[3] { quad([[fx, y0, fz + 1.0], [fx, y1, fz + 1.0], [fx + 1.0, y1, fz + 1.0], [fx + 1.0, y0, fz + 1.0]], [0.0, 0.0, 1.0], side_tex, &UVS_B); }
                if !below { quad([[fx, y0, fz], [fx, y0, fz + 1.0], [fx + 1.0, y0, fz + 1.0], [fx + 1.0, y0, fz]], [0.0, -1.0, 0.0], bot_tex, &UVS_A); }
                if !above { quad([[fx, y1, fz + 1.0], [fx, y1, fz], [fx + 1.0, y1, fz], [fx + 1.0, y1, fz + 1.0]], [0.0, 1.0, 0.0], top_tex, &UVS_A); }
            }
            crate::Shape::StairNorth | crate::Shape::StairSouth
            | crate::Shape::StairWest | crate::Shape::StairEast => {
                // high half span on the walking axis
                let (hx0, hx1, hz0, hz1) = match shape {
                    crate::Shape::StairNorth => (fx, fx + 1.0, fz, fz + 0.5),
                    crate::Shape::StairSouth => (fx, fx + 1.0, fz + 0.5, fz + 1.0),
                    crate::Shape::StairWest => (fx, fx + 0.5, fz, fz + 1.0),
                    _ => (fx + 0.5, fx + 1.0, fz, fz + 1.0),
                };
                // bottom slab (full footprint, y 0..0.5); its top face only
                // on the LOW half so it never coincides with the high box
                let below = opaque_cube((cell.0, cell.1 - 1, cell.2));
                let cull = [opaque_cube((cell.0 - 1, cell.1, cell.2)), opaque_cube((cell.0 + 1, cell.1, cell.2)),
                            opaque_cube((cell.0, cell.1, cell.2 - 1)), opaque_cube((cell.0, cell.1, cell.2 + 1))];
                // slab sides at half height
                if !cull[0] { quad([[fx, fy, fz], [fx, fy + 0.5, fz], [fx, fy + 0.5, fz + 1.0], [fx, fy, fz + 1.0]], [-1.0, 0.0, 0.0], side_tex, &UVS_A); }
                if !cull[1] { quad([[fx + 1.0, fy, fz + 1.0], [fx + 1.0, fy + 0.5, fz + 1.0], [fx + 1.0, fy + 0.5, fz], [fx + 1.0, fy, fz]], [1.0, 0.0, 0.0], side_tex, &UVS_A); }
                if !cull[2] { quad([[fx + 1.0, fy, fz], [fx + 1.0, fy + 0.5, fz], [fx, fy + 0.5, fz], [fx, fy, fz]], [0.0, 0.0, -1.0], side_tex, &UVS_B); }
                if !cull[3] { quad([[fx, fy, fz + 1.0], [fx, fy + 0.5, fz + 1.0], [fx + 1.0, fy + 0.5, fz + 1.0], [fx + 1.0, fy, fz + 1.0]], [0.0, 0.0, 1.0], side_tex, &UVS_B); }
                if !below { quad([[fx, fy, fz], [fx, fy, fz + 1.0], [fx + 1.0, fy, fz + 1.0], [fx + 1.0, fy, fz]], [0.0, -1.0, 0.0], bot_tex, &UVS_A); }
                // low-half top strip (the step you walk onto)
                let (lx0, lx1, lz0, lz1) = (hx0, hx1, hz0, hz1);
                let (tx0, tx1, tz0, tz1) = if hx0 == fx && hx1 == fx + 1.0 {
                    (fx, fx + 1.0, if hz0 == fz { fz + 0.5 } else { fz }, if hz0 == fz { fz + 1.0 } else { fz + 0.5 })
                } else {
                    (if hx0 == fx { fx + 0.5 } else { fx }, if hx0 == fx { fx + 1.0 } else { fx + 0.5 }, fz, fz + 1.0)
                };
                quad([[tx0, fy + 0.5, tz1], [tx0, fy + 0.5, tz0], [tx1, fy + 0.5, tz0], [tx1, fy + 0.5, tz1]], [0.0, 1.0, 0.0], top_tex, &UVS_A);
                // high box (y 0.5..1) on the back half: 4 sides + top; no
                // bottom (it sits on the slab)
                let above = opaque_cube((cell.0, cell.1 + 1, cell.2));
                if !cull[0] { quad([[hx0, fy + 0.5, hz0], [hx0, fy + 1.0, hz0], [hx0, fy + 1.0, hz1], [hx0, fy + 0.5, hz1]], [-1.0, 0.0, 0.0], side_tex, &UVS_A); }
                if !cull[1] { quad([[hx1, fy + 0.5, hz1], [hx1, fy + 1.0, hz1], [hx1, fy + 1.0, hz0], [hx1, fy + 0.5, hz0]], [1.0, 0.0, 0.0], side_tex, &UVS_A); }
                if !cull[2] { quad([[hx1, fy + 0.5, hz0], [hx1, fy + 1.0, hz0], [hx0, fy + 1.0, hz0], [hx0, fy + 0.5, hz0]], [0.0, 0.0, -1.0], side_tex, &UVS_B); }
                if !cull[3] { quad([[hx0, fy + 0.5, hz1], [hx0, fy + 1.0, hz1], [hx1, fy + 1.0, hz1], [hx1, fy + 0.5, hz1]], [0.0, 0.0, 1.0], side_tex, &UVS_B); }
                if !above { quad([[hx0, fy + 1.0, hz1], [hx0, fy + 1.0, hz0], [hx1, fy + 1.0, hz0], [hx1, fy + 1.0, hz1]], [0.0, 1.0, 0.0], top_tex, &UVS_A); }
                let _ = (lx0, lx1, lz0, lz1);
            }
        }
    };

    for x in 0..SECTION_SIZE {
        for y in 0..SECTION_SIZE {
            for z in 0..SECTION_SIZE {
                let block = section.get(x, y, z);
                if block == BlockState::AIR {
                    continue;
                }
                // Loop 331: ground plants render Minecraft-style — two
                // diagonal cutout quads (each emitted twice so the backface
                // cull keeps them visible from both sides), lit by their own
                // cell, with the foliage wind sway. No cube faces at all.
                if registry::is_plant(block.id()) && !registry::is_banner(block.id()) {
                    let l = light_of(x as i32, y as i32, z as i32);
                    let light = [l, l, l, l];
                    let ao = [1.0f32; 4];
                    let tex = tex_of(block, Face::Side);
                    let (x0, z0) = (x as f32 + 0.146, z as f32 + 0.146);
                    let (x1, z1) = (x as f32 + 0.854, z as f32 + 0.854);
                    let mut quad = |vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>,
                                    corners: [[f32; 3]; 4], normal: [f32; 3]| {
                        push_face(vertices, indices, corners, UVS_A, normal, tex, ao, light, 1.0);
                    };
                    // diagonal A: (-z..+x), front and back
                    let (fy0, fy1) = (y as f32, y as f32 + 1.0);
                    let a = [[x0, fy0, z0], [x0, fy1, z0], [x1, fy1, z1], [x1, fy0, z1]];
                    quad(&mut vertices, &mut indices, a, [0.0, 0.0, -1.0]);
                    let a_rev = [a[3], a[2], a[1], a[0]];
                    quad(&mut vertices, &mut indices, a_rev, [0.0, 0.0, 1.0]);
                    // diagonal B: (+x..+z), front and back
                    let b = [[x1, fy0, z0], [x1, fy1, z0], [x0, fy1, z1], [x0, fy0, z1]];
                    quad(&mut vertices, &mut indices, b, [1.0, 0.0, 0.0]);
                    let b_rev = [b[3], b[2], b[1], b[0]];
                    quad(&mut vertices, &mut indices, b_rev, [-1.0, 0.0, 0.0]);
                    continue;
                }

                // P34: shaped blocks take their own emission path; plain
                // cubes keep the original code untouched below
                let shape = block.shape();
                if shape != crate::Shape::Cube {
                    let cell = (x as i32, y as i32, z as i32);
                    let get = |gx: i32, gy: i32, gz: i32| get_block(gx, gy, gz);
                    push_shaped(&mut vertices, &mut indices, cell,
                        x as f32, y as f32, z as f32, block, shape, &get);
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
                // Flowing water renders as a shortened column: the surface
                // sinks with the flow level (source = full). Side faces
                // against a taller water neighbor are drawn at full height
                // so the level step does not show a slit.
                let is_water = block.id() == crate::registry::block::WATER;
                let wy1 = if is_water { fy + water_surface_height(block) } else { fy1 };
                let my_level = crate::water_level(block);
                // Faces render when the neighbor does not hide them (air,
                // water, leaves). No faces between two water blocks of
                // compatible heights.
                let face_visible = |nb: BlockState| {
                    if is_water && nb.id() == crate::registry::block::WATER {
                        // hidden only when the neighbor is at least as tall
                        return crate::water_level(nb) > my_level;
                    }
                    !crate::registry::is_opaque(nb)
                };
                // Side-face top for water: full height when the neighbor is
                // taller water (covers the step), else the surface height.
                let side_top = |nb: BlockState| -> f32 {
                    if is_water && nb.id() == crate::registry::block::WATER && crate::water_level(nb) <= my_level {
                        fy1
                    } else {
                        wy1
                    }
                };

                // -X face
                {
                    let nb = get_block(x as i32 - 1, y as i32, z as i32);
                    if face_visible(nb) {
                        let t = side_top(nb);
                        let corners = [[fx, fy, fz], [fx, t, fz], [fx, t, fz1], [fx, fy, fz1]];
                        let (ao, light) = corner_shades(cell, [-1, 0, 0], &corners, [fx, fy, fz]);
                        push_face(&mut vertices, &mut indices, corners, UVS_A, [-1.0, 0.0, 0.0], conn_tex(block, nb, tex_of(block, Face::West)), ao, light, sway);
                    }
                }
                // +X face
                {
                    let nb = get_block(x as i32 + 1, y as i32, z as i32);
                    if face_visible(nb) {
                        let t = side_top(nb);
                        let corners = [[fx1, fy, fz1], [fx1, t, fz1], [fx1, t, fz], [fx1, fy, fz]];
                        let (ao, light) = corner_shades(cell, [1, 0, 0], &corners, [fx, fy, fz]);
                        push_face(&mut vertices, &mut indices, corners, UVS_A, [1.0, 0.0, 0.0], conn_tex(block, nb, tex_of(block, Face::East)), ao, light, sway);
                    }
                }
                // -Y face (corners wound so the outward normal points down)
                if face_visible(get_block(x as i32, y as i32 - 1, z as i32)) {
                    let nb = get_block(x as i32, y as i32 - 1, z as i32);
                    let corners = [[fx, fy, fz], [fx, fy, fz1], [fx1, fy, fz1], [fx1, fy, fz]];
                    let (ao, light) = corner_shades(cell, [0, -1, 0], &corners, [fx, fy, fz]);
                    push_face(&mut vertices, &mut indices, corners, UVS_B, [0.0, -1.0, 0.0], conn_tex(block, nb, tex_of(block, Face::Bottom)), ao, light, sway);
                }
                // +Y face
                if face_visible(get_block(x as i32, y as i32 + 1, z as i32)) {
                    let nb = get_block(x as i32, y as i32 + 1, z as i32);
                    let corners = [[fx, wy1, fz1], [fx, wy1, fz], [fx1, wy1, fz], [fx1, wy1, fz1]];
                    let (ao, light) = corner_shades(cell, [0, 1, 0], &corners, [fx, fy, fz]);
                    push_face(&mut vertices, &mut indices, corners, UVS_B, [0.0, 1.0, 0.0], conn_tex(block, nb, tex_of(block, Face::Top)), ao, light, sway);
                }
                // -Z face
                {
                    let nb = get_block(x as i32, y as i32, z as i32 - 1);
                    if face_visible(nb) {
                        let t = side_top(nb);
                        let corners = [[fx1, fy, fz], [fx1, t, fz], [fx, t, fz], [fx, fy, fz]];
                        let (ao, light) = corner_shades(cell, [0, 0, -1], &corners, [fx, fy, fz]);
                        push_face(&mut vertices, &mut indices, corners, UVS_B, [0.0, 0.0, -1.0], conn_tex(block, nb, tex_of(block, Face::North)), ao, light, sway);
                    }
                }
                // +Z face
                {
                    let nb = get_block(x as i32, y as i32, z as i32 + 1);
                    if face_visible(nb) {
                        let t = side_top(nb);
                        let corners = [[fx, fy, fz1], [fx, t, fz1], [fx1, t, fz1], [fx1, fy, fz1]];
                        let (ao, light) = corner_shades(cell, [0, 0, 1], &corners, [fx, fy, fz]);
                        push_face(&mut vertices, &mut indices, corners, UVS_B, [0.0, 0.0, 1.0], conn_tex(block, nb, tex_of(block, Face::South)), ao, light, sway);
                    }
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
            &|_b, f| match f {
                Face::Top => 100, Face::Bottom => 101, _ => 102,
            },
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

    /// Goal Section 1: a multi-block surface must tile its texture per
    /// block — each block emits its own quad with UVs spanning exactly
    /// 0..1 (a seam vertex between blocks), never one stretched quad.
    /// This is the invariant any future greedy meshing must preserve by
    /// scaling UVs to the merged span and switching to repeat addressing.
    #[test]
    fn multi_block_walls_tile_per_block_not_stretched() {
        let mut s = VoxelSection::new_empty();
        s.set(7, 8, 8, BlockState(crate::registry::block::PLANKS));
        s.set(8, 8, 8, BlockState(crate::registry::block::PLANKS));
        let mesh = mesh_section(&s, None, None, None, None, None, None, &tex, &|_, _, _| 0xFF);
        let front: Vec<&Vertex> = mesh.vertices.iter()
            .filter(|v| v.normal[2] > 0.5)
            .collect();
        assert_eq!(front.len(), 8, "two blocks = two quads = 8 verts (a merged quad would be 4)");
        assert!(front.iter().any(|v| (v.position[0] - 9.0).abs() < 1e-4),
            "a seam vertex at the block boundary keeps the quads per-block");
        for v in &front {
            for u in [v.tex_coord[0], v.tex_coord[1]] {
                assert!(u.abs() < 1e-4 || (u - 1.0).abs() < 1e-4,
                    "per-block quads span UVs exactly 0..1, got {}", u);
            }
        }
    }

    #[test]
    fn flowing_water_renders_lower_than_sources() {
        let mut s = VoxelSection::new_empty();
        // floor under both cells so only the water surfaces matter
        s.set(7, 7, 8, BlockState::STONE);
        s.set(8, 7, 8, BlockState::STONE);
        s.set(7, 8, 8, crate::water_with_level(0)); // source: full cell
        s.set(8, 8, 8, crate::water_with_level(4)); // flowing: lowered
        let mesh = mesh_section(&s, None, None, None, None, None, None, &tex, &|_, _, _| 0xFF);
        let has_top_at = |y: f32, x: f32| -> bool {
            mesh.vertices.iter()
                .any(|v| v.normal[1] > 0.5 && v.position[0] == x && (v.position[1] - y).abs() < 1e-3)
        };
        assert!(has_top_at(9.0, 7.0), "source surface fills its cell (y=9)");
        assert!(has_top_at(8.5, 9.0), "level-4 surface sits at 1-4/8 of the cell (y=8.5)");
        assert!(!has_top_at(9.0, 9.0), "the flowing cell must NOT have a full-height surface");
    }

    /// P34: a bottom slab renders a half-height box — its top face sits at
    /// y+0.5 and every triangle still faces outward.
    #[test]
    fn slab_renders_a_half_box() {
        use crate::Shape;
        let mut s = VoxelSection::new_empty();
        s.set(8, 8, 8, BlockState::STONE.with_shape(Shape::SlabBottom));
        let mesh = mesh_section(&s, None, None, None, None, None, None, &tex, &|_, _, _| 0xFF);
        // 5 visible faces (no culling neighbors): 4 sides + top + bottom = 6
        assert_eq!(mesh.vertices.len(), 24, "slab = 6 faces like a cube, at half height");
        let tops: Vec<&Vertex> = mesh.vertices.iter()
            .filter(|v| v.normal == [0.0, 1.0, 0.0]).collect();
        assert_eq!(tops.len(), 4, "one top quad");
        for v in &tops {
            assert!((v.position[1] - 8.5).abs() < 1e-4, "top face at the half plane, got {}", v.position[1]);
        }
        // winding guard still holds for shaped geometry
        for i in (0..mesh.indices.len()).step_by(3) {
            let (a, b, d) = (mesh.vertices[mesh.indices[i] as usize], mesh.vertices[mesh.indices[i + 1] as usize], mesh.vertices[mesh.indices[i + 2] as usize]);
            let u = [b.position[0] - a.position[0], b.position[1] - a.position[1], b.position[2] - a.position[2]];
            let v = [d.position[0] - a.position[0], d.position[1] - a.position[1], d.position[2] - a.position[2]];
            let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            let dot = n[0] * a.normal[0] + n[1] * a.normal[1] + n[2] * a.normal[2];
            assert!(dot > 0.0, "shaped triangle must face outward along its normal");
        }
    }

    /// P34: a stair is a bottom slab + a back half-box; the step (low top
    /// strip) and the back top both exist, and nothing coincides.
    #[test]
    fn stair_renders_step_and_back() {
        use crate::Shape;
        let mut s = VoxelSection::new_empty();
        s.set(8, 8, 8, BlockState::STONE.with_shape(Shape::StairSouth));
        let mesh = mesh_section(&s, None, None, None, None, None, None, &tex, &|_, _, _| 0xFF);
        // slab(6 faces) + back box(5 faces: no bottom) + low strip top... the
        // slab's top is ONLY the low strip, so total = 4 sides + slab-bottom
        // + low-strip-top + 4 back sides + back top = 11 faces
        assert_eq!(mesh.vertices.len(), 44, "stair = 11 exterior faces, got {}", mesh.vertices.len());
        let top_y: Vec<f32> = mesh.vertices.iter()
            .filter(|v| v.normal == [0.0, 1.0, 0.0])
            .map(|v| v.position[1]).collect();
        assert!(top_y.iter().all(|y| (*y - 8.5).abs() < 1e-4 || (*y - 9.0).abs() < 1e-4),
            "tops only at the step (8.5) and the back (9.0)");
        // the back box occupies the +Z half: at least one +Z side quad at z=9
        assert!(mesh.vertices.iter().any(|v| v.normal == [0.0, 0.0, 1.0] && (v.position[2] - 9.0).abs() < 1e-4));
    }

    /// P34: shaped faces against a full opaque cube are culled.
    #[test]
    fn shaped_faces_cull_against_full_cubes() {
        use crate::Shape;
        let mut s = VoxelSection::new_empty();
        s.set(8, 8, 8, BlockState::STONE.with_shape(Shape::SlabBottom));
        s.set(8, 7, 8, BlockState::STONE); // full cube below: bottom face culled
        let mesh = mesh_section(&s, None, None, None, None, None, None, &tex, &|_, _, _| 0xFF);
        // slab = 5 faces (its bottom culled by the cube below) + the
        // supporting cube = 5 faces (its top culled by the slab)
        assert_eq!(mesh.vertices.len(), 40, "slab over a cube = 10 faces, got {}", mesh.vertices.len());
        let slab_bottoms = mesh.vertices.iter()
            .filter(|v| v.normal == [0.0, -1.0, 0.0] && (v.position[1] - 8.0).abs() < 1e-4).count();
        assert_eq!(slab_bottoms, 0, "the slab's own bottom face is culled at y=8");
    }

    /// Step 11: the connected-variant contract — stone/planks faces
    /// against the SAME block swap to the edgeless layers.
    #[test]
    fn connected_variants_map_the_two_families() {
        assert_eq!(lf_assets_conn(0), Some(83), "stone -> stone_conn");
        assert_eq!(lf_assets_conn(15), Some(84), "planks -> planks_conn");
        assert_eq!(lf_assets_conn(1), None, "grass has no variant");
    }

    /// Loop 331: a cross-plant renders as exactly two diagonal quads x2
    /// sides (16 vertices, all inside the cell) — no cube faces.
    #[test]
    fn cross_plants_emit_diagonal_quads_not_cubes() {
        let mut sec = crate::VoxelSection::new_empty();
        sec.set(8, 8, 8, BlockState(crate::registry::block::TALL_GRASS));
        let tex_of = &|_b, _f| 7u32;
        let light_of = &|_, _, _| 0xF0u32;
        let mesh = mesh_section(&sec, None, None, None, None, None, None, tex_of, light_of);
        assert_eq!(mesh.vertices.len(), 16, "4 quads x 4 verts, got {}",
            mesh.vertices.len());
        assert_eq!(mesh.indices.len(), 24, "4 quads x 6 indices");
        // every vertex stays inside the plant cell and touches only two
        // diagonal corner pairs (x==z or x+z==1 within the cell)
        for v in &mesh.vertices {
            let (lx, ly, lz) = (v.position[0] - 8.0, v.position[1] - 8.0, v.position[2] - 8.0);
            assert!((0.0..=1.0).contains(&lx) && (0.0..=1.0).contains(&ly) && (0.0..=1.0).contains(&lz));
            let on_diag_a = (lx - lz).abs() < 0.01;
            let on_diag_b = (lx + lz - 1.0).abs() < 0.01;
            assert!(on_diag_a || on_diag_b, "vertex off the diagonals: {:?}", v.position);
        }
        // control: a stone cube in the same spot emits the usual culled cube
        let mut sec2 = crate::VoxelSection::new_empty();
        sec2.set(8, 8, 8, BlockState(crate::registry::block::STONE));
        let cube = mesh_section(&sec2, None, None, None, None, None, None, tex_of, light_of);
        assert!(cube.vertices.len() > 16, "a cube has more geometry than a cross");
    }
}
