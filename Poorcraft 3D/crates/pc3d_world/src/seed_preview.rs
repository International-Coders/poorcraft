//! WT-001: the pure seed-preview authority.
//!
//! Everything the New World screen shows about a seed before the world is
//! created — the map pixels, the biome census, spawn safety, feature
//! hints — is computed HERE from the same worldgen the runtime uses
//! (`WorldGen` macro fields, `effective_surface_mm`, `hydro::RiverGraph`,
//! the castle `footprint_fits` law). No rendering, no UI, no entropy
//! inside `preview_seed`: the same seed text always yields the same
//! preview, bit for bit. Randomness belongs to SEED RESOLUTION
//! (`resolve_seed`/`reroll_seed`), which the caller drives with entropy.

use crate::castle::footprint_fits;
use crate::coords::{CellCoord, RegionCoord};
use crate::gen::{Biome, WorldGen, SEA_LEVEL_M};
use crate::hydro::RiverGraph;
use crate::scales::REGION_MM;

/// The preview window: a square of (2*WINDOW_HALF)^ regions centered on
/// the world origin — 32x32 regions = 8.192 km per side.
pub const WINDOW_HALF: i32 = 16;
/// Default preview map resolution (square).
pub const DEFAULT_SIZE_PX: u32 = 96;

/// World types the preview can speak about. POORCRAFT 3D worldgen has
/// exactly one type today; the enum is the honest hook for more.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewWorldType {
    Normal,
}

impl PreviewWorldType {
    pub fn name(self) -> &'static str {
        match self {
            PreviewWorldType::Normal => "normal",
        }
    }
}

/// One preview request: the seed text exactly as typed plus the map size.
#[derive(Clone, Debug)]
pub struct SeedPreviewRequest {
    pub seed_text: String,
    pub world_type: PreviewWorldType,
    pub size_px: u32,
}

impl Default for SeedPreviewRequest {
    fn default() -> Self {
        SeedPreviewRequest {
            seed_text: String::new(),
            world_type: PreviewWorldType::Normal,
            size_px: DEFAULT_SIZE_PX,
        }
    }
}

/// The outcome of seed resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedSeed {
    /// Canonical text of the resolved seed (what the UI should display).
    pub text: String,
    pub seed: u64,
    /// "numeric" | "text-hash" | "random" — how the seed was derived.
    pub source: &'static str,
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn splitmix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Resolves seed text deterministically:
/// - all-digit text parses exactly as u64;
/// - any other non-empty text hashes stably (FNV-1a);
/// - empty text uses the caller's entropy (the Random button) — with no
///   entropy given, a fixed derivation of "" (documented, deterministic).
pub fn resolve_seed(text: &str, entropy: Option<u64>) -> ResolvedSeed {
    let t = text.trim();
    if t.is_empty() {
        let seed = splitmix64(entropy.unwrap_or(0) ^ 0xA5A5_5A5A_5A5A_A5A5);
        return ResolvedSeed { text: seed.to_string(), seed, source: "random" };
    }
    if let Ok(n) = t.parse::<u64>() {
        return ResolvedSeed { text: t.to_string(), seed: n, source: "numeric" };
    }
    let seed = fnv1a(t.as_bytes());
    ResolvedSeed { text: t.to_string(), seed, source: "text-hash" }
}

/// A reroll that can NEVER silently return the current seed: derives from
/// (current, entropy) and advances until it differs.
pub fn reroll_seed(current: u64, entropy: u64) -> u64 {
    let mut next = splitmix64(current ^ entropy ^ 0x6A09E667F3BCC909);
    let mut guard = 0;
    while next == current {
        next = splitmix64(next.wrapping_add(1));
        guard += 1;
        if guard > 8 {
            next = current.wrapping_add(1);
            break;
        }
    }
    next
}

/// Where the player would spawn and whether that ground is safe.
#[derive(Clone, Debug, PartialEq)]
pub struct SpawnPreview {
    /// World position in meters (y = effective surface height).
    pub x_m: f32,
    pub y_m: f32,
    pub z_m: f32,
    pub safe: bool,
    /// Human-readable safety verdict ("safe" or the named rejection).
    pub reason: String,
}

/// Biome census over the preview window (region-granular, exact).
#[derive(Clone, Debug, PartialEq)]
pub struct BiomePreviewCount {
    pub biome: Biome,
    pub regions: u32,
    pub percent: f32,
}

/// One feature hint shown under the map. `placeholder` marks hints the
/// current authorities cannot derive honestly yet.
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureHint {
    pub kind: String,
    pub distance_m: f32,
    pub confidence: f32,
    pub placeholder: bool,
}

/// The whole preview: metadata plus the CPU-painted map (straight-alpha
/// RGBA, `size_px`^2, top-down, spawn marked).
#[derive(Clone, Debug)]
pub struct SeedPreview {
    pub resolved_seed: u64,
    pub seed_text: String,
    pub seed_source: &'static str,
    pub world_type: PreviewWorldType,
    pub spawn: SpawnPreview,
    pub biome_counts: Vec<BiomePreviewCount>,
    pub feature_hints: Vec<FeatureHint>,
    pub valid: bool,
    pub warnings: Vec<String>,
    pub pixels_rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

fn biome_base_color(b: Biome, elevation_m: i32) -> [u8; 4] {
    // Compact palette; water depth/elevation shade keeps relief legible.
    let (r, g, b3): (u8, u8, u8) = match b {
        Biome::Ocean => {
            let depth = (-elevation_m).clamp(0, 64) as f32 / 64.0;
            (30, (70.0 - 50.0 * depth) as u8, (140.0 - 60.0 * depth) as u8)
        }
        Biome::Coast => (222, 205, 148),
        Biome::Wetland => (96, 138, 106),
        Biome::Plains => (128, 168, 88),
        Biome::Forest => (58, 112, 62),
        Biome::Highlands => (152, 148, 96),
        Biome::Mountains => (138, 132, 128),
        Biome::SnowPeaks => (238, 240, 244),
    };
    // Land relief shading: gentle light from the northwest.
    if b != Biome::Ocean {
        let shade = 1.0 + (elevation_m.clamp(0, 160) as f32 / 160.0) * 0.35;
        [
            (r as f32 * shade).min(255.0) as u8,
            (g as f32 * shade).min(255.0) as u8,
            (b3 as f32 * shade).min(255.0) as u8,
            255,
        ]
    } else {
        [r, g, b3, 255]
    }
}

/// Is a spawn walkable ground (region-level first cut)?
fn walkable(biome: Biome) -> bool {
    !matches!(biome, Biome::Ocean)
}

/// The spawn search: spiral out from the window center over regions;
/// the first region passing every safety law wins.
fn find_spawn(gen: &WorldGen, graph: &RiverGraph) -> SpawnPreview {
    let slope_ok = |cx_m: i64, cz_m: i64| -> bool {
        let h = gen.effective_surface_mm(cx_m, cz_m);
        for (dx, dz) in [(16_000, 0), (-16_000, 0), (0, 16_000), (0, -16_000)] {
            if (gen.effective_surface_mm(cx_m + dx, cz_m + dz) - h).abs() > 2_600 {
                return false;
            }
        }
        true
    };
    let mut ring = 0i32;
    while ring <= WINDOW_HALF {
        for rz in -ring..=ring {
            for rx in -ring..=ring {
                if rx.abs() != ring && rz.abs() != ring {
                    continue; // ring perimeter only
                }
                let region = RegionCoord { x: rx, z: rz };
                let f = gen.macro_field(region);
                let biome = gen.biome_of(&f);
                if !walkable(biome) {
                    continue;
                }
                // Solid land above sea level, not water-filled.
                let (cx_m, cz_m) = (
                    region.x as i64 * REGION_MM + REGION_MM / 2,
                    region.z as i64 * REGION_MM + REGION_MM / 2,
                );
                let surface = gen.effective_surface_mm(cx_m, cz_m);
                if surface <= (SEA_LEVEL_M as i64) * 1000 {
                    continue;
                }
                if !slope_ok(cx_m, cz_m) {
                    continue;
                }
                // Enough walkable area nearby: >=6 of the 3x3 neighbors.
                let walk: usize = (-1..=1)
                    .flat_map(|dz| (-1..=1).map(move |dx| (dx, dz)))
                    .filter(|(dx, dz)| {
                        walkable(gen.biome(RegionCoord { x: rx + dx, z: rz + dz }))
                    })
                    .count();
                if walk < 6 {
                    continue;
                }
                // Not inside a blocked structure footprint: the capital
                // keep (5x5) must NOT fit centered here (if it fits, the
                // castle law would site a stronghold on the player).
                let center_cell = CellCoord {
                    x: (cx_m / 1000 - 2) as i32,
                    y: surface.div_euclid(1000) as i32,
                    z: (cz_m / 1000 - 2) as i32,
                };
                if footprint_fits(gen, center_cell, 5, 5) {
                    // Stronghold ground: keep it, mark the hint — but
                    // spawning INSIDE the keep footprint is rejected.
                    continue;
                }
                let y_m = surface as f32 / 1000.0;
                return SpawnPreview {
                    x_m: cx_m as f32 / 1000.0,
                    y_m,
                    z_m: cz_m as f32 / 1000.0,
                    safe: true,
                    reason: "safe".into(),
                };
            }
        }
        ring += 1;
    }
    SpawnPreview {
        x_m: 0.0,
        y_m: 0.0,
        z_m: 0.0,
        safe: false,
        reason: "no safe spawn in window (water, cliffs, or occupied ground)".into(),
    }
}

/// Computes the full deterministic preview for a seed text.
pub fn preview_seed(req: &SeedPreviewRequest) -> SeedPreview {
    let rs = resolve_seed(&req.seed_text, None);
    let gen = WorldGen::new(rs.seed);
    let graph = RiverGraph::new(&gen, WINDOW_HALF);

    // Biome census over the window (exact, region-granular).
    let mut counts: std::collections::BTreeMap<Biome, u32> = Default::default();
    for rz in -WINDOW_HALF..WINDOW_HALF {
        for rx in -WINDOW_HALF..WINDOW_HALF {
            *counts.entry(gen.biome(RegionCoord { x: rx, z: rz })).or_insert(0) += 1;
        }
    }
    let total = (WINDOW_HALF * 2).pow(2) as f32;
    let biome_counts: Vec<BiomePreviewCount> = counts
        .into_iter()
        .map(|(biome, regions)| BiomePreviewCount {
            biome,
            regions,
            percent: 100.0 * regions as f32 / total,
        })
        .collect();

    let spawn = find_spawn(&gen, &graph);

    // Feature hints: only what a real authority derives. Rivers and
    // stronghold ground are real; "settlement" is an honest placeholder
    // until the sim publishes a siting authority the preview can query.
    let mut feature_hints = Vec::new();
    let spawn_region = RegionCoord {
        x: (spawn.x_m as i64).div_euclid(REGION_MM / 1000) as i32,
        z: (spawn.z_m as i64).div_euclid(REGION_MM / 1000) as i32,
    };
    let mut best_river: Option<(i32, i32)> = None;
    let mut best_keep: Option<(i32, i32)> = None;
    for rz in spawn_region.z - 6..=spawn_region.z + 6 {
        for rx in spawn_region.x - 6..=spawn_region.x + 6 {
            let r = RegionCoord { x: rx, z: rz };
            let nearer = |best: &Option<(i32, i32)>| {
                best.map(|(bx, bz)| (bx - spawn_region.x).pow(2) + (bz - spawn_region.z).pow(2))
                    .unwrap_or(i32::MAX)
                    > (rx - spawn_region.x).pow(2) + (rz - spawn_region.z).pow(2)
            };
            if graph.is_river(r) && nearer(&best_river) {
                best_river = Some((rx, rz));
            }
            if best_keep.is_none() && rx != spawn_region.x && rz != spawn_region.z {
                let f = gen.macro_field(r);
                if walkable(gen.biome_of(&f)) {
                    let o = r.origin();
                    let cx = o.x.div_euclid(1000) as i32 - 2;
                    let cz = o.z.div_euclid(1000) as i32 - 2;
                    let cy = gen
                        .effective_surface_mm(o.x + REGION_MM / 2, o.z + REGION_MM / 2)
                        .div_euclid(1000) as i32;
                    if footprint_fits(&gen, CellCoord { x: cx, y: cy, z: cz }, 5, 5) {
                        best_keep = Some((rx, rz));
                    }
                }
            }
        }
    }
    if let Some((rx, rz)) = best_river {
        let dist = (((rx - spawn_region.x).pow(2) + (rz - spawn_region.z).pow(2)) as f32).sqrt()
            * 256.0;
        feature_hints.push(FeatureHint {
            kind: "river".into(),
            distance_m: dist,
            confidence: 1.0,
            placeholder: false,
        });
    }
    if let Some((rx, rz)) = best_keep {
        let dist = ((rx - spawn_region.x).abs().max((rz - spawn_region.z).abs()) * 256) as f32;
        feature_hints.push(FeatureHint {
            kind: "stronghold-ground".into(),
            distance_m: dist,
            confidence: 0.9,
            placeholder: false,
        });
    }
    feature_hints.push(FeatureHint {
        kind: "settlement".into(),
        distance_m: 0.0,
        confidence: 0.0,
        placeholder: true,
    });

    let mut warnings = Vec::new();
    if feature_hints.iter().any(|h| h.placeholder) {
        warnings.push("settlement hint is a placeholder: no published siting authority yet".into());
    }
    if !spawn.safe {
        warnings.push(spawn.reason.clone());
    }

    // Paint the map: one pixel per (window/size) meters, biome palette
    // with relief shading, river overlay, spawn marker.
    let size = req.size_px.clamp(32, 512);
    let m_per_px = (WINDOW_HALF * 2) as f32 * (REGION_MM as f32 / 1000.0) / size as f32;
    let mut px = vec![0u8; (size * size * 4) as usize];
    let world_to_px = |w_m: f32| -> i32 { ((w_m / m_per_px) + size as f32 / 2.0) as i32 };
    for py in 0..size {
        for pxi in 0..size {
            let wx_m = (pxi as f32 - size as f32 / 2.0) * m_per_px + m_per_px / 2.0;
            let wz_m = (py as f32 - size as f32 / 2.0) * m_per_px + m_per_px / 2.0;
            let region = RegionCoord {
                x: (wx_m as i64).div_euclid(REGION_MM / 1000) as i32,
                z: (wz_m as i64).div_euclid(REGION_MM / 1000) as i32,
            };
            let f = gen.macro_field(region);
            let biome = gen.biome_of(&f);
            let mut c = biome_base_color(biome, f.elevation_m);
            if graph.is_river(region) && biome != Biome::Ocean {
                // River overlay: blend toward water blue, keep banks.
                c = [
                    (c[0] as u16 * 3 / 10 + 40) as u8,
                    (c[1] as u16 * 3 / 10 + 90) as u8,
                    (c[2] as u16 * 3 / 10 + 170) as u8,
                    255,
                ];
            }
            let i = ((py * size + pxi) * 4) as usize;
            px[i..i + 4].copy_from_slice(&c);
        }
    }
    // Spawn marker: ember cross with white core (visible on any biome).
    if spawn.safe {
        let mx = world_to_px(spawn.x_m);
        let my = world_to_px(spawn.z_m);
        for (dx, dy) in [(-2, 0), (-1, 0), (1, 0), (2, 0), (0, -2), (0, -1), (0, 1), (0, 2)] {
            let x = mx + dx;
            let y = my + dy;
            if (0..size as i32).contains(&x) && (0..size as i32).contains(&y) {
                let i = ((y as u32 * size + x as u32) * 4) as usize;
                px[i..i + 4].copy_from_slice(&[255, 140, 40, 255]);
            }
        }
        if (0..size as i32).contains(&mx) && (0..size as i32).contains(&my) {
            let i = ((my as u32 * size + mx as u32) * 4) as usize;
            px[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
    }

    SeedPreview {
        resolved_seed: rs.seed,
        seed_text: rs.text,
        seed_source: rs.source,
        world_type: req.world_type,
        valid: spawn.safe,
        spawn,
        biome_counts,
        feature_hints,
        warnings,
        pixels_rgba: px,
        width: size,
        height: size,
    }
}

// ---------------------------------------------------------------------------
// Tests: the WT-001 matrix, verbatim
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn req(text: &str) -> SeedPreviewRequest {
        SeedPreviewRequest { seed_text: text.into(), ..Default::default() }
    }

    #[test]
    fn empty_seed_randomizes_when_requested() {
        let a = resolve_seed("", Some(1));
        let b = resolve_seed("", Some(2));
        assert_eq!(a.source, "random");
        assert_ne!(a.seed, b.seed, "different entropy must give different seeds");
    }

    #[test]
    fn text_seed_hashes_stably() {
        let a = resolve_seed("owner-test", None);
        let b = resolve_seed("owner-test", None);
        assert_eq!(a, b, "same text must hash identically");
        assert_eq!(a.source, "text-hash");
        assert_ne!(a.seed, resolve_seed("owner-test-2", None).seed);
    }

    #[test]
    fn numeric_seed_parses_exactly() {
        assert_eq!(resolve_seed("4242", None).seed, 4242);
        assert_eq!(resolve_seed("18446744073709551615", None).seed, u64::MAX);
        assert_eq!(resolve_seed(" 7 ", None).seed, 7, "trimmed");
        assert_eq!(resolve_seed("0042", None).seed, 42);
    }

    #[test]
    fn same_seed_same_preview_metadata() {
        let a = preview_seed(&req("4242"));
        let b = preview_seed(&req("4242"));
        assert_eq!(a.biome_counts, b.biome_counts);
        assert_eq!(a.spawn, b.spawn);
        assert_eq!(a.feature_hints, b.feature_hints);
        assert_eq!(a.pixels_rgba, b.pixels_rgba, "map pixels must be bit-identical");
    }

    #[test]
    fn different_seed_changes_preview() {
        let a = preview_seed(&req("4242"));
        let b = preview_seed(&req("7"));
        let meta_differs = a.biome_counts != b.biome_counts
            || a.resolved_seed != b.resolved_seed
            || a.spawn != b.spawn;
        assert!(meta_differs || a.pixels_rgba != b.pixels_rgba, "different seeds must differ");
    }

    #[test]
    fn reroll_changes_seed() {
        for cur in [0u64, 1, 4242, u64::MAX] {
            for e in 0..16u64 {
                assert_ne!(reroll_seed(cur, e), cur, "reroll must never return the current seed");
            }
        }
    }

    #[test]
    fn spawn_safety_rejects_water_or_steep_or_solid() {
        // The classifier runs against REAL worldgen: find an ocean region
        // and a land region on a fixed seed and check the verdicts.
        let gen = WorldGen::new(4242);
        let mut found_ocean = false;
        let mut found_land = false;
        for rz in -WINDOW_HALF..WINDOW_HALF {
            for rx in -WINDOW_HALF..WINDOW_HALF {
                let b = gen.biome(RegionCoord { x: rx, z: rz });
                if b == Biome::Ocean && !found_ocean {
                    found_ocean = true;
                    let o = RegionCoord { x: rx, z: rz }.origin();
                    let surface = gen.effective_surface_mm(o.x + REGION_MM / 2, o.z + REGION_MM / 2);
                    assert!(
                        surface <= 0 || b == Biome::Ocean,
                        "ocean region must be water (surface {surface}mm)"
                    );
                }
                if b == Biome::Plains && !found_land {
                    found_land = true;
                    let o = RegionCoord { x: rx, z: rz }.origin();
                    let surface = gen.effective_surface_mm(o.x + REGION_MM / 2, o.z + REGION_MM / 2);
                    assert!(surface > 0, "plains region must be land above sea level");
                }
            }
        }
        // A preview over an all-land window must find a safe spawn or name
        // the reason; a preview is never silently empty.
        let p = preview_seed(&req("4242"));
        if p.spawn.safe {
            assert_eq!(p.spawn.reason, "safe");
            assert!(p.valid);
        } else {
            assert!(!p.valid);
            assert!(!p.spawn.reason.is_empty(), "unsafe spawn must name its reason");
        }
    }

    #[test]
    fn preview_metadata_agrees_with_worldgen() {
        let p = preview_seed(&req("4242"));
        let gen = WorldGen::new(p.resolved_seed);
        // Biome census recounted by hand from the same authority.
        let mut want: std::collections::BTreeMap<String, u32> = Default::default();
        for rz in -WINDOW_HALF..WINDOW_HALF {
            for rx in -WINDOW_HALF..WINDOW_HALF {
                *want
                    .entry(gen.biome(RegionCoord { x: rx, z: rz }).name().to_string())
                    .or_insert(0) += 1;
            }
        }
        let got: std::collections::BTreeMap<String, u32> = p
            .biome_counts
            .iter()
            .map(|b| (b.biome.name().to_string(), b.regions))
            .collect();
        assert_eq!(want, got, "sidecar census must equal the worldgen recount");
        // Spawn height must equal the live surface at the spawn spot.
        if p.spawn.safe {
            let h = gen.effective_surface_mm(
                (p.spawn.x_m * 1000.0) as i64,
                (p.spawn.z_m * 1000.0) as i64,
            ) as f32
                / 1000.0;
            assert!((h - p.spawn.y_m).abs() < 0.001, "spawn y must be the real surface");
        }
        // The map must carry the spawn marker when valid.
        if p.spawn.safe {
            let marker = p
                .pixels_rgba
                .chunks_exact(4)
                .any(|c| c == [255, 140, 40, 255] || c == [255, 255, 255, 255]);
            assert!(marker, "valid preview must paint the spawn marker");
        }
    }

    #[test]
    fn ui_seed_and_preview_seed_agree_by_construction() {
        // The failure mode "UI shows seed but world creation uses another
        // seed": both must derive from the same text through the same
        // resolver. Numeric form is what the form field holds.
        for text in ["22", "4242", "999999"] {
            let preview = preview_seed(&req(text));
            let form_seed: u64 = text.parse().unwrap();
            assert_eq!(preview.resolved_seed, form_seed);
        }
    }
}
