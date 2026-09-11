# MCP Tools Spec

The first implementation can be CLI-only. If a true MCP server is added later,
use the same command names and JSON shapes.

## Tool Names

- `pc3d.capture_screenshot`
- `pc3d.capture_wireframe`
- `pc3d.capture_overlay`
- `pc3d.inspect_scene`
- `pc3d.inspect_asset`
- `pc3d.inspect_ui`
- `pc3d.inspect_player`
- `pc3d.inspect_npc`
- `pc3d.inspect_machine`
- `pc3d.dump_mesh`
- `pc3d.dump_materials`
- `pc3d.replay_input_route`
- `pc3d.compare_evidence`
- `pc3d.audit_gpu_markers`

## Common Arguments

- `scene`: scene id or saved world route;
- `seed`: deterministic seed string;
- `viewport`: width/height;
- `quality`: Low, Mid, High;
- `camera`: named camera or explicit transform;
- `route`: optional route id;
- `outdir`: output folder;
- `build_hash`: auto-filled when possible.

## Common Outputs

- `summary.json`;
- screenshot PNGs;
- overlay PNGs;
- wireframe PNGs;
- layout JSON;
- mesh/material JSON;
- perf JSON;
- route transcript JSON;
- verdict JSON.
