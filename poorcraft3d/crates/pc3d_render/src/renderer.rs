//! The windowed/offscreen renderer core: pipelines, frame submission, and
//! screenshot readback.
//!
//! One draw path ([`encode_frame`]) serves three targets: the live window
//! surface, the live-window screenshot copy (surface configured with
//! `COPY_SRC`), and the offscreen proof target used by automated tests. That
//! keeps the tested pixels and the displayed pixels identical.

use crate::gpu::{surface_action, GpuContext, SurfaceAction, SurfaceGuard};
pub use crate::scene::PixelReport;
use std::path::Path;
use std::sync::Arc;
use wgpu::util::DeviceExt;

const SHADER: &str = include_str!("../shaders/scene.wgsl");

struct Pipelines {
    sky: wgpu::RenderPipeline,
    scene: wgpu::RenderPipeline,
}

impl Pipelines {
    fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pc3d_render scene.wgsl"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pc3d_render layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let make = |vs: &str,
                    fs: &str,
                    vertex_buffers: &[wgpu::VertexBufferLayout<'static>],
                    cull: Option<wgpu::Face>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("pc3d_render pipeline"),
                layout: Some(&layout),
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
                        blend: None,
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
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        Self {
            sky: make("vs_sky", "fs_sky", &[], None),
            scene: make(
                "vs_scene",
                "fs_scene",
                std::slice::from_ref(&crate::scene::VERTEX_LAYOUT),
                Some(wgpu::Face::Back),
            ),
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
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene vertices"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene indices"),
            contents: bytemuck::cast_slice(&idx),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            vertex_buffer,
            index_buffer,
            index_count: idx.len() as u32,
        }
    }
}

/// The single draw path shared by the window, the live-window screenshot, and
/// the offscreen proof target: sky gradient + sun, then the indexed banner
/// mesh.
fn encode_frame(
    pipelines: &Pipelines,
    gpu_scene: &GpuScene,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
) {
    let clear = wgpu::Color {
        r: 0.13,
        g: 0.27,
        b: 0.42,
        a: 1.0,
    };
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("pc3d_render pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    // Pass 1: sky gradient + sun (fullscreen triangle, no bindings yet).
    pass.set_pipeline(&pipelines.sky);
    pass.draw(0..3, 0..1);
    // Pass 2: indexed vertex-colored banner mesh, backface culled.
    pass.set_pipeline(&pipelines.scene);
    pass.set_vertex_buffer(0, gpu_scene.vertex_buffer.slice(..));
    pass.set_index_buffer(gpu_scene.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
    pass.draw_indexed(0..gpu_scene.index_count, 0, 0..1);
}

pub struct Renderer {
    ctx: GpuContext,
    surface: Option<SurfaceGuard>,
    /// Kept so a `Lost` surface can be recreated against the same window.
    window: Option<Arc<winit::window::Window>>,
    format: wgpu::TextureFormat,
    pipelines: Pipelines,
    gpu_scene: GpuScene,
    /// Offscreen proof target (tests): a COPY_SRC render target standing in
    /// for the swapchain.
    offscreen: Option<wgpu::Texture>,
}

impl Renderer {
    /// Live windowed renderer: surface owns the swapchain, sized to the window.
    pub fn windowed(window: &Arc<winit::window::Window>) -> Self {
        let ctx = GpuContext::new(None);
        let guard = SurfaceGuard::new(&ctx, window);
        let format = guard.format();
        let pipelines = Pipelines::new(&ctx.device, format);
        let gpu_scene = GpuScene::new(&ctx.device);
        Self {
            ctx,
            surface: Some(guard),
            window: Some(window.clone()),
            format,
            pipelines,
            gpu_scene,
            offscreen: None,
        }
    }

    /// Headless proof renderer: same pipelines and scene, render target
    /// instead of a swapchain, so `cargo test` exercises the exact GPU path.
    pub fn offscreen(width: u32, height: u32) -> Self {
        let ctx = GpuContext::new(None);
        let format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let pipelines = Pipelines::new(&ctx.device, format);
        let gpu_scene = GpuScene::new(&ctx.device);
        let texture = create_target(&ctx.device, width, height, format);
        Self {
            ctx,
            surface: None,
            window: None,
            format,
            pipelines,
            gpu_scene,
            offscreen: Some(texture),
        }
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

    /// Reconfigure the active target for a new size. Zero-sized axes are
    /// clamped (wgpu refuses 0-width/height configurations).
    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some(guard) = self.surface.as_mut() {
            guard.resize(&self.ctx.device, width, height);
        } else if let Some(old) = self.offscreen.take() {
            let (w, h) = crate::gpu::clamp_size(width, height);
            if old.width() != w || old.height() != h {
                self.offscreen = Some(create_target(&self.ctx.device, w, h, self.format));
            } else {
                self.offscreen = Some(old);
            }
        }
    }

    /// Renders one frame. Windowed: acquire → draw → present, with the
    /// documented recovery policy for lost/outdated/timeout surfaces.
    /// Offscreen: draws into the proof target.
    pub fn render_frame(&mut self) -> Result<(), wgpu::SurfaceError> {
        if self.surface.is_some() {
            let mut attempt = 0;
            let output = loop {
                // The texture is owned, so the borrow of self ends here and
                // recovery can mutate the surface.
                let err = match self.surface.as_ref().unwrap().surface.get_current_texture()
                {
                    Ok(t) => break t,
                    Err(e) => e,
                };
                attempt += 1;
                match surface_action(&err) {
                    SurfaceAction::RecreateRetry if attempt == 1 => {
                        let window = self.window.clone().expect("windowed renderer window");
                        self.surface
                            .as_mut()
                            .unwrap()
                            .recreate(&self.ctx, &window);
                    }
                    SurfaceAction::ReconfigureRetry if attempt == 1 => {
                        self.surface
                            .as_ref()
                            .unwrap()
                            .reconfigure(&self.ctx.device);
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
            encode_frame(&self.pipelines, &self.gpu_scene, &mut encoder, &view);
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
            encode_frame(&self.pipelines, &self.gpu_scene, &mut encoder, &view);
            self.ctx.queue.submit(Some(encoder.finish()));
        }
        Ok(())
    }

    /// Renders a frame and saves it as a PNG. Windowed: the copy comes from
    /// the actual swapchain texture (the surface carries COPY_SRC), so this
    /// is a screenshot of the presented frame, not a separate render.
    /// Offscreen: copies the proof target. Returns the semantic pixel report.
    pub fn capture_png(&mut self, path: &Path) -> PixelReport {
        let (width, height) = self.size();
        assert!(width >= 1 && height >= 1, "cannot capture a 0-sized frame");

        // Copy layout: bytes_per_row must be a multiple of 256.
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

        let mut size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        if self.surface.is_some() {
            let mut attempt = 0;
            let output = loop {
                let err = match self.surface.as_ref().unwrap().surface.get_current_texture()
                {
                    Ok(t) => break t,
                    Err(e) => e,
                };
                attempt += 1;
                match surface_action(&err) {
                    SurfaceAction::RecreateRetry if attempt == 1 => {
                        let window = self.window.clone().expect("windowed renderer window");
                        self.surface
                            .as_mut()
                            .unwrap()
                            .recreate(&self.ctx, &window);
                    }
                    SurfaceAction::ReconfigureRetry if attempt == 1 => {
                        self.surface
                            .as_ref()
                            .unwrap()
                            .reconfigure(&self.ctx.device);
                    }
                    _ => panic!("surface unavailable for capture: {err:?}"),
                }
            };
            size = output.texture.size();
            let view = output.texture.create_view(&Default::default());
            encode_frame(&self.pipelines, &self.gpu_scene, &mut encoder, &view);
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
                size,
            );
            self.ctx.queue.submit(Some(encoder.finish()));
            // Wait for the copy before presenting, which may recycle the
            // swapchain texture.
            let _ = self.ctx.device.poll(wgpu::Maintain::Wait);
            output.present();
        } else {
            let tex = self.offscreen.as_ref().expect("renderer has no target");
            let view = tex.create_view(&Default::default());
            encode_frame(&self.pipelines, &self.gpu_scene, &mut encoder, &view);
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
                size,
            );
            self.ctx.queue.submit(Some(encoder.finish()));
            let _ = self.ctx.device.poll(wgpu::Maintain::Wait);
        }

        let rgba = read_buffer(&self.ctx.device, &readback, bytes_per_row, width, height);
        image::save_buffer(path, &rgba, width, height, image::ColorType::Rgba8)
            .expect("write screenshot png");
        crate::scene::verify_frame_rgba(&rgba, width, height)
    }
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
// Tests: real GPU frames through the same draw path the window uses.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const W: u32 = 384;
    const H: u32 = 288;

    #[test]
    fn offscreen_gpu_frame_matches_scene_and_is_deterministic() {
        let mut renderer = Renderer::offscreen(W, H);
        let path = std::env::temp_dir().join("pc3d_render_offscreen_proof_a.png");

        let t0 = std::time::Instant::now();
        let report_a = renderer.capture_png(&path);
        let capture_us = t0.elapsed().as_secs_f64() * 1e6;

        println!(
            "offscreen capture {W}x{H}: {} distinct colors, {capture_us:.0} us frame+readback+encode",
            report_a.distinct_colors
        );
        assert!(
            report_a.passes(),
            "offscreen frame failed semantic verification: {report_a:?}"
        );

        // Determinism: the same renderer, asked twice, must produce the same
        // pixels byte-for-byte.
        let bytes_a = std::fs::read(&path).expect("read proof png");
        let path_b = std::env::temp_dir().join("pc3d_render_offscreen_proof_b.png");
        let report_b = renderer.capture_png(&path_b);
        let bytes_b = std::fs::read(&path_b).expect("read second proof png");
        assert_eq!(report_a, report_b);
        assert_eq!(bytes_a, bytes_b, "GPU frames differ between runs");
    }

    #[test]
    fn offscreen_resize_reconfigures_the_render_target() {
        let mut renderer = Renderer::offscreen(W, H);
        renderer.resize(256, 192);
        assert_eq!(renderer.size(), (256, 192));

        let path = std::env::temp_dir().join("pc3d_render_offscreen_resized.png");
        let report = renderer.capture_png(&path);
        assert_eq!(report.width, 256);
        assert_eq!(report.height, 192);
        assert!(
            report.passes(),
            "resized frame failed semantic verification: {report:?}"
        );

        // Zero-sized resize must clamp, not poison the target.
        renderer.resize(0, 0);
        assert_eq!(renderer.size(), (1, 1));
    }
}
