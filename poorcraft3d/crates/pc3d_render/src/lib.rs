//! pc3d_render — the POORCRAFT 3D windowed renderer (visual reset R3DV-001).
//!
//! Architecture law (docs/POORCRAFT-3D-VISUAL-RESET/02-RENDERER-ARCHITECTURE.md):
//! this crate reads visual snapshots or query interfaces from the simulation;
//! it never mutates canonical world state, and it owns no world data of its
//! own. In R3DV-001 the proof scene is renderer-local placeholder geometry
//! because the camera/terrain binding is R3DV-002+; the boundary is already
//! in place — nothing here references `pc3d_world`.
//!
//! World convention (binding for all later crates, documented per
//! 02-RENDERER-ARCHITECTURE.md): right-handed coordinates, meters as the
//! world unit, +X east, +Y up, +Z south. Cameras look along -Z at scene
//! origin by default (wired in R3DV-002). The R3DV-001 proof scene is drawn
//! directly in normalized device coordinates and contains no world geometry.
//!
//! Current ability (R3DV-001 gate): a real `winit` window, a `wgpu` surface,
//! a nonuniform GPU scene (dawn-gradient sky with a sun disc plus an indexed,
//! vertex-colored banner mesh — "three banners at dawn", original POORCRAFT
//! placeholder art), surface resize/loss recovery, and screenshot readback
//! captured from the live swapchain texture.

pub mod app;
pub mod gpu;
pub mod renderer;
pub mod scene;

pub use app::{run_windowed, WindowConfig, WindowReport};
pub use renderer::PixelReport;
