use std::path::Path;

use image::{Rgba, RgbaImage};

use crate::camera::Camera;
use crate::scene::{Env, GpuScene, GpuVertex, MeshBatch};

/// An egui overlay to composite over the scene, plus any texture deltas
/// captured from warmup passes. Warmup passes are required before windows
/// materialize (a fresh single-pass context produces empty window shapes),
/// but their `end_pass` output carries the font-atlas texture delta away —
/// pass it back here or every font/white-texture draw vanishes.
pub struct UiOverlay<'a> {
    pub ctx: &'a egui::Context,
    pub extra_textures: &'a [(egui::TextureId, epaint::ImageDelta)],
}

/// Renders `vertices`/`indices` offscreen (no window or surface) and writes a
/// PNG to `out_path`. This is the backbone of honest proof screenshots: the
/// image always comes from the real renderer fed by real game data.
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
            .ok_or_else(|| "no GPU adapter available".to_string())?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Headless Device"),
                    required_features: wgpu::Features::TEXTURE_BINDING_ARRAY
                        | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                    required_limits: wgpu::Limits {
                        max_sampled_textures_per_shader_stage: 32,
                        ..wgpu::Limits::default()
                    },
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .map_err(|e| format!("device request failed: {e:?}"))?;

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let scene = GpuScene::new(&device, &queue, format, vertices, indices, textures);
        let water_scene = if water_vertices.is_empty() {
            None
        } else {
            Some(GpuScene::new(&device, &queue, format, water_vertices, water_indices, textures))
        };
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

        scene.update_camera(&queue, camera, env);
        if let Some(ws) = &water_scene {
            ws.update_camera(&queue, camera, env);
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Headless Encoder") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Headless Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: clear[0], g: clear[1], b: clear[2], a: clear[3] }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            scene.draw(&mut pass);
            if let Some(ws) = &water_scene {
                ws.batch.draw(&mut pass, &ws.resources, true);
            }
        }

        // Optional egui overlay (honest UI proof screenshots).
        if let Some(overlay) = ui {
            let ctx = overlay.ctx;
            let full_output = ctx.end_pass();
            let jobs = ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
            let screen = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [width, height],
                pixels_per_point: full_output.pixels_per_point,
            };
            let mut egui_renderer = egui_wgpu::Renderer::new(&device, format, None, 1, true);
            // warmup-pass textures first (includes the font atlas), then any
            // textures the final pass itself added
            for (id, delta) in overlay.extra_textures {
                egui_renderer.update_texture(&device, &queue, *id, delta);
            }
            for (id, delta) in &full_output.textures_delta.set {
                egui_renderer.update_texture(&device, &queue, *id, delta);
            }
            egui_renderer.update_buffers(&device, &queue, &mut encoder, &jobs, &screen);
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui Headless Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &color_view,
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
        let unpadded = width * bytes_per_pixel;
        let padded = ((unpadded + 255) / 256) * 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Readback Buffer"),
            size: (padded * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::Maintain::Wait);
        let data = slice.get_mapped_range().to_vec();
        buffer.unmap();

        // Unpad into an image and save.
        let mut img = RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let src = (y * padded + x * bytes_per_pixel) as usize;
                img.put_pixel(x, y, Rgba([data[src], data[src + 1], data[src + 2], data[src + 3]]));
            }
        }
        img.save(out_path).map_err(|e| format!("failed to save PNG: {e}"))
    })
}
