//! P3D-704 (part 2): the advanced magic branch — Ley attunement.
//!
//! Beyond the basic runes of `magic.rs`, attuned casters bind a region
//! to the Ley and work rituals with real, two-sided consequences: a
//! Blessing raises a settlement's prosperity but drains the caster's
//! mana and herbs and taints them with strain; a Blight rots a foreign
//! settlement but costs far more and stains the caster's karma-bearing
//! record. Rituals have cooldowns, costs are charged atomically (the
//! inventory law: nothing is consumed unless the ritual actually
//! casts), and every roll is integer-deterministic.

use crate::coords::RegionCoord;
use crate::settlement::{Aggregate, Settlements};

/// Ley attunement tiers and their ritual ceilings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Attunement {
    Dormant,
    Wakened,
    Bound,
    Chorus,
}

impl Attunement {
    /// Mana cost multiplier in percent (Dormant cannot work Ley magic).
    pub fn cost_pct(self) -> i64 {
        match self {
            Attunement::Dormant => 0,
            Attunement::Wakened => 100,
            Attunement::Bound => 80,
            Attunement::Chorus => 60,
        }
    }
    /// Days between rituals at this tier.
    pub fn cooldown_days(self) -> u32 {
        match self {
            Attunement::Dormant => 0,
            Attunement::Wakened => 20,
            Attunement::Bound => 12,
            Attunement::Chorus => 7,
        }
    }
}

/// Base ritual costs before attunement discount.
pub const BLESS_MANA: i64 = 40;
pub const BLESS_HERBS: i64 = 5;
pub const BLESS_PROSPERITY: i64 = 6;
pub const BLIGHT_MANA: i64 = 90;
pub const BLIGHT_HERBS: i64 = 15;
pub const BLIGHT_DRAIN: i64 = 8;
/// Strain per cast; at BACKLASH_STRAIN the Ley turns on the caster.
pub const BLESS_STRAIN: i64 = 3;
pub const BLIGHT_STRAIN: i64 = 12;
pub const BACKLASH_STRAIN: i64 = 30;
/// Strain recovered per day of rest.
pub const STRAIN_RECOVERY: i64 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ritual {
    Bless,
    Blight,
}

impl Ritual {
    fn base_mana(self) -> i64 {
        match self {
            Ritual::Bless => BLESS_MANA,
            Ritual::Blight => BLIGHT_MANA,
        }
    }
    fn herbs(self) -> i64 {
        match self {
            Ritual::Bless => BLESS_HERBS,
            Ritual::Blight => BLIGHT_HERBS,
        }
    }
    fn strain(self) -> i64 {
        match self {
            Ritual::Bless => BLESS_STRAIN,
            Ritual::Blight => BLIGHT_STRAIN,
        }
    }
}

/// Why a ritual refused to cast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RitualError {
    Dormant,
    Exhausted,
    Cooldown,
    MissingHerbs,
    NoTarget,
}

/// An attuned caster's state. Costs are charged only when the ritual
/// actually resolves (atomic-cast law).
#[derive(Clone, Debug)]
pub struct LeyCaster {
    pub attunement: Attunement,
    pub mana: i64,
    pub herbs: i64,
    pub strain: i64,
    pub cd_days: u32,
    /// Cumulative good/bad workings — the karma-facing ledger.
    pub blessings: u64,
    pub blights: u64,
}

impl LeyCaster {
    pub fn new(attunement: Attunement) -> LeyCaster {
        LeyCaster {
            attunement,
            mana: 100,
            herbs: 20,
            strain: 0,
            cd_days: 0,
            blessings: 0,
            blights: 0,
        }
    }

    /// Day tick: cooldowns fall, strain recovers with rest.
    pub fn tick_day(&mut self) {
        self.cd_days = self.cd_days.saturating_sub(1);
        self.strain = (self.strain - STRAIN_RECOVERY).max(0);
        self.mana = (self.mana + 10).min(100);
    }

    /// Work a ritual on the nearest settlement within `radius` of
    /// `at`. On success the settlement aggregate moves and ALL costs
    /// (mana, herbs, strain, cooldown) are charged together. Returns
    /// the settlement id worked on.
    pub fn work(
        &mut self,
        ritual: Ritual,
        at: RegionCoord,
        radius: i32,
        settlements: &mut Settlements,
    ) -> Result<u64, RitualError> {
        if self.attunement == Attunement::Dormant {
            return Err(RitualError::Dormant);
        }
        if self.cd_days > 0 {
            return Err(RitualError::Cooldown);
        }
        let pct = self.attunement.cost_pct();
        let mana_cost = ritual.base_mana() * pct / 100;
        let herb_cost = ritual.herbs();
        if self.mana < mana_cost {
            return Err(RitualError::Exhausted);
        }
        if self.herbs < herb_cost {
            return Err(RitualError::MissingHerbs);
        }
        // Target: nearest settlement center within radius (ties → id).
        let mut target: Option<(i32, u64)> = None;
        for s in &settlements.list {
            let d = (s.center.x - at.x).abs().max((s.center.z - at.z).abs());
            if d > radius {
                continue;
            }
            let key = (d, s.id);
            if target.map(|t| key < t).unwrap_or(true) {
                target = Some(key);
            }
        }
        let Some((_, sid)) = target else {
            return Err(RitualError::NoTarget);
        };
        let s = settlements
            .list
            .iter_mut()
            .find(|s| s.id == sid)
            .expect("target id came from the list");
        match ritual {
            Ritual::Bless => {
                s.aggregate.prosperity =
                    (s.aggregate.prosperity + BLESS_PROSPERITY).min(100);
                s.aggregate.population += 1;
                self.blessings += 1;
            }
            Ritual::Blight => {
                let a: &mut Aggregate = &mut s.aggregate;
                a.prosperity = (a.prosperity - BLIGHT_DRAIN).max(0);
                a.food = (a.food - BLIGHT_DRAIN * 5).max(0);
                self.blights += 1;
            }
        }
        // Atomic charge: the ritual happened, so everything is paid.
        self.mana -= mana_cost;
        self.herbs -= herb_cost;
        self.strain += ritual.strain();
        self.cd_days = self.attunement.cooldown_days();
        Ok(sid)
    }

    /// True when accumulated strain crosses the backlash line: the next
    /// working should fail catastrophically (caller enforces).
    pub fn backlash_due(&self) -> bool {
        self.strain >= BACKLASH_STRAIN
    }

    /// The backlash strike: caster's mana burns, herbs scatter, strain
    /// resets past the line (the Ley takes its due).
    pub fn take_backlash(&mut self) {
        self.mana = 0;
        self.herbs = self.herbs / 2;
        self.strain -= BACKLASH_STRAIN / 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settlement::{Settlement, SettlementState};

    fn one_town(id: u64, x: i32, z: i32, prosperity: i64) -> Settlements {
        Settlements {
            list: vec![Settlement {
                id,
                name: "Leytown",
                center: RegionCoord { x, z },
                state: SettlementState::Aggregate,
                aggregate: Aggregate {
                    population: 80,
                    food: 200,
                    defense: 30,
                    prosperity,
                },
            }],
        }
    }

    /// Blessing raises the near settlement and charges all costs
    /// atomically; dormant casters and spent mana/herbs are refused
    /// with NOTHING consumed.
    #[test]
    fn p3d704_bless_costs_are_atomic() {
        let mut c = LeyCaster::new(Attunement::Wakened);
        let mut s = one_town(1, 2, 0, 50);

        let id = c.work(Ritual::Bless, RegionCoord { x: 0, z: 0 }, 5, &mut s).unwrap();
        assert_eq!(id, 1);
        let town = &s.list[0].aggregate;
        assert_eq!(town.prosperity, 56, "blessing lifts");
        assert_eq!(town.population, 81);
        assert_eq!(c.mana, 60, "100 - 40 at Wakened");
        assert_eq!(c.herbs, 15);
        assert_eq!(c.strain, BLESS_STRAIN);
        assert_eq!(c.cd_days, Attunement::Wakened.cooldown_days());
        assert_eq!(c.blessings, 1);

        // Cooldown blocks the very next day; nothing is consumed.
        assert_eq!(
            c.work(Ritual::Bless, RegionCoord { x: 0, z: 0 }, 5, &mut s),
            Err(RitualError::Cooldown)
        );
        assert_eq!((c.mana, c.herbs), (60, 15), "refused cast costs nothing");

        // Dormant casters cannot work the Ley at all.
        let mut d = LeyCaster::new(Attunement::Dormant);
        assert_eq!(
            d.work(Ritual::Bless, RegionCoord { x: 0, z: 0 }, 5, &mut s),
            Err(RitualError::Dormant)
        );

        // Exhausted mana refused atomically.
        let mut e = LeyCaster::new(Attunement::Wakened);
        e.mana = 10;
        assert_eq!(
            e.work(Ritual::Bless, RegionCoord { x: 0, z: 0 }, 5, &mut s),
            Err(RitualError::Exhausted)
        );
        assert_eq!((e.mana, e.herbs), (10, 20), "nothing consumed");
    }

    /// Blight rots a foreign town for a steeper price and stains the
    /// ledger; strain accumulates toward backlash, which the caster
    /// can then take; rest recovers strain and cooldowns fall.
    #[test]
    fn p3d704_blight_strain_and_backlash() {
        let mut c = LeyCaster::new(Attunement::Chorus);
        let mut s = one_town(9, 0, 0, 50);

        assert!(c.work(Ritual::Blight, RegionCoord { x: 0, z: 0 }, 3, &mut s).is_ok());
        let town = &s.list[0].aggregate;
        assert_eq!(town.prosperity, 42, "drained");
        assert_eq!(town.food, 160, "rotted stores");
        assert_eq!(c.blights, 1);
        assert_eq!(c.mana, 100 - BLIGHT_MANA * 60 / 100, "Chorus discount");
        assert_eq!(c.strain, BLIGHT_STRAIN);

        // Work blessings fast (Chorus cooldown) to climb strain: one
        // blight (12) + three blessings (9) = 21, one more blight
        // crosses the line at 30.
        for _ in 0..3 {
            c.mana = 100;
            c.herbs = 50;
            c.cd_days = 0;
            assert!(c.work(Ritual::Bless, RegionCoord { x: 0, z: 0 }, 3, &mut s).is_ok());
        }
        assert!(!c.backlash_due(), "21 < 30 so far");
        c.mana = 100;
        c.herbs = 50;
        c.cd_days = 0;
        assert!(c.work(Ritual::Blight, RegionCoord { x: 0, z: 0 }, 3, &mut s).is_ok());
        assert_eq!(c.strain, 33, "12 + 3×3 + 12");
        assert!(c.backlash_due());

        c.take_backlash();
        assert_eq!(c.mana, 0, "the Ley takes its due");
        assert_eq!(c.herbs, 17, "half the satchel scatters (35 after the cast)");
        assert_eq!(c.strain, 18, "strain released past the line: 33 - 15");

        // Rest: strain recovers, mana regenerates, cooldowns fall.
        let mut r = LeyCaster::new(Attunement::Wakened);
        r.strain = 10;
        r.mana = 0;
        r.cd_days = 5;
        for _ in 0..5 {
            r.tick_day();
        }
        assert_eq!(r.strain, 0, "10 - 5×2");
        assert_eq!(r.mana, 50);
        assert_eq!(r.cd_days, 0);
    }

    /// Attunement tiers price and gate the work: higher tiers are
    /// cheaper and faster; out-of-range targets cast nothing.
    #[test]
    fn p3d704_tier_pricing_and_range() {
        // Chorus is cheaper than Wakened for the same blessing.
        let mut w = LeyCaster::new(Attunement::Wakened);
        let mut ch = LeyCaster::new(Attunement::Chorus);
        let mut sw = one_town(1, 1, 1, 40);
        let mut sch = one_town(1, 1, 1, 40);
        assert!(w.work(Ritual::Bless, RegionCoord { x: 0, z: 0 }, 5, &mut sw).is_ok());
        assert!(ch.work(Ritual::Bless, RegionCoord { x: 0, z: 0 }, 5, &mut sch).is_ok());
        assert_eq!(w.mana, 60);
        assert_eq!(ch.mana, 100 - BLESS_MANA * 60 / 100);
        assert_eq!(Attunement::Bound.cooldown_days(), 12);
        assert!(Attunement::Chorus.cooldown_days() < Attunement::Bound.cooldown_days());

        // A town outside the radius: the ritual does not resolve and
        // consumes nothing (the "did not cast" refusal).
        let mut c = LeyCaster::new(Attunement::Wakened);
        let mut far = one_town(1, 30, 30, 40);
        assert_eq!(
            c.work(Ritual::Bless, RegionCoord { x: 0, z: 0 }, 5, &mut far),
            Err(RitualError::NoTarget),
            "no target in reach"
        );
        assert_eq!((c.mana, c.herbs, c.cd_days, c.blessings), (100, 20, 0, 0));
        assert_eq!(far.list[0].aggregate.prosperity, 40);
    }
}
