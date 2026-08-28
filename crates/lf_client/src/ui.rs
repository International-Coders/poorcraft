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

// ------------------------------------------------------------------

/// Gameplay HUD visibility: hidden behind menus that own the whole view.
/// The audit caught hearts + hotbar rendering under the title menu (and
/// under settings opened from the title); pause keeps the HUD visible the
/// way Minecraft-style pause overlays do.
fn hud_visible(ui_open: &UiOpen, settings_from_title: bool) -> bool {
    !matches!(ui_open, UiOpen::Title)
        && !(*ui_open == UiOpen::Settings && settings_from_title)
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
            UiOpen::Inventory => self.draw_workbench(ctx, true),
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
                .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(8.0, -150.0))
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
                // selected item name, fading after a switch
                let name = self.inventory.slots[self.hotbar_index].as_ref()
                    .and_then(|s| item_def(&s.item_id).map(|d| d.name.to_string()));
                let alpha = (self.hotbar_pick_time * 255.0).min(255.0) as u8;
                match (name, alpha) {
                    (Some(n), a) if a > 8 => {
                        ui.label(egui::RichText::new(n).small().color(
                            egui::Color32::from_rgba_unmultiplied(Theme::ACCENT.r(), Theme::ACCENT.g(), Theme::ACCENT.b(), a)));
                    }
                    _ => { ui.label(egui::RichText::new("").small()); }
                }
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
        // info line (top-left): facing, biome, coords, clock, weather, net, FPS
        let facing = crate::map::compass_facing(self.player.yaw);
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
        egui::Area::new(egui::Id::new("info_line"))
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 8.0))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new(info).small().color(egui::Color32::from_rgba_premultiplied(Theme::TEXT.r(), Theme::TEXT.g(), Theme::TEXT.b(), 200)));
            });
        // lore-and-visuals A3/C4: companion status tiles under the info
        // line (one per active companion, trust + morale bars, state chip)
        if !self.companions.is_empty() {
            egui::Area::new(egui::Id::new("companion_tiles"))
                .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 26.0))
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

        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ctx, |ui| {
                kit::vignette(ui, 190);
                let screen = ctx.screen_rect();
                // dark wash: the workbench is a screen, not a tooltip —
                // text must out-shout the world behind it
                ui.painter_at(screen).rect_filled(screen, 0.0,
                    Color32::from_rgba_unmultiplied(kit::Theme::BG.r(), kit::Theme::BG.g(), kit::Theme::BG.b(), 195));
                let strip_h = 4.0 * (SLOT_SIZE + 8.0) + 32.0;
                let zone_h = screen.height() - strip_h - 48.0;
                // zone 1: category sidebar (~15%)
                let sidebar_w = (screen.width() * 0.15).clamp(130.0, 190.0);
                let list_w = (screen.width() * 0.34).clamp(240.0, 420.0);
                let detail_w = screen.width() - sidebar_w - list_w - 48.0;
                // header
                ui.painter().text(
                    egui::Pos2::new(screen.left() + 24.0, screen.top() + 20.0),
                    egui::Align2::LEFT_CENTER,
                    if basic_only { "CRAFT — by hand" } else { "CRAFTING TABLE" },
                    egui::FontId::proportional(24.0), Theme::TEXT);
                ui.painter().text(
                    egui::Pos2::new(screen.right() - 24.0, screen.top() + 20.0),
                    egui::Align2::RIGHT_CENTER,
                    "press E or Esc to close",
                    egui::FontId::proportional(11.0), Theme::TEXT_DISABLED);
                ui.add_space(36.0);
                ui.horizontal(|ui| {
                    // ---------- Zone 1: categories ----------
                    ui.allocate_ui(egui::vec2(sidebar_w, zone_h), |ui| {
                        ui.vertical(|ui| {
                            for (i, cat_ref) in workbench::CATEGORIES.iter().enumerate() {
                                let cat = *cat_ref;
                                let selected = i == self.wb_category;
                                let cat_entries: Vec<&(&CatalogEntry, bool, bool)> = visible.iter()
                                    .filter(|(e, _, _)| workbench::categorize(&e.output) == cat)
                                    .collect();
                                let craftable_n = cat_entries.iter().filter(|(_, can, _)| *can).count();
                                let total_n = cat_entries.len() + locked.iter()
                                    .filter(|e| workbench::categorize(&e.output) == cat).count();
                                let _ = selected;
                                let (rect, resp) = ui.allocate_exact_size(
                                    egui::vec2(sidebar_w - 8.0, 30.0), egui::Sense::click());
                                if selected {
                                    // left accent border — NOT a filled background
                                    ui.painter().rect_filled(
                                        egui::Rect::from_min_size(rect.left_top(), egui::vec2(3.0, rect.height())),
                                        0.0, Theme::ACCENT);
                                }
                                let text_col = if selected { Theme::TEXT }
                                    else if resp.hovered() { Theme::TEXT_BRIGHT }
                                    else { egui::Color32::from_rgb(0xb5, 0xa8, 0x93) };
                                let icon_x = rect.left() + 12.0;
                                let icon_rect = egui::Rect::from_center_size(
                                    egui::Pos2::new(icon_x, rect.center().y), egui::vec2(18.0, 18.0));
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
                            // queue badge lives at the sidebar's foot
                            ui.add_space(8.0);
                            if !self.craft_queue.is_empty() {
                                ui.painter().text(
                                    egui::Pos2::new(ui.cursor().min.x + 8.0, ui.cursor().min.y + 8.0),
                                    egui::Align2::LEFT_TOP,
                                    format!("Queue: {}", self.craft_queue.len()),
                                    egui::FontId::proportional(11.0), Theme::WARNING);
                            }
                        });
                    });
                    ui.add_space(8.0);
                    // ---------- Zone 2: recipe list ----------
                    ui.allocate_ui(egui::vec2(list_w, zone_h), |ui| {
                        ui.vertical(|ui| {
                            let cat = workbench::CATEGORIES[self.wb_category.min(workbench::CATEGORIES.len() - 1)];
                            egui::ScrollArea::vertical().max_height(zone_h - 8.0).show(ui, |ui| {
                                let mut rows: Vec<&(&CatalogEntry, bool, bool)> = visible.iter()
                                    .filter(|(e, _, _)| workbench::categorize(&e.output) == cat)
                                    .collect();
                                let locked_rows: Vec<&&CatalogEntry> = locked.iter()
                                    .filter(|e| workbench::categorize(&e.output) == cat).collect();
                                if rows.is_empty() && locked_rows.is_empty() {
                                    ui.label(egui::RichText::new("Nothing here yet. Gather, and the workbench will teach you.")
                                        .color(Theme::TEXT_DIM).size(12.0));
                                }
                                for (e, can_craft, partial) in rows.drain(..) {
                                    let name = item_def(&e.output).map(|d| d.name).unwrap_or(&e.output);
                                    let selected = self.wb_selected.as_deref() == Some(e.output.as_str());
                                    // row height follows its content: one line for
                                    // the name, one for the material summary
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
                                    // craftable check is DRAWN (the shipped
                                    // font has no check glyph); a dot otherwise
                                    if *can_craft {
                                        kit::paint_check(ui.painter(),
                                            egui::Pos2::new(rect.right() - 14.0, rect.center().y),
                                            Theme::OK, 1.8);
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
                        });
                    });
                    ui.add_space(8.0);
                    // ---------- Zone 3: detail panel ----------
                    ui.allocate_ui(egui::vec2(detail_w, zone_h), |ui| {
                        ui.vertical(|ui| {
                            let cat = workbench::CATEGORIES[self.wb_category.min(workbench::CATEGORIES.len() - 1)];
                            let selected_entry = visible.iter()
                                .find(|(e, _, _)| self.wb_selected.as_deref() == Some(e.output.as_str()))
                                .map(|(e, can, _)| (*e, *can));
                            let Some((e, can_craft)) = selected_entry else {
                                // empty state: the category speaks
                                ui.add_space(zone_h * 0.30);
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
                            // the single craft action
                            if all_available {
                                if kit::menu_link(ui, &format!("Craft {}", qty), "wb-craft", 1.0, true, true) {
                                    self.craft_from_workbench(&e.ingredients, &e.output, e.output_count, qty);
                                    self.wb_qty = 1;
                                }
                            } else {
                                ui.painter().text(
                                    egui::Pos2::new(ui.cursor().min.x + 8.0, ui.cursor().min.y + 14.0),
                                    egui::Align2::LEFT_CENTER, "Missing materials",
                                    egui::FontId::proportional(15.0), Theme::TEXT_DISABLED);
                            }
                            ui.add_space(4.0);
                            if kit::menu_link(ui, "Add to Queue", "wb-queue", 1.0, false, true) {
                                self.craft_queue.push((e.output.clone(), qty));
                                self.chat_log.push(format!("queued: {}x {}", qty, name));
                                if self.chat_log.len() > 6 { self.chat_log.remove(0); }
                            }
                        });
                    });
                });
                // ---------- inventory strip ----------
                let strip_y = screen.bottom() - strip_h;
                ui.painter().line_segment(
                    [egui::Pos2::new(screen.left() + 16.0, strip_y - 10.0),
                     egui::Pos2::new(screen.right() - 16.0, strip_y - 10.0)],
                    egui::Stroke::new(1.0, Theme::BORDER));
                ui.allocate_ui_at_rect(
                    egui::Rect::from_min_size(
                        egui::Pos2::new(screen.left() + 24.0, strip_y),
                        egui::vec2(screen.width() - 48.0, strip_h)), |ui| {
                        ui.vertical(|ui| {
                            // needed-ingredient highlight map
                            let needed: Vec<String> = self.wb_selected.as_ref().and_then(|sel| {
                                visible.iter().find(|(e, _, _)| e.output == *sel)
                                    .map(|(e, _, _)| e.ingredients.iter().map(|(id, _)| id.clone()).collect())
                            }).unwrap_or_default();
                            for row in 0..4 {
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
                    });
            });
    }

    /// Consume a batch from the inventory and grant the output (the craft
    /// button's action). Ingredients are re-verified here; the UI button is
    /// only enabled when they check out, but state can change between frames.
    fn craft_from_workbench(&mut self, ingredients: &[(String, u8)], output: &str,
                            output_count: u8, qty: u32) {
        // verify
        for (id, n) in ingredients {
            let need = (*n as u32) * qty;
            let got: u32 = self.inventory.slots.iter().take(36).flatten()
                .filter(|s| s.item_id == *id)
                .map(|s| s.count as u32).sum();
            if got < need {
                self.chat_log.push(format!("missing materials for {}", output));
                return;
            }
        }
        // consume
        for (id, n) in ingredients {
            let mut left = (*n as u32) * qty;
            for slot in self.inventory.slots.iter_mut().take(36) {
                if left == 0 { break; }
                if let Some(s) = slot {
                    if s.item_id == *id {
                        let take = (s.count as u32).min(left) as u8;
                        s.count -= take;
                        left -= take as u32;
                        if s.count == 0 { *slot = None; }
                    }
                }
            }
        }
        // grant
        let total = output_count as u32 * qty;
        let mut remaining = total;
        while remaining > 0 {
            let batch = remaining.min(u8::MAX as u32) as u8;
            let leftover = self.inventory.add_item(output, batch);
            if leftover > 0 {
                self.spawn_drop(output, leftover, self.player.eye_position());
                break;
            }
            remaining -= batch as u32;
        }
        self.quest_event(QuestEvent::Crafted(output.to_string()));
    }

    fn draw_furnace(&mut self, ctx: &egui::Context, pos: (i32, i32, i32)) {
        let Some(BlockEntity::Furnace(mut furnace)) = self.block_entities.get(&pos).cloned() else {
            self.close_ui();
            return;
        };
        egui::Window::new("Furnace")
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -30.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
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
        self.block_entities.insert(pos, BlockEntity::Furnace(furnace));
    }

    fn draw_chest(&mut self, ctx: &egui::Context, pos: (i32, i32, i32)) {
        let mut chest_slots = match self.block_entities.get(&pos).cloned() {
            Some(BlockEntity::Chest { slots }) => slots,
            _ => vec![None; 27],
        };
        egui::Window::new("Chest")
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -30.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
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
        let s = &mut self.settings;
        kit::section_header(ui, "Gameplay", 1.0);
        ui.add_space(6.0);
        kit::setting_slider(ui, "Mouse sensitivity", &mut s.sensitivity, (0.0005, 0.01), &|v| format!("{:.1}", v * 1000.0));
        kit::toggle(ui, "Invert mouse Y", &mut s.invert_y);
        kit::toggle(ui, "Show FPS", &mut s.show_fps);
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
        egui::Window::new(format!("{} — {}", villager.name, crate::factions::job_label(villager.job)))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -30.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
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
                    self.add_standing(&faction, bump);
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
        egui::Window::new(format!("{} — commands", c.display_name))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -30.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
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
        egui::Window::new("Lore Book")
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -30.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
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
        egui::Window::new("Smithing Forge")
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -30.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
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
        egui::Window::new(title)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -30.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
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
        egui::Window::new("Technology — K to close")
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -20.0))
            .min_size(egui::vec2(640.0, 380.0))
            .collapsible(false)
            .show(ctx, |ui| {
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

