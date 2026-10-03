//! The generator memo (PERF-101): one bounded cache that answers the
//! generator's three hot questions — a region's macro field, a region's
//! biome, a column's effective surface — without re-deriving the noise.
//!
//! THE LAW (value identity): every cached answer is bit-equal to the pure
//! generator call it memoizes. The cache never changes what the world
//! looks like; it only stops recomputing it. `final_solid` through the
//! cache therefore keeps the P3D-202 contract (it agrees with
//! `regenerate_patch` cell-for-cell) because both read the same pure
//! functions underneath.
//!
//! Bounds: the surface map is the big one (one entry per queried column);
//! it clears itself at [`GenCache::SURFACE_CAP`] entries so a long session
//! cannot grow it without limit. Region maps are tiny by construction.

use crate::coords::RegionCoord;
use crate::gen::{Biome, MacroField, WorldGen};
use crate::terrain::SolidAnswer;
use std::cell::RefCell;
use std::collections::HashMap;

/// Memoized answers for ONE seed's generator. Not `Send`/`Sync` on purpose:
/// the renderer is single-threaded and the cache is a per-consumer working
/// set, not shared state.
#[derive(Default)]
pub struct GenCache {
    fields: RefCell<HashMap<i64, MacroField>>,
    biomes: RefCell<HashMap<i64, Biome>>,
    surface: RefCell<HashMap<(i64, i64), i64>>,
}

impl GenCache {
    /// Column entries before the surface map self-clears (65,536 columns
    /// ≈ a 256×256 m sweep — one cache generation of streaming work).
    const SURFACE_CAP: usize = 1 << 16;
    /// Region entries before the region maps self-clear.
    const REGION_CAP: usize = 1 << 12;

    pub fn new() -> Self {
        Self::default()
    }

    fn region_key(region: RegionCoord) -> i64 {
        ((region.x as i64) << 32) | (region.z as i64 & 0xffff_ffff)
    }

    /// Macro field for a region — memoized (`WorldGen::macro_field` value).
    pub fn macro_field(&self, gen: &WorldGen, region: RegionCoord) -> MacroField {
        let key = Self::region_key(region);
        if let Some(f) = self.fields.borrow().get(&key) {
            return *f;
        }
        let f = gen.macro_field(region);
        let mut fields = self.fields.borrow_mut();
        if fields.len() >= Self::REGION_CAP {
            fields.clear();
        }
        fields.insert(key, f);
        f
    }

    /// Biome for a region — memoized (`WorldGen::biome` value, pockets
    /// included; the pocket field is part of the memoized answer).
    pub fn biome(&self, gen: &WorldGen, region: RegionCoord) -> Biome {
        let key = Self::region_key(region);
        if let Some(b) = self.biomes.borrow().get(&key) {
            return *b;
        }
        let b = gen.biome(region);
        let mut biomes = self.biomes.borrow_mut();
        if biomes.len() >= Self::REGION_CAP {
            biomes.clear();
        }
        biomes.insert(key, b);
        b
    }

    /// Effective surface (mm) for a world column — memoized.
    pub fn effective_surface_mm(&self, gen: &WorldGen, wx: i64, wz: i64) -> i64 {
        let key = (wx, wz);
        if let Some(s) = self.surface.borrow().get(&key) {
            return *s;
        }
        let s = gen.effective_surface_mm(wx, wz);
        let mut surface = self.surface.borrow_mut();
        if surface.len() >= Self::SURFACE_CAP {
            surface.clear();
        }
        surface.insert(key, s);
        s
    }

    /// THE authoritative cell answer through the cache — bit-equal to
    /// [`crate::terrain::final_solid`] (proved by law test), without the
    /// repeated biome/surface re-derivation.
    pub fn final_solid(&self, gen: &WorldGen, wx: i64, wy: i64, wz: i64) -> SolidAnswer {
        let surface_mm = self.effective_surface_mm(gen, wx, wz);
        let region = crate::coords::WorldPos::from_mm(wx, wy, wz).region();
        let biome = self.biome(gen, region);
        let depth_mm = surface_mm - wy;
        let material = gen.carve(crate::gen::cell_material(biome, wy, surface_mm), wx, wy, wz, depth_mm);
        SolidAnswer {
            solid: !matches!(material, crate::gen::CellMaterial::Air | crate::gen::CellMaterial::Water),
            material,
        }
    }

    /// The renderer's surface-material sample (the top-meter trick the
    /// mesher uses): bit-equal to `final_solid(gen, wx, surface−500).material`.
    pub fn surface_material(
        &self,
        gen: &WorldGen,
        wx: i64,
        wz: i64,
    ) -> crate::gen::CellMaterial {
        let surface_mm = self.effective_surface_mm(gen, wx, wz);
        self.final_solid(gen, wx, surface_mm.saturating_sub(500), wz)
            .material
    }

    /// Drop everything (used by tests and by consumers that jump the
    /// viewer a long way in one step).
    pub fn clear(&self) {
        self.fields.borrow_mut().clear();
        self.biomes.borrow_mut().clear();
        self.surface.borrow_mut().clear();
    }

    /// Live entry counts (tests + streamer counters).
    pub fn len(&self) -> (usize, usize, usize) {
        (
            self.fields.borrow().len(),
            self.biomes.borrow().len(),
            self.surface.borrow().len(),
        )
    }

    pub fn is_empty(&self) -> bool {
        let (f, b, s) = self.len();
        f == 0 && b == 0 && s == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::final_solid;

    /// THE value-identity law: through a warm cache, every answer is
    /// bit-equal to the pure generator's — surfaces, biomes, macro fields,
    /// and full final-solid cells, across seeds and a coordinate sweep.
    #[test]
    fn perf101_cached_answers_equal_pure_answers() {
        for seed in [7u64, 4242, 999983] {
            let gen = WorldGen::new(seed);
            let cache = GenCache::new();
            for x in -3..=3 {
                for z in -3..=3 {
                    let region = RegionCoord { x, z };
                    assert_eq!(cache.macro_field(&gen, region), gen.macro_field(region));
                    assert_eq!(cache.biome(&gen, region), gen.biome(region));
                    for dx in 0..17 {
                        for dz in 0..17 {
                            let wx = (x * 256 + dx) as i64 * 1000;
                            let wz = (z * 256 + dz) as i64 * 1000;
                            assert_eq!(
                                cache.effective_surface_mm(&gen, wx, wz),
                                gen.effective_surface_mm(wx, wz)
                            );
                            for dy in [0i64, 12_000, 40_000, 90_000] {
                                let pure = final_solid(&gen, wx, dy, wz);
                                let cached = cache.final_solid(&gen, wx, dy, wz);
                                assert_eq!(pure, cached, "seed {seed} at {wx},{dy},{wz}");
                            }
                            assert_eq!(
                                cache.surface_material(&gen, wx, wz),
                                final_solid(
                                    &gen,
                                    wx,
                                    gen.effective_surface_mm(wx, wz).saturating_sub(500),
                                    wz
                                )
                                .material
                            );
                        }
                    }
                }
            }
        }
    }

    /// A cold cache answers exactly like the pure generator (no stale
    /// warm-up dependence), and `clear()` returns it to that state.
    #[test]
    fn perf101_cache_clears_to_cold_identity() {
        let gen = WorldGen::new(31);
        let cache = GenCache::new();
        let wx = 4_000i64;
        let wz = -9_000i64;
        let warm = cache.effective_surface_mm(&gen, wx, wz);
        assert_eq!(warm, gen.effective_surface_mm(wx, wz));
        assert!(!cache.is_empty());
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.effective_surface_mm(&gen, wx, wz), warm);
    }

    /// The bounds law: flooding the surface map past its cap clears it
    /// instead of growing without limit.
    #[test]
    fn perf101_surface_map_self_clears_at_cap() {
        let gen = WorldGen::new(5);
        let cache = GenCache::new();
        // One past the cap, from a fixed spread of columns.
        let n = (GenCache::SURFACE_CAP + 1) as i64;
        for i in 0..n {
            let wx = (i % 4096) * 1000;
            let wz = (i / 4096) * 1000;
            cache.effective_surface_mm(&gen, wx, wz);
        }
        let (_, _, surface_entries) = cache.len();
        assert!(
            surface_entries <= GenCache::SURFACE_CAP,
            "surface map grew past its cap: {surface_entries}"
        );
    }
}
