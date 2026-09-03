use wgpu::util::DeviceExt;

use crate::camera::Camera;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LineVertex {
    position: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

/// Wireframe block outline (targeting highlight). Line-list pipeline; the
/// vertex buffer is rebuilt whenever the target block changes.
pub struct OutlineScene {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    /// (block, box count, up to two pick boxes) — stairs carry two.
    current_block: Option<((i32, i32, i32), usize, [[f32; 6]; 2])>,
}

impl OutlineScene {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("outline_uniform_layout"),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Outline Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Outline Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("line.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Outline Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Outline Uniform Buffer"),
            contents: bytemuck::cast_slice(&[Uniforms { view_proj: glam::Mat4::IDENTITY.to_cols_array_2d() }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() }],
            label: Some("outline_uniform_bind_group"),
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Outline Vertex Buffer"),
            contents: &[],
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            pipeline,
            vertex_buffer,
            num_vertices: 0,
            uniform_buffer,
            uniform_bind_group,
            current_block: None,
        }
    }

    /// Point the outline at a block (min corner) with its local pick
    /// boxes — skips GPU work when unchanged. Boxes come from
    /// `lf_voxel::registry::pick_boxes` so the wireframe traces the
    /// block's real shape (loop 347: slabs half-height, torches thin,
    /// plants inset) instead of always the full cell.
    pub fn set_target(&mut self, device: &wgpu::Device, block: Option<((i32, i32, i32), &[[f32; 6]])>) {
        let key = block.map(|((x, y, z), b)| {
            let mut boxes = [[0.0f32; 6]; 2];
            let n = b.len().min(2);
            boxes[..n].copy_from_slice(&b[..n]);
            ((x, y, z), n, boxes)
        });
        if self.current_block == key {
            return;
        }
        self.current_block = key;
        let vertices: Vec<LineVertex> = match key {
            Some(((bx, by, bz), n, boxes)) => {
                let mut out = Vec::with_capacity(n * 24);
                for b in &boxes[..n] {
                    // hairline inflation so the wire never z-fights the
                    // block faces it traces
                    let x0 = bx as f32 + b[0] - 0.002;
                    let y0 = by as f32 + b[1] - 0.002;
                    let z0 = bz as f32 + b[2] - 0.002;
                    let x1 = bx as f32 + b[3] + 0.002;
                    let y1 = by as f32 + b[4] + 0.002;
                    let z1 = bz as f32 + b[5] + 0.002;
                    let c = [
                        [x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1], // bottom
                        [x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1], // top
                    ];
                    // 12 edges
                    for [a, bb] in [[0, 1], [1, 2], [2, 3], [3, 0], [4, 5], [5, 6], [6, 7], [7, 4], [0, 4], [1, 5], [2, 6], [3, 7]] {
                        out.push(LineVertex { position: c[a] });
                        out.push(LineVertex { position: c[bb] });
                    }
                }
                out
            }
            None => Vec::new(),
        };
        self.num_vertices = vertices.len() as u32;
        self.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Outline Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, camera: &Camera) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[Uniforms { view_proj: camera.build_view_projection_matrix().to_cols_array_2d() }]),
        );
    }

    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.num_vertices == 0 {
            return;
        }
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.num_vertices, 0..1);
    }
}
