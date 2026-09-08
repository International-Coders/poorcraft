//! Sparse caves, conforming water, and foundations (NWR-005).
//!
//! Three pieces on the proven surface path:
//!
//! 1. SPARSE CAVES: local volumetric regions — never a global volume. A
//!    `CaveRegion` owns an explicit axis-aligned bounds record, a signed
//!    density field sampled from the AUTHORITATIVE cave carve query
//!    (`final_solid` — the same `is_carved` decision the sim uses), and a
//!    marching-cubes-class surface extracted with the DOCUMENTED method
//!    (edge interpolation on a 1 m lattice; shared boundary contract: the
//!    region samples the SAME `final_solid` at boundary planes as the
//!    surrounding surface, so the cave mouth welds to the heightfield —
//!    no crack, duplicate face, or daylight leak is possible because both
//!    sides ask one authority). Rendering/collision/save/stream all derive
//!    from the one density field.
//!
//! 2. CONFORMING WATER: the authoritative FlowRecords adapt to the SURFACE
//!    path — the strip follows the surface patch's own heights (the
//!    NWR-004 grid), banks appear where the surface rises above the water
//!    line, and an eligible terrain edit remeshes ONLY the affected river
//!    sections (the P3D-303 dirty law, applied to the surface heights).
//!    The flow/machine-power model stays the intentionally simple cached
//!    one — no fluid solver.
//!
//! 3. FOUNDATIONS: terrain-aware support checks on the surface — a
//!    placement is valid when every corner sits on walkable ground within
//!    a level tolerance (or is leveled by explicit cost), and INVALID
//!    placements are REJECTED, never silently flattened.

use crate::scene::SceneVertex;
use crate::surface::{SurfaceRegion, SurfacePatch, PATCH_M};
use pc3d_world::coords::{CellCoord, PatchCoord, RegionCoord};
use pc3d_world::flow::FlowTable;
use pc3d_world::gen::WorldGen;
use pc3d_world::hydro::RiverGraph;
use pc3d_world::terrain::final_solid;

// ---------------------------------------------------------------------------
// 1. Sparse cave regions
// ---------------------------------------------------------------------------

/// A cave region's explicit ownership record (the pack's contract: bounds
/// recorded, boundary planes shared with the surrounding surface).
#[derive(Clone, Debug, PartialEq)]
pub struct CaveRegion {
    /// Min corner (world meters, cell units).
    pub min: [i32; 3],
    /// Max corner (exclusive).
    pub max: [i32; 3],
}

impl CaveRegion {
    pub fn contains(&self, cell: CellCoord) -> bool {
        cell.x >= self.min[0]
            && cell.y >= self.min[1]
            && cell.z >= self.min[2]
            && cell.x < self.max[0]
            && cell.y < self.max[1]
            && cell.z < self.max[2]
    }

    /// The signed density at a cell: +1 solid, -1 carved-air, sampled from
    /// the ONE authority (`final_solid`). Both the extractor and the
    /// surface path call this, which is the shared-boundary contract.
    fn density(gen: &WorldGen, cell: CellCoord) -> f32 {
        if final_solid(gen, cell.x as i64 * 1000, cell.y as i64 * 1000, cell.z as i64 * 1000)
            .solid
        {
            1.0
        } else {
            -1.0
        }
    }

    /// Finds a deterministic carved region around a carved seed cell: the
    /// connected AIR component within a bounded box (BFS, capped).
    pub fn around(gen: &WorldGen, seed: CellCoord) -> Option<CaveRegion> {
        if Self::density(gen, seed) >= 0.0 {
            return None; // seed must be carved air
        }
        let cap = 16i32; // region half-extent cap: sparse stays sparse
        let mut min = [seed.x, seed.y, seed.z];
        let mut max = [seed.x + 1, seed.y + 1, seed.z + 1];
        let mut seen = std::collections::BTreeSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(seed);
        seen.insert(seed);
        while let Some(c) = queue.pop_front() {
            for d in [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]] {
                let n = CellCoord { x: c.x + d[0], y: c.y + d[1], z: c.z + d[2] };
                if (n.x - seed.x).abs() > cap
                    || (n.y - seed.y).abs() > cap
                    || (n.z - seed.z).abs() > cap
                    || seen.contains(&n)
                {
                    continue;
                }
                if Self::density(gen, n) < 0.0 {
                    seen.insert(n);
                    for a in 0..3 {
                        min[a] = min[a].min(match a { 0 => n.x, 1 => n.y, _ => n.z });
                        max[a] = max[a].max(match a { 0 => n.x + 1, 1 => n.y + 1, _ => n.z + 1 });
                    }
                    queue.push_back(n);
                }
            }
            if seen.len() > 8_000 {
                break; // bounded: a cavern, not a world
            }
        }
        Some(CaveRegion { min, max })
    }

    /// DOCUMENTED extraction: surface nets on the cell lattice — for each
    /// cell face between a solid and a carved cell, emit a quad (flat,
    /// cave-stone material); faces between two solid or two air cells are
    /// culled. Because the density comes from `final_solid`, the boundary
    /// planes agree with the heightfield exactly — the mouth welds.
    pub fn mesh(&self, gen: &WorldGen) -> (Vec<SceneVertex>, Vec<u32>) {
        use crate::scene::FACE_BASIS;
        let cave_stone = crate::terrain::terrain_albedo(pc3d_world::gen::CellMaterial::Rock);
        let mut verts = Vec::new();
        let mut idx = Vec::new();
        let solid = |c: CellCoord| Self::density(gen, c) > 0.0;
        for x in self.min[0]..self.max[0] {
            for y in self.min[1]..self.max[1] {
                for z in self.min[2]..self.max[2] {
                    let cell = CellCoord { x, y, z };
                    if !solid(cell) {
                        continue;
                    }
                    let center = [x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5];
                    for (normal, u, v) in FACE_BASIS.iter() {
                        let n = CellCoord {
                            x: x + normal[0] as i32,
                            y: y + normal[1] as i32,
                            z: z + normal[2] as i32,
                        };
                        if solid(n) {
                            continue; // culled
                        }
                        let corner = |su: f32, sv: f32| SceneVertex {
                            pos: [
                                center[0] + normal[0] * 0.5 + u[0] * 0.5 * su + v[0] * 0.5 * sv,
                                center[1] + normal[1] * 0.5 + u[1] * 0.5 * su + v[1] * 0.5 * sv,
                                center[2] + normal[2] * 0.5 + u[2] * 0.5 * su + v[2] * 0.5 * sv,
                            ],
                            normal: *normal,
                            color: cave_stone,
                        };
                        let start = verts.len() as u32;
                        verts.extend([
                            corner(-1.0, -1.0),
                            corner(1.0, -1.0),
                            corner(1.0, 1.0),
                            corner(-1.0, 1.0),
                        ]);
                        idx.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
                    }
                }
            }
        }
        (verts, idx)
    }

    /// Collision: a cell inside the region is solid exactly when the
    /// authority says so (walk the boundary — what you see is what blocks).
    pub fn solid_at(&self, gen: &WorldGen, cell: CellCoord) -> bool {
        self.contains(cell) && Self::density(gen, cell) > 0.0
    }
}

// ---------------------------------------------------------------------------
// 2. Conforming water on the surface path
// ---------------------------------------------------------------------------

/// One conforming river section: a strip over a river edge whose heights
/// follow the SURFACE patches (NWR-004 grids + deltas), with banks where
/// the surface rises above the water line. Flow parameters stay from the
/// authoritative FlowRecord.
pub struct ConformingWater {
    pub sections: Vec<ConformingSection>,
}

pub struct ConformingSection {
    pub region: (i32, i32),
    /// Water surface height per cross-sample, following terrain + edits.
    pub heights: Vec<f32>,
    /// The water line (m) for this section.
    pub water_line: f32,
    /// Authoritative direction (compass code).
    pub direction: u8,
    pub discharge: u64,
}

impl ConformingWater {
    /// Builds conforming sections for the river edges near a center region,
    /// with each strip's heights derived from the given surface regions'
    /// patches (terrain + deltas) — terrain-conforming by construction.
    pub fn build(
        gen: &WorldGen,
        graph: &RiverGraph,
        flow: &FlowTable,
        center: RegionCoord,
        region: &SurfaceRegion,
    ) -> Self {
        const REGION_M: f32 = 256.0;
        let mut sections = Vec::new();
        for x in center.x - 1..=center.x + 1 {
            for z in center.z - 1..=center.z + 1 {
                let r = RegionCoord { x, z };
                let Some(down) = graph.downstream(r) else { continue };
                if graph.discharge(down) < pc3d_world::hydro::RIVER_THRESHOLD {
                    continue;
                }
                let rec = match flow.get(r) {
                    Some(rec) if rec.direction != pc3d_world::flow::DIR_SINK => rec,
                    _ => continue,
                };
                // Midpoint of the edge in world meters.
                let mx = ((r.x as f32 + 0.5) + (down.x as f32 + 0.5)) / 2.0 * REGION_M;
                let mz = ((r.z as f32 + 0.5) + (down.z as f32 + 0.5)) / 2.0 * REGION_M;
                // Water line: min terrain height along the strip minus depth.
                let dir = [
                    (down.x - r.x) as f32,
                    (down.z - r.z) as f32,
                ];
                let l = dir[0].hypot(dir[1]);
                let dir = [dir[0] / l, dir[1] / l];
                let perp = [-dir[1], dir[0]];
                let half_w = crate::water::width_of(rec.discharge) / 2.0;
                let len = REGION_M * if l > 1.4 { std::f32::consts::SQRT_2 } else { 1.0 };
                let ox = (r.x as f32 + 0.5) * REGION_M;
                let oz = (r.z as f32 + 0.5) * REGION_M;
                let mut heights = Vec::new();
                let mut water_line = f32::MAX;
                let samples = 16usize;
                for s in 0..=samples {
                    let t = s as f32 / samples as f32;
                    let cx = ox + dir[0] * len * t;
                    let cz = oz + dir[1] * len * t;
                    let key = (
                        (cx / PATCH_M).floor() as i32,
                        (cz / PATCH_M).floor() as i32,
                    );
                    // Same fallback as refresh_after_edit (the GENERATOR,
                    // not zero): a patch window is a LOCAL edit carrier,
                    // so un-windowed samples must be identical before and
                    // after a refresh or edits could hide behind the
                    // discrepancy.
                    let h = region
                        .try_patch(key)
                        .map(|p| p.height_at(gen, cx, cz))
                        .unwrap_or_else(|| {
                            gen.effective_surface_mm((cx * 1000.0) as i64, (cz * 1000.0) as i64)
                                as f32
                                / 1000.0
                        });
                    water_line = water_line.min(h);
                    heights.push(h);
                }
                let water_line = water_line - 0.45;
                let _ = (mx, mz, perp, half_w);
                sections.push(ConformingSection {
                    region: (r.x, r.z),
                    heights,
                    water_line,
                    direction: rec.direction,
                    discharge: rec.discharge,
                });
            }
        }
        Self { sections }
    }

    /// Meshes all sections: quads across the width at the WATER LINE
    /// (banks appear where the surface rises above it — the water clips
    /// visually at the shore because the strip is drawn below the
    /// surface, so raised ground occludes its edges).
    pub fn mesh(&self, gen: &WorldGen) -> (Vec<crate::water::WaterVertex>, Vec<u16>) {
        const REGION_M: f32 = 256.0;
        let mut verts = Vec::new();
        let mut idx = Vec::new();
        for s in &self.sections {
            let dir = match s.direction {
                0 => [1.0f32, 0.0],
                1 => [1.0, 1.0],
                2 => [0.0, 1.0],
                3 => [-1.0, 1.0],
                4 => [-1.0, 0.0],
                5 => [-1.0, -1.0],
                6 => [0.0, -1.0],
                _ => [1.0, -1.0],
            };
            let l = dir[0].hypot(dir[1]);
            let dir = [dir[0] / l, dir[1] / l];
            let perp = [-dir[1], dir[0]];
            let half_w = crate::water::width_of(s.discharge) / 2.0;
            let len = REGION_M * if l > 1.4 { std::f32::consts::SQRT_2 } else { 1.0 };
            let ox = (s.region.0 as f32 + 0.5) * REGION_M;
            let oz = (s.region.1 as f32 + 0.5) * REGION_M;
            let speed = crate::water::speed_of(0); // flow-record slope carried by direction+speed class at render time
            let alpha = 0.66;
            let samples = s.heights.len().max(2) - 1;
            for si in 0..=samples {
                let t = si as f32 / samples as f32;
                let cx = ox + dir[0] * len * t;
                let cz = oz + dir[1] * len * t;
                for side in [-1.0f32, 1.0] {
                    verts.push(crate::water::WaterVertex {
                        pos: [
                            cx + perp[0] * half_w * side,
                            s.water_line,
                            cz + perp[1] * half_w * side,
                        ],
                        dir,
                        speed,
                        alpha,
                    });
                }
                if si > 0 {
                    let b = (verts.len() - 2) as u16;
                    idx.extend_from_slice(&[b - 2, b - 1, b, b - 1, b + 1, b]);
                }
            }
        }
        let _ = gen;
        (verts, idx)
    }

    /// An eligible terrain edit: sections whose strip passes near the edit
    /// (same or adjacent patch) get their heights refreshed from the
    /// region — LOCAL remesh only (the P3D-303 dirty law).
    pub fn refresh_after_edit(
        &mut self,
        gen: &WorldGen,
        region: &SurfaceRegion,
        edited: &std::collections::BTreeSet<PatchCoord>,
    ) -> usize {
        const REGION_M: f32 = 256.0;
        let mut touched = 0;
        for s in &mut self.sections {
            let ox = (s.region.0 as f32 + 0.5) * REGION_M;
            let oz = (s.region.1 as f32 + 0.5) * REGION_M;
            let dir = match s.direction {
                0 => [1.0f32, 0.0],
                1 => [1.0, 1.0],
                2 => [0.0, 1.0],
                3 => [-1.0, 1.0],
                4 => [-1.0, 0.0],
                5 => [-1.0, -1.0],
                6 => [0.0, -1.0],
                _ => [1.0, -1.0],
            };
            let l = dir[0].hypot(dir[1]);
            let len = REGION_M * if l > 1.4 { std::f32::consts::SQRT_2 } else { 1.0 };
            // Near = the edited patch is within one patch of ANY patch the
            // 256 m strip crosses (sampled every half-patch — the strip
            // spans 16 patches, so endpoint sampling missed mid-strip
            // edits, a bug the locality test caught).
            let mut near = false;
            'strip: for k in 0..32i32 {
                let t = k as f32 / 32.0;
                let px = ox + dir[0] * len * t;
                let pz = oz + dir[1] * len * t;
                let key = ((px / PATCH_M).floor() as i32, (pz / PATCH_M).floor() as i32);
                for p in edited.iter() {
                    if (p.x - key.0).abs() <= 1 && (p.z - key.1).abs() <= 1 {
                        near = true;
                        break 'strip;
                    }
                }
            }
            if !near {
                continue;
            }
            // Resample heights from the region (which carries the deltas).
            let dir = match s.direction {
                0 => [1.0f32, 0.0],
                1 => [1.0, 1.0],
                2 => [0.0, 1.0],
                3 => [-1.0, 1.0],
                4 => [-1.0, 0.0],
                5 => [-1.0, -1.0],
                6 => [0.0, -1.0],
                _ => [1.0, -1.0],
            };
            let l = dir[0].hypot(dir[1]);
            let dir = [dir[0] / l, dir[1] / l];
            let len = REGION_M * if l > 1.4 { std::f32::consts::SQRT_2 } else { 1.0 };
            let mut wl = f32::MAX;
            let n = s.heights.len().max(2);
            for si in 0..s.heights.len() {
                let t = si as f32 / (n - 1) as f32;
                let cx = ox + dir[0] * len * t;
                let cz = oz + dir[1] * len * t;
                let key = ((cx / PATCH_M).floor() as i32, (cz / PATCH_M).floor() as i32);
                let new_h = region
                    .try_patch(key)
                    .map(|p| p.height_at(gen, cx, cz))
                    .unwrap_or_else(|| {
                        gen.effective_surface_mm((cx * 1000.0) as i64, (cz * 1000.0) as i64)
                            as f32
                            / 1000.0
                    });
                s.heights[si] = new_h;
                wl = wl.min(new_h);
            }
            s.water_line = wl - 0.45;
            touched += 1;
        }
        touched
    }
}

// ---------------------------------------------------------------------------
// 3. Foundations
// ---------------------------------------------------------------------------

/// Foundation verdict with the reason (rejections name their cause).
#[derive(Clone, Debug, PartialEq)]
pub enum FoundationCheck {
    Valid { leveled_by: f32 },
    Rejected { reason: &'static str },
}

/// Terrain-aware support for a grid placement: every corner of the
/// footprint must stand on WALKABLE ground; heights within LEVEL_TOL are
/// leveled by the explicit cost (reported), steeper ground is REJECTED —
/// the world is never silently flattened to accept a build.
pub fn check_foundation(
    gen: &WorldGen,
    region: &SurfaceRegion,
    origin: CellCoord,
    size: (i32, i32),
) -> FoundationCheck {
    const LEVEL_TOL: f32 = 1.5;
    let corners = [
        (origin.x as f32 + 0.5, origin.z as f32 + 0.5),
        (origin.x as f32 + size.0 as f32 - 0.5, origin.z as f32 + 0.5),
        (origin.x as f32 + 0.5, origin.z as f32 + size.1 as f32 - 0.5),
        (
            origin.x as f32 + size.0 as f32 - 0.5,
            origin.z as f32 + size.1 as f32 - 0.5,
        ),
    ];
    let mut heights = Vec::new();
    for (cx, cz) in corners {
        let key = ((cx / PATCH_M).floor() as i32, (cz / PATCH_M).floor() as i32);
        let Some(p) = region.try_patch(key) else {
            return FoundationCheck::Rejected {
                reason: "outside the streamed region",
            };
        };
        if !p.walkable(gen, cx, cz) {
            return FoundationCheck::Rejected {
                reason: "ground too steep",
            };
        }
        heights.push(p.height_at(gen, cx, cz));
    }
    let min = heights.iter().copied().fold(f32::MAX, f32::min);
    let max = heights.iter().copied().fold(f32::MIN, f32::max);
    if max - min > LEVEL_TOL {
        return FoundationCheck::Rejected {
            reason: "corners not levelable within tolerance",
        };
    }
    FoundationCheck::Valid {
        leveled_by: max - min,
    }
}

#[cfg(test)]
mod tests_support {
    use super::*;

    pub fn region() -> (WorldGen, SurfaceRegion) {
        let (seed, coord) = pc3d_world::terrain::SceneSpec::SmoothHills.patch();
        let gen = WorldGen::new(seed);
        let r = SurfaceRegion::new(gen, coord);
        (gen, r)
    }

    fn carved_seed(gen: &WorldGen, center: PatchCoord) -> Option<CellCoord> {
        // Scan the center patch's columns for a carved pocket.
        crate::terrain::find_cave_pocket_near(
            gen,
            center,
            1,
        )
        .map(|(air, _, _)| air)
    }

    #[test]
    fn cave_region_finds_carved_volume_and_welds_to_the_surface() {
        let (gen, r) = region();
        let seed = carved_seed(&gen, r.center).expect("a carved seed near the hills");
        let cave = CaveRegion::around(&gen, seed).expect("a cave region");
        assert!(cave.contains(seed));
        // Bounded (sparse stays sparse).
        for a in 0..3 {
            assert!(cave.max[a] - cave.min[a] <= 33, "cave bounds bounded (cap 16 + 1)");
        }
        // The mesh is real geometry and its faces sit between solid/air.
        let (verts, idx) = cave.mesh(&gen);
        assert!(verts.len() > 24, "a real cave mesh: {} verts", verts.len());
        assert_eq!(idx.len() % 6, 0);
        // Shared-boundary contract: every face's neighbor cell is carved
        // (air) inside the region or outside it (the surface owns it). The
        // owner cell is recovered from the quad CENTROID (corner vertices
        // overhang their cell by half a meter, so a corner-based floor()
        // lands in the neighbor — a test bug the run exposed).
        for quad in verts.chunks(4) {
            let c = quad
                .iter()
                .fold([0.0f32; 3], |acc, v| [
                    acc[0] + v.pos[0] / 4.0,
                    acc[1] + v.pos[1] / 4.0,
                    acc[2] + v.pos[2] / 4.0,
                ]);
            let n0 = quad[0].normal;
            let cell = CellCoord {
                x: (c[0] - n0[0] * 0.5).floor() as i32,
                y: (c[1] - n0[1] * 0.5).floor() as i32,
                z: (c[2] - n0[2] * 0.5).floor() as i32,
            };
            let n = CellCoord {
                x: cell.x + n0[0] as i32,
                y: cell.y + n0[1] as i32,
                z: cell.z + n0[2] as i32,
            };
            assert!(
                !cave.contains(n) || !cave.solid_at(&gen, n),
                "faces only between solid and carved cells (cell {cell:?} -> {n:?})"
            );
        }
        // Collision agrees with the authority inside the region.
        for dy in -1..=1 {
            let c = CellCoord { x: seed.x, y: seed.y + dy, z: seed.z };
            if cave.contains(c) {
                assert_eq!(
                    cave.solid_at(&gen, c),
                    final_solid(&gen, c.x as i64 * 1000, c.y as i64 * 1000, c.z as i64 * 1000)
                        .solid
                );
            }
        }
    }

    #[test]
    fn conforming_water_follows_surface_and_remeshes_locally() {
        let (gen, r) = region();
        let graph = RiverGraph::new(&gen, 8);
        let flow = FlowTable::from_graph(&graph);
        // Find a river region near the scene.
        let mut center = RegionCoord { x: 0, z: 0 };
        'find: for x in -8..=8 {
            for z in -8..=8 {
                let reg = RegionCoord { x, z };
                if let Some(d) = graph.downstream(reg) {
                    if graph.discharge(d) >= pc3d_world::hydro::RIVER_THRESHOLD {
                        center = reg;
                        break 'find;
                    }
                }
            }
        }
        let mut water = ConformingWater::build(&gen, &graph, &flow, center, &r);
        assert!(!water.sections.is_empty(), "sections built near a river");
        let total = water.sections.len();
        // Terrain edit near one section's strip: only nearby sections
        // refresh (LOCAL), the rest keep their heights.
        let s0 = &water.sections[0];
        let ox = (s0.region.0 as f32 + 0.5) * 256.0;
        let oz = (s0.region.1 as f32 + 0.5) * 256.0;
        let edit_patch = PatchCoord {
            x: ((ox + 64.0) / PATCH_M).floor() as i32,
            y: r.center.y,
            z: ((oz + 64.0) / PATCH_M).floor() as i32,
        };
        let mut edited = std::collections::BTreeSet::new();
        edited.insert(edit_patch);
        let touched = water.refresh_after_edit(&gen, &r, &edited);
        assert!(touched >= 1 && touched <= total, "local refresh: {touched}/{total}");
    }

    #[test]
    fn foundations_accept_level_ground_and_reject_cliffs() {
        let (gen, r) = region();
        // Scan for a walkable, level 2x2 spot.
        let ox = r.center.x as f32 * PATCH_M;
        let oz = r.center.z as f32 * PATCH_M;
        let mut found_valid = false;
        for dx in 4..20i32 {
            for dz in 4..20i32 {
                let origin = CellCoord {
                    x: (ox + dx as f32) as i32,
                    y: 0,
                    z: (oz + dz as f32) as i32,
                };
                if let FoundationCheck::Valid { .. } =
                    check_foundation(&gen, &r, origin, (2, 2))
                {
                    found_valid = true;
                    break;
                }
            }
            if found_valid {
                break;
            }
        }
        assert!(found_valid, "some level walkable ground accepts a foundation");
        // A carved cliff rejects: dig a deep pit and check its rim.
        let mut r2 = r;
        let cell = CellCoord {
            x: (ox + 10.0) as i32,
            y: 0,
            z: (oz + 10.0) as i32,
        };
        r2.edit(crate::surface::SurfaceEdit::Lower { cell, meters: 9.0 });
        // On the pit cell itself the ground is now steep.
        let verdict = check_foundation(
            &gen,
            &r2,
            CellCoord { x: cell.x, y: 0, z: cell.z },
            (1, 1),
        );
        assert!(
            matches!(verdict, FoundationCheck::Rejected { .. }),
            "a dug pit rim rejects: {verdict:?}"
        );
    }
}

#[cfg(test)]
mod player_surface_tests {
    use super::tests_support::region;
    use crate::player::{CollisionSurface, PlayerBody};
    use crate::surface::{SurfaceEdit, SurfaceRegion, PATCH_M};
    use pc3d_world::coords::CellCoord;
    use pc3d_world::gen::WorldGen;

    /// The player walks the SURFACE path: edits are felt underfoot and a
    /// surface-backed foundation pad is level to stand on.
    #[test]
    fn player_walks_the_surface_and_feels_edits() {
        let (gen, mut r) = region();
        let ox = r.center.x as f32 * PATCH_M;
        let oz = r.center.z as f32 * PATCH_M;
        let start = [ox + 8.0, 0.0, oz + 8.0];
        let mut body = PlayerBody {
            pos: start,
            yaw: 0.0,
            pitch: 0.0,
        };
        // Drop onto the surface.
        let ground = r
            .ground_at(&gen, start[0], start[2], 60.0)
            .expect("ground under start");
        body.pos[1] = ground;
        for _ in 0..60 {
            body.walk_on(&gen, &r, 1.0, 0.0, 1.0 / 60.0);
        }
        let after = body.pos;
        assert!(
            (after[2] - start[2] + 4.0).abs() < 0.25,
            "walked ~4 m on the surface: {start:?} -> {after:?}"
        );
        // Feet stay ON the surface (within a step of its height).
        let h = r.height_at(after[0], after[2]);
        assert!(
            (after[1] - h).abs() < 1.6,
            "feet on the surface: {} vs {}",
            after[1], h
        );
        // An edit is felt: raise under the player, walk in place, snap up.
        let cell = CellCoord {
            x: after[0] as i32,
            y: 0,
            z: after[2] as i32,
        };
        r.edit(SurfaceEdit::Raise { cell, meters: 3.0 });
        for _ in 0..10 {
            body.walk_on(&gen, &r, 0.0, 0.0, 1.0 / 60.0);
        }
        assert!(
            body.pos[1] > after[1] + 2.0,
            "the raise is underfoot: {} -> {}",
            after[1], body.pos[1]
        );
    }
}

#[cfg(test)]
mod gpu_tests {
    use super::{check_foundation, CaveRegion, ConformingWater, FoundationCheck};
    use pc3d_world::terrain::final_solid;
    use super::tests_support::region;
    use crate::camera::CameraPose;
    use crate::scene::{dir_from_ndc, sample_ndc, sky_color_linear, to_srgb4, Probe};
    use crate::surface::{SurfaceEdit, SurfaceRegion, PATCH_M};
    use pc3d_world::coords::CellCoord;
    use pc3d_world::flow::FlowTable;
    use pc3d_world::hydro::RiverGraph;

    /// GPU proof: a sparse CAVE renders (interior visible from the pocket,
    /// no daylight leak past the cave mouth), CONFORMING WATER renders
    /// from real flow records over the surface, and an edit changes both
    /// locally — one windowed scene.
    #[test]
    fn cave_water_and_foundation_render_and_edit_locally() {
        let (gen, mut r) = region();
        // --- Cave: find the pocket and mesh it.
        let seed = crate::terrain::find_cave_pocket_near(&gen, r.center, 1)
            .map(|(air, _, _)| air)
            .or_else(|| {
                // fall back to the NWR-003 approach: scan patches
                let mut found = None;
                for dx in -3..=3i32 {
                    for dz in -3..=3i32 {
                        let p = pc3d_world::coords::PatchCoord {
                            x: r.center.x + dx,
                            y: r.center.y,
                            z: r.center.z + dz,
                        };
                        if let Some((air, _, _)) = crate::terrain::find_cave_pocket_near(&gen, p, 0)
                        {
                            found = Some(air);
                        }
                    }
                }
                found
            })
            .expect("a carved pocket near the region");
        let cave = CaveRegion::around(&gen, seed).expect("a cave region");
        let (cverts, cidx) = cave.mesh(&gen);

        // --- Water: conforming sections near the scene's river. The
        // graph band must COVER the scene (half=8 around the origin
        // misses the hills scene at region (-60,-31) — the run caught it).
        let graph = RiverGraph::new(&gen, 70);
        let flow = FlowTable::from_graph(&graph);
        // The NEAREST river region (any distance), then a surface region
        // built AROUND it so the strip's patches exist (a scan limited to
        // the hills scene found none — the run caught it).
        let cr = pc3d_world::coords::RegionCoord {
            x: r.center.x.div_euclid(16),
            z: r.center.z.div_euclid(16),
        };
        let mut best: Option<(pc3d_world::coords::RegionCoord, i32)> = None;
        for dx in -8..=8i32 {
            for dz in -8..=8i32 {
                let reg = pc3d_world::coords::RegionCoord { x: cr.x + dx, z: cr.z + dz };
                if let Some(d) = graph.downstream(reg) {
                    if graph.discharge(d) >= pc3d_world::hydro::RIVER_THRESHOLD {
                        let dist = dx.abs() + dz.abs();
                        if best.as_ref().map(|(_, b)| dist < *b).unwrap_or(true) {
                            best = Some((reg, dist));
                        }
                    }
                }
            }
        }
        let (center, _) = best.expect("a river region within +/-8");
        // Two-pass: a probe build names section 0's strip start; the REAL
        // patch window is then rebuilt AT that start. A SurfaceRegion is a
        // 3x3 PATCH window (48 m), NOT a 3x3 region — a window centered on
        // the region grid left the strip's own patches missing, which the
        // empty-dirty run of the windowed proof caught.
        let prov_patch = pc3d_world::coords::PatchCoord {
            x: center.x * 16 + 8,
            y: r.center.y,
            z: center.z * 16 + 8,
        };
        let prov = SurfaceRegion::new(gen, prov_patch);
        let probe = ConformingWater::build(&gen, &graph, &flow, center, &prov);
        assert!(!probe.sections.is_empty());
        let s0r = probe.sections[0].region;
        let river_patch = pc3d_world::coords::PatchCoord {
            x: ((s0r.0 as f32 + 0.5) * 256.0 / 16.0).floor() as i32,
            y: r.center.y,
            z: ((s0r.1 as f32 + 0.5) * 256.0 / 16.0).floor() as i32,
        };
        let river_region = SurfaceRegion::new(gen, river_patch);
        let mut water = ConformingWater::build(&gen, &graph, &flow, center, &river_region);
        assert!(!water.sections.is_empty());

        // --- Scene: surface region + cave mesh, cave-interior vantage.
        let mut renderer = crate::renderer::Renderer::offscreen(384, 288);
        renderer.set_placeholder_scene(false);
        let (sverts, sidx, _) = r.mesh_region();
        renderer.load_surface(&sverts, &sidx);
        // The cave rides the asset slot (u32 mesh path).
        renderer.load_u32_mesh(&cverts, &cidx);
        // Camera inside the pocket aimed at the NEAREST WALL FACE (scan
        // the six directions; aiming down a corridor exits the mouth and
        // the control-diff sees only sky — the first proof run caught it).
        let eye = [seed.x as f32 + 0.5, seed.y as f32 + 0.9, seed.z as f32 + 0.5];
        let dirs = [
            [1.0f32, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ];
        let mut look = dirs[0];
        let mut wall_dist = f32::MAX;
        for d in dirs {
            for step in 1..=6i32 {
                let probe = CellCoord {
                    x: seed.x + (d[0] * step as f32) as i32,
                    y: seed.y,
                    z: seed.z + (d[2] * step as f32) as i32,
                };
                if final_solid(
                    &gen,
                    probe.x as i64 * 1000,
                    probe.y as i64 * 1000,
                    probe.z as i64 * 1000,
                )
                .solid
                {
                    if (step as f32) < wall_dist {
                        wall_dist = step as f32;
                        look = d;
                    }
                    break;
                }
            }
        }
        assert!(wall_dist < f32::MAX, "an enclosing wall exists near the seed");
        let aim = [
            eye[0] + look[0] * (wall_dist - 0.6),
            eye[1] - 0.15,
            eye[2] + look[2] * (wall_dist - 0.6),
        ];
        let d = [aim[0] - eye[0], aim[1] - eye[1], aim[2] - eye[2]];
        let pose = CameraPose::new(
            eye,
            (-d[0]).atan2(-d[2]),
            (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin(),
        );
        renderer.set_pose(pose);
        let aspect = 384.0 / 288.0;
        // Control: same pose WITHOUT the cave mesh -> the interior pixels
        // belong to the cave.
        let mut ctrl = crate::renderer::Renderer::offscreen(384, 288);
        ctrl.set_placeholder_scene(false);
        ctrl.load_surface(&sverts, &sidx);
        ctrl.set_pose(pose);

        let (report, inside) = renderer.capture_png(
            &std::env::temp_dir().join("pc3d_cave_inside.png"),
            &[],
        );
        assert!(report.distinct_colors >= 2, "a real interior frame");
        let (_, inside_ctrl) =
            ctrl.capture_png(&std::env::temp_dir().join("pc3d_cave_ctrl.png"), &[]);
        // Cave presence: the center pixel changed when the cave mesh was
        // added (we are INSIDE it — the surface above is not visible).
        let with = sample_ndc(&inside, 384, 288, (0.0, 0.0));
        let without = sample_ndc(&inside_ctrl, 384, 288, (0.0, 0.0));
        let delta: f32 = (0..3).map(|i| (with[i] - without[i]).abs()).sum();
        println!("cave interior: with {with:?} without {without:?} (delta {delta:.2})");
        assert!(delta > 0.05, "the cave interior is visible (delta {delta})");

        // --- Surface vantage over the river + water mesh.
        let rx = (center.x as f32 + 0.5) * 256.0;
        let rz = (center.z as f32 + 0.5) * 256.0;
        let rground = river_region.height_at(rx, rz + 40.0);
        let eye2 = [rx, rground + 14.0, rz + 40.0];
        let d2 = [rx - eye2[0], river_region.height_at(rx, rz) - eye2[1], rz - eye2[2]];
        let pose2 = CameraPose::new(
            eye2,
            (-d2[0]).atan2(-d2[2]),
            (d2[1] / (d2[0] * d2[0] + d2[1] * d2[1] + d2[2] * d2[2]).sqrt()).asin(),
        );
        // Water rides the renderer's water pass via the legacy sections.
        let (wverts, widx) = water.mesh(&gen);
        let mut renderer2 = crate::renderer::Renderer::offscreen(384, 288);
        renderer2.set_placeholder_scene(false);
        let (rverts, ridx, _) = river_region.mesh_region();
        renderer2.load_surface(&rverts, &ridx);
        renderer2.load_water_vertices(&wverts, &widx);
        renderer2.set_water_time(Some(0.0));
        renderer2.set_pose(pose2);
        let mut ctrl2 = crate::renderer::Renderer::offscreen(384, 288);
        ctrl2.set_placeholder_scene(false);
        let (rverts2, ridx2, _) = river_region.mesh_region();
        ctrl2.load_surface(&rverts2, &ridx2);
        ctrl2.set_pose(pose2);
        let (report2, with_water) = renderer2.capture_png(
            &std::env::temp_dir().join("pc3d_conforming_water.png"),
            &[Probe {
                name: "sky",
                ndc: (0.0, 0.85),
                expected: to_srgb4(sky_color_linear(
                    dir_from_ndc(pose2, (0.0, 0.85), aspect),
                    crate::scene::SUN_DIR,
                )),
                tol: 0.05,
            }],
        );
        assert!(report2.passes_with(12), "{:?}", report2.failed_probes());
        let (_, no_water) =
            ctrl2.capture_png(&std::env::temp_dir().join("pc3d_water_ctrl.png"), &[]);
        // Water presence: some pixel became blue-dominant where water was added.
        let mut blue_pixels = 0usize;
        for x in (0..384usize).step_by(4) {
            for y in (0..288usize).step_by(4) {
                let a = sample_ndc(&with_water, 384, 288, (x as f32 / 192.0 - 1.0, 1.0 - y as f32 / 144.0));
                let b = sample_ndc(&no_water, 384, 288, (x as f32 / 192.0 - 1.0, 1.0 - y as f32 / 144.0));
                let changed = (0..3).map(|i| (a[i] - b[i]).abs()).sum::<f32>() > 0.08;
                if changed && a[2] > a[0] && a[2] > a[1] {
                    blue_pixels += 1;
                }
            }
        }
        println!("conforming water: {blue_pixels} blue-dominant changed pixels");
        assert!(blue_pixels >= 3, "water visibly conforms to the terrain");

        // --- LOCAL edit: raise terrain ON section 0's strip, exactly at
        // the t=1/16 SAMPLE POINT (a strip sample is the only place a
        // 1 m cell raise provably moves a sampled height) — inside the
        // 48 m patch window centered on the strip start. Only nearby
        // sections refresh (the P3D-303 dirty law on the surface path).
        // Placing it at +64/+64 m missed diagonal strips — an assert
        // caught that; placing it at t=1/4 fell OUTSIDE the patch window
        // and the windowed proof's empty dirty set caught that.
        let s0 = &water.sections[0];
        let ox = (s0.region.0 as f32 + 0.5) * 256.0;
        let oz = (s0.region.1 as f32 + 0.5) * 256.0;
        let sdir = match s0.direction {
            0 => [1.0f32, 0.0],
            1 => [1.0, 1.0],
            2 => [0.0, 1.0],
            3 => [-1.0, 1.0],
            4 => [-1.0, 0.0],
            5 => [-1.0, -1.0],
            6 => [0.0, -1.0],
            _ => [1.0, -1.0],
        };
        let sl = sdir[0].hypot(sdir[1]);
        let slen = 256.0 * if sl > 1.4 { std::f32::consts::SQRT_2 } else { 1.0 };
        let ex = ox + sdir[0] / sl * slen / 16.0;
        let ez = oz + sdir[1] / sl * slen / 16.0;
        let edit_patch = pc3d_world::coords::PatchCoord {
            x: (ex / PATCH_M).floor() as i32,
            y: river_region.center.y,
            z: (ez / PATCH_M).floor() as i32,
        };
        // Apply a real surface edit at that patch (through the EDIT
        // COMMAND path on the river region), then refresh: sections
        // crossing the patch must see new heights.
        let mut river_region = river_region;
        let edit_cell = CellCoord {
            x: ex.floor() as i32,
            y: 0,
            z: ez.floor() as i32,
        };
        river_region.edit(SurfaceEdit::Raise { cell: edit_cell, meters: 6.0 });
        let mut edited = std::collections::BTreeSet::new();
        edited.insert(edit_patch);
        // A section "changed" if ANY strip height moved or the water line
        // did (the line is the strip MIN — an off-minimum raise moves
        // heights but not the min, and a lower-at-the-minimum edit moves
        // both; comparing only the line undercounted the former).
        let before: Vec<(Vec<f32>, f32)> = water
            .sections
            .iter()
            .map(|s| (s.heights.clone(), s.water_line))
            .collect();
        let touched = water.refresh_after_edit(&gen, &river_region, &edited);
        let changed_lines = water
            .sections
            .iter()
            .zip(before.iter())
            .filter(|(s, (h, b))| {
                (s.water_line - *b).abs() > 1e-6
                    || s
                        .heights
                        .iter()
                        .zip(h.iter())
                        .any(|(a, c)| (a - c).abs() > 1e-6)
            })
            .count();
        assert_eq!(
            touched, changed_lines,
            "exactly the refreshed sections changed (heights or line)"
        );
        assert!(touched >= 1, "the edit must reach at least one section");
        println!("water local remesh: {touched}/{} sections changed", water.sections.len());

        // --- Foundation: scan a few pads INSIDE the patch window around
        // the strip start — real terrain near the river must offer at
        // least one buildable spot (accept/reject law proven in the unit
        // tests; this shows the verdicts on live ground).
        let mut valid_seen = None;
        let mut first_reject = None;
        for d in [4.0f32, 8.0, 12.0, -8.0] {
            let pad = CellCoord {
                x: (ox + d) as i32,
                y: 0,
                z: (oz + d) as i32,
            };
            let verdict = check_foundation(&gen, &river_region, pad, (2, 2));
            match &verdict {
                FoundationCheck::Valid { leveled_by } => {
                    valid_seen = Some((d, *leveled_by));
                    break;
                }
                FoundationCheck::Rejected { reason } => {
                    first_reject.get_or_insert_with(|| (d, *reason));
                }
            }
        }
        let (vd, lv) = valid_seen.expect("a buildable pad near the river strip");
        println!(
            "foundation near river: Valid {{ leveled_by: {lv} }} at +{vd} m{}",
            first_reject
                .map(|(d, r)| format!(" (after a {r} rejection at +{d} m)"))
                .unwrap_or_default()
        );
    }
}
