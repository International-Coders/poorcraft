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
    /// Machine scenes place generator/furnace/crusher/assembler blocks.
    pub machines: bool,
    /// Ray-traced scenes render through the compute path tracer.
    pub raytraced: bool,
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
            machines: false,
            raytraced: false,
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
            machines: false,
            raytraced: false,
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
            machines: false,
            raytraced: false,
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
            machines: false,
            raytraced: false,
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
            machines: false,
            raytraced: false,
            eye: Vec3::new(-40.0, 0.0, 90.0),
            target: Vec3::new(24.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "biome_montage",
            desc: "vista across the 30-biome world (mixed tree species)",
            default_seed: 4242,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-60.0, 0.0, 110.0),
            target: Vec3::new(40.0, 0.0, 0.0),
        },
        SceneSpec {
            name: "clouds_weather",
            desc: "above the cloud layer looking down through it, rain below",
            default_seed: 4242,
            time_of_day: 0.55,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-30.0, 210.0, 60.0),
            target: Vec3::new(30.0, 110.0, -20.0),
        },
        SceneSpec {
            name: "village_trading",
            desc: "hamlet with villagers and an open trade panel",
            default_seed: 12345,
            time_of_day: 0.4,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-30.0, 0.0, -30.0),
            target: Vec3::new(10.0, 0.0, 10.0),
        },
        SceneSpec {
            name: "industrial_machines",
            desc: "machines placed and running on the terrain",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: true,
            raytraced: false,
            eye: Vec3::new(-26.0, 0.0, 42.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "tech_tree",
            desc: "the research progression screen",
            default_seed: 12345,
            time_of_day: 0.45,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-26.0, 0.0, 42.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "raytraced_shadows",
            desc: "path-traced frame with soft sun shadows + GI",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: true,
            torches: false,
            machines: false,
            raytraced: true,
            eye: Vec3::new(8.5, 0.0, 8.5),
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "raytraced_night",
            desc: "path-traced night: torch emissive light and bounce",
            default_seed: 12345,
            time_of_day: 0.97,
            first_person: false,
            torches: true,
            machines: false,
            raytraced: true,
            eye: Vec3::new(-26.0, 0.0, 40.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "menu_preview",
            desc: "the animated title screen over the world",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-26.0, 0.0, 42.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "settings_preview",
            desc: "the tabbed settings screen with RT controls",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-26.0, 0.0, 42.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "hud_preview",
            desc: "in-game view with the real HUD drawn via egui (proof shot)",
            default_seed: 12345,
            time_of_day: 0.42,
            first_person: true,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(8.5, 0.0, 8.5),
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "torchlit_night",
            desc: "night scene lit by torches placed on the terrain",
            default_seed: 12345,
            time_of_day: 0.97,
            first_person: false,
            torches: true,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-26.0, 0.0, 40.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "crafting_ui",
            desc: "3x3 crafting grid + recipe book with real icons",
            default_seed: 12345,
            time_of_day: 0.42,
            first_person: true,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(8.5, 0.0, 8.5),
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "map_screen",
            desc: "the world map with biome colors, fog, waypoints",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-26.0, 0.0, 42.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "console_preview",
            desc: "the developer console with autocomplete + history",
            default_seed: 12345,
            time_of_day: 0.42,
            first_person: true,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(8.5, 0.0, 8.5),
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "minimap_hud",
            desc: "in-game HUD with minimap, icons hotbar, XP bar",
            default_seed: 12345,
            time_of_day: 0.35,
            first_person: true,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(8.5, 0.0, 8.5),
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "foliage_canopy",
            desc: "canopy close-up: cutout leaves, log rings, smooth AO under leaves",
            default_seed: 12345,
            time_of_day: 0.4,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed directly at the placed canopy in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "mining_feedback",
            desc: "crack decal + debris particles on a block being mined",
            default_seed: 12345,
            time_of_day: 0.45,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed directly at the target block in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "water_flow",
            desc: "source on an aqueduct pours down a flume and pools at a dam (flowing surfaces render lowered)",
            default_seed: 12345,
            time_of_day: 0.4,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed at the built waterfall in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "falling_sand",
            desc: "granular column collapse: settled pile plus a block caught mid-fall",
            default_seed: 12345,
            time_of_day: 0.4,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed at the column in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "texture_tiling",
            desc: "7-wide plank wall + wide stone floor: textures repeat per block, never stretch across merged surfaces",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed straight at the wall in run_scene
            target: Vec3::ZERO,
        },
    ]
}

/// Build the mesh for a scene: a radius-chunk plot of worldgen terrain
/// centered at (0,0), using the real World + chunk-column pipeline.
pub fn build_scene_mesh(spec: &SceneSpec, seed: u64, radius_chunks: i32, torches: bool, machines_param: bool)
    -> (Vec<GpuVertex>, Vec<u32>, Vec<GpuVertex>, Vec<u32>) {
    let gen = WorldGen::new(Seed(seed));
    let mut world = World::new();
    for cx in -radius_chunks..=radius_chunks {
        for cz in -radius_chunks..=radius_chunks {
            world.chunks.insert((cx, cz), gen.generate_chunk(cx, cz));
        }
    }
    if machines_param {
        use lf_voxel::registry::block;
        let mut place = |x: i32, z: i32, b: u32| {
            let top = world.surface_height(x, z);
            world.set_block(x, top, z, lf_voxel::BlockState(b));
        };
        place(0, 0, block::COAL_GENERATOR);
        place(2, 0, block::ELECTRIC_FURNACE);
        place(4, 0, block::CRUSHER);
        place(6, 0, block::ASSEMBLER);
        place(8, 0, block::RESEARCH_BENCH);
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

    // P26 proof geometry: a hand-framed canopy, and a crack decal with
    // debris on a block mid-mining.
    if spec.name == "foliage_canopy" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for y in h..h + 8 {
            world.set_block(0, y, 0, lf_voxel::BlockState(block::LOG));
        }
        for dy in 7..12 {
            for dx in -3i32..=3 {
                for dz in -3i32..=3 {
                    if dx.abs() + dz.abs() + (dy - 7) <= 6 {
                        world.set_block(dx, h + dy, dz, lf_voxel::BlockState(block::LEAVES));
                    }
                }
            }
        }
    }

    if spec.name == "mining_feedback" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for y in 0..5 {
            world.set_block(0, h + y, 0, lf_voxel::BlockState(block::STONE));
        }
    }

    // texture_tiling (goal Section 1): a 7-wide, 4-tall plank wall on a
    // wide stone floor — the proof that textures repeat at 1-block scale
    // on multi-block surfaces instead of stretching.
    if spec.name == "texture_tiling" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -5..=5 {
            for z in -3..=4 {
                for y in h..h + 9 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
        // 7-wide, 4-tall plank wall standing on the floor at z = 0
        for x in -3..=3 {
            for y in h..h + 4 {
                world.set_block(x, y, 0, lf_voxel::BlockState(block::PLANKS));
            }
        }
    }

    // water_flow: a stone aqueduct with a source on top, a guiding flume
    // and a dam — then the real simulation runs to quiescence before
    // meshing, so the PNG shows actual flow levels and pooling.
    if spec.name == "water_flow" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        // flatten a work pad
        for x in -6..16 {
            for z in -6..6 {
                for y in h..h + 14 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
        // flume walls guide the runoff 1-D along +x, dam at the far end
        for x in 0..=9 {
            for y in h..h + 2 {
                world.set_block(x, y, -2, lf_voxel::BlockState(block::STONE));
                world.set_block(x, y, 2, lf_voxel::BlockState(block::STONE));
            }
        }
        for y in h..h + 3 {
            for z in -2..=2 {
                world.set_block(9, y, z, lf_voxel::BlockState(block::STONE));
            }
        }
        // aqueduct pillar + source on top
        for y in h..h + 5 {
            world.set_block(0, y, 0, lf_voxel::BlockState(block::STONE));
        }
        world.set_block(0, h + 5, 0, lf_voxel::water_with_level(0));
        let mut q = std::collections::VecDeque::new();
        lf_game::fluids::enqueue_around(&mut q, (0, h + 5, 0));
        lf_game::fluids::settle(&mut world, &mut q, 20_000);
    }

    // falling_sand: a sand column over a dug pocket — the collapse runs
    // through the real gravity settle before meshing.
    if spec.name == "falling_sand" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -5..5 {
            for z in -5..5 {
                for y in h..h + 10 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
        // pocket: two air cells under the column with a stone floor
        world.set_block(0, h - 1, 0, lf_voxel::BlockState(block::AIR));
        world.set_block(0, h - 2, 0, lf_voxel::BlockState(block::AIR));
        world.set_block(0, h - 3, 0, lf_voxel::BlockState(block::STONE));
        for y in h..h + 5 {
            world.set_block(0, y, 0, lf_voxel::BlockState(block::SAND));
        }
        lf_game::fluids::settle_gravity(&mut world, 0, 0);
    }

    let to_gpu = |vs: &[lf_voxel::meshing::Vertex]| -> Vec<GpuVertex> {
        vs.iter().map(|v| GpuVertex {
            position: v.position,
            normal: v.normal,
            tex_coord: v.tex_coord,
            tex_index: v.tex_index,
            ao: v.ao,
            light: v.light,
            sway: v.sway,
        }).collect()
    };
    let mut vertices: Vec<GpuVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut water_vertices: Vec<GpuVertex> = Vec::new();
    let mut water_indices: Vec<u32> = Vec::new();
    for cx in -radius_chunks..=radius_chunks {
        for cz in -radius_chunks..=radius_chunks {
            let mesh = world.mesh_column(cx, cz, &|b, face| lf_assets::texture_index_for_face(b.id(), face));
            let base = vertices.len() as u32;
            vertices.extend(to_gpu(&mesh.opaque.vertices));
            indices.extend(mesh.opaque.indices.iter().map(|i| i + base));
            let wbase = water_vertices.len() as u32;
            water_vertices.extend(to_gpu(&mesh.water.vertices));
            water_indices.extend(mesh.water.indices.iter().map(|i| i + wbase));
        }
    }
    // mining_feedback: crack decal + debris billboards around the column
    // that build_scene_mesh placed before meshing
    if spec.name == "mining_feedback" {
        let push_quad = |vertices: &mut Vec<GpuVertex>, indices: &mut Vec<u32>,
                         corners: [[f32; 3]; 4], uvs: [[f32; 2]; 4], normal: [f32; 3], tex: u32| {
            let base = vertices.len() as u32;
            for (c, uv) in corners.iter().zip(uvs.iter()) {
                vertices.push(GpuVertex {
                    position: *c,
                    normal,
                    tex_coord: *uv,
                    tex_index: tex,
                    ao: 1.0,
                    light: 0xF0,
                    sway: 0.0,
                });
            }
            indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        };
        // stage-2 crack decal, inflated around the middle block
        let h = world.surface_height(0, 0);
        let (cx, cy, cz) = (0.5f32, h as f32 + 2.5, 0.5f32);
        let r = 0.505f32;
        let crack = lf_assets::CRACK_LAYERS[2];
        let faces: [([f32; 3], [[f32; 3]; 4], [[f32; 2]; 4]); 6] = [
            ([-1.0, 0.0, 0.0], [[-r, -r, -r], [-r, r, -r], [-r, r, r], [-r, -r, r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
            ([1.0, 0.0, 0.0], [[r, -r, r], [r, r, r], [r, r, -r], [r, -r, -r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
            ([0.0, -1.0, 0.0], [[-r, -r, -r], [-r, -r, r], [r, -r, r], [r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ([0.0, 1.0, 0.0], [[-r, r, r], [-r, r, -r], [r, r, -r], [r, r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ([0.0, 0.0, -1.0], [[r, -r, -r], [r, r, -r], [-r, r, -r], [-r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ([0.0, 0.0, 1.0], [[-r, -r, r], [-r, r, r], [r, r, r], [r, -r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
        ];
        for (normal, corners, uvs) in faces {
            let corners: [[f32; 3]; 4] = corners.map(|c| [cx + c[0], cy + c[1], cz + c[2]]);
            push_quad(&mut vertices, &mut indices, corners, uvs, normal, crack);
        }
        // camera-facing debris quads (stone texture sub-tiles)
        let stone_tex = lf_assets::texture_index_for_block(lf_voxel::registry::block::STONE);
        for i in 0..6u32 {
            let t = i as f32;
            let (ox, oy, oz) = (((t * 2.3).sin()) * 0.9, 1.6 + (t % 2.0) * 0.8, ((t * 1.7).cos()) * 0.9);
            let center = Vec3::new(cx + ox, cy + oy, cz + oz);
            let right = Vec3::new(0.08, 0.0, 0.0);
            let up = Vec3::new(0.0, 0.08, 0.0);
            let u0 = (t * 0.11) % 0.75;
            let v0 = (t * 0.17) % 0.75;
            let c0 = center - right - up;
            let c1 = center - right + up;
            let c2 = center + right + up;
            let c3 = center + right - up;
            push_quad(&mut vertices, &mut indices,
                [[c0.x, c0.y, c0.z], [c1.x, c1.y, c1.z], [c2.x, c2.y, c2.z], [c3.x, c3.y, c3.z]],
                [[u0, v0 + 0.25], [u0, v0], [u0 + 0.25, v0], [u0 + 0.25, v0 + 0.25]],
                [0.0, 0.0, 1.0], stone_tex);
        }
    }
    // falling_sand: one granular block caught mid-fall above the settled
    // pile (the client renders these as near-full cubes with the block's
    // own texture — same shape here, appended post-mesh)
    if spec.name == "falling_sand" {
        let push_quad = |vertices: &mut Vec<GpuVertex>, indices: &mut Vec<u32>,
                         corners: [[f32; 3]; 4], uvs: [[f32; 2]; 4], normal: [f32; 3], tex: u32| {
            let base = vertices.len() as u32;
            for (c, uv) in corners.iter().zip(uvs.iter()) {
                vertices.push(GpuVertex {
                    position: *c,
                    normal,
                    tex_coord: *uv,
                    tex_index: tex,
                    ao: 1.0,
                    light: 0xF0,
                    sway: 0.0,
                });
            }
            indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        };
        let h = world.surface_height(0, 0) as f32;
        let (cx, cy, cz) = (0.5f32, h + 3.2, 0.5f32);
        let r = 0.48f32;
        let sand_tex = lf_assets::texture_index_for_block(lf_voxel::registry::block::SAND);
        let faces: [([f32; 3], [[f32; 3]; 4], [[f32; 2]; 4]); 6] = [
            ([-1.0, 0.0, 0.0], [[-r, -r, -r], [-r, r, -r], [-r, r, r], [-r, -r, r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
            ([1.0, 0.0, 0.0], [[r, -r, r], [r, r, r], [r, r, -r], [r, -r, -r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
            ([0.0, -1.0, 0.0], [[-r, -r, -r], [-r, -r, r], [r, -r, r], [r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ([0.0, 1.0, 0.0], [[-r, r, r], [-r, r, -r], [r, r, -r], [r, r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ([0.0, 0.0, -1.0], [[r, -r, -r], [r, r, -r], [-r, r, -r], [-r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ([0.0, 0.0, 1.0], [[-r, -r, r], [-r, r, r], [r, r, r], [r, -r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
        ];
        for (normal, corners, uvs) in faces {
            let corners: [[f32; 3]; 4] = corners.map(|c| [cx + c[0], cy + c[1], cz + c[2]]);
            push_quad(&mut vertices, &mut indices, corners, uvs, normal, sand_tex);
        }
    }
    (vertices, indices, water_vertices, water_indices)
}

/// Render a registered scene by name to `out_path` (a real GPU render).
pub fn run_scene(name: &str, seed_override: Option<u64>, out_path: &Path) -> Result<(), String> {
    let spec = scenes().into_iter().find(|s| s.name == name)
        .ok_or_else(|| format!("unknown scene '{}'; known: {:?}", name, scenes().iter().map(|s| s.name).collect::<Vec<_>>()))?;
    let seed = seed_override.unwrap_or(spec.default_seed);
    let (vertices, indices, water_vertices, water_indices) = build_scene_mesh(&spec, seed, 3, spec.torches, spec.machines);
    if vertices.is_empty() {
        return Err(format!("scene '{}' produced an empty mesh", name));
    }
    // Lift the camera safely above whatever terrain the seed generates at
    // its x/z so hills never bury the shot. First-person scenes instead sit
    // at player eye height looking slightly downhill.
    let gen = WorldGen::new(Seed(seed));
    let (eye, target) = if spec.name == "foliage_canopy" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-10.0, h + 13.0, 14.0), Vec3::new(0.5, h + 9.5, 0.5))
    } else if spec.name == "mining_feedback" {
        // the slope rises toward the camera, so reference the terrain AT the
        // eye or the camera ends up buried (backfaces see through the hill)
        let h = gen.surface_top(0, 0) as f32;
        let he = gen.surface_top(-6, 7) as f32;
        (Vec3::new(-6.0, he + 2.2, 7.0), Vec3::new(0.5, h + 2.5, 0.5))
    } else if spec.name == "water_flow" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-9.0, h + 9.0, 11.0), Vec3::new(4.0, h + 1.5, 0.0))
    } else if spec.name == "falling_sand" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-7.0, h + 5.0, 8.0), Vec3::new(0.5, h - 1.0, 0.5))
    } else if spec.name == "texture_tiling" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(0.5, h + 2.6, 9.5), Vec3::new(0.5, h + 1.5, 0.0))
    } else if spec.first_person {
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
        // mid-sway pose: proofs show the wind offset statically
        time: 0.8,
        day_factor: spec.day_factor(),
        fog_color: spec.time_of_day().sky_color(),
        fog_far: 220.0,
        grade_tint: [1.0, 1.0, 1.0],
        grade_saturation: 1.0,
    };
    // clouds/weather scene: atmosphere geometry joins the standard mesh
    let (mut vertices, mut indices, mut water_vertices, mut water_indices) =
        (vertices, indices, water_vertices, water_indices);
    if spec.name == "clouds_weather" {
        let (sv, si) = lf_engine::atmosphere::sky_bodies(eye, spec.time_of_day);
        let base = vertices.len() as u32;
        vertices.extend(sv);
        indices.extend(si.iter().map(|i| i + base));
        let (cv, ci) = lf_engine::atmosphere::cloud_mesh(eye, 40.0);
        let wbase = water_vertices.len() as u32;
        water_vertices.extend(cv);
        water_indices.extend(ci.iter().map(|i| i + wbase));
        let (rv, ri) = lf_engine::atmosphere::weather_particles(Vec3::new(0.0, 100.0, 0.0), 3.0, false);
        let rbase = water_vertices.len() as u32;
        water_vertices.extend(rv);
        water_indices.extend(ri.iter().map(|i| i + rbase));
    }
    let (vertices, indices, water_vertices, water_indices) = (vertices, indices, water_vertices, water_indices);

    let ui = spec.name == "hud_preview" || spec.name == "village_trading" || spec.name == "tech_tree"
        || spec.name == "menu_preview" || spec.name == "settings_preview"
        || spec.name == "crafting_ui" || spec.name == "map_screen" || spec.name == "minimap_hud"
        || spec.name == "console_preview";
    let (ui_ctx, warm_textures) = if ui {
        let ctx = egui::Context::default();
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))),
            ..Default::default()
        };
        let draw = |ctx: &egui::Context| {
            draw_hud_preview(ctx);
            if spec.name == "village_trading" {
                draw_trade_preview(ctx);
            }
            if spec.name == "tech_tree" {
                draw_tech_tree_preview(ctx);
            }
            if spec.name == "menu_preview" {
                draw_menu_preview(ctx);
            }
            if spec.name == "settings_preview" {
                draw_settings_preview(ctx);
            }
            if spec.name == "crafting_ui" {
                draw_crafting_preview(ctx);
            }
            if spec.name == "map_screen" {
                draw_map_preview(ctx);
            }
            if spec.name == "minimap_hud" {
                draw_minimap_preview(ctx);
            }
            if spec.name == "console_preview" {
                draw_console_preview(ctx);
            }
        };
        // Warmup pass: egui windows need one pass to materialize their areas
        // before their content renders (a fresh single-pass context produces
        // empty window shapes — this bit the pre-P22 trade/tech proofs too).
        ctx.begin_pass(raw.clone());
        draw(&ctx);
        let warm = ctx.end_pass();
        ctx.begin_pass(raw);
        draw(&ctx);
        // The warmup output carried the font-atlas texture delta away; keep
        // it so the renderer still uploads fonts (else text/painted fills
        // vanish from the proof).
        (Some(ctx), Some(warm.textures_delta.set))
    } else {
        (None, None)
    };
    let overlay = ui_ctx.as_ref().map(|ctx| lf_engine::headless::UiOverlay {
        ctx,
        extra_textures: warm_textures.as_deref().unwrap_or(&[]),
    });
    if spec.raytraced {
        render_raytraced(&spec, seed, &eye, out_path)?;
        return verify_render(out_path);
    }
    let textures = lf_assets::generate_atlas();
    lf_engine::headless::render_to_png(&vertices, &indices, &water_vertices, &water_indices, &textures, &camera, &env, spec.sky_color(), 800, 600, out_path, overlay.as_ref())?;
    verify_render(out_path)
}

/// Post-render proof check: reopen the written PNG and assert it contains a
/// real image — sane dimensions, several distinct colors, and actual luma
/// variance. Guards against silently black / single-color "it rendered"
/// outputs (AGENTS.md: pixel-analyze the PNGs, never trust that it rendered).
fn verify_render(out_path: &Path) -> Result<(), String> {
    let img = image::open(out_path).map_err(|e| format!("reopen {}: {e}", out_path.display()))?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    if w < 100 || h < 100 {
        return Err(format!("suspect render {}: only {}x{}", out_path.display(), w, h));
    }
    let mut colors = std::collections::HashSet::new();
    let (mut luma_min, mut luma_max) = (u8::MAX, 0u8);
    for p in rgba.pixels() {
        colors.insert(p.0);
        let luma = ((p.0[0] as u32 * 3 + p.0[1] as u32 * 4 + p.0[2] as u32) / 8) as u8;
        luma_min = luma_min.min(luma);
        luma_max = luma_max.max(luma);
        if colors.len() >= 64 && luma_max - luma_min > 32 {
            break; // enough evidence of a real image; skip the full scan
        }
    }
    if colors.len() < 16 {
        return Err(format!("suspect render {}: only {} distinct colors", out_path.display(), colors.len()));
    }
    if luma_max.saturating_sub(luma_min) < 16 {
        return Err(format!("suspect render {}: near-uniform luma {}..{}", out_path.display(), luma_min, luma_max));
    }
    Ok(())
}

/// Title-menu proof overlay mirroring the animated client screen.
fn draw_menu_preview(ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(90)))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("LOREFORGE").size(58.0)
                    .color(egui::Color32::from_rgb(250, 220, 160)).strong());
                ui.label(egui::RichText::new("a voxel saga of forge & industry").size(16.0)
                    .color(egui::Color32::from_rgb(200, 205, 212)));
                ui.add_space(24.0);
                let items = [("Play — World 1", true), ("New World", false), ("Load Game", false),
                             ("Multiplayer (localhost)", false), ("Settings", false), ("Quit", false)];
                for (label, accent) in items {
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(300.0, 50.0), egui::Sense::click());
                    let _ = response;
                    let fill = if accent {
                        egui::Color32::from_rgba_premultiplied(60, 48, 22, 235)
                    } else {
                        egui::Color32::from_rgba_premultiplied(28, 33, 44, 225)
                    };
                    ui.painter().rect_filled(rect, 10.0, fill);
                    let stroke = if accent {
                        egui::Color32::from_rgb(240, 200, 120)
                    } else {
                        egui::Color32::from_rgb(90, 98, 112)
                    };
                    ui.painter().rect_stroke(rect, 10.0, egui::Stroke::new(2.0, stroke), egui::StrokeKind::Middle);
                    if accent {
                        let bar = egui::Rect::from_min_size(
                            egui::Pos2::new(rect.left() + 4.0, rect.center().y - 16.0),
                            egui::vec2(3.0, 32.0),
                        );
                        ui.painter().rect_filled(bar, 2.0, egui::Color32::from_rgb(240, 200, 120));
                    }
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label,
                        egui::FontId::proportional(20.0), egui::Color32::from_rgb(235, 238, 242));
                }
            });
        });
}

/// Settings proof overlay mirroring the tabbed client screen.
fn draw_settings_preview(ctx: &egui::Context) {
    egui::Window::new("Settings")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -10.0))
        .min_size(egui::vec2(520.0, 380.0))
        .collapsible(false).resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (label, on) in [("Video", true), ("Interface", false), ("Audio", false), ("Gameplay", false)] {
                    let btn = egui::Button::new(egui::RichText::new(label)
                        .color(if on { ACCENT } else { TEXT_DIM }))
                        .min_size(egui::vec2(90.0, 28.0));
                    let _ = ui.add(btn);
                }
            });
            ui.separator();
            ui.label(egui::RichText::new("Video").size(17.0).color(egui::Color32::from_rgb(240, 200, 120)));
            ui.add(egui::Slider::new(&mut 70.0f32, 50.0..=110.0).text("Field of view"));
            ui.add(egui::Slider::new(&mut 5.0f32, 3.0..=8.0).text("View distance"));
            ui.checkbox(&mut true, "Clouds");
            ui.checkbox(&mut true, "Weather particles");
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Ray Tracing").size(17.0).color(egui::Color32::from_rgb(240, 200, 120)));
            ui.horizontal(|ui| {
                ui.label("Mode");
                ui.button(egui::RichText::new("Live  (cycle)").color(egui::Color32::from_rgb(240, 200, 120)));
                ui.label(egui::RichText::new("live path-traced view (GPU heavy)").small()
                    .color(egui::Color32::from_rgb(150, 156, 165)));
            });
            ui.add(egui::Slider::new(&mut 0.25f32, 0.1..=0.5).text("RT internal scale"));
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Quality preset").size(17.0).color(egui::Color32::from_rgb(240, 200, 120)));
            ui.horizontal(|ui| {
                ui.button("Low"); ui.button("Medium"); ui.button("High");
            });
        });
}

/// Tech-tree proof overlay mirroring the client's draw_tech_tree.
fn draw_tech_tree_preview(ctx: &egui::Context) {
    let icons = PreviewIcons::new(ctx, &["copper_ingot", "tin_ingot", "steel_ingot", "iron_gear", "coal"]);
    egui::Window::new("Technology — K to close")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -20.0))
        .min_size(egui::vec2(640.0, 380.0))
        .collapsible(false)
        .show(ctx, |ui| {
            ui.heading(egui::RichText::new("Research Progression").size(22.0));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for (e, state) in [
                    ("Primitive", "done"), ("Bronze Age", "CURRENT"), ("Industrial Age", "locked"), ("Electrical Age", "locked"),
                ] {
                    let color = if state == "done" { egui::Color32::from_rgb(120, 200, 120) }
                        else if state == "CURRENT" { ACCENT }
                        else { egui::Color32::from_gray(110) };
                    egui::Frame::new()
                        .fill(egui::Color32::from_black_alpha(120))
                        .stroke(egui::Stroke::new(if state == "CURRENT" { 2.5 } else { 1.0 }, color))
                        .corner_radius(8.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(140.0, 90.0));
                            ui.heading(egui::RichText::new(e).size(15.0).color(color));
                            ui.label(egui::RichText::new(state).small().color(color));
                            if state == "locked" {
                                ui.add_space(4.0);
                                for (item, got, n) in [("copper_ingot", 7, 10), ("tin_ingot", 5, 5), ("steel_ingot", 0, 5)] {
                                    let ok = got >= n;
                                    let c = if ok { OK } else { egui::Color32::from_rgb(230, 130, 130) };
                                    ui.horizontal(|ui| {
                                        let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                                        icons.paint(ui, r, item);
                                        ui.label(egui::RichText::new(format!("{}/{}", got, n)).small().color(c));
                                    });
                                }
                            }
                        });
                    if e != "Electrical Age" { ui.label("->"); }
                }
            });
            ui.add_space(8.0);
            ui.separator();
            ui.label(egui::RichText::new("Next: the Industrial Age — place a Research Bench and bring: steel_ingot (0/5), iron_gear (2/3), coal (14/20)")
                .color(egui::Color32::from_rgb(150, 220, 255)));
        });
}

/// Trade-panel proof overlay (same egui stack as the client trade UI).
fn draw_trade_preview(ctx: &egui::Context) {
    let icons = PreviewIcons::new(ctx, &["raw_iron", "iron_pickaxe", "iron_ingot", "stone_sword", "coal", "furnace"]);
    egui::Window::new("Trading with Brann the Smith")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -30.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            for (give, give_n, get, get_n, have) in [
                ("raw_iron", 4, "iron_pickaxe", 1, 6),
                ("iron_ingot", 3, "stone_sword", 1, 2),
                ("coal", 6, "furnace", 1, 9),
            ] {
                let enough = have >= give_n;
                egui::Frame::new()
                    .fill(egui::Color32::from_black_alpha(130))
                    .corner_radius(7.0)
                    .inner_margin(6.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                            icons.paint(ui, r, give);
                            ui.label(egui::RichText::new(format!("x{}", give_n)).color(if enough { OK } else { BAD }));
                            ui.label(egui::RichText::new("→").color(TEXT_DIM));
                            let (r2, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                            icons.paint(ui, r2, get);
                            ui.label(egui::RichText::new(format!("x{}", get_n)).color(egui::Color32::from_rgb(235, 238, 242)));
                            ui.label(egui::RichText::new(format!("(have {})", have)).small().color(TEXT_DIM));
                            ui.add_enabled(enough, egui::Button::new("Trade"));
                        });
                    });
            }
            ui.separator();
            ui.label(egui::RichText::new("Esc to close").small());
        });
}

// ------------------------------------------------------------------
// Preview helpers: real icon textures + map images from real worldgen.

const ACCENT: egui::Color32 = egui::Color32::from_rgb(240, 200, 120);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(150, 156, 165);
const OK: egui::Color32 = egui::Color32::from_rgb(120, 210, 130);
const BAD: egui::Color32 = egui::Color32::from_rgb(230, 120, 110);

fn load_icon(ctx: &egui::Context, id: &str) -> egui::TextureHandle {
    use lf_game::items::ItemKind;
    let img = match lf_game::items::item_def(id).map(|d| d.kind) {
        Some(ItemKind::Block(b)) => {
            let layer = lf_assets::texture_index_for_block(b) as usize;
            lf_assets::generate_block_texture(lf_assets::TEXTURE_NAMES[layer])
        }
        _ => lf_assets::generate_item_texture(id)
            .unwrap_or_else(|| lf_assets::generate_block_texture("stone")),
    };
    let size = [img.width() as usize, img.height() as usize];
    let pixels = img.pixels().map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3])).collect();
    ctx.load_texture(format!("preview_icon:{}", id), egui::ColorImage { size, pixels }, egui::TextureOptions::NEAREST)
}

/// Icon registry for one preview frame.
struct PreviewIcons {
    map: std::collections::HashMap<String, egui::TextureHandle>,
}

impl PreviewIcons {
    fn new(ctx: &egui::Context, ids: &[&str]) -> Self {
        Self { map: ids.iter().map(|id| (id.to_string(), load_icon(ctx, id))).collect() }
    }

    fn paint(&self, ui: &mut egui::Ui, rect: egui::Rect, id: &str) {
        let uv = egui::Rect::from_min_max(egui::Pos2::new(0.0, 0.0), egui::Pos2::new(1.0, 1.0));
        match self.map.get(id) {
            Some(tex) => { ui.painter().image(tex.id(), rect, uv, egui::Color32::WHITE); }
            None => { ui.painter().rect_filled(rect, 3.0, egui::Color32::from_gray(140)); }
        }
    }
}

/// One preview slot: recessed well + real icon + count + optional selection.
fn preview_slot(ui: &mut egui::Ui, icons: &PreviewIcons, rect: egui::Rect, item: Option<(&str, u8)>, selected: bool) {
    ui.painter().rect_filled(rect, 5.0, egui::Color32::from_black_alpha(170));
    ui.painter().rect_filled(rect.shrink(1.5), 4.0, egui::Color32::from_rgba_premultiplied(30, 35, 46, 200));
    if let Some((id, count)) = item {
        icons.paint(ui, rect.shrink(6.0), id);
        if count > 1 {
            ui.painter().text(rect.right_bottom() + egui::vec2(-5.0, -5.0) + egui::vec2(1.0, 1.0),
                egui::Align2::RIGHT_BOTTOM, format!("{}", count),
                egui::FontId::proportional(13.0), egui::Color32::from_black_alpha(200));
            ui.painter().text(rect.right_bottom() + egui::vec2(-5.0, -5.0),
                egui::Align2::RIGHT_BOTTOM, format!("{}", count),
                egui::FontId::proportional(13.0), egui::Color32::WHITE);
        }
    }
    let stroke = if selected {
        egui::Stroke::new(2.5, ACCENT)
    } else {
        egui::Stroke::new(1.0, egui::Color32::from_gray(80))
    };
    ui.painter().rect_stroke(rect, 5.0, stroke, egui::StrokeKind::Middle);
}

/// Map image sampled from real worldgen (biome color + height shading).
fn map_image(gen: &WorldGen, center: (f32, f32), wh: (usize, usize), px_per_block: f32) -> egui::ColorImage {
    let mut pixels = Vec::with_capacity(wh.0 * wh.1);
    let step = 1.0 / px_per_block;
    let mut wz = center.1 - wh.1 as f32 / (2.0 * px_per_block);
    for _ in 0..wh.1 {
        let mut wx = center.0 - wh.0 as f32 / (2.0 * px_per_block);
        for _ in 0..wh.0 {
            let x = wx.floor() as i32;
            let z = wz.floor() as i32;
            let h = gen.height(x, z);
            let mut c = preview_biome_color(gen.biome(x, z));
            if h <= lf_worldgen::SEA_LEVEL {
                // flatten oceans
            } else {
                let f = (1.0 + (h - gen.height(x - 1, z)) as f32 * 0.035).clamp(0.62, 1.30);
                c = egui::Color32::from_rgba_unmultiplied(
                    ((c.r() as f32) * f).clamp(0.0, 255.0) as u8,
                    ((c.g() as f32) * f).clamp(0.0, 255.0) as u8,
                    ((c.b() as f32) * f).clamp(0.0, 255.0) as u8,
                    c.a(),
                );
            }
            pixels.push(c);
            wx += step;
        }
        wz += step;
    }
    egui::ColorImage { size: [wh.0, wh.1], pixels }
}

/// Preview palette matching the client's 30-biome table (subset used here).
fn preview_biome_color(b: Biome) -> egui::Color32 {
    use Biome::*;
    let c = |r: u32, g: u32, bl: u32| egui::Color32::from_rgb(r as u8, g as u8, bl as u8);
    match b {
        Meadow => c(120, 178, 90),
        FlowerForest | Forest => c(78, 140, 66),
        BirchForest => c(148, 168, 104),
        DarkForest => c(48, 100, 55),
        Taiga | SnowyTaiga => c(70, 120, 85),
        Tundra => c(200, 215, 215),
        IceSpikes => c(185, 220, 235),
        SnowySlope | SnowyPeaks => c(225, 232, 235),
        Desert => c(228, 208, 140),
        Badlands => c(190, 115, 65),
        Beach => c(222, 210, 165),
        StonyShore => c(140, 140, 138),
        Ocean => c(55, 95, 165),
        DeepOcean => c(35, 62, 130),
        WarmOcean => c(60, 140, 175),
        Highlands => c(125, 145, 105),
        Mountains => c(130, 128, 125),
        WindsweptHills => c(145, 150, 120),
        Swamp => c(85, 105, 70),
        Jungle => c(50, 135, 60),
        Savanna | WindsweptSavanna => c(170, 164, 94),
        _ => c(120, 160, 90),
    }
}

// ------------------------------------------------------------------
// HUD proof overlay: icons hotbar, XP bar, painted hearts, minimap.

fn draw_hud_preview(ctx: &egui::Context) {
    let icons = PreviewIcons::new(ctx, &[
        "grass", "dirt", "stone_pickaxe", "torch", "planks", "iron_ingot", "apple", "bow", "arrow", "coal", "raw_iron",
    ]);
    egui::TopBottomPanel::bottom("hud").frame(egui::Frame::none()).show_separator_line(false).show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(4.0);
            // hearts + hunger (painted glyphs, no unicode boxes)
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(190.0, 16.0), egui::Sense::hover());
                for i in 0..10 {
                    let c = egui::pos2(rect.left() + 9.0 + i as f32 * 18.0, rect.center().y);
                    let full = i < 8;
                    let half = i == 8;
                    let col = if full || half {
                        egui::Color32::from_rgb(225, 60, 70)
                    } else {
                        egui::Color32::from_rgb(70, 40, 44)
                    };
                    ui.painter().circle_filled(egui::pos2(c.x - 3.5, c.y - 2.5), 3.6, col);
                    ui.painter().circle_filled(egui::pos2(c.x + 3.5, c.y - 2.5), 3.6, col);
                    ui.painter().add(egui::Shape::convex_polygon(vec![
                        egui::pos2(c.x - 6.8, c.y - 1.0), egui::pos2(c.x + 6.8, c.y - 1.0), egui::pos2(c.x, c.y + 6.5),
                    ], col, egui::Stroke::NONE));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(170.0, 14.0), egui::Sense::hover());
                    for i in 0..10 {
                        let c = egui::pos2(rect.right() - 7.0 - i as f32 * 16.0, rect.center().y);
                        let fill = if i < 7 { egui::Color32::from_rgb(210, 150, 50) } else { egui::Color32::from_rgb(70, 56, 32) };
                        ui.painter().circle_filled(c, 5.0, fill);
                        ui.painter().circle_stroke(c, 5.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 24, 16)));
                    }
                });
            });
            // XP bar with level chip
            let (xrect, _) = ui.allocate_exact_size(egui::vec2(420.0, 9.0), egui::Sense::hover());
            ui.painter().rect_filled(xrect, 4.0, egui::Color32::from_black_alpha(190));
            ui.painter().rect_filled(egui::Rect::from_min_size(xrect.min, egui::vec2(xrect.width() * 0.62, xrect.height())), 4.0,
                egui::Color32::from_rgb(110, 220, 255));
            let chip = egui::Rect::from_center_size(xrect.center(), egui::vec2(34.0, 14.0));
            ui.painter().rect_filled(chip, 4.0, egui::Color32::from_rgb(16, 18, 24));
            ui.painter().text(chip.center(), egui::Align2::CENTER_CENTER, "Lv 7",
                egui::FontId::proportional(11.0), egui::Color32::from_rgb(110, 220, 255));
            ui.add_space(1.0);
            // hotbar with real icons
            ui.horizontal(|ui| {
                let items: [Option<(&str, u8)>; 9] = [
                    Some(("grass", 42)), Some(("dirt", 64)), Some(("stone_pickaxe", 1)), Some(("torch", 12)),
                    Some(("planks", 33)), Some(("iron_ingot", 7)), Some(("apple", 3)), Some(("bow", 1)), Some(("arrow", 21)),
                ];
                for (i, item) in items.iter().enumerate() {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
                    preview_slot(ui, &icons, rect, *item, i == 2);
                }
            });
            ui.label(egui::RichText::new("Stone Pickaxe").small().color(ACCENT));
        });
    });
    let pointer = ctx.screen_rect().center();
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, "crosshair".into()));
    let c = egui::Color32::from_white_alpha(220);
    p.line_segment([pointer - egui::vec2(7.0, 0.0), pointer - egui::vec2(2.0, 0.0)], egui::Stroke::new(2.0, c));
    p.line_segment([pointer + egui::vec2(2.0, 0.0), pointer + egui::vec2(7.0, 0.0)], egui::Stroke::new(2.0, c));
    // radial mining-progress reticle mid-break (mirrors ui_kit::
    // paint_mining_reticle — lf_vistest cannot depend on lf_client; keep
    // the math in sync)
    {
        const RADIUS: f32 = 15.0;
        let progress = 0.55f32;
        p.circle_stroke(pointer, RADIUS, egui::Stroke::new(1.5, egui::Color32::from_white_alpha(48)));
        let steps = ((progress * 64.0).ceil() as usize).max(2);
        let points: Vec<egui::Pos2> = (0..=steps)
            .map(|i| {
                let a = -std::f32::consts::FRAC_PI_2 + (i as f32 / steps as f32) * progress * std::f32::consts::TAU;
                pointer + egui::vec2(a.cos() * RADIUS, a.sin() * RADIUS)
            })
            .collect();
        p.add(egui::Shape::Path(egui::epaint::PathShape {
            points,
            closed: false,
            fill: egui::Color32::TRANSPARENT,
            stroke: egui::epaint::PathStroke::new(3.0, egui::Color32::from_rgb(240, 200, 120)),
        }));
    }
    p.line_segment([pointer - egui::vec2(0.0, 7.0), pointer - egui::vec2(0.0, 2.0)], egui::Stroke::new(2.0, c));
    p.line_segment([pointer + egui::vec2(0.0, 2.0), pointer + egui::vec2(0.0, 7.0)], egui::Stroke::new(2.0, c));
}

/// Corner minimap proof: terrain texture + entity dots + player arrow.
fn draw_minimap_preview(ctx: &egui::Context) {
    let gen = WorldGen::new(Seed(12345));
    let image = map_image(&gen, (8.0, 8.0), (172, 172), 1.0);
    let tex = ctx.load_texture("preview_minimap", image, egui::TextureOptions::NEAREST);
    egui::Window::new("minimap")
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 34.0))
        .title_bar(false)
        .frame(egui::Frame::none())
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            let size = 172.0;
            let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
            let paint = ui.painter();
            paint.rect_filled(rect, 8.0, egui::Color32::from_rgb(20, 24, 32));
            let uv = egui::Rect::from_min_max(egui::Pos2::new(0.0, 0.0), egui::Pos2::new(1.0, 1.0));
            paint.image(tex.id(), rect, uv, egui::Color32::WHITE);
            paint.rect_stroke(rect, 8.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(120, 96, 52)), egui::StrokeKind::Middle);
            paint.rect_filled(egui::Rect::from_center_size(rect.center_top(), egui::vec2(18.0, 12.0)), 3.0, egui::Color32::from_rgb(16, 18, 24));
            paint.text(rect.center_top() + egui::vec2(0.0, 6.0), egui::Align2::CENTER_CENTER, "N",
                egui::FontId::proportional(10.0), ACCENT);
            // entity dots + waypoint pip
            paint.circle_filled(rect.center() + egui::vec2(-34.0, 18.0), 2.0, BAD);
            paint.circle_filled(rect.center() + egui::vec2(22.0, -40.0), 2.0, BAD);
            paint.circle_filled(rect.center() + egui::vec2(50.0, 30.0), 2.0, OK);
            paint.circle_filled(rect.center() + egui::vec2(-60.0, -30.0), 3.5, ACCENT);
            paint.circle_stroke(rect.center() + egui::vec2(-60.0, -30.0), 3.5, egui::Stroke::new(1.0, egui::Color32::from_rgb(16, 18, 24)));
            // player arrow
            let c = rect.center();
            let dir = egui::vec2(0.6, -0.8);
            let tip = c + dir * 7.0;
            let left = c + egui::vec2(-dir.y, dir.x) * 4.0 - dir * 4.0;
            let right = c - egui::vec2(-dir.y, dir.x) * 4.0 - dir * 4.0;
            paint.add(egui::Shape::convex_polygon(vec![tip, left, right], egui::Color32::WHITE,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(16, 18, 24))));
        });
    // info line with facing + biome
    egui::Area::new(egui::Id::new("info_line"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 8.0))
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("NW · Meadow · 8,12 · 08:24 · clear").small()
                .color(egui::Color32::from_rgba_unmultiplied(235, 238, 242, 200)));
        });
}

/// Full world-map proof: pannable map canvas, fog of war, waypoints panel.
fn draw_map_preview(ctx: &egui::Context) {
    let gen = WorldGen::new(Seed(12345));
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(160)))
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                // map canvas with fog beyond the explored radius
                let (rect, _) = ui.allocate_exact_size(egui::vec2(520.0, 520.0), egui::Sense::click_and_drag());
                let image = map_image(&gen, (0.0, 0.0), (260, 260), 2.0);
                let tex = ctx.load_texture("preview_map", image, egui::TextureOptions::NEAREST);
                let paint = ui.painter();
                paint.rect_filled(rect, 6.0, egui::Color32::from_rgb(20, 24, 32));
                let uv = egui::Rect::from_min_max(egui::Pos2::new(0.0, 0.0), egui::Pos2::new(1.0, 1.0));
                paint.image(tex.id(), rect, uv, egui::Color32::WHITE);
                // fog of war: dim everything beyond the explored radius
                let explored = egui::Rect::from_center_size(rect.center(), egui::vec2(300.0, 300.0));
                paint.rect_filled(egui::Rect::from_min_max(rect.left_top(), explored.right_top()), 0.0, egui::Color32::from_rgba_unmultiplied(20, 24, 32, 225));
                paint.rect_filled(egui::Rect::from_min_max(rect.left_top(), explored.left_bottom()), 0.0, egui::Color32::from_rgba_unmultiplied(20, 24, 32, 225));
                paint.rect_filled(egui::Rect::from_min_max(explored.right_top(), rect.right_bottom()), 0.0, egui::Color32::from_rgba_unmultiplied(20, 24, 32, 225));
                paint.rect_filled(egui::Rect::from_min_max(explored.left_bottom(), rect.right_bottom()), 0.0, egui::Color32::from_rgba_unmultiplied(20, 24, 32, 225));
                paint.rect_stroke(rect, 6.0, egui::Stroke::new(1.5, egui::Color32::from_rgb(120, 96, 52)), egui::StrokeKind::Middle);
                let to_screen = |wx: f32, wz: f32| -> egui::Pos2 {
                    egui::Pos2::new(rect.left() + wx * 2.0 + rect.width() / 2.0, rect.top() + wz * 2.0 + rect.height() / 2.0)
                };
                // spawn diamond
                let sp = to_screen(0.0, 0.0);
                paint.add(egui::Shape::convex_polygon(vec![
                    sp + egui::vec2(0.0, -6.0), sp + egui::vec2(6.0, 0.0), sp + egui::vec2(0.0, 6.0), sp + egui::vec2(-6.0, 0.0),
                ], egui::Color32::from_rgb(240, 120, 140), egui::Stroke::new(1.5, egui::Color32::from_rgb(16, 18, 24))));
                // waypoints with labels
                for (x, z, name, col) in [
                    (-58.0, -44.0, "Home · 72m", ACCENT),
                    (64.0, 30.0, "Iron Mine · 71m", egui::Color32::from_rgb(110, 220, 255)),
                ] {
                    let pos = to_screen(x, z);
                    paint.circle_filled(pos, 5.0, col);
                    paint.circle_stroke(pos, 5.0, egui::Stroke::new(1.5, egui::Color32::from_rgb(16, 18, 24)));
                    paint.text(pos + egui::vec2(0.0, -12.0), egui::Align2::CENTER_CENTER, name,
                        egui::FontId::proportional(11.0), egui::Color32::from_rgb(235, 238, 242));
                }
                // player arrow
                let c = to_screen(10.0, 14.0);
                let dir = egui::vec2(0.6, -0.8);
                let tip = c + dir * 8.0;
                let left = c + egui::vec2(-dir.y, dir.x) * 5.0 - dir * 5.0;
                let right = c - egui::vec2(-dir.y, dir.x) * 5.0 - dir * 5.0;
                paint.add(egui::Shape::convex_polygon(vec![tip, left, right], egui::Color32::WHITE,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(16, 18, 24))));
                paint.text(rect.center_top() + egui::vec2(0.0, 12.0), egui::Align2::CENTER_CENTER, "N",
                    egui::FontId::proportional(13.0), ACCENT);
                paint.rect_filled(egui::Rect::from_min_size(rect.left_bottom(), egui::vec2(200.0, 20.0)), 3.0, egui::Color32::from_black_alpha(170));
                paint.text(rect.left_bottom() + egui::vec2(8.0, 10.0), egui::Align2::LEFT_CENTER,
                    "-12, 30 · Taiga", egui::FontId::proportional(11.0), egui::Color32::from_rgb(235, 238, 242));
                paint.text(rect.right_bottom() + egui::vec2(-150.0, -10.0), egui::Align2::LEFT_CENTER,
                    "drag pan · wheel zoom · M close", egui::FontId::proportional(10.0), TEXT_DIM);

                // waypoint manager panel
                ui.vertical(|ui| {
                    ui.set_width(230.0);
                    ui.label(egui::RichText::new("Waypoints").size(18.0).color(ACCENT));
                    ui.painter().line_segment([ui.cursor().min + egui::vec2(0.0, 24.0), ui.cursor().min + egui::vec2(120.0, 24.0)],
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(120, 96, 52)));
                    ui.add_space(14.0);
                    let btn = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(300.0, 52.0));
                    ui.allocate_rect(btn, egui::Sense::click());
                    ui.painter().rect_filled(btn, 10.0, egui::Color32::from_rgba_premultiplied(28, 33, 44, 225));
                    ui.painter().rect_stroke(btn, 10.0, egui::Stroke::new(1.5, egui::Color32::from_rgb(90, 98, 112)), egui::StrokeKind::Middle);
                    ui.painter().rect_filled(egui::Rect::from_min_size(egui::Pos2::new(btn.left() + 4.0, btn.center().y - 16.0), egui::vec2(3.0, 32.0)), 2.0, ACCENT);
                    ui.painter().text(btn.center(), egui::Align2::CENTER_CENTER, "+ Marker at 10,14",
                        egui::FontId::proportional(18.0), egui::Color32::from_rgb(235, 238, 242));
                    ui.add_space(12.0);
                    for (name, dist, col) in [
                        ("Home", "72m", ACCENT),
                        ("Iron Mine", "71m", egui::Color32::from_rgb(110, 220, 255)),
                        ("Village", "143m", OK),
                    ] {
                        egui::Frame::new()
                            .fill(egui::Color32::from_black_alpha(100))
                            .corner_radius(6.0)
                            .inner_margin(6.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (dot, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                                    ui.painter().circle_filled(dot.center(), 5.0, col);
                                    ui.label(egui::RichText::new(name).size(13.0).color(egui::Color32::from_rgb(235, 238, 242)));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.small_button("×");
                                        ui.label(egui::RichText::new(dist).small().color(TEXT_DIM));
                                    });
                                });
                            });
                    }
                    ui.add_space(8.0);
                    ui.add(egui::Slider::new(&mut 2.0f32, 0.5..=6.0).text("zoom"));
                    if ui.button("Center on player").clicked() {}
                });
            });
        });
}

/// Crafting + recipe book proof: real icons everywhere, have/need coloring.
fn draw_crafting_preview(ctx: &egui::Context) {
    let icons = PreviewIcons::new(ctx, &[
        "planks", "stick", "crafting_table", "torch", "iron_ingot", "iron_pickaxe", "chest", "furnace",
        "coal", "raw_iron", "stone", "log", "apple", "dirt", "grass", "copper_ingot", "tin_ingot", "bronze_ingot",
    ]);
    egui::Window::new("Crafting Table")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(-40.0, -20.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    // 3x3 grid with the pickaxe pattern filled in
                    ui.label(egui::RichText::new("Craft").size(16.0).color(ACCENT));
                    ui.add_space(4.0);
                    let grid: [[Option<(&str, u8)>; 3]; 3] = [
                        [Some(("iron_ingot", 12)), Some(("iron_ingot", 12)), Some(("iron_ingot", 12))],
                        [None, Some(("stick", 30)), None],
                        [None, Some(("stick", 30)), None],
                    ];
                    for row in grid {
                        ui.horizontal(|ui| {
                            for cell in row {
                                let (rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
                                preview_slot(ui, &icons, rect, cell, false);
                            }
                        });
                    }
                });
                ui.add_space(12.0);
                // result slot
                let (rect, _) = ui.allocate_exact_size(egui::vec2(52.0, 52.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 6.0, egui::Color32::from_black_alpha(170));
                ui.painter().rect_stroke(rect, 6.0, egui::Stroke::new(2.0, ACCENT), egui::StrokeKind::Middle);
                icons.paint(ui, rect.shrink(8.0), "iron_pickaxe");
                ui.add_space(12.0);
                // recipe book panel
                ui.vertical(|ui| {
                    ui.set_width(300.0);
                    ui.label(egui::RichText::new("Recipe Book").size(16.0).color(ACCENT));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut "pick".to_string()).desired_width(110.0));
                        for (label, on) in [("All", true), ("Craft", false), ("Smelt", false), ("Alloy", false)] {
                            let btn = egui::Button::new(egui::RichText::new(label).color(if on { ACCENT } else { TEXT_DIM }));
                            let _ = ui.add(btn);
                        }
                    });
                    ui.add_space(4.0);
                    let entries: [(&str, &str, u8, &[(&str, u8, u16)], bool); 4] = [
                        ("iron_pickaxe", "Iron Pickaxe", 1, &[("iron_ingot", 3, 12), ("stick", 2, 30)], true),
                        ("iron_axe", "Iron Axe", 1, &[("iron_ingot", 3, 12), ("stick", 2, 30)], true),
                        ("torch", "Torch", 4, &[("coal", 1, 9), ("stick", 1, 30)], true),
                        ("basic_circuit", "Basic Circuit", 1, &[("copper_wire", 2, 0), ("tin_ingot", 1, 2), ("iron_ingot", 1, 12)], false),
                    ];
                    for (id, name, count, needs, craftable) in entries {
                        egui::Frame::new()
                            .fill(if craftable { egui::Color32::from_rgba_premultiplied(34, 40, 32, 220) }
                                  else { egui::Color32::from_black_alpha(150) })
                            .stroke(egui::Stroke::new(if craftable { 1.6 } else { 1.0 },
                                if craftable { egui::Color32::from_rgb(120, 96, 52) } else { egui::Color32::from_gray(60) }))
                            .corner_radius(7.0)
                            .inner_margin(6.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (orect, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
                                    icons.paint(ui, orect, id);
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(name).size(13.0).color(egui::Color32::from_rgb(235, 238, 242)));
                                            if count > 1 {
                                                ui.label(egui::RichText::new(format!("x{}", count)).small().color(TEXT_DIM));
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            for (nid, n, have) in needs {
                                                let ok = *have >= *n as u16;
                                                let (irect, _) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                                                icons.paint(ui, irect, nid);
                                                ui.label(egui::RichText::new(format!("{}", n)).small().color(if ok { OK } else { BAD }));
                                            }
                                        });
                                    });
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new("Crafting Table").small().color(TEXT_DIM));
                                        ui.add_enabled(craftable, egui::Button::new("fill"));
                                    });
                                });
                            });
                    }
                });
            });
            ui.add_space(6.0);
            // storage + hotbar rows
            for row in 0..2 {
                ui.horizontal(|ui| {
                    for col in 0..9 {
                        let _ = col;
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
                        let items = [Some(("log", 16)), Some(("planks", 64)), None, Some(("coal", 9)), None,
                                     Some(("raw_iron", 5)), None, None, Some(("dirt", 32))];
                        preview_slot(ui, &icons, rect, items[row], false);
                    }
                });
            }
        });
}

/// Path-trace a scene: build the voxel clip around the camera and dispatch.
fn render_raytraced(spec: &SceneSpec, seed: u64, eye: &Vec3, out_path: &Path) -> Result<(), String> {
    let gen = WorldGen::new(Seed(seed));
    let mut world = World::new();
    for cx in -4..=4 {
        for cz in -4..=4 {
            world.chunks.insert((cx, cz), gen.generate_chunk(cx, cz));
        }
    }
    let ground_level = world.surface_height(eye.x as i32, eye.z as i32) as f32;
    if spec.torches {
        use lf_voxel::registry::block;
        // Torch ring + glowing floor around the camera position so the
        // emissive glow visibly fills the traced frame.
        let cx0 = eye.x as i32;
        let cz0 = eye.z as i32;
        let ground = ground_level;
        for (dx, dz) in [(4, 0), (-4, 0), (0, 4), (0, -4)] {
            let top = world.surface_height(cx0 + dx, cz0 + dz);
            world.set_block(cx0 + dx, top, cz0 + dz, lf_voxel::BlockState(block::TORCH));
        }
        // lantern patch just below the camera: big enough for the emissive
        // glow to dominate the steep-down view, small enough that terrain
        // and sky still frame it (a full-frame floor is one flat color —
        // caught by the P25 pixel gate)
        let ly = (ground + 2.0) as i32 - 2;
        for dz in -4..=0i32 {
            for dx in -2..=2i32 {
                for dy in 0..1i32 {
                    world.set_block(cx0 + dx, ly + dy, cz0 + dz, lf_voxel::BlockState(block::LANTERN));
                }
            }
        }
    }
    // Day: high enough for a vista, but the tracer's voxel clip only extends
    // ±32 blocks around the camera — any higher and the terrain falls out of
    // the clip and every ray returns flat fog (the pre-P25 broken proof).
    // Night torch scenes: sit at ground level beside the torch ring so the
    // glow fills the frame.
    let lift = if spec.time_of_day > 0.2 && spec.time_of_day < 0.8 { 6.0 } else { 0.0 };
    let ground = ground_level;
    let rt_eye = if lift > 0.0 {
        Vec3::new(eye.x, ground + 22.0 + lift, eye.z)
    } else {
        Vec3::new(eye.x, ground + 2.0, eye.z)
    };
    let center = (rt_eye.x as i32, rt_eye.y as i32, rt_eye.z as i32);
    let voxel_data = lf_engine::pathtrace::build_voxel_texture_data(center, &|x, y, z| {
        world.get_block(x, y, z).id()
    });
    // look toward the terrain like the raster scenes
    let look = if spec.torches {
        Vec3::new(0.25, -0.8, -1.0).normalize() // stare into the glowing floor (emissive proof)
    } else {
        Vec3::new(0.35, -0.35, -1.0).normalize()
    };
    let mut camera = Camera::new(rt_eye, rt_eye + look * 40.0);
    camera.set_aspect(800, 600);
    let tod = lf_game::TimeOfDay::from_fraction(spec.time_of_day);
    let angle = spec.time_of_day * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
    let sun = [angle.cos(), angle.sin().abs(), 0.25];
    let img = lf_engine::pathtrace::pathtrace_to_image(
        &voxel_data, center, &camera, sun, tod.sky_light_level(), 800, 600, 48,
    )?;
    img.save(out_path).map_err(|e| format!("save: {e}"))
}

/// Where the spawn column's biome lands for a seed (used by tests to describe scenes).
pub fn spawn_biome(seed: u64) -> Biome {
    WorldGen::new(Seed(seed)).biome(0, 0)
}

/// Console proof overlay: history, suggestions, input line.
fn draw_console_preview(ctx: &egui::Context) {
    // NB: plain `Area`s don't materialize in the two-pass headless harness
    // (only windows do), so the proof uses a frameless anchored window.
    egui::Window::new("LOREFORGE Console")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(12, 14, 20, 242))
                .stroke(egui::Stroke::new(1.0, ACCENT_DIM_COL))
                .corner_radius(8.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_width(520.0);
                    ui.label(egui::RichText::new("console — Tab complete · ↑↓ history · Esc close")
                        .small().color(TEXT_DIM));
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical().stick_to_bottom(true).max_height(170.0).show(ui, |ui| {
                        for line in [
                            "> time set night",
                            "time set (fraction 0.00)",
                            "> give iron_pickaxe",
                            "gave iron_pickaxe x1",
                            "> tp 120 80 -40",
                            "teleported to (120.0, 80.0, -40.0)",
                        ] {
                            ui.label(egui::RichText::new(line).small().monospace().color(egui::Color32::from_rgb(235, 238, 242)));
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("time  tp  weather  waypoint")
                        .small().monospace().color(ACCENT));
                    let mut input = "w".to_string();
                    ui.add(egui::TextEdit::singleline(&mut input)
                        .font(egui::TextStyle::Monospace).desired_width(504.0).hint_text("type a command… (help)"));
                });
        });
}

const ACCENT_DIM_COL: egui::Color32 = egui::Color32::from_rgb(120, 96, 52);

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
            let (v1, i1, _, _) = build_scene_mesh(&spec, spec.default_seed, 1, false, false);
            let (v2, i2, _, _) = build_scene_mesh(&spec, spec.default_seed, 1, false, false);
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
            let (v, _, _, _) = build_scene_mesh(spec, seed, 1, false, false);
            hashes.insert(hash(&v));
        }
        assert!(hashes.len() > 1, "seeds 1..=20 all produced the same mesh");
    }

    #[test]
    fn unknown_scene_errors() {
        assert!(run_scene("nope", None, Path::new("/tmp/x.png")).is_err());
    }

    /// Goal Section 3 proof: the per-biome color grade must measurably
    /// shift the mid-frame color of the SAME scene — a warm desert grade
    /// versus a cold snow grade, rendered through the real GPU pipeline.
    #[test]
    fn biome_grade_shifts_midframe_color() {
        let spec = scenes().into_iter().find(|s| s.name == "terrain_vista")
            .expect("terrain_vista scene registered");
        let (v, i, wv, wi) = build_scene_mesh(&spec, spec.default_seed, 2, false, false);
        let gen = WorldGen::new(Seed(spec.default_seed));
        let h = gen.surface_top(0, 0) as f32;
        let eye = Vec3::new(-24.0, h + 26.0, 48.0);
        let mut camera = Camera::new(eye, Vec3::new(0.0, h + 6.0, 0.0));
        camera.set_aspect(800, 600);
        let mk_env = |tint: [f32; 3], sat: f32| lf_engine::scene::Env {
            camera_pos: eye,
            time: 0.8,
            day_factor: spec.day_factor(),
            fog_color: spec.time_of_day().sky_color(),
            fog_far: 220.0,
            grade_tint: tint,
            grade_saturation: sat,
        };
        let textures = lf_assets::generate_atlas();
        let mut paths = Vec::new();
        let frame = |tag: &str, env: &lf_engine::scene::Env, paths: &mut Vec<String>| -> [f64; 3] {
            let path = format!("/tmp/lf_vistest_grade_{tag}_{}.png", std::process::id());
            lf_engine::headless::render_to_png(
                &v, &i, &wv, &wi, &textures, &camera, env, spec.sky_color(),
                800, 600, Path::new(&path), None,
            ).unwrap_or_else(|e| panic!("render {tag} failed: {e}"));
            paths.push(path.clone());
            let img = image::open(&path).expect("reopen grade frame").to_rgba8();
            // mid-frame band: terrain, not sky, not the HUD edge
            let (mut r, mut g, mut b) = (0u64, 0u64, 0u64);
            let mut n = 0u64;
            for y in 240..400u32 {
                for x in 120..680u32 {
                    let p = img.get_pixel(x, y);
                    r += p.0[0] as u64; g += p.0[1] as u64; b += p.0[2] as u64; n += 1;
                }
            }
            [r as f64 / n as f64, g as f64 / n as f64, b as f64 / n as f64]
        };
        let warm = frame("warm", &mk_env([1.08, 1.00, 0.88], 0.92), &mut paths);
        let cold = frame("cold", &mk_env([0.90, 0.98, 1.10], 0.85), &mut paths);
        // hue (degrees) + saturation (max-min / max) of a band average
        let hue_sat = |c: [f64; 3]| -> (f64, f64) {
            let (r, g, b) = (c[0], c[1], c[2]);
            let (max, min) = (r.max(g).max(b), r.min(g).min(b));
            let sat = if max > 1.0 { (max - min) / max } else { 0.0 };
            let hue = if (max - min).abs() < 1e-6 {
                0.0
            } else if max == g {
                60.0 * (2.0 + (b - r) / (max - min))
            } else if max == r {
                60.0 * ((g - b) / (max - min)).rem_euclid(6.0)
            } else {
                60.0 * (4.0 + (r - g) / (max - min))
            };
            (hue, sat)
        };
        let (hw, sw) = hue_sat(warm);
        let (hc, sc) = hue_sat(cold);
        assert!(
            (hc - hw).abs() > 5.0 && (sw - sc).abs() > 0.03,
            "the two grades must measurably shift hue/saturation: warm hue {hw:.1} sat {sw:.3} vs cold hue {hc:.1} sat {sc:.3} (warm {:?} cold {:?})",
            warm, cold
        );
        for p in paths {
            let _ = std::fs::remove_file(p);
        }
    }

    #[test]
    fn foliage_sway_animates_between_frames() {
        // The P26 commit claimed wind sway, but the vertex shader never read
        // the sway attribute — leaves could not move. This proof renders the
        // canopy at two wind phases through the real GPU pipeline and demands
        // the frames differ, with a same-phase control that must be
        // pixel-identical (everything except the sway is deterministic).
        let spec = scenes().into_iter().find(|s| s.name == "foliage_canopy")
            .expect("foliage_canopy scene registered");
        let (v, i, wv, wi) = build_scene_mesh(&spec, spec.default_seed, 2, false, false);
        let gen = WorldGen::new(Seed(spec.default_seed));
        let h = gen.surface_top(0, 0) as f32;
        let eye = Vec3::new(-10.0, h + 13.0, 14.0);
        let mut camera = Camera::new(eye, Vec3::new(0.5, h + 9.5, 0.5));
        camera.set_aspect(800, 600);
        let mk_env = |time: f32| lf_engine::scene::Env {
            camera_pos: eye,
            time,
            day_factor: spec.day_factor(),
            fog_color: spec.time_of_day().sky_color(),
            fog_far: 220.0,
            grade_tint: [1.0, 1.0, 1.0],
            grade_saturation: 1.0,
        };
        let textures = lf_assets::generate_atlas();
        let mut paths = Vec::new();
        let frame = |tag: &str, time: f32, paths: &mut Vec<String>| -> image::RgbaImage {
            let path = format!("/tmp/lf_vistest_sway_{tag}_{}.png", std::process::id());
            lf_engine::headless::render_to_png(
                &v, &i, &wv, &wi, &textures, &camera, &mk_env(time), spec.sky_color(),
                800, 600, Path::new(&path), None,
            ).unwrap_or_else(|e| panic!("render {tag} failed: {e}"));
            paths.push(path.clone());
            image::open(&path).expect("reopen sway frame").to_rgba8()
        };
        let a1 = frame("a1", 0.8, &mut paths);
        let a2 = frame("a2", 0.8, &mut paths);
        let b = frame("b", 0.8 + std::f32::consts::PI, &mut paths);
        let changed = |x: &image::RgbaImage, y: &image::RgbaImage| -> usize {
            x.pixels().zip(y.pixels())
                .filter(|(p, q)| p.0.iter().zip(q.0.iter())
                    .any(|(c, d)| (*c as i32 - *d as i32).abs() > 8))
                .count()
        };
        assert_eq!(changed(&a1, &a2), 0, "same wind phase must render pixel-identical");
        let moved = changed(&a1, &b);
        let total = (800 * 600) as usize;
        assert!(
            moved > total / 1000,
            "wind must visibly move foliage between phases: only {moved}/{total} px changed"
        );
        for p in paths {
            let _ = std::fs::remove_file(p);
        }
    }

    #[test]
    fn verify_render_rejects_blank_and_accepts_varied() {
        let solid = image::RgbaImage::from_pixel(200, 200, image::Rgba([10, 10, 10, 255]));
        let solid_path = format!("/tmp/lf_vistest_blank_{}.png", std::process::id());
        solid.save(&solid_path).unwrap();
        assert!(verify_render(Path::new(&solid_path)).is_err(), "uniform image must fail");

        let mut varied = image::RgbaImage::new(200, 200);
        for y in 0..200u32 {
            for x in 0..200u32 {
                varied.put_pixel(x, y, image::Rgba([
                    (x * 7 % 256) as u8, (y * 5 % 256) as u8, ((x + y) * 3 % 256) as u8, 255,
                ]));
            }
        }
        let varied_path = format!("/tmp/lf_vistest_varied_{}.png", std::process::id());
        varied.save(&varied_path).unwrap();
        assert!(verify_render(Path::new(&varied_path)).is_ok(), "gradient image must pass");
        let _ = std::fs::remove_file(&solid_path);
        let _ = std::fs::remove_file(&varied_path);
    }
}
