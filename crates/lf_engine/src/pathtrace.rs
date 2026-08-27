//! GPU voxel-DDA path tracer: builds a 128x64x128 block texture around a
//! center, dispatches the compute tracer, and copies the accumulated frame
//! out as an image. Used by the R-toggle in-game and headless proofs.

use image::{Rgba, RgbaImage};

use crate::camera::Camera;
use wgpu::util::DeviceExt;

pub const CLIP_DIMS: (u32, u32, u32) = (128, 64, 128);

/// RGB palette indexed by block id (procedural colors mirroring lf_assets).
fn palette() -> [[f32; 4]; 128] {
    let mut p = [[0.0f32; 4]; 128];
    let set = |p: &mut [[f32; 4]; 128], id: u32, c: [f32; 4]| {
        if (id as usize) < p.len() {
            p[id as usize] = c;
        }
    };
    set(&mut p, 1, [0.5, 0.5, 0.5, 1.0]);        // stone
    set(&mut p, 2, [0.35, 0.63, 0.24, 1.0]);     // grass
    set(&mut p, 3, [0.52, 0.38, 0.26, 1.0]);     // dirt
    set(&mut p, 4, [0.86, 0.81, 0.64, 1.0]);     // sand
    set(&mut p, 5, [0.55, 0.51, 0.63, 1.0]);     // mycelium
    set(&mut p, 6, [0.94, 0.96, 0.96, 1.0]);     // snow
    set(&mut p, 7, [0.40, 0.32, 0.20, 1.0]);     // log
    set(&mut p, 8, [0.24, 0.47, 0.16, 1.0]);     // leaves
    set(&mut p, 9, [0.35, 0.35, 0.38, 1.0]);     // coal ore
    set(&mut p, 10, [0.72, 0.55, 0.45, 1.0]);    // iron ore
    set(&mut p, 11, [0.12, 0.30, 0.55, 1.0]);    // water
    set(&mut p, 12, [1.0, 0.75, 0.35, 1.0]);     // torch (emissive)
    set(&mut p, 13, [1.0, 0.85, 0.5, 1.0]);      // lantern (emissive)
    set(&mut p, 14, [0.63, 0.47, 0.28, 1.0]);    // crafting table
    set(&mut p, 15, [0.45, 0.45, 0.47, 1.0]);    // furnace
    set(&mut p, 16, [0.59, 0.39, 0.20, 1.0]);    // chest
    set(&mut p, 17, [0.65, 0.55, 0.38, 1.0]);    // planks
    set(&mut p, 18, [0.75, 0.85, 0.9, 0.5]);     // glass
    set(&mut p, 19, [0.82, 0.82, 0.85, 1.0]);    // birch log
    set(&mut p, 20, [0.27, 0.31, 0.25, 1.0]);    // spruce log
    set(&mut p, 21, [0.22, 0.18, 0.13, 1.0]);    // dark log
    set(&mut p, 22, [0.55, 0.35, 0.35, 1.0]);    // cherry log
    set(&mut p, 23, [0.55, 0.78, 0.45, 1.0]);    // birch leaves
    set(&mut p, 24, [0.14, 0.30, 0.18, 1.0]);    // spruce leaves
    set(&mut p, 25, [0.12, 0.15, 0.10, 1.0]);    // dark leaves
    set(&mut p, 26, [0.90, 0.65, 0.75, 1.0]);    // cherry leaves
    set(&mut p, 27, [0.60, 0.62, 0.55, 1.0]);    // pale leaves
    set(&mut p, 28, [0.75, 0.35, 0.25, 1.0]);    // red sand
    set(&mut p, 29, [0.72, 0.45, 0.30, 1.0]);    // terracotta
    set(&mut p, 30, [0.25, 0.45, 0.20, 1.0]);    // moss
    set(&mut p, 31, [0.65, 0.85, 0.95, 0.8]);    // ice
    for id in 32..=41u32 {
        set(&mut p, id, [0.5, 0.5, 0.52, 1.0]);  // machines/ores
    }
    // ids 42..47: biome-identity surfaces + Water Age machines
    set(&mut p, 42, [0.16, 0.55, 0.20, 1.0]);     // jungle grass
    set(&mut p, 43, [0.70, 0.66, 0.29, 1.0]);     // savanna grass
    set(&mut p, 44, [0.92, 0.27, 0.27, 1.0]);     // wildflower
    set(&mut p, 45, [0.66, 0.52, 0.31, 1.0]);     // water wheel
    set(&mut p, 46, [0.80, 0.72, 0.62, 1.0]);     // battery
    set(&mut p, 47, [0.78, 0.43, 0.24, 1.0]);     // pipe
    set(&mut p, 48, [0.55, 0.53, 0.50, 1.0]);     // boiler
    set(&mut p, 49, [0.62, 0.58, 0.54, 1.0]);     // steam engine
    // Oil Age (P31)
    set(&mut p, 50, [0.06, 0.05, 0.04, 1.0]);     // crude oil
    set(&mut p, 51, [0.33, 0.29, 0.25, 1.0]);     // pumpjack
    set(&mut p, 52, [0.45, 0.42, 0.38, 1.0]);     // refinery
    set(&mut p, 53, [0.50, 0.33, 0.20, 1.0]);     // combustion generator
    // Fallback for ids beyond the hand-tuned set (future content): a stable
    // muted color per id so new blocks are never invisible/wrong in RT
    // before they get a hand-tuned entry (registry-driven palette fix, P29).
    let mut id = 54usize;
    while id < p.len() {
        let h = (id as u32).wrapping_mul(2654435761);
        let r = 0.35 + (h & 0xFF) as f32 / 255.0 * 0.3;
        let g = 0.35 + ((h >> 8) & 0xFF) as f32 / 255.0 * 0.3;
        let b = 0.38 + ((h >> 16) & 0xFF) as f32 / 255.0 * 0.3;
        p[id] = [r, g, b, 1.0];
        id += 1;
    }
    // lore-and-visuals blocks (68..=105): hand-tuned to their atlas art so
    // faction structures read correctly in RT captures
    set(&mut p, 68, [0.55, 0.57, 0.62, 1.0]);    // accord stone
    set(&mut p, 69, [0.58, 0.60, 0.65, 1.0]);    // accord pillar
    set(&mut p, 70, [0.37, 0.27, 0.20, 1.0]);    // ironborn brick
    set(&mut p, 71, [0.36, 0.32, 0.29, 0.7]);    // ironborn grate
    set(&mut p, 72, [0.17, 0.14, 0.11, 1.0]);    // covenantwood
    set(&mut p, 73, [0.83, 0.59, 0.26, 1.0]);    // ember glowstone (emissive-ish)
    set(&mut p, 74, [0.77, 0.65, 0.40, 1.0]);    // freeholds thatch
    set(&mut p, 75, [0.86, 0.83, 0.76, 1.0]);    // freeholds daub
    set(&mut p, 76, [0.79, 0.80, 0.82, 1.0]);    // ashen marble
    set(&mut p, 77, [0.50, 0.48, 0.50, 1.0]);    // ashen bookshelf
    set(&mut p, 78, [0.34, 0.29, 0.23, 1.0]);    // nameless rotwood
    set(&mut p, 79, [0.22, 0.20, 0.19, 1.0]);    // nameless scorched
    set(&mut p, 80, [0.80, 0.20, 0.15, 1.0]);    // mushroom cap
    set(&mut p, 81, [0.92, 0.52, 0.39, 1.0]);    // coral block
    set(&mut p, 82, [0.38, 0.44, 0.52, 1.0]);    // permafrost
    set(&mut p, 83, [0.18, 0.16, 0.17, 1.0]);    // volcanic basalt
    set(&mut p, 84, [0.16, 0.17, 0.22, 1.0]);    // deep slate
    set(&mut p, 85, [0.85, 0.47, 0.27, 1.0]);    // mesa terracotta
    set(&mut p, 86, [0.72, 0.62, 0.25, 1.0]);    // gilded grass
    set(&mut p, 87, [0.20, 0.16, 0.12, 1.0]);    // bog peat
    set(&mut p, 88, [0.68, 0.53, 0.34, 1.0]);    // carved oak
    set(&mut p, 89, [0.50, 0.51, 0.53, 1.0]);    // carved stone
    set(&mut p, 90, [0.55, 0.53, 0.55, 1.0]);    // carved iron
    set(&mut p, 91, [0.80, 0.18, 0.18, 0.55]);   // stained glass red
    set(&mut p, 92, [0.90, 0.52, 0.12, 0.55]);   // stained glass orange
    set(&mut p, 93, [0.90, 0.82, 0.20, 0.55]);   // stained glass yellow
    set(&mut p, 94, [0.24, 0.71, 0.28, 0.55]);   // stained glass green
    set(&mut p, 95, [0.24, 0.43, 0.82, 0.55]);   // stained glass blue
    set(&mut p, 96, [0.59, 0.28, 0.78, 0.55]);   // stained glass purple
    set(&mut p, 97, [0.12, 0.12, 0.14, 0.7]);    // stained glass black
    set(&mut p, 98, [0.92, 0.92, 0.92, 0.55]);   // stained glass white
    set(&mut p, 99, [0.29, 0.48, 0.71, 1.0]);    // banner accord
    set(&mut p, 100, [0.55, 0.27, 0.07, 1.0]);   // banner ironborn
    set(&mut p, 101, [0.77, 0.38, 0.16, 1.0]);   // banner covenant
    set(&mut p, 102, [0.42, 0.56, 0.14, 1.0]);   // banner freeholds
    set(&mut p, 103, [0.69, 0.69, 0.69, 1.0]);   // banner ashen
    set(&mut p, 104, [0.18, 0.18, 0.18, 1.0]);   // banner nameless
    set(&mut p, 105, [1.0, 0.85, 0.5, 1.0]);     // hanging lantern (emissive)
    p
}

/// Pack the world into the clip texture. `center` is the block-space center;
/// `get` maps world block coords to a block id (0 = air).
pub fn build_voxel_texture_data(center: (i32, i32, i32), get: &dyn Fn(i32, i32, i32) -> u32) -> Vec<u32> {
    let (w, h, d) = CLIP_DIMS;
    let mut data = vec![0u32; (w * h * d) as usize];
    for y in 0..h as i32 {
        for z in 0..d as i32 {
            for x in 0..w as i32 {
                let wx = center.0 + x - (w as i32) / 2;
                let wy = center.1 + y - (h as i32) / 2;
                let wz = center.2 + z - (d as i32) / 2;
                let id = get(wx, wy, wz);
                let idx = (x + y * w as i32 + z * w as i32 * h as i32) as usize;
                // block id in low bits; light nibble unused by the tracer
                data[idx] = id & 0xFF;
            }
        }
    }
    data
}

/// Persistent path tracer: GPU resources created once, frames reused for
/// live rendering (the RT "Live" video setting).
pub struct Pathtracer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    voxel_texture: wgpu::Texture,
    palette_buf: wgpu::Buffer,
    accum: wgpu::Texture,
    accum_view: wgpu::TextureView,
    readback: wgpu::Buffer,
    pub width: u32,
    pub height: u32,
    last_center: (i32, i32, i32),
    frame: u32,
}

impl Pathtracer {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        pollster::block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::PRIMARY,
                ..Default::default()
            });
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .ok_or_else(|| "no GPU adapter".to_string())?;
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("Live PT Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                }, None)
                .await
                .map_err(|e| format!("device: {e:?}"))?;
            let (vw, vh, vd) = CLIP_DIMS;
            let voxel_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Live PT Voxels"),
                size: wgpu::Extent3d { width: vw, height: vh, depth_or_array_layers: vd },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: wgpu::TextureFormat::R32Uint,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let accum = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Live PT Accum"),
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let accum_view = accum.create_view(&wgpu::TextureViewDescriptor::default());
            let padded = ((width * 8) + 255) / 256 * 256;
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Live PT Readback"),
                size: (padded * height) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let palette_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Live PT Palette"),
                contents: bytemuck::cast_slice(&palette()),
                usage: wgpu::BufferUsages::STORAGE,
            });
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Pathtrace Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("pathtrace.wgsl").into()),
            });
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Live PT Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Uint, view_dimension: wgpu::TextureViewDimension::D3, multisampled: false },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::Rgba16Float, view_dimension: wgpu::TextureViewDimension::D2 },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                        count: None,
                    },
                ],
            });
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Live PT Pipeline"),
                layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Live PT Pipeline Layout"),
                    bind_group_layouts: &[&layout],
                    push_constant_ranges: &[],
                })),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
            Ok(Self { device, queue, pipeline, layout, voxel_texture, palette_buf, accum, accum_view, readback, width, height, last_center: (i32::MIN, 0, 0), frame: 0 })
        })
    }

    /// Upload the voxel clip if the center moved.
    pub fn upload_voxels(&mut self, center: (i32, i32, i32), data: &[u32]) {
        if center == self.last_center {
            return;
        }
        self.last_center = center;
        let (vw, vh, vd) = CLIP_DIMS;
        self.queue.write_texture(
            wgpu::ImageCopyTexture { texture: &self.voxel_texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            bytemuck::cast_slice(data),
            wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * vw), rows_per_image: Some(vh) },
            wgpu::Extent3d { width: vw, height: vh, depth_or_array_layers: vd },
        );
    }

    /// Force the next upload even if the center is unchanged (world edit).
    pub fn invalidate_voxels(&mut self) {
        self.last_center = (i32::MIN, 0, 0);
    }

    /// Trace one frame and read it back as an RGBA image.
    pub fn render_frame(
        &mut self,
        camera: &Camera,
        center: (i32, i32, i32),
        sun_dir: [f32; 3],
        day_factor: f32,
    ) -> Result<RgbaImage, String> {
        let look = (camera.target - camera.eye).normalize();
        let fov_tan = camera.fovy.tan();
        let right = look.cross(glam::Vec3::Y).normalize() * fov_tan;
        let up = right.cross(look).normalize() * fov_tan;
        let voxel_view = self.voxel_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D3),
            ..Default::default()
        });
        let mut u = Vec::with_capacity(96);
        u.extend_from_slice(bytemuck::cast_slice(&[[camera.eye.x, camera.eye.y, camera.eye.z, camera.aspect]]));
        u.extend_from_slice(bytemuck::cast_slice(&[[look.x, look.y, look.z, fov_tan]]));
        u.extend_from_slice(bytemuck::cast_slice(&[[right.x, right.y, right.z, self.frame as f32]]));
        u.extend_from_slice(bytemuck::cast_slice(&[[up.x, up.y, up.z, 0.0]]));
        let (cw, ch, cd) = CLIP_DIMS;
        u.extend_from_slice(bytemuck::cast_slice(&[[
            (center.0 - cw as i32 / 2) as f32,
            (center.1 - ch as i32 / 2) as f32,
            (center.2 - cd as i32 / 2) as f32,
            0.0,
        ]]));
        u.extend_from_slice(bytemuck::cast_slice(&[[sun_dir[0], sun_dir[1], sun_dir[2], day_factor]]));
        let uniform_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Live PT Uniform"),
            contents: &u,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Live PT Bind"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&voxel_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&self.accum_view) },
                wgpu::BindGroupEntry { binding: 3, resource: self.palette_buf.as_entire_binding() },
            ],
        });
        let width = self.width;
        let height = self.height;
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Live PT Enc") });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Live PT Pass"), timestamp_writes: None });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups((width + 7) / 8, (height + 7) / 8, 1);
        }
        let padded = ((width * 8) + 255) / 256 * 256;
        enc.copy_texture_to_buffer(
            wgpu::ImageCopyTexture { texture: &self.accum, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::ImageCopyBuffer {
                buffer: &self.readback,
                layout: wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(height) },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        self.queue.submit(std::iter::once(enc.finish()));
        let _ = self.device.poll(wgpu::Maintain::Wait);
        self.frame += 1;
        let slice = self.readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::Maintain::Wait);
        let data = slice.get_mapped_range().to_vec();
        let mut img = RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let px = decode_f16_pair(&data, (y * padded + x * 8) as usize);
                img.put_pixel(x, y, Rgba([
                    (px[0].clamp(0.0, 1.0) * 255.0) as u8,
                    (px[1].clamp(0.0, 1.0) * 255.0) as u8,
                    (px[2].clamp(0.0, 1.0) * 255.0) as u8,
                    255,
                ]));
            }
        }
        Ok(img)
    }
}

/// Render one path-traced frame (N accumulation samples) offscreen and
/// return it as an RGBA image. Sizes are in pixels.
pub fn pathtrace_to_image(
    voxel_data: &[u32],
    center: (i32, i32, i32),
    camera: &Camera,
    sun_dir: [f32; 3],
    day_factor: f32,
    width: u32,
    height: u32,
    samples: u32,
) -> Result<RgbaImage, String> {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| "no GPU adapter".to_string())?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Pathtrace Device"),
                required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            }, None)
            .await
            .map_err(|e| format!("device: {e:?}"))?;

        // voxel 3D texture
        let (vw, vh, vd) = CLIP_DIMS;
        let voxel_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Voxel Clip"),
            size: wgpu::Extent3d { width: vw, height: vh, depth_or_array_layers: vd },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::R32Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture { texture: &voxel_texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            bytemuck::cast_slice(voxel_data),
            wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4 * vw), rows_per_image: Some(vh) },
            wgpu::Extent3d { width: vw, height: vh, depth_or_array_layers: vd },
        );
        let voxel_view = voxel_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D3),
            ..Default::default()
        });

        // accumulation storage texture
        let accum = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("PT Accum"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let accum_view = accum.create_view(&wgpu::TextureViewDescriptor::default());

        // readback buffer (rgba16float = 8 bytes/px)
        let padded = ((width * 8) + 255) / 256 * 256;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PT Readback"),
            size: (padded * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // uniforms + palette
        let look = (camera.target - camera.eye).normalize();
        let right = look.cross(glam::Vec3::Y).normalize() * camera.fovy.tan();
        let up = right.cross(look).normalize() * camera.fovy.tan();
        let palette_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("PT Palette"),
            contents: bytemuck::cast_slice(&palette()),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pathtrace Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("pathtrace.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("PT Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Uint, view_dimension: wgpu::TextureViewDimension::D3, multisampled: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::Rgba16Float, view_dimension: wgpu::TextureViewDimension::D2 },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("PT Pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("PT Pipeline Layout"),
                bind_group_layouts: &[&layout],
                push_constant_ranges: &[],
            })),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // accumulate N samples
        let mut frame_idx = 0u32;
        for _ in 0..samples.max(1) {
            let mut u = Vec::with_capacity(80);
            u.extend_from_slice(bytemuck::cast_slice(&[[camera.eye.x, camera.eye.y, camera.eye.z, camera.aspect]]));
            u.extend_from_slice(bytemuck::cast_slice(&[[look.x, look.y, look.z, camera.fovy.tan()]]));
            // up_right packs right.xy and up.y per the wgsl usage
            u.extend_from_slice(bytemuck::cast_slice(&[[right.x, right.y, up.y, frame_idx as f32]]));
            u.extend_from_slice(bytemuck::cast_slice(&[[up.x, up.y, up.z, 0.0]]));
            let (cw, ch, cd) = CLIP_DIMS;
            u.extend_from_slice(bytemuck::cast_slice(&[[
                (center.0 - cw as i32 / 2) as f32,
                (center.1 - ch as i32 / 2) as f32,
                (center.2 - cd as i32 / 2) as f32,
                0.0,
            ]]));
            u.extend_from_slice(bytemuck::cast_slice(&[[sun_dir[0], sun_dir[1], sun_dir[2], day_factor]]));
            let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("PT Uniform"),
                contents: &u,
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("PT Bind"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: uniform_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&voxel_view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&accum_view) },
                    wgpu::BindGroupEntry { binding: 3, resource: palette_buf.as_entire_binding() },
                ],
            });
            let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("PT Enc") });
            {
                let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("PT Pass"), timestamp_writes: None });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind, &[]);
                pass.dispatch_workgroups((width + 7) / 8, (height + 7) / 8, 1);
            }
            queue.submit(std::iter::once(enc.finish()));
            let _ = device.poll(wgpu::Maintain::Wait);
            frame_idx += 1;
        }

        // copy out
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("PT Copy") });
        enc.copy_texture_to_buffer(
            wgpu::ImageCopyTexture { texture: &accum, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::ImageCopyBuffer {
                buffer: &readback,
                layout: wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(height) },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        queue.submit(std::iter::once(enc.finish()));
        let _ = device.poll(wgpu::Maintain::Wait);

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::Maintain::Wait);
        let data = slice.get_mapped_range().to_vec();

        // decode rgba16float -> rgba8 with simple tonemap
        let mut img = RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let base = ((y * padded + x * 8) / 4) as usize;
                if base + 1 >= data.len() / 4 {
                    continue;
                }
                let px = decode_f16_pair(&data, (y * padded + x * 8) as usize);
                img.put_pixel(x, y, Rgba([
                    (px[0].clamp(0.0, 1.0) * 255.0) as u8,
                    (px[1].clamp(0.0, 1.0) * 255.0) as u8,
                    (px[2].clamp(0.0, 1.0) * 255.0) as u8,
                    255,
                ]));
            }
        }
        let _ = center;
        Ok(img)
    })
}

/// Decode two f16 values (rg) then two more (ba) from bytes.
fn decode_f16_pair(data: &[u8], offset: usize) -> [f32; 4] {
    let f16_to_f32 = |bits: u16| -> f32 {
        let sign = ((bits >> 15) & 1) as u32;
        let exp = ((bits >> 10) & 0x1F) as u32;
        let frac = (bits & 0x3FF) as u32;
        if exp == 0 {
            return if frac == 0 { 0.0 } else { (frac as f32) / 1024.0 * 2.0f32.powi(-14) } * if sign == 1 { -1.0 } else { 1.0 };
        }
        if exp == 0x1F {
            return f32::NAN;
        }
        let val = ((exp + 112) << 23) | (frac << 13);
        f32::from_bits((sign << 31) | val)
    };
    let mut out = [0.0f32; 4];
    for i in 0..4 {
        let o = offset + i * 2;
        if o + 1 < data.len() {
            out[i] = f16_to_f32(u16::from_le_bytes([data[o], data[o + 1]]));
        }
    }
    out
}
