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

pub const AMBIENT: f32 = 0.35;
pub const SUN_STRENGTH: f32 = 1.1;

/// Mirror of the WGSL mesh lighting so pixel probes share one source of truth.
pub fn lit_color(albedo: [f32; 3], normal: [f32; 3]) -> [f32; 3] {
    // d = dot(normal, sun_dir) — the albedo is applied after the light term.
    let d = normal
        .iter()
        .zip(SUN_DIR)
        .map(|(n, s)| n * s)
        .sum::<f32>();
    let light = (AMBIENT + SUN_STRENGTH * d.max(0.0)).min(1.0);
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
    ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),  // south +Z
    ([0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]), // north -Z
    ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),  // east +X
    ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]), // west -X
    ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),  // top +Y
    ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]), // bottom -Y
];

/// One axis-aligned box: six faces, each with its own albedo.
fn box_faces(center: [f32; 3], half: f32, colors: &[[f32; 3]; 6]) -> ([SceneVertex; 24], [u16; 36]) {
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
        SceneVertex { pos: [-20.0, 0.0, 20.0], normal: [0.0, 1.0, 0.0], color: COLOR_STONE },
        SceneVertex { pos: [20.0, 0.0, 20.0], normal: [0.0, 1.0, 0.0], color: COLOR_STONE },
        SceneVertex { pos: [20.0, 0.0, -20.0], normal: [0.0, 1.0, 0.0], color: COLOR_STONE },
        SceneVertex { pos: [-20.0, 0.0, -20.0], normal: [0.0, 1.0, 0.0], color: COLOR_STONE },
    ];
    push(&gv, &[0, 1, 2, 0, 2, 3]);

    // Near stone at (0, 1, -4): six distinct face colors, south face crimson,
    // east face gold. The eye height (1.7 m) puts its south face at screen
    // center from pose A.
    let (cv, ci) = box_faces(
        [0.0, 1.0, -4.0],
        1.0,
        &[
            COLOR_CRIMSON, // +Z south
            COLOR_JADE,    // -Z north
            COLOR_GOLD,    // +X east
            COLOR_PLUM,    // -X west
            COLOR_PALE,    // +Y top
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
        self.distinct_colors >= min_distinct
            && self.opaque
            && self.probes.iter().all(|(_, ok)| *ok)
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
    [
        to_srgb(c[0]),
        to_srgb(c[1]),
        to_srgb(c[2]),
        1.0,
    ]
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
    let ground_ndc = if pose == pose_b() { (0.0, -0.85) } else { (0.0, -0.6) };
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
            let e1 = [b.pos[0] - a.pos[0], b.pos[1] - a.pos[1], b.pos[2] - a.pos[2]];
            let e2 = [c.pos[0] - a.pos[0], c.pos[1] - a.pos[1], c.pos[2] - a.pos[2]];
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
        let ca = a.iter().find(|p| p.name == "center_is_crimson_south_face").unwrap();
        let cb = b.iter().find(|p| p.name == "center_is_gold_east_face").unwrap();
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

    #[test]
    fn lighting_mirror_matches_shader_contract() {
        // East face gets the full sun; north face gets ambient only.
        let east = lit_color(COLOR_GOLD, [1.0, 0.0, 0.0]);
        let north = lit_color(COLOR_GOLD, [0.0, 0.0, -1.0]);
        assert!(east[0] > 0.85, "sunlit east face must be near-full gold");
        assert!((north[0] - COLOR_GOLD[0] * AMBIENT).abs() < 1e-3);
    }
}
