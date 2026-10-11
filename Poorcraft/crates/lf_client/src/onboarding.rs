//! N01 (nightly-beta `03-HUD-AND-CRAFTING`): the persisted first-minute
//! tutorial state machine and the pinned starter-objective source. This
//! module is pure state + copy — `ui::paint_onboarding_prompt` /
//! `ui::paint_pinned_objective` draw it with the real HUD painters (the
//! vistest proofs call the same painters), and `GameState` feeds observed
//! gameplay facts in. Prompts adapt to the live keymap so a rebound "use"
//! key is what the card shows.

use serde::{Deserialize, Serialize};

use crate::input::{key_glyph, Action, Keymap};

/// Horizontal displacement (blocks) that completes the Move step.
pub const MOVE_COMPLETE_BLOCKS: f32 = 3.0;
/// Combined |Δyaw|+|Δpitch| (radians) that completes the Look step.
pub const LOOK_COMPLETE_RADIANS: f32 = 1.6;

/// The ordered tutorial. `Done` hands the top-center slot to the pinned
/// starter objective full-time.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TutorialStep {
    /// Walk with the movement keys.
    Move,
    /// Turn the camera.
    Look,
    /// Mine a natural block until its drop reaches the inventory.
    Gather,
    /// Craft in the hand grid (E) — planks first.
    Craft,
    /// Place a solid block.
    Build,
    /// Finished; the pinned objective owns the slot.
    Done,
}

impl TutorialStep {
    /// 1-based position for the "n/5" chip (Done reads as 5/5).
    pub fn number(self) -> usize {
        match self {
            TutorialStep::Move => 1,
            TutorialStep::Look => 2,
            TutorialStep::Gather => 3,
            TutorialStep::Craft => 4,
            TutorialStep::Build | TutorialStep::Done => 5,
        }
    }

    pub const TOTAL: usize = 5;
}

/// Item ids whose pickup counts as "gathered from the land" — the natural
/// surface/punchable materials a fresh player reaches first.
const NATURAL_PICKUPS: [&str; 4] = ["log", "dirt", "stone", "sand"];

/// Persisted tutorial state (rides in `ClientSave`; a fresh world resets
/// it, an old save without the field starts at Move).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Onboarding {
    pub step: TutorialStep,
    /// The player hid the tutorial card (Gameplay settings restores it).
    pub dismissed: bool,
    /// The player hid the pinned objective line.
    pub objective_dismissed: bool,
    /// Accumulated horizontal displacement while on Move (blocks).
    move_accum: f32,
    /// Accumulated look travel while on Look (radians).
    look_accum: f32,
    /// Last observed pose, for delta computation (None on the first frame).
    last_pos: Option<[f32; 3]>,
    last_yaw: f32,
    last_pitch: f32,
}

impl Default for Onboarding {
    fn default() -> Self {
        Self {
            step: TutorialStep::Move,
            dismissed: false,
            objective_dismissed: false,
            move_accum: 0.0,
            look_accum: 0.0,
            last_pos: None,
            last_yaw: 0.0,
            last_pitch: 0.0,
        }
    }
}

/// What the HUD card paints: a verb, key chips, and a short action label.
/// Chips come from the live keymap so rebinding changes the card.
#[derive(Clone, Debug, PartialEq)]
pub struct OnboardingPrompt {
    pub verb: String,
    pub chips: Vec<String>,
    pub label: String,
}

impl Onboarding {
    /// Feed the per-frame pose. Displacement and look travel only advance
    /// their own step, so idling never completes Move and standing still
    /// with wild mouse swings completes Look but not Move.
    pub fn observe_frame(&mut self, pos: [f32; 3], yaw: f32, pitch: f32) {
        if let Some(lp) = self.last_pos {
            if self.step == TutorialStep::Move {
                let dx = pos[0] - lp[0];
                let dz = pos[2] - lp[2];
                self.move_accum += (dx * dx + dz * dz).sqrt();
                if self.move_accum >= MOVE_COMPLETE_BLOCKS {
                    self.step = TutorialStep::Look;
                }
            }
            if self.step == TutorialStep::Look {
                self.look_accum += (yaw - self.last_yaw).abs() + (pitch - self.last_pitch).abs();
                if self.look_accum >= LOOK_COMPLETE_RADIANS {
                    self.step = TutorialStep::Gather;
                }
            }
        }
        self.last_pos = Some(pos);
        self.last_yaw = yaw;
        self.last_pitch = pitch;
    }

    /// A drop reached the inventory. Only natural materials count while on
    /// Gather — picking up a dropped sword teaches nothing about mining.
    pub fn observe_collected(&mut self, item_id: &str) {
        if self.step == TutorialStep::Gather && NATURAL_PICKUPS.contains(&item_id) {
            self.step = TutorialStep::Craft;
        }
    }

    /// Any hand-crafted output completes Craft (the prompt points at
    /// planks; the starter quest chain reinforces it).
    pub fn observe_crafted(&mut self) {
        if self.step == TutorialStep::Craft {
            self.step = TutorialStep::Build;
        }
    }

    /// A block was placed; only solid placements complete Build (torches
    /// and flowers are not shelter).
    pub fn observe_placed(&mut self, solid: bool) {
        if self.step == TutorialStep::Build && solid {
            self.step = TutorialStep::Done;
        }
    }

    /// Settings action: walk the whole tutorial again from scratch.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The card copy for the current step, with chips from the live
    /// keymap (a rebound inventory key shows its real glyph).
    pub fn prompt(&self, keys: &Keymap) -> OnboardingPrompt {
        let g = |a: Action| key_glyph(keys.key(a));
        match self.step {
            TutorialStep::Move => OnboardingPrompt {
                verb: "Find your footing".into(),
                chips: vec![g(Action::Forward), g(Action::Left), g(Action::Back), g(Action::Right)],
                label: "walk — any direction".into(),
            },
            TutorialStep::Look => OnboardingPrompt {
                verb: "Look around".into(),
                chips: Vec::new(),
                label: "move the mouse to turn".into(),
            },
            TutorialStep::Gather => OnboardingPrompt {
                verb: "Gather from the land".into(),
                chips: vec!["LMB".into()],
                label: "hold on a tree — wood for everything".into(),
            },
            TutorialStep::Craft => OnboardingPrompt {
                verb: "Shape your first planks".into(),
                chips: vec![g(Action::Inventory)],
                label: "open the pack · craft logs into planks".into(),
            },
            TutorialStep::Build | TutorialStep::Done => OnboardingPrompt {
                verb: "Raise a shelter".into(),
                chips: vec!["RMB".into()],
                label: "place a solid block — night comes".into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::KeyCode;

    fn walking() -> Onboarding {
        Onboarding::default()
    }

    fn advance_to(ob: &mut Onboarding, step: TutorialStep) {
        // drive the machine with synthetic observations until it reaches
        // the requested step (the public API is observation-only)
        let mut t = 0.0f32;
        let mut guard = 0;
        while ob.step != step && ob.step != TutorialStep::Done && guard < 64 {
            guard += 1;
            t += 1.5;
            match ob.step {
                TutorialStep::Move => ob.observe_frame([t, 0.0, t], 0.0, 0.0),
                TutorialStep::Look => ob.observe_frame([t, 0.0, t], t, -t * 0.25),
                TutorialStep::Gather => ob.observe_collected("log"),
                TutorialStep::Craft => ob.observe_crafted(),
                TutorialStep::Build => ob.observe_placed(true),
                TutorialStep::Done => break,
            }
        }
        assert_eq!(ob.step, step, "advance_to did not reach the step");
    }

    #[test]
    fn movement_then_look_complete_in_order() {
        let mut ob = walking();
        // seed the pose
        ob.observe_frame([0.0, 0.0, 0.0], 0.0, 0.0);
        // small drift below the threshold does not complete
        ob.observe_frame([0.4, 0.0, 0.0], 0.0, 0.0);
        assert_eq!(ob.step, TutorialStep::Move);
        // accumulate past the threshold (jump/fall on Y must not count:
        // only horizontal displacement advances the step)
        ob.observe_frame([0.4, 900.0, 0.0], 0.0, 0.0);
        ob.observe_frame([3.5, 900.0, 0.0], 0.0, 0.0);
        assert_eq!(ob.step, TutorialStep::Look);
        // look travel accumulates
        ob.observe_frame([3.5, 900.0, 0.0], 0.8, 0.1);
        assert_eq!(ob.step, TutorialStep::Look);
        ob.observe_frame([3.5, 900.0, 0.0], 2.0, 0.1);
        assert_eq!(ob.step, TutorialStep::Gather);
    }

    #[test]
    fn look_without_moving_does_not_skip_move() {
        let mut ob = walking();
        ob.observe_frame([0.0, 0.0, 0.0], 0.0, 0.0);
        // wild camera, zero displacement: still on Move
        for k in 0..16 {
            ob.observe_frame([0.0, 0.0, 0.0], k as f32, -k as f32 * 0.5);
        }
        assert_eq!(ob.step, TutorialStep::Move);
    }

    #[test]
    fn natural_pickup_completes_gather_but_junk_does_not() {
        let mut ob = walking();
        advance_to(&mut ob, TutorialStep::Gather);
        ob.observe_collected("iron_sword");
        assert_eq!(ob.step, TutorialStep::Gather, "a picked-up item is not mining");
        ob.observe_collected("sand");
        assert_eq!(ob.step, TutorialStep::Craft);
    }

    #[test]
    fn crafting_then_solid_placement_finish_the_tutorial() {
        let mut ob = walking();
        advance_to(&mut ob, TutorialStep::Craft);
        ob.observe_crafted();
        assert_eq!(ob.step, TutorialStep::Build);
        // a torch is not shelter
        ob.observe_placed(false);
        assert_eq!(ob.step, TutorialStep::Build);
        ob.observe_placed(true);
        assert_eq!(ob.step, TutorialStep::Done);
    }

    #[test]
    fn out_of_order_events_are_ignored() {
        let mut ob = walking();
        // everything at once from a fresh state must not skip ahead
        ob.observe_collected("log");
        ob.observe_crafted();
        ob.observe_placed(true);
        assert_eq!(ob.step, TutorialStep::Move);
    }

    #[test]
    fn reset_returns_to_a_fresh_machine() {
        let mut ob = walking();
        advance_to(&mut ob, TutorialStep::Done);
        ob.dismissed = true;
        ob.reset();
        assert_eq!(ob.step, TutorialStep::Move);
        assert!(!ob.dismissed);
        assert_eq!(ob.move_accum, 0.0);
    }

    #[test]
    fn serde_round_trip_preserves_progress() {
        let mut ob = walking();
        advance_to(&mut ob, TutorialStep::Gather);
        let bytes = serde_json::to_vec(&ob).unwrap();
        let back: Onboarding = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.step, TutorialStep::Gather);
        assert_eq!(back.step.number(), 3);
        assert_eq!(TutorialStep::TOTAL, 5);
    }

    #[test]
    fn prompts_adapt_to_the_live_keymap() {
        let mut keys = Keymap::default();
        let mut ob = walking();
        advance_to(&mut ob, TutorialStep::Craft);
        let prompt = ob.prompt(&keys);
        assert_eq!(prompt.chips, vec!["E".to_string()], "default inventory key is E");
        keys.rebind(Action::Inventory, KeyCode::KeyO);
        let prompt = ob.prompt(&keys);
        assert_eq!(prompt.chips, vec!["O".to_string()], "rebound key must show its glyph");
    }

    #[test]
    fn every_step_has_a_verb_and_label() {
        let keys = Keymap::default();
        let mut ob = walking();
        for step in [
            TutorialStep::Move,
            TutorialStep::Look,
            TutorialStep::Gather,
            TutorialStep::Craft,
            TutorialStep::Build,
        ] {
            ob.step = step;
            let p = ob.prompt(&keys);
            assert!(!p.verb.is_empty() && !p.label.is_empty(), "{step:?} copy missing");
        }
    }
}
