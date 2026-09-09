//! P3D-604: production, trade, needs, and economic effects.
//!
//! The economic engine: workshops produce goods deterministically,
//! settlements trade surplus for deficit, population consumes, and
//! prosperity tracks economic health. All integer arithmetic, all pure.

/// Production from workshops: each workshop produces a fixed output per
/// day. Pure integer arithmetic.
pub fn produce(workshops: u32, output_per_workshop: i64) -> i64 {
    (workshops as i64).max(0) * output_per_workshop.max(0)
}

/// Food consumed by population per day.
pub fn consume_food(population: i64) -> i64 {
    population.max(0)
}

/// A trade route sending goods from one settlement to another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TradeRoute {
    pub from_id: u64,
    pub to_id: u64,
    pub goods_per_day: u64,
}

/// Execute a trade: source loses goods, target gains goods.
/// Returns the actual amount transferred (bounded by source's stock).
pub fn execute_trade(source: &mut i64, target: &mut i64, amount: u64) -> u64 {
    let avail = (*source).max(0) as u64;
    let sent = avail.min(amount);
    *source -= sent as i64;
    *target += sent as i64;
    sent
}

/// The economic state of one settlement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EconomicState {
    pub goods: i64,
    pub food: i64,
    pub prosperity: i64,
    pub population: i64,
}

impl EconomicState {
    pub fn new(population: i64, food: i64) -> Self {
        EconomicState {
            goods: 0,
            food,
            prosperity: 50,
            population,
        }
    }

    /// Simulate one economic day: production adds goods, consumption
    /// drains food, prosperity tracks the surplus/deficit.
    pub fn simulate_day(&mut self, workshops: u32, output_per_workshop: i64) {
        let produced = produce(workshops, output_per_workshop);
        self.goods += produced;
        let consumed = consume_food(self.population);
        self.food -= consumed;
        if self.food < 0 {
            // Starvation: population shrinks, prosperity drops.
            self.population = (self.population - 1).max(0);
            self.prosperity = (self.prosperity - 5).max(0);
            self.food = 0;
        } else if self.food > self.population * 3 {
            // Abundance: prosperity grows.
            self.prosperity = (self.prosperity + 2).min(100);
        }
        // Trade goods boost prosperity marginally.
        if self.goods > 50 {
            self.prosperity = (self.prosperity + 1).min(100);
        }
    }

    /// Execute a trade with another settlement.
    pub fn trade(&mut self, other: &mut EconomicState, goods: u64) {
        execute_trade(&mut self.goods, &mut other.goods, goods);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Production is deterministic: same workshops → same output.
    #[test]
    fn p3d604_production_is_deterministic() {
        assert_eq!(produce(3, 10), 30);
        assert_eq!(produce(0, 10), 0);
        assert_eq!(produce(5, 0), 0);
    }

    /// Consumption: population eats food per day.
    #[test]
    fn p3d604_consumption_drains_food() {
        assert_eq!(consume_food(10), 10);
        assert_eq!(consume_food(0), 0);
    }

    /// Trade: goods transfer from source to target, bounded by source.
    #[test]
    fn p3d604_trade_transfers_goods() {
        let mut src = 100i64;
        let mut dst = 0i64;
        let sent = execute_trade(&mut src, &mut dst, 60);
        assert_eq!(sent, 60);
        assert_eq!(src, 40);
        assert_eq!(dst, 60);
        // Overflow is bounded.
        let sent2 = execute_trade(&mut src, &mut dst, 100);
        assert_eq!(sent2, 40);
        assert_eq!(src, 0);
        assert_eq!(dst, 100);
    }

    /// THE economic loop: production adds goods, consumption drains food,
    /// prosperity tracks health, trade balances deficits.
    #[test]
    fn p3d604_economic_loop_tracks_prosperity() {
        let mut state = EconomicState::new(10, 200);
        let initial_pop = state.population;
        // Simulate 5 days with 3 workshops.
        for _ in 0..5 {
            state.simulate_day(3, 10);
        }
        assert!(state.goods > 0, "workshops must produce goods");
        assert_eq!(
            state.population, initial_pop,
            "no starvation with surplus food"
        );
        assert!(
            state.prosperity >= 50,
            "prosperity should grow with surplus"
        );
    }

    /// Starvation: food runs out, population shrinks, prosperity drops.
    #[test]
    fn p3d604_starvation_shrinks_population() {
        let mut state = EconomicState::new(10, 0);
        state.simulate_day(0, 0);
        assert_eq!(state.population, 9, "starvation shrinks population");
        assert_eq!(state.prosperity, 45, "prosperity drops");
    }

    /// Trade between two settlements balances goods.
    #[test]
    fn p3d604_trade_between_settlements() {
        let mut producer = EconomicState::new(10, 200);
        let mut consumer = EconomicState::new(10, 10);
        producer.goods = 100;
        producer.trade(&mut consumer, 50);
        assert_eq!(producer.goods, 50);
        assert_eq!(consumer.goods, 50);
    }
}
