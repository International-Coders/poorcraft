// pc3d_render shaders (R3DV-002): one real 3D scene, three pipelines.
//
// World convention: right-handed, meters, +X east / +Y up / +Z south. All
// geometry comes from P3D world coordinates through a view-projection
// matrix; the sky reconstructs per-pixel view directions, so turning the
// camera moves the sun and horizon — there is no 2D fallback path.
//
// Pipelines:
//   sky  — fullscreen triangle; world-ray gradient + sun disc (depth off)
//   mesh — lit indexed geometry: lambert sun + ambient, depth-tested
//   hud  — bitmap-font debug line (alpha-blended texture quad)
//
// Colors are linear albedo/normals; the sRGB target encodes on write.

struct Globals {
    view_proj: mat4x4f,
    cam_pos: vec4f,
    fwd: vec4f,
    right: vec4f,
    up: vec4f,
    sun_dir: vec4f,
    tan_aspect: vec4f, // x = tan(fov_y/2), y = aspect
};

@group(0) @binding(0) var<uniform> globals: Globals;

// --- Sky -------------------------------------------------------------------

struct SkyOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_sky(@builtin(vertex_index) vi: u32) -> SkyOut {
    var corners = array<vec2f, 3>(
        vec2f(-1.0, -1.0), vec2f(3.0, -1.0), vec2f(-1.0, 3.0)
    );
    var out: SkyOut;
    out.pos = vec4f(corners[vi], 0.9999, 1.0);
    out.uv = corners[vi];
    return out;
}

@fragment
fn fs_sky(in: SkyOut) -> @location(0) vec4f {
    // Reconstruct the world-space view ray for this pixel.
    let dir = normalize(
        globals.fwd.xyz
        + in.uv.x * globals.tan_aspect.x * globals.tan_aspect.y * globals.right.xyz
        + in.uv.y * globals.tan_aspect.x * globals.up.xyz
    );

    let t = smoothstep(-0.05, 0.5, dir.y);
    var col = mix(vec3f(0.95, 0.70, 0.44), vec3f(0.13, 0.27, 0.42), t);

    // Dawn sun tied to the actual world sun direction: turning moves it.
    let s = dot(dir, globals.sun_dir.xyz);
    col = mix(col, vec3f(1.0, 0.80, 0.55), 0.55 * smoothstep(0.98, 0.998, s));
    col = mix(col, vec3f(1.0, 0.93, 0.80), smoothstep(0.997, 0.9995, s));

    return vec4f(col, 1.0);
}

// --- Lit world mesh --------------------------------------------------------

struct MeshIn {
    @location(0) pos: vec3f,
    @location(1) normal: vec3f,
    @location(2) color: vec3f,
};

struct MeshOut {
    @builtin(position) pos: vec4f,
    @location(0) normal: vec3f,
    @location(1) color: vec3f,
};

@vertex
fn vs_mesh(v: MeshIn) -> MeshOut {
    var out: MeshOut;
    out.pos = globals.view_proj * vec4f(v.pos, 1.0);
    out.normal = v.normal;
    out.color = v.color;
    return out;
}

@fragment
fn fs_mesh(in: MeshOut) -> @location(0) vec4f {
    let n = normalize(in.normal);
    let sun = max(dot(n, globals.sun_dir.xyz), 0.0);
    let light = min(0.35 + 1.1 * sun, 1.0);
    return vec4f(in.color * light, 1.0);
}

// --- HUD (bitmap-font debug line) ------------------------------------------

struct HudIn {
    @location(0) pos: vec2f,
    @location(1) uv: vec2f,
};

struct HudOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_hud(v: HudIn) -> HudOut {
    var out: HudOut;
    out.pos = vec4f(v.pos, 0.0, 1.0);
    out.uv = v.uv;
    return out;
}

@group(0) @binding(1) var hud_sampler: sampler;
@group(0) @binding(2) var hud_texture: texture_2d<f32>;

@fragment
fn fs_hud(in: HudOut) -> @location(0) vec4f {
    let a = textureSample(hud_texture, hud_sampler, in.uv).a;
    // White text with a dark backing for readability on any sky.
    let text = vec4f(1.0, 1.0, 1.0, a);
    let backing = vec4f(0.05, 0.07, 0.10, 0.55);
    return mix(backing, text, a);
}
