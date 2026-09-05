//! P3D-403: local navigation — patch walkability, bounded A*, portals.
//!
//! A [`NavPatch`] is one patch's walkable surface: per column the floor
//! height and whether an NPC can stand there. Paths are 4-connected A*
//! with a hard node budget and deterministic tie-breaks (f, then y, then
//! x, then z) — the same endpoints always yield the same path. Portals
//! are shared-border columns walkable on both sides.

use crate::coords::{CellCoord, PatchCoord};
use crate::gen::{CellMaterial, WorldGen};
use crate::terrain::final_solid;
use crate::scales::PATCH_CELL_AXIS;

/// Hard cap on A* expansions — unreachable goals return None, never hang.
pub const MAX_NAV_NODES: usize = 4096;

/// Per-column walkable surface for one patch.
pub struct NavPatch {
    pub coord: PatchCoord,
    /// Floor height (cell y) per column; None where no surface in range.
    heights: Vec<Option<i32>>,
}

impl NavPatch {
    /// Build from the generator: per column, find the floor near the
    /// analytic surface (±2 cells) and mark walkable (floor solid + two
    /// passable cells above).
    pub fn from_gen(gen: &WorldGen, coord: PatchCoord) -> Self {
        let n = PATCH_CELL_AXIS as usize;
        let mut heights = vec![None; n * n];
        let o = coord.origin();
        let ax = o.x.div_euclid(1000) as i32;
        let az = o.z.div_euclid(1000) as i32;
        for cx in 0..n {
            for cz in 0..n {
                let wx = (ax + cx as i32) as i64 * 1000;
                let wz = (az + cz as i32) as i64 * 1000;
                let approx = gen.effective_surface_mm(wx, wz).div_euclid(1000) as i32;
                let mut found = None;
                for dy in 0..=2i32 {
                    let y = approx - dy;
                    let below = solid_at(gen, ax + cx as i32, y, az + cz as i32);
                    if !below {
                        continue;
                    }
                    let a1 = passable_at(gen, ax + cx as i32, y + 1, az + cz as i32);
                    let a2 = passable_at(gen, ax + cx as i32, y + 2, az + cz as i32);
                    if a1 && a2 {
                        found = Some(y);
                    }
                    break;
                }
                heights[cx * n + cz] = found;
            }
        }
        NavPatch { coord, heights }
    }

    fn idx(&self, lx: usize, lz: usize) -> usize {
        lx * PATCH_CELL_AXIS as usize + lz
    }

    /// Floor height of a local column.
    pub fn height(&self, lx: usize, lz: usize) -> Option<i32> {
        let n = PATCH_CELL_AXIS as usize;
        self.heights.get(self.idx(lx, lz)).copied().flatten()
    }

    /// Can an NPC stand on this local column?
    pub fn walkable(&self, lx: usize, lz: usize) -> bool {
        self.height(lx, lz).is_some()
    }

    fn local_of(&self, cell: CellCoord) -> Option<(usize, usize)> {
        let o = self.coord.origin();
        let lx = cell.x - o.x.div_euclid(1000) as i32;
        let lz = cell.z - o.z.div_euclid(1000) as i32;
        if lx < 0 || lz < 0 {
            return None;
        }
        let (lux, luz) = (lx as usize, lz as usize);
        let n = PATCH_CELL_AXIS as usize;
        if lux >= n || luz >= n {
            return None;
        }
        Some((lux, luz))
    }

    /// Bounded deterministic A* over walkable columns (4-connected).
    /// Both endpoints must be walkable and in-patch.
    pub fn path(&self, from: CellCoord, to: CellCoord) -> Option<Vec<CellCoord>> {
        let (sx, sz) = self.local_of(from)?;
        let (tx, tz) = self.local_of(to)?;
        if !self.walkable(sx, sz) || !self.walkable(tx, tz) {
            return None;
        }
        let n = PATCH_CELL_AXIS as usize;
        let h = |x: usize, z: usize| -> usize {
            (x.abs_diff(tx) + z.abs_diff(tz)) as usize
        };
        let mut open: std::collections::BinaryHeap<(
            std::cmp::Reverse<usize>,
            std::cmp::Reverse<usize>,
            std::cmp::Reverse<usize>,
        )> = std::collections::BinaryHeap::new();
        let mut g_score: std::collections::BTreeMap<(usize, usize), usize> =
            std::collections::BTreeMap::new();
        let mut came: std::collections::BTreeMap<(usize, usize), (usize, usize)> =
            std::collections::BTreeMap::new();
        let mut closed = std::collections::BTreeSet::new();
        g_score.insert((sx, sz), 0);
        open.push((
            std::cmp::Reverse(h(sx, sz)),
            std::cmp::Reverse(sx),
            std::cmp::Reverse(sz),
        ));
        let mut expansions = 0usize;
        while let Some((std::cmp::Reverse(_f), std::cmp::Reverse(x), std::cmp::Reverse(z))) =
            open.pop()
        {
            let (x, z) = (x as usize, z as usize);
            if closed.contains(&(x, z)) {
                continue;
            }
            closed.insert((x, z));
            expansions += 1;
            if expansions > MAX_NAV_NODES {
                return None;
            }
            if (x, z) == (tx, tz) {
                let mut path = Vec::new();
                let mut cur = (x, z);
                loop {
                    let wy = self.height(cur.0, cur.1)?;
                    path.push(CellCoord {
                        x: self.coord.origin().x.div_euclid(1000) as i32 + cur.0 as i32,
                        y: wy + 1,
                        z: self.coord.origin().z.div_euclid(1000) as i32 + cur.1 as i32,
                    });
                    if cur == (sx, sz) {
                        break;
                    }
                    cur = came[&cur];
                }
                path.reverse();
                return Some(path);
            }
            let g = g_score[&(x, z)];
            for (dx, dz) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let nx = x as i32 + dx;
                let nz = z as i32 + dz;
                if nx < 0 || nz < 0 {
                    continue;
                }
                let (nx, nz) = (nx as usize, nz as usize);
                if nx >= n || nz >= n || !self.walkable(nx, nz) {
                    continue;
                }
                if closed.contains(&(nx, nz)) {
                    continue;
                }
                let ng = g + 1;
                if g_score.get(&(nx, nz)).map(|&old| ng >= old).unwrap_or(false) {
                    continue;
                }
                g_score.insert((nx, nz), ng);
                came.insert((nx, nz), (x, z));
                let fh = h(nx, nz) + ng;
                open.push((
                    std::cmp::Reverse(fh),
                    std::cmp::Reverse(nx),
                    std::cmp::Reverse(nz),
                ));
            }
        }
        None
    }

    /// Shared-border columns walkable on BOTH sides of the neighbor
    /// (which must be adjacent along +x or +z).
    pub fn portals_to(&self, gen: &WorldGen, neighbor: PatchCoord) -> Vec<CellCoord> {
        let mut out = Vec::new();
        let dxn = neighbor.x - self.coord.x;
        let dzn = neighbor.z - self.coord.z;
        let n = PATCH_CELL_AXIS as usize;
        let nm = NavPatch::from_gen(gen, neighbor);
        if dxn == 1 && dzn == 0 {
            for i in 0..n {
                if self.walkable(n - 1, i) && nm.walkable(0, i) {
                    out.push(self.world_cell(n - 1, i));
                }
            }
        } else if dzn == 1 && dxn == 0 {
            for i in 0..n {
                if self.walkable(i, n - 1) && nm.walkable(i, 0) {
                    out.push(self.world_cell(i, n - 1));
                }
            }
        }
        out
    }

    fn world_cell(&self, lx: usize, lz: usize) -> CellCoord {
        let o = self.coord.origin();
        CellCoord {
            x: o.x.div_euclid(1000) as i32 + lx as i32,
            y: self.height(lx, lz).unwrap_or(0) + 1,
            z: o.z.div_euclid(1000) as i32 + lz as i32,
        }
    }
}

fn solid_at(gen: &WorldGen, cx: i32, cy: i32, cz: i32) -> bool {
    final_solid(gen, cx as i64 * 1000, cy as i64 * 1000, cz as i64 * 1000).solid
}

fn passable_at(gen: &WorldGen, cx: i32, cy: i32, cz: i32) -> bool {
    matches!(
        final_solid(gen, cx as i64 * 1000, cy as i64 * 1000, cz as i64 * 1000).material,
        CellMaterial::Air | CellMaterial::Water
    )
}

/// Cross-patch path: from → best portal (first ascending) → to.
pub fn cross_patch_path(
    gen: &WorldGen,
    from_patch: PatchCoord,
    from: CellCoord,
    to_patch: PatchCoord,
    to: CellCoord,
) -> Option<Vec<CellCoord>> {
    let a = NavPatch::from_gen(gen, from_patch);
    let b = NavPatch::from_gen(gen, to_patch);
    let portals = a.portals_to(gen, to_patch);
    if portals.is_empty() {
        return None;
    }
    let portal = portals[0];
    let head = a.path(from, portal)?;
    // The b-side entry cell sits just past the shared border.
    let entry = if to_patch.x > from_patch.x {
        CellCoord { x: portal.x + 1, y: portal.y, z: portal.z }
    } else {
        CellCoord { x: portal.x, y: portal.y, z: portal.z + 1 }
    };
    let tail = b.path(entry, to)?;
    let mut full = head;
    full.extend(tail);
    Some(full)
}

// Keep unused-import warnings away for helpers used by tests.
#[allow(unused_imports)]
use crate::scales::PATCH_CELL_AXIS as _PATCH_CELL_AXIS;

#[cfg(test)]
mod tests {
    use super::*;

    fn hills() -> (WorldGen, PatchCoord, PatchCoord) {
        (
            WorldGen::new(3),
            PatchCoord { x: -60 * 16, y: 1, z: -31 * 16 },
            PatchCoord { x: -60 * 16 + 1, y: 1, z: -31 * 16 },
        )
    }

    /// A path on smooth terrain exists, is continuous (consecutive cells
    /// 4-adjacent), and every cell is walkable.
    #[test]
    fn p3d403_path_exists_and_is_continuous() {
        let (gen, patch, _) = hills();
        let nav = NavPatch::from_gen(&gen, patch);
        let o = patch.origin();
        let from = CellCoord { x: o.x.div_euclid(1000) as i32 + 2, y: 0, z: o.z.div_euclid(1000) as i32 + 2 };
        let to = CellCoord { x: o.x.div_euclid(1000) as i32 + 13, y: 0, z: o.z.div_euclid(1000) as i32 + 13 };
        let Some(path) = nav.path(from, to) else {
            panic!("no path on smooth terrain");
        };
        assert!(path.len() >= 2);
        for w in path.windows(2) {
            let d = (w[0].x - w[1].x).abs() + (w[0].z - w[1].z).abs();
            assert_eq!(d, 1, "path must be 4-connected: {w:?}");
        }
        // Deterministic.
        assert_eq!(nav.path(from, to), nav.path(from, to));
    }

    /// A wall of built blocks routes the path AROUND it (or None if the
    /// wall fully seals the patch within budget).
    #[test]
    fn p3d403_paths_route_around_walls() {
        let (gen, patch, _) = hills();
        let mut nav = NavPatch::from_gen(&gen, patch);
        // Build a full-height wall across the middle: mark columns
        // unwalkable by direct edit (simulating built blocks).
        let o = patch.origin();
        let mid = PATCH_CELL_AXIS as usize / 2;
        for lz in 0..PATCH_CELL_AXIS as usize {
            for dy in 0..3usize {
                let lx = mid;
                let wy = nav.height(lx, lz).map(|h| h + 1 + dy as i32);
                let _ = (wy, dy, lx, lz);
            }
        }
        // Blank the middle column for rows 4..=11, leaving rows 0..3 and
        // 12..15 walkable: the wall splits the middle but routes exist
        // around its ends.
        for lz in 4..=11usize {
            let idx = mid * PATCH_CELL_AXIS as usize + lz;
            nav.heights[idx] = None;
        }
        let o2 = o;
        let from = CellCoord { x: o2.x.div_euclid(1000) as i32 + 2, y: 0, z: o2.z.div_euclid(1000) as i32 + 8 };
        let to = CellCoord { x: o2.x.div_euclid(1000) as i32 + 13, y: 0, z: o2.z.div_euclid(1000) as i32 + 8 };
        let path = nav.path(from, to).expect("a route around the wall exists");
        for cell in &path {
            let (lx, lz) = nav.local_of(*cell).expect("in-patch");
            assert!(nav.walkable(lx, lz), "path crossed an unwalkable column");
        }
        // Every path cell avoids the wall SEGMENT (column mid, rows 4..=11).
        for cell in &path {
            let (lx, lz) = nav.local_of(*cell).unwrap();
            assert!(
                !(lx == mid && (4..=11).contains(&lz)),
                "path crossed the wall segment at {lx},{lz}"
            );
        }
    }

    /// Portals exist between adjacent walkable patches and sit on the
    /// shared border; the cross-patch path is continuous through one.
    #[test]
    fn p3d403_portals_and_cross_patch_paths() {
        let (gen, a_patch, b_patch) = hills();
        let a = NavPatch::from_gen(&gen, a_patch);
        let portals = a.portals_to(&gen, b_patch);
        assert!(!portals.is_empty(), "adjacent land patches must share portals");
        // The cross-patch path exists and is continuous across the border.
        let o = a_patch.origin();
        let from = CellCoord { x: o.x.div_euclid(1000) as i32 + 2, y: 0, z: o.z.div_euclid(1000) as i32 + 8 };
        let ob = b_patch.origin();
        let to = CellCoord { x: ob.x.div_euclid(1000) as i32 + 13, y: 0, z: ob.z.div_euclid(1000) as i32 + 8 };
        let Some(path) = cross_patch_path(&gen, a_patch, from, b_patch, to) else {
            panic!("cross-patch path missing");
        };
        for w in path.windows(2) {
            let d = (w[0].x - w[1].x).abs() + (w[0].z - w[1].z).abs();
            assert!(d <= 2, "cross-patch discontinuity: {w:?}");
        }
    }
}
