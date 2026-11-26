//! The campfire authority (pure): fuel -> heat -> light + warmth + risk.
//!
//! N11: the campfire is the night's answer. It burns the same milli-fuel
//! the forge burns, exposes exactly what the first night needs — burn
//! state (light/audio/smoke draw from it), warmth radius (the cold law
//! queries it), and risk (an unattended fire CAN gutter out; a fuel-hoarding
//! player is never punished beyond darkness). Deterministic: same inputs
//! + ticks, same state. Persistence is a fixed 24-byte payload (VS V6)
//! — the session trailer carries it (pre-N11 saves decode to None).

/// Milli-fuel one Wood item loads (a log is heavy; the forge agrees).
pub const FUEL_PER_WOOD: i64 = 1000;
/// Milli-heat per milli-fuel (the forge's own exchange rate).
pub const HEAT_PER_FUEL: i64 = 2;
/// Burn rate per tick (milli-fuel) — 1000/tick means a full log lasts
/// 10 ticks; at ~6 ticks/s a log burns ~1.7 real minutes.
pub const BURN_PER_TICK: i64 = 100;
/// The warmth radius (meters): the player this close to a BURNING fire
/// does not lose warmth at night.
pub const WARMTH_RADIUS_M: f32 = 4.0;
/// The light radius (meters): darkness lifts this far from a burning fire.
pub const LIGHT_RADIUS_M: f32 = 9.0;
/// Cap on stored fuel (the wood pile's physical size).
pub const FUEL_CAP: i64 = 5000;

/// The campfire's burn state — exactly what the HUD and the render need.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CampfireState {
    /// Nothing burning, no ember heat.
    Cold,
    /// Fuel burning (light + warmth + smoke live).
    Burning,
    /// Fuel spent but ember heat remains (fades to Cold; no light).
    Embers,
}

/// One campfire.
#[derive(Clone, Debug, PartialEq)]
pub struct Campfire {
    pub fuel_milli: i64,
    /// Residual heat after the fuel is gone (milli-heat).
    pub ember_heat_milli: i64,
    /// Lifetime counters (audits).
    pub lifetime_fuel_burned: i64,
}

impl Default for Campfire {
    fn default() -> Self {
        Campfire {
            fuel_milli: 0,
            ember_heat_milli: 0,
            lifetime_fuel_burned: 0,
        }
    }
}

impl Campfire {
    /// The INPUT anchor: one Wood from the pack. Refuses past the cap —
    /// the honest "pile is full".
    pub fn add_wood(&mut self) -> bool {
        if self.fuel_milli + FUEL_PER_WOOD > FUEL_CAP {
            return false;
        }
        self.fuel_milli += FUEL_PER_WOOD;
        true
    }

    /// One burn tick: fuel becomes heat; spent fuel becomes embers.
    pub fn tick(&mut self) {
        if self.fuel_milli > 0 {
            let burn = self.fuel_milli.min(BURN_PER_TICK);
            self.fuel_milli -= burn;
            self.ember_heat_milli += burn * HEAT_PER_FUEL;
            self.lifetime_fuel_burned += burn;
        }
        // Embers cool: residual heat decays without feeding light.
        if self.fuel_milli == 0 && self.ember_heat_milli > 0 {
            self.ember_heat_milli = (self.ember_heat_milli - BURN_PER_TICK).max(0);
        }
    }

    /// The burn state (light/warmth draw from BURNING only).
    pub fn state(&self) -> CampfireState {
        if self.fuel_milli > 0 {
            CampfireState::Burning
        } else if self.ember_heat_milli > 0 {
            CampfireState::Embers
        } else {
            CampfireState::Cold
        }
    }

    /// The light fraction [0,1] the render dims toward (BURNING only —
    /// embers glow faintly at a fixed 15% floor).
    pub fn light_frac(&self) -> f32 {
        match self.state() {
            CampfireState::Burning => (self.fuel_milli as f32 / FUEL_PER_WOOD as f32).min(1.0),
            CampfireState::Embers => 0.15,
            CampfireState::Cold => 0.0,
        }
    }

    /// True when a body at `dist_m` is inside the warmth radius (the
    /// cold law's warm answer). BURNING only — embers do not warm.
    pub fn warms(&self, dist_m: f32) -> bool {
        self.state() == CampfireState::Burning && dist_m <= WARMTH_RADIUS_M
    }

    /// Fixed 24-byte payload for the session trailer (VS V6).
    pub fn encode(&self) -> [u8; 24] {
        let mut b = [0u8; 24];
        b[0..8].copy_from_slice(&self.fuel_milli.to_le_bytes());
        b[8..16].copy_from_slice(&self.ember_heat_milli.to_le_bytes());
        b[16..24].copy_from_slice(&self.lifetime_fuel_burned.to_le_bytes());
        b
    }

    pub fn decode(bytes: &[u8]) -> Option<Campfire> {
        if bytes.len() < 24 {
            return None;
        }
        Some(Campfire {
            fuel_milli: i64::from_le_bytes(bytes[0..8].try_into().ok()?),
            ember_heat_milli: i64::from_le_bytes(bytes[8..16].try_into().ok()?),
            lifetime_fuel_burned: i64::from_le_bytes(bytes[16..24].try_into().ok()?),
        })
    }
}

/// The cold law (pure): one night-tick of the body's warmth in [0,1].
///
/// `night` is the shader's own night factor (0 day .. 1 night) — the SAME
/// authority the sky reads, so the cold arrives exactly when the dark does.
/// Shelter and a burning campfire within reach both hold the line; the
/// night alone drains at `DRAIN_PER_TICK`. This never touches health —
/// the DECISION is "get warm", the failure is darkness, not instadeath.
pub const DRAIN_PER_TICK: f32 = 0.004;
pub const REGEN_PER_TICK: f32 = 0.02;

pub fn warmth_tick(warmth: f32, night: f32, sheltered: bool, near_burning_fire: bool) -> f32 {
    let w = warmth.clamp(0.0, 1.0);
    if near_burning_fire || sheltered {
        (w + REGEN_PER_TICK).min(1.0)
    } else if std::env::var("PC3D_MUTATE_COLD_OFF").is_ok() {
        // Mutation lever (debug): PC3D_MUTATE_COLD_OFF severs the cold
        // drain — the night stops being a threat. The route's
        // warmth_drained_away verdict must go RED under it (the gate
        // is load-bearing); clean runs leave the law whole.
        w
    } else {
        (w - DRAIN_PER_TICK * night).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p3dn11_wood_burns_and_gutters() {
        let mut f = Campfire::default();
        assert_eq!(f.state(), CampfireState::Cold);
        assert!(f.add_wood());
        assert_eq!(f.state(), CampfireState::Burning);
        // 10 ticks at 100/tick burns the 1000 log exactly; embers remain.
        for _ in 0..10 {
            f.tick();
        }
        assert_eq!(f.fuel_milli, 0);
        assert_eq!(f.state(), CampfireState::Embers);
        // Embers cool without feeding light: light dips to the 15% floor,
        // then the fire goes cold.
        assert!((f.light_frac() - 0.15).abs() < 1e-4);
        for _ in 0..300 {
            f.tick();
        }
        assert_eq!(f.state(), CampfireState::Cold);
        assert_eq!(f.light_frac(), 0.0);
    }

    #[test]
    fn p3dn11_fuel_cap_refuses_honest() {
        let mut f = Campfire::default();
        assert!(f.add_wood());
        f.fuel_milli = FUEL_CAP;
        assert!(!f.add_wood(), "pile full must refuse");
        assert_eq!(f.fuel_milli, FUEL_CAP);
    }

    #[test]
    fn p3dn11_warmth_radius_is_burning_only() {
        let mut f = Campfire::default();
        assert!(!f.warms(1.0), "cold fire warms nobody");
        f.add_wood();
        assert!(f.warms(WARMTH_RADIUS_M));
        assert!(f.warms(WARMTH_RADIUS_M - 0.1));
        assert!(!f.warms(WARMTH_RADIUS_M + 0.1));
        // Ember state does NOT warm (the law: keep it fed).
        f.fuel_milli = 0;
        f.ember_heat_milli = 400;
        assert!(!f.warms(0.5), "embers glow, they do not warm");
    }

    #[test]
    fn p3dn11_cold_law_sides() {
        // Day (night=0): nothing drains.
        assert_eq!(warmth_tick(1.0, 0.0, false, false), 1.0);
        // Night in the open: drains.
        let w = warmth_tick(1.0, 1.0, false, false);
        assert!((w - (1.0 - DRAIN_PER_TICK)).abs() < 1e-5);
        // Night by a burning fire: regenerates.
        let w = warmth_tick(0.5, 1.0, false, true);
        assert!((w - (0.5 + REGEN_PER_TICK).min(1.0)).abs() < 1e-5);
        // Night inside a shelter (no fire): holds the line.
        let w = warmth_tick(0.5, 1.0, true, false);
        assert!((w - (0.5 + REGEN_PER_TICK)).abs() < 1e-5);
        // Clamps hold.
        assert_eq!(warmth_tick(0.0, 1.0, false, false), 0.0);
        assert_eq!(warmth_tick(0.99, 1.0, true, false), 1.0);
    }

    #[test]
    fn p3dn11_cold_gate_is_load_bearing_mutation_check() {
        // The lever's own sanity: with the law LIVE the open night
        // drains; with PC3D_MUTATE_COLD_OFF set the same call holds —
        // the exact delta the route's mutation run proves at runtime.
        let live = warmth_tick(1.0, 1.0, false, false);
        std::env::set_var("PC3D_MUTATE_COLD_OFF", "1");
        let severed = warmth_tick(1.0, 1.0, false, false);
        std::env::remove_var("PC3D_MUTATE_COLD_OFF");
        assert!(live < 1.0, "the open night must drain (cold law live)");
        assert_eq!(severed, 1.0, "the lever must sever the cold drain");
    }

    #[test]
    fn p3dn11_encode_round_trips() {
        let mut f = Campfire::default();
        f.add_wood();
        f.tick();
        f.tick();
        let bytes = f.encode();
        assert_eq!(Campfire::decode(&bytes), Some(f));
        assert_eq!(Campfire::decode(&bytes[..20]), None, "truncated refuses");
    }

    #[test]
    fn p3dn11_deterministic_same_ticks_same_ashes() {
        let burn = |wood: usize, ticks: usize| {
            let mut f = Campfire::default();
            for _ in 0..wood {
                f.add_wood();
            }
            for _ in 0..ticks {
                f.tick();
            }
            f
        };
        assert_eq!(burn(3, 25), burn(3, 25));
        assert_ne!(burn(3, 25), burn(2, 25));
    }
}
