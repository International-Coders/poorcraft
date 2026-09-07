//! pc3d_render — the POORCRAFT 3D renderer (visual reset R3DV-001/002).
//!
//! Architecture law (docs/POORCRAFT-3D-VISUAL-RESET/02-RENDERER-ARCHITECTURE.md):
//! this crate reads visual snapshots or query interfaces from the simulation;
//! it never mutates canonical world state, and it owns no world data of its
//! own. The R3DV-002 proof scene is renderer-local placeholder geometry
//! (terrain meshing is R3DV-004+); the boundary is already in place —
//! nothing here references `pc3d_world`.
//!
//! World convention (binding): right-handed coordinates, meters as the world
//! unit, +X east, +Y up, +Z south ([`camera`] documents yaw/pitch). Cameras
//! and geometry live in P3D world coordinates; there is no separate renderer
//! space.
//!
//! Current ability (R3DV-001 + R3DV-002 gates): a real `winit` window, a
//! `wgpu` surface with `COPY_SRC` for live screenshots, a perspective camera
//! with mouse look + WASD movement, a depth-tested sunlit indexed mesh in
//! world coordinates, a ray-reconstructed sky tied to the world sun
//! direction, a bitmap-font HUD debug line, surface resize/loss recovery,
//! and semantic pixel proofs (face-flip, occlusion, parallax).

pub mod app;
pub mod camera;
pub mod font;
pub mod gpu;
pub mod renderer;
pub mod scene;

pub use app::{run_windowed, Shot, WindowConfig, WindowReport};
pub use camera::CameraPose;
pub use renderer::PixelReport;
