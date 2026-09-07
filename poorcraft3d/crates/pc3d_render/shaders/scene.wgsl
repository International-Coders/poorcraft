// pc3d_render bootstrap shaders (R3DV-001).
//
// Two pipelines share this module:
//   sky   — fullscreen triangle, dawn gradient + sun disc, no vertex buffer
//   scene — indexed vertex-colored mesh ("three banners at dawn")
//
// Colors are linear; the render target is an sRGB format so the GPU encodes
// them for display. NDC convention: x right, y up (matches the documented
// world axes +X east / +Y up until the R3DV-002 camera introduces P3D
// world coordinates).

struct SkyOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_sky(@builtin(vertex_index) vi: u32) -> SkyOut {
    var corners = array<vec2f, 3>(
        vec2f(-1.0, -1.0), vec2f(3.0, -1.0), vec2f(-1.0, 3.0)
    );
    let xy = corners[vi];
    var out: SkyOut;
    out.pos = vec4f(xy, 0.0, 1.0);
    out.uv = xy;
    return out;
}

@fragment
fn fs_sky(in: SkyOut) -> @location(0) vec4f {
    // t = 0 at the bottom (horizon) and 1 at the top (zenith).
    let t = clamp((in.uv.y + 1.0) * 0.5, 0.0, 1.0);
    let zenith = vec3f(0.13, 0.27, 0.42);
    let horizon = vec3f(0.95, 0.70, 0.44);
    var col = mix(horizon, zenith, pow(t, 0.75));

    // Rising sun disc with a soft halo, mirrored by the pixel probes
    // (SUN_CENTER in scene.rs).
    let sun = vec2f(0.62, 0.55);
    let d = distance(in.uv, sun);
    col = mix(vec3f(1.0, 0.93, 0.80), col, smoothstep(0.055, 0.16, d));

    return vec4f(col, 1.0);
}

struct SceneIn {
    @location(0) pos: vec3f,
    @location(1) color: vec4f,
};

struct SceneOut {
    @builtin(position) pos: vec4f,
    @location(0) color: vec4f,
};

@vertex
fn vs_scene(v: SceneIn) -> SceneOut {
    var out: SceneOut;
    out.pos = vec4f(v.pos, 1.0);
    out.color = v.color;
    return out;
}

@fragment
fn fs_scene(in: SceneOut) -> @location(0) vec4f {
    return in.color;
}
