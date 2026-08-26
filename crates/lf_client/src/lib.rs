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
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, WindowAttributes},
};

pub mod console;
pub mod icons;
pub mod map;
pub mod slots;
pub mod net;
pub mod ui;
pub mod ui_kit;

use lf_engine::camera::Camera;
use lf_engine::outline::OutlineScene;
use lf_engine::scene::{GpuVertex, MeshBatch, SceneResources};
use lf_chronicle::{ChronicleEvent, EventType, SagaGenerator};
use lf_game::research::{Era, ResearchState};
use lf_game::combat::{grant_xp, mitigate, worn_armor_points, Arrow};
use lf_game::crafting::{consume_ingredients, match_recipe};
use lf_game::items::{item_def, tier_durability, ItemKind};
use lf_game::mining::{break_time, tool_satisfies};
use lf_game::mobs::{roll_spawn, MobEntity, MobType};
use lf_npc::{trade_offers, Villager, VillagerJob};
use lf_game::player::{Player, PlayerInput, EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT};
use lf_game::items::tool_damage;
use lf_game::survival::{Inventory, ItemStack, PlayerStats};
use lf_story::{QuestEvent, QuestLog, starter_quests};
use lf_voxel::raycast::raycast_voxel;
use lf_voxel::registry;
use lf_voxel::world::{PlayerSave, WorldStorage};
use lf_voxel::{BlockState, ChunkColumn, World};
use lf_worldgen::{Seed, WorldGen};

use serde::{Deserialize, Serialize};
use ui::EguiPlatform;

const WORLD_SEED: u64 = 12345;
const WORLD_DIR: &str = "worlds/default";
const DEFAULT_VIEW_RADIUS: i32 = 5; // chunks generated/kept around the player
const UNLOAD_RADIUS: i32 = 8;    // chunks beyond this are dropped (after save)
const BOOT_RADIUS: i32 = 1;      // chunks generated synchronously at boot
const REACH: f32 = 6.0;
const LOOK_SENSITIVITY: f32 = 0.0025;
const AUTOSAVE_INTERVAL: Duration = Duration::from_secs(30);
const DAY_SKY: [f64; 4] = [0.53, 0.81, 0.98, 1.0];

/// Blocks available in the hotbar (P4 replaces this with a real inventory).
const HOTBAR: [u32; 9] = [
    registry::block::GRASS,
    registry::block::DIRT,
    registry::block::STONE,
    registry::block::SAND,
    registry::block::SNOW,
    registry::block::LOG,
    registry::block::LEAVES,
    registry::block::MYCELIUM,
    registry::block::TORCH,
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
            radius: DEFAULT_VIEW_RADIUS,
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UiOpen {
    None,
    Title,
    Pause,
    Chat,
    QuestLog,
    Inventory,
    CraftingTable,
    Furnace((i32, i32, i32)),
    Chest((i32, i32, i32)),
    Machine((i32, i32, i32)),
    Trade(usize),
    Book,
    Smithing,
    TechTree,
    Map,
    Console,
    Slots,
    Settings,
    Death,
}

/// State attached to placed functional blocks.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum BlockEntity {
    Furnace(lf_game::smelting::Furnace),
    Chest { slots: Vec<Option<ItemStack>> },
    Generator(lf_game::machines::Generator),
    ElectricFurnace(lf_game::machines::ElectricFurnace),
    Crusher(lf_game::machines::Crusher),
    Assembler(lf_game::machines::Assembler),
}

#[derive(Clone, Debug)]
pub struct MiningState {
    pub pos: (i32, i32, i32),
    pub progress: f32,
    pub total: f32,
}

#[derive(Clone, Debug)]
pub struct ItemDrop {
    pub stack: ItemStack,
    pub position: Vec3,
    pub velocity: Vec3,
    pub age: f32,
}

/// Ray-tracing mode: off, R-key captures only, or a live path-traced view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum RtMode {
    #[default]
    Off,
    Captures,
    Live,
}

impl RtMode {
    pub fn next(self) -> Self {
        match self {
            RtMode::Off => RtMode::Captures,
            RtMode::Captures => RtMode::Live,
            RtMode::Live => RtMode::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RtMode::Off => "Off",
            RtMode::Captures => "Captures (R)",
            RtMode::Live => "Live",
        }
    }
}

/// Quality presets mapping onto the video settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Quality {
    Low,
    Medium,
    High,
}

/// Player-tunable settings, persisted with the world save.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub sensitivity: f32,
    pub invert_y: bool,
    pub fov_degrees: f32,
    pub view_distance: i32,
    pub clouds: bool,
    pub particles: bool,
    pub rt_mode: RtMode,
    pub rt_scale: f32,
    pub volume_master: f32,
    pub volume_sfx: f32,
    pub volume_music: f32,
    pub show_fps: bool,
    #[serde(default = "default_true")]
    pub show_minimap: bool,
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,
}

fn default_true() -> bool {
    true
}

fn default_ui_scale() -> f32 {
    1.0
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sensitivity: 0.0025,
            invert_y: false,
            fov_degrees: 70.0,
            view_distance: 5,
            clouds: true,
            particles: true,
            rt_mode: RtMode::Off,
            rt_scale: 0.25,
            volume_master: 0.8,
            volume_sfx: 0.9,
            volume_music: 0.6,
            show_fps: false,
            show_minimap: true,
            ui_scale: 1.0,
        }
    }
}

impl Settings {
    /// Apply a quality preset to the video-related knobs.
    pub fn apply_quality(&mut self, q: Quality) {
        match q {
            Quality::Low => {
                self.view_distance = 3;
                self.clouds = false;
                self.particles = false;
                self.rt_mode = RtMode::Off;
            }
            Quality::Medium => {
                self.view_distance = 5;
                self.clouds = true;
                self.particles = true;
            }
            Quality::High => {
                self.view_distance = 7;
                self.clouds = true;
                self.particles = true;
            }
        }
    }
}

/// Client-side save extras (inventory/stats/time) next to PlayerSave.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ClientSave {
    pub slots: Vec<Option<ItemStack>>,
    pub health: f32,
    pub hunger: f32,
    pub time_ticks: u64,
    pub block_entities: Vec<((i32, i32, i32), BlockEntity)>,
    pub mobs: Vec<MobEntity>,
    pub kills: u32,
    pub quest_log: Option<QuestLog>,
    pub chronicle: Vec<ChronicleEvent>,
    pub world_type: Option<lf_worldgen::WorldType>,
    pub villagers: Vec<Villager>,
    pub research: Option<ResearchState>,
    pub settings: Option<Settings>,
    #[serde(default)]
    pub waypoints: Vec<map::Waypoint>,
}

struct App {
    state: Option<GameState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(
                WindowAttributes::new()
                    .with_title("LOREFORGE")
                    .with_inner_size(winit::dpi::LogicalSize::new(1280, 720)),
            )
            .expect("Window build failed");
        self.state = Some(pollster::block_on(GameState::new(Arc::new(window))));
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: winit::event::DeviceId, event: DeviceEvent) {
        let Some(state) = &mut self.state else { return };
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            if state.input.cursor_locked && state.ui_open == UiOpen::None {
                state.input.mouse_dx += dx as f32;
                state.input.mouse_dy += dy as f32;
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        let Some(state) = &mut self.state else { return };
        state.tick();
        state.render();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: winit::window::WindowId, event: WindowEvent) {
        let Some(state) = &mut self.state else { return };
        // egui sees events only while a screen is open (the HUD itself is
        // display-only).
        if state.ui_open != UiOpen::None {
            let consumed = state.egui.on_event(&state.window, &event);
            if consumed {
                return;
            }
        }
        match event {
            WindowEvent::CloseRequested => {
                state.shutdown(event_loop);
            }
            WindowEvent::Resized(size) => {
                state.resize(size.width, size.height);
            }
            _ => {}
                        WindowEvent::Focused(focused) => {
                            if focused
                                && !state.input.cursor_locked
                                && state.ui_open == UiOpen::None
                                && state.stats.health > 0.0
                            {
                                state.lock_cursor();
                            }
                        }
                        WindowEvent::KeyboardInput { event: key, .. } => {
                            if let PhysicalKey::Code(code) = key.physical_key {
                                let pressed = key.state == ElementState::Pressed;
                                if pressed {
                                    match code {
                                        KeyCode::Escape => {
                                            match state.ui_open {
                                                UiOpen::Title => {}
                                                UiOpen::Pause => {
                                                    // Esc resumes from pause (used to be a
                                                    // silent no-op only the Resume button fixed)
                                                    state.close_ui();
                                                    return;
                                                }
                                                UiOpen::None if state.stats.health > 0.0 => {
                                                    state.ui_open = UiOpen::Pause;
                                                    state.menu_reveal = 0.0;
                                                    state.unlock_cursor();
                                                    return;
                                                }
                                                UiOpen::None => {}
                                                _ => {
                                                    state.close_ui();
                                                    return;
                                                }
                                            }
                                            if state.ui_open == UiOpen::None && !state.input.cursor_locked {
                                                state.shutdown(event_loop);
                                            }
                                            return;
                                        }
                                        KeyCode::KeyT => {
                                            if state.net.is_some() && state.ui_open == UiOpen::None && state.stats.health > 0.0 {
                                                state.chat_input = Some(String::new());
                                                state.unlock_cursor();
                                                return;
                                            }
                                        }
                                        KeyCode::Backquote => {
                                            if matches!(state.ui_open, UiOpen::None) && state.stats.health > 0.0 {
                                                state.console_open();
                                                return;
                                            }
                                        }
                                        KeyCode::Slash => {
                                            if matches!(state.ui_open, UiOpen::None) && state.stats.health > 0.0 {
                                                state.console_open();
                                                state.console.input = "/".to_string();
                                                return;
                                            }
                                        }
                                        KeyCode::KeyM => {
                                            if matches!(state.ui_open, UiOpen::None | UiOpen::Map) && state.stats.health > 0.0 {
                                                if state.ui_open == UiOpen::Map {
                                                    state.close_ui();
                                                } else {
                                                    state.ui_open = UiOpen::Map;
                                                    state.menu_reveal = 0.0;
                                                    state.map.following = true;
                                                    state.unlock_cursor();
                                                }
                                                return;
                                            }
                                        }
                                        KeyCode::KeyK => {
                                            if matches!(state.ui_open, UiOpen::None | UiOpen::TechTree) && state.stats.health > 0.0 {
                                                if state.ui_open == UiOpen::TechTree {
                                                    state.close_ui();
                                                } else {
                                                    state.ui_open = UiOpen::TechTree;
                                                    state.unlock_cursor();
                                                }
                                                return;
                                            }
                                        }
                                        KeyCode::KeyJ => {
                                            if matches!(state.ui_open, UiOpen::None | UiOpen::QuestLog) && state.stats.health > 0.0 {
                                                if state.ui_open == UiOpen::QuestLog {
                                                    state.close_ui();
                                                } else {
                                                    state.ui_open = UiOpen::QuestLog;
                                                    state.unlock_cursor();
                                                }
                                                return;
                                            }
                                        }
                                        KeyCode::KeyE => {
                                            if state.stats.health > 0.0 {
                                                if state.ui_open == UiOpen::Inventory {
                                                    state.close_ui();
                                                } else if state.ui_open == UiOpen::None {
                                                    state.ui_open = UiOpen::Inventory;
                                                    state.unlock_cursor();
                                                }
                                                return;
                                            }
                                        }
                                        KeyCode::KeyF => {
                                            state.player.flying = !state.player.flying;
                                            state.player.velocity = Vec3::ZERO;
                                        }
                                        KeyCode::F2 => state.take_screenshot(),
                                        KeyCode::F3 => {
                                            state.show_debug = !state.show_debug;
                                        }
                                        KeyCode::KeyR => { if state.settings.rt_mode == crate::RtMode::Captures { state.take_raytraced_screenshot(); } }
                                        KeyCode::Digit1 | KeyCode::Digit2 | KeyCode::Digit3
                                        | KeyCode::Digit4 | KeyCode::Digit5 | KeyCode::Digit6
                                        | KeyCode::Digit7 | KeyCode::Digit8 | KeyCode::Digit9 => {
                                            let idx = match code {
                                                KeyCode::Digit1 => 0,
                                                KeyCode::Digit2 => 1,
                                                KeyCode::Digit3 => 2,
                                                KeyCode::Digit4 => 3,
                                                KeyCode::Digit5 => 4,
                                                KeyCode::Digit6 => 5,
                                                KeyCode::Digit7 => 6,
                                                KeyCode::Digit8 => 7,
                                                _ => 8,
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
                        WindowEvent::MouseInput { state: button_state, button, .. } => {
                            if state.ui_open != UiOpen::None {
                                return;
                            }
                            if !state.input.cursor_locked && state.stats.health > 0.0 {
                                // Re-enter input mode but let this click act too —
                                // eating the first click felt dead to players.
                                state.lock_cursor();
                            }
                            let pressed = button_state == ElementState::Pressed;
                            match button {
                                MouseButton::Left => state.input.break_pressed = pressed,
                                MouseButton::Right => state.input.place_pressed = pressed,
                                _ => {}
                            }
                        }
                        WindowEvent::MouseWheel { delta, .. } => {
                            let dy = match delta {
                                MouseScrollDelta::LineDelta(_, y) => y,
                                MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                            };
                            state.input.scroll += dy;
                        }
            _ => {}
        }
    }
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let event_loop = EventLoop::new().expect("EventLoop failed");
    let mut app = App { state: None };
    event_loop.run_app(&mut app);
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
    water_batches: HashMap<(i32, i32), MeshBatch>,
    cpu_meshes: HashMap<(i32, i32), (Vec<GpuVertex>, Vec<u32>)>,
    time: lf_game::TimeOfDay,
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
    pub inventory: Inventory,
    pub stats: PlayerStats,
    pub cursor_stack: Option<ItemStack>,
    pub ui_open: UiOpen,
    pub craft_grid: [Option<ItemStack>; 9],
    pub mining: Option<MiningState>,
    pub drops: Vec<ItemDrop>,
    drop_batch: Option<MeshBatch>,
    pub block_entities: HashMap<(i32, i32, i32), BlockEntity>,
    pub mobs: Vec<MobEntity>,
    pub villagers: Vec<Villager>,
    pub arrows: Vec<Arrow>,
    pub forge: lf_game::smithing::ForgeMinigame,
    pub research: ResearchState,
    pub xp_level: u32,
    pub xp_progress: u32,
    bow_charge: Option<f32>,
    mob_batch: Option<MeshBatch>,
    next_mob_id: u64,
    next_spawn_attempt: Instant,
    attack_cooldown: f32,
    pub kills: u32,
    pub quit_requested: bool,
    pub live_tracer: Option<lf_engine::pathtrace::Pathtracer>,
    pub live_rt_texture: Option<egui::TextureHandle>,
    pub title_orbit: f32,
    pub menu_reveal: f32,
    pub settings_tab: usize,
    pub hotbar_hover: Option<String>,
    /// F3 debug readout (input gates, position, seed) for diagnosing input.
    pub show_debug: bool,
    /// Developer console session (input, history, suggestions).
    pub console: console::ConsoleState,
    /// The active world's seed (persisted per slot with the world).
    pub world_seed: u64,
    /// The active save-slot directory (worlds/<slot-name>/).
    pub world_dir: std::path::PathBuf,
    /// The active slot's metadata (name/type/seed/updated).
    pub slot_meta: slots::SlotMeta,
    /// Slot picker state: new-world name input + chosen type.
    pub slot_name_input: String,
    pub slot_new_type: lf_worldgen::WorldType,
    /// Title screen: "New World" sub-menu expanded.
    pub title_show_new: bool,
    /// Real pixel-art item icons (one egui texture per item id).
    pub icons: icons::ItemIcons,
    /// Minimap + world-map state (tile cache, view, waypoints view state).
    pub map: map::MapState,
    /// Persisted player markers, shown on the map + minimap.
    pub waypoints: Vec<map::Waypoint>,
    /// HUD feedback timers, 1 -> 0.
    pub hud_flash: f32,
    pub hit_flash: f32,
    pub xp_flash: f32,
    pub hotbar_pick_time: f32,
    last_hotbar_index: usize,
    /// Recipe book panel state.
    pub recipe_book_open: bool,
    pub recipe_search: String,
    pub recipe_station: usize,
    pub recipe_craftable_only: bool,
    pub last_fps: f32,
    pub quest_log: QuestLog,
    pub chronicle: Vec<ChronicleEvent>,
    pub net: Option<net::NetClient>,
    pub chat_input: Option<String>,
    pub chat_log: Vec<String>,
    /// Clear -> rain/snow cycle with random transitions.
    pub weather_raining: bool,
    weather_next_change: Instant,
    cloud_batch: Option<MeshBatch>,
    sky_batch: Option<MeshBatch>,
    weather_batch: Option<MeshBatch>,
    last_cloud_rebuild: Instant,
    pub settings: Settings,
    pub world_type: lf_worldgen::WorldType,
    pub air: u8,
    spawn_point: Vec3,
    pub egui: EguiPlatform,
    last_instant: Instant,
    next_autosave: Instant,
    next_hunger_tick: Instant,
    next_regen_tick: Instant,
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

        // Load save (also tells us the world type) before generating.
        let (inventory, stats, time, block_entities, mobs, villagers, kills, quest_log, chronicle, research, settings, world_type, waypoints) =
            load_client_save(Path::new(WORLD_DIR));
        // Load mods into the live registries before touching the world.
        let mods = lf_modapi::load_mods_dir(Path::new("mods"));
        if !mods.is_empty() {
            tracing::info!("loaded {} mod(s): {:?}", mods.len(),
                mods.iter().map(|m| m.manifest.id.clone()).collect::<Vec<_>>());
        }

        // Persistence + world bootstrap: the slot owns the directory AND the
        // seed (fresh random seed for a brand-new world).
        let mut slot_meta = slots::boot_slot();
        let world_dir = slots::slot_dir(&slot_meta.name);
        slot_meta.world_type = world_type; // save (or default) wins over meta
        let storage = WorldStorage::open(&world_dir);
        let world_seed = storage.load_seed().unwrap_or_else(|| {
            let s = slot_meta.seed;
            let _ = storage.save_seed(s);
            s
        });
        slot_meta.seed = world_seed;
        let saved_set = storage.saved_chunks();
        let gen = WorldGen::with_type(Seed(world_seed), world_type);
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
        let mut water_batches = HashMap::new();
        let mut cpu_meshes = HashMap::new();
        let mut column_bounds = HashMap::new();
        for (cx, cz) in world.chunks.keys().copied().collect::<Vec<_>>() {
            let (v, i, wv, wi) = mesh_column_gpu(&world, cx, cz);
            let (min_y, max_y) = bounds_of(&v, &wv);
            batches.insert((cx, cz), MeshBatch::new(&device, &resources, &v, &i));
            if !wv.is_empty() {
                water_batches.insert((cx, cz), MeshBatch::new(&device, &resources, &wv, &wi));
            }
            cpu_meshes.insert((cx, cz), (v, i));
            column_bounds.insert((cx, cz), (min_y, max_y));
        }

        let outline = OutlineScene::new(&device, config.format);
        let (depth_texture, depth_view) = MeshBatch::create_depth_texture(&device, config.width, config.height);

        // Player: restore from save when present, else spawn on the surface.
        let spawn_point = Vec3::new(0.5, world.surface_height(0, 0) as f32 + 0.2, 0.5);
        let mut player = match storage.load_player() {
            Some(p) => Player::new(Vec3::from(p.position)).with_look(p.yaw, p.pitch),
            None => Player::new(spawn_point),
        };
        player.flying = false;

        // Inventory/stats/time from the extras file.


        let egui = EguiPlatform::new(&device, config.format, &window);
        let icons = icons::ItemIcons::new(&egui.ctx);

        // The worker skips chunks that come from the save.
        let mut worker_skip = saved_set.clone();
        worker_skip.extend(world.chunks.keys().copied());
        let streamer = Streamer::spawn(world_seed, worker_skip);

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
            water_batches,
            cpu_meshes,
            time,
            egui,
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
            inventory,
            stats,
            cursor_stack: None,
            ui_open: UiOpen::None,
            craft_grid: std::array::from_fn(|_| None),
            mining: None,
            drops: Vec::new(),
            drop_batch: None,
            block_entities: HashMap::new(),
            mobs: Vec::new(),
            villagers: Vec::new(),
            arrows: Vec::new(),
            forge: lf_game::smithing::ForgeMinigame::new(3),
            xp_level: 0,
            xp_progress: 0,
            bow_charge: None,
            mob_batch: None,
            next_mob_id: 1,
            next_spawn_attempt: Instant::now() + Duration::from_secs(2),
            attack_cooldown: 0.0,
            kills: 0,
            quit_requested: false,
            live_tracer: None,
            live_rt_texture: None,
            title_orbit: 0.0,
            menu_reveal: 0.0,
            settings_tab: 0,
            hotbar_hover: None,
            show_debug: std::env::var("LOREFORGE_DEBUG_INPUT").is_ok(),
            console: console::ConsoleState::default(),
            world_seed,
            world_dir,
            slot_meta,
            slot_name_input: String::new(),
            slot_new_type: lf_worldgen::WorldType::Normal,
            title_show_new: false,
            icons,
            map: map::MapState::new(world_type, world_seed),
            waypoints,
            hud_flash: 0.0,
            hit_flash: 0.0,
            xp_flash: 0.0,
            hotbar_pick_time: 0.0,
            last_hotbar_index: 0,
            recipe_book_open: true,
            recipe_search: String::new(),
            recipe_station: usize::MAX,
            recipe_craftable_only: false,
            last_fps: 0.0,
            quest_log,
            chronicle,
            research,
            settings,
            world_type,
            net: None,
            chat_input: None,
            chat_log: Vec::new(),
            weather_raining: false,
            weather_next_change: Instant::now() + Duration::from_secs(90),
            cloud_batch: None,
            sky_batch: None,
            weather_batch: None,
            last_cloud_rebuild: Instant::now() - Duration::from_secs(5),
            air: 10,
            spawn_point,
            last_instant: Instant::now(),
            next_autosave: Instant::now() + AUTOSAVE_INTERVAL,
            next_hunger_tick: Instant::now() + Duration::from_secs(45),
            next_regen_tick: Instant::now() + Duration::from_secs(3),
            frame: 0,
            screenshot_counter: 0,
            running: Arc::new(AtomicBool::new(true)),
        };
        state.ui_open = UiOpen::Title;
        state.update_title();
        state
    }

    /// Close any open screen, returning crafting-grid contents to the
    /// inventory (leftovers drop at the player).
    pub fn close_ui(&mut self) {
        // A stale chat input would force ui_open = Chat again next frame —
        // an invisible screen that eats every key and click.
        self.chat_input = None;
        let grid = std::mem::take(&mut self.craft_grid);
        for slot in grid.into_iter().flatten() {
            let leftover = self.inventory.add_item(&slot.item_id, slot.count);
            if leftover > 0 {
                self.drops.push(ItemDrop {
                    stack: ItemStack { count: leftover, ..slot },
                    position: self.player.eye_position() + self.player.look_dir(),
                    velocity: Vec3::new(0.0, 2.0, 0.0),
                    age: 0.0,
                });
            }
        }
        if let Some(cursor) = self.cursor_stack.take() {
            let leftover = self.inventory.add_item(&cursor.item_id, cursor.count);
            if leftover > 0 {
                self.drops.push(ItemDrop {
                    stack: ItemStack { count: leftover, ..cursor },
                    position: self.player.position + Vec3::new(0.0, 1.0, 0.0),
                    velocity: Vec3::ZERO,
                    age: 0.0,
                });
            }
        }
        self.ui_open = UiOpen::None;
        if self.stats.health > 0.0 {
            self.lock_cursor();
        }
    }

    /// Restart the background chunk generator with a new seed/ skip set
    /// (the worker captures its WorldGen at spawn, so seed changes need a
    /// fresh streamer).
    fn restart_streamer(&mut self, seed: u64, skip: HashSet<(i32, i32)>) {
        self.streamer.shutdown();
        self.streamer = Streamer::spawn(seed, skip);
    }

    /// Switch to a brand-new save slot with a fresh random seed.
    pub fn new_world(&mut self, world_type: lf_worldgen::WorldType) {
        self.new_world_named("World 1", world_type);
    }

    /// Create (or fully reset) the named slot with a fresh random seed.
    pub fn new_world_named(&mut self, name: &str, world_type: lf_worldgen::WorldType) {
        self.save_world();
        let name = slots::sanitize(name);
        let dir = slots::slot_dir(&name);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let seed = slots::random_seed();
        let meta = slots::SlotMeta {
            name: name.clone(),
            world_type,
            seed,
            updated_secs: slots::now_secs(),
        };
        let _ = std::fs::create_dir_all(&dir);
        let _ = WorldStorage::open(&dir).save_seed(seed);
        slots::write_meta(&dir, &meta);
        // point the client at the new slot
        self.storage = WorldStorage::open(&dir);
        self.world_dir = dir;
        self.slot_meta = meta;
        self.world_seed = seed;
        self.world_type = world_type;
        // fresh state
        self.inventory = Inventory::new();
        self.stats = PlayerStats::default();
        self.mobs.clear();
        self.drops.clear();
        self.arrows.clear();
        self.mining = None;
        self.cursor_stack = None;
        self.craft_grid = std::array::from_fn(|_| None);
        self.block_entities.clear();
        self.waypoints.clear();
        self.xp_level = 0;
        self.xp_progress = 0;
        self.kills = 0;
        self.map = map::MapState::new(world_type, seed);
        self.quest_log = {
            let mut log = QuestLog::new();
            for q in starter_quests() {
                log.add_quest(q);
            }
            log
        };
        self.chronicle.clear();
        self.time = lf_game::TimeOfDay::from_fraction(0.30);
        // regenerate the loaded chunks with the new seed
        let gen = WorldGen::with_type(Seed(seed), world_type);
        self.world = World::new();
        self.batches.clear();
        self.water_batches.clear();
        self.cpu_meshes.clear();
        self.column_bounds.clear();
        self.dirty.clear();
        self.saved_set.clear();
        let mut worker_skip = HashSet::new();
        for cx in -BOOT_RADIUS..=BOOT_RADIUS {
            for cz in -BOOT_RADIUS..=BOOT_RADIUS {
                let col = gen.generate_chunk(cx, cz);
                self.world.chunks.insert((cx, cz), col);
                self.add_column_batch(cx, cz);
            }
        }
        worker_skip.extend(self.world.chunks.keys().copied());
        self.restart_streamer(seed, worker_skip);
        self.spawn_point = Vec3::new(0.5, self.world.surface_height(0, 0) as f32 + 0.2, 0.5);
        self.player = Player::new(self.spawn_point);
        self.update_title();
        self.close_ui();
    }

    /// Load another save slot mid-session. Returns Err with a user-facing
    /// message when the slot doesn't exist.
    pub fn load_world(&mut self, name: &str) -> Result<(), String> {
        let name = slots::sanitize(name);
        let dir = slots::slot_dir(&name);
        let Some(meta) = slots::read_meta(&dir) else {
            return Err(format!("no save slot named '{}'", name));
        };
        self.save_world();
        // state from the slot's save files
        let (inventory, stats, time, block_entities, mobs, villagers, kills, quest_log,
             chronicle, research, settings, world_type, waypoints) = load_client_save(&dir);
        let storage = WorldStorage::open(&dir);
        let seed = storage.load_seed().unwrap_or(meta.seed);
        let saved_set = storage.saved_chunks();
        self.world_dir = dir;
        self.slot_meta = slots::SlotMeta { seed, updated_secs: meta.updated_secs, ..meta };
        self.world_seed = seed;
        self.storage = storage;
        self.world_type = world_type;
        self.saved_set = saved_set.clone();
        self.inventory = inventory;
        self.stats = stats;
        self.time = time;
        self.block_entities = block_entities;
        self.mobs = mobs;
        self.villagers = villagers;
        self.kills = kills;
        self.quest_log = quest_log;
        self.chronicle = chronicle;
        self.research = research;
        self.settings = settings;
        self.waypoints = waypoints;
        self.xp_level = 0;
        self.xp_progress = 0;
        self.drops.clear();
        self.arrows.clear();
        self.mining = None;
        self.cursor_stack = None;
        self.craft_grid = std::array::from_fn(|_| None);
        self.dirty.clear();
        self.map = map::MapState::new(world_type, seed);
        // rebuild the world from the slot: saved chunks first, gen fallback
        let gen = WorldGen::with_type(Seed(seed), world_type);
        self.world = World::new();
        self.batches.clear();
        self.water_batches.clear();
        self.cpu_meshes.clear();
        self.column_bounds.clear();
        let mut worker_skip = HashSet::new();
        for cx in -BOOT_RADIUS..=BOOT_RADIUS {
            for cz in -BOOT_RADIUS..=BOOT_RADIUS {
                let col = if saved_set.contains(&(cx, cz)) {
                    self.storage.load_chunk(cx, cz).unwrap_or_else(|| gen.generate_chunk(cx, cz))
                } else {
                    gen.generate_chunk(cx, cz)
                };
                self.world.chunks.insert((cx, cz), col);
                self.add_column_batch(cx, cz);
            }
        }
        worker_skip.extend(saved_set);
        worker_skip.extend(self.world.chunks.keys().copied());
        self.restart_streamer(seed, worker_skip);
        self.spawn_point = Vec3::new(0.5, self.world.surface_height(0, 0) as f32 + 0.2, 0.5);
        self.player = match self.storage.load_player() {
            Some(p) => Player::new(Vec3::from(p.position)).with_look(p.yaw, p.pitch),
            None => Player::new(self.spawn_point),
        };
        self.player.flying = false;
        self.update_title();
        self.close_ui();
        Ok(())
    }

    pub fn respawn(&mut self) {
        self.stats.health = self.stats.max_health;
        self.stats.hunger = self.stats.max_hunger;
        self.stats.saturation = 5.0;
        self.air = 10;
        self.player = Player::new(self.spawn_point);
        self.mining = None;
        self.ui_open = UiOpen::None;
        self.lock_cursor();
    }

    fn shutdown(&mut self, event_loop: &ActiveEventLoop) {
        self.save_world();
        self.streamer.shutdown();
        self.running.store(false, Ordering::Relaxed);
        event_loop.exit();
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
        }
        let extras = ClientSave {
            slots: self.inventory.slots.clone(),
            health: self.stats.health,
            hunger: self.stats.hunger,
            time_ticks: self.time.ticks,
            block_entities: self.block_entities.iter().map(|(k, v)| (*k, v.clone())).collect(),
            mobs: self.mobs.clone(),
            villagers: self.villagers.clone(),
            kills: self.kills,
            quest_log: Some(self.quest_log.clone()),
            chronicle: self.chronicle.clone(),
            world_type: Some(self.world_type),
            research: Some(self.research.clone()),
            settings: Some(self.settings),
            waypoints: self.waypoints.clone(),
        };
        if let Ok(bytes) = bincode::serialize(&extras) {
            let _ = std::fs::write(self.world_dir.join("player_extras.dat"), bytes);
        }
        if !self.chronicle.is_empty() {
            let md = SagaGenerator::export_markdown(&self.chronicle);
            let _ = std::fs::write(self.world_dir.join("chronicle.md"), md);
        }
        // keep the slot metadata current (ordering + seed + type)
        let meta = slots::SlotMeta {
            name: self.slot_meta.name.clone(),
            world_type: self.world_type,
            seed: self.world_seed,
            updated_secs: slots::now_secs(),
        };
        slots::write_meta(&self.world_dir, &meta);
        self.slot_meta = meta;
        let _ = self.storage.save_seed(self.world_seed);
        tracing::info!("world '{}' saved to {}", self.slot_meta.name, self.world_dir.display());
    }

    fn lock_cursor(&mut self) {
        // Enter input mode even if the OS grab fails (some window managers
        // refuse grabs): mouse motion still arrives via DeviceEvent, and
        // clicks must keep working instead of being swallowed forever.
        if self.window.set_cursor_grab(CursorGrabMode::Locked).is_ok() {
            self.window.set_cursor_visible(false);
        }
        self.input.cursor_locked = true;
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
            "LOREFORGE — {} [{}/9] — pos ({:.1}, {:.1}, {:.1}) — chunks {} — F fly · F2 shot · Esc release",
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
        let fps = 1.0 / dt.max(0.0001);
        self.last_fps = if self.last_fps == 0.0 { fps } else { self.last_fps * 0.9 + fps * 0.1 };
        self.last_instant = now;
        self.frame += 1;
        if self.quit_requested {
            self.save_world();
            self.streamer.shutdown();
            std::process::exit(0);
        }

        // Live ray tracing: trace at the internal scale each frame.
        if self.settings.rt_mode == RtMode::Live && self.ui_open == UiOpen::None {
            let scale = self.settings.rt_scale.clamp(0.1, 0.5);
            let w = ((self.config.width as f32 * scale) as u32).max(64);
            let h = ((self.config.height as f32 * scale) as u32).max(48);
            if self.live_tracer.as_ref().map(|t| t.width != w || t.height != h).unwrap_or(true) {
                self.live_tracer = lf_engine::pathtrace::Pathtracer::new(w, h).ok();
            }
            let eye = self.player.eye_position();
            let center = (eye.x as i32, (eye.y as i32 + 20).min(220), eye.z as i32);
            let data = lf_engine::pathtrace::build_voxel_texture_data(center, &|x, y, z| {
                self.world.get_block(x, y, z).id()
            });
            let camera = self.camera();
            let angle = self.time.fraction() * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            let sun = [angle.cos(), angle.sin().abs(), 0.25];
            let day = self.time.sky_light_level();
            if let Some(tracer) = &mut self.live_tracer {
                tracer.upload_voxels(center, &data);
                let camera = &camera;
                let sun = sun;
                if let Ok(img) = tracer.render_frame(camera, center, sun, day) {
                    let size = [img.width() as usize, img.height() as usize];
                    let ctx = self.egui.ctx.clone();
                    let pixels: Vec<egui::Color32> = img.pixels().map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3])).collect();
                    let image = egui::ColorImage { size, pixels };
                    let tex = match self.live_rt_texture.take() {
                        Some(mut t) => {
                            t.set(image, egui::TextureOptions::LINEAR);
                            t
                        }
                        None => ctx.load_texture("live_rt", image, egui::TextureOptions::LINEAR),
                    };
                    self.live_rt_texture = Some(tex);
                }
            }
        }

        // UI frame.
        self.menu_reveal = (self.menu_reveal + dt).min(3.0);
        if self.ui_open == UiOpen::Title {
            self.title_orbit += dt * 0.05; // slow menu camera orbit
        }
        // HUD feedback timers + hotbar switch detection.
        self.hud_flash = (self.hud_flash - dt * 1.6).max(0.0);
        self.hit_flash = (self.hit_flash - dt * 3.0).max(0.0);
        self.xp_flash = (self.xp_flash - dt * 1.2).max(0.0);
        self.hotbar_pick_time = (self.hotbar_pick_time - dt * 0.55).max(0.0);
        if self.hotbar_index != self.last_hotbar_index {
            self.last_hotbar_index = self.hotbar_index;
            self.hotbar_pick_time = 1.0;
        }
        let window = self.window.clone();
        self.egui.begin_frame(&window);
        let ctx = self.egui.ctx.clone();
        self.draw_ui(&ctx);

        let playing = matches!(self.ui_open, UiOpen::None) && self.stats.health > 0.0;
        let input = if playing {
            PlayerInput {
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
                yaw_delta: self.input.mouse_dx * self.settings.sensitivity,
                pitch_delta: self.input.mouse_dy * self.settings.sensitivity
                    * if self.settings.invert_y { 1.0 } else { -1.0 },
            }
        } else {
            PlayerInput::default()
        };
        self.input.mouse_dx = 0.0;
        self.input.mouse_dy = 0.0;

        if self.input.scroll.abs() >= 1.0 {
            let steps = self.input.scroll.signum() as i32;
            self.input.scroll = 0.0;
            self.hotbar_index = ((self.hotbar_index as i32 + steps).rem_euclid(9)) as usize;
            self.update_title();
        }

        self.player.update(dt, &input, &self.world);
        self.survival_tick(dt);
        self.update_drops(dt);
        // Furnaces and machines tick whether or not their UI is open.
        // Generators are taken out while machines draw from them (single
        // mutable owner at a time), then reinserted.
        let mut generators: Vec<((i32, i32, i32), lf_game::machines::Generator)> = Vec::new();
        let mut machine_positions: Vec<((i32, i32, i32), u32)> = Vec::new();
        for (pos, entity) in self.block_entities.iter_mut() {
            match entity {
                BlockEntity::Generator(g) => {
                    g.tick(dt);
                    generators.push((*pos, g.clone()));
                }
                BlockEntity::ElectricFurnace(_) | BlockEntity::Crusher(_) | BlockEntity::Assembler(_) => {
                    machine_positions.push((*pos, 1));
                }
                BlockEntity::Furnace(f) => {
                    f.tick(dt);
                }
                _ => {}
            }
        }
        let need = lf_game::machines::DRAW_RATE * dt;
        for (mpos, _) in &machine_positions {
            let mut powered = 0.0;
            for (gpos, gen) in generators.iter_mut() {
                let d = ((gpos.0 - mpos.0).pow(2) + (gpos.1 - mpos.1).pow(2) + (gpos.2 - mpos.2).pow(2)) as f32;
                if d.sqrt() <= lf_game::machines::POWER_RANGE && powered < need {
                    powered += gen.draw(need - powered);
                }
            }
            if let Some(entity) = self.block_entities.get_mut(mpos) {
                match entity {
                    BlockEntity::ElectricFurnace(f) => { f.tick(dt, powered); }
                    BlockEntity::Crusher(c) => { c.tick(dt, powered); }
                    BlockEntity::Assembler(a) => { a.tick(dt, powered); }
                    _ => {}
                }
            }
        }
        for (gpos, gen) in generators {
            self.block_entities.insert(gpos, BlockEntity::Generator(gen));
        }

        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
        self.update_mobs(dt);
        self.update_villagers(dt);
        self.update_arrows(dt);
        // bow: hold RMB to charge
        if playing && self.input.place_pressed {
            let held_id = self.inventory.slots[self.hotbar_index].as_ref().map(|s| s.item_id.clone());
            if held_id.as_deref() == Some("bow") {
                self.bow_charge = Some(self.bow_charge.unwrap_or(0.0) + dt);
            }
        } else if let Some(charge) = self.bow_charge.take() {
            // released: fire if we have an arrow
            let has_arrow = self.inventory.slots.iter()
                .filter_map(|s| s.as_ref())
                .any(|s| s.item_id == "arrow");
            if charge > 0.25 && has_arrow {
                // consume one arrow
                'find: for slot in self.inventory.slots.iter_mut() {
                    if let Some(stack) = slot {
                        if stack.item_id == "arrow" {
                            stack.count -= 1;
                            if stack.count == 0 { *slot = None; }
                            break 'find;
                        }
                    }
                }
                let eye = self.player.eye_position();
                let look = self.player.look_dir();
                let speed = 12.0 + charge.min(1.2) * 18.0;
                self.arrows.push(Arrow { position: eye + look * 0.5, velocity: look * speed, age: 0.0 });
            }
        }
        if let Some(n) = &mut self.net {
            n.send_state(self.player.position.to_array(), self.player.yaw, self.player.pitch);
            for msg in n.poll() {
                if let lf_protocol::ServerMessage::BlockUpdate { x, y, z, block } = msg {
                    if self.world.set_block(x, y, z, BlockState(block)).is_some() {
                        self.remesh_around(x, z);
                    }
                }
            }
        }

        // 20-minute day/night cycle.
        let ticks = (dt * lf_game::TimeOfDay::TICKS_PER_SECOND as f32) as u64;
        self.time = lf_game::TimeOfDay::new(self.time.ticks + ticks);

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

        // Attacking: LMB on a mob (sphere test along the look ray).
        if playing && self.input.break_pressed {
            if let Some(mob_hit) = self.mob_in_crosshair() {
                if self.attack_cooldown <= 0.0 {
                    self.attack_cooldown = 0.5;
                    let held = self.inventory.slots[self.hotbar_index].clone();
                    let damage = held
                        .as_ref()
                        .and_then(|s| item_def(&s.item_id))
                        .map(|d| match d.kind {
                            ItemKind::Tool(kind, tier) => tool_damage(kind, tier),
                            _ => 1.0,
                        })
                        .unwrap_or(1.0);
                    let from = self.player.eye_position();
                    let killed = {
                        let mob = &mut self.mobs[mob_hit];
                        let dead = mob.take_hit(damage, from);
                        dead
                    };
                    self.hit_flash = 1.0;
                    if killed {
                        let (kind, pos) = {
                            let mob = &self.mobs[mob_hit];
                            (mob.mob_type, mob.position)
                        };
                        for (item, n) in kind.drops() {
                            self.spawn_drop(item, *n, pos + Vec3::new(0.0, 0.5, 0.0));
                        }
                        self.kills += 1;
                        if self.kills == 1 {
                            self.chronicle_event(EventType::FirstBlood, "struck down the first creature".into());
                        }
                        if kind == MobType::NullKnight {
                            self.chronicle_event(EventType::BossSlain, "the Null Knight falls".into());
                        }
                        let kind_name = format!("{:?}", kind);
                        self.quest_event(QuestEvent::Killed(kind_name.clone()));
                        tracing::info!("killed a {:?}", kind);
                        self.mobs.remove(mob_hit);
                    }
                }
                self.mining = None;
                return; // don't also mine this frame
            }
        }

        // Mining: hold LMB on the same block to progress.
        if playing && self.input.break_pressed {
            if let Some((pos, _)) = target {
                let key = (pos.x, pos.y, pos.z);
                let block_id = self.world.get_block(pos.x, pos.y, pos.z).id();
                let held = self.inventory.slots[self.hotbar_index].clone();
                let total = break_time(block_id, held.as_ref()).unwrap_or(f32::INFINITY);
                match &mut self.mining {
                    Some(m) if m.pos == key => m.progress += dt,
                    Some(m) => {
                        m.pos = key;
                        m.progress = dt;
                        m.total = total;
                    }
                    None => self.mining = Some(MiningState { pos: key, progress: dt, total }),
                }
                if let Some(m) = &mut self.mining {
                    m.total = total;
                    if m.progress >= m.total {
                        self.mining = None;
                        if self.world.set_block(pos.x, pos.y, pos.z, BlockState::AIR).is_some() {
                            self.break_block_drops(block_id, pos);
                            self.use_durability();
                            self.remesh_around(pos.x, pos.z);
                            let (l, p) = grant_xp(self.xp_level, self.xp_progress, 1);
                            self.xp_level = l;
                            self.xp_progress = p;
                            self.xp_flash = 1.0;
                            if let Some(n) = &self.net {
                                n.send_block(pos.x, pos.y, pos.z, registry::block::AIR);
                            }
                            // container contents spill out
                            let key = (pos.x, pos.y, pos.z);
                            if let Some(entity) = self.block_entities.remove(&key) {
                                let stacks: Vec<Option<ItemStack>> = match entity {
                                    BlockEntity::Furnace(f) => vec![f.input, f.fuel, f.output],
                                    BlockEntity::Chest { slots } => slots,
                                    BlockEntity::Generator(g) => vec![g.fuel],
                                    BlockEntity::ElectricFurnace(f) => vec![f.input, f.output],
                                    BlockEntity::Crusher(c) => vec![c.input, c.output],
                                    BlockEntity::Assembler(a) => vec![a.input_a, a.input_b, a.output],
                                };
                                for s in stacks.into_iter().flatten() {
                                    self.spawn_drop(&s.item_id, s.count,
                                        Vec3::new(pos.x as f32 + 0.5, pos.y as f32 + 0.5, pos.z as f32 + 0.5));
                                }
                            }
                        }
                    }
                }
            } else {
                self.mining = None;
            }
        } else {
            self.mining = None;
        }

        // Right click: open crafting table, eat, or place the held block.
        if playing && self.input.place_pressed {
            self.input.place_pressed = false; // one action per click
            if let Some((pos, _)) = target {
                match self.world.get_block(pos.x, pos.y, pos.z).id() {
                    registry::block::CRAFTING_TABLE => {
                        self.ui_open = UiOpen::CraftingTable;
                        self.unlock_cursor();
                    }
                    registry::block::SMITHING_TABLE => {
                        self.ui_open = UiOpen::Smithing;
                        self.unlock_cursor();
                    }
                    registry::block::RESEARCH_BENCH => {
                        let era = self.research.era;
                        if let Some(next) = self.research.advance(&mut self.inventory.slots) {
                            tracing::info!("researched the {}", next.name());
                            self.chronicle_event(lf_chronicle::EventType::ActCompleted, format!("entered the {}", next.name()));
                        } else if era.next().is_none() {
                            self.chat_log.push("final era reached".into());
                        } else {
                            self.chat_log.push(format!("not enough materials for the {} — press K for the tree", era.next().unwrap().name()));
                        }
                    }
                    registry::block::COAL_GENERATOR
                    | registry::block::ELECTRIC_FURNACE
                    | registry::block::CRUSHER
                    | registry::block::ASSEMBLER => {
                        let key = (pos.x, pos.y, pos.z);
                        let block_id_here = self.world.get_block(pos.x, pos.y, pos.z).id();
                        self.block_entities.entry(key).or_insert_with(|| match block_id_here {
                            registry::block::COAL_GENERATOR => BlockEntity::Generator(Default::default()),
                            registry::block::ELECTRIC_FURNACE => BlockEntity::ElectricFurnace(Default::default()),
                            registry::block::CRUSHER => BlockEntity::Crusher(Default::default()),
                            _ => BlockEntity::Assembler(Default::default()),
                        });
                        self.ui_open = UiOpen::Machine(key);
                        self.unlock_cursor();
                    }
                    registry::block::FURNACE => {
                        let key = (pos.x, pos.y, pos.z);
                        self.block_entities.entry(key)
                            .or_insert_with(|| BlockEntity::Furnace(Default::default()));
                        self.ui_open = UiOpen::Furnace(key);
                        self.unlock_cursor();
                    }
                    registry::block::CHEST => {
                        let key = (pos.x, pos.y, pos.z);
                        self.block_entities.entry(key)
                            .or_insert_with(|| BlockEntity::Chest { slots: vec![None; 27] });
                        self.ui_open = UiOpen::Chest(key);
                        self.unlock_cursor();
                    }
                    _ => {}
                }
            }
            if self.ui_open == UiOpen::None {
                // villager in the crosshair? trade instead
                if let Some(vi) = self.villager_in_crosshair() {
                    self.ui_open = UiOpen::Trade(vi);
                    self.unlock_cursor();
                    return;
                }
                let held = self.inventory.slots[self.hotbar_index].clone();
                if held.as_ref().map(|s| s.item_id.as_str()) == Some("book") {
                    self.ui_open = UiOpen::Book;
                    self.unlock_cursor();
                    return;
                }
                if let Some(stack) = held {
                    let def = item_def(&stack.item_id);
                    match def.map(|d| d.kind) {
                        Some(ItemKind::Food(heal)) if self.stats.hunger < self.stats.max_hunger => {
                            self.stats.hunger = (self.stats.hunger + heal as f32).min(self.stats.max_hunger);
                            self.consume_selected(1);
                        }
                        Some(ItemKind::Block(b)) => {
                            if let Some((pos, normal)) = target {
                                let place = pos + normal;
                                if !self.block_intersects_player(place) {
                                    if self.world.set_block(place.x, place.y, place.z, BlockState(b)).is_some() {
                                        if let Some(n) = &self.net {
                                            n.send_block(place.x, place.y, place.z, b);
                                        }
                                        let key = (place.x, place.y, place.z);
                                        if b == registry::block::FURNACE {
                                            self.block_entities.insert(key, BlockEntity::Furnace(Default::default()));
                                        } else if b == registry::block::CHEST {
                                            self.block_entities.insert(key, BlockEntity::Chest { slots: vec![None; 27] });
                                        } else if b == registry::block::COAL_GENERATOR {
                                            self.block_entities.insert(key, BlockEntity::Generator(Default::default()));
                                        } else if b == registry::block::ELECTRIC_FURNACE {
                                            self.block_entities.insert(key, BlockEntity::ElectricFurnace(Default::default()));
                                        } else if b == registry::block::CRUSHER {
                                            self.block_entities.insert(key, BlockEntity::Crusher(Default::default()));
                                        } else if b == registry::block::ASSEMBLER {
                                            self.block_entities.insert(key, BlockEntity::Assembler(Default::default()));
                                        }
                                        self.remesh_around(place.x, place.z);
                                        self.consume_selected(1);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if now >= self.next_autosave {
            self.next_autosave = now + AUTOSAVE_INTERVAL;
            self.save_world();
        }

        // Weather cycle: toggles every few minutes.
        if now >= self.weather_next_change {
            self.weather_raining = !self.weather_raining;
            let span = if self.weather_raining { 120 } else { 240 };
            self.weather_next_change = now + Duration::from_secs(span + (pseudo_random(self.frame) % 120));
        }
        // Sky bodies every frame (they rotate); clouds drift (rebuild 2/s).
        let eye = self.player.eye_position();
        let (sv, si) = lf_engine::atmosphere::sky_bodies(eye, self.time.fraction());
        self.sky_batch = Some(MeshBatch::new(&self.device, &self.resources, &sv, &si));
        if self.last_cloud_rebuild.elapsed() >= Duration::from_millis(500) {
            self.last_cloud_rebuild = now;
            let (cv, ci) = lf_engine::atmosphere::cloud_mesh(eye, self.frame as f32 / 60.0);
            self.cloud_batch = Some(MeshBatch::new(&self.device, &self.resources, &cv, &ci));
        }
        if self.weather_raining && self.settings.particles {
            let cold = self.gen_biome_temp_at_player();
            let (wv, wi) = lf_engine::atmosphere::weather_particles(eye, self.frame as f32 / 60.0, cold);
            self.weather_batch = Some(MeshBatch::new(&self.device, &self.resources, &wv, &wi));
        } else {
            self.weather_batch = None;
        }
    }

    /// Is the player's biome cold enough for snow?
    fn gen_biome_temp_at_player(&self) -> bool {
        // The client doesn't own a WorldGen; approximate coldness from the
        // surface block under the player.
        let under = self.world.get_block(
            self.player.position.x as i32,
            (self.player.position.y as i32 - 1).max(0),
            self.player.position.z as i32,
        ).id();
        matches!(under, registry::block::SNOW | registry::block::ICE)
    }

    fn consume_selected(&mut self, n: u8) {
        if let Some(stack) = &mut self.inventory.slots[self.hotbar_index] {
            stack.count = stack.count.saturating_sub(n);
            if stack.count == 0 {
                self.inventory.slots[self.hotbar_index] = None;
            }
        }
    }

    fn use_durability(&mut self) {
        let slot = &mut self.inventory.slots[self.hotbar_index];
        if let Some(stack) = slot {
            if let Some(def) = item_def(&stack.item_id) {
                if let ItemKind::Tool(_, tier) = def.kind {
                    // durability stored as count on a 1-stack (abused as hits left)
                    let max = tier_durability(tier) as u8;
                    if stack.count >= max {
                        *slot = None; // tool broke
                    } else {
                        stack.count += 1;
                    }
                }
            }
        }
    }

    fn break_block_drops(&mut self, block_id: u32, pos: glam::IVec3) {
        let held = self.inventory.slots[self.hotbar_index].clone();
        let harvestable = tool_satisfies(block_id, held.as_ref());
        if !harvestable {
            return; // wrong tool: block breaks but yields nothing
        }
        if let Some(item) = lf_game::items::block_drop(block_id) {
            self.spawn_drop(&item, 1, Vec3::new(pos.x as f32 + 0.5, pos.y as f32 + 0.3, pos.z as f32 + 0.5));
        }
        // rare apple bonus from leaves
        if block_id == registry::block::LEAVES && pseudo_random(self.frame) % 20 == 0 {
            self.spawn_drop("apple", 1, Vec3::new(pos.x as f32 + 0.5, pos.y as f32 + 0.3, pos.z as f32 + 0.5));
        }
    }

    fn spawn_drop(&mut self, item: &str, count: u8, pos: Vec3) {
        self.drops.push(ItemDrop {
            stack: ItemStack { item_id: item.to_string(), count },
            position: pos,
            velocity: Vec3::new(0.0, 1.5, 0.0),
            age: 0.0,
        });
    }

    /// Hunger drain, regen, fall damage, drowning, death.
    fn survival_tick(&mut self, dt: f32) {
        if self.stats.health <= 0.0 {
            return; // dead: nothing ticks
        }
        let now = Instant::now();
        // Fall damage on landing.
        if self.player.just_landed && !self.player.flying {
            let impact = -self.player.last_impact;
            let damage = (impact * impact / 64.0 - 3.0).max(0.0);
            if damage > 0.5 {
                self.damage(damage);
            }
        }
        // Drowning.
        let eye_block = self.world.get_block(
            self.player.eye_position().x as i32,
            self.player.eye_position().y as i32,
            self.player.eye_position().z as i32,
        );
        if eye_block.id() == registry::block::WATER {
            if self.air == 0 {
                if now >= self.next_regen_tick {
                    self.next_regen_tick = now + Duration::from_secs(1);
                    self.damage(2.0);
                }
            } else if self.frame % 20 == 0 {
                self.air -= 1;
            }
        } else if self.air < 10 && self.frame % 4 == 0 {
            self.air = (self.air + 1).min(10);
        }
        // Hunger drains slowly.
        if now >= self.next_hunger_tick {
            self.next_hunger_tick = now + Duration::from_secs(45);
            self.stats.hunger = (self.stats.hunger - 1.0).max(0.0);
        }
        // Regen when well fed; starve at zero.
        if now >= self.next_regen_tick {
            self.next_regen_tick = now + Duration::from_secs(3);
            if self.stats.hunger >= 15.0 && self.stats.health < self.stats.max_health {
                self.stats.health = (self.stats.health + 1.0).min(self.stats.max_health);
            } else if self.stats.hunger <= 0.0 && self.stats.health > 1.0 {
                self.stats.health -= 1.0;
            }
        }
        let _ = dt;
    }

    pub fn damage(&mut self, amount: f32) {
        let armor = worn_armor_points(&self.inventory.slots);
        let amount = mitigate(amount, armor);
        self.stats.health = (self.stats.health - amount).max(0.0);
        self.hud_flash = 1.0;
        if self.stats.health <= 0.0 {
            self.chronicle_event(EventType::Death, "the Smith fell".into());
            // death
            self.ui_open = UiOpen::Death;
            self.unlock_cursor();
            tracing::info!("player died");
        }
    }

    /// Feed a gameplay event into quests (and log chronicle milestones).
    pub fn quest_event(&mut self, event: QuestEvent) {
        let finished = self.quest_log.record_event(&event);
        for id in finished {
            let title = self.quest_log.quests.iter().find(|q| q.id == id)
                .map(|q| q.title.clone()).unwrap_or_default();
            tracing::info!("quest complete: {}", title);
            self.chronicle_event(EventType::ActCompleted, format!("completed quest '{}'", title));
        }
    }

    pub fn chronicle_event(&mut self, event_type: EventType, payload: String) {
        self.chronicle.push(ChronicleEvent {
            id: format!("e{}", self.chronicle.len() + 1),
            event_type,
            in_game_date: self.time.ticks / lf_game::TimeOfDay::TICKS_PER_DAY,
            location: self.player.position.to_array(),
            actors: vec!["The Smith".into()],
            payload,
        });
    }

    fn update_arrows(&mut self, dt: f32) {
        let mut remove: Vec<usize> = Vec::new();
        let mut events: Vec<(usize, f32)> = Vec::new(); // (mob idx, damage)
        for (i, arrow) in self.arrows.iter_mut().enumerate() {
            let before = arrow.position;
            let done = arrow.update(dt, |x, y, z| self.world.is_solid(x, y, z));
            if done {
                remove.push(i);
                continue;
            }
            // mob hit test along the step
            let dir = arrow.position - before;
            let step = dir.length().max(0.001);
            let dir = dir / step;
            for (mi, mob) in self.mobs.iter().enumerate() {
                let size = mob.mob_type.stats().size;
                let center = mob.position + Vec3::new(0.0, size, 0.0);
                let to = center - before;
                let t = to.dot(dir);
                if t < 0.0 || t > step + size {
                    continue;
                }
                if (before + dir * t - center).length() < size + 0.3 {
                    events.push((mi, 6.0));
                    remove.push(i);
                    break;
                }
            }
        }
        for i in remove.into_iter().rev() {
            self.arrows.remove(i);
        }
        for (mi, damage) in events {
            if mi < self.mobs.len() {
                let killed = self.mobs[mi].take_hit(damage, self.player.position);
                if killed {
                    let (kind, pos) = (self.mobs[mi].mob_type, self.mobs[mi].position);
                    for (item, n) in kind.drops() {
                        self.spawn_drop(item, *n, pos + Vec3::new(0.0, 0.5, 0.0));
                    }
                    self.kills += 1;
                    let (l, p) = grant_xp(self.xp_level, self.xp_progress, 5);
                    self.xp_level = l;
                    self.xp_progress = p;
                    self.xp_flash = 1.0;
                    self.mobs.remove(mi);
                }
            }
        }
    }

    /// Villagers wander by day and rest at night (schedule data).
    fn update_villagers(&mut self, dt: f32) {
        if self.stats.health <= 0.0 {
            return;
        }
        let hour = (self.time.fraction() * 24.0) as i32;
        for (i, villager) in self.villagers.iter_mut().enumerate() {
            // deterministic per-villager wander seed
            let t = self.frame as u64 / 30; // change direction ~every half second
            let seed = (villager.id).wrapping_mul(2654435761).wrapping_add(t).wrapping_add(i as u64);
            let resting = villager.should_rest(hour);
            if !resting && seed % 3 == 0 {
                let a = (seed % 360) as f32 / 57.3;
                let speed = 1.2;
                let next = glam::Vec3::from(villager.position) + glam::Vec3::new(a.cos() * speed * dt, 0.0, a.sin() * speed * dt);
                // stay on ground
                if !self.world.is_solid(next.x as i32, next.y as i32, next.z as i32) {
                    if self.world.is_solid(next.x as i32, (next.y - 1.0) as i32, next.z as i32) {
                        villager.position = [next.x, next.y, next.z];
                    }
                }
            }
            // despawn logic none: villagers persist
        }
    }

    /// Spawn villagers when chunks containing hamlets load (throttled).
    fn try_spawn_villagers(&mut self) {
        if self.villagers.len() >= 12 || self.frame % 60 != 0 {
            return;
        }
        let player = self.player.position;
        for ((cx, cz), col) in self.world.chunks.iter() {
            // only near the player
            let center = (*cx as f32 * 16.0 + 8.0, *cz as f32 * 16.0 + 8.0);
            let dist = ((center.0 - player.x).powi(2) + (center.1 - player.z).powi(2)).sqrt();
            if dist > 60.0 {
                continue;
            }
            // hamlet marker: crafting table on the surface band
            let mut has_hut = false;
            let mut hut_spot = None;
            for lx in 4..12usize {
                for lz in 4..12usize {
                    for y in 60..200usize {
                        if col.get(lx, y, lz).id() == registry::block::CRAFTING_TABLE {
                            has_hut = true;
                            hut_spot = Some((cx * 16 + lx as i32, y as i32 + 1, cz * 16 + lz as i32));
                            break;
                        }
                    }
                }
            }
            if !has_hut {
                continue;
            }
            let (hx, hy, hz) = hut_spot.unwrap();
            // already staffed? (one villager per ~6 blocks of hut)
            let staffed = self.villagers.iter().any(|v| {
                (v.position[0] - hx as f32).abs() < 8.0 && (v.position[2] - hz as f32).abs() < 8.0
            });
            if staffed {
                continue;
            }
            let id = 1000 + self.villagers.len() as u64;
            let jobs = [VillagerJob::Farmer, VillagerJob::Smith, VillagerJob::Trader,
                        VillagerJob::Guard, VillagerJob::Bard, VillagerJob::Lorekeeper];
            let job = jobs[(id as usize) % jobs.len()];
            let name = match job {
                VillagerJob::Farmer => "Old Maisie",
                VillagerJob::Smith => "Brann",
                VillagerJob::Trader => "Sila",
                VillagerJob::Guard => "Dora",
                VillagerJob::Bard => "Pip",
                VillagerJob::Lorekeeper => "Wex",
            };
            let spawn = if self.world.is_solid(hx, hy - 1, hz + 2) { (hx, hy, hz + 2) } else { (hx, hy, hz) };
            self.villagers.push(Villager::new(id, job, name.to_string(),
                [spawn.0 as f32 + 0.5, spawn.1 as f32 + 0.2, spawn.2 as f32 + 0.5]));
            tracing::info!("villager {} the {:?} settled a hamlet", name, job);
            return; // one per check
        }
    }

    fn villager_in_crosshair(&self) -> Option<usize> {
        let eye = self.player.eye_position();
        let look = self.player.look_dir();
        let mut best: Option<(f32, usize)> = None;
        for (i, v) in self.villagers.iter().enumerate() {
            let center = glam::Vec3::from(v.position) + glam::Vec3::new(0.0, 0.9, 0.0);
            let to = center - eye;
            let t = to.dot(look);
            if t < 0.0 || t > REACH + 1.0 {
                continue;
            }
            let closest = eye + look * t;
            if (closest - center).length() < 1.0 {
                if best.map(|(d, _)| t < d).unwrap_or(true) {
                    best = Some((t, i));
                }
            }
        }
        best.map(|(_, i)| i)
    }

    /// Index of the nearest mob roughly under the crosshair, within reach.
    fn mob_in_crosshair(&self) -> Option<usize> {
        let eye = self.player.eye_position();
        let look = self.player.look_dir();
        let mut best: Option<(f32, usize)> = None;
        for (i, mob) in self.mobs.iter().enumerate() {
            let size = mob.mob_type.stats().size;
            let to = mob.position + Vec3::new(0.0, size, 0.0) - eye;
            let t = to.dot(look);
            if t < 0.0 || t > REACH + 1.0 {
                continue;
            }
            let closest = eye + look * t;
            let center = mob.position + Vec3::new(0.0, size, 0.0);
            if (closest - center).length() < size + 0.45 {
                if best.map(|(d, _)| t < d).unwrap_or(true) {
                    best = Some((t, i));
                }
            }
        }
        best.map(|(_, i)| i)
    }

    /// Advance mob AI/physics and run the spawn/despawn cycle.
    fn update_mobs(&mut self, dt: f32) {
        if self.stats.health <= 0.0 {
            return;
        }
        let player = self.player.position;
        let world = &self.world;
        let mut damage_to_player = 0.0;
        for mob in self.mobs.iter_mut() {
            if let Some(dmg) = mob.update(dt, world, player) {
                damage_to_player += dmg;
            }
        }
        if damage_to_player > 0.0 {
            self.damage(damage_to_player);
        }
        // despawn far mobs
        self.mobs.retain(|m| (m.position - player).length() < 80.0);

        // spawn cycle
        if Instant::now() >= self.next_spawn_attempt && self.mobs.len() < 12 {
            self.next_spawn_attempt = Instant::now() + Duration::from_secs(2);
            let seed = self.frame.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(0x51ed270b);
            let is_day = self.time.is_day();
            if let Some(kind) = roll_spawn(seed, is_day) {
                // random point 20-40 blocks out
                let ang = (seed % 360) as f32 / 57.3;
                let dist = 20.0 + ((seed >> 9) % 20) as f32;
                let sx = (player.x + ang.cos() * dist) as i32;
                let sz = (player.z + ang.sin() * dist) as i32;
                // only spawn on loaded ground
                if let Some((cx, lx)) = Some((sx.div_euclid(16), sx.rem_euclid(16))) {
                    let (cz, lz) = (sz.div_euclid(16), sz.rem_euclid(16));
                    if self.world.chunk(cx, cz).is_some() {
                        let top = self.world.surface_height(sx, sz);
                        if top > lf_worldgen::SEA_LEVEL || !kind.is_hostile() {
                            let id = self.next_mob_id;
                            self.next_mob_id += 1;
                            let pos = Vec3::new(sx as f32 + 0.5, top as f32 + 0.2, sz as f32 + 0.5);
                            self.mobs.push(MobEntity::spawn(id, kind, pos));
                        }
                    }
                }
            }
        }
    }

    /// Gravity + magnet pickup for item drops.
    fn update_drops(&mut self, dt: f32) {
        let player_center = self.player.position + Vec3::new(0.0, 0.9, 0.0);
        let mut to_remove: Vec<usize> = Vec::new();
        let mut collected: Vec<String> = Vec::new();
        for (i, drop) in self.drops.iter_mut().enumerate() {
            drop.age += dt;
            drop.velocity.y -= 20.0 * dt;
            let next = drop.position + drop.velocity * dt;
            // land on solid blocks
            if drop.velocity.y < 0.0
                && self.world.is_solid(next.x as i32, (next.y - 0.1) as i32, next.z as i32)
            {
                drop.velocity = Vec3::ZERO;
            } else {
                drop.position = next;
            }
            // magnet then pickup
            let d = player_center - drop.position;
            let dist = d.length();
            if dist < 2.0 && drop.age > 0.5 {
                drop.position += d.normalize() * (6.0 * dt);
            }
            if dist < 1.2 && drop.age > 0.5 {
                let taken = drop.stack.count.saturating_sub(
                    self.inventory.add_item(&drop.stack.item_id, drop.stack.count)
                );
                if taken > 0 {
                    collected.push(drop.stack.item_id.clone());
                }
                if drop.stack.count == taken {
                    to_remove.push(i);
                } else {
                    drop.stack.count -= taken;
                }
            }
        }
        for i in to_remove.into_iter().rev() {
            self.drops.remove(i);
        }
        for item in collected {
            let first_ever = self.chronicle.is_empty();
            self.quest_event(QuestEvent::Collected(item.clone()));
            if first_ever && item == "log" {
                self.chronicle_event(EventType::FirstCraft, "collected the first logs".into());
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
        let vr = self.settings.view_distance;
        for dx in -vr..=vr {
            for dz in -vr..=vr {
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

        self.try_spawn_villagers();

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
        let (v, i, wv, wi) = mesh_column_gpu(&self.world, cx, cz);
        let (min_y, max_y) = bounds_of(&v, &wv);
        self.batches.insert((cx, cz), MeshBatch::new(&self.device, &self.resources, &v, &i));
        if wv.is_empty() {
            self.water_batches.remove(&(cx, cz));
        } else {
            self.water_batches.insert((cx, cz), MeshBatch::new(&self.device, &self.resources, &wv, &wi));
        }
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
            self.water_batches.remove(&pos);
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
            let (v, i, wv, wi) = mesh_column_gpu(&self.world, bx, bz);
            let (min_y, max_y) = bounds_of(&v, &wv);
            self.batches.insert((bx, bz), MeshBatch::new(&self.device, &self.resources, &v, &i));
            if wv.is_empty() {
                self.water_batches.remove(&(bx, bz));
            } else {
                self.water_batches.insert((bx, bz), MeshBatch::new(&self.device, &self.resources, &wv, &wi));
            }
            self.cpu_meshes.insert((bx, bz), (v, i));
            self.column_bounds.insert((bx, bz), (min_y, max_y));
            self.dirty.insert((bx, bz));
        }
    }

    fn env(&self) -> lf_engine::scene::Env {
        let mut sky = self.time.sky_color();
        let mut day = self.time.sky_light_level();
        let mut fog_far = (self.settings.view_distance as f32 + 2.0) * 16.0;
        if self.weather_raining {
            for c in sky.iter_mut() {
                *c *= 0.7;
            }
            day *= 0.8;
            fog_far *= 0.6;
        }
        let eye_block = self.world.get_block(
            self.player.eye_position().x as i32,
            self.player.eye_position().y as i32,
            self.player.eye_position().z as i32,
        ).id();
        if let Some((fog_color, far)) = lf_engine::atmosphere::underwater_env(eye_block) {
            sky = fog_color;
            fog_far = far;
        }
        lf_engine::scene::Env {
            camera_pos: self.player.eye_position(),
            day_factor: day,
            fog_color: sky,
            fog_far,
        }
    }

    fn camera(&self) -> Camera {
        if self.ui_open == UiOpen::Title {
            let r = 34.0;
            let cx = self.spawn_point.x + self.title_orbit.cos() * r;
            let cz = self.spawn_point.z + self.title_orbit.sin() * r;
            let cy = self.spawn_point.y + 14.0;
            let mut camera = Camera::new(glam::Vec3::new(cx, cy, cz), self.spawn_point + glam::Vec3::new(0.0, 2.0, 0.0));
            camera.set_aspect(self.config.width, self.config.height);
            camera.fovy = self.settings.fov_degrees.to_radians();
            return camera;
        }
        let mut camera = Camera::new(self.player.eye_position(), self.player.eye_position() + self.player.look_dir());
        camera.set_aspect(self.config.width, self.config.height);
        camera.fovy = self.settings.fov_degrees.to_radians();
        camera
    }

    /// Path-trace the current view through the compute tracer (R key).
    fn take_raytraced_screenshot(&mut self) {
        let eye = self.player.eye_position();
        let look = self.player.look_dir();
        let center = (eye.x as i32, (eye.y as i32 + 20).min(220), eye.z as i32);
        let voxel_data = lf_engine::pathtrace::build_voxel_texture_data(center, &|x, y, z| {
            self.world.get_block(x, y, z).id()
        });
        let mut camera = self.camera();
        camera.set_aspect(800, 600);
        let tod = self.time.clone();
        let angle = tod.fraction() * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let sun = [angle.cos(), angle.sin().abs(), 0.25];
        self.screenshot_counter += 1;
        let path = std::path::PathBuf::from(format!("shots/rt_frame_{}.png", self.screenshot_counter));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        match lf_engine::pathtrace::pathtrace_to_image(
            &voxel_data, center, &camera, sun, tod.sky_light_level(), 800, 600, 32,
        ) {
            Ok(img) => {
                let _ = img.save(&path);
                tracing::info!("path-traced frame saved to {}", path.display());
                self.chat_log = vec![format!("ray-traced frame: {}", path.display())];
            }
            Err(e) => tracing::error!("path trace failed: {}", e),
        }
    }

    fn take_screenshot(&mut self) {
        let mut vertices: Vec<GpuVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for (v, i) in self.cpu_meshes.values() {
            let base = vertices.len() as u32;
            vertices.extend_from_slice(v);
            indices.extend(i.iter().map(|idx| idx + base));
        }
        let _ = (&self.water_batches); // water omitted from quick screenshots
        self.screenshot_counter += 1;
        let path = std::path::PathBuf::from(format!("shots/screenshot_{}.png", self.screenshot_counter));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let camera = self.camera();
        let env = self.env();
        let textures = lf_assets::generate_atlas();
        match lf_engine::headless::render_to_png(&vertices, &indices, &[], &[], &textures, &camera, &env, self.clear_color(), 1280, 720, &path, None) {
            Ok(()) => tracing::info!("screenshot saved to {}", path.display()),
            Err(e) => tracing::error!("screenshot failed: {}", e),
        }
    }

    fn clear_color(&self) -> [f64; 4] {
        let env = self.env();
        [env.fog_color[0] as f64, env.fog_color[1] as f64, env.fog_color[2] as f64, 1.0]
    }

    fn render(&mut self) {
        self.rebuild_drop_batch();
        let camera = self.camera();
        let env = self.env();
        let view_proj = camera.build_view_projection_matrix();
        for batch in self.batches.values() {
            batch.update_camera(&self.queue, &camera, &env);
        }
        for batch in self.water_batches.values() {
            batch.update_camera(&self.queue, &camera, &env);
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
        let clear = self.clear_color();
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Game Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: clear[0], g: clear[1], b: clear[2], a: clear[3] }),
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
                batch.draw(&mut pass, resources, false);
            }
            // sun/moon/stars above the terrain
            if let Some(batch) = &self.sky_batch {
                batch.draw(&mut pass, resources, false);
            }
            self.outline.draw(&mut pass);
            // Water: alpha-blended, far columns first for correct layering.
            let mut water_order: Vec<(f32, (i32, i32))> = self
                .water_batches
                .keys()
                .copied()
                .map(|pos| {
                    let dx = pos.0 as f32 * 16.0 + 8.0 - eye.x;
                    let dz = pos.1 as f32 * 16.0 + 8.0 - eye.z;
                    (-(dx * dx + dz * dz), pos)
                })
                .collect();
            water_order.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            for (_, pos) in water_order {
                if let Some(batch) = self.water_batches.get(&pos) {
                    batch.draw(&mut pass, resources, true);
                }
            }
            // clouds and weather ride the transparent pass
            if let Some(batch) = &self.cloud_batch {
                batch.draw(&mut pass, resources, true);
            }
            if let Some(batch) = &self.weather_batch {
                batch.draw(&mut pass, resources, true);
            }
            // Item drops ride the opaque pass.
            if let Some(batch) = &self.drop_batch {
                batch.draw(&mut pass, resources, false);
            }
        }
        // egui UI pass on top of the world.
        let (paint_jobs, screen) = {
            let window = &self.window;
            let device = &self.device;
            let queue = &self.queue;
            self.egui.end_frame(window, device, queue, &mut encoder)
        };
        self.egui.paint(&mut encoder, &view, paint_jobs, &screen);
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        let _ = self.window.pre_present_notify();
    }
}

impl GameState {
    /// Rebuild the single batch holding item-drop and mob cubes.
    /// Rebuild the single batch holding item-drop and mob cubes.
    fn rebuild_drop_batch(&mut self) {
        if self.drops.is_empty() && self.mobs.is_empty() {
            self.drop_batch = None;
            return;
        }
        let (mut vertices, mut indices) = (Vec::new(), Vec::new());
        let mut push_cube = |cx: f32, cy: f32, cz: f32, r: f32, tex: u32, vertices: &mut Vec<GpuVertex>, indices: &mut Vec<u32>| {
            let base = vertices.len() as u32;
            for (normal, corners, uvs) in cube_faces(r) {
                for (c, uv) in corners.iter().zip(uvs.iter()) {
                    vertices.push(GpuVertex {
                        position: [cx + c[0], cy + c[1], cz + c[2]],
                        normal,
                        tex_coord: *uv,
                        tex_index: tex,
                        ao: 1.0,
                        light: 0xF0,
                    });
                }
                indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
            }
        };
        // mobs (white cubes tinted by hurt flash; unique colors arrive with P7 mobs art)
        for mob in &self.mobs {
            let size = mob.mob_type.stats().size;
            let tex = match mob.mob_type {
                MobType::NullKnight => lf_assets::texture_index_for_block(registry::block::STONE),
                m if m.is_hostile() => lf_assets::texture_index_for_block(registry::block::MYCELIUM),
                _ => lf_assets::texture_index_for_block(registry::block::SNOW),
            };
            push_cube(mob.position.x, mob.position.y + size, mob.position.z, size, tex, &mut vertices, &mut indices);
        }
        // arrows render as thin pale streaks
        for arrow in &self.arrows {
            let tex = lf_assets::texture_index_for_block(registry::block::SNOW);
            push_cube(arrow.position.x, arrow.position.y, arrow.position.z, 0.08, tex, &mut vertices, &mut indices);
        }
        // villagers render as earthy cubes
        for v in &self.villagers {
            let tex = match v.job {
                VillagerJob::Farmer => lf_assets::texture_index_for_block(registry::block::GRASS),
                VillagerJob::Smith => lf_assets::texture_index_for_block(registry::block::IRON_ORE),
                VillagerJob::Trader => lf_assets::texture_index_for_block(registry::block::SAND),
                VillagerJob::Guard => lf_assets::texture_index_for_block(registry::block::STONE),
                VillagerJob::Bard => lf_assets::texture_index_for_block(registry::block::CHERRY_LEAVES),
                VillagerJob::Lorekeeper => lf_assets::texture_index_for_block(registry::block::CRAFTING_TABLE),
            };
            push_cube(v.position[0], v.position[1] + 0.9, v.position[2], 0.45, tex, &mut vertices, &mut indices);
        }
        // remote players render as pale cubes
        if let Some(n) = &self.net {
            for (_, rp) in n.remote_players.iter() {
                let tex = lf_assets::texture_index_for_block(registry::block::SNOW);
                push_cube(rp.pos[0], rp.pos[1] + 0.9, rp.pos[2], 0.45, tex, &mut vertices, &mut indices);
            }
        }
        // item drops bob
        for drop in &self.drops {
            let tex = drop_tex_layer(&drop.stack.item_id);
            let bob = (drop.age * 2.0).sin() * 0.05;
            push_cube(drop.position.x, drop.position.y + 0.15 + bob, drop.position.z, 0.15, tex, &mut vertices, &mut indices);
        }
        self.drop_batch = Some(MeshBatch::new(&self.device, &self.resources, &vertices, &indices));
    }
}

/// Six faces of an axis-aligned cube with mesher-compatible winding.
fn cube_faces(r: f32) -> [([f32; 3], [[f32; 3]; 4], [[f32; 2]; 4]); 6] {
    [
        ([-1.0, 0.0, 0.0], [[-r, -r, -r], [-r, r, -r], [-r, r, r], [-r, -r, r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
        ([1.0, 0.0, 0.0], [[r, -r, r], [r, r, r], [r, r, -r], [r, -r, -r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
        ([0.0, -1.0, 0.0], [[-r, -r, -r], [-r, -r, r], [r, -r, r], [r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
        ([0.0, 1.0, 0.0], [[-r, r, r], [-r, r, -r], [r, r, -r], [r, r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
        ([0.0, 0.0, -1.0], [[r, -r, -r], [r, r, -r], [-r, r, -r], [-r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
        ([0.0, 0.0, 1.0], [[-r, -r, r], [-r, r, r], [r, r, r], [r, -r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
    ]
}

/// Texture layer for a dropped item's cube.
fn drop_tex_layer(item_id: &str) -> u32 {
    match item_def(item_id).map(|d| d.kind) {
        Some(ItemKind::Block(b)) => lf_assets::texture_index_for_block(b),
        Some(ItemKind::Food(_)) => lf_assets::texture_index_for_block(registry::block::LEAVES),
        Some(ItemKind::Tool(_, _)) => lf_assets::texture_index_for_block(registry::block::LOG),
        _ => lf_assets::texture_index_for_block(registry::block::STONE),
    }
}

/// Mesh one column and convert to GPU vertices (opaque + water channels).
fn mesh_column_gpu(world: &World, cx: i32, cz: i32)
    -> (Vec<GpuVertex>, Vec<u32>, Vec<GpuVertex>, Vec<u32>) {
    let mesh = world.mesh_column(cx, cz, &|b| lf_assets::texture_index_for_block(b.id()));
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
    let vertices = to_gpu(&mesh.opaque.vertices);
    let water = to_gpu(&mesh.water.vertices);
    (vertices, mesh.opaque.indices, water, mesh.water.indices)
}

fn bounds_of(vertices: &[GpuVertex], water: &[GpuVertex]) -> (f32, f32) {
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for v in vertices.iter().chain(water.iter()) {
        min = min.min(v.position[1]);
        max = max.max(v.position[1]);
    }
    if min == f32::MAX {
        (0.0, 0.0)
    } else {
        (min, max)
    }
}

fn load_client_save(dir: &Path)
    -> (Inventory, PlayerStats, lf_game::TimeOfDay, HashMap<(i32, i32, i32), BlockEntity>, Vec<MobEntity>, Vec<Villager>, u32, QuestLog, Vec<ChronicleEvent>, ResearchState, Settings, lf_worldgen::WorldType, Vec<map::Waypoint>) {
    let mut inventory = Inventory::new();
    let mut stats = PlayerStats::default();
    let time = lf_game::TimeOfDay::from_fraction(0.30);
    let mut entities = HashMap::new();
    let mut mobs = Vec::new();
    let mut villagers = Vec::new();
    let mut kills = 0;
    let mut research = ResearchState::default();
    let mut settings = Settings::default();
    let mut quest_log = {
        let mut log = QuestLog::new();
        for q in starter_quests() {
            log.add_quest(q);
        }
        log
    };
    let mut chronicle = Vec::new();
    let mut research = ResearchState::default();
    let mut settings = Settings::default();
    if let Ok(bytes) = std::fs::read(dir.join("player_extras.dat")) {
        if let Ok(save) = bincode::deserialize::<ClientSave>(&bytes) {
            for (i, slot) in save.slots.iter().enumerate().take(inventory.slots.len()) {
                inventory.slots[i] = slot.clone();
            }
            stats.health = save.health.max(1.0);
            stats.hunger = save.hunger;
            entities.extend(save.block_entities);
            mobs = save.mobs;
            kills = save.kills;
            if let Some(q) = save.quest_log {
                quest_log = q;
            }
            chronicle = save.chronicle;
            let world_type = save.world_type.unwrap_or_default();
            if let Some(r) = save.research { research = r; }
            if let Some(s) = save.settings { settings = s; }
            villagers = save.villagers;
            let waypoints = save.waypoints;
            return (inventory, stats, lf_game::TimeOfDay::new(save.time_ticks), entities, mobs, villagers, kills, quest_log, chronicle, research, settings, world_type, waypoints);
        }
    }
    (inventory, stats, time, entities, mobs, villagers, kills, quest_log, chronicle, research, settings, lf_worldgen::WorldType::Normal, Vec::new())
}

/// Tiny deterministic-enough hash for cosmetic randomness.
fn pseudo_random(seed: u64) -> u64 {
    let mut h = seed.wrapping_mul(0x9E3779B97F4A7C15);
    h ^= h >> 31;
    h.wrapping_mul(0xC2B2AE3D27D4EB4F)
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
