# Baseline and Architecture

## Evidence-based baseline

The current `poorcraft3d/` workspace is not empty or simulation-only. It has a
native `winit`/`wgpu` renderer, first-person movement/collision, bounded
streaming, a real windowed regression suite, cached river strips, city/NPC
render bindings, and a walkable save/reload slice. Keep these strengths.

Its presentation is still prototype-grade:

| Area | Current implementation | Rebuild result |
| --- | --- | --- |
| Natural terrain | Culled block faces from `final_solid` | Low-poly terrain patch mesh; sparse cave mesh only where needed |
| City/castle | Procedural cuboids/prisms | Original reusable glTF modules with collision and LOD |
| NPCs | Primitive mesh bodies | Original rig-ready low-poly character assets, role gear and animation states |
| Materials | Registry colours + procedural noise | Compact original texture/material library and readable material response |
| Vegetation/props | No finished asset library | Instanced trees, rocks, foliage, bridge/dock and biome props |
| Water | Authoritative transparent strips | Terrain-conforming river mesh with visual flow and shore treatment |

## Runtime boundaries

```text
pc3d_world  -> deterministic generation, edits, flow, NPCs, settlements
pc3d_save   -> versioned authoritative persistence and migrations
pc3d_assets -> asset manifest, source/compiled paths, material and LOD metadata
pc3d_render -> terrain/asset mesh loading, GPU resources, lighting, draw passes
app          -> input, window, player presentation; submits host commands only
asset tools  -> Blender/Python offline only; reproducibly build .glb and textures
```

Natural-terrain rendering may query an immutable world snapshot. Terrain edits
must arrive as host commands and dirty only intersecting terrain patches. An
asset loader may not define gameplay collision rules ad hoc: it consumes the
manifest’s explicit collision and navigation metadata.

## Platform budget

Target Steam Deck as a first-class lowest common denominator:

- 1280x800, 60 fps target; stable 30 fps is acceptable only for intentionally
  heavier showcase scenes and must be reported.
- Low tier must have bounded memory/upload work, shadow and foliage reductions,
  terrain LOD, and a controllable internal render scale.
- Medium/high improve density and distance—not rules, save compatibility, or
  visibility of essential interaction objects.
- Profile CPU frame time, GPU frame time when available, resident GPU memory,
  draw calls, triangles, asset memory, mesh jobs, and upload budget.

