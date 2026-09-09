//! P3D-608: the oversight panel — real data, no decoration.
//!
//! Queries the settlement aggregate, garrison, and economy systems to
//! produce a single summary. Every metric reads from the actual
//! underlying state — the panel is a VIEW, not a source of truth.

use crate::coords::RegionCoord;
use crate::economy::EconomicState;
use crate::garrison::Garrison;
use crate::gen::Biome;
use crate::settlement::Aggregate;

/// The aggregated oversight summary for one settlement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OversightSummary {
    pub population: i64,
    pub food: i64,
    pub defense: i64,
    pub prosperity: i64,
    pub garrison_soldiers: u32,
    pub garrison_readiness: u8,
    pub goods: i64,
    /// Composite health 0–100 from real metrics.
    pub health: u8,
}

/// The oversight panel: queries underlying systems.
pub struct OversightPanel;

impl OversightPanel {
    /// Query all metrics from real state. No fake numbers.
    pub fn query(
        agg: &Aggregate,
        garrison: &Garrison,
        economy: &EconomicState,
    ) -> OversightSummary {
        let food_sufficient = if agg.population > 0 {
            economy.food >= agg.population
        } else {
            true
        };
        let defense_ok = garrison.soldiers > 0;
        let prosperity_ok = agg.prosperity > 20;

        // Health composite: food, defense, prosperity each contribute.
        let food_score = if food_sufficient { 40 } else { 0 };
        let defense_score = garrison.readiness() as i64 * 30 / 100;
        let prosperity_score = agg.prosperity * 30 / 100;
        let health =
            (food_score + defense_score as i64 + prosperity_score as i64).clamp(0, 100) as u8;

        OversightSummary {
            population: agg.population,
            food: economy.food,
            defense: garrison.soldiers as i64,
            prosperity: agg.prosperity,
            garrison_soldiers: garrison.soldiers,
            garrison_readiness: garrison.readiness(),
            goods: economy.goods,
            health: health as u8,
        }
    }
}

/// P3D-608 also tracks CONSTRUCTION PROJECTS — civic works that the
/// settlement can commission, with progress tracked over time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    pub name: &'static str,
    pub work_required: u64,
    pub work_done: u64,
    pub completed: bool,
}

impl Project {
    pub fn new(name: &'static str, work_required: u64) -> Self {
        Project {
            name,
            work_required,
            work_done: 0,
            completed: false,
        }
    }

    /// Advance work; returns total done. Completes at >= work_required.
    pub fn advance(&mut self, work: u64) -> u64 {
        self.work_done = (self.work_done + work).min(self.work_required);
        if self.work_done >= self.work_required {
            self.completed = true;
        }
        self.work_done
    }

    /// Progress as a percentage (0–100).
    pub fn progress_pct(&self) -> u8 {
        if self.work_required == 0 {
            return 100;
        }
        ((self.work_done * 100) / self.work_required).min(100) as u8
    }
}

/// Biome health check: is the settlement in a viable biome?
pub fn biome_viability(biome: Biome) -> u8 {
    match biome {
        Biome::Ocean => 0,
        Biome::Coast => 60,
        Biome::Plains => 80,
        Biome::Forest => 75,
        Biome::Wetland => 65,
        Biome::Highlands => 55,
        Biome::Mountains => 40,
        Biome::SnowPeaks => 20,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coords::RegionCoord;
    use crate::gen::WorldGen;

    fn setup() -> (Aggregate, Garrison, EconomicState) {
        let agg = Aggregate {
            population: 20,
            food: 100,
            defense: 20,
            prosperity: 50,
        };
        let mut garrison = Garrison::new(10, 500);
        garrison.recruit(0, 5);
        garrison.supply = 200;
        let econ = EconomicState {
            goods: 50,
            food: 200,
            prosperity: 50,
            population: 20,
        };
        (agg, garrison, econ)
    }

    /// The oversight query reads REAL data: population/food/defense match
    /// the underlying systems exactly.
    #[test]
    fn p3d608_oversight_reads_real_data() {
        let (agg, garrison, econ) = setup();
        let summary = OversightPanel::query(&agg, &garrison, &econ);
        assert_eq!(summary.population, 20);
        assert_eq!(summary.food, 200);
        assert_eq!(summary.garrison_soldiers, 5);
        assert!(summary.garrison_readiness > 0);
        assert!(summary.health > 0);
    }

    /// Zero-state handling: empty garrison, no food, no population —
    /// health is low but doesn't panic.
    #[test]
    fn p3d608_zero_state_handled() {
        let agg = Aggregate::default();
        let garrison = Garrison::new(0, 0);
        let econ = EconomicState {
            goods: 0,
            food: 0,
            prosperity: 0,
            population: 0,
        };
        let summary = OversightPanel::query(&agg, &garrison, &econ);
        assert_eq!(summary.population, 0);
        // Health is 40 because food_sufficient is vacuously true for
        // 0 population and prosperity_score is 0. The composite formula
        // gives food_score=40, defense_score=0, prosperity_score=0.
        assert_eq!(summary.health, 40);
    }

    /// Projects: work advances, completes at threshold, progress is
    /// percentage-bounded.
    #[test]
    fn p3d608_projects_advance_and_complete() {
        let mut p = Project::new("aqueduct", 100);
        assert_eq!(p.progress_pct(), 0);
        p.advance(60);
        assert_eq!(p.progress_pct(), 60);
        assert!(!p.completed);
        p.advance(60); // overshoots: clamps at 100
        assert!(p.completed);
        assert_eq!(p.progress_pct(), 100);
    }

    /// Biome viability: settlements in viable biomes score higher.
    #[test]
    fn p3d608_biome_viability() {
        assert!(biome_viability(Biome::Plains) > biome_viability(Biome::Ocean));
        assert!(biome_viability(Biome::Forest) > biome_viability(Biome::SnowPeaks));
    }
}
