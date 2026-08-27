//! LOREFORGE UI kit: theme, easing, and animated widgets on top of egui.
//! The design system every menu/screen shares — buttons with hover glow and
//! spring easing, fade/slide-in panels, toggles, sliders, and painted
//! heart/hunger glyphs (no unicode font boxes).

use egui::{Align2, Color32, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

use lf_game::items::{item_def, tool_damage, ItemKind, ToolKind};
use lf_game::survival::ItemStack;

// ------------------------------------------------------------------
// Theme

pub struct Theme;

impl Theme {
    pub const BG: Color32 = Color32::from_rgb(16, 18, 24);
    pub const PANEL: Color32 = Color32::from_rgba_premultiplied(18, 22, 30, 235);
    pub const ACCENT: Color32 = Color32::from_rgb(240, 200, 120);      // warm gold
    pub const ACCENT_DIM: Color32 = Color32::from_rgb(120, 96, 52);
    pub const TEXT: Color32 = Color32::from_rgb(235, 238, 242);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(150, 156, 165);
    pub const OK: Color32 = Color32::from_rgb(120, 210, 130);
    pub const BAD: Color32 = Color32::from_rgb(230, 120, 110);
    pub const HEART: Color32 = Color32::from_rgb(225, 60, 70);
    pub const HUNGER: Color32 = Color32::from_rgb(210, 150, 50);
    pub const XP: Color32 = Color32::from_rgb(110, 220, 255);

    pub fn title_glow(t: f32) -> Color32 {
        let pulse = 0.5 + 0.5 * (t * 1.4).sin();
        Color32::from_rgba_premultiplied(240, 210, 140, (200.0 + 40.0 * pulse) as u8)
    }
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

/// Primary menu button: rounded frame, hover glow + slight grow, spring
/// press. Returns true when clicked.
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
    let rounding = 10.0;
    let fill = if accent {
        Color32::from_rgba_premultiplied(60 + (40.0 * hover_amt) as u8, 48, 22, (235.0 * reveal) as u8)
    } else {
        Color32::from_rgba_premultiplied(28, 33, 44, (225.0 * reveal) as u8)
    };
    ui.painter().rect_filled(r, rounding, fill);
    let stroke_col = if accent {
        Color32::from_rgba_premultiplied(240, 200, 120, ((200.0 + 55.0 * hover_amt) as u8).min(255))
    } else {
        Color32::from_rgba_premultiplied(90, 98, 112, ((120.0 + 120.0 * hover_amt) as u8).min(255))
    };
    ui.painter().rect_stroke(r, rounding, Stroke::new(1.5 + hover_amt * 1.5, stroke_col), egui::StrokeKind::Middle);
    if hover_amt > 0.01 {
        let bar = Rect::from_min_size(
            Pos2::new(r.left() + 4.0, r.center().y - 16.0 * hover_amt),
            Vec2::new(3.0, 32.0 * hover_amt),
        );
        ui.painter().rect_filled(bar, 2.0, Theme::ACCENT);
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

/// Slide+fade container; draws children with an opacity-adjusted panel.
pub fn slide_panel(ui: &mut Ui, reveal: f32, add_contents: impl FnOnce(&mut Ui)) {
    let e = ease_out_cubic(reveal);
    let frame = egui::Frame::new()
        .fill(Color32::from_rgba_premultiplied(18, 22, 30, ((235.0 * e) as u8).min(255)))
        .corner_radius(14.0);
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
        let track = if amt > 0.5 { Theme::OK } else { Color32::from_rgb(60, 64, 72) };
        ui.painter().rect_filled(rect, 12.0, Color32::from_rgb(40, 44, 52));
        ui.painter().rect_filled(
            Rect::from_min_size(Pos2::new(rect.left() + 2.0, rect.top() + 2.0), Vec2::new(42.0, 20.0)),
            10.0,
            Color32::from_rgba_premultiplied(track.r(), track.g(), track.b(), ((255.0 * (0.3 + 0.7 * amt)) as u8).min(255)),
        );
        let knob_x = rect.left() + 4.0 + amt * 22.0;
        ui.painter().circle_filled(Pos2::new(knob_x + 8.0, rect.center().y), 9.0, Theme::TEXT);
        ui.label(egui::RichText::new(label).color(Theme::TEXT));
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
        .fill(Color32::from_rgba_premultiplied(14, 17, 24, 248))
        .corner_radius(8.0)
        .stroke(egui::Stroke::new(1.0, Theme::ACCENT_DIM))
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
