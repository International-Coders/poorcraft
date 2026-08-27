//! egui integration: platform plumbing plus HUD, inventory/crafting screens,
//! the recipe book, container screens and the death screen. All immediate-
//! mode drawing against GameState, styled by the ui_kit design system and
//! the real pixel-art icons from `crate::icons`.

use egui_wgpu::Renderer;
use image;
use egui_winit::State as EguiWinitState;

use crate::ui_kit::{self as kit, Theme};
use crate::{BlockEntity, GameState, RtMode, UiOpen};
use lf_game::crafting::{consume_ingredients, match_recipe};
use lf_game::items::{item_def, ItemKind};
use lf_game::research::Era;
use lf_game::survival::ItemStack;
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
        egui::FontId::proportional(13.0), egui::Color32::WHITE);
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
            UiOpen::Inventory => self.draw_inventory(ctx, 2),
            UiOpen::CraftingTable => self.draw_inventory(ctx, 3),
            UiOpen::Furnace(pos) => self.draw_furnace(ctx, pos),
            UiOpen::Chest(pos) => self.draw_chest(ctx, pos),
            UiOpen::Trade(index) => self.draw_trade(ctx, index),
            UiOpen::Book => self.draw_book(ctx),
            UiOpen::Smithing => self.draw_smithing(ctx),
            UiOpen::Machine(pos) => self.draw_machine(ctx, pos),
            UiOpen::TechTree => self.draw_tech_tree(ctx),
            UiOpen::Map => self.draw_map_screen(ctx),
            UiOpen::Console => self.draw_console(ctx),
            UiOpen::Slots => self.draw_slots(ctx),
            UiOpen::Death => self.draw_death(ctx),
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
                        egui::FontId::proportional(12.0), egui::Color32::WHITE);
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
                    ui.label(egui::RichText::new(format!("air {}", "·".repeat(self.air as usize))).color(Theme::XP));
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

    fn draw_inventory(&mut self, ctx: &egui::Context, grid: usize) {
        let title = if grid == 3 { "Crafting Table" } else { "Inventory" };
        let book = self.recipe_book_open;
        egui::Window::new(title)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -30.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            // crafting grid
                            ui.vertical(|ui| {
                                kit::section_header(ui, if grid == 3 { "Craft" } else { "2x2" }, 1.0);
                                ui.add_space(8.0);
                                for row in 0..grid {
                                    ui.horizontal(|ui| {
                                        for col in 0..grid {
                                            let idx = row * grid + col;
                                            let mut stack = self.craft_grid[idx].clone();
                                            let mut cursor = self.cursor_stack.take();
                                            let out = slot_button(ui, &mut stack, &mut cursor, false, &self.icons);
                                            self.cursor_stack = cursor;
                                            if let Some(mut q) = out.quick_moved {
                                                quick_insert(&mut self.inventory.slots[..36], &mut q);
                                            }
                                            self.craft_grid[idx] = stack;
                                        }
                                    });
                                }
                            });
                            ui.add_space(10.0);
                            // result slot
                            ui.vertical(|ui| {
                                ui.add_space(12.0);
                                let grid_ref: Vec<Option<ItemStack>> = self.craft_grid.iter().take(grid * grid).cloned().collect();
                                let result = match_recipe(&grid_ref);
                                let locked = match &result {
                                    Some((out, _)) => Era::required_for(out) > self.research.era,
                                    None => false,
                                };
                                let (rect, response) = ui.allocate_exact_size(egui::vec2(SLOT_SIZE + 8.0, SLOT_SIZE + 8.0), egui::Sense::click());
                                ui.painter().rect_filled(rect, 6.0, egui::Color32::from_black_alpha(170));
                                ui.painter().rect_stroke(rect, 6.0, egui::Stroke::new(2.0,
                                    if result.is_some() && !locked { Theme::ACCENT } else { egui::Color32::from_gray(90) }), egui::StrokeKind::Middle);
                                if let Some((out, n)) = &result {
                                    let stack = ItemStack { item_id: out.clone(), count: *n };
                                    paint_item(ui, rect.shrink(4.0), &stack, &self.icons);
                                    paint_count(ui, rect, *n);
                                    if locked {
                                        // dark veil + era note
                                        ui.painter().rect_filled(rect, 6.0, egui::Color32::from_black_alpha(140));
                                        ui.painter().text(rect.center_bottom() + egui::vec2(0.0, 10.0), egui::Align2::CENTER_CENTER,
                                            format!("needs {}", Era::required_for(out).name()),
                                            egui::FontId::proportional(10.0), egui::Color32::from_rgb(230, 130, 130));
                                    } else {
                                        kit::hover_item_tooltip(&response, &stack, &self.icons);
                                    }
                                }
                                if response.clicked() {
                                    if let Some((out, n)) = result {
                                        if !locked {
                                            let crafted = ItemStack { item_id: out, count: n };
                                            let can_take = match &self.cursor_stack {
                                                None => true,
                                                Some(c) => c.item_id == crafted.item_id
                                                    && c.count as u16 + n as u16 <= item_def(&crafted.item_id).map(|d| d.max_stack).unwrap_or(64) as u16,
                                            };
                                            if can_take {
                                                match &mut self.cursor_stack {
                                                    None => self.cursor_stack = Some(crafted.clone()),
                                                    Some(c) => c.count += n,
                                                }
                                                let mut grid_slots: Vec<Option<ItemStack>> =
                                                    self.craft_grid.iter().take(grid * grid).cloned().collect();
                                                consume_ingredients(&mut grid_slots);
                                                for (i, s) in grid_slots.into_iter().enumerate() {
                                                    self.craft_grid[i] = s;
                                                }
                                                self.quest_event(crate::QuestEvent::Crafted(crafted.item_id.clone()));
                                            }
                                        }
                                    }
                                }
                            });
                        });
                        ui.add_space(8.0);
                        // storage 9x3 (shift-click bounces to the hotbar and back)
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
                        // hotbar row (shift-click bounces to storage)
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
                    });
                    if book {
                        self.draw_recipe_book(ui, grid);
                    } else {
                        ui.vertical(|ui| {
                            ui.add_space(60.0);
                            if kit::menu_button(ui, "Recipes", 1.0, false) {
                                self.recipe_book_open = true;
                            }
                            ui.label(egui::RichText::new("browse &\nauto-fill").small().color(Theme::TEXT_DIM));
                        });
                    }
                });
            });
    }

    /// The recipe book side panel: unified catalog with search, station
    /// filters, have/need coloring and click-to-auto-fill.
    fn draw_recipe_book(&mut self, ui: &mut egui::Ui, grid: usize) {
        let catalog = build_catalog();
        let have: std::collections::HashMap<String, u16> = {
            let mut h = std::collections::HashMap::new();
            for s in self.inventory.slots.iter().take(36).flatten() {
                *h.entry(s.item_id.clone()).or_insert(0) += s.count as u16;
            }
            h
        };
        ui.vertical(|ui| {
            ui.set_width(340.0);
            kit::section_header(ui, "Recipe Book", 1.0);
            if kit::menu_button(ui, "× close", 1.0, false) {
                self.recipe_book_open = false;
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.recipe_search).desired_width(150.0).hint_text("search..."));
                let stations = [("All", usize::MAX), ("Craft", 0), ("Smelt", 1), ("Alloy", 2), ("Crush", 3)];
                for (label, idx) in stations {
                    let on = if idx == usize::MAX { self.recipe_station == usize::MAX } else { self.recipe_station == idx };
                    if ui.add(egui::Button::new(egui::RichText::new(label)
                        .color(if on { Theme::ACCENT } else { Theme::TEXT_DIM }))).clicked() {
                        self.recipe_station = idx;
                    }
                }
            });
            if ui.add(egui::Button::new(egui::RichText::new(if self.recipe_craftable_only {
                "✓ craftable only"
            } else {
                "craftable only"
            }).color(if self.recipe_craftable_only { Theme::OK } else { Theme::TEXT_DIM }))).clicked() {
                self.recipe_craftable_only = !self.recipe_craftable_only;
            }
            ui.add_space(4.0);
            let search = self.recipe_search.to_lowercase();
            let station_of = |e: &CatalogEntry| match e.station {
                Station::Craft => 0,
                Station::Smelt => 1,
                Station::Alloy => 2,
                Station::Crush => 3,
            };
            egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                for entry in &catalog {
                    if self.recipe_station != usize::MAX && station_of(entry) != self.recipe_station {
                        continue;
                    }
                    let name = item_def(&entry.output).map(|d| d.name).unwrap_or(&entry.output);
                    if !search.is_empty() && !name.to_lowercase().contains(&search) {
                        continue;
                    }
                    let era_ok = Era::required_for(&entry.output) <= self.research.era;
                    let have_all = entry.ingredients.iter().all(|(id, n)| have.get(id).copied().unwrap_or(0) >= *n as u16);
                    let fits_grid = entry.station != Station::Craft || entry.grid_size <= grid;
                    let craftable = era_ok && have_all && fits_grid;
                    if self.recipe_craftable_only && !craftable {
                        continue;
                    }
                    let selected_entry = entry.station == Station::Craft && fits_grid;
                    egui::Frame::new()
                        .fill(if craftable { egui::Color32::from_rgba_premultiplied(34, 40, 32, 220) }
                              else { egui::Color32::from_black_alpha(150) })
                        .stroke(egui::Stroke::new(if craftable { 1.6 } else { 1.0 },
                            if craftable { Theme::ACCENT_DIM } else { egui::Color32::from_gray(60) }))
                        .corner_radius(7.0)
                        .inner_margin(6.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (orect, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
                                paint_item(ui, orect, &ItemStack { item_id: entry.output.clone(), count: 1 }, &self.icons);
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(name).size(13.0)
                                            .color(if era_ok { Theme::TEXT } else { egui::Color32::from_rgb(230, 130, 130) }));
                                        if entry.output_count > 1 {
                                            ui.label(egui::RichText::new(format!("x{}", entry.output_count)).small().color(Theme::TEXT_DIM));
                                        }
                                        if !era_ok {
                                            ui.label(egui::RichText::new(format!("[{}]", Era::required_for(&entry.output).name())).small().color(Theme::BAD));
                                        } else if entry.station == Station::Craft && !fits_grid {
                                            ui.label(egui::RichText::new("[needs table]").small().color(Theme::TEXT_DIM));
                                        }
                                    });
                                    // ingredient icons with have/need counts
                                    ui.horizontal(|ui| {
                                        for (id, n) in &entry.ingredients {
                                            let got = have.get(id).copied().unwrap_or(0);
                                            let ok = got >= *n as u16;
                                            let (irect, _) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                                            paint_item(ui, irect, &ItemStack { item_id: id.clone(), count: 1 }, &self.icons);
                                            ui.label(egui::RichText::new(format!("{}", n)).small()
                                                .color(if ok { Theme::OK } else { Theme::BAD }));
                                            let short_name = item_def(id).map(|d| d.name).unwrap_or(id);
                                            ui.label(egui::RichText::new(short_name).small().color(Theme::TEXT_DIM));
                                        }
                                    });
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(entry.station.label()).small().color(Theme::TEXT_DIM));
                                    if selected_entry && ui.add_enabled(craftable, egui::Button::new("fill")).clicked() {
                                        if let Some(pattern) = &entry.pattern {
                                            self.autofill_recipe(pattern);
                                        }
                                    }
                                });
                            });
                            // hover preview: pattern mini-grid + output tooltip
                            let resp = ui.allocate_response(egui::vec2(ui.available_width(), 0.0), egui::Sense::hover());
                            let entry_out = entry.output.clone();
                            let icons_ptr = &self.icons;
                            let pattern = entry.pattern.clone();
                            resp.on_hover_ui(|ui| {
                                let stack = ItemStack { item_id: entry_out.clone(), count: 1 };
                                kit::item_tooltip_body(ui, &stack, icons_ptr);
                                if let Some(pattern) = &pattern {
                                    ui.add_space(4.0);
                                    let h = pattern.len();
                                    let w = pattern.iter().map(|r| r.len()).max().unwrap_or(0);
                                    ui.horizontal(|ui| {
                                        for row in 0..h {
                                            ui.vertical(|ui| {
                                                for col in 0..w {
                                                    let cell = pattern[row].get(col).copied().flatten();
                                                    let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                                                    let p = ui.painter();
                                                    p.rect_filled(rect, 4.0, egui::Color32::from_black_alpha(170));
                                                    p.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_gray(80)), egui::StrokeKind::Middle);
                                                    if let Some(id) = cell {
                                                        paint_item(ui, rect, &ItemStack { item_id: id.to_string(), count: 1 }, icons_ptr);
                                                    }
                                                }
                                            });
                                        }
                                    });
                                }
                            });
                        });
                }
                if catalog.is_empty() {
                    ui.label(egui::RichText::new("no recipes match").small().color(Theme::TEXT_DIM));
                }
            });
        });
    }

    /// Move the current grid contents back to the inventory, then pull the
    /// recipe's ingredients into the grid (only if everything is available).
    fn autofill_recipe(&mut self, pattern: &[Vec<Option<&'static str>>]) {
        // return whatever is in the grid first
        let grid = std::mem::take(&mut self.craft_grid);
        for s in grid.into_iter().flatten() {
            let leftover = self.inventory.add_item(&s.item_id, s.count);
            if leftover > 0 {
                self.spawn_drop(&s.item_id, leftover, self.player.eye_position() + self.player.look_dir());
            }
        }
        // count needs, verify availability
        let mut needed: std::collections::HashMap<&str, u8> = std::collections::HashMap::new();
        for row in pattern {
            for cell in row.iter().flatten() {
                *needed.entry(cell).or_insert(0) += 1;
            }
        }
        for (id, n) in &needed {
            let got: u16 = self.inventory.slots.iter().take(36).flatten()
                .filter(|s| s.item_id == *id)
                .map(|s| s.count as u16).sum();
            if got < *n as u16 {
                return; // not enough — leave the grid empty rather than partial
            }
        }
        for (y, row) in pattern.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if let Some(id) = cell {
                    self.craft_grid[y * 3 + x] = take_one(&mut self.inventory.slots, id);
                }
            }
        }
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

    fn draw_quest_log(&mut self, ctx: &egui::Context) {
        egui::Window::new("Quest Log — J to close")
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(12.0, 12.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                for quest in &self.quest_log.quests {
                    let heading_color = if quest.completed {
                        egui::Color32::from_rgb(120, 200, 120)
                    } else {
                        egui::Color32::from_rgb(240, 210, 140)
                    };
                    ui.heading(egui::RichText::new(&quest.title).size(18.0).color(heading_color));
                    ui.label(egui::RichText::new(format!("Act {} — {}", quest.act, quest.description)).small());
                    for obj in &quest.objectives {
                        let mark = if obj.completed { "[x]" } else { "[ ]" };
                        ui.label(format!("  {} {} — {}/{}", mark, obj.target, obj.progress.min(obj.count), obj.count));
                    }
                    ui.add_space(6.0);
                }
                if !self.chronicle.is_empty() {
                    ui.separator();
                    ui.heading("Chronicle");
                    let md = lf_chronicle::SagaGenerator::export_markdown(&self.chronicle);
                    egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                        ui.label(egui::RichText::new(md).small().monospace());
                    });
                }
            });
    }

    fn draw_title(&mut self, ctx: &egui::Context) {
        // subtle dark vignette over the orbiting world background
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(90)))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.add_space(ui.available_height() * 0.10);
                    let t = self.menu_reveal;
                    let reveal = kit::ease_out_cubic((t / 0.7).clamp(0.0, 1.0));
                    let glow = Theme::title_glow(t);
                    ui.label(egui::RichText::new("LOREFORGE")
                        .size(64.0 * (0.8 + 0.2 * reveal))
                        .color(glow)
                        .strong());
                    let sub_r = kit::ease_out_cubic(((t - 0.4) / 0.6).clamp(0.0, 1.0));
                    ui.label(egui::RichText::new("a voxel saga of forge & industry")
                        .size(17.0)
                        .color(egui::Color32::from_rgba_premultiplied(Theme::TEXT.r(), Theme::TEXT.g(), Theme::TEXT.b(), ((200.0 * sub_r) as u8).min(255))));
                    ui.add_space(26.0);
                    let btn = |ui: &mut egui::Ui, label: &str, delay: f32, accent: bool| -> bool {
                        let r = ((t - delay) / 0.45).clamp(0.0, 1.0);
                        kit::menu_button(ui, label, r, accent)
                    };
                    if btn(ui, &format!("Play — {}", self.slot_meta.name), 0.7, true) {
                        self.close_ui();
                    }
                    ui.add_space(8.0);
                    if btn(ui, "New World", 0.85, false) {
                        self.title_show_new = !self.title_show_new;
                    }
                    if self.title_show_new {
                        ui.horizontal(|ui| {
                            if btn(ui, "Normal", 0.95, false) {
                                self.new_world_named(&format!("World {}", crate::slots::list_slots().len() + 1),
                                    lf_worldgen::WorldType::Normal);
                            }
                            ui.add_space(6.0);
                            if btn(ui, "Superflat", 1.0, false) {
                                self.new_world_named(&format!("World {}", crate::slots::list_slots().len() + 1),
                                    lf_worldgen::WorldType::Superflat);
                            }
                            ui.add_space(6.0);
                            if btn(ui, "Amplified", 1.05, false) {
                                self.new_world_named(&format!("World {}", crate::slots::list_slots().len() + 1),
                                    lf_worldgen::WorldType::Amplified);
                            }
                        });
                    }
                    ui.add_space(8.0);
                    if btn(ui, "Load Game", 1.1, false) {
                        self.ui_open = UiOpen::Slots;
                        self.menu_reveal = 0.0;
                    }
                    ui.add_space(8.0);
                    let transport = if lf_steam::preferred_transport() == lf_steam::Transport::Udp {
                        "localhost"
                    } else {
                        "Steam P2P"
                    };
                    if btn(ui, &format!("Multiplayer ({})", transport), 1.2, false) {
                        match crate::net::NetClient::connect("127.0.0.1:25565", "smith") {
                            Ok(n) => {
                                self.net = Some(n);
                                self.chat_log = vec!["joining localhost:25565...".to_string()];
                                self.close_ui();
                            }
                            Err(e) => {
                                self.chat_log = vec![format!("connect failed: {}", e)];
                            }
                        }
                    }
                    ui.add_space(8.0);
                    if btn(ui, "Settings", 1.3, false) {
                        self.ui_open = UiOpen::Settings;
                        self.settings_from_title = true;
                        self.menu_reveal = 0.0;
                    }
                    ui.add_space(8.0);
                    if btn(ui, "Quit", 1.4, false) {
                        self.quit_requested = true;
                    }
                });
            });
        egui::TopBottomPanel::bottom("title_footer")
            .frame(egui::Frame::none())
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(env!("CARGO_PKG_VERSION")).small().color(Theme::TEXT_DIM));
                    ui.label(egui::RichText::new("LOREFORGE 0.1  ·  ").small().color(Theme::TEXT_DIM));
                });
            });
    }

    fn draw_pause(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(130)))
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.add_space(ui.available_height() * 0.18);
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
                    ui.add_space(30.0);
                    kit::slide_panel(ui, (t / 0.5).clamp(0.0, 1.0), |ui| {
                        ui.set_width(520.0);
                        ui.vertical(|ui| {
                            ui.add_space(10.0);
                            ui.heading(egui::RichText::new("Settings").size(26.0).color(Theme::TEXT));
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                for (i, label) in ["Video", "Interface", "Audio", "Controls", "Gameplay"].iter().enumerate() {
                                    let on = self.settings_tab == i;
                                    let btn = egui::Button::new(egui::RichText::new(*label)
                                        .color(if on { Theme::ACCENT } else { Theme::TEXT_DIM }))
                                        .min_size(egui::vec2(90.0, 28.0));
                                    if ui.add(btn).clicked() {
                                        self.settings_tab = i;
                                    }
                                }
                            });
                            ui.separator();
                            match self.settings_tab {
                                0 => self.settings_video(ui),
                                1 => self.settings_interface(ui),
                                2 => self.settings_audio(ui),
                                3 => self.settings_controls(ui),
                                _ => self.settings_gameplay(ui),
                            }
                            ui.add_space(10.0);
                            if kit::menu_button(ui, "Back", 1.0, true) {
                                self.close_settings();
                            }
                            ui.add_space(10.0);
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
        egui::Window::new(format!("Trading with {} the {:?}", villager.name, villager.job))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -30.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                for (give, give_n, get, get_n) in trade_offers(villager.job) {
                    let have = self.inventory.slots.iter()
                        .filter_map(|s| s.as_ref())
                        .filter(|s| s.item_id == *give)
                        .map(|s| s.count as u16)
                        .sum::<u16>();
                    let enough = have >= *give_n as u16;
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
                                ui.label(egui::RichText::new(format!("x{}", give_n))
                                    .color(if enough { Theme::OK } else { Theme::BAD }));
                                ui.label(egui::RichText::new("→").color(Theme::TEXT_DIM));
                                let (r2, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                                paint_item(ui, r2, &get_stack, &self.icons);
                                ui.label(egui::RichText::new(format!("x{}", get_n)).color(Theme::TEXT));
                                ui.label(egui::RichText::new(format!("(have {})", have)).small().color(Theme::TEXT_DIM));
                                if ui.add_enabled(enough, egui::Button::new("Trade")).clicked() {
                                    let mut left = *give_n as u16;
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
                                    let leftover = self.inventory.add_item(get, *get_n);
                                    if leftover > 0 {
                                        self.spawn_drop(get, leftover, self.player.eye_position() + self.player.look_dir());
                                    }
                                }
                            });
                        });
                }
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
                                ui.label(egui::RichText::new("→").color(Theme::TEXT_DIM));
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
                            ui.label(egui::RichText::new("→").color(Theme::TEXT_DIM));
                        }
                    }
                });
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

    /// Save-slot picker: list worlds (Load / Delete) + create a new one
    /// with a custom name and type.
    fn draw_slots(&mut self, ctx: &egui::Context) {
        let reveal = kit::ease_out_cubic((self.menu_reveal / 0.35).clamp(0.0, 1.0));
        let mut open_game = None;
        let mut delete_slot: Option<String> = None;
        let mut create: Option<(String, lf_worldgen::WorldType)> = None;
        let slots = crate::slots::list_slots();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(150)))
            .show(ctx, |ui| {
                kit::slide_panel(ui, reveal, |ui| {
                    ui.set_width(460.0);
                    ui.vertical(|ui| {
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("Save Slots").size(26.0).color(Theme::TEXT).strong());
                        ui.label(egui::RichText::new(format!("current: {}", self.slot_meta.name)).small().color(Theme::TEXT_DIM));
                        ui.add_space(10.0);
                        ui.separator();
                        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                            if slots.is_empty() {
                                ui.label(egui::RichText::new("no saved worlds yet").color(Theme::TEXT_DIM));
                            }
                            for meta in &slots {
                                let current = meta.name == self.slot_meta.name;
                                egui::Frame::new()
                                    .fill(if current { egui::Color32::from_rgba_premultiplied(34, 40, 32, 220) }
                                          else { egui::Color32::from_black_alpha(130) })
                                    .stroke(egui::Stroke::new(if current { 1.6 } else { 1.0 },
                                        if current { Theme::ACCENT_DIM } else { egui::Color32::from_gray(60) }))
                                    .corner_radius(7.0)
                                    .inner_margin(8.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            // Step 14: live thumbnail captured by the autosave
                                            let thumb_path = crate::slots::slot_dir(&meta.name).join("thumb.png");
                                            let key = meta.name.clone();
                                            if !self.slot_thumbs.contains_key(&key) {
                                                let decoded = image::ImageReader::open(&thumb_path)
                                                    .map_err(|e| e.to_string())
                                                    .and_then(|mut r| r.decode().map_err(|e| e.to_string()));
                                                if let Ok(img) = decoded.map(|d| d.to_rgba8()) {
                                                    let size = [img.width() as usize, img.height() as usize];
                                                    let color = egui::ColorImage::from_rgba_unmultiplied(size, &img);
                                                    let tex = ui.ctx().load_texture(format!("thumb_{key}"), color, egui::TextureOptions::LINEAR);
                                                    self.slot_thumbs.insert(key.clone(), tex);
                                                }
                                            }
                                            if let Some(tex) = self.slot_thumbs.get(&key) {
                                                ui.image((tex.id(), egui::vec2(96.0, 54.0)))
                                                    .on_hover_text("last autosave view");
                                            }
                                            ui.vertical(|ui| {
                                                ui.label(egui::RichText::new(&meta.name).size(16.0)
                                                    .color(if current { Theme::ACCENT } else { Theme::TEXT }));
                                                ui.label(egui::RichText::new(format!("{:?} · seed {} · saved {}",
                                                    meta.world_type, meta.seed, time_ago(meta.updated_secs)))
                                                    .small().color(Theme::TEXT_DIM));
                                            });
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let del_id = egui::Id::new(("delslot", &meta.name));
                                                let confirming = ui.ctx().data(|d| d.get_temp::<bool>(del_id).unwrap_or(false));
                                                if ui.button(if confirming { "really delete?" } else { "Delete" }).clicked() {
                                                    if confirming {
                                                        delete_slot = Some(meta.name.clone());
                                                        ui.ctx().data_mut(|d| d.insert_temp(del_id, false));
                                                    } else {
                                                        ui.ctx().data_mut(|d| d.insert_temp(del_id, true));
                                                    }
                                                }
                                                if confirming && ui.button("keep").clicked() {
                                                    ui.ctx().data_mut(|d| d.insert_temp(del_id, false));
                                                }
                                                if ui.button("Load").clicked() {
                                                    open_game = Some(meta.name.clone());
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(4.0);
                            }
                        });
                        ui.add_space(8.0);
                        ui.separator();
                        ui.label(egui::RichText::new("New World").size(17.0).color(Theme::ACCENT));
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("name").small().color(Theme::TEXT_DIM));
                            ui.add(egui::TextEdit::singleline(&mut self.slot_name_input).desired_width(160.0).hint_text("My World"));
                            for (label, wt) in [("Normal", lf_worldgen::WorldType::Normal),
                                                ("Superflat", lf_worldgen::WorldType::Superflat),
                                                ("Amplified", lf_worldgen::WorldType::Amplified)] {
                                let on = self.slot_new_type == wt;
                                if ui.add(egui::Button::new(egui::RichText::new(label)
                                    .color(if on { Theme::ACCENT } else { Theme::TEXT_DIM }))).clicked() {
                                    self.slot_new_type = wt;
                                }
                            }
                            if ui.button("Create").clicked() {
                                let name = if self.slot_name_input.trim().is_empty() {
                                    format!("World {}", crate::slots::list_slots().len() + 1)
                                } else {
                                    self.slot_name_input.clone()
                                };
                                create = Some((name, self.slot_new_type));
                            }
                        });
                        ui.add_space(10.0);
                        if kit::menu_button(ui, "Back", 1.0, true) {
                            self.ui_open = UiOpen::Title;
                            self.menu_reveal = 0.0;
                        }
                        ui.add_space(10.0);
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
                    None => self.new_world_named("World 1", lf_worldgen::WorldType::Normal),
                }
            }
        }
        if let Some((name, wt)) = create {
            self.new_world_named(&name, wt);
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
