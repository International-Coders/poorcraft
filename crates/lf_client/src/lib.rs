//! The playable client: window, input, first-person gameplay, world editing,
//! chunk streaming and persistence.
//!
//! NOTE: unreachable_patterns is DENIED because a stray `_ => {}` wildcard
//! once sat mid-match in window_event, silently killing every keyboard and
//! mouse handler after it (the compiler only warns by default).
#![deny(unreachable_patterns)]
//!
//! P2 scope: P1 gameplay plus background chunk generation, view distance,
//! frustum culling, world save/load with autosave, water/trees/caves/ores.

use std::collections::{HashMap, HashSet, VecDeque};
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
pub mod input;
pub mod lore;
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
const DEFAULT_VIEW_RADIUS: i32 = 5; // chunks generated/kept around the player
// Chunks past view_distance + this margin are dropped (after save); the
// margin keeps streaming from thrashing when the player crosses borders.
// Was a fixed 8, which left zero headroom at view distance 8.
const UNLOAD_MARGIN: i32 = 3;
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

/// Update the worker's wish: new center + radius (the view-distance
/// setting — the audit caught the radius hard-wired to 5, so the High
/// preset never streamed farther), and allow regeneration of chunks that
/// fell out of range.
fn sync_wish(w: &mut StreamWish, center: (i32, i32), radius: i32) {
    w.center = center;
    w.radius = radius;
    w.requested.retain(|p| chebyshev(*p, center) <= radius);
}

/// Background chunk generator: picks the nearest wanted chunk and generates
/// it until none are missing.
struct Streamer {
    wish: Arc<Mutex<StreamWish>>,
    rx: mpsc::Receiver<((i32, i32), ChunkColumn)>,
}

impl Streamer {
    fn spawn(seed: u64, skip: HashSet<(i32, i32)>, radius: i32) -> Self {
        let wish = Arc::new(Mutex::new(StreamWish {
            center: (0, 0),
            radius,
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

    fn set_center(&self, center: (i32, i32), radius: i32) {
        let mut w = self.wish.lock().unwrap();
        sync_wish(&mut w, center, radius);
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
    LoreBook,
    Smithing,
    TechTree,
    Map,
    Spellbook,
    Imbue,
    Carve,
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
    WaterWheel(lf_game::machines::WaterWheel),
    Battery(lf_game::machines::BatteryCell),
    Pipe(lf_game::machines::Pipe),
    Boiler(lf_game::machines::Boiler),
    SteamEngine(lf_game::machines::SteamEngine),
    Pump(lf_game::machines::PumpJack),
    Refinery(lf_game::machines::Refinery),
    Combustion(lf_game::machines::CombustionGenerator),
    Reactor(lf_game::machines::Reactor),
    Conduit,
    Screen { page: u8 },
}

#[derive(Clone, Debug)]
pub struct MiningState {
    pub pos: (i32, i32, i32),
    pub progress: f32,
    pub total: f32,
}

/// A small break/mining debris particle (billboard quad, cutout-safe).
#[derive(Clone, Debug)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub life: f32,
    pub tex: u32,
    /// Sub-tile of the block texture to sample (0..0.75, quarter-tile).
    pub uv_off: [f32; 2],
}

#[derive(Clone, Debug)]
pub struct ItemDrop {
    pub stack: ItemStack,
    pub position: Vec3,
    pub velocity: Vec3,
    pub age: f32,
}

/// A granular block (sand/dirt-family) detached from its support and
/// falling; lands and re-places itself as a block.
#[derive(Clone, Debug)]
pub struct FallingBlock {
    pub position: Vec3,
    pub velocity: f32,
    pub block: BlockState,
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
    /// Raster look at Medium + the live path-traced view on top (Pillar 4:
    /// the showcase, not the baseline).
    PathTraced,
}

/// Player-tunable settings, persisted with the world save.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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
    /// Rebindable keys as (action-index, key-name) pairs (Step 13).
    #[serde(default)]
    pub keymap_pairs: Vec<(u8, String)>,
    /// Active quality tier 0=Low 1=Medium 2=High 3=Path-Traced.
    #[serde(default = "default_quality")]
    pub quality: u8,
    /// Corner minimap rotates with the player (Step 15).
    #[serde(default = "default_true")]
    pub rotate_minimap: bool,
    /// Corner minimap zoom, px per block (Step 15).
    #[serde(default = "default_minimap_zoom")]
    pub minimap_zoom: f32,
}

fn default_minimap_zoom() -> f32 {
    1.0
}

fn default_quality() -> u8 {
    1
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
            keymap_pairs: Vec::new(),
            quality: default_quality(),
            rotate_minimap: true,
            minimap_zoom: 1.0,
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
                self.rt_mode = RtMode::Off;
            }
            Quality::High => {
                self.view_distance = 7;
                self.clouds = true;
                self.particles = true;
                self.rt_mode = RtMode::Off;
            }
            Quality::PathTraced => {
                self.view_distance = 5;
                self.clouds = true;
                self.particles = true;
                self.rt_mode = RtMode::Live;
            }
        }
        self.quality = match q {
            Quality::Low => 0,
            Quality::Medium => 1,
            Quality::High => 2,
            Quality::PathTraced => 3,
        };
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
    /// P33 magic: mana pool + learned spells. Extras moved to JSON in the
    /// same phase so serde(default) actually applies on old saves (bincode
    /// would EOF instead — caught empirically, see the migration tests).
    #[serde(default = "default_mana")]
    pub mana: f32,
    #[serde(default)]
    pub spellbook: Option<lf_game::magic::Spellbook>,
    #[serde(default)]
    pub runed: Vec<(String, String)>,
}

fn default_mana() -> f32 {
    lf_game::magic::MAX_MANA
}

/// The pre-magic ClientSave shape (bincode era). Kept frozen so worlds
/// saved before P33 migrate instead of silently resetting their extras.
#[derive(serde::Serialize, serde::Deserialize)]
struct LegacyClientSave {
    slots: Vec<Option<ItemStack>>,
    health: f32,
    hunger: f32,
    time_ticks: u64,
    block_entities: Vec<((i32, i32, i32), BlockEntity)>,
    mobs: Vec<lf_game::mobs::MobEntity>,
    kills: u32,
    quest_log: Option<QuestLog>,
    chronicle: Vec<ChronicleEvent>,
    world_type: Option<lf_worldgen::WorldType>,
    villagers: Vec<Villager>,
    research: Option<lf_game::research::ResearchState>,
    settings: Option<Settings>,
    #[serde(default)]
    waypoints: Vec<map::Waypoint>,
}

impl From<LegacyClientSave> for ClientSave {
    fn from(old: LegacyClientSave) -> Self {
        ClientSave {
            slots: old.slots,
            health: old.health,
            hunger: old.hunger,
            time_ticks: old.time_ticks,
            block_entities: old.block_entities,
            mobs: old.mobs,
            kills: old.kills,
            quest_log: old.quest_log,
            chronicle: old.chronicle,
            world_type: old.world_type,
            villagers: old.villagers,
            research: old.research,
            settings: old.settings,
            waypoints: old.waypoints,
            mana: default_mana(),
            spellbook: None,
            runed: Vec::new(),
        }
    }
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
        // Deep-input diagnosis (LOREFORGE_DEBUG_INPUT): trace every key/mouse
        // event from the OS through egui consumption to the input map.
        let debug_input = std::env::var("LOREFORGE_DEBUG_INPUT").is_ok();
        if debug_input && matches!(event, WindowEvent::KeyboardInput { .. } | WindowEvent::MouseInput { .. }) {
            tracing::info!("[input] {:?} ui_open={:?} locked={}", event, state.ui_open, state.input.cursor_locked);
        }
        // egui sees events only while a screen is open (the HUD itself is
        // display-only).
        if state.ui_open != UiOpen::None {
            let consumed = state.egui.on_event(&state.window, &event);
            if debug_input && matches!(event, WindowEvent::KeyboardInput { .. } | WindowEvent::MouseInput { .. }) {
                tracing::info!("[input] egui consumed={}", consumed);
            }
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
                                // Step 13: a pending rebind captures the next press
                                if pressed {
                                    if let Some(action) = state.rebind_capture.take() {
                                        state.keymap.rebind(action, code);
                                        state.settings.keymap_pairs = state.keymap.to_pairs();
                                        return;
                                    }
                                }
                                // Step 13: rebindable action keys (settings > controls)
                                let chat_key = state.keymap.key(crate::input::Action::Chat);
                                let console_key = state.keymap.key(crate::input::Action::Console);
                                let map_key = state.keymap.key(crate::input::Action::Map);
                                let tech_key = state.keymap.key(crate::input::Action::TechTree);
                                let quest_key = state.keymap.key(crate::input::Action::QuestLog);
                                let inv_key = state.keymap.key(crate::input::Action::Inventory);
                                let fly_key = state.keymap.key(crate::input::Action::Fly);
                                let shot_key = state.keymap.key(crate::input::Action::Screenshot);
                                let dbg_key = state.keymap.key(crate::input::Action::DebugInfo);
                                let rt_key = state.keymap.key(crate::input::Action::RtCapture);
                                let grid_key = state.keymap.key(crate::input::Action::GridOverlay);
                                let book_key = state.keymap.key(crate::input::Action::Spellbook);
                                let sym_key = state.keymap.key(crate::input::Action::Symmetry);
                                let spell_keys = [
                                    state.keymap.key(crate::input::Action::Spell1),
                                    state.keymap.key(crate::input::Action::Spell2),
                                    state.keymap.key(crate::input::Action::Spell3),
                                ];
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
                                                UiOpen::Settings => {
                                                    // returns to the title
                                                    // screen when opened
                                                    // from there
                                                    state.close_settings();
                                                    return;
                                                }
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
                                        k if k == chat_key => {
                                            if state.net.is_some() && state.ui_open == UiOpen::None && state.stats.health > 0.0 {
                                                state.chat_input = Some(String::new());
                                                state.unlock_cursor();
                                                return;
                                            }
                                        }
                                        k if k == console_key => {
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
                                        k if k == map_key => {
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
                                        k if k == tech_key => {
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
                                        k if k == quest_key => {
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
                                        k if k == inv_key => {
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
                                        k if k == fly_key => {
                                            state.player.flying = !state.player.flying;
                                            state.player.velocity = Vec3::ZERO;
                                        }
                                        k if k == shot_key => state.take_screenshot(),
                                        k if k == dbg_key => {
                                            state.show_debug = !state.show_debug;
                                        }
                                        k if k == rt_key => { if state.settings.rt_mode == crate::RtMode::Captures { state.take_raytraced_screenshot(); } }
                                        k if k == sym_key => {
                                            if matches!(state.ui_open, UiOpen::None) && state.stats.health > 0.0 {
                                                state.symmetry_plane = match state.symmetry_plane {
                                                    Some(_) => {
                                                        state.push_hint("symmetry off");
                                                        None
                                                    }
                                                    None => {
                                                        state.push_hint("symmetry plane set — place & break are mirrored (V to clear)");
                                                        Some(state.player.position.x)
                                                    }
                                                };
                                                return;
                                            }
                                        }
                                        k if k == book_key => {
                                            if matches!(state.ui_open, UiOpen::None | UiOpen::Spellbook) && state.stats.health > 0.0 {
                                                if state.ui_open == UiOpen::Spellbook {
                                                    state.close_ui();
                                                } else {
                                                    state.ui_open = UiOpen::Spellbook;
                                                    state.menu_reveal = 0.0;
                                                    state.unlock_cursor();
                                                }
                                                return;
                                            }
                                        }
                                        k if spell_keys.contains(&k) => {
                                            if state.ui_open == UiOpen::None && state.stats.health > 0.0 {
                                                let slot = spell_keys.iter().position(|&sk| sk == k).unwrap_or(0);
                                                state.cast_from_slot(slot);
                                                return;
                                            }
                                        }
                                        k if k == grid_key => {
                                            if matches!(state.ui_open, UiOpen::None) && state.stats.health > 0.0 {
                                                state.grid_overlay = !state.grid_overlay;
                                                if !state.grid_overlay {
                                                    state.overlay_batch = None;
                                                }
                                                return;
                                            }
                                        }
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
    /// (target, stage) the crack decal batch was built for.
    crack_state: Option<((i32, i32, i32), u32)>,
    crack_batch: Option<MeshBatch>,
    particle_batch: Option<MeshBatch>,
    pub particles: Vec<Particle>,
    particle_timer: f32,
    /// Total elapsed seconds (drives foliage wind).
    elapsed: f32,
    pub drops: Vec<ItemDrop>,
    drop_batch: Option<MeshBatch>,
    pub block_entities: HashMap<(i32, i32, i32), BlockEntity>,
    pub mobs: Vec<MobEntity>,
    pub villagers: Vec<Villager>,
    pub arrows: Vec<Arrow>,
    /// Granular blocks mid-fall (sand/dirt-family), animated until they
    /// land and re-place themselves.
    pub falling_blocks: Vec<FallingBlock>,
    /// Pending water-simulation cells (deduplicated by `fluid_queued`).
    fluid_queue: VecDeque<(i32, i32, i32)>,
    fluid_queued: HashSet<(i32, i32, i32)>,
    /// Current per-biome color grade (smoothly lerped toward the biome the
    /// player stands in — hard cuts read as bugs).
    pub grade_tint: [f32; 3],
    pub grade_sat: f32,
    /// Procedural sound player (None = no output device: silent fallback).
    pub audio: Option<lf_audio::Audio>,
    /// Screen-shake amplitude (impact pulse on heavy breaks), decays fast.
    pub shake: f32,
    /// Rebindable action -> key map (Step 13), persisted via Settings.
    pub keymap: crate::input::Keymap,
    /// When Some, the next key press rebinds this action (Controls tab).
    pub rebind_capture: Option<crate::input::Action>,
    /// Last slot-thumbnail capture (throttled; Step 14).
    last_thumb: Instant,
    /// Loaded save-slot thumbnails for the picker (Step 14).
    pub slot_thumbs: std::collections::HashMap<String, egui::TextureHandle>,
    /// Lore tomes from lore/books.toml (Step 20) + reader page position.
    pub lore: lore::LoreLibrary,
    pub open_lore_page: usize,
    /// Title of the tome currently open (matches lore.for_item by item).
    pub open_lore_title: Option<String>,
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
    /// Settings was opened from the title screen — Back/Esc must return
    /// there, not drop the player into the world (doc 02 first-launch audit).
    pub settings_from_title: bool,
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
    /// World-space waypoint beacon beams (Step 15), transparent pass.
    waypoint_batch: Option<MeshBatch>,
    /// Power-grid overlay (Step 25): on = tint cubes over every machine,
    /// green = powered, red = starved. `machine_power` holds the EU ratio
    /// granted last tick per machine position.
    pub grid_overlay: bool,
    pub machine_power: std::collections::HashMap<(i32, i32, i32), f32>,
    overlay_batch: Option<MeshBatch>,
    /// P33 magic: the learned spell set + 3 cast slots.
    pub spellbook: lf_game::magic::Spellbook,
    /// Ward spell: seconds of damage absorption left.
    pub ward_timer: f32,
    /// Firebolts in flight (arrows that hit harder and burn).
    pub firebolts: Vec<lf_game::combat::Arrow>,
    /// Temporary light blocks placed by Hearthlight (despawn positions).
    hearth_lights: std::collections::HashMap<(i32, i32, i32), f32>,
    /// The enchanting minigame (P33).
    pub imbue: lf_game::magic::ImbueMinigame,
    /// P34: symmetry plane x (mirrors place/break across it).
    pub symmetry_plane: Option<f32>,
    /// P35: live producer positions (elevator/climate power checks).
    pub producer_positions: Vec<(i32, i32, i32)>,
    /// P35: the screen texture is rewritten only when its data changes.
    screen_signature: u64,
    /// P34 blueprints: first corner marker, the captured clipboard, and
    /// the file it came from.
    pub bp_corner_a: Option<(i32, i32, i32)>,
    pub bp_clip: Option<lf_game::construction::Blueprint>,
    pub bp_path: Option<std::path::PathBuf>,
    /// P34: the statue-carving minigame + which stone it works on.
    pub carve: lf_game::smithing::CarveMinigame,
    pub carve_target: Option<(i32, i32, i32)>,
    /// Runed tools: item id -> rune item id (P33). Persisted in extras.
    pub runed_tools: std::collections::HashMap<String, String>,
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

        // Load mods into the live registries before touching the world.
        let mods = lf_modapi::load_mods_dir(Path::new("mods"));
        if !mods.is_empty() {
            tracing::info!("loaded {} mod(s): {:?}", mods.len(),
                mods.iter().map(|m| m.manifest.id.clone()).collect::<Vec<_>>());
        }
        if let Some(line) = lf_modapi::smoke_line(&mods) {
            tracing::info!("{line}");
        }

        // Persistence + world bootstrap: the slot owns the directory AND the
        // seed (fresh random seed for a brand-new world). Player extras come
        // from the booted slot — the old pre-slot code loaded the legacy
        // worlds/default before boot_slot() ran, so a slotted player booted
        // with default inventory/settings until they clicked Play.
        let mut slot_meta = slots::boot_slot();
        let world_dir = slots::slot_dir(&slot_meta.name);
        slots::sync_generator_version(&world_dir);
        let (inventory, mut stats, time, block_entities, mobs, villagers, kills, quest_log, chronicle, research, settings, world_type, waypoints, spellbook, runed) =
            load_client_save(&world_dir);
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
        let streamer = Streamer::spawn(world_seed, worker_skip, settings.view_distance);

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
            crack_state: None,
            crack_batch: None,
            particle_batch: None,
            particles: Vec::new(),
            particle_timer: 0.0,
            elapsed: 0.0,
            block_entities: HashMap::new(),
            mobs: Vec::new(),
            villagers: Vec::new(),
            arrows: Vec::new(),
            falling_blocks: Vec::new(),
            fluid_queue: VecDeque::new(),
            fluid_queued: HashSet::new(),
            grade_tint: [1.0, 1.0, 1.0],
            grade_sat: 1.0,
            audio: lf_audio::Audio::new(),
            shake: 0.0,
            keymap: crate::input::Keymap::from_pairs(&settings.keymap_pairs),
            rebind_capture: None,
            last_thumb: Instant::now() - Duration::from_secs(600), // capture soon after boot
            slot_thumbs: std::collections::HashMap::new(),
            lore: lore::LoreLibrary::load(Path::new("lore/books.toml")),
            open_lore_page: 0,
            open_lore_title: None,
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
            settings_from_title: false,
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
            spellbook,
            runed_tools: runed,
            imbue: lf_game::magic::ImbueMinigame::new(3),
            symmetry_plane: None,
            producer_positions: Vec::new(),
            screen_signature: 0,
            bp_corner_a: None,
            bp_clip: None,
            bp_path: None,
            carve: lf_game::smithing::CarveMinigame::new(3),
            carve_target: None,
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
            waypoint_batch: None,
            grid_overlay: false,
            machine_power: std::collections::HashMap::new(),
            overlay_batch: None,
            ward_timer: 0.0,
            firebolts: Vec::new(),
            hearth_lights: std::collections::HashMap::new(),
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
        self.streamer = Streamer::spawn(seed, skip, self.settings.view_distance);
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
        let _ = lf_worldgen::save_generator_version(&dir, lf_worldgen::GENERATOR_VERSION);
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
        slots::sync_generator_version(&dir);
        self.save_world();
        // state from the slot's save files
        let (inventory, stats, time, block_entities, mobs, villagers, kills, quest_log,
             chronicle, research, settings, world_type, waypoints, spellbook, runed) = load_client_save(&dir);
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
        self.runed_tools = runed;
        self.research = research;
        self.settings = settings;
        self.keymap = crate::input::Keymap::from_pairs(&self.settings.keymap_pairs);
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
            settings: Some(self.settings.clone()),
            waypoints: self.waypoints.clone(),
            mana: self.stats.mana,
            spellbook: Some(self.spellbook.clone()),
            runed: self.runed_tools.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        };
        // JSON (self-describing) so future field additions with
        // serde(default) load old bytes — bincode EOFs on them instead.
        if let Ok(bytes) = serde_json::to_vec(&extras) {
            let _ = std::fs::write(self.world_dir.join("player_extras.json"), bytes);
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
        self.capture_slot_thumbnail();
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
        // Deep-input diagnosis: 1Hz state summary.
        if std::env::var("LOREFORGE_DEBUG_INPUT").is_ok() && self.frame % 60 == 0 {
            tracing::info!(
                "[input] tick#{} ui_open={:?} locked={} playing={} health={:.1} keys_held={} frame_ms={:.1}",
                self.frame, self.ui_open, self.input.cursor_locked,
                matches!(self.ui_open, UiOpen::None) && self.stats.health > 0.0,
                self.stats.health,
                self.input.keys.values().filter(|&&p| p).count(),
                (dt * 1000.0).min(9999.0),
            );
        }
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
            // Step 13: movement reads the rebindable keymap
            let km = &self.keymap;
            use crate::input::Action as A;
            PlayerInput {
                forward: self.input.held(km.key(A::Forward)),
                back: self.input.held(km.key(A::Back)),
                left: self.input.held(km.key(A::Left)),
                right: self.input.held(km.key(A::Right)),
                jump: self.input.held(km.key(A::Jump)),
                sneak: self.input.held(km.key(A::Sneak)),
                sprint: self.input.held(km.key(A::Sprint)),
                fly_up: self.input.held(km.key(A::FlyUp)),
                fly_down: self.input.held(km.key(A::FlyDown)),
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
        // Power: sources (generator/wheel/battery) are pulled out, the pure
        // distribute_power runs the field (producers first, batteries cover
        // gaps, surplus recharges), then everything is reinserted.
        use lf_game::machines::PowerSource;
        let mut sources: Vec<((i32, i32, i32), PowerSource)> = Vec::new();
        let mut machine_positions: Vec<(i32, i32, i32)> = Vec::new();
        let mut conduit_positions: Vec<(i32, i32, i32)> = Vec::new();
        for (pos, entity) in self.block_entities.iter_mut() {
            match entity {
                BlockEntity::Generator(g) => {
                    g.tick(dt);
                    sources.push((*pos, PowerSource::Generator(g.clone())));
                }
                BlockEntity::WaterWheel(w) => {
                    // river-gated: any adjacent water (4 sides + below) spins it
                    let has_water = [
                        (pos.0 + 1, pos.1, pos.2), (pos.0 - 1, pos.1, pos.2),
                        (pos.0, pos.1, pos.2 + 1), (pos.0, pos.1, pos.2 - 1),
                        (pos.0, pos.1 - 1, pos.2),
                    ].iter().any(|p| self.world.get_block(p.0, p.1, p.2).id() == registry::block::WATER);
                    w.tick(dt, has_water);
                    sources.push((*pos, PowerSource::Wheel(w.clone())));
                }
                BlockEntity::Battery(b) => {
                    sources.push((*pos, PowerSource::Battery(b.clone())));
                }
                BlockEntity::SteamEngine(_) => {} // fed in the steam pass below
                BlockEntity::Pipe(_) | BlockEntity::Boiler(_) => {} // steam pass below
                BlockEntity::Combustion(c) => {
                    c.tick(dt);
                    sources.push((*pos, PowerSource::Combustion(c.clone())));
                }
                BlockEntity::Reactor(_) => {} // nuclear pass below
                BlockEntity::Pump(_) | BlockEntity::Refinery(_) => {
                    // powered consumers — ticked in the granted loop below
                    machine_positions.push(*pos);
                }
                BlockEntity::Conduit => {
                    // P35: relays extend the field (positions only)
                    conduit_positions.push(*pos);
                }
                BlockEntity::Screen { .. } => {}
                BlockEntity::Furnace(f) => {
                    f.tick(dt);
                }
                BlockEntity::ElectricFurnace(_) | BlockEntity::Crusher(_) | BlockEntity::Assembler(_) => {
                    machine_positions.push(*pos);
                }
                BlockEntity::Chest { .. } => {}
            }
        }
        // Steam Age pass: equalize adjacent pipes pairwise (canonical
        // direction per pair), then feed boilers from adjacent water
        // sources (like a pump) and pipes.
        {
            let keys: Vec<(i32, i32, i32)> = self.block_entities.keys().copied().collect();
            for i in 0..keys.len() {
                for j in (i + 1)..keys.len() {
                    let (a, b) = (keys[i], keys[j]);
                    let manhattan = (a.0 - b.0).abs() + (a.1 - b.1).abs() + (a.2 - b.2).abs();
                    if manhattan != 1 {
                        continue;
                    }
                    let (Some(BlockEntity::Pipe(mut x)), Some(BlockEntity::Pipe(mut y))) =
                        (self.block_entities.remove(&a), self.block_entities.remove(&b))
                    else {
                        continue;
                    };
                    x.equalize_with(&mut y);
                    self.block_entities.insert(a, BlockEntity::Pipe(x));
                    self.block_entities.insert(b, BlockEntity::Pipe(y));
                }
            }
            for k in keys.clone() {
                let neighbors = [
                    (k.0 + 1, k.1, k.2), (k.0 - 1, k.1, k.2),
                    (k.0, k.1, k.2 + 1), (k.0, k.1, k.2 - 1),
                    (k.0, k.1 - 1, k.2),
                ];
                let mut water_in = 0u16;
                for n in &neighbors {
                    if self.world.get_block(n.0, n.1, n.2).id() == registry::block::WATER {
                        water_in += 40; // pump-like: sources are not consumed
                    }
                }
                for n in &neighbors {
                    if let Some(BlockEntity::Pipe(p)) = self.block_entities.get_mut(n) {
                        water_in += p.draw(lf_game::machines::FluidKind::Water, 30);
                    }
                }
                if let Some(BlockEntity::Boiler(b)) = self.block_entities.get_mut(&k) {
                    let burning = b.tick(dt, water_in);
                    if burning && self.settings.particles && self.frame % 6 == 0 {
                        // steam puffs rise from the drum
                        let tex = lf_assets::texture_index_for_block(registry::block::SNOW);
                        let seed = (k.0 as u32).wrapping_mul(31).wrapping_add(k.2 as u32).wrapping_add(self.frame as u32);
                        let r1 = (seed % 97) as f32 / 97.0;
                        self.particles.push(Particle {
                            position: Vec3::new(k.0 as f32 + 0.3 + r1 * 0.4, k.1 as f32 + 1.05, k.2 as f32 + 0.5),
                            velocity: Vec3::new((r1 - 0.5) * 0.4, 1.4 + r1, (r1 - 0.5) * 0.4),
                            life: 0.9,
                            tex,
                            uv_off: [0.0, 0.0],
                        });
                    }
                }
            }
            // engines drink from adjacent boilers, then buffer power
            for k in keys.clone() {
                let is_engine = matches!(self.block_entities.get(&k), Some(BlockEntity::SteamEngine(_)));
                if !is_engine {
                    continue;
                }
                let mut e = match self.block_entities.get(&k) {
                    Some(BlockEntity::SteamEngine(e)) => e.clone(),
                    _ => continue,
                };
                let neighbors = [
                    (k.0 + 1, k.1, k.2), (k.0 - 1, k.1, k.2),
                    (k.0, k.1, k.2 + 1), (k.0, k.1, k.2 - 1),
                    (k.0, k.1 - 1, k.2), (k.0, k.1 + 1, k.2),
                ];
                let mut steam_in = 0.0f32;
                for n in &neighbors {
                    if let Some(BlockEntity::Boiler(b)) = self.block_entities.get_mut(n) {
                        steam_in += b.draw_steam(lf_game::machines::STEAM_ENGINE_INTAKE * dt * 2.0);
                    }
                }
                e.tick(dt, steam_in);
                if let Some(entity) = self.block_entities.get_mut(&k) {
                    if let BlockEntity::SteamEngine(target) = entity {
                        *target = e.clone();
                    }
                }
                sources.push((k, PowerSource::Engine(e)));
            }
        }
        // P35: remember where the producers are (elevator/climate power
        // checks read this without touching the source structs).
        let producer_positions: Vec<(i32, i32, i32)> = sources.iter().map(|(p, _)| *p).collect();
        // Oil Age pass: refineries drink crude from adjacent pipes before
        // the power step (fluid movement is free; refining is not).
        let mut refinery_feed: std::collections::HashMap<(i32, i32, i32), u16> =
            std::collections::HashMap::new();
        {
            let keys: Vec<(i32, i32, i32)> = self.block_entities.keys().copied().collect();
            for k in keys {
                if !matches!(self.block_entities.get(&k), Some(BlockEntity::Refinery(_))) {
                    continue;
                }
                let neighbors = [
                    (k.0 + 1, k.1, k.2), (k.0 - 1, k.1, k.2),
                    (k.0, k.1, k.2 + 1), (k.0, k.1, k.2 - 1),
                    (k.0, k.1 - 1, k.2), (k.0, k.1 + 1, k.2),
                ];
                let mut crude_in = 0u16;
                for n in &neighbors {
                    if let Some(BlockEntity::Pipe(p)) = self.block_entities.get_mut(n) {
                        crude_in += p.draw(lf_game::machines::FluidKind::Crude, 40);
                    }
                }
                refinery_feed.insert(k, crude_in);
            }
        }
        // Nuclear pass (P32): reactors drink coolant from adjacent pipes
        // and water, tick the heat/output curve, and a meltdown is applied
        // to the world immediately (never silently safe).
        {
            let keys: Vec<(i32, i32, i32)> = self.block_entities.keys().copied().collect();
            for k in keys {
                let mut reactor = match self.block_entities.get(&k) {
                    Some(BlockEntity::Reactor(r)) => r.clone(),
                    _ => continue,
                };
                let neighbors = [
                    (k.0 + 1, k.1, k.2), (k.0 - 1, k.1, k.2),
                    (k.0, k.1, k.2 + 1), (k.0, k.1, k.2 - 1),
                    (k.0, k.1 - 1, k.2), (k.0, k.1 + 1, k.2),
                ];
                let mut coolant_in = 0u16;
                for n in &neighbors {
                    if self.world.get_block(n.0, n.1, n.2).id() == registry::block::WATER {
                        coolant_in += 40;
                    }
                }
                for n in &neighbors {
                    if let Some(BlockEntity::Pipe(p)) = self.block_entities.get_mut(n) {
                        coolant_in += p.draw(lf_game::machines::FluidKind::Water, 30);
                    }
                }
                let event = reactor.tick(dt, coolant_in);
                if event == lf_game::machines::ReactorEvent::Meltdown {
                    self.apply_meltdown(k);
                    continue;
                }
                if event == lf_game::machines::ReactorEvent::Scrammed
                    && self.settings.particles && self.frame % 10 == 0
                {
                    // steam venting from a hot core
                    let tex = lf_assets::texture_index_for_block(registry::block::SNOW);
                    self.particles.push(Particle {
                        position: Vec3::new(k.0 as f32 + 0.5, k.1 as f32 + 1.1, k.2 as f32 + 0.5),
                        velocity: Vec3::new(0.0, 2.2, 0.0),
                        life: 0.7,
                        tex,
                        uv_off: [0.0, 0.0],
                    });
                }
                if let Some(entity) = self.block_entities.get_mut(&k) {
                    if let BlockEntity::Reactor(target) = entity {
                        *target = reactor.clone();
                    }
                }
                sources.push((k, PowerSource::Reactor(reactor)));
            }
        }
        self.producer_positions = producer_positions;
        let need = lf_game::machines::DRAW_RATE * dt;
        self.machine_power.clear();
        let granted = lf_game::machines::distribute_power_relayed(
            &mut sources, &conduit_positions, &machine_positions, need);        for (spos, src) in sources {
            let entity = match src {
                PowerSource::Generator(g) => BlockEntity::Generator(g),
                PowerSource::Wheel(w) => BlockEntity::WaterWheel(w),
                PowerSource::Battery(b) => BlockEntity::Battery(b),
                PowerSource::Engine(e) => BlockEntity::SteamEngine(e),
                PowerSource::Combustion(c) => BlockEntity::Combustion(c),
                PowerSource::Reactor(r) => BlockEntity::Reactor(r),
            };
            let _ = &conduit_positions;
            self.block_entities.insert(spos, entity);
        }
        for (mi, mpos) in machine_positions.iter().enumerate() {
            let powered = granted[mi];
            // grid overlay (Step 25): remember this frame's power verdict as
            // a ratio (granted / needed) so the tint cubes can classify
            self.machine_power.insert(*mpos, powered / need.max(1e-6));
            // pumpjacks lift crude while powered and sitting on oil
            let mut pump_out: Vec<((i32, i32, i32), u16)> = Vec::new();
            let pump_adjacent_oil = matches!(self.block_entities.get(mpos), Some(BlockEntity::Pump(_))) && [
                (mpos.0 + 1, mpos.1, mpos.2), (mpos.0 - 1, mpos.1, mpos.2),
                (mpos.0, mpos.1, mpos.2 + 1), (mpos.0, mpos.1, mpos.2 - 1),
                (mpos.0, mpos.1 - 1, mpos.2),
            ].iter().any(|p| {
                let b = self.world.get_block(p.0, p.1, p.2);
                b.id() == registry::block::OIL && lf_voxel::oil_level(b) == 0
            });
            if let Some(entity) = self.block_entities.get_mut(mpos) {
                match entity {
                    BlockEntity::ElectricFurnace(f) => { f.tick(dt, powered); }
                    BlockEntity::Crusher(cr) => { cr.tick(dt, powered); }
                    BlockEntity::Assembler(a) => { a.tick(dt, powered); }
                    BlockEntity::Pump(p) => {
                        let mb = p.tick(dt, powered, pump_adjacent_oil);
                        if mb > 0 {
                            pump_out.push((*mpos, mb));
                        }
                    }
                    BlockEntity::Refinery(r) => {
                        let feed = refinery_feed.remove(mpos).unwrap_or(0);
                        r.tick(dt, powered, feed);
                    }
                    _ => {}
                }
            }
            // pumped crude flows into adjacent pipes
            for (ppos, mb) in pump_out {
                let neighbors = [
                    (ppos.0 + 1, ppos.1, ppos.2), (ppos.0 - 1, ppos.1, ppos.2),
                    (ppos.0, ppos.1, ppos.2 + 1), (ppos.0, ppos.1, ppos.2 - 1),
                    (ppos.0, ppos.1 - 1, ppos.2),
                ];
                let mut left = mb;
                for n in &neighbors {
                    if left == 0 {
                        break;
                    }
                    if let Some(BlockEntity::Pipe(p)) = self.block_entities.get_mut(n) {
                        let took = p.fill(lf_game::machines::FluidKind::Crude, left);
                        left -= took;
                    }
                }
            }
        }

        // Water simulation + animated falling blocks.
        self.update_falling_blocks(dt);
        self.tick_fluids();

        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
        self.update_firebolts(dt);
        self.tick_hearth_lights(dt);
        self.ward_timer = (self.ward_timer - dt).max(0.0);
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

        self.elapsed += dt;

        // Break particles: gravity + simple ground stop.
        for pt in self.particles.iter_mut() {
            pt.life -= dt;
            pt.velocity.y -= 18.0 * dt;
            let next = pt.position + pt.velocity * dt;
            if registry::is_solid(self.world.get_block(next.x as i32, next.y as i32, next.z as i32)) {
                pt.velocity = Vec3::ZERO;
            } else {
                pt.position = next;
            }
        }
        self.particles.retain(|pt| pt.life > 0.0);

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
                let mut total = break_time(block_id, held.as_ref()).unwrap_or(f32::INFINITY);
                // P33: a bound Rune of Haste speeds the held tool
                if let Some(h) = held.as_ref() {
                    if let Some(rune_id) = self.runed_tools.get(&h.item_id) {
                        if let Some(rune) = lf_game::magic::Rune::from_item(rune_id) {
                            total /= rune.mining_multiplier();
                        }
                    }
                }
                match &mut self.mining {
                    Some(m) if m.pos == key => m.progress += dt,
                    Some(m) => {
                        m.pos = key;
                        m.progress = dt;
                        m.total = total;
                    }
                    None => self.mining = Some(MiningState { pos: key, progress: dt, total }),
                }
                // steady debris while grinding the block
                self.particle_timer += dt;
                if self.particle_timer > 0.15 && self.settings.particles {
                    self.particle_timer = 0.0;
                    self.spawn_break_particles(block_id, (pos.x, pos.y, pos.z), 2);
                }
                if let Some(m) = &mut self.mining {
                    m.total = total;
                    if m.progress >= m.total {
                        self.mining = None;
                        if self.world.set_block(pos.x, pos.y, pos.z, BlockState::AIR).is_some() {
                            self.break_block_drops(block_id, pos);
                            self.play_block_sound(block_id, lf_audio::Action::Break);
                            self.break_impulse(block_id);
                            if self.settings.particles {
                                self.spawn_break_particles(block_id, (pos.x, pos.y, pos.z), 16);
                            }
                            self.use_durability();
                            // P34: breaking a scaffold drops the whole
                            // connected column above it (bulk-remove)
                            if block_id == registry::block::SCAFFOLD {
                                let mut y = pos.y + 1;
                                while self.world.get_block(pos.x, y, pos.z).id() == registry::block::SCAFFOLD {
                                    self.world.set_block(pos.x, y, pos.z, BlockState::AIR);
                                    self.spawn_drop("scaffold", 1, Vec3::new(pos.x as f32 + 0.5, y as f32 + 0.5, pos.z as f32 + 0.5));
                                    y += 1;
                                }
                                if y > pos.y + 1 {
                                    self.remesh_around(pos.x, pos.z);
                                }
                            }
                            // P34: symmetry mirrors the break across the plane
                            if let Some(px) = self.symmetry_plane {
                                let mx = (2.0 * px - pos.x as f32).round() as i32;
                                let mirrored = self.world.get_block(mx, pos.y, pos.z);
                                if mirrored.id() == block_id {
                                    self.world.set_block(mx, pos.y, pos.z, BlockState::AIR);
                                    self.break_block_drops(block_id, glam::IVec3::new(mx, pos.y, pos.z));
                                    self.remesh_around(mx, pos.z);
                                    self.after_edit(mx, pos.y, pos.z);
                                    if let Some(n) = &self.net {
                                        n.send_block(mx, pos.y, pos.z, registry::block::AIR);
                                    }
                                }
                            }
                            self.remesh_around(pos.x, pos.z);
                            // wake water + drop unsupported granular blocks
                            self.after_edit(pos.x, pos.y, pos.z);
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
                                    BlockEntity::WaterWheel(_) | BlockEntity::Battery(_)
                                    | BlockEntity::Pipe(_) | BlockEntity::SteamEngine(_)
                                    | BlockEntity::Pump(_) => vec![],
                                    BlockEntity::Boiler(b) => vec![b.fuel],
                                    BlockEntity::Refinery(r) => vec![r.fuel_out, r.tar_out],
                                    BlockEntity::Combustion(c) => vec![c.fuel],
                                    BlockEntity::Reactor(r) => vec![r.fuel],
                                    BlockEntity::Conduit | BlockEntity::Screen { .. } => vec![],
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
                    registry::block::COMPUTER => {
                        // P35: the screen cycles its page on interact —
                        // tech tree / chronicle / grid status
                        let key = (pos.x, pos.y, pos.z);
                        let page = match self.block_entities.entry(key) {
                            std::collections::hash_map::Entry::Occupied(mut e) => {
                                if let BlockEntity::Screen { page } = e.get_mut() {
                                    *page = *page % 3 + 1;
                                    *page
                                } else {
                                    1
                                }
                            }
                            std::collections::hash_map::Entry::Vacant(v) => {
                                v.insert(BlockEntity::Screen { page: 1 });
                                1
                            }
                        };
                        self.push_hint(match page {
                            1 => "screen: research status",
                            2 => "screen: chronicle",
                            _ => "screen: power grid",
                        });
                    }
                    registry::block::ENCHANTING_TABLE => {
                        self.ui_open = UiOpen::Imbue;
                        self.menu_reveal = 0.0;
                        self.unlock_cursor();
                    }
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
                    | registry::block::ASSEMBLER
                    | registry::block::WATER_WHEEL
                    | registry::block::BATTERY
                    | registry::block::PIPE
                    | registry::block::BOILER
                    | registry::block::STEAM_ENGINE
                    | registry::block::PUMP
                    | registry::block::REFINERY
                    | registry::block::COMBUSTION_GENERATOR
                    | registry::block::REACTOR => {
                        let key = (pos.x, pos.y, pos.z);
                        let block_id_here = self.world.get_block(pos.x, pos.y, pos.z).id();
                        self.block_entities.entry(key).or_insert_with(|| match block_id_here {
                            registry::block::COAL_GENERATOR => BlockEntity::Generator(Default::default()),
                            registry::block::ELECTRIC_FURNACE => BlockEntity::ElectricFurnace(Default::default()),
                            registry::block::CRUSHER => BlockEntity::Crusher(Default::default()),
                            registry::block::WATER_WHEEL => BlockEntity::WaterWheel(Default::default()),
                            registry::block::BATTERY => BlockEntity::Battery(Default::default()),
                            registry::block::PIPE => BlockEntity::Pipe(Default::default()),
                            registry::block::BOILER => BlockEntity::Boiler(Default::default()),
                            registry::block::STEAM_ENGINE => BlockEntity::SteamEngine(Default::default()),
                            registry::block::PUMP => BlockEntity::Pump(Default::default()),
                            registry::block::REFINERY => BlockEntity::Refinery(Default::default()),
                            registry::block::COMBUSTION_GENERATOR => BlockEntity::Combustion(Default::default()),
                            registry::block::REACTOR => BlockEntity::Reactor(Default::default()),
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
                // lore tomes (Step 20): page through real book content
                if let Some(book) = held.as_ref()
                    .and_then(|s| self.lore.for_item(&s.item_id).map(|b| b.title.clone()))
                {
                    self.open_lore_title = Some(book);
                    self.open_lore_page = 0;
                    self.ui_open = UiOpen::LoreBook;
                    self.unlock_cursor();
                    return;
                }
                // spell scrolls (P33): right-click to learn the spell.
                if let Some(stack) = &held {
                    if let Some(spell) = lf_game::magic::Spell::from_scroll(&stack.item_id) {
                        if self.spellbook.learn(spell) {
                            self.chronicle_event(
                                EventType::Discovery,
                                format!("a scroll unravels — {} learned", spell.name()),
                            );
                            self.consume_selected(1);
                        }
                        return;
                    }
                }
                // buckets: scoop a water/oil source or pour one. The raycast
                // skips fluids, so the target cell is the face-adjacent one.
                // An oil bucket aimed at a refinery feeds its tank instead.
                if let Some(stack) = &held {
                    if matches!(stack.item_id.as_str(), "bucket" | "water_bucket" | "oil_bucket") {
                        if let Some((pos, normal)) = target {
                            if stack.item_id == "oil_bucket"
                                && self.world.get_block(pos.x, pos.y, pos.z).id() == registry::block::REFINERY
                            {
                                let key = (pos.x, pos.y, pos.z);
                                let fed = match self.block_entities.get_mut(&key) {
                                    Some(BlockEntity::Refinery(r)) => r.pour_bucket(),
                                    _ => false,
                                };
                                if fed {
                                    self.inventory.slots[self.hotbar_index] =
                                        Some(ItemStack { item_id: "bucket".into(), count: 1 });
                                }
                                return;
                            }
                            let cell = pos + normal;
                            let state_there = self.world.get_block(cell.x, cell.y, cell.z);
                            let held_id = stack.item_id.clone();
                            let pour = match held_id.as_str() {
                                "water_bucket" => Some(lf_voxel::water_with_level(0)),
                                "oil_bucket" => Some(lf_voxel::oil_with_level(0)),
                                _ => None,
                            };
                            if pour.is_none()
                                && state_there.id() == registry::block::WATER
                                && lf_voxel::water_level(state_there) == 0
                            {
                                self.apply_sim_edit(cell.x, cell.y, cell.z, BlockState::AIR);
                                self.after_edit(cell.x, cell.y, cell.z);
                                self.inventory.slots[self.hotbar_index] =
                                    Some(ItemStack { item_id: "water_bucket".into(), count: 1 });
                            } else if pour.is_none()
                                && state_there.id() == registry::block::OIL
                                && lf_voxel::oil_level(state_there) == 0
                            {
                                self.apply_sim_edit(cell.x, cell.y, cell.z, BlockState::AIR);
                                self.after_edit(cell.x, cell.y, cell.z);
                                self.inventory.slots[self.hotbar_index] =
                                    Some(ItemStack { item_id: "oil_bucket".into(), count: 1 });
                            } else if let Some(fluid) = pour.filter(|_| state_there == BlockState::AIR) {
                                self.apply_sim_edit(cell.x, cell.y, cell.z, fluid);
                                self.after_edit(cell.x, cell.y, cell.z);
                                self.inventory.slots[self.hotbar_index] =
                                    Some(ItemStack { item_id: "bucket".into(), count: 1 });
                            }
                        }
                        return;
                    }
                }
                if let Some(stack) = held {
                        // P34 blueprints: two-corner capture, then paste. Sneak-click clears.
                    if stack.item_id == "blueprint" {
                        if let Some((pos, _)) = target {
                            if self.input.held(self.keymap.key(crate::input::Action::Sneak)) {
                                self.bp_corner_a = None;
                                self.push_hint("blueprint marker cleared");
                            } else if self.bp_clip.is_none() {
                                match self.bp_corner_a {
                                    None => {
                                        self.bp_corner_a = Some((pos.x, pos.y, pos.z));
                                        self.push_hint("corner A marked — click the far corner");
                                    }
                                    Some(a) => {
                                        let bp = lf_game::construction::capture(
                                            &self.world, a, (pos.x, pos.y, pos.z));
                                        let dir = self.world_dir.join("blueprints");
                                        let _ = std::fs::create_dir_all(&dir);
                                        let stamp = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .map(|d| d.as_secs()).unwrap_or(0);
                                        let path = dir.join(format!("bp{}.bp", stamp));
                                        if let Err(e) = bp.save(&path) {
                                            self.push_hint(&format!("blueprint save failed: {}", e));
                                        } else {
                                            self.push_hint(&format!(
                                                "captured {}x{}x{} ({} blocks) — next clicks paste it",
                                                bp.sx, bp.sy, bp.sz, bp.blocks.iter().filter(|b| b.id() != 0).count()));
                                            self.bp_clip = Some(bp);
                                            self.bp_path = Some(path);
                                        }
                                        self.bp_corner_a = None;
                                    }
                                }
                            } else if let Some(bp) = self.bp_clip.clone() {
                                // paste at the face-adjacent cell
                                let at = (pos.x + 1, pos.y, pos.z);
                                let targets = lf_game::construction::paste_targets(&self.world, &bp, at);
                                // materials: the bill for exactly these blocks
                                let mut need: Vec<(String, u16)> = Vec::new();
                                for (_, b) in &targets {
                                    if let Some(item) = lf_game::items::block_drop(b.id()) {
                                        match need.iter_mut().find(|(id, _)| *id == item) {
                                            Some((_, n)) => *n += 1,
                                            None => need.push((item, 1)),
                                        }
                                    }
                                }
                                let have = |id: &str| -> u16 {
                                    self.inventory.slots.iter().flatten()
                                        .filter(|s| s.item_id == id)
                                        .map(|s| s.count as u16).sum()
                                };
                                let short = need.iter().find(|(id, n)| have(id) < *n);
                                if let Some((id, n)) = short {
                                    self.push_hint(&format!("paste needs {} x{}", n, id));
                                } else {
                                    for (id, n) in &need {
                                        let mut left = *n;
                                        for slot in self.inventory.slots.iter_mut() {
                                            if left == 0 { break; }
                                            if let Some(s) = slot {
                                                if s.item_id == *id {
                                                    let take = (s.count as u16).min(left);
                                                    s.count -= take as u8;
                                                    left -= take;
                                                    if s.count == 0 { *slot = None; }
                                                }
                                            }
                                        }
                                    }
                                    let mut remesh = false;
                                    for ((x, y, z), b) in targets {
                                        if self.world.set_block(x, y, z, b).is_some() {
                                            if let Some(n) = &self.net {
                                                n.send_block(x, y, z, b.id());
                                            }
                                            remesh = true;
                                        }
                                    }
                                    if remesh {
                                        self.remesh_around(at.0, at.2);
                                        self.push_hint("blueprint pasted");
                                    }
                                }
                            }
                        }
                        return;
                    }
                    // P34 chisel: right-click stone with a chisel to carve.
                    if stack.item_id == "chisel" {
                        if let Some((pos, _)) = target {
                            if self.world.get_block(pos.x, pos.y, pos.z).id() == registry::block::STONE {
                                self.carve_target = Some((pos.x, pos.y, pos.z));
                                self.ui_open = UiOpen::Carve;
                                self.menu_reveal = 0.0;
                                self.unlock_cursor();
                            } else {
                                self.push_hint("the chisel works stone, nothing else");
                            }
                        }
                        return;
                    }
                    // P34 shaped blocks: slabs + stairs place with shape
                    // (a slab aimed at a matching bottom slab merges).
                    if let Some(shaped) = lf_game::items::shaped_placement(&stack.item_id, self.player.yaw) {
                        if let Some((pos, normal)) = target {
                            let existing = self.world.get_block(pos.x, pos.y, pos.z);
                            let merge = lf_game::items::slab_merge(existing, shaped);
                            let place = if merge.is_some() { pos } else { pos + normal };
                            if merge.is_some() || !self.block_intersects_player(place) {
                                let final_state = merge.unwrap_or(shaped);
                                if self.world.set_block(place.x, place.y, place.z, final_state).is_some() {
                                    if let Some(n) = &self.net {
                                        n.send_block(place.x, place.y, place.z, final_state.id());
                                    }
                                    self.remesh_around(place.x, place.z);
                                    self.after_edit(place.x, place.y, place.z);
                                    self.play_block_sound(final_state.id(), lf_audio::Action::Place);
                                    self.consume_selected(1);
                                    // symmetry mirrors the placement
                                    if let Some(px) = self.symmetry_plane {
                                        let mx = (2.0 * px - place.x as f32).round() as i32;
                                        if self.world.get_block(mx, place.y, place.z) == BlockState::AIR {
                                            self.world.set_block(mx, place.y, place.z, final_state);
                                            self.remesh_around(mx, place.z);
                                        }
                                    }
                                }
                            }
                        }
                        return;
                    }
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
                                        } else if b == registry::block::WATER_WHEEL {
                                            self.block_entities.insert(key, BlockEntity::WaterWheel(Default::default()));
                                        } else if b == registry::block::BATTERY {
                                            self.block_entities.insert(key, BlockEntity::Battery(Default::default()));
                                        } else if b == registry::block::PIPE {
                                            self.block_entities.insert(key, BlockEntity::Pipe(Default::default()));
                                        } else if b == registry::block::BOILER {
                                            self.block_entities.insert(key, BlockEntity::Boiler(Default::default()));
                                        } else if b == registry::block::STEAM_ENGINE {
                                            self.block_entities.insert(key, BlockEntity::SteamEngine(Default::default()));
                                        } else if b == registry::block::PUMP {
                                            self.block_entities.insert(key, BlockEntity::Pump(Default::default()));
                                        } else if b == registry::block::REFINERY {
                                            self.block_entities.insert(key, BlockEntity::Refinery(Default::default()));
                                        } else if b == registry::block::COMBUSTION_GENERATOR {
                                            self.block_entities.insert(key, BlockEntity::Combustion(Default::default()));
                                        } else if b == registry::block::REACTOR {
                                            self.block_entities.insert(key, BlockEntity::Reactor(Default::default()));
                                        } else if b == registry::block::CONDUIT {
                                            self.block_entities.insert(key, BlockEntity::Conduit);
                                        } else if b == registry::block::COMPUTER {
                                            self.block_entities.insert(key, BlockEntity::Screen { page: 1 });
                                        }
                                        self.remesh_around(place.x, place.z);
                                        // a placed block can dam water or
                                        // catch a floating granular column
                                        self.after_edit(place.x, place.y, place.z);
                                        self.play_block_sound(b, lf_audio::Action::Place);
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
        // Biome color grade: lerp toward the biome the player stands in
        // (~0.3s time constant — a hard cut would read as a bug).
        {
            let (tint_t, sat_t) = biome_grade(self.map.biome_at(
                self.player.position.x as i32,
                self.player.position.z as i32,
            ));
            let k = 1.0 - (-dt * 3.0).exp();
            for i in 0..3 {
                self.grade_tint[i] += (tint_t[i] - self.grade_tint[i]) * k;
            }
            self.grade_sat += (sat_t - self.grade_sat) * k;
        }
        // impact-pulse decay (Step 3)
        self.shake = shake_decay(self.shake, dt);
        // Sky bodies every frame (they rotate); clouds drift (rebuild 2/s).
        let eye = self.player.eye_position();
        let (sv, si) = lf_engine::atmosphere::sky_bodies(eye, self.time.fraction());
        self.sky_batch = Some(MeshBatch::new(&self.device, &self.resources, &sv, &si));
        if !self.settings.clouds {
            self.cloud_batch = None; // the toggle was previously unwired
        } else if self.last_cloud_rebuild.elapsed() >= Duration::from_millis(500) {
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
        // waypoint beacons (Step 15): slim translucent beams rising from
        // the ground under each waypoint, tinted by its color
        {
            let (mut bv, mut bi) = (Vec::new(), Vec::new());
            for wp in &self.waypoints {
                let ground = self.world.surface_height(wp.x as i32, wp.z as i32).max(wp.y as i32 - 2) as f32;
                let layer = lf_assets::WAYPOINT_LAYERS[wp.color_idx % lf_assets::WAYPOINT_LAYERS.len()];
                push_beam_quads(&mut bv, &mut bi,
                    wp.x as f32 + 0.5, ground, wp.z as f32 + 0.5, 24.0, layer);
            }
            self.waypoint_batch = if bv.is_empty() {
                None
            } else {
                Some(MeshBatch::new(&self.device, &self.resources, &bv, &bi))
            };
        }
    }

    /// Is the player's biome cold enough for snow? (Step 19: the actual
    /// biome field via the map's generator, not the old block proxy.)
    fn gen_biome_temp_at_player(&self) -> bool {
        self.map.biome_at(
            self.player.position.x as i32,
            self.player.position.z as i32,
        ).is_cold()
    }

    /// P33: cast the spell in a slot (Z/X/C). The pure gating lives in
    /// lf_game::magic; this applies the effect to the world/player.
    /// P35: compose the computer screen's 16x16 face from live data and
    /// upload it to the dynamic atlas layer when (and only when) the data
    /// changed. Pages: 1 = research, 2 = chronicle, 3 = power grid.
    fn update_screen_texture(&mut self) {
        let (page, owned_screen) = self.block_entities.values().find_map(|e| match e {
            BlockEntity::Screen { page } => Some((*page, true)),
            _ => None,
        }).unwrap_or((1, false));
        let powered = self.machine_power.values().filter(|r| **r >= 0.9).count();
        let starved = self.machine_power.values().filter(|r| **r < 0.9).count();
        let era = self.research.era as u8;
        let branches = self.research.branches.len() as u8;
        let events = self.chronicle.len().min(255) as u8;
        if !owned_screen {
            return;
        }
        // signature: every datum the face shows
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (page, powered, starved, era, branches, events).hash(&mut h);
        let sig = h.finish();
        if sig == self.screen_signature {
            return;
        }
        self.screen_signature = sig;
        let img = compose_screen_face(page, era, branches, events, powered, starved);
        self.resources.write_atlas_layer(&self.queue, lf_assets::SCREEN_LAYER, &img);
    }

    /// P34: a builder's hint line in the chat log.
    fn push_hint(&mut self, msg: &str) {
        self.chat_log.push(msg.to_string());
        let len = self.chat_log.len();
        if len > 6 {
            self.chat_log.drain(..len - 6);
        }
    }

    pub fn cast_from_slot(&mut self, slot: usize) {
        use lf_game::magic::{SpellEffect};
        let Ok((effect, mana_left)) = self.spellbook.try_cast(slot, self.stats.mana) else {
            return; // not learned / no mana: the HUD shows the pool, stay quiet
        };
        self.stats.mana = mana_left;
        match effect {
            SpellEffect::Firebolt => {
                let eye = self.player.eye_position();
                let look = self.player.look_dir();
                self.firebolts.push(lf_game::combat::Arrow {
                    position: eye + look * 0.5,
                    velocity: look * 22.0,
                    age: 0.0,
                });
            }
            SpellEffect::Blink { forward } => {
                // step along the gaze until a wall, land just before it
                let eye = self.player.eye_position();
                let look = self.player.look_dir();
                let mut dest = eye + look * forward;
                let steps = (forward / 0.5) as usize;
                for i in 1..=steps {
                    let p = eye + look * (i as f32 * 0.5);
                    if self.world.is_solid(p.x as i32, p.y as i32, p.z as i32) {
                        let back = eye + look * ((i as f32 - 1.0) * 0.5);
                        dest = back;
                        break;
                    }
                }
                self.player.position = dest - Vec3::new(0.0, 1.7, 0.0); // eye height back to feet
                // blink wisp particles at both ends
                if self.settings.particles {
                    let tex = lf_assets::LUMEN_LAYER;
                    for (pos, spread) in [(dest, 0.2), (eye, 0.3)] {
                        self.particles.push(Particle {
                            position: pos,
                            velocity: Vec3::new(spread, 0.6, spread),
                            life: 0.5,
                            tex,
                            uv_off: [0.0, 0.0],
                        });
                    }
                }
            }
            SpellEffect::Ward { secs } => {
                self.ward_timer = self.ward_timer.max(secs);
            }
            SpellEffect::Hearthlight => {
                // soften one ore by hand (the Smith's trick)
                let ids: Vec<String> = self.inventory.slots.iter()
                    .filter_map(|s| s.as_ref().map(|s| s.item_id.clone()))
                    .collect();
                let refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
                if let Some((from, to)) = lf_game::magic::hearthlight_pick(&refs) {
                    // consume one, add one
                    'find: for slot_item in self.inventory.slots.iter_mut() {
                        if let Some(stack) = slot_item {
                            if stack.item_id == from {
                                stack.count -= 1;
                                if stack.count == 0 {
                                    *slot_item = None;
                                }
                                let leftover = self.inventory.add_item(&to, 1);
                                if leftover > 0 {
                                    self.spawn_drop(&to, 1, self.player.eye_position());
                                }
                                break 'find;
                            }
                        }
                    }
                }
                // and light the targeted cell (or under the player)
                let target = raycast_voxel(
                    self.player.eye_position(),
                    self.player.look_dir(),
                    REACH,
                    |pos| registry::is_targetable(self.world.get_block(pos.x, pos.y, pos.z)),
                )
                .map(|(pos, normal)| pos + normal)
                .unwrap_or_else(|| glam::IVec3::new(
                    self.player.position.x as i32,
                    self.player.position.y as i32 + 2,
                    self.player.position.z as i32,
                ));
                if self.world.get_block(target.x, target.y, target.z) == BlockState::AIR {
                    self.world.set_block(target.x, target.y, target.z, BlockState(registry::block::LUMEN_BLOCK));
                    self.hearth_lights.insert((target.x, target.y, target.z), 90.0);
                    self.remesh_around(target.x, target.z);
                }
            }
        }
    }

    /// P33: advancing firebolts — arrows that hit harder and spark.
    fn update_firebolts(&mut self, dt: f32) {
        let mut remove: Vec<usize> = Vec::new();
        let mut events: Vec<(usize, f32)> = Vec::new();
        for (i, bolt) in self.firebolts.iter_mut().enumerate() {
            let before = bolt.position;
            let done = bolt.update(dt, |x, y, z| self.world.is_solid(x, y, z));
            if done {
                remove.push(i);
                continue;
            }
            let dir = bolt.position - before;
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
                    events.push((mi, 8.0));
                    remove.push(i);
                    break;
                }
            }
        }
        for i in remove.into_iter().rev() {
            let p = self.firebolts[i].position;
            if self.settings.particles {
                let tex = lf_assets::texture_index_for_block(registry::block::LANTERN);
                for j in 0..6u32 {
                    let a = j as f32 * 1.05;
                    self.particles.push(Particle {
                        position: p,
                        velocity: Vec3::new(a.cos() * 2.0, 1.5, a.sin() * 2.0),
                        life: 0.5,
                        tex,
                        uv_off: [0.0, 0.0],
                    });
                }
            }
            self.firebolts.remove(i);
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
                    let (l, pr) = lf_game::combat::grant_xp(self.xp_level, self.xp_progress, 5);
                    self.xp_level = l;
                    self.xp_progress = pr;
                    self.xp_flash = 1.0;
                }
            }
        }
    }

    /// P33: temporary hearthlight blocks burn out.
    fn tick_hearth_lights(&mut self, dt: f32) {
        let expired: Vec<(i32, i32, i32)> = self.hearth_lights.iter_mut()
            .filter_map(|(pos, t)| {
                *t -= dt;
                if *t <= 0.0 { Some(*pos) } else { None }
            })
            .collect();
        for pos in expired {
            self.hearth_lights.remove(&pos);
            if self.world.get_block(pos.0, pos.1, pos.2).id() == registry::block::LUMEN_BLOCK {
                self.world.set_block(pos.0, pos.1, pos.2, BlockState::AIR);
                self.remesh_around(pos.0, pos.2);
            }
        }
    }

    /// P32: a reactor meltdown — the core and everything near it is
    /// destroyed, radiation residue is scattered through the crater, and
    /// the chronicle records it. The crater glows an unhealthy green.
    fn apply_meltdown(&mut self, pos: (i32, i32, i32)) {
        self.block_entities.remove(&pos);
        let mut edits: Vec<(i32, i32)> = Vec::new();
        let r = 3i32;
        let mut placed_residue = 0usize;
        for dx in -r..=r {
            for dy in -r..=r {
                for dz in -r..=r {
                    if dx * dx + dy * dy + dz * dz > r * r {
                        continue;
                    }
                    let (x, y, z) = (pos.0 + dx, pos.1 + dy, pos.2 + dz);
                    let here = self.world.get_block(x, y, z);
                    if here.id() == registry::block::AIR || !registry::is_solid(here) {
                        continue;
                    }
                    // residue crusts onto roughly a third of the crater
                    let residue = (x * 7 + y * 13 + z * 5).rem_euclid(3) == 0;
                    let block = if residue && placed_residue < 14 {
                        placed_residue += 1;
                        BlockState(registry::block::RADIATION)
                    } else {
                        BlockState::AIR
                    };
                    if self.world.set_block(x, y, z, block).is_some() {
                        edits.push((x, z));
                    }
                }
            }
        }
        for (x, z) in edits {
            self.remesh_around(x, z);
        }
        // blast debris
        if self.settings.particles {
            let tex = lf_assets::RADIATION_LAYER;
            for i in 0..24u32 {
                let a = i as f32 * 0.26;
                let speed = 4.0 + (i % 5) as f32;
                self.particles.push(Particle {
                    position: Vec3::new(pos.0 as f32 + 0.5, pos.1 as f32 + 1.0, pos.2 as f32 + 0.5),
                    velocity: Vec3::new(a.cos() * speed, 5.0 + (i % 4) as f32, a.sin() * speed),
                    life: 1.2,
                    tex,
                    uv_off: [0.0, 0.0],
                });
            }
        }
        self.shake = (self.shake + 0.35).min(0.5);
        self.chronicle_event(
            lf_chronicle::EventType::Meltdown,
            "a reactor cooks itself — the crater still glows".into(),
        );
    }

    /// Step 25 grid overlay: one translucent tint cube per powered machine
    /// (green = fed, red = starved), plus the P34 symmetry plane and the
    /// blueprint ghost — rebuilt every few frames while any is active.
    fn rebuild_overlay_batch(&mut self) {
        let (mut bv, mut bi) = (Vec::new(), Vec::new());
        for (pos, ratio) in &self.machine_power {
            let layer = if *ratio >= 0.9 {
                lf_assets::GRID_OK_LAYER
            } else {
                lf_assets::GRID_STARVED_LAYER
            };
            push_overlay_cube(&mut bv, &mut bi, pos.0 as f32, pos.1 as f32, pos.2 as f32, layer);
        }
        // P34 symmetry plane: a translucent wall at the mirror x
        if let Some(px) = self.symmetry_plane {
            let layer = lf_assets::GRID_OK_LAYER;
            let (x0, x1) = (px - 0.05, px + 0.05);
            let y0 = self.player.position.y - 3.0;
            let y1 = y0 + 14.0;
            let z0 = self.player.position.z - 12.0;
            let z1 = z0 + 24.0;
            for (cx, cz, n) in [
                (x0, z0, [-1.0f32, 0.0, 0.0]),
                (x1, z0, [1.0, 0.0, 0.0]),
            ] {
                let base = bv.len() as u32;
                for (corner, uv) in [
                    ([cx, y0, cz], [0.0, 1.0]), ([cx, y1, cz], [0.0, 0.0]),
                    ([cx, y1, z1], [1.0, 0.0]), ([cx, y0, z1], [1.0, 1.0]),
                ] {
                    bv.push(GpuVertex {
                        position: corner, normal: n, tex_coord: uv,
                        tex_index: layer, ao: 1.0, light: 0xF0, sway: 0.0,
                    });
                }
                bi.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
            }
        }
        // P34 blueprint ghost: the captured build hanging where it would
        // paste (capped — huge captures draw a cut of themselves)
        let holding_bp = self.inventory.slots[self.hotbar_index].as_ref()
            .map(|s| s.item_id == "blueprint").unwrap_or(false);
        if holding_bp {
            if let Some(bp) = &self.bp_clip {
                if let Some((pos, normal)) = raycast_voxel(
                    self.player.eye_position(), self.player.look_dir(), REACH,
                    |p| registry::is_targetable(self.world.get_block(p.x, p.y, p.z)),
                ) {
                    let at = (pos.x + normal.x, pos.y + normal.y, pos.z + normal.z);
                    let mut drawn = 0usize;
                    'outer: for y in 0..bp.sy as i32 {
                        for z in 0..bp.sz as i32 {
                            for x in 0..bp.sx as i32 {
                                let b = bp.get(x, y, z);
                                if b.id() == 0 {
                                    continue;
                                }
                                push_overlay_cube(&mut bv, &mut bi,
                                    (at.0 + x) as f32, (at.1 + y) as f32, (at.2 + z) as f32,
                                    lf_assets::GRID_OK_LAYER);
                                drawn += 1;
                                if drawn >= 600 {
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
            }
        }
        self.overlay_batch = if bv.is_empty() {
            None
        } else {
            Some(MeshBatch::new(&self.device, &self.resources, &bv, &bi))
        };
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
        // P33: mana regenerates passively (clamped at the pool).
        self.stats.mana = (self.stats.mana + lf_game::magic::MANA_REGEN * dt)
            .min(self.stats.max_mana);
        // P35: a powered climate unit nearby regenerates a little health
        // (checked on a cadence — the scan is a 9x6x9 box).
        if self.frame % 30 == 0 {
            let p = (self.player.position.x as i32, self.player.position.y as i32, self.player.position.z as i32);
            if lf_game::building::climate_comfort(&self.world, p, &self.producer_positions)
                && self.stats.hunger > 6.0
            {
                self.stats.health = (self.stats.health + 0.5).min(self.stats.max_health);
            }
        }
        // P35: rewrite the dynamic screen layer only when its data changed
        self.update_screen_texture();
        // P32: radiation residue poisons anyone standing near it until the
        // crater is scrubbed clean (the blocks are breakable).
        let p = self.player.position;
        let mut irradiated = false;
        for dx in -3..=3i32 {
            for dy in -2..=3i32 {
                for dz in -3..=3i32 {
                    if self.world.get_block(p.x as i32 + dx, p.y as i32 + dy, p.z as i32 + dz).id()
                        == registry::block::RADIATION
                    {
                        irradiated = true;
                    }
                }
            }
        }
        if irradiated {
            self.damage(2.0 * dt);
        }
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
        if self.ward_timer > 0.0 {
            // the ward drinks it (P33); the timer still runs down in tick
            self.hud_flash = 1.0;
            return;
        }
        let mut armor = worn_armor_points(&self.inventory.slots);
        // P33: a held Rune of Warding tool adds flat armor
        if let Some(h) = self.inventory.slots[self.hotbar_index].as_ref() {
            if let Some(rune_id) = self.runed_tools.get(&h.item_id) {
                if let Some(rune) = lf_game::magic::Rune::from_item(rune_id) {
                    armor = (armor as f32 + rune.armor_bonus()) as u8;
                }
            }
        }
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
            // hamlet marker: crafting table on the surface band;
            // wizard tower marker: an enchanting table (P33)
            let mut has_hut = false;
            let mut tower_spot = None;
            let mut hut_spot = None;
            for lx in 4..12usize {
                for lz in 4..12usize {
                    for y in 60..200usize {
                        let id_here = col.get(lx, y, lz).id();
                        if id_here == registry::block::CRAFTING_TABLE {
                            has_hut = true;
                            hut_spot = Some((cx * 16 + lx as i32, y as i32 + 1, cz * 16 + lz as i32));
                        } else if id_here == registry::block::ENCHANTING_TABLE {
                            tower_spot = Some((cx * 16 + lx as i32, y as i32 + 1, cz * 16 + lz as i32));
                        }
                    }
                }
            }
            // a wizard settles the tower (max two per world, one per tower)
            if let Some((tx, ty, tz)) = tower_spot {
                let wizards = self.villagers.iter()
                    .filter(|v| v.job == VillagerJob::Wizard).count();
                let staffed = self.villagers.iter().any(|v| {
                    v.job == VillagerJob::Wizard
                        && (v.position[0] - tx as f32).abs() < 12.0
                        && (v.position[2] - tz as f32).abs() < 12.0
                });
                if wizards < 2 && !staffed {
                    let id = 1000 + self.villagers.len() as u64;
                    let spawn = if self.world.is_solid(tx, ty - 1, tz + 2) { (tx, ty, tz + 2) } else { (tx, ty, tz) };
                    self.villagers.push(Villager::new(id, VillagerJob::Wizard, "Ysolde".into(),
                        [spawn.0 as f32 + 0.5, spawn.1 as f32 + 0.2, spawn.2 as f32 + 0.5]));
                    tracing::info!("Ysolde the Wizard settled a tower");
                    return;
                }
                continue;
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
                VillagerJob::Wizard => "Ysolde",
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
            // spawn point's biome decides the fauna (Step 18)
            let spawn_cold = {
                let ang2 = (seed % 360) as f32 / 57.3;
                let dist2 = 20.0 + ((seed >> 9) % 20) as f32;
                let bx = (player.x + ang2.cos() * dist2) as i32;
                let bz = (player.z + ang2.sin() * dist2) as i32;
                self.map.biome_at(bx, bz).is_cold()
            };
            if let Some(kind) = roll_spawn(seed, is_day, spawn_cold) {
                // random point 20-40 blocks out
                let ang = (seed % 360) as f32 / 57.3;
                let dist = 20.0 + ((seed >> 9) % 20) as f32;
                let sx = (player.x + ang.cos() * dist) as i32;
                let sz = (player.z + ang.sin() * dist) as i32;
                // only spawn on loaded ground
                if let Some((cx, lx)) = Some((sx.div_euclid(16), sx.rem_euclid(16))) {
                    // P33: a warding pylon keeps hostiles out of its reach
                    let warded = kind.is_hostile() && {
                        let top0 = self.world.surface_height(sx, sz);
                        (-3..=3i32).any(|dx| (-3..=3i32).any(|dz| {
                            (-1..=2i32).any(|dy| {
                                self.world.get_block(sx + dx, top0 + dy, sz + dz).id()
                                    == registry::block::WARDING_PYLON
                            })
                        }))
                    };
                    if !warded {
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
    }

    /// Procedural one-shot for block edits (Step 4): category from the
    /// block family, volume from the persisted sliders. Silent when no
    /// output device exists.
    fn play_block_sound(&mut self, block_id: u32, action: lf_audio::Action) {
        let volume = self.settings.volume_master * self.settings.volume_sfx;
        if let Some(a) = &mut self.audio {
            a.play(lf_audio::block_category(block_id), action, volume);
        }
    }

    /// Impact pulse for heavy breaks (Step 3): a short camera shake scaled
    /// by block hardness and tool weight.
    fn break_impulse(&mut self, block_id: u32) {
        let amp = break_shake(lf_game::mining::hardness(block_id), held_tool_tier(self));
        self.shake = (self.shake + amp).min(0.6);
    }

    /// Water/gravity aftermath of any block edit (player or simulation):
    /// wake the water around the cell and drop any granular block that
    /// just lost its support.
    fn after_edit(&mut self, x: i32, y: i32, z: i32) {
        self.enqueue_fluid_around(x, y, z);
        self.spawn_faller_from_above(x, y, z);
    }

    fn enqueue_fluid_around(&mut self, x: i32, y: i32, z: i32) {
        for p in [
            (x, y, z),
            (x + 1, y, z),
            (x - 1, y, z),
            (x, y + 1, z),
            (x, y - 1, z),
            (x, y, z + 1),
            (x, y, z - 1),
        ] {
            if self.fluid_queued.insert(p) {
                self.fluid_queue.push_back(p);
            }
        }
    }

    /// If the block above `(x, y, z)` is granular and the cell is now
    /// non-supporting, detach it into an animated faller. Cascades up the
    /// column (each detachment re-checks the block above it).
    fn spawn_faller_from_above(&mut self, x: i32, y: i32, z: i32) {
        let above = self.world.get_block(x, y + 1, z);
        if !registry::has_gravity(above.id()) || self.world.is_solid(x, y, z) {
            return;
        }
        if self.world.set_block(x, y + 1, z, BlockState::AIR).is_some() {
            self.remesh_around(x, z);
            if let Some(n) = &self.net {
                n.send_block(x, y + 1, z, registry::block::AIR);
            }
            self.falling_blocks.push(FallingBlock {
                position: Vec3::new(x as f32 + 0.5, y as f32 + 1.5, z as f32 + 0.5),
                velocity: 0.0,
                block: above,
            });
            self.spawn_faller_from_above(x, y + 1, z);
        }
    }

    /// Advance animated falling blocks; landing re-places the block
    /// (displacing water, crushing nothing v1) through the same edit path
    /// as a player placement so remesh + network stay consistent.
    fn update_falling_blocks(&mut self, dt: f32) {
        let mut landed: Vec<(i32, i32, i32, BlockState)> = Vec::new();
        let mut dropped_items: Vec<BlockState> = Vec::new();
        self.falling_blocks.retain_mut(|f| {
            let cell = (f.position.x.floor() as i32, f.position.y.floor() as i32, f.position.z.floor() as i32);
            let in_water = self.world.get_block(cell.0, cell.1, cell.2).id() == registry::block::WATER;
            f.velocity += 24.0 * dt;
            if in_water {
                f.velocity = f.velocity.min(2.5); // sinks, does not rocket through the pool
            }
            let new_y = f.position.y - f.velocity * dt;
            let feet_cell = (new_y - 0.5).floor() as i32;
            if feet_cell < 0 {
                return false; // fell out of the world
            }
            if self.world.is_solid(cell.0, feet_cell, cell.2) {
                let land_y = feet_cell + 1;
                let occupied = self.world.get_block(cell.0, land_y, cell.2);
                if registry::is_solid(occupied) {
                    dropped_items.push(f.block);
                } else {
                    landed.push((cell.0, land_y, cell.2, f.block));
                }
                return false;
            }
            f.position.y = new_y;
            true
        });
        for (x, y, z, b) in landed {
            self.apply_sim_edit(x, y, z, b);
            self.after_edit(x, y, z);
        }
        for b in dropped_items {
            if let Some(item) = lf_game::items::block_drop(b.id()) {
                self.spawn_drop(&item, 1, self.player.eye_position() + self.player.look_dir());
            }
        }
    }

    /// Apply a simulation-driven block change with the same remesh +
    /// network-broadcast treatment as a player edit.
    fn apply_sim_edit(&mut self, x: i32, y: i32, z: i32, state: BlockState) {
        if self.world.set_block(x, y, z, state).is_some() {
            self.remesh_around(x, z);
            if let Some(n) = &self.net {
                n.send_block(x, y, z, state.id());
            }
        }
    }

    /// Run a bounded slice of the water simulation. Cell edits are applied
    /// through [`Self::apply_sim_edit`] and re-enqueue their neighborhood,
    /// so floods/recessions propagate across ticks.
    fn tick_fluids(&mut self) {
        const BUDGET: usize = 64;
        let mut edits: Vec<((i32, i32, i32), BlockState)> = Vec::new();
        for _ in 0..BUDGET {
            let Some((x, y, z)) = self.fluid_queue.pop_front() else { break };
            self.fluid_queued.remove(&(x, y, z));
            edits.extend(lf_game::fluids::step_cell(&mut self.world, x, y, z));
        }
        for ((x, y, z), state) in edits {
            self.apply_sim_edit(x, y, z, state);
            self.enqueue_fluid_around(x, y, z);
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
        self.streamer.set_center(center, self.settings.view_distance);

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
        let unload_radius = self.settings.view_distance + UNLOAD_MARGIN;
        let far: Vec<(i32, i32)> = self
            .world
            .chunks
            .keys()
            .copied()
            .filter(|p| chebyshev(*p, center) > unload_radius)
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
        let env_time = if self.settings.particles {
            self.elapsed
        } else {
            0.0 // freeze wind when particles/FX are off (low quality tier)
        };
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
            time: env_time,
            grade_tint: self.grade_tint,
            grade_saturation: self.grade_sat,
        }
    }

    fn camera(&self) -> Camera {
        if self.ui_open == UiOpen::Title {
            let r = 34.0;
            let cx = self.spawn_point.x + self.title_orbit.cos() * r;
            let cz = self.spawn_point.z + self.title_orbit.sin() * r;
            // The old fixed spawn+14 buried the camera inside ring terrain
            // on hilly worlds (audit Step 1: World_5 had 12/64 orbit points
            // under higher ground — a flat-dark title backdrop). Unloaded
            // columns report surface 0, which keeps the classic offset.
            let ground_at_eye = self.world.surface_height(cx as i32, cz as i32);
            let cy = title_eye_y(self.spawn_point.y, ground_at_eye);
            let mut camera = Camera::new(glam::Vec3::new(cx, cy, cz), self.spawn_point + glam::Vec3::new(0.0, 2.0, 0.0));
            camera.set_aspect(self.config.width, self.config.height);
            camera.fovy = self.settings.fov_degrees.to_radians();
            return camera;
        }
        let mut camera = Camera::new(self.player.eye_position(), self.player.eye_position() + self.player.look_dir());
        camera.set_aspect(self.config.width, self.config.height);
        camera.fovy = self.settings.fov_degrees.to_radians();
        // impact pulse: jitter the look target only, never the player state
        let (sr, su) = shake_offset(self.shake, self.frame as u64);
        if sr != 0.0 || su != 0.0 {
            let look = self.player.look_dir();
            let right = Vec3::new(-look.z, 0.0, look.x).normalize();
            camera.target += right * sr + Vec3::new(0.0, su, 0.0);
        }
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

    /// Step 14: a small thumbnail of the live view for the save-slot
    /// picker (worlds/<slot>/thumb.png). Throttled to ~2 min so the 30s
    /// autosave does not pay GPU capture cost every time.
    fn capture_slot_thumbnail(&mut self) {
        if self.last_thumb.elapsed() < Duration::from_secs(120) {
            return;
        }
        self.last_thumb = Instant::now();
        let mut vertices: Vec<GpuVertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for (v, i) in self.cpu_meshes.values() {
            let base = vertices.len() as u32;
            vertices.extend_from_slice(v);
            indices.extend(i.iter().map(|idx| idx + base));
        }
        let camera = self.camera();
        let env = self.env();
        let textures = lf_assets::generate_atlas();
        let path = self.world_dir.join("thumb.png");
        match lf_engine::headless::render_to_png(&vertices, &indices, &[], &[], &textures, &camera, &env, self.clear_color(), 256, 144, &path, None) {
            Ok(()) => {}
            Err(e) => tracing::debug!("thumbnail capture failed: {}", e),
        }
    }

    fn clear_color(&self) -> [f64; 4] {
        let env = self.env();
        let mut c = env.fog_color;
        // mirror the shader's post-fog grade so the open sky carries the
        // same color cast as the graded geometry (no horizon seam)
        let luma = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
        for i in 0..3 {
            c[i] = (luma + (c[i] - luma) * self.grade_sat) * self.grade_tint[i];
        }
        [c[0] as f64, c[1] as f64, c[2] as f64, 1.0]
    }

    fn render(&mut self) {
        self.rebuild_drop_batch();
        self.rebuild_crack_batch();
        self.rebuild_particle_batch();
        let camera = self.camera();
        let env = self.env();
        let view_proj = camera.build_view_projection_matrix();
        for batch in self.batches.values() {
            batch.update_camera(&self.queue, &camera, &env);
        }
        for batch in self.water_batches.values() {
            batch.update_camera(&self.queue, &camera, &env);
        }
        if let Some(batch) = &self.waypoint_batch {
            batch.update_camera(&self.queue, &camera, &env);
        }
        // Step 25 overlay cubes (grid tint, symmetry plane, blueprint
        // ghost) — rebuilt periodically while any is active
        let overlay_active = self.grid_overlay
            || self.symmetry_plane.is_some()
            || (self.bp_clip.is_some()
                && self.inventory.slots[self.hotbar_index].as_ref()
                    .map(|s| s.item_id == "blueprint").unwrap_or(false));
        if overlay_active && (self.frame % 15 == 0 || self.overlay_batch.is_none()) {
            self.rebuild_overlay_batch();
        }
        if !overlay_active {
            self.overlay_batch = None;
        }
        if let Some(batch) = &self.overlay_batch {
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
            // Cull/sort against the RENDER camera (the title orbit camera
            // differs from the player eye — the audit caught them mixed).
            let eye = camera.eye;
            for (pos, batch) in self.batches.iter() {
                if let Some(&(min_y, max_y)) = self.column_bounds.get(pos) {
                    if !column_in_view(&view_proj, eye, *pos, min_y, max_y, self.settings.view_distance) {
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
            // waypoint beacons ride the transparent pass
            if let Some(batch) = &self.waypoint_batch {
                batch.draw(&mut pass, resources, true);
            }
            // power-grid overlay cubes (Step 25) ride the transparent pass
            if let Some(batch) = &self.overlay_batch {
                batch.draw(&mut pass, resources, true);
            }
            // Item drops ride the opaque pass.
            if let Some(batch) = &self.drop_batch {
                batch.draw(&mut pass, resources, false);
            }
            // Crack decal on the block being mined (cutout, depth-tested).
            if let Some(batch) = &self.crack_batch {
                batch.draw(&mut pass, resources, false);
            }
            // Break debris billboards.
            if let Some(batch) = &self.particle_batch {
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
        if self.drops.is_empty() && self.mobs.is_empty() && self.falling_blocks.is_empty() {
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
                        sway: 0.0,
                    });
                }
                indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
            }
        };
        // falling granular blocks render near-full-size with their own texture
        for fb in &self.falling_blocks {
            let tex = lf_assets::texture_index_for_block(fb.block.id());
            push_cube(fb.position.x, fb.position.y, fb.position.z, 0.48, tex, &mut vertices, &mut indices);
        }
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
        // firebolts glow (P33)
        for bolt in &self.firebolts {
            push_cube(bolt.position.x, bolt.position.y, bolt.position.z, 0.12,
                lf_assets::texture_index_for_block(registry::block::LANTERN), &mut vertices, &mut indices);
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
                VillagerJob::Wizard => lf_assets::ENCHANTING_LAYER,
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
impl GameState {
    /// Crack decal on the block being mined: an inflated cube textured with
    /// the stage-N crack layer; the shader's alpha cutout keeps it hollow.
    fn rebuild_crack_batch(&mut self) {
        let want = self.mining.as_ref().map(|m| {
            let stage = ((m.progress / m.total).clamp(0.0, 1.0) * lf_assets::CRACK_LAYERS.len() as f32) as u32;
            (m.pos, stage.min(lf_assets::CRACK_LAYERS.len() as u32 - 1))
        });
        if want == self.crack_state {
            return;
        }
        self.crack_state = want;
        self.crack_batch = None;
        let Some(((x, y, z), stage)) = want else { return };
        let tex = lf_assets::CRACK_LAYERS[stage as usize];
        let (cx, cy, cz) = (x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
        let r = 0.505; // just clear of the block surface to avoid z-fighting
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for (normal, corners, uvs) in cube_faces(r) {
            let base = vertices.len() as u32;
            for (c, uv) in corners.iter().zip(uvs.iter()) {
                vertices.push(GpuVertex {
                    position: [cx + c[0], cy + c[1], cz + c[2]],
                    normal,
                    tex_coord: *uv,
                    tex_index: tex,
                    ao: 1.0,
                    light: 0xF0,
                    sway: 0.0,
                });
            }
            indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
        self.crack_batch = Some(MeshBatch::new(&self.device, &self.resources, &vertices, &indices));
    }

    /// Camera-facing debris quads for break/mining particles.
    fn rebuild_particle_batch(&mut self) {
        if self.particles.is_empty() {
            self.particle_batch = None;
            return;
        }
        let look = self.player.look_dir();
        let right = look.cross(Vec3::Y).normalize();
        let up = right.cross(look).normalize();
        let r = 0.07;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for pt in &self.particles {
            let base = vertices.len() as u32;
            let corners = [
                (-right * r - up * r, [pt.uv_off[0], pt.uv_off[1] + 0.25]),
                (-right * r + up * r, [pt.uv_off[0], pt.uv_off[1]]),
                (right * r + up * r, [pt.uv_off[0] + 0.25, pt.uv_off[1]]),
                (right * r - up * r, [pt.uv_off[0] + 0.25, pt.uv_off[1] + 0.25]),
            ];
            for (offset, uv) in corners {
                vertices.push(GpuVertex {
                    position: [pt.position.x + offset.x, pt.position.y + offset.y, pt.position.z + offset.z],
                    normal: [look.x, look.y, look.z],
                    tex_coord: uv,
                    tex_index: pt.tex,
                    ao: 1.0,
                    light: 0xF0,
                    sway: 0.0,
                });
            }
            indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
        self.particle_batch = Some(MeshBatch::new(&self.device, &self.resources, &vertices, &indices));
    }

    /// Spawn debris flying off a block that is being ground down or broken.
    fn spawn_break_particles(&mut self, block_id: u32, pos: (i32, i32, i32), count: usize) {
        let tex = lf_assets::texture_index_for_block(block_id);
        for i in 0..count {
            // cheap deterministic-ish jitter from the elapsed clock
            let seed = (self.elapsed.to_bits() as u32).wrapping_add(i as u32 * 2654435761);
            let r1 = (seed % 1000) as f32 / 1000.0;
            let r2 = ((seed >> 10) % 1000) as f32 / 1000.0;
            let r3 = ((seed >> 20) % 1000) as f32 / 1000.0;
            self.particles.push(Particle {
                position: Vec3::new(
                    pos.0 as f32 + 0.2 + r1 * 0.6,
                    pos.1 as f32 + 0.2 + r2 * 0.6,
                    pos.2 as f32 + 0.2 + r3 * 0.6,
                ),
                velocity: Vec3::new((r1 - 0.5) * 3.0, 2.0 + r2 * 2.0, (r3 - 0.5) * 3.0),
                life: 0.4 + r2 * 0.5,
                tex,
                uv_off: [r1 * 0.75, r3 * 0.75],
            });
        }
        // keep the debris budget bounded
        let max = 128;
        if self.particles.len() > max {
            let excess = self.particles.len() - max;
            self.particles.drain(0..excess);
        }
    }
}

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
    let mesh = world.mesh_column(cx, cz, &|b, face| lf_assets::texture_index_for_face(b.id(), face));
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
    -> (Inventory, PlayerStats, lf_game::TimeOfDay, HashMap<(i32, i32, i32), BlockEntity>, Vec<MobEntity>, Vec<Villager>, u32, QuestLog, Vec<ChronicleEvent>, ResearchState, Settings, lf_worldgen::WorldType, Vec<map::Waypoint>, lf_game::magic::Spellbook, std::collections::HashMap<String, String>) {
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
    // P33: extras are JSON now; the old bincode file migrates through the
    // frozen LegacyClientSave shape (never a silent extras reset).
    let loaded: Option<ClientSave> = (|| {
        if let Ok(bytes) = std::fs::read(dir.join("player_extras.json")) {
            return serde_json::from_slice::<ClientSave>(&bytes).ok();
        }
        if let Ok(bytes) = std::fs::read(dir.join("player_extras.dat")) {
            return bincode::deserialize::<LegacyClientSave>(&bytes).ok().map(ClientSave::from);
        }
        None
    })();
    if let Some(save) = loaded {
        {
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
            stats.mana = save.mana.clamp(0.0, stats.max_mana);
            let spellbook = save.spellbook.unwrap_or_default();
            let runed = save.runed.into_iter().collect();
            return (inventory, stats, lf_game::TimeOfDay::new(save.time_ticks), entities, mobs, villagers, kills, quest_log, chronicle, research, settings, world_type, waypoints, spellbook, runed);
        }
    }
    (inventory, stats, time, entities, mobs, villagers, kills, quest_log, chronicle, research, settings, lf_worldgen::WorldType::Normal, Vec::new(), lf_game::magic::Spellbook::default(), std::collections::HashMap::new())
}

/// Tiny deterministic-enough hash for cosmetic randomness.
/// Four vertical quads forming a slim translucent beam column (waypoint
/// beacons, Step 15). One texture tile tall so the in-texture falloff fades
/// the beam toward its top.
/// P35: the computer screen's 16x16 face — a styled readout, not text:
/// page 1 shows era pips (mainline + branches), page 2 chronicle rows,
/// page 3 a green/red power bar split. Pure + unit-tested.
pub fn compose_screen_face(
    page: u8,
    era: u8,
    branches: u8,
    events: u8,
    powered: usize,
    starved: usize,
) -> image::RgbaImage {
    let mut img = image::RgbaImage::new(16, 16);
    let px = |img: &mut image::RgbaImage, x: u32, y: u32, r: u8, g: u8, b: u8| {
        if x < 16 && y < 16 {
            img.put_pixel(x, y, image::Rgba([r, g, b, 255]));
        }
    };
    for y in 0..16 {
        for x in 0..16 {
            let bg = if (y % 4) == 0 { 20 } else { 12 };
            px(&mut img, x, y, bg, bg + 6, bg + 14);
        }
    }
    match page {
        1 => {
            // mainline era pips (bottom-up) + branch ticks on the right
            for i in 0..(era.min(4) as u32) {
                for x in 2..6 {
                    px(&mut img, x, 13 - i * 3, 120, 220, 255);
                }
            }
            for i in 0..(branches.min(3) as u32) {
                for y in 3..6 {
                    px(&mut img, 9 + i * 2, y, 185, 130, 255);
                }
            }
        }
        2 => {
            // chronicle rows: one bar per 12 events (max 9 rows)
            let rows = ((events as u32) / 12).min(9);
            for i in 0..rows {
                for x in 2..14 {
                    px(&mut img, x, 2 + i, 240, 200, 120);
                }
            }
        }
        _ => {
            // power grid: powered green left, starved red right
            let total = (powered + starved).max(1) as f32;
            let green_w = ((powered as f32 / total) * 12.0).round() as u32;
            for x in 0..12 {
                let color = if x < green_w { (110, 240, 130) } else { (250, 90, 80) };
                for y in 6..10 {
                    px(&mut img, x + 2, y, color.0, color.1, color.2);
                }
            }
        }
    }
    img
}

fn push_beam_quads(vertices: &mut Vec<GpuVertex>, indices: &mut Vec<u32>,
                   cx: f32, ground_y: f32, cz: f32, height: f32, tex: u32) {
    let r = 0.35f32;
    let y0 = ground_y;
    let y1 = ground_y + height;
    let faces: [([f32; 3], [[f32; 3]; 4]); 4] = [
        ([0.0, 0.0, -1.0], [[cx - r, y0, cz - r], [cx - r, y1, cz - r], [cx + r, y1, cz - r], [cx + r, y0, cz - r]]),
        ([0.0, 0.0, 1.0], [[cx + r, y0, cz + r], [cx + r, y1, cz + r], [cx - r, y1, cz + r], [cx - r, y0, cz + r]]),
        ([-1.0, 0.0, 0.0], [[cx - r, y0, cz + r], [cx - r, y1, cz + r], [cx - r, y1, cz - r], [cx - r, y0, cz - r]]),
        ([1.0, 0.0, 0.0], [[cx + r, y0, cz - r], [cx + r, y1, cz - r], [cx + r, y1, cz + r], [cx + r, y0, cz + r]]),
    ];
    for (normal, corners) in faces {
        let base = vertices.len() as u32;
        for (corner, uv) in corners.iter().zip([[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]) {
            vertices.push(GpuVertex {
                position: *corner,
                normal,
                tex_coord: uv,
                tex_index: tex,
                ao: 1.0,
                light: 0xF0,
                sway: 0.0,
            });
        }
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }
}

/// A translucent tint cube over one block (Step 25 power-grid overlay):
/// six faces slightly inflated so they don't z-fight the machine block,
/// drawn in the transparent pass like the waypoint beams.
fn push_overlay_cube(vertices: &mut Vec<GpuVertex>, indices: &mut Vec<u32>,
                     bx: f32, by: f32, bz: f32, tex: u32) {
    let e = 0.51f32; // half-extent inflated past the unit block
    let cx = bx + 0.5;
    let cy = by + 0.5;
    let cz = bz + 0.5;
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        ([0.0, 0.0, -1.0], [[cx - e, cy - e, cz - e], [cx - e, cy + e, cz - e], [cx + e, cy + e, cz - e], [cx + e, cy - e, cz - e]]),
        ([0.0, 0.0, 1.0], [[cx + e, cy - e, cz + e], [cx + e, cy + e, cz + e], [cx - e, cy + e, cz + e], [cx - e, cy - e, cz + e]]),
        ([-1.0, 0.0, 0.0], [[cx - e, cy - e, cz + e], [cx - e, cy + e, cz + e], [cx - e, cy + e, cz - e], [cx - e, cy - e, cz - e]]),
        ([1.0, 0.0, 0.0], [[cx + e, cy - e, cz - e], [cx + e, cy + e, cz - e], [cx + e, cy + e, cz + e], [cx + e, cy - e, cz + e]]),
        ([0.0, 1.0, 0.0], [[cx - e, cy + e, cz + e], [cx - e, cy + e, cz - e], [cx + e, cy + e, cz - e], [cx + e, cy + e, cz + e]]),
        ([0.0, -1.0, 0.0], [[cx - e, cy - e, cz - e], [cx + e, cy - e, cz - e], [cx + e, cy - e, cz + e], [cx - e, cy - e, cz + e]]),
    ];
    for (normal, corners) in faces {
        let base = vertices.len() as u32;
        for (corner, uv) in corners.iter().zip([[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]) {
            vertices.push(GpuVertex {
                position: *corner,
                normal,
                tex_coord: uv,
                tex_index: tex,
                ao: 1.0,
                light: 0xF0,
                sway: 0.0,
            });
        }
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    }
}

fn pseudo_random(seed: u64) -> u64 {    let mut h = seed.wrapping_mul(0x9E3779B97F4A7C15);
    h ^= h >> 31;
    h.wrapping_mul(0xC2B2AE3D27D4EB4F)
}

/// Title orbit eye height: never below the classic spawn+14, always above
/// the terrain the eye sweeps over (see camera() for the audit context).
fn title_eye_y(spawn_y: f32, ground_at_eye: i32) -> f32 {
    (spawn_y + 14.0).max(ground_at_eye as f32 + 6.0)
}

/// Screen-shake helpers (Step 3 impact pulse): heavy blocks broken with
/// heavy tools kick the camera a little; the amplitude decays fast so it
/// reads as an impact, not a wobble.
pub fn break_shake(block_hardness: f32, tool_tier: u8) -> f32 {
    ((block_hardness * 0.05) * (1.0 + tool_tier as f32 * 0.35)).min(0.35)
}

fn held_tool_tier(state: &GameState) -> u8 {
    state.inventory.slots[state.hotbar_index].as_ref()
        .and_then(|s| item_def(&s.item_id))
        .map(|d| match d.kind {
            ItemKind::Tool(_, tier) => tier,
            _ => 0,
        })
        .unwrap_or(0)
}

pub fn shake_decay(shake: f32, dt: f32) -> f32 {
    shake * (-dt * 8.0).exp()
}

/// Deterministic shake direction offset (right, up) in blocks for a frame.
pub fn shake_offset(shake: f32, frame: u64) -> (f32, f32) {
    if shake <= 0.01 {
        return (0.0, 0.0);
    }
    let t = frame as f32;
    (
        (t * 2.7).sin() * shake * 0.09,
        (t * 4.3).sin() * shake * 0.06,
    )
}

/// Per-biome color grade (goal Section 3): (tint multiply per channel,
/// saturation). The film-grade layer on top of per-block palettes and fog —
/// hot/dry pushes warm amber, cold pushes cool blue and desaturates,
/// lush/wet pushes green, the hollow goes eerie pale, oceans go teal, and
/// the temperate majority stays the neutral baseline.
pub fn biome_grade(b: lf_worldgen::Biome) -> ([f32; 3], f32) {
    use lf_worldgen::Biome as B;
    match b {
        B::Desert | B::Badlands => ([1.08, 1.00, 0.88], 0.92),
        B::Savanna | B::WindsweptSavanna => ([1.04, 1.00, 0.94], 0.97),
        B::SnowyTaiga | B::GiantTaiga | B::Tundra | B::IceSpikes | B::SnowySlope
        | B::SnowyPeaks | B::FrozenOcean => ([0.90, 0.98, 1.10], 0.85),
        B::Swamp | B::Jungle => ([0.93, 1.05, 0.95], 1.00),
        B::MushroomHollow => ([0.96, 0.94, 1.04], 0.80),
        B::Ocean | B::DeepOcean | B::WarmOcean => ([0.94, 1.00, 1.05], 0.96),
        _ => ([1.0, 1.0, 1.0], 1.0),
    }
}

/// Frustum + distance culling for a chunk column, using its mesh bounds.
fn column_in_view(view_proj: &glam::Mat4, eye: Vec3, pos: (i32, i32), min_y: f32, max_y: f32, view_distance: i32) -> bool {
    // Distance cull: columns past ~1.5x the kept radius (view distance +
    // unload margin) can't be visible anyway.
    let center_x = pos.0 as f32 * 16.0 + 8.0;
    let center_z = pos.1 as f32 * 16.0 + 8.0;
    let dx = center_x - eye.x;
    let dz = center_z - eye.z;
    let dist2 = dx * dx + dz * dz;
    let limit = ((view_distance + UNLOAD_MARGIN + 1) as f32 * 16.0 * 1.25).powi(2);
    if dist2 > limit {
        return false;
    }

    // Sphere-frustum test (Gribb-Hartmann planes from the view-projection).
    let cy = (min_y + max_y) * 0.5;
    let half_h = (max_y - min_y) * 0.5;
    // Bounding sphere of the column AABB (16 x 16 x height) = half-diagonal.
    // The old `half_h.max(11.4)` only covered the footprint along its axes;
    // the true corner distance is sqrt(128 + half_h^2) (~13.6 even for flat
    // ground), so columns poking into the frame edge were wrongly culled —
    // most visibly along the bottom of the view when looking up (P27).
    // The small margin absorbs the foliage wind sway.
    let radius = (128.0 + half_h * half_h).sqrt() + 0.1;
    let m = view_proj.to_cols_array();
    // planes as (a, b, c, d): a*x + b*y + c*z + d >= -radius = inside.
    // Normalized so the radius stays in world units (raw Gribb-Hartmann
    // normals vary in length: the near plane's is ~2, the far's ~0.0002).
    let rows = [
        [m[3] + m[0], m[7] + m[4], m[11] + m[8], m[15] + m[12]],  // left
        [m[3] - m[0], m[7] - m[4], m[11] - m[8], m[15] - m[12]],  // right
        [m[3] + m[1], m[7] + m[5], m[11] + m[9], m[15] + m[13]],  // bottom
        [m[3] - m[1], m[7] - m[5], m[11] - m[9], m[15] - m[13]],  // top
        [m[3] + m[2], m[7] + m[6], m[11] + m[10], m[15] + m[14]], // near
        [m[3] - m[2], m[7] - m[6], m[11] - m[10], m[15] - m[14]], // far
    ];
    for p in rows {
        let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        if len < 1e-6 {
            continue;
        }
        let dist = (p[0] * center_x + p[1] * cy + p[2] * center_z + p[3]) / len;
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
        assert!(column_in_view(&vp, eye, (0, -3), 60.0, 100.0, 5));
        // behind: culled
        assert!(!column_in_view(&vp, eye, (0, 3), 60.0, 100.0, 5));
        // far to the side: culled
        assert!(!column_in_view(&vp, eye, (30, -3), 60.0, 100.0, 5));
    }

    /// P27 regression: the old bounding sphere (max(half_h, 11.4)) ignored
    /// the footprint's corner distance (~13.6 for flat ground, ~17.7 for a
    /// 20-tall column), so columns still poking into the frame edge were
    /// wrongly culled — "objects disappear when looking up" (and, as the
    /// pinned case shows, tall columns vanish even near level pitch).
    /// Property: whenever any AABB corner projects inside the frustum, the
    /// column must be kept.
    #[test]
    fn looking_up_does_not_cull_visible_columns() {
        for pitch_deg in [5.0f32, 10.0, 15.0, 20.0, 30.0, 45.0, 60.0, 75.0, 85.0] {
            let pitch = pitch_deg.to_radians();
            for eye_y in [80.0f32, 90.0, 100.0, 120.0] {
                let eye = Vec3::new(8.0, eye_y, 8.0);
                let dir = Vec3::new(0.0, pitch.sin(), -pitch.cos());
                let vp = camera_frustum(eye, eye + dir * 40.0);
                for cx in -3..=3i32 {
                    for cz in -6..=2i32 {
                        // ground columns through tall (tree/mountain) ones
                        for (min_y, max_y) in [
                            (70.0f32, 75.0f32), (70.0, 90.0), (70.0, 95.0),
                            (60.0, 100.0), (75.0, 82.0),
                        ] {
                            let mut any_inside = false;
                            for corner_x in [cx as f32 * 16.0, cx as f32 * 16.0 + 16.0] {
                                for corner_y in [min_y, max_y] {
                                    for corner_z in [cz as f32 * 16.0, cz as f32 * 16.0 + 16.0] {
                                        let c = vp * glam::Vec4::new(corner_x, corner_y, corner_z, 1.0);
                                        if c.w > 1e-6
                                            && c.x.abs() <= c.w
                                            && c.y.abs() <= c.w
                                            && c.z.abs() <= c.w
                                        {
                                            any_inside = true;
                                        }
                                    }
                                }
                            }
                            if any_inside {
                                assert!(
                                    column_in_view(&vp, eye, (cx, cz), min_y, max_y, 5),
                                    "pitch {}° eye_y {}: column ({},{}) bounds {:?} pokes into view but was culled",
                                    pitch_deg, eye_y, cx, cz, (min_y, max_y)
                                );
                            }
                        }
                    }
                }
            }
        }
        // the concrete pre-fix failure (found by scanning the old test's
        // blind spots): tall column at the frame edge, near-level pitch —
        // the old max(half_h, 11.4) sphere culled it while a corner was
        // plainly inside the frustum
        let eye = Vec3::new(8.0, 80.0, 8.0);
        let five = 5.0f32.to_radians();
        let vp = camera_frustum(eye, eye + Vec3::new(0.0, five.sin(), -five.cos()) * 40.0);
        assert!(column_in_view(&vp, eye, (-3, -4), 70.0, 90.0, 5),
            "pinned case: tall column at the frame edge must stay visible");
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

    /// Audit Step 1 fix: the streamer's wish radius used to be hard-wired to
    /// 5, so raising the view-distance setting never generated farther
    /// chunks. sync_wish must both widen and narrow the request set.
    #[test]
    fn sync_wish_follows_view_distance() {
        let mut w = StreamWish {
            center: (0, 0),
            radius: 5,
            requested: HashSet::new(),
            stop: false,
        };
        for x in -5..=5 {
            for z in -5..=5 {
                w.requested.insert((x, z));
            }
        }
        sync_wish(&mut w, (0, 0), 8);
        assert_eq!(w.radius, 8);
        // the radius-5 ring is all requested, so the next candidate must
        // reach past it
        assert!(nearest_missing(&w).map(|p| chebyshev(p, (0, 0)) > 5).unwrap_or(false),
            "radius 8 must reach past the old radius-5 ring");
        w.requested.insert((8, 0));
        sync_wish(&mut w, (0, 0), 5);
        assert!(!w.requested.contains(&(8, 0)), "shrinking the radius prunes out-of-range chunks");
        assert!(w.requested.contains(&(4, 0)));
    }

    /// Audit Step 1 fix: the title orbit camera's fixed spawn+14 eye buried
    /// it inside ring terrain on hilly worlds (World_5: terrain up to y=128
    /// on the ring vs eye y=119 -> flat-dark title backdrop).
    #[test]
    fn title_eye_clears_ring_terrain() {
        assert_eq!(title_eye_y(105.0, 128), 134.0, "hill on the ring lifts the eye");
        assert_eq!(title_eye_y(105.0, 90), 119.0, "low terrain keeps the classic offset");
        assert_eq!(title_eye_y(105.0, 0), 119.0, "unloaded column (surface 0) keeps the classic offset");
    }

    /// Step 3 impact pulse: heavier blocks + heavier tools shake more, the
    /// shake decays to nothing, and the camera offset stays tiny (an impact,
    /// not a wobble) and zeroes below the noise floor.
    /// Step 13 done-when: a rebound key AND the quality tier survive a
    /// P33 save migration: pre-magic bincode extras load through the frozen
    /// LegacyClientSave shape — and would EOF on the current struct (that
    /// gap is exactly why the legacy path exists).
    #[test]
    fn legacy_bincode_extras_migrate_instead_of_resetting() {
        let old = LegacyClientSave {
            slots: vec![Some(ItemStack { item_id: "iron_pickaxe".into(), count: 1 })],
            health: 17.0,
            hunger: 12.0,
            time_ticks: 90_000,
            block_entities: vec![],
            mobs: vec![],
            kills: 9,
            quest_log: None,
            chronicle: vec![],
            world_type: None,
            villagers: vec![],
            research: None,
            settings: None,
            waypoints: vec![],
        };
        let bytes = bincode::serialize(&old).expect("legacy serialize");
        // the current struct would fail on those bytes (bincode + new fields = EOF)
        assert!(bincode::deserialize::<ClientSave>(&bytes).is_err(),
            "the legacy shape is load-bearing; if this passes the test is stale");
        let migrated = bincode::deserialize::<LegacyClientSave>(&bytes)
            .map(ClientSave::from)
            .expect("legacy path loads");
        assert_eq!(migrated.health, 17.0);
        assert_eq!(migrated.kills, 9);
        assert!((migrated.mana - lf_game::magic::MAX_MANA).abs() < 1e-4, "migrated worlds start with a full pool");
        assert!(migrated.spellbook.is_none());
    }

    /// Future field additions stay compatible: JSON extras missing the
    /// newest field default it (serde default works on self-describing
    /// formats — the reason extras moved off bincode).
    #[test]
    fn json_extras_tolerate_missing_new_fields() {
        let save = ClientSave::default();
        let json = serde_json::to_string(&save).expect("json serialize");
        let stripped = json.replace(&format!("\"mana\":{}", save.mana), "\"mana_stripped\":0");
        let loaded: ClientSave = serde_json::from_str(&stripped).expect("older json loads");
        assert!((loaded.mana - lf_game::magic::MAX_MANA).abs() < 1e-4, "missing mana defaults full");
    }

    /// The spellbook persists through the JSON extras path.
    #[test]
    fn spellbook_persists_through_client_save() {
        let mut book = lf_game::magic::Spellbook::default();
        book.learn(lf_game::magic::Spell::Firebolt);
        book.learn(lf_game::magic::Spell::Ward);
        let save = ClientSave { spellbook: Some(book), mana: 12.5, ..Default::default() };
        let loaded: ClientSave = serde_json::from_str(&serde_json::to_string(&save).unwrap()).unwrap();
        let book = loaded.spellbook.unwrap();
        assert!(book.knows(lf_game::magic::Spell::Firebolt));
        assert!((loaded.mana - 12.5).abs() < 1e-4);
    }

    /// P35: the screen face is a live readout of real state.
    #[test]
    fn screen_face_pages_read_out_state() {
        let p1 = compose_screen_face(1, 3, 2, 40, 0, 0);
        // era 3 -> pips at rows 13, 10, 7; branches 2 -> two violet ticks
        let has_pip = p1.get_pixel(3, 13).0[1] > 200;
        assert!(has_pip, "era pips render");
        let p2 = compose_screen_face(2, 0, 0, 36, 0, 0);
        assert!(p2.get_pixel(6, 2).0[0] > 200, "chronicle rows render (36 events = 3 rows)");
        let p3 = compose_screen_face(3, 0, 0, 0, 3, 1);
        let mut greens = 0u32;
        for y in 0..16u32 {
            for x in 0..16u32 {
                let px = p3.get_pixel(x, y).0;
                if px[1] > 200 && px[0] < 200 {
                    greens += 1;
                }
            }
        }
        assert!(greens > 0, "the grid page shows the green share");
    }

    /// ClientSave bincode round trip (the same path save_world/load use).
    #[test]
    fn rebind_and_quality_tier_persist_through_client_save() {
        let mut save = ClientSave::default();
        let mut settings = Settings::default();
        let mut km = crate::input::Keymap::default();
        km.rebind(crate::input::Action::Jump, KeyCode::KeyN);
        settings.keymap_pairs = km.to_pairs();
        settings.apply_quality(Quality::PathTraced);
        save.settings = Some(settings);
        let bytes = bincode::serialize(&save).expect("serialize ClientSave");
        let loaded: ClientSave = bincode::deserialize(&bytes).expect("deserialize ClientSave");
        let s = loaded.settings.expect("settings survived");
        assert_eq!(s.quality, 3, "PathTraced tier persists");
        assert_eq!(s.rt_mode, RtMode::Live, "tier drives the render path");
        let km2 = crate::input::Keymap::from_pairs(&s.keymap_pairs);
        assert_eq!(km2.key(crate::input::Action::Jump), KeyCode::KeyN, "rebound jump persists");
        assert_eq!(km2.key(crate::input::Action::Forward), KeyCode::KeyW, "untouched binding keeps default");
    }

    #[test]
    fn screen_shake_envelope() {
        assert!(break_shake(4.5, 2) > break_shake(1.5, 0), "harder block + heavier tool shakes more");
        assert!(break_shake(99.0, 5) <= 0.35, "capped");
        let mut s = 0.3f32;
        for _ in 0..30 {
            s = shake_decay(s, 1.0 / 60.0);
        }
        assert!(s < 0.01, "half a second of decay kills the shake, got {s}");
        assert_eq!(shake_offset(0.0, 42).0, 0.0);
        let (r, u) = shake_offset(0.35, 7);
        assert!(r.abs() <= 0.09 * 0.35 * 1.01 && u.abs() <= 0.06 * 0.35 * 1.01, "offsets scale with amplitude");
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
