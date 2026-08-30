//! Rebindable keymap (build-pack Step 13 / V1REBRAND P28). Actions map to
//! physical key codes; defaults match the original hardcoded bindings, so
//! behavior is unchanged until the player rebinds in Settings > Controls.
//! Persistence stores key NAMES (stable across winit bumps); unknown names
//! on load fall back to the default binding.

use std::collections::HashMap;

use winit::keyboard::KeyCode;

/// Every rebindable action. Hotbar digits 1-9 stay fixed (industry
/// standard), as does Escape for menus.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Forward,
    Back,
    Left,
    Right,
    Jump,
    Sneak,
    Sprint,
    Fly,
    FlyUp,
    FlyDown,
    Inventory,
    QuestLog,
    TechTree,
    Map,
    Chat,
    Console,
    Screenshot,
    DebugInfo,
    RtCapture,
    GridOverlay,
    Spell1,
    Spell2,
    Spell3,
    Spellbook,
    Symmetry,
    PathsScreen,
}

impl Action {
    pub const ALL: [Action; 26] = [
        Action::Forward,
        Action::Back,
        Action::Left,
        Action::Right,
        Action::Jump,
        Action::Sneak,
        Action::Sprint,
        Action::Fly,
        Action::FlyUp,
        Action::FlyDown,
        Action::Inventory,
        Action::QuestLog,
        Action::TechTree,
        Action::Map,
        Action::Chat,
        Action::Console,
        Action::Screenshot,
        Action::DebugInfo,
        Action::RtCapture,
        Action::GridOverlay,
        Action::Spell1,
        Action::Spell2,
        Action::Spell3,
        Action::Spellbook,
        Action::Symmetry,
        Action::PathsScreen,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Action::Forward => "Walk forward",
            Action::Back => "Walk back",
            Action::Left => "Strafe left",
            Action::Right => "Strafe right",
            Action::Jump => "Jump",
            Action::Sneak => "Sneak (careful walk)",
            Action::Sprint => "Sprint",
            Action::Fly => "Toggle fly",
            Action::FlyUp => "Fly up",
            Action::FlyDown => "Fly down",
            Action::Inventory => "Inventory",
            Action::QuestLog => "Quest log / chronicle",
            Action::TechTree => "Tech tree",
            Action::Map => "World map",
            Action::Chat => "Chat",
            Action::Console => "Console",
            Action::Screenshot => "Screenshot",
            Action::DebugInfo => "Debug overlay",
            Action::RtCapture => "Path-traced capture",
            Action::GridOverlay => "Power-grid overlay",
            Action::Spell1 => "Cast spell slot 1",
            Action::Spell2 => "Cast spell slot 2",
            Action::Spell3 => "Cast spell slot 3",
            Action::Spellbook => "Spellbook",
            Action::Symmetry => "Toggle build symmetry",
            Action::PathsScreen => "Paths & specialization",
        }
    }
}

/// The key names we can persist and load. Deliberately bounded to the
/// codes a player would realistically bind.
fn code_from_name(name: &str) -> Option<KeyCode> {
    use KeyCode::*;
    Some(match name {
        "KeyA" => KeyA, "KeyB" => KeyB, "KeyC" => KeyC, "KeyD" => KeyD, "KeyE" => KeyE,
        "KeyF" => KeyF, "KeyG" => KeyG, "KeyH" => KeyH, "KeyI" => KeyI, "KeyJ" => KeyJ,
        "KeyK" => KeyK, "KeyL" => KeyL, "KeyM" => KeyM, "KeyN" => KeyN, "KeyO" => KeyO,
        "KeyP" => KeyP, "KeyQ" => KeyQ, "KeyR" => KeyR, "KeyS" => KeyS, "KeyT" => KeyT,
        "KeyU" => KeyU, "KeyV" => KeyV, "KeyW" => KeyW, "KeyX" => KeyX, "KeyY" => KeyY,
        "KeyZ" => KeyZ,
        "Digit1" => Digit1, "Digit2" => Digit2, "Digit3" => Digit3, "Digit4" => Digit4,
        "Digit5" => Digit5, "Digit6" => Digit6, "Digit7" => Digit7, "Digit8" => Digit8,
        "Digit9" => Digit9, "Digit0" => Digit0,
        "Space" => Space,
        "ShiftLeft" => ShiftLeft, "ShiftRight" => ShiftRight,
        "ControlLeft" => ControlLeft, "ControlRight" => ControlRight,
        "AltLeft" => AltLeft, "AltRight" => AltRight,
        "Tab" => Tab, "Enter" => Enter, "Backquote" => Backquote, "Slash" => Slash,
        "F1" => F1, "F2" => F2, "F3" => F3, "F4" => F4, "F5" => F5, "F6" => F6,
        "F7" => F7, "F8" => F8, "F9" => F9, "F10" => F10, "F11" => F11, "F12" => F12,
        "ArrowUp" => ArrowUp, "ArrowDown" => ArrowDown,
        "ArrowLeft" => ArrowLeft, "ArrowRight" => ArrowRight,
        _ => return None,
    })
}

#[derive(Clone, Debug)]
pub struct Keymap {
    bindings: HashMap<Action, KeyCode>,
}

impl Default for Keymap {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        let mut set = |a: Action, k: KeyCode| {
            bindings.insert(a, k);
        };
        use KeyCode::*;
        set(Action::Forward, KeyW);
        set(Action::Back, KeyS);
        set(Action::Left, KeyA);
        set(Action::Right, KeyD);
        set(Action::Jump, Space);
        // king-quest controls (Minecraft scheme): SHIFT runs, CTRL crouches
        set(Action::Sneak, ControlLeft);
        set(Action::Sprint, ShiftLeft);
        set(Action::Fly, KeyF);
        set(Action::FlyUp, Space);
        set(Action::FlyDown, ControlLeft);
        set(Action::Inventory, KeyE);
        set(Action::QuestLog, KeyJ);
        set(Action::TechTree, KeyK);
        set(Action::Map, KeyM);
        set(Action::Chat, KeyT);
        set(Action::Console, Backquote);
        set(Action::Screenshot, F2);
        set(Action::DebugInfo, F3);
        set(Action::RtCapture, KeyR);
        set(Action::GridOverlay, KeyG);
        set(Action::Spell1, KeyZ);
        set(Action::Spell2, KeyX);
        set(Action::Spell3, KeyC);
        set(Action::Spellbook, KeyB);
        set(Action::Symmetry, KeyV);
        set(Action::PathsScreen, KeyP);
        Self { bindings }
    }
}

impl Keymap {
    pub fn key(&self, action: Action) -> KeyCode {
        self.bindings.get(&action).copied().unwrap_or(KeyCode::KeyW)
    }

    pub fn rebind(&mut self, action: Action, key: KeyCode) {
        self.bindings.insert(action, key);
    }

    /// First action bound to this key (used by the key press dispatcher).
    pub fn action_for(&self, key: KeyCode) -> Option<Action> {
        // deterministic scan order so a shared key prefers the earlier action
        Action::ALL.iter().copied().find(|a| self.key(*a) == key)
    }

    /// Serialize bindings as (action-index, key-name) pairs for ClientSave.
    pub fn to_pairs(&self) -> Vec<(u8, String)> {
        Action::ALL.iter().enumerate()
            .map(|(i, a)| (i as u8, format!("{:?}", self.key(*a))))
            .collect()
    }

    /// Load from serialized pairs; unknown names or indices keep defaults.
    pub fn from_pairs(pairs: &[(u8, String)]) -> Self {
        let mut km = Self::default();
        for (idx, name) in pairs {
            if let Some(action) = Action::ALL.get(*idx as usize).copied() {
                if let Some(code) = code_from_name(name) {
                    km.rebind(action, code);
                }
            }
        }
        km
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_original_hardcoded_bindings() {
        let km = Keymap::default();
        use KeyCode::*;
        assert_eq!(km.key(Action::Forward), KeyW);
        assert_eq!(km.key(Action::Jump), Space);
        assert_eq!(km.key(Action::Inventory), KeyE);
        assert_eq!(km.key(Action::Map), KeyM);
        assert_eq!(km.action_for(KeyK), Some(Action::TechTree));
        // king-quest controls: SHIFT runs, CTRL crouches (Minecraft scheme)
        assert_eq!(km.key(Action::Sprint), ShiftLeft);
        assert_eq!(km.key(Action::Sneak), ControlLeft);
        assert_eq!(km.key(Action::FlyDown), ControlLeft);
    }

    /// Step 13 done-when: a rebinding survives a save/reload round trip
    /// (serialize to pairs, load back, binding intact), and junk input
    /// falls back to defaults instead of panicking.
    #[test]
    fn rebinding_survives_serialization_roundtrip() {
        let mut km = Keymap::default();
        km.rebind(Action::Jump, KeyCode::KeyN);
        km.rebind(Action::TechTree, KeyCode::F6);
        let pairs = km.to_pairs();
        let loaded = Keymap::from_pairs(&pairs);
        assert_eq!(loaded.key(Action::Jump), KeyCode::KeyN);
        assert_eq!(loaded.key(Action::TechTree), KeyCode::F6);
        // untouched actions keep defaults
        assert_eq!(loaded.key(Action::Forward), KeyCode::KeyW);

        // unknown key name / bogus index: defaults survive
        let junk = Keymap::from_pairs(&[(0u8, "NoSuchKey".into()), (200u8, "KeyP".into())]);
        assert_eq!(junk.key(Action::Forward), KeyCode::KeyW);
    }
}
