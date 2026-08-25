//! The playable client: window, input, first-person gameplay, world editing,
//! chunk streaming and persistence.
//!
//! P2 scope: P1 gameplay plus background chunk generation, view distance,
//! frustum culling, world save/load with autosave, water/trees/caves/ores.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use glam::{Vec3, Vec4};
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
use lf_voxel::registry;
use lf_voxel::world::{PlayerSave, WorldStorage};
use lf_voxel::{BlockState, ChunkColumn, World};
use lf_worldgen::{Seed, WorldGen};

const WORLD_SEED: u64 = 12345;
const WORLD_DIR: &str = "worlds/default";
const VIEW_RADIUS: i32 = 5;      // chunks generated/kept around the player
const UNLOAD_RADIUS: i32 = 8;    // chunks beyond this are dropped (after save)
const BOOT_RADIUS: i32 = 1;      // chunks generated synchronously at boot
const REACH: f32 = 6.0;
const LOOK_SENSITIVITY: f32 = 0.0025;
const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(30);
const DAY_SKY: [f64; 4] = [0.53, 0.81, 0.98, 1.0];

/// Blocks available in the hotbar (P4 replaces this with a real inventory).
const HOTBAR: [u32; 8] = [
    registry::block::GRASS,
    registry::block::DIRT,
    registry::block::STONE,
    registry::block::SAND,
    registry::block::SNOW,
    registry::block::LOG,
    registry::block::LEAVES,
    registry::block::MYCELIUM,
];

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

/// Worker-side view of what chunks are wanted.
#[derive(Clone)]
struct StreamWish {
    center: (i32, i32),
    radius: i32,
    requested: HashSet<(i32, i32)>,
    stop: bool,
}

/// Background chunk generator: picks the nearest wanted chunk and generates
/// it until none are missing.
struct Streamer {
    wish: Arc<Mutex<StreamWish>>,
    rx: mpsc::Receiver<((i32, i32), ChunkColumn)>,
}

impl Streamer {
    fn spawn(seed: u64, skip: HashSet<(i32, i32)>) -> Self {
        let wish = Arc::new(Mutex::new(StreamWish {
            center: (0, 0),
            radius: VIEW_RADIUS,
            requested: skip,
            stop: false,
        }));
        let (tx, rx) = mpsc::channel::<((i32, i32), ChunkColumn)>();
        let worker_wish = Arc::clone(&wish);
        let handle = thread::spawn(move || {
            let gen = WorldGen::new(Seed(seed));
            loop {
                let next = {
                    let mut w = worker_wish.lock().unwrap();
                    if w.stop {
                        return;
                    }
                    nearest_missing(&w)
                };
                match next {
                    Some(pos) => {
                        let col = gen.generate_chunk(pos.0, pos.1);
                        if tx.send((pos, col)).is_err() {
                            return; // receiver dropped
                        }
                        worker_wish.lock().unwrap().requested.insert(pos);
                    }
                    None => thread::sleep(Duration::from_millis(50)),
                }
            }
        });
        let _ = handle;
        Self { wish, rx }
    }

    fn set_center(&self, center: (i32, i32)) {
        let mut w = self.wish.lock().unwrap();
        w.center = center;
        // allow regeneration of unloaded chunks
        let radius = w.radius;
        w.requested.retain(|p| chebyshev(*p, center) <= radius);
    }

    /// Chunks already delivered that fell out of range (so they can stream
    /// again later).
    fn forget(&self, pos: (i32, i32)) {
        self.wish.lock().unwrap().requested.remove(&pos);
    }

    fn shutdown(&self) {
        self.wish.lock().unwrap().stop = true;
    }
}

fn chebyshev(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs().max((a.1 - b.1).abs())
}

fn nearest_missing(w: &StreamWish) -> Option<(i32, i32)> {
    let mut best: Option<(i32, i32)> = None;
    let mut best_d = i32::MAX;
    for dx in -w.radius..=w.radius {
        for dz in -w.radius..=w.radius {
            let pos = (w.center.0 + dx, w.center.1 + dz);
            if w.requested.contains(&pos) {
                continue;
            }
            let d = dx * dx + dz * dz;
            if d < best_d {
                best_d = d;
                best = Some(pos);
            }
        }
    }
    best
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
                    state.shutdown(elwt);
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
                                        state.shutdown(elwt);
                                    }
                                    return;
                                }
                                KeyCode::KeyF => {
                                    state.player.flying = !state.player.flying;
                                    state.player.velocity = Vec3::ZERO;
                                }
                                KeyCode::F2 => state.take_screenshot(),
                                KeyCode::Digit1 | KeyCode::Digit2 | KeyCode::Digit3
                                | KeyCode::Digit4 | KeyCode::Digit5 | KeyCode::Digit6
                                | KeyCode::Digit7 | KeyCode::Digit8 => {
                                    let idx = match code {
                                        KeyCode::Digit1 => 0,
                                        KeyCode::Digit2 => 1,
                                        KeyCode::Digit3 => 2,
                                        KeyCode::Digit4 => 3,
                                        KeyCode::Digit5 => 4,
                                        KeyCode::Digit6 => 5,
                                        KeyCode::Digit7 => 6,
                                        _ => 7,
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
    /// Vertical bounds of each column's mesh, for frustum culling.
    column_bounds: HashMap<(i32, i32), (f32, f32)>,
    outline: OutlineScene,
    world: World,
    storage: WorldStorage,
    dirty: HashSet<(i32, i32)>,
    saved_set: HashSet<(i32, i32)>,
    streamer: Streamer,
    player: Player,
    input: InputState,
    hotbar_index: usize,
    last_instant: Instant,
    next_autosave: Instant,
    frame: u64,
    screenshot_counter: u32,
    running: Arc<AtomicBool>,
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

        // Persistence + world bootstrap.
        let storage = WorldStorage::open(Path::new(WORLD_DIR));
        let saved_set = storage.saved_chunks();
        let gen = WorldGen::new(Seed(WORLD_SEED));
        let mut world = World::new();
        for cx in -BOOT_RADIUS..=BOOT_RADIUS {
            for cz in -BOOT_RADIUS..=BOOT_RADIUS {
                let col = if saved_set.contains(&(cx, cz)) {
                    storage.load_chunk(cx, cz).unwrap_or_else(|| gen.generate_chunk(cx, cz))
                } else {
                    gen.generate_chunk(cx, cz)
                };
                world.chunks.insert((cx, cz), col);
            }
        }

        let textures = lf_assets::generate_atlas();
        let resources = SceneResources::new(&device, &queue, config.format, &textures);

        let mut batches = HashMap::new();
        let mut cpu_meshes = HashMap::new();
        let mut column_bounds = HashMap::new();
        for (cx, cz) in world.chunks.keys().copied().collect::<Vec<_>>() {
            let (v, i) = mesh_column_gpu(&world, cx, cz);
            let (min_y, max_y) = bounds_of(&v);
            batches.insert((cx, cz), MeshBatch::new(&device, &resources, &v, &i));
            cpu_meshes.insert((cx, cz), (v, i));
            column_bounds.insert((cx, cz), (min_y, max_y));
        }

        let outline = OutlineScene::new(&device, config.format);
        let (depth_texture, depth_view) = MeshBatch::create_depth_texture(&device, config.width, config.height);

        // Player: restore from save when present, else spawn on the surface.
        let mut player = match storage.load_player() {
            Some(p) => Player::new(Vec3::from(p.position)).with_look(p.yaw, p.pitch),
            None => {
                let spawn_y = world.surface_height(0, 0) as f32 + 0.2;
                Player::new(Vec3::new(0.5, spawn_y, 0.5))
            }
        };
        player.flying = false;

        // The worker skips chunks that come from the save.
        let mut worker_skip = saved_set.clone();
        worker_skip.extend(world.chunks.keys().copied());
        let streamer = Streamer::spawn(WORLD_SEED, worker_skip);

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
            column_bounds,
            outline,
            world,
            storage,
            dirty: HashSet::new(),
            saved_set,
            streamer,
            player,
            input: InputState::default(),
            hotbar_index: 0,
            last_instant: Instant::now(),
            next_autosave: Instant::now() + AUTOSAVE_INTERVAL,
            frame: 0,
            screenshot_counter: 0,
            running: Arc::new(AtomicBool::new(true)),
        };
        state.lock_cursor();
        state.update_title();
        state
    }

    fn shutdown(&mut self, elwt: &winit::event_loop::EventLoopWindowTarget<()>) {
        self.save_world();
        self.streamer.shutdown();
        self.running.store(false, Ordering::Relaxed);
        elwt.exit();
    }

    fn save_world(&mut self) {
        let dirty: Vec<(i32, i32)> = self.dirty.drain().collect();
        for pos in dirty {
            if let Some(col) = self.world.chunk(pos.0, pos.1) {
                if let Err(e) = self.storage.save_chunk(pos.0, pos.1, col) {
                    tracing::error!("save chunk {:?} failed: {}", pos, e);
                } else {
                    self.saved_set.insert(pos);
                }
            }
        }
        let player = PlayerSave {
            position: self.player.position.to_array(),
            yaw: self.player.yaw,
            pitch: self.player.pitch,
        };
        if let Err(e) = self.storage.save_player(&player) {
            tracing::error!("save player failed: {}", e);
        } else {
            tracing::info!("world saved to {}", WORLD_DIR);
        }
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
            "LOREFORGE — {} [{}/8] — pos ({:.1}, {:.1}, {:.1}) — chunks {} — F fly · F2 shot · Esc release",
            registry::block::name(HOTBAR[self.hotbar_index]),
            self.hotbar_index + 1,
            p.position.x, p.position.y, p.position.z,
            self.batches.len(),
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

    fn player_chunk(&self) -> (i32, i32) {
        (
            self.player.position.x.div_euclid(16.0) as i32,
            self.player.position.z.div_euclid(16.0) as i32,
        )
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
            // Mouse right (dx>0) turns the view right (yaw+); mouse down
            // (dy>0) looks down (pitch-). Matches standard FPS feel.
            yaw_delta: self.input.mouse_dx * LOOK_SENSITIVITY,
            pitch_delta: -self.input.mouse_dy * LOOK_SENSITIVITY,
        };
        self.input.mouse_dx = 0.0;
        self.input.mouse_dy = 0.0;

        if self.input.scroll.abs() >= 1.0 {
            let steps = self.input.scroll.signum() as i32;
            self.input.scroll = 0.0;
            let len = HOTBAR.len() as i32;
            self.hotbar_index = ((self.hotbar_index as i32 + steps).rem_euclid(len)) as usize;
            self.update_title();
        }

        let mut did_edit = false;
        self.player.update(dt, &input, &self.world);

        self.stream_chunks();
        if self.frame % 120 == 0 {
            self.unload_far_chunks();
            self.update_title();
        }

        // Targeting (water is not targetable).
        let eye = self.player.eye_position();
        let look = self.player.look_dir();
        let target = raycast_voxel(eye, look, REACH, |pos| {
            registry::is_targetable(self.world.get_block(pos.x, pos.y, pos.z))
        });
        self.outline.set_target(&self.device, target.map(|(pos, _)| (pos.x, pos.y, pos.z)));

        if self.input.break_pressed {
            if let Some((pos, _)) = target {
                if self.world.set_block(pos.x, pos.y, pos.z, BlockState::AIR).is_some() {
                    self.remesh_around(pos.x, pos.z);
                    did_edit = true;
                    self.input.break_pressed = false; // one block per click
                }
            }
        }
        if self.input.place_pressed {
            if let Some((pos, normal)) = target {
                let place = pos + normal;
                if !self.block_intersects_player(place) {
                    let block = BlockState(HOTBAR[self.hotbar_index]);
                    if self.world.set_block(place.x, place.y, place.z, block).is_some() {
                        self.remesh_around(place.x, place.z);
                        did_edit = true;
                        self.input.place_pressed = false;
                    }
                }
            }
        }
        let _ = did_edit;

        if now >= self.next_autosave {
            self.next_autosave = now + AUTOSAVE_INTERVAL;
            if !self.dirty.is_empty() {
                self.save_world();
            }
        }
    }

    /// Pull finished chunks from the streamer (and load saved ones directly),
    /// meshing a bounded number per frame.
    fn stream_chunks(&mut self) {
        let center = self.player_chunk();
        self.streamer.set_center(center);

        // Saved chunks load straight from disk on the main thread.
        let mut loaded = 0;
        for dx in -VIEW_RADIUS..=VIEW_RADIUS {
            for dz in -VIEW_RADIUS..=VIEW_RADIUS {
                if loaded >= 2 {
                    break;
                }
                let pos = (center.0 + dx, center.1 + dz);
                if self.world.chunk(pos.0, pos.1).is_none() && self.saved_set.contains(&pos) {
                    if let Some(col) = self.storage.load_chunk(pos.0, pos.1) {
                        self.world.chunks.insert(pos, col);
                        self.add_column_batch(pos.0, pos.1);
                        loaded += 1;
                    }
                }
            }
        }

        // Freshly generated chunks from the worker.
        let mut budget = 4;
        while budget > 0 {
            match self.streamer.rx.try_recv() {
                Ok((pos, col)) => {
                    // Re-apply nothing: generated columns are pristine; edits
                    // live only in saved columns.
                    self.world.chunks.insert(pos, col);
                    self.add_column_batch(pos.0, pos.1);
                    budget -= 1;
                }
                Err(_) => break,
            }
        }
    }

    fn add_column_batch(&mut self, cx: i32, cz: i32) {
        let (v, i) = mesh_column_gpu(&self.world, cx, cz);
        let (min_y, max_y) = bounds_of(&v);
        self.batches.insert((cx, cz), MeshBatch::new(&self.device, &self.resources, &v, &i));
        self.cpu_meshes.insert((cx, cz), (v, i));
        self.column_bounds.insert((cx, cz), (min_y, max_y));
    }

    /// Drop far chunks (saving dirty ones first) to bound memory.
    fn unload_far_chunks(&mut self) {
        let center = self.player_chunk();
        let far: Vec<(i32, i32)> = self
            .world
            .chunks
            .keys()
            .copied()
            .filter(|p| chebyshev(*p, center) > UNLOAD_RADIUS)
            .collect();
        for pos in far {
            if self.dirty.remove(&pos) {
                if let Some(col) = self.world.chunk(pos.0, pos.1) {
                    let _ = self.storage.save_chunk(pos.0, pos.1, col);
                    self.saved_set.insert(pos);
                }
            }
            self.world.chunks.remove(&pos);
            self.batches.remove(&pos);
            self.cpu_meshes.remove(&pos);
            self.column_bounds.remove(&pos);
            self.streamer.forget(pos);
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
            let (min_y, max_y) = bounds_of(&v);
            self.batches.insert((bx, bz), MeshBatch::new(&self.device, &self.resources, &v, &i));
            self.cpu_meshes.insert((bx, bz), (v, i));
            self.column_bounds.insert((bx, bz), (min_y, max_y));
            self.dirty.insert((bx, bz));
        }
    }

    fn camera(&self) -> Camera {
        let mut camera = Camera::new(self.player.eye_position(), self.player.eye_position() + self.player.look_dir());
        camera.set_aspect(self.config.width, self.config.height);
        camera
    }

    fn take_screenshot(&mut self) {
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
        let view_proj = camera.build_view_projection_matrix();
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
            let eye = self.player.eye_position();
            for (pos, batch) in self.batches.iter() {
                if let Some(&(min_y, max_y)) = self.column_bounds.get(pos) {
                    if !column_in_view(&view_proj, eye, *pos, min_y, max_y) {
                        continue;
                    }
                }
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

fn bounds_of(vertices: &[GpuVertex]) -> (f32, f32) {
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for v in vertices {
        min = min.min(v.position[1]);
        max = max.max(v.position[1]);
    }
    if vertices.is_empty() {
        (0.0, 0.0)
    } else {
        (min, max)
    }
}

/// Frustum + distance culling for a chunk column, using its mesh bounds.
fn column_in_view(view_proj: &glam::Mat4, eye: Vec3, pos: (i32, i32), min_y: f32, max_y: f32) -> bool {
    // Distance cull: columns past 1.5x view radius can't be visible anyway.
    let center_x = pos.0 as f32 * 16.0 + 8.0;
    let center_z = pos.1 as f32 * 16.0 + 8.0;
    let dx = center_x - eye.x;
    let dz = center_z - eye.z;
    let dist2 = dx * dx + dz * dz;
    let limit = (UNLOAD_RADIUS as f32 * 16.0 * 1.25).powi(2);
    if dist2 > limit {
        return false;
    }

    // Sphere-frustum test (Gribb-Hartmann planes from the view-projection).
    let cy = (min_y + max_y) * 0.5;
    let half_h = (max_y - min_y) * 0.5;
    let radius = half_h.max(11.4); // covers the 16x16 footprint
    let m = view_proj.to_cols_array();
    // planes as (a, b, c, d) with a*x + b*y + c*z + d >= -radius = inside
    let rows = [
        [m[3] + m[0], m[7] + m[4], m[11] + m[8], m[15] + m[12]],  // left
        [m[3] - m[0], m[7] - m[4], m[11] - m[8], m[15] - m[12]],  // right
        [m[3] + m[1], m[7] + m[5], m[11] + m[9], m[15] + m[13]],  // bottom
        [m[3] - m[1], m[7] - m[5], m[11] - m[9], m[15] - m[13]],  // top
        [m[3] + m[2], m[7] + m[6], m[11] + m[10], m[15] + m[14]], // near
        [m[3] - m[2], m[7] - m[6], m[11] - m[10], m[15] - m[14]], // far
    ];
    for p in rows {
        let dist = p[0] * center_x + p[1] * cy + p[2] * center_z + p[3];
        if dist < -radius {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Mat4;

    fn camera_frustum(eye: Vec3, target: Vec3) -> Mat4 {
        let mut cam = Camera::new(eye, target);
        cam.set_aspect(800, 600);
        cam.build_view_projection_matrix()
    }

    #[test]
    fn hotbar_names_cover_all_slots() {
        for b in HOTBAR {
            assert_ne!(registry::block::name(b), "Unknown", "unnamed hotbar block {}", b);
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
        let player = Player::new(Vec3::new(0.5, h as f32 + 0.2, 0.5));
        assert!(player.eye_position().y - EYE_HEIGHT >= h as f32);
        assert!(world.is_solid(0, h - 1, 0));
    }

    #[test]
    fn frustum_culls_columns_behind_and_beside_camera() {
        let vp = camera_frustum(Vec3::new(0.0, 80.0, 0.0), Vec3::new(0.0, 80.0, -10.0));
        let eye = Vec3::new(0.0, 80.0, 0.0);
        // straight ahead: visible
        assert!(column_in_view(&vp, eye, (0, -3), 60.0, 100.0));
        // behind: culled
        assert!(!column_in_view(&vp, eye, (0, 3), 60.0, 100.0));
        // far to the side: culled
        assert!(!column_in_view(&vp, eye, (30, -3), 60.0, 100.0));
    }

    #[test]
    fn nearest_missing_picks_closest_unrequested() {
        let mut w = StreamWish {
            center: (0, 0),
            radius: 2,
            requested: [(0, 0)].into_iter().collect(),
            stop: false,
        };
        let pick = nearest_missing(&w).expect("something missing");
        assert_eq!(chebyshev(pick, (0, 0)), 1, "should pick an adjacent chunk, got {:?}", pick);
        for p in [(0, 1), (1, 0), (-1, 0), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            w.requested.insert(p);
        }
        if let Some(pick) = nearest_missing(&w) {
            assert_eq!(chebyshev(pick, (0, 0)), 2, "ring of 1 exhausted, got {:?}", pick);
        }
    }

    #[test]
    fn edited_chunks_roundtrip_through_storage() {
        let tmp = tempfile::tempdir().unwrap();
        let mut storage = WorldStorage::open(tmp.path());
        let gen = WorldGen::new(Seed(9));
        let mut world = World::new();
        world.chunks.insert((5, 5), gen.generate_chunk(5, 5));
        world.set_block(5 * 16 + 9, 90, 5 * 16 + 3, BlockState(registry::block::LOG)).unwrap();
        storage.save_chunk(5, 5, world.chunk(5, 5).unwrap()).unwrap();

        let loaded = storage.load_chunk(5, 5).unwrap();
        assert_eq!(loaded.get(9, 90, 3), BlockState(registry::block::LOG));
        assert_eq!(storage.saved_chunks(), [(5, 5)].into_iter().collect::<HashSet<_>>());
    }
}
