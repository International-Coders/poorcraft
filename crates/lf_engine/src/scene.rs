use image::RgbaImage;
use wgpu::util::DeviceExt;

use crate::camera::Camera;

/// GPU vertex layout: must match shader.wgsl vs_main locations 0..4.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
    pub tex_index: u32,
    pub ao: f32,
    /// Packed light: sky in the high nibble, block light in the low nibble.
    pub light: u32,
    /// Wind-sway weight (foliage = 1.0); the vertex shader offsets the
    /// position by a world-position-phased wave scaled by this.
    pub sway: f32,
}

impl GpuVertex {
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32x2 },
                wgpu::VertexAttribute { offset: 32, shader_location: 3, format: wgpu::VertexFormat::Uint32 },
                wgpu::VertexAttribute { offset: 36, shader_location: 4, format: wgpu::VertexFormat::Float32 },
                wgpu::VertexAttribute { offset: 40, shader_location: 5, format: wgpu::VertexFormat::Uint32 },
                wgpu::VertexAttribute { offset: 44, shader_location: 6, format: wgpu::VertexFormat::Float32 },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    view_proj: [[f32; 4]; 4],
    // xyz = camera position, w = day factor [0..1] scaling sky light
    cam_pos_day: [f32; 4],
    // rgb = fog color, w = fog end distance in blocks
    fog: [f32; 4],
    // x = elapsed seconds (drives foliage wind), yzw spare
    time_sway: [f32; 4],
    // rgb = color-grade tint multiplier, w = saturation (1 = unchanged)
    grade: [f32; 4],
}

/// Environment parameters passed every frame.
#[derive(Copy, Clone, Debug)]
pub struct Env {
    pub camera_pos: glam::Vec3,
    /// Day factor [0..1] scaling sky light.
    pub day_factor: f32,
    pub fog_color: [f32; 3],
    /// Fog end distance in blocks.
    pub fog_far: f32,
    /// Elapsed seconds; drives the vertex-shader foliage sway.
    pub time: f32,
    /// Color-grade tint multiplier per channel (1.0 = unchanged) — the
    /// per-biome "film grade" applied to the final fragment color.
    pub grade_tint: [f32; 3],
    /// Color-grade saturation (1.0 = unchanged, <1 desaturates).
    pub grade_saturation: f32,
}

impl Env {
    /// Neutral (temperate) grade — the baseline every biome grade lerps
    /// from and back to.
    pub fn neutral_grade() -> ([f32; 3], f32) {
        ([1.0, 1.0, 1.0], 1.0)
    }
}

/// Pipeline + texture array shared by every mesh drawn in a frame.
pub struct SceneResources {
    render_pipeline: wgpu::RenderPipeline,
    water_pipeline: wgpu::RenderPipeline,
    diffuse_bind_group: wgpu::BindGroup,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
}

impl SceneResources {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, target_format: wgpu::TextureFormat,
               textures: &[RgbaImage]) -> Self {
        let texture_size = wgpu::Extent3d {
            width: 16,
            height: 16,
            depth_or_array_layers: textures.len().max(1) as u32,
        };
        // 16x16 layers -> 5 mip levels (16, 8, 4, 2, 1); distance sampling
        // picks a filtered mip so terrain stops shimmering at range
        const MIP_LEVELS: u32 = 5;
        let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: MIP_LEVELS,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("diffuse_texture_array"),
            view_formats: &[],
        });
        for (i, img) in textures.iter().enumerate() {
            let mut level = img.clone();
            for mip in 0..MIP_LEVELS {
                let (w, h) = level.dimensions();
                queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture: &diffuse_texture,
                        mip_level: mip,
                        origin: wgpu::Origin3d { x: 0, y: 0, z: i as u32 },
                        aspect: wgpu::TextureAspect::All,
                    },
                    level.as_raw(),
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * w),
                        rows_per_image: Some(h),
                    },
                    wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                );
                level = downsample_2x(&level);
            }
        }
        let diffuse_texture_view = diffuse_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            array_layer_count: Some(textures.len().max(1) as u32),
            ..Default::default()
        });
        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let diffuse_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("diffuse_bind_group_layout"),
        });
        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &diffuse_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&diffuse_texture_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&diffuse_sampler) },
            ],
            label: Some("diffuse_bind_group"),
        });

        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniform_bind_group_layout"),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, &diffuse_bind_group_layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let make_pipeline = |label: &str, blend: Option<wgpu::BlendState>, depth_write: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[GpuVertex::layout()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: depth_write,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState { count: 1, mask: !0, alpha_to_coverage_enabled: false },
                multiview: None,
                cache: None,
            })
        };
        let render_pipeline = make_pipeline("Render Pipeline", Some(wgpu::BlendState::REPLACE), true);
        let water_pipeline = make_pipeline("Water Pipeline", Some(wgpu::BlendState::ALPHA_BLENDING), false);

        Self {
            render_pipeline,
            water_pipeline,
            diffuse_bind_group,
            uniform_bind_group_layout,
        }
    }
}

/// One drawable mesh (own vertex/index/camera-uniform buffers) using shared
/// SceneResources. Typically one per chunk column.
pub struct MeshBatch {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
}

impl MeshBatch {
    pub fn new(device: &wgpu::Device, resources: &SceneResources,
               vertices: &[GpuVertex], indices: &[u32]) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[Uniforms {
                view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
                cam_pos_day: [0.0; 4],
                fog: [0.0; 4],
                time_sway: [0.0; 4],
                grade: [1.0, 1.0, 1.0, 1.0],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &resources.uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() }],
            label: Some("uniform_bind_group"),
        });
        Self {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            uniform_buffer,
            uniform_bind_group,
        }
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, camera: &Camera, env: &Env) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[Uniforms {
                view_proj: camera.build_view_projection_matrix().to_cols_array_2d(),
                cam_pos_day: [env.camera_pos.x, env.camera_pos.y, env.camera_pos.z, env.day_factor],
                fog: [env.fog_color[0], env.fog_color[1], env.fog_color[2], env.fog_far],
                time_sway: [env.time, 0.0, 0.0, 0.0],
                grade: [env.grade_tint[0], env.grade_tint[1], env.grade_tint[2], env.grade_saturation],
            }]),
        );
    }

    /// Record the draw into an existing render pass. `water` selects the
    /// alpha-blended pipeline (draw after opaque, roughly back to front).
    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, resources: &'a SceneResources, water: bool) {
        render_pass.set_pipeline(if water { &resources.water_pipeline } else { &resources.render_pipeline });
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &resources.diffuse_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }

    /// Convenience: create a depth texture + view for a target size.
    pub fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }
}

/// Single-mesh convenience used by the demo app and headless renderer.
pub struct GpuScene {
    pub resources: SceneResources,
    pub batch: MeshBatch,
}

impl GpuScene {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, target_format: wgpu::TextureFormat,
               vertices: &[GpuVertex], indices: &[u32], textures: &[RgbaImage]) -> Self {
        let resources = SceneResources::new(device, queue, target_format, textures);
        let batch = MeshBatch::new(device, &resources, vertices, indices);
        Self { resources, batch }
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, camera: &Camera, env: &Env) {
        self.batch.update_camera(queue, camera, env);
    }

    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        self.batch.draw(render_pass, &self.resources, false);
    }
}

/// Box-filter a texture down by exactly 2x (used to build the atlas mip
/// chain on the CPU at startup).
fn downsample_2x(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    if nw == w || nh == h {
        return img.clone();
    }
    let mut out = RgbaImage::new(nw, nh);
    for y in 0..nh {
        for x in 0..nw {
            let (x0, y0) = (x * 2, y * 2);
            let mut acc = [0u32; 4];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let px = img.get_pixel(x0 + dx, y0 + dy);
                for c in 0..4 {
                    acc[c] += px.0[c] as u32;
                }
            }
            out.put_pixel(x, y, image::Rgba([
                (acc[0] / 4) as u8,
                (acc[1] / 4) as u8,
                (acc[2] / 4) as u8,
                (acc[3] / 4) as u8,
            ]));
        }
    }
    out
}
