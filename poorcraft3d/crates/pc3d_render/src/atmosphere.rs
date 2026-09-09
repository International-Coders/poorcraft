//! NWR-006 — materials and atmosphere.
//!
//! A small, original, stylized atmosphere layer over the existing lit-mesh
//! pipeline. Everything here is measurable and Deck-budgeted:
//!
//! * DIRECTIONAL SUN SHADOWS — one stable orthographic shadow map over the
//!   near field. Stability comes from TEXEL SNAPPING: the ortho box center
//!   is rounded to the light-space texel grid, so a still camera produces a
//!   bit-identical map frame over frame (asserted). Low tier has no shadow
//!   pass at all; Mid 1024²; High 2048².
//! * DISTANCE FOG — an exp² falloff to a warm haze color, mixed in LINEAR
//!   space after lighting. The CPU mirror `fog_factor`/`apply_fog` is the
//!   probe source of truth (the WGSL mirrors it).
//! * MATERIAL DETAIL — a 4-tile procedural atlas (grass/rock/sand/snow)
//!   selected by VERTEX-COLOR weights (`material_weights` — no vertex
//!   format change; the WGSL mirrors the same weights). pc3d_assets owns
//!   the per-material metadata (`material_detail`); this module generates
//!   deterministic RGBA bytes from it. No image files, no bloat.
//! * WATER GLINT — a restrained view-dependent sun specular on the water
//!   pass (off at Low).
//! * ALPHA-CUTOUT FOLIAGE — a pipeline that discards below a mask
//!   threshold; `foliage_quads` generates original crossed leaf quads and
//!   `leaf_mask` their deterministic cutout mask.
//!
//! No screen-space effects, no GI, no photoreal textures — every number
//! below is chosen for the Steam Deck budget rows in `AtmosphereTier`.

use crate::camera::{mul, view_matrix, Mat4};

// ---------------------------------------------------------------------------
// 1. Tiers and parameters
// ---------------------------------------------------------------------------

/// Every atmosphere setting in one struct — the renderer applies it verbatim.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Atmosphere {
    /// Shadow map square resolution; 0 = no shadow pass (Low tier).
    pub shadow_res: u32,
    /// Ortho half-extent (m) around the camera focus — the shadow field.
    pub shadow_half_m: f32,
    /// exp² fog density (1/m); 0 = no fog.
    pub fog_density: f32,
    /// Linear-space haze color the distance fades toward.
    pub fog_color: [f32; 3],
    /// Material-detail blend strength 0..=1; 0 = flat albedo.
    pub detail_strength: f32,
    /// Atlas UVs per meter (world-space tiling).
    pub detail_scale: f32,
    /// Sun glint on water.
    pub glint: bool,
}

/// The legacy (R3DV) look: every atmosphere term off. This is the renderer's
/// default state, so all pre-NWR-006 proofs stay bit-identical unless a tier
/// is applied explicitly.
pub const LEGACY: Atmosphere = Atmosphere {
    shadow_res: 0,
    shadow_half_m: 0.0,
    fog_density: 0.0,
    fog_color: [0.0; 3],
    detail_strength: 0.0,
    detail_scale: 0.0,
    glint: false,
};

/// Low/medium/high behavior with documented default budgets (NWR-006 law:
/// every visual setting has a tier and a number).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtmosphereTier {
    Low,
    Mid,
    High,
}

impl AtmosphereTier {
    pub fn params(self) -> Atmosphere {
        match self {
            // No shadow pass, thin fog only: the cheapest frame.
            AtmosphereTier::Low => Atmosphere {
                shadow_res: 0,
                shadow_half_m: 0.0,
                fog_density: 0.0040,
                fog_color: HAZE,
                detail_strength: 0.0,
                detail_scale: 0.0,
                glint: false,
            },
            // The recommended default: 1024² depth (+~2.6 MB VRAM, one
            // extra geometry pass), moderate fog, material detail, glint.
            AtmosphereTier::Mid => Atmosphere {
                shadow_res: 1024,
                shadow_half_m: 110.0,
                fog_density: 0.0055,
                fog_color: HAZE,
                detail_strength: 0.55,
                detail_scale: 0.35,
                glint: true,
            },
            // 2048² depth (+~10.5 MB VRAM), fuller atmosphere.
            AtmosphereTier::High => Atmosphere {
                shadow_res: 2048,
                shadow_half_m: 130.0,
                fog_density: 0.0070,
                fog_color: HAZE,
                detail_strength: 0.85,
                detail_scale: 0.35,
                glint: true,
            },
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AtmosphereTier::Low => "low",
            AtmosphereTier::Mid => "mid",
            AtmosphereTier::High => "high",
        }
    }
}

/// The warm dawn haze the distance fades into (linear RGB — a desaturated
/// mix of the sky's horizon band).
pub const HAZE: [f32; 3] = [0.78, 0.66, 0.52];

// ---------------------------------------------------------------------------
// 2. Fog (CPU mirror of the WGSL)
// ---------------------------------------------------------------------------

/// exp² fog: 0 at the camera, saturating with distance. Mirrored exactly
/// by fs_mesh/fs_cutout.
pub fn fog_factor(dist: f32, density: f32) -> f32 {
    let t = dist * density;
    1.0 - (-t * t).exp()
}

/// Linear-space fog mix (the probe expectation source).
pub fn apply_fog(color: [f32; 3], dist: f32, density: f32, fog_color: [f32; 3]) -> [f32; 3] {
    let f = fog_factor(dist, density);
    [
        color[0] + (fog_color[0] - color[0]) * f,
        color[1] + (fog_color[1] - color[1]) * f,
        color[2] + (fog_color[2] - color[2]) * f,
    ]
}

// ---------------------------------------------------------------------------
// 3. Material weights (CPU mirror of the WGSL atlas blend)
// ---------------------------------------------------------------------------

/// Blends the atlas tiles from the vertex albedo alone (no vertex format
/// change): green excess → grass, warm tan → sand, bright → snow, low
/// saturation → rock. Returns [grass, rock, sand, snow] summing to ~1.
/// Mirrored exactly by fs_mesh.
pub fn material_weights(albedo: [f32; 3]) -> [f32; 4] {
    let (r, g, b) = (albedo[0], albedo[1], albedo[2]);
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let sat = mx - mn;
    let val = (r + g + b) / 3.0;
    let w_grass = (g - r.max(b)).max(0.0);
    let w_rock = (1.0 - sat * 3.0).max(0.0);
    let w_sand = (r.min(g) - b).max(0.0);
    let w_snow = ((val - 0.70) * 6.0).max(0.0);
    let sum = w_grass + w_rock + w_sand + w_snow;
    if sum < 1e-5 {
        return [0.0, 1.0, 0.0, 0.0];
    }
    [w_grass / sum, w_rock / sum, w_sand / sum, w_snow / sum]
}

// ---------------------------------------------------------------------------
// 4. The stable light matrix
// ---------------------------------------------------------------------------

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let l = dot3(v, v).sqrt().max(1e-9);
    [v[0] / l, v[1] / l, v[2] / l]
}

/// Right-handed orthographic projection, NDC depth in [0, 1] (wgpu
/// convention, matching camera::perspective). The RH view looks down
/// MINUS z, so depth maps -z_view: z' = (-z - n) / (f - n). The first
/// shadow run assumed +z forward and every depth fell outside [0,1] —
/// every compare lit, delta 0.00; the CPU mirror caught it.
fn ortho(half: f32, near: f32, far: f32) -> Mat4 {
    [
        1.0 / half,
        0.0,
        0.0,
        0.0, //
        0.0,
        1.0 / half,
        0.0,
        0.0, //
        0.0,
        0.0,
        -1.0 / (far - near),
        0.0, //
        0.0,
        0.0,
        -near / (far - near),
        1.0,
    ]
}

/// The sun's view-projection over the near field, TEXEL-SNAPPED for
/// temporal stability: the box center rounds to the light-space texel
/// grid, so sub-texel camera motion cannot shimmer the shadows. The
/// world-to-light axes are fixed by the sun direction, so only the box
/// center moves.
pub fn light_view_proj(
    focus: [f32; 3],
    sun_dir: [f32; 3],
    half_m: f32,
    res: u32,
    snap: bool,
) -> Mat4 {
    // SUN_DIR points TOWARD the sun (the lighting convention); light
    // TRAVELS along -SUN_DIR, so the light camera looks along the ray
    // with its eye up-sun. The first shadow run looked uphill at the sun
    // and every depth ordered backwards — the CPU mirror caught it.
    let fwd = normalize3([-sun_dir[0], -sun_dir[1], -sun_dir[2]]);
    let right = normalize3(cross(fwd, [0.0, 1.0, 0.0]));
    let up = cross(right, fwd);
    let mut cx = dot3(focus, right);
    let mut cy = dot3(focus, up);
    let mut cz = dot3(focus, fwd);
    if snap && res > 0 {
        let texel = 2.0 * half_m / res as f32;
        cx = (cx / texel).round() * texel;
        cy = (cy / texel).round() * texel;
        // Depth rides a coarser 1 m grid: ortho XY is indifferent to it,
        // but snapping it too makes the WHOLE matrix sub-texel-stable
        // (absorbed by the 380 m back-off, far from near/far).
        cz = cz.round();
    }
    // Build the view from the (snapped) light-space projections directly:
    // reconstructing a snapped world center and re-projecting it wobbles
    // in the last ulp — the exact opposite of stability.
    let eye_back = 380.0;
    let view: Mat4 = [
        right[0],
        up[0],
        -fwd[0],
        0.0, //
        right[1],
        up[1],
        -fwd[1],
        0.0, //
        right[2],
        up[2],
        -fwd[2],
        0.0, //
        -(cx - eye_back * dot3(right, fwd)),
        -(cy - eye_back * dot3(up, fwd)),
        cz - eye_back,
        1.0,
    ];
    let proj = ortho(half_m, 1.0, 760.0);
    mul(&proj, &view)
}

/// World meters per shadow texel (the shader's normal-offset unit).
pub fn shadow_texel(half_m: f32, res: u32) -> f32 {
    if res == 0 {
        0.0
    } else {
        2.0 * half_m / res as f32
    }
}

// ---------------------------------------------------------------------------
// 5. Deterministic procedural textures (original, tiny)
// ---------------------------------------------------------------------------

/// Integer hash → [0,1) — the single noise source for every texture here.
fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393)
        ^ (y as u32).wrapping_mul(668265263)
        ^ seed.wrapping_mul(2246822519);
    h = h.wrapping_mul(2654435761);
    h ^= h >> 13;
    h = h.wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0x00ff_ffff) as f32 / 16_777_216.0
}

fn smooth_noise(x: f32, y: f32, scale: f32, seed: u32) -> f32 {
    let fx = x / scale;
    let fy = y / scale;
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sy = ty * ty * (3.0 - 2.0 * ty);
    let n00 = hash2(x0, y0, seed);
    let n10 = hash2(x0 + 1, y0, seed);
    let n01 = hash2(x0, y0 + 1, seed);
    let n11 = hash2(x0 + 1, y0 + 1, seed);
    let a = n00 + (n10 - n00) * sx;
    let b = n01 + (n11 - n01) * sx;
    a + (b - a) * sy
}

pub const ATLAS_TILES: usize = 4; // grass, rock, sand, snow
pub const ATLAS_TILE_PX: u32 = 64;

/// The material-detail atlas: 4 tiles side by side (256×64 RGBA), each a
/// deterministic stylized grain keyed by the pc3d_assets material spec
/// (grain scale + amplitude). Tile order matches `material_weights`.
pub fn material_atlas_rgba(specs: &[[u32; 2]; ATLAS_TILES]) -> Vec<u8> {
    let w = ATLAS_TILE_PX * ATLAS_TILES as u32;
    let h = ATLAS_TILE_PX;
    let mut out = vec![0u8; (w * h * 4) as usize];
    for ty in 0..h {
        for tx in 0..w {
            let tile = (tx / ATLAS_TILE_PX) as usize;
            let lx = tx % ATLAS_TILE_PX;
            let (scale, seed) = (specs[tile][0], specs[tile][1]);
            // Stylized per-tile grain: two octaves of value noise; the
            // rock tile is RIDGED (|n-0.5| — fracture seams), the others
            // are soft mottling; snow gets a fine sparkle octave.
            let (x, y) = (lx as f32, ty as f32);
            let mut v = smooth_noise(x, y, scale as f32, seed) * 0.65
                + smooth_noise(x, y, (scale as f32) * 0.45, seed ^ 0x5f37) * 0.35;
            if tile == 1 {
                v = 1.0 - ((v - 0.5) * 2.0).abs(); // rock: ridge seams
            }
            if tile == 3 {
                v = v * 0.8 + hash2(tx as i32, ty as i32, seed ^ 0x11) * 0.2; // snow sparkle
            }
            let g = (v.clamp(0.0, 1.0) * 255.0) as u8;
            let i = ((ty * w + tx) * 4) as usize;
            out[i] = g;
            out[i + 1] = g;
            out[i + 2] = g;
            out[i + 3] = 255;
        }
    }
    out
}

pub const LEAF_MASK_PX: u32 = 64;

/// The foliage cutout mask (64×64 RGBA): a three-lobed leaf silhouette
/// with nibbled holes — alpha 255 inside the leaf, 0 in the holes and
/// background; RGB is the leaf green the cutout shader tints.
pub fn leaf_mask_rgba() -> Vec<u8> {
    let n = LEAF_MASK_PX;
    let mut out = vec![0u8; (n * n * 4) as usize];
    let c = (n - 1) as f32 * 0.5;
    for y in 0..n {
        for x in 0..n {
            let (px, py) = (x as f32 - c, y as f32 - c);
            // Three overlapping ellipses (lobes) rotated 120° apart.
            let mut inside = false;
            for k in 0..3 {
                let a = k as f32 * std::f32::consts::TAU / 3.0;
                let (ca, sa) = (a.cos(), a.sin());
                let rx = px * ca + py * sa;
                let ry = -px * sa + py * ca;
                let ex = rx / (c * 0.92);
                let ey = (ry - c * 0.18) / (c * 0.42);
                if ex * ex + ey * ey < 1.0 {
                    inside = true;
                }
            }
            // Deterministic nibble holes eat into the silhouette.
            let nib = hash2(x as i32, y as i32, 0x1eaf);
            if inside && nib > 0.93 {
                inside = false;
            }
            let i = ((y * n + x) * 4) as usize;
            let a = if inside { 255 } else { 0 };
            out[i] = 96;
            out[i + 1] = 138;
            out[i + 2] = 62;
            out[i + 3] = a;
        }
    }
    out
}

/// The grass-tuft mask (NWR-007): five tapered blade silhouettes on a
/// 64x64 alpha card — the flora instancing's wind-animated ground layer.
pub fn grass_mask_rgba() -> Vec<u8> {
    let n = LEAF_MASK_PX;
    let mut out = vec![0u8; (n * n * 4) as usize];
    // Five blades: base x positions and a lean each.
    let blades: [(f32, f32, f32); 5] = [
        (0.18, 0.06, 1.00),
        (0.34, -0.04, 0.85),
        (0.50, 0.02, 1.00),
        (0.66, 0.05, 0.80),
        (0.82, -0.06, 0.90),
    ];
    for y in 0..n {
        for x in 0..n {
            let (fx, fy) = (x as f32 / n as f32, y as f32 / n as f32);
            let mut inside = false;
            for (bx, lean, hmax) in blades {
                // Taper: half-width shrinks to a tip at fy = hmax.
                let t = fy / hmax;
                if t > 1.0 {
                    continue;
                }
                let w = 0.055 * (1.0 - t * t).max(0.0);
                let cx = bx + lean * t;
                if (fx - cx).abs() < w {
                    inside = true;
                }
            }
            let i = ((y * n + x) * 4) as usize;
            let a = if inside { 255 } else { 0 };
            out[i] = 88;
            out[i + 1] = 132;
            out[i + 2] = 54;
            out[i + 3] = a;
        }
    }
    out
}

/// Which mask the cutout pipeline tests against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutoutMask {
    /// The nibbled leaf silhouette.
    Leaf,
    /// Fully opaque — the proof CONTROL: the same quads with no holes
    /// must hide the background completely.
    Solid,
}

/// A fully opaque mask (the control for cutout proofs: the same quads
/// with no holes must hide the background completely).
pub fn solid_mask_rgba() -> Vec<u8> {
    let mut v = Vec::with_capacity((LEAF_MASK_PX * LEAF_MASK_PX * 4) as usize);
    for _ in 0..(LEAF_MASK_PX * LEAF_MASK_PX) {
        v.extend_from_slice(&[96, 138, 62, 255]);
    }
    v
}

// ---------------------------------------------------------------------------
// 6. Foliage quads (cutout proof geometry — original crossed leaf cards)
// ---------------------------------------------------------------------------

/// One foliage card vertex: position, outward normal, tint, mask UV.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CutoutVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
    pub uv: [f32; 2],
}

pub const CUTOUT_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<CutoutVertex>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &[
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 12,
            shader_location: 1,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 24,
            shader_location: 2,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 36,
            shader_location: 3,
        },
    ],
};

/// A field of crossed leaf cards (two quads per plant, 90° apart) at
/// `cols`×`rows` grid positions with deterministic height/tint variation
/// — the cutout proof asset (original foliage, not any other game's).
pub fn foliage_quads(
    origin: [f32; 3],
    cols: usize,
    rows: usize,
    spacing: f32,
) -> (Vec<CutoutVertex>, Vec<u16>) {
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    for iz in 0..rows {
        for ix in 0..cols {
            let jx = hash2(ix as i32, iz as i32, 0x7001);
            let jz = hash2(ix as i32, iz as i32, 0x7002);
            let hh = hash2(ix as i32, iz as i32, 0x7003);
            let x = origin[0] + ix as f32 * spacing + (jx - 0.5) * spacing * 0.4;
            let z = origin[2] + iz as f32 * spacing + (jz - 0.5) * spacing * 0.4;
            let h = 1.1 + hh * 1.3;
            let tint = 0.8 + hash2(ix as i32, iz as i32, 0x7004) * 0.4;
            let color = [0.30 * tint, 0.54 * tint, 0.20 * tint];
            for rot in [0.0f32, std::f32::consts::FRAC_PI_2] {
                let (c, s) = (rot.cos(), rot.sin());
                // Card corners in local space (±w/2, 0..h).
                let w = h * 0.8;
                let corners = [
                    ([-w * 0.5, 0.0, 0.0], [0.0, 0.0]),
                    ([w * 0.5, 0.0, 0.0], [1.0, 0.0]),
                    ([w * 0.5, h, 0.0], [1.0, 1.0]),
                    ([-w * 0.5, h, 0.0], [0.0, 1.0]),
                ];
                let base = verts.len() as u16;
                let normal = [s, 0.2, c];
                for (p, uv) in corners {
                    verts.push(CutoutVertex {
                        pos: [
                            x + p[0] * c + p[2] * s,
                            origin[1] + p[1],
                            z - p[0] * s + p[2] * c,
                        ],
                        normal,
                        color,
                        uv,
                    });
                }
                idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
    }
    (verts, idx)
}

// ---------------------------------------------------------------------------
// Tests — the CPU laws (GPU proofs live in gpu_tests below)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fog_factor_is_monotone_and_saturating() {
        let d = AtmosphereTier::Mid.params().fog_density;
        assert_eq!(fog_factor(0.0, d), 0.0, "no fog at the camera");
        let mut prev = -1.0;
        for dist in [10.0, 50.0, 120.0, 250.0, 500.0] {
            let f = fog_factor(dist, d);
            assert!(f > prev, "monotone in distance ({dist} m -> {f})");
            assert!(f <= 1.0);
            prev = f;
        }
        assert!(
            fog_factor(600.0, d) > 0.9,
            "far field saturates toward haze"
        );
        assert_eq!(fog_factor(500.0, 0.0), 0.0, "zero density = no fog");
    }

    #[test]
    fn fog_mirror_mixes_to_haze() {
        let a = AtmosphereTier::High.params();
        let near = apply_fog([0.2, 0.5, 0.2], 8.0, a.fog_density, a.fog_color);
        let far = apply_fog([0.2, 0.5, 0.2], 400.0, a.fog_density, a.fog_color);
        assert!((near[0] - 0.2).abs() < 0.01, "near color unchanged");
        for i in 0..3 {
            assert!(
                (far[i] - a.fog_color[i]).abs() < 0.05,
                "far field approaches haze"
            );
            assert!(far[i] > near[i], "distance only moves toward the haze");
        }
    }

    #[test]
    fn material_weights_match_the_registry_albedos() {
        // (name, albedo, expected dominant tile index)
        let cases = [
            ("mat.grass", [0.30, 0.55, 0.22], 0),
            ("mat.rock", [0.55, 0.54, 0.50], 1),
            ("mat.sand", [0.80, 0.72, 0.48], 2),
            ("mat.snow", [0.92, 0.94, 0.97], 3),
        ];
        for (name, albedo, want) in cases {
            let w = material_weights(albedo);
            let dom = (0..4).max_by(|a, b| w[*a].total_cmp(&w[*b])).unwrap();
            assert_eq!(dom, want, "{name} weights {w:?}");
            assert!(w[dom] > 0.5, "{name} dominant weight {w:?}");
            let sum: f32 = w.iter().sum();
            assert!((sum - 1.0).abs() < 1e-4, "weights normalize ({sum})");
        }
    }

    #[test]
    fn light_matrix_is_texel_stable() {
        let p = AtmosphereTier::High.params();
        // Pick a focus MID-CELL in light space: snapping steps by whole
        // texels, so a focus sitting ON a grid boundary legitimately
        // rounds one way under sub-texel motion — mid-cell proves the
        // no-swim law exactly.
        let fwd = {
            let s = crate::scene::SUN_DIR;
            let l = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
            [s[0] / l, s[1] / l, s[2] / l]
        };
        let right = {
            let c = [
                fwd[1] * 0.0 - fwd[2] * 1.0,
                fwd[2] * 0.0 - fwd[0] * 0.0,
                fwd[0] * 1.0 - fwd[1] * 0.0,
            ];
            let l = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
            [c[0] / l, c[1] / l, c[2] / l]
        };
        let up = [
            right[1] * fwd[2] - right[2] * fwd[1],
            right[2] * fwd[0] - right[0] * fwd[2],
            right[0] * fwd[1] - right[1] * fwd[0],
        ];
        let texel = shadow_texel(p.shadow_half_m, p.shadow_res);
        let mut focus = [12.0f32, 3.0, -7.0];
        for probe in 0..40 {
            let f = probe as f32 * 0.25;
            let fx =
                ((focus[0] * right[0] + focus[1] * right[1] + focus[2] * right[2]) / texel) % 1.0;
            let _ = f;
            if fx > 0.2 && fx < 0.8 {
                break;
            }
            focus[0] += texel * 0.37; // slide along +X until mid-cell
        }
        let a = light_view_proj(
            focus,
            crate::scene::SUN_DIR,
            p.shadow_half_m,
            p.shadow_res,
            true,
        );
        // Sub-texel motion (0.05 m << 0.127 m texel) must not move the box.
        let b = light_view_proj(
            [focus[0] + 0.03, focus[1] + 0.02, focus[2] + 0.02],
            crate::scene::SUN_DIR,
            p.shadow_half_m,
            p.shadow_res,
            true,
        );
        assert_eq!(a, b, "sub-texel focus motion leaves the light matrix fixed");
        // Whole-texel motion DOES move it (the box follows the field).
        let texel = shadow_texel(p.shadow_half_m, p.shadow_res);
        let c = light_view_proj(
            [12.0 + texel * 2.0, 3.0, -7.0],
            crate::scene::SUN_DIR,
            p.shadow_half_m,
            p.shadow_res,
            true,
        );
        assert_ne!(a, c, "texel-scale motion re-centers the box");
        // Unsnapped is exactly the snapped one at a grid-aligned focus.
        let d = light_view_proj(
            [0.0, 0.0, 0.0],
            crate::scene::SUN_DIR,
            p.shadow_half_m,
            p.shadow_res,
            false,
        );
        let e = light_view_proj(
            [0.0, 0.0, 0.0],
            crate::scene::SUN_DIR,
            p.shadow_half_m,
            p.shadow_res,
            true,
        );
        assert_eq!(d, e, "origin focus: snapping is a no-op");
    }

    #[test]
    fn tier_budgets_are_documented_numbers() {
        let low = AtmosphereTier::Low.params();
        let mid = AtmosphereTier::Mid.params();
        let high = AtmosphereTier::High.params();
        assert_eq!(low.shadow_res, 0, "Low has no shadow pass");
        assert_eq!(mid.shadow_res, 1024);
        assert_eq!(high.shadow_res, 2048);
        assert!(mid.fog_density > low.fog_density && high.fog_density > mid.fog_density);
        assert!(mid.detail_strength > 0.0 && high.detail_strength > mid.detail_strength);
        assert!(!low.glint && mid.glint && high.glint);
        // VRAM rows: depth32 = res^2 * 4 bytes.
        assert_eq!(
            mid.shadow_res as u64 * mid.shadow_res as u64 * 4,
            4 * 1024 * 1024
        );
        assert_eq!(
            high.shadow_res as u64 * high.shadow_res as u64 * 4,
            16 * 1024 * 1024
        );
        assert_eq!(
            LEGACY,
            Atmosphere {
                shadow_res: 0,
                ..LEGACY
            }
        );
        assert_eq!(LEGACY.fog_density, 0.0);
        assert!(!LEGACY.glint);
    }

    #[test]
    fn procedural_textures_are_deterministic_and_nonflat() {
        let specs = [[8u32, 11], [10, 23], [6, 37], [7, 53]];
        let a = material_atlas_rgba(&specs);
        let b = material_atlas_rgba(&specs);
        assert_eq!(a, b, "atlas regeneration is byte-identical");
        assert_eq!(a.len(), (ATLAS_TILE_PX * 4 * ATLAS_TILE_PX * 4) as usize);
        // Non-flat per tile: each tile's channel spans a real range.
        for tile in 0..ATLAS_TILES {
            let mut mn = 255u8;
            let mut mx = 0u8;
            for i in 0..(ATLAS_TILE_PX * ATLAS_TILE_PX) as usize {
                let x = tile * ATLAS_TILE_PX as usize + i % ATLAS_TILE_PX as usize;
                let y = i / ATLAS_TILE_PX as usize;
                let v = a[(y * ATLAS_TILE_PX as usize * ATLAS_TILES + x) * 4];
                mn = mn.min(v);
                mx = mx.max(v);
            }
            assert!(mx > mn + 40, "tile {tile} has real grain ({mn}..{mx})");
        }
        // Tiles differ from each other (different seeds/scales).
        let at = |tile: usize, x: usize, y: usize| {
            a[(y * ATLAS_TILE_PX as usize * ATLAS_TILES + tile * ATLAS_TILE_PX as usize + x) * 4]
        };
        assert_ne!(at(0, 20, 20), at(1, 20, 20), "grass tile differs from rock");

        let m1 = leaf_mask_rgba();
        let m2 = leaf_mask_rgba();
        assert_eq!(m1, m2, "leaf mask regeneration is byte-identical");
        let holes = m1.chunks_exact(4).filter(|c| c[3] == 0).count();
        let solid = m1.chunks_exact(4).filter(|c| c[3] == 255).count();
        assert!(holes > 40, "the mask has nibble holes ({holes})");
        assert!(solid > 1500, "the mask is mostly leaf ({solid})");
        assert_eq!(
            solid_mask_rgba().chunks_exact(4).all(|c| c[3] == 255),
            true,
            "solid control is fully opaque"
        );
    }

    #[test]
    fn foliage_quads_are_well_formed() {
        let (verts, idx) = foliage_quads([0.0, 0.0, 0.0], 3, 3, 1.5);
        assert_eq!(verts.len(), 3 * 3 * 8, "two quads (8 verts) per plant");
        assert_eq!(idx.len(), 3 * 3 * 12, "two triangles per quad");
        // Every index is in range; UVs cover the full mask.
        assert!(idx.iter().all(|i| (*i as usize) < verts.len()));
        assert!(verts.iter().all(|v| v.uv[0] >= 0.0 && v.uv[0] <= 1.0));
        assert!(verts.iter().any(|v| v.uv == [0.0, 0.0]));
        assert!(verts.iter().any(|v| v.uv == [1.0, 1.0]));
        // Heights vary deterministically per plant.
        let hs: Vec<f32> = (0..9).map(|i| verts[i * 8 + 2].pos[1]).collect();
        let mut sorted = hs.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        assert_ne!(sorted[0], sorted[8], "plant heights vary");
        // Deterministic regeneration.
        let (v2, i2) = foliage_quads([0.0, 0.0, 0.0], 3, 3, 1.5);
        assert_eq!(idx, i2);
        assert!(verts.iter().zip(v2.iter()).all(|(a, b)| a.pos == b.pos));
    }
}

// ---------------------------------------------------------------------------
// GPU proofs (NWR-006): every atmosphere term, control-rendered.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod gpu_tests {
    use super::*;
    use crate::camera::CameraPose;
    use crate::scene::{sample_ndc, to_srgb4, SceneVertex, SUN_DIR};

    const W: usize = 384;
    const H: usize = 288;

    fn quad(
        verts: &mut Vec<SceneVertex>,
        idx: &mut Vec<u32>,
        a: [f32; 3],
        b: [f32; 3],
        c: [f32; 3],
        d: [f32; 3],
        normal: [f32; 3],
        color: [f32; 3],
    ) {
        let base = verts.len() as u32;
        for p in [a, b, c, d] {
            verts.push(SceneVertex {
                pos: p,
                normal,
                color,
            });
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// A grass-colored ground plane with one tall pillar at the origin —
    /// the shadow/cast test scene.
    fn ground_and_pillar() -> (Vec<SceneVertex>, Vec<u32>) {
        let mut v = Vec::new();
        let mut i = Vec::new();
        let grass: [f32; 3] = [0.30, 0.55, 0.22];
        // Ground: 9x9 tiles over [-60, 60]^2 (flat, +Y normal).
        for gz in 0..9 {
            for gx in 0..9 {
                let (x0, x1) = (
                    -60.0 + gx as f32 * 120.0 / 9.0,
                    -60.0 + (gx + 1) as f32 * 120.0 / 9.0,
                );
                let (z0, z1) = (
                    -60.0 + gz as f32 * 120.0 / 9.0,
                    -60.0 + (gz + 1) as f32 * 120.0 / 9.0,
                );
                quad(
                    &mut v,
                    &mut i,
                    [x0, 0.0, z0],
                    [x0, 0.0, z1],
                    [x1, 0.0, z1],
                    [x1, 0.0, z0],
                    [0.0, 1.0, 0.0],
                    grass,
                );
            }
        }
        // Pillar: 3x14x3 rock box at the origin.
        let rock: [f32; 3] = [0.55, 0.54, 0.50];
        let (lo, hi) = ([-1.5f32, 0.0, -1.5], [1.5f32, 14.0, 1.5]);
        quad(
            &mut v,
            &mut i,
            [lo[0], lo[1], hi[2]],
            [hi[0], lo[1], hi[2]],
            [hi[0], hi[1], hi[2]],
            [lo[0], hi[1], hi[2]],
            [0.0, 0.0, 1.0],
            rock,
        ); // +Z
        quad(
            &mut v,
            &mut i,
            [hi[0], lo[1], lo[2]],
            [lo[0], lo[1], lo[2]],
            [lo[0], hi[1], lo[2]],
            [hi[0], hi[1], lo[2]],
            [0.0, 0.0, -1.0],
            rock,
        ); // -Z
        quad(
            &mut v,
            &mut i,
            [hi[0], lo[1], hi[2]],
            [hi[0], lo[1], lo[2]],
            [hi[0], hi[1], lo[2]],
            [hi[0], hi[1], hi[2]],
            [1.0, 0.0, 0.0],
            rock,
        ); // +X
        quad(
            &mut v,
            &mut i,
            [lo[0], lo[1], lo[2]],
            [lo[0], lo[1], hi[2]],
            [lo[0], hi[1], hi[2]],
            [lo[0], hi[1], lo[2]],
            [-1.0, 0.0, 0.0],
            rock,
        ); // -X
        quad(
            &mut v,
            &mut i,
            [lo[0], hi[1], hi[2]],
            [hi[0], hi[1], hi[2]],
            [hi[0], hi[1], lo[2]],
            [lo[0], hi[1], lo[2]],
            [0.0, 1.0, 0.0],
            rock,
        ); // +Y
        (v, i)
    }

    fn pose_looking_at(eye: [f32; 3], at: [f32; 3]) -> CameraPose {
        let d = [at[0] - eye[0], at[1] - eye[1], at[2] - eye[2]];
        CameraPose::new(
            eye,
            (-d[0]).atan2(-d[2]),
            (d[1] / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()).asin(),
        )
    }

    fn all_off() -> Atmosphere {
        Atmosphere {
            shadow_res: 0,
            shadow_half_m: 60.0,
            fog_density: 0.0,
            fog_color: HAZE,
            detail_strength: 0.0,
            detail_scale: 0.35,
            glint: false,
        }
    }

    /// The pillar's shadow: the sun casts the pillar top toward -X/-Z; the
    /// shadow lands where the ray from the top through the sun direction
    /// meets y=0. Control (shadows off) vs on: the ground AT that point
    /// darkens, the sun-side ground does not, and a still camera produces
    /// a bit-identical map frame over frame.
    #[test]
    fn shadows_darken_ground_and_are_stable() {
        let (verts, idx) = ground_and_pillar();
        // Shadow of the pillar TOP (0,14,0): t = 14 / SUN_DIR[1].
        let t = 14.0 / SUN_DIR[1];
        let sp = [-SUN_DIR[0] * t, 0.0, -SUN_DIR[2] * t];
        let eye = [sp[0] - 12.0, 9.0, sp[2] + 26.0];
        let pose = pose_looking_at(eye, [sp[0], 0.4, sp[2]]);

        let mut r = crate::renderer::Renderer::offscreen(W as u32, H as u32);
        r.set_placeholder_scene(false);
        r.load_surface(&verts, &idx);
        r.set_pose(pose);
        r.set_atmosphere(all_off());
        let (_, no_shadow) =
            r.capture_png(&std::env::temp_dir().join("pc3d_atm_shadow_off.png"), &[]);
        r.set_atmosphere(Atmosphere {
            shadow_res: 1024,
            ..all_off()
        });
        let (_, with_shadow) =
            r.capture_png(&std::env::temp_dir().join("pc3d_atm_shadow_on.png"), &[]);
        // Diagnostics: what does the sun map actually hold?
        {
            let m = r.debug_shadow_map();
            let n = (1024usize * 1024).min(m.len());
            let mut mn = f32::MAX;
            let mut mx = f32::MIN;
            let mut ones = 0usize;
            for i in 0..n {
                mn = mn.min(m[i]);
                mx = mx.max(m[i]);
                if m[i] > 0.999 {
                    ones += 1;
                }
            }
            let center = m[512 * 512 + 512];
            println!("shadow map: n {n} min {mn:.4} max {mx:.4} ones {ones} center {center:.4}");
        }

        // The cast streak runs diagonally across the ground (the first
        // probe pinned ONE guessed pixel and missed it) — scan for the
        // strongest darkening instead; it must be LOCAL, so the mean
        // absolute delta stays tiny.
        let mut best = 0.0f32;
        let mut best_at = (0.0f32, 0.0f32);
        let mut mean = 0.0f32;
        let mut n = 0.0f32;
        for x in (0..W).step_by(2) {
            for y in (0..H).step_by(2) {
                let ndc = (x as f32 / 192.0 - 1.0, 1.0 - y as f32 / 144.0);
                let a = sample_ndc(&with_shadow, W as u32, H as u32, ndc);
                let b = sample_ndc(&no_shadow, W as u32, H as u32, ndc);
                let d: f32 = (0..3).map(|i| (a[i] - b[i]).abs()).sum();
                if d > best {
                    best = d;
                    best_at = ndc;
                }
                mean += d;
                n += 1.0;
            }
        }
        mean /= n;
        println!(
            "shadow: strongest darkening {best:.2} at NDC {best_at:?}, mean |delta| {mean:.4}"
        );
        assert!(best > 0.08, "the cast shadow darkens its ground ({best})");
        assert!(
            mean < 0.03,
            "the shadow change is local, not global ({mean})"
        );

        // Sun-side ground is NOT in shadow: a point +X of the pillar.
        let lit_pt = sample_ndc(&with_shadow, W as u32, H as u32, (0.7, -0.15));
        let lit_ctl = sample_ndc(&no_shadow, W as u32, H as u32, (0.7, -0.15));
        let lit_delta: f32 = (0..3).map(|i| (lit_pt[i] - lit_ctl[i]).abs()).sum();
        println!("sun-side ground delta {lit_delta:.3}");
        assert!(lit_delta < 0.04, "lit ground stays lit ({lit_delta})");

        // Temporal stability: two identical frames are bit-identical.
        let (_, again) =
            r.capture_png(&std::env::temp_dir().join("pc3d_atm_shadow_again.png"), &[]);
        assert_eq!(
            with_shadow, again,
            "a still camera gives a bit-identical shadow frame"
        );
    }

    /// Distance fog: a long ground strip recedes to the horizon; with fog
    /// on, FAR rows approach the haze color while NEAR rows keep their
    /// lit ground color (mirrored by apply_fog).
    #[test]
    fn fog_recedes_to_haze_over_distance() {
        let mut v = Vec::new();
        let mut i = Vec::new();
        let grass: [f32; 3] = [0.30, 0.55, 0.22];
        for gz in 0..30 {
            let z0 = -4.0 + gz as f32 * 12.0;
            quad(
                &mut v,
                &mut i,
                [-30.0, 0.0, z0],
                [-30.0, 0.0, z0 + 12.0],
                [30.0, 0.0, z0 + 12.0],
                [30.0, 0.0, z0],
                [0.0, 1.0, 0.0],
                grass,
            );
        }
        let eye = [0.0, 6.0, -14.0];
        let pose = pose_looking_at(eye, [0.0, 0.0, 220.0]);

        let mut r = crate::renderer::Renderer::offscreen(W as u32, H as u32);
        r.set_placeholder_scene(false);
        r.load_surface(&v, &i);
        r.set_pose(pose);
        r.set_atmosphere(all_off());
        let (_, clear) = r.capture_png(&std::env::temp_dir().join("pc3d_atm_fog_off.png"), &[]);
        let foggy = AtmosphereTier::High.params();
        r.set_atmosphere(Atmosphere {
            shadow_res: 0,
            ..foggy
        });
        let (_, fog) = r.capture_png(&std::env::temp_dir().join("pc3d_atm_fog_on.png"), &[]);

        let haze_srgb = to_srgb4(HAZE);
        let dist_to_haze = |img: &Vec<u8>, ndc_y: f32| -> f32 {
            let p = sample_ndc(img, W as u32, H as u32, (0.0, ndc_y));
            (0..3).map(|i| (p[i] - haze_srgb[i]).abs()).sum::<f32>()
        };
        // GROUND rows only (the first probe run sampled a sky row and
        // 'proved' nothing): far ground sits just under the horizon
        // (NDC y ~0.05), near ground low in frame (y ~-0.7).
        let far_off = dist_to_haze(&clear, 0.0);
        let far_on = dist_to_haze(&fog, 0.0);
        let near_off = dist_to_haze(&clear, -0.7);
        let near_on = dist_to_haze(&fog, -0.7);
        println!(
            "fog: far off {far_off:.2} on {far_on:.2}; near off {near_off:.2} on {near_on:.2}"
        );
        assert!(
            far_on < far_off * 0.7,
            "fog pulls the far field toward the haze"
        );
        assert!(
            near_on > far_on + 0.1,
            "near ground keeps its lit color more than the far field"
        );
        // The far field visibly changed between control and fog.
        let far_c = sample_ndc(&clear, W as u32, H as u32, (0.0, 0.0));
        let far_f = sample_ndc(&fog, W as u32, H as u32, (0.0, 0.0));
        let d: f32 = (0..3).map(|i| (far_c[i] - far_f[i]).abs()).sum();
        assert!(d > 0.1, "fog visibly lightens the far field ({d})");
    }

    /// Material detail: side-by-side grass and rock patches gain grain
    /// (within-patch variety grows) while staying separated (the
    /// cross-patch distance is preserved, each patch keeps its hue).
    #[test]
    fn material_detail_textures_and_separates_materials() {
        let mut v = Vec::new();
        let mut i = Vec::new();
        quad(
            &mut v,
            &mut i,
            [-20.0, 0.0, -10.0],
            [-20.0, 0.0, 10.0],
            [-1.0, 0.0, 10.0],
            [-1.0, 0.0, -10.0],
            [0.0, 1.0, 0.0],
            [0.30, 0.55, 0.22],
        ); // grass (left)
        quad(
            &mut v,
            &mut i,
            [1.0, 0.0, -10.0],
            [1.0, 0.0, 10.0],
            [20.0, 0.0, 10.0],
            [20.0, 0.0, -10.0],
            [0.0, 1.0, 0.0],
            [0.55, 0.54, 0.50],
        ); // rock (right)
        let eye = [0.0, 7.0, 26.0];
        let pose = pose_looking_at(eye, [0.0, 0.0, 0.0]);

        let mut r = crate::renderer::Renderer::offscreen(W as u32, H as u32);
        r.set_placeholder_scene(false);
        r.load_surface(&v, &i);
        r.set_pose(pose);
        r.set_atmosphere(all_off());
        let (_, flat) = r.capture_png(&std::env::temp_dir().join("pc3d_atm_mat_off.png"), &[]);
        r.set_atmosphere(Atmosphere {
            detail_strength: 0.85,
            ..all_off()
        });
        let (_, detailed) = r.capture_png(&std::env::temp_dir().join("pc3d_atm_mat_on.png"), &[]);

        let region_stats = |img: &Vec<u8>, x0: usize, x1: usize| -> (usize, [f32; 3]) {
            let mut colors = std::collections::BTreeSet::new();
            let mut acc = [0.0f32; 3];
            let mut n = 0.0;
            for x in (x0..x1).step_by(2) {
                for y in (132..178usize).step_by(2) {
                    let p = sample_ndc(
                        img,
                        W as u32,
                        H as u32,
                        (x as f32 / 192.0 - 1.0, 1.0 - y as f32 / 144.0),
                    );
                    colors.insert([
                        (p[0] * 255.0) as u32,
                        (p[1] * 255.0) as u32,
                        (p[2] * 255.0) as u32,
                    ]);
                    for i in 0..3 {
                        acc[i] += p[i];
                    }
                    n += 1.0;
                }
            }
            (colors.len(), [acc[0] / n, acc[1] / n, acc[2] / n])
        };
        let (flat_g, g_mean_flat) = region_stats(&flat, 30, 90);
        let (flat_r, r_mean_flat) = region_stats(&flat, 300, 360);
        let (det_g, g_mean_det) = region_stats(&detailed, 30, 90);
        let (det_r, r_mean_det) = region_stats(&detailed, 300, 360);
        println!("material detail: grass colors {flat_g} -> {det_g}, rock {flat_r} -> {det_r}");
        println!("means: grass {g_mean_flat:?} -> {g_mean_det:?}; rock {r_mean_flat:?} -> {r_mean_det:?}");
        assert!(det_g > flat_g + 20, "the grass patch gains grain");
        assert!(det_r > flat_r + 20, "the rock patch gains grain");
        // Separation: the cross-patch mean distance is preserved, and each
        // patch keeps its dominant hue.
        let sep_flat: f32 = (0..3)
            .map(|i| (g_mean_flat[i] - r_mean_flat[i]).abs())
            .sum();
        let sep_det: f32 = (0..3).map(|i| (g_mean_det[i] - r_mean_det[i]).abs()).sum();
        assert!(
            sep_det > sep_flat * 0.8,
            "materials stay separated ({sep_flat} -> {sep_det})"
        );
        assert!(g_mean_det[1] > g_mean_det[0], "grass stays green-dominant");
        assert!(
            (r_mean_det[0] - r_mean_det[1]).abs() < 0.06,
            "rock stays gray"
        );
    }

    /// Water glint: looking sunward across the water, the glint-on frame
    /// has visibly brighter water highlights; the look direction is the
    /// exact sun reflection so the lobe is in frame.
    #[test]
    fn water_glint_highlights_toward_the_sun() {
        let mut wverts = Vec::new();
        let mut widx = Vec::new();
        for z in 0..24 {
            let z0 = -40.0 + z as f32 * 5.0;
            let z1 = z0 + 5.0;
            let base = wverts.len() as u16;
            for p in [
                [-40.0, 0.4, z0],
                [40.0, 0.4, z0],
                [40.0, 0.4, z1],
                [-40.0, 0.4, z1],
            ] {
                wverts.push(crate::water::WaterVertex {
                    pos: p,
                    dir: [0.0, 1.0],
                    speed: 2.0,
                    alpha: 0.66,
                });
            }
            widx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        // Eye along the sun reflection: R = reflect(-S, up).
        let refl = [-SUN_DIR[0], SUN_DIR[1], -SUN_DIR[2]];
        let eye = [refl[0] * 40.0, refl[1] * 40.0 + 2.0, refl[2] * 40.0];
        let pose = pose_looking_at(eye, [0.0, 0.4, 0.0]);

        let mut r = crate::renderer::Renderer::offscreen(W as u32, H as u32);
        r.set_placeholder_scene(false);
        r.load_water_vertices(&wverts, &widx);
        r.set_water_time(Some(0.0));
        r.set_pose(pose);
        r.set_atmosphere(all_off());
        let (_, no_glint) =
            r.capture_png(&std::env::temp_dir().join("pc3d_atm_glint_off.png"), &[]);
        r.set_atmosphere(Atmosphere {
            glint: true,
            ..all_off()
        });
        let (_, glint) = r.capture_png(&std::env::temp_dir().join("pc3d_atm_glint_on.png"), &[]);

        let mut best_off = 0.0f32;
        let mut best_on = 0.0f32;
        // Scan the WATER half only: the sun disc saturates white in BOTH
        // frames and would mask the glint (the first probe run proved
        // nothing by comparing suns).
        for x in (0..W).step_by(4) {
            for y in (H / 2..H).step_by(4) {
                let a = sample_ndc(
                    &no_glint,
                    W as u32,
                    H as u32,
                    (x as f32 / 192.0 - 1.0, 1.0 - y as f32 / 144.0),
                );
                let b = sample_ndc(
                    &glint,
                    W as u32,
                    H as u32,
                    (x as f32 / 192.0 - 1.0, 1.0 - y as f32 / 144.0),
                );
                best_off = best_off.max(a[0] + a[1] + a[2]);
                best_on = best_on.max(b[0] + b[1] + b[2]);
            }
        }
        println!("water glint: brightest pixel sum off {best_off:.2} on {best_on:.2}");
        assert!(
            best_on > best_off + 0.15,
            "the sun glint adds a bright highlight on the water"
        );
    }

    /// Cutout foliage: crossed leaf cards before a marker wall — with the
    /// LEAF mask the wall shows through the nibble holes; with the SOLID
    /// control it does not.
    #[test]
    fn cutout_foliage_shows_the_background_through_holes() {
        // Marker wall (magenta — unique in the scene).
        let mut v = Vec::new();
        let mut i = Vec::new();
        quad(
            &mut v,
            &mut i,
            [-14.0, 0.0, -12.0],
            [14.0, 0.0, -12.0],
            [14.0, 8.0, -12.0],
            [-14.0, 8.0, -12.0],
            [0.0, 0.0, 1.0],
            [0.55, 0.25, 0.60],
        );
        let (fverts, fidx) = foliage_quads([-6.0, 0.0, -4.0], 4, 3, 3.2);
        let eye = [0.0, 3.5, 14.0];
        let pose = pose_looking_at(eye, [0.0, 1.6, -8.0]);

        let mut r = crate::renderer::Renderer::offscreen(W as u32, H as u32);
        r.set_placeholder_scene(false);
        r.load_surface(&v, &i);
        let tris = r.load_cutout(&fverts, &fidx, CutoutMask::Leaf);
        r.set_pose(pose);
        let (_, leaf) = r.capture_png(&std::env::temp_dir().join("pc3d_atm_cutout_leaf.png"), &[]);
        r.load_cutout(&fverts, &fidx, CutoutMask::Solid);
        let (_, solid) =
            r.capture_png(&std::env::temp_dir().join("pc3d_atm_cutout_solid.png"), &[]);

        // Magenta-dominant pixels in the FOLIAGE screen band (the wall
        // seen through holes): leaf > threshold, solid ~ none.
        let magenta = |img: &Vec<u8>, y0: usize, y1: usize| -> usize {
            let mut n = 0;
            for x in (0..W).step_by(2) {
                for y in (y0..y1).step_by(2) {
                    let p = sample_ndc(
                        img,
                        W as u32,
                        H as u32,
                        (x as f32 / 192.0 - 1.0, 1.0 - y as f32 / 144.0),
                    );
                    // The plum wall under dawn light lands near
                    // sRGB (0.64, 0.43, 0.67) — dominance, not ratio 1.8.
                    if p[0] > 0.45 && p[0] > p[1] * 1.25 && p[2] > p[1] * 1.25 {
                        n += 1;
                    }
                }
            }
            n
        };
        // The band shows open wall around the plants in BOTH frames; the
        // measurable difference is the wall the LEAF holes ADD where the
        // solid cards would block it.
        let through_leaf = magenta(&leaf, 60, 200);
        let through_solid = magenta(&solid, 60, 200);
        let diff = crate::scene::pixel_difference_fraction(&leaf, &solid);
        println!(
            "cutout: wall pixels leaf {through_leaf} vs solid {through_solid}, frame diff {diff:.3} ({tris} foliage tris)"
        );
        assert!(
            through_leaf > through_solid + 25,
            "the leaf holes show wall the solid cards block ({through_leaf} vs {through_solid})"
        );
        assert!(diff > 0.01, "the mask visibly changes the frame ({diff})");
    }

    /// Tier fallback: every tier renders a valid frame; Low skips the
    /// shadow pass entirely (its budget), Mid/High draw it. Timing rows
    /// print as evidence.
    #[test]
    fn atmosphere_tiers_render_and_time() {
        let (verts, idx) = ground_and_pillar();
        let eye = [-24.0, 9.0, 28.0];
        let pose = pose_looking_at(eye, [0.0, 2.0, 0.0]);
        for tier in [
            AtmosphereTier::Low,
            AtmosphereTier::Mid,
            AtmosphereTier::High,
        ] {
            let mut r = crate::renderer::Renderer::offscreen(W as u32, H as u32);
            r.set_placeholder_scene(false);
            r.load_surface(&verts, &idx);
            r.set_pose(pose);
            r.set_atmosphere_tier(tier);
            let t0 = std::time::Instant::now();
            let mut last = Vec::new();
            for _ in 0..10 {
                let (_, rgba) = r.capture_png(&std::env::temp_dir().join("pc3d_atm_tier.png"), &[]);
                last = rgba;
            }
            let us = t0.elapsed().as_micros() / 10;
            let report = crate::scene::verify_frame_rgba(&last, W as u32, H as u32, &[]);
            assert!(report.distinct_colors > 20, "{tier:?} renders a real scene");
            assert!(report.opaque, "{tier:?} frame is opaque");
            println!(
                "tier {:?} (label {}): shadow {}, {} us/frame avg over 10 frames, distinct {}",
                tier,
                tier.label(),
                r.atmosphere().shadow_res,
                us,
                report.distinct_colors
            );
            assert_eq!(
                r.atmosphere().shadow_res,
                tier.params().shadow_res,
                "the renderer carries the tier's shadow budget"
            );
        }
    }
}
