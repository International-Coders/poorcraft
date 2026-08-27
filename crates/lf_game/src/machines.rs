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
        // nuclear line (P32): enriched rods assembled from smelted ore
        ("uranium_ingot", 2, "iron_ingot", 1, "fuel_rod", 1),
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
    Engine(SteamEngine),
    Combustion(CombustionGenerator),
    Reactor(Reactor),
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
            PowerSource::Engine(e) => e.draw(want),
            PowerSource::Combustion(c) => c.draw(want),
            PowerSource::Reactor(r) => r.draw(want),
        }
    }

    /// How much buffered energy the node holds (UI/debug).
    pub fn stored(&self) -> f32 {
        match self {
            PowerSource::Generator(g) => g.buffer,
            PowerSource::Wheel(w) => w.buffer,
            PowerSource::Battery(b) => b.charge,
            PowerSource::Engine(e) => e.buffer,
            PowerSource::Combustion(c) => c.buffer,
            PowerSource::Reactor(r) => r.buffer,
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
                PowerSource::Engine(e) => e.buffer -= took,
                PowerSource::Combustion(c) => c.buffer -= took,
                PowerSource::Reactor(r) => r.buffer -= took,
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

// ------------------------------------------------------------------
// Steam Age (V1REBRAND doc 04 / build-pack Step 24)

/// mB of water a pipe segment holds.
pub const PIPE_CAP: u16 = 1000;
/// mB of water the boiler consumes per second while burning.
pub const BOILER_WATER_RATE: u16 = 80;
/// Steam (arbitrary units) produced per second while burning.
pub const BOILER_STEAM_RATE: f32 = 40.0;
pub const BOILER_STEAM_CAP: f32 = 400.0;
/// EU/s the steam engine produces while fed — between the wheel's 12 and
/// the coal generator's 20 (doc 04: "you've committed to a boiler room").
pub const STEAM_ENGINE_RATE: f32 = 16.0;
/// Steam the engine consumes per second.
pub const STEAM_ENGINE_INTAKE: f32 = 20.0;

/// One pipe segment: carries water AND crude on separate channels (P31
/// pipes v2 — fluid typing; the channels never mix, and an empty channel
/// has no identity so adjacent pipes connect per fluid). Equal-share, no
/// pressure sim — DECISIONS entry. Ticked by the client per segment.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Pipe {
    pub water: u16,
    #[serde(default)]
    pub crude: u16,
}

impl Pipe {
    /// Equalize both channels with one neighbor (call per adjacent pair,
    /// one direction per tick to avoid double-transfer).
    pub fn equalize_with(&mut self, neighbor: &mut Pipe) {
        let total_w = self.water as u32 + neighbor.water as u32;
        let rem_w = (total_w % 2) as u16;
        self.water = (total_w / 2) as u16 + rem_w;
        neighbor.water = (total_w / 2) as u16;
        let total_c = self.crude as u32 + neighbor.crude as u32;
        let rem_c = (total_c % 2) as u16;
        self.crude = (total_c / 2) as u16 + rem_c;
        neighbor.crude = (total_c / 2) as u16;
    }

    /// Pull up to `want` mB of one fluid (a boiler or refinery drinking).
    pub fn draw(&mut self, kind: FluidKind, want: u16) -> u16 {
        let channel = match kind {
            FluidKind::Water => &mut self.water,
            FluidKind::Crude => &mut self.crude,
        };
        let take = want.min(*channel);
        *channel -= take;
        take
    }

    pub fn fill(&mut self, kind: FluidKind, offer: u16) -> u16 {
        let channel = match kind {
            FluidKind::Water => &mut self.water,
            FluidKind::Crude => &mut self.crude,
        };
        let take = offer.min(PIPE_CAP - *channel);
        *channel += take;
        take
    }

    pub fn amount(&self, kind: FluidKind) -> u16 {
        match kind {
            FluidKind::Water => self.water,
            FluidKind::Crude => self.crude,
        }
    }
}

/// What a pipe or tank carries. Water feeds boilers; crude feeds refineries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FluidKind {
    #[default]
    Water,
    Crude,
}

impl FluidKind {
    pub fn name(self) -> &'static str {
        match self {
            FluidKind::Water => "water",
            FluidKind::Crude => "crude oil",
        }
    }
}

/// Boiler: burns fuel (existing fuel_seconds table) + water into steam.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Boiler {
    pub fuel: Option<ItemStack>,
    pub burn_left: f32,
    /// Water tank (mB). Filled by the client tick from adjacent sources
    /// and pipes.
    pub water: u16,
    pub steam: f32,
}

impl Boiler {
    /// One tick. `water_in` is what the environment (pipes/sources) fed
    /// this tick, in mB. Returns true while burning (drives particles).
    pub fn tick(&mut self, dt: f32, water_in: u16) -> bool {
        self.water = (self.water + water_in).min(40_000);
        let has_fuel = self.burn_left > 0.0
            || self.fuel.as_ref().map(|f| crate::smelting::fuel_seconds(&f.item_id) > 0.0).unwrap_or(false);
        if !has_fuel || self.water < 1 {
            // idle: steam dissipates slowly
            self.steam = (self.steam - dt * 2.0).max(0.0);
            return false;
        }
        if self.burn_left <= 0.0 {
            // light the next fuel item
            if let Some(f) = self.fuel.take() {
                let secs = crate::smelting::fuel_seconds(&f.item_id);
                if secs > 0.0 {
                    self.burn_left = secs;
                    if f.count > 1 {
                        self.fuel = Some(ItemStack { count: f.count - 1, ..f });
                    }
                } else {
                    self.fuel = Some(f);
                }
            }
        }
        if self.burn_left <= 0.0 {
            return false;
        }
        self.burn_left -= dt;
        let want_water = (BOILER_WATER_RATE as f32 * dt).round() as u16;
        if self.water < want_water {
            return false;
        }
        self.water -= want_water;
        self.steam = (self.steam + BOILER_STEAM_RATE * dt).min(BOILER_STEAM_CAP);
        true
    }

    /// The engine drinks steam directly from an adjacent boiler.
    pub fn draw_steam(&mut self, want: f32) -> f32 {
        let take = want.min(self.steam);
        self.steam -= take;
        take
    }
}

/// Steam engine: consumes steam (from an adjacent boiler, piped or not)
/// and buffers electrical output like the other sources.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SteamEngine {
    pub buffer: f32,
    pub steam_avail: f32, // steam pulled from the boiler this tick, per client wiring
}

impl SteamEngine {
    pub fn tick(&mut self, dt: f32, steam_in: f32) {
        self.steam_avail = steam_in;
        if steam_in > 0.0 {
            let ratio = (steam_in / (STEAM_ENGINE_INTAKE * dt)).min(1.0);
            self.buffer = (self.buffer + STEAM_ENGINE_RATE * dt * ratio).min(600.0);
        }
    }

    pub fn draw(&mut self, want: f32) -> f32 {
        let take = want.min(self.buffer);
        self.buffer -= take;
        take
    }
}

#[cfg(test)]
mod steam_age_tests {
    use super::*;

    fn pipe(w: u16) -> Pipe {
        Pipe { water: w, ..Default::default() }
    }

    #[test]
    fn pipes_equalize_water() {
        let mut a = pipe(1000);
        let mut b = pipe(0);
        a.equalize_with(&mut b);
        assert_eq!(a.water, 500);
        assert_eq!(b.water, 500);
        // total is conserved (odd amounts keep the remainder in `a`)
        let mut c = pipe(3);
        let mut d = pipe(0);
        c.equalize_with(&mut d);
        assert_eq!(c.water + d.water, 3);
        // fill/draw respect the cap
        let mut p = pipe(900);
        assert_eq!(p.fill(FluidKind::Water, 500), 100);
        assert_eq!(p.water, PIPE_CAP);
        assert_eq!(p.draw(FluidKind::Water, 400), 400);
    }

    #[test]
    fn boiler_burns_fuel_and_water_into_steam() {
        let mut b = Boiler {
            fuel: Some(ItemStack { item_id: "coal".into(), count: 2 }),
            burn_left: 0.0,
            water: 10_000,
            steam: 0.0,
        };
        assert!(b.tick(1.0, 0), "fueled + water = burning");
        assert!((b.steam - BOILER_STEAM_RATE).abs() < 1e-3);
        assert_eq!(b.water, 10_000 - BOILER_WATER_RATE);
        // no water: no steam
        let mut dry = Boiler { fuel: b.fuel.clone(), burn_left: 10.0, water: 0, steam: 0.0 };
        assert!(!dry.tick(1.0, 0));
        // no fuel: idle, steam dissipates
        let mut cold = Boiler { fuel: None, burn_left: 0.0, water: 100, steam: 50.0 };
        assert!(!cold.tick(1.0, 0));
        assert!(cold.steam < 50.0);
    }

    #[test]
    fn engine_turns_steam_into_power_between_wheel_and_coal() {
        let mut e = SteamEngine::default();
        e.tick(1.0, STEAM_ENGINE_INTAKE);
        assert!((e.buffer - STEAM_ENGINE_RATE).abs() < 1e-3, "full intake = full rate");
        // starved steam scales output down
        let mut half = SteamEngine::default();
        half.tick(1.0, STEAM_ENGINE_INTAKE / 2.0);
        assert!(half.buffer < e.buffer && half.buffer > 0.0);
        // tiering: wheel < engine < coal generator
        assert!(WHEEL_RATE < STEAM_ENGINE_RATE && STEAM_ENGINE_RATE < GENERATE_RATE);
    }

    /// The full chain: fueled boiler + engine drives a machine through the
    /// same distribute_power the client uses.
    #[test]
    fn boiler_engine_chain_powers_a_machine() {
        let mut boiler = Boiler {
            fuel: Some(ItemStack { item_id: "coal".into(), count: 5 }),
            burn_left: 0.0,
            water: 10_000,
            steam: 0.0,
        };
        let mut engine = SteamEngine::default();
        let machine = (2, 0, 0);
        let dt = 1.0 / 20.0;
        for _ in 0..600 {
            let burning = boiler.tick(dt, 0);
            let steam_in = if burning { boiler.draw_steam(STEAM_ENGINE_INTAKE * dt * 2.0) } else { 0.0 };
            engine.tick(dt, steam_in);
        }
        let mut sources = vec![((0, 0, 0), PowerSource::Engine(engine))];
        let granted = distribute_power(&mut sources, &[machine], DRAW_RATE * dt);
        assert!(granted[0] >= DRAW_RATE * dt * 0.99, "chain covers a machine, got {}", granted[0]);
    }
}

// ------------------------------------------------------------------
// Oil Age (V1REBRAND doc 04 / build-pack Step 25)

/// mB of crude a powered pumpjack lifts per second from an adjacent pool.
pub const PUMP_RATE: u16 = 120;
/// Crude mB consumed per refined-fuel batch.
pub const REFINERY_BATCH: u16 = 240;
/// Seconds a powered refinery works per batch.
pub const REFINERY_TIME: f32 = 6.0;
/// mB an oil bucket pours into a refinery.
pub const OIL_BUCKET_MB: u16 = 1000;
/// EU/s the combustion generator makes from refined fuel — above the coal
/// generator's 20, deliberately below the nuclear reactor (P32 lands on
/// top; doc 04: "top-below-nuclear output").
pub const COMBUSTION_RATE: f32 = 26.0;
/// Seconds one refined fuel unit burns.
pub const COMBUSTION_FUEL_SECONDS: f32 = 45.0;
pub const COMBUSTION_CAP: f32 = 3000.0;

/// Pumpjack: a powered consumer that lifts crude from adjacent oil sources
/// into pipes. The client decides adjacency and where the crude goes; the
/// machine itself only meters the rate.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PumpJack {
    /// Lifetime crude lifted in mB (debug/chronicle fodder).
    pub lifetime_mb: u64,
}

impl PumpJack {
    pub fn tick(&mut self, dt: f32, powered: f32, oil_adjacent: bool) -> u16 {
        if powered <= 0.0 || !oil_adjacent {
            return 0;
        }
        let mb = (PUMP_RATE as f32 * dt).round() as u16;
        self.lifetime_mb += mb as u64;
        mb
    }
}

/// Refinery: crude (piped or bucketed) + power -> refined fuel + tar.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Refinery {
    pub crude: u16,
    pub fuel_out: Option<ItemStack>,
    pub tar_out: Option<ItemStack>,
    pub progress: f32,
}

impl Refinery {
    /// One tick. `crude_in` is what adjacent pipes fed this tick. Returns
    /// true while actively refining (drives particles/UI).
    pub fn tick(&mut self, dt: f32, powered: f32, crude_in: u16) -> bool {
        self.crude = (self.crude + crude_in).min(20_000);
        let room = self.fuel_out.as_ref().map(|s| s.count < 64).unwrap_or(true)
            && self.tar_out.as_ref().map(|s| s.count < 64).unwrap_or(true);
        if powered <= 0.0 || self.crude < REFINERY_BATCH || !room {
            self.progress = (self.progress - dt * 0.5).max(0.0);
            return false;
        }
        self.progress += dt;
        if self.progress >= REFINERY_TIME {
            self.progress = 0.0;
            self.crude -= REFINERY_BATCH;
            match &mut self.fuel_out {
                Some(s) => s.count += 1,
                None => self.fuel_out = Some(ItemStack { item_id: "refined_fuel".into(), count: 1 }),
            }
            match &mut self.tar_out {
                Some(s) => s.count += 1,
                None => self.tar_out = Some(ItemStack { item_id: "tar".into(), count: 1 }),
            }
        }
        true
    }

    /// Pour an oil bucket in (the UI/bucket hook calls this).
    pub fn pour_bucket(&mut self) -> bool {
        if self.crude + OIL_BUCKET_MB > 20_000 {
            return false;
        }
        self.crude += OIL_BUCKET_MB;
        true
    }
}

/// Combustion generator: burns refined fuel into EU at the top-below-
/// nuclear tier. Only refined_fuel lights (crude is for the refinery).
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CombustionGenerator {
    pub fuel: Option<ItemStack>,
    pub burn_left: f32,
    pub buffer: f32,
}

impl CombustionGenerator {
    pub fn tick(&mut self, dt: f32) -> bool {
        let mut burning = false;
        if self.burn_left <= 0.0 {
            let lights = self.fuel.as_ref().map(|f| f.item_id == "refined_fuel").unwrap_or(false);
            if lights {
                self.burn_left = COMBUSTION_FUEL_SECONDS;
                let fuel = self.fuel.as_mut().unwrap();
                fuel.count -= 1;
                if fuel.count == 0 {
                    self.fuel = None;
                }
            }
        }
        if self.burn_left > 0.0 {
            self.burn_left = (self.burn_left - dt).max(0.0);
            self.buffer = (self.buffer + COMBUSTION_RATE * dt).min(COMBUSTION_CAP);
            burning = true;
        }
        burning
    }

    pub fn draw(&mut self, want: f32) -> f32 {
        let given = want.min(self.buffer);
        self.buffer -= given;
        given
    }
}

#[cfg(test)]
mod oil_age_tests {
    use super::*;

    #[test]
    fn typed_pipes_never_mix_fluids() {
        let mut a = Pipe { water: 1000, crude: 0 };
        let mut b = Pipe { water: 0, crude: 800 };
        a.equalize_with(&mut b);
        assert_eq!(a.water, 500);
        assert_eq!(b.water, 500);
        assert_eq!(a.crude, 400);
        assert_eq!(b.crude, 400);
        // fill/draw are channel-aware
        let mut p = Pipe::default();
        assert_eq!(p.fill(FluidKind::Crude, 700), 700);
        assert_eq!(p.amount(FluidKind::Water), 0, "crude never becomes water");
        assert_eq!(p.draw(FluidKind::Water, 100), 0);
        assert_eq!(p.draw(FluidKind::Crude, 100), 100);
    }

    #[test]
    fn pump_needs_both_power_and_oil() {
        let mut p = PumpJack::default();
        assert_eq!(p.tick(1.0, 0.0, true), 0, "unpowered pump lifts nothing");
        assert_eq!(p.tick(1.0, 1.0, false), 0, "dry pump lifts nothing");
        let mb = p.tick(1.0, 1.0, true);
        assert_eq!(mb, PUMP_RATE);
        assert_eq!(p.lifetime_mb, PUMP_RATE as u64);
    }

    #[test]
    fn refinery_mass_balance_is_exact() {
        let mut r = Refinery { crude: REFINERY_BATCH * 3, ..Default::default() };
        let dt = 1.0f32 / 20.0;
        let mut batches = 0usize;
        for _ in 0..(REFINERY_TIME * 20.0 * 4.0) as usize {
            if r.tick(dt, DRAW_RATE, 0) {
                batches += 1;
            }
        }
        assert_eq!(r.fuel_out.as_ref().map(|s| s.count), Some(3), "3 batches of fuel");
        assert_eq!(r.tar_out.as_ref().map(|s| s.count), Some(3), "byproduct tar per batch");
        assert_eq!(r.crude, 0, "every mB accounted for");
        assert!(batches >= 3 * (REFINERY_TIME * 20.0 - 1.0) as usize);
        // unpowered or dry: stalls and decays progress
        let mut s = Refinery { crude: REFINERY_BATCH, ..Default::default() };
        s.tick(1.0, 0.0, 0);
        assert_eq!(s.fuel_out, None);
    }

    #[test]
    fn combustion_burns_refined_fuel_only() {
        let mut c = CombustionGenerator {
            fuel: Some(ItemStack { item_id: "refined_fuel".into(), count: 3 }),
            ..Default::default()
        };
        for _ in 0..50 {
            c.tick(1.0);
        }
        assert!(c.buffer > 500.0, "45s at 26 EU/s charges well, got {}", c.buffer);
        assert_eq!(c.fuel.as_ref().map(|f| f.count), Some(1),
            "50s burns one full unit and lights the next");
        // crude/coal do not light it
        let mut bad = CombustionGenerator {
            fuel: Some(ItemStack { item_id: "coal".into(), count: 5 }),
            ..Default::default()
        };
        bad.tick(10.0);
        assert_eq!(bad.buffer, 0.0);
        // tiering: steam < coal < combustion < (nuclear, P32)
        assert!(STEAM_ENGINE_RATE < GENERATE_RATE && GENERATE_RATE < COMBUSTION_RATE
            && COMBUSTION_RATE < 40.0);
    }

    /// The full P31 chain headless: pump -> pipes -> refinery -> fuel ->
    /// combustion generator -> a machine via the real distribute_power.
    #[test]
    fn oil_chain_powers_a_machine() {
        let dt = 1.0f32 / 20.0;
        let mut pump = PumpJack::default();
        let mut pipes = vec![Pipe::default(); 4];
        let mut refinery = Refinery::default();
        let mut gen = CombustionGenerator::default();
        let mut fuel_pool = 0u8;
        for _ in 0..4000 {
            // pump feeds the first pipe (adjacent oil assumed)
            let mut lifted = pump.tick(dt, DRAW_RATE * dt, true);
            while lifted > 0 {
                let took = pipes[0].fill(FluidKind::Crude, lifted.min(60));
                if took == 0 { break; }
                lifted -= took;
            }
            for i in 0..pipes.len() - 1 {
                let (mut x, mut y) = (pipes[i].clone(), pipes[i + 1].clone());
                x.equalize_with(&mut y);
                pipes[i] = x;
                pipes[i + 1] = y;
            }
            // refinery drinks from the last pipe
            let mut crude_in = pipes[3].draw(FluidKind::Crude, 40);
            crude_in += pipes[2].draw(FluidKind::Crude, 20);
            refinery.tick(dt, DRAW_RATE * dt, crude_in);
            // haul finished fuel to the generator
            if let Some(out) = refinery.fuel_out.take() {
                fuel_pool += out.count;
                if fuel_pool > 0 && gen.fuel.is_none() {
                    gen.fuel = Some(ItemStack { item_id: "refined_fuel".into(), count: fuel_pool });
                    fuel_pool = 0;
                }
            }
            gen.tick(dt);
        }
        assert_eq!(pump.lifetime_mb > 0, true);
        let machine = (2, 0, 0);
        let mut sources = vec![((0, 0, 0), PowerSource::Combustion(gen))];
        let granted = distribute_power(&mut sources, &[machine], DRAW_RATE * dt);
        assert!(granted[0] >= DRAW_RATE * dt * 0.99,
            "the oil chain covers a machine, got {}", granted[0]);
    }
}

// ------------------------------------------------------------------
// Nuclear tier (P32) — the ceiling (DECISIONS Pillar 5)

/// EU/s the reactor produces while burning a rod: above everything
/// (wheel 12 < steam 16 < coal 20 < combustion 26 < REACTOR 32).
pub const REACTOR_RATE: f32 = 32.0;
/// Seconds one fuel rod lasts.
pub const ROD_SECONDS: f32 = 120.0;
/// Heat units gained per second while fissioning.
pub const HEAT_UP_RATE: f32 = 4.0;
/// Heat lost per second while fully cooled — MUST exceed HEAT_UP_RATE so
/// a properly-cooled running core reaches equilibrium, not SCRAM.
pub const HEAT_COOL_RATE: f32 = 5.0;
/// Passive heat loss per second (even uncooled).
pub const HEAT_PASSIVE_LOSS: f32 = 0.5;
/// Residual heat per second after SCRAM while rods remain loaded —
/// the reason you cannot walk away from a scrammed core.
pub const HEAT_RESIDUAL: f32 = 0.8;
/// mB of cooling water per second the core demands.
pub const REACTOR_COOLANT_RATE: u16 = 60;
/// Auto-SCRAM threshold.
pub const SCRAM_AT: f32 = 80.0;
/// Manual un-SCRAM is allowed below this.
pub const UNSCRAM_BELOW: f32 = 60.0;
/// Meltdown threshold — never silently survivable.
pub const MELTDOWN_AT: f32 = 100.0;
pub const REACTOR_CAP: f32 = 6000.0;

/// What one reactor tick reports back to the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReactorEvent {
    Idle,
    Running,
    /// Automatic safety drop: fission halted, decay heat continues.
    Scrammed,
    /// The end: the caller must destroy the surroundings and place
    /// radiation residue.
    Meltdown,
}

/// Fission reactor: fuel rods + cooling water -> the highest EU output in
/// the game, with a heat curve that punishes neglect. Never silently
/// safe: a scrammed core still cooks itself without coolant.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Reactor {
    pub fuel: Option<ItemStack>,
    pub burn_left: f32,
    pub heat: f32,
    pub scram: bool,
    pub coolant: u16,
    pub buffer: f32,
    /// Lifetime mB of coolant burned (debug/chronicle).
    pub lifetime_coolant: u64,
}

impl Reactor {
    /// One tick. `coolant_in` = mB the environment (pipes/adjacent water)
    /// delivered this tick. Returns the event for this tick.
    pub fn tick(&mut self, dt: f32, coolant_in: u16) -> ReactorEvent {
        self.coolant = (self.coolant + coolant_in).min(20_000);
        // thermal balance first: cooling beats heating when supplied
        let demand = (REACTOR_COOLANT_RATE as f32 * dt).round() as u16;
        let cooled = self.coolant.saturating_sub(demand.min(self.coolant)) == self.coolant.saturating_sub(demand)
            && self.coolant >= demand;
        if self.coolant >= demand {
            self.coolant -= demand;
            self.lifetime_coolant += demand as u64;
            self.heat = (self.heat - HEAT_COOL_RATE * dt).max(0.0);
        } else {
            self.heat = (self.heat - HEAT_PASSIVE_LOSS * dt).max(0.0);
        }
        // fission
        let mut event = ReactorEvent::Idle;
        if !self.scram {
            if self.burn_left <= 0.0 {
                if let Some(f) = &mut self.fuel {
                    if f.item_id == "fuel_rod" {
                        self.burn_left = ROD_SECONDS;
                        f.count -= 1;
                        if f.count == 0 {
                            self.fuel = None;
                        }
                    }
                }
            }
            if self.burn_left > 0.0 {
                self.burn_left = (self.burn_left - dt).max(0.0);
                self.buffer = (self.buffer + REACTOR_RATE * dt).min(REACTOR_CAP);
                self.heat += HEAT_UP_RATE * dt;
                event = ReactorEvent::Running;
            }
        } else if self.fuel.is_some() || self.burn_left > 0.0 {
            // decay heat keeps rising even with fission halted
            self.heat += HEAT_RESIDUAL * dt;
            event = ReactorEvent::Scrammed;
        }
        // thresholds
        if self.heat >= MELTDOWN_AT {
            self.heat = MELTDOWN_AT;
            self.burn_left = 0.0;
            self.fuel = None;
            self.buffer = 0.0;
            return ReactorEvent::Meltdown;
        }
        if self.heat >= SCRAM_AT {
            self.scram = true;
            if event == ReactorEvent::Running {
                event = ReactorEvent::Scrammed;
            }
        }
        event
    }

    /// Manual SCRAM (the big red button) and its cautious reverse.
    pub fn scram(&mut self) {
        self.scram = true;
    }

    pub fn try_unscram(&mut self) -> bool {
        if self.heat < UNSCRAM_BELOW {
            self.scram = false;
            return true;
        }
        false
    }

    pub fn draw(&mut self, want: f32) -> f32 {
        let given = want.min(self.buffer);
        self.buffer -= given;
        given
    }
}

#[cfg(test)]
mod nuclear_tests {
    use super::*;

    #[test]
    fn reactor_output_is_the_ceiling() {
        assert!(WHEEL_RATE < STEAM_ENGINE_RATE);
        assert!(STEAM_ENGINE_RATE < GENERATE_RATE);
        assert!(GENERATE_RATE < COMBUSTION_RATE);
        assert!(COMBUSTION_RATE < REACTOR_RATE, "doc 04: nuclear sits on top");
    }

    #[test]
    fn cooled_reactor_runs_cold() {
        let mut r = Reactor {
            fuel: Some(ItemStack { item_id: "fuel_rod".into(), count: 4 }),
            ..Default::default()
        };
        // feed exactly the coolant demand (60 mB/s) for 60s
        let dt = 1.0 / 20.0;
        for _ in 0..1200 {
            let ev = r.tick(dt, (REACTOR_COOLANT_RATE as f32 * dt).ceil() as u16);
            assert_eq!(ev, ReactorEvent::Running);
        }
        assert!(r.heat < 30.0, "full cooling holds the core cold, heat={}", r.heat);
        assert!(r.buffer > REACTOR_RATE * 55.0, "1200 ticks at 32 EU/s charge the buffer");
    }

    #[test]
    fn uncooled_reactor_scrams_then_melts_down() {
        let mut r = Reactor {
            fuel: Some(ItemStack { item_id: "fuel_rod".into(), count: 9 }),
            ..Default::default()
        };
        let dt = 1.0 / 20.0;
        let mut saw_scram = false;
        let mut melted = false;
        for _ in 0..4000 {
            match r.tick(dt, 0) {
                ReactorEvent::Scrammed => saw_scram = true,
                ReactorEvent::Meltdown => {
                    melted = true;
                    break;
                }
                _ => {}
            }
        }
        assert!(saw_scram, "neglect triggers the auto-SCRAM first");
        assert!(melted, "and residual heat still melts it down - never silently safe");
        assert!(r.fuel.is_none() && r.buffer == 0.0, "the core is spent");
    }

    fn coolant_saves_a_scrammed_core_and_unscram_is_gated() {
        let mut r = Reactor {
            fuel: Some(ItemStack { item_id: "fuel_rod".into(), count: 9 }),
            ..Default::default()
        };
        let dt = 1.0 / 20.0;
        // run hot into SCRAM with no coolant
        while !r.scram {
            r.tick(dt, 0);
        }
        assert!(!r.try_unscram(), "no unscram while hot");
        // now flood it: heat falls below the gate and the core comes back
        let dt_c = (REACTOR_COOLANT_RATE as f32 * dt).ceil() as u16;
        for _ in 0..800 {
            r.tick(dt, dt_c * 4);
        }
        assert!(r.heat < UNSCRAM_BELOW, "flooding cools the core, heat={}", r.heat);
        assert!(r.try_unscram());
    }
}
