//! The playable client: window, input, first-person gameplay, world editing.
//! P1 scope: walk/jump/sprint/sneak/fly, mouse look, break/place with block
//! outline, hotbar selection, F2 screenshots.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use glam::Vec3;
use winit::{
    event::{DeviceEvent, ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, WindowBuilder},
};

use lf_engine::camera::Camera;
use lf_engine::outline::OutlineScene;
use lf_engine::scene::{GpuVertex, MeshBatch, SceneResources};
use lf_game::player::{Player, PlayerInput, EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT};
use lf_voxel::raycast::raycast_voxel;
use lf_voxel::{BlockState, World};
use lf_worldgen::{Seed, WorldGen};

const CHUNK_RADIUS: i32 = 4; // 9x9 chunk columns at boot (streaming in P2)
const REACH: f32 = 6.0;
const LOOK_SENSITIVITY: f32 = 0.0025;
const DAY_SKY: [f64; 4] = [0.53, 0.81, 0.98, 1.0];

/// Blocks available in the hotbar (P1: the six textured blocks).
const HOTBAR: [BlockState; 6] = [
    BlockState::GRASS,
    BlockState::DIRT,
    BlockState::STONE,
    BlockState(4), // sand
    BlockState(5), // mycelium
    BlockState(6), // snow
];

fn hotbar_name(b: BlockState) -> &'static str {
    match b.0 {
        1 => "Stone",
        2 => "Grass",
        3 => "Dirt",
        4 => "Sand",
        5 => "Mycelium",
        6 => "Snow",
        _ => "?",
    }
}

#[derive(Default)]
struct InputState {
    keys: HashMap<KeyCode, bool>,
    mouse_dx: f32,
    mouse_dy: f32,
    break_pressed: bool,
    place_pressed: bool,
    scroll: f32,
    cursor_locked: bool,
}

impl InputState {
    fn held(&self, code: KeyCode) -> bool {
        self.keys.get(&code).copied().unwrap_or(false)
    }
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let event_loop = EventLoop::new().expect("EventLoop failed");
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("LOREFORGE")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
            .build(&event_loop)
            .expect("Window build failed"),
    );

    let mut state = pollster::block_on(GameState::new(window.clone()));

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);
            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    elwt.exit();
                }
                Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                    state.resize(size.width, size.height);
                }
                Event::WindowEvent { event: WindowEvent::Focused(focused), .. } => {
                    if focused && !state.input.cursor_locked {
                        state.lock_cursor();
                    }
                }
                Event::WindowEvent { event: WindowEvent::KeyboardInput { event: key, .. }, .. } => {
                    if let PhysicalKey::Code(code) = key.physical_key {
                        let pressed = key.state == ElementState::Pressed;
                        if pressed {
                            match code {
                                KeyCode::Escape => {
                                    if state.input.cursor_locked {
                                        state.unlock_cursor();
                                    } else {
                                        elwt.exit();
                                    }
                                    return;
                                }
                                KeyCode::KeyF => {
                                    state.player.flying = !state.player.flying;
                                    state.player.velocity = Vec3::ZERO;
                                }
                                KeyCode::F2 => state.take_screenshot(),
                                KeyCode::Digit1 | KeyCode::Digit2 | KeyCode::Digit3
                                | KeyCode::Digit4 | KeyCode::Digit5 | KeyCode::Digit6 => {
                                    let idx = match code {
                                        KeyCode::Digit1 => 0,
                                        KeyCode::Digit2 => 1,
                                        KeyCode::Digit3 => 2,
                                        KeyCode::Digit4 => 3,
                                        KeyCode::Digit5 => 4,
                                        _ => 5,
                                    };
                                    state.hotbar_index = idx;
                                    state.update_title();
                                }
                                _ => {}
                            }
                        }
                        state.input.keys.insert(code, pressed);
                    }
                }
                Event::WindowEvent { event: WindowEvent::MouseInput { state: button_state, button, .. }, .. } => {
                    if !state.input.cursor_locked {
                        state.lock_cursor();
                        return;
                    }
                    let pressed = button_state == ElementState::Pressed;
                    match button {
                        MouseButton::Left => state.input.break_pressed = pressed,
                        MouseButton::Right => state.input.place_pressed = pressed,
                        _ => {}
                    }
                }
                Event::WindowEvent { event: WindowEvent::MouseWheel { delta, .. }, .. } => {
                    let dy = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                    };
                    state.input.scroll += dy;
                }
                Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta: (dx, dy) }, .. } => {
                    if state.input.cursor_locked {
                        state.input.mouse_dx += dx as f32;
                        state.input.mouse_dy += dy as f32;
                    }
                }
                Event::AboutToWait => {
                    state.tick();
                    state.render();
                }
                _ => {}
            }
        })
        .expect("event loop failed");
}

struct GameState {
    window: Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth_view: wgpu::TextureView,
    _depth_texture: wgpu::Texture,
    resources: SceneResources,
    batches: HashMap<(i32, i32), MeshBatch>,
    cpu_meshes: HashMap<(i32, i32), (Vec<GpuVertex>, Vec<u32>)>,
    outline: OutlineScene,
    world: World,
    player: Player,
    input: InputState,
    hotbar_index: usize,
    last_instant: Instant,
    frame: u64,
    screenshot_counter: u32,
}

impl GameState {
    async fn new(window: Arc<winit::window::Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).expect("create surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("request adapter");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Client Device"),
                    required_features: wgpu::Features::TEXTURE_BINDING_ARRAY
                        | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                    required_limits: wgpu::Limits {
                        max_sampled_textures_per_shader_stage: 32,
                        ..wgpu::Limits::default()
                    },
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .expect("request device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(caps.formats[0]);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // World + meshes.
        let mut world = World::new();
        let gen = WorldGen::new(Seed(12345));
        for cx in -CHUNK_RADIUS..=CHUNK_RADIUS {
            for cz in -CHUNK_RADIUS..=CHUNK_RADIUS {
                let col = gen.generate_chunk(cx, cz);
                world.chunks.insert((cx, cz), col);
            }
        }

        let textures = lf_assets::generate_atlas();
        let resources = SceneResources::new(&device, &queue, config.format, &textures);

        let mut batches = HashMap::new();
        let mut cpu_meshes = HashMap::new();
        for cx in -CHUNK_RADIUS..=CHUNK_RADIUS {
            for cz in -CHUNK_RADIUS..=CHUNK_RADIUS {
                let (v, i) = mesh_column_gpu(&world, cx, cz);
                let batch = MeshBatch::new(&device, &resources, &v, &i);
                batches.insert((cx, cz), batch);
                cpu_meshes.insert((cx, cz), (v, i));
            }
        }

        let outline = OutlineScene::new(&device, config.format);
        let (depth_texture, depth_view) = MeshBatch::create_depth_texture(&device, config.width, config.height);

        let spawn_x = 0.5;
        let spawn_z = 0.5;
        let spawn_y = world.surface_height(0, 0) as f32;
        let mut player = Player::new(Vec3::new(spawn_x, spawn_y + 0.5, spawn_z));
        player.yaw = 0.0;

        let mut state = Self {
            window,
            surface,
            device,
            queue,
            config,
            depth_view,
            _depth_texture: depth_texture,
            resources,
            batches,
            cpu_meshes,
            outline,
            world,
            player,
            input: InputState::default(),
            hotbar_index: 0,
            last_instant: Instant::now(),
            frame: 0,
            screenshot_counter: 0,
        };
        state.lock_cursor();
        state.update_title();
        state
    }

    fn lock_cursor(&mut self) {
        if self.window.set_cursor_grab(CursorGrabMode::Locked).is_ok() {
            self.window.set_cursor_visible(false);
            self.input.cursor_locked = true;
        }
    }

    fn unlock_cursor(&mut self) {
        let _ = self.window.set_cursor_grab(CursorGrabMode::None);
        self.window.set_cursor_visible(true);
        self.input.cursor_locked = false;
        self.input.keys.clear();
        self.input.break_pressed = false;
        self.input.place_pressed = false;
    }

    fn update_title(&self) {
        let p = &self.player;
        let title = format!(
            "LOREFORGE — {} [{}/6] — pos ({:.1}, {:.1}, {:.1}){} — F fly · F2 shot · Esc release",
            hotbar_name(HOTBAR[self.hotbar_index]),
            self.hotbar_index + 1,
            p.position.x, p.position.y, p.position.z,
            if p.flying { " · FLYING" } else { "" },
        );
        self.window.set_title(&title);
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            let (texture, view) = MeshBatch::create_depth_texture(&self.device, width, height);
            self._depth_texture = texture;
            self.depth_view = view;
        }
    }

    fn tick(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_instant).as_secs_f32().min(0.25);
        self.last_instant = now;
        self.frame += 1;

        let input = PlayerInput {
            forward: self.input.held(KeyCode::KeyW),
            back: self.input.held(KeyCode::KeyS),
            left: self.input.held(KeyCode::KeyA),
            right: self.input.held(KeyCode::KeyD),
            jump: self.input.held(KeyCode::Space),
            sneak: self.input.held(KeyCode::ShiftLeft),
            sprint: self.input.held(KeyCode::ControlLeft),
            fly_up: self.input.held(KeyCode::Space),
            fly_down: self.input.held(KeyCode::ShiftLeft),
            yaw_delta: -self.input.mouse_dx * LOOK_SENSITIVITY,
            pitch_delta: -self.input.mouse_dy * LOOK_SENSITIVITY,
        };
        self.input.mouse_dx = 0.0;
        self.input.mouse_dy = 0.0;

        // Hotbar scrolling.
        if self.input.scroll.abs() >= 1.0 {
            let steps = self.input.scroll.signum() as i32;
            self.input.scroll = 0.0;
            let len = HOTBAR.len() as i32;
            self.hotbar_index = ((self.hotbar_index as i32 + steps).rem_euclid(len)) as usize;
            self.update_title();
        }

        let mut did_edit = false;
        self.player.update(dt, &input, &self.world);

        // Targeting.
        let eye = self.player.eye_position();
        let look = self.player.look_dir();
        let target = raycast_voxel(eye, look, REACH, |pos| {
            self.world.get_block(pos.x, pos.y, pos.z) != BlockState::AIR
        });
        self.outline.set_target(&self.device, target.map(|(pos, _)| (pos.x, pos.y, pos.z)));

        // Break (hold to repeat every frame is too fast; one per press for P1).
        if self.input.break_pressed {
            if let Some((pos, _)) = target {
                if self.world.set_block(pos.x, pos.y, pos.z, BlockState::AIR).is_some() {
                    self.remesh_around(pos.x, pos.z);
                    did_edit = true;
                    self.input.break_pressed = false; // one block per click
                }
            }
        }
        // Place on the face we hit.
        if self.input.place_pressed {
            if let Some((pos, normal)) = target {
                let place = pos + normal;
                if !self.block_intersects_player(place) {
                    let block = HOTBAR[self.hotbar_index];
                    if self.world.set_block(place.x, place.y, place.z, block).is_some() {
                        self.remesh_around(place.x, place.z);
                        did_edit = true;
                        self.input.place_pressed = false; // one block per click
                    }
                }
            }
        }

        if did_edit || self.frame % 60 == 0 {
            self.update_title();
        }
    }

    fn block_intersects_player(&self, block: glam::IVec3) -> bool {
        let p = &self.player;
        let (min_b, max_b) = (
            Vec3::new(block.x as f32, block.y as f32, block.z as f32),
            Vec3::new(block.x as f32 + 1.0, block.y as f32 + 1.0, block.z as f32 + 1.0),
        );
        let min_p = Vec3::new(p.position.x - PLAYER_HALF_WIDTH, p.position.y, p.position.z - PLAYER_HALF_WIDTH);
        let max_p = Vec3::new(p.position.x + PLAYER_HALF_WIDTH, p.position.y + PLAYER_HEIGHT, p.position.z + PLAYER_HALF_WIDTH);
        min_b.x < max_p.x && max_b.x > min_p.x
            && min_b.y < max_p.y && max_b.y > min_p.y
            && min_b.z < max_p.z && max_b.z > min_p.z
    }

    /// Rebuild the GPU mesh for the chunk column at a block, plus neighbors
    /// when the block sits on a column border.
    fn remesh_around(&mut self, x: i32, z: i32) {
        let (cx, lx) = (x.div_euclid(16), x.rem_euclid(16));
        let (cz, lz) = (z.div_euclid(16), z.rem_euclid(16));
        let mut to_rebuild = vec![(cx, cz)];
        if lx == 0 { to_rebuild.push((cx - 1, cz)); }
        if lx == 15 { to_rebuild.push((cx + 1, cz)); }
        if lz == 0 { to_rebuild.push((cx, cz - 1)); }
        if lz == 15 { to_rebuild.push((cx, cz + 1)); }
        for (bx, bz) in to_rebuild {
            if !self.batches.contains_key(&(bx, bz)) {
                continue;
            }
            let (v, i) = mesh_column_gpu(&self.world, bx, bz);
            let batch = MeshBatch::new(&self.device, &self.resources, &v, &i);
            self.batches.insert((bx, bz), batch);
            self.cpu_meshes.insert((bx, bz), (v, i));
        }
    }

    fn camera(&self) -> Camera {
        let mut camera = Camera::new(self.player.eye_position(), self.player.eye_position() + self.player.look_dir());
        camera.set_aspect(self.config.width, self.config.height);
        camera
    }

    fn take_screenshot(&mut self) {
        // Render the current view offscreen from the CPU-side meshes.
        let mut vertices: Vec<GpuVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for (v, i) in self.cpu_meshes.values() {
            let base = vertices.len() as u32;
            vertices.extend_from_slice(v);
            indices.extend(i.iter().map(|idx| idx + base));
        }
        self.screenshot_counter += 1;
        let path = std::path::PathBuf::from(format!("shots/screenshot_{}.png", self.screenshot_counter));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let camera = self.camera();
        let textures = lf_assets::generate_atlas();
        match lf_engine::headless::render_to_png(&vertices, &indices, &textures, &camera, DAY_SKY, 1280, 720, &path) {
            Ok(()) => tracing::info!("screenshot saved to {}", path.display()),
            Err(e) => tracing::error!("screenshot failed: {}", e),
        }
    }

    fn render(&mut self) {
        let camera = self.camera();
        for batch in self.batches.values() {
            batch.update_camera(&self.queue, &camera);
        }
        self.outline.update_camera(&self.queue, &camera);

        let output = match self.surface.get_current_texture() {
            Ok(o) => o,
            Err(wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(e) => {
                tracing::warn!("surface error: {:?}", e);
                return;
            }
        };
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Game Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: DAY_SKY[0], g: DAY_SKY[1], b: DAY_SKY[2], a: DAY_SKY[3] }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            let resources = &self.resources;
            for batch in self.batches.values() {
                batch.draw(&mut pass, resources);
            }
            self.outline.draw(&mut pass);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        let _ = self.window.pre_present_notify();
    }
}

/// Mesh one column and convert to GPU vertices.
fn mesh_column_gpu(world: &World, cx: i32, cz: i32) -> (Vec<GpuVertex>, Vec<u32>) {
    let mesh = world.mesh_column(cx, cz, &|b| lf_assets::texture_index_for_block(b.id()));
    let vertices = mesh.vertices.iter().map(|v| GpuVertex {
        position: v.position,
        normal: v.normal,
        tex_coord: v.tex_coord,
        tex_index: v.tex_index,
        ao: v.ao,
    }).collect();
    (vertices, mesh.indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotbar_names_cover_all_slots() {
        for b in HOTBAR {
            assert_ne!(hotbar_name(b), "?", "unnamed hotbar block {}", b.0);
        }
    }

    #[test]
    fn spawn_places_player_above_ground() {
        let mut world = World::new();
        let gen = WorldGen::new(Seed(12345));
        for cx in -1..=1 {
            for cz in -1..=1 {
                world.chunks.insert((cx, cz), gen.generate_chunk(cx, cz));
            }
        }
        let h = world.surface_height(0, 0);
        assert!(h > 0);
        let player = Player::new(Vec3::new(0.5, h as f32 + 0.5, 0.5));
        assert!(player.eye_position().y - EYE_HEIGHT >= h as f32);
        // the block under the spawn feet must be solid
        assert!(world.is_solid(0, h - 1, 0));
    }
}
