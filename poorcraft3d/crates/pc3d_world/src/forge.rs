//! The forge authority (pure): fuel + ore -> work heat -> metal bars.
//!
//! The forge is the machine-chain's workshop application: it burns the
//! same milli-units the Boiler network uses, can be charged directly
//! from a live Boiler's steam output (`charge_from_steam` — the journey
//! chain), and exposes exactly the affordances the semantic registry
//! declared for `working_forge`: input (fuel/ore), output (bars),
//! heat (the working temperature), and a work/blocked state machine.
//! Deterministic: same inputs + ticks, same bars.

/// How much heat one unit of fuel yields (milli-heat per milli-fuel).
pub const HEAT_PER_FUEL: i64 = 2;
/// Working heat needed to smelt one ore into one bar.
pub const HEAT_PER_BAR: i64 = 600;
/// Water keeps the firebox safe: required per fuel load (milli).
pub const WATER_PER_FUEL: i64 = 100;
/// Cap on stored ore and bars (the physical slots).
pub const ORE_SLOTS: u8 = 8;
pub const BAR_SLOTS: u8 = 8;

/// The forge's work state — exactly what the registry promised.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForgeState {
    /// No fuel burning and no heat stored.
    Cold,
    /// Fuel burning, heat below working temperature.
    Heating,
    /// Hot enough to smelt; ore present converts as heat allows.
    Ready,
    /// Something specific blocks the work (named).
    Blocked(&'static str),
}

/// One forge.
#[derive(Clone, Debug)]
pub struct Forge {
    pub fuel_milli: i64,
    pub water_milli: i64,
    /// Accumulated working heat (milli-degrees in effect).
    pub heat_milli: i64,
    pub ore: u8,
    pub bars: u8,
    /// Lifetime counters (audits).
    pub lifetime_bars: u64,
    pub lifetime_fuel_burned: i64,
}

impl Default for Forge {
    fn default() -> Self {
        Forge {
            fuel_milli: 0,
            water_milli: 0,
            heat_milli: 0,
            ore: 0,
            bars: 0,
            lifetime_bars: 0,
            lifetime_fuel_burned: 0,
        }
    }
}

impl Forge {
    /// The INPUT anchor: load fuel + the water the firebox needs.
    /// Refuses (false) when the fuel/water ratio is unsafe — a dry fire
    /// is a blocked state, not a quiet one.
    pub fn load_fuel(&mut self, fuel_milli: i64, water_milli: i64) -> bool {
        if fuel_milli <= 0 || water_milli < WATER_PER_FUEL {
            return false;
        }
        self.fuel_milli += fuel_milli;
        self.water_milli += water_milli;
        true
    }

    /// The INPUT anchor: load ore into the slots (refuses past full).
    pub fn load_ore(&mut self, ore: u8) -> bool {
        if ore == 0 || self.ore + ore > ORE_SLOTS {
            return false;
        }
        self.ore += ore;
        true
    }

    /// Charge heat directly from a live Boiler's steam output (the
    /// journey-proven chain feeds the forge).
    pub fn charge_from_steam(&mut self, steam_milli: i64) {
        self.heat_milli += steam_milli;
    }

    /// One work tick: burn fuel into heat; when heat reaches the bar
    /// threshold AND ore is present, smelt (heat consumed, ore -> bar).
    pub fn tick(&mut self) {
        if self.fuel_milli > 0 && self.water_milli >= WATER_PER_FUEL {
            let burn = self.fuel_milli.min(50);
            self.fuel_milli -= burn;
            self.water_milli -= (burn * WATER_PER_FUEL / 1000).max(0);
            self.heat_milli += burn * HEAT_PER_FUEL;
            self.lifetime_fuel_burned += burn;
        }
        while self.heat_milli >= HEAT_PER_BAR && self.ore > 0 && self.bars < BAR_SLOTS {
            self.heat_milli -= HEAT_PER_BAR;
            self.ore -= 1;
            self.bars += 1;
            self.lifetime_bars += 1;
        }
        if self.heat_milli > HEAT_PER_BAR * 4 {
            self.heat_milli = HEAT_PER_BAR * 4; // radiation cap
        }
    }

    /// The OUTPUT anchor: take all smelted bars.
    pub fn take_bars(&mut self) -> u8 {
        let b = self.bars;
        self.bars = 0;
        b
    }

    /// The work/blocked state.
    pub fn state(&self) -> ForgeState {
        if self.bars >= BAR_SLOTS && self.ore > 0 {
            return ForgeState::Blocked("output full — take the bars");
        }
        if self.fuel_milli <= 0 && self.heat_milli < HEAT_PER_BAR {
            if self.ore > 0 {
                return ForgeState::Blocked("no fuel");
            }
            return ForgeState::Cold;
        }
        if self.heat_milli >= HEAT_PER_BAR {
            if self.ore == 0 {
                return ForgeState::Blocked("no ore");
            }
            ForgeState::Ready
        } else {
            ForgeState::Heating
        }
    }
}

// ---------------------------------------------------------------------------
// Tests: the full loop, the blocked states, conservation, determinism
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_full_loop_smelts_ore_into_bars() {
        let mut f = Forge::default();
        assert!(f.load_fuel(2000, 1000));
        assert!(f.load_ore(3));
        // Enough heat: 2000 fuel -> 4000 heat -> 3 bars (1800) + rest.
        for _ in 0..40 {
            f.tick();
        }
        assert_eq!(f.take_bars(), 3, "three ore -> three bars");
        assert_eq!(f.lifetime_bars, 3);
        assert!(f.fuel_milli < 2000, "fuel burned");
    }

    #[test]
    fn the_blocked_states_are_named_and_honest() {
        // Dry firebox refused at the input.
        let mut f = Forge::default();
        assert!(!f.load_fuel(1000, 0), "no water = refused");
        assert_eq!(f.state(), ForgeState::Cold);
        // Ore without fuel = blocked, not cold.
        assert!(f.load_ore(1));
        assert_eq!(f.state(), ForgeState::Blocked("no fuel"));
        // Heat without ore = blocked at working temperature.
        let mut f = Forge::default();
        f.charge_from_steam(HEAT_PER_BAR + 10);
        assert_eq!(f.state(), ForgeState::Blocked("no ore"));
        // Full output while ore remains = blocked with a named reason.
        let mut f = Forge::default();
        f.bars = BAR_SLOTS;
        f.ore = 1;
        f.heat_milli = HEAT_PER_BAR;
        assert!(matches!(f.state(), ForgeState::Blocked(_)));
    }

    #[test]
    fn slots_are_physical() {
        let mut f = Forge::default();
        assert!(f.load_ore(ORE_SLOTS));
        assert!(!f.load_ore(1), "no overfilling the ore slots");
    }

    #[test]
    fn ticks_are_deterministic_and_conservative() {
        let run = || {
            let mut f = Forge::default();
            f.load_fuel(1500, 800);
            f.load_ore(2);
            for _ in 0..25 {
                f.tick();
            }
            (f.heat_milli, f.ore, f.bars, f.fuel_milli)
        };
        assert_eq!(run(), run(), "same inputs + ticks: same state");
        // Conservation: 2 ore became 2 bars (never more).
        let (_, ore, bars, _) = run();
        assert_eq!((ore, bars), (0, 2));
    }
}
