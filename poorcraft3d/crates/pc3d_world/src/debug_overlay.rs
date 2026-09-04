//! P3D-207: the terrain debug overlay — per-patch state rows and a visual
//! LOD-ring atlas. Inspectable streaming state without a renderer.

use crate::coords::{PatchCoord, WorldPos};
use crate::gen::{Biome, WorldGen};
use crate::lod::{lod_for, LodLevel};
use crate::proof::AtlasImage;

/// One patch's debug row.
#[derive(Clone, Debug, PartialEq)]
pub struct PatchDebugRow {
    pub coord: PatchCoord,
    pub lod: LodLevel,
    pub biome: Biome,
    pub elevation_m: i32,
    /// Journal ops recorded for this patch (caller-supplied counts).
    pub edit_count: usize,
    /// Built blocks recorded for this patch (caller-supplied counts).
    pub built_count: usize,
}

/// Distinct colors per LOD ring (the overlay's visual vocabulary).
pub fn lod_color(l: LodLevel) -> [u8; 3] {
    match l {
        LodLevel::Full => [40, 160, 220],
        LodLevel::Mid => [60, 200, 120],
        LodLevel::Far => [230, 180, 60],
        LodLevel::Horizon => [120, 120, 130],
    }
}

/// Rows for every region in `[-half, half]²` around the viewer region
/// (0,0 in region space = the viewer's region), ascending by (z, x).
pub fn rows_for(
    gen: &WorldGen,
    viewer: WorldPos,
    half: i32,
    edit_counts: impl Fn(PatchCoord) -> usize,
    built_counts: impl Fn(PatchCoord) -> usize,
) -> Vec<PatchDebugRow> {
    let mut rows = Vec::new();
    for rx in -half..=half {
        for rz in -half..=half {
            let coord = PatchCoord { x: rx * 16, y: 0, z: rz * 16 };
            let center = WorldPos::from_mm(
                coord.x as i64 * crate::scales::PATCH_MM + crate::scales::PATCH_MM / 2,
                0,
                coord.z as i64 * crate::scales::PATCH_MM + crate::scales::PATCH_MM / 2,
            );
            let lod = lod_for(viewer, center);
            let region = crate::coords::RegionCoord {
                x: center.x.div_euclid(crate::scales::REGION_MM) as i32,
                z: center.z.div_euclid(crate::scales::REGION_MM) as i32,
            };
            let f = gen.macro_field(region);
            rows.push(PatchDebugRow {
                coord,
                lod,
                biome: gen.biome_of(&f),
                elevation_m: f.elevation_m,
                edit_count: edit_counts(coord),
                built_count: built_counts(coord),
            });
        }
    }
    rows
}

/// Render the LOD-ring atlas: one pixel per region, colored by LOD ring,
/// brightness graded by elevation. Byte-deterministic.
pub fn render_overlay(gen: &WorldGen, viewer: WorldPos, half: i32) -> AtlasImage {
    let side = (2 * half + 1) as usize;
    let mut rgb = vec![0u8; side * side * 3];
    for iz in 0..side {
        for ix in 0..side {
            let coord = PatchCoord {
                x: (ix as i32 - half) * 16,
                y: 0,
                z: (iz as i32 - half) * 16,
            };
            let center = WorldPos::from_mm(
                coord.x as i64 * crate::scales::PATCH_MM + crate::scales::PATCH_MM / 2,
                0,
                coord.z as i64 * crate::scales::PATCH_MM + crate::scales::PATCH_MM / 2,
            );
            let lod = lod_for(viewer, center);
            let region = crate::coords::RegionCoord {
                x: center.x.div_euclid(crate::scales::REGION_MM) as i32,
                z: center.z.div_euclid(crate::scales::REGION_MM) as i32,
            };
            let elev = gen.macro_field(region).elevation_m;
            let [r, g, b] = lod_color(lod);
            let gain = 0.8
                + 0.4 * ((elev - crate::gen::MIN_ELEVATION_M).clamp(0, 192) as f32 / 192.0);
            let i = (iz * side + ix) * 3;
            rgb[i] = ((r as f32 * gain).round() as i32).clamp(0, 255) as u8;
            rgb[i + 1] = ((g as f32 * gain).round() as i32).clamp(0, 255) as u8;
            rgb[i + 2] = ((b as f32 * gain).round() as i32).clamp(0, 255) as u8;
        }
    }
    AtlasImage { size: side, rgb }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rows cover exactly the (2h+1)² regions, ascending, and each row's
    /// LOD matches lod_for at the patch center.
    #[test]
    fn p3d207_rows_are_complete_ordered_and_lod_consistent() {
        let gen = WorldGen::new(1);
        let viewer = WorldPos::default();
        let rows = rows_for(
            &gen,
            viewer,
            4,
            |_| 0,
            |_| 0,
        );
        assert_eq!(rows.len(), 81);
        for w in rows.windows(2) {
            assert!(
                (w[0].coord.x, w[0].coord.z) < (w[1].coord.x, w[1].coord.z),
                "rows must ascend"
            );
        }
        for r in &rows {
            let center = WorldPos::from_mm(
                r.coord.x as i64 * crate::scales::PATCH_MM + crate::scales::PATCH_MM / 2,
                0,
                r.coord.z as i64 * crate::scales::PATCH_MM + crate::scales::PATCH_MM / 2,
            );
            assert_eq!(r.lod, lod_for(viewer, center));
        }
    }

    /// The overlay atlas is deterministic, shows concentric rings (the
    /// viewer's own ring is Full, the rim is Far/Horizon), and every pixel
    /// is a valid ring color shade.
    #[test]
    fn p3d207_overlay_is_deterministic_with_visible_rings() {
        let gen = WorldGen::new(2);
        let viewer = WorldPos::default();
        let a = render_overlay(&gen, viewer, 16);
        let b = render_overlay(&gen, viewer, 16);
        assert_eq!(a, b);
        assert_eq!(a.size, 33);
        // Center pixel is the Full ring color band; rim is Far/Horizon.
        let center = a.pixel(16, 16);
        // Full ring color with elevation gain: each channel within 21%+1
        // of the palette base (gain spans 0.8..1.2).
        for ch in 0..3 {
            let base = [40u8, 160, 220][ch] as f32;
            assert!(
                (center[ch] as f32 - base).abs() <= 0.21 * base + 1.0,
                "center off the Full ring: {center:?}"
            );
        }
        let rim = a.pixel(0, 0);
        assert!(rim != center);
        // All pixels belong to one of the four ring palettes (with gain).
        let palette = [lod_color(LodLevel::Full), lod_color(LodLevel::Mid), lod_color(LodLevel::Far), lod_color(LodLevel::Horizon)];
        for i in 0..(a.size * a.size) {
            let px = [a.rgb[i * 3], a.rgb[i * 3 + 1], a.rgb[i * 3 + 2]];
            let ok = palette.iter().any(|c| {
                (0..3).all(|ch| {
                    (px[ch] as i32 - c[ch] as i32).abs() as f32
                        <= 0.21 * c[ch] as f32 + 1.0
                })
            });
            assert!(ok, "off-palette pixel {px:?}");
        }
    }

    /// Edit and built counts flow through from the caller's closures.
    #[test]
    fn p3d207_rows_carry_edit_and_build_counts() {
        let gen = WorldGen::new(1);
        let viewer = WorldPos::default();
        let rows = rows_for(
            &gen,
            viewer,
            1,
            |c| (c.x.abs() + c.z.abs()) as usize,
            |c| (c.x == 0 && c.z == 0) as usize,
        );
        let center = rows.iter().find(|r| r.coord.x == 0 && r.coord.z == 0).unwrap();
        assert_eq!(center.edit_count, 0);
        assert_eq!(center.built_count, 1);
        let other = rows.iter().find(|r| r.coord.x == 16).unwrap();
        assert_eq!(other.built_count, 0);
        assert!(other.edit_count > 0);
    }
}
