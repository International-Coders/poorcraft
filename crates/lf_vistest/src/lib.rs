//! Visual test harness: builds real scenes from real game data (worldgen ->
//! voxel sections -> mesher) and renders them with the real renderer to PNG.
//! Every proof screenshot must come through here.

use std::path::Path;

use glam::Vec3;
use lf_engine::{Camera, GpuVertex};
use lf_voxel::{BlockState, World};
use lf_worldgen::{Biome, Seed, WorldGen};

/// One registered visual test scene.
pub struct SceneSpec {
    pub name: &'static str,
    pub desc: &'static str,
    pub default_seed: u64,
    /// Time of day as a fraction of the day cycle [0..1]; drives sky color.
    pub time_of_day: f32,
    /// First-person scenes put the camera at player eye height on the terrain.
    pub first_person: bool,
    /// Scenes with torches place a lit torch grid on the terrain before meshing.
    pub torches: bool,
    /// Scene-relative camera placement (eye/target in world blocks).
    pub eye: Vec3,
    pub target: Vec3,
}

impl SceneSpec {
    fn time_of_day(&self) -> lf_game::TimeOfDay {
        lf_game::TimeOfDay::from_fraction(self.time_of_day)
    }

    fn sky_color(&self) -> [f64; 4] {
        let c = self.time_of_day().sky_color();
        [c[0] as f64, c[1] as f64, c[2] as f64, 1.0]
    }

    fn day_factor(&self) -> f32 {
        self.time_of_day().sky_light_level()
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
            first_person: false,
            torches: false,
            eye: Vec3::new(-28.0, 92.0, 64.0),
            target: Vec3::new(24.0, 66.0, 8.0),
        },
        SceneSpec {
            name: "terrain_vista",
            desc: "wider view over varied biomes at noon",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            eye: Vec3::new(-60.0, 130.0, 120.0),
            target: Vec3::new(48.0, 64.0, 24.0),
        },
        SceneSpec {
            name: "night_watch",
            desc: "same terrain at night",
            default_seed: 12345,
            time_of_day: 0.0,
            first_person: false,
            torches: false,
            eye: Vec3::new(-28.0, 92.0, 64.0),
            target: Vec3::new(24.0, 66.0, 8.0),
        },
        SceneSpec {
            name: "first_person_view",
            desc: "in-game eye height over real terrain (what the player sees)",
            default_seed: 12345,
            time_of_day: 0.35,
            first_person: true,
            torches: false,
            eye: Vec3::new(8.5, 0.0, 8.5),
            target: Vec3::ZERO, // computed from terrain in run_scene
        },
        SceneSpec {
            name: "terrain_features",
            desc: "meadow with trees, ores in cliffs, water at the shore",
            default_seed: 12345,
            time_of_day: 0.45,
            first_person: false,
            torches: false,
            eye: Vec3::new(-40.0, 0.0, 90.0),
            target: Vec3::new(24.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "torchlit_night",
            desc: "night scene lit by torches placed on the terrain",
            default_seed: 12345,
            time_of_day: 0.97,
            first_person: false,
            torches: true,
            eye: Vec3::new(-26.0, 0.0, 40.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
    ]
}

/// Build the mesh for a scene: a radius-chunk plot of worldgen terrain
/// centered at (0,0), using the real World + chunk-column pipeline.
pub fn build_scene_mesh(_spec: &SceneSpec, seed: u64, radius_chunks: i32, torches: bool)
    -> (Vec<GpuVertex>, Vec<u32>, Vec<GpuVertex>, Vec<u32>) {
    let gen = WorldGen::new(Seed(seed));
    let mut world = World::new();
    for cx in -radius_chunks..=radius_chunks {
        for cz in -radius_chunks..=radius_chunks {
            world.chunks.insert((cx, cz), gen.generate_chunk(cx, cz));
        }
    }
    if torches {
        use lf_voxel::registry::block;
        // A grid of torches near the origin, placed on the terrain surface.
        for tx in (-24..=8).step_by(8) {
            for tz in (-24..=8).step_by(8) {
                let top = world.surface_height(tx, tz);
                world.set_block(tx, top, tz, lf_voxel::BlockState(block::TORCH));
            }
        }
    }

    let to_gpu = |vs: &[lf_voxel::meshing::Vertex]| -> Vec<GpuVertex> {
        vs.iter().map(|v| GpuVertex {
            position: v.position,
            normal: v.normal,
            tex_coord: v.tex_coord,
            tex_index: v.tex_index,
            ao: v.ao,
            light: v.light,
        }).collect()
    };
    let mut vertices: Vec<GpuVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut water_vertices: Vec<GpuVertex> = Vec::new();
    let mut water_indices: Vec<u32> = Vec::new();
    for cx in -radius_chunks..=radius_chunks {
        for cz in -radius_chunks..=radius_chunks {
            let mesh = world.mesh_column(cx, cz, &|b| lf_assets::texture_index_for_block(b.id()));
            let base = vertices.len() as u32;
            vertices.extend(to_gpu(&mesh.opaque.vertices));
            indices.extend(mesh.opaque.indices.iter().map(|i| i + base));
            let wbase = water_vertices.len() as u32;
            water_vertices.extend(to_gpu(&mesh.water.vertices));
            water_indices.extend(mesh.water.indices.iter().map(|i| i + wbase));
        }
    }
    (vertices, indices, water_vertices, water_indices)
}

/// Render a registered scene by name to `out_path` (a real GPU render).
pub fn run_scene(name: &str, seed_override: Option<u64>, out_path: &Path) -> Result<(), String> {
    let spec = scenes().into_iter().find(|s| s.name == name)
        .ok_or_else(|| format!("unknown scene '{}'; known: {:?}", name, scenes().iter().map(|s| s.name).collect::<Vec<_>>()))?;
    let seed = seed_override.unwrap_or(spec.default_seed);
    let (vertices, indices, water_vertices, water_indices) = build_scene_mesh(&spec, seed, 3, spec.torches);
    if vertices.is_empty() {
        return Err(format!("scene '{}' produced an empty mesh", name));
    }
    // Lift the camera safely above whatever terrain the seed generates at
    // its x/z so hills never bury the shot. First-person scenes instead sit
    // at player eye height looking slightly downhill.
    let gen = WorldGen::new(Seed(seed));
    let (eye, target) = if spec.first_person {
        // Find a viewpoint with an open vista: a local rise whose best look
        // direction drops the most over 30 blocks, so the frame shows both
        // nearby terrain and the horizon — what a player actually sees.
        let dirs = [
            Vec3::new(0.35, -0.18, -1.0),
            Vec3::new(-1.0, -0.18, -0.35),
            Vec3::new(-0.35, -0.18, 1.0),
            Vec3::new(1.0, -0.18, 0.35),
        ];
        let flat: Vec<Vec3> = dirs.iter().map(|d| Vec3::new(d.x, 0.0, d.z).normalize()).collect();
        // A moderate ~20-block drop at 30 blocks distance keeps the terrain
        // inside a 45-degree vertical frame; steeper vistas fall below view.
        let mut best_pos = (8i32, 8i32);
        let mut best_score = i32::MAX;
        let mut best_dir = 0usize;
        for x in (-20..20).step_by(4) {
            for z in (-20..20).step_by(4) {
                let h = gen.surface_top(x, z);
                for (i, d) in flat.iter().enumerate() {
                    let drop = h - gen.surface_top(x + (d.x * 30.0) as i32, z + (d.z * 30.0) as i32);
                    if drop < 8 {
                        continue; // needs some view at all
                    }
                    let score = (drop - 20).abs();
                    if score < best_score {
                        best_score = score;
                        best_pos = (x, z);
                        best_dir = i;
                    }
                }
            }
        }
        let h = gen.surface_top(best_pos.0, best_pos.1);
        let eye = Vec3::new(best_pos.0 as f32 + 0.5, h as f32 + 1.62, best_pos.1 as f32 + 0.5);
        (eye, eye + dirs[best_dir].normalize() * 40.0)
    } else {
        let h_eye = gen.surface_top(spec.eye.x as i32, spec.eye.z as i32);
        let h_target = gen.surface_top(spec.target.x as i32, spec.target.z as i32);
        (
            Vec3::new(spec.eye.x, spec.eye.y.max(h_eye as f32 + 22.0), spec.eye.z),
            Vec3::new(spec.target.x, h_target as f32 + 2.0, spec.target.z),
        )
    };
    let mut camera = Camera::new(eye, target);
    camera.set_aspect(800, 600);
    let env = lf_engine::scene::Env {
        camera_pos: eye,
        day_factor: spec.day_factor(),
        fog_color: spec.time_of_day().sky_color(),
        fog_far: 220.0,
    };
    let textures = lf_assets::generate_atlas();
    lf_engine::headless::render_to_png(&vertices, &indices, &water_vertices, &water_indices, &textures, &camera, &env, spec.sky_color(), 800, 600, out_path)
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
            let (v1, i1, _, _) = build_scene_mesh(&spec, spec.default_seed, 1, false);
            let (v2, i2, _, _) = build_scene_mesh(&spec, spec.default_seed, 1, false);
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
            let (v, _, _, _) = build_scene_mesh(spec, seed, 1, false);
            hashes.insert(hash(&v));
        }
        assert!(hashes.len() > 1, "seeds 1..=20 all produced the same mesh");
    }

    #[test]
    fn unknown_scene_errors() {
        assert!(run_scene("nope", None, Path::new("/tmp/x.png")).is_err());
    }
}
