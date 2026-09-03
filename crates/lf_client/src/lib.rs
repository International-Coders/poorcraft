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
pub mod factions;
pub mod hud_channels;
pub mod input;
pub mod lore;
pub mod icons;
pub mod map;
pub mod onboarding;
pub mod slots;
pub mod smoke;
pub mod net;
pub mod ui;
pub mod ui_kit;
pub mod workbench;

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
use lf_voxel::raycast::{raycast_voxel, raycast_voxel_boxes};
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

/// Drain the accumulated scroll notches into whole hotbar steps, keeping
/// the fractional remainder (macOS trackpads emit sub-notch PixelDelta).
/// One notch = one slot and a fast flick delivers every notch it
/// accumulated — nothing is dropped (loop 347 wheel fix).
pub fn consume_scroll_steps(accum: &mut f32) -> i32 {
    let steps = *accum as i32; // truncates toward zero: sign-correct steps
    *accum -= steps as f32;
    steps
}

/// Index of the nearest mob actually under the crosshair and in plain
/// sight: cone-tested along the look ray, nearest-first, then
/// occlusion-filtered against the world. A mob on the far side of the
/// block you are aiming at must NOT steal the LMB into the throttled
/// attack branch — that reroute was the "creative breaks 2 blocks a
/// second" bug (loop 347).
fn crosshair_mob(
    mobs: &[MobEntity],
    world: &lf_voxel::World,
    eye: Vec3,
    look: Vec3,
    reach: f32,
) -> Option<usize> {
    let mut candidates: Vec<(f32, usize)> = Vec::new();
    for (i, mob) in mobs.iter().enumerate() {
        if mob.death_t.is_some() {
            continue; // corpses are not targets
        }
        let size = mob.mob_type.stats().size;
        let center = mob.position + Vec3::new(0.0, size, 0.0);
        let t = (center - eye).dot(look);
        if t < 0.0 || t > reach + 1.0 {
            continue;
        }
        let closest = eye + look * t;
        if (closest - center).length() < size + 0.45 {
            candidates.push((t, i));
        }
    }
    candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    for (_, i) in candidates {
        let mob = &mobs[i];
        let center = mob.position + Vec3::new(0.0, mob.mob_type.stats().size, 0.0);
        if lf_game::mobs::has_line_of_sight(eye, center, world) {
            return Some(i);
        }
    }
    None
}

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
    /// Hand-crafting (the "craft by hand" route from the inventory).
    HandCraft,
    /// C1: the world-creation screen (name, seed, type, mode, difficulty).
    NewWorld,
    /// C3: direct connect / host world / lobby stub.
    Multiplayer,
    Settings,
    Death,
    Paths,
    /// B3: the companion command menu (index held in `companion_menu`).
    CompanionMenu,
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
    Belt(lf_game::machines::Belt),
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
    /// Unique id (carry targeting survives vec churn).
    pub id: u64,
    /// Rigid prop state: gravity + bounces off floor and walls, tumbles
    /// while moving, sleeps when it runs out of energy (lf_game::props).
    pub body: lf_game::props::PropBody,
    pub age: f32,
}

/// A granular block (sand/dirt-family) detached from its support and
/// falling; lands and re-places itself as a block.
#[derive(Clone, Debug)]
pub struct FallingBlock {
    pub position: Vec3,
    pub velocity: f32,
    pub block: BlockState,
    // loop 330 deep-fall polish: render-only tumble + one landing bounce
    pub tumble_axis: Vec3,
    pub angle: f32,
    pub angvel: f32,
    pub bounced: bool,
}

/// Tumble advances linearly in angvel (the physics stays the scalar drop).
fn tumble_step(angle: f32, angvel: f32, dt: f32) -> f32 {
    angle + angvel * dt
}

/// What happens when a faller touches down: fast first impacts bounce once
/// (restitution 0.18, upward velocity returned), everything else settles.
enum FallerLanding {
    Place,
    Bounce(f32),
}

fn faller_landing(velocity: f32, bounced: bool) -> FallerLanding {
    if !bounced && velocity > 6.0 {
        FallerLanding::Bounce(velocity * 0.18)
    } else {
        FallerLanding::Place
    }
}

/// Deterministic per-faller tumble axis (visual variety without RNG).
fn faller_tumble_axis(seed: u64) -> Vec3 {
    // fibonacci-hash first: nearby cell coords must not give near-identical axes
    let h = seed.wrapping_mul(0x9E3779B97F4A7C15);
    let a = ((h >> 20) % 1000) as f32 / 1000.0 - 0.5;
    let b = ((h >> 44) % 1000) as f32 / 1000.0 - 0.5;
    Vec3::new(0.5 + a, 0.0, 0.5 + b).normalize()
}

/// A felled tree mid-fall (loop 330): the blocks are already removed from
/// the world; the entity renders the rigid rotation and lands into the
/// fall_plan's horizontal log row.
pub struct FallingTree {
    pub tree: lf_game::timber::Tree,
    pub dir: lf_game::timber::FallDir,
    pub angle: f32,
    pub angvel: f32,
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
    #[serde(default)]
    pub paths: Option<lf_game::paths::Paths>,
    // lore-and-visuals (Sections A + B)
    /// Per-faction standing (-100..+100); None = fresh defaults.
    #[serde(default)]
    pub faction_standing: Option<lf_lore::StandingState>,
    #[serde(default)]
    pub companions: Vec<lf_game::companions::Companion>,
    /// Trust remembered across dismiss/quit/re-hire (archetype -> trust).
    #[serde(default)]
    pub companion_memory: std::collections::HashMap<String, i32>,
    /// Biome variant names the player has visited (ashen_q2 tracking).
    #[serde(default)]
    pub visited_biomes: Vec<String>,
    /// First-time-discovered faction structures (key, x, y, z).
    #[serde(default)]
    pub discovered_structures: Vec<(String, i32, i32, i32)>,
    /// Absolute in-game day count (wages/re-hire timing; TimeOfDay wraps).
    #[serde(default)]
    pub day_index: u64,
    /// F3: the earned recipe set (first pickups drive unlocks).
    #[serde(default)]
    pub recipe_book: Option<workbench::RecipeBook>,
    /// The workbench "Add to Queue" placeholder queue (output, batch).
    #[serde(default)]
    pub craft_queue: Vec<(String, u32)>,
    /// loop 345: kingdoms discovered (the throne scan settles a court and
    /// records the site; the compass works without discovery too).
    #[serde(default)]
    pub kingdoms: Vec<KingdomRecord>,
    /// N01: first-minute tutorial + pinned-objective state (None on old
    /// saves = fresh tutorial, which only shows for a brand-new player
    /// anyway because old saves have progress).
    #[serde(default)]
    pub onboarding: Option<onboarding::Onboarding>,
}

/// One discovered kingdom (loop 345): display name + throne position.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct KingdomRecord {
    pub name: String,
    pub throne: [i32; 3],
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
            paths: None,
            faction_standing: None,
            companions: Vec::new(),
            companion_memory: std::collections::HashMap::new(),
            visited_biomes: Vec::new(),
            discovered_structures: Vec::new(),
            day_index: 0,
            recipe_book: None,
            craft_queue: Vec::new(),
            kingdoms: Vec::new(),
            onboarding: None,
        }
    }
}

struct App {
    state: Option<GameState>,
    autostart: bool,
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
        if self.autostart {
            if let Some(state) = &mut self.state {
                // the exact code path the single-player New World screen runs
                state.open_new_world_screen();
                let _ = state.create_world_from_screen();
            }
        }
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
                                let build_shape_key = state.keymap.key(crate::input::Action::BuildShape);
                                let paths_key = state.keymap.key(crate::input::Action::PathsScreen);
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
                                                // fullscreen menu screens opened
                                                // from the title: Esc returns to
                                                // the title, not into the world
                                                UiOpen::Slots | UiOpen::NewWorld | UiOpen::Multiplayer => {
                                                    state.ui_open = UiOpen::Title;
                                                    state.menu_reveal = 0.0;
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
                                                // N03: the pack key is also the way OUT of
                                                // every container/station screen — one key,
                                                // one way back to play (Escape still closes
                                                // everything). Pure contract tested in ui.rs.
                                                if crate::ui::inventory_key_closes(&state.ui_open) {
                                                    state.close_ui();
                                                } else if state.ui_open == UiOpen::None {
                                                    state.ui_open = UiOpen::Inventory;
                                                    state.unlock_cursor();
                                                }
                                                return;
                                            }
                                        }
                                        k if k == fly_key => {
                                            // creative-only since loop 329 (was
                                            // an ungated debug toggle in survival)
                                            if state.game_mode.may_fly() {
                                                state.player.flying = !state.player.flying;
                                                state.player.velocity = Vec3::ZERO;
                                            }
                                        }
                                        k if k == shot_key => state.take_screenshot(),
                                        k if k == dbg_key => {
                                            state.show_debug = !state.show_debug;
                                        }
                                        k if k == rt_key => { if state.settings.rt_mode == crate::RtMode::Captures { state.take_raytraced_screenshot(); } }
                                        k if k == paths_key => {
                                            if matches!(state.ui_open, UiOpen::None | UiOpen::Paths) && state.stats.health > 0.0 {
                                                if state.ui_open == UiOpen::Paths {
                                                    state.close_ui();
                                                } else {
                                                    state.ui_open = UiOpen::Paths;
                                                    state.menu_reveal = 0.0;
                                                    state.unlock_cursor();
                                                }
                                                return;
                                            }
                                        }
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
                                        k if k == build_shape_key => {
                                            if matches!(state.ui_open, UiOpen::None) && state.stats.health > 0.0 {
                                                state.build_shape = state.build_shape.next();
                                                state.push_hint(&format!(
                                                    "placement shape: {} (R to cycle)",
                                                    state.build_shape.label()
                                                ));
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
    run_with_autostart(false);
}

/// `autostart` (debug harness): boot straight into a freshly created
/// world, skipping the title-screen clicks, so the menu → game render
/// transition can be exercised without a pointer.
pub fn run_with_autostart(autostart: bool) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let event_loop = EventLoop::new().expect("EventLoop failed");
    let mut app = App { state: None, autostart };
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
    /// Save-slot storage. `None` on the title screen's version-seeded
    /// preview world (ui-world-craft B): the preview generates in memory,
    /// displays, and never touches `worlds/`.
    storage: Option<WorldStorage>,
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
    /// The active world's difficulty (C1). Drives mob damage, hunger pace
    /// and hostile spawns; Peaceful keeps hostiles out entirely.
    pub difficulty: slots::Difficulty,
    /// The active world's game mode (C1). Saved with the world; Creative
    /// is a stub that plays like Survival until content gates exist.
    pub game_mode: slots::GameMode,
    /// (target, stage) the crack decal batch was built for.
    crack_state: Option<((i32, i32, i32), u32)>,
    crack_batch: Option<MeshBatch>,
    particle_batch: Option<MeshBatch>,
    pub particles: Vec<Particle>,
    particle_timer: f32,
    /// Total elapsed seconds (drives foliage wind).
    elapsed: f32,
    /// Frame delta kept for render-side animation (remote-player gait).
    last_dt: f32,
    /// Remote-player motion estimate: (last pos, walk phase, amplitude).
    remote_motion: std::collections::HashMap<u64, (Vec3, f32, f32)>,
    /// Item-drop prop ids (carry targeting survives vec churn).
    next_drop_id: u64,
    /// The prop held by right-click carry (gravity-gun style): its id and
    /// the distance along the view ray it was grabbed at.
    carried_drop: Option<u64>,
    carry_dist: f32,
    pub drops: Vec<ItemDrop>,
    drop_batch: Option<MeshBatch>,
    pub block_entities: HashMap<(i32, i32, i32), BlockEntity>,
    pub mobs: Vec<MobEntity>,
    pub villagers: Vec<Villager>,
    pub arrows: Vec<Arrow>,
    /// Granular blocks mid-fall (sand/dirt-family), animated until they
    /// land and re-place themselves.
    pub falling_blocks: Vec<FallingBlock>,
    pub falling_trees: Vec<FallingTree>,
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
    /// N01: the persisted first-minute tutorial + pinned-objective state.
    pub onboarding: onboarding::Onboarding,
    /// N02: countdown to the next craft-queue job completion.
    craft_queue_timer: f32,
    /// N04: contextual HUD channels (prompts data, toasts, banners, hit dir).
    pub hud_channels: hud_channels::HudChannels,
    /// N04: settlements already announced this session (banner fires once).
    settlement_seen: std::collections::HashSet<String>,
    /// N04: hostiles near the player with line of sight (throttled scan).
    threat_count: u8,
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
    // ---- world-creation screen state (ui-world-craft C1) ----
    pub new_world_name: String,
    /// The seed field: a number OR an arbitrary string (strings hash).
    pub new_world_seed: String,
    pub new_world_type_idx: usize,
    pub new_world_mode_idx: usize,
    pub new_world_diff_idx: usize,
    /// "World needs a name." — shown after a failed Create click.
    pub new_world_error: Option<String>,
    // ---- load-screen state (C2) ----
    /// Slot awaiting a delete confirmation ("This cannot be undone.").
    pub delete_confirm: Option<String>,
    // ---- multiplayer screen state (C3) ----
    pub mp_address: String,
    pub mp_port: String,
    pub mp_host_idx: usize,
    pub mp_status: Option<String>,
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
    /// Crosshair target of this frame (cell + block id) — drives the
    /// looked-at-block caption above the hotbar (loop 347).
    pub look_target: Option<(glam::IVec3, u32)>,
    /// Ground distance since the last footstep (loop 329 audio set).
    step_distance: f32,
    /// Loop 349: chest-in-water state of the previous frame (splash edge).
    was_in_water: bool,
    /// Previous frame's open screen — drives the ui transition click.
    pub prev_ui_open: UiOpen,
    // ---- workbench state (ui-world-craft F) ----
    /// The earned recipe set (pickups + eras), persisted with the world.
    pub recipe_book: workbench::RecipeBook,
    /// Selected sidebar category (index into workbench::CATEGORIES).
    pub wb_category: usize,
    /// The recipe whose detail is open (output item id).
    pub wb_selected: Option<String>,
    /// Craft batch size in the detail panel.
    pub wb_qty: u32,
    /// N03: workbench discovery state — text search, filter chip, station
    /// chip (session UI state, not persisted).
    pub wb_search: String,
    pub wb_filter: u8,
    pub wb_station: u8,
    /// "Add to Queue" placeholder queue (output, batch size).
    pub craft_queue: Vec<(String, u32)>,
    /// Quest log tab: 0 = active quests, 1 = chronicle.
    pub quest_tab: usize,
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
    /// Loop 343 building HUD: the picked placement shape for held blocks
    /// (R cycles; the strip above the hotbar selects directly).
    pub build_shape: lf_game::items::BuildShape,
    /// P35: live producer positions (elevator/climate power checks).
    pub producer_positions: Vec<(i32, i32, i32)>,
    /// P35: the screen texture is rewritten only when its data changes.
    screen_signature: u64,
    /// P36: the dragon the player rides (mob id).
    pub mounted_dragon: Option<u64>,
    /// P37: path standings + focus (accrued by play, never decay).
    pub paths: lf_game::paths::Paths,
    /// Lore-and-visuals: factions, world events, roster, dialogue (A1) —
    /// loaded from lore/*.toml at boot, never serialized.
    pub lore_data: lf_lore::LoreRegistry,
    /// Per-faction standing (A1), persisted in ClientSave.
    pub standings: lf_lore::StandingState,
    /// Active hired companions (Section B), persisted in ClientSave.
    pub companions: Vec<lf_game::companions::Companion>,
    /// Trust remembered across dismiss/re-hire (archetype -> trust).
    pub companion_memory: std::collections::HashMap<String, i32>,
    /// Trust-memory view for re-hire ("they remember what happened").
    pub visited_biomes: std::collections::HashSet<String>,
    /// Discovered faction structures (key, x, y, z) — map icons + D2.
    pub discovered_structures: Vec<(String, i32, i32, i32)>,
    /// loop 345: kingdoms whose thrones have been seen (map crown icons,
    /// chronicle, persisted).
    pub kingdoms: Vec<KingdomRecord>,
    /// Cached kingdom-compass readout (name, bearing rad, meters) — the
    /// worldgen query is throttled to once a second.
    pub kingdom_compass_state: Option<(String, f32, i32)>,
    /// Compass cache age in frames.
    kingdom_compass_age: u32,
    /// Absolute in-game day count (wages at sunrise).
    pub day_index: u64,
    /// Previous day-fraction (sunrise edge detection).
    prev_day_fraction: f32,
    /// Companion attack cooldowns (parallel to `companions`).
    companion_cooldowns: Vec<f32>,
    /// Per-companion contextual-line timers (seconds until next line).
    companion_line_timers: Vec<f32>,
    /// The companion the command menu is open for (index).
    pub companion_menu: Option<usize>,
    /// Standing-change pulse for the HUD widget (1 -> 0).
    pub faction_pulse: f32,
    /// C3: factions whose +75 acknowledgement is still pending (consumed
    /// on the next interaction with any NPC of that faction).
    pub honored_ack: std::collections::HashSet<String>,
    /// The faction whose widget is shown (id, value) for pulse detection.
    faction_widget_state: Option<(String, i32)>,
    /// Ambient ember emission accumulator (C4).
    ember_timer: f32,
    /// Faction villagers spawned this session keyed by marker block pos
    /// (prevents re-settling after save/load churn).
    settled_markers: std::collections::HashSet<(i32, i32, i32)>,
    /// Index of the mob that most recently damaged the player (companions
    /// defend them, B4); cleared when the mob dies or cools down.
    last_attacker: Option<usize>,
    /// Quest-tag cells already fired (road markers / ember formations).
    road_cells: std::collections::HashSet<(i32, i32)>,
    ember_cells: std::collections::HashSet<(i32, i32)>,
    /// Steps 21-22: the chronicle surfaces DURING play — milestone
    /// events toast across the top for a few seconds.
    pub chronicle_toast: Option<(String, f32)>,
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
                        // king-quest asset pass: the atlas (194 named + one
                        // generated layer per mod block) can exceed the 256
                        // default; Metal/Vulkan adapters expose 2048+
                        max_texture_array_layers: 512,
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
            // Opaque (loop 331): the compositor must ignore framebuffer
            // alpha. With a premultiplied/inherited mode the desktop behind
            // the window blended through pixels whose alpha wasn't 1
            // (water, ice, unlit regions) — the reported "black box" while
            // playing was the dark window behind showing through.
            alpha_mode: if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
                wgpu::CompositeAlphaMode::Opaque
            } else {
                caps.alpha_modes[0]
            },
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Load mods into the live registries before touching the world.
        // Steam Workshop items installed into `workshop/` (UGC staging dir
        // — the Steam client delivers here in a packaged build) load with
        // exactly the same path as bundled mods.
        let mut mods = lf_modapi::load_mods_dir(Path::new("mods"));
        let workshop = std::env::var("LOREFORGE_WORKSHOP_DIR")
            .unwrap_or_else(|_| "workshop".into());
        let installed = lf_steam::workshop::scan_installed(Path::new(&workshop));
        if !installed.is_empty() {
            for item in &installed {
                if let Ok(data) = lf_modapi::load_mod(std::path::Path::new(&item.path)) {
                    if mods.iter().any(|m| m.manifest.id == data.manifest.id) {
                        continue; // bundled copy wins
                    }
                    lf_modapi::apply_mod(&data);
                    mods.push(data);
                    tracing::info!("workshop mod loaded: {} ({})", item.title, item.id);
                }
            }
        }
        if !mods.is_empty() {
            tracing::info!("loaded {} mod(s): {:?}", mods.len(),
                mods.iter().map(|m| m.manifest.id.clone()).collect::<Vec<_>>());
        }
        // Steam transport selection (loop 335): with the `steam` feature
        // and a running, logged-in client this logs SteamP2p + the player's
        // Steam ID; otherwise it stays UDP.
        tracing::info!("transport = {:?}", lf_steam::preferred_transport());
        if let Some(line) = lf_modapi::smoke_line(&mods) {
            tracing::info!("{line}");
        }

        // Persistence: the title screen sits on a version-seeded PREVIEW
        // world (ui-world-craft B1) — generated in memory from the game
        // version, never persisted, so no `worlds/` directory is touched
        // until the player creates or loads a real slot. Player extras load
        // with the slot when one is opened.
        let lore_reg = lf_lore::LoreRegistry::load(Path::new("lore"));
        let (inventory, stats, time, block_entities, mobs, villagers, kills, quest_log, chronicle, research, settings, world_type, waypoints, spellbook, runed, paths, lore_extras) = {
            // a path that never exists: load_client_save degrades to fresh
            // defaults for the preview session; real values arrive with the
            // loaded slot
            load_client_save(Path::new(""), &lore_reg)
        };
        let start_day_fraction = time.fraction();
        let world_seed = lf_worldgen::preview::version_preview_seed();
        let world_dir = std::path::PathBuf::new(); // no slot: nothing saves
        let saved_set = HashSet::new();
        let storage: Option<WorldStorage> = None;
        let gen = WorldGen::with_type(Seed(world_seed), world_type);
        let mut world = World::new();
        for cx in -BOOT_RADIUS..=BOOT_RADIUS {
            for cz in -BOOT_RADIUS..=BOOT_RADIUS {
                world.chunks.insert((cx, cz), gen.generate_chunk(cx, cz));
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
            // fix: black-square artifact — empty meshes get no batch
            if !v.is_empty() {
                batches.insert((cx, cz), MeshBatch::new(&device, &resources, &v, &i));
            }
            if !wv.is_empty() {
                water_batches.insert((cx, cz), MeshBatch::new(&device, &resources, &wv, &wi));
            }
            cpu_meshes.insert((cx, cz), (v, i));
            column_bounds.insert((cx, cz), (min_y, max_y));
        }

        let outline = OutlineScene::new(&device, config.format);
        let (depth_texture, depth_view) = MeshBatch::create_depth_texture(&device, config.width, config.height);

        // Player: the preview session spawns fresh on the surface; a real
        // slot restores its saved player in load_world.
        let spawn_point = Vec3::new(0.5, world.surface_height(0, 0) as f32 + 0.2, 0.5);
        let mut player = Player::new(spawn_point);
        player.flying = false;
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
            last_dt: 0.0,
            remote_motion: std::collections::HashMap::new(),
            next_drop_id: 1,
            carried_drop: None,
            carry_dist: 0.0,
            block_entities: HashMap::new(),
            mobs: Vec::new(),
            villagers: Vec::new(),
            arrows: Vec::new(),
            falling_blocks: Vec::new(),
            falling_trees: Vec::new(),
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
            slot_meta: slots::SlotMeta {
                name: "Preview".into(),
                world_type,
                seed: world_seed,
                updated_secs: 0,
                ..Default::default()
            },
            difficulty: slots::Difficulty::Easy,
            game_mode: slots::GameMode::Survival,
            slot_name_input: String::new(),
            slot_new_type: lf_worldgen::WorldType::Normal,
            title_show_new: false,
            new_world_name: String::new(),
            new_world_seed: String::new(),
            new_world_type_idx: 0,
            new_world_mode_idx: 0,
            new_world_diff_idx: 1,
            new_world_error: None,
            delete_confirm: None,
            mp_address: String::new(),
            mp_port: "25565".to_string(),
            mp_host_idx: 0,
            mp_status: None,
            icons,
            map: map::MapState::new(world_type, world_seed),
            waypoints,
            spellbook,
            runed_tools: runed,
            paths,
            chronicle_toast: None,
            lore_data: lore_reg,
            standings: lore_extras.standings,
            companions: lore_extras.companions.clone(),
            companion_memory: lore_extras.companion_memory,
            visited_biomes: lore_extras.visited_biomes,
            discovered_structures: lore_extras.discovered_structures,
            kingdoms: lore_extras.kingdoms,
            kingdom_compass_state: None,
            kingdom_compass_age: 1000, // first held frame queries immediately
            day_index: lore_extras.day_index,
            prev_day_fraction: start_day_fraction,
            companion_cooldowns: vec![0.0; lore_extras.companions.len()],
            companion_line_timers: vec![8.0; lore_extras.companions.len()],
            companion_menu: None,
            faction_pulse: 0.0,
            honored_ack: Default::default(),
            faction_widget_state: None,
            ember_timer: 0.0,
            settled_markers: std::collections::HashSet::new(),
            last_attacker: None,
            road_cells: std::collections::HashSet::new(),
            ember_cells: std::collections::HashSet::new(),
            imbue: lf_game::magic::ImbueMinigame::new(3),
            symmetry_plane: None,
            build_shape: lf_game::items::BuildShape::Block,
            producer_positions: Vec::new(),
            screen_signature: 0,
            mounted_dragon: None,
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
            look_target: None,
            step_distance: 0.0,
            was_in_water: false,
            onboarding: onboarding::Onboarding::default(),
            craft_queue_timer: 0.0,
            hud_channels: hud_channels::HudChannels::default(),
            settlement_seen: std::collections::HashSet::new(),
            threat_count: 0,
            prev_ui_open: UiOpen::Title,
            recipe_book: workbench::RecipeBook::default(),
            wb_category: 0,
            wb_selected: None,
            wb_qty: 1,
            wb_search: String::new(),
            wb_filter: 0,
            wb_station: 0,
            craft_queue: Vec::new(),
            quest_tab: 0,
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
                let id = self.next_drop_id;
                self.next_drop_id += 1;
                let pos = self.player.eye_position() + self.player.look_dir();
                self.drops.push(ItemDrop {
                    stack: ItemStack { count: leftover, ..slot },
                    id,
                    body: lf_game::props::PropBody::new(
                        pos, Vec3::new(0.0, 2.0, 0.0), faller_tumble_axis(id)),
                    age: 0.0,
                });
            }
        }
        if let Some(cursor) = self.cursor_stack.take() {
            let leftover = self.inventory.add_item(&cursor.item_id, cursor.count);
            if leftover > 0 {
                let id = self.next_drop_id;
                self.next_drop_id += 1;
                let pos = self.player.position + Vec3::new(0.0, 1.0, 0.0);
                self.drops.push(ItemDrop {
                    stack: ItemStack { count: leftover, ..cursor },
                    id,
                    body: lf_game::props::PropBody::new(
                        pos, Vec3::ZERO, faller_tumble_axis(id)),
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
        self.create_world("World 1", slots::random_seed(), world_type,
            slots::Difficulty::Easy, slots::GameMode::Survival);
    }

    /// Create (or fully reset) the named slot with a fresh random seed.
    pub fn new_world_named(&mut self, name: &str, world_type: lf_worldgen::WorldType) {
        self.create_world(name, slots::random_seed(), world_type,
            slots::Difficulty::Easy, slots::GameMode::Survival);
    }

    /// C1: open the world-creation screen with fresh defaults — the world
    /// gets the next free "World N" name and a visible random seed the
    /// player can reroll or replace.
    pub fn open_new_world_screen(&mut self) {
        let n = crate::slots::list_slots().len() + 1;
        self.new_world_name = format!("World {}", n);
        self.new_world_seed = slots::random_seed().to_string();
        self.new_world_type_idx = 0;
        self.new_world_mode_idx = 0;
        self.new_world_diff_idx = 1;
        self.new_world_error = None;
        self.ui_open = UiOpen::NewWorld;
        self.menu_reveal = 0.0;
    }

    /// C1: create a world from the New World screen's fields. The seed is
    /// a number (parsed) or any other string (hashed) — a string seed is
    /// never an error. Returns Err with a user-facing message on invalid
    /// input (empty name).
    pub fn create_world_from_screen(&mut self) -> Result<(), String> {
        let name = self.new_world_name.trim().to_string();
        if name.is_empty() {
            self.new_world_error = Some("World needs a name.".into());
            return Err(self.new_world_error.clone().unwrap());
        }
        let seed = slots::parse_seed_field(&self.new_world_seed);
        let world_type = [
            lf_worldgen::WorldType::Normal,
            lf_worldgen::WorldType::Superflat,
            lf_worldgen::WorldType::Amplified,
        ][self.new_world_type_idx];
        let difficulty = slots::Difficulty::ALL[self.new_world_diff_idx.min(3)];
        let game_mode = slots::GameMode::ALL[self.new_world_mode_idx.min(1)];
        self.create_world(&name, seed, world_type, difficulty, game_mode);
        Ok(())
    }

    /// Create (or fully reset) the named slot with an explicit seed and
    /// ruleset — the one true world-creation path.
    pub fn create_world(&mut self, name: &str, seed: u64,
        world_type: lf_worldgen::WorldType, difficulty: slots::Difficulty,
        game_mode: slots::GameMode) {
        self.save_world();
        let name = slots::sanitize(name);
        let dir = slots::slot_dir(&name);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let meta = slots::SlotMeta {
            name: name.clone(),
            world_type,
            seed,
            updated_secs: slots::now_secs(),
            created_secs: slots::now_secs(),
            difficulty,
            game_mode,
            version_created: env!("CARGO_PKG_VERSION").into(),
        };
        let _ = std::fs::create_dir_all(&dir);
        let _ = WorldStorage::open(&dir).save_seed(seed);
        let _ = lf_worldgen::save_generator_version(&dir, lf_worldgen::GENERATOR_VERSION);
        slots::write_meta(&dir, &meta);
        // point the client at the new slot
        self.storage = Some(WorldStorage::open(&dir));
        self.world_dir = dir;
        self.slot_meta = meta;
        self.world_seed = seed;
        self.world_type = world_type;
        self.difficulty = difficulty;
        self.game_mode = game_mode;
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
        // N01: a brand-new world walks the first-minute tutorial again
        self.onboarding = onboarding::Onboarding::default();
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
        // fix: black-square artifact — the Live RT pathtracer kept the
        // previous world's voxel clip (`upload_voxels` early-returns on an
        // unchanged center) and the egui handle kept the previous world's
        // composited image, so a stale frame covered the new world after a
        // transition. Drop both; the tracer is rebuilt on the next tick.
        self.live_tracer = None;
        self.live_rt_texture = None;
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
        let lore_reg = lf_lore::LoreRegistry::load(Path::new("lore"));
        let (inventory, stats, time, block_entities, mobs, villagers, kills, quest_log,
             chronicle, research, settings, world_type, waypoints, spellbook, runed, paths, lore) = load_client_save(&dir, &lore_reg);
        let load_day_fraction = time.fraction();
        let storage = WorldStorage::open(&dir);
        let seed = storage.load_seed().unwrap_or(meta.seed);
        let saved_set = storage.saved_chunks();
        self.world_dir = dir;
        self.slot_meta = slots::SlotMeta { seed, updated_secs: meta.updated_secs, ..meta };
        self.difficulty = self.slot_meta.difficulty;
        self.game_mode = self.slot_meta.game_mode;
        self.world_seed = seed;
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
        self.paths = paths;
        self.lore_data = lore_reg;
        self.standings = lore.standings;
        self.prev_day_fraction = load_day_fraction;
        self.companions = lore.companions;
        self.companion_memory = lore.companion_memory;
        self.visited_biomes = lore.visited_biomes;
        self.discovered_structures = lore.discovered_structures;
        self.day_index = lore.day_index;
        self.recipe_book = lore.recipe_book.clone();
        self.craft_queue = lore.craft_queue.clone();
        self.onboarding = lore.onboarding;
        self.companion_cooldowns = vec![0.0; self.companions.len()];
        self.companion_line_timers = vec![8.0; self.companions.len()];
        self.companion_menu = None;
        self.settled_markers.clear();
        self.road_cells.clear();
        self.ember_cells.clear();
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
                    storage.load_chunk(cx, cz).unwrap_or_else(|| gen.generate_chunk(cx, cz))
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
        // fix: black-square artifact — the Live RT pathtracer kept the
        // previous world's voxel clip (`upload_voxels` early-returns on an
        // unchanged center) and the egui handle kept the previous world's
        // composited image, so a stale frame covered the new world after a
        // transition. Drop both; the tracer is rebuilt on the next tick.
        self.live_tracer = None;
        self.live_rt_texture = None;
        self.spawn_point = Vec3::new(0.5, self.world.surface_height(0, 0) as f32 + 0.2, 0.5);
        self.player = match storage.load_player() {
            Some(p) => Player::new(Vec3::from(p.position)).with_look(p.yaw, p.pitch),
            None => Player::new(self.spawn_point),
        };
        self.player.flying = false;
        self.storage = Some(storage);
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
        // The version-seeded preview world has no slot: nothing to save.
        let Some(storage) = &self.storage else { return };
        let dirty: Vec<(i32, i32)> = self.dirty.drain().collect();
        for pos in dirty {
            if let Some(col) = self.world.chunk(pos.0, pos.1) {
                if let Err(e) = storage.save_chunk(pos.0, pos.1, col) {
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
        if let Err(e) = storage.save_player(&player) {
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
            paths: Some(self.paths.clone()),
            faction_standing: Some(self.standings.clone()),
            companions: self.companions.clone(),
            companion_memory: self.companion_memory.clone(),
            visited_biomes: self.visited_biomes.iter().cloned().collect(),
            discovered_structures: self.discovered_structures.clone(),
            day_index: self.day_index,
            recipe_book: Some(self.recipe_book.clone()),
            craft_queue: self.craft_queue.clone(),
            kingdoms: self.kingdoms.clone(),
            onboarding: Some(self.onboarding.clone()),
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
            created_secs: self.slot_meta.created_secs,
            difficulty: self.difficulty,
            game_mode: self.game_mode,
            version_created: if self.slot_meta.version_created.is_empty() {
                env!("CARGO_PKG_VERSION").to_string()
            } else {
                self.slot_meta.version_created.clone()
            },
        };
        slots::write_meta(&self.world_dir, &meta);
        self.capture_slot_thumbnail();
        self.slot_meta = meta;
        if let Some(storage) = &self.storage {
            let _ = storage.save_seed(self.world_seed);
        }
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
            // title_orbit now counts seconds; the orbit period lives in
            // lf_worldgen::preview (B2: one scenic lap every 90s).
            self.title_orbit += dt;
        }
        // Wheel: consume every full notch accumulated this frame BEFORE
        // the UI frame is laid out, so the highlight and the caption
        // react in the same frame the wheel moved. Fast flicks land all
        // their notches at once instead of one per frame with the rest
        // discarded (the old "heavy wheel" feel); the fractional
        // remainder stays for trackpad smoothing (loop 347).
        let steps = consume_scroll_steps(&mut self.input.scroll);
        if steps != 0 {
            self.hotbar_index = ((self.hotbar_index as i32 + steps).rem_euclid(9)) as usize;
            // no window.set_title here: a window-server round trip per
            // notch read as input lag — the 2s HUD tick refreshes it
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

        self.player.update(dt, &input, &self.world);
        self.footstep_tick(dt);
        self.splash_tick();
        // N01: the tutorial watches the real pose (displacement + look
        // travel), never synthetic input state
        self.onboarding.observe_frame(
            self.player.position.to_array(),
            self.player.yaw,
            self.player.pitch,
        );
        self.update_falling_trees(dt);
        self.survival_tick(dt);
        self.craft_queue_tick(dt);
        self.contextual_hud_tick(dt);
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
                BlockEntity::Belt(_) => {} // belt pass below
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
        // P37: machines running accrue the Engineer path (sampled on a
        // cadence; any powered machine counts once per window)
        if self.frame % 60 == 0 && granted.iter().any(|g| *g > 0.0) {
            if let Some((p, tier)) = self.paths.accrue(lf_game::paths::PathEvent::MachineRan) {
                self.chronicle_event(EventType::Discovery,
                    format!("the {} path deepens — tier {}", p.name(), tier));
            }
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

        // Step 27 belt pass: on cooldown, push the held stack into the
        // first adjacent machine input that takes it.
        {
            let keys: Vec<(i32, i32, i32)> = self.block_entities.keys().copied().collect();
            for k in keys.clone() {
                let (stack, mut cd) = match self.block_entities.get(&k) {
                    Some(BlockEntity::Belt(b)) => (b.stack.clone(), b.cooldown),
                    _ => continue,
                };
                cd -= dt;
                if cd > 0.0 {
                    if let Some(BlockEntity::Belt(b)) = self.block_entities.get_mut(&k) {
                        b.cooldown = cd;
                    }
                    continue;
                }
                let Some(stack) = stack else { continue };
                let neighbors = [
                    (k.0 + 1, k.1, k.2), (k.0 - 1, k.1, k.2),
                    (k.0, k.1, k.2 + 1), (k.0, k.1, k.2 - 1),
                ];
                let mut moved = false;
                for n in &neighbors {
                    let target = self.block_entities.get_mut(n);
                    let r = match target {
                        Some(BlockEntity::Furnace(f)) => Some(lf_game::machines::BlockEntityRef::Furnace(f)),
                        Some(BlockEntity::ElectricFurnace(f)) => Some(lf_game::machines::BlockEntityRef::ElectricFurnace(f)),
                        Some(BlockEntity::Crusher(cr)) => Some(lf_game::machines::BlockEntityRef::Crusher(cr)),
                        Some(BlockEntity::Assembler(a)) => Some(lf_game::machines::BlockEntityRef::Assembler(a)),
                        Some(BlockEntity::Boiler(b)) => Some(lf_game::machines::BlockEntityRef::Boiler(b)),
                        _ => None,
                    };
                    if let Some(mut r) = r {
                        let belt = lf_game::machines::Belt { stack: Some(stack.clone()), cooldown: 0.0 };
                        if lf_game::machines::belt_push(&belt, &mut r) {
                            moved = true;
                            break;
                        }
                    }
                }
                if let Some(BlockEntity::Belt(b)) = self.block_entities.get_mut(&k) {
                    if moved {
                        // consume one from the held stack
                        if stack.count > 1 {
                            b.stack = Some(ItemStack { count: stack.count - 1, ..stack });
                        } else {
                            b.stack = None;
                        }
                        b.cooldown = lf_game::machines::BELT_CD;
                    } else {
                        b.cooldown = lf_game::machines::BELT_CD;
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
        // lore-and-visuals: companions, world tags, day/wage rollover,
        // faction NPC settling, ambient embers, HUD pulse decay.
        if playing {
            self.update_companions(dt);
            self.quest_tag_checks();
            self.ambient_ember_particles(dt);
            self.faction_pulse = (self.faction_pulse - dt * 1.4).max(0.0);
            let now_fraction = self.time.fraction();
            if self.prev_day_fraction < 0.2 && now_fraction >= 0.2 {
                self.on_day_rollover();
            }
            self.prev_day_fraction = now_fraction;
            if self.frame % 60 == 0 {
                self.try_settle_faction_npcs();
                self.try_settle_kingdoms();
                self.sync_map_faction_data();
            }
            // kingdom compass: the worldgen query costs a region scan, so
            // refresh the readout at most every 20 frames while held
            let compass_held = self.inventory.slots[self.hotbar_index].as_ref()
                .map(|s| s.item_id == "kingdom_compass")
                .unwrap_or(false);
            if compass_held && self.kingdom_compass_age >= 20 {
                self.kingdom_compass_state = self.kingdom_compass_readout();
                self.kingdom_compass_age = 0;
            } else {
                self.kingdom_compass_age = self.kingdom_compass_age.saturating_add(1);
            }
        }
        // GMod-style prop carry: hold RMB while aiming at an item prop to
        // pin it to the view ray; release to drop/throw it. Runs before the
        // bow charger and the interact/place chain and suppresses both for
        // that press (and the whole hold).
        if playing {
            let eye = self.player.eye_position();
            let look = self.player.look_dir();
            if let Some(id) = self.carried_drop {
                if self.drops.iter().any(|d| d.id == id) {
                    if self.input.place_pressed {
                        // still held this frame — refresh the grab distance
                        // with the wheel-free default (scroll could pinch)
                        if let Some(d) = self.drops.iter().find(|d| d.id == id) {
                            self.carry_dist =
                                ((d.body.position - eye).length()).clamp(1.4, 5.5);
                        }
                        self.input.place_pressed = false;
                    } else {
                        // released: the prop keeps its spring momentum plus
                        // a small flick along the look direction
                        if let Some(d) = self.drops.iter_mut().find(|d| d.id == id) {
                            d.body.held = false;
                            d.body.rest = false;
                            d.body.velocity = d.body.velocity * 0.6 + look * 2.5;
                        }
                        self.carried_drop = None;
                    }
                } else {
                    self.carried_drop = None;
                }
            } else if self.input.place_pressed {
                // press while aiming at a prop within reach grabs it (a
                // solid block in between blocks the grab — no pulling
                // through walls)
                let wall_hit = raycast_voxel(eye, look, 6.0, |pos| {
                    registry::is_targetable(self.world.get_block(pos.x, pos.y, pos.z))
                })
                .map(|(pos, _)| (Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32) - eye).length());
                let mut grab: Option<(u64, f32)> = None;
                for d in &self.drops {
                    if d.age < 0.1 {
                        continue;
                    }
                    let half = lf_game::props::prop_half(d.stack.count);
                    let to = d.body.position - eye;
                    let t = to.dot(look);
                    if t < 0.5 || t > 6.0 {
                        continue;
                    }
                    if wall_hit.map(|wt| wt < t - half).unwrap_or(false) {
                        continue; // a wall stands between the player and the prop
                    }
                    if (eye + look * t - d.body.position).length() < half + 0.3 {
                        if grab.map(|(_, gt)| t < gt).unwrap_or(true) {
                            grab = Some((d.id, t));
                        }
                    }
                }
                if let Some((id, dist)) = grab {
                    self.carried_drop = Some(id);
                    self.carry_dist = dist.clamp(1.4, 5.5);
                    if let Some(d) = self.drops.iter_mut().find(|d| d.id == id) {
                        d.body.held = true;
                        d.body.rest = false;
                    }
                    self.input.place_pressed = false;
                    self.push_hint("carrying — release to drop, walk close to pocket");
                }
            }
        }
        // bow: hold RMB to charge (not while carrying a prop)
        if playing && self.input.place_pressed && self.carried_drop.is_none() {
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
                self.play_sfx(lf_audio::Sfx::BowShoot, 0.8);
            }
        }
        if let Some(n) = &mut self.net {
            n.send_state(self.player.position.to_array(), self.player.yaw, self.player.pitch);
            for msg in n.poll() {
                match msg {
                    lf_protocol::ServerMessage::BlockUpdate { x, y, z, block } => {
                        if self.world.set_block(x, y, z, BlockState(block)).is_some() {
                            self.remesh_around(x, z);
                        }
                    }
                    // P37: escrowed trades deliver to the inventory
                    lf_protocol::ServerMessage::TradeResolved { accepted, items, .. } => {
                        if accepted {
                            for (id, count) in items {
                                let leftover = self.inventory.add_item(&id, count);
                                if leftover > 0 {
                                    self.spawn_drop(&id, leftover, self.player.eye_position());
                                }
                            }
                            self.push_hint("trade completed");
                        } else {
                            self.push_hint("trade cancelled");
                        }
                    }
                    lf_protocol::ServerMessage::TradeOffered { from_name, give, .. } => {
                        self.push_hint(&format!(
                            "{} offers {}x{} — type /tradeaccept to accept",
                            from_name, give.first().map(|(i, _)| i.clone()).unwrap_or_default(),
                            give.first().map(|(_, n)| *n).unwrap_or(0)));
                    }
                    _ => {}
                }
            }
        }

        if let Some((_, t)) = &mut self.chronicle_toast {
            *t -= dt;
            if *t <= 0.0 {
                self.chronicle_toast = None;
            }
        }
        self.elapsed += dt;
        self.last_dt = dt;

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

        // Targeting (water is not targetable). Shape-aware: the pick
        // raycast tests `pick_boxes`, so the crosshair lands on the slab's
        // solid half, the torch's stick, or through a flower's gaps —
        // and the wireframe traces the same shape (loop 347).
        let eye = self.player.eye_position();
        let look = self.player.look_dir();
        let target = raycast_voxel_boxes(eye, look, REACH, |pos| {
            self.world.get_block(pos.x, pos.y, pos.z)
        });
        self.look_target = target.map(|(pos, _)| {
            (pos, self.world.get_block(pos.x, pos.y, pos.z).id())
        });
        self.outline.set_target(&self.device, target.map(|(pos, _)| {
            let state = self.world.get_block(pos.x, pos.y, pos.z);
            ((pos.x, pos.y, pos.z), registry::pick_boxes(state))
        }));

        // Attacking: LMB on a mob (sphere test along the look ray).
        if playing && self.input.break_pressed {
            if let Some(mob_hit) = self.mob_in_crosshair() {
                if self.attack_cooldown <= 0.0 {
                    self.attack_cooldown = 0.5;
                    self.play_sfx(lf_audio::Sfx::MeleeSwing, 0.5);
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
                    self.play_sfx(lf_audio::Sfx::MobHit, 0.9);
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
                        if kind == MobType::Dragon {
                            self.chronicle_event(EventType::BossSlain,
                                "the dragon of the peaks falls — the saga turns a page".into());
                        }
                        if matches!(kind, MobType::Dragon | MobType::NullKnight) {
                            let _ = self.paths.accrue(lf_game::paths::PathEvent::BossSlain);
                        }
                        let kind_name = factions::mob_kind_id(kind).to_string();
                        self.quest_event(QuestEvent::Killed(kind_name.clone()));
                        tracing::info!("killed a {:?}", kind);
                        // the corpse topples and rests before removal
                        self.mobs[mob_hit].begin_death();
                        self.play_sfx(lf_audio::Sfx::MobDeath, 0.9);
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
                // creative (loop 329): any block pops in one hit
                if self.game_mode.instant_mining() {
                    total = 0.0;
                }
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
                            // loop 330: breaking a trunk fells the whole tree
                            if registry::is_log(block_id) {
                                self.try_fell_tree(pos);
                            }
                            self.break_block_drops(block_id, pos);
                            self.play_block_sound(block_id, lf_audio::Action::Break);
                            self.break_impulse(block_id);
                            if self.settings.particles {
                                self.spawn_break_particles(block_id, (pos.x, pos.y, pos.z), 16);
                            }
                            self.use_durability();
                            // lore-and-visuals: quest Break events + the
                            // destroy-structure-block standing penalty
                            {
                                // quest targets use the item-id form (accord_pillar)
                                let broke_key = lf_game::items::block_drop(block_id)
                                    .unwrap_or_else(|| registry::block::name(block_id).to_string());
                                self.quest_event(QuestEvent::Broke(broke_key));
                                if let Some(faction) = factions::faction_of_block(block_id) {
                                    let penalty = self.lore_data.standing_events.destroy_structure_block;
                                    self.add_standing(&faction, penalty, "destroyed their structure");
                                    // C3: nearby same-faction NPCs call it out
                                    let here = [pos.x as f32, pos.y as f32, pos.z as f32];
                                    let reactor = self.villagers.iter().find(|v| {
                                        v.faction.as_deref() == Some(faction)
                                            && (glam::Vec3::from(v.position) - glam::Vec3::from(here)).length() < 24.0
                                    }).map(|v| v.name.clone());
                                    if let Some(name) = reactor {
                                        let line = lf_npc::reaction_line(&name,
                                            &lf_npc::NpcReactionEvent::BlockBrokenInStructure);
                                        self.push_hint(&line);
                                    }
                                }
                            }
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
                            if l > self.xp_level {
                                self.play_sfx(lf_audio::Sfx::Xp, 0.8);
                            }
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
                                    BlockEntity::Belt(b) => vec![b.stack],
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
                            // F3: era unlocks surface their recipes in the
                            // workbench — count what just came into view
                            let known = workbench::catalog_pairs().into_iter()
                                .filter(|(out, _)| {
                                    let req = lf_game::research::Era::required_for(out);
                                    req != lf_game::research::Era::Primitive && req == next
                                })
                                .count();
                            if known > 0 {
                                let plural = if known == 1 { "recipe" } else { "recipes" };
                                self.chronicle_toast = Some((
                                    format!("Recipes unlocked: {} new {}", known, plural), 4.0));
                            }
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
                        self.play_sfx(lf_audio::Sfx::ChestOpen, 0.8);
                    }
                    _ => {}
                }
            }
            if self.ui_open == UiOpen::None {
                // lore-and-visuals B3: an active companion in the
                // crosshair opens the command menu instead
                if let Some(ci) = self.companion_in_crosshair() {
                    self.companion_menu = Some(ci);
                    self.ui_open = UiOpen::CompanionMenu;
                    self.unlock_cursor();
                    return;
                }
                // villager in the crosshair? trade instead — faction NPCs
                // greet through the dialogue layer first (A4/D1)
                if let Some(vi) = self.villager_in_crosshair() {
                    let vinfo = self.villagers.get(vi).map(|v| {
                        (v.name.clone(), v.faction.clone(), v.archetype.clone(), v.activity, v.memory.clone())
                    });
                    let Some((vname, vfaction, archetype, vactivity, vmem)) = vinfo else {
                        return;
                    };
                    // king-quest: SNEAK-use is the oath/collect gesture
                    if self.input.held(self.keymap.key(crate::input::Action::Sneak)) {
                        let is_vassal = self.villagers.get(vi).map(|v| v.vassal.is_some()).unwrap_or(false);
                        if is_vassal {
                            // the liege collects the vassal's stacked work
                            let take = self.villagers.get_mut(vi)
                                .map(|v| lf_npc::vassals::collect(v.vassal.as_mut().unwrap()));
                            if let Some(stock) = take {
                                let mut total = 0;
                                for (item, count) in stock {
                                    total += count as u32;
                                    let leftover = self.inventory.add_item(&item, count as u8);
                                    if leftover > 0 {
                                        self.spawn_drop(&item, leftover, self.player.eye_position());
                                    }
                                }
                                if total > 0 {
                                    self.push_hint(&format!("[{}]: Your share, my liege — {} goods delivered.", vname, total));
                                } else {
                                    self.push_hint(&format!("[{}]: Nothing stockpiled yet — give me a day.", vname));
                                }
                            }
                            return;
                        }
                        let standing = vfaction.as_deref().map(|f| self.standings.get(f)).unwrap_or(0);
                        if lf_npc::vassals::can_recruit(standing, false) {
                            let job = self.villagers.get(vi).map(|v| v.job).unwrap_or(VillagerJob::Trader);
                            let day = self.day_index as u32;
                            if let Some(v) = self.villagers.get_mut(vi) {
                                v.vassal = Some(lf_npc::vassals::recruit(job, day));
                            }
                            self.push_hint(&format!(
                                "[{}]: I kneel. From today my work is yours, my liege.", vname));
                        } else {
                            self.push_hint(&format!(
                                "[{}]: I serve no crown but my own. Earn our trust first. ({}/75 standing)", vname, standing));
                        }
                        return;
                    }
                    // C3: holding an item gifts it (one from the stack)
                    let held = self.inventory.slots[self.hotbar_index].clone();
                    if let Some(stack) = held {
                        let slot = &mut self.inventory.slots[self.hotbar_index];
                        match slot {
                            Some(st) if st.count > 1 => st.count -= 1,
                            _ => *slot = None,
                        }
                        if let Some(faction) = vfaction.as_deref() {
                            self.add_standing(faction, 2, "a gift well received");
                        }
                        let line = lf_npc::reaction_line(&vname,
                            &lf_npc::NpcReactionEvent::GiftedItem { item_id: stack.item_id.clone() });
                        self.push_hint(&line);
                        if let Some(v) = self.villagers.get_mut(vi) {
                            v.record_interaction(lf_npc::NpcEvent::Gifted, self.day_index as u32);
                        }
                        return;
                    }
                    // C2: sleeping NPCs only murmur; no trade, no quests
                    if vactivity == lf_npc::NpcActivityState::Sleeping {
                        self.push_hint(&format!("[{}]: {}", vname,
                            lf_npc::activity_opening(vactivity)));
                        return;
                    }
                    // C3: +75 acknowledgement (once per faction crossing)
                    if let Some(faction) = vfaction.as_deref() {
                        if self.honored_ack.remove(faction) {
                            let title = self.lore_data.faction(faction)
                                .map(|f| lf_lore::StandingBand::Honored.title(f).to_string())
                                .unwrap_or_else(|| "honored".into());
                            self.push_hint(&lf_npc::reaction_line(&vname,
                                &lf_npc::NpcReactionEvent::FactionHonored { title }));
                        }
                    }
                    // C4: fresh memory colors the greeting
                    if let Some(ev) = vmem.recall(self.day_index as u32) {
                        if let Some(line) = lf_npc::memory_greeting(ev) {
                            self.push_hint(&format!("[{}]: {}", vname, line));
                        }
                    }
                    if let Some(archetype) = archetype {
                        if let Some((line, close)) = self.npc_interact(&archetype) {
                            self.push_hint(&line);
                            if close {
                                // hostile standing: the door stays shut
                                return;
                            }
                        }
                        if let Some(v) = self.villagers.get_mut(vi) {
                            v.record_interaction(lf_npc::NpcEvent::QuestGiven, self.day_index as u32);
                        }
                    }
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
                // P36 mount: bare-hand right-click a dragon to ride it.
                if held.is_none() {
                    if let Some(mi) = self.mob_in_crosshair() {
                        if let Some(mob) = self.mobs.get(mi) {
                            if mob.mob_type == MobType::Dragon {
                                self.mounted_dragon = Some(mob.id);
                                self.push_hint("you take the saddle — sneak to dismount");
                                self.play_sfx(lf_audio::Sfx::DragonRoar, 1.0);
                                return;
                            }
                        }
                    }
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
                        // (P36 mount lives in the held.is_none() arm above)
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
                            self.play_sfx(lf_audio::Sfx::Eat, 0.9);
                        }
                        Some(ItemKind::Block(b)) => {
                            if let Some((pos, normal)) = target {
                                let mut place = pos + normal;
                                // P37: the generalized gate is enforced at
                                // PLACEMENT too (it was UI-only before)
                                let gate = lf_game::paths::gate_for(&stack.item_id);
                                if !gate.passes(&self.research, &self.paths) {
                                    self.push_hint(&gate.label());
                                    return;
                                }
                                if let Some((p, tier)) = self.paths.accrue(lf_game::paths::PathEvent::BlockPlaced) {
                                    self.chronicle_event(EventType::Discovery,
                                        format!("the {} path deepens — tier {}", p.name(), tier));
                                }
                                // Loop 343 building HUD: honor the picked
                                // shape (slab/stairs variants of the held
                                // block; a slab onto a matching bottom slab
                                // still merges into a full cube)
                                let mut final_state = lf_game::items::build_shape_state(
                                    BlockState(b), self.build_shape, self.player.yaw,
                                ).unwrap_or(BlockState(b));
                                if self.build_shape == lf_game::items::BuildShape::Slab {
                                    let existing = self.world.get_block(pos.x, pos.y, pos.z);
                                    if let Some(merged) = lf_game::items::slab_merge(existing, final_state) {
                                        final_state = merged;
                                        place = pos; // merge fills the aimed cell
                                    }
                                }
                                if !self.block_intersects_player(place) {
                                    if self.world.set_block(place.x, place.y, place.z, final_state).is_some() {
                                        // lore-and-visuals: quest Place events
                                        // (targets use the item-id form)
                                        let placed_item = stack.item_id.clone();
                                        self.onboarding.observe_placed(registry::is_solid(final_state));
                                        self.quest_event(QuestEvent::Placed(placed_item));
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
        if self.frame == 600 {

        }
        let (sv, si) = lf_engine::atmosphere::sky_bodies(eye, self.time.fraction());
        if self.frame == 600 {
            tracing::warn!("BLACKBOX DEBUG: sky_vertices={} first={:?}",
                sv.len(), sv.first().map(|v| v.position));
        }
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
        if let Some((p, tier)) = self.paths.accrue(lf_game::paths::PathEvent::SpellCast) {
            self.chronicle_event(EventType::Discovery,
                format!("the {} path deepens — tier {}", p.name(), tier));
        }
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
                if mob.death_t.is_some() {
                    continue; // projectiles pass over corpses
                }
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
                if killed && self.mobs[mi].mob_type == MobType::Dragon {
                    self.chronicle_event(EventType::BossSlain,
                        "the dragon of the peaks falls — the saga turns a page".into());
                }
                if killed {
                    let (kind, pos) = (self.mobs[mi].mob_type, self.mobs[mi].position);
                    for (item, n) in kind.drops() {
                        self.spawn_drop(item, *n, pos + Vec3::new(0.0, 0.5, 0.0));
                    }
                    self.kills += 1;
                    let (l, pr) = lf_game::combat::grant_xp(self.xp_level, self.xp_progress, 5);
                    if l > self.xp_level {
                        self.play_sfx(lf_audio::Sfx::Xp, 0.8);
                    }
                    self.xp_level = l;
                    self.xp_progress = pr;
                    self.xp_flash = 1.0;
                    // firebolt kills now play the same death animation
                    // (they previously left an immortal corpse ticking)
                    self.mobs[mi].begin_death();
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
        // creative (loop 329): the inventory is infinite
        if !self.game_mode.consumes_items() {
            return;
        }
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
        let id = self.next_drop_id;
        self.next_drop_id += 1;
        // pop out with a deterministic sideways flick so mined items
        // scatter instead of stacking into a pillar
        let h = id.wrapping_mul(0x9E3779B97F4A7C15);
        let a = ((h >> 16) % 628) as f32 / 100.0 - 3.14;
        let velocity = Vec3::new(a.sin() * 0.9, 2.4, a.cos() * 0.9);
        let body = lf_game::props::PropBody::new(pos, velocity, faller_tumble_axis(id));
        self.drops.push(ItemDrop {
            stack: ItemStack { item_id: item.to_string(), count },
            id,
            body,
            age: 0.0,
        });
    }

    /// Hunger drain, regen, fall damage, drowning, death.
    /// N02: seconds of play between craft-queue job completions.
    const CRAFT_JOB_SECONDS: f32 = 1.25;

    /// N02: the craft queue is REAL — entries hold (output, qty), nothing
    /// is reserved at enqueue, and every job completion runs the full
    /// transactional engine against the live inventory. Documented rule:
    /// queueing reserves nothing; a job consumes exactly at completion;
    /// a blocked job (missing materials / no room) waits and shows its
    /// reason; cancel is free because nothing was consumed. The queue
    /// advances during play and while the workbench is open — not behind
    /// pause or menus that own the world.
    fn craft_queue_tick(&mut self, dt: f32) {
        if self.craft_queue.is_empty() {
            self.craft_queue_timer = 0.0;
            return;
        }
        let active = self.stats.health > 0.0
            && matches!(self.ui_open, UiOpen::None | UiOpen::Chat | UiOpen::HandCraft | UiOpen::CraftingTable);
        if !active {
            return;
        }
        self.craft_queue_timer -= dt;
        if self.craft_queue_timer > 0.0 {
            return;
        }
        self.craft_queue_timer = Self::CRAFT_JOB_SECONDS;
        let (output, qty) = self.craft_queue[0].clone();
        let Some((ingredients, output_count)) = crate::ui::catalog_craft_entry(&output) else {
            // the recipe vanished (mod unloaded / catalog rename): drop the
            // job honestly instead of spinning forever
            self.craft_queue.remove(0);
            self.push_hint(&format!("queue: recipe for {} is gone — job dropped", output));
            return;
        };
        match lf_game::crafting::execute(&mut self.inventory, &ingredients, &output, output_count, qty) {
            lf_game::crafting::CraftOutcome::Crafted { granted, .. } => {
                self.craft_queue.remove(0);
                // exactly one event set per completed job — same as a click
                self.quest_event(QuestEvent::Crafted(output.clone()));
                self.onboarding.observe_crafted();
                self.play_sfx(lf_audio::Sfx::CraftDone, 0.7);
                self.push_hint(&format!("queue delivered: {} × {}", granted, output));
            }
            lf_game::crafting::CraftOutcome::Blocked(_) => {
                // the queue strip shows the live reason; retry next tick
            }
        }
    }

    /// N04: drive the contextual HUD channels — transient fades, a
    /// throttled threat scan (hostiles with line of sight), and the
    /// settlement entry banner (fires once per kingdom per session).
    fn contextual_hud_tick(&mut self, dt: f32) {
        self.hud_channels.tick(dt);
        if self.stats.health <= 0.0 {
            return;
        }
        // threat scan: near hostiles the player can be seen by
        if self.frame % 15 == 0 {
            let p = self.player.position;
            let mut n = 0u8;
            for m in &self.mobs {
                if m.mob_type.is_hostile() && m.health > 0.0
                    && (m.position - p).length() < 14.0
                    && lf_game::mobs::has_line_of_sight(m.position + Vec3::new(0.0, 1.0, 0.0),
                        p + Vec3::new(0.0, 1.0, 0.0), &self.world)
                {
                    n += 1;
                    if n >= 9 { break; }
                }
            }
            self.threat_count = n;
        }
        // settlement entry: the banner announces a kingdom once
        if self.frame % 30 == 0 {
            if let Some((name, _bearing, meters)) = self.kingdom_compass_readout() {
                if meters <= 90 && self.settlement_seen.insert(name.clone()) {
                    let gates_barred = self.standings.refuses_trade("accord");
                    let state_line = if gates_barred {
                        "the gates are barred to you".to_string()
                    } else {
                        "a safe road in".to_string()
                    };
                    self.hud_channels.enter_settlement(name, state_line);
                }
            }
        }
    }

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
        // Hunger drains slowly; the difficulty sets the pace (Hard is
        // stricter, Peaceful doesn't starve you). Creative doesn't eat.
        if self.game_mode.drains_hunger() && now >= self.next_hunger_tick {
            let rate = self.difficulty.hunger_rate();
            if rate > 0.0 {
                self.next_hunger_tick = now + Duration::from_secs((45.0 / rate) as u64);
                self.stats.hunger = (self.stats.hunger - 1.0).max(0.0);
            } else {
                self.next_hunger_tick = now + Duration::from_secs(45);
            }
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
        // creative (loop 329): nothing ever hurts
        if !self.game_mode.takes_damage() {
            return;
        }
        if self.ward_timer > 0.0 {
            // the ward drinks it (P33); the timer still runs down in tick
            self.hud_flash = 1.0;
            return;
        }
        self.play_sfx(lf_audio::Sfx::Hurt, 0.9);
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
            self.play_sfx(lf_audio::Sfx::PlayerDeath, 1.0);
            tracing::info!("player died");
        }
    }

    /// Feed a gameplay event into quests (and log chronicle milestones).
    pub fn quest_event(&mut self, event: QuestEvent) {
        let finished = self.quest_log.record_event(&event);
        for id in finished {
            let quest = self.quest_log.quests.iter().find(|q| q.id == id).cloned();
            let title = quest.as_ref().map(|q| q.title.clone()).unwrap_or_default();
            tracing::info!("quest complete: {}", title);
            self.chronicle_event(EventType::ActCompleted, format!("completed quest '{}'", title));
            // A4: faction quests move standing (issuing faction + ripples)
            if let Some(q) = quest {
                self.apply_quest_standing(&q);
            }
        }
    }

    pub fn chronicle_event(&mut self, event_type: EventType, payload: String) {
        // Steps 21-22: the saga is visible while playing, not only in the J log
        self.chronicle_toast = Some((format!("✦ {}", payload), 4.0));
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
        let mut stuck = false; // an arrow hit terrain (sound played after: the loop borrows self.arrows)
        for (i, arrow) in self.arrows.iter_mut().enumerate() {
            let before = arrow.position;
            let done = arrow.update(dt, |x, y, z| self.world.is_solid(x, y, z));
            if done {
                stuck = true;
                remove.push(i);
                continue;
            }
            // mob hit test along the step
            let dir = arrow.position - before;
            let step = dir.length().max(0.001);
            let dir = dir / step;
            for (mi, mob) in self.mobs.iter().enumerate() {
                if mob.death_t.is_some() {
                    continue; // projectiles pass over corpses
                }
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
        if stuck {
            self.play_sfx(lf_audio::Sfx::ArrowHit, 0.8);
        }
        let hit_mobs = !events.is_empty();
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
                    self.mobs[mi].begin_death();
                    self.play_sfx(lf_audio::Sfx::MobDeath, 0.9);
                }
            }
        }
        if hit_mobs {
            self.play_sfx(lf_audio::Sfx::MobHit, 0.85);
        }
    }

    /// C1: nearest furnace around a spawn point — the workstation anchor
    /// NPCs walk to during the Work slot (None → they work at home).
    fn scan_workstation(&self, center: [f32; 3], radius: i32) -> Option<[i32; 3]> {
        let (cx, cy, cz) = (center[0] as i32, center[1] as i32, center[2] as i32);
        let mut best = None;
        let mut best_d = i32::MAX;
        for dx in -radius..=radius {
            for dy in -6..=8 {
                for dz in -radius..=radius {
                    let (x, y, z) = (cx + dx, cy + dy, cz + dz);
                    if self.world.get_block(x, y, z).id() == registry::block::FURNACE {
                        let d = dx * dx + dy * dy + dz * dz;
                        if d < best_d {
                            best_d = d;
                            best = Some([x, y, z]);
                        }
                    }
                }
            }
        }
        best
    }

    /// Villagers follow the enriched day schedule and REALLY walk: the
    /// locomotion module (loop 345) steps up bumps, walks down slopes,
    /// refuses cliffs, falls with gravity, and sidesteps around obstacles
    /// — the old loop froze any NPC whose next cell was not perfectly
    /// flat ground.
    fn update_villagers(&mut self, dt: f32) {
        if self.stats.health <= 0.0 {
            return;
        }
        let day_fraction = self.time.fraction() as f32;
        let now_ticks = self.time.ticks;
        let frame = self.frame;
        let world = &self.world;
        let player_pos = self.player.position;
        let table = lf_npc::default_schedule_entries();
        let solid = |x: i32, y: i32, z: i32| world.is_solid(x, y, z);
        for (i, villager) in self.villagers.iter_mut().enumerate() {
            // C1: the enriched day (sleep/eat/work/socialize/return) picks
            // the activity; the workstation anchor is resolved at spawn
            let entry = lf_npc::enriched_slot_at(&table, day_fraction);
            let home = villager.schedule.location;
            let target: [f32; 3] = match (entry.activity, villager.workstation_pos) {
                (lf_npc::ScheduleSlot::Work, Some([wx, wy, wz])) => {
                    [wx as f32 + 0.5, wy as f32 + 0.2, wz as f32 + 0.5]
                }
                _ => home,
            };
            let pos = glam::Vec3::from(villager.position);
            let flat = (glam::Vec3::new(target[0], pos.y, target[2]) - pos).length();
            let panicking = villager.flee_until_ticks > now_ticks;
            // deterministic per-villager wander seed
            let t = frame as u64 / 90; // re-pick the wander point ~every 1.5s
            let seed = villager.id.wrapping_mul(2654435761)
                .wrapping_add(t.wrapping_mul(0x9E3779B9))
                .wrapping_add(i as u64);
            let speed = if panicking { 2.4 } else { 1.2 };
            // where this NPC wants to walk right now, and whether it wants
            // to walk at all
            let mut wish: Option<(f32, f32)> = None; // (bearing, speed)
            if panicking {
                // run directly away from the player
                let away = pos - glam::Vec3::new(player_pos.x, pos.y, player_pos.z);
                if away.length_squared() > 0.01 {
                    wish = Some(((away.x).atan2(away.z), speed));
                }
                villager.activity = lf_npc::NpcActivityState::Walking;
            } else {
                villager.activity = lf_npc::activity_state_for(&entry, flat > 1.5);
                use lf_npc::NpcActivityState as Act;
                match villager.activity {
                    Act::Walking => {
                        if flat > 1.5 {
                            wish = Some(((target[0] - pos.x).atan2(target[2] - pos.z), speed));
                        }
                    }
                    Act::Idle | Act::Socializing => {
                        // idle wander: a fresh point near home every ~1.5s.
                        // The radius stays under the 1.5 en-route threshold
                        // so shuffling never flips the NPC back to "walking
                        // home" (which would ping-pong).
                        let a = (seed % 360) as f32 / 57.3;
                        let r = 0.6 + (seed % 9) as f32 * 0.1; // 0.6..1.4
                        let wx = home[0] + a.cos() * r;
                        let wz = home[2] + a.sin() * r;
                        let d = ((wx - pos.x) * (wx - pos.x) + (wz - pos.z) * (wz - pos.z)).sqrt();
                        if d > 0.5 {
                            wish = Some(((wx - pos.x).atan2(wz - pos.z), speed * 0.75));
                        }
                    }
                    _ => {}
                }
                // guards on patrol circuit the block even while "home"
                if entry.activity == lf_npc::ScheduleSlot::Patrol
                    && villager.job == VillagerJob::Guard
                {
                    let corner = ((frame as u64 / 140) % 4) as f32; // new post every 7s
                    let a = corner * std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4;
                    let px = home[0] + a.cos() * 5.0;
                    let pz = home[2] + a.sin() * 5.0;
                    let d = ((px - pos.x) * (px - pos.x) + (pz - pos.z) * (pz - pos.z)).sqrt();
                    if d > 1.2 {
                        wish = Some(((px - pos.x).atan2(pz - pos.z), speed));
                        villager.activity = lf_npc::NpcActivityState::Walking;
                    }
                }
            }
            // one honest locomotion tick: gravity first, then the step
            let bearing = wish.map(|(yaw, _)| yaw).unwrap_or(villager.yaw);
            let outcome = lf_npc::locomotion::tick(
                &mut villager.loco, &mut villager.position, wish.map(|(yaw, sp)| (yaw, sp)),
                dt, now_ticks, &solid,
            );
            // face the heading actually walked (sidesteps included)
            let eff_yaw = villager.loco.heading(bearing, now_ticks);
            let mut delta = eff_yaw - villager.yaw;
            while delta > std::f32::consts::PI {
                delta -= std::f32::consts::TAU;
            }
            while delta < -std::f32::consts::PI {
                delta += std::f32::consts::PI;
            }
            villager.yaw += delta.clamp(-9.0 * dt, 9.0 * dt);
            let moving = wish.is_some() && outcome == lf_npc::Move::Stepped;
            if moving {
                villager.walk_phase += dt * wish.map(|(_, sp)| sp).unwrap_or(speed) * 4.4;
                villager.walk_amp = (villager.walk_amp + dt * 10.0).min(1.0);
            } else {
                villager.walk_amp = (villager.walk_amp - dt * 6.0).max(0.0);
            }
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
                    let mut wiz = Villager::new(id, VillagerJob::Wizard, "Ysolde".into(),
                        [spawn.0 as f32 + 0.5, spawn.1 as f32 + 0.2, spawn.2 as f32 + 0.5]);
                    wiz.schedule.location = wiz.position;
                    self.villagers.push(wiz);
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
                VillagerJob::Monarch => "Aldric",
            };
            let spawn = if self.world.is_solid(hx, hy - 1, hz + 2) { (hx, hy, hz + 2) } else { (hx, hy, hz) };
            let mut v = Villager::new(id, job, name.to_string(),
                [spawn.0 as f32 + 0.5, spawn.1 as f32 + 0.2, spawn.2 as f32 + 0.5]);
            // loop 345 fix: the default schedule anchored every hamlet
            // villager at the world origin (8, 64, 8) — they beelined
            // hundreds of blocks to (0,0) and looked frozen en route. Home
            // is the hamlet they settled.
            v.schedule.location = v.position;
            v.loco.side_bias = if id % 2 == 0 { 1.0 } else { -1.0 };
            v.workstation_pos = self.scan_workstation(v.position, 12);
            self.villagers.push(v);
            tracing::info!("villager {} the {:?} settled a hamlet", name, job);
            return; // one per check
        }
    }

    /// P36: one dragon settles each roost (marker = a dragon egg), max
    /// two alive at once, flying the ring above the clutch.
    fn try_settle_dragons(&mut self) {
        if self.frame % 180 != 0 {
            return;
        }
        let dragons = self.mobs.iter()
            .filter(|m| m.mob_type == lf_game::mobs::MobType::Dragon).count();
        if dragons >= 2 || dragons == 0 && self.mobs.len() >= 12 {
            return;
        }
        let player = self.player.position;
        for ((cx, cz), col) in self.world.chunks.iter() {
            let center = (*cx as f32 * 16.0 + 8.0, *cz as f32 * 16.0 + 8.0);
            let dist = ((center.0 - player.x).powi(2) + (center.1 - player.z).powi(2)).sqrt();
            if dist > 70.0 {
                continue;
            }
            let mut egg = None;
            'scan: for lx in 5..11usize {
                for lz in 5..11usize {
                    for y in 60..230usize {
                        if col.get(lx, y, lz).id() == registry::block::DRAGON_EGG {
                            egg = Some((cx * 16 + lx as i32, y as i32, cz * 16 + lz as i32));
                            break 'scan;
                        }
                    }
                }
            }
            let Some((ex, ey, ez)) = egg else { continue };
            let staffed = self.mobs.iter().any(|m| {
                m.roost.map(|r| (r[0] as i32 - ex).abs() < 16 && (r[2] as i32 - ez).abs() < 16).unwrap_or(false)
            });
            if staffed {
                continue;
            }
            let mut dragon = lf_game::mobs::MobEntity::spawn(
                self.next_mob_id,
                lf_game::mobs::MobType::Dragon,
                Vec3::new(ex as f32 + 0.5, ey as f32 + 6.0, ez as f32 + 0.5),
            );
            dragon.roost = Some([ex as f32 + 0.5, ey as f32, ez as f32 + 0.5]);
            dragon.dragon = Some(lf_game::dragons::DragonBrain {
                phase: lf_game::dragons::Phase::Circling,
                ..Default::default()
            });
            self.next_mob_id += 1;
            self.mobs.push(dragon);
            self.chronicle_event(
                EventType::Discovery,
                "wings circle the peaks — a dragon guards its clutch".into(),
            );
            return;
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

    /// An active companion roughly under the crosshair (B3 command menu).
    fn companion_in_crosshair(&self) -> Option<usize> {
        let eye = self.player.eye_position();
        let look = self.player.look_dir();
        let mut best: Option<(f32, usize)> = None;
        for (i, c) in self.companions.iter().enumerate() {
            let center = c.position + glam::Vec3::new(0.0, 0.9, 0.0);
            let to = center - eye;
            let t = to.dot(look);
            if t < 0.0 || t > REACH + 1.0 {
                continue;
            }
            let closest = eye + look * t;
            if (closest - center).length() < 1.1 {
                if best.map(|(d, _)| t < d).unwrap_or(true) {
                    best = Some((t, i));
                }
            }
        }
        best.map(|(_, i)| i)
    }

    /// Index of the nearest visible mob roughly under the crosshair,
    /// within reach (see `crosshair_mob` for the occlusion contract).
    fn mob_in_crosshair(&self) -> Option<usize> {
        crosshair_mob(
            &self.mobs,
            &self.world,
            self.player.eye_position(),
            self.player.look_dir(),
            REACH,
        )
    }

    /// Advance mob AI/physics and run the spawn/despawn cycle.
    fn update_mobs(&mut self, dt: f32) {
        if self.stats.health <= 0.0 {
            return;
        }
        let player = self.player.position;
        let world = &self.world;
        let mut damage_to_player = 0.0;
        let mut attacker_of_frame: Option<usize> = None;
        let mut breathers: Vec<glam::Vec3> = Vec::new();
        for (mi, mob) in self.mobs.iter_mut().enumerate() {
            if mob.mob_type == lf_game::mobs::MobType::Dragon && mob.roost.is_some() && mob.death_t.is_none() {
                // P36: the dragon's flight brain owns its position
                let roost = mob.roost.unwrap();
                let center = Vec3::from(roost);
                let (pos, breathing) = match &mut mob.dragon {
                    Some(brain) => brain.tick(dt, center, Some(player), center + Vec3::new(0.0, 3.0, 0.0)),
                    None => (mob.position, false),
                };
                // loop 347: the flight brain proposes a position but may
                // not phase the dragon through terrain — refuse the
                // horizontal leg into solid rock (keep the old x/z, take
                // the y if that too is clear) so dragons skim mountains
                // instead of vanishing into them
                let clear = |p: Vec3| !world.is_solid(p.x as i32, p.y as i32, p.z as i32);
                if clear(pos) {
                    mob.position = pos;
                } else {
                    let mut slide = mob.position;
                    slide.y = pos.y;
                    if clear(slide) {
                        mob.position = slide;
                    }
                }
                mob.yaw = (player.x - pos.x).atan2(-(player.z - pos.z));
                if breathing {
                    breathers.push(pos);
                }
                continue;
            }
            // B3: the player's standing with this mob's faction widens or
            // calms its aggro radius (unaffiliated mobs use standing 0)
            let standing = mob.faction_id.as_deref().map(|f| self.standings.get(f)).unwrap_or(0);
            if let Some(dmg) = mob.update_with_standing(dt, world, player, standing) {
                damage_to_player += dmg;
                attacker_of_frame = Some(mi);
            }
        }
        // B4: first-order group aggro — self-aggroed mobs pinged once this
        // frame; neighbours join with a 0.5s reaction delay (never chains)
        let pings: Vec<usize> = self
            .mobs
            .iter()
            .enumerate()
            .filter(|(_, m)| m.group_ping)
            .map(|(i, _)| i)
            .collect();
        for i in pings {
            lf_game::mobs::MobEntity::propagate_group_aggro(&mut self.mobs, i, player);
        }
        for m in &mut self.mobs {
            m.group_ping = false;
        }
        // companions remember who hit the player this frame (B4 defense)
        if let Some(mi) = attacker_of_frame {
            self.last_attacker = Some(mi);
            // N04: the hit-direction arc — absolute bearing, the painter
            // subtracts the live yaw so it stays world-true while turning
            if let Some(m) = self.mobs.get(mi) {
                let dx = m.position.x - player.x;
                let dz = m.position.z - player.z;
                self.hud_channels.note_hit_from(dx.atan2(dz));
            }
            // C3: NPCs within 24 blocks of combat bolt for a while
            let flee_until = self.time.ticks + 200; // 10s of world ticks
            for v in &mut self.villagers {
                if (glam::Vec3::from(v.position) - player).length() < 24.0 {
                    v.flee_until_ticks = flee_until;
                }
            }
        }
        if damage_to_player > 0.0 {
            let scaled = damage_to_player * self.difficulty.mob_damage();
            if scaled > 0.0 {
                self.damage(scaled);
            }
        }
        // P36: fire breath scorches the player in range
        for mouth in breathers {
            if (player - mouth).length() < lf_game::dragons::BREATH_RANGE + 1.0 {
                self.damage(6.0 * dt * 10.0);
                if self.settings.particles && self.frame % 2 == 0 {
                    let tex = lf_assets::texture_index_for_block(registry::block::LANTERN);
                    for i in 0..5u32 {
                        let a = i as f32 * 1.3 + self.elapsed * 3.0;
                        let dir = (player - mouth).normalize();
                        self.particles.push(Particle {
                            position: mouth + dir * (a % 3.0),
                            velocity: dir * 6.0,
                            life: 0.4,
                            tex,
                            uv_off: [0.0, 0.0],
                        });
                    }
                }
            }
        }
        // P36 mount: the rider sits on the bonded dragon
        if let Some(dragon_id) = self.mounted_dragon {
            if let Some(dragon) = self.mobs.iter().find(|m| m.id == dragon_id) {
                self.player.position = dragon.position + Vec3::new(0.0, 2.0, 0.0);
                self.player.velocity = Vec3::ZERO;
                if self.input.held(self.keymap.key(crate::input::Action::Sneak)) {
                    self.mounted_dragon = None;
                    self.push_hint("dismounted — the wing beats settle");
                }
            } else {
                self.mounted_dragon = None;
            }
        }
        // despawn far mobs; finished corpses leave; anything that fell out
        // of the world is gone (no immortal void-tickers)
        self.mobs.retain(|m| {
            !m.dead_and_gone()
                && m.position.y > -10.0
                && (m.position - player).length() < 80.0
        });

        // spawn cycle
        if Instant::now() >= self.next_spawn_attempt && self.mobs.len() < 12 {
            self.next_spawn_attempt = Instant::now() + Duration::from_secs(2);
            let seed = self.frame.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(0x51ed270b);
            let is_day = self.time.is_day();
            // spawn point's biome decides the fauna (Step 18)
            let (spawn_cold, spawn_nameless) = {
                let ang2 = (seed % 360) as f32 / 57.3;
                let dist2 = 20.0 + ((seed >> 9) % 20) as f32;
                let bx = (player.x + ang2.cos() * dist2) as i32;
                let bz = (player.z + ang2.sin() * dist2) as i32;
                let biome = self.map.biome_at(bx, bz);
                (biome.is_cold(), matches!(biome, lf_worldgen::Biome::PaleGarden | lf_worldgen::Biome::DarkForest))
            };
            // king-quest C: ambient animals (chickens/wolves/bears/dogs)
            let (spawn_forest, spawn_settlement) = {
                let ang2 = (seed % 360) as f32 / 57.3;
                let dist2 = 20.0 + ((seed >> 9) % 20) as f32;
                let bx = (player.x + ang2.cos() * dist2) as i32;
                let bz = (player.z + ang2.sin() * dist2) as i32;
                let biome = self.map.biome_at(bx, bz);
                let forest = matches!(biome, lf_worldgen::Biome::Forest | lf_worldgen::Biome::DarkForest | lf_worldgen::Biome::RedwoodForest
                    | lf_worldgen::Biome::Taiga | lf_worldgen::Biome::SnowyTaiga | lf_worldgen::Biome::PineBarrens | lf_worldgen::Biome::MapleForest);
                (forest, self.villagers.iter().any(|v| {
                    (glam::Vec3::from(v.position) - player).length() < 60.0
                }))
            };
            if let Some(kind) = lf_game::mobs::roll_animal_spawn(
                seed ^ 0x51ed270b, is_day, spawn_cold, spawn_forest, spawn_settlement) {
                // Peaceful keeps hostiles out entirely (C1)
                let peaceful_skip = kind.is_hostile() && self.difficulty == slots::Difficulty::Peaceful;
                if !peaceful_skip {
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

    /// Non-block one-shot (loop 329): ui click / eat / hurt / xp / step.
    fn play_sfx(&mut self, sfx: lf_audio::Sfx, volume: f32) {
        let volume = self.settings.volume_master * self.settings.volume_sfx * volume;
        if let Some(a) = &mut self.audio {
            a.play_sfx(sfx, volume);
        }
    }

    /// Footsteps (loop 329): every ~2.1 blocks of ground travel plays a
    /// soft step colored by the material underfoot. Silent in the air.
    fn footstep_tick(&mut self, dt: f32) {
        if !self.player.on_ground || self.ui_open != UiOpen::None {
            self.step_distance = 1.2; // first step lands promptly on landing
            return;
        }
        let v = self.player.velocity;
        let speed = (v.x * v.x + v.z * v.z).sqrt();
        if speed < 0.5 {
            return;
        }
        self.step_distance += speed * dt;
        if self.step_distance >= 2.1 {
            self.step_distance = 0.0;
            let (x, y, z) = (
                self.player.position.x as i32,
                (self.player.position.y - 0.6) as i32,
                self.player.position.z as i32,
            );
            let ground = self.world.get_block(x, y, z).id();
            self.play_sfx(lf_audio::Sfx::Footstep(lf_audio::block_category(ground)), 0.45);
        }
    }

    /// Loop 349: splash on the dry→wet edge (chest block enters water).
    fn splash_tick(&mut self) {
        let p = self.player.position;
        let in_water = self.world.get_block(p.x as i32, (p.y + 0.9) as i32, p.z as i32).id()
            == registry::block::WATER;
        if in_water && !self.was_in_water {
            self.play_sfx(lf_audio::Sfx::Splash, 0.8);
        }
        self.was_in_water = in_water;
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
        // loop 331: ground plants pop when their support breaks
        let above = self.world.get_block(x, y + 1, z).id();
        if registry::is_plant(above) && !registry::is_banner(above) {
            if self.world.set_block(x, y + 1, z, BlockState::AIR).is_some() {
                self.remesh_around(x, z);
                if let Some(n) = &self.net {
                    n.send_block(x, y + 1, z, registry::block::AIR);
                }
            }
        }
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
            let seed = ((x as u64) << 32) ^ ((y as u64) << 8) ^ (z as u64);
            self.falling_blocks.push(FallingBlock {
                position: Vec3::new(x as f32 + 0.5, y as f32 + 1.5, z as f32 + 0.5),
                velocity: 0.0,
                block: above,
                tumble_axis: faller_tumble_axis(seed),
                angle: 0.0,
                angvel: 1.2 + (seed % 13) as f32 / 13.0,
                bounced: false,
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
        let mut dust: Vec<Vec3> = Vec::new();
        self.falling_blocks.retain_mut(|f| {
            let cell = (f.position.x.floor() as i32, f.position.y.floor() as i32, f.position.z.floor() as i32);
            let in_water = self.world.get_block(cell.0, cell.1, cell.2).id() == registry::block::WATER;
            f.velocity += 24.0 * dt;
            if in_water {
                f.velocity = f.velocity.min(2.5); // sinks, does not rocket through the pool
            }
            f.angle = tumble_step(f.angle, f.angvel, dt);
            let new_y = f.position.y - f.velocity * dt;
            let feet_cell = (new_y - 0.5).floor() as i32;
            if feet_cell < 0 {
                return false; // fell out of the world
            }
            if self.world.is_solid(cell.0, feet_cell, cell.2) {
                // loop 330: one small bounce on a hard fast impact (dust +
                // recoil), then the settle lands for real
                match faller_landing(f.velocity, f.bounced) {
                    FallerLanding::Bounce(up) => {
                        f.bounced = true;
                        f.velocity = -up;
                        dust.push(f.position);
                        return true;
                    }
                    FallerLanding::Place => {
                        let land_y = feet_cell + 1;
                        let occupied = self.world.get_block(cell.0, land_y, cell.2);
                        if registry::is_solid(occupied) {
                            dropped_items.push(f.block);
                        } else {
                            landed.push((cell.0, land_y, cell.2, f.block));
                        }
                        return false;
                    }
                }
            }
            f.position.y = new_y;
            true
        });
        for d in dust {
            // impact puff — the block's own texture, budget-capped
            if self.settings.particles {
                self.spawn_break_particles(
                    self.falling_blocks.first().map(|f| f.block.id()).unwrap_or(registry::block::SAND),
                    (d.x as i32, d.y as i32, d.z as i32), 4);
            }
        }
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

    /// Loop 330 felling: a broken trunk removes the whole tree (trunk +
    /// canopy, each cell broadcast), spawns the falling-tree entity, and
    /// the timber creak plays. Valheim-style: what falls is everything
    /// above the cut.
    fn try_fell_tree(&mut self, stump: glam::IVec3) {
        let above = [stump.x, stump.y + 1, stump.z];
        let Some(tree) = lf_game::timber::find_tree(&self.world, above) else {
            return;
        };
        for cell in tree.trunk.iter().chain(tree.leaves.iter()) {
            if self.world.set_block(cell[0], cell[1], cell[2], BlockState::AIR).is_some() {
                if let Some(n) = &self.net {
                    n.send_block(cell[0], cell[1], cell[2], registry::block::AIR);
                }
            }
        }
        self.remesh_around(stump.x, stump.z);
        let look = self.player.look_dir();
        let dir = lf_game::timber::FallDir::from_look(look.x, look.z);
        self.falling_trees.push(FallingTree { tree, dir, angle: 0.0, angvel: 0.55 });
        self.play_sfx(lf_audio::Sfx::TreeCreak, 1.0);
    }

    /// Advance felled trees; impact applies the landing plan (horizontal
    /// log row, drops for blocked cells, canopy shatter), with crash sound
    /// and camera shake scaled to the tree.
    fn update_falling_trees(&mut self, dt: f32) {
        let mut landed: Vec<(lf_game::timber::Tree, lf_game::timber::FallDir)> = Vec::new();
        for t in &mut self.falling_trees {
            t.angvel += 1.9 * dt; // gravity torque on the leaning trunk
            t.angle += t.angvel * dt;
            if t.angle >= lf_game::timber::LAND_ANGLE {
                landed.push((t.tree.clone(), t.dir));
            }
        }
        self.falling_trees.retain(|t| t.angle < lf_game::timber::LAND_ANGLE);
        for (tree, dir) in landed {
            let is_free = |cell: [i32; 3]| {
                !registry::is_solid(self.world.get_block(cell[0], cell[1], cell[2]))
            };
            let plan = lf_game::timber::fall_plan(&tree, dir, is_free);
            for (cell, h_id) in &plan.place {
                self.apply_sim_edit(cell[0], cell[1], cell[2], BlockState(*h_id));
            }
            let [dvx, dvz] = dir.vec();
            let hinge = Vec3::new(
                tree.base[0] as f32 + 0.5,
                tree.base[1] as f32 + 0.5,
                tree.base[2] as f32 + 0.5,
            );
            for item in &plan.drop_items {
                self.spawn_drop(item, 1, hinge + Vec3::new(dvx * 1.5, 0.2, dvz * 1.5));
            }
            // canopy shatter: debris where the rotated canopy lands
            if self.settings.particles {
                let parts = lf_game::timber::tree_parts(&tree, lf_game::timber::LAND_ANGLE, dir);
                for (c, _) in parts.iter().skip(tree.trunk.len()).step_by(2) {
                    self.spawn_break_particles(
                        tree.leaf_id,
                        (c[0] as i32, c[1] as i32, c[2] as i32), 3);
                }
            }
            self.play_sfx(lf_audio::Sfx::TreeCrash, 1.0);
            self.shake = (self.shake + (0.12 + 0.025 * tree.trunk.len() as f32).min(0.5)).min(0.6);
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

    /// GMod-style item props: rigid physics (bounce off floor and walls,
    /// tumble, settle), same-item stacks merge up to five and grow with
    /// count, the carried prop springs along the view ray, and walking
    /// into a stack picks it up.
    fn update_drops(&mut self, dt: f32) {
        let player_center = self.player.position + Vec3::new(0.0, 0.9, 0.0);
        let mut to_remove: Vec<usize> = Vec::new();
        let mut collected: Vec<String> = Vec::new();
        let carried = self.carried_drop;
        for (i, drop) in self.drops.iter_mut().enumerate() {
            drop.age += dt;
            let half = lf_game::props::prop_half(drop.stack.count);
            if Some(drop.id) == carried {
                // gravity-gun carry: pin the prop to its grab distance
                // along the view ray with a soft spring; walls stop it
                let eye = self.player.eye_position();
                let look = self.player.look_dir();
                let target = eye + look * self.carry_dist;
                let delta = target - drop.body.position;
                drop.body.rest = false;
                drop.body.velocity = delta * 12.0;
                drop.body.position += drop.body.velocity * dt;
                if drop.body.velocity.length() > 30.0 {
                    drop.body.velocity = drop.body.velocity.normalize() * 30.0;
                }
                // walk right up to a carried prop to pocket it
                if drop.age > 0.2
                    && (player_center - drop.body.position).length() < 1.0 + half
                {
                    let taken = drop.stack.count.saturating_sub(
                        self.inventory.add_item(&drop.stack.item_id, drop.stack.count),
                    );
                    if taken > 0 {
                        collected.push(drop.stack.item_id.clone());
                    }
                    if drop.stack.count == taken {
                        to_remove.push(i);
                        self.carried_drop = None;
                    } else {
                        drop.stack.count -= taken;
                    }
                }
                continue;
            }
            lf_game::props::step_prop(&self.world, &mut drop.body, half, dt);
            // walking into a resting stack picks it up (close grab; the old
            // 2-block magnet vacuum is gone — items now litter the floor)
            if drop.age > 0.35
                && (player_center - drop.body.position).length() < 0.95 + half
            {
                let taken = drop.stack.count.saturating_sub(
                    self.inventory.add_item(&drop.stack.item_id, drop.stack.count),
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
        // same-item resting stacks merge up to five (survivor keeps the
        // lower position so the pile looks continuous)
        let mut merges: Vec<(usize, usize)> = Vec::new();
        for a in 0..self.drops.len() {
            for b in (a + 1)..self.drops.len() {
                let (da, db) = (&self.drops[a], &self.drops[b]);
                if da.stack.item_id != db.stack.item_id
                    || !da.body.rest || !db.body.rest
                    || da.body.held || db.body.held
                {
                    continue;
                }
                let reach = lf_game::props::merge_distance(da.stack.count, db.stack.count);
                if da.body.position.distance(db.body.position) <= reach {
                    merges.push((a, b));
                }
            }
        }
        // one merge per frame: removals would shift the collected indices,
        // and leftover piles drain over a few frames anyway
        if let Some(&(a, b)) = merges.first() {
            if let Some((surviving, leftover)) =
                lf_game::props::merged_counts(self.drops[a].stack.count, self.drops[b].stack.count)
            {
                let (keep, give) = if self.drops[a].body.position.y
                    <= self.drops[b].body.position.y
                {
                    (a, b)
                } else {
                    (b, a)
                };
                self.drops[keep].stack.count = surviving;
                self.drops[keep].body.rest = false; // the bigger cube re-settles
                if leftover == 0 {
                    self.drops.remove(give);
                } else {
                    self.drops[give].stack.count = leftover;
                }
            }
        }
        let catalog_pairs_ref = workbench::catalog_pairs();
        if !collected.is_empty() {
            self.play_sfx(lf_audio::Sfx::ItemPickup, 0.6);
        }
        for item in collected {
            let first_ever = self.chronicle.is_empty();
            self.onboarding.observe_collected(&item);
            self.quest_event(QuestEvent::Collected(item.clone()));
            // "any_food" quest targets match any collected food item
            if matches!(item_def(&item).map(|d| d.kind), Some(ItemKind::Food(_))) {
                self.quest_event(QuestEvent::Collected("any_food".into()));
            }
            if first_ever && item == "log" {
                self.chronicle_event(EventType::FirstCraft, "collected the first logs".into());
            }
            // F3: a first pickup unlocks every recipe this item is an
            // ingredient of; the workbench whispers what's now possible
            let unlocked = self.recipe_book.unlock_on_pickup(&item, &catalog_pairs_ref, self.research.era);
            if unlocked > 0 {
                let plural = if unlocked == 1 { "recipe" } else { "recipes" };
                self.chronicle_toast = Some((
                    format!("Recipes unlocked: {} new {}", unlocked, plural), 4.0));
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
                    if let Some(col) = self.storage.as_ref().and_then(|s| s.load_chunk(pos.0, pos.1)) {
                        self.world.chunks.insert(pos, col);
                        self.add_column_batch(pos.0, pos.1);
                        loaded += 1;
                    }
                }
            }
        }

        self.try_spawn_villagers();
        self.try_settle_dragons();

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
        // fix: black-square artifact — an empty column mesh must never
        // register a batch; a zero-filled vertex buffer drawn as a quad
        // reads as a black rectangle where the chunk should be (water
        // already had this guard).
        if v.is_empty() {
            self.batches.remove(&(cx, cz));
        } else {
            self.batches.insert((cx, cz), MeshBatch::new(&self.device, &self.resources, &v, &i));
        }
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
                    if let Some(storage) = &self.storage {
                        let _ = storage.save_chunk(pos.0, pos.1, col);
                    }
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
            // fix: black-square artifact — same guard as add_column_batch
            if v.is_empty() {
                self.batches.remove(&(bx, bz));
            } else {
                self.batches.insert((bx, bz), MeshBatch::new(&self.device, &self.resources, &v, &i));
            }
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
            sun_direction: lf_engine::atmosphere::sun_direction(self.time.fraction()).to_array(),
        }
    }

    fn camera(&self) -> Camera {
        if self.ui_open == UiOpen::Title {
            // B2: scenic elliptical orbit with a slow altitude oscillation,
            // looking at a point offset from the world center. Parameters
            // live in lf_worldgen::preview so the vistest harness drives
            // the identical path.
            let c = [self.spawn_point.x, self.spawn_point.y, self.spawn_point.z];
            let (eye, look) =
                lf_worldgen::preview::preview_camera(self.title_orbit as f64, c);
            // Never sink the eye into ring terrain (audit Step 1), and
            // unloaded columns report surface 0 (keeps the classic offset).
            let ground_at_eye = self.world.surface_height(eye[0] as i32, eye[2] as i32);
            let cy = title_eye_y(self.spawn_point.y, ground_at_eye).max(eye[1]);
            let mut camera = Camera::new(
                glam::Vec3::new(eye[0], cy, eye[2]),
                glam::Vec3::from_slice(&look),
            );
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
        // fix: black-square artifact — these batches were never given the
        // camera, so they rendered with MeshBatch::new's IDENTITY uniform:
        // any entity cube within ±1 unit of the world origin (the spawn!)
        // landed inside the clip volume and filled the screen with a giant
        // black rectangle, while the sun/moon/clouds/drops were invisible.
        if let Some(batch) = &self.sky_batch {
            batch.update_camera(&self.queue, &camera, &env);
        }
        if let Some(batch) = &self.cloud_batch {
            batch.update_camera(&self.queue, &camera, &env);
        }
        if let Some(batch) = &self.weather_batch {
            batch.update_camera(&self.queue, &camera, &env);
        }
        if let Some(batch) = &self.drop_batch {
            batch.update_camera(&self.queue, &camera, &env);
        }
        if let Some(batch) = &self.crack_batch {
            batch.update_camera(&self.queue, &camera, &env);
        }
        if let Some(batch) = &self.particle_batch {
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
    /// Rebuild the dynamic entity batch: articulated humanoids, mobs,
    /// alpha-cutout item sprites, projectiles, and falling blocks.
    fn rebuild_drop_batch(&mut self) {
        let remotes_empty = self.net.as_ref().map(|n| n.remote_players.is_empty()).unwrap_or(true);
        if self.drops.is_empty() && self.mobs.is_empty() && self.falling_blocks.is_empty()
            && self.companions.is_empty() && self.falling_trees.is_empty()
            && self.villagers.is_empty() && self.arrows.is_empty() && self.firebolts.is_empty()
            && remotes_empty {
            self.drop_batch = None;
            return;
        }
        let (mut vertices, mut indices) = (Vec::new(), Vec::new());
        let mut push_faces = |faces: Vec<([[f32; 3]; 4], [f32; 3])>, tex: u32,
                              uvs: &[[f32; 2]; 4], vertices: &mut Vec<GpuVertex>, indices: &mut Vec<u32>| {
            for (corners, normal) in faces {
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
            }
        };
        let mut push_cube = |cx: f32, cy: f32, cz: f32, r: f32, tex: u32, vertices: &mut Vec<GpuVertex>, indices: &mut Vec<u32>| {
            let faces = cube_faces(r).into_iter()
                .map(|(normal, corners, _)| {
                    let translated = corners.map(|c| [c[0] + cx, c[1] + cy, c[2] + cz]);
                    (translated, normal)
                }).collect();
            push_faces(faces, tex, &UVS_CUBE, vertices, indices);
        };
        let mut push_humanoid = |feet: Vec3, yaw: f32, gait: f32, crouch: f32, tex: u32,
                                  vertices: &mut Vec<GpuVertex>, indices: &mut Vec<u32>| {
            push_faces(
                lf_engine::scene::humanoid_faces(feet, yaw, gait, crouch),
                tex, &UVS_CUBE, vertices, indices,
            );
        };
        let mut push_item_sprite = |center: Vec3, r: f32, tex: u32,
                                    vertices: &mut Vec<GpuVertex>, indices: &mut Vec<u32>| {
            // Two crossed, double-sided alpha-cutout cards. Inventory art is
            // therefore recognizable from any angle without a 3D tool mesh.
            let y0 = center.y - r;
            let y1 = center.y + r;
            let faces = vec![
                ([[center.x-r,y0,center.z], [center.x-r,y1,center.z], [center.x+r,y1,center.z], [center.x+r,y0,center.z]], [0.0,0.0,1.0]),
                ([[center.x+r,y0,center.z], [center.x+r,y1,center.z], [center.x-r,y1,center.z], [center.x-r,y0,center.z]], [0.0,0.0,-1.0]),
                ([[center.x,y0,center.z-r], [center.x,y1,center.z-r], [center.x,y1,center.z+r], [center.x,y0,center.z+r]], [1.0,0.0,0.0]),
                ([[center.x,y0,center.z+r], [center.x,y1,center.z+r], [center.x,y1,center.z-r], [center.x,y0,center.z-r]], [-1.0,0.0,0.0]),
            ];
            push_faces(faces, tex, &UVS_CUBE, vertices, indices);
        };
        // falling granular blocks: tumbling cubes (loop 330 deep-fall)
        for fb in &self.falling_blocks {
            let tex = lf_assets::texture_index_for_block(fb.block.id());
            let faces = lf_engine::scene::rotated_cube_faces(fb.position, 0.48, fb.tumble_axis, fb.angle);
            push_faces(faces, tex, &UVS_CUBE, &mut vertices, &mut indices);
        }
        // felled trees: the rigid rotated assembly from the pure layout fn
        for ft in &self.falling_trees {
            let bark = lf_assets::texture_index_for_block(ft.tree.log_id);
            let leaf = lf_assets::texture_index_for_block(ft.tree.leaf_id);
            let (axis, sign) = lf_game::timber::fall_rotation(ft.dir);
            let axis = Vec3::from_array(axis);
            let parts = lf_game::timber::tree_parts(&ft.tree, ft.angle, ft.dir);
            for (i, (c, half)) in parts.iter().enumerate() {
                let tex = if i < ft.tree.trunk.len() { bark } else { leaf };
                let faces = lf_engine::scene::rotated_cube_faces(
                    Vec3::from_slice(c), half[0], axis, sign * ft.angle);
                push_faces(faces, tex, &UVS_CUBE, &mut vertices, &mut indices);
            }
        }
        // mobs: per-type skins (C2 refresh) with biome-tint variants for
        // the common hostiles, red hurt-flash copies while `hurt_flash`
        // lives, articulated animals (leg swing + real facing), humanized
        // raiders, and toppled corpses during the death animation.
        let player_biome = self.map.biome_at(self.player.position.x as i32, self.player.position.z as i32);
        for mob in &self.mobs {
            let size = mob.mob_type.stats().size;
            let mut tex = match mob.mob_type {
                MobType::NullKnight => lf_assets::MOB_NULL_KNIGHT_LAYER,
                MobType::Dragon => lf_assets::DRAGON_BODY_LAYER,
                MobType::Boar => lf_assets::MOB_BOAR_LAYER,
                MobType::Woolbeast => lf_assets::MOB_WOOLBEAST_LAYER,
                MobType::NamelessRaider => lf_assets::VILLAGER_NAMELESS_LAYER,
                common => {
                    let (base, tints) = match common {
                        MobType::Glitchling => (lf_assets::MOB_GLITCHLING_LAYER, lf_assets::MOB_GLITCHLING_TINTS),
                        MobType::Stalker => (lf_assets::MOB_STALKER_LAYER, lf_assets::MOB_STALKER_TINTS),
                        _ => (lf_assets::MOB_CRAWLER_LAYER, lf_assets::MOB_CRAWLER_TINTS),
                    };
                    match player_biome {
                        lf_worldgen::Biome::Desert | lf_worldgen::Biome::Badlands
                        | lf_worldgen::Biome::Volcanic | lf_worldgen::Biome::Savanna => tints[0],
                        lf_worldgen::Biome::Tundra | lf_worldgen::Biome::SnowyTaiga
                        | lf_worldgen::Biome::SnowySlope | lf_worldgen::Biome::SnowyPeaks
                        | lf_worldgen::Biome::IceSpikes => tints[1],
                        lf_worldgen::Biome::Swamp | lf_worldgen::Biome::PaleGarden
                        | lf_worldgen::Biome::DarkForest => tints[2],
                        _ => base,
                    }
                }
            };
            let animal_tex = match mob.mob_type {
                MobType::Chicken => Some(lf_assets::mob_chicken_layer()),
                MobType::Wolf => Some(lf_assets::mob_wolf_layer()),
                MobType::Dog => Some(lf_assets::mob_dog_layer()),
                MobType::Bear => Some(lf_assets::mob_bear_layer()),
                MobType::Boar => Some(lf_assets::MOB_BOAR_LAYER),
                MobType::Woolbeast => Some(lf_assets::MOB_WOOLBEAST_LAYER),
                _ => None,
            };
            // hurt flash: mostly-on flicker while the damage tint lives
            let flashing = mob.hurt_flash > 0.0 && (self.elapsed * 24.0).sin() > -0.5;
            if flashing {
                tex = lf_assets::hurt_layer_for(tex);
            }
            let animal = animal_tex
                .map(|a| if flashing { lf_assets::hurt_layer_for(a) } else { a });
            // death topple: ease-out fall onto the face around a ground
            // axis perpendicular to the mob's facing
            let topple = mob
                .death_t
                .map(|t| (t / lf_game::mobs::DEATH_TOPPLE_S).min(1.0))
                .map(|p| (1.0 - (1.0 - p).powi(3)) * 1.45);
            let (sy, cy) = mob.yaw.sin_cos();
            let side_axis = Vec3::new(cy, 0.0, -sy);
            if mob.mob_type == MobType::Dragon {
                // P36: multi-part assembly — body/head/wings/tail with
                // sine animation from the shared layout fn
                let t = self.elapsed;
                for (offset, part_size) in lf_game::dragons::dragon_parts(t, mob.yaw) {
                    let p = mob.position + offset + Vec3::new(0.0, size, 0.0);
                    push_cube(p.x, p.y, p.z, part_size, tex, &mut vertices, &mut indices);
                }
            } else if let Some(animal) = animal {
                // articulated assembly: every part pitches around its own
                // pivot (legs swing in trot pairs), the whole body yaws to
                // the facing, and the corpse topples around the feet
                let mut faces = Vec::new();
                for part in lf_game::mobs::animal_parts(
                    mob.mob_type, mob.gait_phase, mob.gait_amp, mob.hurt_flash,
                ) {
                    faces.extend(lf_engine::scene::cuboid_part_faces(
                        mob.position,
                        mob.yaw,
                        Vec3::from_array(part.center),
                        Vec3::from_array(part.half),
                        part.pitch,
                        Vec3::from_array(part.pivot),
                    ));
                }
                if let Some(angle) = topple {
                    faces = lf_engine::scene::topple_faces(faces, mob.position, side_axis, angle);
                }
                push_faces(faces, animal, &UVS_CUBE, &mut vertices, &mut indices);
            } else if mob.mob_type == MobType::NamelessRaider {
                // the raiders walk as people — gait from the same cycle
                let gait = mob.gait_phase.sin() * 0.55 * mob.gait_amp;
                let mut faces = lf_engine::scene::humanoid_faces(mob.position, mob.yaw, gait, 0.0);
                if let Some(angle) = topple {
                    faces = lf_engine::scene::topple_faces(faces, mob.position, side_axis, angle);
                }
                push_faces(faces, tex, &UVS_CUBE, &mut vertices, &mut indices);
            } else {
                // silhouette differentiation (C2): crawler low+wide,
                // stalker tall+lean, knight imposing
                let (r, lift) = match mob.mob_type {
                    MobType::Crawler => (size * 1.35, size * 0.55),
                    MobType::Stalker => (size * 0.85, size * 1.25),
                    _ => (size, size),
                };
                match topple {
                    Some(angle) => {
                        // spin the cube's center around the ground pivot,
                        // then let rotated_cube_faces tumble the cube itself
                        let c = Vec3::new(0.0, lift, 0.0);
                        let rot = c * angle.cos() + side_axis.cross(c) * angle.sin();
                        let faces = lf_engine::scene::rotated_cube_faces(
                            mob.position + rot, r, side_axis, angle,
                        );
                        push_faces(faces, tex, &UVS_CUBE, &mut vertices, &mut indices);
                    }
                    None => push_cube(mob.position.x, mob.position.y + lift, mob.position.z, r, tex, &mut vertices, &mut indices),
                }
            }
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
        // Villagers: faction skins for roster NPCs and profession outfits
        // for everyone else. All use the same six-part articulated body.
        for v in &self.villagers {
            let job_key = match v.job {
                VillagerJob::Farmer => "farmer", VillagerJob::Smith => "smith",
                VillagerJob::Trader => "trader", VillagerJob::Guard => "guard",
                VillagerJob::Bard => "bard", VillagerJob::Lorekeeper => "lorekeeper",
                VillagerJob::Wizard => "wizard", VillagerJob::Monarch => "monarch",
            };
            let job_tex = lf_assets::villager_job_layer(job_key);
            let tex = match v.archetype.as_deref() {
                Some("the_unmarked") => lf_assets::VILLAGER_UNMARKED_LAYER,
                Some("maren_voss") => lf_assets::VILLAGER_MAREN_LAYER,
                Some(_) => match v.faction.as_deref() {
                    Some("accord") => lf_assets::VILLAGER_ACCORD_LAYER,
                    Some("ironborn") => lf_assets::VILLAGER_IRONBORN_LAYER,
                    Some("ember_covenant") => lf_assets::VILLAGER_COVENANT_LAYER,
                    Some("free_holds") => lf_assets::VILLAGER_FREEHOLDS_LAYER,
                    Some("ashen_order") => lf_assets::VILLAGER_ASHEN_LAYER,
                    Some("nameless") => lf_assets::VILLAGER_NAMELESS_LAYER,
                    _ => job_tex,
                },
                None => job_tex,
            };
            let (gait, crouch) = match v.activity {
                lf_npc::NpcActivityState::Walking => (v.walk_phase.sin() * 0.55 * v.walk_amp, 0.0),
                lf_npc::NpcActivityState::Working => ((self.elapsed * 3.0).sin() * 0.24, 0.12),
                lf_npc::NpcActivityState::Eating => ((self.elapsed * 4.0).sin() * 0.12, 0.32),
                lf_npc::NpcActivityState::Sleeping => (0.0, 0.9),
                _ => (0.0, 0.0),
            };
            push_humanoid(Vec3::from_array(v.position), v.yaw, gait, crouch, tex, &mut vertices, &mut indices);
        }
        // companions: their archetype skin, swapping to the trust-badge
        // variant at trust >= 50 (ENTITY_SKIN_SPEC)
        for c in &self.companions {
            let base = lf_assets::COMPANION_LAYERS
                .iter()
                .find(|(id, _)| *id == c.npc_archetype_id)
                .map(|(_, layer)| *layer)
                .unwrap_or(lf_assets::VILLAGER_ACCORD_LAYER);
            let tex = if c.trust >= lf_game::companions::TRUST_BADGE {
                lf_assets::trusted_companion_layer(base)
            } else {
                base
            };
            let gait = if c.velocity.length_squared() > 0.01 { (self.elapsed * 7.0).sin() * 0.58 } else { 0.0 };
            push_humanoid(c.position, c.yaw, gait, 0.0, tex, &mut vertices, &mut indices);
        }
        // Network players share a proper neutral skin until cosmetic ids are
        // added to the protocol; yaw arrives on the wire and the gait is
        // estimated from position deltas so remote walkers visibly walk.
        let remotes: Vec<(u64, [f32; 3], f32)> = self.net.as_ref()
            .map(|n| n.remote_players.iter().map(|(id, rp)| (*id, rp.pos, rp.yaw)).collect())
            .unwrap_or_default();
        if !remotes.is_empty() {
            let dt = self.last_dt;
            for (id, pos_arr, yaw) in remotes.clone() {
                let pos = Vec3::from_array(pos_arr);
                let m = self.remote_motion.entry(id).or_insert((pos, 0.0, 0.0));
                let speed = ((pos - m.0) / dt.max(0.001)).length().min(8.0);
                m.0 = pos;
                let target = if speed > 0.4 { 1.0 } else { 0.0 };
                m.2 += (target - m.2).clamp(-dt * 8.0, dt * 8.0);
                if m.2 > 0.01 {
                    m.1 += dt * speed * 4.2;
                }
                let gait = m.1.sin() * 0.55 * m.2;
                push_humanoid(pos, yaw, gait, 0.0,
                    lf_assets::player_wayfarer_layer(), &mut vertices, &mut indices);
            }
            self.remote_motion.retain(|id, _| remotes.iter().any(|(rid, _, _)| rid == id));
        }
        // Item props: block items are rigid tumbling cubes sized by their
        // stack count (1 = a chunk, 5 = a full block); non-block drops use
        // their authored inventory sprite as crossed alpha-cutout impostors
        // scaled by the same rule.
        for drop in &self.drops {
            let half = lf_game::props::prop_half(drop.stack.count);
            let center = drop.body.position;
            if let Some(tex) = lf_assets::item_texture_layer(&drop.stack.item_id) {
                push_item_sprite(center, half, tex, &mut vertices, &mut indices);
            } else {
                let faces = lf_engine::scene::rotated_cube_faces(
                    center, half * 0.98, drop.body.tumble_axis, drop.body.angle,
                );
                push_faces(faces, drop_tex_layer(&drop.stack.item_id), &UVS_CUBE, &mut vertices, &mut indices);
            }
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

/// The standard full-face UVs for dynamic cubes (loop 330).
const UVS_CUBE: [[f32; 2]; 4] = [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];

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

/// lore-and-visuals save extras (Sections A + B) bundled so the load
/// tuple stays manageable.
#[derive(Default)]
pub struct LoreExtras {
    pub standings: lf_lore::StandingState,
    pub companions: Vec<lf_game::companions::Companion>,
    pub companion_memory: std::collections::HashMap<String, i32>,
    pub visited_biomes: std::collections::HashSet<String>,
    pub discovered_structures: Vec<(String, i32, i32, i32)>,
    pub day_index: u64,
    /// F3: the earned recipe set + workbench queue, restored with the slot.
    pub recipe_book: workbench::RecipeBook,
    pub craft_queue: Vec<(String, u32)>,
    /// loop 345: discovered kingdoms.
    pub kingdoms: Vec<KingdomRecord>,
    /// N01: first-minute tutorial state (default = fresh tutorial).
    pub onboarding: onboarding::Onboarding,
}

fn load_client_save(dir: &Path, lore_reg: &lf_lore::LoreRegistry)
    -> (Inventory, PlayerStats, lf_game::TimeOfDay, HashMap<(i32, i32, i32), BlockEntity>, Vec<MobEntity>, Vec<Villager>, u32, QuestLog, Vec<ChronicleEvent>, ResearchState, Settings, lf_worldgen::WorldType, Vec<map::Waypoint>, lf_game::magic::Spellbook, std::collections::HashMap<String, String>, lf_game::paths::Paths, LoreExtras) {
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
    let mut lore = LoreExtras {
        standings: lf_lore::StandingState::starting(lore_reg),
        ..Default::default()
    };
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
            let paths = save.paths.unwrap_or_default();
            if let Some(s) = save.faction_standing {
                lore.standings = s;
            }
            lore.companions = save.companions;
            lore.companion_memory = save.companion_memory;
            lore.visited_biomes = save.visited_biomes.into_iter().collect();
            lore.discovered_structures = save.discovered_structures;
            lore.kingdoms = save.kingdoms;
            lore.day_index = save.day_index;
            lore.recipe_book = save.recipe_book.unwrap_or_default();
            lore.craft_queue = save.craft_queue;
            lore.onboarding = save.onboarding.unwrap_or_default();
            return (inventory, stats, lf_game::TimeOfDay::new(save.time_ticks), entities, mobs, villagers, kills, quest_log, chronicle, research, settings, world_type, waypoints, spellbook, runed, paths, lore);
        }
    }
    (inventory, stats, time, entities, mobs, villagers, kills, quest_log, chronicle, research, settings, lf_worldgen::WorldType::Normal, Vec::new(), lf_game::magic::Spellbook::default(), std::collections::HashMap::new(), lf_game::paths::Paths::default(), lore)
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
        // the volcanic belt: warm, desaturated, slightly dark (C1)
        B::Volcanic => ([1.05, 0.98, 0.94], 0.88),
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

    /// Loop 347 wheel fix: every accumulated notch becomes a step in the
    /// same frame (fast flicks aren't dropped), the fractional remainder
    /// survives for trackpads, and sub-notch drift changes nothing.
    #[test]
    fn scroll_notches_all_count_and_remainder_survives() {
        let mut a = 3.7;
        assert_eq!(consume_scroll_steps(&mut a), 3);
        assert!((a - 0.7).abs() < 1e-6, "remainder kept, got {}", a);
        let mut b = -2.5;
        assert_eq!(consume_scroll_steps(&mut b), -2);
        assert!((b + 0.5).abs() < 1e-6, "negative remainder kept, got {}", b);
        let mut c = 0.9;
        assert_eq!(consume_scroll_steps(&mut c), 0, "sub-notch does nothing");
        assert!((c - 0.9).abs() < 1e-6);
        let mut d = -0.4;
        assert_eq!(consume_scroll_steps(&mut d), 0);
        // accumulating trackpad ticks eventually cross a notch
        d += -0.8;
        assert_eq!(consume_scroll_steps(&mut d), -1);
    }

    /// Loop 347 creative-break fix: a mob behind a wall must NOT capture
    /// the crosshair — before the occlusion filter, any mob roughly on
    /// the look line stole LMB into the 0.5s attack cooldown, capping
    /// block removal at 2/second anywhere near an animal.
    #[test]
    fn crosshair_mob_requires_line_of_sight() {
        use lf_voxel::BlockState;
        let mut world = lf_voxel::World::new();
        for cx in -1..=1 {
            for cz in -1..=1 {
                world.ensure_chunk(cx, cz);
            }
        }
        for x in -6..6 {
            for z in -6..6 {
                world.set_block(x, 0, z, BlockState::STONE);
            }
        }
        // stone wall at z = 0, full height
        for x in -6..6 {
            for y in 1..4 {
                world.set_block(x, y, 0, BlockState::STONE);
            }
        }
        let eye = Vec3::new(0.0, 1.6, -3.0);
        let look = Vec3::new(0.0, 0.0, 1.0);
        // boar behind the wall, dead center on the ray
        let hidden = MobEntity::spawn(1, MobType::Boar, Vec3::new(0.0, 1.0, 2.0));
        assert_eq!(crosshair_mob(&[hidden], &world, eye, look, 6.0), None,
            "mob behind the wall must not capture the crosshair");
        // same boar with the wall removed: a fair target
        let mut open = lf_voxel::World::new();
        for cx in -1..=1 {
            for cz in -1..=1 {
                open.ensure_chunk(cx, cz);
            }
        }
        for x in -6..6 {
            for z in -6..6 {
                open.set_block(x, 0, z, BlockState::STONE);
            }
        }
        let visible = MobEntity::spawn(2, MobType::Boar, Vec3::new(0.0, 1.0, 2.0));
        assert_eq!(crosshair_mob(&[visible.clone()], &open, eye, look, 6.0), Some(0));
        // nearest-first: with both visible, the closer one wins
        let near = MobEntity::spawn(3, MobType::Boar, Vec3::new(0.0, 1.0, 1.0));
        assert_eq!(crosshair_mob(&[visible.clone(), near], &open, eye, look, 6.0), Some(1));
    }

    /// Loop 330 deep-fall: tumble advances monotonically, fast first
    /// impacts bounce exactly once with restitution, slow ones settle.
    #[test]
    fn fallers_bounce_once_then_settle() {
        // tumble: linear in dt
        assert!((tumble_step(1.0, 2.0, 0.5) - 2.0).abs() < 1e-5);
        // fast first impact bounces with 0.18 restitution
        match faller_landing(10.0, false) {
            FallerLanding::Bounce(up) => assert!((up - 1.8).abs() < 1e-5),
            FallerLanding::Place => panic!("10 m/s must bounce"),
        }
        // a bounced faller never bounces again
        assert!(matches!(faller_landing(10.0, true), FallerLanding::Place));
        // slow settle lands immediately
        assert!(matches!(faller_landing(3.0, false), FallerLanding::Place));
    }

    #[test]
    fn faller_tumble_axes_are_deterministic_and_unit() {
        let a = faller_tumble_axis(0xDEAD_BEEF);
        let b = faller_tumble_axis(0xDEAD_BEEF);
        assert_eq!(a, b, "same seed, same axis");
        assert!((a.length() - 1.0).abs() < 1e-4, "normalized, got {}", a.length());
        assert_ne!(faller_tumble_axis(1), faller_tumble_axis(2), "varied per block");
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

    /// N02: the craft queue persists exactly (jobs survive reload; the
    /// engine re-verifies them against live inventory on every tick, so a
    /// stale save cannot mint items).
    #[test]
    fn craft_queue_persists_through_client_save() {
        let save = ClientSave {
            craft_queue: vec![
                ("torch".to_string(), 4),
                ("planks".to_string(), 12),
            ],
            ..Default::default()
        };
        let loaded: ClientSave = serde_json::from_str(&serde_json::to_string(&save).unwrap()).unwrap();
        assert_eq!(loaded.craft_queue, vec![
            ("torch".to_string(), 4),
            ("planks".to_string(), 12),
        ]);
        // an old save without the field defaults to an empty queue
        let old_json = serde_json::to_string(&ClientSave::default())
            .unwrap()
            .replace("craft_queue", "legacy_ignored");
        let old: ClientSave = serde_json::from_str(&old_json).unwrap();
        assert!(old.craft_queue.is_empty());
    }

    /// N01: the tutorial state persists through the JSON extras path, and
    /// an old save without the field resumes a fresh tutorial (Move) —
    /// the load path must not inherit GameState::new's stale copy.
    #[test]
    fn onboarding_persists_and_defaults_through_client_save() {
        let mut done = onboarding::Onboarding::default();
        done.observe_frame([0.0, 0.0, 0.0], 0.0, 0.0);
        done.observe_frame([9.0, 0.0, 0.0], 0.0, 0.0); // Move
        done.observe_frame([9.0, 0.0, 0.0], 2.0, 1.0); // Look
        done.observe_collected("log"); // Gather
        done.observe_crafted(); // Craft
        done.observe_placed(true); // Build -> Done
        assert_eq!(done.step, onboarding::TutorialStep::Done);
        let save = ClientSave { onboarding: Some(done.clone()), ..Default::default() };
        let loaded: ClientSave = serde_json::from_str(&serde_json::to_string(&save).unwrap()).unwrap();
        assert_eq!(loaded.onboarding.unwrap().step, onboarding::TutorialStep::Done);

        // old save (field absent) -> default tutorial state
        let old_json = serde_json::to_string(&ClientSave::default())
            .unwrap()
            .replace("onboarding", "legacy_ignored");
        let old: ClientSave = serde_json::from_str(&old_json).unwrap();
        assert!(old.onboarding.is_none());
        assert_eq!(old.onboarding.unwrap_or_default().step, onboarding::TutorialStep::Move);
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

    /// Steps 21-22: the lore anchors (the Smith, the Null, the river
    /// wardens) are consistent ACROSS systems — books, items, and the
    /// NPC who trades them.
    #[test]
    fn lore_anchors_span_three_systems() {
        // books (lore/books.toml)
        let books_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lore/books.toml");
        let lib = crate::lore::LoreLibrary::load(&books_path);
        assert!(lib.books.len() >= 3);
        let all_pages: String = lib.books.iter()
            .flat_map(|b| b.pages.iter().map(|p| p.as_str()))
            .collect::<Vec<_>>().join(" ");
        for anchor in ["the smith", "null", "river"] {
            assert!(all_pages.to_lowercase().contains(anchor),
                "anchor {:?} missing from the books", anchor);
        }
        // items: each tome maps to its book
        for item in ["tome_of_the_forge", "tome_of_the_null", "wardens_ledger"] {
            assert!(lib.for_item(item).is_some(), "no book behind item {}", item);
            assert!(lf_game::items::item_def(item).is_some(), "tome item {} missing", item);
        }
        // the NPC: the Lorekeeper trades all three tomes
        let trades = lf_npc::trade_offers(lf_npc::VillagerJob::Lorekeeper);
        for item in ["tome_of_the_forge", "tome_of_the_null", "wardens_ledger"] {
            assert!(trades.iter().any(|(give, _, _, _)| give == &item),
                "the Lorekeeper does not trade {}", item);
        }
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
