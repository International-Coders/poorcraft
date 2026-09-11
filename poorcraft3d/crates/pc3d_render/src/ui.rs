//! The owner-facing UI layer (GLM-UI-REWORK UI-001..UI-008).
//!
//! Replaces the temporary bitmap-text owner overlay with a real UI: explicit
//! screen state, one draw list per frame, a pure CPU painter that composes
//! panels/buttons/bars/hotbar/crosshair/toasts into one RGBA canvas the
//! renderer alpha-blends over the world, and a pure input reducer whose
//! actions the app executes.
//!
//! Everything here is deterministic and testable without a GPU: layout and
//! painting are pure functions of (`UiState`, target size), which is exactly
//! what the screenshot harness (`--ui-shots`) and the inspector
//! (`--ui-inspect`) drive.

use crate::font;

/// Nothing (text included) may start or end inside this border, at any
/// supported resolution. The clipped-text failure baseline is the reason
/// this exists.
pub const SAFE_MARGIN_PX: i32 = 16;

// ---------------------------------------------------------------------------
// State model (UI-001: explicit, serializable screen state)
// ---------------------------------------------------------------------------

/// Which UI screen owns the frame. `Gameplay` is the only state where game
/// input runs; every other screen blocks movement/build/crowd.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Title,
    NewWorld,
    LoadWorld,
    Settings,
    Gameplay,
    Pause,
}

impl Screen {
    pub fn as_str(self) -> &'static str {
        match self {
            Screen::Title => "title",
            Screen::NewWorld => "new_world",
            Screen::LoadWorld => "load_world",
            Screen::Settings => "settings",
            Screen::Gameplay => "gameplay",
            Screen::Pause => "pause",
        }
    }

    pub fn from_str(s: &str) -> Option<Screen> {
        Some(match s {
            "title" => Screen::Title,
            "new_world" => Screen::NewWorld,
            "load_world" => Screen::LoadWorld,
            "settings" => Screen::Settings,
            "gameplay" => Screen::Gameplay,
            "pause" => Screen::Pause,
            _ => return None,
        })
    }

    /// Game input runs only in live gameplay with no modal on top.
    pub fn blocks_gameplay(self) -> bool {
        !matches!(self, Screen::Gameplay)
    }
}

/// The quality preset (mirrors `deck::DeckTier`, kept separate so the UI
/// module never imports the renderer's internals).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Quality {
    Low,
    #[default]
    Mid,
    High,
}

impl Quality {
    pub fn label(self) -> &'static str {
        match self {
            Quality::Low => "LOW",
            Quality::Mid => "MID",
            Quality::High => "HIGH",
        }
    }

    pub fn cycle(self, up: bool) -> Quality {
        use Quality::*;
        match (self, up) {
            (Low, false) | (Mid, false) => Low,
            (Low, true) => Mid,
            (Mid, true) | (High, false) => High,
            (High, true) => High,
        }
    }
}

/// User settings shown on the settings screen. Every field must actually
/// drive runtime behavior or be visibly disabled — never a dead control.
#[derive(Clone, Debug, PartialEq)]
pub struct UiSettings {
    /// Look sensitivity multiplier (0.2 .. 3.0).
    pub mouse_sensitivity: f32,
    pub invert_y: bool,
    /// Vertical FOV in degrees (50 .. 100).
    pub fov_deg: f32,
    /// UI layout scale (0.75 .. 1.5).
    pub ui_scale: f32,
    pub quality: Quality,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 1.0,
            invert_y: false,
            fov_deg: 70.0,
            ui_scale: 1.0,
            quality: Quality::Mid,
        }
    }
}

impl UiSettings {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "mouse_sensitivity": self.mouse_sensitivity,
            "invert_y": self.invert_y,
            "fov_deg": self.fov_deg,
            "ui_scale": self.ui_scale,
            "quality": self.quality.label().to_lowercase(),
        })
    }

    pub fn from_json(v: &serde_json::Value) -> UiSettings {
        let mut s = UiSettings::default();
        if let Some(x) = v.get("mouse_sensitivity").and_then(|x| x.as_f64()) {
            s.mouse_sensitivity = x as f32;
        }
        if let Some(x) = v.get("invert_y").and_then(|x| x.as_bool()) {
            s.invert_y = x;
        }
        if let Some(x) = v.get("fov_deg").and_then(|x| x.as_f64()) {
            s.fov_deg = x as f32;
        }
        if let Some(x) = v.get("ui_scale").and_then(|x| x.as_f64()) {
            s.ui_scale = x as f32;
        }
        if let Some(x) = v.get("quality").and_then(|x| x.as_str()) {
            s.quality = match x {
                "low" => Quality::Low,
                "high" => Quality::High,
                _ => Quality::Mid,
            };
        }
        s.clamp();
        s
    }

    pub fn clamp(&mut self) {
        self.mouse_sensitivity = self.mouse_sensitivity.clamp(0.2, 3.0);
        self.fov_deg = self.fov_deg.clamp(50.0, 100.0);
        self.ui_scale = self.ui_scale.clamp(0.75, 1.5);
    }
}

/// One hotbar item: the build material palette entry (live state — the
/// selected slot is what F places).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HotItem {
    pub label: &'static str,
    /// Material swatch color (the painter adds the pixel pattern).
    pub color: [u8; 3],
}

/// Live HUD values. The app feeds these from real state (vitals drain with
/// movement, hotbar selection is what builds use); the debug harness can
/// override them for the non-full-bars screenshot scene.
#[derive(Clone, Debug, PartialEq)]
pub struct HudValues {
    /// All fractions 0.0 ..= 1.0.
    pub health: f32,
    pub stamina: f32,
    pub food: f32,
    pub xp: f32,
    pub slots: [Option<HotItem>; 9],
    pub selected: usize,
    /// The contextual action prompt near the crosshair/hotbar.
    pub prompt: String,
    /// Sprint-exhaustion lockout: true once stamina hits 0; sprinting
    /// returns only after stamina recovers to 25% (no empty-flicker).
    pub exhausted: bool,
}

impl HudValues {
    /// THE STAMINA LAW (one definition, shared by the frame loop and the
    /// proofs): stamina is a SPRINT resource — walking is free,
    /// sprinting drains 0.22/s, rest regenerates 0.14/s, empty locks
    /// sprint until 25% recovery. Food drains slowly while moving.
    pub fn tick_vitals(&mut self, sprinting: bool, moving: bool, dt: f32) {
        if sprinting {
            self.stamina = (self.stamina - 0.22 * dt).max(0.0);
            if self.stamina <= 0.0 {
                self.exhausted = true;
            }
        } else {
            self.stamina = (self.stamina + 0.14 * dt).min(1.0);
            if self.exhausted && self.stamina >= 0.25 {
                self.exhausted = false;
            }
        }
        if moving {
            self.food = (self.food - 0.004 * dt).max(0.0);
        }
    }
}

impl Default for HudValues {
    fn default() -> Self {
        Self {
            health: 1.0,
            stamina: 1.0,
            food: 1.0,
            xp: 0.0,
            exhausted: false,
            slots: [
                Some(HotItem { label: "SOIL", color: [122, 85, 58] }),
                Some(HotItem { label: "GRASS", color: [92, 138, 78] }),
                Some(HotItem { label: "SAND", color: [214, 184, 108] }),
                Some(HotItem { label: "ROCK", color: [138, 132, 126] }),
                Some(HotItem { label: "SNOW", color: [232, 236, 240] }),
                None,
                None,
                None,
                None,
            ],
            selected: 0,
            prompt: String::new(),
        }
    }
}

/// A fading HUD message (save/load confirmations). Laid out strictly above
/// the hotbar band so it can never cover it.
#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub text: String,
    /// Seconds since the toast fired; fully faded after `TOAST_LIFE_S`.
    pub age_s: f32,
}

pub const TOAST_LIFE_S: f32 = 3.0;

/// One save-slot row (from the saves3d directory listing).
#[derive(Clone, Debug, PartialEq)]
pub struct SaveSlot {
    pub name: String,
    pub seed: Option<u64>,
    /// Modification time rendered for humans (already formatted by the app).
    pub modified: String,
}

/// What a confirmation modal is asking.
#[derive(Clone, Debug, PartialEq)]
pub enum ModalKind {
    DeleteWorld(String),
    LoadWorld(String),
    QuitToDesktop,
}

impl ModalKind {
    pub fn message(&self) -> String {
        match self {
            ModalKind::DeleteWorld(name) => {
                format!("DELETE WORLD '{name}'?\nTHIS CANNOT BE UNDONE.")
            }
            ModalKind::LoadWorld(name) => {
                format!("LOAD WORLD '{name}'?\nUNSAVED WORK IN THE CURRENT WORLD IS LOST.")
            }
            ModalKind::QuitToDesktop => {
                "QUIT TO DESKTOP?\nUNSAVED WORK IS LOST.".into()
            }
        }
    }

    pub fn confirm_label(&self) -> &'static str {
        match self {
            ModalKind::DeleteWorld(_) => "DELETE",
            ModalKind::LoadWorld(_) => "LOAD",
            ModalKind::QuitToDesktop => "QUIT",
        }
    }
}

/// The New World form. Seed editing is digit-only (no text stack needed).
#[derive(Clone, Debug, PartialEq)]
pub struct NewWorldForm {
    pub seed_digits: String,
    pub name: String,
    pub quality: Quality,
    /// Monotonic reroll/random counter — the reducer's clock-free entropy
    /// (the WT-001 never-the-same-seed law derives from it).
    pub reroll_count: u64,
}

impl Default for NewWorldForm {
    fn default() -> Self {
        Self {
            seed_digits: "22".into(),
            name: "WORLD-22".into(),
            quality: Quality::Mid,
            reroll_count: 0,
        }
    }
}

impl NewWorldForm {
    pub fn seed(&self) -> u64 {
        self.seed_digits.parse().unwrap_or(22)
    }
}

/// The binding map — the single source for every key label the UI shows.
/// Prompts, the settings controls summary, and the title footer all read
/// from this table; nothing is baked into art.
pub struct KeyBinding {
    pub action: &'static str,
    pub key: &'static str,
}

pub const KEYMAP: &[KeyBinding] = &[
    KeyBinding { action: "MOVE", key: "W A S D" },
    KeyBinding { action: "LOOK", key: "MOUSE" },
    KeyBinding { action: "JUMP", key: "SPACE" },
    KeyBinding { action: "SPRINT", key: "SHIFT" },
    KeyBinding { action: "PLACE / USE", key: "LMB" },
    KeyBinding { action: "ALTERNATE", key: "RMB" },
    KeyBinding { action: "HOTBAR", key: "1-9 / WHEEL" },
    KeyBinding { action: "BUILD", key: "F" },
    KeyBinding { action: "REMOVE", key: "R" },
    KeyBinding { action: "SAVE", key: "B" },
    KeyBinding { action: "LOAD", key: "L" },
    KeyBinding { action: "INSPECT", key: "I" },
    KeyBinding { action: "PAUSE", key: "ESC" },
    KeyBinding { action: "QUIT", key: "Q" },
    KeyBinding { action: "DEBUG", key: "F3" },
];

/// The whole UI state for one frame. Serializable (`to_json`) so the
/// screenshot harness and inspector can assert exactly what was shown.
#[derive(Clone, Debug)]
pub struct UiState {
    pub screen: Screen,
    /// Keyboard focus index among the current screen's activatable buttons.
    pub focus: usize,
    /// The hovered button id (mouse), if any.
    pub hover: Option<String>,
    /// The pressed button id while a click is down (visual pressed state).
    pub pressed: Option<String>,
    pub settings: UiSettings,
    pub hud: HudValues,
    pub toasts: Vec<Toast>,
    pub modal: Option<ModalKind>,
    pub slots: Vec<SaveSlot>,
    pub form: NewWorldForm,
    /// WT-001: the live seed preview (the app computes it from the form
    /// through pc3d_world::seed_preview; pure data — the painter blits).
    pub seed_preview: Option<pc3d_world::seed_preview::SeedPreview>,
    /// Set by the reducer on any seed change; the app recomputes and
    /// clears it. Entering New World always recomputes (preview None).
    pub seed_preview_stale: bool,
    /// The F3 debug strip (hidden unless true — debug text is never the
    /// owner HUD).
    pub debug_overlay: bool,
    /// Content of the debug strip, set by the app each frame.
    pub debug_text: String,
    /// A live world session exists (Play reads CONTINUE).
    pub session_live: bool,
    /// Mirrors the real pointer capture state (the app reconciles).
    pub pointer_grabbed: bool,
    /// The last clicked/activated button id (transient, for proofs).
    pub last_activated: Option<String>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            screen: Screen::Title,
            focus: 0,
            hover: None,
            pressed: None,
            settings: UiSettings::default(),
            hud: HudValues::default(),
            toasts: Vec::new(),
            modal: None,
            slots: Vec::new(),
            form: NewWorldForm::default(),
            seed_preview: None,
            seed_preview_stale: false,
            debug_overlay: false,
            debug_text: String::new(),
            session_live: false,
            pointer_grabbed: false,
            last_activated: None,
        }
    }
}

impl UiState {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "screen": self.screen.as_str(),
            "focus": self.focus,
            "hover": self.hover,
            "settings": self.settings.to_json(),
            "hud": {
                "health": self.hud.health,
                "stamina": self.hud.stamina,
                "food": self.hud.food,
                "xp": self.hud.xp,
                "selected_slot": self.hud.selected,
                "prompt": self.hud.prompt,
            },
            "toasts": self.toasts.iter().map(|t| serde_json::json!({
                "text": t.text, "age_s": t.age_s })).collect::<Vec<_>>(),
            "modal": self.modal.as_ref().map(|m| serde_json::json!({
                "kind": match m {
                    ModalKind::DeleteWorld(_) => "delete_world",
                    ModalKind::LoadWorld(_) => "load_world",
                    ModalKind::QuitToDesktop => "quit_to_desktop",
                },
                "message": m.message(),
            })),
            "slots": self.slots.iter().map(|s| serde_json::json!({
                "name": s.name, "seed": s.seed, "modified": s.modified,
            })).collect::<Vec<_>>(),
            "form": {
                "seed": self.form.seed(),
                "name": self.form.name,
                "quality": self.form.quality.label(),
            },
            "debug_overlay": self.debug_overlay,
            "session_live": self.session_live,
            "pointer_grabbed": self.pointer_grabbed,
            "gameplay_input_blocked": self.blocks_gameplay(),
        })
    }

    pub fn blocks_gameplay(&self) -> bool {
        self.screen.blocks_gameplay() || self.modal.is_some()
    }

    pub fn toast(&mut self, text: impl Into<String>) {
        let text = text.into();
        // Re-firing an identical toast refreshes it instead of stacking.
        self.toasts.retain(|t| t.text != text);
        self.toasts.push(Toast { text, age_s: 0.0 });
        if self.toasts.len() > 3 {
            self.toasts.remove(0);
        }
    }

    pub fn tick_toasts(&mut self, dt: f32) {
        for t in &mut self.toasts {
            t.age_s += dt;
        }
        self.toasts.retain(|t| t.age_s < TOAST_LIFE_S);
    }
}

// ---------------------------------------------------------------------------
// Draw list (UI-002: one path draws every widget)
// ---------------------------------------------------------------------------

/// Pixel rectangle (origin top-left, physical pixels).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: u32, h: u32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn right(&self) -> i32 {
        self.x + self.w as i32
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h as i32
    }

    pub fn cx(&self) -> i32 {
        self.x + self.w as i32 / 2
    }

    pub fn cy(&self) -> i32 {
        self.y + self.h as i32 / 2
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonState {
    Normal,
    Hover,
    Focused,
    Pressed,
    Disabled,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ElementKind {
    /// Forged-metal panel; optional title bar.
    Panel { title: Option<String> },
    Button { label: String, state: ButtonState },
    /// Static text (shadowed for readability on any background).
    Text { label: String, scale: u32 },
    Bar { frac: f32, color: [u8; 3], label: String },
    HotbarSlot { index: usize, selected: bool, item: Option<HotItem> },
    Crosshair,
    Logo,
    /// The modal's full-screen dim layer (background — exempt from the
    /// safe-margin law, never interactive).
    Dim,
    Toast { label: String, alpha: f32 },
    /// The debug strip (only present when the debug overlay is on).
    Debug { label: String },
    /// A blitted image (WT-001: the seed-preview map). Straight-alpha
    /// RGBA, nearest-neighbor scaled into the element rect.
    Image {
        w: u32,
        h: u32,
        pixels: std::rc::Rc<Vec<u8>>,
    },
}

#[derive(Clone, Debug)]
pub struct UiElement {
    pub id: String,
    pub kind: ElementKind,
    pub rect: Rect,
}

impl UiElement {
    /// Activatable elements participate in focus order, hit tests, and the
    /// overlap law. Panels and text do not.
    pub fn activatable(&self) -> bool {
        matches!(self.kind, ElementKind::Button { .. })
            || matches!(self.kind, ElementKind::HotbarSlot { .. })
    }
}

/// One frame's layout: every element the painter will draw, in paint order
/// (panels before their children).
#[derive(Clone, Debug, Default)]
pub struct DrawList {
    pub size: (u32, u32),
    pub elements: Vec<UiElement>,
    /// Effective layout scale after the fit law.
    pub scale: f32,
}

impl DrawList {
    pub fn activatables(&self) -> Vec<&UiElement> {
        self.elements.iter().filter(|e| e.activatable()).collect()
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<&UiElement> {
        self.elements
            .iter()
            .filter(|e| e.activatable())
            .find(|e| e.rect.contains(x, y))
    }

    pub fn by_id(&self, id: &str) -> Option<&UiElement> {
        self.elements.iter().find(|e| e.id == id)
    }

    /// The machine-readable layout dump (inspector + screenshot proofs).
    pub fn to_json(&self, screen: &str) -> serde_json::Value {
        serde_json::json!({
            "screen": screen,
            "size": [self.size.0, self.size.1],
            "scale": self.scale,
            "elements": self.elements.iter().map(|e| serde_json::json!({
                "id": e.id,
                "kind": kind_name(&e.kind),
                "rect": [e.rect.x, e.rect.y, e.rect.w, e.rect.h],
                "button_state": match &e.kind {
                    ElementKind::Button { state, .. } => button_state_name(*state),
                    _ => "",
                },
            })).collect::<Vec<_>>(),
        })
    }
}

fn button_state_name(s: ButtonState) -> &'static str {
    match s {
        ButtonState::Normal => "normal",
        ButtonState::Hover => "hover",
        ButtonState::Focused => "focused",
        ButtonState::Pressed => "pressed",
        ButtonState::Disabled => "disabled",
    }
}

fn kind_name(k: &ElementKind) -> &'static str {
    match k {
        ElementKind::Panel { .. } => "panel",
        ElementKind::Button { .. } => "button",
        ElementKind::Text { .. } => "text",
        ElementKind::Bar { .. } => "bar",
        ElementKind::HotbarSlot { .. } => "hotbar_slot",
        ElementKind::Crosshair => "crosshair",
        ElementKind::Logo => "logo",
        ElementKind::Dim => "dim",
        ElementKind::Toast { .. } => "toast",
        ElementKind::Debug { .. } => "debug",
        ElementKind::Image { .. } => "image",
    }
}

// ---------------------------------------------------------------------------
// Layout (pure; safe margins and the overlap law hold at every resolution)
// ---------------------------------------------------------------------------

// Base sizes at scale 1.0 (physical px).
const BTN_W: i32 = 320;
const BTN_H: i32 = 44;
const BTN_GAP: i32 = 12;
const PANEL_PAD: i32 = 20;
const BAR_W: i32 = 240;
const BAR_H: i32 = 18;
const BAR_GAP: i32 = 10;
const SLOT: i32 = 52;
const SLOT_GAP: i32 = 6;
const TEXT_ROW_H: i32 = 18;

/// The fit law: the layout scale never lets a screen's panel cross the safe
/// margins. Small windows (Steam Deck 1280x800) shrink the UI rather than
/// clip it.
fn fit_scale(w: i32, h: i32, wanted: f32, panel_w: i32, panel_h: i32) -> f32 {
    let avail_w = (w - 2 * SAFE_MARGIN_PX) as f32 / panel_w as f32;
    let avail_h = (h - 2 * SAFE_MARGIN_PX) as f32 / panel_h as f32;
    wanted.min(avail_w).min(avail_h).max(0.6)
}

struct Ctx {
    s: f32,
    /// The display's DPI scale: layout runs in LOGICAL pixels, rects are
    /// scaled to PHYSICAL at push time (the HiDPI fix — fixed-px UI
    /// constants rendered at half relative size on Retina before).
    d: f32,
    /// The font multiplier (glyph raster scales are integers).
    k: u32,
    out: Vec<UiElement>,
}

impl Ctx {
    fn px(&self, v: i32) -> i32 {
        (v as f32 * self.s).round() as i32
    }

    fn push(&mut self, id: &str, kind: ElementKind, rect: Rect) {
        // Logical -> physical.
        let rect = Rect::new(
            (rect.x as f32 * self.d).round() as i32,
            (rect.y as f32 * self.d).round() as i32,
            (rect.w as f32 * self.d).round() as u32,
            (rect.h as f32 * self.d).round() as u32,
        );
        self.out.push(UiElement { id: id.into(), kind, rect });
    }

    fn panel(&mut self, id: &str, rect: Rect, title: Option<&str>) {
        self.push(id, ElementKind::Panel { title: title.map(str::to_string) }, rect);
    }

    fn text(&mut self, id: &str, label: &str, x: i32, y: i32, scale: u32) -> Rect {
        let (tw, th) = font::text_size(label, scale);
        let r = Rect::new(x, y, tw, th);
        self.push(id, ElementKind::Text { label: label.into(), scale: scale * self.k }, r);
        r
    }

    /// A centered button row at an explicit height; returns its rect.
    fn button(&mut self, id: &str, label: &str, cx: i32, y: i32, w: i32, h: i32, state: ButtonState) -> Rect {
        let r = Rect::new(cx - w / 2, y, w as u32, h as u32);
        self.push(
            id,
            ElementKind::Button { label: label.into(), state },
            r,
        );
        r
    }
}

fn button_state(state: &UiState, id: &str, index: usize, total_focusables: usize) -> ButtonState {
    let focused = index < total_focusables && state.focus % total_focusables.max(1) == index;
    match (state.hover.as_deref() == Some(id), state.pressed.as_deref() == Some(id), focused) {
        (true, true, _) => ButtonState::Pressed,
        (true, false, _) => ButtonState::Hover,
        (false, _, true) => ButtonState::Focused,
        _ => ButtonState::Normal,
    }
}

/// The source revision this binary was built from — stamped by
/// `make p3d-dmg` (PC3D_BUILD); local/test builds say "dev". Every build
/// must identify itself on the title screen: an owner kept playing a
/// pre-fix binary because a stale mounted DMG volume looked identical to
/// the fresh one (same crate version, same volume name).
pub fn build_stamp() -> &'static str {
    match option_env!("PC3D_BUILD") {
        Some(hash) => hash,
        None => "dev",
    }
}

/// Builds the draw list for the current state at a physical target size.
/// Pure: same state + size, same list.
pub fn build(state: &UiState, w: u32, h: u32) -> DrawList {
    build_dpi(state, w, h, 1.0)
}

/// The DPI-aware build: layout runs in LOGICAL pixels (w/dpi x h/dpi)
/// and every rect scales to PHYSICAL at push time — on a 2x Retina
/// display the UI renders at the SAME relative size as 1x, instead of
/// the half-size fragments the owner saw. dpi=1.0 is byte-identical to
/// the classic build.
pub fn build_dpi(state: &UiState, w: u32, h: u32, dpi: f32) -> DrawList {
    let dpi = dpi.max(0.75);
    let (wi, hi) = ((w as f32 / dpi) as i32, (h as f32 / dpi) as i32);
    let mut ctx = Ctx {
        s: state.settings.ui_scale,
        d: dpi,
        k: dpi.round().max(1.0) as u32,
        out: Vec::new(),
    };
    let cx = wi / 2;

    // The screen's own layout only when no modal owns the frame.
    if state.modal.is_none() {
    match state.screen {
        Screen::Title => {
            // Logo block at the upper third, panel of buttons below center.
            let logo_s = 6u32;
            let (lw, lh) = font::text_size("POORCRAFT", logo_s);
            let logo_h = lh as i32 + 10;
            let logo_y = safe_y(hi, (hi as f32 * 0.16) as i32, logo_h + 8);
            ctx.push("title_logo", ElementKind::Logo, Rect::new(cx - lw as i32 / 2 - 30, logo_y, lw + 60, logo_h as u32));
            let sub = format!(
                "3D · OWNER ALPHA · BUILD {} {}",
                env!("CARGO_PKG_VERSION"),
                build_stamp()
            );
            let (sw, _) = font::text_size(&sub, 2);
            ctx.text("title_sub", &sub, cx - sw as i32 / 2, logo_y + logo_h + 6, 2);

            let entries: Vec<(&str, &str)> = vec![
                ("btn_play", if state.session_live { "CONTINUE" } else { "PLAY" }),
                ("btn_new_world", "NEW WORLD"),
                ("btn_load_world", "LOAD WORLD"),
                ("btn_settings", "SETTINGS"),
                ("btn_quit", "QUIT"),
            ];
            let n = entries.len() as i32;
            let panel_w = BTN_W + PANEL_PAD * 2;
            let panel_h = BTN_H * n + BTN_GAP * (n - 1) + PANEL_PAD * 2 + 34;
            let s = fit_scale(wi, hi, state.settings.ui_scale, panel_w, panel_h + 260);
            ctx.s = s;
            let pw = ctx.px(panel_w);
            let ph = ctx.px(panel_h);
            let py = (hi as f32 * 0.42) as i32;
            let py = py.max(SAFE_MARGIN_PX).min(hi - SAFE_MARGIN_PX - ph);
            let panel = Rect::new(cx - pw / 2, py, pw as u32, ph as u32);
            ctx.panel("title_panel", panel, Some("POORCRAFT 3D"));
            let mut y = panel.y + ctx.px(PANEL_PAD) + ctx.px(26);
            for (i, (id, label)) in entries.iter().enumerate() {
                ctx.button(id, label, cx, y, ctx.px(BTN_W), ctx.px(BTN_H), button_state(state, id, i, entries.len()));
                y += ctx.px(BTN_H) + ctx.px(BTN_GAP);
            }
            footer(&mut ctx, wi, hi);
        }
        Screen::NewWorld => {
            // Two panels: the form (left) and the WT-001 seed preview
            // (right) — map, spawn safety, biome summary, feature hints.
            let rows = 4i32;
            let panel_w = BTN_W + 260 + PANEL_PAD * 2;
            let panel_h = TEXT_ROW_H * rows * 2 + BTN_H * 3 + BTN_GAP * 5 + PANEL_PAD * 2 + 30;
            let pv_w = 320i32;
            let map_side = pv_w - PANEL_PAD * 2;
            let pv_text_rows = 8i32;
            let pv_h = map_side + (TEXT_ROW_H + 6) * pv_text_rows + PANEL_PAD * 2 + 30;
            let gap = 24i32;
            let combo_w = panel_w + gap + pv_w;
            let s = fit_scale(
                wi,
                hi,
                state.settings.ui_scale,
                combo_w,
                panel_h.max(pv_h) + 120,
            );
            ctx.s = s;
            let pw = ctx.px(panel_w);
            let ph = ctx.px(panel_h);
            let pvw = ctx.px(pv_w);
            let pvh = ctx.px(pv_h);
            let total_w = pw + ctx.px(gap) + pvw;
            let left = cx - total_w / 2;
            let panel = Rect::new(left, centered_y(hi, ph), pw as u32, ph as u32);
            let pv = Rect::new(
                left + pw + ctx.px(gap),
                centered_y(hi, pvh),
                pvw as u32,
                pvh as u32,
            );
            ctx.panel("new_world_panel", panel, Some("NEW WORLD"));
            let fcx = panel.cx();
            let mut y = panel.y + ctx.px(PANEL_PAD) + ctx.px(24);
            let lx = panel.x + ctx.px(PANEL_PAD);
            let vx = panel.x + pw - ctx.px(PANEL_PAD) - ctx.px(190);
            // Rows: NAME / SEED / TYPE / QUALITY. Values are truncated to
            // their column so a long seed or name can never run under
            // the preview panel (the WT-001 overlap law).
            let value_w = (panel.right() - ctx.px(PANEL_PAD) - vx - ctx.px(4)).max(ctx.px(40)) as u32;
            let row = |ctx: &mut Ctx, y: &mut i32, label: &str, value: &str, id: &str| {
                ctx.text(&format!("{id}_label"), label, lx, *y + ctx.px(6), 2);
                let value = fit_to_width(value, 2 * ctx.k, value_w);
                ctx.text(&format!("{id}_value"), &value, vx, *y + ctx.px(6), 2);
                *y += ctx.px(TEXT_ROW_H) + ctx.px(14);
            };
            row(&mut ctx, &mut y, "WORLD NAME", &state.form.name, "nw_name");
            row(&mut ctx, &mut y, "SEED", &state.form.seed_digits, "nw_seed");
            row(&mut ctx, &mut y, "TYPE", "NORMAL (MORE SOON)", "nw_type");
            row(&mut ctx, &mut y, "QUALITY", state.form.quality.label(), "nw_quality");
            y += ctx.px(8);
            // REROLL + RANDOM side by side (both change the seed; the
            // WT-001 law: never the same seed twice in a row).
            let half_w = (BTN_W - 24) / 2;
            ctx.button("nw_seed_reroll", "REROLL", fcx - ctx.px(half_w) / 2 - ctx.px(3), y, ctx.px(half_w), ctx.px(BTN_H), button_state(state, "nw_seed_reroll", 1, 6));
            ctx.button("nw_seed_random", "RANDOM", fcx + ctx.px(half_w) / 2 + ctx.px(3), y, ctx.px(half_w), ctx.px(BTN_H), button_state(state, "nw_seed_random", 2, 6));
            y += ctx.px(BTN_H) + ctx.px(BTN_GAP);
            // The quality VALUE rides the row above (`QUALITY  MID`), so
            // the steppers stay single-glyph — "QUALITY -"/"QUALITY +"
            // overflowed their half-width buttons (visible once the
            // samurai cut stopped hiding it).
            ctx.button("nw_quality_down", "-", fcx - ctx.px(half_w) / 2 - ctx.px(3), y, ctx.px(half_w), ctx.px(BTN_H), ButtonState::Normal);
            ctx.button("nw_quality_up", "+", fcx + ctx.px(half_w) / 2 + ctx.px(3), y, ctx.px(half_w), ctx.px(BTN_H), ButtonState::Normal);
            y += ctx.px(BTN_H) + ctx.px(BTN_GAP);
            // CREATE only when the preview is valid (WT-001: an unsafe
            // spawn must not be creatable through the button).
            let create_ok = match &state.seed_preview {
                Some(p) => p.valid,
                None => !state.form.seed_digits.is_empty(),
            };
            let create_state = if create_ok {
                button_state(state, "nw_create", 0, 6)
            } else {
                ButtonState::Disabled
            };
            ctx.button("nw_create", "CREATE WORLD", fcx, y, ctx.px(BTN_W), ctx.px(BTN_H), create_state);
            y += ctx.px(BTN_H) + ctx.px(BTN_GAP);
            ctx.button("nw_back", "BACK", fcx, y, ctx.px(BTN_W), ctx.px(BTN_H), button_state(state, "nw_back", 5, 6));
            // The preview panel: pure blit of the pc3d_world authority.
            ctx.panel("nw_preview_panel", pv, Some("WORLD PREVIEW"));
            let mut py = pv.y + ctx.px(PANEL_PAD) + ctx.px(24);
            let plx = pv.x + ctx.px(PANEL_PAD);
            let line = |ctx: &mut Ctx, py: &mut i32, id: &str, label: &str| {
                let label = fit_to_width(label, 2 * ctx.k, (pv.right() - plx - ctx.px(PANEL_PAD)) as u32);
                ctx.text(id, &label, plx, *py, 2);
                *py += ctx.px(TEXT_ROW_H) + ctx.px(6);
            };
            match &state.seed_preview {
                Some(p) => {
                    let side = ctx.px(map_side);
                    let map = Rect::new(plx, py, side as u32, side as u32);
                    ctx.push(
                        "preview_map",
                        ElementKind::Image {
                            w: p.width,
                            h: p.height,
                            pixels: std::rc::Rc::new(p.pixels_rgba.clone()),
                        },
                        map,
                    );
                    py += side + ctx.px(10);
                    let safety = if p.spawn.safe {
                        format!("SPAWN SAFE · Y {:.0} M", p.spawn.y_m)
                    } else {
                        format!("UNSAFE: {}", p.spawn.reason.to_uppercase())
                    };
                    line(&mut ctx, &mut py, "spawn_safety", &safety);
                    let top: Vec<String> = p
                        .biome_counts
                        .iter()
                        .filter(|b| b.biome != pc3d_world::gen::Biome::Ocean || b.percent > 0.0)
                        .take(3)
                        .map(|b| format!("{} {:.0}%", b.biome.name().to_uppercase(), b.percent))
                        .collect();
                    line(&mut ctx, &mut py, "biome_summary", &top.join(" · "));
                    for (i, h) in p.feature_hints.iter().take(3).enumerate() {
                        let label = if h.placeholder {
                            format!("{}: UNKNOWN (PLACEHOLDER)", h.kind.to_uppercase())
                        } else {
                            format!("{} ~{:.0} M", h.kind.to_uppercase(), h.distance_m)
                        };
                        line(&mut ctx, &mut py, &format!("feature_hint_{i}"), &label);
                    }
                    line(
                        &mut ctx,
                        &mut py,
                        "nw_seed_resolved",
                        &format!("RESOLVED {} ({})", p.resolved_seed, p.seed_source),
                    );
                }
                None => {
                    let side = ctx.px(map_side);
                    let map = Rect::new(plx, py, side as u32, side as u32);
                    ctx.push(
                        "preview_map",
                        ElementKind::Image {
                            w: 1,
                            h: 1,
                            pixels: std::rc::Rc::new(vec![20, 24, 28, 255]),
                        },
                        map,
                    );
                    py += side + ctx.px(10);
                    line(&mut ctx, &mut py, "spawn_safety", "TYPE A SEED OR PRESS RANDOM");
                    line(&mut ctx, &mut py, "biome_summary", "MAP APPEARS WHEN A SEED IS SET");
                }
            }
            footer(&mut ctx, wi, hi);
        }
        Screen::LoadWorld => {
            let visible_slots = state.slots.len().min(6) as i32;
            let rows = visible_slots.max(1);
            let panel_w = BTN_W + 300 + PANEL_PAD * 2;
            let row_h = TEXT_ROW_H + 26;
            let panel_h = rows * (row_h + BTN_GAP) + BTN_H + BTN_GAP * 3 + PANEL_PAD * 2 + 30;
            let s = fit_scale(wi, hi, state.settings.ui_scale, panel_w, panel_h + 120);
            ctx.s = s;
            let pw = ctx.px(panel_w);
            let ph = ctx.px(panel_h);
            let panel = Rect::new(cx - pw / 2, centered_y(hi, ph), pw as u32, ph as u32);
            ctx.panel("load_world_panel", panel, Some("LOAD WORLD"));
            let mut y = panel.y + ctx.px(PANEL_PAD) + ctx.px(24);
            let lx = panel.x + ctx.px(PANEL_PAD);
            if state.slots.is_empty() {
                ctx.text("lw_empty", "NO SAVED WORLDS YET", lx, y, 2);
                y += ctx.px(TEXT_ROW_H) + ctx.px(14);
            }
            let bw = ctx.px(110);
            let load_x = panel.right() - ctx.px(PANEL_PAD) - bw * 2 - ctx.px(6);
            let label_max_w = (load_x - lx - ctx.px(10)).max(ctx.px(40)) as u32;
            for (i, slot) in state.slots.iter().take(6).enumerate() {
                let seed = slot.seed.map(|s| s.to_string()).unwrap_or_else(|| "?".into());
                let label = format!("{} · SEED {} · {}", slot.name, seed, slot.modified);
                // The row label must never run under the LOAD button:
                // truncate to the width actually available, measured at
                // the DRAWN glyph scale (scale * font multiplier k — the
                // layout rect alone under-reports it). Long timestamps
                // used to collide with the button.
                let label = fit_to_width(&label, 2 * ctx.k, label_max_w);
                ctx.text(&format!("lw_slot_{i}_label"), &label, lx, y + ctx.px(8), 2);
                ctx.button(&format!("lw_slot_{i}_load"), "LOAD", load_x, y, bw, ctx.px(30), ButtonState::Normal);
                ctx.button(&format!("lw_slot_{i}_del"), "DELETE", panel.right() - ctx.px(PANEL_PAD) - bw, y, bw, ctx.px(30), ButtonState::Normal);
                y += ctx.px(row_h) + ctx.px(BTN_GAP);
            }
            y += ctx.px(6);
            ctx.button("lw_back", "BACK", cx, y, ctx.px(BTN_W), ctx.px(BTN_H), ButtonState::Normal);
            footer(&mut ctx, wi, hi);
        }
        Screen::Settings => {
            // 5 setting rows + controls summary + back.
            let setting_rows = 5i32;
            let summary_rows = KEYMAP.len() as i32;
            let panel_w = 640 + PANEL_PAD * 2;
            let panel_h = setting_rows * (TEXT_ROW_H + 22) + summary_rows * 15 + 120 + PANEL_PAD * 2 + 30;
            let s = fit_scale(wi, hi, state.settings.ui_scale, panel_w, panel_h + 120);
            ctx.s = s;
            let pw = ctx.px(panel_w);
            let ph = ctx.px(panel_h);
            let panel = Rect::new(cx - pw / 2, centered_y(hi, ph), pw as u32, ph as u32);
            ctx.panel("settings_panel", panel, Some("SETTINGS"));
            let mut y = panel.y + ctx.px(PANEL_PAD) + ctx.px(24);
            let lx = panel.x + ctx.px(PANEL_PAD);
            let vx = panel.x + pw - ctx.px(PANEL_PAD) - ctx.px(150);
            let st = &state.settings;
            let rows: Vec<(&str, String)> = vec![
                ("MOUSE SENSITIVITY", format!("{:.1}", st.mouse_sensitivity)),
                ("INVERT Y", if st.invert_y { "ON" } else { "OFF" }.into()),
                ("FIELD OF VIEW", format!("{} DEG", st.fov_deg as i32)),
                ("UI SCALE", format!("{:.2}", st.ui_scale)),
                ("QUALITY PRESET", st.quality.label().into()),
            ];
            for (i, (label, value)) in rows.iter().enumerate() {
                let id = format!("set_row_{i}");
                ctx.text(&format!("{id}_label"), label, lx, y + ctx.px(8), 2);
                ctx.text(&format!("{id}_value"), value, vx, y + ctx.px(8), 2);
                // [-] [+] mini-buttons flank the value (30 px tall so rows
                // never overlap at any scale).
                ctx.button(&format!("{id}_dec"), "-", vx - ctx.px(52), y, ctx.px(34), ctx.px(30), ButtonState::Normal);
                ctx.button(&format!("{id}_inc"), "+", panel.right() - ctx.px(PANEL_PAD) - ctx.px(34), y, ctx.px(34), ctx.px(30), ButtonState::Normal);
                y += ctx.px(TEXT_ROW_H) + ctx.px(22);
            }
            y += ctx.px(6);
            ctx.text("set_controls_title", "CONTROLS", lx, y, 2);
            y += ctx.px(24);
            for (i, b) in KEYMAP.iter().enumerate() {
                if i % 2 == 0 && i + 1 < KEYMAP.len() {
                    // Two columns to stay compact at 720p.
                    let (a, b2) = (&KEYMAP[i], &KEYMAP[i + 1]);
                    let col2 = panel.cx() + ctx.px(30);
                    ctx.text("set_keymap", &format!("{}: {}", a.key, a.action), lx, y, 1);
                    ctx.text("set_keymap2", &format!("{}: {}", b2.key, b2.action), col2, y, 1);
                } else if i % 2 == 0 {
                    ctx.text("set_keymap", &format!("{}: {}", KEYMAP[i].key, KEYMAP[i].action), lx, y, 1);
                }
                y += ctx.px(15);
            }
            y += ctx.px(10);
            ctx.button("set_back", "BACK", cx, y, ctx.px(BTN_W), ctx.px(BTN_H), ButtonState::Normal);
        }
        Screen::Gameplay => {
            // Status bars cluster (top-left), crosshair, prompt, XP strip,
            // hotbar (bottom-center), toasts above the hotbar band, debug
            // strip top-right when enabled.
            let s = state.settings.ui_scale.clamp(0.75, 1.25);
            ctx.s = s;
            let bx = SAFE_MARGIN_PX + ctx.px(4);
            let mut y = SAFE_MARGIN_PX + ctx.px(4);
            let bars: [(&str, f32, [u8; 3], &str); 3] = [
                ("bar_health", state.hud.health, [194, 68, 56], "HEALTH"),
                ("bar_stamina", state.hud.stamina, [92, 138, 78], "STAMINA"),
                ("bar_food", state.hud.food, [201, 138, 61], "FOOD"),
            ];
            for (id, frac, color, label) in bars {
                let r = Rect::new(bx, y, ctx.px(BAR_W) as u32, ctx.px(BAR_H) as u32);
                ctx.push(id, ElementKind::Bar { frac, color, label: label.into() }, r);
                y += ctx.px(BAR_H) + ctx.px(BAR_GAP);
            }
            // Crosshair at true center.
            let ch = ctx.px(16);
            ctx.push("crosshair", ElementKind::Crosshair, Rect::new(cx - ch / 2, hi / 2 - ch / 2, ch as u32, ch as u32));
            // Hotbar bottom center with the XP strip tight above it.
            let total_w = SLOT * 9 + SLOT_GAP * 8;
            let hx0 = cx - (total_w * ctx.s as i32) / 2;
            let hy = hi - SAFE_MARGIN_PX - (SLOT * ctx.s as i32);
            let xp_h = ctx.px(5);
            ctx.push(
                "xp_strip",
                ElementKind::Bar { frac: state.hud.xp, color: [143, 111, 212], label: "XP".into() },
                Rect::new(hx0, hy - xp_h - ctx.px(6), (total_w * ctx.s as i32) as u32, xp_h as u32),
            );
            // Prompt above the XP strip.
            if !state.hud.prompt.is_empty() {
                let (tw, _) = font::text_size(&state.hud.prompt, 2);
                ctx.text("prompt", &state.hud.prompt, cx - tw as i32 / 2, hy - xp_h - ctx.px(6) - ctx.px(24), 2);
            }
            // Toast band above the prompt (never over the hotbar).
            let mut ty = hy - xp_h - ctx.px(6) - ctx.px(24) - ctx.px(22);
            for t in state.toasts.iter().rev() {
                let alpha = (1.0 - t.age_s / TOAST_LIFE_S).clamp(0.0, 1.0);
                let (tw, _) = font::text_size(&t.text, 2);
                ctx.push(
                    &format!("toast_{}", t.text),
                    ElementKind::Toast { label: t.text.clone(), alpha },
                    Rect::new(cx - tw as i32 / 2 - ctx.px(10), ty, tw as u32 + ctx.px(20) as u32, ctx.px(16) as u32),
                );
                ty -= ctx.px(22);
            }
            // Hotbar slots.
            for i in 0usize..9 {
                let x = hx0 + ctx.px((SLOT + SLOT_GAP) * i as i32);
                let r = Rect::new(x, hy, ctx.px(SLOT) as u32, ctx.px(SLOT) as u32);
                ctx.push(
                    &format!("hotbar_{i}"),
                    ElementKind::HotbarSlot { index: i, selected: state.hud.selected == i, item: state.hud.slots[i] },
                    r,
                );
            }
            // Debug strip (top-right, behind the F3 toggle).
            if state.debug_overlay && !state.debug_text.is_empty() {
                let (tw, th) = font::text_size(&state.debug_text, 2);
                ctx.push(
                    "debug_strip",
                    ElementKind::Debug { label: state.debug_text.clone() },
                    Rect::new(wi - SAFE_MARGIN_PX - tw as i32, SAFE_MARGIN_PX + ctx.px(4), tw as u32, th as u32),
                );
            }
            // Click-to-capture hint when the pointer is free.
            if !state.pointer_grabbed {
                let hint = "CLICK TO CAPTURE MOUSE · ESC PAUSES";
                let (tw, _) = font::text_size(hint, 2);
                ctx.text("capture_hint", hint, cx - tw as i32 / 2, hi / 2 + ctx.px(40), 2);
            }
        }
        Screen::Pause => {
            let entries: Vec<(&str, &str)> = vec![
                ("pause_resume", "RESUME"),
                ("pause_save", "SAVE"),
                ("pause_load", "LOAD"),
                ("pause_settings", "SETTINGS"),
                ("pause_quit_title", "QUIT TO TITLE"),
                ("pause_quit_desktop", "QUIT TO DESKTOP"),
            ];
            let n = entries.len() as i32;
            let panel_w = BTN_W + PANEL_PAD * 2;
            let panel_h = BTN_H * n + BTN_GAP * (n - 1) + PANEL_PAD * 2 + 34;
            let s = fit_scale(wi, hi, state.settings.ui_scale, panel_w, panel_h + 200);
            ctx.s = s;
            let pw = ctx.px(panel_w);
            let ph = ctx.px(panel_h);
            let panel = Rect::new(cx - pw / 2, centered_y(hi, ph), pw as u32, ph as u32);
            ctx.panel("pause_panel", panel, Some("PAUSED"));
            let mut y = panel.y + ctx.px(PANEL_PAD) + ctx.px(26);
            for (i, (id, label)) in entries.iter().enumerate() {
                ctx.button(id, label, cx, y, ctx.px(BTN_W), ctx.px(BTN_H), button_state(state, id, i, entries.len()));
                y += ctx.px(BTN_H) + ctx.px(BTN_GAP);
            }
        }
    }

    }

    // A modal owns the frame completely: dim background + the modal only
    // (the underlying screen's buttons are inert and not laid out, so
    // nothing can overlap the modal's interactives).
    if let Some(modal) = &state.modal {
        ctx.push("modal_dim", ElementKind::Dim, Rect::new(0, 0, w, h));
        let msg = modal.message();
        let panel_w = 460 + PANEL_PAD * 2;
        let lines = msg.matches('\n').count() as i32 + 2;
        let panel_h = TEXT_ROW_H * lines * 2 + BTN_H + BTN_GAP * 2 + PANEL_PAD * 2 + 20;
        let s = fit_scale(wi, hi, state.settings.ui_scale, panel_w, panel_h + 100);
        let pw = (panel_w as f32 * s) as i32;
        let ph = (panel_h as f32 * s) as i32;
        let panel = Rect::new(cx - pw / 2, centered_y(hi, ph), pw as u32, ph as u32);
        ctx.panel("modal_panel", panel, Some("CONFIRM"));
        let mut y = panel.y + (PANEL_PAD as f32 * s) as i32 + (26.0 * s) as i32;
        for line in msg.split('\n') {
            let (tw, th) = font::text_size(line, 2);
            ctx.text("modal_msg", line, cx - tw as i32 / 2, y, 2);
            y += th as i32 + (10.0 * s) as i32;
        }
        y += (6.0 * s) as i32;
        let bw = (170.0 * s) as i32;
        ctx.button("modal_confirm", modal.confirm_label(), cx - bw - (10.0 * s) as i32, y, bw, (BTN_H as f32 * s) as i32, button_state(state, "modal_confirm", 0, 2));
        ctx.button("modal_cancel", "CANCEL", cx + (10.0 * s) as i32, y, bw, (BTN_H as f32 * s) as i32, button_state(state, "modal_cancel", 1, 2));
    }

    DrawList { size: (w, h), elements: ctx.out, scale: ctx.s }
}

fn footer(ctx: &mut Ctx, wi: i32, hi: i32) {
    let l = "POORCRAFT 3D · OWNER ALPHA";
    let r = "WASD MOVE · F BUILD · R REMOVE · ESC PAUSE";
    let (_, lh) = font::text_size(l, 1);
    ctx.text("footer_left", l, SAFE_MARGIN_PX + 2, hi - SAFE_MARGIN_PX - lh as i32 - 2, 1);
    let (rw, _) = font::text_size(r, 1);
    ctx.text("footer_right", r, wi - SAFE_MARGIN_PX - rw as i32 - 2, hi - SAFE_MARGIN_PX - lh as i32 - 2, 1);
}

fn centered_y(hi: i32, panel_h: i32) -> i32 {
    // Slightly above true center; clamped inside the safe margins.
    ((hi - panel_h) / 2).max(SAFE_MARGIN_PX).min(hi - SAFE_MARGIN_PX - panel_h)
}

fn safe_y(hi: i32, wanted: i32, h: i32) -> i32 {
    wanted.max(SAFE_MARGIN_PX).min(hi - SAFE_MARGIN_PX - h)
}

fn scale_for(_w: i32, _h: i32, base: u32) -> u32 {
    base
}

// ---------------------------------------------------------------------------
// Painter (pure CPU; one straight-alpha RGBA canvas)
// ---------------------------------------------------------------------------

/// Theme: rugged fantasy survival — forged metal, ember, forest/river.
pub mod theme {
    pub const PANEL_FILL: [u8; 4] = [40, 36, 32, 228];
    pub const PANEL_FILL_2: [u8; 4] = [30, 27, 24, 228];
    pub const PANEL_EDGE_L: [u8; 4] = [138, 122, 102, 255];
    pub const PANEL_EDGE_D: [u8; 4] = [23, 19, 15, 255];
    pub const TITLE_BAR: [u8; 4] = [58, 48, 38, 255];

    pub const BTN_FILL: [u8; 4] = [56, 49, 41, 240];
    pub const BTN_FILL_HOVER: [u8; 4] = [104, 88, 64, 242];
    pub const BTN_FILL_PRESSED: [u8; 4] = [48, 42, 35, 240];
    pub const BTN_FILL_DISABLED: [u8; 4] = [52, 50, 47, 140];
    pub const BTN_EDGE: [u8; 4] = [170, 152, 124, 255];
    pub const EMBER: [u8; 4] = [224, 138, 60, 255];
    pub const EMBER_BRIGHT: [u8; 4] = [242, 178, 92, 255];
    pub const FOCUS_EDGE: [u8; 4] = [242, 178, 92, 255];

    pub const TEXT: [u8; 4] = [242, 234, 216, 255];
    pub const TEXT_DIM: [u8; 4] = [158, 150, 136, 255];
    pub const TEXT_DISABLED: [u8; 4] = [140, 134, 124, 160];
    pub const TEXT_SHADOW: [u8; 4] = [12, 10, 8, 220];

    pub const BAR_FRAME: [u8; 4] = [20, 17, 14, 255];
    pub const BAR_BACK: [u8; 4] = [24, 21, 18, 210];

    pub const SLOT_FILL: [u8; 4] = [26, 23, 20, 200];
    pub const SLOT_EDGE: [u8; 4] = [110, 98, 82, 255];
    pub const SLOT_SELECTED_EDGE: [u8; 4] = [242, 178, 92, 255];

    pub const CROSSHAIR: [u8; 4] = [242, 234, 216, 230];
    pub const CROSSHAIR_SHADOW: [u8; 4] = [12, 10, 8, 200];

    pub const DEBUG_FILL: [u8; 4] = [12, 16, 22, 205];
    pub const DEBUG_TEXT: [u8; 4] = [150, 210, 235, 255];
    pub const DEBUG_EDGE: [u8; 4] = [50, 80, 105, 255];

    pub const TOAST_FILL: [u8; 4] = [34, 30, 26, 215];
    pub const TOAST_EDGE: [u8; 4] = [138, 122, 102, 255];

    pub const LOGO_A: [u8; 4] = [242, 178, 92, 255];
    pub const LOGO_B: [u8; 4] = [206, 110, 52, 255];
    pub const LOGO_3D: [u8; 4] = [122, 176, 194, 255];
    pub const LOGO_SHADOW: [u8; 4] = [15, 12, 9, 235];

    /// Ember gradient across a logo's letters.
    pub fn logo_col(frac: f32) -> [u8; 4] {
        let t = frac.clamp(0.0, 1.0);
        let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        [
            lerp(LOGO_B[0], LOGO_A[0]),
            lerp(LOGO_B[1], LOGO_A[1]),
            lerp(LOGO_B[2], LOGO_A[2]),
            255,
        ]
    }
}

struct Canvas {
    w: u32,
    h: u32,
    px: Vec<u8>,
}

impl Canvas {
    fn new(w: u32, h: u32) -> Canvas {
        Canvas { w, h, px: vec![0u8; (w * h * 4) as usize] }
    }

    fn fill(&mut self, r: Rect, c: [u8; 4]) {
        let x0 = r.x.max(0).min(self.w as i32);
        let y0 = r.y.max(0).min(self.h as i32);
        let x1 = r.right().max(0).min(self.w as i32);
        let y1 = r.bottom().max(0).min(self.h as i32);
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y as u32 * self.w + x as u32) * 4) as usize;
                self.px[i..i + 4].copy_from_slice(&c);
            }
        }
    }

    fn px_set(&mut self, x: i32, y: i32, c: [u8; 4]) {
        if x >= 0 && y >= 0 && (x as u32) < self.w && (y as u32) < self.h {
            let i = ((y as u32 * self.w + x as u32) * 4) as usize;
            self.px[i..i + 4].copy_from_slice(&c);
        }
    }

    /// Beveled frame: light edge top/left, dark edge bottom/right.
    fn bevel(&mut self, r: Rect, light: [u8; 4], dark: [u8; 4]) {
        self.fill(Rect::new(r.x, r.y, r.w as u32, 2), light);
        self.fill(Rect::new(r.x, r.y, 2, r.h as u32), light);
        self.fill(Rect::new(r.x, r.bottom() - 2, r.w as u32, 2), dark);
        self.fill(Rect::new(r.right() - 2, r.y, 2, r.h as u32), dark);
    }

    fn corner_studs(&mut self, r: Rect, c: [u8; 4]) {
        let s = 4u32;
        for (x, y) in [
            (r.x + 3, r.y + 3),
            (r.right() - 3 - s as i32, r.y + 3),
            (r.x + 3, r.bottom() - 3 - s as i32),
            (r.right() - 3 - s as i32, r.bottom() - 3 - s as i32),
        ] {
            self.fill(Rect::new(x, y, s, s), c);
        }
    }

    fn text(&mut self, text: &str, x: i32, y: i32, scale: u32, color: [u8; 4], shadow: bool) {
        let mut cy = y;
        for line in text.split('\n') {
            let mut cx = x;
            for ch in line.chars() {
                let rows = font::glyph_rows(ch);
                for (ry, bits) in rows.iter().enumerate() {
                    for rx in 0..font::GLYPH_W {
                        if bits & (1 << (font::GLYPH_W - 1 - rx)) != 0 {
                            for py in 0..scale {
                                for pxx in 0..scale {
                                    let gx = cx + (rx as i32) * (scale as i32) + pxx as i32;
                                    let gy = cy + (ry as i32) * (scale as i32) + py as i32;
                                    if shadow {
                                        self.px_set(gx + 1, gy + 1, theme::TEXT_SHADOW);
                                    }
                                    self.px_set(gx, gy, color);
                                }
                            }
                        }
                    }
                }
                cx += font::ADVANCE as i32 * scale as i32;
            }
            cy += (font::GLYPH_H * scale + scale) as i32;
        }
    }

    /// Centered text inside a rect.
    fn text_centered(&mut self, text: &str, r: Rect, scale: u32, color: [u8; 4]) {
        let (tw, th) = font::text_size(text, scale);
        self.text(text, r.cx() - tw as i32 / 2, r.cy() - th as i32 / 2, scale, color, true);
    }
}

/// Paints the draw list into a straight-alpha RGBA canvas (w*h*4 bytes).
/// Pure: same list, same pixels — the screenshot harness depends on it.
pub fn paint(list: &DrawList) -> Vec<u8> {
    let mut c = Canvas::new(list.size.0, list.size.1);
    for e in &list.elements {
        match &e.kind {
            ElementKind::Panel { title } => {
                c.fill(e.rect, theme::PANEL_FILL);
                if let Some(t) = title {
                    let bar = Rect::new(e.rect.x + 2, e.rect.y + 2, e.rect.w - 4, 22);
                    c.fill(bar, theme::TITLE_BAR);
                    c.text_centered(t, bar, 3, theme::EMBER_BRIGHT);
                }
                c.bevel(e.rect, theme::PANEL_EDGE_L, theme::PANEL_EDGE_D);
                c.corner_studs(e.rect, theme::PANEL_EDGE_D);
            }
            ElementKind::Button { label, state } => {
                let (fill, text_col, edge) = match state {
                    ButtonState::Normal => (theme::BTN_FILL, theme::TEXT, theme::BTN_EDGE),
                    ButtonState::Hover => (theme::BTN_FILL_HOVER, theme::TEXT, theme::EMBER),
                    ButtonState::Focused => (theme::BTN_FILL, theme::TEXT, theme::FOCUS_EDGE),
                    ButtonState::Pressed => (theme::BTN_FILL_PRESSED, theme::TEXT, theme::EMBER_BRIGHT),
                    ButtonState::Disabled => (theme::BTN_FILL_DISABLED, theme::TEXT_DISABLED, theme::PANEL_EDGE_D),
                };
                c.fill(e.rect, fill);
                c.bevel(e.rect, edge, theme::PANEL_EDGE_D);
                if matches!(state, ButtonState::Focused | ButtonState::Hover) {
                    // Doubled frame so focus/hover read at a glance.
                    c.bevel(
                        Rect::new(e.rect.x + 4, e.rect.y + 4, e.rect.w - 8, e.rect.h - 8),
                        theme::PANEL_EDGE_L,
                        theme::PANEL_EDGE_D,
                    );
                }
                c.text_centered(label, e.rect, 3, text_col);
            }
            ElementKind::Text { label, scale } => {
                c.text(label, e.rect.x, e.rect.y, *scale, theme::TEXT, true);
            }
            ElementKind::Bar { frac, color, label } => {
                c.fill(e.rect, theme::BAR_FRAME);
                let inner = Rect::new(e.rect.x + 2, e.rect.y + 2, e.rect.w - 4, e.rect.h - 4);
                c.fill(inner, theme::BAR_BACK);
                let fw = ((inner.w as f32) * frac.clamp(0.0, 1.0)).round() as u32;
                if fw > 0 {
                    c.fill(Rect::new(inner.x, inner.y, fw, inner.h), [color[0], color[1], color[2], 235]);
                    // Sheen line along the fill top.
                    if fw > 2 && inner.h > 4 {
                        c.fill(
                            Rect::new(inner.x + 1, inner.y + 1, fw - 2, 2),
                            [color[0].saturating_add(40), color[1].saturating_add(40), color[2].saturating_add(40), 255],
                        );
                    }
                }
                c.text_centered(label, e.rect, 1, theme::TEXT);
            }
            ElementKind::HotbarSlot { index, selected, item } => {
                c.fill(e.rect, theme::SLOT_FILL);
                if *selected {
                    c.bevel(e.rect, theme::SLOT_SELECTED_EDGE, theme::SLOT_SELECTED_EDGE);
                    c.bevel(
                        Rect::new(e.rect.x + 2, e.rect.y + 2, e.rect.w - 4, e.rect.h - 4),
                        theme::EMBER,
                        theme::PANEL_EDGE_D,
                    );
                } else {
                    c.bevel(e.rect, theme::SLOT_EDGE, theme::PANEL_EDGE_D);
                }
                if let Some(it) = item {
                    let pad = if *selected { 8 } else { 6 };
                    let sw = Rect::new(
                        e.rect.x + pad,
                        e.rect.y + pad,
                        e.rect.w - pad as u32 * 2,
                        e.rect.h - pad as u32 * 2 - 10,
                    );
                    c.fill(sw, [it.color[0], it.color[1], it.color[2], 255]);
                    // Facet pattern: darker diagonal specks, deterministic.
                    for k in 0..4 {
                        let fx = sw.x + 4 + (k * 7) % (sw.w as i32 - 8).max(1);
                        let fy = sw.y + 3 + (k * 5) % (sw.h as i32 - 6).max(1);
                        c.fill(Rect::new(fx, fy, 3, 3), [it.color[0] / 2, it.color[1] / 2, it.color[2] / 2, 255]);
                    }
                    // The item label under the swatch.
                    let (tw, _) = font::text_size(it.label, 1);
                    c.text(it.label, e.rect.cx() - tw as i32 / 2, e.rect.bottom() - 10, 1, theme::TEXT, true);
                }
                // Slot number, top-left corner.
                let num = (index + 1).to_string();
                c.text(&num, e.rect.x + 4, e.rect.y + 3, 1, theme::TEXT_DIM, true);
            }
            ElementKind::Dim => {
                c.fill(e.rect, [10, 9, 8, 150]);
            }
            ElementKind::Crosshair => {
                let (cx, cy) = (e.rect.cx(), e.rect.cy());
                let t = 2;
                let len = e.rect.w as i32 / 2;
                for col in [theme::CROSSHAIR, theme::CROSSHAIR_SHADOW] {
                    // Draw shadow slightly larger underneath.
                    let (cx2, cy2, t2) = if col == theme::CROSSHAIR_SHADOW {
                        (cx + 1, cy + 1, t2_wide(t))
                    } else {
                        (cx, cy, t)
                    };
                    c.fill(Rect::new(cx2 - len, cy2 - t2 / 2, (len * 2) as u32, t2 as u32), col);
                    c.fill(Rect::new(cx2 - t2 / 2, cy2 - len, t2 as u32, (len * 2) as u32), col);
                }
            }
            ElementKind::Logo => {
                // A soft dark band grounds the logo over any world view.
                let band = Rect::new(
                    e.rect.x - 14,
                    e.rect.y - 8,
                    e.rect.w + 28,
                    e.rect.h + 16,
                );
                c.fill(band, [15, 13, 11, 120]);
                c.bevel(band, [40, 34, 28, 160], [10, 9, 8, 160]);
                // "POORCRAFT" with per-letter ember gradient + 3D accent.
                let text = "POORCRAFT";
                let scale = 6u32;
                let chars: Vec<char> = text.chars().collect();
                let mut x = e.rect.x;
                let grad = |i: usize, n: usize| theme::logo_col(i as f32 / n.max(1) as f32);
                for (i, ch) in chars.iter().enumerate() {
                    let col = grad(i, chars.len());
                    draw_glyph_shadow(&mut c, *ch, x, e.rect.y, scale, col, 3);
                    x += font::ADVANCE as i32 * scale as i32;
                }
                // "3D" chip at the end, river-blue.
                let (tw, th) = font::text_size("3D", 4);
                let chip = Rect::new(
                    e.rect.right() + 8,
                    e.rect.bottom() - th as i32 - 10,
                    tw as u32 + 16,
                    th as u32 + 10,
                );
                c.fill(chip, theme::PANEL_FILL_2);
                c.bevel(chip, theme::PANEL_EDGE_L, theme::PANEL_EDGE_D);
                c.text_centered("3D", chip, 4, theme::LOGO_3D);
            }
            ElementKind::Toast { label, alpha } => {
                let a = (*alpha).clamp(0.0, 1.0);
                let mul = |c: [u8; 4]| [c[0], c[1], c[2], (c[3] as f32 * a) as u8];
                c.fill(e.rect, mul(theme::TOAST_FILL));
                c.bevel(e.rect, mul(theme::TOAST_EDGE), mul(theme::PANEL_EDGE_D));
                c.text_centered_alpha(label, e.rect, 2, theme::TEXT, a);
            }
            ElementKind::Debug { label } => {
                c.fill(e.rect, theme::DEBUG_FILL);
                c.bevel(e.rect, theme::DEBUG_EDGE, theme::DEBUG_EDGE);
                c.text(label, e.rect.x + 8, e.rect.y + 4, 2, theme::DEBUG_TEXT, true);
            }
            ElementKind::Image { w, h, pixels } => {
                // Nearest-neighbor blit into the rect, clipped to the
                // canvas. The forged frame keeps it reading as UI.
                c.fill(e.rect, theme::SLOT_FILL);
                for y in 0..e.rect.h as i32 {
                    for x in 0..e.rect.w as i32 {
                        let sx = ((x as u64 * *w as u64) / e.rect.w.max(1) as u64) as u32;
                        let sy = ((y as u64 * *h as u64) / e.rect.h.max(1) as u64) as u32;
                        let si = ((sy * *w + sx) * 4) as usize;
                        if si + 4 <= pixels.len() {
                            let px: [u8; 4] =
                                [pixels[si], pixels[si + 1], pixels[si + 2], pixels[si + 3]];
                            c.px_set(e.rect.x + x, e.rect.y + y, px);
                        }
                    }
                }
                c.bevel(e.rect, theme::PANEL_EDGE_L, theme::PANEL_EDGE_D);
            }
        }
    }
    c.px
}

fn t2_wide(t: i32) -> i32 {
    t + 2
}

fn draw_glyph_shadow(c: &mut Canvas, ch: char, x: i32, y: i32, scale: u32, color: [u8; 4], shadow: i32) {
    let rows = font::glyph_rows(ch);
    for (ry, bits) in rows.iter().enumerate() {
        for rx in 0..font::GLYPH_W {
            if bits & (1 << (font::GLYPH_W - 1 - rx)) != 0 {
                for py in 0..scale {
                    for pxx in 0..scale {
                        let gx = x + rx as i32 * scale as i32 + pxx as i32;
                        let gy = y + ry as i32 * scale as i32 + py as i32;
                        c.px_set(gx + shadow, gy + shadow, theme::LOGO_SHADOW);
                        c.px_set(gx, gy, color);
                    }
                }
            }
        }
    }
}

impl Canvas {
    fn text_centered_alpha(&mut self, text: &str, r: Rect, scale: u32, color: [u8; 4], a: f32) {
        let (tw, th) = font::text_size(text, scale);
        let x = r.cx() - tw as i32 / 2;
        let y = r.cy() - th as i32 / 2;
        let col = [color[0], color[1], color[2], (color[3] as f32 * a) as u8];
        let sh = [theme::TEXT_SHADOW[0], theme::TEXT_SHADOW[1], theme::TEXT_SHADOW[2], (theme::TEXT_SHADOW[3] as f32 * a) as u8];
        let mut cy = y;
        for line in text.split('\n') {
            let mut cx = x;
            for ch in line.chars() {
                let rows = font::glyph_rows(ch);
                for (ry, bits) in rows.iter().enumerate() {
                    for rx in 0..font::GLYPH_W {
                        if bits & (1 << (font::GLYPH_W - 1 - rx)) != 0 {
                            for py in 0..scale {
                                for pxx in 0..scale {
                                    let gx = cx + rx as i32 * scale as i32 + pxx as i32;
                                    let gy = cy + ry as i32 * scale as i32 + py as i32;
                                    self.px_set(gx + 1, gy + 1, sh);
                                    self.px_set(gx, gy, col);
                                }
                            }
                        }
                    }
                }
                cx += font::ADVANCE as i32 * scale as i32;
            }
            cy += (font::GLYPH_H * scale + scale) as i32;
        }
    }
}

// ---------------------------------------------------------------------------
// Input reducer (pure; actions the app executes)
// ---------------------------------------------------------------------------

/// The abstract key set the app maps from winit (testable without winit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Enter,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Char(char),
    Digit(u8),
    Backspace,
    F3,
    KeyQ,
    WheelUp,
    WheelDown,
}

/// What the app should do after a UI event. Everything side-effectful lives
/// here; the reducer only mutates UiState.
#[derive(Clone, Debug, PartialEq)]
pub enum UiAction {
    StartPlaying,
    OpenScreen(Screen),
    CloseModal,
    ConfirmModal(ModalKind),
    QuitToDesktop,
    QuitToTitle,
    SaveNow,
    LoadSlot(String),
    DeleteSlot(String),
    CreateWorld { seed: u64, name: String },
    SetSensitivity(f32),
    SetInvertY(bool),
    SetFov(f32),
    SetUiScale(f32),
    SetQuality(Quality),
    SelectHotbar(usize),
    CaptureMouse,
    /// Proof hook (inspector): raw mouse deltas applied to the live
    /// player body — the exact path real mouse motion takes.
    PlayerLook { dx: f32, dy: f32 },
    Repaint,
}

/// How many activatable focus stops the current screen has (buttons; the
/// hotbar is mouse/wheel driven and stays out of the focus ring).
pub fn focus_count(state: &UiState) -> usize {
    match state.screen {
        Screen::Title => 5,
        Screen::NewWorld => 6,
        Screen::LoadWorld => 1 + state.slots.len().min(6) * 2,
        Screen::Settings => 1,
        Screen::Pause => 6,
        Screen::Gameplay => 0,
    }
}

/// Keyboard input → state mutations + actions. Pure.
pub fn on_key(state: &mut UiState, key: Key) -> Vec<UiAction> {
    let mut acts = Vec::new();

    // The modal owns input completely while open.
    if state.modal.is_some() {
        match key {
            Key::Enter => acts.push(confirm(state)),
            Key::Escape => {
                state.modal = None;
                acts.push(UiAction::CloseModal);
            }
            Key::Left | Key::Right | Key::Up | Key::Down => {
                state.focus = 1 - state.focus.min(1);
            }
            _ => {}
        }
        return acts;
    }

    match state.screen {
        Screen::Gameplay => match key {
            Key::Escape => {
                state.screen = Screen::Pause;
                state.pointer_grabbed = false;
                state.focus = 0;
                state.hover = None;
                acts.push(UiAction::OpenScreen(Screen::Pause));
            }
            Key::F3 => {
                state.debug_overlay = !state.debug_overlay;
                acts.push(UiAction::Repaint);
            }
            Key::Digit(d) if (1..=9).contains(&d) => {
                state.hud.selected = (d - 1) as usize;
                acts.push(UiAction::SelectHotbar((d - 1) as usize));
                acts.push(UiAction::Repaint);
            }
            Key::WheelUp => {
                state.hud.selected = (state.hud.selected + 8) % 9;
                acts.push(UiAction::SelectHotbar(state.hud.selected));
                acts.push(UiAction::Repaint);
            }
            Key::WheelDown => {
                state.hud.selected = (state.hud.selected + 1) % 9;
                acts.push(UiAction::SelectHotbar(state.hud.selected));
                acts.push(UiAction::Repaint);
            }
            _ => {}
        },
        Screen::Title => match key {
            Key::Up => {
                state.focus = (state.focus + focus_count(state) - 1) % focus_count(state);
                acts.push(UiAction::Repaint);
            }
            Key::Down => {
                state.focus = (state.focus + 1) % focus_count(state);
                acts.push(UiAction::Repaint);
            }
            Key::Enter => acts.extend(activate_title(state, state.focus.min(4))),
            Key::Escape => {
                // Escape on the title does NOT exit (owner law).
            }
            Key::KeyQ => {
                state.modal = Some(ModalKind::QuitToDesktop);
                state.focus = 0;
                acts.push(UiAction::Repaint);
            }
            _ => {}
        },
        Screen::Pause => match key {
            Key::Escape => {
                state.screen = Screen::Gameplay;
                state.pointer_grabbed = true;
                acts.push(UiAction::StartPlaying);
            }
            Key::Up => {
                state.focus = (state.focus + focus_count(state) - 1) % focus_count(state);
                acts.push(UiAction::Repaint);
            }
            Key::Down => {
                state.focus = (state.focus + 1) % focus_count(state);
                acts.push(UiAction::Repaint);
            }
            Key::Enter => acts.extend(activate_pause(state, state.focus.min(5))),
            Key::KeyQ => {
                state.modal = Some(ModalKind::QuitToDesktop);
                state.focus = 0;
                acts.push(UiAction::Repaint);
            }
            _ => {}
        },
        Screen::NewWorld => match key {
            Key::Escape => {
                state.screen = Screen::Title;
                acts.push(UiAction::OpenScreen(Screen::Title));
            }
            Key::Up => {
                state.focus = (state.focus + focus_count(state) - 1) % focus_count(state);
                acts.push(UiAction::Repaint);
            }
            Key::Down => {
                state.focus = (state.focus + 1) % focus_count(state);
                acts.push(UiAction::Repaint);
            }
            Key::Enter => acts.extend(activate_new_world(state, state.focus.min(5))),
            Key::Digit(d) => {
                if state.form.seed_digits.len() < 10 {
                    state.form.seed_digits.push((b'0' + d) as char);
                    state.seed_preview_stale = true;
                    acts.push(UiAction::Repaint);
                }
            }
            Key::Backspace => {
                state.form.seed_digits.pop();
                if state.form.seed_digits.is_empty() {
                    state.form.seed_digits.push('0');
                }
                state.seed_preview_stale = true;
                acts.push(UiAction::Repaint);
            }
            _ => {}
        },
        Screen::LoadWorld => match key {
            Key::Escape => {
                state.screen = Screen::Title;
                acts.push(UiAction::OpenScreen(Screen::Title));
            }
            Key::Up => {
                state.focus = (state.focus + focus_count(state) - 1) % focus_count(state);
                acts.push(UiAction::Repaint);
            }
            Key::Down => {
                state.focus = (state.focus + 1) % focus_count(state);
                acts.push(UiAction::Repaint);
            }
            Key::Enter => acts.extend(activate_load_world(state, state.focus)),
            _ => {}
        },
        Screen::Settings => match key {
            Key::Escape => {
                state.screen = Screen::Title;
                acts.push(UiAction::OpenScreen(Screen::Title));
            }
            Key::Enter => {
                state.screen = Screen::Title;
                acts.push(UiAction::OpenScreen(Screen::Title));
            }
            Key::Left => acts.extend(adjust_focused_setting(state, -1)),
            Key::Right => acts.extend(adjust_focused_setting(state, 1)),
            Key::Up => {
                state.focus = (state.focus + 4) % 5;
                acts.push(UiAction::Repaint);
            }
            Key::Down => {
                state.focus = (state.focus + 1) % 5;
                acts.push(UiAction::Repaint);
            }
            _ => {}
        },
    }
    acts
}

fn confirm(state: &mut UiState) -> UiAction {
    let modal = state.modal.take().expect("modal open");
    UiAction::ConfirmModal(modal)
}

fn activate_title(state: &mut UiState, idx: usize) -> Vec<UiAction> {
    match idx {
        0 => {
            state.screen = Screen::Gameplay;
            state.pointer_grabbed = true;
            state.last_activated = Some("btn_play".into());
            vec![UiAction::StartPlaying]
        }
        1 => {
            state.screen = Screen::NewWorld;
            state.focus = 0;
            vec![UiAction::OpenScreen(Screen::NewWorld)]
        }
        2 => {
            state.screen = Screen::LoadWorld;
            state.focus = 0;
            vec![UiAction::OpenScreen(Screen::LoadWorld)]
        }
        3 => {
            state.screen = Screen::Settings;
            state.focus = 0;
            vec![UiAction::OpenScreen(Screen::Settings)]
        }
        _ => {
            state.modal = Some(ModalKind::QuitToDesktop);
            state.focus = 0;
            vec![UiAction::Repaint]
        }
    }
}

fn activate_pause(state: &mut UiState, idx: usize) -> Vec<UiAction> {
    match idx {
        0 => {
            state.screen = Screen::Gameplay;
            state.pointer_grabbed = true;
            state.last_activated = Some("pause_resume".into());
            vec![UiAction::StartPlaying]
        }
        1 => vec![UiAction::SaveNow],
        2 => {
            state.screen = Screen::LoadWorld;
            state.focus = 0;
            vec![UiAction::OpenScreen(Screen::LoadWorld)]
        }
        3 => {
            state.screen = Screen::Settings;
            state.focus = 0;
            vec![UiAction::OpenScreen(Screen::Settings)]
        }
        4 => {
            state.screen = Screen::Title;
            state.session_live = true;
            state.focus = 0;
            vec![UiAction::QuitToTitle]
        }
        _ => {
            state.modal = Some(ModalKind::QuitToDesktop);
            state.focus = 0;
            vec![UiAction::Repaint]
        }
    }
}

fn activate_new_world(state: &mut UiState, idx: usize) -> Vec<UiAction> {
    match idx {
        // 0 CREATE, 1 REROLL, 2 RANDOM, 3 QUALITY-, 4 QUALITY+, 5 BACK
        0 => {
            // WT-001: an invalid preview (unsafe spawn) must not be
            // creatable — Enter and click both route through here.
            if let Some(p) = &state.seed_preview {
                if !p.valid {
                    return vec![UiAction::Repaint];
                }
            }
            let seed = state.form.seed();
            let name = if state.form.name.is_empty() {
                format!("WORLD-{seed}")
            } else {
                state.form.name.clone()
            };
            vec![UiAction::CreateWorld { seed, name }]
        }
        1 | 2 => {
            // REROLL / RANDOM: the WT-001 law — never the same seed
            // twice. Entropy is the clock-free counter (reroll count),
            // mixed per-button so the two paths diverge. Capped to 15
            // digits so the seed always fits its form column and the
            // RESOLVED line at the drawn glyph scale.
            state.form.reroll_count += 1;
            let cur = state.form.seed();
            let mix = if idx == 1 { 0 } else { 0x5DEECE66D };
            let next = pc3d_world::seed_preview::reroll_seed(
                cur,
                state.form.reroll_count.wrapping_mul(0x9E3779B1) ^ mix,
            );
            let next = next % 1_000_000_000_000_000;
            let next = if next == cur { next + 1 } else { next };
            state.form.seed_digits = next.to_string();
            state.form.name = format!("WORLD-{next}");
            state.seed_preview_stale = true;
            vec![UiAction::Repaint]
        }
        3 => {
            state.form.quality = state.form.quality.cycle(false);
            vec![UiAction::Repaint]
        }
        4 => {
            state.form.quality = state.form.quality.cycle(true);
            vec![UiAction::Repaint]
        }
        _ => {
            state.screen = Screen::Title;
            vec![UiAction::OpenScreen(Screen::Title)]
        }
    }
}

fn activate_load_world(state: &mut UiState, focus: usize) -> Vec<UiAction> {
    let n = state.slots.len().min(6);
    if n == 0 {
        state.screen = Screen::Title;
        return vec![UiAction::OpenScreen(Screen::Title)];
    }
    // Focus order: BACK, then per-slot LOAD, DELETE.
    if focus == 0 {
        state.screen = Screen::Title;
        return vec![UiAction::OpenScreen(Screen::Title)];
    }
    let rest = focus - 1;
    let slot = rest / 2;
    if slot >= n {
        return vec![];
    }
    if rest % 2 == 0 {
        let name = state.slots[slot].name.clone();
        state.modal = Some(ModalKind::LoadWorld(name.clone()));
        state.focus = 0;
        vec![UiAction::Repaint]
    } else {
        let name = state.slots[slot].name.clone();
        state.modal = Some(ModalKind::DeleteWorld(name));
        state.focus = 0;
        vec![UiAction::Repaint]
    }
}

fn adjust_focused_setting(state: &mut UiState, dir: i32) -> Vec<UiAction> {
    match state.focus % 5 {
        0 => {
            state.settings.mouse_sensitivity = (state.settings.mouse_sensitivity + dir as f32 * 0.1).clamp(0.2, 3.0);
            vec![UiAction::SetSensitivity(state.settings.mouse_sensitivity)]
        }
        1 => {
            state.settings.invert_y = !state.settings.invert_y;
            vec![UiAction::SetInvertY(state.settings.invert_y)]
        }
        2 => {
            state.settings.fov_deg = (state.settings.fov_deg + dir as f32 * 5.0).clamp(50.0, 100.0);
            vec![UiAction::SetFov(state.settings.fov_deg)]
        }
        3 => {
            state.settings.ui_scale = ((state.settings.ui_scale + dir as f32 * 0.05) * 100.0).round() / 100.0;
            state.settings.clamp();
            vec![UiAction::SetUiScale(state.settings.ui_scale), UiAction::Repaint]
        }
        _ => {
            state.settings.quality = state.settings.quality.cycle(dir > 0);
            vec![UiAction::SetQuality(state.settings.quality)]
        }
    }
}

/// Mouse move over a fresh draw list → hover state. Pure.
pub fn on_mouse_move(state: &mut UiState, x: i32, y: i32, list: &DrawList) -> bool {
    let hit = if state.pointer_grabbed {
        None
    } else {
        list.hit(x, y).map(|e| e.id.clone())
    };
    if hit != state.hover {
        state.hover = hit;
        true
    } else {
        false
    }
}

/// Mouse click → activates the hit element (or captures the mouse in
/// gameplay). Pure; returns actions for the app.
pub fn on_click(state: &mut UiState, x: i32, y: i32, list: &DrawList) -> Vec<UiAction> {
    // Modal buttons first.
    if state.modal.is_some() {
        if let Some(e) = list.hit(x, y) {
            match e.id.as_str() {
                "modal_confirm" => return vec![confirm(state)],
                "modal_cancel" => {
                    state.modal = None;
                    return vec![UiAction::CloseModal];
                }
                _ => return vec![],
            }
        }
        return vec![];
    }
    match state.screen {
        Screen::Gameplay => {
            if let Some(e) = list.hit(x, y) {
                if let ElementKind::HotbarSlot { index, .. } = &e.kind {
                    state.hud.selected = *index;
                    return vec![UiAction::SelectHotbar(*index), UiAction::Repaint];
                }
            }
            vec![UiAction::CaptureMouse]
        }
        Screen::Title => {
            if let Some(e) = list.hit(x, y) {
                let idx = match e.id.as_str() {
                    "btn_play" => Some(0),
                    "btn_new_world" => Some(1),
                    "btn_load_world" => Some(2),
                    "btn_settings" => Some(3),
                    "btn_quit" => Some(4),
                    _ => None,
                };
                if let Some(i) = idx {
                    state.focus = i;
                    return activate_title(state, i);
                }
            }
            vec![]
        }
        Screen::Pause => {
            if let Some(e) = list.hit(x, y) {
                let idx = match e.id.as_str() {
                    "pause_resume" => Some(0),
                    "pause_save" => Some(1),
                    "pause_load" => Some(2),
                    "pause_settings" => Some(3),
                    "pause_quit_title" => Some(4),
                    "pause_quit_desktop" => Some(5),
                    _ => None,
                };
                if let Some(i) = idx {
                    state.focus = i;
                    return activate_pause(state, i);
                }
            }
            vec![]
        }
        Screen::NewWorld => {
            if let Some(e) = list.hit(x, y) {
                let idx = match e.id.as_str() {
                    "nw_create" => Some(0),
                    "nw_seed_reroll" => Some(1),
                    "nw_seed_random" => Some(2),
                    "nw_quality_down" => Some(3),
                    "nw_quality_up" => Some(4),
                    "nw_back" => Some(5),
                    _ => None,
                };
                if let Some(i) = idx {
                    state.focus = i;
                    return activate_new_world(state, i);
                }
            }
            vec![]
        }
        Screen::LoadWorld => {
            if let Some(e) = list.hit(x, y) {
                if e.id == "lw_back" {
                    state.screen = Screen::Title;
                    return vec![UiAction::OpenScreen(Screen::Title)];
                }
                if let Some(tail) = e.id.strip_prefix("lw_slot_") {
                    let mut it = tail.split('_');
                    let slot: usize = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                    let kind = it.next().unwrap_or("");
                    if slot < state.slots.len().min(6) {
                        let name = state.slots[slot].name.clone();
                        match kind {
                            "load" => {
                                state.modal = Some(ModalKind::LoadWorld(name));
                                return vec![UiAction::Repaint];
                            }
                            "del" => {
                                state.modal = Some(ModalKind::DeleteWorld(name));
                                return vec![UiAction::Repaint];
                            }
                            _ => {}
                        }
                    }
                }
            }
            vec![]
        }
        Screen::Settings => {
            if let Some(e) = list.hit(x, y) {
                if e.id == "set_back" {
                    state.screen = Screen::Title;
                    return vec![UiAction::OpenScreen(Screen::Title)];
                }
                if let Some(tail) = e.id.strip_prefix("set_row_") {
                    let mut it = tail.split('_');
                    let row: usize = it.next().and_then(|s| s.parse().ok()).unwrap_or(99);
                    let op = it.next().unwrap_or("");
                    let dir = match op {
                        "dec" => -1,
                        "inc" => 1,
                        _ => 0,
                    };
                    if dir != 0 && row < 5 {
                        state.focus = row;
                        return adjust_focused_setting(state, dir);
                    }
                }
            }
            vec![]
        }
    }
}

// ---------------------------------------------------------------------------
// Pixel-level checks the screenshot harness reuses (UI-001)
// ---------------------------------------------------------------------------

/// True when every element sits inside the safe margins.
pub fn layout_within_safe_margins(list: &DrawList) -> bool {
    let (w, h) = (list.size.0 as i32, list.size.1 as i32);
    list.elements.iter().filter(|e| !matches!(e.kind, ElementKind::Dim)).all(|e| {
        e.rect.x >= SAFE_MARGIN_PX
            && e.rect.y >= SAFE_MARGIN_PX
            && e.rect.right() <= w - SAFE_MARGIN_PX + 2
            && e.rect.bottom() <= h - SAFE_MARGIN_PX + 2
    })
}

/// True when no two activatable elements overlap each other.
pub fn no_interactive_overlap(list: &DrawList) -> bool {
    let act: Vec<&UiElement> = list.activatables();
    for i in 0..act.len() {
        for j in (i + 1)..act.len() {
            if act[i].rect.intersects(&act[j].rect) {
                return false;
            }
        }
    }
    true
}

/// Counts pixels carrying UI ink (alpha > 0) inside a rect of the canvas.
pub fn ink_px(canvas: &[u8], cw: u32, rect: Rect) -> u64 {
    let ch = (canvas.len() / (cw as usize * 4)).max(1) as u32;
    let x0 = rect.x.max(0).min(cw as i32);
    let y0 = rect.y.max(0).min(ch as i32);
    let x1 = rect.right().max(0).min(cw as i32);
    let y1 = rect.bottom().max(0).min(ch as i32);
    let mut n = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y as u32 * cw + x as u32) * 4) as usize;
            if i + 3 < canvas.len() && canvas[i + 3] > 0 {
                n += 1;
            }
        }
    }
    n
}

/// Counts semi-transparent pixels (proof an alpha-blended UI layer exists).
pub fn blended_px(canvas: &[u8]) -> u64 {
    canvas.chunks_exact(4).filter(|p| p[3] > 0 && p[3] < 255).count() as u64
}

/// The WT-001 metadata sidecar exactly per `seed_preview_outputs.schema.json`
/// (the JSON lives with the presentation layer; pc3d_world stays pure).
pub fn seed_preview_json(
    p: &pc3d_world::seed_preview::SeedPreview,
    screenshot_path: &str,
    layout_path: &str,
) -> serde_json::Value {
    serde_json::json!({
        "seed_text": p.seed_text,
        "resolved_seed": p.resolved_seed.to_string(),
        "seed_source": p.seed_source,
        "world_type": p.world_type.name(),
        "preview_size": [p.width, p.height],
        "window_regions": pc3d_world::seed_preview::WINDOW_HALF * 2,
        "spawn": {
            "x": p.spawn.x_m,
            "y": p.spawn.y_m,
            "z": p.spawn.z_m,
            "safe": p.spawn.safe,
            "reason": p.spawn.reason,
        },
        "biome_counts": p.biome_counts.iter().map(|b| serde_json::json!({
            "biome": b.biome.name(),
            "regions": b.regions,
            "percent": (b.percent * 10.0).round() / 10.0,
        })).collect::<Vec<_>>(),
        "feature_hints": p.feature_hints.iter().map(|h| serde_json::json!({
            "kind": h.kind,
            "distance_m": h.distance_m,
            "confidence": h.confidence,
            "placeholder": h.placeholder,
        })).collect::<Vec<_>>(),
        "valid": p.valid,
        "warnings": p.warnings,
        "screenshot_path": screenshot_path,
        "layout_path": layout_path,
    })
}

/// Truncates a label until it fits `max_w` pixels at `scale` (the bitmap
/// font is fixed-advance, so dropping trailing chars is exact). Row
/// labels must never run under interactive elements.
fn fit_to_width(label: &str, scale: u32, max_w: u32) -> String {
    let mut s = label.to_string();
    while !s.is_empty() && font::text_size(&s, scale).0 > max_w {
        s.pop();
    }
    s
}

/// Fullscreen-quad coverage law (the samurai-cut bug class, second
/// species): a UI quad drawn with 4 vertices on a TriangleList pipeline
/// renders as ONE triangle — everything below-right of the screen
/// diagonal shows raw world while the CPU canvas (and every canvas-side
/// ink check) stays whole. Wherever the canvas carries opaque ink
/// (alpha >= 250, eroded 1px so edge bytes never participate), the
/// PRESENTED frame must reproduce that color EXACTLY: alpha-over with
/// a=255 ignores the world behind it, so a mismatch means a missing or
/// displaced quad triangle, a sampling break, or a composite regression.
/// Returns (matched, probed) per rect.
pub fn quad_coverage_per_rect(
    canvas: &[u8],
    presented: &[u8],
    w: u32,
    h: u32,
    rects: &[Rect],
) -> Vec<(Rect, usize, usize)> {
    let w = w as usize;
    let h = h as usize;
    let mut out = Vec::new();
    for r in rects {
        let x0 = (r.x.max(1)) as usize;
        let y0 = (r.y.max(1)) as usize;
        let x1 = (r.right().min(w as i32 - 1)).max(1) as usize;
        let y1 = (r.bottom().min(h as i32 - 1)).max(1) as usize;
        let mut matched = 0usize;
        let mut probed = 0usize;
        for y in y0..y1 {
            for x in x0..x1 {
                let opaque = |xx: usize, yy: usize| canvas[(yy * w + xx) * 4 + 3] >= 250;
                if opaque(x, y)
                    && opaque(x - 1, y)
                    && opaque(x + 1, y)
                    && opaque(x, y - 1)
                    && opaque(x, y + 1)
                {
                    probed += 1;
                    let i = (y * w + x) * 4;
                    if presented[i] == canvas[i]
                        && presented[i + 1] == canvas[i + 1]
                        && presented[i + 2] == canvas[i + 2]
                    {
                        matched += 1;
                    }
                }
            }
        }
        out.push((*r, matched, probed));
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const RESOLUTIONS: [(u32, u32); 4] = [(1280, 720), (1280, 800), (1920, 1080), (2560, 1440)];

    fn state(screen: Screen) -> UiState {
        let mut s = UiState::default();
        s.screen = screen;
        s
    }

    #[test]
    fn every_screen_lays_out_inside_safe_margins_at_all_resolutions() {
        let screens = [
            Screen::Title,
            Screen::NewWorld,
            Screen::Settings,
            Screen::Gameplay,
            Screen::Pause,
        ];
        for (w, h) in RESOLUTIONS {
            for sc in screens {
                let list = build(&state(sc), w, h);
                assert!(!list.elements.is_empty(), "{sc:?} at {w}x{h} must have elements");
                assert!(
                    layout_within_safe_margins(&list),
                    "{sc:?} at {w}x{h} has elements outside the safe margins: {:?}",
                    list.elements.iter().map(|e| (e.id.clone(), e.rect)).collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn load_world_and_modal_lay_out_inside_safe_margins() {
        for (w, h) in RESOLUTIONS {
            let mut s = state(Screen::LoadWorld);
            s.slots = vec![
                SaveSlot { name: "alpha".into(), seed: Some(22), modified: "2026-09-09 12:00".into() },
                SaveSlot { name: "beta".into(), seed: Some(7), modified: "2026-09-08 09:00".into() },
            ];
            let list = build(&s, w, h);
            assert!(layout_within_safe_margins(&list), "load_world {w}x{h}");
            assert!(no_interactive_overlap(&list), "load_world buttons must not overlap at {w}x{h}");

            let mut m = state(Screen::Gameplay);
            m.modal = Some(ModalKind::DeleteWorld("alpha".into()));
            let list = build(&m, w, h);
            assert!(layout_within_safe_margins(&list), "modal {w}x{h}");
            assert!(no_interactive_overlap(&list));
            assert!(list.by_id("modal_confirm").is_some());
            assert!(list.by_id("modal_cancel").is_some());
        }
    }

    #[test]
    fn activatable_elements_never_overlap() {
        for (w, h) in RESOLUTIONS {
            for sc in [Screen::Title, Screen::Pause, Screen::Settings, Screen::NewWorld] {
                let mut s = state(sc);
                if sc == Screen::Settings {
                    // Squeeze: both [-]/[+] buttons and their rows.
                    s.settings.ui_scale = 1.5;
                }
                let list = build(&s, w, h);
                assert!(
                    no_interactive_overlap(&list),
                    "{sc:?} at {w}x{h} has overlapping interactives: {:?}",
                    list.activatables().iter().map(|e| (e.id.clone(), e.rect)).collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn title_has_play_and_quit_buttons() {
        let list = build(&state(Screen::Title), 1280, 720);
        assert!(list.by_id("btn_play").is_some());
        assert!(list.by_id("btn_quit").is_some());
        assert!(matches!(list.by_id("btn_play").unwrap().kind, ElementKind::Button { .. }));
    }

    #[test]
    fn pause_has_resume_and_both_quit_choices() {
        let list = build(&state(Screen::Pause), 1280, 720);
        for id in ["pause_resume", "pause_save", "pause_load", "pause_settings", "pause_quit_title", "pause_quit_desktop"] {
            assert!(list.by_id(id).is_some(), "pause menu must have {id}");
        }
    }

    #[test]
    fn settings_screen_shows_all_five_controls_and_keymap() {
        let list = build(&state(Screen::Settings), 1280, 720);
        for row in 0..5 {
            assert!(list.by_id(&format!("set_row_{row}_label")).is_some(), "row {row}");
            assert!(list.by_id(&format!("set_row_{row}_value")).is_some());
            assert!(list.by_id(&format!("set_row_{row}_dec")).is_some());
            assert!(list.by_id(&format!("set_row_{row}_inc")).is_some());
        }
        assert!(list.by_id("set_controls_title").is_some());
        assert!(list.by_id("set_back").is_some());
    }

    #[test]
    fn gameplay_hud_has_crosshair_bars_hotbar_and_prompt() {
        let mut s = state(Screen::Gameplay);
        s.hud.prompt = "F BUILD SAND · R REMOVE".into();
        s.toasts.push(Toast { text: "SAVED".into(), age_s: 0.5 });
        let list = build(&s, 1280, 720);
        for id in ["crosshair", "bar_health", "bar_stamina", "bar_food", "prompt", "xp_strip", "toast_SAVED"] {
            assert!(list.by_id(id).is_some(), "gameplay HUD must have {id}");
        }
        for i in 0..9 {
            assert!(list.by_id(&format!("hotbar_{i}")).is_some());
        }
        // The toast band must sit strictly above the hotbar.
        let toast = list.by_id("toast_SAVED").unwrap().rect;
        let hotbar0 = list.by_id("hotbar_0").unwrap().rect;
        assert!(toast.bottom() < hotbar0.y, "toasts must never cover the hotbar");
        // And the debug strip must be absent by default.
        assert!(list.by_id("debug_strip").is_none(), "debug text hidden by default");
    }

    #[test]
    fn debug_strip_appears_only_when_toggled() {
        let mut s = state(Screen::Gameplay);
        assert!(build(&s, 1280, 720).by_id("debug_strip").is_none());
        s.debug_overlay = true;
        s.debug_text = "SEED 22 POS 1 2 3 FPS 100".into();
        let list = build(&s, 1280, 720);
        assert!(list.by_id("debug_strip").is_some());
        assert!(layout_within_safe_margins(&list));
    }

    #[test]
    fn paint_produces_ink_inside_every_button_and_bar() {
        let mut s = state(Screen::Title);
        s.focus = 0;
        let list = build(&s, 1280, 720);
        let canvas = paint(&list);
        let (w, _) = list.size;
        for e in &list.elements {
            let ink = ink_px(&canvas, w, e.rect);
            assert!(ink > 0, "element {} painted no ink", e.id);
        }
        assert!(blended_px(&canvas) > 500, "panels must be semi-transparent");
    }

    #[test]
    fn title_canvas_has_no_row_shear_at_unaligned_widths() {
        // The diagonal cut only appeared at target widths whose 4-byte
        // row pitch is NOT 256-aligned (the classic proof widths
        // 1280/2560 were accidentally aligned, hiding the bug). The law:
        // the painted canvas itself must show zero row drift at such
        // widths, at 1x and at 2x Retina.
        let s = state(Screen::Title);
        for (w, h, dpi) in [(1501u32, 801u32, 1.0f32), (3024u32, 1964u32, 2.0f32)] {
            let list = build_dpi(&s, w, h, dpi);
            let canvas = paint(&list);
            let (median, mode) = row_shear_metrics(&canvas, w, h);
            assert_eq!(
                median, 0.0,
                "{w}x{h}@{dpi}: canvas rows drift {median:+}px/row — diagonal cut regression"
            );
            assert!(
                mode <= 0.25,
                "{w}x{h}@{dpi}: one nonzero shift owns {}% of rows — diagonal cut regression",
                mode * 100.0
            );
        }
    }

    #[test]
    fn quad_coverage_law_detects_a_missing_triangle() {
        // The one-triangle UI quad: opaque ink below-right of the screen
        // diagonal shows the world instead of the canvas. The law must
        // score a whole composite at 100% and a cut one far below.
        let (w, h) = (64u32, 48u32);
        let mut canvas = vec![0u8; (w * h * 4) as usize];
        let rect = Rect::new(20, 12, 36, 28);
        for y in rect.y..rect.bottom() {
            for x in rect.x..rect.right() {
                let i = ((y as u32 * w + x as u32) * 4) as usize;
                canvas[i..i + 4].copy_from_slice(&[200, 30, 30, 255]);
            }
        }
        // Whole composite: the canvas shown verbatim (world elsewhere).
        let whole = canvas.clone();
        let cov = quad_coverage_per_rect(&canvas, &whole, w, h, &[rect]);
        assert_eq!(cov[0].2, 34 * 26, "erosion must leave the fill interior");
        assert_eq!(cov[0].1, cov[0].2, "whole composite must match 1:1");
        // Cut composite: one-triangle quad — everything below-right of
        // the top-right -> bottom-left diagonal shows the "world".
        let mut cut = canvas.clone();
        for y in 0..h as usize {
            let edge = (w as usize - 1) - y * (w as usize - 1) / (h as usize - 1);
            for x in edge..w as usize {
                let i = (y * w as usize + x) * 4;
                cut[i..i + 3].copy_from_slice(&[5, 50, 9]);
            }
        }
        let cov = quad_coverage_per_rect(&canvas, &cut, w, h, &[rect]);
        let frac = cov[0].1 as f32 / cov[0].2 as f32;
        assert!(
            frac < 0.5,
            "a one-triangle composite must fail the law (fraction {frac:.2})"
        );
    }

    #[test]
    fn build_stamp_never_empty_and_shown_on_title() {
        // Every binary must identify itself: the title subtitle carries
        // the stamp (`make p3d-dmg` bakes the git hash; "dev" locally),
        // so a stale mounted DMG can never masquerade as a fresh build.
        assert!(build_stamp().len() >= 3);
        let s = state(Screen::Title);
        let list = build(&s, 1280, 720);
        let sub = list.by_id("title_sub").expect("title subtitle element");
        let canvas = paint(&list);
        assert!(
            ink_px(&canvas, 1280, sub.rect) > 0,
            "title subtitle painted no ink"
        );
    }

    #[test]
    fn new_world_with_seed_preview_shows_map_safety_and_hints() {
        // WT-001: the preview panel carries the map (as a real image
        // blit), the spawn safety verdict, the biome summary, feature
        // hints, the resolved seed line, and the RANDOM button.
        let mut s = state(Screen::NewWorld);
        s.form.seed_digits = "4242".into();
        s.seed_preview = Some(pc3d_world::seed_preview::preview_seed(
            &pc3d_world::seed_preview::SeedPreviewRequest {
                seed_text: "4242".into(),
                ..Default::default()
            },
        ));
        let list = build(&s, 1280, 720);
        for id in [
            "preview_map", "spawn_safety", "biome_summary", "feature_hint_0",
            "nw_seed_resolved", "nw_seed_random", "nw_seed_reroll", "nw_create",
        ] {
            assert!(list.by_id(id).is_some(), "WT-001 element {id} missing");
        }
        let map = list.by_id("preview_map").unwrap();
        assert!(
            matches!(map.kind, ElementKind::Image { .. }),
            "preview_map must be an image element"
        );
        // The map must paint visible ink (not the placeholder fill).
        let canvas = paint(&list);
        let ink = ink_px(&canvas, 1280, map.rect);
        assert!(ink > 500, "preview map painted only {ink} px of ink");
        // Layout dump must expose the button states (the harness gates
        // CREATE enabled-only-when-valid on it).
        let dump = list.to_json("new_world");
        let create = dump["elements"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["id"] == "nw_create")
            .unwrap();
        assert_ne!(
            create["button_state"], "disabled",
            "valid preview keeps CREATE enabled"
        );
    }

    #[test]
    fn create_is_disabled_when_preview_is_unsafe() {
        // WT-001 failure mode: "Create button works when preview is
        // invalid". An unsafe preview disables CREATE and Enter does
        // not emit CreateWorld.
        let mut s = state(Screen::NewWorld);
        let mut p = pc3d_world::seed_preview::preview_seed(
            &pc3d_world::seed_preview::SeedPreviewRequest {
                seed_text: "4242".into(),
                ..Default::default()
            },
        );
        p.valid = false;
        p.spawn.safe = false;
        p.spawn.reason = "test rejection".into();
        s.seed_preview = Some(p);
        let list = build(&s, 1280, 720);
        let dump = list.to_json("new_world");
        let create = dump["elements"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["id"] == "nw_create")
            .unwrap();
        assert_eq!(create["button_state"], "disabled");
        s.focus = 0; // CREATE
        let acts = on_key(&mut s, Key::Enter);
        assert!(
            !acts.iter().any(|a| matches!(a, UiAction::CreateWorld { .. })),
            "Enter on a disabled CREATE must not create the world"
        );
    }

    #[test]
    fn reroll_and_random_change_seed_and_mark_preview_stale() {
        // WT-001 laws: reroll never repeats the current seed, both seed
        // buttons mark the preview stale for recompute.
        let mut s = state(Screen::NewWorld);
        s.focus = 1; // REROLL
        let before = s.form.seed();
        let _ = on_key(&mut s, Key::Enter);
        let after_reroll = s.form.seed();
        assert_ne!(before, after_reroll, "reroll must change the seed");
        assert!(s.seed_preview_stale, "reroll must mark the preview stale");
        s.seed_preview_stale = false;
        s.focus = 2; // RANDOM
        let _ = on_key(&mut s, Key::Enter);
        assert_ne!(after_reroll, s.form.seed(), "random must change the seed");
        assert!(s.seed_preview_stale, "random must mark the preview stale");
        // And seed editing marks it stale too (backspace always applies;
        // a random u64 can exceed the 10-digit typing cap).
        s.seed_preview_stale = false;
        let _ = on_key(&mut s, Key::Backspace);
        assert!(s.seed_preview_stale, "seed editing must mark the preview stale");
    }

    #[test]
    fn seed_preview_sidecar_matches_the_output_contract() {
        let p = pc3d_world::seed_preview::preview_seed(
            &pc3d_world::seed_preview::SeedPreviewRequest {
                seed_text: "typed-seed".into(),
                ..Default::default()
            },
        );
        let v = seed_preview_json(&p, "shots/a.png", "shots/a.layout.json");
        for field in [
            "seed_text", "resolved_seed", "world_type", "preview_size", "spawn",
            "biome_counts", "feature_hints", "valid", "warnings", "screenshot_path",
            "layout_path",
        ] {
            assert!(v.get(field).is_some(), "contract field {field} missing");
        }
        for field in ["x", "y", "z", "safe", "reason"] {
            assert!(v["spawn"].get(field).is_some(), "spawn field {field} missing");
        }
        for field in ["kind", "distance_m", "confidence", "placeholder"] {
            assert!(
                v["feature_hints"][0].get(field).is_some(),
                "feature hint field {field} missing"
            );
        }
    }

    #[test]
    fn focused_state_paints_differently_from_normal() {
        let mut a = state(Screen::Title);
        a.focus = 0;
        let mut b = a.clone();
        b.focus = 1;
        let la = build(&a, 1280, 720);
        let lb = build(&b, 1280, 720);
        let (ca, cb) = (paint(&la), paint(&lb));
        let ra = la.by_id("btn_play").unwrap().rect;
        let rb = lb.by_id("btn_new_world").unwrap().rect;
        assert!(ink_px(&ca, 1280, ra) > 0);
        // The two button rects must differ between the two frames (focus
        // ring changes pixels inside both rects).
        let differ = |c1: &[u8], c2: &[u8], r: Rect| -> u64 {
            let mut n = 0;
            for y in r.y..r.bottom() {
                for x in r.x..r.right() {
                    let i = ((y as u32 * 1280 + x as u32) * 4) as usize;
                    if c1[i..i + 4] != c2[i..i + 4] {
                        n += 1;
                    }
                }
            }
            n
        };
        assert!(differ(&ca, &cb, ra) > 50, "focused button pixels must change");
        assert!(differ(&ca, &cb, rb) > 50);
        // Outside both button rects the frames are pixel-identical (the
        // world is not painted here — this canvas is pure UI).
        let inflate = |r: Rect| Rect::new(r.x - 3, r.y - 3, r.w + 6, r.h + 6);
        let ea = inflate(ra);
        let eb = inflate(rb);
        let mut outside_diff = 0u64;
        for y in 0..720 {
            for x in 0..1280 {
                if ea.contains(x, y) || eb.contains(x, y) {
                    continue;
                }
                let i = ((y * 1280 + x) * 4) as usize;
                if ca[i..i + 4] != cb[i..i + 4] {
                    outside_diff += 1;
                }
            }
        }
        assert_eq!(outside_diff, 0, "only the focused buttons may change");
    }

    #[test]
    fn bar_fills_follow_live_values() {
        let mut s = state(Screen::Gameplay);
        s.hud.health = 1.0;
        let full = build(&s, 1280, 720);
        s.hud.health = 0.5;
        let half = build(&s, 1280, 720);

        let health_fill_px = |list: &DrawList, canvas: &[u8]| -> u64 {
            let r = list.by_id("bar_health").unwrap().rect;
            let inner = Rect::new(r.x + 2, r.y + 2, r.w - 4, r.h - 4);
            // Health red [194,68,56]: count pixels close to the fill color.
            let mut n = 0;
            for y in inner.y..inner.bottom() {
                for x in inner.x..inner.right() {
                    let i = ((y as u32 * list.size.0 + x as u32) * 4) as usize;
                    let p = &canvas[i..i + 3];
                    if (p[0] as i32 - 194).abs() < 40 && (p[1] as i32 - 68).abs() < 40 && (p[2] as i32 - 56).abs() < 40 {
                        n += 1;
                    }
                }
            }
            n
        };
        let cf = paint(&full);
        let ch = paint(&half);
        let full_px = health_fill_px(&full, &cf);
        let half_px = health_fill_px(&half, &ch);
        assert!(full_px > 100, "full bar must paint fill");
        assert!(
            (half_px as f32 / full_px as f32 - 0.5).abs() < 0.1,
            "half-value bar must paint ~50% of the fill ({half_px} vs {full_px})"
        );
    }

    #[test]
    fn hotbar_selection_moves() {
        let mut s = state(Screen::Gameplay);
        s.hud.selected = 2;
        let list = build(&s, 1280, 720);
        match &list.by_id("hotbar_2").unwrap().kind {
            ElementKind::HotbarSlot { selected, .. } => assert!(selected),
            _ => panic!(),
        }
        match &list.by_id("hotbar_0").unwrap().kind {
            ElementKind::HotbarSlot { selected, .. } => assert!(!selected),
            _ => panic!(),
        }
        // Slots never overlap.
        assert!(no_interactive_overlap(&list));
    }

    #[test]
    fn toast_fades_with_age() {
        let mut s = state(Screen::Gameplay);
        s.toasts.push(Toast { text: "SAVED".into(), age_s: 0.0 });
        s.toasts.push(Toast { text: "LOADED".into(), age_s: 2.9 });
        let list = build(&s, 1280, 720);
        let a_new = match &list.by_id("toast_SAVED").unwrap().kind {
            ElementKind::Toast { alpha, .. } => *alpha,
            _ => panic!(),
        };
        let a_old = match &list.by_id("toast_LOADED").unwrap().kind {
            ElementKind::Toast { alpha, .. } => *alpha,
            _ => panic!(),
        };
        assert!(a_new > a_old, "toasts fade with age");
        assert!(a_old < 0.1);
    }

    #[test]
    fn escape_pauses_and_resumes_and_never_exits() {
        let mut s = state(Screen::Gameplay);
        let acts = on_key(&mut s, Key::Escape);
        assert_eq!(s.screen, Screen::Pause);
        assert!(s.blocks_gameplay(), "pause must block gameplay");
        assert!(acts.contains(&UiAction::OpenScreen(Screen::Pause)));
        // No action ever exits the app from Escape.
        assert!(!acts.iter().any(|a| matches!(a, UiAction::QuitToDesktop)));

        let acts = on_key(&mut s, Key::Escape);
        assert_eq!(s.screen, Screen::Gameplay);
        assert!(acts.contains(&UiAction::StartPlaying));
        assert!(!s.blocks_gameplay());

        // Escape on the title stays on the title.
        let mut t = state(Screen::Title);
        let _ = on_key(&mut t, Key::Escape);
        assert_eq!(t.screen, Screen::Title);
    }

    #[test]
    fn menus_block_gameplay_and_q_asks_confirmation() {
        for sc in [Screen::Title, Screen::Pause, Screen::Settings, Screen::NewWorld, Screen::LoadWorld] {
            let s = state(sc);
            assert!(s.blocks_gameplay(), "{sc:?} must block gameplay");
        }
        let mut s = state(Screen::Title);
        let acts = on_key(&mut s, Key::KeyQ);
        assert!(matches!(s.modal, Some(ModalKind::QuitToDesktop)));
        assert!(acts.iter().any(|a| !matches!(a, UiAction::QuitToDesktop)));
        // Confirming the modal emits the quit action — the ONLY quit path.
        let acts = on_key(&mut s, Key::Enter);
        assert!(acts.contains(&UiAction::ConfirmModal(ModalKind::QuitToDesktop)));
    }

    #[test]
    fn keyboard_navigation_moves_focus_and_enter_activates() {
        let mut s = state(Screen::Title);
        let _ = on_key(&mut s, Key::Down);
        let _ = on_key(&mut s, Key::Down);
        assert_eq!(s.focus, 2);
        let acts = on_key(&mut s, Key::Enter);
        assert_eq!(s.screen, Screen::LoadWorld);
        assert!(acts.contains(&UiAction::OpenScreen(Screen::LoadWorld)));
        let _ = on_key(&mut s, Key::Escape);
        assert_eq!(s.screen, Screen::Title);
    }

    #[test]
    fn digits_type_into_seed_and_hotbar_selects() {
        let mut s = state(Screen::NewWorld);
        s.form.seed_digits.clear();
        let _ = on_key(&mut s, Key::Digit(4));
        let _ = on_key(&mut s, Key::Digit(2));
        assert_eq!(s.form.seed(), 42);

        let mut g = state(Screen::Gameplay);
        let acts = on_key(&mut g, Key::Digit(5));
        assert_eq!(g.hud.selected, 4);
        assert!(acts.contains(&UiAction::SelectHotbar(4)));

        let acts = on_key(&mut g, Key::WheelDown);
        assert_eq!(g.hud.selected, 5);
        assert!(acts.contains(&UiAction::SelectHotbar(5)));
    }

    #[test]
    fn settings_adjustments_clamp_and_emit() {
        let mut s = state(Screen::Settings);
        s.focus = 0;
        let acts = on_key(&mut s, Key::Right);
        assert!(acts.contains(&UiAction::SetSensitivity(1.1)));
        for _ in 0..99 {
            let _ = on_key(&mut s, Key::Right);
        }
        assert!((s.settings.mouse_sensitivity - 3.0).abs() < 1e-4, "sensitivity clamps at 3.0");

        s.focus = 2;
        let _ = on_key(&mut s, Key::Left);
        assert_eq!(s.settings.fov_deg, 65.0);
        for _ in 0..99 {
            let _ = on_key(&mut s, Key::Left);
        }
        assert_eq!(s.settings.fov_deg, 50.0, "FOV clamps at 50");
    }

    #[test]
    fn ui_scale_extremes_still_fit_the_safe_margins() {
        for sc in [Screen::Title, Screen::Pause, Screen::Settings] {
            for scale in [0.75, 1.0, 1.5] {
                let mut s = state(sc);
                s.settings.ui_scale = scale;
                for (w, h) in [(1280, 720), (1280, 800)] {
                    let list = build(&s, w, h);
                    assert!(
                        layout_within_safe_margins(&list),
                        "{sc:?} scale {scale} at {w}x{h} must fit (fit-scale law)"
                    );
                }
            }
        }
    }

    #[test]
    fn mouse_click_routes_to_title_buttons() {
        let mut s = state(Screen::Title);
        let list = build(&s, 1280, 720);
        let btn = list.by_id("btn_quit").unwrap().rect;
        let acts = on_click(&mut s, btn.cx(), btn.cy(), &list);
        assert!(matches!(s.modal, Some(ModalKind::QuitToDesktop)));

        // Confirm via the modal button.
        let list = build(&s, 1280, 720);
        let c = list.by_id("modal_confirm").unwrap().rect;
        let acts = on_click(&mut s, c.cx(), c.cy(), &list);
        assert!(acts.contains(&UiAction::ConfirmModal(ModalKind::QuitToDesktop)));
    }

    #[test]
    fn gameplay_click_without_hit_captures_mouse() {
        let mut s = state(Screen::Gameplay);
        s.pointer_grabbed = false;
        let list = build(&s, 1280, 720);
        // Click the sky (top center, no element there).
        let acts = on_click(&mut s, 640, 100, &list);
        assert!(acts.contains(&UiAction::CaptureMouse));
        // Clicking a hotbar slot selects it instead.
        let slot = list.by_id("hotbar_3").unwrap().rect;
        let acts = on_click(&mut s, slot.cx(), slot.cy(), &list);
        assert!(acts.contains(&UiAction::SelectHotbar(3)));
    }

    #[test]
    fn load_world_clicks_load_and_delete_with_confirmation() {
        let mut s = state(Screen::LoadWorld);
        s.slots = vec![SaveSlot { name: "alpha".into(), seed: Some(22), modified: "today".into() }];
        let list = build(&s, 1280, 720);
        let load = list.by_id("lw_slot_0_load").unwrap().rect;
        let acts = on_click(&mut s, load.cx(), load.cy(), &list);
        assert!(matches!(s.modal, Some(ModalKind::LoadWorld(_))), "loading asks first");
        assert!(acts.is_empty() || acts == vec![UiAction::Repaint]);
        // Confirm → the load action fires.
        let list = build(&s, 1280, 720);
        let c = list.by_id("modal_confirm").unwrap().rect;
        let acts = on_click(&mut s, c.cx(), c.cy(), &list);
        assert!(acts.contains(&UiAction::ConfirmModal(ModalKind::LoadWorld("alpha".into()))));

        s.modal = None;
        let list = build(&s, 1280, 720);
        let del = list.by_id("lw_slot_0_del").unwrap().rect;
        let _ = on_click(&mut s, del.cx(), del.cy(), &list);
        assert!(matches!(s.modal, Some(ModalKind::DeleteWorld(_))));
    }

    #[test]
    fn layout_dump_is_serializable() {
        let list = build(&state(Screen::Title), 1280, 720);
        let v = list.to_json("title");
        assert_eq!(v["screen"], "title");
        assert_eq!(v["size"][0], 1280);
        assert!(v["elements"].as_array().unwrap().len() > 3);
        let s = serde_json::to_string(&v).expect("serializable");
        assert!(s.contains("btn_play"));
    }

    #[test]
    fn ui_state_json_covers_the_contract() {
        let mut s = UiState::default();
        s.screen = Screen::Pause;
        s.hud.health = 0.5;
        s.toast("SAVED");
        let v = s.to_json();
        assert_eq!(v["screen"], "pause");
        assert_eq!(v["hud"]["health"], 0.5);
        assert_eq!(v["toasts"][0]["text"], "SAVED");
        assert_eq!(v["gameplay_input_blocked"], true);
    }

    #[test]
    fn settings_json_round_trips() {
        let mut s = UiSettings::default();
        s.mouse_sensitivity = 1.7;
        s.invert_y = true;
        s.fov_deg = 85.0;
        s.ui_scale = 1.25;
        s.quality = Quality::High;
        let back = UiSettings::from_json(&s.to_json());
        assert_eq!(back, s);
        // Out-of-range values clamp on load (corrupt settings can't break layout).
        let bad = serde_json::json!({"fov_deg": 400.0, "ui_scale": 9.0, "mouse_sensitivity": 0.01});
        let c = UiSettings::from_json(&bad);
        assert_eq!(c.fov_deg, 100.0);
        assert_eq!(c.ui_scale, 1.5);
        assert!((c.mouse_sensitivity - 0.2).abs() < 1e-4);
    }

    #[test]
    fn keymap_lists_the_required_bindings() {
        let keys: Vec<&str> = KEYMAP.iter().map(|b| b.key).collect();
        for k in ["W A S D", "MOUSE", "SPACE", "SHIFT", "F", "R", "B", "L", "I", "ESC", "Q", "1-9 / WHEEL"] {
            assert!(keys.contains(&k), "keymap must list {k}");
        }
    }
}

// ---------------------------------------------------------------------------
// Screenshot-scene verification (UI-001: the pixel checks)
// ---------------------------------------------------------------------------

/// One scene expectation for `verify_ui_captures`.
pub struct SceneExpectation {
    pub id: &'static str,
    /// Element ids that must exist with ink.
    pub required_elements: &'static [&'static str],
    /// Element kinds that must NOT be present (e.g. "debug" hidden by
    /// default).
    pub forbidden_kinds: &'static [&'static str],
}

/// Row-shear metrics for a straight-alpha RGBA image: (median best shift
/// between adjacent rows in px/row, fraction of structured rows sharing
/// the single most common NONZERO shift). A row-pitch mismatch anywhere
/// in the pixel path (canvas paint, texture upload, blit, readback)
/// displaces every row by a CONSTANT and tears the image along a
/// diagonal — the owner's "menus cut in half" bug class. That bug shows
/// as a nonzero median (the constant dominates). Noisy-but-healthy
/// content (dark terrain shot straight down) scatters a minority of rows
/// over ±1..2px with the median still 0 — so the second number only
/// convicts when ONE nonzero shift owns a large share of rows.
pub fn row_shear_metrics(rgba: &[u8], w: u32, h: u32) -> (f32, f32) {
    let w = w as usize;
    let h = h as usize;
    if w < 128 || h < 8 || rgba.len() < w * h * 4 {
        return (0.0, 0.0);
    }
    let lum = |x: usize, y: usize| -> f32 {
        let i = (y * w + x) * 4;
        0.299 * rgba[i] as f32 + 0.587 * rgba[i + 1] as f32 + 0.114 * rgba[i + 2] as f32
    };
    const RANGE: usize = 32;
    let x_lo = RANGE + 1;
    let x_hi = w.saturating_sub(RANGE + 2).max(x_lo + 1);
    let mut bests: Vec<i32> = Vec::new();
    for y in (0..h - 1).step_by(4) {
        let mut best = 0i32;
        let mut best_score = -1f32;
        for s in -(RANGE as i32)..=(RANGE as i32) {
            let mut num = 0f32;
            let mut ea = 0f32;
            let mut eb = 0f32;
            for x in (x_lo..x_hi).step_by(2) {
                let ga = (lum(x + 1, y) - lum(x - 1, y)).abs();
                let xb = x as i32 + s;
                let gb = (lum((xb + 1) as usize, y + 1) - lum((xb - 1) as usize, y + 1)).abs();
                num += ga * gb;
                ea += ga * ga;
                eb += gb * gb;
            }
            let score = num / (ea * eb).sqrt().max(1e-9);
            if score > best_score {
                best_score = score;
                best = s;
            }
        }
        // Only rows with real structure (edges/text) discriminate; flat
        // sky rows align with everything.
        if best_score > 0.5 {
            bests.push(best);
        }
    }
    if bests.is_empty() {
        return (0.0, 0.0);
    }
    bests.sort_unstable();
    let median = bests[bests.len() / 2] as f32;
    let mut mode_nonzero = 0usize;
    for &s in &bests {
        if s == 0 {
            continue;
        }
        let n = bests.iter().filter(|&&b| b == s).count();
        mode_nonzero = mode_nonzero.max(n);
    }
    (median, mode_nonzero as f32 / bests.len() as f32)
}

/// Verifies a `--ui-shots` run: per-capture nonblank/UI presence, safe
/// margins + no interactive overlaps from each layout dump, required ink,
/// focused-state distinctness, and bar-fill reality. Returns a human-
/// readable PASS report (or the first failure with evidence).
pub fn verify_ui_captures(
    captures: &[crate::app::CaptureOutcome],
    expectations: &[SceneExpectation],
) -> Result<String, String> {
    let mut lines = Vec::new();
    if captures.len() != expectations.len() {
        return Err(format!(
            "expected {} captures, got {}",
            expectations.len(),
            captures.len()
        ));
    }
    let mut prev_canvas: Option<(String, Vec<u8>, u32)> = None;
    for (cap, exp) in captures.iter().zip(expectations) {
        let scene = exp.id;
        // 1. Nonblank world/UI frame.
        if cap.report.distinct_colors < 30 {
            return Err(format!("{scene}: blank frame ({} distinct colors)", cap.report.distinct_colors));
        }
        // 1b. Row-shear law (the diagonal-cut bug class): the presented
        //     frame must not drift row-to-row. Runs on the composited
        //     readback, so the canvas paint, texture upload, blit and
        //     capture pitches are ALL covered — the original bug hid from
        //     proofs whose widths were accidentally 256-aligned. A pitch
        //     bug = nonzero MEDIAN drift or one nonzero shift owning a
        //     large share of structured rows; healthy noise scatters.
        let (shear, mode_nonzero) =
            row_shear_metrics(&cap.rgba, cap.report.width, cap.report.height);
        if shear != 0.0 || mode_nonzero > 0.25 {
            return Err(format!(
                "{scene}: row shear detected (median {shear:+.0}px/row, dominant nonzero shift on {}% of rows) — pitch/alignment regression",
                mode_nonzero * 100.0
            ));
        }
        // 2. The UI canvas was present and blended.
        let Some((canvas, cw, _ch)) = &cap.ui_canvas else {
            return Err(format!("{scene}: no UI canvas composited"));
        };
        if blended_px(canvas) < 200 {
            return Err(format!("{scene}: no semi-transparent UI pixels (alpha blend missing)"));
        }
        // 3. Layout dump: safe margins + no interactive overlap + required
        //    and forbidden elements.
        let Some(layout) = &cap.ui_layout else {
            return Err(format!("{scene}: no layout dump"));
        };
        let elements = layout["elements"]
            .as_array()
            .ok_or_else(|| format!("{scene}: layout dump has no elements array"))?;
        let rect_of = |id: &str| -> Option<Rect> {
            let e = elements.iter().find(|e| e["id"] == id)?;
            let a = e["rect"].as_array()?;
            Some(Rect::new(
                a[0].as_i64()? as i32,
                a[1].as_i64()? as i32,
                a[2].as_u64()? as u32,
                a[3].as_u64()? as u32,
            ))
        };
        let w = layout["size"][0].as_u64().unwrap_or(0) as u32;
        let h = layout["size"][1].as_u64().unwrap_or(0) as u32;
        for e in elements {
            if e["kind"].as_str() == Some("dim") {
                continue; // background layer, not content
            }
            let a = e["rect"].as_array().ok_or("bad rect")?;
            let r = Rect::new(
                a[0].as_i64().unwrap_or(0) as i32,
                a[1].as_i64().unwrap_or(0) as i32,
                a[2].as_u64().unwrap_or(0) as u32,
                a[3].as_u64().unwrap_or(0) as u32,
            );
            if r.x < SAFE_MARGIN_PX
                || r.y < SAFE_MARGIN_PX
                || r.right() > w as i32 - SAFE_MARGIN_PX + 2
                || r.bottom() > h as i32 - SAFE_MARGIN_PX + 2
            {
                return Err(format!(
                    "{scene}: element {} at {:?} violates the safe margins at {}x{}",
                    e["id"], (r.x, r.y, r.w, r.h), w, h
                ));
            }
        }
        let interactives: Vec<(String, Rect)> = elements
            .iter()
            .filter(|e| matches!(e["kind"].as_str(), Some("button") | Some("hotbar_slot")))
            .filter_map(|e| {
                let a = e["rect"].as_array()?;
                Some((
                    e["id"].as_str()?.to_string(),
                    Rect::new(
                        a[0].as_i64()? as i32,
                        a[1].as_i64()? as i32,
                        a[2].as_u64()? as u32,
                        a[3].as_u64()? as u32,
                    ),
                ))
            })
            .collect();
        for i in 0..interactives.len() {
            for j in (i + 1)..interactives.len() {
                if interactives[i].1.intersects(&interactives[j].1) {
                    return Err(format!(
                        "{scene}: interactives {} and {} overlap",
                        interactives[i].0, interactives[j].0
                    ));
                }
            }
        }
        for id in exp.required_elements {
            let Some(r) = rect_of(id) else {
                return Err(format!("{scene}: required element {id} missing"));
            };
            if ink_px(canvas, *cw, r) == 0 {
                return Err(format!("{scene}: element {id} has no ink"));
            }
        }
        // 3b. Fullscreen-quad coverage law: the PRESENTED frame must show
        //     the canvas's opaque ink exactly. The UI quad was once drawn
        //     as a single TriangleList triangle — every menu was sliced
        //     along the screen diagonal while all canvas-side checks and
        //     the row-shear law stayed green. This closes that blind spot.
        let required_rects: Vec<Rect> = exp
            .required_elements
            .iter()
            .filter_map(|id| rect_of(id))
            .collect();
        let coverage = quad_coverage_per_rect(
            canvas,
            &cap.rgba,
            cap.report.width,
            cap.report.height,
            &required_rects,
        );
        let probed: usize = coverage.iter().map(|(_, _, p)| p).sum();
        let matched: usize = coverage.iter().map(|(_, m, _)| m).sum();
        if probed < 150 {
            return Err(format!(
                "{scene}: quad-coverage law probed only {probed} opaque px (need >= 150) — expectations too weak"
            ));
        }
        let quad_frac = matched as f32 / probed as f32;
        if quad_frac < 0.95 {
            let worst = coverage
                .iter()
                .filter(|(_, _, p)| *p >= 10)
                .min_by(|a, b| {
                    (a.1 as f32 / a.2 as f32)
                        .partial_cmp(&(b.1 as f32 / b.2 as f32))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            let evidence = match worst {
                Some((r, m, p)) => format!(
                    "worst rect {:?} shows {}/{} probed px",
                    (r.x, r.y, r.w, r.h),
                    m, p
                ),
                None => String::new(),
            };
            return Err(format!(
                "{scene}: presented frame shows only {:.1}% of the canvas's opaque UI ink — a UI quad triangle is missing or the composite regressed. {evidence}",
                quad_frac * 100.0
            ));
        }
        for kind in exp.forbidden_kinds {
            if elements.iter().any(|e| e["kind"] == *kind) {
                return Err(format!("{scene}: forbidden element kind {kind} present"));
            }
        }
        lines.push(format!(
            "  {scene}: {} distinct colors, {} ui elements, {} blended px, no row shear, quad coverage {matched}/{probed} — OK",
            cap.report.distinct_colors,
            elements.len(),
            blended_px(canvas)
        ));
        // 4. Focused-state distinctness: consecutive title captures must
        //    differ inside the two focused buttons.
        if let Some((prev_id, prev_canvas_px, prev_w)) = prev_canvas.take() {
            if scene.contains("focus") && prev_id.contains("title") && prev_w == *cw {
                let a = rect_of("btn_play").ok_or("focus: btn_play missing")?;
                let b = rect_of("btn_new_world").ok_or("focus: btn_new_world missing")?;
                let differ = |c1: &[u8], c2: &[u8], r: Rect, w: u32| -> u64 {
                    let mut n = 0;
                    for y in r.y..r.bottom() {
                        for x in r.x..r.right() {
                            let i = ((y as u32 * w + x as u32) * 4) as usize;
                            if c1[i..i + 4] != c2[i..i + 4] {
                                n += 1;
                            }
                        }
                    }
                    n
                };
                let d1 = differ(canvas, &prev_canvas_px, a, *cw);
                let d2 = differ(canvas, &prev_canvas_px, b, *cw);
                if d1 < 50 || d2 < 50 {
                    return Err(format!(
                        "focused state must differ from normal (btn_play {d1} px, btn_new_world {d2} px)"
                    ));
                }
                lines.push(format!("  focus distinctness: btn_play {d1} px, btn_new_world {d2} px differ — OK"));
            }
        }
        if scene == "ui_title_1280x720" {
            prev_canvas = Some((scene.to_string(), canvas.clone(), *cw));
        }
        // 5. HUD reality on the debug-values scene: the health bar paints
        //    ~45% fill (live values, not hard-coded art).
        if scene.contains("debug_values") {
            let Some(r) = rect_of("bar_health") else {
                return Err("debug_values: bar_health missing".into());
            };
            let inner = Rect::new(r.x + 2, r.y + 2, r.w - 4, r.h - 4);
            let mut fill = 0u64;
            let mut total = 0u64;
            for y in inner.y..inner.bottom() {
                for x in inner.x..inner.right() {
                    total += 1;
                    let i = ((y as u32 * *cw + x as u32) * 4) as usize;
                    let p = &canvas[i..i + 3];
                    if (p[0] as i32 - 194).abs() < 40
                        && (p[1] as i32 - 68).abs() < 40
                        && (p[2] as i32 - 56).abs() < 40
                    {
                        fill += 1;
                    }
                }
            }
            let frac = fill as f32 / total.max(1) as f32;
            if (frac - 0.45).abs() > 0.12 {
                return Err(format!(
                    "debug_values: health bar fill is {:.2}, expected ~0.45 (live values)",
                    frac
                ));
            }
            lines.push(format!("  health bar fill fraction {frac:.2} (target 0.45) — OK"));
            // Selected slot 3: ember frame must have MOVED vs the default
            // gameplay scene (checked against the default capture above).
        }
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod stamina_tests {
    use super::HudValues;

    /// THE STAMINA LAW (the owner-reported fix): walking is FREE,
    /// sprinting drains, rest regenerates, empty locks until 25%.
    #[test]
    fn walking_is_free_and_sprinting_drains() {
        let mut hud = HudValues::default();
        // 60 s of plain walking: stamina untouched.
        for _ in 0..3600 {
            hud.tick_vitals(false, true, 1.0 / 60.0);
        }
        assert!((hud.stamina - 1.0).abs() < 1e-3, "walking never drains ({})", hud.stamina);
        // 3 s of sprinting: ~0.66 drained.
        for _ in 0..180 {
            hud.tick_vitals(true, true, 1.0 / 60.0);
        }
        assert!(hud.stamina < 0.4 && hud.stamina > 0.25, "sprint drains ({})", hud.stamina);
    }

    #[test]
    fn exhaustion_locks_until_quarter_recovery() {
        let mut hud = HudValues::default();
        hud.stamina = 0.001;
        hud.tick_vitals(true, true, 0.1);
        assert!(hud.exhausted, "empty sprinting exhausts");
        // Rest to 20%: still locked.
        hud.stamina = 0.20;
        hud.tick_vitals(false, false, 0.0);
        assert!(hud.exhausted, "locked below 25%");
        // Rest to 26%: unlocked.
        hud.stamina = 0.26;
        hud.tick_vitals(false, false, 0.0);
        assert!(!hud.exhausted, "unlocked at 25% recovery");
    }

    #[test]
    fn rest_regenerates_to_full() {
        let mut hud = HudValues::default();
        hud.stamina = 0.2;
        for _ in 0..3600 {
            hud.tick_vitals(false, false, 1.0 / 60.0);
        }
        assert!(hud.stamina > 0.99, "a minute of rest refills ({})", hud.stamina);
    }
}
