//! GPU context and surface ownership for pc3d_render.
//!
//! The surface is configured with `COPY_SRC` in addition to `RENDER_ATTACHMENT`
//! so a live windowed frame can be copied to a readback buffer and saved as a
//! real screenshot (the R3DV-001 proof). Recovery policy: `Lost` recreates the
//! surface, `Outdated` reconfigures, `Timeout`/`Occluded` skip the frame.

use std::sync::Arc;

pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GpuContext {
    pub fn new(compatible_surface: Option<&wgpu::Surface<'static>>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface,
            force_fallback_adapter: false,
        }))
        .expect("no suitable GPU adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("pc3d_render device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .expect("GPU device request failed");
        Self {
            instance,
            adapter,
            device,
            queue,
        }
    }
}

/// What to do after a failed `get_current_texture` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceAction {
    /// Reconfigure the surface with the current config, then retry once.
    ReconfigureRetry,
    /// Drop and recreate the surface against the window, then retry once.
    RecreateRetry,
    /// Give up on this frame (present pool exhausted, OOM).
    SkipFrame,
}

pub fn surface_action(err: &wgpu::SurfaceError) -> SurfaceAction {
    match err {
        wgpu::SurfaceError::Lost => SurfaceAction::RecreateRetry,
        wgpu::SurfaceError::Outdated => SurfaceAction::ReconfigureRetry,
        wgpu::SurfaceError::Timeout => SurfaceAction::SkipFrame,
        wgpu::SurfaceError::OutOfMemory => SurfaceAction::SkipFrame,
        _ => SurfaceAction::SkipFrame,
    }
}

/// Window sizes must stay >= 1 in both axes: wgpu rejects zero-sized
/// surface configurations (minimize/dock events deliver 0x0 on some
/// platforms).
pub fn clamp_size(width: u32, height: u32) -> (u32, u32) {
    (width.max(1), height.max(1))
}

/// Owns the swapchain configuration; the surface itself is recreated on
/// `Lost` via [`SurfaceGuard::recreate`].
pub struct SurfaceGuard {
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    format: wgpu::TextureFormat,
}

impl SurfaceGuard {
    pub fn new(ctx: &GpuContext, window: &Arc<winit::window::Window>) -> Self {
        let surface = ctx
            .instance
            .create_surface(window.clone())
            .expect("create window surface");
        let caps = surface.get_capabilities(&ctx.adapter);
        // Prefer an sRGB format so linear shader colors are display-correct.
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);
        let size = window.inner_size();
        let (width, height) = clamp_size(size.width, size.height);
        let config = wgpu::SurfaceConfiguration {
            // COPY_SRC is what makes live-window screenshot readback legal.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&ctx.device, &config);
        Self {
            surface,
            config,
            format,
        }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub fn reconfigure(&self, device: &wgpu::Device) {
        self.surface.configure(device, &self.config);
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let (w, h) = clamp_size(width, height);
        if w == self.config.width && h == self.config.height {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        self.reconfigure(device);
    }

    pub fn recreate(&mut self, ctx: &GpuContext, window: &Arc<winit::window::Window>) {
        let fresh = SurfaceGuard::new(ctx, window);
        self.surface = fresh.surface;
        self.config = fresh.config;
        self.format = fresh.format;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_size_rejects_zero_axes() {
        assert_eq!(clamp_size(0, 0), (1, 1));
        assert_eq!(clamp_size(0, 720), (1, 720));
        assert_eq!(clamp_size(1280, 0), (1280, 1));
        assert_eq!(clamp_size(800, 600), (800, 600));
    }

    #[test]
    fn surface_error_maps_to_policy() {
        assert_eq!(
            surface_action(&wgpu::SurfaceError::Lost),
            SurfaceAction::RecreateRetry
        );
        assert_eq!(
            surface_action(&wgpu::SurfaceError::Outdated),
            SurfaceAction::ReconfigureRetry
        );
        assert_eq!(
            surface_action(&wgpu::SurfaceError::Timeout),
            SurfaceAction::SkipFrame
        );
        assert_eq!(
            surface_action(&wgpu::SurfaceError::OutOfMemory),
            SurfaceAction::SkipFrame
        );
    }
}
