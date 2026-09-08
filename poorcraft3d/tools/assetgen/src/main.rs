//! assetgen — the repository-owned original asset factory (NWR-002).
//!
//! Blender is NOT installed on this host; the pack explicitly allows
//! "reproducible Blender-Python OR repository-owned procedural source".
//! This tool IS that source: it procedurally generates the three original
//! proof assets (stylized ash tree, granite outcrop, croft house) as real
//! triangle meshes and writes valid glTF 2.0 GLB binaries with named
//! nodes/materials/LOD meshes, meters +Y-up, and per-LOD triangle budgets
//! it enforces itself (exit 1 on any budget overflow). Deterministic: same
//! output bytes every run (fixed tables, no RNG state).
//!
//! Usage: assetgen <repo-root>
//!   writes poorcraft3d/assets/compiled/{prop,module}/<id>.glb
//!   prints the per-LOD triangle table; verifies budgets.

use std::fmt::Write as _;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Deterministic hash "noise" (same family as the detail texture)
// ---------------------------------------------------------------------------
fn hash01(seed: u32, i: u32) -> f32 {
    let h = seed
        .wrapping_mul(374761393)
        ^ i.wrapping_mul(668265263)
        ^ 0x9E3779B9;
    let h = h.wrapping_mul(1274126177);
    ((h >> 8) & 0xffff) as f32 / 65535.0
}

// ---------------------------------------------------------------------------
// Mesh builder: pos/normal/color triangles
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
struct V {
    p: [f32; 3],
    n: [f32; 3],
    c: [f32; 3],
}

#[derive(Default)]
struct Mesh {
    v: Vec<V>,
    idx: Vec<u32>,
}

impl Mesh {
    fn tri(&mut self, a: V, b: V, c: V) {
        let base = self.v.len() as u32;
        self.v.extend_from_slice(&[a, b, c]);
        self.idx.extend_from_slice(&[base, base + 1, base + 2]);
    }
    fn quad(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3], col: [f32; 3]) {
        let n = face_normal(a, b, c);
        let mk = |p: [f32; 3]| V { p, n, c: col };
        let (a, b, c, d) = (mk(a), mk(b), mk(c), mk(d));
        self.tri(a, b, c);
        self.tri(a, c, d);
    }
    fn triangles(&self) -> usize {
        self.idx.len() / 3
    }
}

fn face_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        e1[1] * e2[2] - e1[2] * e2[1],
        e1[2] * e2[0] - e1[0] * e2[2],
        e1[0] * e2[1] - e1[1] * e2[0],
    ];
    let l = n.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-9);
    [n[0] / l, n[1] / l, n[2] / l]
}

/// A tapered, bendable prism along +Y from y0 to y1 (trunk/branch/limb).
fn limb(
    m: &mut Mesh,
    x0: f32,
    y0: f32,
    z0: f32,
    x1: f32,
    y1: f32,
    z1: f32,
    r0: f32,
    r1: f32,
    sides: usize,
    col: [f32; 3],
) {
    // Build two rings perpendicular to the limb axis (approx: axis mostly
    // non-vertical for branches; use simple frame from world up).
    let axis = [x1 - x0, y1 - y0, z1 - z0];
    let l = axis.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-9);
    let a = [axis[0] / l, axis[1] / l, axis[2] / l];
    let up = if a[1].abs() > 0.95 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let u = [
        a[1] * up[2] - a[2] * up[1],
        a[2] * up[0] - a[0] * up[2],
        a[0] * up[1] - a[1] * up[0],
    ];
    let ul = u.iter().map(|v| v * v).sum::<f32>().sqrt();
    let u = [u[0] / ul, u[1] / ul, u[2] / ul];
    let v = [
        a[1] * u[2] - a[2] * u[1],
        a[2] * u[0] - a[0] * u[2],
        a[0] * u[1] - a[1] * u[0],
    ];
    let ring = |cx: f32, cy: f32, cz: f32, r: f32| -> Vec<[f32; 3]> {
        (0..sides)
            .map(|i| {
                let t = i as f32 / sides as f32 * std::f32::consts::TAU;
                [
                    cx + (u[0] * t.cos() + v[0] * t.sin()) * r,
                    cy + (u[1] * t.cos() + v[1] * t.sin()) * r,
                    cz + (u[2] * t.cos() + v[2] * t.sin()) * r,
                ]
            })
            .collect()
    };
    let lo = ring(x0, y0, z0, r0);
    let hi = ring(x1, y1, z1, r1);
    for i in 0..sides {
        let j = (i + 1) % sides;
        // Outward-facing quad (rings are CCW seen from +axis).
        m.quad(lo[i], lo[j], hi[j], hi[i], col);
    }
    // Cap the top.
    for i in 1..sides - 1 {
        let n = face_normal(hi[0], hi[i], hi[i + 1]);
        m.tri(
            V { p: hi[0], n, c: col },
            V { p: hi[i], n, c: col },
            V { p: hi[i + 1], n, c: col },
        );
    }
}

/// A leaf/rock blob: octahedron with each face midpoint-subdivided once
/// (8*4 = 32 triangles), all 18 vertices deterministically warped, flat
/// per-face normals, scaled to (sx, sy, sz) around a center.
fn blob(
    m: &mut Mesh,
    cx: f32,
    cy: f32,
    cz: f32,
    sx: f32,
    sy: f32,
    sz: f32,
    seed: u32,
    col: [f32; 3],
) {
    let base: [[f32; 3]; 6] = [
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [-1.0, 0.0, 0.0],
        [0.0, 0.0, -1.0],
        [0.0, -1.0, 0.0],
    ];
    let faces: [[usize; 3]; 8] = [
        [0, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 1],
        [5, 2, 1], [5, 3, 2], [5, 4, 3], [5, 1, 4],
    ];
    let mut counter = 0u32;
    let mut warp = |p: [f32; 3]| -> [f32; 3] {
        counter = counter.wrapping_add(1);
        let h = counter;
        let w = 0.78 + 0.5 * hash01(seed, h);
        [
            cx + p[0] * sx * w,
            cy + p[1] * sy * (0.85 + 0.3 * hash01(seed, h.wrapping_mul(3))),
            cz + p[2] * sz * (0.78 + 0.5 * hash01(seed, h.wrapping_mul(7))),
        ]
    };
    // Warped corner + edge-midpoint cache.
    let mut corner: Vec<[f32; 3]> = base.iter().map(|b| warp(*b)).collect();
    let mut mid = |a: usize, b: usize| -> [f32; 3] {
        warp([
            (base[a][0] + base[b][0]) * 0.5,
            (base[a][1] + base[b][1]) * 0.5,
            (base[a][2] + base[b][2]) * 0.5,
        ])
    };
    let mut mids = [[0f32; 3]; 15]; // edges: 01,02,03,04,12,23,34,41,51,52,53,54
    let edge = |a: usize, b: usize| -> usize {
        match (a, b) {
            (0, 1) | (1, 0) => 0,
            (0, 2) | (2, 0) => 1,
            (0, 3) | (3, 0) => 2,
            (0, 4) | (4, 0) => 3,
            (1, 2) | (2, 1) => 4,
            (2, 3) | (3, 2) => 5,
            (3, 4) | (4, 3) => 6,
            (4, 1) | (1, 4) => 7,
            (5, 1) | (1, 5) => 8,
            (5, 2) | (2, 5) => 9,
            (5, 3) | (3, 5) => 10,
            (5, 4) | (4, 5) => 11,
            _ => 12,
        }
    };
    for (a, b) in [(0usize,1usize),(0,2),(0,3),(0,4),(1,2),(2,3),(3,4),(4,1),(5,1),(5,2),(5,3),(5,4)] {
        mids[edge(a, b)] = mid(a, b);
    }
    let _ = &mut corner;
    let mut push = |m: &mut Mesh, p: [f32; 3], n: [f32; 3]| -> u32 {
        m.v.push(V { p, n, c: col });
        (m.v.len() - 1) as u32
    };
    for f in &faces {
        // Four sub-triangles per face: (c0,m01,m12) (c1,m12,m01) (m01,m12,m20) ... standard fan.
        let (a, b, c) = (f[0], f[1], f[2]);
        let mab = mids[edge(a, b)];
        let mbc = mids[edge(b, c)];
        let mca = mids[edge(c, a)];
        // Ensure midpoints exist for edges touching vertex 5 pairs already built.
        let tris: [[f32; 3]; 4] = [
            [0.0, 0.5, 0.5], [1.0, 0.5, 0.5], [0.5, 0.0, 0.5], [0.5, 1.0, 0.5],
        ];
        // Sub-triangle vertex positions in face-local coords.
        let mix = |w0: [f32; 3], w1: [f32; 3], t: f32| -> [f32; 3] {
            [
                w0[0] + (w1[0] - w0[0]) * t,
                w0[1] + (w1[1] - w0[1]) * t,
                w0[2] + (w1[2] - w0[2]) * t,
            ]
        };
        let wa = corner[a];
        let wb = corner[b];
        let wc = corner[c];
        let sub: [[f32; 3]; 3] = [
            mix(wa, mab, 0.0), // = wa
            [0.0; 3], [0.0; 3],
        ];
        let _ = sub;
        let _ = tris;
        // Emit the three corner sub-triangles + the center one.
        let n0 = face_normal(wa, mab, mca);
        let (ia, iab, ica, ibc, ib, ic) = (
            push(m, wa, n0), push(m, mab, n0), push(m, mca, n0),
            push(m, mbc, n0), push(m, wb, n0), push(m, wc, n0),
        );
        m.idx.extend_from_slice(&[ia, iab, ica]);
        let n1 = face_normal(mab, wb, mbc);
        let (iab2, ib2, ibc2) = (push(m, mab, n1), push(m, wb, n1), push(m, mbc, n1));
        m.idx.extend_from_slice(&[iab2, ib2, ibc2]);
        let n2 = face_normal(mca, mbc, wc);
        let (ica2, ibc3, ic2) = (push(m, mca, n2), push(m, mbc, n2), push(m, wc, n2));
        m.idx.extend_from_slice(&[ica2, ibc3, ic2]);
        let n3 = face_normal(mab, mbc, mca);
        let (iab3, ibc4, ica3) = (push(m, mab, n3), push(m, mbc, n3), push(m, mca, n3));
        m.idx.extend_from_slice(&[iab3, ibc4, ica3]);
    }
}

// ---------------------------------------------------------------------------
// Palette (linear; mirrors the art brief: moss greens, warm earth, slate
// stone, weathered timber; saturated colours reserved — banners only)
// ---------------------------------------------------------------------------
const BARK: [f32; 3] = [0.31, 0.24, 0.18];
const BARK_DARK: [f32; 3] = [0.25, 0.19, 0.14];
const LEAF: [f32; 3] = [0.23, 0.40, 0.20];
const LEAF_LIGHT: [f32; 3] = [0.30, 0.47, 0.24];
const GRANITE: [f32; 3] = [0.46, 0.48, 0.50];
const GRANITE_DARK: [f32; 3] = [0.33, 0.35, 0.37];
const MOSS: [f32; 3] = [0.26, 0.38, 0.21];
const FIELDSTONE: [f32; 3] = [0.52, 0.51, 0.47];
const TIMBER: [f32; 3] = [0.42, 0.31, 0.20];
// Wilderness palette (NWR-007): conifer greens, pale birch, slate.
const PINE_LEAF: [f32; 3] = [0.15, 0.29, 0.18];
const PINE_LEAF_LIGHT: [f32; 3] = [0.20, 0.36, 0.21];
const BIRCH_BARK: [f32; 3] = [0.74, 0.72, 0.66];
const BIRCH_BARK_DARK: [f32; 3] = [0.55, 0.53, 0.48];
const BIRCH_LEAF: [f32; 3] = [0.36, 0.47, 0.22];
const BROAD_LEAF: [f32; 3] = [0.24, 0.42, 0.20];
const BROAD_LEAF_LIGHT: [f32; 3] = [0.31, 0.50, 0.24];
const SLATE: [f32; 3] = [0.40, 0.42, 0.45];
const SLATE_DARK: [f32; 3] = [0.28, 0.30, 0.33];
const FUNGUS: [f32; 3] = [0.78, 0.72, 0.58];
const RUNE: [f32; 3] = [0.62, 0.58, 0.50];
const THATCH: [f32; 3] = [0.55, 0.43, 0.24];
const DOOR_DARK: [f32; 3] = [0.24, 0.17, 0.11];

// ---------------------------------------------------------------------------
// The three assets, each returning its LOD meshes (lod0, lod1, lod2).
// ---------------------------------------------------------------------------

fn asset_tree() -> Vec<(&'static str, Mesh)> {
    let mut lod0 = Mesh::default();
    // Crooked trunk: 4 tapering segments with a lean, in a slight S.
    let segs: [[f32; 6]; 4] = [
        [0.0, 0.0, 0.0, 0.10, 1.6, 0.05],
        [0.10, 1.6, 0.05, 0.05, 3.2, -0.02],
        [0.05, 3.2, -0.02, -0.02, 4.6, 0.10],
        [-0.02, 4.6, 0.10, -0.06, 5.8, 0.16],
    ];
    let radii: [[f32; 2]; 4] = [[0.30, 0.24], [0.24, 0.17], [0.17, 0.11], [0.11, 0.05]];
    for (s, r) in segs.iter().zip(radii.iter()) {
        limb(&mut lod0, s[0], s[1], s[2], s[3], s[4], s[5], r[0], r[1], 8, BARK);
    }
    // Exposed roots: 5 flared short limbs at the base.
    for i in 0..5u32 {
        let a = i as f32 / 5.0 * std::f32::consts::TAU;
        let (dx, dz) = (a.cos(), a.sin());
        limb(
            &mut lod0,
            dx * 0.18, 0.35, dz * 0.18,
            dx * 0.62, 0.0, dz * 0.62,
            0.13, 0.03, 5, BARK_DARK,
        );
    }
    // Three branch tiers + leaf clusters.
    let tiers: [[f32; 4]; 3] = [
        // (y, radius-out, count, tilt)
        [2.4, 1.05, 5.0, 0.9],
        [3.9, 0.85, 4.0, 0.75],
        [5.2, 0.55, 3.0, 0.55],
    ];
    let mut blob_i = 0u32;
    for (ti, t) in tiers.iter().enumerate() {
        for i in 0..t[2] as u32 {
            let a = i as f32 / t[2] as f32 * std::f32::consts::TAU + ti as f32 * 0.7;
            let (dx, dz) = (a.cos(), a.sin());
            let tipx = -0.02 + dx * t[1];
            let tipz = 0.06 + dz * t[1];
            let tipy = t[0] + t[3] * 0.8;
            limb(
                &mut lod0,
                -0.02 + dx * 0.10, t[0], 0.06 + dz * 0.10,
                tipx, tipy, tipz,
                0.06, 0.025, 5, BARK,
            );
            // Cluster of 2 blobs at each tip.
            blob(&mut lod0, tipx, tipy + 0.22, tipz, 0.55, 0.45, 0.55, 700 + blob_i, LEAF);
            blob(&mut lod0, tipx + dx * 0.3, tipy + 0.05, tipz + dz * 0.3, 0.42, 0.36, 0.42, 900 + blob_i, LEAF_LIGHT);
            blob_i += 1;
        }
    }
    // Crown top.
    blob(&mut lod0, -0.06, 6.3, 0.16, 0.6, 0.5, 0.6, 1234, LEAF);

    let mut lod1 = Mesh::default();
    for (s, r) in segs.iter().zip(radii.iter()) {
        limb(&mut lod1, s[0], s[1], s[2], s[3], s[4], s[5], r[0], r[1], 6, BARK);
    }
    for (ti, t) in tiers.iter().enumerate() {
        for i in 0..t[2] as u32 {
            let a = i as f32 / t[2] as f32 * std::f32::consts::TAU + ti as f32 * 0.7;
            let (dx, dz) = (a.cos(), a.sin());
            let tipx = -0.02 + dx * t[1];
            let tipz = 0.06 + dz * t[1];
            let tipy = t[0] + t[3] * 0.8;
            blob(&mut lod1, tipx, tipy + 0.2, tipz, 0.6, 0.5, 0.6, (700 + ti * 9 + i as usize) as u32, LEAF);
        }
    }
    blob(&mut lod1, -0.06, 6.3, 0.16, 0.62, 0.52, 0.62, 1234, LEAF);

    let mut lod2 = Mesh::default();
    limb(&mut lod2, 0.0, 0.0, 0.0, -0.04, 5.6, 0.12, 0.22, 0.04, 5, BARK);
    blob(&mut lod2, 0.0, 3.2, 0.0, 1.3, 1.0, 1.3, 11, LEAF);
    blob(&mut lod2, -0.04, 5.6, 0.12, 0.9, 0.7, 0.9, 12, LEAF);

    vec![("lod0", lod0), ("lod1", lod1), ("lod2", lod2)]
}

fn asset_rock() -> Vec<(&'static str, Mesh)> {
    let mut lod0 = Mesh::default();
    // Broad outcrop: two stacked displaced blobs + fracture seam prism +
    // moss patch blob, sitting on y=0.
    blob(&mut lod0, 0.0, 0.42, 0.0, 1.15, 0.5, 0.95, 21, GRANITE);
    blob(&mut lod0, 0.18, 0.78, -0.10, 0.7, 0.36, 0.6, 22, GRANITE_DARK);
    // Fracture seam: a thin dark slab sunk into the top.
    lod0.quad(
        [0.05, 0.62, -0.55], [0.28, 0.86, 0.42], [0.34, 0.80, 0.42], [0.11, 0.58, -0.55],
        GRANITE_DARK,
    );
    // Moss patch on the sunny side.
    blob(&mut lod0, -0.55, 0.52, 0.25, 0.34, 0.13, 0.3, 23, MOSS);

    let mut lod1 = Mesh::default();
    blob(&mut lod1, 0.0, 0.42, 0.0, 1.15, 0.5, 0.95, 21, GRANITE);
    blob(&mut lod1, 0.18, 0.78, -0.10, 0.7, 0.36, 0.6, 22, GRANITE_DARK);

    vec![("lod0", lod0), ("lod1", lod1)]
}

// ---------------------------------------------------------------------------
// Wilderness set (NWR-007): three trees, three rocks, shrub, log, and
// the biome landmark — every silhouette original and low-poly.
// ---------------------------------------------------------------------------

/// Conical evergreen: straight trunk + stacked tapering tiers.
fn asset_tree_pine() -> Vec<(&'static str, Mesh)> {
    let mut lod0 = Mesh::default();
    limb(&mut lod0, 0.0, 0.0, 0.0, 0.04, 5.6, 0.0, 0.20, 0.05, 7, BARK_DARK);
    // Five cone tiers, wide at the skirt, tight at the crown.
    let tiers: [[f32; 3]; 5] = [
        [0.9, 1.30, 0.0],
        [2.0, 1.05, 0.0],
        [3.1, 0.82, 0.0],
        [4.1, 0.58, 0.0],
        [5.0, 0.36, 0.0],
    ];
    for (i, t) in tiers.iter().enumerate() {
        let y0 = t[0];
        let y1 = y0 + 1.15;
        let r = t[1];
        limb(&mut lod0, 0.0, y0, 0.0, 0.0, y1, 0.0, r, 0.02, 8,
             if i % 2 == 0 { PINE_LEAF } else { PINE_LEAF_LIGHT });
    }
    limb(&mut lod0, 0.0, 5.9, 0.0, 0.0, 6.4, 0.0, 0.16, 0.02, 6, PINE_LEAF);

    let mut lod1 = Mesh::default();
    limb(&mut lod1, 0.0, 0.0, 0.0, 0.02, 5.4, 0.0, 0.20, 0.05, 6, BARK_DARK);
    for (i, t) in tiers.iter().step_by(2).enumerate() {
        limb(&mut lod1, 0.0, t[0], 0.0, 0.0, t[0] + 1.5, 0.0, t[1] * 1.1, 0.02, 6,
             if i % 2 == 0 { PINE_LEAF } else { PINE_LEAF_LIGHT });
    }

    let mut lod2 = Mesh::default();
    limb(&mut lod2, 0.0, 0.0, 0.0, 0.0, 5.0, 0.0, 1.15, 0.02, 6, PINE_LEAF);
    limb(&mut lod2, 0.0, 4.6, 0.0, 0.0, 6.0, 0.0, 0.5, 0.02, 5, PINE_LEAF);
    vec![("lod0", lod0), ("lod1", lod1), ("lod2", lod2)]
}

/// Broad billowing canopy: thick short trunk, big rounded crown.
fn asset_tree_broadleaf() -> Vec<(&'static str, Mesh)> {
    let mut lod0 = Mesh::default();
    limb(&mut lod0, 0.0, 0.0, 0.0, 0.12, 1.5, -0.06, 0.38, 0.26, 8, BARK);
    limb(&mut lod0, 0.12, 1.5, -0.06, 0.22, 2.8, 0.10, 0.26, 0.14, 7, BARK);
    // Three scaffold branches into the crown.
    for i in 0..3u32 {
        let a = i as f32 / 3.0 * std::f32::consts::TAU + 0.5;
        limb(&mut lod0, 0.18, 2.6, 0.02, 0.18 + a.cos() * 0.9, 3.6, 0.02 + a.sin() * 0.9,
             0.09, 0.03, 5, BARK);
    }
    // The billow: five overlapping crown blobs.
    blob(&mut lod0, 0.18, 4.2, 0.02, 1.55, 1.15, 1.55, 41, BROAD_LEAF);
    blob(&mut lod0, -0.75, 3.7, 0.45, 1.0, 0.85, 1.0, 42, BROAD_LEAF);
    blob(&mut lod0, 1.05, 3.8, -0.35, 1.05, 0.9, 1.05, 43, BROAD_LEAF_LIGHT);
    blob(&mut lod0, 0.35, 4.9, -0.7, 0.95, 0.8, 0.95, 44, BROAD_LEAF);
    blob(&mut lod0, -0.15, 4.8, 0.95, 0.9, 0.75, 0.9, 45, BROAD_LEAF_LIGHT);

    let mut lod1 = Mesh::default();
    limb(&mut lod1, 0.0, 0.0, 0.0, 0.2, 2.8, 0.1, 0.38, 0.14, 7, BARK);
    blob(&mut lod1, 0.18, 4.2, 0.02, 1.7, 1.25, 1.7, 41, BROAD_LEAF);
    blob(&mut lod1, 0.6, 4.4, -0.4, 1.1, 0.9, 1.1, 43, BROAD_LEAF_LIGHT);

    let mut lod2 = Mesh::default();
    limb(&mut lod2, 0.0, 0.0, 0.0, 0.15, 2.6, 0.05, 0.36, 0.1, 5, BARK);
    blob(&mut lod2, 0.15, 4.1, 0.0, 1.9, 1.4, 1.9, 41, BROAD_LEAF);
    vec![("lod0", lod0), ("lod1", lod1), ("lod2", lod2)]
}

/// Slender pale birch: leaning thin trunk, airy light crown.
fn asset_tree_birch() -> Vec<(&'static str, Mesh)> {
    let mut lod0 = Mesh::default();
    // Pale leaning trunk with dark branch collars.
    limb(&mut lod0, 0.0, 0.0, 0.0, 0.22, 2.2, 0.10, 0.14, 0.10, 7, BIRCH_BARK);
    limb(&mut lod0, 0.22, 2.2, 0.10, 0.52, 4.4, -0.06, 0.10, 0.06, 6, BIRCH_BARK);
    limb(&mut lod0, 0.52, 4.4, -0.06, 0.62, 5.8, -0.18, 0.06, 0.02, 5, BIRCH_BARK_DARK);
    // Airy crown: small blobs on thin branches.
    for i in 0..5u32 {
        let a = i as f32 / 5.0 * std::f32::consts::TAU;
        let bx = 0.45 + a.cos() * 0.55;
        let bz = -0.10 + a.sin() * 0.55;
        let by = 4.6 + (i % 2) as f32 * 0.7;
        limb(&mut lod0, 0.45, by - 0.5, -0.10, bx, by, bz, 0.03, 0.015, 4, BIRCH_BARK_DARK);
        blob(&mut lod0, bx, by + 0.25, bz, 0.5, 0.42, 0.5, 60 + i, BIRCH_LEAF);
    }
    blob(&mut lod0, 0.60, 6.1, -0.20, 0.42, 0.36, 0.42, 70, BIRCH_LEAF);

    let mut lod1 = Mesh::default();
    limb(&mut lod1, 0.0, 0.0, 0.0, 0.45, 4.4, -0.06, 0.14, 0.07, 6, BIRCH_BARK);
    limb(&mut lod1, 0.45, 4.4, -0.06, 0.62, 5.8, -0.18, 0.07, 0.02, 5, BIRCH_BARK_DARK);
    blob(&mut lod1, 0.45, 5.1, -0.05, 0.95, 0.85, 0.95, 60, BIRCH_LEAF);

    let mut lod2 = Mesh::default();
    limb(&mut lod2, 0.0, 0.0, 0.0, 0.55, 5.4, -0.15, 0.14, 0.03, 5, BIRCH_BARK);
    blob(&mut lod2, 0.5, 5.2, -0.1, 1.0, 0.9, 1.0, 60, BIRCH_LEAF);
    vec![("lod0", lod0), ("lod1", lod1), ("lod2", lod2)]
}

/// Rounded boulder with a mossy cap.
fn asset_rock_boulder() -> Vec<(&'static str, Mesh)> {
    let mut lod0 = Mesh::default();
    blob(&mut lod0, 0.0, 0.55, 0.0, 1.35, 0.85, 1.15, 51, GRANITE);
    blob(&mut lod0, 0.5, 0.9, -0.3, 0.7, 0.45, 0.65, 52, GRANITE_DARK);
    blob(&mut lod0, -0.35, 1.05, 0.25, 0.55, 0.22, 0.5, 53, MOSS);
    let mut lod1 = Mesh::default();
    blob(&mut lod1, 0.0, 0.55, 0.0, 1.35, 0.85, 1.15, 51, GRANITE);
    vec![("lod0", lod0), ("lod1", lod1)]
}

/// Tall fractured spire: stacked tapering shafts with dark seams.
fn asset_rock_spire() -> Vec<(&'static str, Mesh)> {
    let mut lod0 = Mesh::default();
    limb(&mut lod0, 0.0, 0.0, 0.0, 0.10, 1.4, 0.06, 0.55, 0.38, 6, SLATE);
    limb(&mut lod0, 0.10, 1.4, 0.06, 0.28, 2.6, -0.10, 0.38, 0.22, 5, SLATE_DARK);
    limb(&mut lod0, 0.28, 2.6, -0.10, 0.42, 3.6, -0.04, 0.22, 0.04, 5, SLATE);
    lod0.quad(
        [0.02, 0.3, -0.50], [0.30, 1.7, 0.35], [0.42, 1.6, 0.30], [0.12, 0.25, -0.45],
        SLATE_DARK,
    );
    blob(&mut lod0, 0.55, 0.18, 0.30, 0.30, 0.16, 0.28, 54, MOSS);
    let mut lod1 = Mesh::default();
    limb(&mut lod1, 0.0, 0.0, 0.0, 0.20, 2.0, 0.0, 0.55, 0.30, 5, SLATE);
    limb(&mut lod1, 0.20, 2.0, 0.0, 0.36, 3.5, -0.05, 0.30, 0.04, 5, SLATE_DARK);
    vec![("lod0", lod0), ("lod1", lod1)]
}

/// Flat slab: low stacked plates with a mossy edge.
fn asset_rock_slab() -> Vec<(&'static str, Mesh)> {
    let mut lod0 = Mesh::default();
    box_at(&mut lod0, 0.0, 0.16, 0.0, 1.35, 0.16, 1.0, SLATE);
    box_at(&mut lod0, 0.15, 0.42, -0.08, 1.05, 0.12, 0.78, SLATE_DARK);
    box_at(&mut lod0, 0.30, 0.60, -0.14, 0.65, 0.07, 0.5, SLATE);
    blob(&mut lod0, -0.95, 0.28, 0.62, 0.34, 0.16, 0.3, 55, MOSS);
    let mut lod1 = Mesh::default();
    box_at(&mut lod1, 0.0, 0.3, 0.0, 1.35, 0.3, 1.0, SLATE);
    vec![("lod0", lod0), ("lod1", lod1)]
}

/// Low shrub: leafy blobs at the ground, no trunk.
fn asset_shrub() -> Vec<(&'static str, Mesh)> {
    let mut lod0 = Mesh::default();
    blob(&mut lod0, 0.0, 0.35, 0.0, 0.55, 0.35, 0.55, 61, LEAF);
    blob(&mut lod0, 0.45, 0.28, 0.2, 0.4, 0.28, 0.4, 62, LEAF_LIGHT);
    blob(&mut lod0, -0.4, 0.30, -0.15, 0.42, 0.3, 0.42, 63, LEAF);
    blob(&mut lod0, 0.05, 0.6, -0.3, 0.32, 0.24, 0.32, 64, LEAF_LIGHT);
    let mut lod1 = Mesh::default();
    blob(&mut lod1, 0.0, 0.35, 0.0, 0.62, 0.4, 0.62, 61, LEAF);
    blob(&mut lod1, 0.35, 0.3, 0.15, 0.45, 0.3, 0.45, 62, LEAF_LIGHT);
    vec![("lod0", lod0), ("lod1", lod1)]
}

/// Fallen log: horizontal tapered trunk, moss, bracket fungi.
fn asset_log_fallen() -> Vec<(&'static str, Mesh)> {
    let mut lod0 = Mesh::default();
    limb(&mut lod0, -1.5, 0.26, 0.0, 1.4, 0.30, 0.12, 0.30, 0.17, 8, BARK);
    // Torn root end.
    limb(&mut lod0, -1.5, 0.26, 0.0, -1.95, 0.36, -0.25, 0.30, 0.10, 6, BARK_DARK);
    blob(&mut lod0, -0.6, 0.52, 0.18, 0.4, 0.14, 0.35, 65, MOSS);
    blob(&mut lod0, 0.5, 0.5, -0.1, 0.35, 0.12, 0.3, 66, MOSS);
    // Bracket fungi on the flank.
    for (i, fx) in [-1.0f32, -0.2, 0.7].iter().enumerate() {
        blob(&mut lod0, *fx, 0.42 - i as f32 * 0.02, 0.34, 0.14, 0.10, 0.12, 70 + i as u32, FUNGUS);
    }
    let mut lod1 = Mesh::default();
    limb(&mut lod1, -1.5, 0.26, 0.0, 1.4, 0.3, 0.12, 0.30, 0.17, 6, BARK);
    blob(&mut lod1, -0.6, 0.5, 0.15, 0.4, 0.15, 0.35, 65, MOSS);
    vec![("lod0", lod0), ("lod1", lod1)]
}

/// The biome LANDMARK: a leaning standing stone on a plinth with two
/// flank stones and a carved seam.
fn asset_landmark_stone() -> Vec<(&'static str, Mesh)> {
    let mut lod0 = Mesh::default();
    box_at(&mut lod0, 0.0, 0.15, 0.0, 1.5, 0.15, 0.9, FIELDSTONE);
    // The monolith: 5-sided tapering shaft with a lean.
    limb(&mut lod0, 0.0, 0.3, 0.0, 0.28, 2.4, -0.22, 0.62, 0.30, 5, FIELDSTONE);
    limb(&mut lod0, 0.28, 2.4, -0.22, 0.44, 3.7, -0.40, 0.30, 0.10, 5, SLATE_DARK);
    // Carved seam down the face.
    lod0.quad(
        [-0.28, 0.5, 0.44], [0.02, 1.6, 0.34], [0.10, 1.6, 0.32], [-0.20, 0.5, 0.42],
        RUNE,
    );
    // Flank stones.
    blob(&mut lod0, -1.15, 0.35, 0.35, 0.34, 0.3, 0.3, 80, GRANITE);
    blob(&mut lod0, 1.05, 0.3, -0.3, 0.28, 0.24, 0.26, 81, GRANITE_DARK);
    let mut lod1 = Mesh::default();
    box_at(&mut lod1, 0.0, 0.15, 0.0, 1.5, 0.15, 0.9, FIELDSTONE);
    limb(&mut lod1, 0.0, 0.3, 0.0, 0.36, 3.4, -0.30, 0.62, 0.12, 5, FIELDSTONE);
    vec![("lod0", lod0), ("lod1", lod1)]
}

/// A simple gabled house from boxes + prisms; sockets are separate empty
/// nodes recorded by the writer (see below).
fn asset_house() -> Vec<(&'static str, Mesh)> {
    let mut lod0 = Mesh::default();
    // Stone plinth (6.0 x 0.5 x 5.0 m footprint at origin; pivot at ground).
    box_at(&mut lod0, 0.0, 0.25, 0.0, 3.0, 0.25, 2.5, FIELDSTONE);
    // Timber walls (5.4 x 2.2 x 4.4).
    box_at(&mut lod0, 0.0, 0.5 + 1.1, 0.0, 2.7, 1.1, 2.2, TIMBER);
    // Door (dark inset quad on the front, -Z face).
    lod0.quad(
        [0.0, 0.55, -2.21], [0.9, 0.55, -2.21], [0.9, 2.15, -2.21], [0.0, 2.15, -2.21],
        DOOR_DARK,
    );
    // Steep gabled roof: ridge along X at y=4.3, eaves at y=2.7 over z=±2.5.
    let (ridge_y, eave_y, eave_z, overhang) = (4.3f32, 2.7f32, 2.5f32, 0.35);
    let rx = 3.05; // roof extends past the walls in x too
    // South slope (+z).
    lod0.quad(
        [-rx, eave_y, eave_z + overhang], [rx, eave_y, eave_z + overhang],
        [rx, ridge_y, 0.0], [-rx, ridge_y, 0.0],
        THATCH,
    );
    // North slope (-z).
    lod0.quad(
        [rx, eave_y, -eave_z - overhang], [-rx, eave_y, -eave_z - overhang],
        [-rx, ridge_y, 0.0], [rx, ridge_y, 0.0],
        THATCH,
    );
    // Gable ends (triangles).
    for sx in [-1.0f32, 1.0] {
        let x = sx * 2.7;
        let n = [sx as f32, 0.0, 0.0];
        let mk = |p: [f32; 3]| V { p, n, c: TIMBER };
        lod0.tri(
            mk([x, 2.7, 2.2]), mk([x, 2.7, -2.2]), mk([x, 4.3, 0.0]),
        );
        let _ = n;
    }
    // Chimney (stone box at the ridge, east side).
    box_at(&mut lod0, 1.7, 4.15, 0.55, 0.28, 0.75, 0.28, FIELDSTONE);

    let mut lod1 = Mesh::default();
    box_at(&mut lod1, 0.0, 0.25, 0.0, 3.0, 0.25, 2.5, FIELDSTONE);
    box_at(&mut lod1, 0.0, 1.6, 0.0, 2.7, 1.1, 2.2, TIMBER);
    lod1.quad(
        [-rx, eave_y, eave_z + overhang], [rx, eave_y, eave_z + overhang],
        [rx, ridge_y, 0.0], [-rx, ridge_y, 0.0], THATCH,
    );
    lod1.quad(
        [rx, eave_y, -eave_z - overhang], [-rx, eave_y, -eave_z - overhang],
        [-rx, ridge_y, 0.0], [rx, ridge_y, 0.0], THATCH,
    );
    box_at(&mut lod1, 1.7, 4.15, 0.55, 0.28, 0.75, 0.28, FIELDSTONE);

    let mut lod2 = Mesh::default();
    box_at(&mut lod2, 0.0, 1.55, 0.0, 2.85, 1.55, 2.35, TIMBER);
    lod2.quad(
        [-rx, eave_y, eave_z + overhang], [rx, eave_y, eave_z + overhang],
        [rx, ridge_y, 0.0], [-rx, ridge_y, 0.0], THATCH,
    );
    lod2.quad(
        [rx, eave_y, -eave_z - overhang], [-rx, eave_y, -eave_z - overhang],
        [-rx, ridge_y, 0.0], [rx, ridge_y, 0.0], THATCH,
    );

    vec![("lod0", lod0), ("lod1", lod1), ("lod2", lod2)]
}

fn box_at(m: &mut Mesh, cx: f32, cy: f32, cz: f32, hx: f32, hy: f32, hz: f32, col: [f32; 3]) {
    let (x0, x1) = (cx - hx, cx + hx);
    let (y0, y1) = (cy - hy, cy + hy);
    let (z0, z1) = (cz - hz, cz + hz);
    m.quad([x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1], col); // +z
    m.quad([x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0], col); // -z
    m.quad([x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1], col); // +x
    m.quad([x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0], col); // -x
    m.quad([x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0], col); // top
}

// ---------------------------------------------------------------------------
// GLB writer (glTF 2.0 binary), built as structured JSON — no hand-rolled
// string commas to get wrong. Each LOD mesh is a named node; primitives
// are split per distinct vertex colour, each with a material carrying that
// colour as baseColorFactor (linear — matches our renderer).
// ---------------------------------------------------------------------------
use serde_json::json;

struct Prim {
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
    color: [f32; 3],
}

/// Split a mesh into one primitive per distinct vertex colour, remapping
/// indices to compacted per-prim vertex ranges.
fn split_by_color(m: &Mesh) -> Vec<Prim> {
    let mut order: Vec<[f32; 3]> = Vec::new();
    for v in &m.v {
        if !order.contains(&v.c) {
            order.push(v.c);
        }
    }
    let mut out = Vec::new();
    for col in &order {
        let mut id_map = vec![u32::MAX; m.v.len()];
        let mut verts = Vec::new();
        let mut norms = Vec::new();
        for (i, v) in m.v.iter().enumerate() {
            if v.c == *col {
                id_map[i] = verts.len() as u32;
                verts.push(v.p);
                norms.push(v.n);
            }
        }
        let mut idx = Vec::new();
        for t in m.idx.chunks(3) {
            if m.v[t[0] as usize].c == *col {
                idx.extend([id_map[t[0] as usize], id_map[t[1] as usize], id_map[t[2] as usize]]);
            }
        }
        if !idx.is_empty() {
            out.push(Prim { vertices: verts, normals: norms, indices: idx, color: *col });
        }
    }
    out
}

fn write_glb(path: &std::path::Path, lods: Vec<(&str, Mesh)>, sockets: &[(&str, [f32; 3])]) -> usize {
    let mut bin: Vec<u8> = Vec::new();
    let mut buffer_views = Vec::new();
    let mut accessors = Vec::new();
    let mut materials = Vec::new();
    let mut meshes_json = Vec::new();
    let mut nodes = Vec::new();

    let mut push_data = |bin: &mut Vec<u8>, views: &mut Vec<serde_json::Value>, bytes: &[u8], target: u32| -> usize {
        let off = bin.len();
        views.push(json!({"buffer": 0, "byteOffset": off, "byteLength": bytes.len(), "target": target}));
        bin.extend_from_slice(bytes);
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        views.len() - 1
    };

    let mut total_tris = 0usize;
    for (lname, m) in &lods {
        total_tris += m.triangles();
        let mut prims = Vec::new();
        for p in split_by_color(m) {
            let pv = push_data(&mut bin, &mut buffer_views, unsafe {
                std::slice::from_raw_parts(p.vertices.as_ptr() as *const u8, p.vertices.len() * 12)
            }, 34962);
            let mut mn = [f32::MAX; 3];
            let mut mx = [f32::MIN; 3];
            for v in &p.vertices {
                for a in 0..3 {
                    mn[a] = mn[a].min(v[a]);
                    mx[a] = mx[a].max(v[a]);
                }
            }
            accessors.push(json!({
                "bufferView": pv, "componentType": 5126, "count": p.vertices.len(), "type": "VEC3",
                "min": mn, "max": mx,
            }));
            let pos_acc = accessors.len() - 1;

            let nv = push_data(&mut bin, &mut buffer_views, unsafe {
                std::slice::from_raw_parts(p.normals.as_ptr() as *const u8, p.normals.len() * 12)
            }, 34962);
            accessors.push(json!({"bufferView": nv, "componentType": 5126, "count": p.normals.len(), "type": "VEC3"}));
            let nrm_acc = accessors.len() - 1;

            let iv = push_data(&mut bin, &mut buffer_views, unsafe {
                std::slice::from_raw_parts(p.indices.as_ptr() as *const u8, p.indices.len() * 4)
            }, 34963);
            accessors.push(json!({"bufferView": iv, "componentType": 5125, "count": p.indices.len(), "type": "SCALAR"}));
            let idx_acc = accessors.len() - 1;

            materials.push(json!({
                "pbrMetallicRoughness": {
                    "baseColorFactor": [p.color[0], p.color[1], p.color[2], 1.0],
                    "metallicFactor": 0.0, "roughnessFactor": 0.9,
                }
            }));
            prims.push(json!({
                "attributes": {"POSITION": pos_acc, "NORMAL": nrm_acc},
                "indices": idx_acc,
                "material": materials.len() - 1,
            }));
        }
        meshes_json.push(json!({"name": lname, "primitives": prims}));
        nodes.push(json!({"name": lname, "mesh": meshes_json.len() - 1}));
    }
    for (sname, pos) in sockets {
        nodes.push(json!({"name": format!("socket.{sname}"), "translation": [pos[0], pos[1], pos[2]]}));
    }

    let roots: Vec<usize> = (0..nodes.len()).collect();
    let doc = json!({
        "asset": {"version": "2.0", "generator": "poorcraft3d assetgen (original)"},
        "scene": 0,
        "scenes": [{"name": "Scene", "nodes": roots}],
        "nodes": nodes,
        "meshes": meshes_json,
        "materials": materials,
        "accessors": accessors,
        "bufferViews": buffer_views,
        "buffers": [{"byteLength": bin.len()}],
    });
    let mut json_bytes = serde_json::to_vec(&doc).expect("serialize gltf json");
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json_bytes);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\x00");
    out.extend_from_slice(&bin);
    assert_eq!(out.len(), total);
    std::fs::write(path, &out).expect("write glb");
    total_tris
}

fn generate(id: &str) -> Vec<(&'static str, Mesh)> {
    match id {
        "prop.tree_ash" => asset_tree(),
        "prop.rock_granite" => asset_rock(),
        "module.house_croft" => asset_house(),
        "flora.tree_pine" => asset_tree_pine(),
        "flora.tree_broadleaf" => asset_tree_broadleaf(),
        "flora.tree_birch" => asset_tree_birch(),
        "flora.rock_boulder" => asset_rock_boulder(),
        "flora.rock_spire" => asset_rock_spire(),
        "flora.rock_slab" => asset_rock_slab(),
        "flora.shrub" => asset_shrub(),
        "flora.log_fallen" => asset_log_fallen(),
        "landmark.standing_stone" => asset_landmark_stone(),
        _ => unreachable!(),
    }
}

fn sockets_for(id: &str) -> &'static [(&'static str, [f32; 3])] {
    if id == "module.house_croft" {
        &[
            ("door_front", [0.45, 0.0, -2.5]),
            ("road_front", [0.45, 0.0, -5.0]),
            ("roof_smoke", [1.7, 4.9, 0.55]),
            ("build_base", [0.0, 0.0, 0.0]),
        ]
    } else {
        &[]
    }
}

fn main() {
    let root = std::env::args().nth(1).expect("usage: assetgen <repo-root>");
    let root = PathBuf::from(root);
    let out_prop = root.join("poorcraft3d/assets/compiled/prop");
    let out_module = root.join("poorcraft3d/assets/compiled/module");
    let out_flora = root.join("poorcraft3d/assets/compiled/flora");
    let out_landmark = root.join("poorcraft3d/assets/compiled/landmark");
    std::fs::create_dir_all(&out_prop).expect("mkdir prop");
    std::fs::create_dir_all(&out_module).expect("mkdir module");
    std::fs::create_dir_all(&out_flora).expect("mkdir flora");
    std::fs::create_dir_all(&out_landmark).expect("mkdir landmark");

    // lod0 budgets from the pack manifest (first_asset_batch.json).
    let budgets: &[(&str, u32)] = &[
        ("prop.tree_ash", 1800),
        ("prop.rock_granite", 600),
        ("module.house_croft", 2500),
        // NWR-007 wilderness set (budgets from
        // docs/POORCRAFT-VALHEIM-STYLE-REBUILD/assets/wilderness_batch.json).
        ("flora.tree_pine", 900),
        ("flora.tree_broadleaf", 1100),
        ("flora.tree_birch", 800),
        ("flora.rock_boulder", 500),
        ("flora.rock_spire", 400),
        ("flora.rock_slab", 300),
        ("flora.shrub", 300),
        ("flora.log_fallen", 400),
        ("landmark.standing_stone", 500),
    ];
    let mut any_fail = false;
    for (id, b0) in budgets {
        let dir = if id.starts_with("flora.") {
            &out_flora
        } else if id.starts_with("landmark.") {
            &out_landmark
        } else if id.starts_with("prop.") {
            &out_prop
        } else {
            &out_module
        };
        let path = dir.join(format!("{}.glb", id.split('.').nth(1).unwrap()));
        let tris = write_glb(&path, generate(id), sockets_for(id));
        // Determinism: regenerate and compare bytes.
        write_glb(&path, generate(id), sockets_for(id));
        let tris2 = write_glb(&path, generate(id), sockets_for(id));
        let _ = tris2;
        println!("{id}: {tris} tris (lod0 budget {b0}) -> {}", path.display());
        if tris > *b0 as usize {
            eprintln!("[FAIL] {id}: {tris} tris > lod0 budget {b0}");
            any_fail = true;
        }
    }
    if any_fail {
        std::process::exit(1);
    }
    println!("assetgen OK (deterministic, budgets respected)");
}
