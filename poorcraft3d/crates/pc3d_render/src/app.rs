//! The windowed shell: a resizable `winit` window driving the 3D renderer.
//!
//! R3DV-002 scope: first-person input (click to grab the mouse, mouse look,
//! WASD + Space/Shift movement), camera scripts for automated proof runs,
//! scheduled live-window screenshot captures, resize/loss recovery, and
//! frame-time accounting.
//!
//! GLM UI rework (UI-003..UI-006): when `owner_menu` is set the shell runs
//! the real UI layer from `crate::ui` — title/new world/load world/settings/
//! gameplay HUD/pause screens with mouse hover/click + keyboard navigation,
//! confirmation modals, save slots, toasts, a live HUD (bars, 9-slot hotbar
//! driving the build material, crosshair, prompt), and the F3 debug strip
//! (debug text is never the owner HUD). Escape pauses/resumes and never
//! exits; quitting goes through explicit menu choices or the Q confirm
//! modal. Automated proof windows leave `owner_menu` off and keep the exact
//! legacy behavior.

use crate::camera::CameraPose;
use crate::renderer::{PixelReport, Renderer};
use crate::ui::{self, Key, ModalKind, Screen, UiAction, UiState};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, NamedKey, PhysicalKey};
use winit::window::{Window, WindowAttributes};

/// One scheduled live-window capture.
#[derive(Clone)]
pub struct Shot {
    /// Frame number at which the presented swapchain frame is captured.
    pub frame: u64,
    pub path: PathBuf,
    /// Verify the sky probe on this capture (false for close-ups whose
    /// frame legitimately contains no sky).
    pub sky: bool,
    /// When set, the UI draw list + state at capture time is written here
    /// as JSON (the layout dump proof artifacts).
    pub ui_dump: Option<PathBuf>,
}

impl Shot {
    pub fn new(frame: u64, path: impl Into<PathBuf>) -> Self {
        Self {
            frame,
            path: path.into(),
            sky: true,
            ui_dump: None,
        }
    }

    pub fn sky(mut self, sky: bool) -> Self {
        self.sky = sky;
        self
    }

    /// Also dump the UI layout + state JSON next to this capture.
    pub fn ui_dump(mut self, path: impl Into<PathBuf>) -> Self {
        self.ui_dump = Some(path.into());
        self
    }
}

/// A callback run once at a scheduled frame, before that frame's capture or
/// render — the hook host-driven proof runs use to submit world edits
/// through their own host and sync the renderer from its read-only state.
pub type FrameHook = Box<dyn FnMut(&mut Renderer)>;

/// Per-frame context handed to UI script steps (the screenshot harness and
/// the inspector drive the UI through these).
pub struct UiFrameCtx {
    pub frame: u64,
    pub p50_ms: f32,
    pub fps: f32,
    /// Actions pushed by the step; the app executes them after the step
    /// returns (same path as real input).
    pub actions: Vec<UiAction>,
}

/// A scripted UI mutation at a scheduled frame (proof runs only).
pub type UiStep = Box<dyn FnMut(&mut UiState, &mut Renderer, &mut UiFrameCtx)>;

/// Which semantic probes a run's captures verify against: the full scene
/// table (placeholder ground + sky) or sky-only (construction runs hide the
/// placeholder scene, so only the sky probe applies).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeSet {
    Scene,
    SkyOnly,
}

/// Data-only slice setup (cloneable config); the app assembles the full
/// SliceHost at window creation via `slice::assemble`.
pub struct SliceSetup {
    pub seed: u64,
    /// NWR-011: assemble the REBUILD stack (surface walk, crowd, kit).
    pub rebuild: bool,
    pub scene: std::rc::Rc<crate::slice::SliceScene>,
    pub save_root: std::rc::Rc<std::path::PathBuf>,
    pub world_name: String,
}

/// The live vertical slice: a walking player on colliding terrain, a host
/// for place/remove through commands, save/reload on disk, and inspect
/// boxes. Keys: WASD walk, mouse look (click), F place at ray target, R
/// remove at ray target, B save, L reload, I inspect boxes; live owner builds
/// use Escape for pause/resume and Q for explicit quit.
pub struct SliceHost {
    pub seed: u64,
    /// The NWR-011 rebuild stack: crowd ticking + surface walking +
    /// foundation-gated placement. None = the classic R3DV slice.
    pub rebuild: bool,
    /// Jump state (the controls spec's Space): vertical velocity + the
    /// remembered ground height.
    pub jump_vy: f32,
    pub jump_held: bool,
    pub ground_y: f32,
    /// The last foundation verdict (placement gate).
    pub foundation_ok: bool,
    pub player: crate::player::PlayerBody,
    pub host: std::rc::Rc<std::cell::RefCell<pc3d_world::host::SoloHost>>,
    pub scene: std::rc::Rc<crate::slice::SliceScene>,
    pub save_root: std::rc::Rc<std::path::PathBuf>,
    pub world_name: String,
    pub inspect: bool,
    pub last_message: String,
}

/// A shared handle to the authoritative host for interactive construction:
/// F places a rock block 6 m ahead of the camera, R removes the built block
/// 6 m ahead — both through `HostCommand` + one tick, never by client-side
/// mutation. The renderer then syncs read-only from `host.construction`.
pub struct InteractiveHost(pub std::rc::Rc<std::cell::RefCell<pc3d_world::host::SoloHost>>);

pub struct WindowConfig {
    pub title: String,
    /// Logical size; the OS DPI scaling decides the physical surface size.
    pub logical_size: (f64, f64),
    /// Treat `logical_size` (and `resize_to`) as PHYSICAL pixels — proof
    /// runs want deterministic capture sizes on any DPI host.
    pub size_is_physical: bool,
    /// Stop after this many presented frames (None: run until closed).
    pub max_frames: Option<u64>,
    /// Scheduled captures (sorted by frame); the run ends after the last.
    pub shots: Vec<Shot>,
    /// Camera poses applied at given frames (automated proof runs).
    pub camera_script: Vec<(u64, CameraPose)>,
    /// One-shot callbacks at given frames (host edits + renderer sync).
    pub frame_hooks: Vec<(u64, FrameHook)>,
    /// Scripted UI state steps (screenshot harness / inspector runs).
    pub ui_script: Vec<(u64, UiStep)>,
    /// Probe selection for scheduled captures.
    pub probe_set: ProbeSet,
    /// Interactive construction (F place / R remove through the host).
    pub interactive_host: Option<InteractiveHost>,
    /// The live vertical slice (R3DV-011): walking player, host building,
    /// save/reload, inspect boxes.
    pub slice_host: Option<Box<SliceHost>>,
    /// Data-only setup: assembled into slice_host at window creation.
    pub slice_setup: Option<SliceSetup>,
    /// Owner-facing live shell: the real UI layer (title/pause/settings
    /// screens + gameplay HUD). Automated proof windows leave this false so
    /// old gate timing stays stable.
    pub owner_menu: bool,
    /// Test-only saves3d override (UI-006): the save-slot browser, settings
    /// persistence, and save/load all use this root instead of the
    /// manifest's saves3d when set.
    pub save_root_override: Option<std::rc::Rc<std::path::PathBuf>>,
    /// If set, the window is resized halfway to the first shot so the run
    /// proves live surface resize recovery before capturing.
    pub resize_to: Option<(f64, f64)>,
    /// Additional scheduled resizes (frame-indexed) for multi-size proof
    /// runs (the UI harness captures 1280x720 then the Deck 1280x800).
    pub resize_script: Vec<(u64, (f64, f64))>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "POORCRAFT 3D".into(),
            logical_size: (1280.0, 720.0),
            size_is_physical: false,
            max_frames: None,
            shots: Vec::new(),
            camera_script: Vec::new(),
            frame_hooks: Vec::new(),
            ui_script: Vec::new(),
            probe_set: ProbeSet::Scene,
            interactive_host: None,
            slice_host: None,
            slice_setup: None,
            owner_menu: false,
            save_root_override: None,
            resize_to: Some((800.0, 500.0)),
            resize_script: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PointerIntent {
    Grab,
    Release,
}

/// One completed live-window capture: the PNG path, its semantic report, the
/// decoded RGBA pixels (for cross-frame assertions like parallax), and — for
/// owner runs — the UI canvas + layout that were composited into it.
#[derive(Clone)]
pub struct CaptureOutcome {
    pub path: PathBuf,
    pub report: PixelReport,
    pub rgba: Vec<u8>,
    /// The UI canvas composited into this frame (straight-alpha RGBA), when
    /// the owner UI was active.
    pub ui_canvas: Option<(Vec<u8>, u32, u32)>,
    /// The UI layout dump for this frame (already also written to
    /// `Shot::ui_dump` when requested).
    pub ui_layout: Option<serde_json::Value>,
    /// The seed preview live at capture time (WT-001 sidecars/checks).
    pub seed_preview: Option<pc3d_world::seed_preview::SeedPreview>,
}

/// What a windowed run produced.
pub struct WindowReport {
    pub frames: u64,
    /// Capture outcomes in schedule order.
    pub captures: Vec<CaptureOutcome>,
    /// Frame times in milliseconds (presented frames only).
    pub frame_ms: Vec<f32>,
    /// Resized events the surface followed (resize-recovery evidence).
    pub resizes_observed: u32,
    /// Physical swapchain size at the end of the run.
    pub final_physical: (u32, u32),
    /// Window scale factor (physical = logical x scale).
    pub scale_factor: f64,
    /// Streaming counters at run end (when a streamer was attached).
    pub final_stream_counters: Option<crate::streaming::StreamCounters>,
    /// The final UI state (owner runs) — the inspector's ui_state answer.
    pub final_ui_state: Option<serde_json::Value>,
    /// WT-007 slice 1: the GPU marker audit of the final frame.
    pub gpu_marker_audit: Option<serde_json::Value>,
}

impl WindowReport {
    pub fn p50_ms(&self) -> f32 {
        percentile(&self.frame_ms, 50)
    }

    pub fn p95_ms(&self) -> f32 {
        percentile(&self.frame_ms, 95)
    }

    pub fn avg_fps(&self) -> f32 {
        if self.frame_ms.is_empty() {
            0.0
        } else {
            1000.0 / (self.frame_ms.iter().sum::<f32>() / self.frame_ms.len() as f32)
        }
    }
}

fn percentile(v: &[f32], p: u32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let mut v = v.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    let idx = ((v.len() as f32 - 1.0) * p as f32 / 100.0).round() as usize;
    v[idx.min(v.len() - 1)]
}

/// The hotbar build palette: slot selection is live state — F places the
/// SELECTED material. Order matches `HudValues::default().slots`.
const BUILD_PALETTE: [Option<pc3d_world::gen::CellMaterial>; 9] = [
    Some(pc3d_world::gen::CellMaterial::Soil),
    Some(pc3d_world::gen::CellMaterial::Grass),
    Some(pc3d_world::gen::CellMaterial::Sand),
    Some(pc3d_world::gen::CellMaterial::Rock),
    Some(pc3d_world::gen::CellMaterial::Snow),
    None,
    None,
    None,
    None,
];

struct WindowState {
    window: Arc<Window>,
    renderer: Renderer,
    frame_no: u64,
    frame_ms: Vec<f32>,
    consecutive_surface_errors: u32,
    resizes_observed: u32,
    final_physical: (u32, u32),
    captures: Vec<CaptureOutcome>,
    keys: HashSet<KeyCode>,
    pointer_grabbed: bool,
    owner_menu: bool,
    done: bool,
    /// Blocks in the interactive host's overlay (HUD readout).
    built_count: usize,

    // ---- The owner UI (owner_menu runs) ----
    ui: UiState,
    /// The last built draw list (hit tests + layout dumps).
    ui_list: Option<ui::DrawList>,
    ui_dirty: bool,
    /// Physical mouse position (hover).
    mouse_px: (f32, f32),
    /// Settings persistence path (inside the effective save root).
    settings_path: Option<PathBuf>,
    /// The effective saves3d root (override or the slice's).
    save_root: Option<std::rc::Rc<PathBuf>>,
    /// The window's current DPI scale (UI layout input).
    ui_dpi: f32,
}

impl WindowState {
    fn gameplay_active(&self) -> bool {
        if !self.owner_menu {
            true
        } else {
            !self.ui.blocks_gameplay()
        }
    }

    fn apply_pointer_intent(&mut self, intent: PointerIntent) {
        match intent {
            PointerIntent::Grab => self.grab_pointer(),
            PointerIntent::Release => self.release_pointer(),
        }
    }

    /// Reconcile the real pointer state with what the UI state believes.
    fn reconcile_pointer(&mut self) {
        if self.ui.pointer_grabbed != self.pointer_grabbed {
            if self.ui.pointer_grabbed {
                self.grab_pointer();
                if !self.pointer_grabbed {
                    // The OS refused the grab — keep the UI honest.
                    self.ui.pointer_grabbed = false;
                    self.ui_dirty = true;
                }
            } else {
                self.release_pointer();
            }
        }
    }

    fn grab_pointer(&mut self) {
        let grabbed = self
            .window
            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
            .or_else(|_| {
                self.window
                    .set_cursor_grab(winit::window::CursorGrabMode::Confined)
            })
            .is_ok();
        self.window.set_cursor_visible(!grabbed);
        self.pointer_grabbed = grabbed;
    }

    fn release_pointer(&mut self) {
        let _ = self
            .window
            .set_cursor_grab(winit::window::CursorGrabMode::None);
        self.window.set_cursor_visible(true);
        self.pointer_grabbed = false;
        self.keys.clear();
    }

    /// First-person movement from the current key set (4 m/s walk).
    fn apply_movement(&mut self, dt: f32) {
        let key = |k: KeyCode| if self.keys.contains(&k) { 1.0f32 } else { 0.0 };
        let fwd = key(KeyCode::KeyW) - key(KeyCode::KeyS);
        let strafe = key(KeyCode::KeyD) - key(KeyCode::KeyA);
        let vert = key(KeyCode::Space) - key(KeyCode::ShiftLeft);
        if fwd != 0.0 || strafe != 0.0 || vert != 0.0 {
            let step = self.renderer.camera_walk_step(fwd, strafe, vert, dt);
            let pos = self.renderer.pose().position;
            self.renderer.set_pose(CameraPose::new(
                [pos[0] + step[0], pos[1] + step[1], pos[2] + step[2]],
                self.renderer.pose().yaw,
                self.renderer.pose().pitch,
            ));
        }
    }

    fn hud_line(&self) -> String {
        let recent: Vec<f32> = self.frame_ms.iter().rev().take(30).copied().collect();
        let fps = if recent.is_empty() {
            0.0
        } else {
            1000.0 / (recent.iter().sum::<f32>() / recent.len() as f32)
        };
        let pose = self.renderer.pose();
        format!(
            "P3D POS {:.1} {:.1} {:.1} YAW {:.2} FPS {:.0} BUILT {}",
            pose.position[0], pose.position[1], pose.position[2], pose.yaw, fps, self.built_count
        )
    }

    fn fps(&self) -> f32 {
        let recent: Vec<f32> = self.frame_ms.iter().rev().take(30).copied().collect();
        if recent.is_empty() {
            0.0
        } else {
            1000.0 / (recent.iter().sum::<f32>() / recent.len() as f32)
        }
    }

    /// Rebuild + paint the UI canvas if dirty (or forced) and upload it.
    fn refresh_ui(&mut self) {
        if !self.owner_menu {
            return;
        }
        let (w, h) = self.renderer.size();
        if w == 0 || h == 0 {
            return;
        }
        // WT-001: the seed preview follows the form — recompute when the
        // reducer marked it stale or when New World opened without one.
        // Same text through the same resolver the world creation uses.
        // An EMPTY seed field shows the placeholder, never a fake map.
        if self.ui.screen == ui::Screen::NewWorld
            && (self.ui.seed_preview_stale || self.ui.seed_preview.is_none())
        {
            if self.ui.form.seed_digits.is_empty() {
                self.ui.seed_preview = None;
            } else {
                self.ui.seed_preview = Some(pc3d_world::seed_preview::preview_seed(
                    &pc3d_world::seed_preview::SeedPreviewRequest {
                        seed_text: self.ui.form.seed_digits.clone(),
                        ..Default::default()
                    },
                ));
            }
            self.ui.seed_preview_stale = false;
            self.ui_dirty = true;
        }
        if self.ui_dirty || self.ui_list.is_none() {
            let list = ui::build_dpi(&self.ui, w, h, self.ui_dpi);
            let canvas = ui::paint(&list);
            self.renderer.set_ui_layer(Some((canvas, w, h)));
            self.ui_list = Some(list);
            self.ui_dirty = false;
        }
    }

    /// Refresh the save-slot browser rows from the effective saves3d root.
    fn refresh_slots(&mut self) {
        let Some(root) = self.save_root.clone() else {
            return;
        };
        let dir = root.join(pc3d_core::P3D_SAVE_DIR);
        let mut slots = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let meta = e.file_name().to_string_lossy().into_owned();
                if meta.starts_with('.') {
                    continue;
                }
                let world_p3d = e.path().join("world.p3d");
                if !world_p3d.is_file() {
                    continue;
                }
                let seed = std::fs::read(&world_p3d)
                    .ok()
                    .and_then(|bytes| parse_world_seed(&bytes));
                let modified = e
                    .metadata()
                    .and_then(|m| m.modified())
                    .map(format_mtime)
                    .unwrap_or_else(|_| "unknown".into());
                slots.push(ui::SaveSlot { name: meta, seed, modified });
            }
        }
        slots.sort_by(|a, b| b.modified.cmp(&a.modified).then(a.name.cmp(&b.name)));
        self.ui.slots = slots;
        self.ui_dirty = true;
    }

    /// The F3 debug strip content (live values, hidden unless toggled).
    fn debug_text(&self, slice: Option<&SliceHost>) -> String {
        let pose = self.renderer.pose();
        let built = slice
            .map(|s| {
                s.host
                    .borrow()
                    .construction
                    .values()
                    .map(|c| c.built_count())
                    .sum::<usize>()
            })
            .unwrap_or(self.built_count);
        let seed = slice.map(|s| s.seed).unwrap_or(0);
        let world = slice.map(|s| s.world_name.as_str()).unwrap_or("-");
        format!(
            "P3D {} SEED {} WORLD {} POS {:.0} {:.0} {:.0} YAW {:.0} FPS {:.0} BUILT {}",
            env!("CARGO_PKG_VERSION"),
            seed,
            world,
            pose.position[0],
            pose.position[1],
            pose.position[2],
            pose.yaw.to_degrees(),
            self.fps(),
            built
        )
    }
}

/// world.p3d seed read without the save crate: frame = header (16 B, see
/// pc3d_core) + payload length (u64) + WorldMeta payload (seed u64 LE then
/// name). Fall back to None if unreadable; the rows tolerate it.
fn parse_world_seed(bytes: &[u8]) -> Option<u64> {
    let start = pc3d_core::HEADER_LEN + 8;
    if bytes.len() < start + 8 {
        return None;
    }
    let mut b = [0u8; 8];
    b.copy_from_slice(&bytes[start..start + 8]);
    Some(u64::from_le_bytes(b))
}

/// Civil-date formatting from a SystemTime without a chrono dependency
/// (Howard Hinnant's civil_from_days).
fn format_mtime(t: std::time::SystemTime) -> String {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (hh, mm) = (rem / 3600, (rem % 3600) / 60);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}")
}

struct App {
    cfg: WindowConfig,
    resize_plan: Option<(u64, f64, f64)>,
    next_script: usize,
    next_hook: usize,
    next_ui_step: usize,
    next_resize: usize,
    next_shot: usize,
    last_frame: Option<Instant>,
    state: Option<WindowState>,
}

impl App {
    fn finish(&self) -> WindowReport {
        match &self.state {
            Some(s) => WindowReport {
                frames: s.frame_no,
                captures: s.captures.clone(),
                frame_ms: s.frame_ms.clone(),
                resizes_observed: s.resizes_observed,
                final_physical: s.final_physical,
                scale_factor: s.window.scale_factor(),
                final_stream_counters: s.renderer.stream_counters(),
                final_ui_state: s.owner_menu.then(|| s.ui.to_json()),
                gpu_marker_audit: Some(
                    s.renderer
                        .gpu_marker_audit(percentile(&s.frame_ms, 50)),
                ),
            },
            None => WindowReport {
                frames: 0,
                captures: Vec::new(),
                frame_ms: Vec::new(),
                resizes_observed: 0,
                final_physical: (0, 0),
                scale_factor: 1.0,
                final_stream_counters: None,
                final_ui_state: None,
                gpu_marker_audit: None,
            },
        }
    }

    /// Execute UI actions through the same path real input takes. Needs the
    /// event loop for the (explicit) quit-to-desktop choice. Each arm holds
    /// the state borrow only as long as it needs — helpers re-borrow self.
    fn exec_actions(&mut self, actions: &[UiAction], event_loop: &ActiveEventLoop) {
        for act in actions {
            match act {
                UiAction::TryTalk => {
                    // The NPC talk slice: resolve the nearest LIVE brain
                    // in talk range and open the dialog with its line.
                    if let Some(s) = self.state.as_mut() {
                        if let Some((_, _, i)) = s.renderer.nearest_talk_target() {
                            if let Some(line) = s.renderer.talk_with_index(i) {
                                s.ui.dialog = Some(line);
                                s.ui_dirty = true;
                            }
                        }
                    }
                }
                UiAction::StartPlaying => {
                    if let Some(s) = self.state.as_mut() {
                        // The reducer sets these itself; script-driven
                        // actions land here directly — set unconditionally.
                        s.ui.screen = Screen::Gameplay;
                        s.ui.pointer_grabbed = true;
                        s.reconcile_pointer();
                    }
                }
                UiAction::OpenScreen(sc) => {
                    if let Some(s) = self.state.as_mut() {
                        // The reducer sets the screen itself before emitting
                        // this action; script-driven actions land here
                        // directly, so set it unconditionally (idempotent).
                        s.ui.screen = *sc;
                        if *sc == Screen::LoadWorld {
                            s.refresh_slots();
                        }
                        s.ui.session_live = true;
                        s.ui.focus = 0;
                        s.ui.hover = None;
                        s.ui_dirty = true;
                        s.reconcile_pointer();
                    }
                }
                UiAction::CloseModal => {
                    if let Some(s) = self.state.as_mut() {
                        s.ui_dirty = true;
                    }
                }
                UiAction::ConfirmModal(modal) => match modal {
                    ModalKind::DeleteWorld(name) => {
                        let gone = self.state.as_ref().and_then(|s| {
                            let p = s
                                .save_root
                                .as_ref()
                                .map(|r| r.join(pc3d_core::P3D_SAVE_DIR).join(name));
                            p.map(|p| std::fs::remove_dir_all(&p).is_ok())
                        });
                        if let Some(s) = self.state.as_mut() {
                            match gone {
                                Some(true) => s.ui.toast(format!("DELETED '{name}'")),
                                _ => s.ui.toast(format!("DELETE FAILED ('{name}')")),
                            }
                            s.refresh_slots();
                        }
                    }
                    ModalKind::LoadWorld(name) => {
                        self.load_world(name, event_loop);
                    }
                    ModalKind::QuitToDesktop => {
                        if let Some(s) = self.state.as_mut() {
                            s.done = true;
                        }
                        event_loop.exit();
                    }
                },
                UiAction::QuitToDesktop => {
                    if let Some(s) = self.state.as_mut() {
                        s.done = true;
                    }
                    event_loop.exit();
                }
                UiAction::QuitToTitle => {
                    if let Some(s) = self.state.as_mut() {
                        s.ui_dirty = true;
                        s.reconcile_pointer();
                    }
                }
                UiAction::SaveNow => {
                    self.save_world();
                }
                UiAction::LoadSlot(name) => {
                    self.load_world(name, event_loop);
                }
                UiAction::DeleteSlot(name) => {
                    if let Some(s) = self.state.as_mut() {
                        s.ui.modal = Some(ModalKind::DeleteWorld(name.clone()));
                        s.ui_dirty = true;
                    }
                }
                UiAction::CreateWorld { seed, name } => {
                    self.create_world(*seed, name.clone());
                }
                UiAction::SetSensitivity(_) | UiAction::SetInvertY(_) => {
                    // Applied live by the look handler; persist.
                    self.persist_settings();
                }
                UiAction::SetFov(deg) => {
                    if let Some(s) = self.state.as_mut() {
                        s.renderer.set_fov_y_deg(*deg);
                    }
                    self.persist_settings();
                }
                UiAction::SetUiScale(_) => {
                    if let Some(s) = self.state.as_mut() {
                        s.ui_dirty = true;
                    }
                    self.persist_settings();
                }
                UiAction::SetQuality(q) => {
                    let tier = match q {
                        ui::Quality::Low => crate::deck::DeckTier::Low,
                        ui::Quality::Mid => crate::deck::DeckTier::Mid,
                        ui::Quality::High => crate::deck::DeckTier::High,
                    };
                    if let Some(s) = self.state.as_mut() {
                        crate::deck::apply(&mut s.renderer, tier);
                    }
                    self.persist_settings();
                }
                UiAction::SelectHotbar(_) => {
                    // F builds with the selected slot's material — live.
                    if let Some(s) = self.state.as_mut() {
                        s.ui_dirty = true;
                    }
                }
                UiAction::CaptureMouse => {
                    if let Some(s) = self.state.as_mut() {
                        s.ui.pointer_grabbed = true;
                        s.reconcile_pointer();
                        s.ui_dirty = true;
                    }
                }
                UiAction::PlayerLook { dx, dy } => {
                    // Proof hook: the EXACT path real mouse motion takes —
                    // deltas into the live player body, then the frame
                    // loop's set_pose(player.pose()) must KEEP them.
                    if let Some(slice) = self.cfg.slice_host.as_mut() {
                        let sens = self
                            .state
                            .as_ref()
                            .map(|s| if s.owner_menu { s.ui.settings.mouse_sensitivity } else { 1.0 })
                            .unwrap_or(1.0);
                        let invert = self
                            .state
                            .as_ref()
                            .map(|s| s.owner_menu && s.ui.settings.invert_y)
                            .unwrap_or(false);
                        slice.player.apply_look(*dx, *dy, sens, invert);
                    }
                }
                UiAction::Repaint => {
                    if let Some(s) = self.state.as_mut() {
                        s.ui_dirty = true;
                    }
                }
            }
        }
    }

    fn save_world(&mut self) {
        let Some(state) = self.state.as_mut() else { return };
        let Some(slice) = self.cfg.slice_host.as_ref() else { return };
        let root = slice.save_root.clone();
        let r = crate::slice::save_slice(
            root.as_ref(),
            &slice.world_name,
            slice.seed,
            &slice.host.borrow(),
            &slice.player,
        );
        state.ui.toast(match r {
            Ok(()) => format!("SAVED '{}'", slice.world_name),
            Err(e) => format!("SAVE ERR {e:?}"),
        });
        state.ui_dirty = true;
    }

    fn load_world(&mut self, name: &str, _event_loop: &ActiveEventLoop) {
        let Some(state) = self.state.as_mut() else { return };
        let root = state
            .save_root
            .clone()
            .or_else(|| self.cfg.slice_host.as_ref().map(|s| s.save_root.clone()));
        let Some(root) = root else { return };
        match crate::slice::load_slice(root.as_ref(), name) {
            Ok((seed, host, player)) => {
                if let Some(slice) = self.cfg.slice_host.as_mut() {
                    slice.seed = seed;
                    *slice.host.borrow_mut() = host;
                    slice.player = player;
                    slice.world_name = name.to_string();
                    let h = slice.host.borrow();
                    state.renderer.update_construction(&h.construction);
                }
                state.ui.screen = Screen::Gameplay;
                state.ui.pointer_grabbed = true;
                state.ui.modal = None;
                state.ui.session_live = true;
                state.ui.toast(format!("LOADED '{name}'"));
                state.ui_dirty = true;
                state.reconcile_pointer();
            }
            Err(e) => {
                state.ui.modal = None;
                state.ui.toast(format!("LOAD FAILED: {e:?}"));
                state.ui_dirty = true;
            }
        }
    }

    /// Create World: search a showcase scene for the seed and assemble the
    /// full rebuild stack live, swapping the slice host in place.
    fn create_world(&mut self, seed: u64, name: String) {
        let Some(state) = self.state.as_mut() else { return };
        let (found_seed, scene) = crate::slice::find_showcase(seed);
        let scene = std::rc::Rc::new(scene);
        let root = state
            .save_root
            .clone()
            .or_else(|| self.cfg.slice_host.as_ref().map(|s| s.save_root.clone()))
            .unwrap_or_else(|| std::rc::Rc::new(PathBuf::from(".")));
        let host = crate::slice::assemble_rebuild(&mut state.renderer, &scene, found_seed, root.clone(), &name);
        self.cfg.slice_host = Some(Box::new(host));
        state.ui.screen = Screen::Gameplay;
        state.ui.pointer_grabbed = true;
        state.ui.modal = None;
        state.ui.session_live = true;
        state.ui.form.quality = state.ui.settings.quality;
        state.ui.toast(format!("WORLD '{name}' · SEED {found_seed}"));
        state.ui_dirty = true;
        state.reconcile_pointer();
    }

    fn persist_settings(&mut self) {
        let Some(state) = self.state.as_ref() else { return };
        let Some(path) = &state.settings_path else { return };
        let v = state.ui.settings.to_json();
        if let Ok(s) = serde_json::to_string_pretty(&v) {
            let _ = std::fs::write(path, s);
        }
    }

    fn load_settings(&mut self) {
        let Some(state) = self.state.as_mut() else { return };
        let Some(path) = state.settings_path.clone() else { return };
        let Some(text) = std::fs::read_to_string(&path).ok() else { return };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return };
        state.ui.settings = ui::UiSettings::from_json(&v);
        // Apply the loaded settings to the live renderer now.
        state.renderer.set_fov_y_deg(state.ui.settings.fov_deg);
        let tier = match state.ui.settings.quality {
            ui::Quality::Low => crate::deck::DeckTier::Low,
            ui::Quality::Mid => crate::deck::DeckTier::Mid,
            ui::Quality::High => crate::deck::DeckTier::High,
        };
        crate::deck::apply(&mut state.renderer, tier);
        state.ui_dirty = true;
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let attrs = WindowAttributes::new()
            .with_title(self.cfg.title.clone())
            .with_inner_size(if self.cfg.size_is_physical {
                winit::dpi::Size::Physical(winit::dpi::PhysicalSize::new(
                    self.cfg.logical_size.0 as u32,
                    self.cfg.logical_size.1 as u32,
                ))
            } else {
                winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(
                    self.cfg.logical_size.0,
                    self.cfg.logical_size.1,
                ))
            })
            .with_resizable(true);
        let window = event_loop.create_window(attrs).expect("create window");
        let window = Arc::new(window);
        let window_dpi = window.scale_factor() as f32;
        let mut renderer = Renderer::windowed(&window);
        if let Some(setup) = self.cfg.slice_setup.take() {
            let host = if setup.rebuild {
                crate::slice::assemble_rebuild(
                    &mut renderer,
                    &setup.scene,
                    setup.seed,
                    setup.save_root,
                    &setup.world_name,
                )
            } else {
                crate::slice::assemble(
                    &mut renderer,
                    &setup.scene,
                    setup.seed,
                    setup.save_root,
                    &setup.world_name,
                )
            };
            self.cfg.slice_host = Some(Box::new(host));
        }
        let built_count = self
            .cfg
            .interactive_host
            .as_ref()
            .map(|h| {
                h.0.borrow()
                    .construction
                    .values()
                    .map(|c| c.built_count())
                    .sum()
            })
            .unwrap_or(0);

        let save_root = self
            .cfg
            .save_root_override
            .clone()
            .or_else(|| self.cfg.slice_host.as_ref().map(|s| s.save_root.clone()));
        let settings_path = save_root.as_ref().map(|r| r.join("settings.json"));

        let mut state = WindowState {
            window,
            renderer,
            frame_no: 0,
            frame_ms: Vec::new(),
            consecutive_surface_errors: 0,
            resizes_observed: 0,
            final_physical: (0, 0),
            captures: Vec::new(),
            keys: HashSet::new(),
            pointer_grabbed: false,
            owner_menu: self.cfg.owner_menu,
            done: false,
            built_count,
            ui: UiState::default(),
            ui_list: None,
            ui_dirty: true,
            mouse_px: (0.0, 0.0),
            ui_dpi: window_dpi,
            settings_path,
            save_root,
        };
        if state.owner_menu {
            // Owner runs draw their whole HUD through the UI layer; the
            // legacy top-left debug line stays blank (hidden).
            state.renderer.set_hud_line("");
            state.ui.session_live = self.cfg.slice_setup.is_some() || self.cfg.slice_host.is_some();
            if state.ui.session_live {
                state.ui.hud.prompt = "F BUILD · R REMOVE · B SAVE · L LOAD · I INSPECT".into();
                // The talk prompt: name the villager in range, live.
                if state.ui.dialog.is_none() {
                    if let Some((name, _, _)) = state.renderer.nearest_talk_target() {
                        state.ui.hud.prompt = format!("E TALK {name} · F BUILD · R REMOVE · ESC PAUSE");
                    }
                }
            }
        }
        self.state = Some(state);
        if self.cfg.owner_menu {
            self.load_settings();
            if let Some(s) = self.state.as_mut() {
                s.refresh_slots();
            }
        }
    }

    /// Global mouse motion: used for first-person look while the pointer is
    /// grabbed and gameplay is active.
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            let Some(state) = &mut self.state else {
                return;
            };
            if !state.pointer_grabbed || !state.gameplay_active() {
                return;
            }
            // Guard against pointer-lock re-entry spikes.
            let (dx, dy) = (dx.clamp(-100.0, 100.0), dy.clamp(-100.0, 100.0));
            // Settings drive the look: sensitivity + invert Y (UI-005).
            let sens = if state.owner_menu {
                state.ui.settings.mouse_sensitivity
            } else {
                1.0
            };
            let invert = state.owner_menu && state.ui.settings.invert_y;
            let dy_signed = if invert { -dy } else { dy };
            // THE MOUSE FIX: in the live slice the BODY owns the camera —
            // the frame loop calls set_pose(player.pose()) every frame,
            // which used to clobber renderer-pose look the instant it
            // happened (the mouse did NOTHING). Look writes the body
            // through the ONE shared definition.
            if let Some(slice) = self.cfg.slice_host.as_mut() {
                slice
                    .player
                    .apply_look(dx as f32, dy as f32, sens, invert);
            } else {
                let pose = state.renderer.pose();
                let mut body = crate::player::PlayerBody {
                    pos: pose.position,
                    yaw: pose.yaw,
                    pitch: pose.pitch,
                };
                body.apply_look(dx as f32, dy as f32, sens, invert);
                state
                    .renderer
                    .set_pose(CameraPose::new(body.pos, body.yaw, body.pitch));
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = &mut self.state else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: winit::keyboard::Key::Named(NamedKey::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                if state.owner_menu {
                    // Escape routes through the UI: gameplay pauses, pause
                    // resumes, sub-screens go back — it NEVER exits.
                    let actions = ui::on_key(&mut state.ui, Key::Escape);
                    state.ui_dirty = true;
                    drop(state);
                    self.exec_actions(&actions, event_loop);
                } else {
                    event_loop.exit();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            // Owner UI owns navigation keys first.
                            if state.owner_menu {
                                if let Some(key) = ui_key(code) {
                                    let actions = ui::on_key(&mut state.ui, key);
                                    state.ui_dirty = true;
                                    // Some keys double as gameplay keys; the
                                    // UI consumes them when a menu is open.
                                    let menu_open = state.ui.blocks_gameplay();
                                    let is_nav = matches!(
                                        code,
                                        KeyCode::Enter
                                            | KeyCode::NumpadEnter
                                            | KeyCode::ArrowUp
                                            | KeyCode::ArrowDown
                                            | KeyCode::ArrowLeft
                                            | KeyCode::ArrowRight
                                            | KeyCode::Backspace
                                            | KeyCode::F3
                                            | KeyCode::KeyQ
                                    );
                                    if menu_open || is_nav {
                                        drop(state);
                                        self.exec_actions(&actions, event_loop);
                                        return;
                                    }
                                    drop(state);
                                    self.exec_actions(&actions, event_loop);
                                    let Some(state) = self.state.as_mut() else {
                                        return;
                                    };
                                    if state.done {
                                        return;
                                    }
                                    // Fall through to gameplay handling below.
                                    state.keys.insert(code);
                                    self.gameplay_key(code, event_loop);
                                    return;
                                }
                                // Non-UI keys in gameplay: fall through.
                            }
                            state.keys.insert(code);
                            self.gameplay_key(code, event_loop);
                        }
                        ElementState::Released => {
                            state.keys.remove(&code);
                        }
                    };
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if state.owner_menu && !state.pointer_grabbed {
                    state.mouse_px = (position.x as f32, position.y as f32);
                    let (mx, my) = (position.x as i32, position.y as i32);
                    if let Some(list) = state.ui_list.clone() {
                        if ui::on_mouse_move(&mut state.ui, mx, my, &list) {
                            state.ui_dirty = true;
                        }
                    }
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                if state.owner_menu {
                    let (mx, my) = (state.mouse_px.0 as i32, state.mouse_px.1 as i32);
                    let actions = state
                        .ui_list
                        .as_ref()
                        .map(|list| ui::on_click(&mut state.ui, mx, my, list))
                        .unwrap_or_default();
                    state.ui_dirty = true;
                    drop(state);
                    self.exec_actions(&actions, event_loop);
                } else {
                    // Click to look around: lock the pointer, FPS-style.
                    state.grab_pointer();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if state.owner_menu && !state.ui.blocks_gameplay() {
                    let up = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y > 0.0,
                        MouseScrollDelta::PixelDelta(p) => p.y > 0.0,
                    };
                    let key = if up { Key::WheelUp } else { Key::WheelDown };
                    let actions = ui::on_key(&mut state.ui, key);
                    drop(state);
                    self.exec_actions(&actions, event_loop);
                }
            }
            // Resize + scale-factor changes both land here; the renderer
            // reconfigures the surface (and clamps 0-sized events).
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(state) = self.state.as_mut() {
                    state.ui_dpi = scale_factor as f32;
                    state.ui_dirty = true;
                }
            }
            WindowEvent::Resized(size) => {
                state.resizes_observed += 1;
                state.final_physical = (size.width, size.height);
                state.renderer.resize(size.width, size.height);
                state.ui_dirty = true;
            }
            WindowEvent::RedrawRequested => {
                self.redraw(event_loop);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        if let Some(state) = &self.state {
            if !state.done {
                state.window.request_redraw();
            }
        }
    }
}

impl App {
    /// Gameplay key handling shared by owner and legacy paths (F/R build,
    /// B/L save, I inspect, interactive construction).
    fn gameplay_key(&mut self, code: KeyCode, event_loop: &ActiveEventLoop) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        if state.owner_menu && state.ui.blocks_gameplay() {
            return;
        }
        if let Some(slice) = self.cfg.slice_host.as_mut() {
            let gen = slice.scene.gen.clone();
            match code {
                KeyCode::KeyF | KeyCode::KeyR => {
                    // Owner mode: F builds with the SELECTED hotbar material.
                    let material = if state.owner_menu {
                        match BUILD_PALETTE.get(state.ui.hud.selected).copied().flatten() {
                            Some(m) => m,
                            None => {
                                if code == KeyCode::KeyF {
                                    state.ui.toast("EMPTY SLOT - PICK A MATERIAL (1-5)");
                                    state.ui_dirty = true;
                                    return;
                                }
                                pc3d_world::gen::CellMaterial::Sand
                            }
                        }
                    } else {
                        pc3d_world::gen::CellMaterial::Sand
                    };
                    let target = slice.player.ray_target(&gen, 8.0);
                    if let Some((hit, air)) = target {
                        let cell = if code == KeyCode::KeyF { air } else { hit };
                        let mut h = slice.host.borrow_mut();
                        if code == KeyCode::KeyF && slice.rebuild {
                            // NWR-011: construction on an INSPECTED
                            // foundation — steep or unlevel ground rejects
                            // with the named reason.
                            let region = crate::surface::SurfaceRegion::new(
                                gen.clone(),
                                pc3d_world::coords::PatchCoord {
                                    x: cell.x.div_euclid(16),
                                    y: cell.y.max(1),
                                    z: cell.z.div_euclid(16),
                                },
                            );
                            let verdict =
                                crate::world_features::check_foundation(&gen, &region, cell, (1, 1));
                            let rejected = matches!(
                                verdict,
                                crate::world_features::FoundationCheck::Rejected { .. }
                            );
                            if let crate::world_features::FoundationCheck::Rejected { reason } = verdict {
                                slice.last_message = format!("FOUNDATION REJECTED: {reason}");
                                if state.owner_menu {
                                    state.ui.toast(format!("REJECTED: {reason}"));
                                    state.ui_dirty = true;
                                }
                            }
                            slice.foundation_ok = !rejected;
                        }
                        let foundation_ok = slice.foundation_ok;
                        if code == KeyCode::KeyF && foundation_ok {
                            h.submit(pc3d_world::host::HostCommand::Build {
                                cell,
                                material,
                                owner: 7,
                            });
                        } else {
                            h.submit(pc3d_world::host::HostCommand::RemoveBuild {
                                cell,
                                owner: 7,
                            });
                        }
                        h.run_ticks(1);
                        state.renderer.update_construction(&h.construction);
                        slice.last_message = format!(
                            "{} {:?}",
                            if code == KeyCode::KeyF { "PLACED" } else { "REMOVED" },
                            cell
                        );
                        if state.owner_menu {
                            let msg = if code == KeyCode::KeyF {
                                "PLACED".to_string()
                            } else {
                                "REMOVED".to_string()
                            };
                            state.ui.toast(msg);
                            state.ui_dirty = true;
                        }
                    } else {
                        slice.last_message = "NO TARGET IN REACH".into();
                        if state.owner_menu {
                            state.ui.toast("NO TARGET IN REACH");
                            state.ui_dirty = true;
                        }
                    }
                }
                KeyCode::KeyB => {
                    let root = slice.save_root.clone();
                    let r = crate::slice::save_slice(
                        root.as_ref(),
                        &slice.world_name,
                        slice.seed,
                        &slice.host.borrow(),
                        &slice.player,
                    );
                    slice.last_message = match r {
                        Ok(()) => "SAVED".into(),
                        Err(e) => format!("SAVE ERR {e:?}"),
                    };
                    if state.owner_menu {
                        state.ui.toast(format!("SAVED '{}'", slice.world_name));
                        state.ui_dirty = true;
                    }
                }
                KeyCode::KeyL => {
                    let root = slice.save_root.clone();
                    let name = slice.world_name.clone();
                    match crate::slice::load_slice(root.as_ref(), &name) {
                        Ok((seed, host, player)) => {
                            slice.seed = seed;
                            *slice.host.borrow_mut() = host;
                            slice.player = player;
                            let h = slice.host.borrow();
                            state.renderer.update_construction(&h.construction);
                            slice.last_message = "RELOADED".into();
                            if state.owner_menu {
                                state.ui.toast("RELOADED");
                                state.ui_dirty = true;
                            }
                        }
                        Err(e) => {
                            slice.last_message = format!("LOAD ERR {e:?}");
                            if state.owner_menu {
                                state.ui.toast("LOAD FAILED - NOTHING SAVED FOR THIS WORLD");
                                state.ui_dirty = true;
                            }
                        }
                    }
                }
                KeyCode::KeyI => {
                    slice.inspect = !slice.inspect;
                    slice.last_message = if slice.inspect {
                        "INSPECT ON".into()
                    } else {
                        "INSPECT OFF".into()
                    };
                    if state.owner_menu {
                        state.ui.toast(slice.last_message.clone());
                        state.ui_dirty = true;
                    }
                }
                _ => {}
            }
        }
        // Interactive construction: F places, R removes — through the HOST
        // command path, one tick, then a read-only renderer sync.
        if let Some(host) = &self.cfg.interactive_host {
            let target_cell = match code {
                KeyCode::KeyF | KeyCode::KeyR => {
                    let pose = state.renderer.pose();
                    let fwd = crate::camera::fwd_of(pose.yaw, pose.pitch);
                    let t = [
                        pose.position[0] + fwd[0] * 6.0,
                        pose.position[1] + fwd[1] * 6.0,
                        pose.position[2] + fwd[2] * 6.0,
                    ];
                    Some(pc3d_world::coords::CellCoord {
                        x: t[0].floor() as i32,
                        y: t[1].floor() as i32,
                        z: t[2].floor() as i32,
                    })
                }
                _ => None,
            };
            if let Some(cell) = target_cell {
                let mut h = host.0.borrow_mut();
                match code {
                    KeyCode::KeyF => {
                        h.submit(pc3d_world::host::HostCommand::Build {
                            cell,
                            material: pc3d_world::gen::CellMaterial::Rock,
                            owner: 7,
                        })
                    }
                    _ => h.submit(pc3d_world::host::HostCommand::RemoveBuild {
                        cell,
                        owner: 7,
                    }),
                };
                h.run_ticks(1);
                state.renderer.update_construction(&h.construction);
                state.built_count =
                    h.construction.values().map(|c| c.built_count()).sum();
            }
        }
        let _ = event_loop;
    }

    /// Runs every UI script step due at the current frame (several steps
    /// can share a frame). Returns false when the run should stop (quit
    /// action fired).
    fn run_ui_script(&mut self, event_loop: &ActiveEventLoop) -> bool {
        loop {
            if self.next_ui_step >= self.cfg.ui_script.len() {
                return true;
            }
            let fire = {
                let Some(state) = self.state.as_ref() else { return true };
                let (frame, _) = &self.cfg.ui_script[self.next_ui_step];
                state.frame_no == *frame
            };
            if !fire {
                return true;
            }
            let actions = {
                let (_, mut step) = std::mem::replace(
                    &mut self.cfg.ui_script[self.next_ui_step],
                    (0, Box::new(|_, _, _| {})),
                );
                let Some(state) = self.state.as_mut() else { return true };
                let mut ctx = UiFrameCtx {
                    frame: state.frame_no,
                    p50_ms: percentile(&state.frame_ms, 50),
                    fps: state.fps(),
                    actions: Vec::new(),
                };
                if state.owner_menu {
                    step(&mut state.ui, &mut state.renderer, &mut ctx);
                    state.ui_dirty = true;
                }
                self.next_ui_step += 1;
                std::mem::take(&mut ctx.actions)
            };
            if !actions.is_empty() {
                self.exec_actions(&actions, event_loop);
            }
            let Some(state) = self.state.as_mut() else { return false };
            if state.done {
                event_loop.exit();
                return false;
            }
            state.reconcile_pointer();
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let dt = match self.last_frame {
            Some(t) => t.elapsed().as_secs_f32().min(0.1),
            None => 1.0 / 60.0,
        };
        self.last_frame = Some(Instant::now());

        // Scheduled camera script (teleport poses for proof runs).
        if self.next_script < self.cfg.camera_script.len() {
            let (frame, pose) = self.cfg.camera_script[self.next_script];
            if state.frame_no == frame {
                state.renderer.set_pose(pose);
                self.next_script += 1;
            }
        }

        // Scheduled one-shot frame hooks (host-driven world edits +
        // read-only renderer sync), before this frame's capture.
        if self.next_hook < self.cfg.frame_hooks.len() {
            let (frame, _) = &self.cfg.frame_hooks[self.next_hook];
            if state.frame_no == *frame {
                let (_, mut hook) = std::mem::replace(
                    &mut self.cfg.frame_hooks[self.next_hook],
                    (0, Box::new(|_| {})),
                );
                hook(&mut state.renderer);
                self.next_hook += 1;
            }
        }

        // Scheduled UI script steps (screenshot harness / inspector). The
        // step's actions execute through the same path as real input.
        if !self.run_ui_script(event_loop) {
            return;
        }
        let Some(state) = self.state.as_mut() else {
            return;
        };

        // Scheduled mid-run resize: proves live surface recovery before the
        // captures run.
        if let Some((frame, w, h)) = self.resize_plan {
            if state.frame_no == frame {
                let size = if self.cfg.size_is_physical {
                    winit::dpi::Size::Physical(winit::dpi::PhysicalSize::new(w as u32, h as u32))
                } else {
                    winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(w, h))
                };
                let _ = state.window.request_inner_size(size);
                self.resize_plan = None;
            }
        }

        // Scripted multi-size resizes (the UI harness's Deck pass).
        if self.next_resize < self.cfg.resize_script.len() {
            let (frame, (w, h)) = self.cfg.resize_script[self.next_resize];
            if state.frame_no == frame {
                let size = if self.cfg.size_is_physical {
                    winit::dpi::Size::Physical(winit::dpi::PhysicalSize::new(w as u32, h as u32))
                } else {
                    winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(w, h))
                };
                let _ = state.window.request_inner_size(size);
                self.next_resize += 1;
            }
        }

        // The live slice: per-frame WALKING on colliding terrain, the
        // camera locked to the player, streaming continues.
        let gameplay_active = state.gameplay_active();
        let mut sprinting = false;
        let moving;
        if let Some(slice) = self.cfg.slice_host.as_mut() {
            let key = |k: KeyCode| state.keys.contains(&k) as i32 as f32;
            let fwd = if gameplay_active {
                key(KeyCode::KeyW) - key(KeyCode::KeyS)
            } else {
                0.0
            };
            let strafe = if gameplay_active {
                key(KeyCode::KeyD) - key(KeyCode::KeyA)
            } else {
                0.0
            };
            moving = gameplay_active && (fwd != 0.0 || strafe != 0.0);
            let gen = slice.scene.gen.clone();
            // SPRINT (the controls spec's Shift): only while moving and
            // only while stamina holds — exhausted bodies must recover
            // to 25% before sprinting again (no flicker at empty).
            let shift = gameplay_active && state.keys.contains(&KeyCode::ShiftLeft);
            let wants_sprint = shift && moving && !state.ui.hud.exhausted;
            sprinting = wants_sprint && state.ui.hud.stamina > 0.0;
            let speed = if slice.rebuild {
                if sprinting {
                    crate::player::SPRINT_SPEED
                } else {
                    crate::player::WALK_SPEED
                }
            } else {
                crate::player::WALK_SPEED
            };
            if slice.rebuild {
                // The NWR-011 walk: on the STREAMED SURFACE at the
                // sprint-aware speed, plus the schedule ticking the
                // crowd's authoritative brains.
                state
                    .renderer
                    .walk_player_surface_speed(&gen, &mut slice.player, fwd, strafe, dt, speed);
                // JUMP (the controls spec's Space): a real minimal hop —
                // vertical velocity + gravity, clamped to the ground.
                if gameplay_active {
                    let grounded = slice.jump_vy <= 0.0;
                    if state.keys.contains(&KeyCode::Space) && grounded && !slice.jump_held {
                        slice.jump_vy = 4.6;
                    }
                    slice.jump_held = state.keys.contains(&KeyCode::Space);
                    if slice.jump_vy > 0.0 || slice.player.pos[1] > slice.ground_y + 0.01 {
                        slice.jump_vy -= 9.8 * dt;
                        slice.player.pos[1] = (slice.player.pos[1] + slice.jump_vy * dt).max(slice.ground_y);
                        if slice.player.pos[1] <= slice.ground_y {
                            slice.player.pos[1] = slice.ground_y;
                            slice.jump_vy = 0.0;
                        }
                    }
                    if slice.jump_vy <= 0.0 {
                        // Standing on ground: remember it for the next hop.
                        slice.ground_y = slice.player.pos[1];
                    }
                }
                if gameplay_active {
                    state.renderer.crowd_tick(0.35, 1);
                }
            } else {
                slice.player.walk(&gen, fwd, strafe, dt);
            }
            state.renderer.set_pose(slice.player.pose());
            // The NPC talk slice, live: while no dialog is open, the
            // prompt names the villager in talk range (E to speak).
            if state.owner_menu && state.ui.dialog.is_none() && !state.ui.blocks_gameplay() {
                match state.renderer.nearest_talk_target() {
                    Some((name, _, _)) => {
                        state.ui.hud.prompt =
                            format!("E TALK {name} · F BUILD · R REMOVE · ESC PAUSE");
                        state.ui_dirty = true;
                    }
                    None => {
                        let base = "F BUILD · R REMOVE · B SAVE · L LOAD · I INSPECT";
                        if state.ui.hud.prompt != base {
                            state.ui.hud.prompt = base.into();
                            state.ui_dirty = true;
                        }
                    }
                }
            }
            let built: usize = slice
                .host
                .borrow()
                .construction
                .values()
                .map(|c| c.built_count())
                .sum();
            if !state.owner_menu {
                state.renderer.set_hud_line(&format!(
                    "SLICE {} POS {:.0} {:.0} {:.0} BUILT {} | {}",
                    slice.seed,
                    slice.player.pos[0],
                    slice.player.pos[1],
                    slice.player.pos[2],
                    built,
                    slice.last_message
                ));
            } else if state.ui.debug_overlay {
                state.ui.debug_text = state.debug_text(Some(slice));
            }
            let _ = state.renderer.stream_frame();
            let _ = state.renderer.surface_stream_frame();
        } else {
            // Interactive movement (free flight), then streaming.
            if gameplay_active {
                state.apply_movement(dt);
            }
            let _ = state.renderer.stream_frame();
            if !state.owner_menu {
                state.renderer.set_hud_line(&state.hud_line());
            }
            moving = false;
        }

        // Owner UI live state: vitals drift with movement, toasts fade,
        // hover follows the pointer. Repaint only when something changed.
        if state.owner_menu {
            state.ui.tick_toasts(dt);
            if state.ui.screen == Screen::Gameplay && !state.ui.blocks_gameplay() {
                // THE STAMINA FIX: stamina is a SPRINT resource — walking
                // is free; sprinting (Shift while moving) drains it at
                // 0.22/s; rest regenerates at 0.14/s; hitting empty
                // locks sprint out until 25% recovery (no flicker).
                // Food drains slowly with travel; health stays full
                // until damage systems exist (honest placeholder).
                let hud = &mut state.ui.hud;
                let before = (hud.stamina * 100.0) as i32 * 100 + (hud.food * 100.0) as i32;
                hud.tick_vitals(sprinting, moving, dt);
                let after = (hud.stamina * 100.0) as i32 * 100 + (hud.food * 100.0) as i32;
                if before != after {
                    state.ui_dirty = true;
                }
                let grabbed_before = state.ui.pointer_grabbed;
                if grabbed_before != state.pointer_grabbed {
                    state.ui.pointer_grabbed = state.pointer_grabbed;
                    state.ui_dirty = true;
                }
            }
            if !state.ui.toasts.is_empty() {
                state.ui_dirty = true;
            }
            state.refresh_ui();
        }

        // Scheduled capture replaces this frame's presentation (the
        // swapchain texture is the copy source); ends after the last.
        let next_shot = self.cfg.shots.get(self.next_shot).cloned();
        if let Some(shot) = next_shot {
            if state.frame_no == shot.frame {
                let pose = state.renderer.pose();
                let aspect = state.renderer.aspect();
                let probes = match (self.cfg.probe_set, shot.sky) {
                    (_, false) => vec![],
                    (ProbeSet::Scene, _) => crate::scene::probes_for_pose(pose, aspect),
                    (ProbeSet::SkyOnly, _) => vec![crate::scene::Probe {
                        name: "sky_above_horizon",
                        ndc: (0.0, 0.8),
                        expected: crate::scene::to_srgb4(crate::scene::sky_color_linear(
                            crate::scene::dir_from_ndc(pose, (0.0, 0.8), aspect),
                            crate::scene::SUN_DIR,
                        )),
                        tol: 0.05,
                    }],
                };
                let (report, rgba) = state.renderer.capture_png(&shot.path, &probes);
                let (ui_canvas, ui_layout) = state
                    .ui_list
                    .as_ref()
                    .map(|list| {
                        (
                            state
                                .renderer
                                .ui_canvas_copy()
                                .map(|(b, w, h)| (b.to_vec(), w, h)),
                            Some(list.to_json(state.ui.screen.as_str())),
                        )
                    })
                    .unwrap_or((None, None));
                if let Some(dump) = &shot.ui_dump {
                    if let Some(layout) = &ui_layout {
                        let mut v = layout.clone();
                        v["ui_state"] = state.ui.to_json();
                        let _ = std::fs::write(
                            dump,
                            serde_json::to_string_pretty(&v).unwrap_or_default(),
                        );
                    }
                }
                state.captures.push(CaptureOutcome {
                    path: shot.path,
                    report,
                    rgba,
                    ui_canvas,
                    ui_layout,
                    seed_preview: state.ui.seed_preview.clone(),
                });
                self.next_shot += 1;
                state.frame_no += 1;
                if self.next_shot >= self.cfg.shots.len() {
                    state.done = true;
                }
                if state.done {
                    event_loop.exit();
                }
                return;
            }
        }

        let t0 = Instant::now();
        match state.renderer.render_frame() {
            Ok(()) => state.consecutive_surface_errors = 0,
            Err(err) => {
                state.consecutive_surface_errors += 1;
                if state.consecutive_surface_errors > 120 {
                    eprintln!(
                        "[FAIL] surface unusable for 120 consecutive frames: {err:?}"
                    );
                    state.done = true;
                }
            }
        }
        state.frame_ms.push(t0.elapsed().as_secs_f32() * 1000.0);
        state.frame_no += 1;

        if self.cfg.max_frames == Some(state.frame_no) {
            state.done = true;
        }
        if state.done {
            event_loop.exit();
        }
    }

}

/// Map winit keycodes onto the UI's abstract key set (None: not a UI key).
fn ui_key(code: KeyCode) -> Option<Key> {
    Some(match code {
        KeyCode::Enter | KeyCode::NumpadEnter => Key::Enter,
        KeyCode::ArrowUp => Key::Up,
        KeyCode::ArrowDown => Key::Down,
        KeyCode::ArrowLeft => Key::Left,
        KeyCode::ArrowRight => Key::Right,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::F3 => Key::F3,
        KeyCode::KeyQ => Key::KeyQ,
        KeyCode::KeyE => Key::Char('e'),
        KeyCode::Digit1 | KeyCode::Numpad1 => Key::Digit(1),
        KeyCode::Digit2 | KeyCode::Numpad2 => Key::Digit(2),
        KeyCode::Digit3 | KeyCode::Numpad3 => Key::Digit(3),
        KeyCode::Digit4 | KeyCode::Numpad4 => Key::Digit(4),
        KeyCode::Digit5 | KeyCode::Numpad5 => Key::Digit(5),
        KeyCode::Digit6 | KeyCode::Numpad6 => Key::Digit(6),
        KeyCode::Digit7 | KeyCode::Numpad7 => Key::Digit(7),
        KeyCode::Digit8 | KeyCode::Numpad8 => Key::Digit(8),
        KeyCode::Digit9 | KeyCode::Numpad9 => Key::Digit(9),
        KeyCode::Digit0 | KeyCode::Numpad0 => Key::Digit(0),
        _ => return None,
    })
}

/// Opens the window and pumps frames until closed, the frame budget runs
/// out, or the last scheduled capture completes. Must be called from the
/// main thread (winit requirement on macOS/Windows).
pub fn run_windowed(cfg: WindowConfig) -> Result<WindowReport, String> {
    if let (Some(first), false) = (cfg.shots.first(), cfg.shots.is_empty()) {
        if cfg.max_frames.is_some_and(|m| m <= first.frame) {
            return Err("max_frames must exceed the last shot frame".into());
        }
    }
    // Schedule the live resize halfway to the first capture so a screenshot
    // run always proves surface resize recovery before it captures.
    let resize_plan = match (cfg.shots.first(), cfg.resize_to) {
        (Some(shot), Some((w, h))) => Some((shot.frame / 2, w, h)),
        _ => None,
    };
    let event_loop = EventLoop::new().map_err(|e| format!("event loop: {e}"))?;
    let mut app = App {
        cfg,
        resize_plan,
        next_script: 0,
        next_hook: 0,
        next_ui_step: 0,
        next_resize: 0,
        next_shot: 0,
        last_frame: None,
        state: None,
    };
    event_loop
        .run_app(&mut app)
        .map_err(|e| format!("event loop error: {e}"))?;
    Ok(app.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_pauses_instead_of_exiting() {
        let mut s = UiState::default();
        s.screen = Screen::Gameplay;
        let acts = ui::on_key(&mut s, Key::Escape);
        assert_eq!(s.screen, Screen::Pause);
        assert!(s.blocks_gameplay());
        assert!(!acts.iter().any(|a| matches!(a, UiAction::QuitToDesktop)));
    }

    #[test]
    fn owner_escape_from_title_stays_on_title() {
        let mut s = UiState::default();
        s.screen = Screen::Title;
        let _ = ui::on_key(&mut s, Key::Escape);
        assert_eq!(s.screen, Screen::Title);
    }

    #[test]
    fn owner_resume_requests_pointer_grab() {
        let mut s = UiState::default();
        s.screen = Screen::Pause;
        let acts = ui::on_key(&mut s, Key::Escape);
        assert_eq!(s.screen, Screen::Gameplay);
        assert!(s.pointer_grabbed);
        assert!(acts.contains(&UiAction::StartPlaying));
        assert!(!s.blocks_gameplay());
    }

    #[test]
    fn gameplay_input_blocked_under_every_menu() {
        let mut s = UiState::default();
        s.screen = Screen::Gameplay;
        assert!(!s.blocks_gameplay());
        for sc in [Screen::Title, Screen::Pause, Screen::Settings, Screen::NewWorld, Screen::LoadWorld] {
            s.screen = sc;
            assert!(s.blocks_gameplay(), "{sc:?} must block gameplay input");
        }
        // And under an open modal on gameplay.
        s.screen = Screen::Gameplay;
        s.modal = Some(ModalKind::QuitToDesktop);
        assert!(s.blocks_gameplay());
    }

    #[test]
    fn mtime_formats_as_civil_date() {
        // Verified UTC stamps (python datetime.utcfromtimestamp).
        let t = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_788_931_200);
        assert_eq!(format_mtime(t), "2026-09-09 05:20");
        let t = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_788_939_001);
        assert_eq!(format_mtime(t), "2026-09-09 07:30");
        // Midnight boundaries cross the date correctly.
        let t = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_788_912_000);
        assert_eq!(format_mtime(t), "2026-09-09 00:00");
    }

    #[test]
    fn world_seed_parses_from_the_framed_meta() {
        use pc3d_core::{FormatHeader, SupportedVersions};
        // frame = header + len + payload
        let meta = {
            let mut p = Vec::new();
            p.extend_from_slice(&22u64.to_le_bytes());
            p.extend_from_slice(b"alpha");
            let h = FormatHeader {
                save: SupportedVersions::epoch1().save,
                ..FormatHeader::current()
            };
            let mut out = h.encode().to_vec();
            out.extend_from_slice(&(p.len() as u64).to_le_bytes());
            out.extend_from_slice(&p);
            out
        };
        assert_eq!(parse_world_seed(&meta), Some(22));
        assert_eq!(parse_world_seed(&[0u8; 10]), None);
    }

    #[test]
    fn build_palette_matches_default_hotbar() {
        // Slots 0..4 carry materials; the rest are reserved-empty.
        assert_eq!(BUILD_PALETTE.iter().filter(|p| p.is_some()).count(), 5);
        assert!(BUILD_PALETTE[8].is_none());
    }
}
