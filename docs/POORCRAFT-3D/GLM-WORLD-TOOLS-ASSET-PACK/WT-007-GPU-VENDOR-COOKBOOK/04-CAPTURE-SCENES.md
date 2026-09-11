# Capture Scenes

Each scene needs deterministic seed, viewport, quality, screenshot, metrics,
and marker audit.

## Required Scenes

- `gpu_title_ui`: title menu with UI pass and input focus;
- `gpu_seed_preview`: New World seed preview with map panel;
- `gpu_semantic_assets`: house, NPC, forge, chest, resource node;
- `gpu_terrain_water`: terrain, water, sky, atmosphere, vegetation;
- `gpu_settlement_crowd`: settlement kit, NPC crowd, pathing markers;
- `gpu_wireframe_overlay`: mesh edges, anchors, bounds, UI overlay;
- `gpu_stress_asset_batch`: many generated assets and UI icons.

## Viewports

- 1280x720;
- 1501x801;
- 1920x1080 when performance budget allows.

## Output

Each capture writes PNG, runtime JSON, marker JSON, perf JSON, and verdict JSON.
