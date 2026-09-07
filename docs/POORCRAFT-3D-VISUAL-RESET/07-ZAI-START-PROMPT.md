# Z.AI Start Prompt: Build the Visible POORCRAFT 3D Game

```text
You are continuing POORCRAFT 3D in /poorcraft3d. Do not work on the completed
pure-simulation roadmap. The current project has deterministic simulation but
NO windowed 3D renderer, NO GPU shader, NO real terrain mesh, NO 3D asset
pipeline, and NO first-person visual client. Do not call CLI atlases or debug
PNGs a rendered game.

Read AGENTS.md, STATE.md, every file in docs/POORCRAFT-3D-VISUAL-RESET/, and
the product decisions in docs/POORCRAFT-3D/ before changing code. Preserve
unrelated dirty edits in poorcraft3d/crates/pc3d_world/src/host.rs and
journey.rs. They are not renderer work.

The immediate goal is R3DV-001 from 06-EXECUTION-QUEUE.md: add a P3D-local
windowed winit/wgpu renderer crate, render a nonuniform GPU scene, handle
resize/surface loss, and create a real screenshot proof. Work one task only.
Do not begin terrain meshing, asset generation, NPCs, castles, new simulation,
or another roadmap rewrite until the task is green.

Architecture is binding:
- pc3d_world remains pure authoritative simulation and must not depend on wgpu,
  winit, UI, textures, or rendering types.
- pc3d_render reads visual snapshots/queries; rendering never mutates
  canonical world state.
- pc3d_assets owns manifest validation, asset lookup, material metadata, and
  compiled asset paths.
- apps/poorcraft3d is the thin input/window/presentation shell.

Use docs/POORCRAFT-3D-VISUAL-RESET/zai_visual_gate.json and
tasks/render_execution_queue.json as mandatory task gates. A task is complete
only with source tests, a real screenshot/readback, human visual inspection,
and proportional performance evidence. Do not mark a task complete based on a
compile, unit test, headless CLI output, or a generated 2D atlas alone.

Use original POORCRAFT expression only. Do not use Warcraft, Heroes of Might
and Magic, Minecraft, Skyrim, All the Mods, or any other game's branded names,
lore, assets, UI, code, models, textures, or one-for-one factions.
```
