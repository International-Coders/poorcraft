# Z-code Implementation Slices

Ship in these slices. Do not skip evidence.

## Slice 1: Contract Loader

- Parse `asset_factory_queue.json`.
- Validate `asset_affordance_schema.json`.
- Add tests for missing door/talk/forge anchors.

## Slice 2: Inspector Sidecars

- Add a pure exporter for bounds, materials, LOD, anchors, collision, nav, and
  gameplay affordances.
- Write JSON sidecars without launching a window.

## Slice 3: Screenshot Modes

- Add beauty, wireframe, and anchor overlay captures.
- Pixel-check nonblank wireframe, visible anchors, readable labels, and no UI
  overlap.

## Slice 4: First Batch

- Generate/import at least one house, one NPC, one forge, one resource node,
  one map marker, and one UI icon set.
- Each must have runtime consumers or explicit backlog entries.

## Slice 5: Semantic Playtest

- Spawn the first batch in a deterministic scene.
- Walk through the house door, talk to the NPC, use the forge, harvest the
  resource, inspect the map marker, and render the HUD icon.

## Slice 6: GPU Markers

- Add pass markers and a capture plan.
- Export before/after perf data before touching optimization code.
