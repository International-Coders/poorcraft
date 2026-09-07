# Renderer Architecture: One Visible World, Not a Second Simulation

## New crate layout

Add these P3D-local crates. Do not make `pc3d_world` depend on rendering.

```text
pc3d_core        deterministic time, ids, events, profiling
pc3d_world       authoritative simulation and read-only visual queries
pc3d_save        world persistence
pc3d_assets      manifests, asset validation, material lookup, GPU-ready data
pc3d_render      winit/wgpu renderer, meshes, shaders, camera, GPU resources
apps/poorcraft3d input/client shell: host + presentation + UI
```

`pc3d_render` may read snapshots or query interfaces from `pc3d_world`; the
world crate must never import `wgpu`, `winit`, a texture, or a UI type. This
preserves deterministic simulation for solo, server, and replay.

The original project’s `lf_engine` may be studied for its bug fixes and proof
techniques, but POORCRAFT 3D must not silently depend on original-game state or
recreate that project’s world underneath the new simulation.

## First technical choices

- Rust 2021/2024 workspace, `winit` for the window/input loop, `wgpu` for
  cross-platform raster rendering, WGSL shaders.
- Raster renderer is mandatory baseline. Ray/path tracing is a later optional
  quality mode, not a dependency for play.
- Right-handed world coordinates with meters as the P3D world unit; document
  the exact axis/up/camera convention in code and asset manifest.
- One indexed mesh format for terrain/construction/castle modules with
  position, normal, tangent or material basis, UV, material id, AO/light, and
  optional animation/entity attributes.
- Opaque pass first, alpha-cutout second, transparent water last, UI after the
  scene before readback/screenshot.

## Frame loop

```text
input -> player command -> authoritative local host tick(s)
      -> visual snapshot -> streaming/dirty-region selection
      -> bounded CPU mesh jobs -> GPU uploads -> culling -> render passes
      -> UI + screenshot/proof readback
```

Rendering interpolates known state. It must not alter terrain, water, NPC
position, inventory, or city state.

## Required initial render passes

1. Sky/clear color and directional sun.
2. Opaque natural terrain and construction.
3. Alpha-cutout foliage/props when available.
4. Transparent river/water mesh.
5. Entity/castle module meshes.
6. Selection outline and debug overlays.
7. First-person HUD and asset/mesh backlog diagnostics.

## Actual performance rules

- Mesh job queues use the existing stream/LOD schedule but perform real work.
- GPU uploads are capped per frame; show backlog instead of freezing.
- Each mesh owns a version from terrain/build/flow/entity revisions; upload
  only when the version changes.
- Frustum cull every mesh; add occlusion only after correct frustum behavior.
- Use material batching/instancing for repeated blocks, castle modules, and
  NPCs.
- Keep low-quality mode playable without expensive shadows, high LOD rings, or
  large texture memory.

## First renderer gates

| Gate | Must prove |
|---|---|
| R-001 | real `winit` window + `wgpu` surface + nonuniform screenshot |
| R-002 | camera sees a lit indexed cube and input changes view |
| R-003 | terrain patch query produces a GPU mesh with collision-aligned bounds |
| R-004 | construction edit invalidates/remeshes the correct patch only |
| R-005 | visual client uses host snapshot; no client-private terrain mutation |
