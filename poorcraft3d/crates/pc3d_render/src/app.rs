//! The windowed shell: a resizable `winit` window driving the renderer.
//!
//! R3DV-001 scope: continuous redraw, resize/loss recovery, Escape/Close to
//! quit, optional live-window screenshot capture at a chosen frame, and
//! frame-time accounting for the performance record. First-person input
//! arrives in R3DV-002.

use crate::renderer::{PixelReport, Renderer};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::NamedKey;
use winit::window::{Window, WindowAttributes};

#[derive(Clone)]
pub struct WindowConfig {
    pub title: String,
    /// Logical size; the OS DPI scaling decides the physical surface size.
    pub logical_size: (f64, f64),
    /// Stop after this many presented frames (None: run until closed).
    pub max_frames: Option<u64>,
    /// If set, the live swapchain frame is captured to this PNG on
    /// [`WindowConfig::screenshot_frame`] and the run ends.
    pub screenshot: Option<PathBuf>,
    pub screenshot_frame: u64,
    /// If set (and a screenshot is requested), the window is resized to this
    /// logical size halfway to the capture frame, so the run proves live
    /// surface resize recovery before the capture.
    pub resize_to: Option<(f64, f64)>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "POORCRAFT 3D".into(),
            logical_size: (1280.0, 720.0),
            max_frames: None,
            screenshot: None,
            screenshot_frame: 20,
            resize_to: Some((800.0, 500.0)),
        }
    }
}

/// What a windowed run produced: frame timings plus the capture report if a
/// screenshot was requested.
pub struct WindowReport {
    pub frames: u64,
    pub captured: Option<PixelReport>,
    /// Frame times in milliseconds (presented frames only).
    pub frame_ms: Vec<f32>,
    /// Resized events the surface followed (resize-recovery evidence).
    pub resizes_observed: u32,
    /// Physical swapchain size at the end of the run.
    pub final_physical: (u32, u32),
    /// Window scale factor (physical = logical × scale).
    pub scale_factor: f64,
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

fn percentile(sorted: &[f32], p: u32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let mut v = sorted.to_vec();
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
    captured: Option<PixelReport>,
    done: bool,
}

struct App {
    cfg: WindowConfig,
    /// Some((frame, w, h)) while the scheduled mid-run resize hasn't fired.
    resize_plan: Option<(u64, f64, f64)>,
    state: Option<WindowState>,
}

impl App {
    fn finish(&self) -> WindowReport {
        match &self.state {
            Some(s) => WindowReport {
                frames: s.frame_no,
                captured: s.captured,
                frame_ms: s.frame_ms.clone(),
                resizes_observed: s.resizes_observed,
                final_physical: s.final_physical,
                scale_factor: s.window.scale_factor(),
            },
            None => WindowReport {
                frames: 0,
                captured: None,
                frame_ms: Vec::new(),
                resizes_observed: 0,
                final_physical: (0, 0),
                scale_factor: 1.0,
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
        let renderer = Renderer::windowed(&window);
        self.state = Some(WindowState {
            window,
            renderer,
            frame_no: 0,
            frame_ms: Vec::new(),
            consecutive_surface_errors: 0,
            resizes_observed: 0,
            final_physical: (0, 0),
            captured: None,
            done: false,
        });
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
            // Resize + scale-factor changes both land here; the renderer
            // reconfigures the surface (and clamps 0-sized events).
            WindowEvent::Resized(size) => {
                state.resizes_observed += 1;
                state.final_physical = (size.width, size.height);
                state.renderer.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                // Scheduled mid-run resize: the window manager delivers the
                // Resized event, the surface reconfigures, and later frames
                // prove rendering still works at the new size.
                if let Some((frame, w, h)) = self.resize_plan {
                    if state.frame_no == frame {
                        state
                            .window
                            .request_inner_size(winit::dpi::LogicalSize::new(w, h));
                        self.resize_plan = None;
                    }
                }
                // Capture, when requested, replaces this frame's presentation
                // (the swapchain texture is the copy source) and ends the
                // run. If a mid-run resize was scheduled, wait for the
                // Resized event to reach the surface first — the capture must
                // show the resized swapchain, not a race.
                let resize_settled =
                    self.resize_plan.is_none() || state.resizes_observed > 0;
                if state.frame_no >= self.cfg.screenshot_frame && resize_settled {
                    if let Some(path) = &self.cfg.screenshot {
                        state.captured = Some(state.renderer.capture_png(path));
                        state.done = true;
                    }
                }
                if !state.done {
                    let t0 = Instant::now();
                    match state.renderer.render_frame() {
                        Ok(()) => state.consecutive_surface_errors = 0,
                        Err(err) => {
                            state.consecutive_surface_errors += 1;
                            if state.consecutive_surface_errors > 120 {
                                eprintln!(
                                    "[FAIL] surface unusable for 120 consecutive frames: {err:?}"
                                );
                                state.done = true;
                            }
                        }
                    }
                    state.frame_ms.push(t0.elapsed().as_secs_f32() * 1000.0);
                    state.frame_no += 1;
                    if self.cfg.max_frames == Some(state.frame_no) {
                        state.done = true;
                    }
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

/// Opens the window and pumps frames until closed, the frame budget runs out,
/// or the screenshot is captured. Must be called from the main thread
/// (winit requirement on macOS/Windows).
pub fn run_windowed(cfg: WindowConfig) -> Result<WindowReport, String> {
    if cfg.screenshot.is_some() && cfg.max_frames.is_some_and(|m| m <= cfg.screenshot_frame) {
        return Err(
            "max_frames must exceed screenshot_frame or the capture never happens".into(),
        );
    }
    // Schedule the live resize halfway to the capture frame so a screenshot
    // run always proves surface resize recovery before it captures.
    let resize_plan = match (&cfg.screenshot, &cfg.resize_to) {
        (Some(_), Some((w, h))) => Some((cfg.screenshot_frame / 2, *w, *h)),
        _ => None,
    };
    let event_loop = EventLoop::new().map_err(|e| format!("event loop: {e}"))?;
    let mut app = App {
        cfg,
        resize_plan,
        state: None,
    };
    event_loop
        .run_app(&mut app)
        .map_err(|e| format!("event loop error: {e}"))?;
    Ok(app.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_and_fps_are_sane() {
        let report = WindowReport {
            frames: 5,
            captured: None,
            frame_ms: vec![16.0, 33.0, 8.0, 12.0, 100.0],
            resizes_observed: 0,
            final_physical: (0, 0),
            scale_factor: 1.0,
        };
        assert_eq!(report.p50_ms(), 16.0);
        assert_eq!(report.p95_ms(), 100.0);
        assert!(report.avg_fps() > 20.0 && report.avg_fps() < 40.0);
        let empty = WindowReport {
            frames: 0,
            captured: None,
            frame_ms: Vec::new(),
            resizes_observed: 0,
            final_physical: (0, 0),
            scale_factor: 1.0,
        };
        assert_eq!(empty.p50_ms(), 0.0);
        assert_eq!(empty.avg_fps(), 0.0);
    }
}
