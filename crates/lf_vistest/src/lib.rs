//! Visual test harness: builds real scenes from real game data (worldgen ->
//! voxel sections -> mesher) and renders them with the real renderer to PNG.
//! Every proof screenshot must come through here.

use std::path::Path;

use glam::Vec3;
use lf_engine::{Camera, GpuVertex};
use lf_voxel::meshing::mesh_section;
use lf_voxel::{BlockState, VoxelSection};
use lf_worldgen::{Biome, BlockId, Seed, WorldGen};

/// One registered visual test scene.
pub struct SceneSpec {
    pub name: &'static str,
    pub desc: &'static str,
    pub default_seed: u64,
    /// Time of day as a fraction of the day cycle [0..1]; drives sky color.
    pub time_of_day: f32,
    /// Scene-relative camera placement (eye/target in world blocks).
    pub eye: Vec3,
    pub target: Vec3,
}

impl SceneSpec {
    fn sky_color(&self) -> [f64; 4] {
        // Mirrors lf_game::TimeOfDay::sky_color() day/night interpolation.
        let day = [0.53f32, 0.81, 0.98];
        let night = [0.05f32, 0.07, 0.15];
        let t = self.time_of_day;
        let is_day = t > 0.2 && t < 0.8;
        let mix = if is_day { (1.0 - (t - 0.5).abs() * 0.8).clamp(0.2, 1.0) } else { 0.2 };
        let c = if is_day {
            [day[0] * mix + night[0] * (1.0 - mix),
             day[1] * mix + night[1] * (1.0 - mix),
             day[2] * mix + night[2] * (1.0 - mix)]
        } else {
            [day[0] * 0.3 + night[0] * 0.7,
             day[1] * 0.3 + night[1] * 0.7,
             day[2] * 0.3 + night[2] * 0.7]
        };
        [c[0] as f64, c[1] as f64, c[2] as f64, 1.0]
    }
}

/// The scene registry. Rendered by `run_scene` and enumerated by tests/xtask.
pub fn scenes() -> Vec<SceneSpec> {
    vec![
        SceneSpec {
            name: "spawn_plains_dawn",
            desc: "meadow spawn at dawn, gentle terrain",
            default_seed: 12345,
            time_of_day: 0.25,
            eye: Vec3::new(-28.0, 92.0, 64.0),
            target: Vec3::new(24.0, 66.0, 8.0),
        },
        SceneSpec {
            name: "terrain_vista",
            desc: "wider view over varied biomes at noon",
            default_seed: 12345,
            time_of_day: 0.5,
            eye: Vec3::new(-60.0, 130.0, 120.0),
            target: Vec3::new(48.0, 64.0, 24.0),
        },
        SceneSpec {
            name: "night_watch",
            desc: "same terrain at night",
            default_seed: 12345,
            time_of_day: 0.0,
            eye: Vec3::new(-28.0, 92.0, 64.0),
            target: Vec3::new(24.0, 66.0, 8.0),
        },
    ]
}

/// Map a worldgen BlockId to the voxel BlockState used by sections.
fn to_block_state(b: BlockId) -> BlockState {
    match b {
        BlockId::AIR => BlockState::AIR,
        BlockId::STONE => BlockState::STONE,
        BlockId::DIRT => BlockState::DIRT,
        BlockId::GRASS => BlockState::GRASS,
        BlockId::SAND => BlockState(4),
        BlockId::MYCELIUM => BlockState(5),
        BlockId::SNOW => BlockState(6),
        BlockId(_) => BlockState::AIR,
    }
}

/// Build the mesh for a scene: a radius-chunk plot of worldgen terrain
/// centered at (0,0), meshed section by section with horizontal neighbors.
pub fn build_scene_mesh(_spec: &SceneSpec, seed: u64, radius_chunks: i32) -> (Vec<GpuVertex>, Vec<u32>) {
    let gen = WorldGen::new(Seed(seed));
    let size = (radius_chunks * 2 + 1) as usize;
    let top_section = 6; // sections 0..6 -> up to y=111 visible

    // sections[column_x][column_z][section_y]
    let mut sections: Vec<Vec<Vec<VoxelSection>>> = Vec::with_capacity(size);
    for cx in -radius_chunks..=radius_chunks {
        let mut col_x = Vec::with_capacity(size);
        for cz in -radius_chunks..=radius_chunks {
            let mut col = Vec::with_capacity(top_section);
            for sy in 0..top_section {
                let mut section = VoxelSection::new_empty();
                for lx in 0..16usize {
                    for lz in 0..16usize {
                        let wx = cx * 16 + lx as i32;
                        let wz = cz * 16 + lz as i32;
                        for (wy, block) in gen.column(wx, wz) {
                            if block == BlockId::AIR {
                                continue;
                            }
                            let sy_of = (wy as usize) / 16;
                            let ly = (wy as usize) % 16;
                            if sy_of == sy {
                                section.set(lx, ly, lz, to_block_state(block));
                            }
                        }
                    }
                }
                col.push(section);
            }
            col_x.push(col);
        }
        sections.push(col_x);
    }

    let tex_of = |b: BlockState| -> u32 { lf_assets::texture_index_for_block(b.id()) };

    let mut vertices: Vec<GpuVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for (ix, col_x) in sections.iter().enumerate() {
        for (iz, col) in col_x.iter().enumerate() {
            for (iy, section) in col.iter().enumerate() {
                let neighbor_px = col_x.get(ix + 1).and_then(|c| c.get(iy));
                let neighbor_nx = if ix > 0 { col_x.get(ix - 1).and_then(|c| c.get(iy)) } else { None };
                let neighbor_pz = col.get(iz + 1);
                let neighbor_nz = if iz > 0 { col.get(iz - 1) } else { None };
                let mesh = mesh_section(section, neighbor_px, neighbor_nx, None, None, neighbor_pz, neighbor_nz, &tex_of);
                let base = vertices.len() as u32;
                let origin = [
                    ((ix as i32 - radius_chunks) * 16) as f32,
                    (iy as i32 * 16) as f32,
                    ((iz as i32 - radius_chunks) * 16) as f32,
                ];
                for v in &mesh.vertices {
                    vertices.push(GpuVertex {
                        position: [v.position[0] + origin[0], v.position[1] + origin[1], v.position[2] + origin[2]],
                        normal: v.normal,
                        tex_coord: v.tex_coord,
                        tex_index: v.tex_index,
                        ao: v.ao,
                    });
                }
                indices.extend(mesh.indices.iter().map(|i| i + base));
            }
        }
    }
    (vertices, indices)
}

/// Render a registered scene by name to `out_path` (a real GPU render).
pub fn run_scene(name: &str, seed_override: Option<u64>, out_path: &Path) -> Result<(), String> {
    let spec = scenes().into_iter().find(|s| s.name == name)
        .ok_or_else(|| format!("unknown scene '{}'; known: {:?}", name, scenes().iter().map(|s| s.name).collect::<Vec<_>>()))?;
    let seed = seed_override.unwrap_or(spec.default_seed);
    let (vertices, indices) = build_scene_mesh(&spec, seed, 2);
    if vertices.is_empty() {
        return Err(format!("scene '{}' produced an empty mesh", name));
    }
    // Lift the camera safely above whatever terrain the seed generates at
    // its x/z so hills never bury the shot.
    let gen = WorldGen::new(Seed(seed));
    let h_eye = gen.height(spec.eye.x as i32, spec.eye.z as i32);
    let h_target = gen.height(spec.target.x as i32, spec.target.z as i32);
    let eye = Vec3::new(spec.eye.x, spec.eye.y.max(h_eye as f32 + 22.0), spec.eye.z);
    let target = Vec3::new(spec.target.x, h_target as f32 + 2.0, spec.target.z);
    let mut camera = Camera::new(eye, target);
    camera.set_aspect(800, 600);
    let textures = lf_assets::generate_atlas();
    lf_engine::headless::render_to_png(&vertices, &indices, &textures, &camera, spec.sky_color(), 800, 600, out_path)
}

/// Where the spawn column's biome lands for a seed (used by tests to describe scenes).
pub fn spawn_biome(seed: u64) -> Biome {
    WorldGen::new(Seed(seed)).biome(0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_scenes_with_unique_names() {
        let scenes = scenes();
        assert!(!scenes.is_empty());
        let mut names: Vec<&str> = scenes.iter().map(|s| s.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), scenes.len());
    }

    #[test]
    fn every_scene_builds_nonempty_deterministic_mesh() {
        for spec in scenes() {
            let (v1, i1) = build_scene_mesh(&spec, spec.default_seed, 1);
            let (v2, i2) = build_scene_mesh(&spec, spec.default_seed, 1);
            assert!(!v1.is_empty(), "{} produced no vertices", spec.name);
            assert_eq!(v1.len(), v2.len(), "{} mesh not deterministic", spec.name);
            assert_eq!(i1.len(), i2.len());
            assert_eq!(i1.len() % 3, 0, "{} indices not triangles", spec.name);
        }
    }

    #[test]
    fn seed_changes_mesh_somewhere() {
        fn hash(vertices: &[GpuVertex]) -> u64 {
            let mut h: u64 = 0xcbf29ce484222325;
            for v in vertices {
                for bits in v.position.map(|f| f.to_bits()) {
                    h = (h ^ bits as u64).wrapping_mul(0x100000001b3);
                }
                h = (h ^ v.tex_index as u64).wrapping_mul(0x100000001b3);
            }
            h
        }
        let spec = &scenes()[0];
        let mut hashes = std::collections::HashSet::new();
        for seed in 1..=20u64 {
            let (v, _) = build_scene_mesh(spec, seed, 1);
            hashes.insert(hash(&v));
        }
        assert!(hashes.len() > 1, "seeds 1..=20 all produced the same mesh");
    }

    #[test]
    fn unknown_scene_errors() {
        assert!(run_scene("nope", None, Path::new("/tmp/x.png")).is_err());
    }
}
