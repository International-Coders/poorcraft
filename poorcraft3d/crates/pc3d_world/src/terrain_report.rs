//! The terrain analysis authority (pure): per-biome census with slope
//! and roughness over a window of the world, plus a CPU relief map.
//! Same numbers everywhere — the tool, the tests, and any future
//! balance dashboard all read this one function.

use crate::coords::RegionCoord;
use crate::gen::{Biome, WorldGen};

/// One biome's terrain profile over the sampled window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiomeProfile {
    pub biome: Biome,
    pub regions: u32,
    /// Mean elevation in meters.
    pub mean_elevation_m: f32,
    /// Mean steepest-neighbor slope in percent (100 = 45 degrees).
    pub mean_slope_pct: f32,
    /// Slope standard deviation — roughness (terraced vs rolling).
    pub roughness_pct: f32,
}

/// The whole report over a (2*half)^ region window.
#[derive(Clone, Debug, PartialEq)]
pub struct TerrainReport {
    pub profiles: Vec<BiomeProfile>,
    /// The full height grid sampled (row-major from -half,-half), for
    /// relief-map painting by the presentation layer.
    pub heights_m: Vec<f32>,
    pub side: usize,
}

fn slope_pct(gen: &WorldGen, x: i64, z: i64) -> f32 {
    let h = gen.effective_surface_mm(x, z);
    let e = gen.effective_surface_mm(x + 256_000, z);
    let w = gen.effective_surface_mm(x - 256_000, z);
    let n = gen.effective_surface_mm(x, z + 256_000);
    let s = gen.effective_surface_mm(x, z - 256_000);
    let dx = (e - w).abs() as f32 / 1000.0;
    let dz = (n - s).abs() as f32 / 1000.0;
    // Neighbors 256 m apart: slope = rise/run as percent.
    (dx.hypot(dz) / 512.0) * 100.0
}

/// Analyzes the terrain over a square window of regions centered on
/// the world origin, sampling each region's center.
pub fn terrain_report(gen: &WorldGen, half: i32) -> TerrainReport {
    let side = (half * 2) as usize;
    let mut acc: std::collections::BTreeMap<Biome, (u32, f64, f64, f64)> = Default::default();
    let mut heights = Vec::with_capacity(side * side);
    for rz in -half..half {
        for rx in -half..half {
            let region = RegionCoord { x: rx, z: rz };
            let biome = gen.biome(region);
            let o = region.origin();
            let x = o.x + 128_000; // region center in mm
            let z = o.z + 128_000;
            let h = gen.effective_surface_mm(x, z) as f32 / 1000.0;
            let slope = slope_pct(gen, x, z);
            heights.push(h);
            let e = acc.entry(biome).or_insert((0, 0.0, 0.0, 0.0));
            e.0 += 1;
            e.1 += h as f64;
            e.2 += slope as f64;
            e.3 += (slope * slope) as f64;
        }
    }
    let profiles = acc
        .into_iter()
        .map(|(biome, (n, h_sum, s_sum, sq_sum))| {
            let mean_h = h_sum / n as f64;
            let mean_s = s_sum / n as f64;
            let var = (sq_sum / n as f64 - mean_s * mean_s).max(0.0);
            BiomeProfile {
                biome,
                regions: n,
                mean_elevation_m: mean_h as f32,
                mean_slope_pct: mean_s as f32,
                roughness_pct: var.sqrt() as f32,
            }
        })
        .collect();
    TerrainReport { profiles, heights_m: heights, side }
}

impl TerrainReport {
    /// The report's data rows (the JSON sidecar is built by the app —
    /// pc3d_world stays serde-free by law).
    pub fn rows(&self) -> impl Iterator<Item = (&'static str, u32, f32, f32, f32)> + '_ {
        self.profiles.iter().map(|p| {
            (
                p.biome.name(),
                p.regions,
                (p.mean_elevation_m * 10.0).round() / 10.0,
                (p.mean_slope_pct * 10.0).round() / 10.0,
                (p.roughness_pct * 10.0).round() / 10.0,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_is_deterministic_and_covers_every_sampled_region() {
        let gen = WorldGen::new(4242);
        let a = terrain_report(&gen, 8);
        let b = terrain_report(&gen, 8);
        assert_eq!(a, b, "same seed + window: identical report");
        assert_eq!(a.heights_m.len(), 256);
        let total: u32 = a.profiles.iter().map(|p| p.regions).sum();
        assert_eq!(total, 256, "every sampled region is profiled");
    }

    #[test]
    fn biome_census_agrees_with_the_generator() {
        let gen = WorldGen::new(4242);
        let report = terrain_report(&gen, 8);
        let mut want: std::collections::BTreeMap<Biome, u32> = Default::default();
        for rz in -8..8 {
            for rx in -8..8 {
                *want.entry(gen.biome(RegionCoord { x: rx, z: rz })).or_insert(0) += 1;
            }
        }
        for p in &report.profiles {
            assert_eq!(p.regions, want[&p.biome], "{} census", p.biome.name());
        }
    }

    #[test]
    fn slopes_and_elevations_are_physical() {
        let gen = WorldGen::new(7);
        let report = terrain_report(&gen, 6);
        for p in &report.profiles {
            assert!(p.mean_slope_pct >= 0.0 && p.mean_slope_pct < 200.0);
            assert!(p.roughness_pct >= 0.0 && p.roughness_pct < 200.0);
            // Ocean sits below sea level; land biomes average above
            // it EXCEPT Coast (the beach band straddles 0 by design —
            // the biome gate is elevation <= SEA+2, so a mean slightly
            // under 0 is physical shoreline, not a bug).
            if p.biome == crate::gen::Biome::Ocean {
                assert!(p.mean_elevation_m < 0.5, "ocean mean {}", p.mean_elevation_m);
            } else if p.biome != crate::gen::Biome::Coast {
                assert!(p.mean_elevation_m >= 0.0, "{} mean", p.biome.name());
            }
        }
        // Mountains are steeper than plains on this generator.
        let find = |b: Biome| report.profiles.iter().find(|p| p.biome == b);
        if let (Some(plains), Some(mtn)) = (find(Biome::Plains), find(Biome::Mountains)) {
            assert!(
                mtn.mean_slope_pct > plains.mean_slope_pct,
                "mountains {}% steeper than plains {}% — the tool must see real relief",
                mtn.mean_slope_pct, plains.mean_slope_pct
            );
        }
    }
}
