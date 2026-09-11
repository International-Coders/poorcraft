# Material, LOD, And Collision Rules

## Materials

- Use original POORCRAFT identity, not borrowed branded looks.
- Name materials by gameplay role: `warm_forge_metal`, `wet_river_wood`,
  `cold_stone_wall`, `faction_red_banner`, `mana_crystal_core`.
- Declare alpha mode, emissive state, atlas/source path, and whether the
  material is interactively important.
- UI textures with alpha must preserve RGBA and be tested over light and dark
  backgrounds.

## LOD

- Every high-volume prop needs at least LOD0 and LOD1 or a documented reason.
- LOD switch must preserve silhouette for doors, ladders, handles, signs,
  NPC faces, and machine affordances.
- Impostors may not hide interactive handles or doors within interaction
  distance.

## Collision

- Collision must match what the player expects, not every decorative detail.
- Doorways need a clearance box large enough for the player capsule.
- Workspots need a clear approach path.
- Bridges and stairs need walkable slopes and edge blockers where appropriate.
- Machines need solid bodies but open interaction volumes.

## Navigation

- NPC nav and player collision must share enough data to avoid "NPC walks
  through wall, player cannot" contradictions.
- Each building declares interior nav nodes and exterior approach nodes.
- If an object blocks nav when closed and opens nav when used, both states
  must be exported and tested.
