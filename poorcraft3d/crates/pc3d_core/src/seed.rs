//! P3D-003: deterministic seed streams.
//!
//! One world seed in, independent per-subsystem streams out — terrain,
//! water, sites, NPCs, machines each get their own reproducible RNG, and no
//! subsystem's consumption can shift another's values (independence comes
//! from the LABEL, not from call order). Derivation is FNV-1a over
//! (seed, label); the stream RNG is SplitMix64.

use crate::journal::fnv1a64;

/// SplitMix64: tiny, fast, fully deterministic, and good enough as a
/// stream mixin for world content (cryptographic strength is not a game
/// requirement; reproducibility is).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(state: u64) -> Self {
        SplitMix64 { state }
    }
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, n) for n > 0 (rejection-free modulo; the 2^64 skew is
    /// negligible for game content and this keeps streams position-stable).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n.max(1)
    }
    /// Uniform [0, 1) as f32 (24 explicit mantissa bits).
    pub fn unit_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// The per-world stream table. Labels are the contract: a subsystem's
/// stream is addressed by name, so adding a new consumer never renumbers
/// the existing ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeedStreams {
    world_seed: u64,
}

/// Well-known labels (defined once; typos are compile errors, not worlds).
pub mod stream {
    pub const TERRAIN: &str = "terrain";
    pub const WATER: &str = "water";
    pub const SITES: &str = "sites";
    pub const NPCS: &str = "npcs";
    pub const MACHINES: &str = "machines";
    pub const WEATHER: &str = "weather";
}

impl SeedStreams {
    pub fn new(world_seed: u64) -> Self {
        SeedStreams { world_seed }
    }

    /// The raw stream seed for a label: FNV-1a over seed bytes || label.
    pub fn stream_seed(&self, label: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in self.world_seed.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h ^= fnv1a64(label.as_bytes());
        h = h.wrapping_mul(0x100000001b3);
        h
    }

    /// An independent RNG for one subsystem.
    pub fn rng(&self, label: &str) -> SplitMix64 {
        SplitMix64::new(self.stream_seed(label))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stream::*;

    /// Same seed + labels → same values, forever and everywhere.
    #[test]
    fn p3d003_streams_are_reproducible() {
        let a = SeedStreams::new(0xDEAD_BEEF);
        let b = SeedStreams::new(0xDEAD_BEEF);
        let mut ra = a.rng(TERRAIN);
        let mut rb = b.rng(TERRAIN);
        for _ in 0..8 {
            assert_eq!(ra.next_u64(), rb.next_u64(), "same stream must advance identically");
        }
        // A cloned stream is an exact snapshot of that moment.
        let mut snapshot = ra.clone();
        assert_eq!(snapshot.next_u64(), ra.next_u64());
        // f32 helpers are stable and in range.
        let mut r = a.rng(WATER);
        for _ in 0..100 {
            let u = r.unit_f32();
            assert!((0.0..1.0).contains(&u));
        }
    }

    /// Different labels are independent; different seeds diverge immediately.
    #[test]
    fn p3d003_streams_are_independent_and_seed_sensitive() {
        let s = SeedStreams::new(42);
        let labels = [TERRAIN, WATER, SITES, NPCS, MACHINES, WEATHER];
        let mut firsts = Vec::new();
        for l in labels {
            let v = s.rng(l).next_u64();
            assert!(!firsts.contains(&v), "label {l} collided with an earlier stream");
            firsts.push(v);
        }
        // Same label, different seed → different stream.
        assert_ne!(s.rng(TERRAIN).next_u64(), SeedStreams::new(43).rng(TERRAIN).next_u64());
        // Stream seeds themselves are stable for persistence.
        assert_eq!(s.stream_seed(SITES), s.stream_seed(SITES));
    }

    /// below(n) stays in range and SplitMix64's sequence is well-mixed.
    #[test]
    fn p3d003_below_and_mix_are_sane() {
        let mut r = SplitMix64::new(7);
        let mut low = 0;
        let mut high = 0;
        for _ in 0..200 {
            if r.below(2) == 0 {
                low += 1;
            } else {
                high += 1;
            }
        }
        assert!(low > 40 && high > 40, "below(2) collapsed: {low} vs {high}");
        assert_eq!(SplitMix64::new(0).below(0), 0, "below(0) is defined as 0, never panics");
        let x = SplitMix64::new(1).next_u64();
        let y = SplitMix64::new(2).next_u64();
        assert_ne!(x, y);
    }
}
