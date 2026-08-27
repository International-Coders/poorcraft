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

/// Every crushable input with its result, for recipe browsers. Only item
/// ids (ore blocks drop raw_* items, so they never sit in the input slot).
pub fn crush_entries() -> &'static [(&'static str, &'static str, u8)] {
    &[("raw_iron", "raw_iron", 2), ("raw_copper", "raw_copper", 2), ("raw_tin", "raw_tin", 2)]
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
                if let Some((_a, an, _b, bn, out, on)) = recipe {
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

// ------------------------------------------------------------------
// Water Age (V1REBRAND doc 04 / build-pack Step 23)

/// EU per second a water wheel produces while sitting in water — enough
/// for one early machine plus a trickle, deliberately below the coal
/// generator's 20 (the wheel is free but river-gated).
pub const WHEEL_RATE: f32 = 12.0;
pub const WHEEL_CAPACITY: f32 = 600.0;
/// Battery storage: covers intermittent sources and blackout gaps.
pub const BATTERY_CAP: f32 = 4000.0;

/// A wheel placed against water. `has_water` is decided by the caller
/// (adjacent water blocks); the wheel itself has no fuel loop.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct WaterWheel {
    pub buffer: f32,
}

impl WaterWheel {
    pub fn tick(&mut self, dt: f32, has_water: bool) {
        if has_water {
            self.buffer = (self.buffer + WHEEL_RATE * dt).min(WHEEL_CAPACITY);
        }
    }

    pub fn draw(&mut self, want: f32) -> f32 {
        let take = want.min(self.buffer);
        self.buffer -= take;
        take
    }
}

/// Rechargeable cell: charges from producer surplus, discharges only when
/// producers in range cannot cover the machines (blackout prevention).
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct BatteryCell {
    pub charge: f32,
}

impl BatteryCell {
    pub fn charge(&mut self, offer: f32) -> f32 {
        let take = offer.min(BATTERY_CAP - self.charge).max(0.0);
        self.charge += take;
        take
    }

    pub fn draw(&mut self, want: f32) -> f32 {
        let take = want.min(self.charge);
        self.charge -= take;
        take
    }
}

/// One power node in the field (position + source), for distribute_power.
#[derive(Clone, Debug)]
pub enum PowerSource {
    Generator(Generator),
    Wheel(WaterWheel),
    Battery(BatteryCell),
}

impl PowerSource {
    fn is_producer(&self) -> bool {
        !matches!(self, PowerSource::Battery(_))
    }

    fn draw(&mut self, want: f32) -> f32 {
        match self {
            PowerSource::Generator(g) => g.draw(want),
            PowerSource::Wheel(w) => w.draw(want),
            PowerSource::Battery(b) => b.draw(want),
        }
    }

    /// How much buffered energy the node holds (UI/debug).
    pub fn stored(&self) -> f32 {
        match self {
            PowerSource::Generator(g) => g.buffer,
            PowerSource::Wheel(w) => w.buffer,
            PowerSource::Battery(b) => b.charge,
        }
    }
}

/// The proximity-field power step, pure so tests (and the vistest scene)
/// can run it headless — the client feeds real block entities through
/// this every tick. Phase 1: machines draw from PRODUCERS in range; phase
/// 2: still-starved machines draw from batteries in range; phase 3:
/// producer surplus charges batteries in range. Returns EU granted per
/// machine (same order as `machines`).
pub fn distribute_power(
    sources: &mut [((i32, i32, i32), PowerSource)],
    machines: &[(i32, i32, i32)],
    need: f32,
) -> Vec<f32> {
    let in_range = |s: (i32, i32, i32), m: (i32, i32, i32)| {
        let d = ((s.0 - m.0).pow(2) + (s.1 - m.1).pow(2) + (s.2 - m.2).pow(2)) as f32;
        d.sqrt() <= POWER_RANGE
    };
    let mut granted = vec![0.0f32; machines.len()];
    // phase 1: producers
    for (mi, &mpos) in machines.iter().enumerate() {
        let deficit = need - granted[mi];
        if deficit <= 0.0 {
            continue;
        }
        for (spos, src) in sources.iter_mut() {
            if *spos == mpos || !in_range(*spos, mpos) {
                continue;
            }
            if !src.is_producer() {
                continue;
            }
            let got = src.draw(deficit);
            granted[mi] += got;
            if granted[mi] >= need {
                break;
            }
        }
    }
    // phase 2: batteries cover the remainder
    for (mi, &mpos) in machines.iter().enumerate() {
        let deficit = need - granted[mi];
        if deficit <= 0.0 {
            continue;
        }
        for (spos, src) in sources.iter_mut() {
            if !in_range(*spos, mpos) || src.is_producer() {
                continue;
            }
            let got = src.draw(deficit);
            granted[mi] += got;
            if granted[mi] >= need {
                break;
            }
        }
    }
    // phase 3: producer surplus charges batteries in range (indexed so
    // both ends can mutate)
    for i in 0..sources.len() {
        if !sources[i].1.is_producer() {
            continue;
        }
        let mut offer = sources[i].1.stored();
        if offer <= 0.0 {
            continue;
        }
        for j in 0..sources.len() {
            if i == j || sources[j].1.is_producer() {
                continue;
            }
            if !in_range(sources[i].0, sources[j].0) {
                continue;
            }
            if offer <= 0.0 {
                break;
            }
            let took = if let PowerSource::Battery(b) = &mut sources[j].1 {
                b.charge(offer)
            } else {
                0.0
            };
            offer -= took;
            match &mut sources[i].1 {
                PowerSource::Generator(g) => g.buffer -= took,
                PowerSource::Wheel(w) => w.buffer -= took,
                PowerSource::Battery(_) => {}
            }
        }
    }
    granted
}

#[cfg(test)]
mod water_age_tests {
    use super::*;

    fn wheel_charged(v: f32) -> ((i32, i32, i32), PowerSource) {
        ((0, 0, 0), PowerSource::Wheel(WaterWheel { buffer: v }))
    }

    #[test]
    fn wheel_trickles_only_in_water() {
        let mut w = WaterWheel::default();
        w.tick(1.0, false);
        assert_eq!(w.buffer, 0.0, "dry wheel produces nothing");
        w.tick(1.0, true);
        assert!((w.buffer - WHEEL_RATE).abs() < 1e-4);
        for _ in 0..1000 {
            w.tick(1.0, true);
        }
        assert!(w.buffer <= WHEEL_CAPACITY + 1e-3, "wheel buffer is capped");
    }

    #[test]
    fn wheel_powers_a_machine_and_battery_covers_gaps() {
        let machine = (2, 0, 0);
        // one wheel, fully spun up: covers the machine's draw
        let mut sources = vec![wheel_charged(WHEEL_CAPACITY)];
        let granted = distribute_power(&mut sources, &[machine], DRAW_RATE);
        assert!(granted[0] >= DRAW_RATE, "wheel covers one machine, got {}", granted[0]);

        // no producers, charged battery: still covers (blackout prevention)
        let mut sources = vec![((0, 0, 0), PowerSource::Battery(BatteryCell { charge: BATTERY_CAP }))];
        let granted = distribute_power(&mut sources, &[machine], DRAW_RATE);
        assert!(granted[0] >= DRAW_RATE, "battery bridges the gap");

        // empty battery: machine starves
        let mut sources = vec![((0, 0, 0), PowerSource::Battery(BatteryCell { charge: 0.0 }))];
        let granted = distribute_power(&mut sources, &[machine], DRAW_RATE);
        assert_eq!(granted[0], 0.0);
    }

    #[test]
    fn surplus_charges_batteries_in_range() {
        let mut sources = vec![
            ((0, 0, 0), PowerSource::Wheel(WaterWheel { buffer: WHEEL_CAPACITY })),
            ((1, 0, 0), PowerSource::Battery(BatteryCell { charge: 0.0 })),
        ];
        let machine = (0, 0, 2);
        distribute_power(&mut sources, &[machine], DRAW_RATE);
        let (_, wheel) = &sources[0];
        let stored = wheel.stored();
        let batt = match &sources[1].1 {
            PowerSource::Battery(b) => b.charge,
            _ => unreachable!(),
        };
        assert!(batt > 0.0, "surplus flows into the battery");
        assert!(stored < WHEEL_CAPACITY, "the wheel gave up buffer");
        // out-of-range battery gets nothing
        let mut far = vec![
            ((0, 0, 0), PowerSource::Wheel(WaterWheel { buffer: WHEEL_CAPACITY })),
            ((50, 0, 0), PowerSource::Battery(BatteryCell { charge: 0.0 })),
        ];
        distribute_power(&mut far, &[], 0.0);
        match &far[1].1 {
            PowerSource::Battery(b) => assert_eq!(b.charge, 0.0, "field radius respected"),
            _ => unreachable!(),
        }
    }
}
