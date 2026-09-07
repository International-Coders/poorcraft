# The First Honest POORCRAFT 3D Vertical Slice

## Player promise

A player launches POORCRAFT 3D and lands in a first-person seed world. They
walk from a natural hill into a visible cave mouth, cross a river, place a few
construction blocks near a small city, see its castle silhouette and several
NPCs, inspect a Bed/Work/Idle box, save, restart, and return to the same
changed place.

This is the first feature that may be called “POORCRAFT 3D playable.” It is
intentionally small. It proves the actual game direction before dragons,
nuclear plants, many factions, or 128-player servers receive visual polish.

## Required scene

One deterministic showcase seed must include, within a short walk:

1. A smooth-ish natural hill and a cliff.
2. A cave or overhang with working collision and visible interior lighting.
3. A flowing river with width, depth, direction, and a visible water surface.
4. A player construction area containing stone and wood blocks.
5. A small castle/city silhouette: gate, wall, tower, house, and workshop.
6. Three NPCs: a resident, worker, and guard.
7. One Bed Box, Work Box, and Idle Box drawn/inspectable in-world.
8. A first-person camera, movement, collision, ray target, place/remove action,
   and save/reload action.

The showcase may use original placeholder geometry and simple materials. It
must not use a 2D map, a text report, an invisible entity, or a fake screenshot
in place of these objects.

## First visual art direction

- Natural land is smooth-ish and low-poly/voxel-readable, not a field of
  perfect cubes and not photorealistic terrain.
- Construction remains clearly block-built and grid-aligned.
- Castle modules use readable silhouettes before fine detail: gatehouse, wall,
  tower, roofs, chimney/forge, banners or faction color family.
- NPCs can start as articulated low-poly/voxel figures with role color/props.
- Water must read as moving directional water, not a flat blue block.
- The scene must be attractive on ordinary hardware with raster rendering.

## Acceptance evidence

The slice closes only when all of these exist:

- source tests for world-to-mesh and save/reload correctness;
- a windowed executable with keyboard/mouse movement;
- a real screenshot from the windowed renderer, not the old atlas generator;
- visual pixel/semantic assertions for terrain, water, construction, city, and
  NPC presence;
- human inspection of the screenshot and a short manual movement/build test;
- a performance record at low, medium, and high quality tiers;
- a written list of temporary placeholders and their asset-manifest rows.

## Explicit non-goals for this slice

- final character animation;
- final faction art kits;
- full procedural capital generation;
- polished combat, dragons, nuclear visuals, or multiplayer;
- global terrain erosion or fully physical water;
- every item/recipe from the simulation.

Those features remain valuable only after they can appear in the visible world.
