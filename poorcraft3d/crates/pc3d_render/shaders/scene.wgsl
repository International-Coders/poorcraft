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

// --- Atmosphere (NWR-006) --------------------------------------------------
// Shared by mesh/cutout/water; every term is identity at its zero value so
// the legacy (pre-NWR-006) look is exactly reproducible.

struct Env {
    light_view_proj: mat4x4f,
    fog_color: vec4f,
    // x fog density, y shadow on, z shadow texel (m), w detail strength
    params1: vec4f,
    // x glint on, y shadow bias, z detail scale (uv/m), w shadow res (px)
    params2: vec4f,
};

@group(0) @binding(3) var<uniform> env: Env;
@group(0) @binding(4) var shadow_sampler: sampler_comparison;
@group(0) @binding(5) var shadow_tex: texture_depth_2d;

// 3x3 PCF over the sun's depth map; 1.0 = fully lit. Outside the ortho
// box the world is simply lit (no shadow data there).
fn shadow_factor(wp: vec3f, sun_cos: f32) -> f32 {
    if env.params1.y < 0.5 { return 1.0; }
    let p = env.light_view_proj * vec4f(wp, 1.0);
    // NDC +y maps to attachment row 0 while texture v=0 also samples row
    // 0 — v must flip, or the lookup reads the map upside-down (the
    // everything-shadowed run proved it the hard way).
    let ndc = p.xy / p.w;
    let uv = vec2f(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || p.w <= 0.0 {
        return 1.0;
    }
    // Constant + SLOPE-scaled bias: a surface grazing the light changes
    // depth by ~texel/NdotL per shadow tap, which a constant alone can't
    // cover (the pillar's lit wall broke out in acne until this landed).
    let bias = env.params2.y
        + (env.params1.z * 1.8 / max(sun_cos, 0.1)) / 759.0;
    let d = p.z / p.w - bias;
    let tr = max(env.params2.w, 1.0);
    var acc = 0.0;
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            acc += textureSampleCompare(
                shadow_tex, shadow_sampler,
                uv + vec2f(f32(dx), f32(dy)) / tr, d,
            );
        }
    }
    return acc / 9.0;
}

@group(0) @binding(1) var detail_sampler: sampler;
@group(0) @binding(2) var detail_texture: texture_2d<f32>;

// Material-detail weights from the vertex albedo alone — the WGSL mirror
// of atmosphere::material_weights: green excess -> grass, warm tan ->
// sand, bright -> snow, low saturation -> rock.
fn material_weights(a: vec3f) -> vec4f {
    let mx = max(a.r, max(a.g, a.b));
    let mn = min(a.r, min(a.g, a.b));
    let sat = mx - mn;
    let val = (a.r + a.g + a.b) / 3.0;
    let w_grass = max(a.g - max(a.r, a.b), 0.0);
    let w_rock = max(1.0 - sat * 3.0, 0.0);
    let w_sand = max(min(a.r, a.g) - a.b, 0.0);
    let w_snow = max((val - 0.70) * 6.0, 0.0);
    let sum = w_grass + w_rock + w_sand + w_snow;
    if sum < 1e-5 { return vec4f(0.0, 1.0, 0.0, 0.0); }
    return vec4f(w_grass, w_rock, w_sand, w_snow) / sum;
}

// One atlas tile sampled with fract-wrap + an inset against bleeding.
fn atlas_tile(uvw: vec2f, tile: u32) -> f32 {
    var u = fract(uvw.x);
    var v = fract(uvw.y);
    u = clamp(u, 0.02, 0.98);
    v = clamp(v, 0.02, 0.98);
    return textureSample(
        detail_texture, detail_sampler,
        vec2f((f32(tile) + u) / 4.0, v),
    ).r;
}

// Distance fog (identity at density 0) — the WGSL mirror of
// atmosphere::fog_factor / apply_fog, mixed in LINEAR space.
fn apply_fog(col: vec3f, world: vec3f) -> vec3f {
    let t = distance(world, globals.cam_pos.xyz) * env.params1.x;
    let f = 1.0 - exp(-t * t);
    return mix(col, env.fog_color.rgb, f);
}

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
    @location(2) world: vec3f,
};

@vertex
fn vs_mesh(v: MeshIn) -> MeshOut {
    var out: MeshOut;
    out.pos = globals.view_proj * vec4f(v.pos, 1.0);
    out.normal = v.normal;
    out.color = v.color;
    out.world = v.pos;
    return out;
}

@fragment
fn fs_mesh(in: MeshOut) -> @location(0) vec4f {
    let n = normalize(in.normal);
    var sun = max(dot(n, globals.sun_dir.xyz), 0.0);
    // Sun shadows (NWR-006): normal-offset the lookup, keep a soft 35%
    // floor in shadow — a stylized penumbra, never black.
    if sun > 0.0 {
        let wp = in.world + n * env.params1.z * 2.0;
        sun = sun * mix(0.35, 1.0, shadow_factor(wp, sun));
    }
    // Hemisphere ambient (R3DV-010): sky above, ground below.
    var light = 0.38 * (0.5 + 0.5 * n.y) + 0.22 * (0.5 - 0.5 * n.y) + 1.05 * sun;
    var albedo = in.color;
    // Detail texture (high tier): subtle deterministic surface variation
    // from world-space UVs; globals.tan_aspect.w is the detail flag.
    // (NWR-006: the noise now reads ATLAS TILE 0 — one texture serves
    // both the legacy flag and the new material blend.)
    if globals.tan_aspect.w > 0.5 {
        let d = atlas_tile(in.pos.xz * 0.25, 0u);
        albedo = albedo * (0.88 + 0.24 * d);
    }
    // Material detail (NWR-006): the atlas tiles blend by the albedo's
    // own material weights — grass/rock/sand/snow grain without any
    // vertex format change.
    let strength = env.params1.w;
    if strength > 0.001 {
        let w = material_weights(albedo);
        let uvw = in.world.xz * env.params2.z;
        let d = w.x * atlas_tile(uvw, 0u)
            + w.y * atlas_tile(uvw, 1u)
            + w.z * atlas_tile(uvw, 2u)
            + w.w * atlas_tile(uvw, 3u);
        albedo = albedo * (1.0 - strength + strength * (0.55 + 0.9 * d));
    }
    var col = albedo * min(light, 1.0);
    col = apply_fog(col, in.world);
    return vec4f(col, 1.0);
}

// --- Shadow depth pass (NWR-006): depth-only from the sun -----------

struct ShadowOut {
    @builtin(position) pos: vec4f,
};

@vertex
fn vs_shadow(v: MeshIn) -> ShadowOut {
    var out: ShadowOut;
    out.pos = env.light_view_proj * vec4f(v.pos, 1.0);
    return out;
}

// Depth-only entry for the cutout vertex layout (solid foliage shadow).
@vertex
fn vs_shadow_cutout(@location(0) pos: vec3f) -> ShadowOut {
    var out: ShadowOut;
    out.pos = env.light_view_proj * vec4f(pos, 1.0);
    return out;
}

// --- Cutout foliage (NWR-006): mask-tested lit quads ----------------

struct CutoutIn {
    @location(0) pos: vec3f,
    @location(1) normal: vec3f,
    @location(2) color: vec3f,
    @location(3) uv: vec2f,
};

struct CutoutOut {
    @builtin(position) pos: vec4f,
    @location(0) normal: vec3f,
    @location(1) color: vec3f,
    @location(2) world: vec3f,
    @location(3) uv: vec2f,
};

// The cutout mask rides its OWN bind group (group 1) so the shared
// module keeps unique bindings.
@group(1) @binding(0) var mask_sampler: sampler;
@group(1) @binding(1) var mask_texture: texture_2d<f32>;

@vertex
fn vs_cutout(v: CutoutIn) -> CutoutOut {
    var out: CutoutOut;
    out.pos = globals.view_proj * vec4f(v.pos, 1.0);
    out.normal = v.normal;
    out.color = v.color;
    out.world = v.pos;
    out.uv = v.uv;
    return out;
}

@fragment
fn fs_cutout(in: CutoutOut) -> @location(0) vec4f {
    // Alpha cutout: hard discard at the mask threshold (no sorting, no
    // alpha blend — Deck-cheap and order-independent).
    let m = textureSample(mask_texture, mask_sampler, in.uv).a;
    if m < 0.5 { discard; }
    let n = normalize(in.normal);
    var sun = max(dot(n, globals.sun_dir.xyz), 0.0);
    if sun > 0.0 {
        let wp = in.world + n * env.params1.z * 2.0;
        sun = sun * mix(0.35, 1.0, shadow_factor(wp, sun));
    }
    var light = 0.38 * (0.5 + 0.5 * n.y) + 0.22 * (0.5 - 0.5 * n.y) + 1.05 * sun;
    var col = in.color * min(light, 1.0);
    col = apply_fog(col, in.world);
    return vec4f(col, 1.0);
}

// --- Instanced wilderness (NWR-007) ----------------------------------------
// One draw per (kind, LOD): the mesh vertex + a per-instance transform
// (slot 1). Wind sways flexible kinds using the shared time uniform
// (frozen for proofs).

struct InstIn {
    @location(0) pos: vec3f,
    @location(1) normal: vec3f,
    @location(2) color: vec3f,
    @location(3) pos_scale: vec4f,
    @location(4) params: vec4f, // x rot-y, y wind, z tint, w unused
};

/// Instance transform + wind, shared by every instanced entry.
fn instance_world(v_pos: vec3f, i_pos_scale: vec4f, i_params: vec4f) -> vec3f {
    let c = cos(i_params.x);
    let s = sin(i_params.x);
    let p = v_pos * i_pos_scale.w;
    var world = vec3f(
        i_pos_scale.x + p.x * c + p.z * s,
        i_pos_scale.y + p.y,
        i_pos_scale.z - p.x * s + p.z * c,
    );
    // Wind: sway grows with height; the phase varies per plant.
    let wind = i_params.y;
    if wind > 0.001 {
        let t = globals.tan_aspect.z;
        let ph = t * 1.6 + dot(i_pos_scale.xz, vec2f(0.9, 1.3));
        let h = clamp(p.y / 2.0, 0.0, 1.0);
        let sway = sin(ph) * wind * 0.12 * h * h;
        world.x += sway;
        world.z += 0.6 * sway;
    }
    return world;
}

@vertex
fn vs_inst(v: InstIn) -> MeshOut {
    var out: MeshOut;
    out.pos = globals.view_proj * vec4f(instance_world(v.pos, v.pos_scale, v.params), 1.0);
    let c = cos(v.params.x);
    let s = sin(v.params.x);
    out.normal = normalize(vec3f(
        v.normal.x * c + v.normal.z * s,
        v.normal.y,
        -v.normal.x * s + v.normal.z * c,
    ));
    out.color = v.color * v.params.z;
    out.world = instance_world(v.pos, v.pos_scale, v.params);
    return out;
}

struct InstShadowOut {
    @builtin(position) pos: vec4f,
};

@vertex
fn vs_inst_shadow(v: InstIn) -> InstShadowOut {
    var out: InstShadowOut;
    out.pos = env.light_view_proj * vec4f(instance_world(v.pos, v.pos_scale, v.params), 1.0);
    return out;
}

struct CutoutInstIn {
    @location(0) pos: vec3f,
    @location(1) normal: vec3f,
    @location(2) color: vec3f,
    @location(3) uv: vec2f,
    @location(4) pos_scale: vec4f,
    @location(5) params: vec4f,
};

@vertex
fn vs_inst_cutout(v: CutoutInstIn) -> CutoutOut {
    var out: CutoutOut;
    out.pos = globals.view_proj * vec4f(instance_world(v.pos, v.pos_scale, v.params), 1.0);
    out.normal = v.normal;
    out.color = v.color * v.params.z;
    out.world = instance_world(v.pos, v.pos_scale, v.params);
    out.uv = v.uv;
    return out;
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

// --- Water (R3DV-007): transparent strip, current from the flow record ---

struct WaterIn {
    @location(0) pos: vec3f,
    @location(1) dir: vec2f,
    @location(2) speed: f32,
    @location(3) alpha: f32,
};

struct WaterOut {
    @builtin(position) pos: vec4f,
    @location(0) color: vec4f,
    @location(1) world: vec3f,
};

@vertex
fn vs_water(v: WaterIn) -> WaterOut {
    var out: WaterOut;
    out.pos = globals.view_proj * vec4f(v.pos, 1.0);
    // The current's phase advances along the flow direction; time moves the
    // wave downstream at the record's speed class.
    let phase = dot(v.pos.xz, v.dir);
    let stripe = 0.5 + 0.5 * sin(phase * 0.8 - globals.tan_aspect.z * v.speed * 3.0);
    let base = vec3f(0.24, 0.52, 0.85);
    out.color = vec4f(base * (0.7 + 0.3 * stripe), v.alpha);
    out.world = v.pos;
    return out;
}

@fragment
fn fs_water(in: WaterOut) -> @location(0) vec4f {
    var col = in.color;
    // Restrained sun glint (NWR-006): one specular lobe off the flat
    // surface toward the viewer; capped so it stays a highlight.
    if env.params2.x > 0.5 {
        let v = normalize(globals.cam_pos.xyz - in.world);
        let r = reflect(-globals.sun_dir.xyz, vec3f(0.0, 1.0, 0.0));
        let g = pow(max(dot(r, v), 0.0), 48.0);
        col = vec4f(col.rgb + vec3f(0.90, 0.85, 0.70) * min(g, 0.8), col.a);
    }
    return vec4f(apply_fog(col.rgb, in.world), col.a);
}
