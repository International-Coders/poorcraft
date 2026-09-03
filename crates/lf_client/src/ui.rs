//! egui integration: platform plumbing plus HUD, inventory/crafting screens,
//! the recipe book, container screens and the death screen. All immediate-
//! mode drawing against GameState, styled by the ui_kit design system and
//! the real pixel-art icons from `crate::icons`.

use egui_wgpu::Renderer;
use egui::Color32;
use image;
use egui_winit::State as EguiWinitState;

use crate::ui_kit::{self as kit, Theme};
use crate::workbench;
use crate::{BlockEntity, GameState, RtMode, UiOpen};
use lf_game::items::{item_def, ItemKind};
use lf_game::research::Era;
use lf_game::survival::ItemStack;
use crate::QuestEvent;
use lf_npc::trade_offers;
use lf_voxel::registry;

/// The single caption line above the hotbar: the just-picked item's
/// name while its fade window is live (immediate scroll/switch
/// feedback), otherwise the name of the block under the crosshair —
/// "what am I pointing at?" (loop 347). Returns (text, is_item_switch).
pub fn hotbar_caption(
    held_name: Option<&str>,
    pick_time: f32,
    target_name: Option<&str>,
) -> Option<(String, bool)> {
    if pick_time > 0.08 {
        if let Some(n) = held_name {
            return Some((n.to_string(), true));
        }
    }
    target_name.map(|n| (n.to_string(), false))
}

pub struct EguiPlatform {
    pub ctx: egui::Context,
    state: EguiWinitState,
    pub renderer: Renderer,
}

impl EguiPlatform {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        window: &winit::window::Window,
    ) -> Self {
        let ctx = egui::Context::default();
        let state = EguiWinitState::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let renderer = Renderer::new(device, format, None, 1, true);
        Self { ctx, state, renderer }
    }

    /// Feed a window event; returns true when egui consumed it.
    pub fn on_event(&mut self, window: &winit::window::Window, event: &winit::event::WindowEvent) -> bool {
        let response = self.state.on_window_event(window, event);
        response.consumed
    }

    pub fn begin_frame(&mut self, window: &winit::window::Window) {
        let input = self.state.take_egui_input(window);
        self.ctx.begin_pass(input);
    }

    /// Finish the frame and return the paint data for the render pass.
    pub fn end_frame(
        &mut self,
        window: &winit::window::Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) -> (Vec<egui::ClippedPrimitive>, egui_wgpu::ScreenDescriptor) {
        let full_output = self.ctx.end_pass();
        self.state.handle_platform_output(window, full_output.platform_output);
        let paint_jobs = self.ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [window.inner_size().width, window.inner_size().height],
            pixels_per_point: full_output.pixels_per_point,
        };
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer.update_texture(device, queue, *id, image_delta);
        }
        self.renderer.update_buffers(device, queue, encoder, &paint_jobs, &screen_descriptor);
        (paint_jobs, screen_descriptor)
    }

    pub fn paint(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        paint_jobs: Vec<egui::ClippedPrimitive>,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        // egui-wgpu wants RenderPass<'static>; the pass is dropped before
        // this function returns, so the borrow never escapes the encoder.
        let mut pass: wgpu::RenderPass<'static> = unsafe { std::mem::transmute(pass) };
        self.renderer.render(&mut pass, &paint_jobs, screen_descriptor);
        drop(pass);
    }
}

// ------------------------------------------------------------------
// Slots: shared drawing + pick/place/split/quick-move semantics

const SLOT_SIZE: f32 = 44.0;

/// Flat-color fallbacks for ids without pixel art (modded/unknown items).
fn item_color(stack: &ItemStack) -> egui::Color32 {
    match item_def(&stack.item_id).map(|d| d.kind) {
        Some(ItemKind::Block(b)) => block_color(b),
        Some(ItemKind::Food(_)) => egui::Color32::from_rgb(200, 60, 60),
        Some(ItemKind::Tool(_, _)) => egui::Color32::from_rgb(150, 130, 90),
        _ => egui::Color32::from_gray(160),
    }
}

fn block_color(block_id: u32) -> egui::Color32 {
    use lf_voxel::registry::block;
    match block_id {
        block::GRASS => egui::Color32::from_rgb(90, 160, 60),
        block::DIRT => egui::Color32::from_rgb(134, 96, 67),
        block::STONE => egui::Color32::from_gray(130),
        block::SAND => egui::Color32::from_rgb(219, 207, 163),
        block::MYCELIUM => egui::Color32::from_rgb(140, 130, 160),
        block::SNOW => egui::Color32::from_rgb(240, 246, 246),
        block::LOG => egui::Color32::from_rgb(102, 81, 50),
        block::LEAVES => egui::Color32::from_rgb(60, 120, 40),
        block::TORCH => egui::Color32::from_rgb(255, 200, 100),
        block::CRAFTING_TABLE => egui::Color32::from_rgb(160, 120, 70),
        _ => egui::Color32::from_gray(120),
    }
}

/// Draw one item glyph (icon texture, flat-color fallback) into a rect.
fn paint_item(ui: &mut egui::Ui, rect: egui::Rect, stack: &ItemStack, icons: &crate::icons::ItemIcons) {
    let inner = rect.shrink(6.0);
    if !icons.paint(ui, inner, &stack.item_id) {
        ui.painter().rect_filled(inner, 3.0, item_color(stack));
    }
}

fn paint_count(ui: &mut egui::Ui, rect: egui::Rect, count: u8) {
    if count <= 1 {
        return;
    }
    let pos = rect.right_bottom() + egui::vec2(-5.0, -5.0);
    // drop shadow then text, so counts stay readable over bright icons
    ui.painter().text(pos + egui::vec2(1.0, 1.0), egui::Align2::RIGHT_BOTTOM, format!("{}", count),
        egui::FontId::proportional(13.0), egui::Color32::from_black_alpha(200));
    ui.painter().text(pos, egui::Align2::RIGHT_BOTTOM, format!("{}", count),
        egui::FontId::proportional(13.0), Theme::TEXT);
}

struct SlotOutcome {
    hovered: Option<ItemStack>,
    /// Set on shift-click: the whole stack was lifted and the caller must
    /// route it into the "other" container.
    quick_moved: Option<ItemStack>,
}

/// One inventory slot: icon + count + hover glow + tooltip, with full
/// pick/place/merge/split semantics (right-click splits/places one).
fn slot_button(
    ui: &mut egui::Ui,
    stack: &mut Option<ItemStack>,
    cursor: &mut Option<ItemStack>,
    selected: bool,
    icons: &crate::icons::ItemIcons,
) -> SlotOutcome {
    let mut outcome = SlotOutcome { hovered: None, quick_moved: None };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(SLOT_SIZE, SLOT_SIZE), egui::Sense::click());
    let hovered = response.hovered();
    let hover_amt = ui.ctx().animate_value_with_time(
        egui::Id::new(("slot", rect.min.x as i32, rect.min.y as i32)),
        if hovered { 1.0 } else { 0.0 },
        0.09,
    );
    // recessed well
    ui.painter().rect_filled(rect, 5.0, egui::Color32::from_black_alpha(170));
    ui.painter().rect_filled(rect.shrink(1.5), 4.0, egui::Color32::from_rgba_premultiplied(30, 35, 46, 200));
    if let Some(s) = stack {
        paint_item(ui, rect, s, icons);
        paint_count(ui, rect, s.count);
        if hovered {
            outcome.hovered = Some(s.clone());
        }
    }
    // selected: pulsing gold frame; hover: white lift
    if selected {
        let pulse = 0.5 + 0.5 * (ui.input(|i| i.time) as f32 * 3.0).sin();
        let a = (170.0 + 60.0 * pulse) as u8;
        ui.painter().rect_stroke(rect, 5.0,
            egui::Stroke::new(2.5, egui::Color32::from_rgba_premultiplied(Theme::ACCENT.r(), Theme::ACCENT.g(), Theme::ACCENT.b(), a)),
            egui::StrokeKind::Middle);
    } else if hover_amt > 0.01 {
        ui.painter().rect_stroke(rect, 5.0,
            egui::Stroke::new(1.0 + hover_amt, egui::Color32::from_white_alpha((150.0 * hover_amt) as u8)),
            egui::StrokeKind::Middle);
    } else {
        ui.painter().rect_stroke(rect, 5.0, egui::Stroke::new(1.0, egui::Color32::from_gray(80)), egui::StrokeKind::Middle);
    }
    if let Some(s) = stack {
        kit::hover_item_tooltip(&response, s, icons);
    }
    let shift = ui.input(|i| i.modifiers.shift);
    if response.clicked_by(egui::PointerButton::Primary) {
        if shift && stack.is_some() && cursor.is_none() {
            outcome.quick_moved = stack.take();
        } else {
            exchange(cursor, stack, false);
        }
    } else if response.clicked_by(egui::PointerButton::Secondary) {
        exchange(cursor, stack, true);
    }
    outcome
}

/// Move stacks between the cursor and a slot. `right_click` splits/places one.
fn exchange(cursor: &mut Option<ItemStack>, slot: &mut Option<ItemStack>, right_click: bool) {
    let cap = slot.as_ref().or(cursor.as_ref())
        .and_then(|s| item_def(&s.item_id))
        .map(|d| d.max_stack)
        .unwrap_or(64);
    match (cursor.take(), slot.take()) {
        (None, None) => {}
        (Some(c), None) => {
            if right_click {
                *slot = Some(ItemStack { item_id: c.item_id.clone(), count: 1 });
                if c.count > 1 {
                    *cursor = Some(ItemStack { count: c.count - 1, ..c });
                }
            } else {
                *slot = Some(c);
            }
        }
        (None, Some(s)) => {
            if right_click {
                let half = (s.count + 1) / 2;
                *cursor = Some(ItemStack { count: half, ..s.clone() });
                if s.count > half {
                    *slot = Some(ItemStack { count: s.count - half, ..s });
                }
            } else {
                *cursor = Some(s);
            }
        }
        (Some(c), Some(s)) => {
            if c.item_id == s.item_id {
                let room = cap.saturating_sub(s.count);
                let move_n = if right_click { room.min(1) } else { room.min(c.count) };
                let mut s = s;
                s.count += move_n;
                *slot = Some(s);
                if c.count > move_n {
                    *cursor = Some(ItemStack { count: c.count - move_n, ..c });
                }
            } else {
                *slot = Some(c);
                *cursor = Some(s);
            }
        }
    }
}

/// Merge a stack into a slot range (quick-move target); consumes what fits.
fn quick_insert(slots: &mut [Option<ItemStack>], stack: &mut ItemStack) {
    let cap = item_def(&stack.item_id).map(|d| d.max_stack).unwrap_or(64);
    for slot in slots.iter_mut() {
        if stack.count == 0 {
            break;
        }
        if let Some(s) = slot {
            if s.item_id == stack.item_id && s.count < cap {
                let add = (cap - s.count).min(stack.count);
                s.count += add;
                stack.count -= add;
            }
        }
    }
    for slot in slots.iter_mut() {
        if stack.count == 0 {
            break;
        }
        if slot.is_none() {
            *slot = Some(ItemStack { item_id: stack.item_id.clone(), count: stack.count.min(cap) });
            stack.count = stack.count.saturating_sub(cap);
        }
    }
}

/// Pull exactly one item of `id` out of a slot range (auto-fill helper).
fn take_one(slots: &mut [Option<ItemStack>], id: &str) -> Option<ItemStack> {
    for slot in slots.iter_mut() {
        if let Some(s) = slot {
            if s.item_id == id && s.count > 0 {
                s.count -= 1;
                let out = Some(ItemStack { item_id: id.to_string(), count: 1 });
                if s.count == 0 {
                    *slot = None;
                }
                return out;
            }
        }
    }
    None
}

// ------------------------------------------------------------------
// Recipe catalog (crafting + smelting + alloying + crushing in one list)

#[derive(Clone, Copy, PartialEq)]
enum Station {
    Craft,
    Smelt,
    Alloy,
    Crush,
}

impl Station {
    fn label(self) -> &'static str {
        match self {
            Station::Craft => "Crafting Table",
            Station::Smelt => "Furnace",
            Station::Alloy => "Assembler",
            Station::Crush => "Crusher",
        }
    }
}

struct CatalogEntry {
    station: Station,
    output: String,
    output_count: u8,
    /// Aggregated (item, count) pairs.
    ingredients: Vec<(String, u8)>,
    /// Shaped pattern (crafting recipes only) for auto-fill + preview.
    pattern: Option<Vec<Vec<Option<&'static str>>>>,
    grid_size: usize,
}

fn build_catalog() -> Vec<CatalogEntry> {
    let mut out = Vec::new();
    for r in lf_game::crafting::all_recipes() {
        let mut counts: Vec<(String, u8)> = Vec::new();
        for row in &r.pattern {
            for cell in row.iter().flatten() {
                match counts.iter_mut().find(|(id, _)| id == cell) {
                    Some(e) => e.1 += 1,
                    None => counts.push((cell.to_string(), 1)),
                }
            }
        }
        out.push(CatalogEntry {
            station: Station::Craft,
            output: r.output.clone(),
            output_count: r.output_count,
            ingredients: counts,
            pattern: Some(r.pattern.clone()),
            grid_size: r.grid_size(),
        });
    }
    for (input, output) in lf_game::smelting::smelt_entries() {
        out.push(CatalogEntry {
            station: Station::Smelt,
            output: output.to_string(),
            output_count: 1,
            ingredients: vec![(input, 1)],
            pattern: None,
            grid_size: 0,
        });
    }
    for (a, an, b, bn, out_id, on) in lf_game::machines::alloy_recipes() {
        out.push(CatalogEntry {
            station: Station::Alloy,
            output: out_id.to_string(),
            output_count: *on,
            ingredients: vec![(a.to_string(), *an), (b.to_string(), *bn)],
            pattern: None,
            grid_size: 0,
        });
    }
    for (input, output, n) in lf_game::machines::crush_entries() {
        out.push(CatalogEntry {
            station: Station::Crush,
            output: output.to_string(),
            output_count: *n,
            ingredients: vec![(input.to_string(), 1)],
            pattern: None,
            grid_size: 0,
        });
    }
    out
}

/// N03: the workbench zone layout — pure rect math shared with the vistest
/// proofs, so the proof rectangles are the in-game rectangles. Normal
/// windows get the three-pane layout (sidebar / list / detail over the
/// inventory strip); windows under 700px wide get the two-pane drill-down
/// (category chip row, then list OR detail, over a one-row strip).
pub struct WbLayout {
    pub header: egui::Rect,
    pub sidebar: egui::Rect,
    pub list: egui::Rect,
    pub detail: egui::Rect,
    pub strip: egui::Rect,
    pub compact: bool,
}

pub fn workbench_layout(screen: egui::Rect) -> WbLayout {
    let compact = screen.width() < 700.0;
    let pad = 16.0;
    let header = egui::Rect::from_min_max(
        egui::pos2(screen.left() + pad, screen.top() + 10.0),
        egui::pos2(screen.right() - pad, screen.top() + 40.0));
    if compact {
        // drill-down: chips row + one pane + a 1-row hotbar strip
        let chips = egui::Rect::from_min_max(
            egui::pos2(header.left(), header.bottom() + 6.0),
            egui::pos2(header.right(), header.bottom() + 34.0));
        let strip_h = SLOT_SIZE + 44.0;
        let strip = egui::Rect::from_min_max(
            egui::pos2(header.left(), screen.bottom() - pad - strip_h),
            egui::pos2(header.right(), screen.bottom() - pad));
        let pane = egui::Rect::from_min_max(
            egui::pos2(chips.left(), chips.bottom() + 8.0),
            egui::pos2(chips.right(), strip.top() - 8.0));
        WbLayout { header, sidebar: chips, list: pane, detail: pane, strip, compact }
    } else {
        let strip_h = 4.0 * (SLOT_SIZE + 8.0) + 32.0;
        let strip = egui::Rect::from_min_max(
            egui::pos2(header.left(), screen.bottom() - pad - strip_h),
            egui::pos2(header.right(), screen.bottom() - pad));
        let body_top = header.bottom() + 8.0;
        let body_bottom = strip.top() - 8.0;
        let sidebar_w = (screen.width() * 0.15).clamp(130.0, 190.0);
        let list_w = (screen.width() * 0.34).clamp(240.0, 420.0);
        let sidebar = egui::Rect::from_min_max(
            egui::pos2(header.left(), body_top),
            egui::pos2(header.left() + sidebar_w, body_bottom));
        let list = egui::Rect::from_min_max(
            egui::pos2(sidebar.right() + 8.0, body_top),
            egui::pos2(sidebar.right() + 8.0 + list_w, body_bottom));
        let detail = egui::Rect::from_min_max(
            egui::pos2(list.right() + 8.0, body_top),
            egui::pos2(header.right(), body_bottom));
        WbLayout { header, sidebar, list, detail, strip, compact }
    }
}

/// Paint one framed, strongly-opaque zone panel (the modal surface the
/// world must not bleed through). Returns the inner rect.
pub fn paint_wb_panel(p: &egui::Painter, rect: egui::Rect) -> egui::Rect {
    p.rect_filled(rect, 10.0, egui::Color32::from_rgba_unmultiplied(
        Theme::PANEL.r(), Theme::PANEL.g(), Theme::PANEL.b(), 252));
    p.rect_stroke(rect, 10.0, egui::Stroke::new(1.0, Theme::BORDER), egui::StrokeKind::Middle);
    rect.shrink(10.0)
}

/// N03: which screens the rebindable inventory key (E) closes — every
/// container/station screen returns to play on the same key that opens
/// the pack. Pure so the input-recovery contract is testable without a
/// window; Escape already closes everything via `close_ui`.
pub fn inventory_key_closes(ui_open: &UiOpen) -> bool {
    matches!(ui_open,
        UiOpen::Inventory | UiOpen::HandCraft | UiOpen::CraftingTable
        | UiOpen::Furnace(_) | UiOpen::Chest(_) | UiOpen::Machine(_))
}

/// N02: the craft-station catalog entry for an output id — what the queue
/// and craft-all need (aggregated ingredients + per-craft output count).
pub fn catalog_craft_entry(output: &str) -> Option<(Vec<(String, u8)>, u8)> {
    build_catalog().into_iter()
        .find(|e| e.station == Station::Craft && e.output == output)
        .map(|e| (e.ingredients, e.output_count))
}

/// N02: live status of the queue head for the queue strip (and tests) —
/// read-only; the mutating work belongs to the transactional engine.
#[derive(Debug)]
pub enum QueueStatus {
    Empty,
    /// The head job could complete right now.
    Running { output: String, remaining: u32 },
    /// The head job is short materials or room; `reason` is player copy.
    Blocked { output: String, remaining: u32, reason: String },
}

pub fn queue_status(queue: &[(String, u32)], inv: &lf_game::survival::Inventory) -> QueueStatus {
    use lf_game::crafting::{execute, CraftOutcome};
    let Some((output, qty)) = queue.first() else {
        return QueueStatus::Empty;
    };
    let Some((ingredients, output_count)) = catalog_craft_entry(output) else {
        return QueueStatus::Blocked {
            output: output.clone(),
            remaining: *qty,
            reason: "recipe unknown".into(),
        };
    };
    // execute against a throwaway clone verifies without mutating
    let mut probe = inv.clone();
    match execute(&mut probe, &ingredients, output, output_count, *qty) {
        CraftOutcome::Crafted { .. } => QueueStatus::Running {
            output: output.clone(),
            remaining: *qty,
        },
        CraftOutcome::Blocked(b) => QueueStatus::Blocked {
            output: output.clone(),
            remaining: *qty,
            reason: b.reason(),
        },
    }
}


// ------------------------------------------------------------------

/// loop 345: the kingdom-compass dial — a player-relative compass rose
/// with the red needle swung toward the nearest kingdom's throne. Public
/// and self-contained so the vistest proof renders the real HUD pixels.
pub fn paint_kingdom_compass(
    painter: &egui::Painter,
    c: egui::Pos2,
    r: f32,
    yaw: f32,
    needle_bearing: Option<f32>,
    label: &str,
) {
    let gold = egui::Color32::from_rgb(240, 200, 110);
    let needle_red = egui::Color32::from_rgb(220, 70, 60);
    // case: dark dial with a gold rim
    painter.circle_filled(c, r, Theme::PANEL);
    painter.circle_stroke(c, r, egui::Stroke::new(2.0, gold));
    // cardinal ticks rotated so the player's facing is up
    use std::f32::consts::{FRAC_PI_2, PI};
    for ang in [0.0, FRAC_PI_2, PI, -FRAC_PI_2] {
        let a = ang - yaw;
        let dir = egui::Vec2::new(a.sin(), -a.cos());
        painter.line_segment(
            [c + dir * (r - 10.0), c + dir * (r - 4.0)],
            egui::Stroke::new(1.5, Theme::TEXT_DIM),
        );
    }
    match needle_bearing {
        Some(bearing) => {
            // the needle: bearing relative to facing (0 = dead ahead = up)
            let rel = bearing - yaw;
            let dir = egui::Vec2::new(rel.sin(), -rel.cos());
            painter.line_segment(
                [c - dir * (r * 0.55), c + dir * (r * 0.72)],
                egui::Stroke::new(3.0, needle_red),
            );
            painter.circle_filled(c + dir * (r * 0.72), 2.5, needle_red);
        }
        None => {
            // no kingdom in reach: the needle rests, dimmed
            painter.line_segment(
                [c - egui::Vec2::new(0.0, r * 0.5), c + egui::Vec2::new(0.0, r * 0.7)],
                egui::Stroke::new(3.0, Theme::TEXT_DISABLED),
            );
        }
    }
    painter.circle_filled(c, 3.0, gold);
    painter.text(
        c + egui::Vec2::new(0.0, r + 13.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(12.0),
        Theme::TEXT,
    );
}

// ---- N01: first-minute tutorial card + pinned starter objective ----
// Pure rect math + painters shared with the vistest proofs, so the proof
// pixels are the in-game pixels (the kingdom-compass pattern).

/// The tutorial card sits top-center, clear of the info line (top-left)
/// and the minimap (top-right). Width caps so a 640px window still shows
/// the full card without touching either corner band.
pub fn onboarding_prompt_rect(screen: egui::Rect) -> egui::Rect {
    let w = 316.0_f32.min(screen.width() - 24.0).max(160.0);
    egui::Rect::from_center_size(
        egui::Pos2::new(screen.center().x, screen.top() + 54.0),
        egui::vec2(w, 48.0),
    )
}

/// The pinned objective line sits directly under the tutorial card (or at
/// the card's position once the tutorial is Done/dismissed).
pub fn onboarding_objective_rect(screen: egui::Rect, tutorial_showing: bool) -> egui::Rect {
    let above = onboarding_prompt_rect(screen);
    let center = if tutorial_showing {
        egui::Pos2::new(above.center().x, above.bottom() + 15.0)
    } else {
        above.center()
    };
    egui::Rect::from_center_size(center, egui::vec2(above.width() * 0.86, 26.0))
}

/// Paint the tutorial card: accent spine, verb, key chips from the live
/// keymap, the action label, the `n/5` step chip, and the dismiss ✕.
pub fn paint_onboarding_prompt(
    painter: &egui::Painter,
    rect: egui::Rect,
    prompt: &crate::onboarding::OnboardingPrompt,
    step_number: usize,
) {
    painter.rect_filled(rect, 8.0, egui::Color32::from_rgba_unmultiplied(0x22, 0x1b, 0x12, 235));
    painter.rect_stroke(rect, 8.0, egui::Stroke::new(1.0, Theme::BORDER), egui::StrokeKind::Middle);
    // accent spine on the left edge — "there is a next action here"
    painter.rect_filled(
        egui::Rect::from_min_size(rect.min, egui::vec2(4.0, rect.height())),
        8.0, Theme::ACCENT,
    );
    painter.text(
        rect.left_top() + egui::vec2(14.0, 13.0),
        egui::Align2::LEFT_CENTER,
        &prompt.verb,
        egui::FontId::proportional(13.5),
        Theme::TEXT_BRIGHT,
    );
    // step chip n/5 (right edge, under the ✕)
    painter.text(
        rect.right_top() + egui::vec2(-14.0, 13.0),
        egui::Align2::RIGHT_CENTER,
        format!("{}/{}", step_number, crate::onboarding::TutorialStep::TOTAL),
        egui::FontId::proportional(10.0),
        Theme::TEXT_DIM,
    );
    // dismiss glyph (the game layers a click target over it)
    painter.text(
        rect.right_top() + egui::vec2(-16.0, -9.0),
        egui::Align2::RIGHT_BOTTOM,
        "✕",
        egui::FontId::proportional(11.0),
        Theme::TEXT_DIM,
    );
    // chips + label row along the bottom
    let mut x = rect.left() + 14.0;
    let y = rect.bottom() - 14.0;
    for chip in &prompt.chips {
        let w = 12.0 + chip.len() as f32 * 7.5;
        let r = egui::Rect::from_min_size(egui::pos2(x, y - 9.0), egui::vec2(w, 17.0));
        painter.rect_filled(r, 4.0, egui::Color32::from_black_alpha(200));
        painter.rect_stroke(r, 4.0, egui::Stroke::new(1.0, Theme::BORDER), egui::StrokeKind::Middle);
        painter.text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            chip,
            egui::FontId::proportional(10.5),
            Theme::TEXT,
        );
        x = r.right() + 6.0;
    }
    painter.text(
        egui::pos2(x + 2.0, y),
        egui::Align2::LEFT_CENTER,
        &prompt.label,
        egui::FontId::proportional(11.0),
        Theme::TEXT_DIM,
    );
}

/// Paint the pinned objective line: the active starter quest + its first
/// incomplete objective progress, one quiet line that never blocks input.
pub fn paint_pinned_objective(
    painter: &egui::Painter,
    rect: egui::Rect,
    title: &str,
    progress: &str,
) {
    painter.rect_filled(rect, 7.0, egui::Color32::from_rgba_unmultiplied(0x22, 0x1b, 0x12, 205));
    painter.rect_stroke(rect, 7.0, egui::Stroke::new(1.0, Theme::BORDER), egui::StrokeKind::Middle);
    painter.text(
        rect.left_center() + egui::vec2(12.0, 0.0),
        egui::Align2::LEFT_CENTER,
        "◈",
        egui::FontId::proportional(11.0),
        Theme::ACCENT,
    );
    painter.text(
        rect.left_center() + egui::vec2(28.0, 0.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(11.5),
        Theme::TEXT,
    );
    painter.text(
        rect.right_center() + egui::vec2(-30.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        progress,
        egui::FontId::proportional(10.5),
        Theme::TEXT_DIM,
    );
    painter.text(
        rect.right_center() + egui::vec2(-11.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        "✕",
        egui::FontId::proportional(10.0),
        Theme::TEXT_DISABLED,
    );
}

/// The first incomplete quest and its first incomplete objective, as the
/// pinned line presents them: `(quest title, "objective n/m")`.
pub fn pinned_objective(log: &crate::QuestLog) -> Option<(String, String)> {
    let quest = log.quests.iter().find(|q| !q.completed)?;
    let obj = quest.objectives.iter().find(|o| !o.completed)?;
    let name = item_def(&obj.target).map(|d| d.name.to_string())
        .unwrap_or_else(|| obj.target.replace('_', " "));
    Some((
        quest.title.clone(),
        format!("{} {}/{}", name.to_lowercase(), obj.progress, obj.count),
    ))
}

// ---- N04: contextual HUD channel painters (shared with the proofs) ----

/// The interaction prompt beside the crosshair: a keycap chip, a verb,
/// the target name, and (in BAD red) the blocked reason.
pub fn paint_interaction_prompt(
    painter: &egui::Painter,
    at: egui::Pos2,
    prompt: &crate::hud_channels::InteractionPrompt,
) {
    let mut x = at.x;
    for chip in &prompt.chips {
        let w = 10.0 + chip.len() as f32 * 7.0;
        let r = egui::Rect::from_min_size(egui::pos2(x, at.y - 9.0), egui::vec2(w, 17.0));
        // opaque well: a keycap must read over any backdrop, bright sky
        // included (semi-transparent black muddies over bright fills)
        painter.rect_filled(r, 4.0, Theme::BG);
        painter.rect_stroke(r, 4.0, egui::Stroke::new(1.0, Theme::BORDER), egui::StrokeKind::Middle);
        painter.text(r.center(), egui::Align2::CENTER_CENTER, chip,
            egui::FontId::proportional(10.5), Theme::TEXT);
        x = r.right() + 6.0;
    }
    let body = if let Some(reason) = &prompt.blocked {
        format!("{} — {} ({})", prompt.verb, prompt.target, reason)
    } else {
        format!("{} — {}", prompt.verb, prompt.target)
    };
    let color = if prompt.blocked.is_some() { Theme::BAD } else { Theme::TEXT };
    kit::text_shadowed(painter, egui::pos2(x, at.y), egui::Align2::LEFT_CENTER,
        body, egui::FontId::proportional(12.0), color);
}

/// The hit-direction arc around the crosshair: a red segment at the
/// bearing the damage came from (0 = ahead), fading over HIT_DIR_LIFE.
pub fn paint_hit_direction(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    bearing: f32,
    fade: f32,
) {
    let a = bearing;
    let dir = egui::Vec2::new(a.sin(), -a.cos());
    let tip = center + dir * radius;
    let alpha = (fade * 255.0) as u8;
    // the arc segment: three strokes fanning the direction
    for spread in [-0.35, 0.0, 0.35] {
        let d = egui::Vec2::new((a + spread).sin(), -(a + spread).cos());
        painter.line_segment(
            [center + d * (radius * 0.66), center + d * radius],
            egui::Stroke::new(3.0, egui::Color32::from_rgba_unmultiplied(220, 60, 50, alpha)),
        );
    }
    painter.circle_filled(tip, 2.5, egui::Color32::from_rgba_unmultiplied(220, 60, 50, alpha));
}

/// Attack readiness: a thin arc under the crosshair that sweeps closed
/// while the cooldown runs (full ring = ready).
pub fn paint_attack_readiness(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    cooldown_frac: f32,
) {
    let pts = kit::reticle_points(center, radius, 1.0 - cooldown_frac.clamp(0.0, 1.0));
    for w in pts.windows(2) {
        painter.line_segment(
            [w[0], w[1]],
            egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(240, 234, 214, 140)),
        );
    }
}

/// One reputation toast: faction crest square in the realm's color, the
/// signed delta, the reason, and the threshold title when crossed.
pub fn paint_reputation_toast(
    painter: &egui::Painter,
    rect: egui::Rect,
    toast: &crate::hud_channels::ReputationToast,
) {
    let fade = (1.0 - toast.age / crate::hud_channels::REP_TOAST_LIFE).clamp(0.0, 1.0);
    let alpha = (fade * 235.0) as u8;
    painter.rect_filled(rect, 7.0, egui::Color32::from_rgba_unmultiplied(0x22, 0x1b, 0x12, alpha));
    painter.rect_stroke(rect, 7.0, egui::Stroke::new(1.0, Theme::BORDER), egui::StrokeKind::Middle);
    let crest = egui::Rect::from_center_size(
        rect.left_center() + egui::vec2(13.0, 0.0), egui::vec2(11.0, 11.0));
    painter.rect_filled(crest, 3.0,
        egui::Color32::from_rgba_unmultiplied(toast.color[0], toast.color[1], toast.color[2], alpha));
    let sign = if toast.delta >= 0 { "+" } else { "" };
    let col = |c: egui::Color32| egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha);
    // faction name bright, delta colored — sign + shape carry polarity
    kit::text_shadowed(painter, rect.left_center() + egui::vec2(24.0, 0.0),
        egui::Align2::LEFT_CENTER,
        format!("{} {}{}", toast.faction_short, sign, toast.delta),
        egui::FontId::proportional(11.5),
        col(if toast.delta >= 0 { Theme::OK } else { Theme::BAD }));
    painter.text(
        rect.left_center() + egui::vec2(24.0, 0.0),
        egui::Align2::LEFT_CENTER,
        &toast.faction_short,
        egui::FontId::proportional(11.5),
        col(Theme::TEXT_BRIGHT),
    );
    kit::text_shadowed(painter, rect.left_center() + egui::vec2(24.0, 12.0),
        egui::Align2::LEFT_CENTER, toast.reason.clone(),
        egui::FontId::proportional(10.0), col(Theme::TEXT_DIM));
    if let Some(title) = &toast.title_line {
        kit::text_shadowed(painter, rect.left_center() + egui::vec2(24.0, -11.0),
            egui::Align2::LEFT_CENTER, title.clone(),
            egui::FontId::proportional(10.0), col(Theme::WARNING));
    }
}

/// The settlement entry banner: realm + place name and its safety state,
/// centered under the pinned objective line, fading.
pub fn paint_settlement_banner(
    painter: &egui::Painter,
    rect: egui::Rect,
    banner: &crate::hud_channels::SettlementBanner,
) {
    let fade = (1.0 - banner.age / crate::hud_channels::SETTLEMENT_LIFE).clamp(0.0, 1.0);
    let alpha = (fade * 225.0) as u8;
    painter.rect_filled(rect, 7.0, egui::Color32::from_rgba_unmultiplied(0x22, 0x1b, 0x12, alpha));
    painter.rect_stroke(rect, 7.0, egui::Stroke::new(1.0, Theme::BORDER), egui::StrokeKind::Middle);
    let col = |c: egui::Color32| egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha);
    kit::text_shadowed(painter, rect.center() + egui::vec2(0.0, -6.0),
        egui::Align2::CENTER_CENTER, banner.name.clone(),
        egui::FontId::proportional(13.5), col(Theme::TEXT_BRIGHT));
    kit::text_shadowed(painter, rect.center() + egui::vec2(0.0, 8.0),
        egui::Align2::CENTER_CENTER, banner.state_line.clone(),
        egui::FontId::proportional(10.5), col(Theme::TEXT_DIM));
}

/// The danger line above the bottom HUD band — one line only (priority
/// resolved in `HudChannels::danger_warning`), severity shown by shape
/// (double chevron at 2, single at 1) as well as color.
pub fn paint_danger_line(
    painter: &egui::Painter,
    center: egui::Pos2,
    text: &str,
    severity: u8,
    pulse: f32,
) {
    let color = match severity {
        2 => Theme::BAD,
        1 => Theme::WARNING,
        _ => Theme::TEXT_DIM,
    };
    let mark = if severity >= 2 { "!! " } else if severity == 1 { "! " } else { "" };
    let alpha = ((0.55 + 0.45 * pulse) * 255.0) as u8;
    let full = format!("{}{}", mark, text);
    // backing plate: the warning must read over busy terrain, bright sky,
    // and night alike (the review caught it drowning in the backdrop)
    let plate_w = 34.0 + full.len() as f32 * 6.2;
    let plate = egui::Rect::from_center_size(center, egui::vec2(plate_w, 20.0));
    painter.rect_filled(plate, 6.0, egui::Color32::from_rgba_unmultiplied(0x1a, 0x14, 0x10, 215));
    painter.rect_stroke(plate, 6.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)),
        egui::StrokeKind::Middle);
    kit::text_shadowed(painter, center, egui::Align2::CENTER_CENTER,
        full,
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha));
}


/// Gameplay HUD visibility: hidden behind menus that own the whole view.
/// The audit caught hearts + hotbar rendering under the title menu (and
/// under settings opened from the title); pause keeps the HUD visible the
/// way Minecraft-style pause overlays do. N03: container/station screens
/// are modals with their own inventory strips — the survival HUD (hearts,
/// hotbar, XP, minimap) must not duplicate beneath them.
fn hud_visible(ui_open: &UiOpen, settings_from_title: bool) -> bool {
    !matches!(ui_open, UiOpen::Title)
        && !(*ui_open == UiOpen::Settings && settings_from_title)
        && !matches!(ui_open,
            UiOpen::HandCraft | UiOpen::CraftingTable
            | UiOpen::Furnace(_) | UiOpen::Chest(_) | UiOpen::Machine(_)
            | UiOpen::Smithing | UiOpen::Imbue | UiOpen::Carve)
}

impl GameState {
    /// Draw every UI surface for this frame.
    pub fn draw_ui(&mut self, ctx: &egui::Context) {
        // every plain widget + egui::Window inherits the kit palette
        kit::apply_kit_style(ctx);
        // soft click whenever the open screen changes (loop 329 audio set)
        if self.ui_open != self.prev_ui_open {
            self.play_sfx(lf_audio::Sfx::UiClick, 0.5);
        }
        self.prev_ui_open = self.ui_open;
        // UI scale = user preference × viewport size (720p reference), on
        // top of the native display density egui-winit provides.
        let native_pts_h = self.config.height as f32 / self.window.scale_factor() as f32;
        let viewport_factor = (native_pts_h / 720.0).clamp(0.8, 1.5);
        ctx.set_zoom_factor(self.settings.ui_scale * viewport_factor);
        if hud_visible(&self.ui_open, self.settings_from_title) {
            self.draw_hud(ctx);
        }
        match self.ui_open {
            UiOpen::None => {}
            UiOpen::Title => self.draw_title(ctx),
            UiOpen::Pause => self.draw_pause(ctx),
            UiOpen::Settings => self.draw_settings(ctx),
            UiOpen::QuestLog => self.draw_quest_log(ctx),
            UiOpen::Chat => {}
            // ui-world-craft F: the workbench replaces the grid everywhere.
            // By hand (E) only the always-known basics; a crafting table
            // shows everything earned.
            UiOpen::Inventory => self.draw_inventory(ctx),
            UiOpen::HandCraft => self.draw_workbench(ctx, true),
            UiOpen::CraftingTable => self.draw_workbench(ctx, false),
            UiOpen::Furnace(pos) => self.draw_furnace(ctx, pos),
            UiOpen::Chest(pos) => self.draw_chest(ctx, pos),
            UiOpen::Trade(index) => self.draw_trade(ctx, index),
            UiOpen::Book => self.draw_book(ctx),
            UiOpen::LoreBook => self.draw_lore_book(ctx),
            UiOpen::Spellbook => self.draw_spellbook(ctx),
            UiOpen::Imbue => self.draw_imbue(ctx),
            UiOpen::Carve => self.draw_carve(ctx),
            UiOpen::Paths => self.draw_paths(ctx),
            UiOpen::Smithing => self.draw_smithing(ctx),
            UiOpen::Machine(pos) => self.draw_machine(ctx, pos),
            UiOpen::TechTree => self.draw_tech_tree(ctx),
            UiOpen::Map => self.draw_map_screen(ctx),
            UiOpen::Console => self.draw_console(ctx),
            UiOpen::Slots => self.draw_slots(ctx),
            UiOpen::NewWorld => self.draw_new_world(ctx),
            UiOpen::Multiplayer => self.draw_multiplayer(ctx),
            UiOpen::Death => self.draw_death(ctx),
            UiOpen::CompanionMenu => self.draw_companion_menu(ctx),
        }
        // Cursor stack follows the pointer (icon + count).
        if let Some(cursor) = &self.cursor_stack {
            if let Some(pointer) = ctx.pointer_hover_pos() {
                let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("cursor-stack"));
                let rect = egui::Rect::from_center_size(pointer, egui::vec2(30.0, 30.0));
                let painter = ctx.layer_painter(layer);
                if let Some(tex) = self.icons.get(&cursor.item_id) {
                    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                    painter.image(tex.id(), rect, uv, egui::Color32::WHITE);
                } else {
                    painter.rect_filled(rect, 3.0, item_color(cursor));
                }
                if cursor.count > 1 {
                    painter.text(rect.right_bottom(), egui::Align2::RIGHT_BOTTOM, format!("{}", cursor.count),
                        egui::FontId::proportional(12.0), Theme::TEXT);
                }
            }
        }
    }

    fn draw_hud(&mut self, ctx: &egui::Context) {
        // Live RT image replaces the world view behind the HUD when enabled.
        if self.settings.rt_mode == RtMode::Live {
            if let Some(tex) = &self.live_rt_texture {
                let screen = ctx.screen_rect();
                egui::CentralPanel::default()
                    .frame(egui::Frame::none())
                    .show(ctx, |ui| {
                        ui.image(egui::load::SizedTexture::new(tex.id(), tex.size_vec2()))
                            .rect.set_width(screen.width());
                    });
            }
        }
        // chat overlay (bottom-left)
        let net_chat: Option<Vec<String>> = self.net.as_ref().map(|n| n.chat_log.iter().rev().take(5).rev().cloned().collect());
        let chat_lines = net_chat.unwrap_or_else(|| self.chat_log.clone());
        if !chat_lines.is_empty() {
            egui::Area::new(egui::Id::new("chat"))
                .anchor(egui::Align2::LEFT_BOTTOM,
                    egui::vec2(8.0, -(kit::HUD_BOTTOM_BAND + 8.0)))
                .show(ctx, |ui| {
                    for line in &chat_lines {
                        ui.label(egui::RichText::new(line).small().color(egui::Color32::from_gray(230)));
                    }
                });
        }
        // chat input (T)
        if self.chat_input.is_some() {
            self.ui_open = UiOpen::Chat;
            let mut text = self.chat_input.take().unwrap();
            let mut send = false;
            egui::Window::new("Chat — Enter send / Esc cancel")
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -180.0))
                .collapsible(false).resizable(false)
                .show(ctx, |ui| {
                    let response = ui.add(egui::TextEdit::singleline(&mut text).desired_width(420.0));
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) { send = true; }
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) { text.clear(); send = true; }
                    response.request_focus();
                });
            if send {
                if !text.trim().is_empty() {
                    if let Some(n) = &self.net { n.send_chat(text.trim()); }
                }
                self.ui_open = UiOpen::None;
                self.lock_cursor();
            } else {
                self.chat_input = Some(text);
            }
        }
        // ---- bottom HUD bar ----
        let hotbar_w = 9.0 * (SLOT_SIZE + 4.0);
        egui::TopBottomPanel::bottom("hud").frame(egui::Frame::none()).show_separator_line(false).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(4.0);
                // hearts + armor (left) / hunger (right)
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() * 0.5 - hotbar_w * 0.5);
                    kit::paint_hearts(ui, self.stats.health, self.stats.max_health);
                    let armor = lf_game::combat::worn_armor_points(&self.inventory.slots);
                    if armor > 0 {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(34.0, 16.0), egui::Sense::hover());
                        let p = ui.painter();
                        p.rect_filled(egui::Rect::from_center_size(rect.center(), egui::vec2(11.0, 11.0)), 2.0, egui::Color32::from_rgb(190, 200, 215));
                        p.rect_filled(egui::Rect::from_center_size(rect.center() + egui::vec2(0.0, -3.5), egui::vec2(13.0, 4.0)), 2.0, egui::Color32::from_rgb(210, 220, 235));
                        p.text(rect.center() + egui::vec2(14.0, 0.0), egui::Align2::LEFT_CENTER, format!("+{}", armor),
                            egui::FontId::proportional(12.0), egui::Color32::from_rgb(190, 200, 215));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(ui.available_width() * 0.5 - hotbar_w * 0.5);
                        kit::paint_hunger(ui, self.stats.hunger, self.stats.max_hunger);
                    });
                });
                if self.air < 10 {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(90.0, 14.0), egui::Sense::hover());
                    kit::text_shadowed(ui.painter(), r.left_center(), egui::Align2::LEFT_CENTER,
                        format!("air {}", "·".repeat(self.air as usize)),
                        egui::FontId::proportional(12.0), Theme::XP);
                }
                // Steps 21-22: the chronicle toasts live milestones in play
                if let Some((text, t)) = &self.chronicle_toast {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover());
                    let alpha = (t / 4.0).clamp(0.0, 1.0);
                    kit::text_shadowed(ui.painter(), r.center(), egui::Align2::CENTER_CENTER,
                        text.clone(), egui::FontId::proportional(14.0),
                        egui::Color32::from_rgba_unmultiplied(Theme::ACCENT.r(), Theme::ACCENT.g(), Theme::ACCENT.b(), (alpha * 255.0) as u8));
                }
                // XP bar mirroring the hotbar width, with level chip + gain flash
                let frac = (self.xp_progress as f32 / lf_game::combat::xp_for_level(self.xp_level).max(1) as f32).clamp(0.0, 1.0);
                let (rect, _) = ui.allocate_exact_size(egui::vec2(hotbar_w, 9.0), egui::Sense::hover());
                let p = ui.painter();
                p.rect_filled(rect, 4.0, egui::Color32::from_black_alpha(190));
                p.rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * frac, rect.height())), 4.0, Theme::XP);
                if self.xp_flash > 0.0 {
                    let a = (self.xp_flash * 130.0) as u8;
                    p.rect_stroke(rect, 4.0, egui::Stroke::new(2.0, egui::Color32::from_rgba_unmultiplied(Theme::XP.r(), Theme::XP.g(), Theme::XP.b(), a)), egui::StrokeKind::Middle);
                }
                let chip = egui::Rect::from_center_size(rect.center(), egui::vec2(34.0, 14.0));
                p.rect_filled(chip, 4.0, Theme::BG);
                p.text(chip.center(), egui::Align2::CENTER_CENTER, format!("Lv {}", self.xp_level),
                    egui::FontId::proportional(11.0), Theme::XP);
                ui.add_space(1.0);
                // Mana bar (P33): only once a spell is known — magic is
                // something you found, not something you were born with.
                if !self.spellbook.learned.is_empty() {
                    let frac = (self.stats.mana / self.stats.max_mana).clamp(0.0, 1.0);
                    let (mrect, _) = ui.allocate_exact_size(egui::vec2(hotbar_w, 5.0), egui::Sense::hover());
                    let p = ui.painter();
                    p.rect_filled(mrect, 3.0, egui::Color32::from_black_alpha(190));
                    p.rect_filled(egui::Rect::from_min_size(mrect.min, egui::vec2(mrect.width() * frac, mrect.height())), 3.0, Theme::MANA);
                    let chip = egui::Rect::from_center_size(mrect.right_center(), egui::vec2(30.0, 12.0));
                    p.rect_filled(chip, 3.0, Theme::BG);
                    p.text(chip.center(), egui::Align2::CENTER_CENTER, format!("{:.0}", self.stats.mana),
                        egui::FontId::proportional(10.0), Theme::MANA);
                    ui.add_space(1.0);
                }
                // caption above the bar (always allocated so the layout
                // never shifts): held-item name while a switch fades out,
                // else the looked-at block's name
                let held_name = self.inventory.slots[self.hotbar_index].as_ref()
                    .and_then(|s| item_def(&s.item_id).map(|d| d.name.to_string()));
                let target_name = self.look_target
                    .map(|(_, id)| registry::block::name(id).to_string());
                let caption = hotbar_caption(held_name.as_deref(), self.hotbar_pick_time,
                    target_name.as_deref());
                let (crect, _) = ui.allocate_exact_size(egui::vec2(hotbar_w, 16.0), egui::Sense::hover());
                if let Some((text, accent)) = caption {
                    let color = if accent {
                        let a = (self.hotbar_pick_time.min(1.0) * 255.0) as u8;
                        egui::Color32::from_rgba_unmultiplied(Theme::ACCENT.r(), Theme::ACCENT.g(), Theme::ACCENT.b(), a)
                    } else {
                        egui::Color32::from_rgba_unmultiplied(230, 230, 230, 205)
                    };
                    kit::text_shadowed(ui.painter(), crect.center(), egui::Align2::CENTER_CENTER,
                        text, egui::FontId::proportional(12.0), color);
                }
                // hotbar
                ui.horizontal(|ui| {
                    for i in 0..9 {
                        let mut stack = self.inventory.slots[i].clone();
                        let selected = i == self.hotbar_index;
                        let mut cursor = self.cursor_stack.take();
                        let outcome = slot_button(ui, &mut stack, &mut cursor, selected, &self.icons);
                        self.cursor_stack = cursor;
                        self.inventory.slots[i] = stack;
                        if let Some(s) = outcome.hovered {
                            self.hotbar_hover = Some(s.item_id);
                        }
                    }
                });
                // mining / bow charge feedback moved to the crosshair
                // reticle (Section 2: the old bottom-of-screen progress bar
                // read as an artifact at the foot of the HUD)
                ui.label(egui::RichText::new("").small());
            });
        });
        // minimap (top-right) while playing
        if self.stats.health > 0.0 && matches!(self.ui_open, UiOpen::None | UiOpen::Chat) {
            self.draw_minimap(ctx);
        }
        // loop 343 building HUD: placement-shape chips + the symmetry
        // indicator, floating above the hotbar band while a block is held
        // or symmetry is live (R cycles, V toggles symmetry)
        if self.stats.health > 0.0 && matches!(self.ui_open, UiOpen::None | UiOpen::Chat) {
            let held_is_block = self.inventory.slots[self.hotbar_index].as_ref()
                .and_then(|s| item_def(&s.item_id))
                .map(|d| matches!(d.kind, ItemKind::Block(_)))
                .unwrap_or(false);
            if held_is_block || self.symmetry_plane.is_some() {
                self.draw_build_hud(ctx);
            }
        }
        // loop 345: the kingdom compass, held — a dial under the crosshair
        // whose needle swings toward the nearest kingdom
        if self.stats.health > 0.0 && matches!(self.ui_open, UiOpen::None | UiOpen::Chat) {
            let compass_held = self.inventory.slots[self.hotbar_index].as_ref()
                .map(|s| s.item_id == "kingdom_compass")
                .unwrap_or(false);
            if compass_held {
                let screen = ctx.screen_rect();
                let c = egui::Pos2::new(screen.center().x, screen.top() + 92.0);
                let (bearing, label) = match &self.kingdom_compass_state {
                    Some((name, bearing, meters)) =>
                        (Some(*bearing), format!("{} · {}m", name, meters)),
                    None => (None, "no kingdom in reach".to_string()),
                };
                paint_kingdom_compass(
                    &ctx.debug_painter(),
                    c,
                    30.0,
                    self.player.yaw,
                    bearing,
                    &label,
                );
            }
        }
        // N01: first-minute tutorial card + pinned starter objective
        // (top-center, compact, never blocks input; prompts pause behind
        // modal screens). Painted by the shared proof painters.
        if self.stats.health > 0.0 && matches!(self.ui_open, UiOpen::None | UiOpen::Chat)
            && self.game_mode == crate::slots::GameMode::Survival {
            let screen = ctx.screen_rect();
            let tutorial_showing = !self.onboarding.dismissed
                && self.onboarding.step != crate::onboarding::TutorialStep::Done;
            let mut dismissed_card = false;
            let mut dismissed_objective = false;
            if tutorial_showing {
                let rect = onboarding_prompt_rect(screen);
                let prompt = self.onboarding.prompt(&self.keymap);
                paint_onboarding_prompt(
                    &ctx.debug_painter(),
                    rect,
                    &prompt,
                    self.onboarding.step.number(),
                );
                // click target over the painted ✕
                let x_rect = egui::Rect::from_min_size(
                    rect.right_top() + egui::vec2(-24.0, 2.0), egui::vec2(22.0, 22.0));
                egui::Area::new(egui::Id::new("onboarding_dismiss"))
                    .fixed_pos(x_rect.min)
                    .order(egui::Order::Foreground)
                    .interactable(true)
                    .show(ctx, |ui| {
                        let (_, resp) = ui.allocate_exact_size(x_rect.size(), egui::Sense::click());
                        if resp.clicked() {
                            dismissed_card = true;
                        }
                    });
            }
            if !self.onboarding.objective_dismissed {
                if let Some((title, progress)) = pinned_objective(&self.quest_log) {
                    let orect = onboarding_objective_rect(screen, tutorial_showing);
                    paint_pinned_objective(&ctx.debug_painter(), orect, &title, &progress);
                    let x_rect = egui::Rect::from_min_size(
                        orect.right_top() + egui::vec2(-24.0, 0.0), egui::vec2(24.0, orect.height()));
                    egui::Area::new(egui::Id::new("objective_dismiss"))
                        .fixed_pos(x_rect.min)
                        .order(egui::Order::Foreground)
                        .interactable(true)
                        .show(ctx, |ui| {
                            let (_, resp) = ui.allocate_exact_size(x_rect.size(), egui::Sense::click());
                            if resp.clicked() {
                                dismissed_objective = true;
                            }
                        });
                }
            }
            if dismissed_card {
                self.onboarding.dismissed = true;
            }
            if dismissed_objective {
                self.onboarding.objective_dismissed = true;
            }
        }
        // N04: contextual channels — interaction prompt beside the
        // crosshair, hit direction + attack readiness around it, the
        // danger line above the bottom band, reputation toasts under the
        // minimap, and the settlement banner under the pinned line.
        // Nothing here blocks input; everything fades.
        if self.stats.health > 0.0 && matches!(self.ui_open, UiOpen::None | UiOpen::Chat) {
            let screen = ctx.screen_rect();
            let center = screen.center();
            let p = ctx.debug_painter();
            // ---- interaction prompt (priority: companion > villager >
            // functional block > mine > place) — built per branch so the
            // borrowed target names resolve inside their own scope ----
            use crate::hud_channels::{interaction_prompt, Focus};
            let prompt: Option<crate::hud_channels::InteractionPrompt>;
            if let Some(ci) = self.companion_in_crosshair() {
                prompt = self.companions.get(ci)
                    .and_then(|c| interaction_prompt(
                        &Focus::Companion { name: &c.display_name }, &self.keymap));
            } else if let Some(vi) = self.villager_in_crosshair() {
                prompt = self.villagers.get(vi)
                    .and_then(|v| interaction_prompt(
                        &Focus::Villager {
                            name: &v.name,
                            barred: v.faction.as_deref()
                                .map(|f| self.standings.refuses_trade(f))
                                .unwrap_or(false),
                        }, &self.keymap));
            } else if let Some((_, id)) = self.look_target {
                use registry::block as blk;
                // held-item facts hoisted out of the match so the place
                // prompt can borrow the name beyond its arm
                let held = self.inventory.slots[self.hotbar_index].as_ref();
                let held_is_block = held
                    .and_then(|st| item_def(&st.item_id))
                    .map(|d| matches!(d.kind, ItemKind::Block(_)))
                    .unwrap_or(false);
                let held_name = held
                    .and_then(|st| item_def(&st.item_id).map(|d| d.name.to_string()))
                    .unwrap_or_default();
                let blocked_by_player = held_is_block
                    && self.crosshair_block_pos()
                        .map(|(x, y, z)| {
                            // the cell one step back along the dominant
                            // eye->hit axis is where the block lands when
                            // close — placing there would clip the player
                            let eye = self.player.eye_position();
                            let d = eye - glam::Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                            let cell = if d.x.abs() >= d.z.abs() {
                                glam::IVec3::new(x + d.x.signum() as i32, y, z)
                            } else {
                                glam::IVec3::new(x, y, z + d.z.signum() as i32)
                            };
                            self.block_intersects_player(cell)
                        })
                        .unwrap_or(false);
                let focus = match id {
                    x if x == blk::CHEST => Some(Focus::Interactable { verb: "Open", name: "Chest" }),
                    x if x == blk::FURNACE => Some(Focus::Interactable { verb: "Use", name: "Furnace" }),
                    x if x == blk::CRAFTING_TABLE => Some(Focus::Interactable { verb: "Craft", name: "Crafting Table" }),
                    x if x == blk::ENCHANTING_TABLE => Some(Focus::Interactable { verb: "Imbue", name: "Enchanting Table" }),
                    x if x == blk::SMITHING_TABLE => Some(Focus::Interactable { verb: "Forge", name: "Smithing Table" }),
                    _ if held_is_block => Some(Focus::Place {
                        name: &held_name, blocked_by_player,
                    }),
                    _ => Some(Focus::Mine { name: registry::block::name(id) }),
                };
                prompt = focus.as_ref()
                    .and_then(|f| interaction_prompt(f, &self.keymap));
            } else {
                prompt = None;
            }
            if let Some(prompt) = &prompt {
                paint_interaction_prompt(&p, center + egui::vec2(34.0, -8.0), prompt);
            }
            // ---- combat: hit direction (world-true bearing vs live yaw)
            // + attack readiness ring ----
            if let Some(h) = &self.hud_channels.hit_dir {
                let fade = (1.0 - h.age / crate::hud_channels::HIT_DIR_LIFE).clamp(0.0, 1.0);
                paint_hit_direction(&p, center, 46.0, h.bearing - self.player.yaw, fade);
            }
            if self.attack_cooldown > 0.0 {
                paint_attack_readiness(&p, center, 24.0, self.attack_cooldown / 0.5);
            }
            // ---- the single danger line (priority-resolved) ----
            if let Some((text, sev)) = self.hud_channels.danger_warning(
                self.stats.health / self.stats.max_health,
                self.stats.hunger, self.air, self.threat_count) {
                let pulse = 0.5 + 0.5 * (self.elapsed * 3.0).sin();
                paint_danger_line(&p,
                    egui::Pos2::new(center.x, screen.bottom() - kit::HUD_BOTTOM_BAND - 26.0),
                    &text, sev, pulse);
            }
            // ---- reputation toasts (top-right, under the minimap) ----
            for (i, toast) in self.hud_channels.rep_toasts.iter().enumerate() {
                let rect = egui::Rect::from_min_size(
                    egui::pos2(screen.right() - 14.0 - 190.0,
                        screen.top() + kit::HUD_INFO_LINE_H + 34.0 + i as f32 * 44.0),
                    egui::vec2(190.0, 38.0));
                paint_reputation_toast(&p, rect, toast);
            }
            // ---- settlement banner (under the pinned objective slot) ----
            if let Some(banner) = &self.hud_channels.settlement {
                let rect = onboarding_objective_rect(screen, true);
                let rect = egui::Rect::from_center_size(
                    egui::Pos2::new(rect.center().x, rect.bottom() + 20.0),
                    egui::vec2(260.0, 34.0));
                paint_settlement_banner(&p, rect, banner);
            }
        }
        // info line (top-left). Minimal by default — research on Minecraft's
        // HUD: the survival screen shows nothing until F3, so clutter here
        // competes with the world. Default = clock + facing only; the dense
        // readout (biome, coords, weather, net, fps, RT) joins F3.
        let facing = crate::map::compass_facing(self.player.yaw);
        let info = if self.show_debug {
            let biome = self.map.biome_at(self.player.position.x as i32, self.player.position.z as i32).name();
            let mut info = format!("{} · {} · {:.0},{:.0} · {}", facing, biome,
                self.player.position.x, self.player.position.z, self.time_label());
            info.push_str(if self.weather_raining { " · rain" } else { " · clear" });
            if let Some(n) = &self.net {
                info.push_str(&format!(" · net:{}", if n.connected { "on" } else { "…" }));
            }
            if self.settings.show_fps {
                info.push_str(&format!(" · {:.0} fps", self.last_fps));
            }
            if self.settings.rt_mode == RtMode::Live {
                info.push_str(" · RT");
            }
            // N05: the full world identity is F3-only (debug telemetry,
            // never required to play)
            info.push_str(&format!(" · {}", self.world_identity.describe()));
            info
        } else {
            format!("{} · {}", self.time_label(), facing)
        };
        let slots = kit::hud_layout(ctx.screen_rect().width(), ctx.screen_rect().height());
        let max_w = slots[0].rect.width();
        egui::Area::new(egui::Id::new("info_line"))
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 8.0))
            .show(ctx, |ui| {
                ui.set_max_width(max_w);
                ui.label(egui::RichText::new(info).small().color(egui::Color32::from_rgba_premultiplied(Theme::TEXT.r(), Theme::TEXT.g(), Theme::TEXT.b(), 200)));
            });
        // lore-and-visuals A3/C4: companion status tiles under the info
        // line (one per active companion, trust + morale bars, state chip)
        if !self.companions.is_empty() {
            egui::Area::new(egui::Id::new("companion_tiles"))
                .anchor(egui::Align2::LEFT_TOP,
                    egui::vec2(10.0, 10.0 + kit::HUD_INFO_LINE_H + 4.0))
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        for c in &self.companions {
                            let color = c.faction_id.as_deref()
                                .and_then(|f| self.lore_data.faction(f))
                                .map(|f| egui::Color32::from_rgb(f.color[0], f.color[1], f.color[2]))
                                .unwrap_or(Theme::TEXT_DIM);
                            egui::Frame::new()
                                .fill(Theme::PANEL)
                                .corner_radius(6.0)
                                .inner_margin(4.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        // portrait tile: faction color + archetype initial
                                        let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                                        ui.painter().rect_filled(r, 4.0, color);
                                        ui.put(r, egui::Label::new(egui::RichText::new(
                                            c.display_name.chars().next().unwrap_or('?').to_string())
                                            .small().strong().color(Theme::BG)).selectable(false));
                                        ui.vertical(|ui| {
                                            ui.label(egui::RichText::new(match &c.state {
                                                lf_game::companions::CompanionState::Idle => "IDLE".into(),
                                                lf_game::companions::CompanionState::Following => "FOLLOW".into(),
                                                lf_game::companions::CompanionState::Guarding { .. } => "GUARD".into(),
                                                lf_game::companions::CompanionState::Resting => "REST".into(),
                                                lf_game::companions::CompanionState::Working => {
                                                    format!("WORK·{}", c.assigned_task.as_ref().map(|t| t.label()).unwrap_or(""))
                                                }
                                            }).small().color(Theme::TEXT_DIM));
                                            let bar = |ui: &mut egui::Ui, v: i32, color: egui::Color32| {
                                                let (r, _) = ui.allocate_exact_size(egui::vec2(64.0, 3.0), egui::Sense::hover());
                                                ui.painter().rect_filled(r, 1.5, egui::Color32::from_black_alpha(150));
                                                ui.painter().rect_filled(
                                                    egui::Rect::from_min_size(r.min, egui::vec2(r.width() * v as f32 / 100.0, r.height())),
                                                    1.5, color);
                                            };
                                            bar(ui, c.trust, Theme::ACCENT);
                                            bar(ui, c.morale, Theme::OK);
                                        });
                                    });
                                });
                        }
                    });
                });
        }
        // lore-and-visuals A3/C4: faction standing widget (bottom-right,
        // above the hotbar; only in territory or near a faction structure).
        // Pulses briefly via the faction_pulse timer when standing changes.
        if let Some(fdef) = self.standing_widget_faction() {
            let standing = self.standings.get(&fdef.id);
            let widget_color = egui::Color32::from_rgb(fdef.color[0], fdef.color[1], fdef.color[2]);
            let value_color = if standing > 15 { Theme::ACCENT } else if standing < -15 { Theme::BAD } else { Theme::TEXT_DIM };
            let pulse = kit::ease_out_cubic(self.faction_pulse);
            let scale = 1.0 + pulse * 0.18;
            egui::Area::new(egui::Id::new("faction_widget"))
                .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-10.0, -96.0))
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgba_premultiplied(Theme::PANEL.r(), Theme::PANEL.g(), Theme::PANEL.b(), 210))
                        .corner_radius((8.0 * scale).min(11.0))
                        .inner_margin(egui::Margin::symmetric(8, 5))
                        .stroke(egui::Stroke::new(1.0 + pulse * 2.0, widget_color))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                                ui.painter().rect_filled(r, 2.0, widget_color);
                                ui.label(egui::RichText::new(fdef.symbol.clone()).small().color(widget_color));
                                ui.label(egui::RichText::new(fdef.short_name.clone()).small().color(Theme::TEXT));
                                // standing bar: red -> grey -> gold
                                let (bar, _) = ui.allocate_exact_size(egui::vec2(46.0, 5.0), egui::Sense::hover());
                                ui.painter().rect_filled(bar, 2.0, egui::Color32::from_black_alpha(150));
                                let frac = (standing + 100) as f32 / 200.0;
                                let bar_color = if standing >= 0 {
                                    egui::Color32::from_rgb(240, 200, 120) // warm gold
                                } else {
                                    Theme::BAD // red
                                };
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_max(bar.left_top(), egui::pos2(bar.left() + bar.width() * frac, bar.bottom())),
                                    2.0, bar_color);
                                ui.label(egui::RichText::new(format!("{:+}", standing)).small().color(value_color));
                            });
                        });
                });
        }
        // F3 debug readout: exposes every gate that can kill input.
        if self.show_debug {
            egui::Area::new(egui::Id::new("debug_readout"))
                .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 26.0))
                .show(ctx, |ui| {
                    let held = self.input.keys.values().filter(|&&p| p).count();
                    let p = self.player.position;
                    let dbg = format!(
                        "ui_open={:?} locked={} playing={} health={:.1} keys_held={} pos=({:.1},{:.1},{:.1}) fps={:.0} frame_ms={:.1}",
                        self.ui_open, self.input.cursor_locked,
                        matches!(self.ui_open, UiOpen::None) && self.stats.health > 0.0,
                        self.stats.health, held, p.x, p.y, p.z, self.last_fps, 1000.0 / self.last_fps.max(0.001),
                    );
                    ui.label(egui::RichText::new(dbg).small().monospace().color(egui::Color32::from_rgb(140, 220, 255)));
                });
        }
        // hurt vignette + low-health pulse
        if self.hud_flash > 0.0 || self.stats.health <= 6.0 {
            let pulse = if self.stats.health <= 6.0 {
                0.12 + 0.08 * (ui_time(ctx) * 4.0).sin()
            } else {
                0.0
            };
            let alpha = ((self.hud_flash * 0.45 + pulse) * 255.0).min(200.0) as u8;
            let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("vignette")));
            let r = ctx.screen_rect();
            let band = 70.0;
            for (a, b) in [(r.left_top(), r.right_top()), (r.left_bottom(), r.right_bottom()),
                           (r.left_top(), r.left_bottom()), (r.right_top(), r.right_bottom())] {
                painter.line_segment([a, b], egui::Stroke::new(band, egui::Color32::from_rgba_unmultiplied(190, 30, 30, alpha / 6)));
            }
            painter.rect_stroke(r.shrink(band / 2.0), 0.0,
                egui::Stroke::new(2.0, egui::Color32::from_rgba_unmultiplied(220, 40, 40, alpha)), egui::StrokeKind::Middle);
        }
        // crosshair: opens with mining progress, hit-marker flash on attacks
        if self.ui_open == UiOpen::None && self.stats.health > 0.0 {
            let pointer = ctx.screen_rect().center();
            let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, "crosshair".into()));
            let grow = self.mining.as_ref()
                .map(|m| 2.0 + (m.progress / m.total).min(1.0) * 6.0)
                .unwrap_or(2.0);
            let c = egui::Color32::from_white_alpha(220);
            p.line_segment([pointer - egui::vec2(7.0 + grow, 0.0), pointer - egui::vec2(2.0, 0.0)], egui::Stroke::new(2.0, c));
            p.line_segment([pointer + egui::vec2(2.0, 0.0), pointer + egui::vec2(7.0 + grow, 0.0)], egui::Stroke::new(2.0, c));
            p.line_segment([pointer - egui::vec2(0.0, 7.0 + grow), pointer - egui::vec2(0.0, 2.0)], egui::Stroke::new(2.0, c));
            p.line_segment([pointer + egui::vec2(0.0, 2.0), pointer + egui::vec2(0.0, 7.0 + grow)], egui::Stroke::new(2.0, c));
            // Section 2: radial progress lives on the crosshair — mining in
            // the accent role, bow charge in the ok role
            if let Some(mining) = &self.mining {
                kit::paint_mining_reticle(&p, pointer, (mining.progress / mining.total).min(1.0), Theme::ACCENT);
            }
            if let Some(charge) = self.bow_charge {
                kit::paint_mining_reticle(&p, pointer, (charge / 1.2).min(1.0), Theme::OK);
            }
            if self.hit_flash > 0.0 {
                let a = (self.hit_flash * 255.0) as u8;
                let mark = egui::Color32::from_rgba_unmultiplied(255, 120, 90, a);
                let d = 5.0 + (1.0 - self.hit_flash) * 8.0;
                for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                    p.line_segment([
                        pointer + egui::vec2(dx * 3.0, dy * 3.0),
                        pointer + egui::vec2(dx * d, dy * d),
                    ], egui::Stroke::new(2.0, mark));
                }
            }
        }
    }


    /// Loop 343 building HUD: the strip above the hotbar — shape chips
    /// (BLOCK / SLAB / STAIRS, selected = accent fill) and the symmetry
    /// chip with the mirror plane. Clicks select; R cycles; V toggles.
    fn draw_build_hud(&mut self, ctx: &egui::Context) {
        egui::Area::new(egui::Id::new("build_hud"))
            .anchor(egui::Align2::LEFT_BOTTOM,
                egui::vec2(10.0, -(kit::HUD_BOTTOM_BAND + 8.0)))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_premultiplied(Theme::PANEL.r(), Theme::PANEL.g(), Theme::PANEL.b(), 225))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .stroke(egui::Stroke::new(1.0, Theme::BORDER))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("SHAPE").small().color(Theme::TEXT_DIM));
                            for shape in [lf_game::items::BuildShape::Block,
                                          lf_game::items::BuildShape::Slab,
                                          lf_game::items::BuildShape::Stairs] {
                                let selected = self.build_shape == shape;
                                let label = egui::RichText::new(shape.label().to_uppercase())
                                    .small().strong()
                                    .color(if selected { Theme::BG } else { Theme::TEXT_DIM });
                                let (r, resp) = ui.allocate_exact_size(
                                    egui::vec2(52.0, 20.0), egui::Sense::click());
                                ui.painter().rect_filled(r, 4.0, if selected {
                                    Theme::ACCENT
                                } else {
                                    egui::Color32::from_black_alpha(120)
                                });
                                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER,
                                    label.text().to_string(),
                                    egui::FontId::proportional(11.0),
                                    if selected { Theme::BG } else { Theme::TEXT_DIM });
                                if resp.clicked() && !selected {
                                    self.build_shape = shape;
                                    self.push_hint(&format!(
                                        "placement shape: {} (R to cycle)", shape.label()));
                                }
                            }
                            ui.separator();
                            // symmetry indicator chip
                            let (r, resp) = ui.allocate_exact_size(
                                egui::vec2(150.0, 20.0), egui::Sense::click());
                            let on = self.symmetry_plane.is_some();
                            ui.painter().rect_filled(r, 4.0, if on {
                                egui::Color32::from_rgba_premultiplied(107, 142, 35, 200)
                            } else {
                                egui::Color32::from_black_alpha(120)
                            });
                            let sym_text = match self.symmetry_plane {
                                Some(px) => format!("SYMMETRY x={:.0}", px),
                                None => "SYMMETRY OFF".to_string(),
                            };
                            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER,
                                sym_text, egui::FontId::proportional(11.0),
                                if on { Theme::BG } else { Theme::TEXT_DIM });
                            if resp.clicked() {
                                self.symmetry_plane = match self.symmetry_plane {
                                    Some(_) => None,
                                    None => Some(self.player.position.x),
                                };
                            }
                            ui.label(egui::RichText::new("R shape · V mirror")
                                .small().color(Theme::TEXT_DISABLED));
                        });
                    });
            });
    }

    /// The inventory screen (E): armor column + player portrait on the
    /// left, storage grid and hotbar on the right — the Minecraft
    /// convention (stuff you manage sits here; crafting lives one click
    /// away). Research notes: keep it one glance, no recipe clutter.
    fn draw_inventory(&mut self, ctx: &egui::Context) {
        let panel = egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(120)));
        panel.show(ctx, |ui| {
            kit::vignette(ui, 120);
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.06);
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_premultiplied(Theme::PANEL.r(), Theme::PANEL.g(), Theme::PANEL.b(), 242))
                    .corner_radius(10.0)
                    .inner_margin(18.0)
                    .stroke(egui::Stroke::new(1.0, Theme::BORDER))
                    .show(ui, |ui| {
                        ui.set_width(620.0);
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("INVENTORY")
                                .size(20.0).strong().color(Theme::TEXT));
                        });
                        ui.add_space(2.0);
                        ui.separator();
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            // --- left: portrait + armor column -------------------
                            ui.vertical(|ui| {
                                let (pr, _) = ui.allocate_exact_size(
                                    egui::vec2(SLOT_SIZE * 4.0, 150.0), egui::Sense::hover());
                                Self::paint_player_portrait(ui, pr);
                                ui.add_space(6.0);
                                for (i, label) in [36usize, 37, 38, 39].iter().zip(["head", "chest", "legs", "feet"]) {
                                    ui.horizontal(|ui| {
                                        let mut stack = self.inventory.slots[*i].clone();
                                        let mut cursor = self.cursor_stack.take();
                                        let out = slot_button(ui, &mut stack, &mut cursor, false, &self.icons);
                                        self.cursor_stack = cursor;
                                        if let Some(mut q) = out.quick_moved {
                                            quick_insert(&mut self.inventory.slots[..36], &mut q);
                                        }
                                        self.inventory.slots[*i] = stack;
                                        ui.label(egui::RichText::new(label.to_string())
                                            .small().color(Theme::TEXT_DIM));
                                    });
                                    ui.add_space(3.0);
                                }
                                // offhand
                                ui.horizontal(|ui| {
                                    let mut stack = self.inventory.slots[40].clone();
                                    let mut cursor = self.cursor_stack.take();
                                    let out = slot_button(ui, &mut stack, &mut cursor, false, &self.icons);
                                    self.cursor_stack = cursor;
                                    if let Some(mut q) = out.quick_moved {
                                        quick_insert(&mut self.inventory.slots[..36], &mut q);
                                    }
                                    self.inventory.slots[40] = stack;
                                    ui.label(egui::RichText::new("off hand")
                                        .small().color(Theme::TEXT_DIM));
                                });
                                ui.add_space(6.0);
                                let armor = lf_game::combat::worn_armor_points(&self.inventory.slots);
                                ui.label(egui::RichText::new(format!("armor  {armor}"))
                                    .strong()
                                    .color(if armor > 0 { Theme::OK } else { Theme::TEXT_DISABLED }));
                            });
                            ui.add_space(16.0);
                            // --- right: storage 3x9 + hotbar band -----------------
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("STORAGE").small().color(Theme::TEXT_DIM));
                                ui.add_space(3.0);
                                for row in 0..3 {
                                    ui.horizontal(|ui| {
                                        for col in 0..9 {
                                            let idx = 9 + row * 9 + col;
                                            let mut stack = self.inventory.slots[idx].clone();
                                            let mut cursor = self.cursor_stack.take();
                                            let out = slot_button(ui, &mut stack, &mut cursor, false, &self.icons);
                                            self.cursor_stack = cursor;
                                            if let Some(mut q) = out.quick_moved {
                                                quick_insert(&mut self.inventory.slots[..9], &mut q);
                                            }
                                            self.inventory.slots[idx] = stack;
                                        }
                                    });
                                }
                                ui.add_space(10.0);
                                ui.label(egui::RichText::new("HOTBAR").small().color(Theme::TEXT_DIM));
                                ui.add_space(3.0);
                                ui.horizontal(|ui| {
                                    for i in 0..9usize {
                                        let mut stack = self.inventory.slots[i].clone();
                                        let mut cursor = self.cursor_stack.take();
                                        let out = slot_button(ui, &mut stack, &mut cursor, i == self.hotbar_index, &self.icons);
                                        self.cursor_stack = cursor;
                                        if let Some(mut q) = out.quick_moved {
                                            quick_insert(&mut self.inventory.slots[9..36], &mut q);
                                        }
                                        self.inventory.slots[i] = stack;
                                    }
                                });
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new("1-9 select · shift-click moves stacks · right-click splits")
                                    .small().color(Theme::TEXT_DISABLED));
                            });
                        });
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui.small_button("craft by hand").clicked() {
                                self.ui_open = UiOpen::HandCraft;
                                self.play_sfx(lf_audio::Sfx::UiClick, 0.6);
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new("E / Esc close · use a crafting table for every recipe")
                                    .small().color(Theme::TEXT_DISABLED));
                            });
                        });
                    });
            });
        });
    }

    /// Painted player portrait for the inventory screen: the six-part
    /// humanoid proportions in kit colors — reads as "this is you" without
    /// a skin pipeline.
    fn paint_player_portrait(ui: &mut egui::Ui, rect: egui::Rect) {
        let bg = rect.shrink(2.0);
        ui.painter().rect_filled(bg, 8.0, egui::Color32::from_black_alpha(140));
        ui.painter().rect_stroke(bg, 8.0, egui::Stroke::new(1.0, Theme::BORDER), egui::StrokeKind::Middle);
        let (cx, base) = (rect.center().x, rect.bottom() - 18.0);
        let s = rect.height() / 190.0; // scale from the humanoid's ~1.8 units
        let body = |ui: &mut egui::Ui, cx_off: f32, y: f32, w: f32, h: f32, color: egui::Color32| {
            let r = egui::Rect::from_center_size(
                egui::pos2(cx + cx_off * s, base - y * s),
                egui::vec2(w * s, h * s));
            ui.painter().rect_filled(r, 3.0, color);
        };
        let parchment = egui::Color32::from_rgb(214, 198, 170);
        let dim = egui::Color32::from_rgb(120, 108, 92);
        let ember = Theme::ACCENT;
        body(ui, 0.0, 157.0, 46.0, 46.0, parchment);           // head
        body(ui, 0.0, 108.0, 54.0, 72.0, dim);                 // torso
        body(ui, -74.0, 112.0, 18.0, 62.0, dim);               // arms
        body(ui, 74.0, 112.0, 18.0, 62.0, dim);
        body(ui, -28.0, 38.0, 22.0, 70.0, ember);              // legs (accent = boots up)
        body(ui, 28.0, 38.0, 22.0, 70.0, ember);
    }

    /// The crafting workbench (ui-world-craft Section F): three zones over
    /// a vignette — category sidebar, recipe list, detail panel — with the
    /// player's inventory strip at the bottom so materials stay in view.
    /// `basic_only` (inventory route) shows the always-known survival set;
    /// a crafting table shows everything the player has earned.
    fn draw_workbench(&mut self, ctx: &egui::Context, basic_only: bool) {
        let catalog = build_catalog();
        let have: std::collections::HashMap<String, u16> = {
            let mut h = std::collections::HashMap::new();
            for s in self.inventory.slots.iter().take(36).flatten() {
                *h.entry(s.item_id.clone()).or_insert(0) += s.count as u16;
            }
            h
        };
        // visibility + sorting: craftable first, then partial, then
        // visible-but-unaffordable, then locked — a workbench conversation
        // starts with what you can make
        let era = self.research.era;
        let mut visible: Vec<(&CatalogEntry, bool, bool)> = Vec::new(); // (entry, can_craft, partial)
        let mut locked: Vec<&CatalogEntry> = Vec::new();
        for e in &catalog {
            let ings: Vec<String> = e.ingredients.iter().map(|(id, _)| id.clone()).collect();
            if basic_only && !kit_workbench_always(&e.output) {
                continue;
            }
            if !self.recipe_book.is_visible(&e.output, &ings, era) {
                continue;
            }
            let gate_ok = lf_game::paths::gate_for(&e.output).passes(&self.research, &self.paths);
            if !gate_ok {
                locked.push(e);
                continue;
            }
            let mut all = true;
            let mut some = false;
            for (id, n) in &e.ingredients {
                let got = have.get(id).copied().unwrap_or(0);
                if got >= *n as u16 { some = true; } else { all = false; }
            }
            visible.push((e, all && !e.ingredients.is_empty(), some && !all));
        }
        visible.sort_by(|a, b| {
            let rank = |x: &( &CatalogEntry, bool, bool )| {
                if x.1 { 0 } else if x.2 { 1 } else { 2 }
            };
            rank(a).cmp(&rank(b)).then_with(|| a.0.output.cmp(&b.0.output))
        });
        // N03 discovery filters narrow the list: category (sidebar) ×
        // station chip × filter chip × text search
        let station_of = |e: &CatalogEntry| match e.station {
            Station::Craft => 1u8,
            Station::Smelt => 2,
            Station::Alloy => 3,
            Station::Crush => 4,
        };
        let search = self.wb_search.to_lowercase();
        let matches_search = |e: &CatalogEntry| {
            search.is_empty()
                || e.output.to_lowercase().contains(&search)
                || item_def(&e.output).map(|d| d.name.to_lowercase().contains(&search)).unwrap_or(false)
        };
        let rows: Vec<(&CatalogEntry, bool, bool)> = visible.iter()
            .filter(|(e, can, _)| {
                if self.wb_station != 0 && station_of(e) != self.wb_station { return false; }
                if !matches_search(e) { return false; }
                match self.wb_filter {
                    1 => *can,
                    2 => !self.recipe_book.seen_items.contains(&e.output),
                    3 => self.recipe_book.favorites.contains(&e.output),
                    _ => true,
                }
            })
            .map(|(e, a, b)| (*e, *a, *b))
            .collect();

        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ctx, |ui| {
                kit::vignette(ui, 190);
                let screen = ctx.screen_rect();
                let lay = workbench_layout(screen);
                let p = ui.painter_at(screen);
                // N03 modal hierarchy: a strong world scrim + strongly
                // opaque framed panels — the world is subordinate, and the
                // survival HUD is not drawn beneath container screens
                // (hud_visible), so nothing duplicates.
                p.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(
                    kit::Theme::BG.r(), kit::Theme::BG.g(), kit::Theme::BG.b(), 215));
                p.text(
                    egui::Pos2::new(lay.header.left() + 8.0, lay.header.center().y),
                    egui::Align2::LEFT_CENTER,
                    if basic_only { "CRAFT — by hand" } else { "CRAFTING TABLE" },
                    egui::FontId::proportional(24.0), Theme::TEXT);
                p.text(
                    egui::Pos2::new(lay.header.right(), lay.header.center().y),
                    egui::Align2::RIGHT_CENTER,
                    "E or Esc closes",
                    egui::FontId::proportional(11.0), Theme::TEXT_DISABLED);
                if !self.craft_queue.is_empty() {
                    p.text(
                        egui::Pos2::new(lay.header.center().x, lay.header.center().y),
                        egui::Align2::CENTER_CENTER,
                        format!("queue: {} job{}", self.craft_queue.len(),
                            if self.craft_queue.len() == 1 { "" } else { "s" }),
                        egui::FontId::proportional(11.0), Theme::WARNING);
                }
                let side_in = paint_wb_panel(&p, lay.sidebar);
                let list_in = paint_wb_panel(&p, lay.list);
                let drill_detail = lay.compact && self.wb_selected.is_some();
                let detail_in = if !lay.compact {
                    Some(paint_wb_panel(&p, lay.detail))
                } else if drill_detail {
                    Some(list_in)
                } else {
                    None
                };
                // deferred primary action (button OR Enter — one owner)
                let mut craft_now: Option<(Vec<(String, u8)>, String, u8, u32)> = None;

                // ---------- Zone 1: categories (+ queue strip when wide) ----------
                ui.allocate_ui_at_rect(side_in, |ui| {
                    if lay.compact {
                        // two-pane drill-down: categories become a chip row
                        ui.horizontal_wrapped(|ui| {
                            for (i, cat) in workbench::CATEGORIES.iter().enumerate() {
                                let on = i == self.wb_category;
                                if kit::menu_link(ui, cat.label(), &format!("wb-cat-{}", i), 1.0, on, true) {
                                    self.wb_category = i;
                                    self.wb_selected = None;
                                    self.wb_qty = 1;
                                }
                                ui.add_space(6.0);
                            }
                        });
                        return;
                    }
                    ui.set_width(side_in.width());
                    for (i, cat_ref) in workbench::CATEGORIES.iter().enumerate() {
                        let cat = *cat_ref;
                        let selected = i == self.wb_category;
                        let cat_entries: Vec<&&CatalogEntry> = rows.iter()
                            .map(|(e, _, _)| e)
                            .filter(|e| workbench::categorize(&e.output) == cat)
                            .collect();
                        let craftable_n = rows.iter()
                            .filter(|(e, can, _)| workbench::categorize(&e.output) == cat && *can)
                            .count();
                        let total_n = cat_entries.len() + locked.iter()
                            .filter(|e| workbench::categorize(&e.output) == cat).count();
                        let (rect, resp) = ui.allocate_exact_size(
                            egui::vec2(side_in.width(), 30.0), egui::Sense::click());
                        if selected {
                            // left accent border — NOT a filled background
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(rect.left_top(), egui::vec2(3.0, rect.height())),
                                0.0, Theme::ACCENT);
                        }
                        let text_col = if selected { Theme::TEXT }
                            else if resp.hovered() { Theme::TEXT_BRIGHT }
                            else { egui::Color32::from_rgb(0xb5, 0xa8, 0x93) };
                        let icon_rect = egui::Rect::from_center_size(
                            egui::Pos2::new(rect.left() + 12.0, rect.center().y), egui::vec2(18.0, 18.0));
                        paint_item(ui, icon_rect, &ItemStack {
                            item_id: cat.icon_item().to_string(), count: 1,
                        }, &self.icons);
                        ui.painter().text(
                            egui::Pos2::new(rect.left() + 26.0, rect.center().y),
                            egui::Align2::LEFT_CENTER, cat.label(),
                            egui::FontId::proportional(14.0), text_col);
                        ui.painter().text(
                            egui::Pos2::new(rect.right() - 8.0, rect.center().y),
                            egui::Align2::RIGHT_CENTER,
                            format!("{}/{}", craftable_n, total_n),
                            egui::FontId::proportional(11.0),
                            if craftable_n > 0 { Theme::OK } else { Theme::TEXT_DISABLED });
                        if resp.clicked() {
                            self.wb_category = i;
                            self.wb_selected = None;
                            self.wb_qty = 1;
                        }
                    }
                    // queue strip at the sidebar's foot (N02): head status
                    // (working/blocked reason), per-job cancel, nothing reserved
                    ui.add_space(8.0);
                    if !self.craft_queue.is_empty() {
                        let status = queue_status(&self.craft_queue, &self.inventory);
                        ui.label(egui::RichText::new("QUEUE").size(10.0).color(Theme::TEXT_DIM));
                        let mut cancel: Option<usize> = None;
                        for (i, (out_id, n)) in self.craft_queue.iter().enumerate().take(3) {
                            let name = item_def(out_id).map(|d| d.name).unwrap_or(out_id.as_str());
                            let (line, col) = match (i, &status) {
                                (0, QueueStatus::Running { .. }) =>
                                    (format!("{} × {} — working", n, name), Theme::OK),
                                (0, QueueStatus::Blocked { reason, .. }) =>
                                    (format!("{} × {} — {}", n, name, reason), Theme::WARNING),
                                (0, QueueStatus::Empty) =>
                                    (format!("{} × {}", n, name), Theme::TEXT_DIM),
                                _ => (format!("{} × {} — queued", n, name), Theme::TEXT_DIM),
                            };
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(line).size(11.0).color(col));
                                if kit::menu_link(ui, "×", &format!("wb-qcancel-{}", i), 1.0, false, true) {
                                    cancel = Some(i);
                                }
                            });
                        }
                        let more = self.craft_queue.len().saturating_sub(3);
                        if more > 0 {
                            ui.label(egui::RichText::new(format!("+ {} more", more))
                                .size(10.0).color(Theme::TEXT_DISABLED));
                        }
                        if let Some(i) = cancel {
                            self.craft_queue.remove(i);
                            // cancel is free: jobs consume only at
                            // completion, so there is nothing to refund
                        }
                    }
                });

                // ---------- Zone 2: discovery + recipe list ----------
                if !drill_detail {
                    ui.allocate_ui_at_rect(list_in, |ui| {
                        ui.set_width(list_in.width());
                        let cat = workbench::CATEGORIES[self.wb_category.min(workbench::CATEGORIES.len() - 1)];
                        // search + filter chips own the top of the list
                        let mut search_buf = self.wb_search.clone();
                        let search_resp = ui.add(
                            egui::TextEdit::singleline(&mut search_buf)
                                .hint_text("search recipes…")
                                .desired_width(list_in.width() - 8.0)
                                .id(egui::Id::new("wb-search")));
                        if search_buf != self.wb_search {
                            self.wb_search = search_buf;
                        }
                        let search_focused = search_resp.has_focus();
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            for (i, label) in ["All", "Can make", "New", "★ Fav"].iter().enumerate() {
                                if kit::menu_link(ui, label, &format!("wb-filter-{}", i), 1.0,
                                    self.wb_filter == i as u8, true) {
                                    self.wb_filter = i as u8;
                                }
                                ui.add_space(6.0);
                            }
                        });
                        ui.horizontal(|ui| {
                            for (i, label) in ["Any station", "Craft", "Smelt", "Alloy", "Crush"].iter().enumerate() {
                                if kit::menu_link(ui, label, &format!("wb-station-{}", i), 1.0,
                                    self.wb_station == i as u8, true) {
                                    self.wb_station = i as u8;
                                }
                                ui.add_space(6.0);
                            }
                        });
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical().max_height((list_in.height() - 96.0).max(80.0)).show(ui, |ui| {
                            let mut list_rows: Vec<&(&CatalogEntry, bool, bool)> = rows.iter()
                                .filter(|(e, _, _)| workbench::categorize(&e.output) == cat)
                                .collect();
                            let locked_rows: Vec<&&CatalogEntry> = locked.iter()
                                .filter(|e| workbench::categorize(&e.output) == cat
                                    && matches_search(e)
                                    && (self.wb_station == 0 || station_of(e) == self.wb_station))
                                .collect();
                            if list_rows.is_empty() && locked_rows.is_empty() {
                                ui.label(egui::RichText::new(
                                    if self.wb_search.is_empty() && self.wb_filter == 0 && self.wb_station == 0 {
                                        "Nothing here yet. Gather, and the workbench will teach you."
                                    } else {
                                        "No recipes match — try another filter."
                                    })
                                    .color(Theme::TEXT_DIM).size(12.0));
                            }
                            let list_w = list_in.width();
                            for (e, can_craft, partial) in list_rows.drain(..) {
                                let name = item_def(&e.output).map(|d| d.name).unwrap_or(&e.output);
                                let selected = self.wb_selected.as_deref() == Some(e.output.as_str());
                                let (rect, resp) = ui.allocate_exact_size(
                                    egui::vec2(list_w - 8.0, 44.0), egui::Sense::click());
                                let bg = if selected {
                                    Color32::from_rgba_premultiplied(0x3d, 0x30, 0x1e, 235)
                                } else if resp.hovered() {
                                    Color32::from_rgba_premultiplied(0x2e, 0x25, 0x1a, 220)
                                } else {
                                    Color32::TRANSPARENT
                                };
                                if bg != Color32::TRANSPARENT {
                                    ui.painter().rect_filled(rect, 0.0, bg);
                                }
                                if selected {
                                    ui.painter().rect_stroke(rect, 0.0,
                                        egui::Stroke::new(1.0, Theme::BORDER), egui::StrokeKind::Middle);
                                }
                                let icon_rect = egui::Rect::from_center_size(
                                    egui::Pos2::new(rect.left() + 18.0, rect.center().y),
                                    egui::vec2(26.0, 26.0));
                                paint_item(ui, icon_rect, &ItemStack {
                                    item_id: e.output.clone(), count: 1,
                                }, &self.icons);
                                ui.painter().text(
                                    egui::Pos2::new(rect.left() + 40.0, rect.top() + 8.0),
                                    egui::Align2::LEFT_CENTER, name,
                                    egui::FontId::proportional(14.0),
                                    if *can_craft { Theme::TEXT } else { Theme::TEXT_DIM });
                                // inline material summary (top 2 by quantity)
                                let mut ings: Vec<&(String, u8)> = e.ingredients.iter().collect();
                                ings.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
                                let summary: Vec<String> = ings.iter().take(2)
                                    .map(|(id, n)| {
                                        let short = item_def(id).map(|d| d.name).unwrap_or(id);
                                        format!("{}x {}", n, short)
                                    }).collect();
                                let more = ings.len().saturating_sub(2);
                                let summary = if more > 0 {
                                    format!("{} +{}", summary.join(", "), more)
                                } else {
                                    summary.join(", ")
                                };
                                ui.painter().text(
                                    egui::Pos2::new(rect.left() + 40.0, rect.bottom() - 9.0),
                                    egui::Align2::LEFT_CENTER, summary,
                                    egui::FontId::proportional(11.0), Theme::TEXT_DIM);
                                if *can_craft {
                                    kit::paint_check(ui.painter(),
                                        egui::Pos2::new(rect.right() - 14.0, rect.center().y),
                                        Theme::OK, 1.8);
                                } else if *partial {
                                    ui.painter().text(
                                        egui::Pos2::new(rect.right() - 12.0, rect.center().y),
                                        egui::Align2::RIGHT_CENTER, "~",
                                        egui::FontId::proportional(16.0), Theme::WARNING);
                                } else {
                                    ui.painter().text(
                                        egui::Pos2::new(rect.right() - 12.0, rect.center().y),
                                        egui::Align2::RIGHT_CENTER, ".",
                                        egui::FontId::proportional(16.0), Theme::TEXT_DIM);
                                }
                                if resp.clicked() {
                                    self.wb_selected = Some(e.output.clone());
                                    self.wb_qty = 1;
                                }
                            }
                            // locked: greyed, last, no recipe shown
                            for e in locked_rows {
                                let name = item_def(&e.output).map(|d| d.name).unwrap_or(&e.output);
                                let gate = lf_game::paths::gate_for(&e.output);
                                let (rect, _resp) = ui.allocate_exact_size(
                                    egui::vec2(list_w - 8.0, 26.0), egui::Sense::hover());
                                ui.painter().text(
                                    egui::Pos2::new(rect.left() + 8.0, rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    format!("{} — locked, needs {}", name, gate.label()),
                                    egui::FontId::proportional(12.0), Theme::TEXT_DISABLED);
                            }
                        });
                        // Enter (when the search box doesn't own the key)
                        // fires the primary action on the selected recipe —
                        // exactly one owner
                        let _ = search_focused; // used below in the detail pane contract
                    });
                }

                // ---------- Zone 3: detail panel ----------
                if let Some(detail_in) = detail_in {
                    ui.allocate_ui_at_rect(detail_in, |ui| {
                        ui.set_width(detail_in.width());
                        if lay.compact {
                            // drill-down back link
                            if kit::menu_link(ui, "← back to recipes", "wb-back", 1.0, false, true) {
                                self.wb_selected = None;
                            }
                            ui.add_space(6.0);
                        }
                        let cat = workbench::CATEGORIES[self.wb_category.min(workbench::CATEGORIES.len() - 1)];
                        let selected_entry = rows.iter()
                            .find(|(e, _, _)| self.wb_selected.as_deref() == Some(e.output.as_str()))
                            .map(|(e, can, _)| (*e, *can))
                            .or_else(|| visible.iter()
                                .find(|(e, _, _)| self.wb_selected.as_deref() == Some(e.output.as_str()))
                                .map(|(e, can, _)| (*e, *can)));
                        let Some((e, can_craft)) = selected_entry else {
                            ui.add_space(detail_in.height() * 0.30);
                            ui.label(egui::RichText::new(workbench::flavor_for_or_greeting(cat))
                                .size(13.0).color(Theme::TEXT_DIM));
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Select a recipe to see details.")
                                .size(12.0).color(Theme::TEXT_DISABLED));
                            return;
                        };
                        let name = item_def(&e.output).map(|d| d.name).unwrap_or(&e.output);
                        let category = workbench::categorize(&e.output);
                        let big_icon = egui::Rect::from_min_size(
                            ui.cursor().min, egui::vec2(56.0, 56.0));
                        paint_item(ui, big_icon, &ItemStack {
                            item_id: e.output.clone(), count: 1,
                        }, &self.icons);
                        ui.painter().text(
                            egui::Pos2::new(big_icon.right() + 12.0, big_icon.center().y - 12.0),
                            egui::Align2::LEFT_CENTER, name,
                            egui::FontId::proportional(20.0), Theme::TEXT);
                        ui.painter().text(
                            egui::Pos2::new(big_icon.right() + 12.0, big_icon.center().y + 12.0),
                            egui::Align2::LEFT_CENTER,
                            format!("{} · {}", category.label(), e.station.label()),
                            egui::FontId::proportional(12.0), Theme::TEXT_DIM);
                        // favorites star (N03 discovery filter)
                        let fav = self.recipe_book.favorites.contains(&e.output);
                        let star_rect = egui::Rect::from_min_size(
                            egui::pos2(detail_in.right() - 44.0, big_icon.top()), egui::vec2(40.0, 28.0));
                        let star_resp = ui.allocate_rect(star_rect, egui::Sense::click());
                        ui.painter().text(
                            star_rect.center(), egui::Align2::CENTER_CENTER,
                            if fav { "★" } else { "☆" },
                            egui::FontId::proportional(18.0),
                            if fav { Theme::WARNING } else { Theme::TEXT_DISABLED });
                        if star_resp.clicked() {
                            if fav {
                                self.recipe_book.favorites.remove(&e.output);
                            } else {
                                self.recipe_book.favorites.insert(e.output.clone());
                            }
                        }
                        ui.add_space(64.0);
                        // flavor text between two hairlines
                        let flavor = workbench::flavor_for(&e.output);
                        ui.painter().line_segment(
                            [ui.cursor().min, ui.cursor().min + egui::vec2(ui.available_width(), 0.0)],
                            egui::Stroke::new(1.0, Theme::BORDER));
                        ui.label(egui::RichText::new(flavor).size(12.0).color(Theme::TEXT_DIM).italics());
                        ui.painter().line_segment(
                            [ui.cursor().min, ui.cursor().min + egui::vec2(ui.available_width(), 0.0)],
                            egui::Stroke::new(1.0, Theme::BORDER));
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("INGREDIENTS").size(11.0).color(Theme::TEXT_DIM));
                        let qty = self.wb_qty.max(1);
                        let mut all_available = true;
                        for (id, n) in &e.ingredients {
                            let need = (*n as u32 * qty).min(u8::MAX as u32) as u8;
                            let got = have.get(id).copied().unwrap_or(0);
                            let (mark, col) = if got as u32 >= need as u32 {
                                ("+", Theme::OK)
                            } else if got > 0 {
                                ("~", Theme::WARNING)
                            } else {
                                ("x", Theme::BAD)
                            };
                            if (got as u32) < (need as u32) {
                                all_available = false;
                            }
                            let (irect, _) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 22.0), egui::Sense::hover());
                            let ing_icon = egui::Rect::from_center_size(
                                egui::Pos2::new(irect.left() + 10.0, irect.center().y),
                                egui::vec2(16.0, 16.0));
                            paint_item(ui, ing_icon, &ItemStack {
                                item_id: id.clone(), count: 1,
                            }, &self.icons);
                            let ing_name = item_def(id).map(|d| d.name).unwrap_or(id);
                            ui.painter().text(
                                egui::Pos2::new(irect.left() + 24.0, irect.center().y),
                                egui::Align2::LEFT_CENTER,
                                format!("{}x {}", need, ing_name),
                                egui::FontId::proportional(13.0), Theme::TEXT);
                            ui.painter().text(
                                egui::Pos2::new(irect.right() - 4.0, irect.center().y),
                                egui::Align2::RIGHT_CENTER,
                                format!("{} have {}", mark, got),
                                egui::FontId::proportional(12.0), col);
                        }
                        ui.add_space(6.0);
                        ui.painter().text(
                            egui::Pos2::new(ui.cursor().min.x + 4.0, ui.cursor().min.y + 10.0),
                            egui::Align2::LEFT_CENTER,
                            format!("makes {}x {}", e.output_count as u32 * qty, name),
                            egui::FontId::proportional(14.0), Theme::TEXT);
                        ui.add_space(24.0);
                        // quantity selector: [-] [n] [+], underline links
                        ui.label(egui::RichText::new("QUANTITY").size(11.0).color(Theme::TEXT_DIM));
                        ui.horizontal(|ui| {
                            if kit::menu_link(ui, "[ - ]", "wb-minus", 1.0, false, qty > 1) {
                                self.wb_qty = (qty - 1).max(1);
                            }
                            ui.painter().text(
                                egui::Pos2::new(ui.cursor().min.x + 30.0, ui.cursor().min.y + 14.0),
                                egui::Align2::CENTER_CENTER, format!("{} ", qty),
                                egui::FontId::proportional(15.0), Theme::TEXT);
                            ui.allocate_exact_size(egui::vec2(60.0, 28.0), egui::Sense::hover());
                            if kit::menu_link(ui, "[ + ]", "wb-plus", 1.0, false, qty < 64) {
                                self.wb_qty = (qty + 1).min(64);
                            }
                            if kit::menu_link(ui, "x8", "wb-x8", 1.0, false, qty != 8) {
                                self.wb_qty = 8;
                            }
                        });
                        ui.add_space(12.0);
                        // the craft actions: exact quantity, then
                        // integer-safe craft-all (limited by materials
                        // AND output room — the engine re-verifies)
                        let all_qty = lf_game::crafting::max_batches(
                            &self.inventory, &e.ingredients, &e.output, e.output_count, 64);
                        if all_available {
                            if kit::menu_link(ui, &format!("Craft {}", qty), "wb-craft", 1.0, true, true) {
                                craft_now = Some((e.ingredients.clone(), e.output.clone(), e.output_count, qty));
                            }
                            ui.add_space(2.0);
                            let all_label = if all_qty > 0 {
                                format!("Craft All ({})", all_qty)
                            } else {
                                "Craft All — no room".to_string()
                            };
                            if kit::menu_link(ui, &all_label, "wb-craft-all", 1.0, false, all_qty > 0) {
                                craft_now = Some((e.ingredients.clone(), e.output.clone(), e.output_count, all_qty));
                            }
                            // Enter fires the primary action — but not while
                            // the search edit owns the keyboard
                            let search_owns = ui.ctx().memory(|m|
                                m.has_focus(egui::Id::new("wb-search")));
                            if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !search_owns {
                                craft_now = Some((e.ingredients.clone(), e.output.clone(), e.output_count, qty));
                            }
                        } else {
                            ui.painter().text(
                                egui::Pos2::new(ui.cursor().min.x + 8.0, ui.cursor().min.y + 14.0),
                                egui::Align2::LEFT_CENTER, "Missing materials",
                                egui::FontId::proportional(15.0), Theme::TEXT_DISABLED);
                            let first_missing: Vec<String> = e.ingredients.iter()
                                .filter(|(id, n)| {
                                    let got = self.inventory.slots.iter().take(36).flatten()
                                        .filter(|s| s.item_id == *id)
                                        .map(|s| s.count as u32).sum::<u32>();
                                    got < (*n as u32).max(1)
                                })
                                .map(|(id, _)| item_def(id).map(|d| d.name).unwrap_or(id.as_str()).to_string())
                                .collect();
                            if !first_missing.is_empty() {
                                ui.add_space(2.0);
                                ui.label(egui::RichText::new(format!("need: {}", first_missing.join(", ")))
                                    .size(11.0).color(Theme::BAD));
                            }
                        }
                        ui.add_space(4.0);
                        if kit::menu_link(ui, "Add to Queue", "wb-queue", 1.0, false, true) {
                            self.craft_queue.push((e.output.clone(), qty));
                            // nothing is reserved at enqueue (N02 rule):
                            // jobs consume exactly at completion
                            self.push_hint(&format!("queued: {} × {} — nothing reserved yet", qty, name));
                        }
                        let _ = can_craft;
                    });
                }

                // ---------- inventory strip (compact: hotbar only) ----------
                ui.allocate_ui_at_rect(lay.strip, |ui| {
                    ui.set_width(lay.strip.width());
                    let rows_n = if lay.compact { 1 } else { 4 };
                    // needed-ingredient highlight map
                    let needed: Vec<String> = self.wb_selected.as_ref().and_then(|sel| {
                        rows.iter().find(|(e, _, _)| e.output == *sel)
                            .map(|(e, _, _)| e.ingredients.iter().map(|(id, _)| id.clone()).collect())
                            .or_else(|| visible.iter().find(|(e, _, _)| e.output == *sel)
                                .map(|(e, _, _)| e.ingredients.iter().map(|(id, _)| id.clone()).collect()))
                    }).unwrap_or_default();
                    for row in 0..rows_n {
                        ui.horizontal(|ui| {
                            for col in 0..9 {
                                let idx = if row == 0 { col } else { 9 + (row - 1) * 9 + col };
                                let srect = ui.cursor();
                                let mut stack = self.inventory.slots[idx].clone();
                                let mut cursor = self.cursor_stack.take();
                                let out = slot_button(ui, &mut stack, &mut cursor,
                                    row == 0 && col == self.hotbar_index, &self.icons);
                                self.cursor_stack = cursor;
                                if let Some(mut q) = out.quick_moved {
                                    quick_insert(&mut self.inventory.slots[..36], &mut q);
                                }
                                self.inventory.slots[idx] = stack;
                                if let Some(s) = &self.inventory.slots[idx] {
                                    if needed.iter().any(|id| id == &s.item_id) {
                                        // faint accent border: this slot matters
                                        ui.painter().rect_stroke(srect, 5.0,
                                            egui::Stroke::new(1.5, Theme::ACCENT),
                                            egui::StrokeKind::Middle);
                                    }
                                }
                            }
                        });
                    }
                });

                // the deferred primary action runs exactly once, outside
                // the layout closures
                if let Some((ingredients, output, output_count, qty)) = craft_now {
                    self.craft_from_workbench(&ingredients, &output, output_count, qty);
                    self.wb_qty = 1;
                }
            });
    }

    /// The craft button's action (N02): fully transactional through the
    /// authoritative host — a blocked craft (short materials, no room)
    /// consumes nothing, records the rejection, and says exactly why. The
    /// UI enable-state is only a convenience; this re-verifies against live
    /// inventory every call, so rapid clicks and queue completions share
    /// one safe path.
    fn craft_from_workbench(&mut self, ingredients: &[(String, u8)], output: &str,
                            output_count: u8, qty: u32) {
        let receipt = self.host.craft_now(
            &mut self.inventory, ingredients.to_vec(), output.to_string(), output_count, qty,
        );
        if receipt.granted.is_some() {
            // one event set per completed craft action — never per batch
            self.quest_event(QuestEvent::Crafted(output.to_string()));
            self.onboarding.observe_crafted();
            self.play_sfx(lf_audio::Sfx::CraftDone, 0.7);
        } else if let Some(reason) = &receipt.reason {
            self.push_hint(&format!("craft blocked: {}", reason));
        }
    }

    fn draw_furnace(&mut self, ctx: &egui::Context, pos: (i32, i32, i32)) {
        let Some(BlockEntity::Furnace(mut furnace)) = self.block_entities.get(&pos).cloned() else {
            self.close_ui();
            return;
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(120)))
            .show(ctx, |ui| {
                kit::vignette(ui, 120);
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.08);
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_premultiplied(Theme::PANEL.r(), Theme::PANEL.g(), Theme::PANEL.b(), 242))
                    .corner_radius(10.0)
                    .inner_margin(16.0)
                    .stroke(egui::Stroke::new(1.0, Theme::BORDER))
                    .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("FURNACE").size(17.0).strong().color(Theme::TEXT));
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Input").small().color(Theme::TEXT_DIM));
                        let mut input = furnace.input.take();
                        let mut cursor = self.cursor_stack.take();
                        let out = slot_button(ui, &mut input, &mut cursor, false, &self.icons);
                        if let Some(mut q) = out.quick_moved {
                            quick_insert(&mut self.inventory.slots[..36], &mut q);
                        }
                        furnace.input = input;
                        self.cursor_stack = cursor;
                        // painted flame (burn remaining)
                        let flame = if furnace.burn_total > 0.0 {
                            (furnace.burn_left / furnace.burn_total).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let (frect, _) = ui.allocate_exact_size(egui::vec2(SLOT_SIZE, 22.0), egui::Sense::hover());
                        let p = ui.painter();
                        p.rect_filled(frect, 3.0, egui::Color32::from_black_alpha(170));
                        let fh = frect.height() * flame;
                        p.rect_filled(egui::Rect::from_min_max(
                            egui::pos2(frect.left() + 2.0, frect.bottom() - 2.0 - fh),
                            egui::pos2(frect.right() - 2.0, frect.bottom() - 2.0)), 3.0,
                            egui::Color32::from_rgb(250, 150, 50));
                        for i in 0..3 {
                            let t = i as f32 / 3.0;
                            let fy = frect.bottom() - 3.0 - (fh * (0.4 + 0.6 * t)).min(fh);
                            p.circle_filled(egui::pos2(frect.left() + 8.0 + i as f32 * 14.0, fy), 3.0,
                                egui::Color32::from_rgb(255, 210, 90));
                        }
                        ui.label(egui::RichText::new("Fuel").small().color(Theme::TEXT_DIM));
                        let mut fuel = furnace.fuel.take();
                        let mut cursor = self.cursor_stack.take();
                        let out = slot_button(ui, &mut fuel, &mut cursor, false, &self.icons);
                        if let Some(mut q) = out.quick_moved {
                            quick_insert(&mut self.inventory.slots[..36], &mut q);
                        }
                        furnace.fuel = fuel;
                        self.cursor_stack = cursor;
                    });
                    ui.add_space(8.0);
                    // smelt progress arrow
                    let frac = (furnace.progress / lf_game::smelting::SMELT_TIME).clamp(0.0, 1.0);
                    let (arect, _) = ui.allocate_exact_size(egui::vec2(70.0, 20.0), egui::Sense::hover());
                    let p = ui.painter();
                    p.rect_filled(arect, 3.0, egui::Color32::from_black_alpha(170));
                    p.rect_filled(egui::Rect::from_min_size(arect.min + egui::vec2(2.0, 2.0),
                        egui::vec2((arect.width() - 4.0) * frac, arect.height() - 4.0)), 3.0, Theme::ACCENT);
                    let tip = arect.right_center() + egui::vec2(6.0, 0.0);
                    p.add(egui::Shape::convex_polygon(vec![
                        tip + egui::vec2(-6.0, -8.0), tip + egui::vec2(6.0, 0.0), tip + egui::vec2(-6.0, 8.0)],
                        Theme::ACCENT, egui::Stroke::NONE));
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Output").small().color(Theme::TEXT_DIM));
                        let mut output = furnace.output.take();
                        let mut cursor = self.cursor_stack.take();
                        let out = slot_button(ui, &mut output, &mut cursor, false, &self.icons);
                        if let Some(mut q) = out.quick_moved {
                            quick_insert(&mut self.inventory.slots[..36], &mut q);
                        }
                        furnace.output = output;
                        self.cursor_stack = cursor;
                    });
                });
                ui.add_space(6.0);
                self.draw_storage_rows(ui);
                    });
                });
            });
        self.block_entities.insert(pos, BlockEntity::Furnace(furnace));
    }

    fn draw_chest(&mut self, ctx: &egui::Context, pos: (i32, i32, i32)) {
        let mut chest_slots = match self.block_entities.get(&pos).cloned() {
            Some(BlockEntity::Chest { slots }) => slots,
            _ => vec![None; 27],
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(120)))
            .show(ctx, |ui| {
                kit::vignette(ui, 120);
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.08);
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_premultiplied(Theme::PANEL.r(), Theme::PANEL.g(), Theme::PANEL.b(), 242))
                    .corner_radius(10.0)
                    .inner_margin(16.0)
                    .stroke(egui::Stroke::new(1.0, Theme::BORDER))
                    .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("CHEST").size(17.0).strong().color(Theme::TEXT));
                });
                ui.add_space(8.0);
                for row in 0..3 {
                    ui.horizontal(|ui| {
                        for col in 0..9 {
                            let idx = row * 9 + col;
                            let mut stack = chest_slots[idx].clone();
                            let mut cursor = self.cursor_stack.take();
                            let out = slot_button(ui, &mut stack, &mut cursor, false, &self.icons);
                            self.cursor_stack = cursor;
                            if let Some(mut q) = out.quick_moved {
                                // chest -> inventory
                                quick_insert(&mut self.inventory.slots[..36], &mut q);
                            }
                            chest_slots[idx] = stack;
                        }
                    });
                }
                ui.add_space(6.0);
                // player rows; shift-click sends stacks into the chest
                for row in 0..3 {
                    ui.horizontal(|ui| {
                        for col in 0..9 {
                            let idx = 9 + row * 9 + col;
                            let mut stack = self.inventory.slots[idx].clone();
                            let mut cursor = self.cursor_stack.take();
                            let out = slot_button(ui, &mut stack, &mut cursor, false, &self.icons);
                            self.cursor_stack = cursor;
                            if let Some(mut q) = out.quick_moved {
                                quick_insert(&mut chest_slots, &mut q);
                            }
                            self.inventory.slots[idx] = stack;
                        }
                    });
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    for i in 0..9 {
                        let mut stack = self.inventory.slots[i].clone();
                        let mut cursor = self.cursor_stack.take();
                        let out = slot_button(ui, &mut stack, &mut cursor, i == self.hotbar_index, &self.icons);
                        self.cursor_stack = cursor;
                        if let Some(mut q) = out.quick_moved {
                            quick_insert(&mut chest_slots, &mut q);
                        }
                        self.inventory.slots[i] = stack;
                    }
                });
                    });
                });
            });
        self.block_entities.insert(pos, BlockEntity::Chest { slots: chest_slots });
    }

    /// Player storage + hotbar below a container screen; shift-click sends
    /// slots across the inventory (storage <-> hotbar).
    fn draw_storage_rows(&mut self, ui: &mut egui::Ui) {
        // armor row (loop 329): head/chest/legs/feet + the total worn readout
        ui.horizontal(|ui| {
            for (i, label) in [36, 37, 38, 39].iter().zip(["head", "chest", "legs", "feet"]) {
                ui.vertical(|ui| {
                    let mut stack = self.inventory.slots[*i].clone();
                    let mut cursor = self.cursor_stack.take();
                    let out = slot_button(ui, &mut stack, &mut cursor, false, &self.icons);
                    self.cursor_stack = cursor;
                    if let Some(mut q) = out.quick_moved {
                        quick_insert(&mut self.inventory.slots[..36], &mut q);
                    }
                    self.inventory.slots[*i] = stack;
                    let (r, _) = ui.allocate_exact_size(egui::vec2(SLOT_SIZE, 11.0), egui::Sense::hover());
                    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, label,
                        egui::FontId::proportional(9.0), Theme::TEXT_DISABLED);
                });
                ui.add_space(4.0);
            }
            let armor = lf_game::combat::worn_armor_points(&self.inventory.slots);
            let (r, _) = ui.allocate_exact_size(egui::vec2(110.0, SLOT_SIZE), egui::Sense::hover());
            ui.painter().text(r.left_center(), egui::Align2::LEFT_CENTER,
                format!("armor {}", armor),
                egui::FontId::proportional(13.0),
                if armor > 0 { Theme::OK } else { Theme::TEXT_DISABLED });
        });
        ui.add_space(4.0);
        for row in 0..3 {
            ui.horizontal(|ui| {
                for col in 0..9 {
                    let idx = 9 + row * 9 + col;
                    let mut stack = self.inventory.slots[idx].clone();
                    let mut cursor = self.cursor_stack.take();
                    let out = slot_button(ui, &mut stack, &mut cursor, false, &self.icons);
                    self.cursor_stack = cursor;
                    if let Some(mut q) = out.quick_moved {
                        quick_insert(&mut self.inventory.slots[..9], &mut q);
                    }
                    self.inventory.slots[idx] = stack;
                }
            });
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            for i in 0..9 {
                let mut stack = self.inventory.slots[i].clone();
                let mut cursor = self.cursor_stack.take();
                let out = slot_button(ui, &mut stack, &mut cursor, i == self.hotbar_index, &self.icons);
                self.cursor_stack = cursor;
                if let Some(mut q) = out.quick_moved {
                    quick_insert(&mut self.inventory.slots[9..36], &mut q);
                }
                self.inventory.slots[i] = stack;
            }
        });
    }

    /// Quest log (loop 329 redesign): a centered kit panel with two tabs —
    /// active quests as cards with per-objective progress bars, and the
    /// full chronicle — replacing the raw default-styled window that used
    /// to hang off the top-left corner.
    fn draw_quest_log(&mut self, ctx: &egui::Context) {
        let reveal = kit::ease_out_cubic((self.menu_reveal / 0.35).clamp(0.0, 1.0));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(150)))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    kit::center_vertically(ui, 480.0);
                    kit::slide_panel(ui, reveal, |ui| {
                        ui.set_width(600.0);
                        ui.vertical(|ui| {
                            ui.add_space(12.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Journal").size(24.0).color(Theme::TEXT));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new("J or Esc to close").small().color(Theme::TEXT_DISABLED));
                                });
                            });
                            ui.add_space(4.0);
                            ui.painter().line_segment(
                                [ui.cursor().min, ui.cursor().min + egui::vec2(ui.available_width(), 0.0)],
                                egui::Stroke::new(1.0, Theme::BORDER));
                            ui.add_space(8.0);
                            // tabs: Quests (n active) | Chronicle
                            let tabs = [
                                format!("Quests ({})", self.quest_log.quests.iter().filter(|q| !q.completed).count()),
                                "Chronicle".to_string(),
                            ];
                            ui.horizontal(|ui| {
                                for (i, label) in tabs.iter().enumerate() {
                                    let on = self.quest_tab == i;
                                    if kit::menu_link(ui, label, &format!("journal-tab-{}", i), 1.0, on, true) {
                                        self.quest_tab = i;
                                    }
                                    ui.add_space(10.0);
                                }
                            });
                            ui.add_space(6.0);
                            if self.quest_tab == 0 {
                                egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                                    if self.quest_log.quests.is_empty() {
                                        ui.label(egui::RichText::new(
                                            "No quests yet — the world will find you.").color(Theme::TEXT_DIM));
                                    }
                                    for quest in &self.quest_log.quests {
                                        let title_col = if quest.completed { Theme::OK } else { Theme::TEXT };
                                        egui::Frame::new()
                                            .fill(if quest.completed { Color32::from_rgba_premultiplied(0x2a, 0x30, 0x1e, 210) }
                                                  else { Color32::from_rgba_premultiplied(0x24, 0x1c, 0x14, 220) })
                                            .stroke(egui::Stroke::new(1.0, if quest.completed { Theme::OK } else { Theme::BORDER }))
                                            .corner_radius(0.0)
                                            .inner_margin(10.0)
                                            .show(ui, |ui| {
                                                ui.set_min_width(ui.available_width());
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(&quest.title).size(17.0).color(title_col).strong());
                                                    if let Some(f) = &quest.faction {
                                                        if let Some(fdef) = self.lore_data.faction(f) {
                                                            let col = egui::Color32::from_rgb(fdef.color[0], fdef.color[1], fdef.color[2]);
                                                            ui.label(egui::RichText::new(format!("{} {}", fdef.symbol, fdef.short_name)).small().color(col));
                                                        }
                                                    }
                                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                        let done = quest.objectives.iter().filter(|o| o.completed).count();
                                                        ui.label(egui::RichText::new(format!(
                                                            "Act {} · {}/{}", quest.act, done, quest.objectives.len()))
                                                            .small().color(Theme::TEXT_DIM));
                                                        if quest.completed {
                                                            let reward = if quest.standing_reward != 0 {
                                                                format!(" · +{} {}", quest.standing_reward, quest.faction.clone().unwrap_or_default())
                                                            } else { String::new() };
                                                            ui.label(egui::RichText::new(format!("complete{}", reward)).small().color(Theme::OK));
                                                        }
                                                    });
                                                });
                                                ui.label(egui::RichText::new(&quest.description).small().color(Theme::TEXT_DIM));
                                                ui.add_space(4.0);
                                                for obj in &quest.objectives {
                                                    let frac = (obj.progress as f32 / obj.count.max(1) as f32).clamp(0.0, 1.0);
                                                    ui.horizontal(|ui| {
                                                        let mark = if obj.completed { "✓" } else { "·" };
                                                        let (r, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                                                        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, mark,
                                                            egui::FontId::proportional(13.0),
                                                            if obj.completed { Theme::OK } else { Theme::TEXT_DISABLED });
                                                        ui.label(egui::RichText::new(&obj.target).size(13.0)
                                                            .color(if obj.completed { Theme::TEXT_DIM } else { Theme::TEXT }));
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            ui.label(egui::RichText::new(format!("{}/{}", obj.progress.min(obj.count), obj.count))
                                                                .small().monospace()
                                                                .color(if obj.completed { Theme::OK } else { Theme::TEXT_DIM }));
                                                        });
                                                    });
                                                    let (bar, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 4.0), egui::Sense::hover());
                                                    ui.painter().rect_filled(bar, 2.0, egui::Color32::from_black_alpha(160));
                                                    let w = bar.width() * frac;
                                                    if w > 1.0 {
                                                        let fill = if obj.completed { Theme::OK } else { Theme::ACCENT };
                                                        ui.painter().rect_filled(egui::Rect::from_min_size(bar.min, egui::vec2(w, 4.0)), 2.0, fill);
                                                    }
                                                    ui.add_space(2.0);
                                                }
                                            });
                                        ui.add_space(6.0);
                                    }
                                });
                            } else {
                                // chronicle tab
                                if self.chronicle.is_empty() {
                                    ui.label(egui::RichText::new(
                                        "The chronicle is empty — every milestone will be inked here.").color(Theme::TEXT_DIM));
                                } else {
                                    let md = lf_chronicle::SagaGenerator::export_markdown(&self.chronicle);
                                    egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                                        ui.set_min_width(ui.available_width());
                                        ui.label(egui::RichText::new(md).small().monospace().color(Theme::TEXT));
                                    });
                                }
                            }
                            ui.add_space(10.0);
                        });
                    });
                });
            });
    }

    /// The title screen (ui-world-craft Section A): the live world render
    /// shows through, a radial vignette sinks the edges into `#1a1410`, the
    /// LOREFORGE logotype hangs top-left with the tagline under it, the
    /// menu is a left-hand column of underline-on-hover links, and the
    /// version + preview seed sit in the bottom-right corner. No panels,
    /// no glow, no drop shadows — the logotype stands on its own.
    fn draw_title(&mut self, ctx: &egui::Context) {
        let t = self.menu_reveal;
        let screen = ctx.screen_rect();
        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ctx, |ui| {
                // vignette first, content above it
                kit::vignette(ui, 200);
                // logotype: top-left at 10% across, ~1/6 of the screen tall,
                // warm off-white — no glow, no shadow (A1)
                let left_x = screen.left() + screen.width() * 0.10;
                let reveal = kit::ease_out_cubic((t / 0.7).clamp(0.0, 1.0));
                let logo_size = (screen.height() / 7.5).clamp(48.0, 104.0);
                let logo_y = screen.top() + screen.height() * 0.16;
                let alpha = ((255.0 * reveal) as u8).min(255);
                let painter = ui.painter_at(screen);
                painter.text(
                    egui::Pos2::new(left_x, logo_y),
                    egui::Align2::LEFT_CENTER,
                    "LOREFORGE",
                    egui::FontId::proportional(logo_size),
                    egui::Color32::from_rgba_premultiplied(
                        kit::Theme::TEXT.r(), kit::Theme::TEXT.g(), kit::Theme::TEXT.b(), alpha),
                );
                // tagline: single line, muted, directly under the logotype
                let sub_r = kit::ease_out_cubic(((t - 0.4) / 0.6).clamp(0.0, 1.0));
                painter.text(
                    egui::Pos2::new(left_x + 4.0, logo_y + logo_size * 0.62),
                    egui::Align2::LEFT_CENTER,
                    "Build. Rule. Endure.",
                    egui::FontId::proportional((logo_size * 0.22).clamp(13.0, 22.0)),
                    egui::Color32::from_rgba_premultiplied(
                        kit::Theme::TEXT_DIM.r(), kit::Theme::TEXT_DIM.g(), kit::Theme::TEXT_DIM.b(),
                        ((230.0 * sub_r) as u8).min(255)),
                );

                // menu: left-aligned column at 10% across, seated in the
                // 55-70% height band so the logotype keeps its room (A2).
                // Button labels are short but not uniformly so — width comes
                // from the text, never a fixed grid.
                let col_x = screen.left() + screen.width() * 0.10;
                let col_top = screen.top() + screen.height() * 0.55;
                let row_h = (screen.height() * 0.052).clamp(34.0, 46.0);
                let items = [
                    "New World",
                    "Load World",
                    "Multiplayer",
                    "Settings",
                    "Quit",
                ];
                ui.vertical(|ui| {
                    ui.allocate_space(egui::vec2(col_x - screen.left(), col_top - screen.top()));
                    for (i, label) in items.iter().enumerate() {
                        let r = ((t - 0.7 - i as f32 * 0.09) / 0.4).clamp(0.0, 1.0);
                        let clicked = ui
                            .allocate_ui_at_rect(
                                egui::Rect::from_min_size(
                                    egui::Pos2::new(col_x, col_top + i as f32 * row_h),
                                    egui::vec2(screen.width() * 0.3, row_h),
                                ),
                                |ui| kit::menu_link(ui, label, label, r, false, true),
                            )
                            .inner;
                        if clicked {
                            match *label {
                                "New World" => {
                                    self.open_new_world_screen();
                                }
                                "Load World" => {
                                    self.ui_open = UiOpen::Slots;
                                    self.menu_reveal = 0.0;
                                }
                                "Multiplayer" => {
                                    self.ui_open = UiOpen::Multiplayer;
                                    self.menu_reveal = 0.0;
                                }
                                "Settings" => {
                                    self.ui_open = UiOpen::Settings;
                                    self.settings_from_title = true;
                                    self.menu_reveal = 0.0;
                                }
                                "Quit" => self.quit_requested = true,
                                _ => {}
                            }
                        }
                    }
                });
            });
        // version + preview seed, bottom-right, always visible (A3 + B1):
        // the same version string that derives the preview world seed.
        egui::Area::new(egui::Id::new("title_version"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -12.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.set_max_width(240.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        ui.label(
                            egui::RichText::new(format!("LOREFORGE v{}", env!("CARGO_PKG_VERSION")))
                                .size(11.0)
                                .color(kit::Theme::TEXT_DIM),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Seed: {}",
                                format_seed(lf_worldgen::preview::version_preview_seed())
                            ))
                            .size(11.0)
                            .color(kit::Theme::TEXT_DIM),
                        );
                    });
                });
            });
    }

    fn draw_pause(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(130)))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    // true two-axis centering (was a fixed 18% top guess)
                    kit::center_vertically(ui, 430.0);
                    let t = self.menu_reveal;
                    kit::slide_panel(ui, (t / 0.5).clamp(0.0, 1.0), |ui| {
                        ui.set_width(360.0);
                        ui.vertical_centered(|ui| {
                            ui.add_space(14.0);
                            ui.label(egui::RichText::new("Paused").size(34.0).color(Theme::TEXT).strong());
                            ui.add_space(16.0);
                            if kit::menu_button(ui, "Resume", ((t - 0.15) / 0.4).clamp(0.0, 1.0), true) {
                                self.close_ui();
                            }
                            ui.add_space(6.0);
                            if kit::menu_button(ui, "Save Now", ((t - 0.20) / 0.4).clamp(0.0, 1.0), false) {
                                self.save_world();
                            }
                            ui.add_space(6.0);
                            if kit::menu_button(ui, "Load Game", ((t - 0.25) / 0.4).clamp(0.0, 1.0), false) {
                                self.ui_open = UiOpen::Slots;
                                self.menu_reveal = 0.0;
                            }
                            ui.add_space(6.0);
                            if kit::menu_button(ui, "Settings", ((t - 0.30) / 0.4).clamp(0.0, 1.0), false) {
                                self.ui_open = UiOpen::Settings;
                                self.settings_from_title = false;
                                self.menu_reveal = 0.0;
                            }
                            ui.add_space(6.0);
                            if kit::menu_button(ui, "Quit to Title", ((t - 0.35) / 0.4).clamp(0.0, 1.0), false) {
                                self.save_world();
                                self.ui_open = UiOpen::Title;
                                self.menu_reveal = 0.0;
                                self.title_show_new = false;
                            }
                            ui.add_space(6.0);
                            if kit::menu_button(ui, "Quit Game", ((t - 0.40) / 0.4).clamp(0.0, 1.0), false) {
                                self.quit_requested = true;
                            }
                            ui.add_space(14.0);
                            ui.label(egui::RichText::new("E inventory · M map · ` console · K tech tree · J quests · T chat · F2 shot · F3 debug")
                                .small().color(Theme::TEXT_DIM));
                            ui.add_space(12.0);
                        });
                    });
                });
            });
    }

    /// Settings screen (tabbed, kit-styled, drives the engine live).
    /// Leave the settings screen the way the player entered it: back to the
    /// title screen when opened from there, otherwise resume play. Both the
    /// Back button and Esc route through here (doc 02 first-launch audit —
    /// Back used to drop a title-screen player straight into the world).
    pub fn close_settings(&mut self) {
        if self.settings_from_title {
            self.ui_open = UiOpen::Title;
            self.menu_reveal = 0.0;
        } else {
            self.ui_open = UiOpen::None;
            if self.stats.health > 0.0 {
                self.lock_cursor();
            }
        }
    }

    fn draw_settings(&mut self, ctx: &egui::Context) {
        let t = self.menu_reveal;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(110)))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    kit::center_vertically(ui, 400.0);
                    kit::slide_panel(ui, (t / 0.5).clamp(0.0, 1.0), |ui| {
                        ui.set_width(560.0);
                        ui.horizontal(|ui| {
                            // left sidebar: hover-underline categories (the
                            // MAIN_MENU_REDESIGN brief spec — settings
                            // navigation is the same species as the menu)
                            ui.vertical(|ui| {
                                ui.add_space(10.0);
                                ui.label(egui::RichText::new("Settings").size(24.0).color(Theme::TEXT));
                                ui.add_space(10.0);
                                for (i, label) in ["Video", "Interface", "Audio", "Controls", "Gameplay"].iter().enumerate() {
                                    let on = self.settings_tab == i;
                                    if kit::menu_link(ui, label, &format!("settings-{}", label), 1.0, on, true) {
                                        self.settings_tab = i;
                                    }
                                    ui.add_space(2.0);
                                }
                                ui.add_space(12.0);
                                if kit::menu_link(ui, "Back", "settings-back", 1.0, false, true) {
                                    self.close_settings();
                                }
                            });
                            ui.add_space(12.0);
                            // 1px warm divider between sidebar and content
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 320.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 0.0, Theme::BORDER);
                            ui.add_space(12.0);
                            ui.vertical(|ui| {
                                ui.add_space(10.0);
                                match self.settings_tab {
                                    0 => self.settings_video(ui),
                                    1 => self.settings_interface(ui),
                                    2 => self.settings_audio(ui),
                                    3 => self.settings_controls(ui),
                                    _ => self.settings_gameplay(ui),
                                }
                            });
                        });
                    });
                });
            });
    }

    fn settings_video(&mut self, ui: &mut egui::Ui) {
        kit::section_header(ui, "Video", 1.0);
        ui.add_space(6.0);
        let s = &mut self.settings;
        kit::setting_slider(ui, "Field of view", &mut s.fov_degrees, (50.0, 110.0), &|v| format!("{:.0}°", v));
        let mut vd = s.view_distance as f32;
        kit::setting_slider(ui, "View distance", &mut vd, (3.0, 8.0), &|v| format!("{} chunks", v as i32));
        s.view_distance = vd as i32;
        ui.add_space(6.0);
        kit::toggle(ui, "Clouds", &mut s.clouds);
        kit::toggle(ui, "Weather particles", &mut s.particles);
        ui.add_space(8.0);
        kit::section_header(ui, "Ray Tracing", 1.0);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Mode").color(Theme::TEXT));
            if ui.button(egui::RichText::new(format!("{}  (cycle)", s.rt_mode.label())).color(Theme::ACCENT)).clicked() {
                s.rt_mode = s.rt_mode.next();
            }
        });
        ui.label(egui::RichText::new(match s.rt_mode {
            RtMode::Off => "classic raster renderer",
            RtMode::Captures => "press R in game to path-trace a frame",
            RtMode::Live => "live path-traced view (GPU heavy)",
        }).small().color(Theme::TEXT_DIM));
        if s.rt_mode == RtMode::Live {
            kit::setting_slider(ui, "RT internal scale", &mut s.rt_scale, (0.1, 0.5), &|v| format!("{:.0}%", v * 100.0));
        }
        ui.add_space(8.0);
        kit::section_header(ui, "Quality preset", 1.0);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let tiers = [
                ("Low", crate::Quality::Low),
                ("Medium", crate::Quality::Medium),
                ("High", crate::Quality::High),
                ("Path-Traced", crate::Quality::PathTraced),
            ];
            for (label, tier) in tiers {
                let on = self.settings.quality == match tier {
                    crate::Quality::Low => 0,
                    crate::Quality::Medium => 1,
                    crate::Quality::High => 2,
                    crate::Quality::PathTraced => 3,
                };
                if ui.add(egui::Button::new(egui::RichText::new(label)
                    .color(if on { Theme::ACCENT } else { Theme::TEXT_DIM }))).clicked() {
                    self.settings.apply_quality(tier);
                }
            }
        });
    }

    fn settings_interface(&mut self, ui: &mut egui::Ui) {
        kit::section_header(ui, "Interface", 1.0);
        ui.add_space(6.0);
        let s = &mut self.settings;
        kit::toggle(ui, "Show minimap", &mut s.show_minimap);
        kit::toggle(ui, "Rotate minimap with view", &mut s.rotate_minimap);
        kit::setting_slider(ui, "Minimap zoom", &mut s.minimap_zoom, (0.5, 3.0), &|v| format!("{:.1}x", v));
        kit::setting_slider(ui, "UI scale", &mut s.ui_scale, (0.7, 1.6), &|v| format!("{:.0}%", v * 100.0));
        ui.label(egui::RichText::new("minimap top-right · M opens the world map · shift-click slots to quick-move")
            .small().color(Theme::TEXT_DIM));
    }

    fn settings_audio(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.settings;
        kit::section_header(ui, "Audio", 1.0);
        ui.add_space(6.0);
        kit::setting_slider(ui, "Master volume", &mut s.volume_master, (0.0, 1.0), &|v| format!("{:.0}%", v * 100.0));
        kit::setting_slider(ui, "SFX volume", &mut s.volume_sfx, (0.0, 1.0), &|v| format!("{:.0}%", v * 100.0));
        kit::setting_slider(ui, "Music volume", &mut s.volume_music, (0.0, 1.0), &|v| format!("{:.0}%", v * 100.0));
        ui.add_space(8.0);
        ui.label(egui::RichText::new("drives the procedural break/place sound engine live")
            .small().color(Theme::TEXT_DIM));
    }

    /// Step 13: click an action, press a key. Digits 1-9 and Escape stay
    /// fixed; bindings persist in ClientSave via Settings.keymap_pairs.
    fn settings_controls(&mut self, ui: &mut egui::Ui) {
        kit::section_header(ui, "Key bindings", 1.0);
        ui.add_space(6.0);
        ui.label(egui::RichText::new("click a key, then press the new binding. hotbar digits 1-9 and Escape stay fixed.").small().color(Theme::TEXT_DIM));
        ui.add_space(8.0);
        egui::Grid::new("controls_grid").num_columns(2).spacing([190.0, 4.0]).show(ui, |ui| {
            for action in crate::input::Action::ALL {
                ui.label(egui::RichText::new(action.label()).small().color(Theme::TEXT));
                if self.rebind_capture == Some(action) {
                    let waiting = egui::Button::new(egui::RichText::new("press a key…").color(Theme::ACCENT))
                        .min_size(egui::vec2(110.0, 20.0));
                    if ui.add(waiting).clicked() {
                        self.rebind_capture = None; // click again cancels
                    }
                } else {
                    let name = format!("{:?}", self.keymap.key(action));
                    if ui.add(egui::Button::new(name).min_size(egui::vec2(110.0, 20.0))).clicked() {
                        self.rebind_capture = Some(action);
                    }
                }
                ui.end_row();
            }
        });
    }

    fn settings_gameplay(&mut self, ui: &mut egui::Ui) {
        // N01: first-minute tutorial controls — hide the card or walk the
        // whole tutorial again (state is persisted with the slot).
        kit::section_header(ui, "Tutorial", 1.0);
        ui.add_space(4.0);
        let mut hints = !self.onboarding.dismissed;
        if kit::toggle(ui, "Show first-minute hints", &mut hints) {
            self.onboarding.dismissed = !hints;
        }
        ui.add_space(4.0);
        if ui.button(egui::RichText::new("Restart tutorial").color(Theme::ACCENT)).clicked() {
            self.onboarding.reset();
            self.push_hint("tutorial restarted — walk, look, gather, craft, build");
        }
        ui.add_space(8.0);
        let s = &mut self.settings;
        kit::section_header(ui, "Gameplay", 1.0);
        ui.add_space(6.0);
        kit::setting_slider(ui, "Mouse sensitivity", &mut s.sensitivity, (0.0005, 0.01), &|v| format!("{:.1}", v * 1000.0));
        kit::toggle(ui, "Invert mouse Y", &mut s.invert_y);
        kit::toggle(ui, "Show FPS", &mut s.show_fps);
    }

    /// Loop 343: the kit panel shell every station/menu screen uses —
    /// vignette + dark wash + framed panel + title (the loop-341 furnace
    /// conversion, extracted so the whole UI speaks one design language).
    /// The body gets a vertical scroll for tall content.
    fn kit_shell<R>(
        ctx: &egui::Context,
        title: &str,
        width: f32,
        body: impl FnOnce(&mut egui::Ui) -> R,
    ) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(120)))
            .show(ctx, |ui| {
                kit::vignette(ui, 120);
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.05);
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgba_premultiplied(Theme::PANEL.r(), Theme::PANEL.g(), Theme::PANEL.b(), 242))
                        .corner_radius(10.0)
                        .inner_margin(16.0)
                        .stroke(egui::Stroke::new(1.0, Theme::BORDER))
                        .show(ui, |ui| {
                            ui.set_width(width);
                            ui.vertical_centered(|ui| {
                                ui.label(egui::RichText::new(title).size(17.0).strong().color(Theme::TEXT));
                            });
                            ui.add_space(8.0);
                            egui::ScrollArea::vertical()
                                .max_height(ui.available_height().max(80.0))
                                .show(ui, |ui| body(ui));
                        });
                });
            });
    }

    fn draw_trade(&mut self, ctx: &egui::Context, index: usize) {
        let Some(villager) = self.villagers.get(index).cloned() else {
            self.close_ui();
            return;
        };
        // lore-and-visuals D1: the dialogue layer + standing gates. The
        // faction line for the current standing heads the window; hostile
        // standing refuses trading; friendly standing gets a 10% discount.
        let archetype = villager.archetype.clone();
        let faction = villager.faction.clone().unwrap_or_default();
        let standing = self.standings.get(&faction);
        let refuses = !faction.is_empty() && self.standings.refuses_trade(&faction);
        let friendly = !faction.is_empty() && self.standings.offers_bonus_trade(&faction);
        let dialogue: Option<String> = archetype.as_deref().and_then(|a| {
            let biome = self.map.biome_at(self.player.position.x as i32, self.player.position.z as i32);
            let biome_key = format!("{:?}", biome);
            let ctx = lf_lore::ConditionCtx {
                standings: Some(&self.standings),
                biome: Some(&biome_key),
                ..Default::default()
            };
            self.lore_data.dialogue_for(a, &ctx).map(|n| n.text.clone())
        });
        let hire_info = archetype.as_deref().and_then(|a| self.lore_data.villager_archetype(a))
            .filter(|a| a.hireable)
            .map(|a| (a.hire_standing, a.hire_fee.clone(), a.companion_form.clone()));
        Self::kit_shell(ctx, &format!("{} — {}", villager.name, crate::factions::job_label(villager.job)), 620.0, |ui| {
                // faction standing chip + dialogue line
                if let Some(fdef) = self.lore_data.faction(&faction) {
                    let color = egui::Color32::from_rgb(fdef.color[0], fdef.color[1], fdef.color[2]);
                    let polarity = if standing > 15 { Theme::OK } else if standing < -15 { Theme::BAD } else { Theme::TEXT_DIM };
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                        ui.painter().rect_filled(r, 2.0, color);
                        ui.label(egui::RichText::new(format!("{} {}", fdef.symbol, fdef.short_name)).color(color));
                        ui.label(egui::RichText::new(format!("standing {}", standing)).color(polarity));
                    });
                    ui.add_space(2.0);
                }
                if let Some(line) = &dialogue {
                    ui.label(egui::RichText::new(format!("\u{201C}{}\u{201D}", line)).italics().color(Theme::TEXT));
                    ui.add_space(6.0);
                }
                if refuses {
                    ui.label(egui::RichText::new("They will not trade with you.").color(Theme::BAD));
                    ui.separator();
                    ui.label(egui::RichText::new("Esc to close").small());
                    return;
                }
                // every completed trade nudges standing (10+ items traded = +2)
                let mut traded_items = 0u16;
                for (give, give_n, get, get_n) in trade_offers(villager.job) {
                    // friendly discount: round the price down ~10%
                    let give_n_eff = if friendly { (*give_n as f32 * 0.9).ceil() as u8 } else { *give_n };
                    let have = self.inventory.slots.iter()
                        .filter_map(|s| s.as_ref())
                        .filter(|s| s.item_id == *give)
                        .map(|s| s.count as u16)
                        .sum::<u16>();
                    let enough = have >= give_n_eff as u16;
                    egui::Frame::new()
                        .fill(egui::Color32::from_black_alpha(130))
                        .corner_radius(7.0)
                        .inner_margin(6.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let give_stack = ItemStack { item_id: give.to_string(), count: 1 };
                                let get_stack = ItemStack { item_id: get.to_string(), count: 1 };
                                let (r, resp) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                                paint_item(ui, r, &give_stack, &self.icons);
                                kit::hover_item_tooltip(&resp, &give_stack, &self.icons);
                                let price_label = if friendly {
                                    format!("x{} (friendly: {})", give_n, give_n_eff)
                                } else {
                                    format!("x{}", give_n_eff)
                                };
                                ui.label(egui::RichText::new(price_label)
                                    .color(if enough { Theme::OK } else { Theme::BAD }));
                                ui.label(egui::RichText::new("->").color(Theme::TEXT_DIM));
                                let (r2, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                                paint_item(ui, r2, &get_stack, &self.icons);
                                ui.label(egui::RichText::new(format!("x{}", get_n)).color(Theme::TEXT));
                                ui.label(egui::RichText::new(format!("(have {})", have)).small().color(Theme::TEXT_DIM));
                                if ui.add_enabled(enough, egui::Button::new("Trade")).clicked() {
                                    let mut left = give_n_eff as u16;
                                    'pay: for slot in self.inventory.slots.iter_mut() {
                                        if let Some(stack) = slot {
                                            if stack.item_id == *give {
                                                let take = (stack.count as u16).min(left);
                                                stack.count -= take as u8;
                                                left -= take;
                                                if stack.count == 0 {
                                                    *slot = None;
                                                }
                                                if left == 0 { break 'pay; }
                                            }
                                        }
                                    }
                                    traded_items += *get_n as u16;
                                    let leftover = self.inventory.add_item(get, *get_n);
                                    if leftover > 0 {
                                        self.spawn_drop(get, leftover, self.player.eye_position() + self.player.look_dir());
                                    }
                                }
                            });
                        });
                }
                // trade accrues standing (FACTIONS_OVERVIEW: 10+ items = +2)
                if traded_items > 0 && !faction.is_empty() {
                    let bump = self.lore_data.standing_events.trade_ten_items;
                    self.add_standing(&faction, bump, "traded fairly");
                    // C4: the NPC remembers the trade
                    if let Some(v) = self.villagers.get_mut(index) {
                        v.record_interaction(lf_npc::NpcEvent::Traded, self.day_index as u32);
                    }
                }
                // B2: the hire button for hireable archetypes
                if let Some((hire_standing, fee, _form)) = hire_info {
                    ui.separator();
                    let fee_str = fee.iter()
                        .map(|(i, n)| format!("{} x{}", i, n))
                        .collect::<Vec<_>>().join(", ");
                    let standing_ok = standing >= hire_standing;
                    let slots_ok = self.companions.len() < lf_game::companions::MAX_ACTIVE_COMPANIONS;
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(
                            if standing_ok { format!("Hire companion — fee: {} (needs standing {})", fee_str, hire_standing) }
                            else { format!("Hire — the {} require standing {} (yours: {})", faction, hire_standing, standing) }
                        ).small().color(if standing_ok { Theme::TEXT_DIM } else { Theme::BAD }));
                        if ui.add_enabled(standing_ok && slots_ok, egui::Button::new("Hire")).clicked() {
                            let msg = self.try_hire(index);
                            self.push_hint(&msg);
                            self.close_ui();
                        }
                    });
                }
                ui.separator();
                ui.label(egui::RichText::new("Esc to close").small());
            });
    }

    /// B3: the companion command menu — every command from the spec, with
    /// the trust/morale readout and the roster identity.
    fn draw_companion_menu(&mut self, ctx: &egui::Context) {
        let Some(ci) = self.companion_menu else {
            self.ui_open = UiOpen::None;
            return;
        };
        let Some(c) = self.companions.get(ci).cloned() else {
            self.companion_menu = None;
            self.ui_open = UiOpen::None;
            return;
        };
        let faction_color = c.faction_id.as_deref()
            .and_then(|f| self.lore_data.faction(f))
            .map(|f| egui::Color32::from_rgb(f.color[0], f.color[1], f.color[2]))
            .unwrap_or(Theme::TEXT_DIM);
        let state_label = match &c.state {
            lf_game::companions::CompanionState::Idle => "IDLE".to_string(),
            lf_game::companions::CompanionState::Following => "FOLLOW".to_string(),
            lf_game::companions::CompanionState::Guarding { .. } => "GUARD".to_string(),
            lf_game::companions::CompanionState::Resting => "REST".to_string(),
            lf_game::companions::CompanionState::Working => {
                format!("WORK — {}", c.assigned_task.as_ref().map(|t| t.label()).unwrap_or(""))
            }
        };
        let target = self.crosshair_block_pos();
        Self::kit_shell(ctx, &format!("{} — commands", c.display_name), 620.0, |ui| {
                // identity + relationship readout (on-kit)
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, 5.0, faction_color);
                    ui.label(egui::RichText::new(c.display_name.chars().next().unwrap_or('?').to_string())
                        .strong().color(Theme::BG));
                    ui.label(egui::RichText::new(state_label.clone()).small().color(Theme::TEXT_DIM));
                });
                let bar = |ui: &mut egui::Ui, label: &str, v: i32, color: egui::Color32| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(150.0, 8.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, 4.0, egui::Color32::from_black_alpha(140));
                    let filled = egui::Rect::from_min_size(r.min, egui::vec2(r.width() * v as f32 / 100.0, r.height()));
                    ui.painter().rect_filled(filled, 4.0, color);
                    ui.put(r, egui::Label::new(egui::RichText::new(format!("{} {}/100", label, v))
                        .small().color(Theme::TEXT)).selectable(false));
                };
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        bar(ui, "Trust", c.trust, Theme::ACCENT);
                        ui.add_space(2.0);
                        bar(ui, "Morale", c.morale, Theme::OK);
                    });
                });
                if !c.cargo.is_empty() {
                    let cargo = c.cargo.iter().map(|(i, n)| format!("{} x{}", i, n)).collect::<Vec<_>>().join(", ");
                    ui.label(egui::RichText::new(format!("cargo: {}", cargo)).small().color(Theme::TEXT_DIM));
                }
                ui.separator();
                macro_rules! cmd {
                    ($ui:expr, $label:expr, $command:expr) => {
                        if $ui.button($label).clicked() {
                            let msg = self.companion_command(ci, $command);
                            self.push_hint(&msg);
                        }
                    };
                }
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        cmd!(ui, "Follow me", lf_game::companions::CompanionCommand::FollowMe);
                        if let Some(p) = target {
                            cmd!(ui, "Mine this", lf_game::companions::CompanionCommand::MineThis { target: [p.0, p.1, p.2] });
                            cmd!(ui, "Chop nearby", lf_game::companions::CompanionCommand::ChopNearby { center: [p.0, p.1, p.2] });
                        } else {
                            ui.add_enabled(false, egui::Button::new("Mine this"));
                            cmd!(ui, "Chop nearby", lf_game::companions::CompanionCommand::ChopNearby {
                                center: [self.player.position.x as i32, self.player.position.y as i32, self.player.position.z as i32],
                            });
                        }
                        cmd!(ui, "Rest", lf_game::companions::CompanionCommand::Rest);
                        ui.add_enabled(false, egui::Button::new("Craft (recipes soon)"));
                    });
                    ui.vertical(|ui| {
                        let here = self.player.position;
                        cmd!(ui, "Stay here", lf_game::companions::CompanionCommand::StayHere {
                            pos: [here.x, here.y, here.z],
                        });
                        cmd!(ui, "Haul to chest", lf_game::companions::CompanionCommand::HaulToChest {
                            src: [here.x as i32, here.y as i32, here.z as i32],
                            dst: [here.x as i32, here.y as i32, here.z as i32],
                        });
                        cmd!(ui, "Guard area", lf_game::companions::CompanionCommand::GuardArea {
                            area: [here.x as i32, here.y as i32, here.z as i32],
                        });
                        if ui.button("Pay now").clicked() {
                            let msg = self.companion_pay_now(ci);
                            self.push_hint(&msg);
                        }
                        if ui.button(egui::RichText::new("Dismiss").color(Theme::BAD)).clicked() {
                            self.dismiss_companion(ci);
                        }
                    });
                });
                ui.separator();
                ui.label(egui::RichText::new("Esc to close").small());
            });
    }

    fn draw_book(&mut self, ctx: &egui::Context) {
        Self::kit_shell(ctx, "LORE BOOK", 620.0, |ui| {
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    if self.chronicle.is_empty() {
                        ui.label("The pages are blank. The world has not yet written its story — punch a tree, craft, survive.");
                    } else {
                        let md = lf_chronicle::SagaGenerator::export_markdown(&self.chronicle);
                        ui.label(egui::RichText::new(md).monospace().small());
                    }
                });
            });
    }

    /// Step 20: the lore tome reader — paginated, on-kit, real text from
    /// lore/books.toml.
    fn draw_lore_book(&mut self, ctx: &egui::Context) {
        let title = self.open_lore_title.clone().unwrap_or_default();
        let Some(book) = self.lore.books.iter().position(|b| b.title == title) else {
            self.ui_open = UiOpen::None;
            return;
        };
        let (book_title, page_count, current_page) = {
            let book = &self.lore.books[book];
            (book.title.clone(), book.pages.len(), book.pages[self.open_lore_page.min(book.pages.len().saturating_sub(1))].clone())
        };
        let page = self.open_lore_page.min(page_count.saturating_sub(1));
        let t = self.menu_reveal;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(140)))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    kit::center_vertically(ui, 360.0);
                    ui.add_space(30.0);
                    kit::slide_panel(ui, (t / 0.5).clamp(0.0, 1.0), |ui| {
                        ui.set_width(460.0);
                        ui.vertical(|ui| {
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new(&book_title).size(22.0).color(Theme::ACCENT));
                            ui.label(egui::RichText::new(format!("page {} of {}", page + 1, page_count))
                                .small().color(Theme::TEXT_DIM));
                            ui.separator();
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(current_page)
                                .size(15.0).color(Theme::TEXT));
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                let prev_enabled = page > 0;
                                let next_enabled = page + 1 < page_count;
                                if ui.add_enabled(prev_enabled, egui::Button::new("< prev")).clicked() {
                                    self.open_lore_page = page - 1;
                                }
                                if ui.add_enabled(next_enabled, egui::Button::new("next >")).clicked() {
                                    self.open_lore_page = page + 1;
                                }
                                if kit::menu_button(ui, "Close", 1.0, true) {
                                    self.ui_open = UiOpen::None;
                                    if self.stats.health > 0.0 {
                                        self.lock_cursor();
                                    }
                                }
                            });
                            ui.add_space(10.0);
                        });
                    });
                });
            });
    }

    fn draw_smithing(&mut self, ctx: &egui::Context) {
        Self::kit_shell(ctx, "SMITHING FORGE", 620.0, |ui| {
                let temp = self.forge.temperature;
                let zone = (60.0..=80.0).contains(&temp);
                kit::section_header(ui, "Forge Heat", 1.0);
                ui.add_space(10.0);
                // temperature bar with the orange work zone marked
                let (rect, _) = ui.allocate_exact_size(egui::vec2(280.0, 26.0), egui::Sense::hover());
                let p = ui.painter();
                p.rect_filled(rect, 5.0, egui::Color32::from_black_alpha(190));
                // cool -> hot gradient
                for i in 0..rect.width() as i32 {
                    let t = i as f32 / rect.width();
                    let heat = ((t * temp as f32) / 100.0).min(1.0);
                    let col = egui::Color32::from_rgb((90.0 + 165.0 * heat) as u8, (60.0 + 60.0 * heat) as u8, (50.0 + 10.0 * heat) as u8);
                    p.rect_filled(egui::Rect::from_min_size(egui::pos2(rect.left() + i as f32, rect.top() + 2.0), egui::vec2(1.0, rect.height() - 4.0)), 0.0, col);
                }
                let zx0 = rect.left() + rect.width() * 0.60;
                let zx1 = rect.left() + rect.width() * 0.80;
                p.rect_stroke(egui::Rect::from_min_max(egui::pos2(zx0, rect.top()), egui::pos2(zx1, rect.bottom())), 3.0,
                    egui::Stroke::new(2.0, if zone { egui::Color32::from_rgb(255, 200, 90) } else { egui::Color32::from_rgb(150, 110, 60) }), egui::StrokeKind::Middle);
                let fill_t = rect.left() + rect.width() * (temp / 100.0).clamp(0.0, 1.0);
                p.line_segment([egui::pos2(fill_t, rect.top() - 3.0), egui::pos2(fill_t, rect.bottom() + 3.0)],
                    egui::Stroke::new(2.5, Theme::TEXT));
                p.text(rect.center_bottom() + egui::vec2(0.0, 14.0), egui::Align2::CENTER_CENTER,
                    format!("{:.0}° — {}", temp, if zone { "STRIKE NOW" } else { "heat to the marked zone" }),
                    egui::FontId::proportional(12.0),
                    if zone { egui::Color32::from_rgb(255, 200, 90) } else { Theme::TEXT_DIM });
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("Pump bellows (+15)").color(Theme::ACCENT)).clicked() {
                        self.forge.bellows(15.0);
                    }
                    // Strike is a click, not a per-frame tick: the audit
                    // caught strike() running every frame (and the result
                    // being granted every frame) while the window stayed
                    // open in the heat zone.
                    if ui.button(egui::RichText::new("Strike").color(Theme::ACCENT)).clicked() {
                        self.forge.strike();
                        self.play_sfx(lf_audio::Sfx::SmithClang, 0.8);
                    }
                    let ready = self.forge.strikes_completed >= self.forge.target_strikes;
                    let status = if ready {
                        "blade ready!".to_string()
                    } else {
                        format!("strikes {}/{}", self.forge.strikes_completed, self.forge.target_strikes)
                    };
                    ui.label(egui::RichText::new(status).color(if ready { Theme::OK } else { Theme::TEXT }));
                    if ready {
                        let leftover = self.inventory.add_item("steel_ingot", 1);
                        if leftover > 0 {
                            self.spawn_drop("steel_ingot", leftover, self.player.eye_position() + self.player.look_dir());
                        }
                        self.forge.reset();
                    }
                });
                ui.separator();
                ui.label(egui::RichText::new("Strike only in the marked zone (60-80). Esc to close.").small());
            });
    }

    /// The spellbook (P33): learned spells, the three cast slots, mana.
    /// Click a learned spell to send it to the clicked slot; the book is
    /// the only place slots change. On-kit: slide panel, no native windows.
    fn draw_spellbook(&mut self, ctx: &egui::Context) {
        use lf_game::magic::Spell;
        let reveal = self.menu_reveal;
        let (mana, max_mana) = (self.stats.mana, self.stats.max_mana);
        let learned = self.spellbook.learned.clone();
        let slots = self.spellbook.slots;
        let mut assign: Option<(usize, Option<Spell>)> = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(150)))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    let panel_w = 460.0_f32.min(ui.available_width() - 24.0);
                    kit::center_vertically(ui, 430.0);
                    kit::slide_panel(ui, reveal, |ui| {
                        egui::Frame::new()
                            .fill(Theme::BG)
                            .stroke(egui::Stroke::new(1.0, Theme::MANA))
                            .corner_radius(10.0)
                            .inner_margin(14.0)
                            .show(ui, |ui| {
                                ui.set_width(panel_w);
                                ui.heading(egui::RichText::new("Spellbook").color(Theme::MANA));
                                ui.label(egui::RichText::new(
                                    "the bounded set — four spells, three slots. The wizard sells the rest of what you're missing.",
                                ).small().color(Theme::TEXT_DIM));
                                ui.add_space(6.0);
                                // mana readout
                                let frac = (mana / max_mana).clamp(0.0, 1.0);
                                let (r, _) = ui.allocate_exact_size(egui::vec2(panel_w - 8.0, 10.0), egui::Sense::hover());
                                let p = ui.painter();
                                p.rect_filled(r, 4.0, egui::Color32::from_black_alpha(190));
                                p.rect_filled(egui::Rect::from_min_size(r.min, egui::vec2(r.width() * frac, r.height())), 4.0, Theme::MANA);
                                ui.add_space(8.0);
                                // three cast slots (Z / X / C)
                                ui.horizontal(|ui| {
                                    for (i, slot) in slots.iter().enumerate() {
                                        let key = ["Z", "X", "C"][i];
                                        let label = slot.map(|s| s.name().to_string()).unwrap_or_else(|| "—".into());
                                        let color = slot.map(|_| Theme::MANA).unwrap_or(Theme::TEXT_DIM);
                                        egui::Frame::new()
                                            .fill(egui::Color32::from_black_alpha(120))
                                            .stroke(egui::Stroke::new(1.5, color))
                                            .corner_radius(8.0)
                                            .inner_margin(8.0)
                                            .show(ui, |ui| {
                                                ui.set_min_size(egui::vec2(112.0, 52.0));
                                                ui.heading(egui::RichText::new(format!("{}  [{}]", label, key)).size(13.0).color(color));
                                                ui.label(egui::RichText::new(format!(
                                                    "{} mana", slot.map(|s| s.cost() as u8).unwrap_or(0)
                                                )).small().color(Theme::TEXT_DIM));
                                            });
                                    }
                                });
                                ui.add_space(10.0);
                                ui.separator();
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new("learned — click to fill the first free slot, click a slot row to clear it").small().color(Theme::TEXT_DIM));
                                ui.add_space(4.0);
                                if learned.is_empty() {
                                    ui.label(egui::RichText::new(
                                        "no spells yet — find the tower and buy a scroll",
                                    ).color(Theme::TEXT_DIM));
                                }
                                for spell in &learned {
                                    ui.horizontal(|ui| {
                                        ui.heading(egui::RichText::new(spell.name()).size(14.0).color(Theme::MANA));
                                        ui.label(egui::RichText::new(format!("{} mana", spell.cost() as u8)).small().color(Theme::TEXT_DIM));
                                        if ui.button("-> slot").clicked() {
                                            assign = Some((slots.iter().position(|s| s.is_none()).unwrap_or(2), Some(*spell)));
                                        }
                                        if let Some(si) = slots.iter().position(|s| *s == Some(*spell)) {
                                            if ui.button("clear slot").clicked() {
                                                assign = Some((si, None));
                                            }
                                        }
                                    });
                                    ui.label(egui::RichText::new(spell.desc()).small().color(Theme::TEXT_DIM));
                                    ui.add_space(3.0);
                                }
                                ui.add_space(8.0);
                                if ui.button("Close").clicked() {
                                    self.ui_open = UiOpen::None;
                                    self.lock_cursor();
                                }
                            });
                    });
                });
            });
        if let Some((slot, spell)) = assign {
            self.spellbook.assign(slot, spell);
        }
    }

    /// The enchanting table (P33): the imbue minigame — channel the
    /// attunement into the band, pulse three times, bind a rune to the
    /// held tool. Mirrors the forge's band/strike rhythm.
    fn draw_imbue(&mut self, ctx: &egui::Context) {
        use lf_game::magic::Rune;
        let reveal = self.menu_reveal;
        let attunement = self.imbue.attunement;
        let pulses = self.imbue.pulses;
        let target = self.imbue.target_pulses;
        let held = self.inventory.slots[self.hotbar_index].clone();
        let mut grant: Option<Rune> = None;
        let mut reset = false;
        // first rune in the inventory (the one that would bind)
        let rune_in_inv = Rune::ALL.iter().copied().find(|r| {
            self.inventory.slots.iter().flatten().any(|s| s.item_id == r.item_id())
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(150)))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    let panel_w = 430.0_f32.min(ui.available_width() - 24.0);
                    kit::center_vertically(ui, 400.0);
                    kit::slide_panel(ui, reveal, |ui| {
                        egui::Frame::new()
                            .fill(Theme::BG)
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(160, 120, 240)))
                            .corner_radius(10.0)
                            .inner_margin(14.0)
                            .show(ui, |ui| {
                                ui.set_width(panel_w);
                                ui.heading(egui::RichText::new("Enchanting Table").color(egui::Color32::from_rgb(185, 130, 255)));
                                ui.label(egui::RichText::new(
                                    "channel the attunement into the marked band, then pulse — three clean pulses bind the rune",
                                ).small().color(Theme::TEXT_DIM));
                                ui.add_space(6.0);
                                // attunement bar with the 55..75 band marked
                                let (r, _) = ui.allocate_exact_size(egui::vec2(panel_w - 8.0, 14.0), egui::Sense::hover());
                                let p = ui.painter();
                                p.rect_filled(r, 4.0, egui::Color32::from_black_alpha(190));
                                let band = egui::Rect::from_min_max(
                                    egui::pos2(r.left() + r.width() * 0.55, r.top()),
                                    egui::pos2(r.left() + r.width() * 0.75, r.bottom()),
                                );
                                p.rect_filled(band, 4.0, egui::Color32::from_rgb(60, 90, 150));
                                let w = (attunement / 100.0).clamp(0.0, 1.0);
                                p.rect_filled(egui::Rect::from_min_size(r.min, egui::vec2(r.width() * w, r.height())), 4.0,
                                    egui::Color32::from_rgb(185, 130, 255));
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new(format!("attunement {:.0} — pulses {}/{}", attunement, pulses, target))
                                    .color(Theme::TEXT));
                                ui.horizontal(|ui| {
                                    if ui.button("focus −10").clicked() { self.imbue.focus(-10.0); }
                                    if ui.button("focus +10").clicked() { self.imbue.focus(10.0); }
                                    let in_band = (55.0..=75.0).contains(&attunement);
                                    if ui.add_enabled(rune_in_inv.is_some(),
                                        egui::Button::new(egui::RichText::new("Pulse").color(
                                            if in_band { Theme::OK } else { Theme::TEXT_DIM }))).clicked() {
                                        if self.imbue.pulse() {
                                            grant = rune_in_inv;
                                        }
                                    }
                                });
                                ui.separator();
                                // what binds, and to what
                                match (&held, rune_in_inv) {
                                    (Some(h), Some(rune)) => {
                                        ui.label(egui::RichText::new(format!(
                                            "{} will bind to the held {}", rune.name(), h.item_id,
                                        )).small().color(Theme::TEXT_DIM));
                                        if self.imbue.ready() {
                                            ui.label(egui::RichText::new("the rune is ready — Pulse once more to bind").color(Theme::OK));
                                        }
                                    }
                                    (None, Some(_)) => {
                                        ui.label(egui::RichText::new("hold the tool you want runed").small().color(egui::Color32::from_rgb(230, 130, 130)));
                                    }
                                    (_, None) => {
                                        ui.label(egui::RichText::new("bring a rune — the wizard sells them, or craft them at the table's price")
                                            .small().color(egui::Color32::from_rgb(230, 130, 130)));
                                    }
                                }
                                if let Some(h) = &held {
                                    if let Some(rune_id) = self.runed_tools.get(&h.item_id) {
                                        ui.label(egui::RichText::new(format!(
                                            "already runed: {}", Rune::from_item(rune_id).map(|r| r.name()).unwrap_or(rune_id),
                                        )).small().color(Theme::OK));
                                    }
                                }
                                ui.add_space(8.0);
                                if ui.button("Close").clicked() {
                                    self.ui_open = UiOpen::None;
                                    self.lock_cursor();
                                }
                            });
                    });
                });
            });
        if let Some(rune) = grant {
            if let Some(held_stack) = self.inventory.slots[self.hotbar_index].as_ref() {
                let tool_id = held_stack.item_id.clone();
                self.runed_tools.insert(tool_id.clone(), rune.item_id().to_string());
                // consume one rune
                'find: for slot in self.inventory.slots.iter_mut() {
                    if let Some(s) = slot {
                        if s.item_id == rune.item_id() {
                            s.count -= 1;
                            if s.count == 0 {
                                *slot = None;
                            }
                            break 'find;
                        }
                    }
                }
                self.chronicle_event(lf_chronicle::EventType::RuneApplied,
                    format!("a {} is bound into the {}", rune.name(), tool_id));
                reset = true;
            }
        }
        if reset {
            self.imbue.reset();
        }
    }

    /// Statue carving (P34): the chisel minigame — hold the detail in
    /// the band, tap three times, the stone becomes a statue.
    fn draw_carve(&mut self, ctx: &egui::Context) {
        let reveal = self.menu_reveal;
        let detail = self.carve.detail;
        let taps = self.carve.taps;
        let target = self.carve_target;
        let mut grant = false;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(150)))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    let panel_w = 400.0_f32.min(ui.available_width() - 24.0);
                    kit::center_vertically(ui, 360.0);
                    kit::slide_panel(ui, reveal, |ui| {
                        egui::Frame::new()
                            .fill(Theme::BG)
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 190, 170)))
                            .corner_radius(10.0)
                            .inner_margin(14.0)
                            .show(ui, |ui| {
                                ui.set_width(panel_w);
                                ui.heading(egui::RichText::new("Carving").color(Theme::ACCENT));
                                ui.label(egui::RichText::new(
                                    "chip, check, chip — hold the detail in the band and tap",
                                ).small().color(Theme::TEXT_DIM));
                                ui.add_space(6.0);
                                let (r, _) = ui.allocate_exact_size(egui::vec2(panel_w - 8.0, 14.0), egui::Sense::hover());
                                let p = ui.painter();
                                p.rect_filled(r, 4.0, egui::Color32::from_black_alpha(190));
                                let band = egui::Rect::from_min_max(
                                    egui::pos2(r.left() + r.width() * 0.65, r.top()),
                                    egui::pos2(r.left() + r.width() * 0.85, r.bottom()),
                                );
                                p.rect_filled(band, 4.0, egui::Color32::from_rgb(60, 90, 150));
                                p.rect_filled(egui::Rect::from_min_size(r.min, egui::vec2(r.width() * (detail / 100.0), r.height())), 4.0, Theme::ACCENT);
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new(format!("detail {:.0} — taps {}/3", detail, taps)).color(Theme::TEXT));
                                ui.horizontal(|ui| {
                                    if ui.button("chip −10").clicked() { self.carve.tap_adjust(-10.0); }
                                    if ui.button("chip +10").clicked() { self.carve.tap_adjust(10.0); }
                                    let in_band = (65.0..=85.0).contains(&detail);
                                    if ui.button(egui::RichText::new("Tap").color(
                                        if in_band { Theme::OK } else { Theme::TEXT_DIM })).clicked() {
                                        if self.carve.chisel_tap() {
                                            grant = true;
                                        }
                                    }
                                });
                                ui.add_space(8.0);
                                if ui.button("Close").clicked() {
                                    self.ui_open = UiOpen::None;
                                    self.lock_cursor();
                                }
                            });
                    });
                });
            });
        if grant {
            if let Some((x, y, z)) = target {
                if self.world.get_block(x, y, z).id() == crate::registry::block::STONE {
                    self.world.set_block(x, y, z, lf_voxel::BlockState(crate::registry::block::STATUE));
                    self.remesh_around(x, z);
                    self.after_edit(x, y, z);
                    self.chronicle_event(
                        lf_chronicle::EventType::Discovery,
                        "the chisel finds a figure in the stone".into(),
                    );
                }
            }
            self.carve.reset();
            self.carve_target = None;
        }
    }

    /// The paths screen (P37): four standings, tiers, focus/respec.
    fn draw_paths(&mut self, ctx: &egui::Context) {
        use lf_game::paths::{Path, Paths, RESPEC_COST, TIER_STEP};
        let reveal = self.menu_reveal;
        let snapshot = self.paths.clone();
        let mut respec: Option<Path> = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(150)))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    let panel_w = 480.0_f32.min(ui.available_width() - 24.0);
                    kit::center_vertically(ui, 430.0);
                    kit::slide_panel(ui, reveal, |ui| {
                        egui::Frame::new()
                            .fill(Theme::BG)
                            .stroke(egui::Stroke::new(1.0, Theme::ACCENT))
                            .corner_radius(10.0)
                            .inner_margin(14.0)
                            .show(ui, |ui| {
                                ui.set_width(panel_w);
                                ui.heading(egui::RichText::new("Paths").color(Theme::ACCENT));
                                ui.label(egui::RichText::new(
                                    "no decay, no lock-in — everything you do deepens a path",
                                ).small().color(Theme::TEXT_DIM));
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    for path in Path::ALL {
                                        let standing = snapshot.standing(path);
                                        let tier = snapshot.tier(path);
                                        let focused = snapshot.focus == Some(path);
                                        let color = if focused { Theme::MANA } else { Theme::ACCENT };
                                        egui::Frame::new()
                                            .fill(egui::Color32::from_black_alpha(120))
                                            .stroke(egui::Stroke::new(if focused { 2.5 } else { 1.0 }, color))
                                            .corner_radius(8.0)
                                            .inner_margin(8.0)
                                            .show(ui, |ui| {
                                                ui.set_min_size(egui::vec2(108.0, 96.0));
                                                ui.heading(egui::RichText::new(path.name()).size(14.0).color(color));
                                                ui.label(egui::RichText::new(format!("tier {} — {}/{}", tier, standing % TIER_STEP, TIER_STEP)).small().color(Theme::TEXT_DIM));
                                                let frac = (standing % TIER_STEP) as f32 / TIER_STEP as f32;
                                                let (r, _) = ui.allocate_exact_size(egui::vec2(92.0, 6.0), egui::Sense::hover());
                                                let p = ui.painter();
                                                p.rect_filled(r, 3.0, egui::Color32::from_black_alpha(190));
                                                p.rect_filled(egui::Rect::from_min_size(r.min, egui::vec2(r.width() * frac, r.height())), 3.0, color);
                                                ui.label(egui::RichText::new(path.desc()).small().color(Theme::TEXT_DIM));
                                                if ui.small_button("focus").clicked() {
                                                    respec = Some(path);
                                                }
                                            });
                                    }
                                });
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new(format!(
                                    "respec: pay {} + {}, standings reset, the focused path accrues double",
                                    RESPEC_COST[0].0, RESPEC_COST[1].0,
                                )).small().color(Theme::TEXT_DIM));
                                if let Some(f) = snapshot.focus {
                                    ui.label(egui::RichText::new(format!("current focus: {}", f.name())).small().color(Theme::MANA));
                                }
                                ui.add_space(8.0);
                                if ui.button("Close").clicked() {
                                    self.ui_open = UiOpen::None;
                                    self.lock_cursor();
                                }
                            });
                    });
                });
            });
        if let Some(path) = respec {
            let afford = RESPEC_COST.iter().all(|(id, n)| {
                self.inventory.slots.iter().flatten()
                    .filter(|s| s.item_id == *id)
                    .map(|s| s.count as u16).sum::<u16>() >= *n as u16
            });
            if afford {
                for (id, n) in RESPEC_COST {
                    let mut left = n as u16;
                    for slot in self.inventory.slots.iter_mut() {
                        if left == 0 { break; }
                        if let Some(s) = slot {
                            if s.item_id == id {
                                let take = (s.count as u16).min(left);
                                s.count -= take as u8;
                                left -= take;
                                if s.count == 0 { *slot = None; }
                            }
                        }
                    }
                }
                self.paths.respec(path);
                self.chronicle_event(lf_chronicle::EventType::Discovery,
                    format!("the {} path becomes the work", path.name()));
            } else {
                self.push_hint("cannot afford the respec");
            }
        }
    }

    fn draw_machine(&mut self, ctx: &egui::Context, pos: (i32, i32, i32)) {
        let Some(entity) = self.block_entities.get(&pos).cloned() else {
            self.close_ui();
            return;
        };
        let title = self.world.get_block(pos.0, pos.1, pos.2).id();
        let title = lf_voxel::registry::block::name(title);
        Self::kit_shell(ctx, &title, 700.0, |ui| {
                // EU / progress bar kit styling
                let mut top_bar = |ui: &mut egui::Ui, frac: f32, label: &str, color| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(230.0, 18.0), egui::Sense::hover());
                    let p = ui.painter();
                    p.rect_filled(rect, 4.0, egui::Color32::from_black_alpha(190));
                    p.rect_filled(egui::Rect::from_min_size(rect.min + egui::vec2(2.0, 2.0),
                        egui::vec2((rect.width() - 4.0) * frac.clamp(0.0, 1.0), rect.height() - 4.0)), 4.0, color);
                    p.text(rect.center(), egui::Align2::CENTER_CENTER, label,
                        egui::FontId::proportional(11.0), Theme::TEXT);
                    ui.add_space(4.0);
                };
                match entity {
                    BlockEntity::WaterWheel(mut w) => {
                        // Water Age (P29): the wheel spins for free while
                        // water touches it (checked in the client tick)
                        let frac = w.buffer / lf_game::machines::WHEEL_CAPACITY;
                        top_bar(ui, frac, "spin-up", Theme::ACCENT);
                        ui.label(egui::RichText::new(if frac <= 0.0 {
                            "still — place against flowing or standing water"
                        } else {
                            "the river turns the wheel"
                        }).small().color(Theme::TEXT_DIM));
                        ui.add_space(4.0);
                        self.draw_storage_rows(ui);
                        self.block_entities.insert(pos, BlockEntity::WaterWheel(w));
                    }
                    BlockEntity::Battery(mut b) => {
                        let frac = b.charge / lf_game::machines::BATTERY_CAP;
                        top_bar(ui, frac, "charge", Theme::OK);
                        ui.label(egui::RichText::new(
                            format!("{:.0} / {:.0} EU — covers machines when producers dip", b.charge, lf_game::machines::BATTERY_CAP),
                        ).small().color(Theme::TEXT_DIM));
                        ui.add_space(4.0);
                        self.draw_storage_rows(ui);
                        self.block_entities.insert(pos, BlockEntity::Battery(b));
                    }
                    BlockEntity::Pipe(mut pipe) => {
                        let frac = pipe.water as f32 / lf_game::machines::PIPE_CAP as f32;
                        top_bar(ui, frac, "water", egui::Color32::from_rgb(90, 160, 210));
                        ui.label(egui::RichText::new(
                            format!("{} / {} mB — pipes equalize between neighbors", pipe.water, lf_game::machines::PIPE_CAP),
                        ).small().color(Theme::TEXT_DIM));
                        ui.add_space(4.0);
                        self.draw_storage_rows(ui);
                        self.block_entities.insert(pos, BlockEntity::Pipe(pipe));
                    }
                    BlockEntity::Boiler(mut b) => {
                        let heat = (b.burn_left / 80.0).clamp(0.0, 1.0);
                        top_bar(ui, heat, "fire", egui::Color32::from_rgb(250, 150, 60));
                        top_bar(ui, b.steam / lf_game::machines::BOILER_STEAM_CAP, "steam", egui::Color32::from_gray(230));
                        ui.horizontal(|ui| {
                            let mut fuel = b.fuel.take();
                            let mut cursor = self.cursor_stack.take();
                            let out = slot_button(ui, &mut fuel, &mut cursor, false, &self.icons);
                            self.cursor_stack = cursor;
                            if let Some(mut q) = out.quick_moved {
                                quick_insert(&mut self.inventory.slots[..36], &mut q);
                            }
                            b.fuel = fuel;
                            ui.label(egui::RichText::new("fuel (coal/log/planks)").small().color(Theme::TEXT_DIM));
                        });
                        ui.label(egui::RichText::new(
                            format!("water tank: {} mB (feed with pipes or place against water)", b.water),
                        ).small().color(Theme::TEXT_DIM));
                        ui.add_space(4.0);
                        self.draw_storage_rows(ui);
                        self.block_entities.insert(pos, BlockEntity::Boiler(b));
                    }
                    BlockEntity::SteamEngine(mut e) => {
                        let frac = e.buffer / 600.0;
                        top_bar(ui, frac, "output", Theme::ACCENT);
                        ui.label(egui::RichText::new(if e.steam_avail > 0.0 {
                            "hisss — the engine drinks the boiler's steam"
                        } else {
                            "cold — place against a fueled, watered boiler"
                        }).small().color(Theme::TEXT_DIM));
                        ui.add_space(4.0);
                        self.draw_storage_rows(ui);
                        self.block_entities.insert(pos, BlockEntity::SteamEngine(e));
                    }
                    BlockEntity::Pump(mut p) => {
                        ui.label(egui::RichText::new(format!(
                            "lifetime crude: {} mB", p.lifetime_mb
                        )).small().color(Theme::TEXT_DIM));
                        ui.label(egui::RichText::new(
                            "needs power in range and an oil source below/adjacent; feeds neighbor pipes",
                        ).small().color(Theme::TEXT_DIM));
                        ui.add_space(4.0);
                        self.draw_storage_rows(ui);
                        self.block_entities.insert(pos, BlockEntity::Pump(p));
                    }
                    BlockEntity::Refinery(mut r) => {
                        top_bar(ui, r.crude as f32 / 20_000.0, "crude", egui::Color32::from_rgb(40, 32, 20));
                        top_bar(ui, r.progress / lf_game::machines::REFINERY_TIME, "refining", Theme::ACCENT);
                        ui.horizontal(|ui| {
                            let mut fuel = r.fuel_out.take();
                            let mut tar = r.tar_out.take();
                            let mut cursor = self.cursor_stack.take();
                            let out_f = slot_button(ui, &mut fuel, &mut cursor, false, &self.icons);
                            let out_t = slot_button(ui, &mut tar, &mut cursor, false, &self.icons);
                            self.cursor_stack = cursor;
                            for mut q in [out_f.quick_moved, out_t.quick_moved].into_iter().flatten() {
                                quick_insert(&mut self.inventory.slots[..36], &mut q);
                            }
                            r.fuel_out = fuel;
                            r.tar_out = tar;
                            ui.label(egui::RichText::new("refined fuel / tar").small().color(Theme::TEXT_DIM));
                        });
                        let held = self.inventory.slots[self.hotbar_index].clone();
                        let holding_oil = held.as_ref().map(|s| s.item_id == "oil_bucket").unwrap_or(false);
                        let can_pour = holding_oil && r.crude + lf_game::machines::OIL_BUCKET_MB <= 20_000;
                        if ui.add_enabled(can_pour, egui::Button::new("pour held crude bucket (+1000 mB)")).clicked() {
                            if r.pour_bucket() {
                                self.inventory.slots[self.hotbar_index] =
                                    Some(ItemStack { item_id: "bucket".into(), count: 1 });
                            }
                        }
                        ui.label(egui::RichText::new(
                            "feed crude with pipes or buckets; needs power in range",
                        ).small().color(Theme::TEXT_DIM));
                        ui.add_space(4.0);
                        self.draw_storage_rows(ui);
                        self.block_entities.insert(pos, BlockEntity::Refinery(r));
                    }
                    BlockEntity::Combustion(mut c) => {
                        top_bar(ui, c.buffer / lf_game::machines::COMBUSTION_CAP,
                            &format!("{:.0} / {} EU", c.buffer, lf_game::machines::COMBUSTION_CAP), Theme::XP);
                        ui.horizontal(|ui| {
                            let mut fuel = c.fuel.take();
                            let mut cursor = self.cursor_stack.take();
                            let out = slot_button(ui, &mut fuel, &mut cursor, false, &self.icons);
                            self.cursor_stack = cursor;
                            if let Some(mut q) = out.quick_moved {
                                quick_insert(&mut self.inventory.slots[..36], &mut q);
                            }
                            c.fuel = fuel;
                            ui.label(egui::RichText::new("fuel (refined fuel only)").small().color(Theme::TEXT_DIM));
                        });
                        ui.label(egui::RichText::new(if c.burn_left > 0.0 {
                            format!("burning — {:.0}s left", c.burn_left)
                        } else {
                            "cold — it only drinks what the refinery makes".to_string()
                        }).small().color(Theme::TEXT_DIM));
                        ui.add_space(4.0);
                        self.draw_storage_rows(ui);
                        self.block_entities.insert(pos, BlockEntity::Combustion(c));
                    }
                    BlockEntity::Reactor(mut r) => {
                        let heat_frac = (r.heat / lf_game::machines::MELTDOWN_AT).clamp(0.0, 1.0);
                        let heat_color = if r.heat >= lf_game::machines::SCRAM_AT {
                            egui::Color32::from_rgb(255, 90, 80)
                        } else if r.heat > lf_game::machines::UNSCRAM_BELOW {
                            egui::Color32::from_rgb(255, 170, 60)
                        } else {
                            Theme::OK
                        };
                        top_bar(ui, heat_frac, &format!("heat {:.0}/{}", r.heat, lf_game::machines::MELTDOWN_AT as u8), heat_color);
                        top_bar(ui, r.coolant as f32 / 20_000.0, "coolant", egui::Color32::from_rgb(90, 160, 210));
                        top_bar(ui, r.buffer / lf_game::machines::REACTOR_CAP,
                            &format!("{:.0} / {} EU", r.buffer, lf_game::machines::REACTOR_CAP), Theme::XP);
                        ui.horizontal(|ui| {
                            let mut fuel = r.fuel.take();
                            let mut cursor = self.cursor_stack.take();
                            let out = slot_button(ui, &mut fuel, &mut cursor, false, &self.icons);
                            self.cursor_stack = cursor;
                            if let Some(mut q) = out.quick_moved {
                                quick_insert(&mut self.inventory.slots[..36], &mut q);
                            }
                            r.fuel = fuel;
                            ui.label(egui::RichText::new("fuel rods").small().color(Theme::TEXT_DIM));
                        });
                        ui.horizontal(|ui| {
                            if ui.button("SCRAM").clicked() {
                                r.scram();
                            }
                            let can_unscram = r.scram && r.heat < lf_game::machines::UNSCRAM_BELOW;
                            if ui.add_enabled(can_unscram, egui::Button::new("restart core")).clicked() {
                                r.try_unscram();
                            }
                            let status = if r.scram {
                                "SCRAMMED — keep the coolant flowing or it still melts".to_string()
                            } else if r.burn_left > 0.0 {
                                format!("fissioning — rod: {:.0}s left", r.burn_left)
                            } else {
                                "idle — load a fuel rod and keep water on it".to_string()
                            };
                            ui.label(egui::RichText::new(status)
                                .small()
                                .color(if r.scram { heat_color } else { Theme::TEXT_DIM }));
                        });
                        ui.label(egui::RichText::new(
                            "feed cooling water with pipes or place it against water",
                        ).small().color(Theme::TEXT_DIM));
                        ui.add_space(4.0);
                        self.draw_storage_rows(ui);
                        self.block_entities.insert(pos, BlockEntity::Reactor(r));
                    }
                    BlockEntity::Generator(mut g) => {
                        top_bar(ui, g.buffer / lf_game::machines::GEN_CAPACITY, &format!("{:.0} / {} EU", g.buffer, lf_game::machines::GEN_CAPACITY), Theme::XP);
                        let mut fuel = g.fuel.take();
                        let mut cursor = self.cursor_stack.take();
                        let out = slot_button(ui, &mut fuel, &mut cursor, false, &self.icons);
                        self.cursor_stack = cursor;
                        if let Some(mut q) = out.quick_moved {
                            quick_insert(&mut self.inventory.slots[..36], &mut q);
                        }
                        g.fuel = fuel;
                        ui.label(egui::RichText::new("fuel (coal/log/planks)").small().color(Theme::TEXT_DIM));
                        ui.add_space(4.0);
                        self.draw_storage_rows(ui);
                        self.block_entities.insert(pos, BlockEntity::Generator(g));
                    }
                    BlockEntity::ElectricFurnace(mut f) => {
                        top_bar(ui, f.progress / (lf_game::smelting::SMELT_TIME / 2.0), "smelt (2x)", Theme::ACCENT);
                        ui.horizontal(|ui| {
                            let mut input = f.input.take();
                            let mut cursor = self.cursor_stack.take();
                            let out = slot_button(ui, &mut input, &mut cursor, false, &self.icons);
                            self.cursor_stack = cursor;
                            if let Some(mut q) = out.quick_moved {
                                quick_insert(&mut self.inventory.slots[..36], &mut q);
                            }
                            f.input = input;
                            let mut output = f.output.take();
                            let mut cursor = self.cursor_stack.take();
                            let out = slot_button(ui, &mut output, &mut cursor, false, &self.icons);
                            self.cursor_stack = cursor;
                            if let Some(mut q) = out.quick_moved {
                                quick_insert(&mut self.inventory.slots[..36], &mut q);
                            }
                            f.output = output;
                        });
                        ui.add_space(4.0);
                        self.draw_storage_rows(ui);
                        self.block_entities.insert(pos, BlockEntity::ElectricFurnace(f));
                    }
                    BlockEntity::Crusher(mut c) => {
                        top_bar(ui, c.progress / lf_game::machines::PROCESS_TIME, "crush", Theme::ACCENT);
                        ui.horizontal(|ui| {
                            let mut input = c.input.take();
                            let mut cursor = self.cursor_stack.take();
                            let out = slot_button(ui, &mut input, &mut cursor, false, &self.icons);
                            self.cursor_stack = cursor;
                            if let Some(mut q) = out.quick_moved {
                                quick_insert(&mut self.inventory.slots[..36], &mut q);
                            }
                            c.input = input;
                            let mut output = c.output.take();
                            let mut cursor = self.cursor_stack.take();
                            let out = slot_button(ui, &mut output, &mut cursor, false, &self.icons);
                            self.cursor_stack = cursor;
                            if let Some(mut q) = out.quick_moved {
                                quick_insert(&mut self.inventory.slots[..36], &mut q);
                            }
                            c.output = output;
                        });
                        ui.label(egui::RichText::new("ores in — 2x raw out").small().color(Theme::TEXT_DIM));
                        ui.add_space(4.0);
                        self.draw_storage_rows(ui);
                        self.block_entities.insert(pos, BlockEntity::Crusher(c));
                    }
                    BlockEntity::Assembler(mut a) => {
                        top_bar(ui, a.progress / lf_game::machines::PROCESS_TIME, "assemble", Theme::ACCENT);
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("A").small().color(Theme::TEXT_DIM));
                                let mut ia = a.input_a.take();
                                let mut cursor = self.cursor_stack.take();
                                let out = slot_button(ui, &mut ia, &mut cursor, false, &self.icons);
                                self.cursor_stack = cursor;
                                if let Some(mut q) = out.quick_moved {
                                    quick_insert(&mut self.inventory.slots[..36], &mut q);
                                }
                                a.input_a = ia;
                            });
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("B").small().color(Theme::TEXT_DIM));
                                let mut ib = a.input_b.take();
                                let mut cursor = self.cursor_stack.take();
                                let out = slot_button(ui, &mut ib, &mut cursor, false, &self.icons);
                                self.cursor_stack = cursor;
                                if let Some(mut q) = out.quick_moved {
                                    quick_insert(&mut self.inventory.slots[..36], &mut q);
                                }
                                a.input_b = ib;
                            });
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Out").small().color(Theme::TEXT_DIM));
                                let mut output = a.output.take();
                                let mut cursor = self.cursor_stack.take();
                                let out = slot_button(ui, &mut output, &mut cursor, false, &self.icons);
                                self.cursor_stack = cursor;
                                if let Some(mut q) = out.quick_moved {
                                    quick_insert(&mut self.inventory.slots[..36], &mut q);
                                }
                                a.output = output;
                            });
                        });
                        // live alloy recipe with icons
                        if let Some((an, an_n, bn, bn_n, out, out_n)) = a.current_recipe() {
                            ui.horizontal(|ui| {
                                for (id, n) in [(an, an_n), (bn, bn_n)] {
                                    let (r, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                                    paint_item(ui, r, &ItemStack { item_id: id.to_string(), count: 1 }, &self.icons);
                                    ui.label(egui::RichText::new(format!("x{} +", n)).small().color(Theme::TEXT_DIM));
                                }
                                ui.label(egui::RichText::new("->").color(Theme::TEXT_DIM));
                                let (r, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                                paint_item(ui, r, &ItemStack { item_id: out.to_string(), count: 1 }, &self.icons);
                                ui.label(egui::RichText::new(format!("x{}", out_n)).small().color(Theme::OK));
                            });
                        } else {
                            ui.label(egui::RichText::new("no recipe (try Cu+Sn, Fe+C, wire+Sn...)").small().color(egui::Color32::from_gray(150)));
                        }
                        ui.add_space(4.0);
                        self.draw_storage_rows(ui);
                        self.block_entities.insert(pos, BlockEntity::Assembler(a));
                    }
                    _ => {}
                }
            });
    }

    /// The tech tree: era columns, states, costs with live have/need, and a
    /// "what to do next" hint — the progression compass.
    fn draw_tech_tree(&mut self, ctx: &egui::Context) {
        let era = self.research.era;
        let have = lf_game::research::ResearchState::have_counts(&self.inventory.slots);
        Self::kit_shell(ctx, "TECHNOLOGY", 760.0, |ui| {
                ui.heading(egui::RichText::new("Research Progression").size(22.0).color(Theme::TEXT));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let eras = [Era::Primitive, Era::Bronze, Era::Industrial, Era::Electrical];
                    for e in eras {
                        let state = if e < era { "done" } else if e == era { "CURRENT" } else { "locked" };
                        let color = if e < era { Theme::OK }
                            else if e == era { Theme::ACCENT }
                            else { egui::Color32::from_gray(110) };
                        egui::Frame::new()
                            .fill(egui::Color32::from_black_alpha(120))
                            .stroke(egui::Stroke::new(if e == era { 2.5 } else { 1.0 }, color))
                            .corner_radius(8.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.set_min_size(egui::vec2(140.0, 90.0));
                                ui.heading(egui::RichText::new(e.name()).size(15.0).color(color));
                                ui.label(egui::RichText::new(state).small().color(color));
                                if e > era {
                                    ui.add_space(4.0);
                                    for (item, n) in e.cost() {
                                        let got = have.iter().find(|(id, _)| id == item).map(|(_, c)| *c).unwrap_or(0);
                                        let ok = got >= *n as u16;
                                        let c = if ok { Theme::OK } else { egui::Color32::from_rgb(230, 130, 130) };
                                        // icon + have/need
                                        ui.horizontal(|ui| {
                                            let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                                            paint_item(ui, r, &ItemStack { item_id: item.to_string(), count: 1 }, &self.icons);
                                            ui.label(egui::RichText::new(format!("{}/{}", got.min(*n as u16), n)).small().color(c));
                                        });
                                    }
                                }
                            });
                        if e != Era::Electrical {
                            ui.label(egui::RichText::new("->").color(Theme::TEXT_DIM));
                        }
                    }
                });
                ui.add_space(8.0);
                // Branch eras (Water/Steam/Oil): unlockable in any order
                // relative to each other, right here in the tree.
                let mut branch_unlocked = None;
                ui.horizontal(|ui| {
                    for e in [Era::Water, Era::Steam, Era::Oil, Era::Nuclear] {
                        let owned = self.research.unlocked(e);
                        let can = self.research.can_unlock(e);
                        let color = if owned { Theme::OK } else if can { Theme::ACCENT } else { egui::Color32::from_gray(110) };
                        egui::Frame::new()
                            .fill(egui::Color32::from_black_alpha(120))
                            .stroke(egui::Stroke::new(if can && !owned { 2.5 } else { 1.0 }, color))
                            .corner_radius(8.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.set_min_size(egui::vec2(150.0, 90.0));
                                ui.heading(egui::RichText::new(e.name()).size(15.0).color(color));
                                ui.label(egui::RichText::new(if owned {
                                    "done"
                                } else if e == Era::Oil {
                                    "branch — needs Steam or Electrical"
                                } else if e == Era::Nuclear {
                                    if self.research.reactor_safety {
                                        "the ceiling — needs the Oil Age"
                                    } else {
                                        "locked — earn the safety certification below"
                                    }
                                } else {
                                    "branch — parallel to the chain"
                                }).small().color(color));
                                if !owned {
                                    ui.add_space(4.0);
                                    let mut affordable = true;
                                    for (item, n) in e.cost() {
                                        let got = have.iter().find(|(id, _)| id == item).map(|(_, c)| *c).unwrap_or(0);
                                        let ok = got >= *n as u16;
                                        affordable &= ok;
                                        let cc = if ok { Theme::OK } else { egui::Color32::from_rgb(230, 130, 130) };
                                        ui.horizontal(|ui| {
                                            let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                                            paint_item(ui, r, &ItemStack { item_id: item.to_string(), count: 1 }, &self.icons);
                                            ui.label(egui::RichText::new(format!("{}/{}", got.min(*n as u16), n)).small().color(cc));
                                        });
                                    }
                                    if can && affordable && ui.button("Unlock").clicked() {
                                        branch_unlocked = Some(e);
                                    }
                                }
                            });
                    }
                });
                if let Some(e) = branch_unlocked {
                    let _ = self.research.unlock(e, &mut self.inventory.slots);
                }
                // P32: the reactor safety certification — study gates the
                // Nuclear branch (glass containment + circuits + a book).
                if self.research.unlocked(Era::Oil) && !self.research.reactor_safety {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Reactor Safety Certification").small().color(Theme::ACCENT));
                        let have = lf_game::research::ResearchState::have_counts(&self.inventory.slots);
                        let mut affordable = true;
                        for (item, n) in lf_game::research::ResearchState::REACTOR_SAFETY_COST {
                            let got = have.iter().find(|(id, _)| id == item).map(|(_, c)| *c).unwrap_or(0);
                            affordable &= got >= n as u16;
                            let cc = if got >= n as u16 { Theme::OK } else { egui::Color32::from_rgb(230, 130, 130) };
                            ui.label(egui::RichText::new(format!("{} {}/{}", item, got.min(n as u16), n)).small().color(cc));
                        }
                        if affordable && ui.button("Study").clicked() {
                            self.research.unlock_reactor_safety(&mut self.inventory.slots);
                        }
                    });
                }
                ui.add_space(8.0);
                ui.separator();
                let hint = match era.next() {
                    Some(next) => {
                        let missing: Vec<String> = next.cost().iter()
                            .map(|(item, n)| {
                                let got = have.iter().find(|(id, _)| id == item).map(|(_, c)| *c).unwrap_or(0);
                                if got >= *n as u16 { format!("{} ok", item) }
                                else { format!("{} ({}/{})", item, got, n) }
                            })
                            .collect();
                        format!("Next: the {} — place a Research Bench and bring: {}. Current era unlocks: {}",
                            next.name(), missing.join(", "),
                            match era {
                                Era::Primitive => "basic tools, furnace, chest",
                                Era::Bronze => "armor, smithing, +everything before",
                                Era::Industrial => "generators, crushers, assemblers",
                                _ => "electric furnace, all machines",
                            })
                    }
                    None => "Final era reached: everything is unlocked.".to_string(),
                };
                ui.label(egui::RichText::new(hint).color(egui::Color32::from_rgb(150, 220, 255)));
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Advance by right-clicking a Research Bench with the materials in your inventory.").small().color(egui::Color32::from_gray(170)));
            });
    }

    /// Load World screen (ui-world-craft C2): one slot per row — seed-
    /// rendered thumbnail, world-type glyph, name, difficulty, last-played —
    /// plus a delete flow with the honest "cannot be undone" prompt.
    fn draw_slots(&mut self, ctx: &egui::Context) {
        let reveal = kit::ease_out_cubic((self.menu_reveal / 0.35).clamp(0.0, 1.0));
        let mut open_game = None;
        let mut delete_slot: Option<String> = None;
        let slots = crate::slots::list_slots();
        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ctx, |ui| {
                kit::vignette(ui, 170);
                // vertical centering: header + list + footer ≈ 480px tall
                kit::center_vertically(ui, 480.0);
                kit::slide_panel(ui, reveal, |ui| {
                    ui.set_width(560.0);
                    ui.vertical(|ui| {
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new("Load World").size(24.0).color(Theme::TEXT));
                        ui.add_space(4.0);
                        ui.painter().line_segment(
                            [ui.cursor().min, ui.cursor().min + egui::vec2(ui.available_width(), 0.0)],
                            egui::Stroke::new(1.0, Theme::BORDER));
                        ui.add_space(10.0);
                        egui::ScrollArea::vertical().max_height(330.0).show(ui, |ui| {
                            if slots.is_empty() {
                                ui.label(egui::RichText::new("No worlds yet. Create one from the title screen.")
                                    .color(Theme::TEXT_DIM));
                            }
                            for meta in &slots {
                                let current = meta.name == self.slot_meta.name;
                                egui::Frame::new()
                                    .fill(if current { Color32::from_rgba_premultiplied(0x3d, 0x30, 0x1e, 235) }
                                          else { Color32::from_rgba_premultiplied(0x24, 0x1c, 0x14, 220) })
                                    .stroke(egui::Stroke::new(if current { 1.5 } else { 1.0 },
                                        if current { Theme::ACCENT } else { Theme::BORDER }))
                                    .corner_radius(0.0)
                                    .inner_margin(8.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            // thumbnail: cached seed render, else the
                                            // live autosave view, else a type-colored tile
                                            let thumb_key = meta.name.clone();
                                            if !self.slot_thumbs.contains_key(&thumb_key) {
                                                let tex = load_slot_thumbnail(ui, &meta.name, meta.seed, meta.world_type);
                                                if let Some(tex) = tex {
                                                    self.slot_thumbs.insert(thumb_key.clone(), tex);
                                                }
                                            }
                                            let (trect, _) = ui.allocate_exact_size(egui::vec2(96.0, 54.0), egui::Sense::hover());
                                            match self.slot_thumbs.get(&thumb_key) {
                                                Some(tex) => {
                                                    ui.painter().image(tex.id(), trect,
                                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                                        Color32::WHITE);
                                                }
                                                None => {
                                                    ui.painter().rect_filled(trect, 0.0, placeholder_color(meta.world_type));
                                                    ui.painter().text(trect.center(), egui::Align2::CENTER_CENTER,
                                                        world_type_glyph(meta.world_type),
                                                        egui::FontId::proportional(20.0), Theme::TEXT_DIM);
                                                }
                                            }
                                            ui.painter().rect_stroke(trect, 0.0, egui::Stroke::new(1.0, Theme::BORDER), egui::StrokeKind::Middle);
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(&meta.name).size(16.0)
                                                        .color(if current { Theme::ACCENT } else { Theme::TEXT }));
                                                    ui.label(egui::RichText::new(world_type_glyph(meta.world_type))
                                                        .color(Theme::TEXT_DIM));
                                                });
                                                ui.label(egui::RichText::new(format!(
                                                    "{:?} · {} · seed {}",
                                                    meta.world_type,
                                                    meta.difficulty.label(),
                                                    format_seed(meta.seed)))
                                                    .small().color(Theme::TEXT_DIM));
                                                ui.label(egui::RichText::new(format!(
                                                    "last played {} · created {}",
                                                    time_ago(meta.updated_secs),
                                                    if meta.created_secs > 0 { time_ago(meta.created_secs) } else { "before v0.4".into() }))
                                                    .small().color(Theme::TEXT_DISABLED));
                                            });
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let confirming = self.delete_confirm.as_deref() == Some(meta.name.as_str());
                                                if confirming {
                                                    ui.label(egui::RichText::new(format!("Delete '{}'? This cannot be undone.", meta.name))
                                                        .small().color(Theme::BAD));
                                                    if ui.add(egui::Button::new(egui::RichText::new("Yes").color(Theme::BAD))).clicked() {
                                                        self.delete_confirm = None;
                                                        delete_slot = Some(meta.name.clone());
                                                    }
                                                    if ui.button("No").clicked() {
                                                        self.delete_confirm = None;
                                                    }
                                                } else {
                                                    if ui.button("Delete").clicked() {
                                                        self.delete_confirm = Some(meta.name.clone());
                                                    }
                                                    if ui.add(egui::Button::new(egui::RichText::new(
                                                        if current { "Play" } else { "Load" })
                                                        .color(Theme::WARNING))).clicked() {
                                                        open_game = Some(meta.name.clone());
                                                    }
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(4.0);
                            }
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            // Back: plain navigation — underline only on hover
                            if kit::menu_link(ui, "Back", "slots-back", 1.0, false, true) {
                                self.ui_open = UiOpen::Title;
                                self.menu_reveal = 0.0;
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // action link: underline pinned on (action, not navigation)
                                if kit::menu_link(ui, "New World", "slots-new", 1.0, true, true) {
                                    self.open_new_world_screen();
                                }
                            });
                        });
                        ui.add_space(8.0);
                    });
                });
            });
        if let Some(name) = open_game {
            let _ = self.load_world(&name);
        }
        if let Some(name) = delete_slot {
            crate::slots::delete_slot(&name);
            if name == self.slot_meta.name {
                // deleted the live world — fall back to the newest other slot
                match crate::slots::list_slots().into_iter().next() {
                    Some(meta) => { let _ = self.load_world(&meta.name); }
                    None => {
                        let seed = crate::slots::random_seed();
                        self.create_world("World 1", seed, lf_worldgen::WorldType::Normal,
                            crate::slots::Difficulty::Easy, crate::slots::GameMode::Survival);
                        self.ui_open = UiOpen::Title;
                        self.menu_reveal = 0.0;
                    }
                }
            }
        }
    }

    /// New World screen (ui-world-craft C1): one panel, five field groups —
    /// name, visible seed with a Roll link, world type, game mode,
    /// difficulty — then Back (navigation link) and Create World (pinned
    /// action link). Strings that don't parse as numbers hash to seeds;
    /// an empty name is the only error, and it says so in danger red.
    fn draw_new_world(&mut self, ctx: &egui::Context) {
        let reveal = kit::ease_out_cubic((self.menu_reveal / 0.35).clamp(0.0, 1.0));
        let mut rolled = false;
        let mut go_back = false;
        let mut create: Option<()> = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ctx, |ui| {
                kit::vignette(ui, 170);
                let screen = ctx.screen_rect();
                let panel_w = (screen.width() * 0.5).clamp(420.0, 560.0);
                let panel_h = 470.0;
                // centered on both axes (loop 329: the panel used to anchor
                // top-left because a fresh top_down cursor starts at 0,0)
                let rect = kit::centered_panel_rect(screen, panel_w, panel_h);
                let painter = ui.painter_at(screen);
                painter.rect_filled(rect, 0.0, Color32::from_rgba_premultiplied(0x33, 0x2a, 0x1c, 242));
                painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, Theme::BORDER), egui::StrokeKind::Middle);
                ui.allocate_ui_at_rect(rect.shrink(32.0), |ui| {
                    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.label(egui::RichText::new("New World").size(24.0).color(Theme::TEXT));
                        ui.add_space(4.0);
                        ui.painter().line_segment(
                            [ui.cursor().min, ui.cursor().min + egui::vec2(ui.available_width(), 0.0)],
                            egui::Stroke::new(1.0, Theme::BORDER));
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("Name").size(13.0).color(Theme::TEXT_DIM));
                        let name_error = self.new_world_error.clone().unwrap_or_default();
                        kit::text_input(ui, &mut self.new_world_name, "nw-name", "Name your world...", panel_w - 64.0);
                        if !name_error.is_empty() {
                            ui.label(egui::RichText::new(name_error).size(12.0).color(Theme::BAD));
                        }
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new("Seed").size(13.0).color(Theme::TEXT_DIM));
                        ui.horizontal(|ui| {
                            kit::text_input(ui, &mut self.new_world_seed, "nw-seed", "numbers or any words", panel_w - 150.0);
                            // [Roll] is a text link in muted, not a button widget
                            if kit::menu_link(ui, "[ Roll ]", "nw-roll", 1.0, false, true) {
                                rolled = true;
                            }
                        });
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new("World Type").size(13.0).color(Theme::TEXT_DIM));
                        let types = ["Normal", "Superflat", "Amplified"];
                        if kit::segment_row(ui, "nw-type", &types, &mut self.new_world_type_idx) {
                            // selection drives generation on create
                        }
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new("Game Mode").size(13.0).color(Theme::TEXT_DIM));
                        let modes = ["Survival", "Creative"];
                        kit::segment_row(ui, "nw-mode", &modes, &mut self.new_world_mode_idx);
                        if self.new_world_mode_idx == 1 {
                            ui.label(egui::RichText::new("Creative: no damage or hunger, infinite blocks, instant mining — F to fly.")
                                .size(11.0).color(Theme::TEXT_DISABLED));
                        }
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new("Difficulty").size(13.0).color(Theme::TEXT_DIM));
                        let diffs = ["Peaceful", "Easy", "Normal", "Hard"];
                        kit::segment_row(ui, "nw-diff", &diffs, &mut self.new_world_diff_idx);
                        let diff_hint = match crate::slots::Difficulty::ALL[self.new_world_diff_idx.min(3)] {
                            crate::slots::Difficulty::Peaceful => "No hostile mobs. Nothing fights back.",
                            crate::slots::Difficulty::Easy => "Mobs exist, hits hurt less.",
                            crate::slots::Difficulty::Normal => "The world as intended.",
                            crate::slots::Difficulty::Hard => "Harder hits, faster hunger.",
                        };
                        ui.label(egui::RichText::new(diff_hint).size(11.0).color(Theme::TEXT_DISABLED));
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            if kit::menu_link(ui, "Back", "nw-back", reveal, false, true) {
                                go_back = true;
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if kit::menu_link(ui, "Create World", "nw-create", reveal, true, true) {
                                    create = Some(());
                                }
                            });
                        });
                    });
                });
            });
        if rolled {
            self.new_world_seed = crate::slots::random_seed().to_string();
            self.new_world_error = None;
        }
        if go_back {
            self.ui_open = UiOpen::Title;
            self.menu_reveal = 0.0;
        }
        if create.is_some() {
            if self.create_world_from_screen().is_ok() {
                self.menu_reveal = 0.0;
            }
        }
    }

    /// Multiplayer screen (ui-world-craft C3): Direct Connect (IP + port +
    /// Connect), Host World (pick a slot, start the dedicated server), and
    /// the honest Steam-lobby stub.
    fn draw_multiplayer(&mut self, ctx: &egui::Context) {
        let reveal = kit::ease_out_cubic((self.menu_reveal / 0.35).clamp(0.0, 1.0));
        let mut go_back = false;
        let mut connect: Option<String> = None;
        let mut host_slot: Option<String> = None;
        let slots = crate::slots::list_slots();
        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ctx, |ui| {
                kit::vignette(ui, 170);
                let screen = ctx.screen_rect();
                let panel_w = (screen.width() * 0.5).clamp(420.0, 560.0);
                // centered on both axes (same top-left fix as New World)
                let rect = kit::centered_panel_rect(screen, panel_w, 420.0);
                let painter = ui.painter_at(screen);
                painter.rect_filled(rect, 0.0, Color32::from_rgba_premultiplied(0x33, 0x2a, 0x1c, 242));
                painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, Theme::BORDER), egui::StrokeKind::Middle);
                ui.allocate_ui_at_rect(rect.shrink(32.0), |ui| {
                    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.label(egui::RichText::new("Multiplayer").size(24.0).color(Theme::TEXT));
                        ui.add_space(4.0);
                        ui.painter().line_segment(
                            [ui.cursor().min, ui.cursor().min + egui::vec2(ui.available_width(), 0.0)],
                            egui::Stroke::new(1.0, Theme::BORDER));
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("Direct Connect").size(13.0).color(Theme::TEXT_DIM));
                        ui.horizontal(|ui| {
                            kit::text_input(ui, &mut self.mp_address, "mp-addr", "IP address", panel_w * 0.42);
                            kit::text_input(ui, &mut self.mp_port, "mp-port", "port", 96.0);
                            if kit::menu_link(ui, "Connect", "mp-connect", reveal, true, true) {
                                let addr = format!("{}:{}",
                                    if self.mp_address.trim().is_empty() { "127.0.0.1" } else { self.mp_address.trim() },
                                    self.mp_port.trim());
                                connect = Some(addr);
                            }
                        });
                        if let Some(status) = &self.mp_status {
                            ui.label(egui::RichText::new(status).size(12.0).color(Theme::WARNING));
                        }
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("Host World").size(13.0).color(Theme::TEXT_DIM));
                        if slots.is_empty() {
                            ui.label(egui::RichText::new("No worlds to host — create one first.")
                                .size(12.0).color(Theme::TEXT_DISABLED));
                        }
                        egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                            for meta in &slots {
                                let selected = slots[self.mp_host_idx.min(slots.len() - 1)].name == meta.name;
                                let (rect, resp) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), 30.0), egui::Sense::click());
                                if selected {
                                    ui.painter().rect_filled(rect, 0.0,
                                        Color32::from_rgba_premultiplied(0x3d, 0x30, 0x1e, 235));
                                    ui.painter().rect_filled(
                                        egui::Rect::from_min_size(rect.left_top(), egui::vec2(3.0, rect.height())),
                                        0.0, Theme::ACCENT);
                                } else if resp.hovered() {
                                    ui.painter().rect_filled(rect, 0.0,
                                        Color32::from_rgba_premultiplied(0x4a, 0x3c, 0x26, 160));
                                }
                                ui.painter().text(
                                    egui::Pos2::new(rect.left() + 12.0, rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    if selected { "▸" } else { " " },
                                    egui::FontId::proportional(13.0), Theme::ACCENT);
                                ui.painter().text(
                                    egui::Pos2::new(rect.left() + 32.0, rect.center().y),
                                    egui::Align2::LEFT_CENTER, &meta.name,
                                    egui::FontId::proportional(14.0),
                                    if selected { Theme::TEXT } else { Theme::TEXT_DIM });
                                ui.painter().text(
                                    egui::Pos2::new(rect.right() - 10.0, rect.center().y),
                                    egui::Align2::RIGHT_CENTER,
                                    format!("seed {}", format_seed(meta.seed)),
                                    egui::FontId::proportional(11.0), Theme::TEXT_DISABLED);
                                if resp.clicked() {
                                    self.mp_host_idx = slots.iter().position(|s| s.name == meta.name).unwrap_or(0);
                                }
                            }
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if kit::menu_link(ui, "Start Server", "mp-host", reveal, true, !slots.is_empty()) {
                                let idx = self.mp_host_idx.min(slots.len().saturating_sub(1));
                                if let Some(meta) = slots.get(idx) {
                                    host_slot = Some(meta.name.clone());
                                }
                            }
                            ui.label(egui::RichText::new("runs the dedicated LOREFORGE server, then connects")
                                .size(11.0).color(Theme::TEXT_DISABLED));
                        });
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("Friends").size(13.0).color(Theme::TEXT_DIM));
                        ui.label(egui::RichText::new("Steam lobbies arrive with the Steam release — direct connect and hosting work today.")
                            .size(11.0).color(Theme::TEXT_DISABLED));
                        ui.add_space(20.0);
                        if kit::menu_link(ui, "Back", "mp-back", reveal, false, true) {
                            go_back = true;
                        }
                    });
                });
            });
        if go_back {
            self.ui_open = UiOpen::Title;
            self.menu_reveal = 0.0;
        }
        if let Some(addr) = connect {
            match crate::net::NetClient::connect(&addr, "smith") {
                Ok(n) => {
                    self.net = Some(n);
                    self.chat_log = vec![format!("joining {}...", addr)];
                    self.close_ui();
                }
                Err(e) => {
                    self.mp_status = Some(format!("connect failed: {}", e));
                }
            }
        }
        if let Some(slot) = host_slot {
            self.mp_status = Some(start_dedicated_server(&slot, &self.mp_port));
        }
    }

    fn draw_death(&mut self, ctx: &egui::Context) {
        // gradient: near-black edges, deep red center
        let screen = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.rect_filled(screen, 0.0, egui::Color32::from_black_alpha(235));
        let c = screen.center();
        for i in 0..40 {
            let t = i as f32 / 40.0;
            let r = t * screen.width().max(screen.height()) * 0.7;
            painter.circle_filled(c, r, egui::Color32::from_rgba_unmultiplied(60, 8, 10, (30.0 * (1.0 - t)) as u8));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.28);
                    ui.label(egui::RichText::new("You died").size(46.0).color(egui::Color32::from_rgb(235, 70, 60)).strong());
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(format!("level {} · {} · {} kills",
                        self.xp_level, self.research.era.name(), self.kills)).color(Theme::TEXT_DIM));
                    ui.add_space(24.0);
                    if kit::menu_button(ui, "Respawn", (self.menu_reveal / 0.4).clamp(0.0, 1.0), true) {
                        self.respawn();
                    }
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("or press Escape to quit").small().color(Theme::TEXT_DIM));
                });
            });
    }

    fn time_label(&self) -> String {
        let frac = self.time.fraction();
        let hours = (frac * 24.0) as u32;
        let minutes = ((frac * 24.0 - hours as f32) * 60.0) as u32;
        format!("{:02}:{:02}", hours, minutes)
    }
}

fn ui_time(ctx: &egui::Context) -> f32 {
    ctx.input(|i| i.time) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Loop 347 caption: the line above the hotbar shows the just-picked
    /// item while its fade window is live (scroll feedback), then hands
    /// the line to the looked-at block's name; empty hands pointing at
    /// nothing show nothing.
    #[test]
    fn hotbar_caption_picks_item_then_target() {
        // fresh switch: the held item wins
        assert_eq!(
            hotbar_caption(Some("Iron Pickaxe"), 1.0, Some("Stone")),
            Some(("Iron Pickaxe".into(), true))
        );
        // faded out: the looked-at block takes the line
        assert_eq!(
            hotbar_caption(Some("Iron Pickaxe"), 0.0, Some("Stone")),
            Some(("Stone".into(), false))
        );
        // nothing held, nothing targeted: no caption
        assert_eq!(hotbar_caption(None, 0.0, None), None);
        // a switch with empty hands falls through to the target name
        assert_eq!(hotbar_caption(None, 1.0, Some("Lavender")), Some(("Lavender".into(), false)));
    }

    /// Audit Step 1: hearts + hotbar rendered under the title menu; the HUD
    /// must hide behind menus that own the whole view but stay visible for
    /// gameplay screens and pause.
    #[test]
    fn hud_hidden_behind_title_not_behind_pause() {
        assert!(!hud_visible(&UiOpen::Title, false), "no HUD on the title screen");
        assert!(!hud_visible(&UiOpen::Title, true));
        assert!(!hud_visible(&UiOpen::Settings, true), "settings opened from the title is still the title view");
        assert!(hud_visible(&UiOpen::Settings, false), "settings from pause keeps the HUD");
        assert!(hud_visible(&UiOpen::None, false));
        assert!(hud_visible(&UiOpen::Pause, false), "pause overlay dims the HUD but keeps it");
        assert!(hud_visible(&UiOpen::Inventory, false));
        // N03: station/container screens are modals — no duplicate survival
        // HUD beneath their own inventory strips
        assert!(!hud_visible(&UiOpen::HandCraft, false), "hand-craft is a modal");
        assert!(!hud_visible(&UiOpen::CraftingTable, false), "the workbench is a modal");
        assert!(!hud_visible(&UiOpen::Furnace((0, 0, 0)), false));
        assert!(!hud_visible(&UiOpen::Chest((0, 0, 0)), false));
        assert!(!hud_visible(&UiOpen::Machine((0, 0, 0)), false));
    }

    /// N03: the input-recovery contract — E (the rebindable inventory key)
    /// closes every container/station screen, Escape closes everything
    /// (via close_ui), and neither leaks a screen that ignores both.
    #[test]
    fn inventory_key_escapes_every_container_screen() {
        let station_screens = [
            UiOpen::Inventory,
            UiOpen::HandCraft,
            UiOpen::CraftingTable,
            UiOpen::Furnace((3, 4, 5)),
            UiOpen::Chest((-1, 2, 7)),
            UiOpen::Machine((0, 0, 0)),
        ];
        for open in &station_screens {
            assert!(inventory_key_closes(open), "{:?} must close on the inventory key", open);
        }
        // screens where E means nothing (so typing in chat, walking, etc.
        // are untouched) — they still close via Escape's close_ui path
        for other in [UiOpen::None, UiOpen::Title, UiOpen::Pause, UiOpen::Chat,
                      UiOpen::QuestLog, UiOpen::TechTree, UiOpen::Map, UiOpen::Spellbook,
                      UiOpen::Trade(0), UiOpen::Book, UiOpen::Death, UiOpen::Paths] {
            assert!(!inventory_key_closes(&other), "{:?} must not react to the inventory key", other);
        }
    }

    /// N02: the queue head reports runnable vs blocked-with-reason, and
    /// the strip's data source is pure (no mutation while peeking).
    #[test]
    fn queue_status_reports_running_and_blocked() {
        use lf_game::survival::Inventory;
        // torch = 1 coal over 1 stick, makes 4
        let mut inv = Inventory::new();
        assert!(inv.add_item("coal", 1) == 0);
        assert!(inv.add_item("stick", 2) == 0);
        let queue = vec![("torch".to_string(), 1u32)];
        match queue_status(&queue, &inv) {
            QueueStatus::Running { output, remaining } => {
                assert_eq!((output.as_str(), remaining), ("torch", 1));
            }
            other => panic!("expected Running, got {:?}", other),
        }
        // spend the coal elsewhere -> blocked with the exact missing item
        assert_eq!(inv.remove_count("coal", 1), 1);
        match queue_status(&queue, &inv) {
            QueueStatus::Blocked { output, reason, .. } => {
                assert_eq!(output, "torch");
                assert!(reason.contains("coal"), "reason names the missing item: {reason}");
            }
            other => panic!("expected Blocked, got {:?}", other),
        }
        // empty queue is Empty, not blocked
        assert!(matches!(queue_status(&[], &inv), QueueStatus::Empty));
    }

    /// N02: an unknown output (mod unloaded) is an honest block, and the
    /// craft-entry lookup finds the real torch recipe with counts.
    #[test]
    fn catalog_entry_lookup_and_unknown_outputs() {
        let (ings, count) = catalog_craft_entry("torch").expect("torch is a craft recipe");
        assert_eq!(count, 4);
        assert!(ings.contains(&("coal".to_string(), 1)));
        assert!(ings.contains(&("stick".to_string(), 1)));
        assert!(catalog_craft_entry("definitely_not_an_item").is_none());
        let queue = vec![("definitely_not_an_item".to_string(), 2u32)];
        assert!(matches!(
            queue_status(&queue, &lf_game::survival::Inventory::new()),
            QueueStatus::Blocked { .. }
        ));
    }

    /// N03: the workbench zone layout is pure, shared with the vistest
    /// proofs, and stays coherent at every required size — normal
    /// three-pane ≥700px, two-pane drill-down below, zones never overlap,
    /// everything on-screen.
    #[test]
    fn workbench_layout_holds_at_required_sizes() {
        for (w, h) in [(640.0, 420.0), (800.0, 600.0), (1280.0, 800.0)] {
            let screen = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(w, h));
            let lay = workbench_layout(screen);
            assert_eq!(lay.compact, w < 700.0, "{w}x{h} picks the wrong mode");
            let mut zones = [("sidebar", lay.sidebar), ("list", lay.list),
                             ("detail", lay.detail), ("strip", lay.strip)];
            for (_, r) in zones.iter_mut() {
                // fully on-screen with margin
                assert!(r.left() >= 8.0 && r.top() >= 8.0
                    && r.right() <= w - 8.0 && r.bottom() <= h - 8.0,
                    "{w}x{h}: zone {:?} clipped", r);
            }
            if !lay.compact {
                // three distinct panes, strictly ordered, no overlap
                assert!(lay.sidebar.right() < lay.list.left());
                assert!(lay.list.right() < lay.detail.left());
                assert!(lay.detail.bottom() < lay.strip.top());
                assert!(lay.sidebar.bottom() < lay.strip.top());
            } else {
                // drill-down: one pane between chips row and the 1-row strip
                assert!(lay.sidebar.bottom() < lay.list.top());
                assert!(lay.list.bottom() <= lay.strip.top() + 0.5);
                assert!(lay.strip.height() < 100.0, "compact strip stays one row");
            }
        }
    }

    /// N01: the pinned objective always presents the first incomplete
    /// starter quest and its first incomplete objective, and falls silent
    /// when the whole chain is complete.
    #[test]
    fn pinned_objective_follows_the_starter_chain() {
        let mut log = crate::QuestLog::new();
        for q in crate::starter_quests() {
            log.add_quest(q);
        }
        let (title, progress) = pinned_objective(&log).expect("fresh chain pins q1");
        assert_eq!(title, "Punch a Tree");
        assert!(progress.starts_with("oak log 0/3"), "progress reads 'oak log 0/3', got {progress}");
        // advance the first objective to done: the line moves to q2
        log.quests[0].objectives[0].progress = 3;
        log.quests[0].objectives[0].completed = true;
        log.quests[0].completed = true;
        let (title2, progress2) = pinned_objective(&log).expect("q2 pins next");
        assert_eq!(title2, "Crafting Basics");
        assert!(progress2.contains("planks"), "q2 objective is planks, got {progress2}");
        // all complete: no pinned line
        for q in log.quests.iter_mut() {
            q.completed = true;
        }
        assert_eq!(pinned_objective(&log), None);
    }

    /// N01: the tutorial card + objective line stay inside the safe band at
    /// every required window size — above the hotbar band, clear of the
    /// minimap corner and the info line (the geometry the small-window
    /// proof also asserts in pixels).
    #[test]
    fn onboarding_rects_never_collide_at_required_sizes() {
        for (w, h) in [(640.0, 420.0), (800.0, 600.0), (1280.0, 800.0), (1600.0, 900.0)] {
            let screen = egui::Rect::from_min_max(
                egui::Pos2::ZERO, egui::pos2(w, h));
            let prect = onboarding_prompt_rect(screen);
            let orect = onboarding_objective_rect(screen, true);
            let orect_solo = onboarding_objective_rect(screen, false);
            // fully on-screen with a margin
            assert!(prect.left() >= 8.0 && prect.right() <= w - 8.0, "{w}x{h}: card clipped");
            assert!(orect.left() >= 8.0 && orect.right() <= w - 8.0, "{w}x{h}: line clipped");
            // clear of the hotbar band (bottom 130px)
            assert!(prect.bottom() < h - 130.0, "{w}x{h}: card hits the hotbar band");
            assert!(orect.bottom() < h - 130.0, "{w}x{h}: line hits the hotbar band");
            // clear of the minimap (top-right 160px) and the info line
            // (top-left 200px): a centered card must end left of the minimap
            // at 640 width and start right of the info text
            if w <= 720.0 {
                assert!(prect.right() < w - 150.0, "{w}x{h}: card reaches the minimap");
                assert!(prect.left() > 130.0, "{w}x{h}: card covers the info line");
            }
            // stacked, never intersecting; solo mode reuses the card slot
            assert!(!prect.intersects(orect), "{w}x{h}: card and line overlap");
            assert_eq!(orect_solo.center().y, prect.center().y);
        }
    }

    #[test]
    fn catalog_merges_all_stations() {
        let catalog = build_catalog();
        assert!(catalog.iter().any(|e| e.station == Station::Craft), "crafting recipes missing");
        assert!(catalog.iter().any(|e| e.station == Station::Smelt), "smelting recipes missing");
        assert!(catalog.iter().any(|e| e.station == Station::Alloy), "alloy recipes missing");
        assert!(catalog.iter().any(|e| e.station == Station::Crush), "crush recipes missing");
        // every output and ingredient is a real item
        for e in &catalog {
            assert!(item_def(&e.output).is_some(), "catalog output '{}' is not an item", e.output);
            for (id, _) in &e.ingredients {
                assert!(item_def(id).is_some(), "catalog ingredient '{}' is not an item", id);
            }
            assert!(e.output_count > 0);
            assert!(!e.ingredients.is_empty());
        }
    }

    #[test]
    fn catalog_aggregates_ingredients() {
        let catalog = build_catalog();
        let pick = catalog.iter().find(|e| e.output == "iron_pickaxe").expect("iron pickaxe recipe");
        let sticks = pick.ingredients.iter().find(|(id, _)| id == "stick").expect("sticks in pickaxe");
        assert_eq!(sticks.1, 2, "pickaxe needs exactly 2 sticks");
        let ingots = pick.ingredients.iter().find(|(id, _)| id == "iron_ingot").expect("ingots in pickaxe");
        assert_eq!(ingots.1, 3);
        assert_eq!(pick.grid_size, 3);
    }

    #[test]
    fn quick_insert_merges_and_fills() {
        let mut slots = vec![Some(ItemStack { item_id: "stone".into(), count: 10 }), None, None];
        let mut stack = ItemStack { item_id: "stone".into(), count: 64 };
        quick_insert(&mut slots, &mut stack);
        assert_eq!(slots[0].as_ref().unwrap().count, 64, "fills the existing stack first");
        assert_eq!(slots[1].as_ref().unwrap().count, 10, "spills the remainder into empty slots");
        assert_eq!(stack.count, 0, "stone stacks to 64 so everything fits");
        // tools never stack: two picks end up in two slots
        let mut slots = vec![None, None];
        let mut stack = ItemStack { item_id: "iron_pickaxe".into(), count: 2 };
        quick_insert(&mut slots, &mut stack);
        assert_eq!(slots[0].as_ref().unwrap().count, 1);
        assert_eq!(slots[1].as_ref().unwrap().count, 1);
        assert_eq!(stack.count, 0);
    }

    #[test]
    fn take_one_pulls_single_items() {
        let mut slots = vec![Some(ItemStack { item_id: "log".into(), count: 2 })];
        let one = take_one(&mut slots, "log").unwrap();
        assert_eq!((one.item_id.as_str(), one.count), ("log", 1));
        assert_eq!(slots[0].as_ref().unwrap().count, 1);
        take_one(&mut slots, "log").unwrap();
        assert!(slots[0].is_none(), "emptied slot clears");
        assert!(take_one(&mut slots, "log").is_none(), "nothing left to take");
    }
}

/// Human-friendly "how long ago" for the slot list.
/// Workbench visibility shortcut: the always-known survival set (the
/// inventory's by-hand crafting route shows only these).
fn kit_workbench_always(output: &str) -> bool {
    crate::workbench::ALWAYS_VISIBLE.contains(&output)
}

fn time_ago(secs: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let delta = now.saturating_sub(secs);
    if delta < 60 {
        "just now".into()
    } else if delta < 3600 {
        format!("{}m ago", delta / 60)
    } else if delta < 86400 {
        format!("{}h ago", delta / 3600)
    } else {
        format!("{}d ago", delta / 86400)
    }
}

/// Thousands-separated seed for the version display (B1: "Seed:
/// 14,203,847,923" — a detail players notice and share).
fn format_seed(seed: u64) -> String {
    let s = seed.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

/// The Load World thumbnail for a slot: the cached `thumbnail.png` seed
/// render if one exists (generated here on first open), else the live
/// autosave view, else a freshly generated seed thumbnail that is written
/// to the slot for next time.
fn load_slot_thumbnail(ui: &egui::Ui, name: &str, seed: u64, world_type: lf_worldgen::WorldType) -> Option<egui::TextureHandle> {
    let dir = crate::slots::slot_dir(name);
    let path = dir.join("thumbnail.png");
    // try the cache, then the older live-view thumb.png
    let decoded: Option<image::RgbaImage> = (|| {
        for p in [path.clone(), dir.join("thumb.png")] {
            if let Ok(mut r) = image::ImageReader::open(&p) {
                if let Ok(d) = r.decode() {
                    return Some(d.to_rgba8());
                }
            }
        }
        None
    })();
    match decoded {
        Some(img) => {
            let size = [img.width() as usize, img.height() as usize];
            let color = egui::ColorImage::from_rgba_unmultiplied(size, &img);
            Some(ui.ctx().load_texture(format!("thumb_{name}"), color, egui::TextureOptions::LINEAR))
        }
        None => {
            // generate once from the seed and cache it (C2)
            let side = 4usize;
            let rgba = crate::map::seed_thumbnail_rgba(seed, world_type, side);
            let side_px = (side * 16) as u32;
            let img = image::RgbaImage::from_fn(side_px, side_px, |x, y| {
                let i = ((y * side_px + x) * 4) as usize;
                image::Rgba([rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]])
            });
            let _ = std::fs::create_dir_all(&dir);
            let _ = img.save(&path);
            let color = egui::ColorImage::from_rgba_unmultiplied([side_px as usize, side_px as usize], &rgba);
            Some(ui.ctx().load_texture(format!("thumb_{name}"), color, egui::TextureOptions::LINEAR))
        }
    }
}

/// Placeholder tile color while a thumbnail cannot exist yet — keyed by
/// world type so the placeholder still tells the truth.
fn placeholder_color(world_type: lf_worldgen::WorldType) -> Color32 {
    match world_type {
        lf_worldgen::WorldType::Normal => Color32::from_rgb(0x3a, 0x4a, 0x2e),
        lf_worldgen::WorldType::Superflat => Color32::from_rgb(0x4a, 0x42, 0x2e),
        lf_worldgen::WorldType::Amplified => Color32::from_rgb(0x4a, 0x32, 0x22),
    }
}

/// Tiny per-type glyph drawn next to the world name (C2): mountains for
/// Normal, a flat line for Superflat, a tall peak for Amplified.
fn world_type_glyph(world_type: lf_worldgen::WorldType) -> &'static str {
    match world_type {
        lf_worldgen::WorldType::Normal => "▲▲",
        lf_worldgen::WorldType::Superflat => "▭",
        lf_worldgen::WorldType::Amplified => "△",
    }
}

/// Spawn the dedicated server binary for a slot (C3 Host World). Returns a
/// user-facing status line: honest about a missing binary instead of
/// pretending a server started.
fn start_dedicated_server(slot: &str, port: &str) -> String {
    let candidates = ["target/release/loreforge-server", "./loreforge-server", "../loreforge-server"];
    let bin = candidates.iter().find(|c| std::path::Path::new(c).exists());
    match bin {
        Some(bin) => {
            let port_num: u16 = port.parse().unwrap_or(25565);
            match std::process::Command::new(bin)
                .arg("--world").arg(slot)
                .arg("--port").arg(port_num.to_string())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(_) => format!("server for '{}' starting on :{} — connect to 127.0.0.1:{}", slot, port_num, port_num),
                Err(e) => format!("could not launch server: {}", e),
            }
        }
        None => "no server binary found — build apps/loreforge-server (cargo build --release) to host".into(),
    }
}

