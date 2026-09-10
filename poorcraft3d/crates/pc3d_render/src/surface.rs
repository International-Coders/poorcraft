//! Surface-first natural terrain spike (NWR-003) — NOT a full migration.
//!
//! An ISOLATED 3x3-patch test region proving the pack's surface contract:
//! each 16 m patch owns a 17x17 vertex boundary grid (shared edges with
//! neighbors by construction — patch p's east column IS p+1's west
//! column), heights sampled from the AUTHORITATIVE generator
//! (`effective_surface_mm`) plus a sparse signed-delta edit layer, a
//! low-poly triangulated surface mesh (two triangles per cell — visibly
//! sloped/faceted, not staircase cubes), collision derived from the SAME
//! data (walkable-slope classification + surface-height query), and edits
//! that dirty only their patch + border neighbors. The old cube-terrain
//! path is untouched; construction keeps its grid representation.
//!
//! The region is a spike: `SurfaceRegion` lives only in pc3d_render's
//! proof path until NWR-004 migrates streaming to it.

use crate::camera::CameraPose;
use crate::scene::{lit_color, SceneVertex};
use crate::terrain::{face_expectation, terrain_albedo};
use pc3d_world::coords::{CellCoord, PatchCoord};
use pc3d_world::gen::WorldGen;

/// Vertices per patch edge (17 = 16 cells + shared boundary).
pub const GRID: usize = 17;
/// Patch size in meters (the world's PATCH_METERS).
pub const PATCH_M: f32 = 16.0;
const STEP: f32 = PATCH_M / (GRID as f32 - 1.0);
/// Slopes above this (rise/run) are not walkable.
pub const MAX_WALK_SLOPE: f32 = 1.6;

/// One patch's surface state: base heights from the generator + signed
/// deltas from edit commands. Heights are meters above sea level.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePatch {
    pub coord: PatchCoord,
    pub(crate) base: Vec<f32>,
    pub(crate) delta: Vec<f32>,
    /// Bumped on any edit; the mesh cache version.
    pub version: u64,
}

/// A terrain edit command (surface layer).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SurfaceEdit {
    Raise {
        cell: CellCoord,
        meters: f32,
    },
    Lower {
        cell: CellCoord,
        meters: f32,
    },
    /// Level a 1 m cell to a target height.
    Level {
        cell: CellCoord,
        to_m: f32,
    },
}

fn sample_height(gen: &WorldGen, wx_m: f32, wz_m: f32) -> f32 {
    gen.effective_surface_mm((wx_m * 1000.0) as i64, (wz_m * 1000.0) as i64) as f32 / 1000.0
}

fn sample_material(gen: &WorldGen, wx_m: f32, wz_m: f32) -> pc3d_world::gen::CellMaterial {
    pc3d_world::terrain::final_solid(gen, (wx_m * 1000.0) as i64, 0, (wz_m * 1000.0) as i64)
        .material
}

impl SurfacePatch {
    /// World-space vertex position (meters) for a grid node. Boundary
    /// nodes are pure functions of world position + delta: two adjacent
    /// patches compute the SAME vertex, so seams cannot gap.
    pub fn vertex_world(&self, gen: &WorldGen, lx: usize, lz: usize) -> [f32; 3] {
        let ox = self.coord.x as f32 * PATCH_M;
        let oz = self.coord.z as f32 * PATCH_M;
        let wx = ox + lx as f32 * STEP;
        let wz = oz + lz as f32 * STEP;
        let i = lz * GRID + lx;
        // Deltas are stored per 1 m CELL for edit precision; vertex delta
        // is the delta of the cell containing the vertex (floor), so a
        // raised cell lifts its four corner vertices — a clean edit
        // footprint with soft falloff to the cell's own corners only.
        [wx, self.base[i] + self.delta[i], wz]
    }

    fn idx(lx: usize, lz: usize) -> usize {
        lz * GRID + lx
    }

    /// The surface height at any world point in this patch (bilinear over
    /// the grid — the same data the mesh draws).
    pub fn height_at(&self, gen: &WorldGen, wx_m: f32, wz_m: f32) -> f32 {
        let ox = self.coord.x as f32 * PATCH_M;
        let oz = self.coord.z as f32 * PATCH_M;
        let fx = ((wx_m - ox) / STEP).clamp(0.0, (GRID - 2) as f32);
        let fz = ((wz_m - oz) / STEP).clamp(0.0, (GRID - 2) as f32);
        let (ix, iz) = (fx.floor() as usize, fz.floor() as usize);
        let (tx, tz) = (fx - ix as f32, fz - iz as f32);
        let h = |x: usize, z: usize| self.vertex_world(gen, x, z)[1];
        let (h00, h10, h01, h11) = (h(ix, iz), h(ix + 1, iz), h(ix, iz + 1), h(ix + 1, iz + 1));
        let (a, b) = (h00 + (h10 - h00) * tx, h01 + (h11 - h01) * tx);
        a + (b - a) * tz
    }

    /// Walkability around a world point: the max vertex-to-vertex rise in
    /// a 3x3 node stencil around the containing grid cell — an edited cell
    /// drops all four of ITS corners (its own quad can read flat), so the
    /// wall only appears against the neighboring nodes; the stencil must
    /// see them or a cliff would classify as a stroll.
    pub fn cell_slope(&self, gen: &WorldGen, wx_m: f32, wz_m: f32) -> f32 {
        let ox = self.coord.x as f32 * PATCH_M;
        let oz = self.coord.z as f32 * PATCH_M;
        let ix = (((wx_m - ox) / STEP).floor().clamp(1.0, (GRID - 3) as f32)) as i32;
        let iz = (((wz_m - oz) / STEP).floor().clamp(1.0, (GRID - 3) as f32)) as i32;
        let mut min_h = f32::MAX;
        let mut max_h = f32::MIN;
        for dz in -1..=1i32 {
            for dx in -1..=1i32 {
                let h = self.vertex_world(gen, (ix + dx) as usize, (iz + dz) as usize)[1];
                min_h = min_h.min(h);
                max_h = max_h.max(h);
            }
        }
        (max_h - min_h) / STEP
    }

    pub fn walkable(&self, gen: &WorldGen, wx_m: f32, wz_m: f32) -> bool {
        self.cell_slope(gen, wx_m, wz_m) <= MAX_WALK_SLOPE
    }
}

/// The 3x3 spike region: a center patch and its eight neighbors.
pub struct SurfaceRegion {
    pub gen: WorldGen,
    pub center: PatchCoord,
    patches: std::collections::BTreeMap<(i32, i32), SurfacePatch>,
}

impl SurfaceRegion {
    /// Deterministic 3x3 region around `center` from the authoritative
    /// generator (heights + materials sampled from effective_surface_mm /
    /// final_solid — the same source the cube path reads).
    pub fn new(gen: WorldGen, center: PatchCoord) -> Self {
        let mut patches = std::collections::BTreeMap::new();
        for dx in -1..=1i32 {
            for dz in -1..=1i32 {
                let coord = PatchCoord {
                    x: center.x + dx,
                    y: center.y,
                    z: center.z + dz,
                };
                patches.insert((coord.x, coord.z), SurfacePatch::build(&gen, coord));
            }
        }
        Self {
            gen,
            center,
            patches,
        }
    }

    /// An empty region scaffold (for edit semantics around one patch).
    pub fn empty(gen: &WorldGen, center: PatchCoord, ring: i32) -> Self {
        let mut patches = std::collections::BTreeMap::new();
        for dx in -ring..=ring {
            for dz in -ring..=ring {
                let coord = PatchCoord {
                    x: center.x + dx,
                    y: center.y,
                    z: center.z + dz,
                };
                patches.insert((coord.x, coord.z), SurfacePatch::build(gen, coord));
            }
        }
        Self {
            gen: *gen,
            center,
            patches,
        }
    }

    /// Replaces one patch's state (used by the streamed-delta path).
    pub fn set_patch(&mut self, key: (i32, i32), p: SurfacePatch) {
        self.patches.insert(key, p);
    }

    /// The patch for a key, or None outside this region.
    pub fn try_patch(&self, coord: (i32, i32)) -> Option<&SurfacePatch> {
        self.patches.get(&coord)
    }

    pub fn patch(&self, coord: (i32, i32)) -> &SurfacePatch {
        self.patches.get(&coord).expect("patch in the 3x3 region")
    }

    /// The patch owning a world x/z (region-local; spikes stay inside).
    fn patch_at(&self, wx_m: f32, wz_m: f32) -> (i32, i32) {
        (
            (wx_m / PATCH_M).floor() as i32,
            (wz_m / PATCH_M).floor() as i32,
        )
    }

    /// Surface height at a world point (cross-patch safe; outside the
    /// region the authoritative generator's own surface answers — the
    /// same value an unedited patch would hold).
    pub fn height_at(&self, wx_m: f32, wz_m: f32) -> f32 {
        let key = self.patch_at(wx_m, wz_m);
        self.try_patch(key)
            .map(|p| p.height_at(&self.gen, wx_m, wz_m))
            .unwrap_or_else(|| {
                self.gen
                    .effective_surface_mm((wx_m * 1000.0) as i64, (wz_m * 1000.0) as i64)
                    as f32
                    / 1000.0
            })
    }

    /// Walkability at a world point (collision derived from the SAME
    /// grid the mesh draws).
    pub fn walkable(&self, wx_m: f32, wz_m: f32) -> bool {
        let key = self.patch_at(wx_m, wz_m);
        self.patch(key).walkable(&self.gen, wx_m, wz_m)
    }

    /// Applies a surface edit command. Returns the set of DIRTY patch
    /// keys: the edited patch plus every border neighbor the edited
    /// cell's vertex footprint touches (an interior edit dirties ONE
    /// patch; an edge edit dirties its neighbor too). Deltas are applied
    /// to the grid nodes of the cell's four corners (uniform cell edits
    /// keep the surface continuous across cell borders).
    pub fn edit(&mut self, edit: SurfaceEdit) -> std::collections::BTreeSet<(i32, i32)> {
        let mut dirty = std::collections::BTreeSet::new();
        // The cell's four corner NODES (cells are 1 m; node coords are
        // meters). Level sets absolute height; raise/lower shift it.
        for node in corner_nodes(match edit {
            SurfaceEdit::Raise { cell, .. } => cell,
            SurfaceEdit::Lower { cell, .. } => cell,
            SurfaceEdit::Level { cell, .. } => cell,
        }) {
            let (wx, wz) = (node.0 as f32, node.1 as f32);
            let key = self.patch_at(wx, wz);
            let shift = match edit {
                SurfaceEdit::Raise { meters, .. } => Some(meters),
                SurfaceEdit::Lower { meters, .. } => Some(-meters),
                SurfaceEdit::Level { to_m, .. } => None,
            };
            let Some(p) = self.patches.get_mut(&key) else {
                // A boundary node owned by a patch outside this region:
                // that neighbor applies it when IT is edited (edits at a
                // region rim are naturally partial).
                continue;
            };
            let gx = ((wx - p.coord.x as f32 * PATCH_M) / STEP).round() as i32;
            let gz = ((wz - p.coord.z as f32 * PATCH_M) / STEP).round() as i32;
            if gx < 0 || gz < 0 || gx >= GRID as i32 || gz >= GRID as i32 {
                continue; // node outside this patch's grid (a seam node
                          // owned by the neighbor patch)
            }
            let i = SurfacePatch::idx(gx as usize, gz as usize);
            match shift {
                Some(d) => p.delta[i] += d,
                None => {
                    let current = p.base[i] + p.delta[i];
                    p.delta[i] += match edit {
                        SurfaceEdit::Level { to_m, .. } => to_m - current,
                        _ => unreachable!(),
                    };
                }
            }
            p.version += 1;
            dirty.insert(key);
            for (nx, nz) in neighbor_keys_if_border(p.coord, gx as usize, gz as usize) {
                dirty.insert((nx, nz));
            }
        }
        dirty
    }

    /// Meshes the whole region (all 9 patches): two triangles per cell,
    /// flat facet normals, material colors sampled from the authoritative
    /// generator at the cell center. Returns vertices/indices + the
    /// patch-keyed version table for bounded remeshing.
    pub fn mesh_region(&self) -> (Vec<SceneVertex>, Vec<u32>, Vec<((i32, i32), u64)>) {
        let mut verts = Vec::new();
        let mut idx = Vec::new();
        let mut versions = Vec::new();
        for (key, p) in &self.patches {
            versions.push((*key, p.version));
            let (v, i) = p.mesh(&self.gen);
            let base = verts.len() as u32;
            verts.extend(v);
            idx.extend(i.iter().map(|k| k + base));
        }
        (verts, idx, versions)
    }
}

impl SurfacePatch {
    pub(crate) fn build(gen: &WorldGen, coord: PatchCoord) -> SurfacePatch {
        let mut base = vec![0.0; GRID * GRID];
        for lz in 0..GRID {
            for lx in 0..GRID {
                let wx = coord.x as f32 * PATCH_M + lx as f32 * STEP;
                let wz = coord.z as f32 * PATCH_M + lz as f32 * STEP;
                base[lz * GRID + lx] = sample_height(gen, wx, wz);
            }
        }
        SurfacePatch {
            coord,
            base,
            delta: vec![0.0; GRID * GRID],
            version: 1,
        }
    }

    /// The patch's low-poly surface: 16x16 cells, 2 triangles each,
    /// split along the FIXED diagonal (NW-SE) for uniform facet grain.
    pub fn mesh(&self, gen: &WorldGen) -> (Vec<SceneVertex>, Vec<u32>) {
        let mut verts = Vec::with_capacity(GRID * GRID);
        for lz in 0..GRID {
            for lx in 0..GRID {
                verts.push(SceneVertex {
                    pos: self.vertex_world(gen, lx, lz),
                    normal: [0.0, 1.0, 0.0],
                    color: [0.0; 3],
                });
            }
        }
        let mut idx = Vec::with_capacity(16 * 16 * 6);
        for lz in 0..GRID - 1 {
            for lx in 0..GRID - 1 {
                let i00 = (lz * GRID + lx) as u32;
                let i10 = i00 + 1;
                let i01 = i00 + GRID as u32;
                let i11 = i01 + 1;
                // Material from the authoritative generator at the cell
                // center; a facet belongs to its lower-left cell.
                let cx = self.coord.x as f32 * PATCH_M + (lx as f32 + 0.5) * STEP;
                let cz = self.coord.z as f32 * PATCH_M + (lz as f32 + 0.5) * STEP;
                let color = terrain_albedo(sample_material(gen, cx, cz));
                for vi in [i00, i10, i01, i11] {
                    verts[vi as usize].color = color;
                }
                // CCW seen from above (+Y): east then SOUTH is clockwise
                // in a right-handed Y-up frame — the pixel probe caught
                // this exact winding bug (backface culling ate the whole
                // terrain). Correct order: i00 -> i11 -> i10 and
                // i00 -> i01 -> i11.
                idx.extend_from_slice(&[i00, i11, i10, i00, i01, i11]);
            }
        }
        // SMOOTH vertex normals (the terrain-quality fix): accumulate every
        // adjacent face's normal onto each shared grid vertex, then
        // normalize. The flat-facet shading produced harsh irregular
        // banding across the TIN quads — the owner read it as broken
        // terrain ("cut diagonally"); smooth normals keep the low-poly
        // silhouette but shade the ground as continuous earth.
        let mut acc = vec![[0.0f32; 3]; verts.len()];
        for t in idx.chunks(3) {
            let a = verts[t[0] as usize].pos;
            let b = verts[t[1] as usize].pos;
            let c = verts[t[2] as usize].pos;
            let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            for vi in t {
                let av = &mut acc[*vi as usize];
                av[0] += n[0];
                av[1] += n[1];
                av[2] += n[2];
            }
        }
        for (v, a) in verts.iter_mut().zip(acc.iter()) {
            let l = a.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
            v.normal = [a[0] / l, a[1] / l, a[2] / l];
        }
        (verts, idx)
    }
}

fn corner_nodes(cell: CellCoord) -> [(i32, i32); 4] {
    [
        (cell.x, cell.z),
        (cell.x + 1, cell.z),
        (cell.x, cell.z + 1),
        (cell.x + 1, cell.z + 1),
    ]
}

fn neighbor_keys_if_border(coord: PatchCoord, gx: usize, gz: usize) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    if gx == 0 {
        out.push((coord.x - 1, coord.z));
    }
    if gx == GRID - 1 {
        out.push((coord.x + 1, coord.z));
    }
    if gz == 0 {
        out.push((coord.x, coord.z - 1));
    }
    if gz == GRID - 1 {
        out.push((coord.x, coord.z + 1));
    }
    // Corners touch diagonals too.
    if gx == 0 && gz == 0 {
        out.push((coord.x - 1, coord.z - 1));
    }
    if gx == GRID - 1 && gz == 0 {
        out.push((coord.x + 1, coord.z - 1));
    }
    if gx == 0 && gz == GRID - 1 {
        out.push((coord.x - 1, coord.z + 1));
    }
    if gx == GRID - 1 && gz == GRID - 1 {
        out.push((coord.x + 1, coord.z + 1));
    }
    out
}

// ---------------------------------------------------------------------------
// Persistence: the sparse delta layer saves through pc3d_save's patch
// framing (payload = a compact delta record), keyed by patch.
// ---------------------------------------------------------------------------

/// Encodes a patch's sparse delta layer: count + (grid_index u16, delta
/// f32 LE) pairs (only nonzero deltas).
pub fn encode_delta(p: &SurfacePatch) -> Vec<u8> {
    let mut out = Vec::new();
    let mut n = 0u32;
    for d in &p.delta {
        if *d != 0.0 {
            n += 1;
        }
    }
    out.extend_from_slice(&n.to_le_bytes());
    for (i, d) in p.delta.iter().enumerate() {
        if *d != 0.0 {
            out.extend_from_slice(&(i as u16).to_le_bytes());
            out.extend_from_slice(&d.to_le_bytes());
        }
    }
    out
}

/// Decodes onto a freshly built patch (base from the same generator).
pub fn decode_delta(
    gen: &WorldGen,
    coord: PatchCoord,
    bytes: &[u8],
) -> Result<SurfacePatch, String> {
    if bytes.len() < 4 {
        return Err("delta payload too short".into());
    }
    let n = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
    let mut p = SurfacePatch::build(gen, coord);
    let mut pos = 4;
    for _ in 0..n {
        if pos + 6 > bytes.len() {
            return Err("truncated delta payload".into());
        }
        let i = u16::from_le_bytes(bytes[pos..pos + 2].try_into().unwrap()) as usize;
        let d = f32::from_le_bytes(bytes[pos + 2..pos + 6].try_into().unwrap());
        if i >= p.delta.len() {
            return Err(format!("delta index {i} out of range"));
        }
        p.delta[i] = d;
        pos += 6;
    }
    p.version = 2; // saved state is at least one edit old
    Ok(p)
}

impl crate::player::CollisionSurface for SurfaceRegion {
    fn ground_at(&self, gen: &WorldGen, x: f32, z: f32, _from_y: f32) -> Option<f32> {
        let key = ((x / PATCH_M).floor() as i32, (z / PATCH_M).floor() as i32);
        self.try_patch(key).map(|p| p.height_at(gen, x, z))
    }
    fn cell_solid(&self, _gen: &WorldGen, _x: i32, _y: i32, _z: i32) -> bool {
        false // surface terrain has no full-solid wall cells; slopes gate
              // walkability at the movement level (see cell_slope).
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region() -> SurfaceRegion {
        let (seed, coord) = pc3d_world::terrain::SceneSpec::SmoothHills.patch();
        SurfaceRegion::new(WorldGen::new(seed), coord)
    }

    #[test]
    fn boundary_vertices_are_identical_across_the_seam() {
        let r = region();
        // For every internal east seam: patch k's column 16 == patch k+1's
        // column 0 at the same local z. Same for south seams.
        let c = r.center;
        for dz in -1..=1i32 {
            let a = r.patch((c.x, c.z + dz));
            let b = r.patch((c.x + 1, c.z + dz));
            for lz in 0..GRID {
                let va = a.vertex_world(&r.gen, GRID - 1, lz);
                let vb = b.vertex_world(&r.gen, 0, lz);
                assert_eq!(va, vb, "east seam mismatch at lz={lz}");
            }
        }
        for dx in -1..=1i32 {
            let a = r.patch((c.x + dx, c.z));
            let b = r.patch((c.x + dx, c.z + 1));
            for lx in 0..GRID {
                let va = a.vertex_world(&r.gen, lx, GRID - 1);
                let vb = b.vertex_world(&r.gen, lx, 0);
                assert_eq!(va, vb, "south seam mismatch at lx={lx}");
            }
        }
    }

    #[test]
    fn mesh_is_two_triangles_per_cell_and_non_cube() {
        let r = region();
        let (verts, idx, versions) = r.mesh_region();
        assert_eq!(versions.len(), 9);
        // 9 patches x 17x17 verts, 9 x 16x16x2 tris.
        assert_eq!(verts.len(), 9 * GRID * GRID);
        assert_eq!(idx.len(), 9 * 16 * 16 * 6);
        // NON-CUBE proof: adjacent grid vertices differ by < STEP in y on
        // gentle ground (a staircase would quantize to 1 m steps) — and
        // facet normals are mostly non-axis-aligned (a cube wall has
        // normal exactly ±X/±Z with |y|=0).
        let p = r.patch((r.center.x, r.center.z));
        let mut non_axis = 0usize;
        let mut n_facets = 0usize;
        for t in idx.chunks(3) {
            let v = &verts[t[0] as usize];
            n_facets += 1;
            if v.normal[1] < 0.999 {
                non_axis += 1;
            }
        }
        // On smooth hills most facets tilt; every facet being perfectly
        // horizontal would mean a flat plane, not terrain.
        assert!(
            non_axis > n_facets / 4,
            "surface must be visibly sloped: {non_axis}/{n_facets} tilted facets"
        );
    }

    #[test]
    fn collision_agrees_with_the_visible_surface() {
        let r = region();
        // The queried height equals the bilinear mesh surface at many
        // points — what you see is what you stand on.
        let c = r.center;
        let ox = c.x as f32 * PATCH_M;
        let oz = c.z as f32 * PATCH_M;
        for k in 0..64 {
            let wx = ox + (k as f32 % 8.0) * 2.0 + 0.5;
            let wz = oz + (k as f32 / 8.0).floor() * 2.0 + 0.5;
            let h = r.height_at(wx, wz);
            // The mesh triangle under this point must contain the height.
            let cell_x = ((wx - ox) / STEP).floor() as usize;
            let cell_z = ((wz - oz) / STEP).floor() as usize;
            let p = r.patch((c.x, c.z));
            let corners = [
                p.vertex_world(&r.gen, cell_x, cell_z)[1],
                p.vertex_world(&r.gen, cell_x + 1, cell_z)[1],
                p.vertex_world(&r.gen, cell_x, cell_z + 1)[1],
                p.vertex_world(&r.gen, cell_x + 1, cell_z + 1)[1],
            ];
            let min = corners.iter().copied().fold(f32::MAX, f32::min);
            let max = corners.iter().copied().fold(f32::MIN, f32::max);
            assert!(
                h >= min - 1e-4 && h <= max + 1e-4,
                "height {h} outside facet range [{min},{max}]"
            );
        }
    }

    #[test]
    fn slope_classification_separates_walkable_from_cliff() {
        let r = region();
        // Somewhere in a 3x3 smooth-hills region both classes exist; and
        // a man-made cliff classifies unwalkable.
        let mut walkable = 0;
        let mut blocked = 0;
        let ox = r.center.x as f32 * PATCH_M - PATCH_M;
        let oz = r.center.z as f32 * PATCH_M - PATCH_M;
        for zx in 0..48 {
            for zz in 0..48 {
                let (wx, wz) = (ox + zx as f32, oz + zz as f32);
                if r.walkable(wx, wz) {
                    walkable += 1;
                } else {
                    blocked += 1;
                }
            }
        }
        assert!(
            walkable > 100,
            "gentle ground must be walkable ({walkable})"
        );
        let _ = blocked;
        // Man-made cliff: lower a cell by 8 m -> its slope cell is a wall.
        let mut r2 = region();
        let cell = CellCoord {
            x: (ox + 24.0) as i32,
            y: 0,
            z: (oz + 24.0) as i32,
        };
        r2.edit(SurfaceEdit::Lower { cell, meters: 8.0 });
        assert!(!r2.walkable(cell.x as f32 + 0.5, cell.z as f32 + 0.5));
    }

    #[test]
    fn edits_dirty_only_local_and_border_patches() {
        let mut r = region();
        let c = r.center;
        let ox = c.x as f32 * PATCH_M;
        let oz = c.z as f32 * PATCH_M;
        // Interior cell (grid middle): ONE dirty patch.
        let interior = CellCoord {
            x: (ox + 8.0) as i32,
            y: 0,
            z: (oz + 8.0) as i32,
        };
        let dirty = r.edit(SurfaceEdit::Raise {
            cell: interior,
            meters: 1.5,
        });
        assert_eq!(dirty.len(), 1, "interior edit dirties one patch: {dirty:?}");
        // Border cell on the center patch's east edge: center + east.
        let border = CellCoord {
            x: (ox + PATCH_M - 1.0) as i32,
            y: 0,
            z: (oz + 8.0) as i32,
        };
        let dirty = r.edit(SurfaceEdit::Raise {
            cell: border,
            meters: 0.5,
        });
        assert!(
            dirty.contains(&(c.x, c.z)) && dirty.contains(&(c.x + 1, c.z)) && dirty.len() == 2,
            "border edit dirties patch + east neighbor: {dirty:?}"
        );
        // Bounded remesh: after an edit, meshing again changes only the
        // dirty patch's version.
        let before: std::collections::BTreeMap<_, _> = r.mesh_region().2.into_iter().collect();
        let dirty = r.edit(SurfaceEdit::Lower {
            cell: CellCoord {
                x: (ox + 4.0) as i32,
                y: 0,
                z: (oz + 4.0) as i32,
            },
            meters: 2.0,
        });
        let after: std::collections::BTreeMap<_, _> = r.mesh_region().2.into_iter().collect();
        let changed: Vec<_> = before
            .iter()
            .filter(|(k, v)| after.get(*k) != Some(v))
            .map(|(k, _)| *k)
            .collect();
        assert_eq!(
            changed.len(),
            dirty.len(),
            "remesh bounded to dirty patches (dirty {dirty:?}, changed {changed:?})"
        );
    }

    #[test]
    fn edits_change_height_and_level_sets_target() {
        let mut r = region();
        let c = r.center;
        let ox = c.x as f32 * PATCH_M;
        let oz = c.z as f32 * PATCH_M;
        let cell = CellCoord {
            x: (ox + 6.0) as i32,
            y: 0,
            z: (oz + 6.0) as i32,
        };
        let before = r.height_at(cell.x as f32 + 0.5, cell.z as f32 + 0.5);
        r.edit(SurfaceEdit::Raise { cell, meters: 3.0 });
        let after = r.height_at(cell.x as f32 + 0.5, cell.z as f32 + 0.5);
        assert!(
            (after - before - 3.0).abs() < 0.05,
            "raise by 3: {before} -> {after}"
        );
        r.edit(SurfaceEdit::Level {
            cell,
            to_m: before + 10.0,
        });
        let leveled = r.height_at(cell.x as f32 + 0.5, cell.z as f32 + 0.5);
        assert!((leveled - (before + 10.0)).abs() < 0.05, "level: {leveled}");
    }

    #[test]
    fn delta_layer_round_trips_through_persistence() {
        let r0 = region();
        let c = r0.center;
        let mut r = region();
        let ox = c.x as f32 * PATCH_M;
        let oz = c.z as f32 * PATCH_M;
        r.edit(SurfaceEdit::Raise {
            cell: CellCoord {
                x: (ox + 5.0) as i32,
                y: 0,
                z: (oz + 7.0) as i32,
            },
            meters: 2.5,
        });
        r.edit(SurfaceEdit::Lower {
            cell: CellCoord {
                x: (ox + 9.0) as i32,
                y: 0,
                z: (oz + 3.0) as i32,
            },
            meters: 1.25,
        });
        let edited = r.patch((c.x, c.z)).clone();
        let bytes = encode_delta(&edited);
        let back = decode_delta(&r.gen, c, &bytes).expect("decode");
        assert_eq!(back.delta, edited.delta);
        // Corrupt payloads refuse.
        assert!(decode_delta(&r.gen, c, &[0, 0]).is_err());
        let mut bad = bytes.clone();
        let n = u32::from_le_bytes(bad[0..4].try_into().unwrap());
        bad.truncate(4 + (n as usize * 6) - 2);
        assert!(decode_delta(&r.gen, c, &bad).is_err());
    }
}

#[cfg(test)]
mod gpu_tests {
    use super::*;
    use crate::camera::CameraPose;
    use crate::scene::{dir_from_ndc, project_ndc, sample_ndc, sky_color_linear, to_srgb4, Probe};

    /// GPU proof: the spike region renders as SLOPED/FACETED ground —
    /// distinguished from the old cube terrain by a semantic discriminator:
    /// along a downhill view ray, the surface pixel varies CONTINUOUSLY
    /// (gradient shading across facets), and a before/after edit capture
    /// shows the raised plateau locally.
    #[test]
    fn surface_renders_sloped_and_edit_visible() {
        let (seed, coord) = pc3d_world::terrain::SceneSpec::SmoothHills.patch();
        let gen = WorldGen::new(seed);
        let mut region = SurfaceRegion::new(gen, coord);

        let (verts, idx, _) = region.mesh_region();
        let mut r = crate::renderer::Renderer::offscreen(384, 288);
        r.set_placeholder_scene(false);
        // Terrain band under the region (cube path DISABLED for this spike:
        // only the surface draws — the discriminator is surface vs sky,
        // not surface vs cubes).
        let ox = coord.x as f32 * PATCH_M - PATCH_M;
        let oz = coord.z as f32 * PATCH_M - PATCH_M;
        // Steep down-slope vantage: eye 12 m above the near ridge, aim at
        // the ground 24 m down the fall line — the center column runs
        // downhill across facets.
        let eye = [
            ox + 24.0,
            region.height_at(ox + 24.0, oz + 34.0) + 12.0,
            oz + 34.0,
        ];
        let aim = [ox + 24.0, region.height_at(ox + 24.0, oz + 10.0), oz + 10.0];
        let d = [aim[0] - eye[0], aim[1] - eye[1], aim[2] - eye[2]];
        let pose = CameraPose::new(
            eye,
            (-d[0]).atan2(-d[2]),
            (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin(),
        );
        r.load_surface(&verts, &idx);
        r.set_pose(pose);
        let aspect = 384.0 / 288.0;

        let (report, before) = r.capture_png(
            &std::env::temp_dir().join("pc3d_surface_before.png"),
            &[Probe {
                name: "sky",
                ndc: (0.0, 0.85),
                expected: to_srgb4(sky_color_linear(
                    dir_from_ndc(pose, (0.0, 0.85), aspect),
                    crate::scene::SUN_DIR,
                )),
                tol: 0.05,
            }],
        );
        assert!(report.passes_with(50), "{:?}", report.failed_probes());

        // CONTINUITY discriminator: a vertical strip down the frame's
        // fall line (x = -0.5 NDC — where the slope actually crosses)
        // quantized into 8 luminance bands — a sloped facet run crosses
        // several bands (a flat plane sits in one; a cube staircase bands
        // into flats at quantized heights).
        let strip =
            |f: &Vec<u8>, y: usize| sample_ndc(f, 384, 288, (-0.5, 1.0 - 2.0 * y as f32 / 288.0));
        let mut luminance_bands = std::collections::BTreeSet::new();
        for y in 40..240usize {
            let px = strip(&before, y);
            let lum = (px[0] + px[1] + px[2]) / 3.0;
            luminance_bands.insert((lum * 8.0) as u8);
        }
        // A flat plane sits in 1 band; a cube staircase bands into flats
        // at quantized heights with hard jumps between them. >= 3 bands +
        // the adjacent-row gradient check below = a continuous slope.
        assert!(
            luminance_bands.len() >= 3,
            "downhill run must shade continuously ({} luminance bands)",
            luminance_bands.len()
        );

        // THE CUBE-VS-SURFACE discriminator: distribution of adjacent-row
        // deltas along the run. A smooth slope shades a LITTLE on nearly
        // every row; a cube staircase has EXACTLY-identical flat bands
        // (delta == 0) punctuated by a few wall-edge jumps. So: many rows
        // with small nonzero change, and few hard jumps.
        let px_at =
            |f: &Vec<u8>, y: usize| sample_ndc(f, 384, 288, (-0.5, 1.0 - 2.0 * y as f32 / 288.0));
        // GROUND rows are identified by CONTROL DIFFERENCE (a render with
        // no surface mesh): palette-independent, the codebase's standard.
        let mut ctrl_r = crate::renderer::Renderer::offscreen(384, 288);
        ctrl_r.set_placeholder_scene(false);
        ctrl_r.set_pose(pose);
        let (_, ctrl) =
            ctrl_r.capture_png(&std::env::temp_dir().join("pc3d_surface_ctrl.png"), &[]);
        let ground = |y: usize| {
            let a = px_at(&before, y);
            let c = px_at(&ctrl, y);
            (0..3).map(|i| (a[i] - c[i]).abs()).sum::<f32>() > 0.05
        };
        let mut ground_rows = 0usize;
        let mut smooth_rows = 0usize;
        let mut hard_jumps = 0usize;
        let mut total_variation = 0f32;
        for y in 40..239usize {
            let a = px_at(&before, y);
            let b = px_at(&before, y + 1);
            if ground(y) && ground(y + 1) {
                ground_rows += 1;
                let d: f32 = (0..3).map(|i| (a[i] - b[i]).abs()).sum();
                total_variation += d;
                if d > 0.003 {
                    smooth_rows += 1;
                }
                if d > 0.12 {
                    hard_jumps += 1;
                }
            }
        }
        assert!(
            ground_rows >= 80,
            "the run must cross ground ({ground_rows} rows)"
        );
        assert!(
            smooth_rows >= ground_rows / 2,
            "a continuous slope shades on most ground rows (smooth {smooth_rows}/{ground_rows})"
        );
        assert!(hard_jumps <= 4, "no band walls (hard jumps {hard_jumps})");
        assert!(
            total_variation > 0.2,
            "the run must visibly darken/lighten (total variation {total_variation:.2})"
        );

        // EDIT VISIBILITY: raise a plateau near the aim point, remesh,
        // capture after — the image must change locally.
        // A 3x3 raised plateau near the aim point: local (one patch),
        // but wide enough to own visible pixels at ~20 m.
        let mut dirty = std::collections::BTreeSet::new();
        for dx in 0..3i32 {
            for dz in 0..3i32 {
                let cell = CellCoord {
                    x: (ox + 23.0) as i32 + dx,
                    y: 0,
                    z: (oz + 11.0) as i32 + dz,
                };
                dirty.extend(region.edit(SurfaceEdit::Raise { cell, meters: 4.0 }));
            }
        }
        assert!(!dirty.is_empty());
        let (verts2, idx2, _) = region.mesh_region();
        r.load_surface(&verts2, &idx2);
        let (_, after) = r.capture_png(&std::env::temp_dir().join("pc3d_surface_after.png"), &[]);
        let diff = crate::scene::pixel_difference_fraction(&before, &after);
        assert!(
            diff > 0.01,
            "the raised plateau must change the view ({diff})"
        );
        println!(
            "surface spike: dirty {dirty:?}, image diff {diff:.2}%, {} verts",
            verts2.len()
        );
    }
}
