//! LOREFORGE UI kit: theme, easing, and animated widgets on top of egui.
//! The design system every menu/screen shares — buttons with hover glow and
//! spring easing, fade/slide-in panels, toggles, sliders, and painted
//! heart/hunger glyphs (no unicode font boxes).

use egui::{Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

use lf_game::items::{item_def, tool_damage, ItemKind, ToolKind};
use lf_game::survival::ItemStack;

// ------------------------------------------------------------------
// Theme

/// The LOREFORGE palette — the only colors the UI is allowed to use
/// (docs/ui-world-craft/menu/MAIN_MENU_REDESIGN.md). Anything else in UI
/// chrome is a bug. Parchment text, ember accents, iron-brown borders: the
/// interface is built by people who live in Valdenmoor.
pub struct Theme;

impl Theme {
    /// Very dark warm brown-black (the world at night).
    pub const BG: Color32 = Color32::from_rgb(0x1a, 0x14, 0x10);
    /// Slightly lighter, for large surfaces behind panels.
    pub const BG_MID: Color32 = Color32::from_rgb(0x2a, 0x20, 0x18);
    /// Visible panel background.
    pub const PANEL: Color32 = Color32::from_rgb(0x33, 0x2a, 0x1c);
    /// Aged parchment.
    pub const TEXT: Color32 = Color32::from_rgb(0xf0, 0xea, 0xd6);
    /// Muted warm grey, secondary text.
    pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8a, 0x7f, 0x6e);
    /// Locked/unavailable.
    pub const TEXT_DISABLED: Color32 = Color32::from_rgb(0x4a, 0x44, 0x38);
    /// Ember-orange: emphasis, never decoration.
    pub const ACCENT: Color32 = Color32::from_rgb(0xc4, 0x60, 0x2a);
    /// Iron-brown (the Ironborn's color).
    pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x8b, 0x45, 0x13);
    /// Warm dark brown for dividers and borders.
    pub const BORDER: Color32 = Color32::from_rgb(0x4a, 0x3f, 0x2e);
    /// Earthy green (the Free Holds).
    pub const OK: Color32 = Color32::from_rgb(0x6b, 0x8e, 0x23);
    /// Amber-gold, for positive-but-urgent notices.
    pub const WARNING: Color32 = Color32::from_rgb(0xc4, 0xa0, 0x2a);
    /// Dark red.
    pub const BAD: Color32 = Color32::from_rgb(0x8b, 0x20, 0x20);
    /// Hovered menu text: a touch brighter than parchment.
    pub const TEXT_BRIGHT: Color32 = Color32::from_rgb(0xff, 0xf8, 0xee);

    // HUD glyphs keep their semantic colors (a heart is red, hunger is
    // amber) — they are game-world signals, not UI chrome.
    pub const HEART: Color32 = Color32::from_rgb(225, 60, 70);
    pub const HUNGER: Color32 = Color32::from_rgb(210, 150, 50);
    pub const XP: Color32 = Color32::from_rgb(110, 220, 255);
    /// P33 magic: a violet that reads "arcane" against every biome so far.
    pub const MANA: Color32 = Color32::from_rgb(185, 130, 255);
}

/// Push the LOREFORGE palette into egui's global style so every plain
/// widget (buttons in windows, sliders, checkboxes, scroll areas) and every
/// `egui::Window` inherits the kit instead of egui's cool-blue defaults.
/// Called once per frame from `GameState::draw_ui`; idempotent.
pub fn apply_kit_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;
    v.dark_mode = true;
    v.override_text_color = Some(Theme::TEXT);
    v.panel_fill = Theme::BG_MID;
    v.window_fill = Theme::PANEL;
    v.extreme_bg_color = Theme::BG; // scroll areas, slider tracks
    v.faint_bg_color = Theme::BG; // striped rows / alt backgrounds
    // square kit corners everywhere
    v.window_corner_radius = egui::CornerRadius::ZERO;
    v.menu_corner_radius = egui::CornerRadius::ZERO;
    v.window_stroke = Stroke::new(1.0, Theme::BORDER);
    v.window_shadow = egui::Shadow::NONE;
    v.popup_shadow = egui::Shadow::NONE;
    v.widgets.noninteractive.bg_fill = Theme::BG_MID;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Theme::TEXT_DIM);
    v.widgets.inactive.bg_fill = Color32::from_rgba_premultiplied(0x3d, 0x30, 0x1e, 220);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, Theme::TEXT);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, Theme::BORDER);
    v.widgets.hovered.bg_fill = Color32::from_rgba_premultiplied(0x4a, 0x3c, 0x26, 235);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, Theme::TEXT_BRIGHT);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, Theme::ACCENT);
    v.widgets.active.bg_fill = Theme::ACCENT_DIM;
    v.widgets.active.fg_stroke = Stroke::new(1.0, Theme::TEXT);
    v.selection.bg_fill = Theme::ACCENT_DIM;
    v.selection.stroke = Stroke::new(1.0, Theme::ACCENT);
    ctx.style_mut(|s| *s = style.clone());
}

// ------------------------------------------------------------------
// Centering

/// The screen-anchored rect for a centered panel of the requested size,
/// clamped to leave a margin on every side (small windows never overflow).
pub fn centered_panel_rect(screen: Rect, w: f32, h: f32) -> Rect {
    let w = w.min(screen.width() - 24.0).max(120.0);
    let h = h.min(screen.height() - 24.0).max(60.0);
    Rect::from_center_size(screen.center(), Vec2::new(w, h))
}

/// Vertically center the next widget block of the given estimated height
/// inside a `top_down` layout: emit the top spacer, return nothing. The
/// caller keeps `Align::Center` for the horizontal axis.
pub fn center_vertically(ui: &mut Ui, panel_h: f32) {
    let avail = ui.available_height();
    let top = ((avail - panel_h) / 2.0).max(0.0);
    ui.add_space(top);
}

// ------------------------------------------------------------------
// Easing

pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

pub fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    let t = t.clamp(0.0, 1.0);
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 { 2.0 * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 }
}

/// Tracks a 0→1 reveal with delta-time stepping; restart() re-runs it.
#[derive(Clone, Debug)]
pub struct Reveal {
    pub t: f32,
    pub duration: f32,
    pub delay: f32,
}

impl Reveal {
    pub fn new(duration: f32) -> Self {
        Self { t: 0.0, duration, delay: 0.0 }
    }

    pub fn delayed(duration: f32, delay: f32) -> Self {
        Self { t: 0.0, duration, delay }
    }

    pub fn restart(&mut self) {
        self.t = 0.0;
    }

    pub fn step(&mut self, dt: f32) {
        if self.delay > 0.0 {
            self.delay -= dt;
            return;
        }
        self.t = (self.t + dt / self.duration.max(0.0001)).min(1.0);
    }

    pub fn eased(&self) -> f32 {
        ease_out_cubic(self.t)
    }

    pub fn done(&self) -> bool {
        self.t >= 1.0
    }
}

// ------------------------------------------------------------------
// Animated widgets

/// Primary menu button: a heavy iron-and-wood plate — sharp corners, warm
/// panel fill, 1px border that warms on hover, spring press. Returns true
/// when clicked.
pub fn menu_button(ui: &mut Ui, label: &str, reveal: f32, accent: bool) -> bool {
    let size = Vec2::new(300.0, 52.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = response.hovered();
    let pressed = response.is_pointer_button_down_on();
    let hover_amt = ui.ctx().animate_value_with_time(
        egui::Id::new(format!("hb:{}", label)),
        if hovered { 1.0 } else { 0.0 },
        0.15,
    );
    let press_amt = ui.ctx().animate_value_with_time(
        egui::Id::new(format!("pb:{}", label)),
        if pressed { 1.0 } else { 0.0 },
        0.08,
    );
    let reveal = ease_out_cubic(reveal);
    let full = rect * reveal;
    let grow = hover_amt * 3.0 - press_amt * 5.0;
    let r = full.expand(grow);
    // LOREFORGE aesthetic: sharp edges. A rounded rect is somebody else's UI.
    let fill = if accent {
        Color32::from_rgba_premultiplied(0x4a, 0x2c, 0x18, (235.0 * reveal) as u8)
    } else {
        Color32::from_rgba_premultiplied(0x2a, 0x20, 0x18, (235.0 * reveal) as u8)
    };
    ui.painter().rect_filled(r, 0.0, fill);
    let stroke_col = if accent {
        blend_stroke(Theme::ACCENT, Theme::ACCENT_DIM, hover_amt)
    } else {
        blend_stroke(Theme::BORDER, Theme::ACCENT_DIM, hover_amt)
    };
    ui.painter().rect_stroke(r, 0.0, Stroke::new(1.0 + hover_amt, stroke_col), egui::StrokeKind::Middle);
    if hover_amt > 0.01 {
        let bar = Rect::from_min_size(
            Pos2::new(r.left() + 4.0, r.center().y - 16.0 * hover_amt),
            Vec2::new(3.0, 32.0 * hover_amt),
        );
        ui.painter().rect_filled(bar, 0.0, Theme::ACCENT);
    }
    let text_col = Color32::from_rgba_premultiplied(
        Theme::TEXT.r(), Theme::TEXT.g(), Theme::TEXT.b(),
        ((245.0 * reveal) as u8).min(255),
    );
    let offset = (1.0 - reveal) * 18.0;
    ui.painter().text(
        Pos2::new(r.center().x, r.center().y + offset),
        Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(21.0),
        text_col,
    );
    response.clicked()
}

/// Linear blend between two stroke colors.
fn blend_stroke(a: Color32, b: Color32, t: f32) -> Color32 {
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgba_premultiplied(
        mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}

/// The LOREFORGE navigation link: left-aligned parchment text with a thin
/// ember underline that sweeps in on hover (120ms ease-out), the text
/// nudging +4px right. Press pulls it back 2px. `pinned` keeps the
/// underline permanently visible — reserved for action buttons (Create
/// World, Apply), never for plain navigation.
pub fn menu_link(ui: &mut Ui, label: &str, id: &str, reveal: f32, pinned: bool, enabled: bool) -> bool {
    let reveal = ease_out_cubic(reveal);
    let font = egui::FontId::proportional(19.0);
    let painter_at = ui.painter_at(ui.max_rect());
    // measure the text first so the underline matches its width
    let galley = painter_at.layout_no_wrap(label.to_string(), font.clone(), Theme::TEXT);
    let text_w = galley.size().x;
    let size = Vec2::new(text_w + 8.0, 30.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = enabled && response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();
    let hover_amt = ui.ctx().animate_value_with_time(
        egui::Id::new(format!("link-h:{}", id)),
        if hovered || pinned { 1.0 } else { 0.0 },
        0.12,
    );
    let press_amt = ui.ctx().animate_value_with_time(
        egui::Id::new(format!("link-p:{}", id)),
        if pressed { 1.0 } else { 0.0 },
        0.06,
    );
    let base_x = rect.left() + hover_amt * 4.0 - press_amt * 2.0;
    let text_col = if !enabled {
        Theme::TEXT_DISABLED
    } else if pressed {
        Theme::ACCENT
    } else if pinned || hover_amt > 0.6 {
        Theme::TEXT_BRIGHT
    } else {
        Theme::TEXT
    };
    let col = Color32::from_rgba_premultiplied(text_col.r(), text_col.g(), text_col.b(),
        ((255.0 * reveal) as u8).min(255));
    ui.painter().text(
        Pos2::new(base_x, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        col,
    );
    if enabled && (hover_amt > 0.01 || pinned) {
        let w = if pinned { text_w } else { text_w * hover_amt };
        let y = rect.center().y + 13.0;
        let alpha = ((255.0 * reveal) as u8).min(255);
        ui.painter().line_segment(
            [Pos2::new(base_x, y), Pos2::new(base_x + w, y)],
            Stroke::new(1.0, Color32::from_rgba_premultiplied(Theme::ACCENT.r(), Theme::ACCENT.g(), Theme::ACCENT.b(), alpha)),
        );
    }
    enabled && response.clicked()
}

/// Radial vignette: a gradient fade from the world render to a dark edge,
/// painted as concentric translucent frames. The center stays fully
/// visible; corners sink into `#1a1410`. Used by every fullscreen overlay.
pub fn vignette(ui: &mut Ui, max_alpha: u8) {
    let rect = ui.max_rect();
    let painter = ui.painter_at(rect);
    // true radial falloff as a vertex-colored grid: egui interpolates the
    // vertex colors, so alpha ramps smoothly from 0 at the center to
    // max_alpha at the corners — the frame edges sink into BG and the
    // middle of the screen stays fully visible.
    let grid = 12usize;
    let cw = rect.width() / grid as f32;
    let ch = rect.height() / grid as f32;
    let center = rect.center();
    let max_r = ((rect.width().powi(2) + rect.height().powi(2)).sqrt()) * 0.5;
    let alpha_at = |p: egui::Pos2| -> u8 {
        let d = (p - center).length();
        let t = (d / max_r).clamp(0.0, 1.0);
        // smoothstep keeps the center clear, then the edge sinks fast
        let s = t * t * (3.0 - 2.0 * t);
        (max_alpha as f32 * s) as u8
    };
    let mut mesh = egui::Mesh::default();
    for gy in 0..=grid {
        for gx in 0..=grid {
            let p = egui::pos2(rect.left() + gx as f32 * cw, rect.top() + gy as f32 * ch);
            let a = alpha_at(p);
            mesh.colored_vertex(p,
                Color32::from_rgba_unmultiplied(Theme::BG.r(), Theme::BG.g(), Theme::BG.b(), a));
        }
    }
    for gy in 0..grid {
        for gx in 0..grid {
            let v = |gx: usize, gy: usize| (gy * (grid + 1) + gx) as u32;
            let (a, b, c, d) = (v(gx, gy), v(gx + 1, gy), v(gx + 1, gy + 1), v(gx, gy + 1));
            mesh.add_triangle(a, b, c);
            mesh.add_triangle(a, c, d);
        }
    }
    painter.add(egui::Shape::Mesh(std::sync::Arc::new(mesh)));
}

/// A drawn checkmark — the shipped font has no check glyph, and a tofu
/// box is the most AI-looking thing a UI can show. Three line segments.
pub fn paint_check(p: &egui::Painter, center: egui::Pos2, color: Color32, width: f32) {
    let s = 5.0;
    p.line_segment(
        [center + egui::vec2(-s, 0.0), center + egui::vec2(-s * 0.25, s * 0.7)],
        Stroke::new(width, color),
    );
    p.line_segment(
        [center + egui::vec2(-s * 0.25, s * 0.7), center + egui::vec2(s, -s * 0.8)],
        Stroke::new(width, color),
    );
}

/// LOREFORGE text input: deep-background field, 1px warm border that
/// brightens to ember while focused. No rounded corners.
pub fn text_input(ui: &mut Ui, text: &mut String, id: &str, hint: &str, width: f32) -> bool {
    let before = text.clone();
    let response = egui::TextEdit::singleline(text)
        .id(egui::Id::new(id))
        .hint_text(egui::RichText::new(hint).color(Theme::TEXT_DISABLED))
        .desired_width(width)
        .font(egui::TextStyle::Button)
        .show(ui)
        .response;
    let focused = response.has_focus();
    // paint the frame after the fact — egui's default frame is blue-grey
    let rect = response.rect;
    ui.painter_at(ui.max_rect()).rect_stroke(rect, 0.0,
        Stroke::new(if focused { 1.5 } else { 1.0 },
            if focused { Theme::ACCENT } else { Theme::BORDER }), egui::StrokeKind::Middle);
    let bg = Color32::from_rgba_premultiplied(Theme::BG.r(), Theme::BG.g(), Theme::BG.b(), 235);
    ui.painter_at(ui.max_rect()).rect_filled(rect, 0.0, bg);
    // repaint the text above the fill we just laid down
    let text_col = if text.is_empty() && !focused { Theme::TEXT_DISABLED } else { Theme::TEXT };
    let shown: String = if text.is_empty() && !focused { hint.to_string() } else { text.clone() };
    let painter = ui.painter_at(ui.max_rect());
    painter.text(rect.left_center() + egui::vec2(8.0, 0.0), Align2::LEFT_CENTER, shown.clone(),
        egui::FontId::proportional(16.0), text_col);
    // keep the cursor visible when focused: re-run egui's own cursor line
    if focused {
        let galley = painter.layout_no_wrap(shown, egui::FontId::proportional(16.0), text_col);
        let cx = rect.left() + 8.0 + galley.size().x.min(width - 16.0);
        let blink = (ui.input(|i| i.time) * 2.0).fract() < 0.6;
        if blink {
            painter.line_segment([Pos2::new(cx, rect.top() + 6.0), Pos2::new(cx, rect.bottom() - 6.0)],
                Stroke::new(1.0, Theme::TEXT));
        }
    }
    &before != text
}

/// Slide+fade container; draws children with an opacity-adjusted panel.
pub fn slide_panel(ui: &mut Ui, reveal: f32, add_contents: impl FnOnce(&mut Ui)) {
    let e = ease_out_cubic(reveal);
    let frame = egui::Frame::new()
        .fill(Color32::from_rgba_premultiplied(0x2a, 0x20, 0x18, ((242.0 * e) as u8).min(255)))
        .stroke(Stroke::new(1.0, Color32::from_rgba_premultiplied(
            Theme::BORDER.r(), Theme::BORDER.g(), Theme::BORDER.b(), ((255.0 * e) as u8).min(255))))
        .corner_radius(0.0);
    frame.show(ui, |ui| {
        ui.set_opacity(e);
        add_contents(ui);
    });
}

/// Styled toggle switch with animated knob. Returns true when changed.
pub fn toggle(ui: &mut Ui, label: &str, value: &mut bool) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(46.0, 24.0), Sense::click());
        if response.clicked() {
            *value = !*value;
            changed = true;
        }
        let target = if *value { 1.0 } else { 0.0 };
        let amt = ui.ctx().animate_value_with_time(egui::Id::new(format!("tg:{}", label)), target, 0.14);
        let track = if amt > 0.5 { Theme::OK } else { Theme::BORDER };
        ui.painter().rect_filled(rect, 0.0, Theme::BG);
        ui.painter().rect_filled(
            Rect::from_min_size(Pos2::new(rect.left() + 2.0, rect.top() + 2.0), Vec2::new(42.0, 20.0)),
            0.0,
            Color32::from_rgba_premultiplied(track.r(), track.g(), track.b(), ((255.0 * (0.3 + 0.7 * amt)) as u8).min(255)),
        );
        let knob_x = rect.left() + 4.0 + amt * 22.0;
        ui.painter().circle_filled(Pos2::new(knob_x + 8.0, rect.center().y), 9.0, Theme::TEXT);
        ui.label(egui::RichText::new(label).color(Theme::TEXT));
    });
    changed
}

/// A row of flat text segments where exactly one is selected — the
/// world-type / difficulty / game-mode control. Selected: `#4a3f2e`
/// backing, parchment text. Unselected: muted text that brightens on
/// hover. Returns true when the selection changed.
pub fn segment_row(ui: &mut Ui, id: &str, options: &[&str], selected: &mut usize) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for (i, opt) in options.iter().enumerate() {
            let on = i == *selected;
            let (rect, resp) = ui.allocate_exact_size(
                egui::vec2(24.0 + opt.len() as f32 * 8.2, 26.0), Sense::click());
            let hover_amt = ui.ctx().animate_value_with_time(
                egui::Id::new(format!("seg:{}:{}", id, i)), if resp.hovered() && !on { 1.0 } else { 0.0 }, 0.12);
            if on {
                ui.painter().rect_filled(rect, 0.0, Theme::BORDER);
            }
            let text = if on {
                Theme::TEXT
            } else {
                blend_stroke(Theme::TEXT_DIM, Theme::TEXT, hover_amt)
            };
            ui.painter().text(rect.center(), Align2::CENTER_CENTER, *opt,
                egui::FontId::proportional(15.0), text);
            if resp.clicked() && !on {
                *selected = i;
                changed = true;
            }
        }
    });
    changed
}

/// Slider with value readout in accent monospace.
pub fn setting_slider(ui: &mut Ui, label: &str, value: &mut f32, range: (f32, f32), fmt: &dyn Fn(f32) -> String) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(Theme::TEXT).size(15.0));
        ui.add(egui::Slider::new(value, range.0..=range.1).show_value(false));
        ui.label(egui::RichText::new(fmt(*value)).color(Theme::ACCENT).monospace().size(14.0));
    });
}

/// Section header with underline sweep on reveal.
pub fn section_header(ui: &mut Ui, title: &str, reveal: f32) {
    let e = ease_out_cubic(reveal);
    let pos = ui.cursor().min;
    ui.add_space(28.0);
    ui.painter().text(pos, Align2::LEFT_TOP, title, egui::FontId::proportional(18.0),
        Color32::from_rgba_premultiplied(Theme::ACCENT.r(), Theme::ACCENT.g(), Theme::ACCENT.b(), ((235.0 * e) as u8).min(255)));
    ui.painter().line_segment(
        [pos + Vec2::new(0.0, 24.0), pos + Vec2::new(120.0 * e, 24.0)],
        Stroke::new(2.0, Theme::ACCENT_DIM),
    );
}

// ------------------------------------------------------------------
// Item tooltips (real icon + stats + era badge)

/// One stat line inside a tooltip: label left, value right.
fn tooltip_line(ui: &mut Ui, label: &str, value: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.0).color(Theme::TEXT_DIM));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(12.0).color(color));
        });
    });
}

/// The tooltip body for an item stack. Draws inside whatever popup/hover
/// container the caller set up (on_hover_ui, popup below...).
pub fn item_tooltip_body(ui: &mut Ui, stack: &ItemStack, icons: &crate::icons::ItemIcons) {
    egui::Frame::new()
        .fill(Color32::from_rgba_premultiplied(0x1a, 0x14, 0x10, 248))
        .corner_radius(0.0)
        .stroke(egui::Stroke::new(1.0, Theme::BORDER))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(190.0);
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(28.0, 28.0), Sense::hover());
                if !icons.paint(ui, rect, &stack.item_id) {
                    ui.painter().rect_filled(rect, 4.0, crate::icons::fallback_color(&stack.item_id));
                }
                ui.vertical(|ui| {
                    let name = item_def(&stack.item_id).map(|d| d.name).unwrap_or("Unknown");
                    ui.label(RichText::new(name).size(15.0).color(Theme::ACCENT).strong());
                    if stack.count > 1 {
                        ui.label(RichText::new(format!("x{}", stack.count)).size(11.0).color(Theme::TEXT_DIM));
                    }
                });
            });
            ui.add_space(3.0);
            let sep = ui.style().visuals.widgets.noninteractive.bg_stroke;
            ui.painter().line_segment(
                [ui.cursor().min, ui.cursor().min + Vec2::new(ui.available_width(), 0.0)],
                sep,
            );
            ui.add_space(4.0);
            // kind-specific stats
            if let Some(def) = item_def(&stack.item_id) {
                match def.kind {
                    ItemKind::Tool(kind, tier) => {
                        let tier_name = match tier { 0 => "Wood", 1 => "Stone", _ => "Iron" };
                        let kind_name = match kind {
                            ToolKind::Pickaxe => "Pickaxe",
                            ToolKind::Axe => "Axe",
                            ToolKind::Shovel => "Shovel",
                            ToolKind::Sword => "Sword",
                            ToolKind::Bow => "Bow",
                        };
                        tooltip_line(ui, kind_name, tier_name, Theme::TEXT);
                        tooltip_line(ui, "Damage", &format!("{:.1}", tool_damage(kind, tier)), Theme::BAD);
                        tooltip_line(ui, "Mining speed", &format!("{:.0}x", lf_game::items::tier_speed(tier)), Theme::OK);
                        if let ToolKind::Bow = kind {
                            tooltip_line(ui, "Use", "hold RMB to charge", Theme::TEXT_DIM);
                        }
                    }
                    ItemKind::Food(h) => {
                        tooltip_line(ui, "Food", &format!("+{} hunger", h), Theme::HUNGER);
                    }
                    ItemKind::Armor(points) => {
                        tooltip_line(ui, "Armor", &format!("-{} damage taken", points), Theme::OK);
                        tooltip_line(ui, "Slot", "equip via inventory", Theme::TEXT_DIM);
                    }
                    ItemKind::Block(b) => {
                        tooltip_line(ui, "Block", lf_voxel::registry::block::name(b), Theme::TEXT);
                    }
                    ItemKind::Material => {
                        tooltip_line(ui, "Material", "crafting input", Theme::TEXT_DIM);
                    }
                }
                if def.max_stack > 1 {
                    tooltip_line(ui, "Stacks to", &format!("{}", def.max_stack), Theme::TEXT_DIM);
                }
            }
            // cross-item facts
            let fuel = lf_game::smelting::fuel_seconds(&stack.item_id);
            if fuel > 0.0 {
                tooltip_line(ui, "Fuel", &format!("{:.0}s", fuel), Theme::HUNGER);
            }
            if let Some(out) = lf_game::smelting::smelt_result(&stack.item_id) {
                let out_name = item_def(out).map(|d| d.name).unwrap_or(out);
                tooltip_line(ui, "Smelts into", out_name, Theme::XP);
            }
            if let Some((out, _)) = lf_game::machines::crush_result(&stack.item_id) {
                let out_name = item_def(out).map(|d| d.name).unwrap_or(out);
                tooltip_line(ui, "Crushes into", out_name, Theme::XP);
            }
            let era = lf_game::research::Era::required_for(&stack.item_id);
            if era > lf_game::research::Era::Primitive {
                tooltip_line(ui, "Requires", era.name(), Theme::BAD);
            }
        });
}

/// Attach the standard item tooltip to a slot response.
pub fn hover_item_tooltip(response: &egui::Response, stack: &ItemStack, icons: &crate::icons::ItemIcons) {
    let stack = stack.clone();
    response.clone().on_hover_ui(|ui| {
        item_tooltip_body(ui, &stack, icons);
    });
}

// ------------------------------------------------------------------
// Painted HUD glyphs (no font dependency)

/// Points of the crosshair reticle's progress arc: clockwise from 12
/// o'clock, sweeping `progress * TAU`. Pure geometry so it can be tested
/// without a painter; the vistest HUD preview mirrors this math (lf_vistest
/// does not depend on lf_client) — keep the two in sync.
pub fn reticle_points(center: Pos2, radius: f32, progress: f32) -> Vec<Pos2> {
    let t = progress.clamp(0.0, 1.0);
    if t <= 0.001 {
        return Vec::new();
    }
    let steps = ((t * 64.0).ceil() as usize).max(2);
    (0..=steps)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + (i as f32 / steps as f32) * t * std::f32::consts::TAU;
            center + Vec2::new(a.cos() * radius, a.sin() * radius)
        })
        .collect()
}

/// Crosshair-centered radial progress ring (mining / bow charge) — the
/// Section-2 replacement for the old bottom-of-screen progress bar. A faint
/// full ring tracks the sweep; the arc fills clockwise with progress.
pub fn paint_mining_reticle(p: &egui::Painter, center: Pos2, progress: f32, color: Color32) {
    if progress <= 0.001 {
        return;
    }
    const RADIUS: f32 = 15.0;
    p.circle_stroke(center, RADIUS, Stroke::new(1.5, Color32::from_white_alpha(48)));
    let points = reticle_points(center, RADIUS, progress);
    if points.len() >= 2 {
        p.add(egui::Shape::Path(egui::epaint::PathShape {
            points,
            closed: false,
            fill: Color32::TRANSPARENT,
            stroke: egui::epaint::PathStroke::new(3.0, color),
        }));
    }
}

pub fn paint_hearts(ui: &mut Ui, health: f32, max: f32) {
    let full = (health / 2.0).floor().max(0.0) as i32;
    let half = health - full as f32 * 2.0 >= 1.0;
    let total = (max / 2.0).ceil().max(1.0) as i32;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total as f32 * 18.0, 16.0), Sense::hover());
    for i in 0..total {
        let c = Pos2::new(rect.left() + 9.0 + i as f32 * 18.0, rect.center().y);
        let (base, alpha) = if i < full {
            (Theme::HEART, 1.0)
        } else if i == full && half {
            (Theme::HEART, 0.55)
        } else {
            (Color32::from_rgb(70, 40, 44), 1.0)
        };
        let fill = Color32::from_rgba_premultiplied(base.r(), base.g(), base.b(), ((255.0 * alpha) as u8).min(255));
        ui.painter().circle_filled(Pos2::new(c.x - 3.5, c.y - 2.5), 3.6, fill);
        ui.painter().circle_filled(Pos2::new(c.x + 3.5, c.y - 2.5), 3.6, fill);
        ui.painter().add(egui::Shape::convex_polygon(
            vec![Pos2::new(c.x - 6.8, c.y - 1.0), Pos2::new(c.x + 6.8, c.y - 1.0), Pos2::new(c.x, c.y + 6.5)],
            fill,
            Stroke::NONE,
        ));
    }
}

pub fn paint_hunger(ui: &mut Ui, hunger: f32, max: f32) {
    let full = (hunger / 2.0).floor().max(0.0) as i32;
    let total = (max / 2.0).ceil().max(1.0) as i32;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total as f32 * 16.0, 14.0), Sense::hover());
    for i in 0..total {
        let c = Pos2::new(rect.right() - 7.0 - i as f32 * 16.0, rect.center().y);
        let fill = if i < full { Theme::HUNGER } else { Color32::from_rgb(70, 56, 32) };
        ui.painter().circle_filled(c, 5.0, fill);
        ui.painter().circle_stroke(c, 5.0, Stroke::new(1.0, Color32::from_rgb(30, 24, 16)));
        ui.painter().rect_filled(Rect::from_center_size(Pos2::new(c.x + 5.0, c.y + 4.0), Vec2::new(5.0, 3.0)), 1.0, fill);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The menus-centering contract: every dialog panel lands on the
    /// screen's center, and a too-big panel clamps instead of overflowing.
    #[test]
    fn centered_panel_rect_is_symmetric_and_clamped() {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0));
        // new world (560x470) and multiplayer (560x420) sizes
        for (w, h) in [(560.0, 470.0), (560.0, 420.0), (460.0, 320.0), (360.0, 300.0)] {
            let r = centered_panel_rect(screen, w, h);
            assert!((r.center().x - screen.center().x).abs() < 0.5, "{}x{} not h-centered", w, h);
            assert!((r.center().y - screen.center().y).abs() < 0.5, "{}x{} not v-centered", w, h);
            assert_eq!(r.width(), w, "{}x{} should fit unclamped", w, h);
        }
        // clamping: an oversized panel keeps the margin on every side
        let r = centered_panel_rect(screen, 4000.0, 2000.0);
        assert_eq!(r.width(), screen.width() - 24.0);
        assert_eq!(r.height(), screen.height() - 24.0);
        assert!((r.center().x - screen.center().x).abs() < 0.5);
        // a tiny screen still yields a usable rect (never inverted)
        let tiny = Rect::from_min_size(Pos2::ZERO, Vec2::new(320.0, 200.0));
        let r = centered_panel_rect(tiny, 560.0, 470.0);
        assert!(r.width() > 100.0 && r.height() > 50.0);
        assert!((r.center().x - tiny.center().x).abs() < 0.5);
    }

    /// The vertical-center spacer must be non-negative and split the
    /// leftover space evenly (a panel of the available height adds none).
    #[test]
    fn center_vertically_splits_leftover_space() {
        let ctx = egui::Context::default();
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            ..Default::default()
        };
        ctx.begin_pass(raw);
        let panel = egui::CentralPanel::default().show(&ctx, |ui| {
            center_vertically(ui, 300.0);
            ui.cursor().top()
        });
        // 600px screen, 300px panel -> ~150px of top space (panel chrome
        // takes a little; the spacer must never be negative)
        let top = panel.inner;
        assert!((150.0 - top).abs() < 6.0, "expected ~150 top spacer, got {}", top);
        assert!(top >= 0.0);
    }

    #[test]
    fn easing_curves() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert_eq!(ease_out_cubic(1.0), 1.0);
        assert!(ease_out_cubic(0.5) > 0.7, "out-cubic starts fast");
        assert_eq!(ease_in_out(0.0), 0.0);
        assert_eq!(ease_in_out(1.0), 1.0);
        assert!((ease_in_out(0.5) - 0.5).abs() < 0.01);
        assert!(ease_out_back(0.7) > 0.95, "back eases fast then settles");
        assert_eq!(ease_out_back(1.0), 1.0);
    }

    #[test]
    fn reveal_steps_and_clamps() {
        let mut r = Reveal::new(0.5);
        r.step(0.25);
        assert!((r.t - 0.5).abs() < 0.01);
        r.step(10.0);
        assert_eq!(r.t, 1.0);
        assert!(r.done());
        let mut d = Reveal::delayed(0.1, 1.0);
        d.step(0.5);
        assert_eq!(d.t, 0.0, "delay holds");
        d.step(0.6); // delay consumed
        assert_eq!(d.t, 0.0, "still in the last of the delay");
        d.step(0.1);
        assert!(d.t > 0.0, "now progressing");
    }

    /// Section 2: the crosshair reticle arc starts at 12 o'clock, sweeps
    /// clockwise, and its angular span tracks progress exactly.
    #[test]
    fn reticle_arc_spans_progress_from_top() {
        let center = Pos2::new(0.0, 0.0);
        assert!(reticle_points(center, 15.0, 0.0).is_empty(), "no arc at zero progress");
        let quarter = reticle_points(center, 10.0, 0.25);
        assert!(!quarter.is_empty());
        let first = quarter[0];
        assert!((first.x - 0.0).abs() < 1e-4 && (first.y + 10.0).abs() < 1e-4,
            "arc starts at 12 o'clock (top of the ring), got {:?}", first);
        let last = *quarter.last().unwrap();
        // a quarter sweep clockwise ends at 3 o'clock
        assert!((last.x - 10.0).abs() < 0.5 && last.y.abs() < 0.5,
            "quarter progress ends at 3 o'clock, got {:?}", last);
        let full = reticle_points(center, 10.0, 1.0);
        let end = *full.last().unwrap();
        // full sweep closes the ring back at the top (TAU wraps to -PI/2)
        assert!((end.x).abs() < 0.5 && (end.y + 10.0).abs() < 0.5,
            "full progress closes the ring at 12 o'clock, got {:?}", end);
        assert!(full.len() > quarter.len(), "more progress = more arc points");
    }
}

/// Step 12 (HUD legibility): text with a hard shadow so labels stay
/// readable over snow, sand and bright sky alike.
pub fn text_shadowed(
    p: &egui::Painter,
    pos: egui::Pos2,
    anchor: egui::Align2,
    text: String,
    font: egui::FontId,
    color: egui::Color32,
) {
    p.text(pos + egui::vec2(1.0, 1.0), anchor, text.clone(), font.clone(),
        egui::Color32::from_black_alpha(180));
    p.text(pos, anchor, text, font, color);
}
