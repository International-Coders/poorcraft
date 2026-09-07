# POORCRAFT 3D Visual Reset

## Purpose

This folder corrects the current POORCRAFT 3D execution course.

`poorcraft3d/` has a substantial deterministic simulation, save, networking,
world-generation, NPC, settlement, machine, faction, and test foundation. It
does **not** currently have a windowed 3D renderer, GPU shaders, a terrain mesh,
a water mesh, a 3D asset loader, or a first-person visual client. Its atlas and
flow-map PNGs are diagnostics, not in-game rendering.

The next agent must use this pack to turn that simulation into a playable,
visible voxel game. Do not add another simulation-only subsystem until the
visual vertical slice in `01-3D-VERTICAL-SLICE.md` passes.

## Read in this order

1. `00-REALITY-CHECK.md`
2. `01-3D-VERTICAL-SLICE.md`
3. `02-RENDERER-ARCHITECTURE.md`
4. `03-VOXEL-TERRAIN-AND-MESHING.md`
5. `04-3D-ASSET-PIPELINE.md`
6. `05-CASTLES-NPCS-AND-CITY-PRESENTATION.md`
7. `06-EXECUTION-QUEUE.md`
8. `07-ZAI-START-PROMPT.md`
9. `zai_visual_gate.json`
10. `tasks/render_execution_queue.json`
11. `assets/asset_manifest.schema.json`
12. `assets/beta_critical_assets.json`

## Authority

When this folder conflicts with an older P3D document, this folder controls
the **visual-rendering execution order**. The product decisions in
`docs/POORCRAFT-3D/00-DESIGN-CONSTITUTION.md` through
`21-FIRST-PERSON-WAR-KARMA-AND-SERVER-SCALE.md` still control the game’s
identity.

The project name is **POORCRAFT 3D**. Do not use Warcraft names, lore, assets,
UI, or other franchise expression. Common fantasy archetypes must be expressed
through original POORCRAFT factions, architecture, characters, and content.

## Non-negotiable reset rule

The phrase “POORCRAFT 3D is playable” is forbidden until a human can launch a
windowed executable and, without a CLI diagnostic:

- see a seeded 3D terrain world;
- move in first person and collide with actual rendered terrain;
- see a cave/overhang, river water, construction blocks, a castle/city
  silhouette, and NPCs;
- place and remove a construction block;
- read Bed, Work, and Idle boxes as real world locations;
- save, reload, and see the same changed world.

## Pack contents

- Markdown documents define the engineering and player-visible contracts.
- JSON files are machine-readable gates, task ordering, and asset manifests.
- No generated image, mesh, or model is accepted as an asset until it has a
  manifest record, a runtime consumer, a material, an LOD/collision decision,
  and a rendered proof.
