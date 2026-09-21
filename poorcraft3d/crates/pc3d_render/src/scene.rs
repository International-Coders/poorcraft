//! The R3DV-002 proof scene: "the dawn proving ground".
//!
//! Original POORCRAFT placeholder art in P3D world coordinates (meters,
//! right-handed, +X east / +Y up / +Z south): a 40x40 m stone ground plane,
//! a 2 m six-colored stone (each face a different palette color), and a 2 m
//! amber marker stone 5 m behind it. The scene exists to prove actual 3D
//! rendering — the proofs below are impossible without a camera, projection,
//! and depth buffer:
//!
//! - FACE FLIP: from the south the near stone reads crimson; from the east
//!   the same screen region reads gold. A 2D image cannot do this.
//! - OCCLUSION: the marker stone sits directly behind the near stone from
//!   the south (zero marker pixels visible); from the southeast it is fully
//!   visible. This is a depth-buffer proof.
//! - PARALLAX: frames from two poses differ in a large fraction of pixels.

use crate::camera::CameraPose;
use std::collections::HashSet;

/// A lit vertex: world position (meters), face normal, albedo color.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

pub const VERTEX_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<SceneVertex>() as wgpu::BufferAddress,
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
    ],
};

// Palette — linear albedo.
pub const COLOR_CRIMSON: [f32; 3] = [0.78, 0.16, 0.18];
pub const COLOR_GOLD: [f32; 3] = [0.92, 0.74, 0.20];
pub const COLOR_JADE: [f32; 3] = [0.16, 0.62, 0.42];
pub const COLOR_PLUM: [f32; 3] = [0.55, 0.25, 0.60];
pub const COLOR_PALE: [f32; 3] = [0.85, 0.83, 0.78];
pub const COLOR_CHARCOAL: [f32; 3] = [0.16, 0.15, 0.14];
pub const COLOR_STONE: [f32; 3] = [0.72, 0.70, 0.62];
pub const COLOR_MARKER: [f32; 3] = [0.95, 0.45, 0.10];

/// Warm dawn sun, low in the east (+X), slightly south.
pub const SUN_DIR: [f32; 3] = {
    // normalize((0.55, 0.35, 0.25)) — const-friendly normalization
    let l = 0.698_273_5; // sqrt(0.55^2+0.35^2+0.25^2)
    [0.55 / l, 0.35 / l, 0.25 / l]
};

/// Day phase in [0, 1): 0 = dawn, 0.25 = noon, 0.5 = dusk, 0.75 = midnight.
/// Returns a unit sun direction and a night factor (0 = full day, 1 = night).
pub fn sun_at_phase(phase: f32) -> ([f32; 3], f32) {
    let p = phase.rem_euclid(1.0);
    // Elevation peaks at noon (p = 0.25) and bottoms at midnight (p = 0.75).
    let elev = ((p - 0.25) * std::f32::consts::TAU).cos();
    let azim = p * std::f32::consts::TAU;
    let horiz = elev.max(0.05).sqrt();
    let mut dir = [azim.sin() * horiz, elev.max(-0.25), azim.cos() * horiz * 0.4];
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2])
        .sqrt()
        .max(1e-6);
    dir = [dir[0] / len, dir[1] / len, dir[2] / len];
    // Horizon (elev ≈ 0) is twilight, not full day — ramp night across
    // elev ∈ [-0.2, 0.2] so dawn/dusk dim while noon stays bright.
    let night = ((0.2 - elev) / 0.4).clamp(0.0, 1.0);
    (dir, night)
}

pub const AMBIENT: f32 = 0.35;
pub const SUN_STRENGTH: f32 = 1.1;

/// Mirror of the WGSL mesh lighting so pixel probes share one source of truth.
pub fn lit_color(albedo: [f32; 3], normal: [f32; 3]) -> [f32; 3] {
    // Hemisphere ambient (R3DV-010): sky above, ground below, sun on top —
    // the exact fs_mesh formula.
    let d = normal.iter().zip(SUN_DIR).map(|(n, s)| n * s).sum::<f32>();
    let light =
        (0.38 * (0.5 + 0.5 * normal[1]) + 0.22 * (0.5 - 0.5 * normal[1]) + 1.05 * d.max(0.0))
            .min(1.0);
    [
        (albedo[0] * light).clamp(0.0, 1.0),
        (albedo[1] * light).clamp(0.0, 1.0),
        (albedo[2] * light).clamp(0.0, 1.0),
    ]
}

// Sky constants — must mirror shaders/scene.wgsl exactly.
pub const SKY_ZENITH: [f32; 3] = [0.13, 0.27, 0.42];
pub const SKY_HORIZON: [f32; 3] = [0.95, 0.70, 0.44];

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Mirror of the WGSL sky color for a view direction (linear space).
pub fn sky_color_linear(dir: [f32; 3], sun: [f32; 3]) -> [f32; 3] {
    let t = smoothstep(-0.05, 0.5, dir[1]);
    let mut col = [
        SKY_HORIZON[0] + (SKY_ZENITH[0] - SKY_HORIZON[0]) * t,
        SKY_HORIZON[1] + (SKY_ZENITH[1] - SKY_HORIZON[1]) * t,
        SKY_HORIZON[2] + (SKY_ZENITH[2] - SKY_HORIZON[2]) * t,
    ];
    let s = dir[0] * sun[0] + dir[1] * sun[1] + dir[2] * sun[2];
    // Halo first, core second (core overwrites halo) — mirrors the shader.
    let halo = 0.55 * smoothstep(0.98, 0.998, s);
    let core = smoothstep(0.997, 0.9995, s);
    for i in 0..3 {
        col[i] = col[i] * (1.0 - halo) + [1.0, 0.80, 0.55][i] * halo;
        col[i] = col[i] * (1.0 - core) + [1.0, 0.93, 0.80][i] * core;
    }
    col
}

/// Face basis for axis-aligned cubes: (normal, u, v) with cross(u, v) =
/// normal, so every quad built from it winds CCW seen from outside
/// (FrontFace::Ccw + backface culling). Shared by the placeholder scene and
/// the construction mesher so both obey the same winding law.
pub const FACE_BASIS: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
    ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]), // south +Z
    ([0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]), // north -Z
    ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]), // east +X
    ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]), // west -X
    ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]), // top +Y
    ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]), // bottom -Y
];

/// One axis-aligned box: six faces, each with its own albedo.
fn box_faces(
    center: [f32; 3],
    half: f32,
    colors: &[[f32; 3]; 6],
) -> ([SceneVertex; 24], [u16; 36]) {
    let basis = FACE_BASIS;
    let mut verts = [SceneVertex {
        pos: [0.0; 3],
        normal: [0.0; 3],
        color: [0.0; 3],
    }; 24];
    let mut idx = [0u16; 36];
    for (f, (normal, u, v)) in basis.iter().enumerate() {
        let color = colors[f];
        let corner = |su: f32, sv: f32| SceneVertex {
            pos: [
                center[0] + normal[0] * half + u[0] * half * su + v[0] * half * sv,
                center[1] + normal[1] * half + u[1] * half * su + v[1] * half * sv,
                center[2] + normal[2] * half + u[2] * half * su + v[2] * half * sv,
            ],
            normal: *normal,
            color,
        };
        let base = (f * 4) as u16;
        verts[f * 4] = corner(-1.0, -1.0);
        verts[f * 4 + 1] = corner(1.0, -1.0);
        verts[f * 4 + 2] = corner(1.0, 1.0);
        verts[f * 4 + 3] = corner(-1.0, 1.0);
        let quad = [base, base + 1, base + 2, base, base + 2, base + 3];
        idx[f * 6..f * 6 + 6].copy_from_slice(&quad);
    }
    (verts, idx)
}

/// Builds the proof scene: ground plane, six-colored stone, marker stone.
pub fn build_scene() -> (Vec<SceneVertex>, Vec<u16>) {
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    let mut push = |v: &[SceneVertex], i: &[u16]| {
        let base = verts.len() as u16;
        verts.extend_from_slice(v);
        idx.extend(i.iter().map(|k| k + base));
    };

    // Ground: 40x40 m plane at y=0 (normal +Y).
    let gv = [
        SceneVertex {
            pos: [-20.0, 0.0, 20.0],
            normal: [0.0, 1.0, 0.0],
            color: COLOR_STONE,
        },
        SceneVertex {
            pos: [20.0, 0.0, 20.0],
            normal: [0.0, 1.0, 0.0],
            color: COLOR_STONE,
        },
        SceneVertex {
            pos: [20.0, 0.0, -20.0],
            normal: [0.0, 1.0, 0.0],
            color: COLOR_STONE,
        },
        SceneVertex {
            pos: [-20.0, 0.0, -20.0],
            normal: [0.0, 1.0, 0.0],
            color: COLOR_STONE,
        },
    ];
    push(&gv, &[0, 1, 2, 0, 2, 3]);

    // Near stone at (0, 1, -4): six distinct face colors, south face crimson,
    // east face gold. The eye height (1.7 m) puts its south face at screen
    // center from pose A.
    let (cv, ci) = box_faces(
        [0.0, 1.0, -4.0],
        1.0,
        &[
            COLOR_CRIMSON,  // +Z south
            COLOR_JADE,     // -Z north
            COLOR_GOLD,     // +X east
            COLOR_PLUM,     // -X west
            COLOR_PALE,     // +Y top
            COLOR_CHARCOAL, // -Y bottom
        ],
    );
    push(&cv, &ci);

    // Marker stone at (0, 1, -9), entirely amber: hidden behind the near
    // stone from pose A, visible from pose C.
    let (mv, mi) = box_faces([0.0, 1.0, -9.0], 1.0, &[COLOR_MARKER; 6]);
    push(&mv, &mi);

    (verts, idx)
}

// ---------------------------------------------------------------------------
// Proof poses. Eye height 1.7 m; poses chosen so the three 3D proofs read
// directly off the screen center.
// ---------------------------------------------------------------------------

/// South of the near stone, looking north: center = crimson south face.
/// The marker stone is fully occluded behind the near stone.
pub fn pose_a() -> CameraPose {
    CameraPose::new([0.0, 1.7, 2.0], 0.0, 0.0)
}

/// East of the near stone, looking west: center = gold east face.
pub fn pose_b() -> CameraPose {
    CameraPose::new([4.0, 1.7, -4.0], std::f32::consts::FRAC_PI_2, 0.0)
}

/// Southeast of the marker stone, looking at it: center = amber marker.
/// yaw = atan(6/7), pitch = -atan(0.7/9.2195) — the eye->marker-center ray.
pub fn pose_c() -> CameraPose {
    CameraPose::new([6.0, 1.7, -2.0], 0.708_626, -0.075_782)
}

// ---------------------------------------------------------------------------
// Generic semantic verification over an RGBA8 framebuffer, shared by the
// offscreen GPU tests and the live windowed capture.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct Probe {
    pub name: &'static str,
    /// NDC coordinates (-1..1, y up).
    pub ndc: (f32, f32),
    /// Expected sRGB RGBA color.
    pub expected: [f32; 4],
    pub tol: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PixelReport {
    pub width: u32,
    pub height: u32,
    /// Unique RGB colors among sampled pixels — a flat clear would report 1.
    pub distinct_colors: usize,
    /// Every sampled pixel is fully opaque.
    pub opaque: bool,
    /// (probe name, passed) for every requested probe.
    pub probes: Vec<(&'static str, bool)>,
}

impl PixelReport {
    pub fn passes(&self) -> bool {
        self.passes_with(100)
    }

    /// `min_distinct` relaxes the nonuniformity floor for close-up interior
    /// views (a cave wall at 2 m is a handful of flat face colors — the
    /// semantic probes carry the proof there); the default gate stays 100
    /// for open scenes.
    pub fn passes_with(&self, min_distinct: usize) -> bool {
        // A flat clear reports 1; any real lit 3D scene yields hundreds.
        self.distinct_colors >= min_distinct && self.opaque && self.probes.iter().all(|(_, ok)| *ok)
    }

    pub fn failed_probes(&self) -> Vec<&'static str> {
        self.probes
            .iter()
            .filter(|(_, ok)| !ok)
            .map(|(name, _)| *name)
            .collect()
    }
}

fn to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

pub fn to_srgb4(c: [f32; 3]) -> [f32; 4] {
    [to_srgb(c[0]), to_srgb(c[1]), to_srgb(c[2]), 1.0]
}

/// NDC (x right, y up) to pixel coordinates (row 0 = top).
fn ndc_to_px(nx: f32, ny: f32, w: u32, h: u32) -> (usize, usize) {
    let px = ((nx + 1.0) * 0.5 * w as f32) as usize;
    let py = ((1.0 - (ny + 1.0) * 0.5) * h as f32) as usize;
    (px.min(w as usize - 1), py.min(h as usize - 1))
}

fn pixel_at(rgba: &[u8], w: u32, h: u32, nx: f32, ny: f32) -> [f32; 4] {
    let (x, y) = ndc_to_px(nx, ny, w, h);
    let i = (y * w as usize + x) * 4;
    [
        rgba[i] as f32 / 255.0,
        rgba[i + 1] as f32 / 255.0,
        rgba[i + 2] as f32 / 255.0,
        rgba[i + 3] as f32 / 255.0,
    ]
}

/// Public pixel sampler at an NDC point (proof code compares specific
/// screen locations between frames).
pub fn sample_ndc(rgba: &[u8], w: u32, h: u32, ndc: (f32, f32)) -> [f32; 4] {
    pixel_at(rgba, w, h, ndc.0, ndc.1)
}

/// Verifies an RGBA8 framebuffer (RGBA byte order) against generic 3D-scene
/// expectations plus caller-supplied probes.
pub fn verify_frame_rgba(rgba: &[u8], width: u32, height: u32, probes: &[Probe]) -> PixelReport {
    assert_eq!(
        rgba.len(),
        (width * height * 4) as usize,
        "framebuffer size mismatch"
    );

    let mut distinct = HashSet::new();
    let mut opaque = true;
    let mut sy = 0;
    while sy < height {
        let mut sx = 0;
        while sx < width {
            let i = (sy * width + sx) as usize * 4;
            distinct.insert((rgba[i], rgba[i + 1], rgba[i + 2]));
            if rgba[i + 3] != 255 {
                opaque = false;
            }
            sx += 2;
        }
        sy += 2;
    }

    let probe_results = probes
        .iter()
        .map(|p| {
            let px = pixel_at(rgba, width, height, p.ndc.0, p.ndc.1);
            let ok = px
                .iter()
                .zip(p.expected)
                .all(|(a, e)| (a - e).abs() <= p.tol);
            (p.name, ok)
        })
        .collect();

    PixelReport {
        width,
        height,
        distinct_colors: distinct.len(),
        opaque,
        probes: probe_results,
    }
}

// ---------------------------------------------------------------------------
// THE SUBJECT LAWS
//
// `PixelReport::passes` asks whether the frame is colorful and opaque. It
// never asks whether the thing the proof is named after is actually on
// screen, which is how `windowed_city.png` passed the castle-city gate while
// showing an empty sky and a grey plate.
//
// A proof now declares the world-space box its subject occupies. The law
// projects that box and reads the pixels inside it.
// ---------------------------------------------------------------------------

/// An axis-aligned world-space box, in meters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Aabb {
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Aabb {
        Aabb { min, max }
    }

    /// The box around a point, `r` meters in every direction.
    pub fn around(c: [f32; 3], r: f32) -> Aabb {
        Aabb {
            min: [c[0] - r, c[1] - r, c[2] - r],
            max: [c[0] + r, c[1] + r, c[2] + r],
        }
    }

    pub fn corners(&self) -> [[f32; 3]; 8] {
        let (a, b) = (self.min, self.max);
        [
            [a[0], a[1], a[2]], [b[0], a[1], a[2]],
            [a[0], b[1], a[2]], [b[0], b[1], a[2]],
            [a[0], a[1], b[2]], [b[0], a[1], b[2]],
            [a[0], b[1], b[2]], [b[0], b[1], b[2]],
        ]
    }
}

/// The verdict of [`subject_in_frame`].
#[derive(Debug, Clone, PartialEq)]
pub struct SubjectReport {
    /// Pixel rect the subject projects to, clipped to the frame.
    pub rect: Option<(u32, u32, u32, u32)>,
    /// Subject pixels as a fraction of the whole frame.
    pub coverage: f32,
    /// Distinct colors inside the subject rect.
    pub distinct_colors: usize,
    /// Fraction of subject pixels that match the frame's sky color.
    pub sky_fraction: f32,
    /// Subject centre in normalised frame coords (0..1, y down). A well-aimed
    /// camera puts this near (0.5, 0.5).
    pub center: (f32, f32),
    pub reason: Option<String>,
}

impl SubjectReport {
    pub fn passes(&self) -> bool {
        self.reason.is_none()
    }
}

/// Projects a world AABB into the pixel rect it covers from `pose`.
/// `None` when the box is entirely behind the camera or off screen.
pub fn project_aabb(
    pose: CameraPose,
    aspect: f32,
    aabb: Aabb,
    width: u32,
    height: u32,
) -> Option<(u32, u32, u32, u32)> {
    let vp = crate::camera::Camera::new(pose).view_proj(aspect);
    let (mut lo_x, mut lo_y) = (f32::MAX, f32::MAX);
    let (mut hi_x, mut hi_y) = (f32::MIN, f32::MIN);
    let mut any = false;
    for p in aabb.corners() {
        let x = vp[0] * p[0] + vp[4] * p[1] + vp[8] * p[2] + vp[12];
        let y = vp[1] * p[0] + vp[5] * p[1] + vp[9] * p[2] + vp[13];
        let w = vp[3] * p[0] + vp[7] * p[1] + vp[11] * p[2] + vp[15];
        // Behind the eye: that corner contributes nothing.
        if w <= 1e-6 {
            continue;
        }
        any = true;
        let (nx, ny) = (x / w, y / w);
        lo_x = lo_x.min(nx);
        hi_x = hi_x.max(nx);
        lo_y = lo_y.min(ny);
        hi_y = hi_y.max(ny);
    }
    if !any {
        return None;
    }
    let to_px_x = |n: f32| ((n + 1.0) * 0.5 * width as f32).clamp(0.0, width as f32);
    // NDC y is up; rows count down.
    let to_px_y = |n: f32| ((1.0 - n) * 0.5 * height as f32).clamp(0.0, height as f32);
    let x0 = to_px_x(lo_x).floor() as u32;
    let x1 = to_px_x(hi_x).ceil() as u32;
    // hi_y (NDC, up) maps to the smaller row.
    let y0 = to_px_y(hi_y).floor() as u32;
    let y1 = to_px_y(lo_y).ceil() as u32;
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some((x0, y0, x1 - x0, y1 - y0))
}

/// THE SUBJECT-IN-FRAME LAW: the thing a proof is named after must occupy a
/// real share of the frame and must not be the empty sky.
///
/// `sky` is the frame's background color (sampled from a corner by
/// [`frame_sky_color`]). A subject rect that is almost entirely sky means the
/// camera is pointed somewhere the subject is not, which is exactly the
/// failure the old gates could not see.
pub fn subject_in_frame(
    rgba: &[u8],
    width: u32,
    height: u32,
    pose: CameraPose,
    aabb: Aabb,
    min_coverage: f32,
) -> SubjectReport {
    let aspect = width as f32 / height as f32;
    let sky = frame_sky_color(rgba, width, height);
    let Some(rect) = project_aabb(pose, aspect, aabb, width, height) else {
        return SubjectReport {
            rect: None,
            coverage: 0.0,
            distinct_colors: 0,
            sky_fraction: 1.0,
            center: (0.5, 0.5),
            reason: Some("subject projects entirely off screen or behind the camera".into()),
        };
    };
    let (rx, ry, rw, rh) = rect;
    let mut distinct = HashSet::new();
    let mut sky_px = 0u64;
    let mut total = 0u64;
    for y in ry..(ry + rh).min(height) {
        for x in rx..(rx + rw).min(width) {
            let i = ((y * width + x) * 4) as usize;
            let c = (rgba[i], rgba[i + 1], rgba[i + 2]);
            distinct.insert(c);
            if near_color(c, sky, 10) {
                sky_px += 1;
            }
            total += 1;
        }
    }
    let coverage = total as f32 / (width as f32 * height as f32);
    let sky_fraction = if total == 0 { 1.0 } else { sky_px as f32 / total as f32 };
    let center = (
        (rx as f32 + rw as f32 / 2.0) / width as f32,
        (ry as f32 + rh as f32 / 2.0) / height as f32,
    );
    // A camera aimed at its subject puts the subject near the middle. Outside
    // this band the subject is clinging to an edge, which is what a
    // mis-pitched proof camera produces.
    let off_center = (center.0 - 0.5).abs().max((center.1 - 0.5).abs()) > 0.2;
    let reason = if coverage < min_coverage {
        Some(format!(
            "subject covers {:.4} of the frame, below the {:.4} the proof demands",
            coverage, min_coverage
        ))
    } else if sky_fraction > 0.95 {
        Some(format!(
            "subject rect is {:.1}% sky — the camera is not looking at it",
            sky_fraction * 100.0
        ))
    } else if off_center {
        Some(format!(
            "subject centre is ({:.2}, {:.2}) — the camera is not aimed at it",
            center.0, center.1
        ))
    } else if distinct.len() < 3 {
        Some(format!(
            "subject rect holds {} distinct colors — nothing is drawn there",
            distinct.len()
        ))
    } else {
        None
    };
    SubjectReport {
        rect: Some(rect),
        coverage,
        distinct_colors: distinct.len(),
        sky_fraction,
        center,
        reason,
    }
}

/// The frame's background color: the most common color along the top row,
/// which is sky in every outdoor proof pose.
pub fn frame_sky_color(rgba: &[u8], width: u32, height: u32) -> (u8, u8, u8) {
    let mut counts: std::collections::HashMap<(u8, u8, u8), u32> = std::collections::HashMap::new();
    let _ = height;
    for x in 0..width {
        let i = (x * 4) as usize;
        *counts.entry((rgba[i], rgba[i + 1], rgba[i + 2])).or_default() += 1;
    }
    counts.into_iter().max_by_key(|(_, n)| *n).map(|(c, _)| c).unwrap_or((0, 0, 0))
}

fn near_color(a: (u8, u8, u8), b: (u8, u8, u8), tol: i32) -> bool {
    (a.0 as i32 - b.0 as i32).abs() <= tol
        && (a.1 as i32 - b.1 as i32).abs() <= tol
        && (a.2 as i32 - b.2 as i32).abs() <= tol
}

/// THE GROUNDED-GEOMETRY LAW: a placed instance's base must sit on the
/// terrain, not hover above it.
///
/// Returns the placements whose base is further than `tol` meters from the
/// ground height sampled beneath them. Floating canopies and hovering props
/// are exactly this defect, and no pixel check can name them.
pub fn ungrounded_placements<F>(
    placements: &[(String, [f32; 3])],
    ground_at: F,
    tol: f32,
) -> Vec<(String, f32)>
where
    F: Fn(f32, f32) -> Option<f32>,
{
    placements
        .iter()
        .filter_map(|(id, pos)| {
            let g = ground_at(pos[0], pos[2])?;
            let gap = pos[1] - g;
            if gap.abs() > tol {
                Some((id.clone(), gap))
            } else {
                None
            }
        })
        .collect()
}

/// View direction for an NDC point given a camera pose (mirrors the sky
/// shader's ray reconstruction; needs the same fov/aspect the renderer used).
pub fn dir_from_ndc(pose: CameraPose, ndc: (f32, f32), aspect: f32) -> [f32; 3] {
    let fwd = crate::camera::fwd_of(pose.yaw, pose.pitch);
    let right = crate::camera::right_of(fwd);
    let up = crate::camera::up_of(fwd, right);
    let tan = crate::camera::Camera::new(pose).tan_half_fov();
    let mut d = [
        fwd[0] + ndc.0 * tan * aspect * right[0] + ndc.1 * tan * up[0],
        fwd[1] + ndc.0 * tan * aspect * right[1] + ndc.1 * tan * up[1],
        fwd[2] + ndc.0 * tan * aspect * right[2] + ndc.1 * tan * up[2],
    ];
    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    d = [d[0] / l, d[1] / l, d[2] / l];
    d
}

/// The NDC position a world-space point projects to for a camera pose —
/// used to place semantic probes on rendered construction cells.
pub fn project_ndc(pose: CameraPose, aspect: f32, world: [f32; 3]) -> (f32, f32) {
    let vp = crate::camera::Camera::new(pose).view_proj(aspect);
    let x = vp[0] * world[0] + vp[4] * world[1] + vp[8] * world[2] + vp[12];
    let y = vp[1] * world[0] + vp[5] * world[1] + vp[9] * world[2] + vp[13];
    let w = vp[3] * world[0] + vp[7] * world[1] + vp[11] * world[2] + vp[15];
    (x / w, y / w)
}

/// Probes for a known pose: screen center content, ground below the horizon,
/// and sky above it. Aspect must match the render target.
pub fn probes_for_pose(pose: CameraPose, aspect: f32) -> Vec<Probe> {
    let mut probes = Vec::new();
    match pose {
        p if p == pose_a() => {
            probes.push(Probe {
                name: "center_is_crimson_south_face",
                ndc: (0.0, 0.0),
                expected: to_srgb4(lit_color(COLOR_CRIMSON, [0.0, 0.0, 1.0])),
                tol: 0.05,
            });
        }
        p if p == pose_b() => {
            probes.push(Probe {
                name: "center_is_gold_east_face",
                ndc: (0.0, 0.0),
                expected: to_srgb4(lit_color(COLOR_GOLD, [1.0, 0.0, 0.0])),
                tol: 0.05,
            });
        }
        p if p == pose_c() => {
            // The eye->marker-center ray enters through the marker's SOUTH
            // face (z = -8 plane at x = 0.857, y = 1.1).
            probes.push(Probe {
                name: "center_is_amber_marker",
                ndc: (0.0, 0.0),
                expected: to_srgb4(lit_color(COLOR_MARKER, [0.0, 0.0, 1.0])),
                tol: 0.06,
            });
        }
        _ => {}
    }
    // Ground below the horizon (stone, top face lighting). From pose B the
    // near stone's east face fills the region just below center, so sample
    // further down where bare ground is guaranteed.
    let ground_ndc = if pose == pose_b() {
        (0.0, -0.85)
    } else {
        (0.0, -0.6)
    };
    probes.push(Probe {
        name: "ground_below_horizon",
        ndc: ground_ndc,
        expected: to_srgb4(lit_color(COLOR_STONE, [0.0, 1.0, 0.0])),
        tol: 0.06,
    });
    // Sky above the horizon (mirror of the shader's gradient math).
    let dir = dir_from_ndc(pose, (0.0, 0.8), aspect);
    probes.push(Probe {
        name: "sky_above_horizon",
        ndc: (0.0, 0.8),
        expected: to_srgb4(sky_color_linear(dir, SUN_DIR)),
        tol: 0.05,
    });
    probes
}

/// Fraction of pixels differing by more than a small threshold — the
/// parallax measure between two frames.
pub fn pixel_difference_fraction(a: &[u8], b: &[u8]) -> f32 {
    assert_eq!(a.len(), b.len(), "frame size mismatch");
    assert!(a.len() % 4 == 0);
    let mut diff = 0usize;
    let total = a.len() / 4;
    for px in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let (p, q) = px;
        if (p[0] as i32 - q[0] as i32).abs() > 8
            || (p[1] as i32 - q[1] as i32).abs() > 8
            || (p[2] as i32 - q[2] as i32).abs() > 8
        {
            diff += 1;
        }
    }
    diff as f32 / total as f32
}

/// Counts amber-marker-colored pixels (hue scan, robust to lighting level):
/// the occlusion proof — pose A must report ZERO, pose C many.
pub fn marker_hue_pixels(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|p| {
            let (r, g, b) = (p[0] as i32, p[1] as i32, p[2] as i32);
            r > 140 && g > 60 && g < 180 && b < 120 && r > g && g > b
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sun_at_phase_arcs_from_dawn_to_night() {
        let (dawn, n0) = sun_at_phase(0.0);
        let (noon, n1) = sun_at_phase(0.25);
        let (_dusk, n2) = sun_at_phase(0.5);
        let (_mid, n3) = sun_at_phase(0.75);
        assert!(noon[1] > dawn[1], "noon is higher than dawn");
        assert!(n1 < 0.15, "noon is day ({n1})");
        assert!(n0 > 0.4 && n0 < 0.9, "dawn is twilight ({n0})");
        assert!((n0 - n2).abs() < 0.15, "dawn and dusk are symmetric");
        assert!(n3 > 0.85, "midnight is night ({n3})");
    }

    #[test]
    fn scene_mesh_is_valid_indexed_geometry() {
        let (verts, idx) = build_scene();
        // 1 ground quad + 2 six-faced boxes = 26 triangles.
        assert_eq!(idx.len(), 26 * 3);
        assert_eq!(verts.len(), 4 + 24 + 24);
        assert!(idx.iter().all(|&i| (i as usize) < verts.len()));
    }

    #[test]
    fn every_triangle_winds_ccw_outward_for_backface_culling() {
        let (verts, idx) = build_scene();
        for tri in idx.chunks(3) {
            let (a, b, c) = (
                verts[tri[0] as usize],
                verts[tri[1] as usize],
                verts[tri[2] as usize],
            );
            let e1 = [
                b.pos[0] - a.pos[0],
                b.pos[1] - a.pos[1],
                b.pos[2] - a.pos[2],
            ];
            let e2 = [
                c.pos[0] - a.pos[0],
                c.pos[1] - a.pos[1],
                c.pos[2] - a.pos[2],
            ];
            let cross = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let dot = cross[0] * a.normal[0] + cross[1] * a.normal[1] + cross[2] * a.normal[2];
            assert!(dot > 0.0, "triangle winds inward: dot {dot}");
        }
    }

    #[test]
    fn near_stone_has_six_distinct_faces() {
        let (verts, _) = build_scene();
        let faces: Vec<[u8; 3]> = verts[4..28]
            .chunks(4)
            .map(|f| f[0].color.map(|c| (c * 255.0) as u8))
            .collect();
        let mut unique = faces.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 6, "six-colored stone must have 6 face colors");
        // South face (drawn first) is crimson; east face (third) is gold.
        let crimson = COLOR_CRIMSON.map(|c| (c * 255.0) as u8);
        let gold = COLOR_GOLD.map(|c| (c * 255.0) as u8);
        assert_eq!(faces[0], crimson);
        assert_eq!(faces[2], gold);
    }

    #[test]
    fn flat_framebuffer_fails_verification() {
        let flat = vec![40u8; (64 * 64 * 4) as usize];
        let report = verify_frame_rgba(&flat, 64, 64, &[]);
        assert_eq!(report.distinct_colors, 1);
        assert!(!report.passes());
    }

    #[test]
    fn probe_expectations_differ_between_poses() {
        // The whole point: the SAME screen region must expect different
        // content from different viewpoints.
        let a = probes_for_pose(pose_a(), 4.0 / 3.0);
        let b = probes_for_pose(pose_b(), 4.0 / 3.0);
        let ca = a
            .iter()
            .find(|p| p.name == "center_is_crimson_south_face")
            .unwrap();
        let cb = b
            .iter()
            .find(|p| p.name == "center_is_gold_east_face")
            .unwrap();
        let dr = (ca.expected[0] - cb.expected[0]).abs();
        assert!(dr > 0.15, "crimson and gold centers must be far apart");
    }

    #[test]
    fn parallax_and_marker_scans_work() {
        let mut a = vec![0u8; 16 * 4];
        let mut b = vec![0u8; 16 * 4];
        assert_eq!(pixel_difference_fraction(&a, &b), 0.0);
        b[0] = 255; // 1 of 16 pixels differs beyond the threshold
        assert_eq!(pixel_difference_fraction(&a, &b), 1.0 / 16.0);
        // Marker hue scan: an amber pixel counts, a crimson one doesn't.
        let mut frame = vec![0u8; 4];
        frame[..3].copy_from_slice(&[220, 120, 40]);
        assert_eq!(marker_hue_pixels(&frame), 1);
        frame[..3].copy_from_slice(&[210, 60, 60]);
        assert_eq!(marker_hue_pixels(&frame), 0);
    }

    /// A synthetic frame: `sky` everywhere, with `subject` painted into the
    /// given pixel rect.
    fn frame_with(
        w: u32,
        h: u32,
        sky: [u8; 3],
        subject: Option<((u32, u32, u32, u32), [u8; 3])>,
    ) -> Vec<u8> {
        let mut f = vec![255u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                f[i..i + 3].copy_from_slice(&sky);
            }
        }
        if let Some(((rx, ry, rw, rh), c)) = subject {
            for y in ry..(ry + rh).min(h) {
                for x in rx..(rx + rw).min(w) {
                    let i = ((y * w + x) * 4) as usize;
                    // Vary slightly so the distinct-color floor is met.
                    f[i] = c[0].saturating_add((x % 3) as u8);
                    f[i + 1] = c[1].saturating_add((y % 3) as u8);
                    f[i + 2] = c[2];
                }
            }
        }
        f
    }

    #[test]
    fn project_aabb_brackets_the_subject_and_rejects_what_is_behind() {
        // Camera at origin looking north (-Z); a 4 m cube 10 m ahead.
        let pose = CameraPose::new([0.0, 0.0, 0.0], 0.0, 0.0);
        let ahead = Aabb::new([-2.0, -2.0, -12.0], [2.0, 2.0, -8.0]);
        let r = project_aabb(pose, 1.0, ahead, 400, 400).expect("cube ahead must project");
        let (x, y, w, h) = r;
        // Symmetric about the center, and not the whole frame.
        assert!((x as i32 + (w / 2) as i32 - 200).abs() <= 2, "not centered: {r:?}");
        assert!((y as i32 + (h / 2) as i32 - 200).abs() <= 2, "not centered: {r:?}");
        assert!(w < 400 && w > 20, "implausible width: {r:?}");

        // The same cube directly behind the eye must not project at all.
        let behind = Aabb::new([-2.0, -2.0, 8.0], [2.0, 2.0, 12.0]);
        assert_eq!(project_aabb(pose, 1.0, behind, 400, 400), None);
    }

    /// THE SUBJECT-IN-FRAME LAW must accept a framed subject and reject the
    /// two ways a proof lies: subject too small, and subject is empty sky.
    #[test]
    fn subject_in_frame_catches_the_empty_sky_and_the_speck() {
        let pose = CameraPose::new([0.0, 0.0, 0.0], 0.0, 0.0);
        let subject = Aabb::new([-2.0, -2.0, -12.0], [2.0, 2.0, -8.0]);
        let sky = [90, 140, 210];
        let rect = project_aabb(pose, 1.0, subject, 400, 400).unwrap();

        // Drawn: passes.
        let good = frame_with(400, 400, sky, Some((rect, [120, 90, 60])));
        let rep = subject_in_frame(&good, 400, 400, pose, subject, 0.01);
        assert!(rep.passes(), "a drawn subject must pass: {rep:?}");
        assert!(rep.sky_fraction < 0.05, "{rep:?}");

        // Not drawn — the rect is pure sky. This is `windowed_city.png`.
        let empty = frame_with(400, 400, sky, None);
        let rep = subject_in_frame(&empty, 400, 400, pose, subject, 0.01);
        assert!(!rep.passes(), "an all-sky subject rect must fail");
        assert!(
            rep.reason.as_deref().unwrap().contains("sky"),
            "wrong reason: {rep:?}"
        );

        // Framed so small it proves nothing: demand more coverage than a
        // 4 m cube at 10 m can give.
        let rep = subject_in_frame(&good, 400, 400, pose, subject, 0.9);
        assert!(!rep.passes(), "a subject below the coverage floor must fail");
        assert!(rep.reason.as_deref().unwrap().contains("covers"));
    }

    /// A camera pitched too shallow for its own height looks over its
    /// subject. The law must both aim the check (the projected rect moves
    /// down the frame) and reject the miss.
    ///
    /// This is the geometry behind `windowed_city.png`: the city overview eye
    /// sits `span*0.9` above and `span*0.8` south of the city centre, which
    /// needs a 48-degree downward pitch, but the proof hardcodes 30 degrees.
    #[test]
    fn subject_in_frame_follows_the_aim_and_rejects_a_miss() {
        let subject = Aabb::new([-20.0, 0.0, -20.0], [20.0, 12.0, 20.0]);
        let eye = [0.0, 90.0, 80.0];
        let sky = [90, 140, 210];
        let shallow = CameraPose::new(eye, 0.0, (-0.75f32).atan2(1.3));
        let aimed = CameraPose::new(eye, 0.0, (-90.0f32).atan2(80.0));

        // The aimed pose centres the subject; the shallow one pushes it down.
        let lo = project_aabb(shallow, 1.6, subject, 800, 500).expect("still on screen");
        let hi = project_aabb(aimed, 1.6, subject, 800, 500).expect("aimed");
        assert!(
            lo.1 > hi.1,
            "the shallow pitch must push the subject down the frame: {lo:?} vs {hi:?}"
        );
        assert!(
            (hi.1 + hi.3 / 2).abs_diff(250) < 40,
            "the aimed pose must centre the subject vertically: {hi:?}"
        );

        // Aim at the subject and draw it: the law is satisfied.
        let drawn = frame_with(800, 500, sky, Some((hi, [120, 110, 100])));
        assert!(subject_in_frame(&drawn, 800, 500, aimed, subject, 0.02).passes());

        // Aim past it so nothing is drawn where the subject should be: the
        // law names the miss even though the frame is not blank.
        let missed = frame_with(800, 500, sky, Some(((0, 450, 800, 50), [120, 110, 100])));
        let rep = subject_in_frame(&missed, 800, 500, aimed, subject, 0.02);
        assert!(!rep.passes(), "a subject rect full of sky must fail: {rep:?}");

        // The shallow pitch draws the subject, but clinging to the bottom
        // edge. Coverage alone would not catch that; the aim check does.
        let low = frame_with(800, 500, sky, Some((lo, [120, 110, 100])));
        let rep = subject_in_frame(&low, 800, 500, shallow, subject, 0.02);
        assert!(!rep.passes(), "an edge-clinging subject must fail: {rep:?}");
        assert!(
            rep.reason.as_deref().unwrap().contains("not aimed"),
            "wrong reason: {rep:?}"
        );
    }

    /// THE GROUNDED-GEOMETRY LAW: a hovering placement is named, a seated
    /// one is not.
    #[test]
    fn ungrounded_placements_names_the_floating_canopy() {
        // Flat ground at y = 50.
        let ground = |_x: f32, _z: f32| Some(50.0);
        let placements = vec![
            ("trunk".to_string(), [10.0, 50.0, 10.0]),
            ("canopy_floating".to_string(), [12.0, 62.0, 12.0]),
            ("shrub_sunk".to_string(), [14.0, 46.0, 14.0]),
            ("within_tolerance".to_string(), [16.0, 50.2, 16.0]),
        ];
        let bad = ungrounded_placements(&placements, ground, 0.5);
        let ids: Vec<&str> = bad.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(ids, vec!["canopy_floating", "shrub_sunk"], "{bad:?}");
        assert_eq!(bad[0].1, 12.0, "must report how far it floats");

        // Unknown ground is not a violation — the law never guesses.
        let bad = ungrounded_placements(&placements, |_, _| None, 0.5);
        assert!(bad.is_empty());
    }

    #[test]
    fn lighting_mirror_matches_shader_contract() {
        // East face gets the full sun; a horizontal north face gets the
        // hemisphere ambient floor (0.38*0 + 0.22*1 = 0.22 at high noon
        // geometry: horizontal faces get the mid band 0.30).
        let east = lit_color(COLOR_GOLD, [1.0, 0.0, 0.0]);
        let down = lit_color(COLOR_GOLD, [0.0, -1.0, 0.0]);
        assert!(east[0] > 0.85, "sunlit east face must be near-full gold");
        // Down-facing: ambient = 0.22 exactly (no sun on a bottom face).
        assert!((down[0] - COLOR_GOLD[0] * 0.22).abs() < 1e-3);
    }
}
