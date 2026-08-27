use std::sync::Arc;
use winit::{
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowAttributes,
};

use crate::camera::Camera;
use crate::scene::{GpuScene, GpuVertex, MeshBatch};

struct DemoApp {
    state: Option<State>,
}

impl winit::application::ApplicationHandler for DemoApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // (re)create the window and GPU state
        let window = event_loop
            .create_window(
                WindowAttributes::new()
                    .with_title("LOREFORGE")
                    .with_inner_size(winit::dpi::LogicalSize::new(1024, 768)),
            )
            .expect("Window build failed");
        self.state = Some(pollster::block_on(State::new(Arc::new(window))));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: winit::window::WindowId, event: WindowEvent) {
        let Some(state) = &mut self.state else { return };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.update();
                state.render();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        let Some(state) = &mut self.state else { return };
        state.window.request_redraw();
    }
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let event_loop = EventLoop::new().expect("EventLoop failed");
    let mut app = DemoApp { state: None };
    event_loop.run_app(&mut app);
}

struct State {
    window: Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth_view: wgpu::TextureView,
    _depth_texture: wgpu::Texture,
    scene: GpuScene,
    camera: Camera,
}

impl State {
    async fn new(window: Arc<winit::window::Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).expect("create surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("request adapter");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
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
            .expect("request device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(caps.formats[0]);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Demo scene: one grass-topped section until the world streams in (P2).
        let mut section = lf_voxel::VoxelSection::new_empty();
        for x in 0..16 {
            for z in 0..16 {
                section.set(x, 0, z, lf_voxel::BlockState::STONE);
                section.set(x, 1, z, lf_voxel::BlockState::DIRT);
                section.set(x, 2, z, lf_voxel::BlockState::GRASS);
            }
        }
        let mesh = lf_voxel::meshing::mesh_section(&section, None, None, None, None, None, None,
            &|b, face| lf_assets::texture_index_for_face(b.id(), face), &|_, _, _| 0xF0);
        let vertices: Vec<GpuVertex> = mesh.vertices.iter().map(|v| GpuVertex {
            position: v.position,
            normal: v.normal,
            tex_coord: v.tex_coord,
            tex_index: v.tex_index,
            ao: v.ao,
            light: v.light,
            sway: v.sway,
        }).collect();

        let textures = lf_assets::generate_atlas();
        let scene = GpuScene::new(&device, &queue, config.format, &vertices, &mesh.indices, &textures);

        let (depth_texture, depth_view) = MeshBatch::create_depth_texture(&device, config.width, config.height);

        let mut camera = Camera::new(
            glam::Vec3::new(24.0, 18.0, 24.0),
            glam::Vec3::new(8.0, 2.0, 8.0),
        );
        camera.set_aspect(config.width, config.height);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            depth_view,
            _depth_texture: depth_texture,
            scene,
            camera,
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.camera.set_aspect(width, height);
            self.surface.configure(&self.device, &self.config);
            let (texture, view) = MeshBatch::create_depth_texture(&self.device, width, height);
            self._depth_texture = texture;
            self.depth_view = view;
        }
    }

    fn update(&mut self) {
        let env = crate::scene::Env {
            camera_pos: self.camera.eye,
            day_factor: 1.0,
            fog_color: [0.53, 0.81, 0.98],
            fog_far: 1000.0,
            time: 0.0, // one-chunk demo has no foliage to animate
            grade_tint: [1.0, 1.0, 1.0],
            grade_saturation: 1.0,
        };
        self.scene.update_camera(&self.queue, &self.camera, &env);
    }

    fn render(&mut self) {
        let output = match self.surface.get_current_texture() {
            Ok(o) => o,
            Err(wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(e) => {
                tracing::warn!("surface error: {:?}", e);
                return;
            }
        };
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.53, g: 0.81, b: 0.98, a: 1.0 }),
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
            self.scene.draw(&mut render_pass);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        let _ = self.window.pre_present_notify();
    }
}
