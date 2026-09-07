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

struct Pipelines {
    sky: wgpu::RenderPipeline,
    mesh: wgpu::RenderPipeline,
    hud: wgpu::RenderPipeline,
    water: wgpu::RenderPipeline,
    layout_globals: wgpu::BindGroupLayout,
    layout_hud: wgpu::BindGroupLayout,
}

impl Pipelines {
    fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pc3d_render scene.wgsl"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout_globals =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("pc3d globals layout"),
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
                Some(depth24),
                None,
            ),
            hud: make(
                "pc3d hud pipeline",
                &pl_hud,
                "vs_hud",
                "fs_hud",
                std::slice::from_ref(&hud_layout),
                None,
                Some(depth_off),
                Some(wgpu::BlendState::ALPHA_BLENDING),
            ),
            water,
            layout_globals,
            layout_hud,
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
    /// River water from flow records (R3DV-007).
    water: Option<crate::water::WaterSections>,
    start: std::time::Instant,
    /// Frozen water time for deterministic proofs (None = wall clock).
    water_time_override: Option<f32>,
}

/// A plain vertex/index buffer pair.
struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl GpuMesh {
    fn from_mesh(device: &wgpu::Device, verts: &[crate::scene::SceneVertex], idx: &[u16]) -> Self {
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
        }
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
        let bg_globals = create_globals_bind_group(&ctx.device, &pipelines.layout_globals, &globals_buf);
        let mut hud = create_hud(&ctx.device, &pipelines.layout_hud);
        hud.line = default_hud_line(&camera);
        let depth = Some(create_depth(&ctx.device, w, h));
        let bg_hud = create_hud_bind_group(
            &ctx.device,
            &pipelines.layout_hud,
            &globals_buf,
            &hud.texture,
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
            depth,
            offscreen: None,
            construction: None,
            terrain: None,
            streamer: None,
            water: None,
            start: std::time::Instant::now(),
            water_time_override: None,
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
        let bg_globals = create_globals_bind_group(&ctx.device, &pipelines.layout_globals, &globals_buf);
        let mut hud = create_hud(&ctx.device, &pipelines.layout_hud);
        hud.line = default_hud_line(&camera);
        let texture = create_target(&ctx.device, width, height, format);
        let depth = Some(create_depth(&ctx.device, width, height));
        let bg_hud = create_hud_bind_group(
            &ctx.device,
            &pipelines.layout_hud,
            &globals_buf,
            &hud.texture,
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
            depth,
            offscreen: Some(texture),
            construction: None,
            terrain: None,
            streamer: None,
            water: None,
            start: std::time::Instant::now(),
            water_time_override: None,
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
            let base = verts.len() as u16;
            verts.extend(v);
            idx.extend(i.iter().map(|k| k + base));
        }
        let stats = crate::terrain::TerrainStats {
            patches: patches.len(),
            vertices: verts.len(),
            triangles: idx.len() / 3,
            mesh_us: t0.elapsed().as_micros(),
        };
        self.terrain = Some(GpuMesh::from_mesh(&self.ctx.device, &verts, &idx));
        stats
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

    /// Streaming counters (None when not attached).
    pub fn stream_counters(&self) -> Option<crate::streaming::StreamCounters> {
        self.streamer.as_ref().map(|s| s.counters())
    }

    /// Test hook: the streamer's current desired set.
    pub fn stream_desired_debug(
        &self,
    ) -> Option<std::collections::BTreeMap<pc3d_world::coords::PatchCoord, pc3d_world::lod::LodLevel>> {
        self.streamer
            .as_ref()
            .map(|s| s.debug_desired())
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
                0.0,
            ],
        };
        self.ctx
            .queue
            .write_buffer(&self.globals_buf, 0, bytemuck::bytes_of(&globals));

        // HUD: re-rasterize the line and refresh the quad to the target size.
        let (bytes, tw, th) = crate::font::rasterize_line(&self.hud.line.clone(), HUD_SCALE);
        if (tw, th) != self.hud.size {
            self.hud.texture = create_hud_texture(&self.ctx.device, tw, th);
            self.hud.size = (tw, th);
            self.bg_hud = create_hud_bind_group(
                &self.ctx.device,
                &self.pipelines.layout_hud,
                &self.globals_buf,
                &self.hud.texture,
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
        let (x0, y1, x1, y0) = hud_quad_ndc(tw, th, w, h);
        let quad: [[f32; 4]; 4] = [
            [x0, y1, 0.0, 0.0],
            [x1, y1, 1.0, 0.0],
            [x0, y0, 0.0, 1.0],
            [x1, y0, 1.0, 1.0],
        ];
        self.ctx.queue.write_buffer(
            &self.hud.vertex_buffer,
            0,
            bytemuck::cast_slice(&quad),
        );
    }

    /// Renders one frame. Windowed: acquire → draw → present, with the
    /// documented recovery policy for lost/outdated/timeout surfaces.
    /// Offscreen: draws into the proof target.
    pub fn render_frame(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.prepare_frame();
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
            let view = output.texture.create_view(&Default::default());
            self.encode_frame(&mut encoder, &view);
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
            let tex = self
                .offscreen
                .take()
                .expect("renderer has no target");
            let view = tex.create_view(&Default::default());
            self.encode_frame(&mut encoder, &view);
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
        pass.set_pipeline(&self.pipelines.sky);
        pass.set_bind_group(0, &self.bg_globals, &[]);
        pass.draw(0..3, 0..1);
        // 2. Depth-tested, sunlit world geometry from P3D coordinates.
        // The mesh pipeline binds once; the placeholder scene draws only
        // when present (hidden construction runs have empty buffers, and
        // wgpu refuses zero-size slices), then construction patches draw
        // through the same lit pipeline.
        pass.set_pipeline(&self.pipelines.mesh);
        pass.set_bind_group(0, &self.bg_globals, &[]);
        if self.gpu_scene.index_count > 0 {
            pass.set_vertex_buffer(0, self.gpu_scene.vertex_buffer.slice(..));
            pass.set_index_buffer(self.gpu_scene.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..self.gpu_scene.index_count, 0, 0..1);
        }
        // 2a. Natural terrain: streamed patches (R3DV-006, frustum-culled,
        // per-patch buffers) or the static load (R3DV-005).
        let view_proj: [f32; 16] = self.camera.view_proj(self.aspect());
        if let Some(streamer) = self.streamer.as_mut() {
            streamer.draw(&mut pass, &view_proj);
        } else if let Some(t) = &self.terrain {
            pass.set_vertex_buffer(0, t.vertex_buffer.slice(..));
            pass.set_index_buffer(t.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..t.index_count, 0, 0..1);
        }
        // 2b. Host-owned construction blocks (same lit pipeline; per-patch
        // buffers with content versions — only changed patches are remeshed).
        if let Some(con) = &self.construction {
            con.draw(&mut pass);
        }
        // 2c. Transparent river water LAST among world geometry: depth-read
        // only, alpha blend — banks show through, terrain occludes.
        if let Some(w) = &self.water {
            pass.set_pipeline(&self.pipelines.water);
            pass.set_bind_group(0, &self.bg_globals, &[]);
            w.draw(&mut pass);
        }
        // 3. HUD debug line (alpha blend, no depth).
        pass.set_pipeline(&self.pipelines.hud);
        pass.set_bind_group(0, &self.bg_hud, &[]);
        pass.set_vertex_buffer(0, self.hud.vertex_buffer.slice(..));
        pass.draw(0..4, 0..1);
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
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("pc3d globals bind group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buf.as_entire_binding(),
        }],
    })
}

fn create_hud(device: &wgpu::Device, _layout: &wgpu::BindGroupLayout) -> HudResources {
    let (_, tw, th) = crate::font::rasterize_line(&" ".repeat(HUD_LINE_CHARS), HUD_SCALE);
    let texture = create_hud_texture(device, tw, th);
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pc3d hud quad"),
        size: 4 * 16,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    HudResources {
        texture,
        size: (tw, th),
        vertex_buffer,
        line: String::new(),
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
        view_formats: &[],
    })
}

fn create_hud_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    globals_buf: &wgpu::Buffer,
    texture: &wgpu::Texture,
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
                    &texture.create_view(&Default::default()),
                ),
            },
        ],
    })
}

/// Top-left pixel-anchored HUD quad in NDC.
fn hud_quad_ndc(tw: u32, th: u32, target_w: u32, target_h: u32) -> (f32, f32, f32, f32) {
    let margin = 8.0;
    let x0 = -1.0 + margin * 2.0 / target_w as f32;
    let y1 = 1.0 - margin * 2.0 / target_h as f32;
    let x1 = x0 + tw as f32 * 2.0 / target_w as f32;
    let y0 = y1 - th as f32 * 2.0 / target_h as f32;
    (x0, y1, x1, y0)
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
        println!("parallax: {:.1}% of pixels differ between poses A and B", diff * 100.0);
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
        assert!(report.passes(), "resized frame failed: {:?}", report.failed_probes());
        // Zero-sized resize must clamp, not poison the target.
        r.resize(0, 0);
        assert_eq!(r.size(), (1, 1));
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
        use pc3d_world::coords::CellCoord;
        use pc3d_world::gen::CellMaterial;
        use pc3d_world::host::HostCommand;
        use crate::scene::{dir_from_ndc, lit_color, project_ndc, sky_color_linear, to_srgb4, Probe, SUN_DIR};

        let mut host = pc3d_world::host::SoloHost::new(4242);
        build_wall(&mut host);
        // 8 blocks, all in patch (0, 0, -1).
        assert_eq!(host.construction.len(), 1);
        assert_eq!(
            host.construction.values().next().unwrap().built_count(),
            8
        );

        let mut r = Renderer::offscreen(W, H);
        r.set_placeholder_scene(false); // construction is the only world geometry
        r.attach_construction();
        let s0 = r.update_construction(&host.construction);
        assert_eq!((s0.added, s0.remeshed, s0.inspected), (1, 0, 1));
        let pose = wall_pose();
        r.set_pose(pose);

        // The (0,1,-8) block's south-face center projects near screen center.
        let target = project_ndc(pose, ASPECT, [2.5, 1.5, -7.0]);
        assert!(target.0.abs() < 0.2 && target.1.abs() < 0.3, "probe at {target:?}");
        let rock_face = to_srgb4(lit_color(material_albedo(CellMaterial::Rock), [0.0, 0.0, 1.0]));
        let sand_face = to_srgb4(lit_color(material_albedo(CellMaterial::Sand), [0.0, 0.0, 1.0]));

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
        assert!(delta > 0.15, "edited cell barely changed: {p_before:?} vs {p_after:?}");
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
        use crate::scene::{dir_from_ndc, Probe, SUN_DIR, sky_color_linear, to_srgb4, project_ndc};
        use crate::terrain::{column_top, face_expectation, overview_pose};
        use pc3d_world::coords::{CellCoord, PatchCoord};
        use pc3d_world::terrain::SceneSpec;

        let (seed, coord) = SceneSpec::SmoothHills.patch();
        let gen = pc3d_world::gen::WorldGen::new(seed);
        let mut r = Renderer::offscreen(W, H);
        r.set_placeholder_scene(false);
        let stats = r.load_terrain(&gen, &[coord]);
        assert_eq!(stats.patches, 1);
        assert!(stats.triangles > 500, "expected a real hill mesh: {stats:?}");
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
                    let east = CellCoord { x: cell.x + 1, y: cell.y, z: cell.z };
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
                            [cell.x as f32 + 1.0, cell.y as f32 + 0.5, cell.z as f32 + 0.5],
                        ));
                        break;
                    }
                }
            }
            if slope.is_some() {
                break;
            }
        }
        let (slope_cell, slope_normal, slope_point) =
            slope.expect("a hill slope step must exist");

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
            assert!(p.ndc.0 > -0.99 && p.ndc.0 < 0.99 && p.ndc.1 > -0.99 && p.ndc.1 < 0.99,
                "probe {} off-screen at {:?}", p.name, p.ndc);
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
        use crate::scene::{dir_from_ndc, Probe, SUN_DIR, sky_color_linear, to_srgb4, project_ndc};
        use crate::terrain::{cave_pose, face_expectation, find_cave_pocket};
        use pc3d_world::coords::PatchCoord;
        use pc3d_world::terrain::SceneSpec;

        // Find a camera-friendly cave pocket near a scene patch.
        let (seed, coord) = SceneSpec::Highlands.patch();
        let gen = pc3d_world::gen::WorldGen::new(seed);
        let (air, wall, dir) =
            crate::terrain::find_cave_pocket_near(&gen, coord, 1)
                .or_else(|| {
                    let (s, c) = SceneSpec::SmoothHills.patch();
                    let _ = s;
                    crate::terrain::find_cave_pocket_near(
                        &pc3d_world::gen::WorldGen::new(3),
                        c,
                        1,
                    )
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
        let ceiling = pc3d_world::coords::CellCoord { x: air.x, y: air.y + 1, z: air.z };
        let corridor = pc3d_world::coords::CellCoord {
            x: air.x + dir[0],
            y: air.y,
            z: air.z + dir[2],
        };
        let corridor_ceiling =
            pc3d_world::coords::CellCoord { x: corridor.x, y: corridor.y + 1, z: corridor.z };
        let wall_face_point = [
            wall.x as f32 + 0.5 - dir[0] as f32 * 0.5,
            wall.y as f32 + 0.5,
            wall.z as f32 + 0.5 - dir[2] as f32 * 0.5,
        ];
        let probes = vec![
            Probe {
                name: "cave_wall_face",
                ndc: project_ndc(pose, ASPECT, wall_face_point),
                expected: face_expectation(
                    &gen,
                    wall,
                    [-dir[0] as f32, 0.0, -dir[2] as f32],
                ),
                tol: 0.06,
            },
            Probe {
                name: "cave_ceiling_overhang_underside",
                ndc: project_ndc(
                    pose,
                    ASPECT,
                    [corridor.x as f32 + 0.5, corridor.y as f32 + 1.0, corridor.z as f32 + 0.5],
                ),
                expected: face_expectation(&gen, corridor_ceiling, [0.0, -1.0, 0.0]),
                tol: 0.06,
            },
        ];
        for p in &probes {
            assert!(p.ndc.0 > -0.99 && p.ndc.0 < 0.99 && p.ndc.1 > -0.99 && p.ndc.1 < 0.99,
                "probe {} off-screen at {:?}", p.name, p.ndc);
        }
        let path = std::env::temp_dir().join("pc3d_terrain_cave.png");
        let (report, rgba) = r.capture_png(&path, &probes);
        for p in &probes {
            let px = crate::scene::sample_ndc(&rgba, W, H, p.ndc);
            println!("probe {} at {:?}: got {:?} want {:?}", p.name, p.ndc, px, p.expected);
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
