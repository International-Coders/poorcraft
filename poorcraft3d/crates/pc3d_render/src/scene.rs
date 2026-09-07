//! The R3DV-001 proof scene: "three banners at dawn".
//!
//! Original POORCRAFT placeholder art (no external game expression): a dawn
//! gradient sky with a rising sun disc behind three banner pennants on poles
//! above a pale stone ground strip. The scene proves a nonuniform GPU image —
//! per-pixel gradient + sun in the fragment shader, plus an indexed,
//! vertex-colored triangle mesh drawn by a second pipeline.
//!
//! Geometry is defined in normalized device coordinates (the R3DV-002 camera
//! will introduce P3D world coordinates and a real projection). The same
//! constants drive both the mesh and the pixel probes in [`verify_frame_rgba`],
//! so the semantic screenshot assertions cannot drift from the scene.

/// A clip-space vertex: position in NDC (z = 0.5, no camera yet) + RGBA color
/// in linear space (the sRGB target format encodes it on write).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneVertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
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
            format: wgpu::VertexFormat::Float32x4,
            offset: 12,
            shader_location: 1,
        },
    ],
};

// Palette — linear RGBA.
pub const COLOR_CRIMSON: [f32; 4] = [0.78, 0.16, 0.18, 1.0];
pub const COLOR_GOLD: [f32; 4] = [0.92, 0.74, 0.20, 1.0];
pub const COLOR_JADE: [f32; 4] = [0.16, 0.62, 0.42, 1.0];
pub const COLOR_CHARCOAL: [f32; 4] = [0.16, 0.15, 0.14, 1.0];
pub const COLOR_STONE: [f32; 4] = [0.72, 0.70, 0.62, 1.0];

// Sun disc position/radius in NDC, mirrored by the sky fragment shader.
pub const SUN_CENTER: (f32, f32) = (0.62, 0.55);

/// One axis-aligned quad as two CCW triangles (viewed from +Z looking down
/// -Z, which is the front face for this scene).
fn quad(x0: f32, y0: f32, x1: f32, y1: f32, color: [f32; 4]) -> ([SceneVertex; 4], [u16; 6]) {
    let v = |x: f32, y: f32| SceneVertex {
        pos: [x, y, 0.5],
        color,
    };
    let verts = [v(x0, y0), v(x1, y0), v(x1, y1), v(x0, y1)];
    // CCW when y0 < y1: (x0,y0) -> (x1,y0) -> (x1,y1), then (x0,y0) -> (x1,y1) -> (x0,y1)
    let idx = [0, 1, 2, 0, 2, 3];
    (verts, idx)
}

/// Builds the proof-scene mesh: ground strip, three poles, three flags.
pub fn build_scene() -> (Vec<SceneVertex>, Vec<u16>) {
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    let mut push = |v: [SceneVertex; 4], i: [u16; 6]| {
        let base = verts.len() as u16;
        verts.extend(v);
        idx.extend(i.map(|k| k + base));
    };

    // Ground strip across the bottom of the frame.
    let (v, i) = quad(-1.0, -1.0, 1.0, -0.62, COLOR_STONE);
    push(v, i);

    // Poles rise from the ground to y = 0.12; flags hang right of each pole.
    let pole = |cx: f32| quad(cx - 0.015, -0.62, cx + 0.015, 0.12, COLOR_CHARCOAL);
    for cx in [-0.55, 0.0, 0.55] {
        let (v, i) = pole(cx);
        push(v, i);
    }

    // Flags at three different heights for a readable silhouette.
    let (v, i) = quad(-0.535, -0.22, -0.315, 0.12, COLOR_CRIMSON);
    push(v, i);
    let (v, i) = quad(0.015, -0.32, 0.235, 0.12, COLOR_GOLD);
    push(v, i);
    let (v, i) = quad(0.565, -0.16, 0.785, 0.12, COLOR_JADE);
    push(v, i);

    (verts, idx)
}

// ---------------------------------------------------------------------------
// Semantic pixel verification — pure function over an RGBA8 framebuffer, used
// by the offscreen GPU tests AND by the live windowed capture. This is the
// "pixel_or_semantic_visual_assertion" evidence for the R3DV-001 gate: the
// screenshot must be nonuniform and must actually contain the scene.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelReport {
    pub width: u32,
    pub height: u32,
    /// Unique RGB colors among sampled pixels — a flat clear would report 1.
    pub distinct_colors: usize,
    /// Sky brightness increases monotonically down an unobstructed column.
    pub gradient_monotonic: bool,
    /// The gradient column spans more than one color and a wide range.
    pub sky_nonuniform: bool,
    pub crimson_flag: bool,
    pub gold_flag: bool,
    pub jade_flag: bool,
    pub pole: bool,
    pub ground: bool,
    /// Sun disc is present (brighter than same-height sky control point).
    pub sun: bool,
    /// Every sampled pixel is fully opaque.
    pub opaque: bool,
}

impl PixelReport {
    pub fn passes(&self) -> bool {
        // A flat clear reports 1; any real gradient yields hundreds even at
        // small proof resolutions (251 at 256x192, thousands at 720p).
        self.distinct_colors >= 100
            && self.gradient_monotonic
            && self.sky_nonuniform
            && self.crimson_flag
            && self.gold_flag
            && self.jade_flag
            && self.pole
            && self.ground
            && self.sun
            && self.opaque
    }
}

fn to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// NDC (x right, y up, -1..1) to pixel coordinates (row 0 = top).
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

fn near(actual: [f32; 4], expected: [f32; 4], tol: f32) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(a, e)| (a - e).abs() <= tol)
}

/// Verifies an RGBA8 framebuffer against the proof scene semantics.
/// The bytes must be in RGBA order (capturers swizzle BGRA first).
pub fn verify_frame_rgba(rgba: &[u8], width: u32, height: u32) -> PixelReport {
    assert_eq!(
        rgba.len(),
        (width * height * 4) as usize,
        "framebuffer size mismatch"
    );

    // Distinct colors + opacity over a fine sample grid (stride 2: enough to
    // see thousands of gradient colors, cheap enough for tests).
    let mut distinct = std::collections::HashSet::new();
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

    // Gradient along an unobstructed sky column at x = -0.85 (left of all
    // flags): brightness must be non-decreasing toward the horizon and span
    // a wide, multi-color range.
    let mut brightness = Vec::new();
    let mut y = 0.95f32;
    while y >= -0.55 {
        let p = pixel_at(rgba, width, height, -0.85, y);
        brightness.push(p[0] + p[1] + p[2]);
        y -= 0.05;
    }
    let gradient_monotonic = brightness.windows(2).all(|ab| ab[1] >= ab[0] - 0.02);
    // Dawn physics: the sky is dark at the zenith and bright at the horizon,
    // so brightness must RISE along the downward-sampled column.
    let sky_nonuniform = brightness[brightness.len() - 1] > brightness[0] + 0.5
        && distinct_brightness_count(&brightness) >= 3;

    let tol = 0.09;
    let crimson_flag = near(pixel_at(rgba, width, height, -0.43, -0.05), to_srgb4(COLOR_CRIMSON), tol);
    let gold_flag = near(pixel_at(rgba, width, height, 0.12, -0.10), to_srgb4(COLOR_GOLD), tol);
    let jade_flag = near(pixel_at(rgba, width, height, 0.67, 0.0), to_srgb4(COLOR_JADE), tol);
    let pole = near(pixel_at(rgba, width, height, -0.55, -0.5), to_srgb4(COLOR_CHARCOAL), tol);
    let ground = near(pixel_at(rgba, width, height, 0.0, -0.8), to_srgb4(COLOR_STONE), tol);

    let sun = {
        let s = pixel_at(rgba, width, height, SUN_CENTER.0, SUN_CENTER.1);
        let c = pixel_at(rgba, width, height, -0.30, SUN_CENTER.1);
        (s[0] + s[1] + s[2]) - (c[0] + c[1] + c[2]) > 0.25
    };

    PixelReport {
        width,
        height,
        distinct_colors: distinct.len(),
        gradient_monotonic,
        sky_nonuniform,
        crimson_flag,
        gold_flag,
        jade_flag,
        pole,
        ground,
        sun,
        opaque,
    }
}

fn distinct_brightness_count(v: &[f32]) -> usize {
    let mut quantized: Vec<u8> = v
        .iter()
        .map(|b| (b * 32.0) as u8) // quantize to tolerate dithering
        .collect();
    quantized.sort_unstable();
    quantized.dedup();
    quantized.len()
}

fn to_srgb4(c: [f32; 4]) -> [f32; 4] {
    [
        to_srgb(c[0]),
        to_srgb(c[1]),
        to_srgb(c[2]),
        c[3],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_mesh_is_valid_indexed_geometry() {
        let (verts, idx) = build_scene();
        // 1 ground + 3 poles + 3 flags = 7 quads = 14 triangles.
        assert_eq!(idx.len(), 14 * 3);
        assert_eq!(verts.len(), 7 * 4);
        assert!(idx.iter().all(|&i| (i as usize) < verts.len()));

        let colors: Vec<[u8; 4]> = verts
            .iter()
            .map(|v| v.color.map(|c| (c * 255.0) as u8))
            .collect();
        let mut unique = colors.clone();
        unique.sort();
        unique.dedup();
        // crimson, gold, jade, charcoal, stone — a nonuniform scene by data.
        assert_eq!(unique.len(), 5);
    }

    #[test]
    fn scene_triangles_are_ccw_for_backface_culling() {
        // History lesson (root project): see-through terrain from wrong
        // winding. This scene uses CullMode::Back + FrontFace::Ccw, so every
        // triangle must wind CCW on the z=0 plane.
        let (verts, idx) = build_scene();
        for tri in idx.chunks(3) {
            let a = verts[tri[0] as usize].pos;
            let b = verts[tri[1] as usize].pos;
            let c = verts[tri[2] as usize].pos;
            let cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            assert!(cross > 0.0, "triangle {tri:?} winds CW (cross {cross})");
        }
    }

    #[test]
    fn flat_framebuffer_fails_verification() {
        // A uniform clear color must NOT pass: this is the "nonuniform" gate.
        let flat = vec![40u8; (64 * 64 * 4) as usize];
        let report = verify_frame_rgba(&flat, 64, 64);
        assert_eq!(report.distinct_colors, 1);
        assert!(!report.passes());
    }

    #[test]
    fn report_requires_every_scene_element() {
        let mut r = PixelReport {
            width: 384,
            height: 288,
            distinct_colors: 1000,
            gradient_monotonic: true,
            sky_nonuniform: true,
            crimson_flag: true,
            gold_flag: true,
            jade_flag: true,
            pole: true,
            ground: true,
            sun: true,
            opaque: true,
        };
        assert!(r.passes());
        // Break each boolean field in turn; the report must fail every time.
        let base = r;
        for name in [
            "gradient_monotonic",
            "sky_nonuniform",
            "crimson_flag",
            "gold_flag",
            "jade_flag",
            "pole",
            "ground",
            "sun",
            "opaque",
        ] {
            r = base;
            match name {
                "gradient_monotonic" => r.gradient_monotonic = false,
                "sky_nonuniform" => r.sky_nonuniform = false,
                "crimson_flag" => r.crimson_flag = false,
                "gold_flag" => r.gold_flag = false,
                "jade_flag" => r.jade_flag = false,
                "pole" => r.pole = false,
                "ground" => r.ground = false,
                "sun" => r.sun = false,
                "opaque" => r.opaque = false,
                _ => unreachable!(),
            }
            assert!(!r.passes(), "{name} alone must fail the report");
        }
    }
}
