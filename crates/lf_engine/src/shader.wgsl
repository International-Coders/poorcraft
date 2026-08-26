struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) @interpolate(flat) tex_index: u32,
    @location(3) ao: f32,
    // sky light in the high nibble, block light in the low nibble
    @location(4) @interpolate(flat) light: u32,
    @location(5) world_pos: vec3<f32>,
};

struct Uniforms {
    view_proj: mat4x4<f32>,
    // xyz = camera position, w = day factor [0..1] scaling sky light
    cam_pos_day: vec4<f32>,
    // rgb = fog color, w = fog end distance in blocks
    fog: vec4<f32>,
    // x = elapsed seconds (foliage wind), yzw spare
    time_sway: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var t_diffuse: texture_2d_array<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) tex_index: u32,
    @location(4) ao: f32,
    @location(5) light: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(position, 1.0);
    out.normal = normal;
    out.tex_coord = tex_coord;
    out.tex_index = tex_index;
    out.ao = ao;
    out.light = light;
    out.world_pos = position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coord, in.tex_index);
    // alpha cutout (leaves, glass panes, crack decals). Water (a ~0.67) and
    // ice (~0.78) sit above the threshold and render solid/blended.
    if (color.a < 0.5) { discard; }
    let sky = f32((in.light >> 4u) & 15u) / 15.0;
    let block_l = f32(in.light & 15u) / 15.0;
    let day = uniforms.cam_pos_day.w;
    // Block light slightly warmer/dimmer than full sky; never fully black.
    var brightness = max(sky * day, block_l * 0.92);
    brightness = max(brightness, 0.045);
    let ambient = in.ao * 0.8 + 0.2;
    var lit = vec4<f32>(color.rgb * brightness * ambient, color.a);

    // Distance fog blending toward the sky color.
    let dist = distance(in.world_pos, uniforms.cam_pos_day.xyz);
    let t = smoothstep(uniforms.fog.w * 0.55, uniforms.fog.w, dist);
    let fogged = mix(lit.rgb, uniforms.fog.rgb, t);
    return vec4<f32>(fogged, lit.a);
}
