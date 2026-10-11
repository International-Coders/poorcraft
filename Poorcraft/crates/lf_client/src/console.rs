//! Developer console (backquote or `/`): command parsing + execution + the
//! egui overlay with TAB autocomplete and input history.

use crate::ui_kit::Theme;
use crate::{GameState, UiOpen};
use glam::Vec3;

/// Every command name, for autocomplete and `help`.
pub const COMMANDS: &[&str] = &[
    "help", "time", "give", "tp", "seed", "weather", "fly", "heal", "feed", "kill",
    "spawn", "clear", "waypoint", "say", "fps", "rt", "save", "slots", "load", "new",
];

/// One parsed console command (pure data — unit-testable without a GPU).
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Help,
    TimeSet(TimeSpec),
    Give { item: String, count: u8 },
    Teleport { x: f32, y: f32, z: f32 },
    Seed,
    Weather(bool),
    Fly,
    Heal,
    Feed,
    Kill,
    Spawn { mob: String, count: u8 },
    ClearInventory,
    WaypointAdd(Option<String>),
    WaypointList,
    WaypointRemove(String),
    Say(String),
    Fps,
    Rt(crate::RtMode),
    Save,
    Slots,
    Load(String),
    NewWorld { world_type: lf_worldgen::WorldType, name: Option<String> },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeSpec {
    Sunrise,
    Day,
    Noon,
    Sunset,
    Night,
    Ticks(u64),
}

impl TimeSpec {
    fn fraction(self) -> f32 {
        match self {
            TimeSpec::Sunrise => 0.25,
            TimeSpec::Day | TimeSpec::Noon => 0.5,
            TimeSpec::Sunset => 0.75,
            TimeSpec::Night => 0.0,
            TimeSpec::Ticks(t) => (t % 24_000) as f32 / 24_000.0,
        }
    }
}

/// Parse a console line (with or without the leading `/`).
pub fn parse(line: &str) -> Result<Command, String> {
    let line = line.trim().trim_start_matches('/').trim();
    if line.is_empty() {
        return Err("empty command".into());
    }
    let mut words = line.split_whitespace();
    let cmd = words.next().unwrap().to_lowercase();
    let rest: Vec<&str> = words.collect();
    match cmd.as_str() {
        "help" | "?" => Ok(Command::Help),
        "time" => {
            match rest.first().copied() {
                Some("set") => {}
                _ => return Err("usage: time set <sunrise|day|noon|sunset|night|0-24000>".into()),
            }
            let arg = rest.get(1).copied().ok_or("usage: time set <sunrise|day|noon|sunset|night|0-24000>")?;
            let spec = match arg.to_lowercase().as_str() {
                "sunrise" | "dawn" => TimeSpec::Sunrise,
                "day" => TimeSpec::Day,
                "noon" => TimeSpec::Noon,
                "sunset" | "dusk" => TimeSpec::Sunset,
                "night" | "midnight" => TimeSpec::Night,
                ticks => TimeSpec::Ticks(ticks.parse::<u64>().map_err(|_| format!("bad time '{}'", ticks))?),
            };
            Ok(Command::TimeSet(spec))
        }
        "give" => {
            let item = rest.first().copied().ok_or("usage: give <item> [count]")?.to_string();
            let count: u8 = rest.get(1).copied()
                .map(|s| s.parse().map_err(|_| format!("bad count '{}'", s)))
                .transpose()?.unwrap_or(1);
            Ok(Command::Give { item, count })
        }
        "tp" => {
            if rest.len() < 3 {
                return Err("usage: tp <x> <y> <z>".into());
            }
            let mut n = rest.iter().map(|s| s.parse::<f32>().map_err(|_| format!("bad coord '{}'", s)));
            let (x, y, z) = (n.next().unwrap()?, n.next().unwrap()?, n.next().unwrap()?);
            Ok(Command::Teleport { x, y, z })
        }
        "seed" => Ok(Command::Seed),
        "weather" => match rest.first().copied() {
            Some("clear") | Some("sun") => Ok(Command::Weather(false)),
            Some("rain") | Some("snow") => Ok(Command::Weather(true)),
            _ => Err("usage: weather <clear|rain>".into()),
        },
        "fly" => Ok(Command::Fly),
        "heal" => Ok(Command::Heal),
        "feed" => Ok(Command::Feed),
        "kill" => Ok(Command::Kill),
        "spawn" => {
            let mob = rest.first().copied().ok_or("usage: spawn <boar|woolbeast|glitchling|stalker|crawler|null_knight> [count]")?.to_string();
            let count: u8 = rest.get(1).copied()
                .map(|s| s.parse().map_err(|_| format!("bad count '{}'", s)))
                .transpose()?.unwrap_or(1);
            Ok(Command::Spawn { mob, count })
        }
        "clear" => Ok(Command::ClearInventory),
        "waypoint" | "wp" => match rest.first().copied() {
            Some("add") => Ok(Command::WaypointAdd(rest.get(1).map(|s| s.to_string()))),
            Some("list") => Ok(Command::WaypointList),
            Some("remove") | Some("rm") => Ok(Command::WaypointRemove(
                rest.get(1).copied().ok_or("usage: waypoint remove <name>")?.to_string())),
            _ => Err("usage: waypoint <add [name]|list|remove <name>>".into()),
        },
        "say" => Ok(Command::Say(rest.join(" "))),
        "fps" => Ok(Command::Fps),
        "rt" => match rest.first().copied() {
            Some("off") => Ok(Command::Rt(crate::RtMode::Off)),
            Some("captures") | Some("capture") => Ok(Command::Rt(crate::RtMode::Captures)),
            Some("live") => Ok(Command::Rt(crate::RtMode::Live)),
            _ => Err("usage: rt <off|captures|live>".into()),
        },
        "save" => Ok(Command::Save),
        "slots" => Ok(Command::Slots),
        "load" => Ok(Command::Load(
            rest.first().copied().ok_or("usage: load <slot>")?.to_string())),
        "new" => {
            let wt = match rest.first().copied() {
                Some("normal") => lf_worldgen::WorldType::Normal,
                Some("superflat") => lf_worldgen::WorldType::Superflat,
                Some("amplified") => lf_worldgen::WorldType::Amplified,
                _ => return Err("usage: new <normal|superflat|amplified> [name]".into()),
            };
            Ok(Command::NewWorld { world_type: wt, name: rest.get(1).map(|s| s.to_string()) })
        }
        other => Err(format!("unknown command '{}' — try help", other)),
    }
}

/// Autocomplete suggestions for a partial line (first token only).
pub fn complete(line: &str) -> Vec<String> {
    let trimmed = line.trim_start_matches('/');
    let prefix = trimmed.split_whitespace().next().unwrap_or("").to_lowercase();
    if trimmed.contains(' ') {
        return Vec::new(); // mid-argument: no suggestions (yet)
    }
    COMMANDS.iter()
        .filter(|c| c.starts_with(&prefix))
        .map(|c| format!("{} ", c))
        .collect()
}

/// Console session state.
#[derive(Default)]
pub struct ConsoleState {
    pub open: bool,
    pub input: String,
    pub lines: Vec<String>,
    pub history: Vec<String>,
    history_idx: Option<usize>,
    tab_idx: usize,
}

impl ConsoleState {
    pub fn log(&mut self, msg: impl Into<String>) {
        let line = msg.into();
        tracing::info!("[console] {}", line);
        self.lines.push(format!("> {}", line));
        if self.lines.len() > 200 {
            self.lines.drain(..100);
        }
    }

    pub fn output(&mut self, msg: impl Into<String>) {
        self.lines.push(msg.into());
        if self.lines.len() > 200 {
            self.lines.drain(..100);
        }
    }
}

impl GameState {
    pub fn console_open(&mut self) {
        self.console.open = true;
        self.console.tab_idx = 0;
        self.ui_open = UiOpen::Console;
        self.unlock_cursor();
    }

    fn run_command(&mut self, cmd: Command) {
        match cmd {
            Command::Help => {
                self.console.output("commands: help · time set <sunrise|day|noon|sunset|night|ticks> · \
                    give <item> [n] · tp <x y z> · seed · weather <clear|rain> · fly · heal · feed · \
                    kill · spawn <mob> [n] · clear · waypoint <add [name]|list|remove <name>> · say <text> · \
                    fps · rt <off|captures|live> · save · slots · load <slot> · new <type> [name]");
            }
            Command::TimeSet(spec) => {
                self.time = lf_game::TimeOfDay::from_fraction(spec.fraction());
                self.console.output(format!("time set (fraction {:.2})", spec.fraction()));
            }
            Command::Give { item, count } => {
                if lf_game::items::item_def(&item).is_none() {
                    self.console.output(format!("unknown item '{}'", item));
                    return;
                }
                let leftover = self.inventory.add_item(&item, count);
                if leftover > 0 {
                    self.spawn_drop(&item, leftover, self.player.eye_position() + self.player.look_dir());
                }
                self.console.output(format!("gave {} x{}", item, count));
            }
            Command::Teleport { x, y, z } => {
                self.player.position = Vec3::new(x, y, z);
                self.player.velocity = Vec3::ZERO;
                self.console.output(format!("teleported to ({:.1}, {:.1}, {:.1})", x, y, z));
            }
            Command::Seed => {
                self.console.output(format!("world seed: {}", self.world_seed));
            }
            Command::Weather(rain) => {
                self.weather_raining = rain;
                self.console.output(if rain { "weather: rain" } else { "weather: clear" });
            }
            Command::Fly => {
                self.player.flying = !self.player.flying;
                self.console.output(if self.player.flying { "flying on" } else { "flying off" });
            }
            Command::Heal => {
                self.stats.health = self.stats.max_health;
                self.console.output("health restored");
            }
            Command::Feed => {
                self.stats.hunger = self.stats.max_hunger;
                self.stats.saturation = 5.0;
                self.console.output("hunger restored");
            }
            Command::Kill => {
                self.console.output("ouch.");
                self.damage(10_000.0);
            }
            Command::Spawn { mob, count } => {
                let kind = match mob.to_lowercase().as_str() {
                    "boar" => Some(lf_game::mobs::MobType::Boar),
                    "woolbeast" => Some(lf_game::mobs::MobType::Woolbeast),
                    "glitchling" => Some(lf_game::mobs::MobType::Glitchling),
                    "stalker" => Some(lf_game::mobs::MobType::Stalker),
                    "crawler" => Some(lf_game::mobs::MobType::Crawler),
                    "null_knight" | "nullknight" | "boss" => Some(lf_game::mobs::MobType::NullKnight),
                    _ => None,
                };
                let Some(kind) = kind else {
                    self.console.output(format!("unknown mob '{}' (boar/woolbeast/glitchling/stalker/crawler/null_knight)", mob));
                    return;
                };
                let pos = self.player.position + self.player.look_dir() * 3.0 + Vec3::new(0.0, 1.0, 0.0);
                for _ in 0..count.max(1) {
                    let id = self.next_mob_id;
                    self.next_mob_id += 1;
                    self.mobs.push(lf_game::mobs::MobEntity::spawn(id, kind, pos));
                }
                self.console.output(format!("spawned {} x{}", mob, count.max(1)));
            }
            Command::ClearInventory => {
                for slot in self.inventory.slots.iter_mut() {
                    *slot = None;
                }
                self.console.output("inventory cleared");
            }
            Command::WaypointAdd(name) => {
                let p = self.player.position;
                let name = name.unwrap_or_else(|| format!("Marker {}", self.waypoints.len() + 1));
                let color = self.waypoints.len() % crate::map::WAYPOINT_COLORS.len();
                self.waypoints.push(crate::map::Waypoint {
                    x: p.x, y: p.y, z: p.z, name: name.clone(), color_idx: color,
                });
                self.console.output(format!("waypoint '{}' added at ({:.0},{:.0})", name, p.x, p.z));
            }
            Command::WaypointList => {
                if self.waypoints.is_empty() {
                    self.console.output("no waypoints");
                }
                let p = self.player.position;
                for wp in &self.waypoints {
                    let d = ((wp.x - p.x).powi(2) + (wp.z - p.z).powi(2)).sqrt();
                    self.console.output(format!("  {} ({:.0},{:.0},{:.0}) · {:.0}m", wp.name, wp.x, wp.y, wp.z, d));
                }
            }
            Command::WaypointRemove(name) => {
                let before = self.waypoints.len();
                self.waypoints.retain(|w| !w.name.eq_ignore_ascii_case(&name));
                if self.waypoints.len() < before {
                    self.console.output(format!("removed waypoint '{}'", name));
                } else {
                    self.console.output(format!("no waypoint named '{}'", name));
                }
            }
            Command::Say(text) => {
                let line = format!("[you] {}", text);
                self.chat_log.push(line.clone());
                self.console.output(line);
            }
            Command::Fps => {
                self.settings.show_fps = !self.settings.show_fps;
                self.console.output(if self.settings.show_fps { "fps display on" } else { "fps display off" });
            }
            Command::Rt(mode) => {
                self.settings.rt_mode = mode;
                self.console.output(format!("rt mode: {}", mode.label()));
            }
            Command::Save => {
                self.save_world();
                self.console.output(format!("world '{}' saved", self.slot_meta.name));
            }
            Command::Slots => {
                let slots = crate::slots::list_slots();
                if slots.is_empty() {
                    self.console.output("no save slots");
                }
                for m in slots {
                    let cur = if m.name == self.slot_meta.name { " (current)" } else { "" };
                    self.console.output(format!("  {} · {:?} · seed {}{}", m.name, m.world_type, m.seed, cur));
                }
            }
            Command::Load(name) => {
                self.console.output(format!("loading slot '{}'…", name));
                match self.load_world(&name) {
                    Ok(()) => self.console.output(format!("loaded '{}'", name)),
                    Err(e) => self.console.output(e),
                }
            }
            Command::NewWorld { world_type, name } => {
                let name = name.unwrap_or_else(|| format!("World {}", crate::slots::list_slots().len() + 1));
                self.console.output(format!("creating new world '{}' ({:?})…", name, world_type));
                self.new_world_named(&name, world_type);
                self.console.output(format!("created '{}' (seed {})", name, self.world_seed));
            }
        }
    }

    /// Execute one console line (echoes it into the history first).
    pub fn run_console_line(&mut self, line: &str) {
        let line = line.trim().to_string();
        if line.is_empty() {
            return;
        }
        self.console.log(line.clone());
        self.console.history.push(line.clone());
        self.console.history_idx = None;
        match parse(&line) {
            Ok(cmd) => self.run_command(cmd),
            Err(e) => self.console.output(e),
        }
    }

    /// The console overlay: history, suggestions, input. TAB autocompletes,
    /// arrows walk history, Esc/` closes, Enter runs.
    pub fn draw_console(&mut self, ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        let mut close = false;
        let mut run = None;
        let width = (screen.width() * 0.62).min(760.0);
        let height = (screen.height() * 0.44).min(340.0);
        egui::Area::new(egui::Id::new("console"))
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(8.0, 8.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(12, 14, 20, 242))
                    .stroke(egui::Stroke::new(1.0, Theme::ACCENT_DIM))
                    .corner_radius(8.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_width(width);
                        ui.set_min_height(height);
                        ui.label(egui::RichText::new("console — Tab complete · ↑↓ history · Esc close")
                            .small().color(Theme::TEXT_DIM));
                        ui.add_space(4.0);
                        // history (sticks to the bottom)
                        egui::ScrollArea::vertical()
                            .id_source("console_history")
                            .stick_to_bottom(true)
                            .max_height(height - 64.0)
                            .show(ui, |ui| {
                                for line in &self.console.lines {
                                    ui.label(egui::RichText::new(line).small().monospace().color(Theme::TEXT));
                                }
                            });
                        ui.add_space(4.0);
                        // suggestions for the current prefix
                        let suggestions = complete(&self.console.input);
                        if !suggestions.is_empty() {
                            ui.label(egui::RichText::new(suggestions.join("  "))
                                .small().monospace().color(Theme::ACCENT));
                        }
                        // input line
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.console.input)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(width - 8.0)
                                .hint_text("type a command… (help)"),
                        );
                        response.request_focus();
                        ui.input(|i| {
                            if i.key_pressed(egui::Key::Enter) {
                                run = Some(self.console.input.clone());
                            }
                            if i.key_pressed(egui::Key::Escape) {
                                close = true;
                            }
                            // the '`' character closes too (egui eats the key event)
                            if i.events.iter().any(|e| matches!(e, egui::Event::Text(t) if t.contains('`'))) {
                                close = true;
                            }
                        });
                        // TAB cycles through the suggestions
                        if ui.input(|i| i.key_pressed(egui::Key::Tab)) && !suggestions.is_empty() {
                            self.console.tab_idx = (self.console.tab_idx + 1) % suggestions.len().max(1);
                            if let Some(s) = suggestions.get(self.console.tab_idx) {
                                self.console.input = s.clone();
                            }
                        }
                        // arrows walk the input history
                        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) && !self.console.history.is_empty() {
                            let idx = match self.console.history_idx {
                                None => self.console.history.len() - 1,
                                Some(0) => 0,
                                Some(i) => i - 1,
                            };
                            self.console.history_idx = Some(idx);
                            self.console.input = self.console.history[idx].clone();
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                            if let Some(idx) = self.console.history_idx {
                                if idx + 1 < self.console.history.len() {
                                    self.console.history_idx = Some(idx + 1);
                                    self.console.input = self.console.history[idx + 1].clone();
                                } else {
                                    self.console.history_idx = None;
                                    self.console.input.clear();
                                }
                            }
                        }
                    });
            });
        if let Some(line) = run {
            self.console.input.clear();
            self.console.tab_idx = 0;
            self.run_console_line(&line);
        }
        if close {
            self.console.input.pop(); // drop a trailing '`' if any
            self.console.open = false;
            self.close_ui();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_core_commands() {
        assert_eq!(parse("help").unwrap(), Command::Help);
        assert_eq!(parse("/time set night").unwrap(), Command::TimeSet(TimeSpec::Night));
        assert_eq!(parse("time set 6000").unwrap(), Command::TimeSet(TimeSpec::Ticks(6000)));
        assert_eq!(parse("give iron_ingot 12").unwrap(), Command::Give { item: "iron_ingot".into(), count: 12 });
        assert_eq!(parse("give apple").unwrap(), Command::Give { item: "apple".into(), count: 1 });
        assert_eq!(parse("tp 10 -5 3.5").unwrap(), Command::Teleport { x: 10.0, y: -5.0, z: 3.5 });
        assert_eq!(parse("weather rain").unwrap(), Command::Weather(true));
        assert_eq!(parse("rt live").unwrap(), Command::Rt(crate::RtMode::Live));
        assert_eq!(parse("spawn stalker 3").unwrap(), Command::Spawn { mob: "stalker".into(), count: 3 });
        assert_eq!(parse("wp add home").unwrap(), Command::WaypointAdd(Some("home".into())));
        assert!(parse("tp 1 2").is_err());
        assert!(parse("give").is_err());
        assert!(parse("frobnicate").is_err());
    }

    #[test]
    fn completes_prefixes() {
        assert!(complete("ti").contains(&"time ".to_string()));
        assert!(complete("w").contains(&"weather ".to_string()));
        assert!(complete("w").contains(&"waypoint ".to_string()));
        assert!(complete("").len() == COMMANDS.len());
        assert!(complete("time set ").is_empty(), "no mid-arg suggestions");
        assert!(complete("zzz").is_empty());
    }

    #[test]
    fn time_specs_map_to_fractions() {
        assert_eq!(TimeSpec::Noon.fraction(), 0.5);
        assert_eq!(TimeSpec::Night.fraction(), 0.0);
        assert_eq!(TimeSpec::Ticks(12_000).fraction(), 0.5);
    }
}
