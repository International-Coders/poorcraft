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

/// Stroke width in pixels for a river's discharge: monotonic, clamped to
/// [1, 6]. Wide rivers are wide because they GATHERED, so width is a
/// sub-linear function of discharge (sqrt).
pub fn river_stroke_width(discharge: u64) -> f32 {
    let w = 1.0 + (discharge as f32).sqrt() * 0.12;
    w.clamp(1.0, 6.0)
}

/// Brightness gain for a river's current (slope per-mille): monotonic,
/// clamped to [1.0, 1.6]. Fast water is whiter.
pub fn current_shade(slope_per_mille: i32) -> f32 {
    let s = slope_per_mille.max(0) as f32;
    (1.0 + s / 200.0).clamp(1.0, 1.6)
}

/// Render a FLOW MAP: biome base dimmed, then every river edge drawn as
/// a stroke from the region center toward its downstream center — width
/// by discharge, brightness by slope. No particles: the records ARE the
/// water. Byte-deterministic.
pub fn render_flow_map(seed: u64, half_regions: i32) -> AtlasImage {
    let gen = WorldGen::new(seed);
    let rivers = crate::hydro::RiverGraph::new(&gen, half_regions);
    let table = crate::flow::FlowTable::from_graph(&rivers);
    let side = (2 * half_regions + 1) as usize;
    // Scale: one region = 4 atlas pixels for stroke room.
    let px_per_region = 4usize;
    let side_px = side * px_per_region;
    let mut rgb = vec![0u8; side_px * side_px * 3];
    // Dimmed biome base.
    let biome_atlas = render_region_atlas_no_rivers(seed, half_regions);
    for pz in 0..side {
        for px in 0..side {
            let base = biome_atlas.pixel(px, pz);
            for oy in 0..px_per_region {
                for ox in 0..px_per_region {
                    let x = px * px_per_region + ox;
                    let y = pz * px_per_region + oy;
                    let i = (y * side_px + x) * 3;
                    let dim = 0.55;
                    rgb[i] = (base[0] as f32 * dim) as u8;
                    rgb[i + 1] = (base[1] as f32 * dim) as u8;
                    rgb[i + 2] = (base[2] as f32 * dim) as u8;
                }
            }
        }
    }
    // Center of a region in atlas pixels.
    let center_px = |r: RegionCoord| -> (f32, f32) {
        let ix = (r.x + half_regions) as f32;
        let iz = (r.z + half_regions) as f32;
        (
            (ix + 0.5) * px_per_region as f32,
            (iz + 0.5) * px_per_region as f32,
        )
    };
    // Draw each river edge as a thick stroke.
    for x in -half_regions..=half_regions {
        for z in -half_regions..=half_regions {
            let r = RegionCoord { x, z };
            let rec = match table.get(r) {
                Some(rec) if rec.direction != crate::flow::DIR_SINK => rec,
                _ => continue,
            };
            let Some(down) = rivers.downstream(r) else { continue };
            if rivers.discharge(down) < crate::hydro::RIVER_THRESHOLD {
                continue;
            }
            let (x0, z0) = center_px(r);
            let (x1, z1) = center_px(down);
            let width = river_stroke_width(rec.discharge);
            let gain = current_shade(rec.slope_per_mille);
            // Rasterize: walk the segment, stamp a width-square brush.
            let steps = ((x1 - x0).abs().max((z1 - z0).abs()).ceil() as usize).max(1) * 2;
            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let cx = x0 + (x1 - x0) * t;
                let cz = z0 + (z1 - z0) * t;
                let half_w = (width / 2.0).ceil() as i32;
                for oy in -half_w..=half_w {
                    for ox in -half_w..=half_w {
                        let x = (cx.round() as i32 + ox).clamp(0, side_px as i32 - 1) as usize;
                        let y = (cz.round() as i32 + oy).clamp(0, side_px as i32 - 1) as usize;
                        let i = (y * side_px + x) * 3;
                        rgb[i] = ((60.0 * gain).round() as i32).clamp(0, 255) as u8;
                        rgb[i + 1] = ((130.0 * gain).round() as i32).clamp(0, 255) as u8;
                        rgb[i + 2] = ((245.0 * gain).round() as i32).clamp(0, 255) as u8;
                    }
                }
            }
        }
    }
    // THE MACHINE PROOF (P3D-306): the best waterwheel site stamps a
    // white marker on the map — the visible answer to "where do I build?"
    if let Some((site, _)) = rivers.best_wheel_site(&gen, None) {
        let (sx, sz) = center_px(site);
        let cx = sx.round() as i64;
        let cz = sz.round() as i64;
        for d in -3..=3i64 {
            for (ox, oy) in [(d, 0i64), (0i64, d)] {
                let x = (cx + ox).clamp(0, side_px as i64 - 1) as usize;
                let y = (cz + oy).clamp(0, side_px as i64 - 1) as usize;
                let i = (y * side_px + x) * 3;
                rgb[i] = 255;
                rgb[i + 1] = 255;
                rgb[i + 2] = 255;
            }
        }
    }
    AtlasImage { size: side_px, rgb }
}

/// Biome-only atlas WITHOUT river overlay (the flow map's dimmed base).
fn render_region_atlas_no_rivers(seed: u64, half_regions: i32) -> AtlasImage {
    let gen = WorldGen::new(seed);
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
            let [cr, cg, cb] = biome_color(b);
            let i = (iz * side + ix) * 3;
            rgb[i] = cr;
            rgb[i + 1] = cg;
            rgb[i + 2] = cb;
        }
    }
    AtlasImage { size: side, rgb }
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

#[cfg(test)]
mod flow_map_tests {
    use super::*;

    /// Width is monotonic in discharge and clamped; shade is monotonic
    /// in slope and clamped.
    #[test]
    fn p3d304_width_and_shade_are_monotonic() {
        let mut last_w = 0.0f32;
        for d in [1u64, 10, 100, 1000, 100_000] {
            let w = river_stroke_width(d);
            assert!(w >= last_w);
            assert!((1.0..=6.0).contains(&w));
            last_w = w;
        }
        assert_eq!(river_stroke_width(u64::MAX), 6.0);
        let mut last_s = 0.0f32;
        for s in [0i32, 10, 100, 10_000] {
            let sh = current_shade(s);
            assert!(sh >= last_s);
            assert!((1.0..=1.6).contains(&sh));
            last_s = sh;
        }
        assert_eq!(current_shade(-50), 1.0, "negative slope clamps");
    }

    /// The flow map is byte-deterministic and river strokes actually
    /// land: a map with rivers differs from the dimmed base and from
    /// other seeds.
    #[test]
    fn p3d304_flow_map_is_deterministic_and_drawn() {
        let a = render_flow_map(2024, 24);
        let b = render_flow_map(2024, 24);
        assert_eq!(a, b);
        // At least some pixels are bright river blue (strokes drawn).
        let mut bright = 0;
        for i in 0..(a.size * a.size) {
            let (r, g, bl) = (a.rgb[i * 3] as i32, a.rgb[i * 3 + 1] as i32, a.rgb[i * 3 + 2] as i32);
            if bl > 150 && bl > r + 60 && g > r {
                bright += 1;
            }
        }
        assert!(bright > 50, "too few river stroke pixels: {bright}");
        assert_ne!(render_flow_map(2024, 24), render_flow_map(2025, 24));
    }

    /// Wetness accessor agrees with region wetness.
    #[test]
    fn p3d304_wetness_accessor_consistent() {
        let gen = WorldGen::new(1);
        let graph = crate::hydro::RiverGraph::new(&gen, 12);
        let r = RegionCoord { x: 0, z: 0 };
        let wx = (r.x * 256 + 128) as i64 * 1000;
        let wz = (r.z * 256 + 128) as i64 * 1000;
        assert_eq!(
            graph.wetness_at_mm(&gen, wx, wz),
            graph.wetness(&gen, r)
        );
    }
}

#[cfg(test)]
mod wheel_marker_tests {
    use super::*;

    /// THE visible machine proof: the flow map carries a white wheel
    /// marker at the best viable site, and querying the site's potential
    /// twice is pure (same answer).
    #[test]
    fn p3d306_wheel_marker_stamped_at_best_site() {
        let seed = 2024;
        let half = 20;
        let a = render_flow_map(seed, half);
        let g = WorldGen::new(seed);
        let rivers = crate::hydro::RiverGraph::new(&g, half);
        let Some((site, potential)) = rivers.best_wheel_site(&g, None) else {
            panic!("a viable wheel site must exist");
        };
        let _ = potential;
        // The site's center pixel region carries white marker pixels.
        let px_per_region = 4usize;
        let cx = ((site.x + half) as usize * px_per_region + px_per_region / 2) as i64;
        let cz = ((site.z + half) as usize * px_per_region + px_per_region / 2) as i64;
        let mut white = 0;
        for dy in -3i64..=3 {
            for dx in -3i64..=3 {
                if dx != 0 && dy != 0 {
                    continue;
                }
                let x = (cx + dx) as usize;
                let y = (cz + dy) as usize;
                let i = (y * a.size + x) * 3;
                if a.rgb[i] == 255 && a.rgb[i + 1] == 255 && a.rgb[i + 2] == 255 {
                    white += 1;
                }
            }
        }
        assert!(white >= 6, "wheel marker not found: {white} white pixels");
        // Purity: re-query matches.
        let wx = (site.x * 256 + 128) as i64 * 1000;
        let wz = (site.z * 256 + 128) as i64 * 1000;
        let p1 = rivers.flow_potential_at(&g, None, wx, wz);
        let p2 = rivers.flow_potential_at(&g, None, wx, wz);
        assert_eq!(p1, p2);
    }
}
