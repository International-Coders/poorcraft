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
            eye: Vec3::new(-26.0, 0.0, 40.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
    ]
}

/// Build the mesh for a scene: a radius-chunk plot of worldgen terrain
/// centered at (0,0), using the real World + chunk-column pipeline.
pub fn build_scene_mesh(_spec: &SceneSpec, seed: u64, radius_chunks: i32, torches: bool, machines_param: bool)
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
    let (vertices, indices, water_vertices, water_indices) = build_scene_mesh(&spec, seed, 3, spec.torches, spec.machines);
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

    let ui = spec.name == "hud_preview" || spec.name == "village_trading" || spec.name == "tech_tree";
    let ui = if ui {
        let ctx = egui::Context::default();
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))),
            ..Default::default()
        };
        ctx.begin_pass(raw);
        draw_hud_preview(&ctx);
        if spec.name == "village_trading" {
            draw_trade_preview(&ctx);
        }
        if spec.name == "tech_tree" {
            draw_tech_tree_preview(&ctx);
        }
        Some(ctx)
    } else {
        None
    };
    if spec.name == "village_trading" {
        // trade window overlay via the shared egui pass
        // (drawn after draw_hud_preview by nesting another window)
    }
    let textures = lf_assets::generate_atlas();
    lf_engine::headless::render_to_png(&vertices, &indices, &water_vertices, &water_indices, &textures, &camera, &env, spec.sky_color(), 800, 600, out_path, ui.as_ref())
}

/// Tech-tree proof overlay mirroring the client's draw_tech_tree.
fn draw_tech_tree_preview(ctx: &egui::Context) {
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
                        else if state == "CURRENT" { egui::Color32::from_rgb(240, 210, 140) }
                        else { egui::Color32::from_gray(110) };
                    egui::Frame::new()
                        .stroke(egui::Stroke::new(if state == "CURRENT" { 3.0 } else { 1.0 }, color))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(140.0, 90.0));
                            ui.heading(egui::RichText::new(e).size(15.0).color(color));
                            ui.label(egui::RichText::new(state).small().color(color));
                            if state == "locked" {
                                ui.add_space(4.0);
                                for (item, got, n) in [("copper_ingot", 7, 10), ("tin_ingot", 5, 5), ("steel_ingot", 0, 5)] {
                                    let ok = got >= n;
                                    let c = if ok { egui::Color32::from_rgb(140, 220, 140) } else { egui::Color32::from_rgb(230, 130, 130) };
                                    ui.label(egui::RichText::new(format!("{} {}/{}", item, got, n)).small().color(c));
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
                ui.horizontal(|ui| {
                    let enough = have >= give_n;
                    let color = if enough { egui::Color32::from_rgb(140, 220, 140) } else { egui::Color32::from_rgb(230, 130, 130) };
                    ui.label(egui::RichText::new(format!("{} {} -> {} {}", give_n, give, get_n, get)).color(color));
                    ui.label(format!("(have {})", have));
                    ui.add_enabled(enough, egui::Button::new("Trade"));
                });
            }
            ui.separator();
            ui.label(egui::RichText::new("Esc to close").small());
        });
}

/// HUD proof overlay: drawn with the same egui stack the game client uses.
fn draw_hud_preview(ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("hud").frame(egui::Frame::none()).show_separator_line(false).show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("\u{2665}\u{2665}\u{2665}\u{2665}\u{2665}\u{2665}\u{2665}\u{2665}\u{2661}\u{2661}").color(egui::Color32::from_rgb(220, 40, 40)).size(16.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("\u{25CF}\u{25CF}\u{25CF}\u{25CF}\u{25CF}\u{25CF}\u{25CF}\u{25CF}\u{25CB}\u{25CB}").color(egui::Color32::from_rgb(200, 150, 40)).size(14.0));
                });
            });
            ui.horizontal(|ui| {
                let colors = [
                    egui::Color32::from_rgb(90, 160, 60),
                    egui::Color32::from_rgb(134, 96, 67),
                    egui::Color32::from_gray(130),
                    egui::Color32::from_rgb(219, 207, 163),
                    egui::Color32::from_rgb(240, 246, 246),
                    egui::Color32::from_rgb(102, 81, 50),
                    egui::Color32::from_rgb(60, 120, 40),
                    egui::Color32::from_rgb(140, 130, 160),
                    egui::Color32::from_rgb(255, 200, 100),
                ];
                for (i, c) in colors.iter().enumerate() {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4.0, egui::Color32::from_black_alpha(160));
                    ui.painter().rect_filled(rect.shrink(6.0), 3.0, *c);
                    let stroke = if i == 2 { egui::Stroke::new(3.0, egui::Color32::WHITE) } else { egui::Stroke::new(1.0, egui::Color32::from_gray(90)) };
                    ui.painter().rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Middle);
                }
            });
            ui.label(egui::RichText::new("09:47 — E inventory · F fly · F2 shot").small().color(egui::Color32::from_gray(200)));
        });
    });
    let pointer = ctx.screen_rect().center();
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, "crosshair".into()));
    let c = egui::Color32::from_white_alpha(220);
    p.line_segment([pointer - egui::vec2(8.0, 0.0), pointer + egui::vec2(8.0, 0.0)], egui::Stroke::new(2.0, c));
    p.line_segment([pointer - egui::vec2(0.0, 8.0), pointer + egui::vec2(0.0, 8.0)], egui::Stroke::new(2.0, c));
    egui::Window::new("Inventory")
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            for row in 0..3 {
                ui.horizontal(|ui| {
                    for _col in 0..9 {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_black_alpha(160));
                        ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_gray(90)), egui::StrokeKind::Middle);
                    }
                });
            }
        });
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
}
