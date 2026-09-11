# Inspector Wireframe Capture

GLM is authorized to add local debug modes, commands, MCP-style endpoints, and
temporary exporters when they produce better evidence. Keep those tools local
and deterministic.

## Required Views

- beauty: normal material render;
- wireframe: triangle edges over shaded asset;
- bounds: AABB/OBB, collision hulls, nav blockers, door clearances;
- anchors: labeled interaction points and volumes;
- materials: albedo/normal/roughness groups, atlas ids, texture scale;
- LOD: switch distance, triangle counts, impostor state;
- runtime: current health/state/progress/inventory/power/fuel where relevant;
- perf: draw calls, instances, triangles, material buckets, frame time.

## Required Export Files

For each screenshot scene write:

- PNG beauty screenshot;
- PNG wireframe screenshot;
- PNG anchor/bounds overlay;
- JSON layout/inspection sidecar;
- JSON performance sidecar;
- optional GLB/OBJ debug dump for generated assets.

## Tool Shape

Prefer commands that can run headless:

- `--asset-inspect <asset_id> <outdir>`;
- `--asset-batch-inspect <contract.json> <outdir>`;
- `--wireframe-scene <scene> <outdir>`;
- `--semantic-playtest <route.json> <outdir>`.

No UI-only claim is accepted without a saved file.
