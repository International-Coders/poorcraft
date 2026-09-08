# Natural Terrain, Caves, Building, and Water

## The chosen hybrid representation

Use a **surface-first hybrid**. It is the best performance/value trade-off for
the intended style and prevents an accidental full-engine rewrite.

```text
Normal landscape: 16 m x 16 m terrain patches
  base procedural height/material samples + sparse sculpt delta layer
  -> low-poly triangulated surface mesh + terrain collider + LOD mesh cache

Caves / overhangs / special cliffs: sparse local volume regions only
  signed density samples + dual contouring or marching-cubes-class extractor
  -> welded boundary mesh + collision proxy + local dirty rebuild

Player construction: existing grid construction overlay
  -> modular/block mesh, snapped placement, separate collision and save data

Rivers: authoritative cached FlowRecords
  -> terrain-conforming ribbon/volume, shore intersection, local remesh only
```

## Surface patch contract

Each patch owns deterministic base samples, material weights, sculpt deltas,
version/dirty state, seam-compatible neighbor sampling, one or more cached LOD
meshes, collision acceleration data, and a save delta. It never owns a copy of
the whole world or runs work every frame.

Start with a 17x17 vertex boundary grid for clean neighbouring edges. Increase
near detail only after profiling proves it necessary. Use coherent low-frequency
shapes and controlled terraces/cliffs; do not smooth every feature into bland
terrain. Normals may be faceted or selectively softened according to biome.

## Editing and construction integration

Terrain edits are commands: raise, lower, level, dig, fill, till, compact, or
place foundation. They modify a local signed height delta/soil layer and mark
the patch plus border neighbours dirty. A building placement must query the
terrain surface for support, slope, clearance, and drainage. It must never
silently mutate terrain to make an invalid placement look valid.

Farming is a surface-cell system: soil depth, moisture, fertility, tilling,
crop state, and irrigation exposure. It does not require 4,096 subvoxels per
old block.

## Caves and seams

Do not implement caves as holes carved blindly into heightfields. A cave volume
has an explicit bounds/ownership record and samples shared boundary planes with
the surrounding surface region. Prove no crack, light leak, collision mismatch,
or duplicate surface at the seam. Build caves after the surface path is proven.

## Water

Keep flow simulation simple and geographical. A river has stable discharge,
slope, direction, width/depth class, local graph revisions, and machine power
queries. Water power is an intentional game abstraction: mechanical potential
is allocated by the river/network rule, while the visible river does not need
particle-level fluid simulation. Terrain edits invalidate only affected flow
segments and their water mesh.

