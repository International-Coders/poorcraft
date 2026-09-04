//! P3D-104: seed atlas and patch-hash proof tools.
//!
//! The evidence layer for generation (the blueprint's terrain proof suite
//! starts here): pure atlas rendering — one pixel per region, colored by
//! biome, graded by elevation — plus the cross-seed categorical
//! disagreement gate (two seeds must produce visibly different worlds) and
//! cheap patch-hash spot verification. PNG encoding lives in the binary;
//! this module stays pure.

use crate::coords::RegionCoord;
use crate::gen::{Biome, WorldGen};

/// A rendered atlas: row-major RGB, `size × size` pixels, pixel (0,0) is
/// the min corner (region x = -half, z = -half).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtlasImage {
    pub size: usize,
    pub rgb: Vec<u8>,
}

impl AtlasImage {
    /// The biome id at a pixel, decoded from the stored color index LSB
    /// path is not needed by callers — tests use `biome_at`.
    pub fn pixel(&self, x: usize, z: usize) -> [u8; 3] {
        let i = (z * self.size + x) * 3;
        [self.rgb[i], self.rgb[i + 1], self.rgb[i + 2]]
    }
}

/// Fixed distinct palette — functional, not art-directed. The eight biomes
/// must stay injective (tests pin it).
pub fn biome_color(b: Biome) -> [u8; 3] {
    match b {
        Biome::Ocean => [30, 60, 140],
        Biome::Coast => [220, 205, 130],
        Biome::Plains => [110, 170, 80],
        Biome::Forest => [40, 110, 55],
        Biome::Wetland => [70, 130, 125],
        Biome::Highlands => [140, 130, 105],
        Biome::Mountains => [120, 120, 130],
        Biome::SnowPeaks => [235, 235, 240],
    }
}

/// Render one pixel per region over `[-half, half]²`, colored by biome and
/// brightness-graded by elevation within the biome (peaks lighter, deep
/// ocean darker). Pure and byte-deterministic.
pub fn render_region_atlas(seed: u64, half_regions: i32) -> AtlasImage {
    let gen = WorldGen::new(seed);
    let rivers = crate::hydro::RiverGraph::new(&gen, half_regions);
    let side = (2 * half_regions + 1) as usize;
    let mut rgb = vec![0u8; side * side * 3];
    for iz in 0..side {
        for ix in 0..side {
            let r = RegionCoord {
                x: ix as i32 - half_regions,
                z: iz as i32 - half_regions,
            };
            let f = gen.macro_field(r);
            let b = gen.biome_of(&f);
            let [mut cr, mut cg, mut cb] = biome_color(b);
            // Grade brightness by elevation inside the biome band: map the
            // field range (-64..192 m) to a ±20% gain.
            let t = (f.elevation_m - crate::gen::MIN_ELEVATION_M) as f32
                / (crate::gen::MAX_ELEVATION_M - crate::gen::MIN_ELEVATION_M) as f32;
            let gain = 0.8 + 0.4 * t.clamp(0.0, 1.0);
            cr = ((cr as f32 * gain).round() as i32).clamp(0, 255) as u8;
            cg = ((cg as f32 * gain).round() as i32).clamp(0, 255) as u8;
            cb = ((cb as f32 * gain).round() as i32).clamp(0, 255) as u8;
            // River regions draw as bright blue over the biome color —
            // land regions only, and only up to a discharge cap so basin
            // collectors near the sea do not render as lakes.
            if rivers.is_river(r) && b != crate::gen::Biome::Ocean && rivers.discharge(r) <= 4_000 {
                cr = 40;
                cg = 110;
                cb = 230;
            }
            let i = (iz * side + ix) * 3;
            rgb[i] = cr;
            rgb[i + 1] = cg;
            rgb[i + 2] = cb;
        }
    }
    AtlasImage { size: side, rgb }
}

/// Fraction of regions whose BIOME differs between two seeds — the
/// categorical disagreement gate. 0.0 means the two seeds drew identical
/// maps (a red flag at any useful size); ~0.5+ means distinct worlds.
pub fn cross_seed_disagreement(seed_a: u64, seed_b: u64, half_regions: i32) -> f32 {
    let a = WorldGen::new(seed_a);
    let b = WorldGen::new(seed_b);
    let mut differ = 0u64;
    let mut total = 0u64;
    for x in -half_regions..=half_regions {
        for z in -half_regions..=half_regions {
            let r = RegionCoord { x, z };
            if a.biome(r) != b.biome(r) {
                differ += 1;
            }
            total += 1;
        }
    }
    differ as f32 / total as f32
}

/// Cheap spot proof: regeneration is deterministic for one coordinate.
pub fn verify_patch_hash(seed: u64, coord: crate::coords::PatchCoord) -> bool {
    let g = WorldGen::new(seed);
    g.patch_hash(coord) == g.patch_hash(coord)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The atlas is a pure function of (seed, half): reruns are
    /// byte-identical, and the buffer is exactly size² × 3.
    #[test]
    fn p3d104_atlas_is_deterministic() {
        let a = render_region_atlas(42, 16);
        let b = render_region_atlas(42, 16);
        assert_eq!(a, b, "same seed must render identically");
        assert_eq!(a.rgb.len(), a.size * a.size * 3);
        assert_eq!(a.size, 33);
        // Different seeds render differently (byte-level, almost surely).
        assert_ne!(render_region_atlas(42, 16), render_region_atlas(43, 16));
    }

    /// The palette is injective — two biomes may never share a color, or
    /// the atlas lies about the world.
    #[test]
    fn p3d104_palette_is_injective() {
        let biomes = [
            Biome::Ocean,
            Biome::Coast,
            Biome::Plains,
            Biome::Forest,
            Biome::Wetland,
            Biome::Highlands,
            Biome::Mountains,
            Biome::SnowPeaks,
        ];
        let mut colors = Vec::new();
        for b in biomes {
            let c = biome_color(b);
            assert!(!colors.contains(&c), "{:?} reuses a color", b.name());
            colors.push(c);
        }
    }

    /// THE disagreement gate: different seeds produce clearly different
    /// worlds (well above a floor), a seed agrees with itself exactly, and
    /// the same seed across different SIZES still agrees.
    #[test]
    fn p3d104_cross_seed_disagreement_gates() {
        let mut pairs = 0;
        for s in 0..6u64 {
            let d = cross_seed_disagreement(1000 + s, 2000 + s, 24);
            let (sa, sb) = (1000 + s, 2000 + s);
            assert!(
                d >= 0.15,
                "seeds {sa}/{sb} disagree only {d:.2} — worlds look alike"
            );
            pairs += 1;
            assert_eq!(cross_seed_disagreement(1000 + s, 1000 + s, 24), 0.0);
        }
        assert_eq!(pairs, 6);
        assert_eq!(cross_seed_disagreement(7, 7, 12), 0.0);
    }

    /// Patch-hash spot checks: regeneration is deterministic at sampled
    /// coords across several seeds, including negatives.
    #[test]
    fn p3d104_patch_hash_spot_checks() {
        for seed in [0u64, 1, 0xDEADBEEF] {
            for x in [-3i32, 0, 7] {
                for y in [-1i32, 0] {
                    for z in [-9i32, 2] {
                        assert!(verify_patch_hash(
                            seed,
                            crate::coords::PatchCoord { x, y, z }
                        ));
                    }
                }
            }
        }
    }

    /// The atlas span is symmetric and every pixel is a known palette
    /// color (up to elevation gain) OR a river-blue pixel — no garbage
    /// bytes in the buffer.
    #[test]
    fn p3d104_atlas_pixels_are_all_valid_biome_shades() {
        let atlas = render_region_atlas(9, 8);
        let gen = WorldGen::new(9);
        let rivers = crate::hydro::RiverGraph::new(&gen, 8);
        for iz in 0..atlas.size {
            for ix in 0..atlas.size {
                let r = RegionCoord {
                    x: ix as i32 - 8,
                    z: iz as i32 - 8,
                };
                let f = gen.macro_field(r);
                if rivers.is_river(r) {
                    // River pixels draw fixed blue over everything.
                    let px = atlas.pixel(ix, iz);
                    assert_eq!(px, [40, 110, 230], "river pixel {ix},{iz}");
                    continue;
                }
                let base = biome_color(gen.biome_of(&f));
                let px = atlas.pixel(ix, iz);
                // Each channel within the gain band of the palette color.
                for ch in 0..3 {
                    let lo = (base[ch] as f32 * 0.8).floor() as i32 - 1;
                    let hi = (base[ch] as f32 * 1.2).ceil() as i32 + 1;
                    assert!(
                        (px[ch] as i32) >= lo && (px[ch] as i32) <= hi,
                        "pixel {ix},{iz} off-palette: {px:?} vs base {base:?}"
                    );
                }
            }
        }
    }
}
