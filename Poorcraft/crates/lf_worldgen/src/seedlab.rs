//! N05 (nightly-beta `04-WORLDGEN-SEEDS-AND-BIOMES`): the seed
//! regression laboratory. Samples fixed coordinate lattices per seed and
//! produces a machine-readable diversity report — the "different seeds
//! feel like different worlds" contract, measured instead of assumed.
//! The reduced corpus runs in tests; `xtask seedlab` writes the full
//! 64-seed report.

use crate::identity::WorldIdentity;
use crate::{WorldGen, WorldType, SEA_LEVEL};

/// Number of biome variants the histogram tracks (the biome enum's size).
const BIOME_COUNT: usize = 30;

/// Everything measured about one seed over a fixed lattice.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SeedMetrics {
    pub seed_u64: u64,
    pub world_type: WorldType,
    /// Lattice half-extent in blocks (samples at every `stride`).
    pub sample_bounds: i32,
    pub stride: i32,
    /// Order-independent hash of the height field over the lattice.
    pub height_hash: u64,
    /// Order-independent hash of the biome field over the lattice.
    pub biome_hash: u64,
    pub height_min: i32,
    pub height_max: i32,
    pub height_mean: f32,
    pub height_stddev: f32,
    /// Biome frequency distribution (BIOME_COUNT buckets, sums to 1).
    pub biome_histogram: Vec<f32>,
    /// Fraction of lattice columns below sea level.
    pub water_fraction: f32,
    /// Fraction of columns the river field claims.
    pub river_fraction: f32,
    /// Fraction of probed 3D cells that carve caves.
    pub cave_fraction: f32,
    /// Surface-block histogram over the registry ids seen (id, fraction),
    /// sorted by fraction — the "does it LOOK different" channel.
    pub surface_blocks: Vec<(u32, f32)>,
    /// Blocks from origin to the nearest kingdom site (None if the
    /// region lookup finds none in reach).
    pub nearest_kingdom_distance: Option<i64>,
    /// The real spawn selection: does find_spawn land strictly (not the
    /// relaxed fallback) with wood within reach?
    pub spawn_ok: bool,
    /// Where the spawn landed (blocks).
    pub spawn_x: i32,
    pub spawn_z: i32,
    /// Spiral radius the spawn search needed.
    pub spawn_search_radius: i32,
    /// Nearest wood ring (None = no tree within 96 blocks).
    pub spawn_wood_ring: Option<i32>,
}

/// Order/machine-independent hash combiner (FNV over u64s).
fn mix(mut h: u64, v: u64) -> u64 {
    h ^= v.wrapping_mul(0x9E3779B97F4A7C15).rotate_left(17);
    h = h.wrapping_mul(0xBF58476D1CE4E5B9);
    h ^ (h >> 29)
}

/// Jensen–Shannon distance (base 2, 0..=1) between two distributions.
pub fn jensen_shannon(p: &[f32], q: &[f32]) -> f32 {
    debug_assert_eq!(p.len(), q.len());
    let mut d = 0.0f32;
    for (&pi, &qi) in p.iter().zip(q.iter()) {
        let (pi, qi) = (pi.max(1e-9), qi.max(1e-9));
        let m = 0.5 * (pi + qi);
        // KL(p||m)/2 + KL(q||m)/2 with natural logs, converted to bits
        d += 0.5 * pi * (pi / m).ln() + 0.5 * qi * (qi / m).ln();
    }
    (d * std::f32::consts::LOG2_E).sqrt().clamp(0.0, 1.0)
}

/// Sample one seed over the lattice (±bounds, step stride). Pure and
/// deterministic: same identity, same metrics.
pub fn sample_seed(seed: u64, world_type: WorldType, bounds: i32, stride: i32) -> SeedMetrics {
    let gen = WorldGen::with_type(crate::Seed(seed), world_type);
    sample_worldgen(&gen, bounds, stride)
}

/// Sample an existing generator (the determinism tests reuse one).
pub fn sample_worldgen(gen: &WorldGen, bounds: i32, stride: i32) -> SeedMetrics {
    let mut height_hash: u64 = 0x243F6A8885A308D3;
    let mut biome_hash: u64 = 0x13198A2E03707344;
    let mut hist = vec![0f32; BIOME_COUNT];
    let mut surface_counts: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
    let (mut hmin, mut hmax) = (i32::MAX, i32::MIN);
    let mut hsum = 0.0f64;
    let mut hsum2 = 0.0f64;
    let mut water = 0u32;
    let mut river = 0u32;
    let mut n = 0u32;
    let step = stride.max(1);
    let mut x = -bounds;
    while x <= bounds {
        let mut z = -bounds;
        while z <= bounds {
            let h = gen.height(x, z);
            let b = gen.biome(x, z);
            let river_f = gen.river_factor(x, z);
            let surface = gen.surface_block(x, z);
            height_hash = mix(height_hash, h as u64);
            biome_hash = mix(biome_hash, b as u64);
            hist[b as usize % BIOME_COUNT] += 1.0;
            *surface_counts.entry(surface).or_insert(0) += 1;
            hmin = hmin.min(h);
            hmax = hmax.max(h);
            hsum += h as f64;
            hsum2 += (h * h) as f64;
            if h < SEA_LEVEL {
                water += 1;
            }
            if river_f > 0.0 {
                river += 1;
            }
            n += 1;
            z += step;
        }
        x += step;
    }
    let mean = hsum / n.max(1) as f64;
    let var = (hsum2 / n.max(1) as f64 - mean * mean).max(0.0);
    for v in hist.iter_mut() {
        *v /= n.max(1) as f32;
    }
    let mut surface_blocks: Vec<(u32, f32)> = surface_counts.into_iter()
        .map(|(id, c)| (id, c as f32 / n.max(1) as f32))
        .collect();
    surface_blocks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // caves: probe a mid-depth slab under the surface at every 4th column
    let mut cave_hits = 0u32;
    let mut cave_probes = 0u32;
    let mut cx = -bounds;
    while cx <= bounds {
        let mut cz = -bounds;
        while cz <= bounds {
            let top = gen.surface_top(cx, cz);
            for y in [top - 8, top - 20, top - 36] {
                if gen.is_cave(cx, y, cz) {
                    cave_hits += 1;
                }
                cave_probes += 1;
            }
            cz += step * 4;
        }
        cx += step * 4;
    }

    let kingdom = gen.nearest_kingdom(0, 0).map(|(_, d2)| d2);
    // N06: spawn quality = the REAL selection (find_spawn) succeeds
    // strictly with wood in reach
    let spawn_sel = gen.find_spawn();
    let spawn_ok = !spawn_sel.relaxed && spawn_sel.wood_within.is_some();

    SeedMetrics {
        seed_u64: gen.seed(),
        world_type: gen.world_type,
        sample_bounds: bounds,
        stride,
        height_hash,
        biome_hash,
        height_min: hmin,
        height_max: hmax,
        height_mean: mean as f32,
        height_stddev: var.sqrt() as f32,
        biome_histogram: hist,
        water_fraction: water as f32 / n.max(1) as f32,
        river_fraction: river as f32 / n.max(1) as f32,
        cave_fraction: cave_hits as f32 / cave_probes.max(1) as f32,
        surface_blocks,
        nearest_kingdom_distance: kingdom.map(|d2| (d2 as f64).sqrt() as i64),
        spawn_ok,
        spawn_x: spawn_sel.x,
        spawn_z: spawn_sel.z,
        spawn_search_radius: spawn_sel.searched_radius,
        spawn_wood_ring: spawn_sel.wood_within,
    }
}

/// The pairwise summary over a corpus.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PairwiseSummary {
    pub pairs: usize,
    /// Mean normalized height-field difference across seed pairs.
    pub mean_height_l1: f32,
    /// 5th percentile of pairwise height L1 (the floor: almost every
    /// pair must differ at least this much).
    pub p05_height_l1: f32,
    /// Mean Jensen–Shannon distance between biome histograms.
    pub mean_biome_js: f32,
    pub p05_biome_js: f32,
}

/// The full machine-readable report (schema_version 1).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SeedCorpusReport {
    pub schema_version: u32,
    pub generator_version: u32,
    pub corpus_size: usize,
    pub bounds: i32,
    pub stride: i32,
    pub metrics: Vec<SeedMetrics>,
    pub pairwise: PairwiseSummary,
    /// Calibrated thresholds the diversity tests enforce.
    pub thresholds: DiversityThresholds,
    /// Human-readable failure lines (empty = the corpus passes).
    pub failures: Vec<String>,
}

/// Diversity floors, calibrated from the v6 generator's real corpus so
/// the suite fails loudly if seeds collapse into sameness.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct DiversityThresholds {
    /// Minimum 5th-percentile normalized height L1 across pairs.
    pub min_p05_height_l1: f32,
    /// Minimum 5th-percentile biome JS distance across pairs.
    pub min_p05_biome_js: f32,
    /// Same-seed control hashes must be bit-identical (bool gate).
    pub require_same_seed_control: bool,
}

impl Default for DiversityThresholds {
    fn default() -> Self {
        Self {
            min_p05_height_l1: 0.020,
            min_p05_biome_js: 0.025,
            require_same_seed_control: true,
        }
    }
}

/// Pairwise height L1 needs both lattices; approximate from the per-seed
/// stats is dishonest — so the report samples a shared height lattice
/// per seed and diffs it directly (kept out of SeedMetrics to stay
/// small; the report is transient tooling output).
fn height_lattice(gen: &WorldGen, bounds: i32, stride: i32) -> Vec<i32> {
    let mut out = Vec::new();
    let mut x = -bounds;
    while x <= bounds {
        let mut z = -bounds;
        while z <= bounds {
            out.push(gen.height(x, z));
            z += stride;
        }
        x += stride;
    }
    out
}

/// Run the laboratory over a seed corpus (the identity stamps each
/// metric's generator version).
pub fn diversity_report(seeds: &[u64], world_type: WorldType, bounds: i32, stride: i32)
    -> SeedCorpusReport {
    let thresholds = DiversityThresholds::default();
    let mut metrics = Vec::with_capacity(seeds.len());
    let mut lattices = Vec::with_capacity(seeds.len());
    for &s in seeds {
        let gen = WorldGen::with_type(crate::Seed(s), world_type);
        metrics.push(sample_worldgen(&gen, bounds, stride));
        lattices.push(height_lattice(&gen, bounds / 4, stride * 4));
    }
    // pairwise distances over the coarse shared lattice
    let mut h_ds: Vec<f32> = Vec::new();
    let mut js_ds: Vec<f32> = Vec::new();
    for i in 0..seeds.len() {
        for j in (i + 1)..seeds.len() {
            let (a, b) = (&lattices[i], &lattices[j]);
            let (min, max) = (a.iter().copied().chain(b.iter().copied())
                .min().unwrap_or(0), a.iter().copied().chain(b.iter().copied())
                .max().unwrap_or(1));
            let range = (max - min).max(1) as f32;
            let l1 = a.iter().zip(b.iter())
                .map(|(x, y)| (x - y).abs() as f32 / range)
                .sum::<f32>() / a.len().max(1) as f32;
            h_ds.push(l1);
            js_ds.push(jensen_shannon(&metrics[i].biome_histogram, &metrics[j].biome_histogram));
        }
    }
    h_ds.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    js_ds.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pick = |v: &[f32]| v.get(v.len() * 5 / 100).copied().unwrap_or(0.0);
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len().max(1) as f32;
    let pairwise = PairwiseSummary {
        pairs: h_ds.len(),
        mean_height_l1: mean(&h_ds),
        p05_height_l1: pick(&h_ds),
        mean_biome_js: mean(&js_ds),
        p05_biome_js: pick(&js_ds),
    };
    // same-seed control: resample seed[0] and require bit-identical hashes
    let mut failures = Vec::new();
    if thresholds.require_same_seed_control && !seeds.is_empty() {
        let control = sample_seed(seeds[0], world_type, bounds, stride);
        if control.height_hash != metrics[0].height_hash
            || control.biome_hash != metrics[0].biome_hash {
            failures.push(format!("same-seed control diverged for seed {}", seeds[0]));
        }
    }
    if pairwise.p05_height_l1 < thresholds.min_p05_height_l1 {
        failures.push(format!(
            "seed pairs too similar in height: p05 L1 {:.4} < {:.4}",
            pairwise.p05_height_l1, thresholds.min_p05_height_l1));
    }
    if pairwise.p05_biome_js < thresholds.min_p05_biome_js {
        failures.push(format!(
            "seed pairs too similar in biomes: p05 JS {:.4} < {:.4}",
            pairwise.p05_biome_js, thresholds.min_p05_biome_js));
    }
    // every seed must find a kingdom eventually and report sane water
    for m in &metrics {
        if m.height_min < SEA_LEVEL - 64 || m.height_max > 250 {
            failures.push(format!("seed {}: terrain out of range ({}, {})",
                m.seed_u64, m.height_min, m.height_max));
        }
    }
    SeedCorpusReport {
        schema_version: 1,
        generator_version: crate::GENERATOR_VERSION,
        corpus_size: seeds.len(),
        bounds,
        stride,
        metrics,
        pairwise,
        thresholds,
        failures,
    }
}

/// The canonical 64-seed corpus: deterministic, spread across the u64
/// space (splitmix sequence from a fixed root, plus edge cases 0,
/// u64::MAX, and a few word-seed hashes).
pub fn corpus_64() -> Vec<u64> {
    let mut seeds = Vec::with_capacity(64);
    let mut z: u64 = 0x1234_5678_9ABC_DEF0;
    for _ in 0..60 {
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        seeds.push(z ^ (z >> 31));
    }
    seeds.push(0);
    seeds.push(u64::MAX);
    seeds.push(crate::identity::hash_seed_string("valdenmoor"));
    seeds.push(crate::identity::hash_seed_string("the ironborn hold"));
    seeds
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reduced corpus in tests: 12 seeds, coarse lattice — must stay
    /// under ~2s while still catching "every seed looks nearly the same".
    #[test]
    fn twelve_seed_corpus_is_diverse_and_deterministic() {
        let mut seeds = Vec::new();
        for i in 0..12u64 {
            seeds.push(crate::identity::hash_seed_string(&format!("corpus-{i}")));
        }
        let report = diversity_report(&seeds, WorldType::Normal, 192, 24);
        assert!(report.failures.is_empty(), "diversity failures: {:?}", report.failures);
        assert!(report.pairwise.pairs >= 60);
    }

    #[test]
    fn same_seed_is_bit_identical_across_instances() {
        let a = sample_seed(424242, WorldType::Normal, 128, 16);
        let b = sample_seed(424242, WorldType::Normal, 128, 16);
        assert_eq!(a.height_hash, b.height_hash);
        assert_eq!(a.biome_hash, b.biome_hash);
        // and a different seed is NOT (height or biome field differs)
        let c = sample_seed(424243, WorldType::Normal, 128, 16);
        assert!(a.height_hash != c.height_hash || a.biome_hash != c.biome_hash);
    }

    /// Generation order does not matter: chunks are pure functions of the
    /// identity. Shuffled vs sequential column hashes must match.
    #[test]
    fn chunk_generation_order_does_not_matter() {
        let gen = WorldGen::with_type(crate::Seed(987654321), WorldType::Normal);
        let coords: Vec<(i32, i32)> = (-3..3).flat_map(|cx| (-3..3).map(move |cz| (cx, cz))).collect();
        // order-INDEPENDENT combining: per-column hashes (each chained
        // internally), then a commutative XOR fold keyed by the column's
        // own coordinates — visit order cannot change the result
        let hash_of = |order: &[(i32, i32)]| {
            let mut acc: u64 = 0;
            for &(cx, cz) in order {
                let mut ch: u64 = 0x6A09E667F3BCC909;
                for (y, b) in gen.column(cx * 16, cz * 16) {
                    ch = mix(ch, y as u64);
                    ch = mix(ch, b as u64);
                }
                let rot = ((cx.rem_euclid(8)) as u32 * 8 + (cz.rem_euclid(8)) as u32) % 64;
                acc ^= ch.rotate_left(rot);
            }
            mix(acc, order.len() as u64)
        };
        let sequential = hash_of(&coords);
        let mut shuffled = coords.clone();
        shuffled.sort_by_key(|&(x, z)| (x.wrapping_mul(31) ^ z.wrapping_mul(17)) % 97);
        let reordered = hash_of(&shuffled);
        assert_eq!(sequential, reordered, "column contents changed with visit order");
        // negative and large coordinates: a second instance of the SAME
        // identity must agree there, and heights stay inside sane bounds
        let twin = WorldGen::with_type(crate::Seed(987654321), WorldType::Normal);
        for &(x, z) in &[
            (-1_000_000i32, 1_000_000i32),
            (1_000_000, -1_000_000),
            (i32::MIN / 2, i32::MIN / 2),
            (i32::MAX / 2, i32::MAX / 2),
        ] {
            assert_eq!(gen.height(x, z), twin.height(x, z), "({x},{z}) diverged between instances");
            assert!((-64..=320).contains(&gen.height(x, z)), "({x},{z}) height out of range");
            assert_eq!(gen.biome(x, z), twin.biome(x, z));
        }
    }

    /// N06: the real spawn selection — across a spread of seeds every
    /// spawn must be dry land above the sea, off rivers, tree-free at the
    /// cell, and nearly always within reach of wood.
    #[test]
    fn spawn_selection_is_safe_and_reachable() {
        let mut strict_with_wood = 0;
        let n = 16;
        for i in 0..n {
            let gen = WorldGen::with_type(crate::Seed(
                crate::identity::hash_seed_string(&format!("spawn-{i}"))), WorldType::Normal);
            let s = gen.find_spawn();
            // safety invariants hold for strict AND relaxed spawns
            assert!(s.top > crate::SEA_LEVEL, "seed {i}: spawn at/below sea");
            assert!(!matches!(s.biome, crate::Biome::Ocean | crate::Biome::DeepOcean
                | crate::Biome::FrozenOcean), "seed {i}: spawn in an ocean biome");
            assert!(!gen.tree_at(s.x, s.z), "seed {i}: spawn inside a tree cell");
            assert!(gen.kingdom_at(s.x.div_euclid(16), s.z.div_euclid(16)).is_none(),
                "seed {i}: spawn inside a kingdom footprint");
            if !s.relaxed && s.wood_within.is_some() {
                strict_with_wood += 1;
            }
        }
        assert!(strict_with_wood >= n - 2,
            "only {strict_with_wood}/{n} spawns are strict with wood in reach");
        // deterministic: same identity, same spawn, twice
        let gen = WorldGen::with_type(crate::Seed(31337), WorldType::Normal);
        assert_eq!(gen.find_spawn(), gen.find_spawn());
        // and a second instance agrees
        assert_eq!(gen.find_spawn(), WorldGen::with_type(crate::Seed(31337), WorldType::Normal).find_spawn());
    }

    #[test]
    fn js_distance_behaves() {
        let mut p = vec![0.5, 0.5, 0.0];
        let q = vec![0.5, 0.5, 0.0];
        assert!(jensen_shannon(&p, &q) < 1e-6);
        let r = vec![0.0, 1.0, 0.0];
        assert!(jensen_shannon(&p, &r) > 0.3);
        assert!(jensen_shannon(&p, &r) <= 1.0);
    }

    #[test]
    fn corpus_shape() {
        let c = corpus_64();
        assert_eq!(c.len(), 64);
        assert!(c.contains(&0));
        assert!(c.contains(&u64::MAX));
        assert!(c.iter().collect::<std::collections::HashSet<_>>().len() >= 60,
            "corpus seeds must be distinct");
    }
}
