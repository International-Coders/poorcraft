//! Powered machines: coal generator, electric furnace, crusher, assembler.
//! Power flows from generators to machines within a 4-block field.

use crate::smelting::smelt_result;
use crate::survival::ItemStack;
use serde::{Deserialize, Serialize};

pub const POWER_RANGE: f32 = 4.0;
/// EU per second a generator produces while burning.
pub const GENERATE_RATE: f32 = 20.0;
/// EU per second a running machine draws.
pub const DRAW_RATE: f32 = 10.0;
/// Machine process time in seconds (electric furnace is 2x the furnace).
pub const PROCESS_TIME: f32 = 5.0;
pub const GEN_CAPACITY: f32 = 2000.0;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Generator {
    pub fuel: Option<ItemStack>,
    pub burn_left: f32,
    pub buffer: f32,
}

impl Generator {
    pub fn tick(&mut self, dt: f32) -> bool {
        let mut changed = false;
        if self.burn_left <= 0.0 {
            if let Some(fuel) = &mut self.fuel {
                let secs = crate::smelting::fuel_seconds(&fuel.item_id);
                if secs > 0.0 {
                    self.burn_left = secs;
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
            self.buffer = (self.buffer + GENERATE_RATE * dt).min(GEN_CAPACITY);
            changed = true;
        }
        changed
    }

    /// Pull EU for a nearby machine. Returns how much was actually drawn.
    pub fn draw(&mut self, want: f32) -> f32 {
        let given = want.min(self.buffer);
        self.buffer -= given;
        given
    }
}

/// Crusher: ore in -> 2 dust out (ore doubling).
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Crusher {
    pub input: Option<ItemStack>,
    pub output: Option<ItemStack>,
    pub progress: f32,
}

pub fn crush_result(input: &str) -> Option<(&'static str, u8)> {
    match input {
        "raw_iron" => Some(("raw_iron", 2)), // crushed ore = 2x raw
        "raw_copper" => Some(("raw_copper", 2)),
        "raw_tin" => Some(("raw_tin", 2)),
        "iron_ore" => Some(("raw_iron", 2)),
        _ => None,
    }
}

impl Crusher {
    /// tick with `powered` = EU granted this frame.
    pub fn tick(&mut self, dt: f32, powered: f32) -> bool {
        let mut changed = false;
        let can_crush = match (&self.input, &self.output) {
            (Some(i), Some(o)) => {
                crush_result(&i.item_id).map(|(out, _)| out == o.item_id).unwrap_or(false) && o.count <= 62
            }
            (Some(i), None) => crush_result(&i.item_id).is_some(),
            _ => false,
        };
        if powered > 0.0 && can_crush {
            self.progress += dt;
            changed = true;
            if self.progress >= PROCESS_TIME {
                self.progress = 0.0;
                if let Some(input) = &mut self.input {
                    let (out, n) = crush_result(&input.item_id).unwrap();
                    input.count -= 1;
                    if input.count == 0 {
                        self.input = None;
                    }
                    match &mut self.output {
                        Some(o) => o.count += n,
                        None => self.output = Some(ItemStack { item_id: out.to_string(), count: n }),
                    }
                }
            }
        } else {
            self.progress = (self.progress - dt * 0.5).max(0.0);
        }
        changed
    }
}

/// Assembler: alloy recipes with 2 ingredient slots.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Assembler {
    pub input_a: Option<ItemStack>,
    pub input_b: Option<ItemStack>,
    pub output: Option<ItemStack>,
    pub progress: f32,
}

/// Alloy recipes: (a, a_n, b, b_n, out, out_n).
pub fn alloy_recipes() -> &'static [(&'static str, u8, &'static str, u8, &'static str, u8)] {
    &[
        ("copper_ingot", 3, "tin_ingot", 1, "bronze_ingot", 4),
        ("iron_ingot", 1, "coal", 2, "steel_ingot", 1),
        ("copper_wire", 2, "tin_ingot", 1, "basic_circuit", 1),
        ("copper_wire", 4, "iron_gear", 2, "machine_frame", 1),
    ]
}

impl Assembler {
    pub fn current_recipe(&self) -> Option<(&'static str, u8, &'static str, u8, &'static str, u8)> {
        alloy_recipes().iter().copied().find(|(a, _, b, _, _, _)| {
            let has_a = self.input_a.as_ref().map(|s| s.item_id == *a && s.count > 0).unwrap_or(false);
            let has_b = self.input_b.as_ref().map(|s| s.item_id == *b && s.count > 0).unwrap_or(false);
            has_a && has_b
        })
    }

    pub fn tick(&mut self, dt: f32, powered: f32) -> bool {
        let mut changed = false;
        let recipe = self.current_recipe();
        let out_ok = match (&recipe, &self.output) {
            (Some((_, _, _, _, out, _)), Some(o)) => o.item_id == *out && o.count < 64,
            (Some(_), None) => true,
            _ => false,
        };
        if powered > 0.0 && recipe.is_some() && out_ok {
            self.progress += dt;
            changed = true;
            if self.progress >= PROCESS_TIME {
                self.progress = 0.0;
                if let Some((a, an, b, bn, out, on)) = recipe {
                    if let Some(sa) = &mut self.input_a {
                        sa.count = sa.count.saturating_sub(an);
                        if sa.count == 0 { self.input_a = None; }
                    }
                    if let Some(sb) = &mut self.input_b {
                        sb.count = sb.count.saturating_sub(bn);
                        if sb.count == 0 { self.input_b = None; }
                    }
                    match &mut self.output {
                        Some(o) => o.count += on,
                        None => self.output = Some(ItemStack { item_id: out.to_string(), count: on }),
                    }
                }
            }
        } else {
            self.progress = (self.progress - dt * 0.5).max(0.0);
        }
        changed
    }
}

/// Electric furnace: like the furnace but 2x speed when powered.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ElectricFurnace {
    pub input: Option<ItemStack>,
    pub output: Option<ItemStack>,
    pub progress: f32,
}

impl ElectricFurnace {
    pub fn tick(&mut self, dt: f32, powered: f32) -> bool {
        let mut changed = false;
        let can_smelt = match (&self.input, &self.output) {
            (Some(i), Some(o)) => {
                smelt_result(&i.item_id) == Some(o.item_id.as_str()) && o.count < 64
            }
            (Some(i), None) => smelt_result(&i.item_id).is_some(),
            _ => false,
        };
        if powered > 0.0 && can_smelt {
            let speed = crate::smelting::SMELT_TIME / 2.0;
            self.progress += dt;
            changed = true;
            if self.progress >= speed {
                self.progress = 0.0;
                if let Some(input) = &mut self.input {
                    let out = smelt_result(&input.item_id).unwrap().to_string();
                    input.count -= 1;
                    if input.count == 0 {
                        self.input = None;
                    }
                    match &mut self.output {
                        Some(o) => o.count += 1,
                        None => self.output = Some(ItemStack { item_id: out, count: 1 }),
                    }
                }
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
    fn generator_burns_and_dispenses() {
        let mut g = Generator { fuel: stack("coal", 1), ..Default::default() };
        for _ in 0..200 {
            g.tick(0.1);
        }
        assert!(g.buffer > 100.0, "20s of coal should charge the buffer, got {}", g.buffer);
        let drawn = g.draw(50.0);
        assert_eq!(drawn, 50.0);
        let partial = g.draw(9999.0);
        assert!(partial < 9999.0, "draw limited by the buffer");
    }

    #[test]
    fn crusher_doubles_ores() {
        let mut c = Crusher { input: stack("iron_ore", 3), ..Default::default() };
        for _ in 0..100 {
            c.tick(0.1, DRAW_RATE);
        }
        assert_eq!(c.output, stack("raw_iron", 2), "one crush cycle doubles");
        assert_eq!(c.input, stack("iron_ore", 2));
    }

    #[test]
    fn crusher_without_power_stalls() {
        let mut c = Crusher { input: stack("raw_iron", 1), ..Default::default() };
        for _ in 0..100 {
            c.tick(0.1, 0.0);
        }
        assert!(c.output.is_none());
    }

    #[test]
    fn assembler_makes_bronze() {
        let mut a = Assembler {
            input_a: stack("copper_ingot", 3),
            input_b: stack("tin_ingot", 1),
            ..Default::default()
        };
        assert!(a.current_recipe().is_some());
        for _ in 0..100 {
            a.tick(0.1, DRAW_RATE);
        }
        assert_eq!(a.output, stack("bronze_ingot", 4));
        assert!(a.input_a.is_none());
        assert!(a.input_b.is_none());
    }

    #[test]
    fn electric_furnace_is_fast() {
        let mut f = ElectricFurnace { input: stack("raw_iron", 2), ..Default::default() };
        // normal furnace takes 10s; electric takes 5s
        for _ in 0..60 {
            f.tick(0.1, DRAW_RATE);
        }
        assert_eq!(f.output, stack("iron_ingot", 1), "5s powered should smelt one");
    }
}
