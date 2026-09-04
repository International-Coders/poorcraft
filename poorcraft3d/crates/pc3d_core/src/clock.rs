//! P3D-003: the deterministic fixed-step clock.
//!
//! Real frames arrive at whatever rate the hardware likes; the simulation
//! advances in whole 60 Hz ticks and only in whole ticks. `advance` returns
//! the INCLUSIVE RANGE of tick numbers to execute — looping over the range
//! (never over a bare count) is what keeps a multi-tick frame honest, so
//! each fired tick runs under its own number.

/// Nominal simulation rate.
pub const SIM_HZ: u32 = 60;

/// One sim tick in whole microseconds (1_000_000 / 60, truncated). Integer
/// accumulation keeps the clock deterministic; the truncated-rate offset is
/// a few parts per million and irrelevant to play.
pub const TICK_US: u64 = 1_000_000 / SIM_HZ as u64;

/// Backlog cap: the most catch-up ticks one `advance` may fire before the
/// remainder is shed. Shedding keeps the sim live through hitches at the
/// cost of sim/wall skew — a deliberate, deterministic policy.
pub const MAX_CATCHUP_TICKS: u32 = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedClock {
    accumulated_us: u64,
    /// Total ticks fired since construction — the simulation's clock.
    pub tick: u64,
}

impl Default for FixedClock {
    fn default() -> Self {
        Self::new()
    }
}

impl FixedClock {
    pub fn new() -> Self {
        FixedClock { accumulated_us: 0, tick: 0 }
    }

    /// Feed real elapsed seconds; get the INCLUSIVE range of ticks that must
    /// execute NOW, in order (empty when none fire).
    pub fn advance(&mut self, real_dt: f32) -> std::ops::Range<u64> {
        let add_us = if real_dt.is_finite() && real_dt > 0.0 {
            (real_dt * 1_000_000.0) as u64
        } else {
            0
        };
        self.accumulated_us = self.accumulated_us.saturating_add(add_us);
        let first = self.tick + 1;
        let mut fired: u32 = 0;
        while self.accumulated_us >= TICK_US && fired < MAX_CATCHUP_TICKS {
            self.accumulated_us -= TICK_US;
            fired += 1;
        }
        if fired == MAX_CATCHUP_TICKS {
            // Shed the backlog rather than banking an unbounded debt.
            self.accumulated_us = 0;
        }
        self.tick += fired as u64;
        first..first + fired as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Same stream, same ticks — bit for bit.
    #[test]
    fn p3d003_clock_replays_identically_and_tracks_wall_time() {
        let frames = 600;
        let dts: Vec<f32> = (0..frames)
            .map(|i| {
                // Deterministic jitter between ~20 and ~90 fps.
                let h = (i as u64).wrapping_mul(0x9E3779B97F4A7C15);
                let r = ((h ^ (h >> 30)) % 1000) as f32 / 1000.0;
                0.011 + r * 0.038
            })
            .collect();
        let expected = (dts.iter().sum::<f32>() * SIM_HZ as f32) as u64;

        let mut a = FixedClock::new();
        let mut b = FixedClock::new();
        for dt in &dts {
            a.advance(*dt);
            b.advance(*dt);
        }
        assert_eq!(a.tick, b.tick, "same stream must replay identically");
        assert!(
            a.tick.abs_diff(expected) <= 2,
            "clock drifted {} ticks from wall time",
            a.tick.abs_diff(expected)
        );
    }

    /// A one-second freeze fires exactly the cap and sheds the rest; bad
    /// input is inert; ticks carry their own numbers.
    #[test]
    fn p3d003_overload_sheds_deterministically() {
        let mut clock = FixedClock::new();
        let fired = clock.advance(1.0);
        assert_eq!(fired, 1..1 + MAX_CATCHUP_TICKS as u64);
        assert_eq!(clock.accumulated_us, 0, "backlog must be shed, not banked");

        let mut a = FixedClock::new();
        let mut b = FixedClock::new();
        for _ in 0..10 {
            a.advance(1.0);
            b.advance(1.0);
        }
        assert_eq!(a.tick, b.tick);
        assert_eq!(a.tick, 10 * MAX_CATCHUP_TICKS as u64);

        let mut c = FixedClock::new();
        assert!(c.advance(f32::NAN).is_empty());
        assert!(c.advance(-0.5).is_empty());
        assert!(c.advance(0.0).is_empty());
    }

    /// Multi-tick frames fire a RANGE: every tick inside a frame carries its
    /// own number — this is the trap the count-returning API set.
    #[test]
    fn p3d003_multi_tick_frames_carry_tick_numbers() {
        let mut clock = FixedClock::new();
        let seen: Vec<u64> = clock.advance(3.0 / 60.0).collect();
        assert_eq!(seen, vec![1, 2, 3], "each fired tick must report its own number");
        assert_eq!(clock.tick, 3);
        // A second frame continues the numbering without gaps.
        let next: Vec<u64> = clock.advance(3.0 / 60.0).collect();
        assert_eq!(next, vec![4, 5, 6]);
    }
}
