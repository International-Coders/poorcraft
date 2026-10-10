struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) @interpolate(flat) tex_index: u32,
    @location(3) ao: f32,
    // R=bits 0..3, sky=4..7, G=8..11, B=12..15
    @location(4) @interpolate(flat) light: u32,
    @location(5) world_pos: vec3<f32>,
    @location(6) @interpolate(flat) atmosphere: f32,
};

struct Uniforms {
    view_proj: mat4x4<f32>,
    // xyz = camera position, w = day factor [0..1] scaling sky light
    cam_pos_day: vec4<f32>,
    // rgb = fog color, w = fog end distance in blocks
    fog: vec4<f32>,
    // x = elapsed seconds (foliage wind), yzw spare
    time_sway: vec4<f32>,
    // rgb = color-grade tint multiplier, w = saturation (1 = unchanged)
    grade: vec4<f32>,
    // xyz = direction from the camera toward the moving sun, w spare
    sun: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var t_diffuse: texture_2d_array<f32>;

// Section E (connected textures): the CTM strip atlas. Vertices whose
// tex_index is >= 4096 carry a CTM marker instead of an atlas layer; their
// tex_coord already points at the right 16x16 tile of this texture.
@group(1) @binding(2)
var t_ctm: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;
// Packed material maps: tangent-space normal in RGB, material ambient
// occlusion in alpha. One lookup serves both channels.
@group(1) @binding(3)
var t_normal: texture_2d_array<f32>;
@group(1) @binding(4)
var t_ctm_normal: texture_2d<f32>;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) tex_index: u32,
    @location(4) ao: f32,
    @location(5) light: u32,
    @location(6) sway: f32,
) -> VertexOutput {
    var pos = position;
    if (sway > 0.0) {
        // Foliage wind: two world-position-phased sines so the canopy
        // ripples instead of moving as one rigid sheet. Max combined
        // horizontal offset ~0.08 blocks — inside the 0.1 sway margin the
        // column frustum cull already reserves.
        let t = uniforms.time_sway.x;
        let phase = position.x * 0.7 + position.y * 0.3 + position.z * 0.9;
        let wave = sin(t * 1.6 + phase) * 0.5 + sin(t * 2.7 + phase * 1.7) * 0.5;
        pos.x += wave * 0.055 * sway;
        pos.z += wave * 0.045 * sway;
        pos.y += abs(wave) * 0.02 * sway;
    }
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(pos, 1.0);
    out.normal = normal;
    out.tex_coord = tex_coord;
    out.tex_index = tex_index;
    out.ao = ao;
    out.light = light;
    out.world_pos = pos;
    out.atmosphere = select(0.0, 1.0, sway < 0.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // textureSample requires uniform control flow, so the CTM branch uses
    // textureSampleBias (bias 0 = the same automatic LOD).
    var color: vec4<f32>;
    var packed_normal: vec3<f32>;
    var material_ao: f32;
    if (in.tex_index >= 4096u) {
        color = textureSampleBias(t_ctm, s_diffuse, in.tex_coord, 0.0);
        let material = textureSampleBias(t_ctm_normal, s_diffuse, in.tex_coord, 0.0);
        packed_normal = material.rgb;
        material_ao = material.a;
    } else {
        color = textureSample(t_diffuse, s_diffuse, in.tex_coord, in.tex_index);
        let material = textureSample(t_normal, s_diffuse, in.tex_coord, in.tex_index);
        packed_normal = material.rgb;
        material_ao = material.a;
    }
    // alpha cutout (leaves, glass panes, crack decals). Water (a ~0.67) and
    // ice (~0.78) sit above the threshold and render solid/blended.
    if (color.a < 0.5) { discard; }
    // Celestial bodies are unreachable background art. They retain depth
    // occlusion behind terrain/clouds but must not be erased by the
    // performance fog that hides unloaded distant terrain.
    if (in.atmosphere > 0.5) {
        return color;
    }
    let sky = f32((in.light >> 4u) & 15u) / 15.0;
    var block_light = vec3<f32>(
        f32(in.light & 15u),
        f32((in.light >> 8u) & 15u),
        f32((in.light >> 12u) & 15u)
    ) / 15.0;
    let day = uniforms.cam_pos_day.w;
    // Warm firelight breathes subtly; cool crystal/radiation sources stay
    // steady. The world-position phase avoids every hearth pulsing together.
    let warmth = max(block_light.r - block_light.b, 0.0);
    let fire_wave = 0.975 + 0.025 * sin(uniforms.time_sway.x * 7.0
        + in.world_pos.x * 1.7 + in.world_pos.z * 2.3);
    block_light *= mix(1.0, fire_wave, warmth);
    // Sky is neutral; per-channel max lets nearby colored sources tint dark
    // rooms without coloring sunlit terrain. Never render fully black.
    var brightness = max(vec3<f32>(sky * day), block_light * 0.92);
    brightness = max(brightness, vec3<f32>(0.045));
    // Geometry AO owns corners/adjacent blocks; material AO adds only the
    // small-scale cavities authored into mortar, bark, soil and ore veins.
    // A 0.62 floor keeps even the deepest texel readable.
    let geometric_ao = in.ao * 0.8 + 0.2;
    let micro_ao = mix(0.62, 1.0, material_ao);
    let ambient = geometric_ao * micro_ao;

    // Reconstruct a stable local frame from the geometric face normal and
    // rotate the sampled tangent-space direction into world space. The
    // resulting directional term creates tiny edge/groove shadows from a
    // colorful map lookup, while geometry, collision, and RT remain unchanged.
    let face_n = normalize(in.normal);
    var tangent = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), face_n));
    if (abs(face_n.y) > 0.99) {
        tangent = vec3<f32>(1.0, 0.0, 0.0);
    }
    let bitangent = normalize(cross(face_n, tangent));
    let local_n = normalize(packed_normal * 2.0 - vec3<f32>(1.0));
    let relief_n = normalize(tangent * local_n.x + bitangent * local_n.y + face_n * local_n.z);
    let sun_dir = normalize(uniforms.sun.xyz);
    let relief_light = 0.78 + 0.22 * max(dot(relief_n, sun_dir), 0.0);
    var lit = vec4<f32>(color.rgb * brightness * ambient * relief_light, color.a);

    // Distance fog blending toward the sky color.
    let dist = distance(in.world_pos, uniforms.cam_pos_day.xyz);
    let t = smoothstep(uniforms.fog.w * 0.55, uniforms.fog.w, dist);
    let fogged = mix(lit.rgb, uniforms.fog.rgb, t);

    // Per-biome color grade (the "film grade"): saturation pull toward
    // luma, then a per-channel tint multiply. Applied after lighting and
    // fog so the whole visible world shifts together.
    let luma = dot(fogged, vec3<f32>(0.299, 0.587, 0.114));
    let graded = mix(vec3<f32>(luma), fogged, uniforms.grade.w) * uniforms.grade.rgb;
    return vec4<f32>(graded, lit.a);
}
