//! Furnace smelting: inputs, fuels, and the smelt-state machine.

use crate::survival::ItemStack;

/// Seconds each item takes to smelt.
pub const SMELT_TIME: f32 = 10.0;

/// What an input smelts into.
pub fn smelt_result(input: &str) -> Option<&'static str> {
    match input {
        "raw_iron" => Some("iron_ingot"),
        "raw_copper" => Some("copper_ingot"),
        "raw_tin" => Some("tin_ingot"),
        // nuclear line (P32): the ore drops raw_uranium
        "raw_uranium" => Some("uranium_ingot"),
        "sand" => Some("glass"),
        _ => mod_smelt(input),
    }
}

// --- runtime mod smelting entries ---
fn mod_smelts() -> &'static std::sync::RwLock<Vec<(String, &'static str)>> {
    static SMELTS: std::sync::OnceLock<std::sync::RwLock<Vec<(String, &'static str)>>> = std::sync::OnceLock::new();
    SMELTS.get_or_init(|| std::sync::RwLock::new(Vec::new()))
}

/// Register a mod smelting entry (input id -> output id). Idempotent.
pub fn register_mod_smelt(input: String, output: String) -> bool {
    let mut entries = mod_smelts().write().unwrap();
    if entries.iter().any(|(i, _)| *i == input) {
        return true;
    }
    entries.push((input, Box::leak(output.into_boxed_str())));
    true
}

fn mod_smelt(input: &str) -> Option<&'static str> {
    mod_smelts()
        .read()
        .unwrap()
        .iter()
        .find(|(i, _)| i == input)
        .map(|(_, out)| *out)
}

/// Every known smelting pair (input, output), vanilla + mods. For recipe
/// browsers; keep in sync with `smelt_result` (a test guards the agreement).
pub fn smelt_entries() -> Vec<(String, &'static str)> {
    let mut out: Vec<(String, &'static str)> = vec![
        ("raw_iron".into(), "iron_ingot"),
        ("raw_copper".into(), "copper_ingot"),
        ("raw_tin".into(), "tin_ingot"),
        ("sand".into(), "glass"),
    ];
    out.extend(mod_smelts().read().unwrap().iter().cloned());
    out
}

/// Seconds of burn per fuel item.
pub fn fuel_seconds(fuel: &str) -> f32 {
    match fuel {
        "coal" => 80.0,
        "log" => 15.0,
        "planks" => 15.0,
        "stick" => 5.0,
        _ => 0.0,
    }
}

/// Furnace state for one placed furnace block.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Furnace {
    pub input: Option<ItemStack>,
    pub fuel: Option<ItemStack>,
    pub output: Option<ItemStack>,
    /// Remaining burn seconds in the current fuel item.
    pub burn_left: f32,
    /// Total burn of the current fuel item (for the UI flame).
    pub burn_total: f32,
    /// Progress toward the next output, in seconds.
    pub progress: f32,
}

impl Furnace {
    /// Advance the furnace by dt seconds. Returns true if any slot changed
    /// (so the client can save/persist).
    pub fn tick(&mut self, dt: f32) -> bool {
        let mut changed = false;
        let can_output = |input: &Option<ItemStack>, output: &Option<ItemStack>| -> bool {
            match (input, output) {
                (Some(i), Some(o)) => {
                    smelt_result(&i.item_id) == Some(o.item_id.as_str())
                        && o.count < 64
                }
                (Some(i), None) => smelt_result(&i.item_id).is_some(),
                _ => false,
            }
        };

        let producing = can_output(&self.input, &self.output);
        if self.burn_left <= 0.0 && producing {
            if let Some(fuel) = &mut self.fuel {
                if fuel_seconds(&fuel.item_id) > 0.0 {
                    self.burn_total = fuel_seconds(&fuel.item_id);
                    self.burn_left = self.burn_total;
                    fuel.count -= 1;
                    if fuel.count == 0 {
                        self.fuel = None;
                    }
                    changed = true;
                }
            }
        }

        if self.burn_left > 0.0 {
            self.burn_left = (self.burn_left - dt).max(0.0);
            if producing {
                self.progress += dt;
                if self.progress >= SMELT_TIME {
                    self.progress = 0.0;
                    if let Some(input) = &mut self.input {
                        let out_id = smelt_result(&input.item_id).unwrap().to_string();
                        input.count -= 1;
                        if input.count == 0 {
                            self.input = None;
                        }
                        match &mut self.output {
                            Some(o) => o.count += 1,
                            None => self.output = Some(ItemStack { item_id: out_id, count: 1 }),
                        }
                    }
                    changed = true;
                }
            } else {
                self.progress = 0.0;
            }
        } else {
            self.progress = (self.progress - dt * 0.5).max(0.0);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(id: &str, n: u8) -> Option<ItemStack> {
        Some(ItemStack { item_id: id.to_string(), count: n })
    }

    #[test]
    fn smelts_iron_with_coal() {
        let mut f = Furnace {
            input: stack("raw_iron", 3),
            fuel: stack("coal", 1),
            ..Default::default()
        };
        // 30 seconds at 10s per item = 3 smelts
        for _ in 0..300 {
            f.tick(0.1);
        }
        assert_eq!(f.output, stack("iron_ingot", 3), "should have smelted 3");
        assert!(f.input.is_none(), "all raw iron consumed");
        assert!(f.burn_left > 40.0, "coal had 80s of burn; ~50 should remain, got {}", f.burn_left);
    }

    #[test]
    fn no_fuel_no_smelt() {
        let mut f = Furnace { input: stack("raw_iron", 1), ..Default::default() };
        for _ in 0..100 {
            f.tick(0.1);
        }
        assert!(f.output.is_none());
        assert_eq!(f.progress, 0.0);
    }

    #[test]
    fn wrong_fuel_does_nothing() {
        let mut f = Furnace {
            input: stack("raw_iron", 1),
            fuel: stack("dirt", 64),
            ..Default::default()
        };
        for _ in 0..100 {
            f.tick(0.1);
        }
        assert!(f.output.is_none());
        assert_eq!(f.burn_left, 0.0);
    }

    #[test]
    fn mod_smelting_registers() {
        register_mod_smelt("ember_ores:ember_ore".into(), "ember_ores:ember_ingot".into());
        register_mod_smelt("ember_ores:ember_ore".into(), "ember_ores:ember_ingot".into());
        assert_eq!(smelt_result("ember_ores:ember_ore"), Some("ember_ores:ember_ingot"));
    }

    #[test]
    fn sand_smelts_to_glass() {
        assert_eq!(smelt_result("sand"), Some("glass"));
        assert_eq!(smelt_result("stone"), None);
        assert!(fuel_seconds("planks") > 0.0);
        assert_eq!(fuel_seconds("iron_ingot"), 0.0);
    }

    #[test]
    fn smelt_entries_agree_with_smelt_result() {
        assert!(!smelt_entries().is_empty());
        for (input, output) in smelt_entries() {
            assert_eq!(smelt_result(&input), Some(output), "entry {} disagrees with smelt_result", input);
        }
    }
}
