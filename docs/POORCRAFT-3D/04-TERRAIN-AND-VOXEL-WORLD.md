# Terrain and Voxel World

## Goal

Natural-looking terrain with the construction clarity of voxels, caves and
overhangs from the first terrain foundation, and bounded CPU/GPU cost.

## Recommended representation

Use a hybrid representation:

- streamed terrain regions are the primary spatial unit;
- a height/density field creates natural surfaces;
- adaptive volumetric detail handles caves, arches, cliffs, and overhangs;
- explicit voxel/block edits preserve building and destruction;
- generated meshes are cached and invalidated only by relevant changes.

A 16×16 meter (or equivalent) patch can be a useful streaming and editing
unit. It should not force every surface to be a rigid 16×16 cube or require
uniform high resolution everywhere.

## Surface behavior

The terrain should be smooth-ish rather than mathematically perfect:

- preserve readable material layers and block-scale interaction;
- blend height samples only where the art direction calls for it;
- retain stepped or faceted transitions in selected materials and structures;
- use authored parameters for slopes, plateaus, shorelines, cliffs, and
  biome transitions;
- allow player edits to create visible, stable changes.

## Caves and overhangs

Caves and overhangs are in scope early. They require:

- a density or occupancy query that is not limited to a heightmap;
- robust collision separate from the render mesh;
- sealed-volume and lighting rules;
- deterministic generation by world seed and region coordinate;
- edit persistence that does not regenerate player work;
- mesh transitions between adaptive resolutions.

The first implementation does not need full geological erosion or destructible
rock simulation. It does need stable traversal, building, lighting, and save/
reload behavior in nontrivial geometry.

## Visibility and LOD

Only data that can affect the player or the visible frame should be fully
materialized. Use frustum, distance, occlusion, and interest-region culling.
Keep a lower-detail representation for distant terrain so the world can be
large without keeping every voxel resident.

## Terrain acceptance questions

- Can a player walk through a cave, climb an overhang, and build beside it?
- Does a changed slope update visual and collision meshes consistently?
- Does save/reload preserve edits at region boundaries?
- Do neighboring LODs avoid visible cracks and collision gaps?
- Can profiling show the cost of a terrain edit and a long-distance travel
  stream separately?

## Player-facing terrain rules

Natural does not mean slippery or unreadable. A player must be able to tell
what can be walked on, mined, planted, built upon, flooded, or protected. The
terrain style may be smooth-ish, but material edges, construction, interaction
distance, and collision need clear feedback.

Shovel/terrain tools should have material costs, permissions, and bounded
brushes where that improves game balance. They must not secretly cause a
whole-world recomputation. Large civil-engineering changes can require staged
projects, workers, machines, magic, or authority rather than allowing a tiny
tool to flatten a continent instantly.

## Authority reference

`15-TERRAIN-TECHNICAL-BLUEPRINT.md` is the detailed proposed data and
performance contract. This document states the product intent; the blueprint
states how a prototype should prove it.
