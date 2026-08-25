// Voxel-DDA path tracer: primary rays through a 3D block texture, soft sun
// shadows (jittered), one-bounce GI sampling sky + emissive torches, fog.

struct Camera {
    pos: vec4<f32>,      // xyz = eye, w = aspect
    look: vec4<f32>,     // xyz = forward, w = fov tan
    right: vec4<f32>,    // xyz = camera right, w = frame index
    up: vec4<f32>,       // xyz = camera up, w unused
    clip_min: vec4<f32>, // xyz = world-space corner of the voxel clip
    sun: vec4<f32>,      // xyz = sun dir, w = day factor
};

@group(0) @binding(0) var<uniform> cam: Camera;
@group(0) @binding(1) var voxels: texture_3d<u32>;
@group(0) @binding(2) var accum: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var<storage, read> palette: array<vec4<f32>, 64>;



fn hash(seed: u32) -> f32 {
    var h = seed;
    h ^= h >> 16u; h *= 0x7feb352du; h ^= h >> 15u; h *= 0x846ca68bu; h ^= h >> 16u;
    return f32(h) / 4294967295.0;
}

fn sample_voxel(p: vec3<i32>) -> u32 {
    let rel = p - vec3<i32>(cam.clip_min.xyz);
    if (rel.x < 0 || rel.y < 0 || rel.z < 0 || rel.x >= 128 || rel.y >= 64 || rel.z >= 128) {
        return 0u;
    }
    return textureLoad(voxels, rel, 0).r;
}

// DDA raycast; returns hit block id and distance.
fn dda(origin: vec3<f32>, dir: vec3<f32>, max_dist: f32) -> vec2<f32> {
    var pos = origin;
    let istep = sign(dir);
    let delta = 1.0 / max(abs(dir), vec3<f32>(1e-5));
    var t_max = (floor(pos) + max(istep, vec3<f32>(0.0)) - pos) / max(abs(dir), vec3<f32>(1e-5));
    var dist = 0.0;
    var id = 0u;
    while (dist < max_dist) {
        if (t_max.x < t_max.y && t_max.x < t_max.z) {
            dist = t_max.x; t_max.x += delta.x; pos.x += istep.x;
        } else if (t_max.y < t_max.z) {
            dist = t_max.y; t_max.y += delta.y; pos.y += istep.y;
        } else {
            dist = t_max.z; t_max.z += delta.z; pos.z += istep.z;
        }
        id = sample_voxel(vec3<i32>(floor(pos)));
        if (id != 0u) {
            return vec2<f32>(f32(id), dist);
        }
    }
    return vec2<f32>(0.0, max_dist);
}

fn sky_color(dir: vec3<f32>, day: f32) -> vec3<f32> {
    let up = clamp(dir.y, 0.0, 1.0);
    let day_c = vec3<f32>(0.53, 0.81, 0.98);
    let night_c = vec3<f32>(0.05, 0.07, 0.15);
    return mix(night_c, day_c, day) * (0.4 + 0.6 * up);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = vec2<f32>(textureDimensions(accum));
    if (gid.x >= u32(dims.x) || gid.y >= u32(dims.y)) { return; }
    let frame = u32(cam.right.w);
    let seed = gid.x * 1973u + gid.y * 9277u + frame * 26699u;

    // 2x2 supersampled camera ray (accumulation-free portable mode)
    var color = vec3<f32>(0.0);
    for (var s = 0u; s < 4u; s = s + 1u) {
        let ox = f32(s % 2u) * 0.5;
        let oy = f32(s / 2u) * 0.5;
        let uv = (vec2<f32>(gid.xy) + vec2<f32>(ox + hash(seed + s * 31u) * 0.5, oy + hash(seed ^ (0x9e37u + s * 7u)) * 0.5)) / dims;
        let ndc = uv * 2.0 - vec2<f32>(1.0);
        let plane = cam.right.xyz * ndc.x * cam.pos.w + cam.up.xyz * ndc.y;
        let dir = normalize(cam.look.xyz + plane);
        color = color + shade_ray(dir, seed + s * 101u);
    }
    color = color / 4.0;
    textureStore(accum, gid.xy, vec4<f32>(color, 1.0));
}

fn shade_ray(dir: vec3<f32>, seed: u32) -> vec3<f32> {
    let hit = dda(cam.pos.xyz, dir, 300.0);
    var color = vec3<f32>(0.0);
    let id = u32(hit.x);
    let dist = hit.y;
    if (id == 0u) {
        color = sky_color(dir, cam.sun.w);
    } else {
        let albedo = palette[min(id, 63u)].rgb;
        // torch/lantern blocks emit
        if (id == 12u || id == 13u) {
            color = vec3<f32>(1.0, 0.75, 0.35) * 2.0;
        } else {
            // soft sun shadow: jitter the sun direction
            let j1 = hash(seed ^ 0x1234u) - 0.5;
            let j2 = hash(seed ^ 0x5678u) - 0.5;
            let sun_dir = normalize(cam.sun.xyz + vec3<f32>(j1, 0.0, j2) * 0.08);
            let shadow_hit = dda(cam.pos.xyz + dir * (dist - 0.05), sun_dir, 200.0);
            let sun_vis = select(0.0, 1.0, u32(shadow_hit.x) == 0u);
            let direct = max(cam.sun.w, 0.05) * sun_vis;

            // one-bounce GI: cosine ray samples sky or bounce
            let a = hash(seed ^ 0xaaaau) * 6.28318;
            let r = sqrt(hash(seed ^ 0xbbbbu));
            let bounce_dir = normalize(vec3<f32>(cos(a) * r, sqrt(1.0 - r * r), sin(a) * r));
            let bounce_hit = dda(cam.pos.xyz + dir * (dist - 0.05), bounce_dir, 60.0);
            var indirect = sky_color(bounce_dir, cam.sun.w) * 0.5;
            if (u32(bounce_hit.x) != 0u) {
                let bid = u32(bounce_hit.x);
                if (bid == 12u || bid == 13u) {
                    indirect = vec3<f32>(1.0, 0.75, 0.35) * (1.5 / max(bounce_hit.y, 1.0));
                } else {
                    indirect *= palette[min(bid, 63u)].rgb * 0.4;
                }
            }
            let lighting = vec3<f32>(direct) + indirect;
            // distance fog toward sky
            let fog = smoothstep(120.0, 280.0, dist);
            color = mix(albedo * lighting, sky_color(dir, cam.sun.w), fog);
        }
    }
    return color;
}
