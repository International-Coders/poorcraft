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
pub mod city;
pub mod machines;
pub mod npcs;
pub mod camera;
pub mod construction;
pub mod streaming;
pub mod terrain;
pub mod water;
pub mod font;
pub mod gpu;
pub mod renderer;
pub mod scene;

pub use app::{run_windowed, InteractiveHost, ProbeSet, Shot, WindowConfig, WindowReport};
pub use camera::CameraPose;
pub use city::{city_scene, mesh_city, CityInfo, NavAnchor};
pub use machines::mesh_water_wheel;
pub use npcs::{advance, cast_for, mesh_npc, npc_world_pos, NpcCast};
pub use renderer::{QualityTier, Renderer};
pub use construction::{material_albedo, mesh_patch, UpdateStats};
pub use streaming::{StreamConfig, StreamCounters, TerrainStreamer};
pub use terrain::{mesh_patch_natural, MeshLod, TerrainStats};
pub use water::WaterStats;
pub use renderer::PixelReport;
