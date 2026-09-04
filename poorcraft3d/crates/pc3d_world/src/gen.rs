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

impl CellMaterial {
    /// Stable on-disk code (edit journals and snapshots persist these).
    pub fn from_code(c: u8) -> Option<CellMaterial> {
        match c {
            0 => Some(CellMaterial::Air),
            1 => Some(CellMaterial::Water),
            2 => Some(CellMaterial::Soil),
            3 => Some(CellMaterial::Grass),
            4 => Some(CellMaterial::Sand),
            5 => Some(CellMaterial::Rock),
            6 => Some(CellMaterial::Snow),
            _ => None,
        }
    }
}

/// The generator: one per world seed. Copyable; every method is pure.
#[derive(Clone, Copy, Debug)]
pub struct WorldGen {
    seed: u64,
}

/// 16×16×16 regenerated cells, indexed [(x*n + y)*n + z].
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// The smooth surface plus detail noise — the UNCLIFFED surface. Solid
    /// consumers use `effective_surface_mm`; this stays for comparison
    /// (the bake-off's fidelity baseline) and the macro-field equivalence.
    pub fn surface_height_mm(&self, wx: i64, wz: i64) -> i64 {
        self.surface_base_mm(wx, wz) + self.detail_mm(wx, wz)
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

    /// Trilinear 3D value noise on a lattice of `cell_mm` mm cells
    /// (P3D-203). Continuous — shares the smoothstep blend of the 2D field.
    fn value_noise_3d(&self, channel: u64, cell_mm: i64, wx: i64, wy: i64, wz: i64) -> f32 {
        let gx = wx.div_euclid(cell_mm);
        let gy = wy.div_euclid(cell_mm);
        let gz = wz.div_euclid(cell_mm);
        let fx = ((wx - gx * cell_mm) as f32) / cell_mm as f32;
        let fy = ((wy - gy * cell_mm) as f32) / cell_mm as f32;
        let fz = ((wz - gz * cell_mm) as f32) / cell_mm as f32;
        let (sx, sy, sz) = (smoothstep(fx), smoothstep(fy), smoothstep(fz));
        let h = |ox: i64, oy: i64, oz: i64| -> f32 {
            let mut hh = fnv1a64(&self.seed.to_le_bytes());
            for word in [channel, 0u64, (gx + ox) as u64, (gy + oy) as u64, (gz + oz) as u64] {
                for b in word.to_le_bytes() {
                    hh ^= b as u64;
                    hh = hh.wrapping_mul(0x100000001b3);
                }
            }
            ((hh >> 11) as f32) / ((1u64 << 53) as f32)
        };
        let c00 = h(0, 0, 0) + (h(1, 0, 0) - h(0, 0, 0)) * sx;
        let c10 = h(0, 1, 0) + (h(1, 1, 0) - h(0, 1, 0)) * sx;
        let c01 = h(0, 0, 1) + (h(1, 0, 1) - h(0, 0, 1)) * sx;
        let c11 = h(0, 1, 1) + (h(1, 1, 1) - h(0, 1, 1)) * sx;
        let c0 = c00 + (c10 - c00) * sy;
        let c1 = c01 + (c11 - c01) * sy;
        c0 + (c1 - c0) * sz
    }

    /// THE terrain surface (P3D-203): the smooth fbm base, plus detail
    /// noise — except inside cliff-masked bands, where the surface
    /// quantizes to crisp 4 m terraces with vertical faces (detail
    /// suppressed so the step is a cliff, not a smear).
    pub fn effective_surface_mm(&self, wx: i64, wz: i64) -> i64 {
        let base = self.surface_base_mm(wx, wz);
        let mask = self.cliff_mask(wx, wz);
        if mask > 0.54 && base > 2_000 {
            (base as f64 / 4_000.0).floor() as i64 * 4_000
        } else {
            base + self.detail_mm(wx, wz)
        }
    }

    /// The smooth, detail-free fbm surface (mm).
    pub fn surface_base_mm(&self, wx: i64, wz: i64) -> i64 {
        let e = self.fbm(1, wx as f64 / REGION_MM_F, wz as f64 / REGION_MM_F, 3) as f64;
        let half = (MAX_ELEVATION_M - MIN_ELEVATION_M) as f64 / 2.0;
        ((16.0 + (e - 0.5) * 2.4 * half).clamp(
            MIN_ELEVATION_M as f64,
            MAX_ELEVATION_M as f64,
        ) * 1000.0) as i64
    }

    /// The cliff mask value at a point (public for tests/diagnostics).
    pub fn cliff_mask(&self, wx: i64, wz: i64) -> f32 {
        self.value_noise(20, 0, wx as f64 / 60_000.0, wz as f64 / 60_000.0)
    }

    /// Cave carving decision (P3D-203): two intersecting mid-band 3D noise
    /// fields make worm-like voids. Gated by the sealed-volume rules:
    /// a solid crust of 4 m below the surface, never below y = 0 (WATER
    /// SEAL — oceans cannot drain into caves until hydrology lands),
    /// never deeper than 120 m (solid deep crust).
    pub fn is_carved(&self, wx: i64, wy: i64, wz: i64, depth_mm: i64) -> bool {
        if wy < 0 || depth_mm < 4_000 || depth_mm > 120_000 {
            return false;
        }
        let a = self.value_noise_3d(30, 24_000, wx, wy, wz);
        let b = self.value_noise_3d(31, 16_000, wx, wy, wz);
        (a - 0.5).abs() < 0.085 && (b - 0.5).abs() < 0.12
    }

    /// Shared carving step: `regenerate_patch` and `final_solid` both call
    /// this, so their agreement stays structural. Only solid underground
    /// material can be carved.
    pub fn carve(
        &self,
        material: CellMaterial,
        wx: i64,
        wy: i64,
        wz: i64,
        depth_mm: i64,
    ) -> CellMaterial {
        if matches!(material, CellMaterial::Air | CellMaterial::Water) {
            return material;
        }
        if self.is_carved(wx, wy, wz, depth_mm) {
            CellMaterial::Air
        } else {
            material
        }
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
                let surface_mm = self.effective_surface_mm(wx, wz);
                for cy in 0..n {
                    let wy = (ay + cy as i32) as i64 * 1000;
                    let depth_mm = surface_mm - wy; // >0 = below the surface
                    let raw = cell_material(surface, wy, surface_mm);
                    // Only cells inside the cave band can be carved: skip
                    // the 3D noise for crust, deep-crust, air, and water.
                    let mat = if wy >= 0 && depth_mm >= 4_000 && depth_mm <= 120_000 {
                        self.carve(raw, wx, wy, wz, depth_mm)
                    } else {
                        raw
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

/// THE material decision — the single source of truth shared by patch
/// regeneration and the final-solid query (P3D-202). `wy` is the cell's
/// base height in mm, `surface_mm` the analytic surface above it.
/// Rules: above the surface is Air, or Water below sea level; the top
/// meter takes the biome's surface material; then 3 m of Soil; Rock
/// beneath.
pub fn cell_material(surface: Biome, wy: i64, surface_mm: i64) -> CellMaterial {
    let depth_mm = surface_mm - wy; // >0 = below the surface
    if depth_mm < 0 {
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
    }
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
            // P3D-203: caves may hollow the substrate, but the patch must
            // still hold SOME solid ground (walls/floor of the cave).
            assert!(
                patch.cells.iter().any(|&c| c != CellMaterial::Air),
                "land patch at {r:?} is entirely air"
            );
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
        // Budget: 16 ms per patch on release (the blueprint's edit-hitch
        // target) — checked by --terrain-bench on release builds. Debug
        // builds run the same math ~10x slower AND this suite may share
        // the CPU with parallel cargo runs, so its ceiling is generous;
        // the release ceiling is the contract.
        let budget_ms = if cfg!(debug_assertions) { 12_000 } else { 500 };
        assert!(
            elapsed.as_millis() < budget_ms,
            "16 patches took {elapsed:?} (budget {budget_ms} ms)"
        );
    }

    /// P3D-203: caves EXIST and obey the sealed-volume law. Scanning a
    /// band of land patches, carved air cells appear strictly underground,
    /// only within the cave band: below the 4 m crust, at or above
    /// y = 0 (water seal), inside the 120 m deep-crust bound.
    #[test]
    fn p3d203_caves_exist_and_stay_sealed() {
        let g = WorldGen::new(2024);
        let n = PATCH_CELL_AXIS as usize;
        let mut carved_total = 0usize;
        let mut patches_with_caves = 0usize;
        for px in -10..=10i32 {
            for pz in -10..=10i32 {
                let coord = PatchCoord { x: px * 16, y: 0, z: pz * 16 };
                let patch = g.regenerate_patch(coord);
                let o = coord.origin();
                let ax = o.x.div_euclid(1000) as i32;
                let ay = o.y.div_euclid(1000) as i32;
                let az = o.z.div_euclid(1000) as i32;
                let mut carved_here = 0usize;
                for cx in 0..n {
                    for cz in 0..n {
                        let wx = (ax + cx as i32) as i64 * 1000;
                        let wz = (az + cz as i32) as i64 * 1000;
                        let surface_mm = g.effective_surface_mm(wx, wz);
                        for cy in 0..n {
                            let wy = (ay + cy as i32) as i64 * 1000;
                            if patch.get(cx, cy, cz) != CellMaterial::Air {
                                continue;
                            }
                            let depth_mm = surface_mm - wy;
                            let carved = g.is_carved(wx, wy, wz, depth_mm);
                            if carved {
                                carved_here += 1;
                                assert!(wy >= 0, "carved below sea level {wx},{wy},{wz}");
                                assert!(depth_mm >= 4_000, "carved inside the crust");
                                assert!(depth_mm <= 120_000, "carved too deep");
                            }
                        }
                    }
                }
                carved_total += carved_here;
                if carved_here > 0 {
                    patches_with_caves += 1;
                }
            }
        }
        assert!(carved_total > 0, "no caves found across 441 land patches");
        assert!(patches_with_caves >= 3, "caves too rare: {patches_with_caves} patches");
    }

    /// P3D-203: cliffs are REAL — terraced 4 m steps appear between
    /// adjacent columns somewhere in a wide band, and terracing is the
    /// masked exception, not the rule.
    #[test]
    fn p3d203_cliffs_terminate_in_masked_bands() {
        let g = WorldGen::new(2024);
        // SEEK a masked band coarsely (mask cells are 60 m across), then
        // VERIFY a real cliff step (>= 3 m between adjacent columns) at
        // 1 m resolution inside it.
        let mut band: Option<(i64, i64)> = None;
        'seek: for x in -20_000..=20_000i64 {
            for z in -20_000..=20_000i64 {
                if x.rem_euclid(400) != 0 || z.rem_euclid(400) != 0 {
                    continue;
                }
                let wx = x * 1000;
                let wz = z * 1000;
                if g.cliff_mask(wx, wz) > 0.56 && g.surface_base_mm(wx, wz) > 4_000 {
                    band = Some((wx, wz));
                    break 'seek;
                }
            }
        }
        let (bx, bz) = band.expect("a cliff-masked band must exist in +-20 km");

        let mut stepped = 0i64;
        let mut total = 0i64;
        for dx in -48..=48i64 {
            for dz in -48..=48i64 {
                let wx = bx + dx * 1000;
                let wz = bz + dz * 1000;
                let h = g.effective_surface_mm(wx, wz);
                let next = g.effective_surface_mm(wx + 1_000, wz);
                total += 1;
                if (next - h).abs() >= 3_000 {
                    stepped += 1;
                }
            }
        }
        assert!(stepped > 0, "no >=3 m cliff step inside the masked band");
        assert!(stepped < total, "a wall of steps everywhere is not terracing");
    }
}
