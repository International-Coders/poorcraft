//! P3D-704 (part 1): the dragon — territory, raids, slaying, pact.
//!
//! A dragon is a long-lived world threat: while awake it grows in power
//! and periodically raids settlements inside its territory (prosperity,
//! population, and defense losses; the region is scorched). The player
//! answers with a slayer assault (deterministic integer combat — no
//! floats) or, when the beast is weakened, a tribute pact that wards its
//! territory at a price. Both endings reshape factions: slaying earns
//! awe, a pact earns suspicion, and a breached pact enrages the dragon.
//! Permanent consequences; D-034 spirit applies to the dragon too.

use crate::coords::RegionCoord;
use crate::settlement::{Aggregate, Settlement, Settlements};
use std::collections::{BTreeMap, BTreeSet};

/// Territory radius in regions (Chebyshev) around the lair.
pub const TERRITORY_RADIUS: i32 = 12;
/// Days between raids while a settlement sits in territory.
pub const RAID_COOLDOWN_DAYS: u32 = 30;
/// Power growth per day while awake.
pub const POWER_GROWTH: i64 = 2;
/// Dragon starting hp / power.
pub const DRAGON_HP: i64 = 100;
pub const DRAGON_POWER: i64 = 20;
/// Trust deltas: awe for slaying, suspicion for pacting.
pub const AWE_FOR_SLAYING: i64 = 20;
pub const SUSPICION_FOR_PACT: i64 = 10;
pub const ENRAGE_POWER: i64 = 20;

/// SplitMix64 finalizer: the deterministic combat die.
pub fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pact {
    /// Tribute owed per month (arbitrary wealth units).
    pub tribute_per_month: i64,
    /// Days since the pact was sealed.
    pub sealed_day: u64,
}

#[derive(Clone, Debug)]
pub struct Dragon {
    pub id: u64,
    pub lair: RegionCoord,
    pub power: i64,
    pub hp: i64,
    pub awake: bool,
    pub alive: bool,
    pub pact: Option<Pact>,
    /// Days until the next raid (while awake, no pact).
    pub raid_in: u32,
}

impl Dragon {
    pub fn new(id: u64, lair: RegionCoord) -> Dragon {
        Dragon {
            id,
            lair,
            power: DRAGON_POWER,
            hp: DRAGON_HP,
            awake: true,
            alive: true,
            pact: None,
            raid_in: RAID_COOLDOWN_DAYS,
        }
    }

    pub fn in_territory(&self, r: RegionCoord) -> bool {
        (self.lair.x - r.x).abs() <= TERRITORY_RADIUS
            && (self.lair.z - r.z).abs() <= TERRITORY_RADIUS
    }

    /// Dragon strength in assault terms: raw power plus its remaining
    /// vitality (hp/10).
    pub fn strength(&self) -> i64 {
        self.power + self.hp / 10
    }
}

/// How an assault ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssaultOutcome {
    /// The dragon is slain permanently.
    Slain,
    /// The party broke; the dragon grows bold.
    Repelled { casualties: i64 },
    /// No such dragon, or it is already dead.
    NoTarget,
}

/// Faction-facing consequences ledger: faction id → cumulative stance
/// from dragon dealings (positive awe, negative suspicion).
#[derive(Clone, Debug, Default)]
pub struct DragonReputation {
    pub stance: BTreeMap<u64, i64>,
}

impl DragonReputation {
    pub fn of(&self, faction: u64) -> i64 {
        self.stance.get(&faction).copied().unwrap_or(0)
    }
}

/// The world's dragon layer: canonical owner of dragons, scorched
/// regions, and dragon-derived faction stance.
#[derive(Clone, Debug, Default)]
pub struct DragonWorld {
    pub dragons: BTreeMap<u64, Dragon>,
    pub scorched: BTreeSet<(i32, i32)>,
    pub reputation: DragonReputation,
    next_id: u64,
}

impl DragonWorld {
    pub fn new() -> DragonWorld {
        DragonWorld::default()
    }

    pub fn spawn(&mut self, lair: RegionCoord) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.dragons.insert(id, Dragon::new(id, lair));
        id
    }

    /// One deterministic day: awake, un-pacted dragons grow and raid
    /// the nearest settlement in territory (ties break by lower id).
    /// Returns the raided settlement id, if any.
    pub fn tick_day(&mut self, settlements: &mut Settlements, day: u64) -> Option<u64> {
        let ids: Vec<u64> = self.dragons.keys().copied().collect();
        let mut raided = None;
        for id in ids {
            let dragon = self.dragons.get_mut(&id).expect("id from keys");
            if !dragon.alive || !dragon.awake || dragon.pact.is_some() {
                continue;
            }
            dragon.power += POWER_GROWTH;
            dragon.raid_in = dragon.raid_in.saturating_sub(1);
            if dragon.raid_in > 0 {
                continue;
            }
            dragon.raid_in = RAID_COOLDOWN_DAYS;
            // Nearest settlement in territory (distance, then id).
            let mut target: Option<(i32, u64)> = None;
            for s in &settlements.list {
                if !dragon.in_territory(s.center) {
                    continue;
                }
                let d = (dragon.lair.x - s.center.x).abs()
                    .max((dragon.lair.z - s.center.z).abs());
                let key = (d, s.id);
                if target.map(|t| key < t).unwrap_or(true) {
                    target = Some(key);
                }
            }
            if let Some((_, sid)) = target {
                let power = dragon.power;
                if let Some(s) = settlements.list.iter_mut().find(|s| s.id == sid) {
                    raid_settlement(&mut s.aggregate, power);
                    self.scorched.insert((s.center.x, s.center.z));
                }
                raided = Some(sid);
            }
            let _ = day;
        }
        raided
    }

    /// A slayer assault: `party_power` vs the dragon's strength, with
    /// one deterministic die from `seed`. Slaying is permanent.
    pub fn assault(
        &mut self,
        dragon_id: u64,
        party_power: i64,
        faction: u64,
        seed: u64,
    ) -> AssaultOutcome {
        let Some(d) = self.dragons.get_mut(&dragon_id) else {
            return AssaultOutcome::NoTarget;
        };
        if !d.alive {
            return AssaultOutcome::NoTarget;
        }
        let roll = (mix64(seed ^ d.id.wrapping_mul(0x9e3779b97f4a7c15)) % 100) as i64;
        if party_power + roll > d.strength() {
            d.alive = false;
            d.awake = false;
            d.hp = 0;
            d.pact = None;
            *self.reputation.stance.entry(faction).or_insert(0) += AWE_FOR_SLAYING;
            AssaultOutcome::Slain
        } else {
            let casualties = (party_power / 2).max(1);
            d.power += 10; // emboldened by the feeble attempt
            d.hp = (d.hp + 20).min(DRAGON_HP);
            AssaultOutcome::Repelled { casualties }
        }
    }

    /// Spare a weakened (hp ≤ 30%) dragon for standing tribute: its
    /// territory becomes warded — no raids while the pact holds — but
    /// the pactmaker's faction earns suspicion.
    pub fn offer_pact(
        &mut self,
        dragon_id: u64,
        tribute_per_month: i64,
        faction: u64,
        day: u64,
    ) -> Result<(), &'static str> {
        let Some(d) = self.dragons.get_mut(&dragon_id) else {
            return Err("no such dragon");
        };
        if !d.alive {
            return Err("a dead dragon signs nothing");
        }
        if d.hp * 10 > DRAGON_HP * 3 {
            return Err("only a weakened dragon accepts terms");
        }
        if tribute_per_month <= 0 {
            return Err("tribute must be real");
        }
        d.pact = Some(Pact { tribute_per_month, sealed_day: day });
        d.awake = false;
        *self.reputation.stance.entry(faction).or_insert(0) -= SUSPICION_FOR_PACT;
        Ok(())
    }

    /// Break the pact: the dragon wakes enraged and raiding resumes.
    pub fn breach_pact(&mut self, dragon_id: u64) -> bool {
        let Some(d) = self.dragons.get_mut(&dragon_id) else { return false };
        if d.pact.take().is_none() {
            return false;
        }
        d.awake = true;
        d.power += ENRAGE_POWER;
        d.raid_in = RAID_COOLDOWN_DAYS / 3; // short fuse when enraged
        true
    }
}

/// Raid damage on one settlement's aggregates: prosperity burns,
/// people die, defenses crumble — all clamped at 0.
pub fn raid_settlement(a: &mut Aggregate, dragon_power: i64) {
    a.prosperity = (a.prosperity - dragon_power / 2).max(0);
    a.population = (a.population - dragon_power / 10).max(0);
    a.defense = (a.defense - dragon_power / 3).max(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settlements_in_territory(lair: RegionCoord) -> Settlements {
        // Two hand-placed settlements: one close in territory, one far.
        let mk = |id: u64, x: i32, z: i32| Settlement {
            id,
            name: "Test",
            center: RegionCoord { x, z },
            state: crate::settlement::SettlementState::Aggregate,
            aggregate: Aggregate { population: 100, food: 200, defense: 50, prosperity: 60 },
        };
        Settlements {
            list: vec![
                mk(1, lair.x + 5, lair.z),
                mk(2, lair.x + 40, lair.z),
            ],
        }
    }

    /// Awake dragons raid on cadence: nearest in-territory settlement
    /// takes clamped losses and the region is scorched; the far
    /// settlement is untouched; the same world reproduces the raid
    /// bit-identically.
    #[test]
    fn p3d704_raid_cadence_and_territory() {
        let lair = RegionCoord { x: 0, z: 0 };
        let build = || {
            let mut w = DragonWorld::new();
            w.spawn(lair);
            let s = settlements_in_territory(lair);
            (w, s)
        };
        let (mut w, mut s) = build();
        // Day 1..29: growth only.
        for day in 1..RAID_COOLDOWN_DAYS as u64 {
            assert_eq!(w.tick_day(&mut s, day), None, "no raid before cooldown");
        }
        assert_eq!(w.dragons[&1].power, DRAGON_POWER + 29 * POWER_GROWTH);
        // Day 30: the raid lands on the NEAR settlement.
        let before_far = s.list[1].aggregate;
        let hit = w.tick_day(&mut s, 30);
        assert_eq!(hit, Some(1));
        let near = &s.list[0].aggregate;
        assert!(near.prosperity < 60 && near.population < 100 && near.defense < 50);
        assert_eq!(s.list[1].aggregate, before_far, "outside territory: untouched");
        assert!(w.scorched.contains(&(lair.x + 5, lair.z)));
        assert!(!w.scorched.contains(&(lair.x + 40, lair.z)));
        // Cadence: next raid a full cooldown later.
        for day in 31..30 + RAID_COOLDOWN_DAYS as u64 {
            assert_eq!(w.tick_day(&mut s, day), None);
        }
        assert_eq!(w.tick_day(&mut s, 30 + RAID_COOLDOWN_DAYS as u64), Some(1));

        // Determinism: identical build, identical 60-day history,
        // identical end state.
        let (mut w2, mut s2) = build();
        for day in 1..=(30 + RAID_COOLDOWN_DAYS as u64) {
            w2.tick_day(&mut s2, day);
        }
        assert_eq!(s2.list[0].aggregate, s.list[0].aggregate);
        assert_eq!(s2.list[1].aggregate, s.list[1].aggregate);
        assert_eq!(w2.dragons[&1].power, w.dragons[&1].power);
        assert_eq!(w2.scorched, w.scorched);
    }

    /// Raid losses clamp at zero and repeated raids grind a settlement
    /// toward ruin without panicking.
    #[test]
    fn p3d704_raid_damage_clamps() {
        let mut a = Aggregate { population: 3, food: 10, defense: 2, prosperity: 5 };
        for _ in 0..10 {
            raid_settlement(&mut a, 500);
        }
        assert_eq!((a.prosperity, a.population, a.defense), (0, 0, 0));
    }

    /// Slaying is permanent, deterministic, and earns awe; a repelled
    /// assault emboldens the dragon.
    #[test]
    fn p3d704_slaying_and_repulse() {
        let mut w = DragonWorld::new();
        let id = w.spawn(RegionCoord { x: 0, z: 0 });
        let d = &w.dragons[&id];
        let strength = d.strength();
        assert_eq!(strength, DRAGON_POWER + DRAGON_HP / 10);

        // Hunt the die: one seed wins, and the win is permanent.
        let mut won = None;
        for seed in 0..500u64 {
            let mut probe = DragonWorld::new();
            let pid = probe.spawn(RegionCoord { x: 0, z: 0 });
            if probe.assault(pid, strength - 10, 7, seed) == AssaultOutcome::Slain {
                won = Some(seed);
                break;
            }
        }
        let seed = won.expect("a winning seed exists below strength+die");
        let out = w.assault(id, strength - 10, 7, seed);
        assert_eq!(out, AssaultOutcome::Slain);
        assert!(!w.dragons[&id].alive && w.dragons[&id].hp == 0);
        assert_eq!(w.reputation.of(7), AWE_FOR_SLAYING, "faction earns awe");
        // Dead dragons stay dead; assaults and days do nothing.
        assert_eq!(w.assault(id, 999, 7, seed), AssaultOutcome::NoTarget);
        let mut s = settlements_in_territory(RegionCoord { x: 0, z: 0 });
        for day in 1..200u64 {
            assert_eq!(w.tick_day(&mut s, day), None);
        }
        // Re-running the same assault on a fresh world: same verdict.
        let mut w2 = DragonWorld::new();
        let id2 = w2.spawn(RegionCoord { x: 0, z: 0 });
        assert_eq!(w2.assault(id2, strength - 10, 7, seed), AssaultOutcome::Slain);

        // A hopeless party is repelled with casualties; dragon grows.
        // (A lone swordsman CAN slay a dragon on a lucky roll — hunt a
        // seed where the die does not smile.)
        let mut w3 = DragonWorld::new();
        let id3 = w3.spawn(RegionCoord { x: 0, z: 0 });
        let weak = 1;
        let mut losing_seed = None;
        for seed in 0..500u64 {
            let mut probe = DragonWorld::new();
            let pid = probe.spawn(RegionCoord { x: 0, z: 0 });
            if matches!(
                probe.assault(pid, weak, 7, seed),
                AssaultOutcome::Repelled { .. }
            ) {
                losing_seed = Some(seed);
                break;
            }
        }
        let seed = losing_seed.expect("a losing seed exists for party 1");
        match w3.assault(id3, weak, 7, seed) {
            AssaultOutcome::Repelled { casualties } => {
                assert!(casualties >= 1);
                assert!(w3.dragons[&id3].power > DRAGON_POWER, "emboldened");
            }
            other => panic!("expected repulse, got {other:?}"),
        }
        assert_eq!(w3.reputation.of(7), 0, "no awe for failure");
    }

    /// A pact wards territory while honored, costs suspicion, and a
    /// breach wakes the dragon enraged with a short raid fuse.
    #[test]
    fn p3d704_pact_wards_then_enrages_on_breach() {
        let mut w = DragonWorld::new();
        let id = w.spawn(RegionCoord { x: 0, z: 0 });
        let lair = RegionCoord { x: 0, z: 0 };
        let mut s = settlements_in_territory(lair);

        // Healthy dragons refuse terms.
        assert!(w.offer_pact(id, 10, 3, 0).is_err());
        // Wound it to ≤30% hp; terms are accepted.
        w.dragons.get_mut(&id).unwrap().hp = DRAGON_HP * 3 / 10;
        assert!(w.offer_pact(id, 25, 3, 100).is_ok());
        assert_eq!(w.reputation.of(3), -SUSPICION_FOR_PACT, "suspicion for pacts");
        // Warded: no raids ever, no growth.
        let snap = (w.dragons[&id].power, s.list[0].aggregate);
        for day in 101..400u64 {
            w.tick_day(&mut s, day);
        }
        assert_eq!(w.dragons[&id].power, snap.0, "warded dragon grows not");
        assert_eq!(s.list[0].aggregate, snap.1, "warded territory is safe");
        assert!(w.scorched.is_empty());

        // Breach: enraged, power spike, raids resume on a short fuse.
        assert!(w.breach_pact(id));
        assert!(w.dragons[&id].awake && w.dragons[&id].pact.is_none());
        assert!(w.dragons[&id].power >= DRAGON_POWER + ENRAGE_POWER);
        let fuse = w.dragons[&id].raid_in;
        assert!(fuse < RAID_COOLDOWN_DAYS, "enraged fuse is short");
        let mut hit = None;
        for day in 0..(fuse as u64 + 2) {
            hit = w.tick_day(&mut s, day);
            if hit.is_some() {
                break;
            }
        }
        assert_eq!(hit, Some(1), "enraged dragon raids again");
        assert!(!w.scorched.is_empty());
    }
}
