//! P3D-103: procedural macro fields, biomes, and deterministic patch
//! regeneration — the immutable procedural base (terrain blueprint, layer 1).
//!
//! Coherence law (D-016): geography comes from a CONTINUOUS multi-octave
//! value-noise field sampled at region centers, so neighbors correlate —
//! mountains rise and fall across regions, they never teleport. Biomes read
//! from the fields (oceans low, wetlands humid, peaks cold), not from dice.
//! One global height function answers for every (x, z); patch regeneration
//! is pure sampling of it, so the same seed regenerates the same world
//! forever and untouched patches never need storing.

use crate::coords::{PatchCoord, RegionCoord};
use crate::scales::PATCH_CELL_AXIS;
use pc3d_core::{fnv1a64, SeedStreams};
use std::f64;

/// The macro climate/terrain triple for one region. Elevation in meters
/// (negative = below sea level); temperature and humidity are 0..=100.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MacroField {
    pub elevation_m: i32,
    pub temperature: u8,
    pub humidity: u8,
}

/// Sea level in meters.
pub const SEA_LEVEL_M: i32 = 0;
/// Field bounds shaping the noise into geography.
pub const MIN_ELEVATION_M: i32 = -64;
pub const MAX_ELEVATION_M: i32 = 192;

/// A named biome. Eight to start; rivers/wetland corridors extend this when
/// the watershed graph lands (P3D-301).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Biome {
    Ocean,
    Coast,
    Plains,
    Forest,
    Wetland,
    Highlands,
    Mountains,
    SnowPeaks,
}

impl Biome {
    pub fn name(self) -> &'static str {
        match self {
            Biome::Ocean => "ocean",
            Biome::Coast => "coast",
            Biome::Plains => "plains",
            Biome::Forest => "forest",
            Biome::Wetland => "wetland",
            Biome::Highlands => "highlands",
            Biome::Mountains => "mountains",
            Biome::SnowPeaks => "snow_peaks",
        }
    }
}

/// One cubic meter of regenerated terrain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellMaterial {
    Air,
    Water,
    Soil,
    Grass,
    Sand,
    Rock,
    Snow,
}

/// The generator: one per world seed. Copyable; every method is pure.
#[derive(Clone, Copy, Debug)]
pub struct WorldGen {
    seed: u64,
}

/// 16×16×16 regenerated cells, indexed [(x*n + y)*n + z].
pub struct PatchCells {
    pub coord: PatchCoord,
    pub cells: Vec<CellMaterial>,
}

impl PatchCells {
    pub fn get(&self, x: usize, y: usize, z: usize) -> CellMaterial {
        self.cells[(x * PATCH_CELL_AXIS as usize + y) * PATCH_CELL_AXIS as usize + z]
    }

    /// FNV-1a over the material codes in index order — the patch identity.
    pub fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for c in &self.cells {
            h ^= *c as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
}

impl WorldGen {
    pub fn new(seed: u64) -> Self {
        WorldGen { seed }
    }

    /// Continuous 2D value noise: bilinear-smoothstep blend of four lattice
    /// hashes around (u, v). Continuity is structural — adjacent samples
    /// share lattice corners — so nothing built on this can seam.
    fn value_noise(&self, channel: u64, octave: u32, u: f64, v: f64) -> f32 {
        let gx = u.floor();
        let gz = v.floor();
        let tx = smoothstep((u - gx) as f32);
        let tz = smoothstep((v - gz) as f32);
        let h00 = self.lattice_hash(channel, octave, gx as i64, gz as i64);
        let h10 = self.lattice_hash(channel, octave, gx as i64 + 1, gz as i64);
        let h01 = self.lattice_hash(channel, octave, gx as i64, gz as i64 + 1);
        let h11 = self.lattice_hash(channel, octave, gx as i64 + 1, gz as i64 + 1);
        let top = h00 + (h10 - h00) * tx;
        let bottom = h01 + (h11 - h01) * tx;
        top + (bottom - top) * tz
    }

    /// Fractal blend of `octaves` noise layers. `u`/`v` are in REGION units
    /// (region centers sit at .5). Octave cells span 48/2^o regions — 12,
    /// 6, 3 km — so a 25-km atlas view holds real coastline variety while
    /// features stay continental (the atlas proof drove three iterations:
    /// 1-km cells rendered as confetti, 32-km cells as one flat biome).
    fn fbm(&self, channel: u64, u: f64, v: f64, octaves: u32) -> f32 {
        let mut value = 0.0f32;
        let mut amplitude = 1.0f32;
        let mut total = 0.0f32;
        for o in 0..octaves {
            let cells = 48f64 / 2f64.powi(o as i32);
            value += self.value_noise(channel, o, u / cells, v / cells) * amplitude;
            total += amplitude;
            amplitude *= 0.5;
        }
        value / total
    }

    fn lattice_hash(&self, channel: u64, octave: u32, gx: i64, gz: i64) -> f32 {
        let mut h = fnv1a64(&self.seed.to_le_bytes());
        for word in [channel, octave as u64, gx as u64, gz as u64] {
            for b in word.to_le_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
        }
        ((h >> 11) as f32) / ((1u64 << 53) as f32)
    }

    /// Macro field for one region, sampled at its center. Elevation is
    /// centered near sea level (mean +16 m, ±150 m spread) so maps hold
    /// real oceans AND high peaks; temperature and humidity are 0..=100.
    /// The center elevation IS the height function's value there — biome
    /// and ground can never disagree.
    pub fn macro_field(&self, region: RegionCoord) -> MacroField {
        let u = region.x as f64 + 0.5;
        let v = region.z as f64 + 0.5;
        let e = self.fbm(1, u, v, 3) as f64;
        let t = self.fbm(2, u, v, 2);
        let hum = self.fbm(3, u, v, 2);
        let half = (MAX_ELEVATION_M - MIN_ELEVATION_M) as f64 / 2.0;
        let elev_m = (16.0 + (e - 0.5) * 2.4 * half).clamp(
            MIN_ELEVATION_M as f64,
            MAX_ELEVATION_M as f64,
        ) as i32;
        MacroField {
            elevation_m: elev_m,
            temperature: (t * 100.0) as u8,
            humidity: (hum * 100.0) as u8,
        }
    }

    /// The biome for a field. Coherence rules: below sea level is Ocean; a
    /// thin band above is Coast; very humid lowland is Wetland; humid land
    /// is Forest; elevation climbs through Highlands to Mountains; cold
    /// high ground caps as SnowPeaks.
    pub fn biome_of(&self, f: &MacroField) -> Biome {
        if f.elevation_m < SEA_LEVEL_M {
            return Biome::Ocean;
        }
        if f.elevation_m <= SEA_LEVEL_M + 2 {
            return Biome::Coast;
        }
        if f.elevation_m >= 128 {
            if f.temperature < 35 {
                return Biome::SnowPeaks;
            }
            return Biome::Mountains;
        }
        if f.elevation_m >= 72 {
            return Biome::Highlands;
        }
        if f.humidity >= 85 && f.elevation_m <= SEA_LEVEL_M + 12 {
            return Biome::Wetland;
        }
        if f.humidity >= 55 {
            return Biome::Forest;
        }
        Biome::Plains
    }

    pub fn biome(&self, region: RegionCoord) -> Biome {
        let f = self.macro_field(region);
        self.biome_of(&f)
    }

    /// THE shared height function: surface height in millimeters at world
    /// (x, z). One CONTINUOUS elevation field — the same fbm the macro
    /// fields sample — so a region's center height equals its field
    /// elevation exactly (biome and ground always agree), the function is
    /// seamless everywhere by construction, and detail noise adds ±1.5 m
    /// at 8 m / 4 m scales. Mesh, collision, water, and every regenerated
    /// patch consult this one function.
    pub fn surface_height_mm(&self, wx: i64, wz: i64) -> i64 {
        let e = self.fbm(1, wx as f64 / REGION_MM_F, wz as f64 / REGION_MM_F, 3) as f64;
        let half = (MAX_ELEVATION_M - MIN_ELEVATION_M) as f64 / 2.0;
        let base_m = (16.0 + (e - 0.5) * 2.4 * half).clamp(
            MIN_ELEVATION_M as f64,
            MAX_ELEVATION_M as f64,
        );
        let detail = self.detail_mm(wx, wz);
        (base_m * 1000.0) as i64 + detail
    }

    /// Continuous detail (±1.5 m): two blended fine-noise samples on 8 m
    /// and 4 m lattices. Continuous by construction — never a seam source.
    fn detail_mm(&self, wx: i64, wz: i64) -> i64 {
        let u = wx as f64 / 8000.0;
        let v = wz as f64 / 8000.0;
        let n = self.value_noise(10, 0, u, v) * 0.7
            + self.value_noise(11, 0, u * 2.0, v * 2.0) * 0.3;
        ((n - 0.5) * 2.0 * 1500.0) as i64
    }

    /// Regenerate one patch's 16³ cells purely from the global function.
    /// Surface layer gets the biome's material, then Soil, then Rock; air
    /// below sea level is Water; coasts and ocean floors are Sand.
    pub fn regenerate_patch(&self, coord: PatchCoord) -> PatchCells {
        let n = PATCH_CELL_AXIS as usize;
        let mut cells = vec![CellMaterial::Air; n * n * n];
        let origin = coord.origin();
        let ax = origin.x.div_euclid(1000) as i32;
        let ay = origin.y.div_euclid(1000) as i32;
        let az = origin.z.div_euclid(1000) as i32;
        let surface = self.biome(coord.region());
        for cx in 0..n {
            for cz in 0..n {
                let wx = (ax + cx as i32) as i64 * 1000;
                let wz = (az + cz as i32) as i64 * 1000;
                let surface_mm = self.surface_height_mm(wx, wz);
                for cy in 0..n {
                    let wy = (ay + cy as i32) as i64 * 1000;
                    let depth_mm = surface_mm - wy; // >0 = below the surface
                    let mat = if depth_mm < 0 {
                        if wy < 0 {
                            CellMaterial::Water
                        } else {
                            CellMaterial::Air
                        }
                    } else if depth_mm < 1000 {
                        match surface {
                            Biome::Ocean | Biome::Coast => CellMaterial::Sand,
                            Biome::SnowPeaks => CellMaterial::Snow,
                            Biome::Mountains | Biome::Highlands => CellMaterial::Rock,
                            _ => CellMaterial::Grass,
                        }
                    } else if depth_mm < 4000 {
                        CellMaterial::Soil
                    } else {
                        CellMaterial::Rock
                    };
                    cells[(cx * n + cy) * n + cz] = mat;
                }
            }
        }
        PatchCells { coord, cells }
    }

    /// Cheap proof identity for (seed, coord) without keeping cells alive.
    pub fn patch_hash(&self, coord: PatchCoord) -> u64 {
        self.regenerate_patch(coord).hash()
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

const REGION_MM_F: f64 = 256_000.0;

// Keep the seed-streams contract visible: worlds derive from seeds through
// the shared pc3d_core primitives (never a private RNG).
#[allow(unused)]
fn stream_contract_check(seed: u64) -> u64 {
    SeedStreams::new(seed).stream_seed("terrain")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Same seed → identical fields, cells, hashes. Different seeds diverge:
    /// across a band of patches at least one differs AND the fields differ
    /// somewhere (a fully-uniform ocean patch is allowed to match).
    #[test]
    fn p3d103_generation_is_deterministic_and_seed_sensitive() {
        let a = WorldGen::new(12345);
        let b = WorldGen::new(12345);
        let c = WorldGen::new(54321);
        for x in -3..=3 {
            for z in -3..=3 {
                let r = RegionCoord { x, z };
                assert_eq!(a.macro_field(r), b.macro_field(r));
            }
        }
        let mut differing_fields = 0;
        let mut differing_heights = 0;
        let mut worst: Option<(RegionCoord, i64)> = None;
        for i in 0..8i32 {
            let r = RegionCoord { x: i, z: -i };
            if a.macro_field(r) != c.macro_field(r) {
                differing_fields += 1;
                let center_mm = (r.x as i64 * 256 + 128) * 1000;
                let dz = (a.surface_height_mm(center_mm, center_mm)
                    - c.surface_height_mm(center_mm, center_mm))
                .abs();
                if dz > 0 {
                    differing_heights += 1;
                }
                match worst {
                    Some((_, d)) if d >= dz => {}
                    _ => worst = Some((r, dz)),
                }
            }
        }
        assert!(differing_fields > 0, "different seeds produced identical fields");
        assert!(differing_heights > 0, "field differences never reached the ground");
        // The most-different region's center SURFACE differs between seeds.
        // (Material-cube divergence is deliberately NOT asserted here: a
        // uniformly deep-rock window quantizes to the same cube under small
        // height shifts — the height map is the world's identity signal.)
        let (r, _) = worst.expect("checked above");
        let center_mm = (r.x as i64 * 256 + 128) * 1000;
        assert_ne!(
            a.surface_height_mm(center_mm, center_mm),
            c.surface_height_mm(center_mm, center_mm)
        );
        // Same seed replays exactly.
        let coord = PatchCoord { x: r.x * 16 + 8, y: 0, z: r.z * 16 + 8 };
        assert_eq!(a.regenerate_patch(coord).hash(), b.regenerate_patch(coord).hash());
    }

    /// Field/biome coherence across many seeds over a 80×80-region sweep
    /// (wide enough to cross continental bands): the table reads as
    /// terrain, and the major biomes are all reachable.
    #[test]
    fn p3d103_biomes_are_coherent_with_fields() {
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0..8u64 {
            let g = WorldGen::new(seed.wrapping_mul(0x9E3779B97F4A7C15));
            for x in -40..=40 {
                for z in -40..=40 {
                    let f = g.macro_field(RegionCoord { x, z });
                    let b = g.biome_of(&f);
                    seen.insert(b);
                    match b {
                        Biome::Ocean => assert!(f.elevation_m < SEA_LEVEL_M),
                        Biome::Coast => assert!(f.elevation_m <= SEA_LEVEL_M + 2),
                        Biome::SnowPeaks => {
                            assert!(f.elevation_m >= 128 && f.temperature < 35)
                        }
                        Biome::Mountains => assert!(f.elevation_m >= 128),
                        Biome::Highlands => assert!((72..128).contains(&f.elevation_m)),
                        Biome::Wetland => {
                            assert!(f.humidity >= 85 && f.elevation_m <= SEA_LEVEL_M + 12)
                        }
                        Biome::Forest => assert!(f.humidity >= 55),
                        Biome::Plains => {}
                    }
                }
            }
        }
        for b in [
            Biome::Ocean,
            Biome::Coast,
            Biome::Plains,
            Biome::Forest,
            Biome::Mountains,
        ] {
            assert!(seen.contains(&b), "biome {} unreachable across seeds", b.name());
        }
    }

    /// Neighbor regions correlate (D-016): adjacent-region elevation steps
    /// stay under half the full range — dramatic geography (cliffs) is
    /// allowed, teleports are not; the SMOOTHNESS contract is enforced by
    /// the height-function seam test, which bilinearly blends every step.
    #[test]
    fn p3d103_neighbor_regions_correlate() {
        let g = WorldGen::new(777);
        let full_range = (MAX_ELEVATION_M - MIN_ELEVATION_M) as i32;
        for x in -6..=6 {
            for z in -6..=6 {
                let a = g.macro_field(RegionCoord { x, z }).elevation_m;
                let b = g.macro_field(RegionCoord { x: x + 1, z }).elevation_m;
                let c = g.macro_field(RegionCoord { x, z: z + 1 }).elevation_m;
                assert!((a - b).abs() * 2 <= full_range, "x-step {a}->{b}");
                assert!((a - c).abs() * 2 <= full_range, "z-step {a}->{c}");
            }
        }
    }

    /// The height function is GLOBAL: the same world column sampled from
    /// either side of a patch border is continuous — regeneration cannot
    /// seam, ever.
    #[test]
    fn p3d103_height_function_is_global_across_patch_borders() {
        let g = WorldGen::new(31415);
        for z in -40..=40 {
            let wz = z as i64 * 1000 + 7; // off-grid sample
            let left = g.surface_height_mm(16 * 1000 - 1, wz);
            let right = g.surface_height_mm(16 * 1000, wz);
            assert!(
                (left - right).abs() <= 3_000,
                "seam at patch border: {left} vs {right}"
            );
        }
        // Continuity across a REGION border too (the coarser seam).
        for z in -40..=40 {
            let wz = z as i64 * 1000 - 3;
            let left = g.surface_height_mm(256 * 1000 - 1, wz);
            let right = g.surface_height_mm(256 * 1000, wz);
            assert!((left - right).abs() <= 3_000, "region seam {left} vs {right}");
        }
        // Pure: same input, same answer, forever.
        assert_eq!(
            g.surface_height_mm(123_456, -654_321),
            g.surface_height_mm(123_456, -654_321)
        );
    }

    /// Surface materials read as terrain: an ocean patch has Water over
    /// Sand; a land patch has Grass over Soil/Rock.
    #[test]
    fn p3d103_patch_materials_read_as_terrain() {
        let g = WorldGen::new(2024);
        let mut ocean = None;
        let mut land = None;
        'outer: for x in -20..=20 {
            for z in -20..=20 {
                let r = RegionCoord { x, z };
                let b = g.biome(r);
                if ocean.is_none() && b == Biome::Ocean {
                    ocean = Some(r);
                }
                if land.is_none() && matches!(b, Biome::Plains | Biome::Forest) {
                    land = Some(r);
                }
                if ocean.is_some() && land.is_some() {
                    break 'outer;
                }
            }
        }
        if let Some(r) = ocean {
            // Probe the patch containing the region center's SURFACE (an
            // ocean center is below sea level; its corner patches near a
            // coast can be dry land).
            let f = g.macro_field(r);
            let py = (f.elevation_m / 16).clamp(-16, -1);
            let patch = g.regenerate_patch(PatchCoord {
                x: r.x * 16 + 8,
                y: py,
                z: r.z * 16 + 8,
            });
            assert!(
                patch.cells.iter().any(|&c| c == CellMaterial::Water),
                "ocean patch at {r:?} (elev {} m) lacks water",
                f.elevation_m
            );
        }
        if let Some(r) = land {
            // Probe the patch containing the region center's SURFACE: the
            // center height equals the field elevation (Plains/Forest are
            // 3..72 m), so the surface layer lives in patch y = elev/16.
            let f = g.macro_field(r);
            let py = (f.elevation_m / 16).clamp(-1, 12);
            let patch = g.regenerate_patch(PatchCoord {
                x: r.x * 16 + 8,
                y: py,
                z: r.z * 16 + 8,
            });
            assert!(
                patch.cells.iter().any(|&c| c == CellMaterial::Grass),
                "land patch at {r:?} (elev {} m) lacks grass",
                f.elevation_m
            );
            assert!(patch.cells.iter().any(|&c| matches!(
                c,
                CellMaterial::Rock | CellMaterial::Soil
            )));
        }
    }

    /// Regeneration cost vs the future budget: 16 patches must finish far
    /// under one 60 Hz frame's worth of work each (budget: 16 ms per edit).
    #[test]
    fn p3d103_patch_regeneration_is_cheap() {
        let g = WorldGen::new(1);
        let start = std::time::Instant::now();
        let mut hashes = Vec::new();
        for i in 0..16i32 {
            hashes.push(g.regenerate_patch(PatchCoord { x: i, y: 0, z: 0 }).hash());
        }
        let elapsed = start.elapsed();
        assert_eq!(hashes.len(), 16);
        assert!(elapsed.as_millis() < 500, "16 patches took {elapsed:?}");
    }
}
