use std::path::Path;

use image::{Rgba, RgbaImage};

use crate::camera::Camera;
use crate::scene::{Env, MeshBatch, SceneResources, GpuVertex};

/// An egui overlay to composite over the scene, plus any texture deltas
/// captured from warmup passes. Warmup passes are required before windows
/// materialize (a fresh single-pass context produces empty window shapes),
/// but their `end_pass` output carries the font-atlas texture delta away —
/// pass it back here or every font/white-texture draw vanishes.
pub struct UiOverlay<'a> {
    pub ctx: &'a egui::Context,
    pub extra_textures: &'a [(egui::TextureId, epaint::ImageDelta)],
}

/// A persistent offscreen renderer: device, atlas and render targets are
/// created once; [`HeadlessRenderer::render`] frames reuse them. This is
/// what makes the perf benchmark measure frame cost instead of setup, and
/// stays the backbone of honest proof screenshots.
pub struct HeadlessRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    resources: SceneResources,
    format: wgpu::TextureFormat,
    color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    width: u32,
    height: u32,
}

impl HeadlessRenderer {
    pub fn new(width: u32, height: u32, textures: &[RgbaImage]) -> Result<Self, String> {
        let init = pollster::block_on(async {
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
                .ok_or_else(|| "no GPU adapter available".to_string())?;
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("Headless Device"),
                        required_features: wgpu::Features::TEXTURE_BINDING_ARRAY
                            | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                        required_limits: wgpu::Limits {
                            max_sampled_textures_per_shader_stage: 32,
                            // king-quest asset pass: see the client device
                            max_texture_array_layers: 512,
                            ..wgpu::Limits::default()
                        },
                        memory_hints: wgpu::MemoryHints::default(),
                    },
                    None,
                )
                .await
                .map_err(|e| format!("device request failed: {e:?}"))?;
            Ok((device, queue))
        });
        let (device, queue) = match init {
            Ok(pair) => pair,
            Err(e) => return Err(e),
        };

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let resources = SceneResources::new(&device, &queue, format, textures);
        let (_, depth_view) = MeshBatch::create_depth_texture(&device, width, height);
        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Headless Color"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Self { device, queue, resources, format, color_texture, color_view, depth_view, width, height })
    }

    /// Render one frame (vertices uploaded fresh — same shape as a live
    /// re-mesh) and write it to `out_path`.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        vertices: &[GpuVertex],
        indices: &[u32],
        water_vertices: &[GpuVertex],
        water_indices: &[u32],
        camera: &Camera,
        env: &Env,
        clear: [f64; 4],
        out_path: &Path,
        ui: Option<&UiOverlay>,
    ) -> Result<(), String> {
        let scene = MeshBatch::new(&self.device, &self.resources, vertices, indices);
        let water = if water_vertices.is_empty() {
            None
        } else {
            Some(MeshBatch::new(&self.device, &self.resources, water_vertices, water_indices))
        };
        scene.update_camera(&self.queue, camera, env);
        if let Some(w) = &water {
            w.update_camera(&self.queue, camera, env);
        }

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Headless Encoder") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Headless Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: clear[0], g: clear[1], b: clear[2], a: clear[3] }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            scene.draw(&mut pass, &self.resources, false);
            if let Some(w) = &water {
                w.draw(&mut pass, &self.resources, true);
            }
        }

        // Optional egui overlay (honest UI proof screenshots).
        if let Some(overlay) = ui {
            let ctx = overlay.ctx;
            let full_output = ctx.end_pass();
            let jobs = ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
            let screen = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.width, self.height],
                pixels_per_point: full_output.pixels_per_point,
            };
            let mut egui_renderer = egui_wgpu::Renderer::new(&self.device, self.format, None, 1, true);
            // warmup-pass textures first (includes the font atlas), then any
            // textures the final pass itself added
            for (id, delta) in overlay.extra_textures {
                egui_renderer.update_texture(&self.device, &self.queue, *id, delta);
            }
            for (id, delta) in &full_output.textures_delta.set {
                egui_renderer.update_texture(&self.device, &self.queue, *id, delta);
            }
            egui_renderer.update_buffers(&self.device, &self.queue, &mut encoder, &jobs, &screen);
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui Headless Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.color_view,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                let mut pass: wgpu::RenderPass<'static> = unsafe { std::mem::transmute(pass) };
                egui_renderer.render(&mut pass, &jobs, &screen);
                drop(pass);
            }
        }

        // Copy the color texture into a CPU-mappable buffer. Rows must be
        // padded to 256-byte alignment.
        let bytes_per_pixel = 4;
        let unpadded = self.width * bytes_per_pixel;
        let padded = ((unpadded + 255) / 256) * 256;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Readback Buffer"),
            size: (padded * self.height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &self.color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::Maintain::Wait);
        let data = slice.get_mapped_range().to_vec();
        buffer.unmap();

        // Unpad into an image and save.
        let mut img = RgbaImage::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let src = (y * padded + x * bytes_per_pixel) as usize;
                img.put_pixel(x, y, Rgba([data[src], data[src + 1], data[src + 2], data[src + 3]]));
            }
        }
        img.save(out_path).map_err(|e| format!("failed to save PNG: {e}"))
    }
}

/// One-shot render (creates the renderer, renders one frame, drops it).
/// The backbone of honest proof screenshots; use [`HeadlessRenderer`]
/// directly when rendering many frames.
#[allow(clippy::too_many_arguments)]
pub fn render_to_png(
    vertices: &[GpuVertex],
    indices: &[u32],
    water_vertices: &[GpuVertex],
    water_indices: &[u32],
    textures: &[RgbaImage],
    camera: &Camera,
    env: &Env,
    clear: [f64; 4],
    width: u32,
    height: u32,
    out_path: &Path,
    ui: Option<&UiOverlay>,
) -> Result<(), String> {
    let renderer = HeadlessRenderer::new(width, height, textures)?;
    renderer.render(vertices, indices, water_vertices, water_indices, camera, env, clear, out_path, ui)
}
