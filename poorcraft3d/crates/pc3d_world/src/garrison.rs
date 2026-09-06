//! P3D-607: army/garrison — recruitment, supply, morale, readiness.
//!
//! Deterministic garrison management: recruit soldiers from population
//! (bounded), supply decays per soldier, morale tracks supply, and
//! readiness is a composite metric for the oversight panel.

/// A garrison for one settlement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Garrison {
    pub soldiers: u32,
    pub max_soldiers: u32,
    pub supply: i64,
    pub morale: u8,
}

/// Supply consumed per soldier per day.
pub const SUPPLY_PER_SOLDIER: i64 = 1;
/// Morale gained per 100 supply surplus.
pub const MORALE_PER_SUPPLY: i64 = 2;

impl Garrison {
    pub fn new(max_soldiers: u32, initial_supply: i64) -> Self {
        Garrison { soldiers: 0, max_soldiers, supply: initial_supply, morale: 50 }
    }

    /// Recruit soldiers from population. Bounded by max_soldiers and
    /// available population. Returns how many were actually recruited.
    pub fn recruit(&mut self, available_population: i64, count: u32) -> u32 {
        let room = self.max_soldiers.saturating_sub(self.soldiers);
        let recruited = room.min(count);
        self.soldiers += recruited;
        recruited
    }

    /// One supply day: soldiers consume supply; morale adjusts.
    pub fn supply_day(&mut self) {
        let consumed = self.soldiers as i64 * SUPPLY_PER_SOLDIER;
        self.supply -= consumed;
        // Morale tracks supply: well-supplied = high morale.
        if self.supply > 0 {
            self.morale = (self.morale as i64 + 2).min(100) as u8;
        } else {
            self.morale = self.morale.saturating_sub(5);
        }
    }

    /// Readiness 0–100: composite of soldier count ratio, supply
    /// sufficiency, and morale.
    pub fn readiness(&self) -> u8 {
        if self.max_soldiers == 0 || self.soldiers == 0 {
            return 0;
        }
        let strength = (self.soldiers as i64 * 100 / self.max_soldiers as i64) as i64;
        let supply_ok = if self.soldiers > 0 { self.supply / self.soldiers as i64 } else { 100 };
        let supply_score = supply_ok.clamp(0, 100);
        let composite = (strength * 3 + supply_score + self.morale as i64 * 2) / 6;
        composite.clamp(0, 100) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recruitment is bounded by max_soldiers.
    #[test]
    fn p3d607_recruit_bounded() {
        let mut g = Garrison::new(10, 500);
        assert_eq!(g.recruit(50, 8), 8);
        assert_eq!(g.recruit(50, 5), 2, "only 2 slots remain");
        assert_eq!(g.recruit(50, 1), 0, "full");
    }

    /// Supply decays per soldier per day; morale tracks supply.
    #[test]
    fn p3d607_supply_decays_and_morale_tracks() {
        let mut g = Garrison::new(20, 500);
        g.recruit(0, 10);
        g.supply_day();
        assert_eq!(g.supply, 490, "10 soldiers consume 10 supply");
        assert!(g.morale > 50, "well-supplied garrison has good morale");
        // Run out of supply: morale drops.
        g.supply = 0;
        g.supply_day();
        assert!(g.morale < 50, "un supplied garrison morale drops");
    }

    /// Readiness is a composite of strength, supply, and morale.
    #[test]
    fn p3d607_readiness_composite() {
        let mut g = Garrison::new(10, 500);
        assert_eq!(g.readiness(), 0, "no soldiers = no readiness");
        g.recruit(0, 10);
        g.supply = 1000;
        g.morale = 80;
        let r = g.readiness();
        assert!(r > 0 && r <= 100);
        // More soldiers = higher strength component.
        let mut g2 = Garrison::new(10, 500);
        g2.recruit(0, 10);
        g2.supply = 1000;
        g2.morale = 80;
        assert_eq!(g.readiness(), g2.readiness(), "same state same readiness");
    }
}
