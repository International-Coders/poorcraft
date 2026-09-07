# Voxel Terrain and Meshing: Make the Existing World Visible

## Current state

P3D already uses 16 m terrain patches and a one-meter cell concept. It has
terrain/cave/overhang queries and construction overlays, but no real mesh.
The first renderer must promote those queries to visible geometry rather than
replace them with a new unrelated terrain implementation.

## Two mesh families

Use different render geometry for the two kinds of world matter:

| World matter | Render representation | Why |
|---|---|---|
| Natural terrain | smooth-ish surface mesh extracted from a density/material field | hills, cliffs, caves, arches, and overhangs feel natural |
| Player/city construction | grid-aligned block/greedy mesh or authored module mesh | building remains readable, precise, and cheap |

The final terrain query must be shared by rendering, collision, raycast,
lighting, water, navigation, and save/load. Do not let the renderer sample a
different terrain function from the player controller.

## Implementation sequence

### 1. Immediate visible voxel mesh

Generate a culled block-face mesh from the current final cell query. This is
not the end art direction, but it proves patch coordinates, material batching,
construction edits, caves, collision alignment, ray target, and GPU upload.

### 2. Natural surface prototype

Expose a deterministic `sample_density(world_position)` and material function
for natural terrain. Prototype dual contouring and marching-cubes-style surface
extraction on the same hill/cliff/cave/edit scenes. Choose using actual mesh
time, triangles, memory, seam behavior, edit rebuild cost, collision agreement,
and human art review.

The preferred target is dual contouring or equivalent feature-preserving
surface extraction for natural terrain, while construction retains blocks. Do
not claim a smooth terrain engine merely because a height map is shaded.

### 3. Patch boundaries and LOD

- Full-resolution patches near the player use natural/cell detail and collision.
- Mid/far rings use lower-resolution natural meshes and no detailed collision.
- Use a real boundary transition method; cracks and collision holes are bugs.
- Construction either forces sufficient local detail or has an independent
  module/block LOD that remains stable.

## Water rendering

Flow records must generate a water surface mesh with height/depth/width,
direction, current speed class, and banks. A vertex/material animation may
show current, foam, and reflection, but flow direction comes from the actual
P3D flow record—not a decorative scrolling texture alone.

Use a transparent pass with depth-aware ordering. A dam/channel edit changes
the flow record, which changes only the affected water mesh sections.

## Required scene tests

- hill + cliff: natural mesh has visible slope and material separation;
- cave + overhang: no missing faces, light leak, visual/collision mismatch, or
  cross-patch crack;
- terrain edit: only bounded patches remesh and the edit appears after upload;
- construction over terrain: blocks remain visible and targetable;
- river edit: water direction/width changes locally;
- LOD walk: no obvious holes or falling through when crossing a ring boundary.
