//! Faction + companion gameplay (lore-and-visuals Sections A, B, D).
//! All logic here works over GameState; lib.rs calls the hooks from the
//! update loop, interactions, rendering, and save/load.

use crate::GameState;
use lf_chronicle::EventType;
use lf_game::companions::{
    Companion, CompanionAction, CompanionCommand, CompanionStats, MAX_ACTIVE_COMPANIONS,
};
use lf_lore::{ConditionCtx, StandingBand, WorldEventTrigger};
use lf_voxel::registry::block;

/// Faction structure marker block -> structure key (C3 banner markers).
pub fn structure_of_marker(id: u32) -> Option<&'static str> {
    Some(match id {
        block::BANNER_ACCORD => "accord_embassy",
        block::BANNER_IRONBORN => "ironborn_forge_camp",
        block::BANNER_COVENANT => "covenant_grove_shrine",
        block::BANNER_FREEHOLDS => "freeholds_longhouse",
        block::BANNER_ASHEN => "ashen_library",
        block::BANNER_NAMELESS => "nameless_camp",
        _ => return None,
    })
}

/// The faction a themed block belongs to (standing penalty when the
/// player destroys their structures).
pub fn faction_of_block(id: u32) -> Option<&'static str> {
    Some(match id {
        block::ACCORD_STONE | block::ACCORD_PILLAR | block::BANNER_ACCORD => "accord",
        block::IRONBORN_BRICK | block::IRONBORN_GRATE | block::BANNER_IRONBORN => "ironborn",
        block::EMBER_COVENANTWOOD | block::EMBER_GLOWSTONE | block::BANNER_COVENANT => "ember_covenant",
        block::FREEHOLDS_THATCH | block::FREEHOLDS_DAUB | block::BANNER_FREEHOLDS => "free_holds",
        block::ASHEN_MARBLE | block::ASHEN_BOOKSHELF | block::BANNER_ASHEN => "ashen_order",
        block::NAMELESS_ROTWOOD | block::NAMELESS_SCORCHED | block::BANNER_NAMELESS => "nameless",
        _ => return None,
    })
}

/// Stable lowercase mob kind id for quest Kill targets.
pub fn mob_kind_id(kind: lf_game::mobs::MobType) -> &'static str {
    use lf_game::mobs::MobType;
    match kind {
        MobType::Boar => "boar",
        MobType::Woolbeast => "woolbeast",
        MobType::Glitchling => "glitchling",
        MobType::Stalker => "stalker",
        MobType::Crawler => "crawler",
        MobType::NullKnight => "null_knight",
        MobType::Dragon => "dragon",
        MobType::NamelessRaider => "nameless_raider",
    }
}

/// Display label for a villager job.
pub fn job_label(job: lf_npc::VillagerJob) -> &'static str {
    use lf_npc::VillagerJob;
    match job {
        VillagerJob::Farmer => "Farmer",
        VillagerJob::Smith => "Smith",
        VillagerJob::Guard => "Guard",
        VillagerJob::Trader => "Trader",
        VillagerJob::Bard => "Herbalist",
        VillagerJob::Lorekeeper => "Lorekeeper",
        VillagerJob::Wizard => "Wizard",
    }
}

/// Roster archetype -> the closest existing villager job (schedules/trades).
pub fn roster_job(archetype: &str) -> lf_npc::VillagerJob {
    use lf_npc::VillagerJob;
    match archetype {
        "accord_herald" => VillagerJob::Guard,
        "ironborn_artisan" | "dag_holtz" => VillagerJob::Smith,
        "covenant_herbalist" => VillagerJob::Bard,
        "freeholds_elder" => VillagerJob::Farmer,
        "freeholds_scout" => VillagerJob::Trader,
        "ashen_archivist" | "maren_voss" => VillagerJob::Lorekeeper,
        _ => VillagerJob::Trader,
    }
}


// ----------------------------------------------------------------------
// Pure cores (testable without a GPU-backed GameState; the GameState
// methods below delegate to them so there is one code path).

/// Outcome of a hire attempt.
pub struct HireOutcome {
    pub ok: bool,
    pub message: String,
    pub chronicle: Option<String>,
}

/// The B2 hire flow: standing gate, fee deduction, companion state
/// transition, and the confirmation chronicle line. Mutates the
/// inventory (fee) and companions (push) on success.
pub fn hire_villager(
    lore: &lf_lore::LoreRegistry,
    standings: &lf_lore::StandingState,
    inventory: &mut lf_game::survival::Inventory,
    companions: &mut Vec<Companion>,
    memory: &std::collections::HashMap<String, i32>,
    next_id: u64,
    archetype_id: &str,
    display_name: &str,
    position: [f32; 3],
) -> HireOutcome {
    use lf_game::companions::MAX_ACTIVE_COMPANIONS;
    let Some(arch) = lore.villager_archetype(archetype_id).cloned() else {
        return HireOutcome { ok: false, message: format!("{} is not for hire.", display_name), chronicle: None };
    };
    if !arch.hireable {
        return HireOutcome { ok: false, message: format!("{} is not for hire.", display_name), chronicle: None };
    }
    if companions.len() >= MAX_ACTIVE_COMPANIONS {
        return HireOutcome { ok: false, message: "You already have three companions — they need your attention as much as your coin.".into(), chronicle: None };
    }
    let faction = arch.faction.clone().unwrap_or_default();
    let faction_name = lore.faction(&faction).map(|f| f.short_name.clone()).unwrap_or_default();
    if standings.get(&faction) < arch.hire_standing {
        return HireOutcome { ok: false, message: format!("{} will not serve someone the {} distrusts.", display_name, faction_name), chronicle: None };
    }
    let count = |inv: &lf_game::survival::Inventory, item: &str| {
        inv.slots.iter().filter_map(|s| s.as_ref())
            .filter(|s| s.item_id == item).map(|s| s.count as u32).sum::<u32>()
    };
    for (item, n) in &arch.hire_fee {
        if count(inventory, item) < *n as u32 {
            let item_name = lf_game::items::item_def(item)
                .map(|d| d.name.to_string())
                .unwrap_or_else(|| item.clone());
            return HireOutcome { ok: false, message: format!("You can't afford the hire fee ({} x{}).", item_name, n), chronicle: None };
        }
    }
    for (item, n) in &arch.hire_fee {
        let mut left = *n as u32;
        for slot in inventory.slots.iter_mut() {
            if left == 0 { break; }
            if let Some(stack) = slot {
                if stack.item_id == *item {
                    let take = left.min(stack.count as u32);
                    stack.count -= take as u8;
                    left -= take;
                    if stack.count == 0 { *slot = None; }
                }
            }
        }
    }
    let form = arch.companion_form.clone().unwrap_or_else(|| archetype_id.to_string());
    let comp_arch = lore.companion_archetype(&form).cloned();
    let mut c = Companion::new(
        next_id,
        &form,
        display_name.to_string(),
        arch.faction.clone(),
        comp_arch.as_ref().map(|a| a.daily_wage.clone()).unwrap_or_default(),
        glam::Vec3::new(position[0], position[1], position[2]),
    );
    if let Some(remembered) = memory.get(&form).copied() {
        c.trust = remembered.clamp(0, 100);
    }
    c.health = comp_arch.as_ref().map(|a| a.health).unwrap_or(30.0);
    let wage_str = c.daily_wage.iter()
        .map(|(i, n)| format!("{} x{}", i, n))
        .collect::<Vec<_>>().join(", ");
    companions.push(c);
    HireOutcome {
        ok: true,
        message: format!("{} joined you.", display_name),
        chronicle: Some(format!(
            "You hired {} of {} at ({:.0}, {:.0}). Their wage is {}. The road is less empty.",
            display_name, faction_name, position[0], position[2], wage_str
        )),
    }
}

/// The B5 quit consequence: trust memory (−15), faction standing −5, the
/// chronicle line. The caller re-creates the villager.
pub fn quit_consequence(
    lore: &lf_lore::LoreRegistry,
    standings: &mut lf_lore::StandingState,
    memory: &mut std::collections::HashMap<String, i32>,
    companion: &Companion,
    at: (f32, f32),
) -> String {
    memory.insert(companion.npc_archetype_id.clone(), companion.trust.saturating_sub(15).max(0));
    if let Some(faction) = companion.faction_id.clone() {
        standings.add(&faction, -5);
    }
    let faction_name = companion.faction_id.as_deref()
        .and_then(|f| lore.faction(f))
        .map(|f| f.short_name.clone())
        .unwrap_or_default();
    format!(
        "Your companion {} of {} departed at ({:.0}, {:.0}), their spirit worn through.",
        companion.display_name, faction_name, at.0, at.1
    )
}

impl GameState {
    /// Sync lore data into the map state (A2 territory tints, D3 icons).
    pub fn sync_map_faction_data(&mut self) {
        if self.map.faction_tints.is_empty() {
            let mut tints = std::collections::HashMap::new();
            for f in &self.lore_data.factions {
                let c = egui::Color32::from_rgb(f.color[0], f.color[1], f.color[2]);
                for b in &f.home_biomes {
                    tints.insert(*b, c);
                }
            }
            self.map.faction_tints = tints;
        }
        self.map.structure_icons = self
            .discovered_structures
            .iter()
            .filter_map(|(key, x, _, z)| {
                let color = self
                    .lore_data
                    .archetypes_for_structure(key)
                    .first()
                    .and_then(|n| n.faction.as_deref())
                    .and_then(|fid| self.lore_data.faction(fid))
                    .map(|f| egui::Color32::from_rgb(f.color[0], f.color[1], f.color[2]))?;
                Some((*x as f32 + 0.5, *z as f32 + 0.5, color))
            })
            .collect();
    }

    /// The block under the crosshair (for "Mine this" targeting).
    pub fn crosshair_block_pos(&self) -> Option<(i32, i32, i32)> {
        let eye = self.player.eye_position();
        let look = self.player.look_dir();
        lf_voxel::raycast::raycast_voxel(eye, look, crate::REACH, |pos| {
            lf_voxel::registry::is_targetable(self.world.get_block(pos.x, pos.y, pos.z))
        })
        .map(|(pos, _)| (pos.x, pos.y, pos.z))
    }

    // ------------------------------------------------------------------
    // Standing + chronicle (A1, D1, D2)
    // ------------------------------------------------------------------

    /// Apply a standing change: clamps, pulses the HUD widget, writes a
    /// chronicle entry when a threshold band is crossed (with the new
    /// title), and applies the rivals-drift rule on honored crossings.
    pub fn add_standing(&mut self, faction: &str, delta: i32) {
        let Some(fdef) = self.lore_data.faction(faction).cloned() else { return };
        let change = self.standings.add(faction, delta);
        self.faction_pulse = 1.0;
        if let Some(band) = change.band {
            let title = band.title(&fdef).to_string();
            let payload = match band {
                StandingBand::Honored | StandingBand::Friendly | StandingBand::Known => {
                    format!("{} now regards you as {}. The world notices who you stand with.", fdef.short_name, title)
                }
                _ => format!("{} now regards you as {}.", fdef.short_name, title),
            };
            self.chronicle_event(EventType::Discovery, payload);
            // honored crossing: rivals drift colder (FACTIONS_OVERVIEW)
            if band == StandingBand::Honored {
                // C3: every NPC of the faction acknowledges it on next talk
                self.honored_ack.insert(faction.to_string());
                let rivals = self.lore_data.rivals_of(faction);
                let amount = self.lore_data.standing_events.rival_honored;
                for rival in &rivals {
                    self.standings.add(rival, amount);
                }
                // D2: reference the faction's founding world event
                self.chronicle_world_event(WorldEventTrigger::StandingHonored, faction);
            }
        }
    }

    /// Reference a canonical world event by name in the chronicle (A1/D2).
    pub fn chronicle_world_event(&mut self, trigger: WorldEventTrigger, faction: &str) {
        let events = self.lore_data.world_events_for(trigger, faction);
        if events.is_empty() {
            return;
        }
        let ev = events[0];
        let payload = format!(
            "— remembered: {} ({}): {}",
            ev.name,
            ev.date(),
            ev.description
        );
        self.chronicle_event(EventType::Discovery, payload);
    }

    /// A4: a quest just completed — apply standing rewards for faction
    /// quests (issuing faction + documented ripples).
    pub fn apply_quest_standing(&mut self, quest: &lf_story::Quest) {
        let Some(faction) = quest.faction.clone() else { return };
        let base = self.lore_data.standing_events.quest_complete;
        let reward = if quest.standing_reward != 0 { quest.standing_reward } else { base };
        self.add_standing(&faction, reward);
        for (other, delta) in quest.other_standing.clone() {
            self.add_standing(&other, delta);
        }
        // C4: the faction's NPCs remember the completed quest
        let day = self.day_index as u32;
        if let Some(v) = self.villagers.iter_mut()
            .find(|v| v.faction.as_deref() == Some(faction.as_str()))
        {
            v.record_interaction(lf_npc::NpcEvent::QuestCompleted, day);
        }
        self.chronicle_world_event(WorldEventTrigger::QuestCompleted, &faction);
    }

    // ------------------------------------------------------------------
    // Territory + HUD data (A2, A3)
    // ------------------------------------------------------------------

    /// The faction whose territory the player stands in.
    pub fn territory_here(&self) -> Option<lf_lore::FactionDef> {
        let (x, z) = (self.player.position.x as i32, self.player.position.z as i32);
        let biome = self.map.biome_at(x, z);
        self.lore_data.territory_owner(biome).cloned()
    }

    /// The faction the standing widget should show: territory owner, else
    /// the nearest discovered structure's faction within 30 blocks (C4).
    pub fn standing_widget_faction(&self) -> Option<lf_lore::FactionDef> {
        if let Some(f) = self.territory_here() {
            return Some(f);
        }
        let p = self.player.position;
        let mut best: Option<(f32, lf_lore::FactionDef)> = None;
        for (key, x, y, z) in &self.discovered_structures {
            let Some(fdef) = self
                .lore_data
                .archetypes_for_structure(key)
                .first()
                .and_then(|n| n.faction.as_deref())
                .and_then(|fid| self.lore_data.faction(fid))
            else {
                continue;
            };
            let dist = ((p.x - *x as f32).powi(2) + (p.y - *y as f32).powi(2) + (p.z - *z as f32).powi(2)).sqrt();
            if dist < 30.0 && best.as_ref().map(|(bd, _)| dist < *bd).unwrap_or(true) {
                best = Some((dist, fdef.clone()));
            }
        }
        best.map(|(_, f)| f)
    }

    // ------------------------------------------------------------------
    // Structure discovery + NPC settling (C3 client side, D2, D3)
    // ------------------------------------------------------------------

    /// Scan loaded chunks near the player for faction banners: settle the
    /// faction's NPCs on first sight, record discovery (+standing, map
    /// icon, chronicle). Same cadence as try_spawn_villagers.
    pub fn try_settle_faction_npcs(&mut self) {
        let p = self.player.position;
        let cch = (p.x as i32) >> 4;
        let ccz = (p.z as i32) >> 4;
        let radius = 3;
        let mut new_markers: Vec<((i32, i32, i32), &'static str)> = Vec::new();
        for cx in (cch - radius)..=(cch + radius) {
            for cz in (ccz - radius)..=(ccz + radius) {
                let Some(col) = self.world.chunk(cx, cz) else { continue };
                for lx in 0..16usize {
                    for lz in 0..16usize {
                        for y in 40..200usize {
                            let id = col.get(lx, y, lz).id();
                            let Some(key) = structure_of_marker(id) else { continue };
                            let wx = cx * 16 + lx as i32;
                            let wz = cz * 16 + lz as i32;
                            let marker = (wx, y as i32, wz);
                            if self.settled_markers.contains(&marker) {
                                continue;
                            }
                            self.settled_markers.insert(marker);
                            new_markers.push((marker, key));
                        }
                    }
                }
            }
        }
        for ((wx, wy, wz), key) in new_markers {
            if !self
                .discovered_structures
                .iter()
                .any(|(k, x, _, z)| *k == key && *x == wx && *z == wz)
            {
                self.discovered_structures.push((key.to_string(), wx, wy, wz));
                self.chronicle_structure_discovery(key, wx, wy, wz);
            }
            // settle the roster archetypes for this structure (throttled
            // by the per-archetype villager cap inside)
            let archetypes: Vec<String> = self
                .lore_data
                .archetypes_for_structure(key)
                .iter()
                .map(|n| n.id.clone())
                .collect();
            for a in archetypes {
                self.spawn_faction_npc(&a, [wx as f32 + 0.5, wy as f32 + 1.0, wz as f32 + 1.5]);
            }
        }
    }

    fn chronicle_structure_discovery(&mut self, key: &str, x: i32, y: i32, z: i32) {
        let faction = self
            .lore_data
            .archetypes_for_structure(key)
            .first()
            .and_then(|n| n.faction.clone())
            .unwrap_or_default();
        let label = key.replace('_', " ");
        self.chronicle_event(
            EventType::Discovery,
            format!("You discovered a {} at ({}, {}, {}).", label, x, y, z),
        );
        if !faction.is_empty() {
            let amount = self.lore_data.standing_events.discover_structure;
            self.add_standing(&faction, amount);
            self.chronicle_world_event(WorldEventTrigger::StructureDiscovered, &faction);
        }
    }

    /// Spawn a faction villager from the roster (unique names per world,
    /// capped per archetype).
    fn spawn_faction_npc(&mut self, archetype_id: &str, pos: [f32; 3]) {
        let Some(arch) = self.lore_data.villager_archetype(archetype_id).cloned() else { return };
        let already = self
            .villagers
            .iter()
            .filter(|v| v.archetype.as_deref() == Some(archetype_id))
            .count();
        if already >= arch.spawn_max.max(1) as usize {
            return;
        }
        let id = 2000 + self.villagers.len() as u64 + self.next_mob_id;
        let name = arch.name_pool[(id as usize) % arch.name_pool.len()].clone();
        let mut v = lf_npc::Villager::new(id, roster_job(archetype_id), name, pos);
        v.faction = arch.faction.clone();
        v.archetype = Some(archetype_id.to_string());
        v.schedule.location = pos;
        v.workstation_pos = self.scan_workstation(v.position, 14);
        self.villagers.push(v);
        self.next_mob_id += 1;
    }

    // ------------------------------------------------------------------
    // Hiring (B2) + commands (B3) + the day cycle (B5)
    // ------------------------------------------------------------------

    /// Try to hire the villager at index `vi`. Returns the player-facing
    /// line either way; on success also writes the chronicle entry.
    pub fn try_hire(&mut self, vi: usize) -> String {
        let Some(v) = self.villagers.get(vi).cloned() else {
            return "No one to hire.".into();
        };
        let Some(archetype) = v.archetype.clone() else {
            return format!("{} is not for hire.", v.name);
        };
        let next_id = 5000 + self.next_mob_id;
        let outcome = hire_villager(
            &self.lore_data,
            &self.standings,
            &mut self.inventory,
            &mut self.companions,
            &self.companion_memory,
            next_id,
            &archetype,
            &v.name,
            v.position,
        );
        if outcome.ok {
            self.villagers.remove(vi);
            self.next_mob_id += 1;
            self.companion_cooldowns.push(0.0);
            self.companion_line_timers.push(8.0);
            if let Some(line) = &outcome.chronicle {
                self.chronicle_event(EventType::Discovery, line.clone());
            }
            let opening = self.lore_data.villager_archetype(&archetype)
                .map(|a| a.opening_hire_dialogue.clone())
                .unwrap_or_default();
            if !opening.is_empty() {
                let name = v.name.clone();
                self.push_hint(&format!("{} says: \"{}\"", name, opening));
            }
        }
        outcome.message
    }

    /// Issue a command to companion `ci` (B3). Returns the player-facing
    /// line (refusals included).
    pub fn companion_command(&mut self, ci: usize, cmd: CompanionCommand) -> String {
        let Some(c) = self.companions.get_mut(ci) else { return String::new() };
        let name = c.display_name.clone();
        match c.command(&cmd) {
            Ok(()) => match cmd {
                CompanionCommand::FollowMe => format!("{} follows you.", name),
                CompanionCommand::StayHere { .. } => format!("{} stands guard here.", name),
                CompanionCommand::Rest => format!("{} rests.", name),
                _ => format!("{} gets to work.", name),
            },
            Err(refusal) => format!("{} says: \"{}\"", name, refusal),
        }
    }

    /// Dismiss companion `ci` — they return to their schedule; trust is
    /// remembered, not punished (COMPANION_SYSTEM dismiss event).
    pub fn dismiss_companion(&mut self, ci: usize) {
        let Some(c) = self.companions.get(ci).cloned() else { return };
        self.companion_memory.insert(c.npc_archetype_id.clone(), c.trust);
        let villager_archetype = self.villager_form_of(&c.npc_archetype_id);
        let mut v = lf_npc::Villager::new(
            2000 + self.villagers.len() as u64 + self.next_mob_id,
            roster_job(&c.npc_archetype_id),
            c.display_name.clone(),
            c.home,
        );
        v.faction = c.faction_id.clone();
        v.archetype = villager_archetype;
        v.schedule.location = c.home;
        self.villagers.push(v);
        self.next_mob_id += 1;
        self.companions.remove(ci);
        if ci < self.companion_cooldowns.len() {
            self.companion_cooldowns.remove(ci);
        }
        if ci < self.companion_line_timers.len() {
            self.companion_line_timers.remove(ci);
        }
        self.companion_menu = None;
        let faction_name = c
            .faction_id
            .as_deref()
            .and_then(|f| self.lore_data.faction(f))
            .map(|f| f.short_name.clone())
            .unwrap_or_else(|| "the wilds".into());
        let (name, px, pz) = (c.display_name.clone(), self.player.position.x, self.player.position.z);
        self.push_hint(&format!(
            "{} says: \"Understood. I'll make my own way.\" They return to {} territory.",
            name, faction_name
        ));
        self.chronicle_event(
            EventType::Discovery,
            format!("You parted with {} at ({:.0}, {:.0}). The road ahead is your own again.", name, px, pz),
        );
    }

    /// The B5 quit path: morale hit zero — chronicle + faction standing
    /// drop + trust memory ("word gets around").
    pub fn companion_quit(&mut self, ci: usize) {
        let Some(c) = self.companions.get(ci).cloned() else { return };
        let villager_archetype = self.villager_form_of(&c.npc_archetype_id);
        let mut v = lf_npc::Villager::new(
            2000 + self.villagers.len() as u64 + self.next_mob_id,
            roster_job(&c.npc_archetype_id),
            c.display_name.clone(),
            c.home,
        );
        v.faction = c.faction_id.clone();
        v.archetype = villager_archetype;
        v.schedule.location = c.home;
        self.villagers.push(v);
        self.next_mob_id += 1;
        self.companions.remove(ci);
        if ci < self.companion_cooldowns.len() {
            self.companion_cooldowns.remove(ci);
        }
        if ci < self.companion_line_timers.len() {
            self.companion_line_timers.remove(ci);
        }
        self.companion_menu = None;
        let (name, px, pz) = (c.display_name.clone(), self.player.position.x, self.player.position.z);
        self.push_hint(&format!(
            "{} says: \"I've had enough. Find someone else.\" {} has left your service.",
            name, name
        ));
        // C3: a same-faction NPC within 24 blocks comments on the quit
        let here = [px, self.player.position.y, pz];
        let reactor = self.villagers.iter().find(|v| {
            v.faction == c.faction_id
                && (glam::Vec3::from(v.position) - glam::Vec3::from(here)).length() < 24.0
        }).map(|v| v.name.clone());
        if let Some(reactor) = reactor {
            self.push_hint(&lf_npc::reaction_line(&reactor,
                &lf_npc::NpcReactionEvent::CompanionMoraleZero { companion_name: name.clone() }));
        }
        let payload = quit_consequence(&self.lore_data, &mut self.standings, &mut self.companion_memory, &c, (px, pz));
        self.chronicle_event(EventType::Discovery, payload);
    }

    fn villager_form_of(&self, companion_form: &str) -> Option<String> {
        self.lore_data
            .npcs
            .iter()
            .find(|n| n.companion_form.as_deref() == Some(companion_form))
            .map(|n| n.id.clone())
    }

    /// Sunrise: advance the day, pay wages (or suffer), handle quits.
    pub fn on_day_rollover(&mut self) {
        self.day_index += 1;
        let mut quitters = Vec::new();
        let mut lines = Vec::new();
        for i in 0..self.companions.len() {
            let c = &self.companions[i];
            let payable = c
                .daily_wage
                .iter()
                .all(|(item, n)| self.count_item(item) >= *n as u32);
            if payable {
                let wage = c.daily_wage.clone();
                for (item, n) in &wage {
                    self.drain_item(item, *n as u32);
                }
                if let Some(c) = self.companions.get_mut(i) {
                    let _ = c.tick_day(true);
                }
            } else {
                lines.push(format!(
                    "{} says: \"My wage, wanderer. I don't work on promises.\"",
                    c.display_name
                ));
                let outcome = self.companions.get_mut(i).map(|c| c.tick_day(false));
                if outcome == Some(lf_game::companions::DayOutcome::Quit) {
                    quitters.push(i);
                }
            }
        }
        for line in lines {
            self.push_hint(&line);
        }
        for qi in quitters.into_iter().rev() {
            self.companion_quit(qi);
        }
    }

    /// Pay now (B3 command): manual wage payment, trust +2.
    pub fn companion_pay_now(&mut self, ci: usize) -> String {
        let Some(c) = self.companions.get(ci).cloned() else { return String::new() };
        let payable = c
            .daily_wage
            .iter()
            .all(|(item, n)| self.count_item(item) >= *n as u32);
        if !payable {
            return format!(
                "You can't cover {}'s wage ({}).",
                c.display_name,
                c.daily_wage
                    .iter()
                    .map(|(i, n)| format!("{} x{}", i, n))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        for (item, n) in &c.daily_wage {
            self.drain_item(item, *n as u32);
        }
        if let Some(c) = self.companions.get_mut(ci) {
            c.pay_now();
        }
        format!("{} takes the day's pay. Trust grows.", c.display_name)
    }

    /// Companion AI tick: follow/guard/work/defend + contextual lines.
    pub fn update_companions(&mut self, dt: f32) {
        let player = self.player.position;
        let world = &self.world;
        let solid = |x: i32, y: i32, z: i32| lf_voxel::registry::is_solid(world.get_block(x, y, z));
        // who attacked the player recently? companions defend (B4)
        let defend: Option<(usize, glam::Vec3)> = self
            .last_attacker
            .filter(|&i| self.mobs.get(i).map(|m| m.hurt_flash > 0.0).unwrap_or(false))
            .and_then(|i| self.mobs.get(i).map(|m| (i, m.position)));
        let mut actions: Vec<(usize, CompanionAction)> = Vec::new();
        let mut chats: Vec<String> = Vec::new();
        let mut cooldown = 0.0f32;
        for i in 0..self.companions.len() {
            let mut cstats = CompanionStats::default();
            {
                let c = &self.companions[i];
                if let Some(arch) = self.lore_data.companion_archetype(&c.npc_archetype_id) {
                    cstats.speed = arch.speed;
                    cstats.damage = arch.damage;
                }
            }
            // defend: swing at the mob that just hit the player
            if let Some((_, target)) = defend {
                let close = (self.companions[i].position - target).length() < 12.0;
                let following = self.companions[i].state
                    == lf_game::companions::CompanionState::Following;
                if close && following {
                    cooldown = self.companion_cooldowns.get(i).copied().unwrap_or(0.0);
                    if let Some(dmg) = self.companions[i].try_attack(target, cstats, &mut cooldown) {
                        self.companion_cooldowns[i] = cooldown;
                        actions.push((usize::MAX, CompanionAction::Crafted(format!("__attack__{dmg}"))));
                    } else {
                        self.companion_cooldowns[i] = cooldown;
                    }
                }
            }
            let action = self.companions[i].step(dt, player, &solid, cstats);
            if action != CompanionAction::None {
                actions.push((i, action));
            }
            // contextual line occasionally (~every 20-30s per companion)
            let timer = self.companion_line_timers.get_mut(i);
            if let Some(t) = timer {
                *t -= dt;
                if *t <= 0.0 {
                    *t = 20.0 + (self.companions[i].id % 13) as f32;
                    let c = &self.companions[i];
                    let biome = self.map.biome_at(c.position.x as i32, c.position.z as i32);
                    let biome_key = format!("{:?}", biome);
                    let first_structure = self.discovered_structures.first().map(|(k, ..)| k.clone());
                    let ctx = ConditionCtx {
                        standings: Some(&self.standings),
                        biome: Some(&biome_key),
                        morale: Some(c.morale),
                        trust: Some(c.trust),
                        structure_discovered: first_structure.as_deref(),
                        lore_book_found: false,
                        near_machine: !self.machine_power.is_empty(),
                    };
                    if let Some(line) = self.lore_data.companion_line_for(&c.npc_archetype_id, &ctx) {
                        chats.push(format!("{} says: \"{}\"", c.display_name, line.text));
                    }
                }
            }
        }
        for chat in chats {
            self.push_hint(&chat);
        }
        for (i, action) in actions {
            match action {
                CompanionAction::Mined(pos) => {
                    let id = self.world.get_block(pos[0], pos[1], pos[2]).id();
                    if id != block::AIR {
                        self.set_block_and_drop_to_cargo(i, (pos[0], pos[1], pos[2]), id);
                    }
                }
                CompanionAction::Chopped => {
                    let p = self.companions.get(i).map(|c| c.position).unwrap_or(player);
                    let logs = |b: u32| {
                        matches!(b, block::LOG | block::BIRCH_LOG | block::SPRUCE_LOG | block::DARK_LOG | block::CHERRY_LOG)
                    };
                    if let Some(pos) = self.nearest_block_around(p, 8, logs) {
                        let id = self.world.get_block(pos.0, pos.1, pos.2).id();
                        self.set_block_and_drop_to_cargo(i, pos, id);
                    }
                }
                CompanionAction::Hauled => {
                    if let Some(c) = self.companions.get_mut(i) {
                        c.cargo.clear();
                    }
                    self.push_hint("Cargo stored in the chest.");
                }
                CompanionAction::Crafted(tag) => {
                    if let Some(dmg) = tag.strip_prefix("__attack__") {
                        let dmg: f32 = dmg.parse().unwrap_or(4.0);
                        if let Some((mi, _)) = defend {
                            if let Some(m) = self.mobs.get_mut(mi) {
                                let dead = m.take_hit(dmg, self.player.position);
                                if dead {
                                    let kind = m.mob_type;
                                    self.kills += 1;
                                    let name = mob_kind_id(kind).to_string();
                                    self.mobs.remove(mi);
                                    self.last_attacker = None;
                                    self.quest_event(lf_story::QuestEvent::Killed(name));
                                }
                            }
                        }
                    }
                }
                CompanionAction::None => {}
            }
        }
    }

    fn set_block_and_drop_to_cargo(&mut self, ci: usize, pos: (i32, i32, i32), id: u32) {
        if let Some(drop) = lf_game::items::block_drop(id) {
            if let Some(c) = self.companions.get_mut(ci) {
                match c.cargo.iter_mut().find(|(item, _)| *item == drop) {
                    Some((_, n)) => *n = n.saturating_add(1),
                    None => {
                        if c.cargo.len() < 9 {
                            c.cargo.push((drop, 1));
                        }
                    }
                }
            }
        }
        self.world.set_block(pos.0, pos.1, pos.2, lf_voxel::BlockState(block::AIR));
        self.dirty.insert((pos.0 >> 4, pos.2 >> 4));
    }

    pub fn count_item(&self, item: &str) -> u32 {
        self.inventory
            .slots
            .iter()
            .filter_map(|s| s.as_ref())
            .filter(|s| s.item_id == item)
            .map(|s| s.count as u32)
            .sum()
    }

    pub fn drain_item(&mut self, item: &str, count: u32) {
        let mut left = count;
        for slot in self.inventory.slots.iter_mut() {
            if left == 0 {
                break;
            }
            if let Some(s) = slot {
                if s.item_id == item {
                    let take = left.min(s.count as u32);
                    left -= take;
                    s.count -= take as u8;
                    if s.count == 0 {
                        *slot = None;
                    }
                }
            }
        }
    }

    /// Nearest block matching a predicate within `radius` of a position
    /// (Chop, proximity quest tags, embers).
    pub fn nearest_block_around(
        &self,
        center: glam::Vec3,
        radius: i32,
        pred: impl Fn(u32) -> bool,
    ) -> Option<(i32, i32, i32)> {
        let cx = center.x as i32;
        let cy = center.y as i32;
        let cz = center.z as i32;
        let mut best: Option<(i32, i32, i32, i32)> = None;
        for dx in -radius..=radius {
            for dy in -3..=5i32 {
                for dz in -radius..=radius {
                    let (x, y, z) = (cx + dx, cy + dy, cz + dz);
                    if pred(self.world.get_block(x, y, z).id()) {
                        let d = dx.abs() + dy.abs() + dz.abs();
                        if best.map(|(bd, ..)| d < bd).unwrap_or(true) {
                            best = Some((d, x, y, z));
                        }
                    }
                }
            }
        }
        best.map(|(_, x, y, z)| (x, y, z))
    }

    // ------------------------------------------------------------------
    // Quest world tags (A4 event emission)
    // ------------------------------------------------------------------

    /// Periodic proximity checks feeding the Reach quest events: road
    /// markers (accord pillars), ember formations, new biomes. Each tag
    /// only fires once per world cell so proximity can't farm progress.
    pub fn quest_tag_checks(&mut self) {
        let p = self.player.position;
        let biome = self.map.biome_at(p.x as i32, p.z as i32);
        let biome_key = format!("{:?}", biome);
        if !self.visited_biomes.contains(&biome_key) {
            self.visited_biomes.insert(biome_key.clone());
            // ashen_q2: each first-visited biome advances the survey
            self.quest_event(lf_story::QuestEvent::Reached("new_biome".into()));
        }
        if self.frame % 20 != 0 {
            return;
        }
        let cell = |x: i32, z: i32| (x >> 3, z >> 3);
        // road markers: any accord pillar within 5 blocks
        if let Some((x, _, z)) = self.nearest_block_around(p, 5, |b| b == block::ACCORD_PILLAR) {
            let c = cell(x, z);
            if self.road_cells.insert(c) {
                self.quest_event(lf_story::QuestEvent::Reached("road_marker".into()));
            }
        }
        // ember formations: a cluster of >= 2 glowstone within 6 blocks
        let ember = |b: u32| b == block::EMBER_GLOWSTONE;
        if let Some(a) = self.nearest_block_around(p, 6, ember) {
            if let Some(b) = self.nearest_block_around(glam::Vec3::new(a.0 as f32, a.1 as f32, a.2 as f32), 2, ember) {
                if b != a {
                    let c = cell(a.0, a.2);
                    if self.ember_cells.insert(c) {
                        self.quest_event(lf_story::QuestEvent::Reached("ember_formation".into()));
                    }
                }
            }
        }
    }

    /// Interacting with a faction NPC: adds their faction quests (once),
    /// fires Interact objectives, returns the dialogue line + whether the
    /// menu stays closed (hostile standing).
    pub fn npc_interact(&mut self, archetype: &str) -> Option<(String, bool)> {
        let faction = self
            .lore_data
            .villager_archetype(archetype)
            .and_then(|a| a.faction.clone());
        if let Some(faction) = faction {
            let fresh: Vec<lf_story::Quest> = self
                .lore_data
                .faction_quests
                .iter()
                .filter(|q| {
                    q.faction.as_deref() == Some(faction.as_str())
                        && !self.quest_log.quests.iter().any(|existing| existing.id == q.id)
                })
                .cloned()
                .collect();
            for q in fresh {
                self.quest_log.add_quest(q);
            }
        }
        self.quest_event(lf_story::QuestEvent::Interacted(archetype.into()));
        let biome = self
            .map
            .biome_at(self.player.position.x as i32, self.player.position.z as i32);
        let biome_key = format!("{:?}", biome);
        let ctx = ConditionCtx {
            standings: Some(&self.standings),
            biome: Some(&biome_key),
            ..Default::default()
        };
        self.lore_data
            .dialogue_for(archetype, &ctx)
            .map(|n| (n.text.clone(), n.action() == lf_lore::DialogueAction::Close))
    }

    // ------------------------------------------------------------------
    // Ambient ember particles (C4)
    // ------------------------------------------------------------------

    pub fn ambient_ember_particles(&mut self, dt: f32) {
        if !self.settings.particles {
            return;
        }
        self.ember_timer -= dt;
        if self.ember_timer > 0.0 {
            return;
        }
        self.ember_timer = 0.4; // ~2-3 checks per second, 2 sparks each
        let p = self.player.position;
        if let Some((x, y, z)) = self.nearest_block_around(p, 14, |b| b == block::EMBER_GLOWSTONE) {
            for k in 0..2u32 {
                let jitter = ((self.frame.wrapping_mul(31) + x as u64 + k as u64) % 7) as f32 / 7.0;
                self.particles.push(crate::Particle {
                    position: glam::Vec3::new(
                        x as f32 + 0.2 + jitter * 0.6,
                        y as f32 + 1.05,
                        z as f32 + 0.2 + jitter * 0.5,
                    ),
                    velocity: glam::Vec3::new((jitter - 0.5) * 0.4, 0.3 + jitter * 0.2, (jitter - 0.5) * 0.4),
                    life: 1.5 + jitter,
                    tex: lf_assets::EMBER_LAYER,
                    uv_off: [jitter * 0.5, 0.25],
                });
            }
            while self.particles.len() > 192 {
                self.particles.remove(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lf_game::survival::Inventory;

    fn lore() -> lf_lore::LoreRegistry {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join("lore");
        lf_lore::LoreRegistry::load(&dir)
    }

    /// B2 verify: sufficient standing + fee deduction + companion state
    /// transition + the confirmation chronicle entry — through the same
    /// code path the trade UI uses (hire_villager).
    #[test]
    fn hire_flow_works_end_to_end() {
        let reg = lore();
        let mut standings = lf_lore::StandingState::starting(&reg);
        standings.add("accord", 75); // the hire threshold
        let mut inv = Inventory::new();
        inv.add_item("iron_ingot", 30);
        let mut companions = Vec::new();
        let memory = std::collections::HashMap::new();

        let out = hire_villager(&reg, &standings, &mut inv, &mut companions, &memory,
            5001, "accord_herald", "Herald Aldis", [8.0, 64.0, 8.0]);
        assert!(out.ok, "hire failed: {}", out.message);
        assert_eq!(companions.len(), 1);
        let c = &companions[0];
        assert_eq!(c.npc_archetype_id, "accord_warden");
        assert_eq!(c.faction_id.as_deref(), Some("accord"));
        assert_eq!(c.state, lf_game::companions::CompanionState::Following);
        assert_eq!(c.trust, 0);
        assert_eq!(c.morale, 50);
        assert_eq!(c.daily_wage, vec![("iron_ingot".to_string(), 8)]);
        // the 12-ingot fee was deducted from the inventory
        let left: u32 = inv.slots.iter().filter_map(|s| s.as_ref())
            .filter(|s| s.item_id == "iron_ingot").map(|s| s.count as u32).sum();
        assert_eq!(left, 30 - 12);
        // the chronicle entry names the hire (DIALOGUE_FRAME template)
        let chronicle = out.chronicle.unwrap();
        assert!(chronicle.contains("You hired Herald Aldis of The Accord"), "{}", chronicle);
        assert!(chronicle.contains("The road is less empty."));
    }

    /// B2 gates: insufficient standing refuses; the 4th companion is
    /// refused with the documented message.
    #[test]
    fn hire_flow_gates() {
        let reg = lore();
        let mut standings = lf_lore::StandingState::starting(&reg);
        standings.add("accord", 74); // one below the threshold
        let mut inv = Inventory::new();
        inv.add_item("iron_ingot", 64);
        let mut companions = Vec::new();
        let memory = std::collections::HashMap::new();
        let out = hire_villager(&reg, &standings, &mut inv, &mut companions, &memory,
            1, "accord_herald", "Herald Cora", [0.0, 64.0, 0.0]);
        assert!(!out.ok);
        assert!(out.message.contains("will not serve"), "{}", out.message);
        assert!(out.chronicle.is_none());
        assert!(companions.is_empty());

        // capacity: three hired, the fourth refused
        standings.add("accord", 1);
        for i in 0..3 {
            let out = hire_villager(&reg, &standings, &mut inv, &mut companions, &memory,
                10 + i, "accord_herald", "Herald", [0.0, 64.0, 0.0]);
            assert!(out.ok);
        }
        let out = hire_villager(&reg, &standings, &mut inv, &mut companions, &memory,
            99, "accord_herald", "Herald Venner", [0.0, 64.0, 0.0]);
        assert!(!out.ok);
        assert!(out.message.contains("three companions"), "{}", out.message);
        assert_eq!(companions.len(), 3);
    }

    /// B5 verify: the morale-zero quit path — state transition out of the
    /// party, chronicle entry, faction standing drop, trust memory.
    #[test]
    fn quit_path_at_zero_morale() {
        let reg = lore();
        let mut standings = lf_lore::StandingState::starting(&reg);
        standings.add("ironborn", 40);
        let mut c = Companion::new(
            7, "ironborn_artisan", "Brunn".into(), Some("ironborn".into()),
            vec![("iron_ingot".into(), 6)], glam::Vec3::new(0.0, 64.0, 0.0),
        );
        c.trust = 60;
        c.morale = 10;
        // unpaid days grind morale to zero -> Quit
        let mut outcome = lf_game::companions::DayOutcome::Unpaid;
        while outcome == lf_game::companions::DayOutcome::Unpaid {
            outcome = c.tick_day(false);
        }
        assert_eq!(outcome, lf_game::companions::DayOutcome::Quit);
        assert_eq!(c.morale, 0);
        let mut memory = std::collections::HashMap::new();
        let payload = quit_consequence(&reg, &mut standings, &mut memory, &c, (12.0, -3.0));
        assert!(payload.contains("Brunn of The Ironborn departed"), "{}", payload);
        assert!(payload.contains("spirit worn through"));
        // word gets around: faction standing dropped by 5
        assert_eq!(standings.get("ironborn"), 35);
        // they remember: trust carried forward minus 15
        assert_eq!(memory.get("ironborn_artisan"), Some(&45));
    }

    /// A4 verify: completing a faction quest changes standing by the
    /// documented amount (+15 for the issuing faction, ripples for
    /// others), and the result survives a ClientSave round-trip.
    #[test]
    fn quest_completion_changes_standing_and_surde_round_trips() {
        let reg = lore();
        let mut standings = lf_lore::StandingState::starting(&reg);
        // accord_q2: +15 accord, -5 free_holds
        let quest = reg.faction_quests.iter()
            .find(|q| q.id == "accord_q2_nameless_camp").unwrap().clone();
        assert_eq!(quest.standing_reward, 15);
        assert_eq!(quest.other_standing, vec![("free_holds".to_string(), -5)]);
        standings.add(quest.faction.as_deref().unwrap_or_default(), quest.standing_reward);
        for (other, delta) in &quest.other_standing {
            standings.add(other, *delta);
        }
        assert_eq!(standings.get("accord"), 15);
        assert_eq!(standings.get("free_holds"), -5);
        assert_eq!(standings.get("nameless"), -50, "untouched faction unchanged");

        // ClientSave round-trip keeps standings + companions + memory
        let mut save = crate::ClientSave::default();
        save.faction_standing = Some(standings.clone());
        save.companions = vec![Companion::new(
            9, "accord_warden", "Herald Aldis".into(), Some("accord".into()),
            vec![("iron_ingot".into(), 8)], glam::Vec3::new(1.0, 64.0, 2.0),
        )];
        save.companion_memory.insert("accord_warden".into(), 42);
        save.visited_biomes = vec!["Meadow".into(), "Volcanic".into()];
        save.discovered_structures = vec![("accord_embassy".to_string(), 4, 64, -2)];
        save.day_index = 3;
        let bytes = serde_json::to_vec(&save).unwrap();
        let back: crate::ClientSave = serde_json::from_slice(&bytes).unwrap();
        let loaded = back.faction_standing.unwrap();
        assert_eq!(loaded.get("accord"), 15);
        assert_eq!(loaded.get("free_holds"), -5);
        assert_eq!(loaded.get("nameless"), -50);
        assert_eq!(back.companions.len(), 1);
        assert_eq!(back.companions[0].npc_archetype_id, "accord_warden");
        assert_eq!(back.companion_memory.get("accord_warden"), Some(&42));
        assert_eq!(back.visited_biomes, vec!["Meadow".to_string(), "Volcanic".to_string()]);
        assert_eq!(back.discovered_structures.len(), 1);
        assert_eq!(back.day_index, 3);
    }

    /// Structure markers map to their factions; every faction has a home.
    #[test]
    fn structure_markers_and_territory() {
        use lf_voxel::registry::block;
        assert_eq!(structure_of_marker(block::BANNER_ACCORD), Some("accord_embassy"));
        assert_eq!(structure_of_marker(block::BANNER_NAMELESS), Some("nameless_camp"));
        assert_eq!(structure_of_marker(block::STONE), None);
        assert_eq!(faction_of_block(block::ACCORD_PILLAR), Some("accord"));
        assert_eq!(faction_of_block(block::EMBER_GLOWSTONE), Some("ember_covenant"));
        assert_eq!(faction_of_block(block::STONE), None);
    }
}
