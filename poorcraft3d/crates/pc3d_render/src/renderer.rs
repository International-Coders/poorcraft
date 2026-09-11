//! The windowed/offscreen renderer core: a real 3D pipeline set.
//!
//! One draw path ([`prepare_frame`] + [`encode_frame`]) serves the live
//! window, the live-window screenshot (surface configured with `COPY_SRC`),
//! and the offscreen proof target — so tested pixels and displayed pixels
//! are the same GPU work: depth-tested lit world geometry under a
//! world-direction sun, a ray-reconstructed sky, and a bitmap HUD.

use crate::camera::{Camera, CameraPose};
use crate::gpu::{surface_action, GpuContext, SurfaceAction, SurfaceGuard};
pub use crate::scene::PixelReport;
use crate::scene::{Probe, SceneVertex, VERTEX_LAYOUT};
use std::path::Path;
use std::sync::Arc;
use wgpu::util::DeviceExt;

const SHADER: &str = include_str!("../shaders/scene.wgsl");
const HUD_SCALE: u32 = 2;
const HUD_LINE_CHARS: usize = 44;

/// Uniform state shared by all three pipelines (160 bytes, 16-aligned).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    view_proj: [f32; 16],
    cam_pos: [f32; 4],
    fwd: [f32; 4],
    right: [f32; 4],
    up: [f32; 4],
    sun_dir: [f32; 4],
    tan_aspect: [f32; 4],
}

/// The atmosphere uniform (NWR-006), shared by mesh/cutout/water —
/// 160 bytes, 16-aligned, mirroring `struct Env` in scene.wgsl.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct EnvGpu {
    light_view_proj: [f32; 16],
    fog_color: [f32; 4],
    /// x fog density, y shadow on, z shadow texel (m), w detail strength
    params1: [f32; 4],
    /// x glint on, y shadow bias, z detail scale, w shadow res
    params2: [f32; 4],
    /// x = the SCREEN HEIGHT MAP debug flag (world-height ramp output).
    params3: [f32; 4],
}

/// The instanced wilderness pipelines (NWR-007) — one draw per
/// (kind, LOD) bucket, plus the grass cutout cards.
pub struct FloraPipelines {
    pub inst: wgpu::RenderPipeline,
    pub inst_cutout: wgpu::RenderPipeline,
    pub inst_shadow: wgpu::RenderPipeline,
    /// NPC crowd boxes (NWR-009): unit cube + per-axis instance scale.
    pub inst_box: wgpu::RenderPipeline,
}

struct Pipelines {
    sky: wgpu::RenderPipeline,
    mesh: wgpu::RenderPipeline,
    /// WT-002/003 slice 3: the same lit vertex path on LineList —
    /// wireframe edges + anchor overlays.
    wire: wgpu::RenderPipeline,
    hud: wgpu::RenderPipeline,
    /// The full-RGBA UI layer (GLM UI rework): passthrough fragment over
    /// the hud vertex layout, alpha-blended, drawn after the HUD line.
    ui: wgpu::RenderPipeline,
    water: wgpu::RenderPipeline,
    /// Depth-only sun pass (NWR-006); no fragment stage.
    shadow: wgpu::RenderPipeline,
    /// The same depth-only pass for the cutout vertex layout.
    shadow_cutout: wgpu::RenderPipeline,
    /// Alpha-cutout foliage (NWR-006); mask at group 1.
    cutout: wgpu::RenderPipeline,
    layout_globals: wgpu::BindGroupLayout,
    layout_hud: wgpu::BindGroupLayout,
    /// The cutout mask bind group layout (group 1).
    layout_mask: wgpu::BindGroupLayout,
    flora: FloraPipelines,
}

impl Pipelines {
    fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pc3d_render scene.wgsl"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout_globals = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pc3d globals layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Detail texture (R3DV-010 high tier; NWR-006: the
                // material atlas).
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Atmosphere (NWR-006): env uniform, the comparison
                // sampler, and the sun's depth map.
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let layout_mask = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pc3d cutout mask layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let layout_hud = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pc3d hud layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let pl_globals = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pc3d globals pipe layout"),
            bind_group_layouts: &[&layout_globals],
            push_constant_ranges: &[],
        });
        let pl_hud = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pc3d hud pipe layout"),
            bind_group_layouts: &[&layout_hud],
            push_constant_ranges: &[],
        });

        let depth24 = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: Default::default(),
            bias: Default::default(),
        };
        // Pipelines that must not touch depth still have to DECLARE the
        // pass's depth format (wgpu validates pipeline targets against the
        // render pass attachment).
        let depth_off = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: Default::default(),
            bias: Default::default(),
        };

        let make = |label: &'static str,
                    layout: &wgpu::PipelineLayout,
                    vs: &str,
                    fs: &str,
                    vertex_buffers: &[wgpu::VertexBufferLayout<'static>],
                    cull: Option<wgpu::Face>,
                    depth: Option<wgpu::DepthStencilState>,
                    blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some(vs),
                    buffers: vertex_buffers,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(fs),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: cull,
                    front_face: wgpu::FrontFace::Ccw,
                    ..Default::default()
                },
                depth_stencil: depth,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };

        let water_layout = crate::water::WATER_VERTEX_LAYOUT;
        let water = make(
            "pc3d water pipeline",
            &pl_globals,
            "vs_water",
            "fs_water",
            std::slice::from_ref(&water_layout),
            None,
            // Depth READ only: terrain banks occlude water, water never
            // occludes itself (transparent pass ordering).
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );

        let hud_layout = wgpu::VertexBufferLayout {
            array_stride: 16,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 8,
                    shader_location: 1,
                },
            ],
        };

        let pl_cutout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pc3d cutout pipe layout"),
            bind_group_layouts: &[&layout_globals, &layout_mask],
            push_constant_ranges: &[],
        });
        // Depth-only sun pass: vertex transforms by env.light_view_proj,
        // no fragment stage at all.
        let shadow = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pc3d shadow pipeline"),
            layout: Some(&pl_globals),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_shadow"),
                buffers: std::slice::from_ref(&VERTEX_LAYOUT),
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                // NO pipeline bias: these units are full float-depth
                // steps on Depth32Float (constant 2 = the whole range —
                // every compare lit; the first shadow run caught it).
                // Acne is handled in the shader: normal offset + 0.0015.
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let shadow_cutout = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pc3d shadow cutout pipeline"),
            layout: Some(&pl_globals),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_shadow_cutout"),
                buffers: std::slice::from_ref(&crate::atmosphere::CUTOUT_LAYOUT),
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let cutout_layout = crate::atmosphere::CUTOUT_LAYOUT;
        let cutout = make(
            "pc3d cutout pipeline",
            &pl_cutout,
            "vs_cutout",
            "fs_cutout",
            std::slice::from_ref(&cutout_layout),
            // Two-sided leaf cards: no culling (a card's back is seen
            // through the field).
            None,
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            None,
        );

        let inst_layout = crate::flora::INSTANCE_LAYOUT;
        let inst = make(
            "pc3d flora inst pipeline",
            &pl_globals,
            "vs_inst",
            "fs_mesh",
            &[VERTEX_LAYOUT.clone(), inst_layout.clone()],
            Some(wgpu::Face::Back),
            Some(depth24.clone()),
            None,
        );
        let inst_cutout = make(
            "pc3d flora cutout pipeline",
            &pl_cutout,
            "vs_inst_cutout",
            "fs_cutout",
            &[
                crate::atmosphere::CUTOUT_LAYOUT,
                crate::flora::INSTANCE_LAYOUT_LATE,
            ],
            None,
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            None,
        );
        let inst_shadow = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pc3d flora inst shadow pipeline"),
            layout: Some(&pl_globals),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_inst_shadow"),
                buffers: &[VERTEX_LAYOUT.clone(), inst_layout],
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let box_inst_layout = crate::npcs::BOX_INSTANCE_LAYOUT;
        let inst_box = make(
            "pc3d crowd box pipeline",
            &pl_globals,
            "vs_inst_box",
            "fs_mesh",
            &[VERTEX_LAYOUT.clone(), box_inst_layout],
            Some(wgpu::Face::Back),
            Some(depth24.clone()),
            None,
        );
        let flora = FloraPipelines {
            inst,
            inst_cutout,
            inst_shadow,
            inst_box,
        };
        Self {
            sky: make(
                "pc3d sky pipeline",
                &pl_globals,
                "vs_sky",
                "fs_sky",
                &[],
                None,
                Some(depth_off.clone()),
                None,
            ),
            mesh: make(
                "pc3d mesh pipeline",
                &pl_globals,
                "vs_mesh",
                "fs_mesh",
                std::slice::from_ref(&VERTEX_LAYOUT),
                Some(wgpu::Face::Back),
                Some(depth24.clone()),
                None,
            ),
            wire: {
                // The mesh path on LineList (debug edges/overlays): lines
                // have no winding, so no culling; unlit-bright comes from
                // sun-aligned normals at build time.
                let mut d = wgpu::RenderPipelineDescriptor {
                    label: Some("pc3d wire pipeline"),
                    layout: Some(&pl_globals),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vs_mesh"),
                        buffers: std::slice::from_ref(&VERTEX_LAYOUT),
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fs_mesh"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: target_format,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::LineList,
                        cull_mode: None,
                        front_face: wgpu::FrontFace::Ccw,
                        ..Default::default()
                    },
                    // X-ray inspection depth: lines rasterize depth in
                    // ulps-different steps than the triangles they lie
                    // on, so LessEqual makes 1px edges lose z-fights and
                    // vanish. Always + no write = the see-through
                    // wireframe the inspector wants (all edges visible
                    // over the dimmed world).
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth24Plus,
                        depth_write_enabled: false,
                        depth_compare: wgpu::CompareFunction::Always,
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: Default::default(),
                    multiview: None,
                    cache: None,
                };
                device.create_render_pipeline(&d)
            },
            hud: make(
                "pc3d hud pipeline",
                &pl_hud,
                "vs_hud",
                "fs_hud",
                std::slice::from_ref(&hud_layout),
                None,
                Some(depth_off.clone()),
                Some(wgpu::BlendState::ALPHA_BLENDING),
            ),
            ui: make(
                "pc3d ui layer pipeline",
                &pl_hud,
                "vs_hud",
                "fs_ui",
                std::slice::from_ref(&hud_layout),
                None,
                Some(depth_off),
                Some(wgpu::BlendState::ALPHA_BLENDING),
            ),
            water,
            shadow,
            shadow_cutout,
            cutout,
            layout_globals,
            layout_hud,
            layout_mask,
            flora,
        }
    }
}

struct GpuScene {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl GpuScene {
    fn new(device: &wgpu::Device) -> Self {
        let (verts, idx) = crate::scene::build_scene();
        Self::from_mesh(device, &verts, &idx)
    }

    fn empty(device: &wgpu::Device) -> Self {
        Self::from_mesh(device, &[], &[])
    }

    fn from_mesh(device: &wgpu::Device, verts: &[crate::scene::SceneVertex], idx: &[u16]) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene vertices"),
            contents: bytemuck::cast_slice(verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene indices"),
            contents: bytemuck::cast_slice(idx),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            vertex_buffer,
            index_buffer,
            index_count: idx.len() as u32,
        }
    }
}

struct HudResources {
    texture: wgpu::Texture,
    size: (u32, u32),
    vertex_buffer: wgpu::Buffer,
    line: String,
    anchor: HudAnchor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HudAnchor {
    TopLeft,
    Center,
}

/// The uploaded UI canvas: one fullscreen straight-alpha RGBA texture.
struct UiLayer {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    size: (u32, u32),
    /// The last uploaded canvas bytes (proof pixel checks read this copy
    /// instead of a GPU readback).
    last_canvas: Vec<u8>,
}

pub struct Renderer {
    ctx: GpuContext,
    surface: Option<SurfaceGuard>,
    window: Option<Arc<winit::window::Window>>,
    format: wgpu::TextureFormat,
    pipelines: Pipelines,
    gpu_scene: GpuScene,
    camera: Camera,
    globals_buf: wgpu::Buffer,
    bg_globals: wgpu::BindGroup,
    bg_hud: wgpu::BindGroup,
    hud: HudResources,
    /// The owner-facing UI layer canvas (GLM UI rework). Uploaded only when
    /// the UI state changes; drawn fullscreen after the HUD debug line.
    ui_layer: Option<UiLayer>,
    /// Depth attachment for the active target (window or offscreen).
    depth: Option<wgpu::Texture>,
    offscreen: Option<wgpu::Texture>,
    /// Host-owned construction overlay, meshed per patch (R3DV-004).
    construction: Option<crate::construction::ConstructionGpu>,
    /// Natural terrain from the authoritative query (R3DV-005): one
    /// combined mesh per load; per-patch versions arrive with streaming
    /// (R3DV-006).
    terrain: Option<GpuMesh>,
    /// Streamed terrain with bounded per-frame work (R3DV-006); replaces
    /// the static terrain mesh while attached.
    streamer: Option<crate::streaming::TerrainStreamer>,
    /// Streamed SURFACE terrain (NWR-004) — the ordinary path when
    /// attached; draws before the legacy paths.
    surface_stream: Option<crate::surface_stream::SurfaceStreamer>,
    /// River water from flow records (R3DV-007).
    water: Option<crate::water::WaterSections>,
    /// Castle/city modules from the placement authorities (R3DV-008).
    city: Option<GpuMesh>,
    /// NPCs + inspect anchor boxes (R3DV-009).
    npcs: Option<GpuMesh>,
    /// Placed GLB assets (NWR-002): (mesh, triangles) for the record.
    assets: Vec<(GpuMesh, usize)>,
    /// WT-002/003 slice 3: per-asset anchors + bounds for the overlay.
    asset_anchors: Vec<AssetAnchors>,
    /// CPU-side deduped wire edges of every loaded asset (rebuilt to GPU
    /// lazily when a debug mode first draws).
    wire_edges: Vec<crate::scene::SceneVertex>,
    wire_dirty: bool,
    /// The GPU edge buffer (rebuilt when wire_dirty).
    wire_gpu: Option<GpuMesh>,
    /// The GPU anchor/bounds overlay buffer (rebuilt when dirty).
    overlay_gpu: Option<GpuMesh>,
    overlay_dirty: bool,
    /// The scene debug mode: Normal / Wireframe / AnchorOverlay.
    scene_debug: SceneDebugMode,
    /// HOUSE ENTRY (WT-002 slice 5): the settlement's collision cells
    /// (wall rings solid, doors + interiors open) mounted over the
    /// streamed surface when the kit attaches.
    settlement_cells: Option<std::collections::BTreeSet<(i32, i32)>>,
    settlement_doors: Vec<crate::settlement::DoorEntry>,
    /// WT-007 slice 1: the debug groups pushed this frame (the marker
    /// tree audit reads this) + the top-level draw-call count.
    marker_log: Vec<&'static str>,
    draw_calls: u32,
    start: std::time::Instant,
    /// Frozen water time for deterministic proofs (None = wall clock).
    water_time_override: Option<f32>,
    /// Detail texture flag (high tier).
    detail_flag: f32,
    /// Atmosphere state (NWR-006): LEGACY (all off) until a tier is set.
    atmosphere: crate::atmosphere::Atmosphere,
    /// The env uniform buffer (fog/shadow/material params + light VP).
    env_buf: wgpu::Buffer,
    /// The sun's depth map view (a cleared 1x1 dummy when shadows are off).
    shadow_view: wgpu::TextureView,
    /// The depth texture itself (diagnostic readback).
    shadow_tex: wgpu::Texture,
    /// The permanent 1x1 dummy view — the SHADOW pass's bind group must
    /// not sample the texture it writes (wgpu usage-scope law), so the
    /// light pass binds globals with the dummy in the sampled slot.
    shadow_dummy: wgpu::TextureView,
    /// The light pass's globals bind group (dummy in binding 5).
    bg_light: wgpu::BindGroup,
    /// The detail/material atlas texture (kept for bind-group rebuilds).
    detail: wgpu::Texture,
    /// Alpha-cutout foliage slot (NWR-006): mesh + its mask bind group.
    cutout: Option<(GpuMesh, wgpu::BindGroup)>,
    /// Streamed wilderness (NWR-007): instanced plants + grass cards.
    flora: Option<crate::flora::FloraStreamer>,
    /// The settlement kit scene (NWR-008): static instanced modules.
    settlement: Option<crate::settlement::SettlementGpu>,
    /// The NPC crowd (NWR-009): rig instances per part color.
    crowd: Option<CrowdGpu>,
    /// The crowd's pose update rate (Hz) — the Deck Low lever.
    crowd_pose_hz: f32,
    /// SCREEN HEIGHT MAP debug: fragments output a world-height ramp.
    height_debug: bool,
    /// The crowd's authority inputs (gen + cast + nav), for per-frame
    /// poses and schedule ticks.
    crowd_gen: Option<std::rc::Rc<pc3d_world::gen::WorldGen>>,
    crowd_cast: Option<std::rc::Rc<std::cell::RefCell<Vec<crate::npcs::NpcCast>>>>,
    crowd_nav: Option<pc3d_world::nav::NavPatch>,
    /// The generator the flora placement reads (authority).
    flora_gen: Option<std::rc::Rc<pc3d_world::gen::WorldGen>>,
}

/// A plain vertex/index buffer pair (u16 or u32 indices).
/// WT-002/003 slice 3: the scene debug capture modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneDebugMode {
    Normal,
    /// Deduped mesh edges as bright lines over a dimmed world.
    Wireframe,
    /// Anchor axis crosses + bounds wire boxes over a dimmed world.
    AnchorOverlay,
}

/// One loaded asset's overlay data: world placement, named sockets,
/// and the lod0 bounds (min/max already translated by placement).
#[derive(Clone, Debug)]
pub struct AssetAnchors {
    pub glb_id: String,
    pub placement: [f32; 3],
    pub sockets: Vec<(String, [f32; 3])>,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
}

/// Extracts the UNIQUE edges of an indexed mesh (each edge once, as a
/// LineList vertex pair). Wireframe lines get sun-aligned normals so
/// the lit path draws them at full, deterministic brightness.
pub fn extract_wire_edges(
    verts: &[crate::scene::SceneVertex],
    idx: &[u32],
    color: [f32; 3],
) -> Vec<crate::scene::SceneVertex> {
    let sun = crate::scene::SUN_DIR;
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for t in idx.chunks_exact(3) {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let key = (a.min(b), a.max(b));
            if !seen.insert(key) {
                continue;
            }
            for vi in [key.0, key.1] {
                let v = verts[vi as usize];
                out.push(crate::scene::SceneVertex {
                    pos: v.pos,
                    normal: sun,
                    color,
                });
            }
        }
    }
    out
}

/// Builds the anchor overlay: per socket a 3-axis cross (X ember, Y
/// green, Z blue) and per asset a white bounds wire box (12 edges).
pub fn build_overlay_edges(anchors: &[AssetAnchors]) -> Vec<crate::scene::SceneVertex> {
    let sun = crate::scene::SUN_DIR;
    let mut out = Vec::new();
    let mut line = |from: [f32; 3], to: [f32; 3], c: [f32; 3], out: &mut Vec<crate::scene::SceneVertex>| {
        for p in [from, to] {
            out.push(crate::scene::SceneVertex { pos: p, normal: sun, color: c });
        }
    };
    const ARM: f32 = 0.22;
    for a in anchors {
        for (_, s) in &a.sockets {
            let p = [a.placement[0] + s[0], a.placement[1] + s[1], a.placement[2] + s[2]];
            line([p[0] - ARM, p[1], p[2]], [p[0] + ARM, p[1], p[2]], [1.0, 0.55, 0.15], &mut out);
            line([p[0], p[1] - ARM, p[2]], [p[0], p[1] + ARM, p[2]], [0.35, 1.0, 0.45], &mut out);
            line([p[0], p[1], p[2] - ARM], [p[0], p[1], p[2] + ARM], [0.35, 0.65, 1.0], &mut out);
        }
        // Bounds box: 12 edges.
        let (mn, mx) = (a.bounds_min, a.bounds_max);
        let corners: [[f32; 3]; 8] = [
            [mn[0], mn[1], mn[2]], [mx[0], mn[1], mn[2]],
            [mx[0], mx[1], mn[2]], [mn[0], mx[1], mn[2]],
            [mn[0], mn[1], mx[2]], [mx[0], mn[1], mx[2]],
            [mx[0], mx[1], mx[2]], [mn[0], mx[1], mx[2]],
        ];
        for (i, j) in [
            (0, 1), (1, 2), (2, 3), (3, 0),
            (4, 5), (5, 6), (6, 7), (7, 4),
            (0, 4), (1, 5), (2, 6), (3, 7),
        ] {
            line(corners[i], corners[j], [0.92, 0.94, 0.98], &mut out);
        }
    }
    out
}

struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    u32_indices: bool,
}

impl GpuMesh {
    fn from_mesh<T: bytemuck::Pod>(device: &wgpu::Device, verts: &[T], idx: &[u16]) -> Self {
        use wgpu::util::DeviceExt;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh vertices"),
            contents: bytemuck::cast_slice(verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh indices"),
            contents: bytemuck::cast_slice(idx),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            vertex_buffer,
            index_buffer,
            index_count: idx.len() as u32,
            u32_indices: false,
        }
    }

    fn from_mesh_u32(
        device: &wgpu::Device,
        verts: &[crate::scene::SceneVertex],
        idx: &[u32],
    ) -> Self {
        use wgpu::util::DeviceExt;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh vertices"),
            contents: bytemuck::cast_slice(verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh indices"),
            contents: bytemuck::cast_slice(idx),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            vertex_buffer,
            index_buffer,
            index_count: idx.len() as u32,
            u32_indices: true,
        }
    }

    fn draw<'rp>(&self, pass: &mut wgpu::RenderPass<'rp>) {
        let format = if self.u32_indices {
            wgpu::IndexFormat::Uint32
        } else {
            wgpu::IndexFormat::Uint16
        };
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), format);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

impl Renderer {
    /// Live windowed renderer.
    pub fn windowed(window: &Arc<winit::window::Window>) -> Self {
        let ctx = GpuContext::new(None);
        let guard = SurfaceGuard::new(&ctx, window);
        let format = guard.format();
        let (w, h) = (guard.config.width, guard.config.height);
        let pipelines = Pipelines::new(&ctx.device, format);
        let gpu_scene = GpuScene::new(&ctx.device);
        let camera = Camera::new(default_pose());
        let globals_buf = create_globals_buffer(&ctx.device);
        let (detail, detail_data) = create_atlas_texture(&ctx.device);
        upload_detail(&ctx.queue, &detail, &detail_data);
        let env_buf = create_env_buffer(&ctx.device);
        let (_dummy_tex, shadow_dummy) = create_shadow_map(&ctx.device, &ctx.queue, 0);
        let shadow_view = shadow_dummy.clone();
        let bg_globals = create_globals_bind_group(
            &ctx.device,
            &pipelines.layout_globals,
            &globals_buf,
            &detail,
            &env_buf,
            &shadow_view,
        );
        let bg_light = create_globals_bind_group(
            &ctx.device,
            &pipelines.layout_globals,
            &globals_buf,
            &detail,
            &env_buf,
            &shadow_dummy,
        );
        let mut hud = create_hud(&ctx.device, &pipelines.layout_hud);
        hud.line = default_hud_line(&camera);
        let depth = Some(create_depth(&ctx.device, w, h));
        let bg_hud = create_hud_bind_group(
            &ctx.device,
            &pipelines.layout_hud,
            &globals_buf,
            &hud.texture,
            ui_view_format(format),
        );
        Self {
            ctx,
            surface: Some(guard),
            window: Some(window.clone()),
            format,
            pipelines,
            gpu_scene,
            camera,
            globals_buf,
            bg_globals,
            bg_hud,
            hud,
            ui_layer: None,
            depth,
            offscreen: None,
            atmosphere: crate::atmosphere::LEGACY,
            env_buf,
            shadow_view,
            shadow_tex: _dummy_tex,
            shadow_dummy,
            bg_light,
            detail,
            cutout: None,
            flora: None,
            flora_gen: None,
            settlement: None,
            crowd: None,
            crowd_gen: None,
            crowd_cast: None,
            crowd_nav: None,
            crowd_pose_hz: 60.0,
            height_debug: false,
            construction: None,
            terrain: None,
            streamer: None,
            surface_stream: None,
            water: None,
            city: None,
            npcs: None,
            assets: Vec::new(),
            asset_anchors: Vec::new(),
            wire_edges: Vec::new(),
            wire_dirty: false,
            wire_gpu: None,
            overlay_gpu: None,
            overlay_dirty: true,
            scene_debug: SceneDebugMode::Normal,
            settlement_cells: None,
            settlement_doors: Vec::new(),
            marker_log: Vec::new(),
            draw_calls: 0,
            start: std::time::Instant::now(),
            water_time_override: None,
            detail_flag: 0.0,
        }
    }

    /// Headless proof renderer: same pipelines and scene, render target
    /// instead of a swapchain, so `cargo test` exercises the exact GPU path.
    pub fn offscreen(width: u32, height: u32) -> Self {
        let ctx = GpuContext::new(None);
        let format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let pipelines = Pipelines::new(&ctx.device, format);
        let gpu_scene = GpuScene::new(&ctx.device);
        let camera = Camera::new(default_pose());
        let globals_buf = create_globals_buffer(&ctx.device);
        let (detail, detail_data) = create_atlas_texture(&ctx.device);
        upload_detail(&ctx.queue, &detail, &detail_data);
        let env_buf = create_env_buffer(&ctx.device);
        let (_dummy_tex, shadow_dummy) = create_shadow_map(&ctx.device, &ctx.queue, 0);
        let shadow_view = shadow_dummy.clone();
        let bg_globals = create_globals_bind_group(
            &ctx.device,
            &pipelines.layout_globals,
            &globals_buf,
            &detail,
            &env_buf,
            &shadow_view,
        );
        let bg_light = create_globals_bind_group(
            &ctx.device,
            &pipelines.layout_globals,
            &globals_buf,
            &detail,
            &env_buf,
            &shadow_dummy,
        );
        let mut hud = create_hud(&ctx.device, &pipelines.layout_hud);
        // Offscreen proof captures render the WORLD, not the dev debug
        // line: a default-on line here quietly covers probe points (and
        // before the samurai-cut fix it only covered HALF its quad, so
        // probes were calibrated around a torn quad). Empty = no draw.
        let texture = create_target(&ctx.device, width, height, format);
        let depth = Some(create_depth(&ctx.device, width, height));
        let bg_hud = create_hud_bind_group(
            &ctx.device,
            &pipelines.layout_hud,
            &globals_buf,
            &hud.texture,
            ui_view_format(format),
        );
        Self {
            ctx,
            surface: None,
            window: None,
            format,
            pipelines,
            gpu_scene,
            camera,
            globals_buf,
            bg_globals,
            bg_hud,
            hud,
            ui_layer: None,
            depth,
            offscreen: Some(texture),
            atmosphere: crate::atmosphere::LEGACY,
            env_buf,
            shadow_view,
            shadow_tex: _dummy_tex,
            shadow_dummy,
            bg_light,
            detail,
            cutout: None,
            flora: None,
            flora_gen: None,
            settlement: None,
            crowd: None,
            crowd_gen: None,
            crowd_cast: None,
            crowd_nav: None,
            crowd_pose_hz: 60.0,
            height_debug: false,
            construction: None,
            terrain: None,
            streamer: None,
            surface_stream: None,
            water: None,
            city: None,
            npcs: None,
            assets: Vec::new(),
            asset_anchors: Vec::new(),
            wire_edges: Vec::new(),
            wire_dirty: false,
            wire_gpu: None,
            overlay_gpu: None,
            overlay_dirty: true,
            scene_debug: SceneDebugMode::Normal,
            settlement_cells: None,
            settlement_doors: Vec::new(),
            marker_log: Vec::new(),
            draw_calls: 0,
            start: std::time::Instant::now(),
            water_time_override: None,
            detail_flag: 0.0,
        }
    }

    pub fn pose(&self) -> CameraPose {
        self.camera.pose
    }

    /// First-person walk step (4 m/s) from the current camera orientation.
    pub fn camera_walk_step(&self, fwd: f32, strafe: f32, vert: f32, dt: f32) -> [f32; 3] {
        self.camera.walk_step(fwd, strafe, vert, dt, 4.0)
    }

    /// Teleports/rotates the camera to a P3D world pose.
    pub fn set_pose(&mut self, pose: CameraPose) {
        self.camera.pose = pose;
    }

    /// Enables the construction overlay renderer (idempotent).
    pub fn attach_construction(&mut self) {
        if self.construction.is_none() {
            self.construction = Some(crate::construction::ConstructionGpu::new());
        }
    }

    /// Syncs construction meshes from the HOST's overlay map (immutable —
    /// the renderer has no route to mutate canonical world state). Edits
    /// reach the world only through host commands.
    pub fn update_construction(
        &mut self,
        host_construction: &std::collections::BTreeMap<
            (i32, i32, i32),
            pc3d_world::build::Construction,
        >,
    ) -> crate::construction::UpdateStats {
        let Some(con) = self.construction.as_mut() else {
            panic!("call attach_construction() before update_construction()");
        };
        con.update(&self.ctx.device, host_construction)
    }

    /// Loads natural-terrain patches meshed from the authoritative
    /// `final_solid` query (read-only world access). Replaces any previous
    /// terrain mesh; per-patch versioned streaming is R3DV-006.
    pub fn load_terrain(
        &mut self,
        gen: &pc3d_world::gen::WorldGen,
        patches: &[pc3d_world::coords::PatchCoord],
    ) -> crate::terrain::TerrainStats {
        let t0 = std::time::Instant::now();
        let mut verts = Vec::new();
        let mut idx = Vec::new();
        for coord in patches {
            let (v, i) = crate::terrain::mesh_patch_natural(gen, *coord);
            let base = verts.len() as u32;
            verts.extend(v);
            idx.extend(i.iter().map(|k| *k as u32 + base));
        }
        let stats = crate::terrain::TerrainStats {
            patches: patches.len(),
            vertices: verts.len(),
            triangles: idx.len() / 3,
            mesh_us: t0.elapsed().as_micros(),
        };
        // u32 indices: concatenated vistas exceed the u16 range.
        self.terrain = Some(GpuMesh::from_mesh_u32(&self.ctx.device, &verts, &idx));
        stats
    }

    /// Applies a quality tier (R3DV-010): streaming rings/budgets, detail
    /// texture, water animation cadence.
    pub fn set_quality(&mut self, tier: QualityTier) {
        self.detail_flag = tier.detail();
        if let Some(streamer) = self.streamer.as_mut() {
            streamer.set_config(crate::streaming::StreamConfig {
                max_mesh_per_frame: tier.max_mesh_per_frame(),
                max_uploads_per_frame: tier.max_mesh_per_frame() * 2,
                gpu_byte_budget: tier.gpu_byte_budget(),
                tiers: tier.tiers(),
            });
        }
    }

    /// Applies an atmosphere tier (NWR-006): shadows/fog/material
    /// detail/water glint with the tier's documented budget. The renderer
    /// starts at LEGACY (every term off) so pre-NWR-006 proofs stay
    /// bit-identical until this is called.
    pub fn set_atmosphere_tier(&mut self, tier: crate::atmosphere::AtmosphereTier) {
        self.set_atmosphere(tier.params());
    }

    /// Applies raw atmosphere parameters (the control-render path for
    /// proofs: same scene with one term toggled).
    pub fn set_atmosphere(&mut self, atm: crate::atmosphere::Atmosphere) {
        if atm.shadow_res != self.atmosphere.shadow_res {
            let (tex, view) = create_shadow_map(&self.ctx.device, &self.ctx.queue, atm.shadow_res);
            self.shadow_tex = tex;
            self.shadow_view = view;
            self.bg_globals = create_globals_bind_group(
                &self.ctx.device,
                &self.pipelines.layout_globals,
                &self.globals_buf,
                &self.detail,
                &self.env_buf,
                &self.shadow_view,
            );
            self.bg_light = create_globals_bind_group(
                &self.ctx.device,
                &self.pipelines.layout_globals,
                &self.globals_buf,
                &self.detail,
                &self.env_buf,
                &self.shadow_dummy,
            );
        }
        self.atmosphere = atm;
    }

    /// The current atmosphere parameters (proof introspection).
    pub fn atmosphere(&self) -> crate::atmosphere::Atmosphere {
        self.atmosphere
    }

    /// Attaches the streamed wilderness (NWR-007): instanced plants
    /// placed by the pure pc3d_world::flora authority, updated with a
    /// bounded slot budget every frame.
    pub fn attach_flora(&mut self, gen: std::rc::Rc<pc3d_world::gen::WorldGen>) {
        let mut f = crate::flora::FloraStreamer::new(&self.ctx.device);
        f.attach_mask(
            &self.ctx.device,
            &self.ctx.queue,
            &self.pipelines.layout_mask,
        );
        self.flora = Some(f);
        self.flora_gen = Some(gen);
    }

    /// Attaches the assembled settlement kit scene (NWR-008). The kit
    /// GLBs load once; placements are static (re-derived on load).
    pub fn attach_settlement(
        &mut self,
        scene: &crate::settlement::KitScene,
        kit: &crate::settlement::SettlementKit,
    ) {
        let viewer = [self.camera.pose.position[0], self.camera.pose.position[2]];
        let gpu = crate::settlement::SettlementGpu::new(&self.ctx.device, scene, kit, viewer);
        self.settlement_cells = Some(scene.collision_cells.clone());
        self.settlement_doors = scene.door_entries.clone();
        self.settlement = Some(gpu);
    }

    /// The settlement's enterable-building record (the route's framing).
    pub fn settlement_door_entries(&self) -> &[crate::settlement::DoorEntry] {
        &self.settlement_doors
    }

    /// The walk-surface ground height at a world XZ (the streamed
    /// surface when attached, else the generator authority) — the
    /// teleport hook's grounding.
    pub fn ground_y_at(&self, gen: &pc3d_world::gen::WorldGen, x: f32, z: f32) -> f32 {
        use crate::player::CollisionSurface as _;
        if let Some(ss) = self.surface_stream.as_ref() {
            if let Some(g) = ss.ground_at(gen, x, z, f32::MAX / 4.0) {
                return g;
            }
        }
        gen.effective_surface_mm((x * 1000.0) as i64, (z * 1000.0) as i64) as f32 / 1000.0
    }

    /// Attaches the NPC crowd (NWR-009): the rig draws from the
    /// authoritative brains; poses rebuild per frame from (intent, t).
    pub fn attach_crowd(
        &mut self,
        gen: std::rc::Rc<pc3d_world::gen::WorldGen>,
        cast: Vec<crate::npcs::NpcCast>,
        nav: pc3d_world::nav::NavPatch,
    ) {
        self.crowd = Some(CrowdGpu::new(&self.ctx.device));
        self.crowd_gen = Some(gen);
        self.crowd_cast = Some(std::rc::Rc::new(std::cell::RefCell::new(cast)));
        self.crowd_nav = Some(nav);
    }

    /// Stages N presentation-copy walkers crossing in front of the
    /// camera (the sim's own Walking intent on cloned brains — proofs
    /// may stage visible motion without touching the sim's cast).
    pub fn crowd_stage_walkers(&mut self, n: usize) {
        let Some(cast) = self.crowd_cast.clone() else {
            return;
        };
        let fwd = self.camera.fwd();
        let pos = self.camera.pose.position;
        let mut cast = cast.borrow_mut();
        for k in 0..n.min(cast.len()) {
            // A path crossing the view ~6 m ahead of the camera.
            let cx = pos[0] + fwd[0] * 6.0 + fwd[2] * (k as f32 - (n as f32 - 1.0) / 2.0) * 2.0;
            let cz = pos[2] + fwd[2] * 6.0 - fwd[0] * (k as f32 - (n as f32 - 1.0) / 2.0) * 2.0;
            let (x0, z0) = (cx - fwd[2] * 4.0, cz + fwd[0] * 4.0);
            let (x1, z1) = (cx + fwd[2] * 4.0, cz - fwd[0] * 4.0);
            cast[k].brain.intent = pc3d_world::npc::Intent::Walking {
                path: vec![
                    pc3d_world::coords::CellCoord {
                        x: x0 as i32,
                        y: 0,
                        z: z0 as i32,
                    },
                    pc3d_world::coords::CellCoord {
                        x: cx as i32,
                        y: 0,
                        z: cz as i32,
                    },
                    pc3d_world::coords::CellCoord {
                        x: x1 as i32,
                        y: 0,
                        z: z1 as i32,
                    },
                ],
                leg: 1,
            };
            cast[k].brain.pos = pc3d_world::coords::CellCoord {
                x: x0 as i32,
                y: 0,
                z: z0 as i32,
            };
        }
    }

    /// Ticks the crowd's AUTHORITATIVE brains (the schedule drives who
    /// walks/works/sleeps; the renderer only poses what the sim says).
    pub fn crowd_tick(&mut self, day_fraction: f32, ticks: usize) {
        if let (Some(cast), Some(nav)) = (self.crowd_cast.clone(), self.crowd_nav.as_ref()) {
            crate::npcs::advance(&mut cast.borrow_mut(), nav, day_fraction, ticks);
        }
    }

    /// The SCREEN HEIGHT MAP mode: every world fragment outputs a
    /// world-height ramp instead of its lit color — a per-pixel map of
    /// WHAT IS SHOWN, for inspecting terrain shape, LOD seams, and holes.
    pub fn set_height_debug(&mut self, on: bool) {
        self.height_debug = on;
    }

    /// WT-002/003 slice 3: the scene debug capture mode — Wireframe
    /// draws the assets' deduped edges over a dimmed world; AnchorOverlay
    /// draws socket axis-crosses + bounds boxes.
    pub fn set_scene_debug(&mut self, mode: SceneDebugMode) {
        self.scene_debug = mode;
    }

    /// The overlay data recorded so far (proof sidecars).
    pub fn asset_anchor_data(&self) -> &[AssetAnchors] {
        &self.asset_anchors
    }

    /// WT-007 slice 1: push a contract marker (debug group + audit log).
    fn mark(&mut self, pass: &mut wgpu::RenderPass, name: &'static str) {
        pass.push_debug_group(name);
        self.marker_log.push(name);
    }

    fn unmark(&self, pass: &mut wgpu::RenderPass) {
        pass.pop_debug_group();
    }

    /// The marker tree recorded this frame (the gpu-marker audit input).
    pub fn marker_tree(&self) -> &[&'static str] {
        &self.marker_log
    }

    /// The NPC talk slice: the nearest LIVE villager within TALK_RANGE
    /// of the camera — (name, distance, cast index). None when no one
    /// is close enough to address.
    pub fn nearest_talk_target(&self) -> Option<(String, f32, usize)> {
        const TALK_RANGE: f32 = 3.0;
        let cast = self.crowd_cast.as_ref()?;
        let gen = self.crowd_gen.as_ref()?;
        let cast = cast.borrow();
        let p = self.camera.pose.position;
        let mut best: Option<(f32, usize)> = None;
        for (i, c) in cast.iter().enumerate() {
            let np = crate::npcs::npc_world_pos(gen, &c.brain);
            let d = ((np[0] - p[0]).powi(2) + (np[1] - p[1]).powi(2) + (np[2] - p[2]).powi(2))
                .sqrt();
            if d <= TALK_RANGE && best.map(|(bd, _)| d < bd).unwrap_or(true) {
                best = Some((d, i));
            }
        }
        let (d, i) = best?;
        Some((pc3d_world::dialog::villager_name(cast[i].brain.home), d, i))
    }

    /// The world position of cast member `i` (talk-route framing).
    pub fn cast_position(&self, i: usize) -> Option<[f32; 3]> {
        let cast = self.crowd_cast.as_ref()?;
        let gen = self.crowd_gen.as_ref()?;
        let cast = cast.borrow();
        Some(crate::npcs::npc_world_pos(gen, &cast.get(i)?.brain))
    }

    /// Speaks with the cast member at `i` — the dialog authority's line
    /// for their LIVE brain.
    pub fn talk_with_index(&self, i: usize) -> Option<pc3d_world::dialog::DialogLine> {
        let cast = self.crowd_cast.as_ref()?.borrow();
        let brain = cast.get(i)?;
        Some(pc3d_world::dialog::talk_with(&brain.brain))
    }

    /// The top-level draw-call count for this frame (per-patch draws
    /// inside streamer modules are counted by their own counters).
    pub fn frame_draw_calls(&self) -> u32 {
        self.draw_calls
    }

    /// The adapter identity for the audit (backend + name).
    pub fn adapter_identity(&self) -> (String, String) {
        let info = self.ctx.adapter.get_info();
        (format!("{:?}", info.backend), info.name)
    }

    /// Whether this device supports GPU timestamp queries (requested
    /// features are empty, so queries are not ENABLED even when
    /// supported — the audit reports support + the honest reason).
    pub fn timestamp_query_supported(&self) -> bool {
        self.ctx
            .adapter
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
    }

    /// WT-007 slice 1: the gpu_marker_contract audit — the required
    /// marker names, adapter identity, timestamp policy, CPU frame
    /// timing, and the top-level draw-call count from the LAST frame.
    pub fn gpu_marker_audit(&self, cpu_p50_ms: f32) -> serde_json::Value {
        const REQUIRED: &[&str] = &[
            "pc3d.frame",
            "pc3d.frame.prepare",
            "pc3d.pass.terrain",
            "pc3d.pass.sky_atmosphere",
            "pc3d.pass.water",
            "pc3d.pass.assets",
            "pc3d.pass.npcs",
            "pc3d.pass.machines",
            "pc3d.pass.wireframe_overlay",
            "pc3d.pass.ui",
            "pc3d.pass.screenshot_readback",
        ];
        let (backend, adapter) = self.adapter_identity();
        let ts_supported = self.timestamp_query_supported();
        let markers_ok = REQUIRED
            .iter()
            .all(|m| self.marker_log.iter().any(|x| x == m));
        serde_json::json!({
            "version": 1,
            "build_hash": crate::ui::build_stamp(),
            "api_backend": backend,
            "adapter_name": adapter,
            "markers_present": markers_ok,
            "markers_recorded": self.marker_log,
            "markers_required": REQUIRED,
            "timestamp_support": ts_supported,
            "gpu_frame_ms_or_reason": if ts_supported {
                serde_json::json!({"value": null, "reason": "timestamps supported but not enabled (device requests no extra features); CPU timing is the evidence"})
            } else {
                serde_json::json!({"value": null, "reason": "adapter does not expose TIMESTAMP_QUERY; CPU timing is the evidence"})
            },
            "cpu_frame_ms": (cpu_p50_ms * 100.0).round() / 100.0,
            "draw_calls": self.draw_calls,
            "triangles": self.assets.iter().map(|(_, t)| t).sum::<usize>(),
            "material_buckets": {
                "note": "asset triangle sum is exact; streamer bucket counts live in their own counters",
            },
        })
    }

    /// The Deck Low lever: gate the rig's pose updates to N Hz (the
    /// rig still moves — at bench-stable steps; deterministic under
    /// frozen time because the step is QUANTIZED).
    pub fn set_crowd_pose_rate(&mut self, hz: f32) {
        self.crowd_pose_hz = hz.max(1.0);
    }

    /// The crowd's draw/count record.
    pub fn crowd_stats(&self) -> (usize, usize) {
        self.crowd
            .as_ref()
            .map(|c| (c.draws, c.instances))
            .unwrap_or((0, 0))
    }

    /// One crowd frame: pose from the authoritative intent at time t
    /// (frozen-able for proofs) and upload the per-color buckets.
    pub fn crowd_frame(&mut self, t: f32) {
        let (Some(g), Some(cast)) = (self.crowd_gen.clone(), self.crowd_cast.clone()) else {
            return;
        };
        let Some(crowd) = self.crowd.as_mut() else {
            return;
        };
        let viewer = [self.camera.pose.position[0], self.camera.pose.position[2]];
        let rows = crate::npcs::crowd_instances(&g, &cast.borrow(), viewer, t, 64.0);
        crowd.upload(&self.ctx.device, &rows);
    }

    /// The settlement kit draw/triangle record.
    pub fn settlement_stats(&self) -> (usize, usize) {
        self.settlement
            .as_ref()
            .map(|s| (s.draws, s.tris))
            .unwrap_or((0, 0))
    }

    /// The flora streamer's config (tier budgets).
    pub fn set_flora_config(&mut self, cfg: crate::flora::FloraConfig) {
        if let Some(f) = self.flora.as_mut() {
            f.set_config(cfg);
        }
    }

    /// The flora counters (bounded-work + eviction proofs).
    pub fn flora_stats(&self) -> crate::flora::FloraStats {
        self.flora.as_ref().map(|f| f.stats).unwrap_or_default()
    }

    /// Diagnostic: reads the sun shadow map back as f32 depths (the
    /// shadow proofs' debugging hook; 1x1 when shadows are off).
    pub fn debug_shadow_map(&self) -> Vec<f32> {
        let dim = self.atmosphere.shadow_res.max(1);
        let bytes_per_row = (dim * 4).div_ceil(256) * 256;
        let buf = self.ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pc3d shadow readback"),
            size: (bytes_per_row * dim) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.ctx.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            self.shadow_tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: dim,
                height: dim,
                depth_or_array_layers: 1,
            },
        );
        self.ctx.queue.submit(Some(encoder.finish()));
        let (snd, rcv) = std::sync::mpsc::channel();
        let mapped = buf.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = snd.send(r);
        });
        let _ = mapped;
        self.ctx.device.poll(wgpu::Maintain::Wait);
        rcv.recv().expect("map");
        let data = buf.slice(..).get_mapped_range().to_vec();
        buf.unmap();
        data.chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect()
    }

    /// Loads alpha-cutout foliage (NWR-006): crossed leaf cards tested
    /// against a deterministic mask (Leaf = nibbled silhouette, Solid =
    /// the opaque control). Returns the triangle count.
    pub fn load_cutout(
        &mut self,
        verts: &[crate::atmosphere::CutoutVertex],
        idx: &[u16],
        mask: crate::atmosphere::CutoutMask,
    ) -> usize {
        let mesh = GpuMesh::from_mesh(&self.ctx.device, verts, idx);
        let data = match mask {
            crate::atmosphere::CutoutMask::Leaf => crate::atmosphere::leaf_mask_rgba(),
            crate::atmosphere::CutoutMask::Solid => crate::atmosphere::solid_mask_rgba(),
        };
        let n = crate::atmosphere::LEAF_MASK_PX;
        let tex = self.ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pc3d cutout mask"),
            size: wgpu::Extent3d {
                width: n,
                height: n,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(n * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: n,
                height: n,
                depth_or_array_layers: 1,
            },
        );
        let sampler = self.ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pc3d mask sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bg = self
            .ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("pc3d cutout mask bind group"),
                layout: &self.pipelines.layout_mask,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &tex.create_view(&Default::default()),
                        ),
                    },
                ],
            });
        let tris = idx.len() / 3;
        self.cutout = Some((mesh, bg));
        tris
    }

    /// Loads a GLB asset LOD at a world placement (meters; pivot at
    /// ground). Draws through the lit mesh pipeline; LOD choice is the
    /// caller's (see glb::Asset::lod_for). Returns the LOD's triangle
    /// count for the draw/triangle record.
    pub fn load_asset(
        &mut self,
        asset: &crate::glb::Asset,
        lod_name: &str,
        placement: [f32; 3],
    ) -> usize {
        let lod = asset
            .lods
            .iter()
            .find(|l| l.name == lod_name)
            .unwrap_or_else(|| asset.lods.last().expect("asset lods"));
        let baked = lod.translated(placement);
        let verts: Vec<crate::scene::SceneVertex> = baked.vertices;
        let idx: Vec<u32> = baked.indices;
        let tris = idx.len() / 3;
        // WT-002/003 slice 3: retain the overlay data (sockets + bounds)
        // and accumulate wire edges for the debug captures.
        let mut mn = [f32::MAX; 3];
        let mut mx = [f32::MIN; 3];
        for v in &verts {
            for i in 0..3 {
                mn[i] = mn[i].min(v.pos[i]);
                mx[i] = mx[i].max(v.pos[i]);
            }
        }
        // The GLB carries no self-id; callers know it (the semantic
        // registry maps ids), so the record is positional.
        self.asset_anchors.push(AssetAnchors {
            glb_id: format!("glb-{}", self.assets.len()),
            placement,
            sockets: asset.sockets.clone(),
            bounds_min: mn,
            bounds_max: mx,
        });
        self.wire_edges
            .extend(extract_wire_edges(&verts, &idx, [1.0, 0.82, 0.35]));
        self.wire_dirty = true;
        self.overlay_dirty = true;
        let mesh = GpuMesh::from_mesh_u32(&self.ctx.device, &verts, &idx);
        self.assets.push((mesh, tris));
        tris
    }

    /// Loads the NWR-003 surface spike mesh (replaces the drawn terrain
    /// for the isolated proof).
    pub fn load_surface(&mut self, verts: &[crate::scene::SceneVertex], idx: &[u32]) {
        self.terrain = Some(GpuMesh::from_mesh_u32(&self.ctx.device, verts, idx));
    }

    /// Test hook: the device (for streamer unit tests driving real
    /// buffers without a window).
    pub fn device_for_tests(&self) -> &wgpu::Device {
        &self.ctx.device
    }

    /// The offscreen renderer's device (unit tests' flora construction).
    pub fn device_for_tests_of(r: &Renderer) -> &wgpu::Device {
        &r.ctx.device
    }

    /// Attaches the SURFACE streamer (NWR-004 ordinary terrain path).
    pub fn attach_surface_stream(&mut self, s: crate::surface_stream::SurfaceStreamer) {
        self.surface_stream = Some(s);
    }

    /// Walks a player on the STREAMED SURFACE when attached (the
    /// NWR-011 live path), else the authority ground — the deferral
    /// from NWR-005 landing here.
    pub fn walk_player_surface(
        &mut self,
        gen: &pc3d_world::gen::WorldGen,
        player: &mut crate::player::PlayerBody,
        fwd: f32,
        strafe: f32,
        dt: f32,
    ) {
        self.walk_player_surface_speed(
            gen,
            player,
            fwd,
            strafe,
            dt,
            crate::player::WALK_SPEED,
        );
    }

    /// The sprint-aware walk: the streamed surface at an explicit speed.
    pub fn walk_player_surface_speed(
        &mut self,
        gen: &pc3d_world::gen::WorldGen,
        player: &mut crate::player::PlayerBody,
        fwd: f32,
        strafe: f32,
        dt: f32,
        speed: f32,
    ) {
        if let Some(cells) = self.settlement_cells.as_ref() {
            // HOUSE ENTRY: the kit's cells ride OVER the streamed
            // surface — walls stop the player, doors + interiors pass.
            if let Some(ss) = self.surface_stream.as_ref() {
                let ground = crate::settlement::SettlementGround {
                    inner: ss,
                    cells: cells.clone(),
                };
                player.walk_on_speed(gen, &ground, fwd, strafe, dt, speed);
                return;
            }
        }
        if let Some(ss) = self.surface_stream.as_ref() {
            player.walk_on_speed(gen, ss, fwd, strafe, dt, speed);
        } else {
            player.walk(gen, fwd, strafe, dt);
        }
    }

    /// One frame of surface streaming from the current camera pose.
    pub fn surface_stream_frame(&mut self) {
        let pose = self.camera.pose;
        let viewer = crate::surface_stream::viewer_of_pub(pose);
        if let Some(s) = self.surface_stream.as_mut() {
            let _ = s.update(&self.ctx.device, viewer);
        }
    }

    /// Loads a raw u32 mesh (cave interiors ride the asset slot).
    pub fn load_u32_mesh(&mut self, verts: &[crate::scene::SceneVertex], idx: &[u32]) {
        self.assets.push((
            GpuMesh::from_mesh_u32(&self.ctx.device, verts, idx),
            idx.len() / 3,
        ));
    }

    /// Loads conforming-water vertices into the water pass (transparent,
    /// depth-read) — the NWR-005 surface-path water.
    pub fn load_water_vertices(&mut self, verts: &[crate::water::WaterVertex], idx: &[u16]) {
        use wgpu::util::DeviceExt;
        if self.water.is_none() {
            self.attach_water();
        }
        // Replace the section set with a single section holding this mesh.
        let vertex_buffer = self
            .ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("conforming water verts"),
                contents: bytemuck::cast_slice(verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = self
            .ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("conforming water indices"),
                contents: bytemuck::cast_slice(idx),
                usage: wgpu::BufferUsages::INDEX,
            });
        if let Some(w) = self.water.as_mut() {
            w.set_single_mesh(vertex_buffer, index_buffer, idx.len() as u32);
        }
    }

    /// Detaches the construction layer (control renders for block proofs).
    pub fn detach_construction(&mut self) {
        self.construction = None;
    }

    /// Detaches NPCs + boxes (control renders for presence proofs).
    pub fn detach_npcs(&mut self) {
        self.npcs = None;
    }

    /// Loads the NPC + anchor-box mesh (R3DV-009).
    pub fn load_npcs(&mut self, verts: &[crate::scene::SceneVertex], idx: &[u16]) {
        self.npcs = Some(GpuMesh::from_mesh(&self.ctx.device, verts, idx));
    }

    /// Loads the castle/city module mesh (R3DV-008) built from the
    /// placement authorities by `city::mesh_city`.
    pub fn load_city(&mut self, verts: &[crate::scene::SceneVertex], idx: &[u16]) {
        self.city = Some(GpuMesh::from_mesh(&self.ctx.device, verts, idx));
    }

    /// Attaches river water from flow records (R3DV-007).
    pub fn attach_water(&mut self) {
        if self.water.is_none() {
            self.water = Some(crate::water::WaterSections::new());
        }
    }

    /// Syncs water sections from the flow table (dirty-region remeshing;
    /// read-only world access).
    pub fn update_water(
        &mut self,
        gen: &pc3d_world::gen::WorldGen,
        graph: &pc3d_world::hydro::RiverGraph,
        table: &pc3d_world::flow::FlowTable,
    ) -> crate::water::WaterStats {
        let Some(w) = self.water.as_mut() else {
            panic!("call attach_water() before update_water()");
        };
        w.update(&self.ctx.device, gen, graph, table)
    }

    /// Detaches the water layer (control renders for transparency proofs).
    pub fn detach_water(&mut self) {
        self.water = None;
    }

    /// Freezes the water current pattern at a fixed time (deterministic
    /// proofs); None returns to wall-clock animation.
    pub fn set_water_time(&mut self, t: Option<f32>) {
        self.water_time_override = t;
    }

    /// Attaches streamed terrain (R3DV-006): the world's own interest rings
    /// and LOD bands drive bounded per-frame meshing and uploads.
    pub fn attach_streaming(
        &mut self,
        gen: std::rc::Rc<pc3d_world::gen::WorldGen>,
        cfg: crate::streaming::StreamConfig,
        y_level: i32,
    ) {
        self.streamer = Some(crate::streaming::TerrainStreamer::new(gen, cfg, y_level));
        self.terrain = None; // the streamer owns terrain drawing while attached
    }

    /// One frame of streaming work from the current camera pose.
    pub fn stream_frame(&mut self) -> Option<crate::streaming::StreamFrameStats> {
        let pose = self.camera.pose;
        let viewer = crate::streaming::viewer_of(pose);
        self.streamer
            .as_mut()
            .map(|s| s.update(&self.ctx.device, viewer))
    }

    /// The placed-asset record: total triangles currently drawn.
    pub fn asset_triangles(&self) -> usize {
        self.assets.iter().map(|(_, t)| t).sum()
    }

    /// Streaming counters (None when not attached).
    pub fn stream_counters(&self) -> Option<crate::streaming::StreamCounters> {
        self.streamer.as_ref().map(|s| s.counters())
    }

    /// Test hook: the streamer's current desired set.
    pub fn stream_desired_debug(
        &self,
    ) -> Option<std::collections::BTreeMap<pc3d_world::coords::PatchCoord, pc3d_world::lod::LodLevel>>
    {
        self.streamer.as_ref().map(|s| s.debug_desired())
    }

    /// Shows or hides the R3DV-002 placeholder scene (ground plane + dawn
    /// stones). Construction proofs hide it so host-built blocks are the
    /// only world geometry (nothing is coplanar with block bottoms).
    pub fn set_placeholder_scene(&mut self, visible: bool) {
        self.gpu_scene = if visible {
            GpuScene::new(&self.ctx.device)
        } else {
            GpuScene::empty(&self.ctx.device)
        };
    }

    /// Sets the HUD debug line (padded/truncated to a fixed width so the
    /// texture never needs reallocation).
    pub fn set_hud_line(&mut self, line: &str) {
        self.hud.line = format!("{:<width$}", line, width = HUD_LINE_CHARS);
        self.hud.anchor = HudAnchor::TopLeft;
    }

    pub fn set_hud_text_centered(&mut self, text: &str) {
        self.hud.line = text.to_string();
        self.hud.anchor = HudAnchor::Center;
    }

    /// Uploads (or clears) the owner-facing UI layer canvas. Straight-alpha
    /// RGBA, sized to the render target; the app only calls this when the
    /// UI actually changed so steady frames upload nothing.
    pub fn set_ui_layer(&mut self, canvas: Option<(Vec<u8>, u32, u32)>) {
        match canvas {
            Some((bytes, w, h)) => {
                assert_eq!(
                    bytes.len(),
                    (w * h * 4) as usize,
                    "ui canvas must be exactly w*h*4 bytes"
                );
                if self.ui_layer.as_ref().map(|l| l.size) != Some((w, h)) {
                    let texture = create_hud_texture(&self.ctx.device, w, h);
                    let bind_group = create_hud_bind_group(
                        &self.ctx.device,
                        &self.pipelines.layout_hud,
                        &self.globals_buf,
                        &texture,
                        ui_view_format(self.format),
                    );
                    let vertex_buffer = self.ctx.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("pc3d ui layer quad"),
                        size: 6 * 16,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    // Fullscreen quad, uv origin top-left (v grows downward
                    // exactly like the canvas rows). SIX vertices: the
                    // pipelines are TriangleList, so 4 vertices drew ONE
                    // triangle and everything below-right of the screen
                    // diagonal (top-right corner -> bottom-left corner)
                    // showed raw world — the owner's "samurai cut" across
                    // every menu, button, and text field.
                    let quad: [[f32; 4]; 6] = [
                        [-1.0, 1.0, 0.0, 0.0],
                        [1.0, 1.0, 1.0, 0.0],
                        [-1.0, -1.0, 0.0, 1.0],
                        [1.0, 1.0, 1.0, 0.0],
                        [1.0, -1.0, 1.0, 1.0],
                        [-1.0, -1.0, 0.0, 1.0],
                    ];
                    self.ctx.queue.write_buffer(
                        &vertex_buffer,
                        0,
                        bytemuck::cast_slice(&quad),
                    );
                    self.ui_layer = Some(UiLayer { texture, bind_group, vertex_buffer, size: (w, h), last_canvas: Vec::new() });
                }
                let layer = self.ui_layer.as_mut().expect("layer just ensured");
                layer.last_canvas = bytes.clone();
                // THE DIAGONAL-CUT FIX: wgpu requires a 256-byte-aligned
                // row pitch for texture uploads — raw w*4 shifts every
                // row progressively at unaligned window widths (the
                // owner's resized window tore diagonally; the proof
                // widths 1280/2560 were accidentally aligned). Stage the
                // rows into an aligned pitch.
                let pitch = (w * 4).div_ceil(256) * 256;
                let staged: Vec<u8> = if pitch == w * 4 {
                    bytes
                } else {
                    let mut v = Vec::with_capacity((pitch * h) as usize);
                    for row in 0..h {
                        let start = (row * w * 4) as usize;
                        v.extend_from_slice(&bytes[start..start + (w * 4) as usize]);
                        v.extend(std::iter::repeat(0u8).take((pitch - w * 4) as usize));
                    }
                    v
                };
                self.ctx.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &layer.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &staged,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(pitch),
                        rows_per_image: None,
                    },
                    wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                );
            }
            None => self.ui_layer = None,
        }
    }

    /// Whether a UI canvas is currently uploaded.
    pub fn has_ui_layer(&self) -> bool {
        self.ui_layer.is_some()
    }

    /// The last uploaded UI canvas (bytes, w, h) — proof pixel checks.
    pub fn ui_canvas_copy(&self) -> Option<(&[u8], u32, u32)> {
        self.ui_layer
            .as_ref()
            .map(|l| (l.last_canvas.as_slice(), l.size.0, l.size.1))
    }

    /// Vertical FOV in degrees (the settings screen drives this live).
    pub fn set_fov_y_deg(&mut self, deg: f32) {
        self.camera.fov_y_rad = deg.clamp(30.0, 120.0).to_radians();
    }

    pub fn size(&self) -> (u32, u32) {
        if let Some(guard) = &self.surface {
            (guard.config.width, guard.config.height)
        } else if let Some(tex) = &self.offscreen {
            (tex.width(), tex.height())
        } else {
            (0, 0)
        }
    }

    pub fn aspect(&self) -> f32 {
        let (w, h) = self.size();
        w as f32 / h.max(1) as f32
    }

    /// Reconfigure the active target for a new size (clamped to >= 1 px).
    pub fn resize(&mut self, width: u32, height: u32) {
        let (w, h) = crate::gpu::clamp_size(width, height);
        if let Some(guard) = self.surface.as_mut() {
            guard.resize(&self.ctx.device, w, h);
        } else if let Some(old) = self.offscreen.take() {
            if old.width() != w || old.height() != h {
                self.offscreen = Some(create_target(&self.ctx.device, w, h, self.format));
            } else {
                self.offscreen = Some(old);
            }
        }
        self.depth = Some(create_depth(&self.ctx.device, w, h));
    }

    /// Uploads camera + HUD state for this frame.
    fn prepare_frame(&mut self) {
        self.marker_log.clear();
        self.draw_calls = 0;
        let (w, h) = self.size();
        let aspect = self.aspect();
        let cam = &self.camera;
        let fwd = cam.fwd();
        let right = cam.right();
        let up = cam.up();
        let globals = Globals {
            view_proj: cam.view_proj(aspect),
            cam_pos: [
                cam.pose.position[0],
                cam.pose.position[1],
                cam.pose.position[2],
                1.0,
            ],
            fwd: [fwd[0], fwd[1], fwd[2], 0.0],
            right: [right[0], right[1], right[2], 0.0],
            up: [up[0], up[1], up[2], 0.0],
            sun_dir: [
                crate::scene::SUN_DIR[0],
                crate::scene::SUN_DIR[1],
                crate::scene::SUN_DIR[2],
                0.0,
            ],
            tan_aspect: [
                cam.tan_half_fov(),
                aspect,
                self.water_time_override
                    .unwrap_or_else(|| self.start.elapsed().as_secs_f32()),
                self.detail_flag,
            ],
        };
        self.ctx
            .queue
            .write_buffer(&self.globals_buf, 0, bytemuck::bytes_of(&globals));

        // Atmosphere (NWR-006): the sun's stable ortho box follows the
        // camera focus; every zero value keeps the legacy look.
        let atm = self.atmosphere;
        let focus = [
            cam.pose.position[0] + fwd[0] * 45.0,
            cam.pose.position[1] + fwd[1] * 45.0,
            cam.pose.position[2] + fwd[2] * 45.0,
        ];
        let lvp = crate::atmosphere::light_view_proj(
            focus,
            crate::scene::SUN_DIR,
            if atm.shadow_res > 0 {
                atm.shadow_half_m
            } else {
                1.0
            },
            atm.shadow_res.max(1),
            atm.shadow_res > 0,
        );
        let env = EnvGpu {
            light_view_proj: lvp,
            fog_color: [atm.fog_color[0], atm.fog_color[1], atm.fog_color[2], 1.0],
            params1: [
                atm.fog_density,
                if atm.shadow_res > 0 { 1.0 } else { 0.0 },
                crate::atmosphere::shadow_texel(atm.shadow_half_m, atm.shadow_res),
                atm.detail_strength,
            ],
            params2: [
                if atm.glint { 1.0 } else { 0.0 },
                0.0015,
                atm.detail_scale,
                atm.shadow_res as f32,
            ],
            params3: [
                if self.height_debug { 1.0 } else { 0.0 },
                if self.scene_debug != SceneDebugMode::Normal { 1.0 } else { 0.0 },
                0.0,
                0.0,
            ],
        };
        self.ctx
            .queue
            .write_buffer(&self.env_buf, 0, bytemuck::bytes_of(&env));

        // WT-002/003 slice 3: debug line buffers rebuild OUTSIDE any
        // encoder (buffers born mid-pass-recording proved unreliable on
        // this backend — the draws executed but rasterized nothing).
        if self.scene_debug != SceneDebugMode::Normal {
            if self.wire_dirty {
                let idx: Vec<u32> = (0..self.wire_edges.len() as u32).collect();
                self.wire_gpu = if self.wire_edges.is_empty() {
                    None
                } else {
                    Some(GpuMesh::from_mesh_u32(&self.ctx.device, &self.wire_edges, &idx))
                };
                self.wire_dirty = false;
            }
            if self.overlay_dirty {
                let edges = build_overlay_edges(&self.asset_anchors);
                let idx: Vec<u32> = (0..edges.len() as u32).collect();
                self.overlay_gpu = if edges.is_empty() {
                    None
                } else {
                    Some(GpuMesh::from_mesh_u32(&self.ctx.device, &edges, &idx))
                };
                self.overlay_dirty = false;
            }
        }

        // The crowd's pose rebuild per frame (NWR-009): time follows the
        // shared clock (frozen when water time is frozen — deterministic
        // proofs); positions always from the authoritative brains.
        if self.crowd.is_some() {
            let t = self
                .water_time_override
                .unwrap_or_else(|| self.start.elapsed().as_secs_f32());
            // Quantize to the pose rate (the Low lever): frozen clocks
            // still land on the same step — deterministic.
            let step = 1.0 / self.crowd_pose_hz;
            self.crowd_frame((t / step).floor() * step);
        }

        // HUD: re-rasterize the line and refresh the quad to the target size.
        let (bytes, tw, th) = crate::font::rasterize_text(&self.hud.line.clone(), HUD_SCALE);
        if (tw, th) != self.hud.size {
            self.hud.texture = create_hud_texture(&self.ctx.device, tw, th);
            self.hud.size = (tw, th);
            self.bg_hud = create_hud_bind_group(
                &self.ctx.device,
                &self.pipelines.layout_hud,
                &self.globals_buf,
                &self.hud.texture,
                ui_view_format(self.format),
            );
        }
        self.ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.hud.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(tw * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: tw,
                height: th,
                depth_or_array_layers: 1,
            },
        );
        let (x0, y1, x1, y0) = hud_quad_ndc(tw, th, w, h, self.hud.anchor);
        // Six vertices = two triangles (see set_ui_layer: TriangleList
        // discards the 4th vertex, cutting the quad along its diagonal).
        let quad: [[f32; 4]; 6] = [
            [x0, y1, 0.0, 0.0],
            [x1, y1, 1.0, 0.0],
            [x0, y0, 0.0, 1.0],
            [x1, y1, 1.0, 0.0],
            [x1, y0, 1.0, 1.0],
            [x0, y0, 0.0, 1.0],
        ];
        self.ctx
            .queue
            .write_buffer(&self.hud.vertex_buffer, 0, bytemuck::cast_slice(&quad));
    }

    /// Renders one frame. Windowed: acquire → draw → present, with the
    /// documented recovery policy for lost/outdated/timeout surfaces.
    /// Offscreen: draws into the proof target.
    pub fn render_frame(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.prepare_frame();
        if self.surface.is_some() {
            let mut attempt = 0;
            let output = loop {
                let err = match self.surface.as_ref().unwrap().surface.get_current_texture() {
                    Ok(t) => break t,
                    Err(e) => e,
                };
                attempt += 1;
                match surface_action(&err) {
                    SurfaceAction::RecreateRetry if attempt == 1 => {
                        let window = self.window.clone().expect("windowed renderer window");
                        self.surface.as_mut().unwrap().recreate(&self.ctx, &window);
                    }
                    SurfaceAction::ReconfigureRetry if attempt == 1 => {
                        self.surface.as_ref().unwrap().reconfigure(&self.ctx.device);
                    }
                    _ => return Err(err),
                }
            };
            let view = output.texture.create_view(&Default::default());
            let mut encoder =
                self.ctx
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("pc3d_render frame"),
                    });
            self.encode_frame(&mut encoder, &view);
            self.ctx.queue.submit(Some(encoder.finish()));
            output.present();
        } else {
            let tex = self.offscreen.as_ref().expect("renderer has no target");
            let view = tex.create_view(&Default::default());
            let mut encoder =
                self.ctx
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("pc3d_render offscreen frame"),
                    });
            self.encode_frame(&mut encoder, &view);
            self.ctx.queue.submit(Some(encoder.finish()));
        }
        Ok(())
    }

    /// Renders a frame and saves it as a PNG, then verifies it against
    /// caller-supplied semantic probes. Windowed: the copy comes from the
    /// actual swapchain texture (the surface carries COPY_SRC), so this is a
    /// screenshot of the presented frame, not a separate render.
    pub fn capture_png(&mut self, path: &Path, probes: &[Probe]) -> (PixelReport, Vec<u8>) {
        self.prepare_frame();
        let (width, height) = self.size();
        assert!(width >= 1 && height >= 1, "cannot capture a 0-sized frame");

        let bytes_per_row = (width * 4).div_ceil(256) * 256;
        let readback = self.ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pc3d_render readback"),
            size: (bytes_per_row * height) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("pc3d_render capture"),
            });

        if self.surface.is_some() {
            let mut attempt = 0;
            let output = loop {
                let err = match self.surface.as_ref().unwrap().surface.get_current_texture() {
                    Ok(t) => break t,
                    Err(e) => e,
                };
                attempt += 1;
                match surface_action(&err) {
                    SurfaceAction::RecreateRetry if attempt == 1 => {
                        let window = self.window.clone().expect("windowed renderer window");
                        self.surface.as_mut().unwrap().recreate(&self.ctx, &window);
                    }
                    SurfaceAction::ReconfigureRetry if attempt == 1 => {
                        self.surface.as_ref().unwrap().reconfigure(&self.ctx.device);
                    }
                    _ => panic!("surface unavailable for capture: {err:?}"),
                }
            };
            let view = output.texture.create_view(&Default::default());
            self.encode_frame(&mut encoder, &view);
            encoder.push_debug_group("pc3d.pass.screenshot_readback");
            self.marker_log.push("pc3d.pass.screenshot_readback");
            encoder.copy_texture_to_buffer(
                output.texture.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer: &readback,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: None,
                    },
                },
                output.texture.size(),
            );
            self.ctx.queue.submit(Some(encoder.finish()));
            // Wait for the copy before presenting, which may recycle the
            // swapchain texture.
            let _ = self.ctx.device.poll(wgpu::Maintain::Wait);
            output.present();
        } else {
            let tex = self.offscreen.take().expect("renderer has no target");
            let view = tex.create_view(&Default::default());
            self.encode_frame(&mut encoder, &view);
            encoder.push_debug_group("pc3d.pass.screenshot_readback");
            self.marker_log.push("pc3d.pass.screenshot_readback");
            encoder.copy_texture_to_buffer(
                tex.as_image_copy(),
                wgpu::ImageCopyBuffer {
                    buffer: &readback,
                    layout: wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: None,
                    },
                },
                tex.size(),
            );
            self.ctx.queue.submit(Some(encoder.finish()));
            let _ = self.ctx.device.poll(wgpu::Maintain::Wait);
            self.offscreen = Some(tex);
        }

        let rgba = read_buffer(&self.ctx.device, &readback, bytes_per_row, width, height);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).expect("create screenshot directory");
            }
        }
        image::save_buffer(path, &rgba, width, height, image::ColorType::Rgba8)
            .expect("write screenshot png");
        let report = crate::scene::verify_frame_rgba(&rgba, width, height, probes);
        (report, rgba)
    }

    /// The single draw path shared by the window, the live-window screenshot,
    /// and the offscreen proof target.
    fn encode_frame(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        // WT-007 slice 1: the frame's marker tree — vendor captures
        // (AMD RGP / NVIDIA Nsight) correlate work by these names.
        encoder.push_debug_group("pc3d.frame");
        self.marker_log.push("pc3d.frame");
        // 0. Sun shadow pass (NWR-006): depth-only from the light's ortho
        // box (env.light_view_proj, written in prepare_frame). The whole
        // opaque world draws; the streamers cull with the LIGHT matrix.
        let light_vp: [f32; 16] = {
            let atm = self.atmosphere;
            let cam = &self.camera;
            let f = cam.fwd();
            crate::atmosphere::light_view_proj(
                [
                    cam.pose.position[0] + f[0] * 45.0,
                    cam.pose.position[1] + f[1] * 45.0,
                    cam.pose.position[2] + f[2] * 45.0,
                ],
                crate::scene::SUN_DIR,
                if atm.shadow_res > 0 {
                    atm.shadow_half_m
                } else {
                    1.0
                },
                atm.shadow_res.max(1),
                atm.shadow_res > 0,
            )
        };
        encoder.push_debug_group("pc3d.frame.prepare");
        self.marker_log.push("pc3d.frame.prepare");
        if self.atmosphere.shadow_res > 0 {
            let mut spass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pc3d sun shadow pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            spass.set_pipeline(&self.pipelines.shadow);
            spass.set_bind_group(0, &self.bg_light, &[]);
            if self.gpu_scene.index_count > 0 {
                spass.set_vertex_buffer(0, self.gpu_scene.vertex_buffer.slice(..));
                spass.set_index_buffer(
                    self.gpu_scene.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint16,
                );
                spass.draw_indexed(0..self.gpu_scene.index_count, 0, 0..1);
            }
            if let Some(s) = self.surface_stream.as_mut() {
                s.draw(&mut spass, &light_vp);
            }
            if let Some(streamer) = self.streamer.as_mut() {
                streamer.draw(&mut spass, &light_vp);
            } else if let Some(t) = &self.terrain {
                t.draw(&mut spass);
            }
            if let Some(con) = &self.construction {
                con.draw(&mut spass);
            }
            if let Some(c) = &self.city {
                c.draw(&mut spass);
            }
            if let Some(n) = &self.npcs {
                n.draw(&mut spass);
            }
            for (a, _) in &self.assets {
                a.draw(&mut spass);
            }
            // Foliage casts a SOLID shadow (depth-only has no mask test —
            // a documented stylized choice).
            if let Some((m, _)) = &self.cutout {
                spass.set_pipeline(&self.pipelines.shadow_cutout);
                m.draw(&mut spass);
            }
            // The settlement kit casts shadows (NWR-008).
            if let Some(st) = self.settlement.as_mut() {
                st.draw_shadow(&mut spass, &self.pipelines.flora, &self.bg_light);
            }
            // Wilderness instances cast shadows too (NWR-007): the
            // placement update runs here so the shadow and color passes
            // agree even mid-stream.
            if let (Some(f), Some(g)) = (self.flora.as_mut(), self.flora_gen.clone()) {
                let vp = self.camera.pose.position;
                f.update(&g, [vp[0], vp[2]]);
                f.upload(&g, &self.ctx.device, [vp[0], vp[2]]);
                f.draw_shadow(&mut spass, &self.pipelines.flora, &self.bg_light);
            }
        }
        // The prepare group closes before the main pass: sibling passes.
        encoder.pop_debug_group();
        let depth_view = self
            .depth
            .as_ref()
            .expect("depth attachment missing")
            .create_view(&Default::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pc3d_render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.13,
                        g: 0.27,
                        b: 0.42,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        // 1. Sky: world-ray gradient + sun (no depth interaction).
        self.mark(&mut pass, "pc3d.pass.sky_atmosphere");
        pass.set_pipeline(&self.pipelines.sky);
        pass.set_bind_group(0, &self.bg_globals, &[]);
        pass.draw(0..3, 0..1);
        self.draw_calls += 1;
        self.unmark(&mut pass);
        self.mark(&mut pass, "pc3d.pass.terrain");
        // 2. Depth-tested, sunlit world geometry from P3D coordinates.
        // The mesh pipeline binds once; the placeholder scene draws only
        // when present (hidden construction runs have empty buffers, and
        // wgpu refuses zero-size slices), then construction patches draw
        // through the same lit pipeline.
        pass.set_pipeline(&self.pipelines.mesh);
        pass.set_bind_group(0, &self.bg_globals, &[]);
        if self.gpu_scene.index_count > 0 {
            pass.set_vertex_buffer(0, self.gpu_scene.vertex_buffer.slice(..));
            pass.set_index_buffer(
                self.gpu_scene.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            pass.draw_indexed(0..self.gpu_scene.index_count, 0, 0..1);
        }
        // 2a. Natural terrain: the SURFACE stream first (NWR-004 — the
        // ordinary path), then the legacy cube stream/static load.
        let view_proj: [f32; 16] = self.camera.view_proj(self.aspect());
        if let Some(s) = self.surface_stream.as_mut() {
            s.draw(&mut pass, &view_proj);
        }
        if let Some(streamer) = self.streamer.as_mut() {
            streamer.draw(&mut pass, &view_proj);
        } else if let Some(t) = &self.terrain {
            t.draw(&mut pass);
        }
        // 2b. Host-owned construction blocks (same lit pipeline; per-patch
        // buffers with content versions — only changed patches are remeshed).
        if let Some(con) = &self.construction {
            con.draw(&mut pass);
        }
        // 2b2. Castle/city modules (opaque, same lit pipeline).
        if let Some(c) = &self.city {
            c.draw(&mut pass);
        }
        self.unmark(&mut pass);
        // 2b3. NPCs + inspect anchor boxes.
        self.mark(&mut pass, "pc3d.pass.npcs");
        if let Some(n) = &self.npcs {
            n.draw(&mut pass);
            self.draw_calls += 1;
        }
        // WT-007: machines are a SIM-side system (no GPU pass yet) — the
        // marker slot exists so captures and audits stay comparable.
        self.mark(&mut pass, "pc3d.pass.machines");
        self.unmark(&mut pass);
        // 2b4. GLB assets (NWR-002).
        self.mark(&mut pass, "pc3d.pass.assets");
        for (a, _) in &self.assets {
            a.draw(&mut pass);
            self.draw_calls += 1;
        }
        // 2b4a. WT-002/003 slice 3: debug wireframe / anchor overlay —
        // the world is dimmed via params3.y, the lines draw sun-aligned
        // bright through the same lit path on LineList.
        self.mark(&mut pass, "pc3d.pass.wireframe_overlay");
        if self.scene_debug != SceneDebugMode::Normal {
            if self.scene_debug == SceneDebugMode::Wireframe {
                if let Some(w) = &self.wire_gpu {
                    pass.set_pipeline(&self.pipelines.wire);
                    w.draw(&mut pass);
                    self.draw_calls += 1;
                }
            } else if let Some(o) = &self.overlay_gpu {
                pass.set_pipeline(&self.pipelines.wire);
                o.draw(&mut pass);
                self.draw_calls += 1;
            }
        }
        self.unmark(&mut pass);
        self.unmark(&mut pass);
        // 2b4b..2b4d: instanced world content (flora, crowd, settlement
        // kit) — the asset-instances pass continuation.
        self.mark(&mut pass, "pc3d.pass.assets");
        if let (Some(f), Some(g)) = (self.flora.as_mut(), self.flora_gen.clone()) {
            let vp = self.camera.pose.position;
            f.update(&g, [vp[0], vp[2]]);
            f.upload(&g, &self.ctx.device, [vp[0], vp[2]]);
            f.draw(&mut pass, &self.pipelines.flora, &self.bg_globals);
            self.draw_calls += 1;
        }
        // 2b4d. The NPC crowd (NWR-009): rigged boxes per part color.
        if let Some(crowd) = self.crowd.as_mut() {
            crowd.draw(&mut pass, &self.pipelines.flora, &self.bg_globals);
            self.draw_calls += 1;
        }
        // 2b4c. The settlement kit (NWR-008): instanced modules + the
        // D-033 anchor markers.
        if let Some(st) = self.settlement.as_mut() {
            st.draw(&mut pass, &self.pipelines.flora, &self.bg_globals);
            st.draw_anchors(&mut pass, &self.pipelines.mesh, &self.bg_globals);
            self.draw_calls += 2;
        }
        self.unmark(&mut pass);
        // 2b5. Alpha-cutout foliage (NWR-006): mask-tested, depth-writing,
        // opaque blend — order-independent and Deck-cheap.
        if let Some((m, bg_mask)) = &self.cutout {
            pass.set_pipeline(&self.pipelines.cutout);
            pass.set_bind_group(0, &self.bg_globals, &[]);
            pass.set_bind_group(1, bg_mask, &[]);
            m.draw(&mut pass);
        }
        // 2c. Transparent river water LAST among world geometry: depth-read
        // only, alpha blend — banks show through, terrain occludes.
        self.mark(&mut pass, "pc3d.pass.water");
        if let Some(w) = &self.water {
            pass.set_pipeline(&self.pipelines.water);
            pass.set_bind_group(0, &self.bg_globals, &[]);
            w.draw(&mut pass);
            self.draw_calls += 1;
        }
        self.unmark(&mut pass);
        // 3. HUD debug line + 4. the owner UI layer — one marker group.
        self.mark(&mut pass, "pc3d.pass.ui");
        if !self.hud.line.trim().is_empty() {
            pass.set_pipeline(&self.pipelines.hud);
            pass.set_bind_group(0, &self.bg_hud, &[]);
            pass.set_vertex_buffer(0, self.hud.vertex_buffer.slice(..));
            pass.draw(0..6, 0..1);
            self.draw_calls += 1;
        }
        // 4. The owner-facing UI layer (GLM UI rework): one fullscreen
        // alpha-blended quad over everything — panels, bars, hotbar, menus.
        if let Some(ui) = &self.ui_layer {
            pass.set_pipeline(&self.pipelines.ui);
            pass.set_bind_group(0, &ui.bind_group, &[]);
            pass.set_vertex_buffer(0, ui.vertex_buffer.slice(..));
            pass.draw(0..6, 0..1);
            self.draw_calls += 1;
        }
        self.unmark(&mut pass);
        drop(pass);
        // Close the marker tree's root group.
        encoder.pop_debug_group();
    }
}

/// The uploaded crowd: per part color, a colored box mesh + its
/// instance buffer (a whole crowd is one draw per color, <= ~8 draws).
struct CrowdGpu {
    /// (vertices, indices, instances, index count, instance count)
    buckets: Vec<(wgpu::Buffer, wgpu::Buffer, wgpu::Buffer, u32, u32)>,
    pub draws: usize,
    pub instances: usize,
}

impl CrowdGpu {
    fn new(_device: &wgpu::Device) -> Self {
        Self {
            buckets: Vec::new(),
            draws: 0,
            instances: 0,
        }
    }

    fn upload(
        &mut self,
        device: &wgpu::Device,
        rows: &std::collections::BTreeMap<crate::npcs::PartColor, Vec<crate::npcs::BoxInstance>>,
    ) {
        use wgpu::util::DeviceExt;
        // One colored box mesh per present part color.
        let (verts, idx) = crate::npcs::unit_box_mesh();
        let mut buckets = Vec::new();
        let mut instances = 0usize;
        for (color, list) in rows {
            if list.is_empty() {
                continue;
            }
            let colored: Vec<SceneVertex> = verts
                .iter()
                .map(|v| SceneVertex {
                    pos: v.pos,
                    normal: v.normal,
                    color: color.albedo(),
                })
                .collect();
            let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("crowd box vertices"),
                contents: bytemuck::cast_slice(&colored),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("crowd box indices"),
                contents: bytemuck::cast_slice(&idx),
                usage: wgpu::BufferUsages::INDEX,
            });
            let inst = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("crowd instances"),
                contents: bytemuck::cast_slice(list),
                usage: wgpu::BufferUsages::VERTEX,
            });
            instances += list.len();
            buckets.push((vb, ib, inst, idx.len() as u32, list.len() as u32));
        }
        self.buckets = buckets;
        self.instances = instances;
    }

    fn draw<'rp>(
        &mut self,
        pass: &mut wgpu::RenderPass<'rp>,
        pipelines: &FloraPipelines,
        bg_globals: &wgpu::BindGroup,
    ) {
        pass.set_pipeline(&pipelines.inst_box);
        pass.set_bind_group(0, bg_globals, &[]);
        let mut draws = 0usize;
        for (vb, ib, inst, ic, n) in &self.buckets {
            if *n == 0 {
                continue;
            }
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_vertex_buffer(1, inst.slice(..));
            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..*ic, 0, 0..*n);
            draws += 1;
        }
        self.draws = draws;
    }
}

/// Default player spawn: 1.7 m eye height south of the near stone, facing
/// north (P3D world coordinates).
fn default_pose() -> CameraPose {
    crate::scene::pose_a()
}

fn default_hud_line(camera: &Camera) -> String {
    let p = camera.pose.position;
    format!(
        "P3D POS {:.1} {:.1} {:.1} YAW {:.2} FPS 0",
        p[0], p[1], p[2], camera.pose.yaw
    )
}

fn create_globals_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pc3d globals"),
        size: std::mem::size_of::<Globals>() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_globals_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buf: &wgpu::Buffer,
    detail: &wgpu::Texture,
    env_buf: &wgpu::Buffer,
    shadow_view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("pc3d detail sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("pc3d shadow comparison sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        compare: Some(wgpu::CompareFunction::LessEqual),
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("pc3d globals bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(
                    &detail.create_view(&Default::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: env_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(&shadow_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::TextureView(shadow_view),
            },
        ],
    })
}

/// The env uniform buffer (NWR-006).
fn create_env_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pc3d env uniform"),
        size: std::mem::size_of::<EnvGpu>() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

/// The sun's depth map (NWR-006). res 0 builds the 1x1 cleared dummy the
/// layout needs when shadows are off — sampling it compares against 1.0
/// depth, i.e. fully lit, so the shader path stays uniform.
fn create_shadow_map(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    res: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let (w, h) = if res == 0 { (1, 1) } else { (res, res) };
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pc3d sun shadow map"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = tex.create_view(&Default::default());
    // Clear once so no undefined memory is ever sampled (the dummy is
    // never rendered into again).
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pc3d shadow clear"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }
    queue.submit(Some(encoder.finish()));
    (tex, view)
}

/// The material-detail atlas (NWR-006): 256x64 RGBA, generated from the
/// pc3d_assets metadata — replaces the old single-tile noise (the legacy
/// detail-flag path now reads atlas tile 0).
fn create_atlas_texture(device: &wgpu::Device) -> (wgpu::Texture, Vec<u8>) {
    let data = crate::atmosphere::material_atlas_rgba(&pc3d_assets::DETAIL_ATLAS_SPECS);
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pc3d material atlas"),
        size: wgpu::Extent3d {
            width: crate::atmosphere::ATLAS_TILE_PX * crate::atmosphere::ATLAS_TILES as u32,
            height: crate::atmosphere::ATLAS_TILE_PX,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    (tex, data)
}

fn upload_detail(queue: &wgpu::Queue, tex: &wgpu::Texture, data: &[u8]) {
    let w = crate::atmosphere::ATLAS_TILE_PX * crate::atmosphere::ATLAS_TILES as u32;
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w * 4),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: w,
            height: crate::atmosphere::ATLAS_TILE_PX,
            depth_or_array_layers: 1,
        },
    );
}

/// Quality tiers (R3DV-010).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QualityTier {
    Low,
    Mid,
    High,
}

impl QualityTier {
    /// Streaming rings: low keeps only the full ring, mid adds the lod
    /// ring, high adds the macro ring.
    pub fn tiers(self) -> &'static [pc3d_world::stream::Tier] {
        use pc3d_world::stream::Tier;
        match self {
            QualityTier::Low => &[Tier::Full],
            QualityTier::Mid => &[Tier::Full, Tier::Lod],
            QualityTier::High => &[Tier::Full, Tier::Lod, Tier::Macro],
        }
    }
    pub fn max_mesh_per_frame(self) -> usize {
        match self {
            QualityTier::Low => 1,
            QualityTier::Mid => 2,
            QualityTier::High => 3,
        }
    }
    pub fn gpu_byte_budget(self) -> usize {
        match self {
            QualityTier::Low => 8 * 1024 * 1024,
            QualityTier::Mid => 24 * 1024 * 1024,
            QualityTier::High => 48 * 1024 * 1024,
        }
    }
    /// Detail texture sampling at high tier only.
    pub fn detail(self) -> f32 {
        match self {
            QualityTier::High => 1.0,
            _ => 0.0,
        }
    }
}

fn create_hud(device: &wgpu::Device, _layout: &wgpu::BindGroupLayout) -> HudResources {
    let (_, tw, th) = crate::font::rasterize_line(&" ".repeat(HUD_LINE_CHARS), HUD_SCALE);
    let texture = create_hud_texture(device, tw, th);
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pc3d hud quad"),
        size: 6 * 16,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    HudResources {
        texture,
        size: (tw, th),
        vertex_buffer,
        line: String::new(),
        anchor: HudAnchor::TopLeft,
    }
}

fn create_hud_texture(device: &wgpu::Device, w: u32, h: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pc3d hud texture"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        // The bind group reinterprets the view as sRGB on sRGB targets
        // (ui_view_format); the base texture must permit that view.
        view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
    })
}

fn create_hud_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    globals_buf: &wgpu::Buffer,
    texture: &wgpu::Texture,
    view_format: wgpu::TextureFormat,
) -> wgpu::BindGroup {
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("pc3d hud sampler"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("pc3d hud bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(
                    // The view picks the color space: an sRGB target needs
                    // an sRGB view so the canvas's sRGB bytes decode on
                    // sample and re-encode on store — byte-exact on screen.
                    &texture.create_view(&wgpu::TextureViewDescriptor {
                        format: Some(view_format),
                        ..Default::default()
                    }),
                ),
            },
        ],
    })
}

/// The UI/HUD canvas is authored as sRGB bytes; the view must linearize
/// them exactly when the render target is sRGB (both real surfaces are —
/// gpu.rs deliberately prefers sRGB). On a linear target the plain view
/// is already byte-exact.
fn ui_view_format(target: wgpu::TextureFormat) -> wgpu::TextureFormat {
    if target.is_srgb() {
        wgpu::TextureFormat::Rgba8UnormSrgb
    } else {
        wgpu::TextureFormat::Rgba8Unorm
    }
}

/// Top-left pixel-anchored HUD quad in NDC.
fn hud_quad_ndc(
    tw: u32,
    th: u32,
    target_w: u32,
    target_h: u32,
    anchor: HudAnchor,
) -> (f32, f32, f32, f32) {
    let margin = 8.0;
    let w = tw as f32 * 2.0 / target_w as f32;
    let h = th as f32 * 2.0 / target_h as f32;
    match anchor {
        HudAnchor::TopLeft => {
            let x0 = -1.0 + margin * 2.0 / target_w as f32;
            let y1 = 1.0 - margin * 2.0 / target_h as f32;
            (x0, y1, x0 + w, y1 - h)
        }
        HudAnchor::Center => {
            let x0 = -w * 0.5;
            let y1 = h * 0.5;
            (x0, y1, x0 + w, y1 - h)
        }
    }
}

fn create_depth(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pc3d depth"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth24Plus,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

fn create_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pc3d_render target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// Maps the readback buffer and strips row padding into a tight RGBA image.
fn read_buffer(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    bytes_per_row: u32,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |res| {
        tx.send(res).expect("map callback send");
    });
    let _ = device.poll(wgpu::Maintain::Wait);
    rx.recv()
        .expect("map callback result")
        .expect("map readback buffer");

    let data = buffer.slice(..).get_mapped_range();
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        let start = (row * bytes_per_row) as usize;
        rgba.extend_from_slice(&data[start..start + (width * 4) as usize]);
    }
    drop(data);
    buffer.unmap();

    // Swapchain/offscreen targets are BGRA; normalize to RGBA for `image`
    // and the pixel verifier.
    for px in rgba.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    rgba
}

// ---------------------------------------------------------------------------
// Tests: real GPU 3D frames through the same draw path the window uses.
// The three proofs here are impossible without camera + projection + depth.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{
        marker_hue_pixels, pixel_difference_fraction, pose_a, pose_b, pose_c, probes_for_pose,
    };

    const W: u32 = 384;
    const H: u32 = 288;
    const ASPECT: f32 = 384.0 / 288.0;

    /// A hand-built indexed unit cube (8 SHARED vertices, 12 triangles):
    /// the ground truth for edge dedup.
    fn unit_cube() -> (Vec<crate::scene::SceneVertex>, Vec<u32>) {
        let p: [[f32; 3]; 8] = [
            [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0],
        ];
        let verts = p
            .iter()
            .map(|v| crate::scene::SceneVertex { pos: *v, normal: [0.0, 1.0, 0.0], color: [1.0, 1.0, 1.0] })
            .collect();
        let idx: Vec<u32> = vec![
            0, 1, 2, 0, 2, 3, // -Z
            4, 6, 5, 4, 7, 6, // +Z
            0, 4, 5, 0, 5, 1, // -Y
            2, 6, 7, 2, 7, 3, // +Y
            0, 3, 7, 0, 7, 4, // -X
            1, 5, 6, 1, 6, 2, // +X
        ];
        (verts, idx)
    }

    #[test]
    fn wire_edge_extraction_dedupes_shared_edges() {
        // A triangulated cube has 18 unique edges: 12 cube edges (each
        // shared by 2 of the 12 triangles) + 6 face diagonals (shared by
        // the 2 triangles of their face) — deduped from 36 instances.
        let (verts, idx) = unit_cube();
        let edges = extract_wire_edges(&verts, &idx, [1.0, 0.8, 0.3]);
        assert_eq!(edges.len(), 36, "18 edges = 36 line vertices");
        // Every line vertex faces the sun (the brightness law).
        for v in &edges {
            assert_eq!(v.normal, crate::scene::SUN_DIR);
        }
    }

    #[test]
    fn overlay_builder_draws_crosses_and_bounds_boxes() {
        let anchors = [AssetAnchors {
            glb_id: "test".into(),
            placement: [0.0, 0.0, 0.0],
            sockets: vec![
                ("door".into(), [1.0, 0.0, 0.0]),
                ("interior".into(), [0.0, 0.5, 0.0]),
            ],
            bounds_min: [-1.0, 0.0, -1.0],
            bounds_max: [1.0, 2.0, 1.0],
        }];
        let edges = build_overlay_edges(&anchors);
        // 2 sockets x 3 axes x 2 verts + 12 box edges x 2 verts.
        assert_eq!(edges.len(), 12 + 24, "crosses + bounds box");
        // The ember X-axis color must be present (the marker law the
        // capture pixel-checks look for).
        assert!(
            edges.iter().any(|v| v.color == [1.0, 0.55, 0.15]),
            "ember axis markers required"
        );
    }

    #[test]
    fn marker_tree_covers_the_contract_after_a_capture() {
        // WT-007 slice 1: one encoded frame must push every required
        // pc3d.* debug group (the vendor-capture correlation tree), and
        // the audit must carry every gpu_marker_contract field.
        let mut r = Renderer::offscreen(W, H);
        r.set_pose(pose_a());
        let probes = probes_for_pose(pose_a(), ASPECT);
        let path = std::env::temp_dir().join("pc3d_marker_proof.png");
        let _ = r.capture_png(&path, &probes);
        for required in [
            "pc3d.frame",
            "pc3d.frame.prepare",
            "pc3d.pass.terrain",
            "pc3d.pass.sky_atmosphere",
            "pc3d.pass.water",
            "pc3d.pass.assets",
            "pc3d.pass.npcs",
            "pc3d.pass.machines",
            "pc3d.pass.wireframe_overlay",
            "pc3d.pass.ui",
            "pc3d.pass.screenshot_readback",
        ] {
            assert!(
                r.marker_tree().contains(&required),
                "marker {required} missing from {:?}",
                r.marker_tree()
            );
        }
        let audit = r.gpu_marker_audit(6.0);
        for field in [
            "build_hash", "api_backend", "adapter_name", "markers_present",
            "timestamp_support", "cpu_frame_ms", "gpu_frame_ms_or_reason",
            "draw_calls", "triangles", "material_buckets",
        ] {
            assert!(audit.get(field).is_some(), "audit field {field} missing");
        }
        assert_eq!(audit["markers_present"], serde_json::json!(true));
        // The fresh offscreen renderer's world may be empty (only the sky
        // draw is unconditional) — the law is that counting is LIVE,
        // and the windowed audit route below carries the real totals.
        assert!(audit["draw_calls"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn scene_debug_modes_dim_the_world_and_stage_the_overlay_data() {
        // The offscreen laws: loading an asset retains its edge + anchor
        // data (the builders' inputs), and the debug flag measurably
        // changes the presented frame (the world dims through params3.y).
        // The VISUAL line/marker law is proven by the WINDOWED
        // --asset-capture arm (the offscreen readback proved unreliable
        // for late-pass draws on this backend: even a hardcoded
        // fullscreen draw after the asset pass painted nothing).
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/compiled/module/house_croft.glb");
        let asset = crate::glb::load_asset_file(&path).expect("house croft glb");
        let mut r = Renderer::offscreen(W, H);
        r.set_pose(CameraPose::new([9.0, 6.0, 11.0], -2.4, -0.35));
        r.load_asset(&asset, "lod0", [0.0, 0.0, 0.0]);

        // Data laws: the wire edges exist, are sun-aligned, and the
        // overlay builder yields crosses + bounds from the anchors.
        assert!(!r.wire_edges.is_empty(), "load must retain wire edges");
        assert!(r
            .wire_edges
            .iter()
            .all(|v| v.normal == crate::scene::SUN_DIR));
        assert_eq!(r.asset_anchors.len(), 1);
        assert!(r.asset_anchors[0].sockets.len() >= 4, "house sockets");
        let overlay = build_overlay_edges(&r.asset_anchors);
        assert!(
            overlay.len() >= 12 + 24,
            "crosses + bounds box vertices ({})",
            overlay.len()
        );

        // Frame law: debug mode dims the world (params3.y reaches the
        // shader through the offscreen path — the sRGB-encoded dim is
        // byte-visible as a large dark region).
        let probes: &[crate::scene::Probe] = &[];
        let out = std::env::temp_dir().join("pc3d_wire_proof");
        std::fs::create_dir_all(&out).unwrap();
        let (_rn, rgba_normal) = r.capture_png(&out.join("normal.png"), probes);
        r.set_scene_debug(SceneDebugMode::Wireframe);
        let (_rw, rgba_wire) = r.capture_png(&out.join("wire.png"), probes);
        let d = pixel_difference_fraction(&rgba_normal, &rgba_wire);
        assert!(d > 0.02, "debug mode must change the frame ({d})");
        let dark = rgba_wire
            .chunks_exact(4)
            .filter(|p| p[0] < 100 && p[1] < 100 && p[2] < 100)
            .count();
        assert!(dark > 2000, "the dimmed world must darken ({dark} px)");
    }

    fn capture_at(path: &Path, pose: CameraPose) -> (PixelReport, Vec<u8>) {
        let mut r = Renderer::offscreen(W, H);
        r.set_pose(pose);
        let probes = probes_for_pose(pose, ASPECT);
        r.capture_png(path, &probes)
    }

    #[test]
    fn face_flip_and_scene_probes_hold_from_two_poses() {
        // POSE A (south, looking north): center must be the crimson face.
        let path_a = std::env::temp_dir().join("pc3d_3d_pose_a.png");
        let (report_a, rgba_a) = capture_at(&path_a, pose_a());
        assert!(
            report_a.passes(),
            "pose A failed: {:?}",
            report_a.failed_probes()
        );
        // The marker stone behind the near stone must be FULLY occluded:
        // zero amber pixels anywhere in the frame (scanned over the decoded
        // RGBA, never the compressed PNG bytes).
        assert_eq!(
            marker_hue_pixels(&rgba_a),
            0,
            "occluded marker stone leaked pixels at pose A"
        );

        // POSE B (east, looking west): the SAME screen region must now be
        // the gold face — a 2D image cannot satisfy both probes.
        let path_b = std::env::temp_dir().join("pc3d_3d_pose_b.png");
        let (report_b, rgba_b) = capture_at(&path_b, pose_b());
        assert!(
            report_b.passes(),
            "pose B failed: {:?}",
            report_b.failed_probes()
        );

        // PARALLAX: the two frames must differ across a large fraction of
        // pixels — different viewpoints of a 3D world.
        let diff = pixel_difference_fraction(&rgba_a, &rgba_b);
        println!(
            "parallax: {:.1}% of pixels differ between poses A and B",
            diff * 100.0
        );
        assert!(diff > 0.15, "poses A and B frames barely differ ({diff})");
    }

    #[test]
    fn occluded_marker_becomes_visible_from_pose_c() {
        // Pose A: zero marker pixels (proven in the face-flip test's scan).
        // Pose C (southeast, looking at the marker): many marker pixels.
        let path_c = std::env::temp_dir().join("pc3d_3d_pose_c.png");
        let (report_c, rgba_c) = capture_at(&path_c, pose_c());
        assert!(
            report_c.passes(),
            "pose C failed: {:?}",
            report_c.failed_probes()
        );
        let visible = marker_hue_pixels(&rgba_c);
        println!("marker pixels visible at pose C: {visible}");
        assert!(visible > 300, "marker stone barely visible from pose C");
    }

    #[test]
    fn same_pose_renders_byte_identical_frames() {
        let path_a = std::env::temp_dir().join("pc3d_3d_det_a.png");
        let path_b = std::env::temp_dir().join("pc3d_3d_det_b.png");
        let (report_a, rgba_a) = capture_at(&path_a, pose_a());
        let (report_b, rgba_b) = capture_at(&path_b, pose_a());
        assert_eq!(report_a, report_b);
        assert_eq!(rgba_a, rgba_b, "same camera pose produced different pixels");
        assert_eq!(
            std::fs::read(&path_a).unwrap(),
            std::fs::read(&path_b).unwrap(),
            "encoded PNGs differ for identical frames"
        );
    }

    #[test]
    fn offscreen_resize_reconfigures_target_and_depth() {
        let mut r = Renderer::offscreen(W, H);
        r.resize(256, 192);
        assert_eq!(r.size(), (256, 192));
        let path = std::env::temp_dir().join("pc3d_3d_resized.png");
        let (report, _) = r.capture_png(&path, &probes_for_pose(pose_a(), 256.0 / 192.0));
        assert_eq!(report.width, 256);
        assert!(
            report.passes(),
            "resized frame failed: {:?}",
            report.failed_probes()
        );
        // Zero-sized resize must clamp, not poison the target.
        r.resize(0, 0);
        assert_eq!(r.size(), (1, 1));
    }

    #[test]
    fn ui_layer_quad_covers_the_entire_target() {
        // THE SAMURAI-CUT LAW: the UI layer quad drawn as ONE TriangleList
        // triangle left everything below-right of the screen diagonal
        // showing raw world — every menu, button, and text field was
        // sliced while canvas-side checks stayed green. An opaque
        // fullscreen canvas must survive the composite EXACTLY at all
        // four corners and the right/bottom edges: the sRGB view decodes
        // the canvas bytes on sample and the sRGB target re-encodes them
        // on store, so alpha-over with a=255 is byte-identical — any
        // other color is a missing triangle or a gamma break.
        let mut r = Renderer::offscreen(W, H);
        r.set_pose(pose_a());
        let (cw, ch) = r.size();
        let color = [12u8, 34, 56, 255];
        let mut canvas = Vec::with_capacity((cw * ch * 4) as usize);
        for _ in 0..cw * ch {
            canvas.extend_from_slice(&color);
        }
        r.set_ui_layer(Some((canvas, cw, ch)));
        let path = std::env::temp_dir().join("pc3d_3d_ui_quad_cover.png");
        let (_report, rgba) = r.capture_png(&path, &probes_for_pose(pose_a(), ASPECT));
        let pts = [
            (0u32, 0u32),
            (cw - 1, 0),
            (0, ch - 1),
            (cw - 1, ch - 1), // the four corners — bottom-right was the cut
            (cw - 1, ch / 2),
            (cw / 2, ch - 1),
            (cw / 2, ch / 2),
        ];
        for (x, y) in pts {
            let i = ((y * cw + x) * 4) as usize;
            assert_eq!(
                &rgba[i..i + 3],
                &color[..3],
                "UI layer does not cover ({x},{y}) with the exact canvas color — a quad triangle is missing or gamma double-applied"
            );
        }
    }

    // -----------------------------------------------------------------
    // R3DV-004: construction mesh from HOST-owned state. Every edit below
    // goes through HostCommand + run_ticks; the renderer only ever reads
    // `host.construction` (immutable) — there is no other route in scope.
    // -----------------------------------------------------------------

    fn build_wall(host: &mut pc3d_world::host::SoloHost) {
        use pc3d_world::coords::CellCoord;
        use pc3d_world::gen::CellMaterial;
        use pc3d_world::host::HostCommand;
        // All cells x=0..=4, z=-8 live in the single patch (0, 0, -1)
        // (negative cells would land in patch x=-1 by Euclidean division).
        for x in 0..=4 {
            host.submit(HostCommand::Build {
                cell: CellCoord { x, y: 0, z: -8 },
                material: CellMaterial::Rock,
                owner: 7,
            });
        }
        for x in [0, 2, 4] {
            host.submit(HostCommand::Build {
                cell: CellCoord { x, y: 1, z: -8 },
                material: CellMaterial::Rock,
                owner: 7,
            });
        }
        host.run_ticks(1);
    }

    fn wall_pose() -> CameraPose {
        // Eye south of the wall's center block (2,1,-8), looking north.
        CameraPose::new([2.0, 2.2, -2.0], 0.0, -0.18)
    }

    #[test]
    fn host_construction_edits_change_the_frame_through_bounded_remesh() {
        use crate::construction::material_albedo;
        use crate::scene::{
            dir_from_ndc, lit_color, project_ndc, sky_color_linear, to_srgb4, Probe, SUN_DIR,
        };
        use pc3d_world::coords::CellCoord;
        use pc3d_world::gen::CellMaterial;
        use pc3d_world::host::HostCommand;

        let mut host = pc3d_world::host::SoloHost::new(4242);
        build_wall(&mut host);
        // 8 blocks, all in patch (0, 0, -1).
        assert_eq!(host.construction.len(), 1);
        assert_eq!(host.construction.values().next().unwrap().built_count(), 8);

        let mut r = Renderer::offscreen(W, H);
        r.set_placeholder_scene(false); // construction is the only world geometry
        r.attach_construction();
        let s0 = r.update_construction(&host.construction);
        assert_eq!((s0.added, s0.remeshed, s0.inspected), (1, 0, 1));
        let pose = wall_pose();
        r.set_pose(pose);

        // The (0,1,-8) block's south-face center projects near screen center.
        let target = project_ndc(pose, ASPECT, [2.5, 1.5, -7.0]);
        assert!(
            target.0.abs() < 0.2 && target.1.abs() < 0.3,
            "probe at {target:?}"
        );
        let rock_face = to_srgb4(lit_color(
            material_albedo(CellMaterial::Rock),
            [0.0, 0.0, 1.0],
        ));
        let sand_face = to_srgb4(lit_color(
            material_albedo(CellMaterial::Sand),
            [0.0, 0.0, 1.0],
        ));

        // BEFORE: the rock block's south face is on screen.
        let before_path = std::env::temp_dir().join("pc3d_build_before.png");
        let (before, rgba_before) = r.capture_png(
            &before_path,
            &[Probe {
                name: "rock_block_south_face",
                ndc: target,
                expected: rock_face,
                tol: 0.06,
            }],
        );
        assert!(before.passes(), "before: {:?}", before.failed_probes());

        // EDIT through the host command path: replace the top-center rock
        // with sand (remove + place in the same cell, same patch).
        host.submit(HostCommand::RemoveBuild {
            cell: CellCoord { x: 2, y: 1, z: -8 },
            owner: 7,
        });
        host.submit(HostCommand::Build {
            cell: CellCoord { x: 2, y: 1, z: -8 },
            material: CellMaterial::Sand,
            owner: 7,
        });
        host.run_ticks(1);
        let s1 = r.update_construction(&host.construction);
        // Bounded: exactly one patch inspected, one remesh, nothing added.
        assert_eq!((s1.added, s1.remeshed, s1.inspected), (0, 1, 1));
        println!("edit remesh: {s1:?}");

        // AFTER: the SAME screen point now shows sand, not rock.
        let after_path = std::env::temp_dir().join("pc3d_build_after.png");
        let (after, rgba_after) = r.capture_png(
            &after_path,
            &[Probe {
                name: "sand_block_south_face",
                ndc: target,
                expected: sand_face,
                tol: 0.06,
            }],
        );
        assert!(after.passes(), "after: {:?}", after.failed_probes());

        // Localized visual edit: the changed cell's pixels moved rock->sand,
        // while a control sky pixel is untouched.
        let sky_ctrl = (-0.75, 0.55);
        let p_before = crate::scene::sample_ndc(&rgba_before, W, H, target);
        let p_after = crate::scene::sample_ndc(&rgba_after, W, H, target);
        let delta = (p_before[0] - p_after[0]).abs() + (p_before[1] - p_after[1]).abs();
        assert!(
            delta > 0.15,
            "edited cell barely changed: {p_before:?} vs {p_after:?}"
        );
        let s_before = crate::scene::sample_ndc(&rgba_before, W, H, sky_ctrl);
        let s_after = crate::scene::sample_ndc(&rgba_after, W, H, sky_ctrl);
        for i in 0..3 {
            assert!(
                (s_before[i] - s_after[i]).abs() < 0.02,
                "control sky pixel changed: {s_before:?} vs {s_after:?}"
            );
        }
        let _ = (dir_from_ndc, sky_color_linear, SUN_DIR);

        // REJECTED command = zero mesh work: a foreign owner cannot remove.
        host.submit(HostCommand::RemoveBuild {
            cell: CellCoord { x: 2, y: 1, z: -8 },
            owner: 999,
        });
        host.run_ticks(1);
        let s2 = r.update_construction(&host.construction);
        assert_eq!(
            (s2.added, s2.remeshed, s2.uploaded_vertices),
            (0, 0, 0),
            "a rejected host command must cause no remesh"
        );
        // The block survived.
        assert!(host
            .construction
            .values()
            .next()
            .unwrap()
            .at(CellCoord { x: 2, y: 1, z: -8 })
            .is_some());
    }

    // -----------------------------------------------------------------
    // R3DV-005: natural terrain from the authoritative query, with probes
    // DERIVED FROM THE QUERY (expected colors come from final_solid's own
    // material at the probed cell).
    // -----------------------------------------------------------------

    #[test]
    fn terrain_hill_renders_with_query_derived_probes() {
        use crate::scene::{dir_from_ndc, project_ndc, sky_color_linear, to_srgb4, Probe, SUN_DIR};
        use crate::terrain::{column_top, face_expectation, overview_pose};
        use pc3d_world::coords::{CellCoord, PatchCoord};
        use pc3d_world::terrain::SceneSpec;

        let (seed, coord) = SceneSpec::SmoothHills.patch();
        let gen = pc3d_world::gen::WorldGen::new(seed);
        let mut r = Renderer::offscreen(W, H);
        r.set_placeholder_scene(false);
        let stats = r.load_terrain(&gen, &[coord]);
        assert_eq!(stats.patches, 1);
        assert!(
            stats.triangles > 500,
            "expected a real hill mesh: {stats:?}"
        );
        let pose = overview_pose(&gen, coord);
        r.set_pose(pose);

        // Standable center cell: its rendered top face must show the
        // material the collision query reports.
        let o = coord.origin();
        let cx = o.x.div_euclid(1000) as i32 + 8;
        let cz = o.z.div_euclid(1000) as i32 + 8;
        let top = column_top(&gen, cx, cz, o.y.div_euclid(1000) as i32 + 15)
            .expect("standable cell at patch center");
        let top_point = [top.x as f32 + 0.5, top.y as f32 + 1.0, top.z as f32 + 0.5];
        let _top_ndc = project_ndc(pose, ASPECT, top_point);

        // A slope side face: the first surface cell whose east neighbor is air.
        let mut slope: Option<(CellCoord, [f32; 3], [f32; 3])> = None;
        for lx in 2..14 {
            for lz in 2..14 {
                let x = o.x.div_euclid(1000) as i32 + lx;
                let z = o.z.div_euclid(1000) as i32 + lz;
                if let Some(cell) = column_top(&gen, x, z, o.y.div_euclid(1000) as i32 + 15) {
                    let east = CellCoord {
                        x: cell.x + 1,
                        y: cell.y,
                        z: cell.z,
                    };
                    let east_solid = pc3d_world::terrain::final_solid(
                        &gen,
                        east.x as i64 * 1000,
                        east.y as i64 * 1000,
                        east.z as i64 * 1000,
                    )
                    .solid;
                    if !east_solid {
                        slope = Some((
                            cell,
                            [1.0, 0.0, 0.0],
                            [
                                cell.x as f32 + 1.0,
                                cell.y as f32 + 0.5,
                                cell.z as f32 + 0.5,
                            ],
                        ));
                        break;
                    }
                }
            }
            if slope.is_some() {
                break;
            }
        }
        let (slope_cell, slope_normal, slope_point) = slope.expect("a hill slope step must exist");

        let probes = vec![
            Probe {
                name: "sky_above_horizon",
                ndc: (0.0, 0.8),
                expected: to_srgb4(sky_color_linear(
                    dir_from_ndc(pose, (0.0, 0.8), ASPECT),
                    SUN_DIR,
                )),
                tol: 0.05,
            },
            Probe {
                name: "ground_top_face_matches_query",
                ndc: project_ndc(pose, ASPECT, top_point),
                expected: face_expectation(&gen, top, [0.0, 1.0, 0.0]),
                tol: 0.06,
            },
            Probe {
                name: "slope_side_face_matches_query",
                ndc: project_ndc(pose, ASPECT, slope_point),
                expected: face_expectation(&gen, slope_cell, slope_normal),
                tol: 0.06,
            },
        ];
        for p in &probes {
            assert!(
                p.ndc.0 > -0.99 && p.ndc.0 < 0.99 && p.ndc.1 > -0.99 && p.ndc.1 < 0.99,
                "probe {} off-screen at {:?}",
                p.name,
                p.ndc
            );
        }
        let path = std::env::temp_dir().join("pc3d_terrain_hills.png");
        let (report, _) = r.capture_png(&path, &probes);
        // A steep overview is mostly flat-shaded terrain tops plus a thin
        // horizon sky band, so the distinct-color floor is relaxed; the
        // query-derived probes carry the semantic proof.
        assert!(
            report.passes_with(12),
            "hills terrain frame: {:?} (report {report:?})",
            report.failed_probes()
        );
        println!("hills terrain: {stats:?}");
    }

    #[test]
    fn terrain_cave_interior_and_overhang_render() {
        use crate::scene::{dir_from_ndc, project_ndc, sky_color_linear, to_srgb4, Probe, SUN_DIR};
        use crate::terrain::{cave_pose, face_expectation, find_cave_pocket};
        use pc3d_world::coords::PatchCoord;
        use pc3d_world::terrain::SceneSpec;

        // Find a camera-friendly cave pocket near a scene patch.
        let (seed, coord) = SceneSpec::Highlands.patch();
        let gen = pc3d_world::gen::WorldGen::new(seed);
        let (air, wall, dir) = crate::terrain::find_cave_pocket_near(&gen, coord, 1)
            .or_else(|| {
                let (s, c) = SceneSpec::SmoothHills.patch();
                let _ = s;
                crate::terrain::find_cave_pocket_near(&pc3d_world::gen::WorldGen::new(3), c, 1)
            })
            .expect("cave pocket");
        let cave_patch = PatchCoord {
            x: air.x.div_euclid(16),
            y: air.y.div_euclid(16),
            z: air.z.div_euclid(16),
        };
        // If the pocket came from the fallback generator, rebuild with it.
        let gen = if find_cave_pocket(&gen, coord) == Some((air, wall, dir)) {
            gen
        } else {
            pc3d_world::gen::WorldGen::new(3)
        };

        // Load the 3x3 patch neighborhood: the corridor can cross patch
        // boundaries, and a one-patch load would show the world through
        // unmeshed neighbors (the streaming/seam lesson for R3DV-006).
        let mut patches = Vec::new();
        for dy in -1..=1 {
            for dx in -1..=1 {
                for dz in -1..=1 {
                    patches.push(pc3d_world::coords::PatchCoord {
                        x: cave_patch.x + dx,
                        y: cave_patch.y + dy,
                        z: cave_patch.z + dz,
                    });
                }
            }
        }
        let mut r = Renderer::offscreen(W, H);
        r.set_placeholder_scene(false);
        let stats = r.load_terrain(&gen, &patches);
        let pose = cave_pose(air, wall);
        r.set_pose(pose);

        // The pocket must be the corridor kind: two air cells toward the
        // wall, so the ceiling underside over the corridor is visible in the
        // same forward view as the wall.
        assert!(
            crate::terrain::pocket_has_corridor(&gen, air, dir),
            "GPU cave proof needs a corridor pocket"
        );
        let ceiling = pc3d_world::coords::CellCoord {
            x: air.x,
            y: air.y + 1,
            z: air.z,
        };
        let corridor = pc3d_world::coords::CellCoord {
            x: air.x + dir[0],
            y: air.y,
            z: air.z + dir[2],
        };
        let corridor_ceiling = pc3d_world::coords::CellCoord {
            x: corridor.x,
            y: corridor.y + 1,
            z: corridor.z,
        };
        let wall_face_point = [
            wall.x as f32 + 0.5 - dir[0] as f32 * 0.5,
            wall.y as f32 + 0.5,
            wall.z as f32 + 0.5 - dir[2] as f32 * 0.5,
        ];
        let probes = vec![
            Probe {
                name: "cave_wall_face",
                ndc: project_ndc(pose, ASPECT, wall_face_point),
                expected: face_expectation(&gen, wall, [-dir[0] as f32, 0.0, -dir[2] as f32]),
                tol: 0.06,
            },
            Probe {
                name: "cave_ceiling_overhang_underside",
                ndc: project_ndc(
                    pose,
                    ASPECT,
                    [
                        corridor.x as f32 + 0.5,
                        corridor.y as f32 + 1.0,
                        corridor.z as f32 + 0.5,
                    ],
                ),
                expected: face_expectation(&gen, corridor_ceiling, [0.0, -1.0, 0.0]),
                tol: 0.06,
            },
        ];
        for p in &probes {
            assert!(
                p.ndc.0 > -0.99 && p.ndc.0 < 0.99 && p.ndc.1 > -0.99 && p.ndc.1 < 0.99,
                "probe {} off-screen at {:?}",
                p.name,
                p.ndc
            );
        }
        let path = std::env::temp_dir().join("pc3d_terrain_cave.png");
        let (report, rgba) = r.capture_png(&path, &probes);
        for p in &probes {
            let px = crate::scene::sample_ndc(&rgba, W, H, p.ndc);
            println!(
                "probe {} at {:?}: got {:?} want {:?}",
                p.name, p.ndc, px, p.expected
            );
        }
        // Interior close-up: a fully enclosed corridor view is a handful of
        // FLAT face colors (wall/floor/ceiling shades + HUD); the
        // pixel-exact probes carry the semantic proof.
        assert!(
            report.passes_with(4),
            "cave terrain frame: {:?} (report {report:?})",
            report.failed_probes()
        );
        let _ = ceiling;
        assert!(stats.triangles > 100);
        println!("cave terrain: {stats:?}");
    }

    #[test]
    fn remesh_is_bounded_to_the_edited_patch_only() {
        use pc3d_world::coords::CellCoord;
        use pc3d_world::gen::CellMaterial;
        use pc3d_world::host::HostCommand;

        let mut host = pc3d_world::host::SoloHost::new(7);
        build_wall(&mut host); // patch (0, 0, -1)
                               // A second, far-away patch: x=20 lives in patch (1, 0, -1).
        host.submit(HostCommand::Build {
            cell: CellCoord { x: 20, y: 0, z: -8 },
            material: CellMaterial::Soil,
            owner: 7,
        });
        host.run_ticks(1);
        assert_eq!(host.construction.len(), 2);

        let mut r = Renderer::offscreen(W, H);
        r.attach_construction();
        let s0 = r.update_construction(&host.construction);
        assert_eq!((s0.added, s0.remeshed, s0.inspected), (2, 0, 2));

        // Edit ONLY the wall patch (an empty cell — occupied cells are
        // refused by the host, which the other test proves); the far patch
        // must not remesh.
        host.submit(HostCommand::Build {
            cell: CellCoord { x: 1, y: 1, z: -8 },
            material: CellMaterial::Grass,
            owner: 7,
        });
        host.run_ticks(1);
        let s1 = r.update_construction(&host.construction);
        assert_eq!((s1.added, s1.remeshed, s1.inspected), (0, 1, 2));
        assert!(s1.uploaded_vertices > 0, "the edited patch uploaded a mesh");
    }
}
