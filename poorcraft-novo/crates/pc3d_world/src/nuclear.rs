//! P3D-703: nuclear branch with explicit safety, world consequence, cap.
//!
//! D-018: open-core nuclear is powerful but hazardous. The reactor is a
//! deterministic thermal machine: control rods throttle the chain
//! reaction, coolant sheds heat, decay heat lingers after shutdown, an
//! automatic SCRAM slams the rods at over-temperature, and a core that
//! outruns its cooling (e.g. coolant lost) melts its containment and
//! leaks dose into a decaying world contamination map. Settlements in
//! the plume lose prosperity. A per-world reactor cap keeps the branch
//! powerful-but-bounded. All integer arithmetic.

use crate::coords::RegionCoord;
use std::collections::BTreeMap;

/// Milli-units: 1_000 = one unit.
pub const MILLI: i64 = 1_000;
/// Milli-heat produced per tick at zero rod insertion (full burn).
pub const FULL_BURN_HEAT: i64 = 6_000;
/// Decay heat after shutdown: this fraction of the last burn lingers
/// and fades 7/8 per tick.
pub const DECAY_HEAT_NUM: i64 = 3;
pub const DECAY_HEAT_DEN: i64 = 4;
/// Milli-heat shed per tick by the coolant loop at full coolant.
pub const COOLING: i64 = 4_000;
/// Passive shed per tick even with no coolant (radiation to air).
pub const PASSIVE_COOL: i64 = 100;
/// Coolant capacity and per-tick boil-off while the core is hot.
pub const FULL_COOLANT: i64 = 100_000;
pub const COOLANT_BOILOFF: i64 = 50;
/// Safe operating temperature; SCRAM fires at/above it, re-arms below.
pub const SAFE_TEMP: i64 = 70_000;
pub const REARM_TEMP: i64 = 50_000;
/// Meltdown temperature: containment takes damage while this hot.
pub const MELTDOWN_TEMP: i64 = 95_000;
/// Containment integrity (milli) and damage per hot tick.
pub const FULL_INTEGRITY: i64 = 100_000;
pub const DAMAGE_PER_TICK: i64 = 5_000;
/// Base milli-dose leaked per tick from a breached core AT meltdown
/// temperature, scaled by how far past meltdown the core is.
pub const LEAK_PER_TICK: i64 = 300;
/// Contamination decay per tick per region.
pub const DECAY_PER_TICK: i64 = 5;
/// Prosperity penalty per 1_000 milli-dose at a settlement's region.
pub const PENALTY_PER_KDOSE: i64 = 2;
/// Hard per-world reactor cap (D-018: powerful but bounded).
pub const MAX_REACTORS: usize = 4;

/// A fission reactor sited at a region.
#[derive(Clone, Debug)]
pub struct Reactor {
    pub id: u64,
    pub site: RegionCoord,
    /// Milli-fuel remaining.
    pub fuel_milli: i64,
    /// Milli-coolant (consumable; lost coolant removes the heat sink).
    pub coolant_milli: i64,
    /// Control-rod insertion 0..=100 (100 = fully inserted = idle).
    pub control_pct: i64,
    /// Core temperature in milli-degrees.
    pub temp_milli: i64,
    /// Lingering decay heat from the last burn, fades 7/8 per tick.
    pub residual_heat: i64,
    /// Containment integrity.
    pub integrity: i64,
    /// True while the automatic SCRAM holds the rods in.
    pub scrammed: bool,
    /// Milli-heat delivered as usable output last tick (the burn the
    /// heat exchanger could draw).
    pub output_heat: i64,
}

impl Reactor {
    pub fn new(id: u64, site: RegionCoord) -> Reactor {
        Reactor {
            id,
            site,
            fuel_milli: 0,
            coolant_milli: FULL_COOLANT,
            control_pct: 100,
            temp_milli: 20_000,
            residual_heat: 0,
            integrity: FULL_INTEGRITY,
            scrammed: false,
            output_heat: 0,
        }
    }

    /// Neutron flux fraction: rods inserted → less reaction. 100%
    /// inserted = 0 flux; 0% = full flux.
    fn flux_pct(&self) -> i64 {
        (100 - self.control_pct.clamp(0, 100)).max(0)
    }

    fn cooling_now(&self) -> i64 {
        let ratio = self.coolant_milli.min(FULL_COOLANT);
        COOLING * ratio / FULL_COOLANT + PASSIVE_COOL
    }

    /// One deterministic tick. Returns the milli-dose leaked this tick
    /// (0 unless containment is breached AND the core is meltdown-hot).
    pub fn tick(&mut self) -> i64 {
        // SCRAM detection at tick start, on last tick's temperature: a
        // tripped SCRAM slams the rods before any burn this tick and
        // re-arms only once the core is cold enough.
        if self.temp_milli >= SAFE_TEMP {
            self.scrammed = true;
        }
        if self.scrammed {
            self.control_pct = 100;
            if self.temp_milli < REARM_TEMP {
                self.scrammed = false;
            }
        }
        // Heat in: burn proportional to flux, fuel-limited; decay heat
        // from the last burn lingers and fades.
        let burn = FULL_BURN_HEAT * self.flux_pct() / 100;
        let burn = burn.min(self.fuel_milli * 6); // fuel-starved cores fade
        if self.fuel_milli > 0 {
            self.fuel_milli = (self.fuel_milli - burn / 6).max(0);
        }
        self.output_heat = burn;
        self.residual_heat = self.residual_heat * 7 / 8;
        if burn > 0 {
            self.residual_heat = burn * DECAY_HEAT_NUM / DECAY_HEAT_DEN;
        }
        // Hot cores boil off coolant; the heat sink shrinks with it.
        if self.temp_milli > 40_000 && self.coolant_milli > 0 {
            self.coolant_milli = (self.coolant_milli - COOLANT_BOILOFF).max(0);
        }
        let cooling = self.cooling_now();
        self.temp_milli += burn + self.residual_heat - cooling;
        if self.temp_milli < 0 {
            self.temp_milli = 0;
        }
        // Meltdown: damage while hot; a breached hot core leaks.
        let mut leaked = 0;
        if self.temp_milli >= MELTDOWN_TEMP {
            if self.integrity > 0 {
                self.integrity = (self.integrity - DAMAGE_PER_TICK).max(0);
            }
            if self.integrity == 0 {
                leaked = LEAK_PER_TICK * (self.temp_milli / MILLI) / 100;
            }
        }
        leaked
    }
}

/// World contamination: region → milli-dose, decaying every tick.
#[derive(Clone, Debug, Default)]
pub struct Contamination {
    pub dose: BTreeMap<(i32, i32), i64>,
}

impl Contamination {
    /// Deposit leaked dose at the reactor's region (and half at each
    /// orthogonal neighbour — the plume spreads).
    pub fn leak(&mut self, site: RegionCoord, milli_dose: i64) {
        let e = self.dose.entry((site.x, site.z)).or_insert(0);
        *e += milli_dose;
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let e = self.dose.entry((site.x + dx, site.z + dz)).or_insert(0);
            *e += milli_dose / 2;
        }
    }

    /// Decay every region by DECAY_PER_TICK; drop empty entries.
    pub fn decay(&mut self) {
        let mut empty = Vec::new();
        for (k, v) in self.dose.iter_mut() {
            *v = (*v - DECAY_PER_TICK).max(0);
            if *v == 0 {
                empty.push(*k);
            }
        }
        for k in empty {
            self.dose.remove(&k);
        }
    }

    /// Dose at a region (0 if clean).
    pub fn at(&self, r: RegionCoord) -> i64 {
        self.dose.get(&(r.x, r.z)).copied().unwrap_or(0)
    }
}

/// Why a reactor was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Siterror {
    /// D-018 cap: the world already hosts MAX_REACTORS.
    CapReached,
}

/// The world's nuclear program: a bounded set of reactors plus the
/// shared contamination map. This is the canonical owner of dose.
#[derive(Clone, Debug, Default)]
pub struct NuclearProgram {
    pub reactors: BTreeMap<u64, Reactor>,
    pub contamination: Contamination,
    next_id: u64,
}

impl NuclearProgram {
    pub fn new() -> NuclearProgram {
        NuclearProgram::default()
    }

    /// Site a new reactor; refused beyond the world cap.
    pub fn site_reactor(&mut self, at: RegionCoord) -> Result<u64, Siterror> {
        if self.reactors.len() >= MAX_REACTORS {
            return Err(Siterror::CapReached);
        }
        self.next_id += 1;
        let id = self.next_id;
        self.reactors.insert(id, Reactor::new(id, at));
        Ok(id)
    }

    /// Load fuel or refill coolant (milli-units). kind: true = fuel.
    pub fn supply(&mut self, id: u64, fuel_milli: i64, coolant_milli: i64) -> bool {
        let Some(r) = self.reactors.get_mut(&id) else {
            return false;
        };
        r.fuel_milli += fuel_milli;
        r.coolant_milli = (r.coolant_milli + coolant_milli).min(FULL_COOLANT * 2);
        true
    }

    /// Set control-rod insertion (0 = full flux, 100 = idle).
    pub fn set_rods(&mut self, id: u64, pct: i64) -> bool {
        let Some(r) = self.reactors.get_mut(&id) else {
            return false;
        };
        r.control_pct = pct.clamp(0, 100);
        true
    }

    /// Advance the whole program one tick: reactors run (SCRAM and
    /// meltdown included), leaks deposit, contamination decays.
    pub fn tick(&mut self) {
        let ids: Vec<u64> = self.reactors.keys().copied().collect();
        for id in ids {
            let (site, leaked) = {
                let r = self.reactors.get_mut(&id).expect("id from keys");
                let leaked = r.tick();
                (r.site, leaked)
            };
            if leaked > 0 {
                self.contamination.leak(site, leaked);
            }
        }
        self.contamination.decay();
    }

    /// Prosperity penalty at a settlement region from current dose.
    pub fn prosperity_penalty(&self, at: RegionCoord) -> i64 {
        self.contamination.at(at) / MILLI * PENALTY_PER_KDOSE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The power curve: rod insertion throttles output deterministically
    /// and fuel is consumed proportionally to flux.
    #[test]
    fn p3d703_rod_insertion_throttles_output() {
        let mut p = NuclearProgram::new();
        let r = p.site_reactor(RegionCoord { x: 0, z: 0 }).unwrap();
        assert!(p.supply(r, 100_000, 0));

        assert!(p.set_rods(r, 100)); // fully inserted = idle
        p.tick();
        assert_eq!(p.reactors[&r].output_heat, 0, "idle rods make no heat");

        assert!(p.set_rods(r, 0)); // full flux
        p.tick();
        assert_eq!(p.reactors[&r].output_heat, FULL_BURN_HEAT);

        assert!(p.set_rods(r, 75)); // 25% flux
        p.tick();
        assert_eq!(p.reactors[&r].output_heat, FULL_BURN_HEAT / 4);

        // Determinism: an identical program with the identical history
        // reproduces output AND temperature exactly.
        let mut q = NuclearProgram::new();
        let rq = q.site_reactor(RegionCoord { x: 0, z: 0 }).unwrap();
        assert!(q.supply(rq, 100_000, 0));
        assert!(q.set_rods(rq, 100));
        q.tick();
        assert!(q.set_rods(rq, 0));
        q.tick();
        assert!(q.set_rods(rq, 75));
        q.tick();
        assert_eq!(q.reactors[&rq].output_heat, p.reactors[&r].output_heat);
        assert_eq!(q.reactors[&rq].temp_milli, p.reactors[&r].temp_milli);
        assert_eq!(q.reactors[&rq].fuel_milli, p.reactors[&r].fuel_milli);
    }

    /// The automatic SCRAM is non-negotiable: run hot with rods out and
    /// the reactor slams itself idle before meltdown, cools, re-arms —
    /// containment never takes damage behind a working safety.
    #[test]
    fn p3d703_scram_protects_the_core() {
        let mut p = NuclearProgram::new();
        let r = p.site_reactor(RegionCoord { x: 3, z: -2 }).unwrap();
        assert!(p.supply(r, 500_000, 0));
        assert!(p.set_rods(r, 0));

        let mut scram_tick = None;
        for t in 0..200 {
            p.tick();
            if p.reactors[&r].scrammed {
                scram_tick = Some(t);
                break;
            }
        }
        assert!(scram_tick.is_some(), "over-temperature must SCRAM");
        assert!(p.reactors[&r].temp_milli >= SAFE_TEMP);
        assert!(
            p.reactors[&r].temp_milli < MELTDOWN_TEMP,
            "SCRAM beats meltdown"
        );
        assert_eq!(p.reactors[&r].control_pct, 100, "rods slammed in");
        assert_eq!(p.reactors[&r].output_heat, 0, "SCRAM stops the chain");
        assert_eq!(p.reactors[&r].integrity, FULL_INTEGRITY, "no damage");

        let mut rearmed = false;
        for _ in 0..600 {
            p.tick();
            if !p.reactors[&r].scrammed {
                rearmed = true;
                break;
            }
        }
        assert!(rearmed, "SCRAM must re-arm below REARM_TEMP");
        assert!(p.reactors[&r].temp_milli < REARM_TEMP);
        assert_eq!(p.reactors[&r].integrity, FULL_INTEGRITY);
    }

    /// With the coolant lost, even a SCRAM cannot save the core: decay
    /// heat melts containment, the breach leaks into the contamination
    /// map, the plume spreads, settlements pay, and dose decays after.
    #[test]
    fn p3d703_lost_coolant_melts_down_and_settlements_pay() {
        let mut p = NuclearProgram::new();
        let r = p.site_reactor(RegionCoord { x: 10, z: 10 }).unwrap();
        assert!(p.supply(r, 900_000, -FULL_COOLANT)); // drain the sink

        let mut hot_ticks = 0;
        for _ in 0..400 {
            assert!(p.set_rods(r, 0)); // operator maxes flux
            p.tick();
            if p.reactors[&r].integrity == 0 && p.reactors[&r].temp_milli >= MELTDOWN_TEMP {
                hot_ticks += 1;
            }
        }
        assert_eq!(
            p.reactors[&r].integrity, 0,
            "lost coolant melts containment"
        );
        assert!(hot_ticks > 0, "breached hot core must leak");
        assert!(
            p.reactors[&r].scrammed,
            "SCRAM still fired — it just was not enough"
        );

        // Plume: center dose, half-dose neighbours, clean far region.
        let here = p.contamination.at(RegionCoord { x: 10, z: 10 });
        let near = p.contamination.at(RegionCoord { x: 11, z: 10 });
        assert!(here > 0 && near > 0 && near <= here);
        assert_eq!(p.contamination.at(RegionCoord { x: 40, z: 40 }), 0);

        // Consequence gradient: plume pays, distance doesn't.
        let hot = p.prosperity_penalty(RegionCoord { x: 10, z: 10 });
        let edge = p.prosperity_penalty(RegionCoord { x: 11, z: 10 });
        let clean = p.prosperity_penalty(RegionCoord { x: 40, z: 40 });
        assert!(hot > 0 && edge > 0 && edge < hot && clean == 0);

        // The core cools eventually; leaking stops; dose decays away.
        for _ in 0..20_000 {
            p.tick();
        }
        assert!(
            p.reactors[&r].temp_milli < MELTDOWN_TEMP,
            "core cools when spent"
        );
        let after = p.contamination.at(RegionCoord { x: 10, z: 10 });
        assert!(after < here / 2, "dose decays: {after} vs {here}");
    }

    /// The world cap holds: MAX_REACTORS sites, then refusal; a dry
    /// core fades output instead of vanishing abruptly.
    #[test]
    fn p3d703_world_cap_and_fuel_fade() {
        let mut p = NuclearProgram::new();
        for i in 0..MAX_REACTORS {
            assert!(p.site_reactor(RegionCoord { x: i as i32, z: 0 }).is_ok());
        }
        assert_eq!(p.reactors.len(), MAX_REACTORS);
        assert_eq!(
            p.site_reactor(RegionCoord { x: 99, z: 0 }),
            Err(Siterror::CapReached),
            "D-018 cap enforced"
        );

        let first = *p.reactors.keys().next().unwrap();
        assert!(p.supply(first, 100, 0)); // 100 milli-fuel = dregs
        assert!(p.set_rods(first, 0));
        p.tick();
        let out = p.reactors[&first].output_heat;
        assert!(out > 0 && out < FULL_BURN_HEAT, "faded output {out}");
        assert_eq!(p.reactors[&first].fuel_milli, 0, "dregs consumed");
        p.tick();
        assert_eq!(p.reactors[&first].output_heat, 0, "dry core idle");
    }
}
