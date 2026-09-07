//! Construction mesh from host-owned state (visual reset R3DV-004).
//!
//! Architecture law: this module READS `pc3d_world`'s construction overlay
//! (`&Construction`, never `&mut`) and turns it into culled block-face
//! geometry in P3D world meters. Edits happen only through the host command
//! path (`SoloHost::submit(HostCommand::Build/RemoveBuild)` + `run_ticks`);
//! the renderer has no mutation route into canonical world state at all —
//! it cannot, because every API here takes immutable references.
//!
//! Meshing rules: one quad per built-cell face whose neighbor cell is not
//! built (faces between two built cells in the same patch are culled);
//! patch-boundary faces are emitted (the neighbor patch is not consulted —
//! cross-patch culling arrives with streamed terrain in R3DV-006). Each
//! 16 m patch owns its own GPU buffers with a content version; a host edit
//! bumps exactly one patch's version, so remesh work is bounded by the
//! patch, not the world.

use crate::scene::{SceneVertex, FACE_BASIS};
use pc3d_world::build::Construction;
use pc3d_world::gen::CellMaterial;
use pc3d_world::scales::PATCH_CELL_AXIS;
use std::collections::BTreeMap;

/// Linear albedo per construction material VIA THE ASSET MANIFEST registry
/// (R3DV-010 material binding): natural materials use their terrain rows,
/// built stone/wood use the block rows.
pub fn material_albedo(m: CellMaterial) -> [f32; 3] {
    let name = match m {
        CellMaterial::Air => "mat.block_stone",
        CellMaterial::Water => "mat.water_flow",
        CellMaterial::Soil => "mat.soil",
        CellMaterial::Grass => "mat.grass",
        CellMaterial::Sand => "mat.sand",
        CellMaterial::Rock => "mat.block_stone",
        CellMaterial::Snow => "mat.snow",
    };
    pc3d_assets::material_albedo(name).unwrap_or([0.5, 0.5, 0.5])
}

/// FNV-1a 64-bit (same family as the host journal hash) over a patch's
/// cells — a deterministic content version for bounded-remesh detection.
pub fn patch_version(con: &Construction) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for cell in &con.cells {
        h = (h ^ cell.map_or(0u8, |b| b.material as u8 + 1) as u64)
            .wrapping_mul(0x100000001b3);
        if let Some(b) = cell {
            h = (h ^ b.owner).wrapping_mul(0x100000001b3);
        }
    }
    h
}

/// Builds the culled-face mesh for one construction patch, in world meters.
/// Cell (x,y,z) occupies the cube [x,x+1) × [y,y+1) × [z,z+1).
pub fn mesh_patch(con: &Construction) -> (Vec<SceneVertex>, Vec<u16>) {
    let n = PATCH_CELL_AXIS as i32;
    let o = con.coord.origin();
    let base = (
        o.x.div_euclid(1000) as i32,
        o.y.div_euclid(1000) as i32,
        o.z.div_euclid(1000) as i32,
    );
    let at = |x: i32, y: i32, z: i32| -> Option<CellMaterial> {
        let (lx, ly, lz) = (x - base.0, y - base.1, z - base.2);
        if lx < 0 || ly < 0 || lz < 0 || lx >= n || ly >= n || lz >= n {
            return None; // outside the patch: treated as air
        }
        let i = ((lx as usize * n as usize + ly as usize) * n as usize + lz as usize) as usize;
        con.cells[i].map(|b| b.material)
    };

    let mut verts = Vec::new();
    let mut idx = Vec::new();
    for lx in 0..n {
        for ly in 0..n {
            for lz in 0..n {
                let Some(block) = con.cells[((lx as usize * n as usize + ly as usize) * n as usize
                    + lz as usize) as usize]
                else {
                    continue;
                };
                let (cx, cy, cz) = (base.0 + lx, base.1 + ly, base.2 + lz);
                let center = [cx as f32 + 0.5, cy as f32 + 0.5, cz as f32 + 0.5];
                let color = material_albedo(block.material);
                for (normal, u, v) in FACE_BASIS {
                    let neighbor = at(cx + normal[0] as i32, cy + normal[1] as i32, cz + normal[2] as i32);
                    if neighbor.is_some() {
                        continue; // culled: the neighbor cell is also built
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

/// What one construction-sync pass did — the bounded-remesh evidence.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UpdateStats {
    /// Patches that got GPU buffers for the first time.
    pub added: usize,
    /// Patches whose content version changed (remeshed buffers).
    pub remeshed: usize,
    /// Patches dropped from the map.
    pub removed: usize,
    /// Vertices uploaded this pass.
    pub uploaded_vertices: usize,
    /// CPU mesh time across remeshed patches.
    pub mesh_us: u128,
    /// Patches inspected (with or without changes).
    pub inspected: usize,
}

struct PatchGpu {
    version: u64,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

/// Per-patch GPU construction buffers, synced from the host's read-only
/// construction map. Only patches whose version changed are remeshed.
pub struct ConstructionGpu {
    patches: BTreeMap<(i32, i32, i32), PatchGpu>,
}

impl ConstructionGpu {
    pub fn new() -> Self {
        Self {
            patches: BTreeMap::new(),
        }
    }

    /// Syncs from the host construction map (immutable). Bounded work: a
    /// single host edit touches one patch key, so at most one remesh.
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        host: &BTreeMap<(i32, i32, i32), Construction>,
    ) -> UpdateStats {
        use wgpu::util::DeviceExt;
        let mut stats = UpdateStats {
            inspected: host.len(),
            ..Default::default()
        };
        let t0 = std::time::Instant::now();

        // Drop patches the host no longer has.
        let stale: Vec<(i32, i32, i32)> = self
            .patches
            .keys()
            .filter(|k| !host.contains_key(k))
            .copied()
            .collect();
        for k in stale {
            self.patches.remove(&k);
            stats.removed += 1;
        }

        for (key, con) in host {
            let version = patch_version(con);
            if let Some(existing) = self.patches.get(key) {
                if existing.version == version {
                    continue; // unchanged: no mesh work at all
                }
            } else {
                stats.added += 1;
            }
            let (verts, idx) = mesh_patch(con);
            stats.uploaded_vertices += verts.len();
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("construction vertices"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("construction indices"),
                contents: bytemuck::cast_slice(&idx),
                usage: wgpu::BufferUsages::INDEX,
            });
            let gpu = PatchGpu {
                version,
                vertex_buffer,
                index_buffer,
                index_count: idx.len() as u32,
            };
            let changed = self.patches.insert(*key, gpu);
            if changed.is_some() {
                stats.remeshed += 1;
            }
        }
        stats.mesh_us = t0.elapsed().as_micros();
        stats
    }

    /// Draws every patch with the lit-mesh pipeline the caller has bound.
    pub fn draw<'rp>(&self, pass: &mut wgpu::RenderPass<'rp>) {
        for p in self.patches.values() {
            if p.index_count == 0 {
                continue;
            }
            pass.set_vertex_buffer(0, p.vertex_buffer.slice(..));
            pass.set_index_buffer(p.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..p.index_count, 0, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pc3d_world::build::BuildBlock;
    use pc3d_world::coords::{CellCoord, PatchCoord};

    fn patch0() -> PatchCoord {
        PatchCoord { x: 0, y: 0, z: 0 }
    }

    fn place(con: &mut Construction, x: i32, y: i32, z: i32, m: CellMaterial) {
        con.place(
            CellCoord { x, y, z },
            BuildBlock { material: m, owner: 7 },
        )
        .expect("place");
    }

    #[test]
    fn lone_block_has_all_six_faces_and_world_meter_positions() {
        let mut con = Construction::new(patch0());
        place(&mut con, 3, 1, 5, CellMaterial::Rock);
        let (verts, idx) = mesh_patch(&con);
        assert_eq!(idx.len(), 6 * 6, "one block = 6 quads = 12 triangles");
        assert_eq!(verts.len(), 6 * 4);
        // Every vertex lies inside the cell cube [3,4)x[1,2)x[5,6) meters.
        for v in &verts {
            assert!(v.pos[0] >= 3.0 && v.pos[0] <= 4.0, "x {}", v.pos[0]);
            assert!(v.pos[1] >= 1.0 && v.pos[1] <= 2.0, "y {}", v.pos[1]);
            assert!(v.pos[2] >= 5.0 && v.pos[2] <= 6.0, "z {}", v.pos[2]);
        }
    }

    #[test]
    fn interior_faces_between_built_cells_are_culled() {
        let mut con = Construction::new(patch0());
        // A 2x2x2 solid cluster: 6 sides x 4 face-quads = 24 quads.
        for x in 0..2 {
            for y in 0..2 {
                for z in 0..2 {
                    place(&mut con, x, y, z, CellMaterial::Rock);
                }
            }
        }
        let (v1, i1) = mesh_patch(&con);
        assert_eq!(i1.len(), 24 * 6);
        assert_eq!(v1.len(), 24 * 4);

        // Adding a block adjacent to the cluster adds its 5 exposed faces
        // AND culls the cluster face it touches: 24 + 5 - 1 = 28 quads.
        place(&mut con, 2, 0, 0, CellMaterial::Sand);
        let (v2, i2) = mesh_patch(&con);
        assert_eq!(i2.len(), 28 * 6, "expected 28 quads");
        assert_eq!(v2.len(), 28 * 4);
    }

    #[test]
    fn every_triangle_winds_ccw_outward_for_backface_culling() {
        let mut con = Construction::new(patch0());
        for x in 0..2 {
            for y in 0..2 {
                for z in 0..2 {
                    place(&mut con, x, y, z, CellMaterial::Rock);
                }
            }
        }
        let (verts, idx) = mesh_patch(&con);
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
            assert!(dot > 0.0, "triangle winds inward: dot {dot}");
        }
    }

    #[test]
    fn negative_cell_coordinates_map_to_negative_meters() {
        // Patch (-1, 0, -1) spans cells x,z in [-16,0).
        let mut con = Construction::new(PatchCoord { x: -1, y: 0, z: -1 });
        place(&mut con, -3, 0, -2, CellMaterial::Soil);
        let (verts, _) = mesh_patch(&con);
        assert!(!verts.is_empty());
        for v in &verts {
            assert!(v.pos[0] >= -3.0 && v.pos[0] <= -2.0, "x {}", v.pos[0]);
            assert!(v.pos[1] >= 0.0 && v.pos[1] <= 1.0);
            assert!(v.pos[2] >= -2.0 && v.pos[2] <= -1.0, "z {}", v.pos[2]);
        }
    }

    #[test]
    fn versions_track_content_not_identity() {
        let mut a = Construction::new(patch0());
        let b = Construction::new(patch0());
        assert_eq!(patch_version(&a), patch_version(&b), "empty = empty");

        place(&mut a, 0, 0, 0, CellMaterial::Rock);
        assert_ne!(patch_version(&a), patch_version(&b));

        let mut c = Construction::new(patch0());
        place(&mut c, 0, 0, 0, CellMaterial::Rock);
        assert_eq!(
            patch_version(&a),
            patch_version(&c),
            "same content in a fresh patch = same version"
        );
    }
}
