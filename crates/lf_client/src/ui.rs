//! egui integration: platform plumbing plus HUD, inventory/crafting screens
//! and the death screen. All immediate-mode drawing against GameState.

use egui_wgpu::Renderer;
use egui_winit::State as EguiWinitState;

use crate::{BlockEntity, GameState, RtMode, UiOpen};
use crate::ui_kit::{self as kit, Theme};
use lf_npc::trade_offers;
use lf_game::crafting::{consume_ingredients, match_recipe};
use lf_game::items::{item_def, ItemKind};
use lf_game::survival::ItemStack;

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

fn item_color(stack: &ItemStack) -> egui::Color32 {
    match item_def(&stack.item_id).map(|d| d.kind) {
        Some(ItemKind::Block(b)) => block_color(b),
        Some(ItemKind::Tool(kind, tier)) => {
            let base = match kind {
                lf_game::items::ToolKind::Pickaxe => (150, 130, 90),
                lf_game::items::ToolKind::Axe => (140, 120, 80),
                lf_game::items::ToolKind::Shovel => (130, 140, 90),
                lf_game::items::ToolKind::Sword => (170, 120, 110),
                lf_game::items::ToolKind::Bow => (150, 110, 70),
            };
            let shade = 1.0 - tier as f32 * 0.12;
            egui::Color32::from_rgb(
                (base.0 as f32 * shade) as u8,
                (base.1 as f32 * shade) as u8,
                (base.2 as f32 * shade) as u8,
            )
        }
        Some(ItemKind::Food(_)) => egui::Color32::from_rgb(200, 60, 60),
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

const SLOT_SIZE: f32 = 44.0;

/// One inventory slot with full pick/place/merge/split semantics.
fn slot_button(
    ui: &mut egui::Ui,
    stack: &mut Option<ItemStack>,
    cursor: &mut Option<ItemStack>,
    selected: bool,
) -> (Option<String>, ()) {
    let mut _hover_info = None;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(SLOT_SIZE, SLOT_SIZE), egui::Sense::click());
    let frame = if selected {
        egui::Stroke::new(3.0, egui::Color32::WHITE)
    } else {
        egui::Stroke::new(1.0, egui::Color32::from_gray(90))
    };
    ui.painter().rect_filled(rect, 4.0, egui::Color32::from_black_alpha(160));
    ui.painter().rect_stroke(rect, 4.0, frame, egui::StrokeKind::Middle);
    if let Some(s) = stack {
        ui.painter().rect_filled(rect.shrink(7.0), 3.0, item_color(s));
        if s.count > 1 {
            ui.painter().text(
                rect.right_bottom() + egui::vec2(-6.0, -6.0),
                egui::Align2::RIGHT_BOTTOM,
                format!("{}", s.count),
                egui::FontId::proportional(13.0),
                egui::Color32::WHITE,
            );
        }
        if response.hovered() {
            _hover_info = Some(s.item_id.clone());
        }
    }
    if response.clicked_by(egui::PointerButton::Primary) {
        exchange(cursor, stack, false);
    } else if response.clicked_by(egui::PointerButton::Secondary) {
        exchange(cursor, stack, true);
    }
    (_hover_info, ())
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

impl GameState {
    /// Draw every UI surface for this frame.
    pub fn draw_ui(&mut self, ctx: &egui::Context) {
        self.draw_hud(ctx);
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
            UiOpen::Death => self.draw_death(ctx),
        }
        // Cursor stack follows the pointer.
        if let Some(cursor) = &self.cursor_stack {
            if let Some(pointer) = ctx.pointer_hover_pos() {
                let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("cursor-stack"));
                let rect = egui::Rect::from_center_size(pointer, egui::vec2(30.0, 30.0));
                ctx.layer_painter(layer).rect_filled(rect, 3.0, item_color(cursor));
                if cursor.count > 1 {
                    ctx.layer_painter(layer).text(
                        rect.right_bottom(),
                        egui::Align2::RIGHT_BOTTOM,
                        format!("{}", cursor.count),
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );
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
                .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(8.0, -130.0))
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
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -160.0))
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
        // HUD bar
        egui::TopBottomPanel::bottom("hud").frame(egui::Frame::none()).show_separator_line(false).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(4.0);
                // painted hearts + hunger
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() * 0.28);
                    crate::ui_kit::paint_hearts(ui, self.stats.health, self.stats.max_health);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        crate::ui_kit::paint_hunger(ui, self.stats.hunger, self.stats.max_hunger);
                    });
                });
                if self.air < 10 {
                    ui.label(egui::RichText::new(format!("air {}", "·".repeat(self.air as usize))).color(Theme::XP));
                }
                // XP bar with level chip
                let frac = (self.xp_progress as f32 / lf_game::combat::xp_for_level(self.xp_level).max(1) as f32).clamp(0.0, 1.0);
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(180.0, 8.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4.0, egui::Color32::from_black_alpha(180));
                    ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * frac, rect.height())), 4.0, Theme::XP);
                    ui.label(egui::RichText::new(format!("Lv {}", self.xp_level)).small().color(Theme::XP));
                });
                // hotbar with hover tooltips
                ui.horizontal(|ui| {
                    for i in 0..9 {
                        let mut stack = self.inventory.slots[i].clone();
                        let selected = i == self.hotbar_index;
                        let mut cursor = self.cursor_stack.take();
                        let (hover_name, _taken) = slot_button(ui, &mut stack, &mut cursor, selected);
                        self.cursor_stack = cursor;
                        self.inventory.slots[i] = stack;
                        if let Some(name) = hover_name {
                            self.hotbar_hover = Some(name);
                        }
                    }
                });
                if let Some(name) = &self.hotbar_hover.clone() {
                    ui.label(egui::RichText::new(name).small().color(Theme::ACCENT));
                } else {
                    ui.label(egui::RichText::new("").small());
                }
                // mining / bow charge
                if let Some(mining) = &self.mining {
                    let frac = (mining.progress / mining.total).min(1.0);
                    ui.add(egui::ProgressBar::new(frac).desired_width(220.0).fill(Theme::ACCENT).text(""));
                }
                if let Some(charge) = self.bow_charge {
                    let frac = (charge / 1.2).min(1.0);
                    ui.add(egui::ProgressBar::new(frac).desired_width(220.0).fill(Theme::OK).text("bow"));
                }
            });
        });
        // info line (top-left): clock, weather, transport, FPS
        let mut info = format!("{} · {}", self.time_label(), if self.weather_raining { "rain" } else { "clear" });
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
        // crosshair (only when playing)
        if self.ui_open == UiOpen::None {
            let pointer = ctx.screen_rect().center();
            let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, "crosshair".into()));
            let c = egui::Color32::from_white_alpha(220);
            p.line_segment([pointer - egui::vec2(8.0, 0.0), pointer + egui::vec2(8.0, 0.0)], egui::Stroke::new(2.0, c));
            p.line_segment([pointer - egui::vec2(0.0, 8.0), pointer + egui::vec2(0.0, 8.0)], egui::Stroke::new(2.0, c));
        }
    }

    fn draw_inventory(&mut self, ctx: &egui::Context, grid: usize) {
        let title = if grid == 3 { "Crafting Table" } else { "Inventory" };
        egui::Window::new(title)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -30.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                // crafting area
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(if grid == 3 { "Craft" } else { "2x2" });
                        let cells: usize = grid * grid;
                        for row in 0..grid {
                            ui.horizontal(|ui| {
                                for col in 0..grid {
                                    let idx = row * grid + col;
                                    let mut stack = self.craft_grid[idx].clone();
                                    let mut cursor = self.cursor_stack.take();
                                    let _ = slot_button(ui, &mut stack, &mut cursor, false);
                                    self.cursor_stack = cursor;
                                    self.craft_grid[idx] = stack;
                                }
                            });
                        }
                    });
                    ui.label("->");
                    // result slot
                    let grid_ref: Vec<Option<ItemStack>> = self.craft_grid.iter().take(grid * grid).cloned().collect();
                    let result = match_recipe(&grid_ref);
                    // era gating: locked recipes show but cannot be taken
                    let locked = match &result {
                        Some((out, _)) => {
                            lf_game::research::Era::required_for(out) > self.research.era
                        }
                        None => false,
                    };
                    let (disabled, color, count) = match &result {
                        Some((out, n)) => (locked, item_color(&ItemStack { item_id: out.clone(), count: *n }), *n),
                        None => (true, egui::Color32::from_gray(60), 0),
                    };
                    if locked {
                        ui.label(egui::RichText::new(format!("requires the {}",
                            lf_game::research::Era::required_for(&result.as_ref().unwrap().0).name())).small().color(egui::Color32::from_rgb(230, 130, 130)));
                    }
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(SLOT_SIZE, SLOT_SIZE), egui::Sense::click());
                    ui.painter().rect_filled(rect, 4.0, egui::Color32::from_black_alpha(160));
                    ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(2.0, egui::Color32::from_gray(120)), egui::StrokeKind::Middle);
                    if !disabled {
                        ui.painter().rect_filled(rect.shrink(7.0), 3.0, color);
                        if count > 1 {
                            ui.painter().text(
                                rect.right_bottom() + egui::vec2(-6.0, -6.0),
                                egui::Align2::RIGHT_BOTTOM,
                                format!("{}", count),
                                egui::FontId::proportional(13.0),
                                egui::Color32::WHITE,
                            );
                        }
                        if response.clicked() {
                            if let Some((out, n)) = result {
                                let crafted = ItemStack { item_id: out, count: n };
                                // take into cursor if possible
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
                ui.add_space(6.0);
                // storage 9x3
                for row in 0..3 {
                    ui.horizontal(|ui| {
                        for col in 0..9 {
                            let idx = 9 + row * 9 + col;
                            let mut stack = self.inventory.slots[idx].clone();
                            let mut cursor = self.cursor_stack.take();
                            let _ = slot_button(ui, &mut stack, &mut cursor, false);
                            self.cursor_stack = cursor;
                            self.inventory.slots[idx] = stack;
                        }
                    });
                }
                ui.add_space(4.0);
                // hotbar row
                ui.horizontal(|ui| {
                    for i in 0..9 {
                        let mut stack = self.inventory.slots[i].clone();
                        let mut cursor = self.cursor_stack.take();
                        let _ = slot_button(ui, &mut stack, &mut cursor, i == self.hotbar_index);
                        self.cursor_stack = cursor;
                        self.inventory.slots[i] = stack;
                    }
                });
            });
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
                        ui.label("Input");
                        let mut input = furnace.input.take();
                        let mut cursor = self.cursor_stack.take();
                        let _ = slot_button(ui, &mut input, &mut cursor, false);
                        furnace.input = input;
                        self.cursor_stack = cursor;
                        // flame indicator
                        let flame = if furnace.burn_total > 0.0 {
                            (furnace.burn_left / furnace.burn_total).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        ui.add(egui::ProgressBar::new(flame).desired_width(SLOT_SIZE).text(if flame > 0.0 { "fire" } else { "" }));
                        ui.label("Fuel");
                        let mut fuel = furnace.fuel.take();
                        let mut cursor = self.cursor_stack.take();
                        let _ = slot_button(ui, &mut fuel, &mut cursor, false);
                        furnace.fuel = fuel;
                        self.cursor_stack = cursor;
                    });
                    ui.vertical(|ui| {
                        let frac = (furnace.progress / lf_game::smelting::SMELT_TIME).clamp(0.0, 1.0);
                        ui.add(egui::ProgressBar::new(frac).desired_width(80.0).text("smelt"));
                        ui.label("->");
                    });
                    ui.vertical(|ui| {
                        ui.label("Output");
                        let mut output = furnace.output.take();
                        let mut cursor = self.cursor_stack.take();
                        let _ = slot_button(ui, &mut output, &mut cursor, false);
                        furnace.output = output;
                        self.cursor_stack = cursor;
                    });
                });
                ui.add_space(6.0);
                Self::draw_storage_rows(ui, self);
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
                            let _ = slot_button(ui, &mut stack, &mut cursor, false);
                            self.cursor_stack = cursor;
                            chest_slots[idx] = stack;
                        }
                    });
                }
                ui.add_space(6.0);
                Self::draw_storage_rows(ui, self);
            });
        self.block_entities.insert(pos, BlockEntity::Chest { slots: chest_slots });
    }

    fn draw_storage_rows(ui: &mut egui::Ui, game: &mut GameState) {
        // storage 9x3 + hotbar (shared by container screens)
        for row in 0..3 {
            ui.horizontal(|ui| {
                for col in 0..9 {
                    let idx = 9 + row * 9 + col;
                    let mut stack = game.inventory.slots[idx].clone();
                    let mut cursor = game.cursor_stack.take();
                    let _ = slot_button(ui, &mut stack, &mut cursor, false);
                    game.cursor_stack = cursor;
                    game.inventory.slots[idx] = stack;
                }
            });
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            for i in 0..9 {
                let mut stack = game.inventory.slots[i].clone();
                let mut cursor = game.cursor_stack.take();
                let _ = slot_button(ui, &mut stack, &mut cursor, i == game.hotbar_index);
                game.cursor_stack = cursor;
                game.inventory.slots[i] = stack;
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
                    if btn(ui, "Play", 0.7, true) {
                        self.close_ui();
                    }
                    ui.add_space(8.0);
                    let transport = if lf_steam::preferred_transport() == lf_steam::Transport::Udp {
                        "localhost"
                    } else {
                        "Steam P2P"
                    };
                    if btn(ui, &format!("Multiplayer ({})", transport), 0.85, false) {
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
                    ui.horizontal(|ui| {
                        if btn(ui, "New Normal", 1.0, false) {
                            self.new_world(lf_worldgen::WorldType::Normal);
                        }
                        ui.add_space(6.0);
                        if btn(ui, "New Superflat", 1.05, false) {
                            self.new_world(lf_worldgen::WorldType::Superflat);
                        }
                        ui.add_space(6.0);
                        if btn(ui, "New Amplified", 1.1, false) {
                            self.new_world(lf_worldgen::WorldType::Amplified);
                        }
                    });
                    ui.add_space(8.0);
                    if btn(ui, "Settings", 1.2, false) {
                        self.ui_open = UiOpen::Settings;
                        self.menu_reveal = 0.0;
                    }
                    ui.add_space(8.0);
                    if btn(ui, "Quit", 1.3, false) {
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
                            if kit::menu_button(ui, "Settings", ((t - 0.25) / 0.4).clamp(0.0, 1.0), false) {
                                self.ui_open = UiOpen::Settings;
                                self.menu_reveal = 0.0;
                            }
                            ui.add_space(6.0);
                            if kit::menu_button(ui, "Save & Quit", ((t - 0.35) / 0.4).clamp(0.0, 1.0), false) {
                                self.quit_requested = true;
                            }
                            ui.add_space(14.0);
                            ui.label(egui::RichText::new("E inventory · K tech tree · J quests · T chat · F2 shot · R RT capture")
                                .small().color(Theme::TEXT_DIM));
                            ui.add_space(12.0);
                        });
                    });
                });
            });
    }

    /// Settings screen (tabbed, kit-styled, drives the engine live).
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
                                for (i, label) in ["Video", "Audio", "Gameplay"].iter().enumerate() {
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
                                1 => self.settings_audio(ui),
                                _ => self.settings_gameplay(ui),
                            }
                            ui.add_space(10.0);
                            if kit::menu_button(ui, "Back", 1.0, true) {
                                self.ui_open = UiOpen::None;
                                if self.stats.health > 0.0 {
                                    self.lock_cursor();
                                }
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
            if ui.button("Low").clicked() { self.settings.apply_quality(crate::Quality::Low); }
            if ui.button("Medium").clicked() { self.settings.apply_quality(crate::Quality::Medium); }
            if ui.button("High").clicked() { self.settings.apply_quality(crate::Quality::High); }
        });
    }

    fn settings_audio(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.settings;
        kit::section_header(ui, "Audio", 1.0);
        ui.add_space(6.0);
        kit::setting_slider(ui, "Master volume", &mut s.volume_master, (0.0, 1.0), &|v| format!("{:.0}%", v * 100.0));
        kit::setting_slider(ui, "SFX volume", &mut s.volume_sfx, (0.0, 1.0), &|v| format!("{:.0}%", v * 100.0));
        kit::setting_slider(ui, "Music volume", &mut s.volume_music, (0.0, 1.0), &|v| format!("{:.0}%", v * 100.0));
        ui.add_space(8.0);
        ui.label(egui::RichText::new("volumes persist now; the audio engine consumes them when it lands (BACKLOG)")
            .small().color(Theme::TEXT_DIM));
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
                    ui.horizontal(|ui| {
                        let have = self.inventory.slots.iter()
                            .filter_map(|s| s.as_ref())
                            .filter(|s| s.item_id == *give)
                            .map(|s| s.count as u16)
                            .sum::<u16>();
                        let enough = have >= *give_n as u16;
                        let color = if enough { egui::Color32::from_rgb(140, 220, 140) } else { egui::Color32::from_rgb(230, 130, 130) };
                        ui.label(egui::RichText::new(format!("{} {} -> {} {}", give_n, give, get_n, get)).color(color));
                        ui.label(format!("(have {})", have));
                        if ui.add_enabled(enough, egui::Button::new("Trade")).clicked() {
                            // pay
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
                // the existing forge minigame from lf_game::smithing
                let temp = self.forge.temperature;
                let zone = (60.0..=80.0).contains(&temp);
                let color = if zone { egui::Color32::from_rgb(255, 160, 60) } else { egui::Color32::from_rgb(120, 120, 130) };
                ui.label(egui::RichText::new(format!("temperature: {:.0}", temp)).color(color));
                ui.add(egui::ProgressBar::new(temp / 100.0).desired_width(240.0).text(if zone { "orange zone" } else { "" }));
                ui.horizontal(|ui| {
                    if ui.button("Pump bellows (+15)").clicked() {
                        self.forge.bellows(15.0);
                    }
                    let done = self.forge.strike();
                    let status = if self.forge.strikes_completed >= self.forge.target_strikes {
                        "blade ready!".to_string()
                    } else {
                        format!("strikes {}/{}", self.forge.strikes_completed, self.forge.target_strikes)
                    };
                    ui.label(status);
                    if done {
                        // award a steel ingot (the smith's product for now)
                        let leftover = self.inventory.add_item("steel_ingot", 1);
                        if leftover > 0 {
                            self.spawn_drop("steel_ingot", leftover, self.player.eye_position() + self.player.look_dir());
                        }
                    }
                });
                ui.separator();
                ui.label(egui::RichText::new("Strike only in the orange zone (60-80). Esc to close.").small());
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
            .show(ctx, |ui| match entity {
                BlockEntity::Generator(mut g) => {
                    ui.add(egui::ProgressBar::new(g.buffer / lf_game::machines::GEN_CAPACITY)
                        .desired_width(200.0).text(format!("{:.0} EU", g.buffer)));
                    let mut fuel = g.fuel.take();
                    let mut cursor = self.cursor_stack.take();
                    let _ = slot_button(ui, &mut fuel, &mut cursor, false);
                    g.fuel = fuel;
                    self.cursor_stack = cursor;
                    ui.label("fuel (coal/log/planks)");
                    self.block_entities.insert(pos, BlockEntity::Generator(g));
                }
                BlockEntity::ElectricFurnace(mut f) => {
                    let frac = (f.progress / (lf_game::smelting::SMELT_TIME / 2.0)).clamp(0.0, 1.0);
                    ui.add(egui::ProgressBar::new(frac).desired_width(200.0).text("smelt (2x)"));
                    ui.horizontal(|ui| {
                        let mut input = f.input.take();
                        let mut cursor = self.cursor_stack.take();
                        let _ = slot_button(ui, &mut input, &mut cursor, false);
                        f.input = input;
                        self.cursor_stack = cursor;
                        let mut output = f.output.take();
                        let mut cursor = self.cursor_stack.take();
                        let _ = slot_button(ui, &mut output, &mut cursor, false);
                        f.output = output;
                        self.cursor_stack = cursor;
                    });
                    self.block_entities.insert(pos, BlockEntity::ElectricFurnace(f));
                }
                BlockEntity::Crusher(mut c) => {
                    let frac = (c.progress / lf_game::machines::PROCESS_TIME).clamp(0.0, 1.0);
                    ui.add(egui::ProgressBar::new(frac).desired_width(200.0).text("crush"));
                    ui.horizontal(|ui| {
                        let mut input = c.input.take();
                        let mut cursor = self.cursor_stack.take();
                        let _ = slot_button(ui, &mut input, &mut cursor, false);
                        c.input = input;
                        self.cursor_stack = cursor;
                        let mut output = c.output.take();
                        let mut cursor = self.cursor_stack.take();
                        let _ = slot_button(ui, &mut output, &mut cursor, false);
                        c.output = output;
                        self.cursor_stack = cursor;
                    });
                    self.block_entities.insert(pos, BlockEntity::Crusher(c));
                }
                BlockEntity::Assembler(mut a) => {
                    let frac = (a.progress / lf_game::machines::PROCESS_TIME).clamp(0.0, 1.0);
                    ui.add(egui::ProgressBar::new(frac).desired_width(200.0).text("assemble"));
                    ui.horizontal(|ui| {
                        let mut ia = a.input_a.take();
                        let mut cursor = self.cursor_stack.take();
                        let _ = slot_button(ui, &mut ia, &mut cursor, false);
                        a.input_a = ia;
                        self.cursor_stack = cursor;
                        let mut ib = a.input_b.take();
                        let mut cursor = self.cursor_stack.take();
                        let _ = slot_button(ui, &mut ib, &mut cursor, false);
                        a.input_b = ib;
                        self.cursor_stack = cursor;
                        let mut output = a.output.take();
                        let mut cursor = self.cursor_stack.take();
                        let _ = slot_button(ui, &mut output, &mut cursor, false);
                        a.output = output;
                        self.cursor_stack = cursor;
                    });
                    if let Some((an, an_n, bn, bn_n, out, out_n)) = a.current_recipe() {
                        ui.label(egui::RichText::new(format!("{}x{} + {}x{} -> {}x{}", an, an_n, bn, bn_n, out, out_n)).small());
                    } else {
                        ui.label(egui::RichText::new("no recipe (try Cu+Sn, Fe+C, wire+Sn...)").small().color(egui::Color32::from_gray(150)));
                    }
                    self.block_entities.insert(pos, BlockEntity::Assembler(a));
                }
                _ => {}
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
                ui.heading(egui::RichText::new("Research Progression").size(22.0));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let eras = [lf_game::research::Era::Primitive, lf_game::research::Era::Bronze,
                                lf_game::research::Era::Industrial, lf_game::research::Era::Electrical];
                    for e in eras {
                        let state = if e < era { "done" } else if e == era { "CURRENT" } else { "locked" };
                        let color = if e < era { egui::Color32::from_rgb(120, 200, 120) }
                            else if e == era { egui::Color32::from_rgb(240, 210, 140) }
                            else { egui::Color32::from_gray(110) };
                        egui::Frame::new()
                            .stroke(egui::Stroke::new(if e == era { 3.0 } else { 1.0 }, color))
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
                                        let c = if ok { egui::Color32::from_rgb(140, 220, 140) } else { egui::Color32::from_rgb(230, 130, 130) };
                                        ui.label(egui::RichText::new(format!("{} {}/{}", item, got.min(*n as u16), n)).small().color(c));
                                    }
                                }
                            });
                        if e != lf_game::research::Era::Electrical {
                            ui.label("->");
                        }
                    }
                });
                ui.add_space(8.0);
                ui.separator();
                // what to do next
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
                                lf_game::research::Era::Primitive => "basic tools, furnace, chest",
                                lf_game::research::Era::Bronze => "armor, smithing, +everything before",
                                lf_game::research::Era::Industrial => "generators, crushers, assemblers",
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

    fn draw_death(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(200)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.3);
                    ui.heading(egui::RichText::new("You died!").size(42.0).color(egui::Color32::RED));
                    ui.add_space(20.0);
                    if ui.button(egui::RichText::new("Respawn").size(24.0)).clicked() {
                        self.respawn();
                    }
                    ui.add_space(8.0);
                    ui.label("or press Escape to quit");
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

fn hearts(ui: &mut egui::Ui, health: f32) {
    let full = (health / 2.0).floor();
    let half = (health / 2.0 - full) >= 0.5;
    let mut s = String::new();
    for i in 0..10 {
        if (i as f32) < full {
            s.push('\u{2665}');
        } else if i as f32 == full && half {
            s.push('\u{2661}');
        } else {
            s.push('\u{2661}');
        }
    }
    ui.label(egui::RichText::new(s).color(egui::Color32::from_rgb(220, 40, 40)).size(16.0));
}

fn hunger(ui: &mut egui::Ui, hunger: f32) {
    let full = (hunger / 2.0).floor() as usize;
    let mut s = String::new();
    for i in 0..10 {
        s.push(if i < full { '\u{25CF}' } else { '\u{25CB}' });
    }
    ui.label(egui::RichText::new(s).color(egui::Color32::from_rgb(200, 150, 40)).size(14.0));
}
