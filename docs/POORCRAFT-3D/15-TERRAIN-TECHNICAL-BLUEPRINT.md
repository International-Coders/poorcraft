# Terrain Technical Blueprint

## Status and purpose

This is the proposed technical baseline for the first POORCRAFT 3D terrain
prototype. Values are deliberately marked as proposals until performance tests
and the owner’s answers confirm them. The goal is natural, editable, cave-capable
terrain that remains cheap enough for ordinary hardware.

## Required properties

- A player can see natural hills, valleys, cliffs, coasts, caves, arches, and
  overhangs.
- A player can build with legible, block-like construction pieces.
- A shovel-like tool can alter soil and slopes without requiring a global
  terrain rebuild.
- The same seed and edit history create the same world on every host.
- Collision, navigation, lighting, fluid queries, and rendering agree about
  solid space.
- Distant terrain is inexpensive; only nearby/visible terrain receives full
  detail.

## Spatial hierarchy: proposed defaults

| Level | Proposed size | Purpose |
|---|---:|---|
| World coordinate | meters, 64-bit integer or fixed-point | stable large-world identity |
| Macro region | 256 x 256 meters horizontal | climate, watershed, biomes, major sites |
| Terrain patch | 16 x 16 x 16 meters | streaming, edits, meshing, persistence unit |
| Base terrain cell | 1 meter cube | natural density/occupancy evaluation |
| Construction cell | 1 meter cube, optional finer decoration | block placement and stable interaction |
| Fine surface detail | shader/detail map or local 0.5-meter sampling | close-range silhouette only when budgeted |

The 16-meter patch is a management unit. It does not mean that every patch is
rendered as a 16-meter cube or that all patches have equal resolution.

## Three terrain layers

### 1. Immutable procedural base

The seed, generator version, macro climate, landmass, biome fields, resource
distribution, cave fields, and initial watershed graph produce the starting
world deterministically. This data is regenerated when a patch has no edits;
it does not need to be stored as full voxels.

### 2. Persistent natural-terrain edits

Sculpting, digging, filling, erosion-like gameplay changes, dams, and other
terrain modifications are stored as ordered operations or a compacted patch
snapshot. Each operation has an id, author, tick, shape, material, and bounds.
When a journal becomes expensive, compact it into a versioned patch delta with
the same deterministic result.

### 3. Construction overlay

Player-built blocks, walls, floors, machines, doors, roads, and castle modules
are explicit construction data. Construction wins over the natural base where
their volumes overlap. It has material, ownership, health, interaction,
collision, and save/network identity.

This separation prevents a smooth terrain edit from destroying the semantics of
a built machine, while allowing construction to sit naturally in the world.

## Density and material evaluation

For a local world position `p`, evaluate in this conceptual order:

```text
base_density(seed, generator_version, p)
  -> apply terrain edit operations affecting p
  -> resolve material strata / soil / rock / cave wall
  -> apply construction occupancy and material override
  -> expose final solid/empty/material result to mesh, physics, and gameplay
```

The exact implementation can use a signed distance field, fixed-point density
field, or a compact occupancy field. The essential contract is that all
subsystems see the same final answer for a coordinate.

## Generation pass order

1. Macro elevation and climate fields.
2. Biome assignment and transition weights.
3. Watershed and river candidates from macro slopes.
4. Local height shaping: valleys, plateaus, ridges, shores, cliffs.
5. Density fields for caves, ravines, arches, and overhangs.
6. Strata/material assignment and ores/resources.
7. Surface decoration, vegetation, ruins, and site reservations.
8. Castle/village/road candidates after terrain and hydrology constraints.

Major terrain and river decisions happen before structures. Structures adapt to
land; they do not flatten every interesting landform into a square.

## Seed and biome constraints

The seed must reproduce the world, but generation is not unconstrained noise.
Biome and hydrology rules influence the result: mountains receive their own
height/shape profiles; coasts may become beach, plain, cliff, marsh, or forest;
rivers create wetter corridors where dense temperate forest, river jungle,
wetland, farms, and settlements can arise. The player should recognize that a
place’s geography suggests different survival, building, and empire choices.

The seed chooses *where* major capitals and opportunities occur. Generator
constraints choose whether a candidate makes sense: a capital must have terrain,
approach routes, water/resource logic, and enough district space for its
faction. This keeps worlds varied without placing a proud fortress in a random
unreachable hole.

## Meshing

Natural terrain uses a surface extraction algorithm such as dual contouring or
marching cubes, chosen by prototype comparison. Dual contouring is attractive
because it preserves sharp features when needed; marching cubes is simpler but
may require careful material/seam handling. Construction can use greedy block
meshing or authored module meshes.

The prototype must compare at least two meshing approaches on the same scenes:

- smooth hill and riverbank;
- cliff with sharp rock layers;
- cave entrance and interior;
- player-dug slope beside block construction;
- adjacent patches at different LODs.

Choose from measured frame time, memory, collision quality, seam behavior,
edit rebuild cost, and art-direction readability—not a tutorial preference.

## Streaming and LOD

Use concentric interest rings around every player. Proposed starting targets:

- 0–96 meters: full terrain patch data, collision, edits, and detailed mesh.
- 96–320 meters: lower-resolution mesh and essential interaction summaries.
- 320–1,024 meters: macro terrain mesh, simplified landmarks, no detailed
  collision.
- beyond 1,024 meters: horizon representation or no draw according to fog and
  view-distance setting.

Values must be configurable and benchmarked. Mesh jobs run through bounded
queues. A fast flight or teleport must not create unlimited meshing work or
freeze the frame.

Adjacent LODs require a seam solution—transvoxel-style transitions, skirts
only where visually acceptable, or an equivalent tested method. Cracks in
visual meshes and collision are release-blocking terrain defects.

## Collision, navigation, and lighting

- Collision is generated from the same final terrain/construction result but
  may be simplified from the visual mesh.
- The player uses a stable capsule or controller against collision triangles
  or a conservative occupancy representation.
- Navigation builds local walkability and portal links per patch; it does not
  run arbitrary global voxel searches every frame.
- Lighting and ambient occlusion query solid/empty/material data through the
  same world service. Caves must have reliable boundaries and no light leaks.

## Terrain editing

The first tool set should be deliberately small:

- dig/lower with a bounded brush;
- fill/raise using acquired material;
- flatten to a selected plane or local slope;
- place/remove construction pieces;
- inspect slope, soil, water, and ownership constraints.

Every edit reports affected patches, rebuild queue state, and nearby hydrology
in debug tools. An edit at a patch edge must produce the same result regardless
of command arrival order or client view.

## Hydrology handoff

Terrain does not solve water alone. It publishes slope, boundary portals,
basins, channel geometry, and terrain revisions to the water system. Water
publishes flow records and local wetness/volume effects back to rendering,
machines, fishing, and NPC queries. See `05-WATER-FLOW-AND-ENVIRONMENT.md`.

## First performance budgets: proposals

These are test targets to refine after profiling, not promises:

- no frame hitch longer than 16 ms attributable to one ordinary terrain edit
  on the target baseline machine;
- bounded mesh queue with visible backlog counter;
- fixed upper bound on generated patches and GPU mesh memory per view tier;
- one patch rebuild independent of total world size;
- no full-world scan for a local terrain or water change;
- deterministic patch hashes for the same seed and edit journal.

## Terrain proof suite

The first terrain milestone requires automated and visual evidence for:

1. same seed + same edits => identical patch hash;
2. save/reload preserves cave, slope, and construction edge edits;
3. adjacent patch and LOD seams have no gaps;
4. player/NPC collision agrees with the rendered cave and overhang;
5. a local edit remeshes only bounded affected patches;
6. river/canal rebuild scope is local and deterministic;
7. low, medium, and high quality tiers retain readable terrain without
   exceeding their declared memory and frame budgets.
