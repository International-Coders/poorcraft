//! The windowed shell: a resizable `winit` window driving the 3D renderer.
//!
//! R3DV-002 scope: first-person input (click to grab the mouse, mouse look,
//! WASD + Space/Shift movement), camera scripts for automated proof runs,
//! scheduled live-window screenshot captures, resize/loss recovery, Escape
//! to quit, and frame-time accounting.

use crate::camera::CameraPose;
use crate::renderer::{PixelReport, Renderer};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, NamedKey, PhysicalKey};
use winit::window::{Window, WindowAttributes};

/// One scheduled live-window capture.
#[derive(Clone)]
pub struct Shot {
    /// Frame number at which the presented swapchain frame is captured.
    pub frame: u64,
    pub path: PathBuf,
    /// Verify the sky probe on this capture (false for close-ups whose
    /// frame legitimately contains no sky).
    pub sky: bool,
}

impl Shot {
    pub fn new(frame: u64, path: impl Into<PathBuf>) -> Self {
        Self {
            frame,
            path: path.into(),
            sky: true,
        }
    }

    pub fn sky(mut self, sky: bool) -> Self {
        self.sky = sky;
        self
    }
}

/// A callback run once at a scheduled frame, before that frame's capture or
/// render — the hook host-driven proof runs use to submit world edits
/// through their own host and sync the renderer from its read-only state.
pub type FrameHook = Box<dyn FnMut(&mut Renderer)>;

/// Which semantic probes a run's captures verify against: the full scene
/// table (placeholder ground + sky) or sky-only (construction runs hide the
/// placeholder scene, so only the sky probe applies).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeSet {
    Scene,
    SkyOnly,
}

/// Data-only slice setup (cloneable config); the app assembles the full
/// SliceHost at window creation via `slice::assemble`.
pub struct SliceSetup {
    pub seed: u64,
    pub scene: std::rc::Rc<crate::slice::SliceScene>,
    pub save_root: std::rc::Rc<std::path::PathBuf>,
    pub world_name: String,
}

/// The live vertical slice: a walking player on colliding terrain, a host
/// for place/remove through commands, save/reload on disk, and inspect
/// boxes. Keys: WASD walk, mouse look (click), F place at ray target, R
/// remove at ray target, B save, L reload, I inspect boxes, Esc quits.
pub struct SliceHost {
    pub seed: u64,
    pub player: crate::player::PlayerBody,
    pub host: std::rc::Rc<std::cell::RefCell<pc3d_world::host::SoloHost>>,
    pub scene: std::rc::Rc<crate::slice::SliceScene>,
    pub save_root: std::rc::Rc<std::path::PathBuf>,
    pub world_name: String,
    pub inspect: bool,
    pub last_message: String,
}

/// A shared handle to the authoritative host for interactive construction:
/// F places a rock block 6 m ahead of the camera, R removes the built block
/// 6 m ahead — both through `HostCommand` + one tick, never by client-side
/// mutation. The renderer then syncs read-only from `host.construction`.
pub struct InteractiveHost(pub std::rc::Rc<std::cell::RefCell<pc3d_world::host::SoloHost>>);

pub struct WindowConfig {
    pub title: String,
    /// Logical size; the OS DPI scaling decides the physical surface size.
    pub logical_size: (f64, f64),
    /// Stop after this many presented frames (None: run until closed).
    pub max_frames: Option<u64>,
    /// Scheduled captures (sorted by frame); the run ends after the last.
    pub shots: Vec<Shot>,
    /// Camera poses applied at given frames (automated proof runs).
    pub camera_script: Vec<(u64, CameraPose)>,
    /// One-shot callbacks at given frames (host edits + renderer sync).
    pub frame_hooks: Vec<(u64, FrameHook)>,
    /// Probe selection for scheduled captures.
    pub probe_set: ProbeSet,
    /// Interactive construction (F place / R remove through the host).
    pub interactive_host: Option<InteractiveHost>,
    /// The live vertical slice (R3DV-011): walking player, host building,
    /// save/reload, inspect boxes.
    pub slice_host: Option<Box<SliceHost>>,
    /// Data-only setup: assembled into slice_host at window creation.
    pub slice_setup: Option<SliceSetup>,
    /// If set, the window is resized halfway to the first shot so the run
    /// proves live surface resize recovery before capturing.
    pub resize_to: Option<(f64, f64)>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "POORCRAFT 3D".into(),
            logical_size: (1280.0, 720.0),
            max_frames: None,
            shots: Vec::new(),
            camera_script: Vec::new(),
            frame_hooks: Vec::new(),
            probe_set: ProbeSet::Scene,
            interactive_host: None,
            slice_host: None,
            slice_setup: None,
            resize_to: Some((800.0, 500.0)),
        }
    }
}

/// One completed live-window capture: the PNG path, its semantic report, and
/// the decoded RGBA pixels (for cross-frame assertions like parallax).
#[derive(Clone)]
pub struct CaptureOutcome {
    pub path: PathBuf,
    pub report: PixelReport,
    pub rgba: Vec<u8>,
}

/// What a windowed run produced.
pub struct WindowReport {
    pub frames: u64,
    /// Capture outcomes in schedule order.
    pub captures: Vec<CaptureOutcome>,
    /// Frame times in milliseconds (presented frames only).
    pub frame_ms: Vec<f32>,
    /// Resized events the surface followed (resize-recovery evidence).
    pub resizes_observed: u32,
    /// Physical swapchain size at the end of the run.
    pub final_physical: (u32, u32),
    /// Window scale factor (physical = logical x scale).
    pub scale_factor: f64,
    /// Streaming counters at run end (when a streamer was attached).
    pub final_stream_counters: Option<crate::streaming::StreamCounters>,
}

impl WindowReport {
    pub fn p50_ms(&self) -> f32 {
        percentile(&self.frame_ms, 50)
    }

    pub fn p95_ms(&self) -> f32 {
        percentile(&self.frame_ms, 95)
    }

    pub fn avg_fps(&self) -> f32 {
        if self.frame_ms.is_empty() {
            0.0
        } else {
            1000.0 / (self.frame_ms.iter().sum::<f32>() / self.frame_ms.len() as f32)
        }
    }
}

fn percentile(v: &[f32], p: u32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let mut v = v.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    let idx = ((v.len() as f32 - 1.0) * p as f32 / 100.0).round() as usize;
    v[idx.min(v.len() - 1)]
}

struct WindowState {
    window: Arc<Window>,
    renderer: Renderer,
    frame_no: u64,
    frame_ms: Vec<f32>,
    consecutive_surface_errors: u32,
    resizes_observed: u32,
    final_physical: (u32, u32),
    captures: Vec<CaptureOutcome>,
    keys: HashSet<KeyCode>,
    pointer_grabbed: bool,
    done: bool,
    /// Blocks in the interactive host's overlay (HUD readout).
    built_count: usize,
}

impl WindowState {
    /// First-person movement from the current key set (4 m/s walk).
    fn apply_movement(&mut self, dt: f32) {
        let key = |k: KeyCode| if self.keys.contains(&k) { 1.0f32 } else { 0.0 };
        let fwd = key(KeyCode::KeyW) - key(KeyCode::KeyS);
        let strafe = key(KeyCode::KeyD) - key(KeyCode::KeyA);
        let vert = key(KeyCode::Space) - key(KeyCode::ShiftLeft);
        if fwd != 0.0 || strafe != 0.0 || vert != 0.0 {
            let step = self.renderer.camera_walk_step(fwd, strafe, vert, dt);
            let pos = self.renderer.pose().position;
            self.renderer
                .set_pose(CameraPose::new(
                    [pos[0] + step[0], pos[1] + step[1], pos[2] + step[2]],
                    self.renderer.pose().yaw,
                    self.renderer.pose().pitch,
                ));
        }
    }

    fn hud_line(&self) -> String {
        let recent: Vec<f32> = self.frame_ms.iter().rev().take(30).copied().collect();
        let fps = if recent.is_empty() {
            0.0
        } else {
            1000.0 / (recent.iter().sum::<f32>() / recent.len() as f32)
        };
        let pose = self.renderer.pose();
        format!(
            "P3D POS {:.1} {:.1} {:.1} YAW {:.2} FPS {:.0} BUILT {}",
            pose.position[0], pose.position[1], pose.position[2], pose.yaw, fps, self.built_count
        )
    }
}

struct App {
    cfg: WindowConfig,
    resize_plan: Option<(u64, f64, f64)>,
    next_script: usize,
    next_hook: usize,
    next_shot: usize,
    last_frame: Option<Instant>,
    state: Option<WindowState>,
}

impl App {
    fn finish(&self) -> WindowReport {
        match &self.state {
            Some(s) => WindowReport {
                frames: s.frame_no,
                captures: s.captures.clone(),
                frame_ms: s.frame_ms.clone(),
                resizes_observed: s.resizes_observed,
                final_physical: s.final_physical,
                scale_factor: s.window.scale_factor(),
                final_stream_counters: s.renderer.stream_counters(),
            },
            None => WindowReport {
                frames: 0,
                captures: Vec::new(),
                frame_ms: Vec::new(),
                resizes_observed: 0,
                final_physical: (0, 0),
                scale_factor: 1.0,
                final_stream_counters: None,
            },
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        let attrs = WindowAttributes::new()
            .with_title(self.cfg.title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.cfg.logical_size.0,
                self.cfg.logical_size.1,
            ))
            .with_resizable(true);
        let window = event_loop.create_window(attrs).expect("create window");
        let window = Arc::new(window);
        let mut renderer = Renderer::windowed(&window);
        if let Some(setup) = self.cfg.slice_setup.take() {
            let host = crate::slice::assemble(
                &mut renderer,
                &setup.scene,
                setup.seed,
                setup.save_root,
                &setup.world_name,
            );
            self.cfg.slice_host = Some(Box::new(host));
        }
        let built_count = self
            .cfg
            .interactive_host
            .as_ref()
            .map(|h| h.0.borrow().construction.values().map(|c| c.built_count()).sum())
            .unwrap_or(0);
        self.state = Some(WindowState {
            window,
            renderer,
            frame_no: 0,
            frame_ms: Vec::new(),
            consecutive_surface_errors: 0,
            resizes_observed: 0,
            final_physical: (0, 0),
            captures: Vec::new(),
            keys: HashSet::new(),
            pointer_grabbed: false,
            done: false,
            built_count,
        });
    }

    /// Global mouse motion: used for first-person look while the pointer is
    /// grabbed (click the window to grab, Escape to quit).
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            let Some(state) = &mut self.state else {
                return;
            };
            if !state.pointer_grabbed {
                return;
            }
            // Guard against pointer-lock re-entry spikes.
            let (dx, dy) = (dx.clamp(-100.0, 100.0), dy.clamp(-100.0, 100.0));
            let pose = state.renderer.pose();
            let yaw = pose.yaw - (dx as f32) * 0.0022;
            let pitch = (pose.pitch - (dy as f32) * 0.0022).clamp(-1.55, 1.55);
            state
                .renderer
                .set_pose(CameraPose::new(pose.position, yaw, pitch));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = &mut self.state else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: winit::keyboard::Key::Named(NamedKey::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            state.keys.insert(code);
                            // Live-slice keys: F/R build at the ray target,
                            // B/L save+reload, I inspect boxes.
                            if let Some(slice) = self.cfg.slice_host.as_mut() {
                                let gen = slice.scene.gen.clone();
                                match code {
                                    KeyCode::KeyF | KeyCode::KeyR => {
                                        let target = slice.player.ray_target(&gen, 8.0);
                                        if let Some((hit, air)) = target {
                                            let cell = if code == KeyCode::KeyF { air } else { hit };
                                            let mut h = slice.host.borrow_mut();
                                            if code == KeyCode::KeyF {
                                                h.submit(pc3d_world::host::HostCommand::Build {
                                                    cell,
                                                    material: pc3d_world::gen::CellMaterial::Sand,
                                                    owner: 7,
                                                });
                                            } else {
                                                h.submit(pc3d_world::host::HostCommand::RemoveBuild {
                                                    cell,
                                                    owner: 7,
                                                });
                                            }
                                            h.run_ticks(1);
                                            state.renderer.update_construction(&h.construction);
                                            slice.last_message = format!(
                                                "{} {:?}",
                                                if code == KeyCode::KeyF { "PLACED" } else { "REMOVED" },
                                                cell
                                            );
                                        } else {
                                            slice.last_message = "NO TARGET IN REACH".into();
                                        }
                                    }
                                    KeyCode::KeyB => {
                                        let root = slice.save_root.clone();
                                        let r = crate::slice::save_slice(
                                            root.as_ref(),
                                            &slice.world_name,
                                            slice.seed,
                                            &slice.host.borrow(),
                                            &slice.player,
                                        );
                                        slice.last_message = match r {
                                            Ok(()) => "SAVED".into(),
                                            Err(e) => format!("SAVE ERR {e:?}"),
                                        };
                                    }
                                    KeyCode::KeyL => {
                                        let root = slice.save_root.clone();
                                        let name = slice.world_name.clone();
                                        match crate::slice::load_slice(root.as_ref(), &name) {
                                            Ok((seed, host, player)) => {
                                                slice.seed = seed;
                                                *slice.host.borrow_mut() = host;
                                                slice.player = player;
                                                let h = slice.host.borrow();
                                                state.renderer.update_construction(&h.construction);
                                                slice.last_message = "RELOADED".into();
                                            }
                                            Err(e) => {
                                                slice.last_message = format!("LOAD ERR {e:?}");
                                            }
                                        }
                                    }
                                    KeyCode::KeyI => {
                                        slice.inspect = !slice.inspect;
                                        slice.last_message = if slice.inspect {
                                            "INSPECT ON".into()
                                        } else {
                                            "INSPECT OFF".into()
                                        };
                                    }
                                    _ => {}
                                }
                            }
                            // Interactive construction: F places, R removes —
                            // through the HOST command path, one tick, then a
                            // read-only renderer sync (no client mutation).
                            if let Some(host) = &self.cfg.interactive_host {
                                let target_cell = match code {
                                    KeyCode::KeyF | KeyCode::KeyR => {
                                        let pose = state.renderer.pose();
                                        let fwd = crate::camera::fwd_of(pose.yaw, pose.pitch);
                                        let t = [
                                            pose.position[0] + fwd[0] * 6.0,
                                            pose.position[1] + fwd[1] * 6.0,
                                            pose.position[2] + fwd[2] * 6.0,
                                        ];
                                        Some(pc3d_world::coords::CellCoord {
                                            x: t[0].floor() as i32,
                                            y: t[1].floor() as i32,
                                            z: t[2].floor() as i32,
                                        })
                                    }
                                    _ => None,
                                };
                                if let Some(cell) = target_cell {
                                    let mut h = host.0.borrow_mut();
                                    match code {
                                        KeyCode::KeyF => h.submit(
                                            pc3d_world::host::HostCommand::Build {
                                                cell,
                                                material: pc3d_world::gen::CellMaterial::Rock,
                                                owner: 7,
                                            },
                                        ),
                                        _ => h.submit(
                                            pc3d_world::host::HostCommand::RemoveBuild { cell, owner: 7 },
                                        ),
                                    };
                                    h.run_ticks(1);
                                    state.renderer.update_construction(&h.construction);
                                    state.built_count = h
                                        .construction
                                        .values()
                                        .map(|c| c.built_count())
                                        .sum();
                                }
                            }
                        }
                        ElementState::Released => {
                            state.keys.remove(&code);
                        }
                    };
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                // Click to look around: lock the pointer, FPS-style.
                let grabbed = state
                    .window
                    .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                    .or_else(|_| {
                        state
                            .window
                            .set_cursor_grab(winit::window::CursorGrabMode::Confined)
                    })
                    .is_ok();
                state.window.set_cursor_visible(!grabbed);
                state.pointer_grabbed = grabbed;
            }
            // Resize + scale-factor changes both land here; the renderer
            // reconfigures the surface (and clamps 0-sized events).
            WindowEvent::Resized(size) => {
                state.resizes_observed += 1;
                state.final_physical = (size.width, size.height);
                state.renderer.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                let dt = match self.last_frame {
                    Some(t) => t.elapsed().as_secs_f32().min(0.1),
                    None => 1.0 / 60.0,
                };
                self.last_frame = Some(Instant::now());

                // Scheduled camera script (teleport poses for proof runs).
                if self.next_script < self.cfg.camera_script.len() {
                    let (frame, pose) = self.cfg.camera_script[self.next_script];
                    if state.frame_no == frame {
                        state.renderer.set_pose(pose);
                        self.next_script += 1;
                    }
                }

                // Scheduled one-shot frame hooks (host-driven world edits +
                // read-only renderer sync), before this frame's capture.
                if self.next_hook < self.cfg.frame_hooks.len() {
                    let (frame, _) = &self.cfg.frame_hooks[self.next_hook];
                    if state.frame_no == *frame {
                        let (_, mut hook) =
                            std::mem::replace(&mut self.cfg.frame_hooks[self.next_hook], (0, Box::new(|_| {})));
                        hook(&mut state.renderer);
                        self.next_hook += 1;
                    }
                }

                // Scheduled mid-run resize: proves live surface recovery
                // before the captures run.
                if let Some((frame, w, h)) = self.resize_plan {
                    if state.frame_no == frame {
                        state
                            .window
                            .request_inner_size(winit::dpi::LogicalSize::new(w, h));
                        self.resize_plan = None;
                    }
                }

                // The live slice: per-frame WALKING on colliding terrain,
                // the camera locked to the player, streaming continues.
                if let Some(slice) = self.cfg.slice_host.as_mut() {
                    let key = |k: KeyCode| state.keys.contains(&k) as i32 as f32;
                    let fwd = key(KeyCode::KeyW) - key(KeyCode::KeyS);
                    let strafe = key(KeyCode::KeyD) - key(KeyCode::KeyA);
                    let gen = slice.scene.gen.clone();
                    slice.player.walk(&gen, fwd, strafe, dt);
                    state.renderer.set_pose(slice.player.pose());
                    let built: usize = slice
                        .host
                        .borrow()
                        .construction
                        .values()
                        .map(|c| c.built_count())
                        .sum();
                    state.renderer.set_hud_line(&format!(
                        "SLICE {} POS {:.0} {:.0} {:.0} BUILT {} | {}",
                        slice.seed,
                        slice.player.pos[0],
                        slice.player.pos[1],
                        slice.player.pos[2],
                        built,
                        slice.last_message
                    ));
                    let _ = state.renderer.stream_frame();
                } else {
                    // Interactive movement (free flight), then streaming.
                    state.apply_movement(dt);
                    let _ = state.renderer.stream_frame();
                    state.renderer.set_hud_line(&state.hud_line());
                }

                // Scheduled capture replaces this frame's presentation (the
                // swapchain texture is the copy source); ends after the last.
                let next_shot = self
                    .cfg
                    .shots
                    .get(self.next_shot)
                    .cloned();
                if let Some(shot) = next_shot {
                    if state.frame_no == shot.frame {
                        let pose = state.renderer.pose();
                        let aspect = state.renderer.aspect();
                        let probes = match (self.cfg.probe_set, shot.sky) {
                            (_, false) => vec![],
                            (ProbeSet::Scene, _) => crate::scene::probes_for_pose(pose, aspect),
                            (ProbeSet::SkyOnly, _) => vec![crate::scene::Probe {
                                name: "sky_above_horizon",
                                ndc: (0.0, 0.8),
                                expected: crate::scene::to_srgb4(crate::scene::sky_color_linear(
                                    crate::scene::dir_from_ndc(pose, (0.0, 0.8), aspect),
                                    crate::scene::SUN_DIR,
                                )),
                                tol: 0.05,
                            }],
                        };
                        let (report, rgba) = state.renderer.capture_png(&shot.path, &probes);
                        state.captures.push(CaptureOutcome {
                            path: shot.path,
                            report,
                            rgba,
                        });
                        self.next_shot += 1;
                        state.frame_no += 1;
                        if self.next_shot >= self.cfg.shots.len() {
                            state.done = true;
                        }
                        if state.done {
                            event_loop.exit();
                        }
                        return;
                    }
                }

                let t0 = Instant::now();
                match state.renderer.render_frame() {
                    Ok(()) => state.consecutive_surface_errors = 0,
                    Err(err) => {
                        state.consecutive_surface_errors += 1;
                        if state.consecutive_surface_errors > 120 {
                            eprintln!("[FAIL] surface unusable for 120 consecutive frames: {err:?}");
                            state.done = true;
                        }
                    }
                }
                state.frame_ms.push(t0.elapsed().as_secs_f32() * 1000.0);
                state.frame_no += 1;

                if self.cfg.max_frames == Some(state.frame_no) {
                    state.done = true;
                }
                if state.done {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);
        if let Some(state) = &self.state {
            if !state.done {
                state.window.request_redraw();
            }
        }
    }
}

/// Opens the window and pumps frames until closed, the frame budget runs
/// out, or the last scheduled capture completes. Must be called from the
/// main thread (winit requirement on macOS/Windows).
pub fn run_windowed(cfg: WindowConfig) -> Result<WindowReport, String> {
    if let (Some(first), false) = (cfg.shots.first(), cfg.shots.is_empty()) {
        if cfg.max_frames.is_some_and(|m| m <= first.frame) {
            return Err("max_frames must exceed the last shot frame".into());
        }
    }
    // Schedule the live resize halfway to the first capture so a screenshot
    // run always proves surface resize recovery before it captures.
    let resize_plan = match (cfg.shots.first(), cfg.resize_to) {
        (Some(shot), Some((w, h))) => Some((shot.frame / 2, w, h)),
        _ => None,
    };
    let event_loop = EventLoop::new().map_err(|e| format!("event loop: {e}"))?;
    let mut app = App {
        cfg,
        resize_plan,
        next_script: 0,
        next_hook: 0,
        next_shot: 0,
        last_frame: None,
        state: None,
    };
    event_loop
        .run_app(&mut app)
        .map_err(|e| format!("event loop error: {e}"))?;
    Ok(app.finish())
}
