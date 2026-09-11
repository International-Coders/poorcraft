# Runtime State Exports

Every proof route writes runtime JSON. The export must be readable without a
GPU debugger.

## Required Top-Level Fields

- `version`;
- `build_hash`;
- `command`;
- `timestamp_utc`;
- `scene`;
- `seed`;
- `viewport`;
- `quality`;
- `camera`;
- `player`;
- `ui`;
- `world`;
- `visible_assets`;
- `interactions`;
- `npcs`;
- `machines`;
- `perf`;
- `gpu_markers`;
- `proof_paths`;
- `verdict`.

## Required State Examples

- `player`: position, velocity, grounded, looking_at, selected_tool,
  input_capture, mouse_locked;
- `ui`: screen, focused_control, buttons, text_inputs, panel bounds,
  clipping verdict, alpha layers;
- `world`: loaded chunks, biome under player, time, weather, spawn, seed hash;
- `visible_assets`: ids, bounds, lod, material buckets, interaction anchors;
- `interactions`: last action, target id, result, reason, side effects;
- `machines`: state, input, output, fuel, power, progress, blocked reason;
- `npcs`: id, faction, schedule, dialogue state, current action, relation.

If a field is unknown, export `null` with a reason. Do not omit it silently.
