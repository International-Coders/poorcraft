# MCP Inspector Expansion

The previous UI pack defines a local inspector. This pack expands it for world,
asset, and performance work.

## New Commands

- `asset_catalog`: list loaded asset ids, semantic tags, file paths, proof
  status.
- `asset_inspect`: return sockets, anchors, collision, nav, materials, triangle
  count, and gameplay law compliance for one asset.
- `world_seed_preview`: generate map preview and metadata for a seed.
- `world_seed_compare`: compare two seeds and report visual/system differences.
- `wireframe_capture`: dump visible mesh or selected asset wireframe.
- `semantic_probe`: test whether an object can be used, entered, talked to,
  opened, harvested, forged, or stored in.
- `gpu_frame_report`: dump per-pass stats and marker names.
- `npc_probe`: return NPC name, role, schedule, dialogue, faction, talk state.
- `workstation_probe`: return workstation active/blocked/recipe/input/output
  state.

## Rule

If GLM cannot answer "what is this thing and how does the player use it?", it
must add or improve an inspector command before claiming the asset is done.
