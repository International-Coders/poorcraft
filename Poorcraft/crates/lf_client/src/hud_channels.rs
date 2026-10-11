//! N04 (nightly-beta `03-HUD-AND-CRAFTING`): the priority-safe contextual
//! HUD channels — interaction prompts beside the crosshair, combat hit
//! direction + attack readiness, reputation deltas, settlement entry
//! banners, and survival warnings. State and copy logic live here (pure,
//! unit-tested); the shared painters in `ui.rs` draw them, and the vistest
//! proofs call the same painters.

use crate::input::{key_glyph, Action, Keymap};

/// Seconds a reputation toast stays on screen.
pub const REP_TOAST_LIFE: f32 = 4.5;
/// Seconds a settlement entry banner stays.
pub const SETTLEMENT_LIFE: f32 = 5.5;
/// Seconds the hit-direction arc stays after a hit lands.
pub const HIT_DIR_LIFE: f32 = 1.6;

// ------------------------------------------------------------------
// Interaction prompt (beside the crosshair)

/// What the crosshair is on, in priority order — the same facts the
/// E-interact and mouse handlers resolve, distilled for the prompt.
#[derive(Clone, Debug, PartialEq)]
pub enum Focus<'a> {
    /// A companion awaits orders (E opens the command menu).
    Companion { name: &'a str },
    /// A faction NPC (E trades/greets; hostile standing bars the door).
    Villager { name: &'a str, barred: bool },
    /// A functional block E actually opens (chest, furnace, table...).
    Interactable { verb: &'a str, name: &'a str },
    /// A minable block under the crosshair.
    Mine { name: &'a str },
    /// A placeable block held while aiming at a face.
    Place { name: &'a str, blocked_by_player: bool },
    /// Nothing actionable.
    None,
}

/// The prompt rendered beside the crosshair: chips (key glyphs), a verb,
/// the target name, and an optional blocked reason.
#[derive(Clone, Debug, PartialEq)]
pub struct InteractionPrompt {
    pub chips: Vec<String>,
    pub verb: String,
    pub target: String,
    pub blocked: Option<String>,
}

impl InteractionPrompt {
    /// The one-line form (tests + the settlement of copy questions).
    pub fn line(&self) -> String {
        let keys = self.chips.join(" ");
        match &self.blocked {
            Some(reason) => format!("{} {} — {} ({})", keys, self.verb, self.target, reason),
            None => format!("{} {} — {}", keys, self.verb, self.target),
        }
    }
}

/// Build the prompt for the current focus, with chips from the LIVE
/// keymap (rebinding changes the prompt, like the tutorial card).
pub fn interaction_prompt(focus: &Focus, keys: &Keymap) -> Option<InteractionPrompt> {
    let g = |a: Action| key_glyph(keys.key(a));
    match focus {
        Focus::None => None,
        Focus::Companion { name } => Some(InteractionPrompt {
            chips: vec![g(Action::Inventory)],
            verb: "Command".into(),
            target: name.to_string(),
            blocked: None,
        }),
        Focus::Villager { name, barred } => Some(InteractionPrompt {
            chips: vec![g(Action::Inventory)],
            verb: "Trade".into(),
            target: name.to_string(),
            blocked: barred.then(|| "gate barred (Hostile)".to_string()),
        }),
        Focus::Interactable { verb, name } => Some(InteractionPrompt {
            chips: vec![g(Action::Inventory)],
            verb: verb.to_string(),
            target: name.to_string(),
            blocked: None,
        }),
        Focus::Mine { name } => Some(InteractionPrompt {
            chips: vec!["LMB".into()],
            verb: "Hold to mine".into(),
            target: name.to_string(),
            blocked: None,
        }),
        Focus::Place { name, blocked_by_player } => Some(InteractionPrompt {
            chips: vec!["RMB".into()],
            verb: "Place".into(),
            target: name.to_string(),
            blocked: blocked_by_player.then(|| "blocked by player".to_string()),
        }),
    }
}

// ------------------------------------------------------------------
// Transient channels

/// A standing delta worth showing: crest color, signed delta, the reason
/// it happened, and the threshold title when a band was crossed.
#[derive(Clone, Debug)]
pub struct ReputationToast {
    pub faction_short: String,
    pub color: [u8; 3],
    pub delta: i32,
    pub reason: String,
    pub title_line: Option<String>,
    pub age: f32,
}

/// Realm-and-place banner on settlement entry.
#[derive(Clone, Debug)]
pub struct SettlementBanner {
    pub name: String,
    pub state_line: String,
    pub age: f32,
}

/// Hit-direction marker: bearing is radians relative to the player's
/// facing (0 = dead ahead), fading fast.
#[derive(Copy, Clone, Debug)]
pub struct HitDir {
    pub bearing: f32,
    pub age: f32,
}

/// The channel manager. One message per channel; everything fades; the
/// danger line is recomputed each frame (it is a state readout, not a
/// queued event) so it can never stack.
#[derive(Default)]
pub struct HudChannels {
    pub rep_toasts: Vec<ReputationToast>,
    pub settlement: Option<SettlementBanner>,
    pub hit_dir: Option<HitDir>,
}

impl HudChannels {
    pub fn tick(&mut self, dt: f32) {
        self.rep_toasts.retain_mut(|t| {
            t.age += dt;
            t.age < REP_TOAST_LIFE
        });
        if self.rep_toasts.len() > 3 {
            let drop = self.rep_toasts.len() - 3;
            self.rep_toasts.drain(0..drop);
        }
        if let Some(s) = &mut self.settlement {
            s.age += dt;
            if s.age >= SETTLEMENT_LIFE {
                self.settlement = None;
            }
        }
        if let Some(h) = &mut self.hit_dir {
            h.age += dt;
            if h.age >= HIT_DIR_LIFE {
                self.hit_dir = None;
            }
        }
    }

    pub fn push_rep_toast(&mut self, toast: ReputationToast) {
        self.rep_toasts.push(toast);
    }

    pub fn enter_settlement(&mut self, name: String, state_line: String) {
        self.settlement = Some(SettlementBanner { name, state_line, age: 0.0 });
    }

    pub fn note_hit_from(&mut self, bearing: f32) {
        self.hit_dir = Some(HitDir { bearing, age: 0.0 });
    }

    /// The single most urgent survival warning, in strict priority:
    /// drowning > low health > starvation > threats. Shape (severity
    /// 0..=2) rides along so the painter can scale urgency without
    /// color alone.
    pub fn danger_warning(&self, health_frac: f32, hunger: f32, air: u8, threats: u8)
        -> Option<(String, u8)> {
        if air <= 3 {
            return Some(("DROWNING — surface now".into(), 2));
        }
        if health_frac <= 0.25 {
            return Some(("health critical".into(), 2));
        }
        if hunger <= 2.0 {
            return Some(("starving — eat something".into(), 1));
        }
        if health_frac <= 0.45 {
            return Some(("health low".into(), 1));
        }
        if threats >= 2 {
            return Some((format!("{} hostiles closing", threats), 1));
        } else if threats == 1 {
            return Some(("hostile nearby".into(), 0));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::KeyCode;

    #[test]
    fn prompts_cover_every_focus_with_keymap_chips() {
        let keys = Keymap::default();
        // villager trade
        let p = interaction_prompt(&Focus::Villager { name: "Mara", barred: false }, &keys).unwrap();
        assert_eq!(p.chips, vec!["E".to_string()]);
        assert_eq!(p.line(), "E Trade — Mara");
        // hostile standing bars the door
        let p = interaction_prompt(&Focus::Villager { name: "Mara", barred: true }, &keys).unwrap();
        assert!(p.line().contains("gate barred (Hostile)"), "{}", p.line());
        // mining and placing
        let p = interaction_prompt(&Focus::Mine { name: "Iron Ore" }, &keys).unwrap();
        assert_eq!(p.line(), "LMB Hold to mine — Iron Ore");
        let p = interaction_prompt(
            &Focus::Place { name: "Planks", blocked_by_player: true }, &keys).unwrap();
        assert!(p.line().contains("blocked by player"), "{}", p.line());
        // nothing actionable stays quiet
        assert!(interaction_prompt(&Focus::None, &keys).is_none());
        // rebinding changes the chip like the tutorial card
        let mut rebound = Keymap::default();
        rebound.rebind(Action::Inventory, KeyCode::KeyO);
        let p = interaction_prompt(&Focus::Interactable { verb: "Open", name: "Chest" }, &rebound).unwrap();
        assert_eq!(p.chips, vec!["O".to_string()]);
    }

    #[test]
    fn transients_tick_out_and_cap() {
        let mut ch = HudChannels::default();
        for i in 0..6 {
            ch.push_rep_toast(ReputationToast {
                faction_short: format!("F{i}"), color: [1, 2, 3], delta: 1,
                reason: "test".into(), title_line: None, age: 0.0,
            });
        }
        assert!(ch.rep_toasts.len() <= 6);
        ch.tick(0.016);
        assert!(ch.rep_toasts.len() <= 3, "at most three toasts stack");
        ch.enter_settlement("Elderfall".into(), "kingdom — safe".into());
        ch.note_hit_from(0.7);
        for _ in 0..400 {
            ch.tick(0.016);
        }
        assert!(ch.rep_toasts.is_empty(), "toasts fade out");
        assert!(ch.settlement.is_none(), "banner fades out");
        assert!(ch.hit_dir.is_none(), "hit direction fades out");
    }

    /// Mutation question: remove the priority ordering and this test must
    /// fail — drowning outranks low health, threats rank last.
    #[test]
    fn danger_priority_is_strict() {
        let ch = HudChannels::default();
        assert_eq!(
            ch.danger_warning(0.1, 0.0, 2, 3),
            Some(("DROWNING — surface now".to_string(), 2)));
        assert_eq!(ch.danger_warning(0.1, 0.0, 8, 0).unwrap().0, "health critical");
        assert_eq!(ch.danger_warning(0.9, 1.0, 8, 0).unwrap().0, "starving — eat something");
        assert_eq!(ch.danger_warning(0.3, 8.0, 8, 0).unwrap().0, "health low");
        assert_eq!(ch.danger_warning(0.9, 8.0, 8, 2).unwrap().0, "2 hostiles closing");
        assert_eq!(ch.danger_warning(0.9, 8.0, 8, 0), None, "healthy and safe stays quiet");
    }
}
