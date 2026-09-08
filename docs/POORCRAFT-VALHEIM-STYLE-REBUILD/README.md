# POORCRAFT: Stylized Natural-World Rebuild

## Why this pack exists

`poorcraft3d/` is now a real, fast Rust/wgpu proof-of-concept: it opens a 3D
window, streams terrain, supports walking, construction, a river, a city,
NPCs, save/load, and visual regression gates. It is **not** yet the intended
game presentation. Its natural ground is culled one-metre cube faces; its
castle, people, and water wheel are procedural primitive meshes; and its
materials are mostly flat colours plus one procedural detail texture.

This pack gives Z.ai / GLM 5.3 an unambiguous next assignment: evolve the
working baseline into an original, lightweight, natural-world fantasy sandbox.
The target feeling is organic terrain, tactile building, readable settlements,
and a world worth exploring—not a blocky Minecraft-like visual style.

"Valheim-like" is only shorthand for the *design qualities* (stylized natural
terrain, low-poly readability, survival crafting, and atmospheric exploration).
Do not copy its art, code, characters, names, models, UI, sounds, or gameplay
content. The same applies to every other commercial game reference.

## The decisive technical direction

- Keep the engine **Rust-first**: `winit` + `wgpu`, the existing pure world
  simulation, and native Steam Deck/Linux support.
- Do not migrate to Unity, Unreal, Bevy, Python runtime, or a mixed C++ engine.
  Python/Blender scripts are allowed only as offline asset-build tools. Add C/C++
  only after a profiler identifies one isolated hot path that Rust cannot meet.
- Keep deliberate player construction as grid-aligned blocks/modules.
- Replace *natural* block terrain with a chunked, low-poly heightfield surface;
  retain a sparse volumetric path only for caves and overhangs.
- Replace primitive-only presentation with original, source-controlled glTF
  assets, real material textures, LODs, collision proxies, and runtime proof.
- Preserve all currently passing rendering gates while adding the new ones.

## Reading order

1. [00-ZAI-GLM-5.3-MASTER-GOAL.md](00-ZAI-GLM-5.3-MASTER-GOAL.md) — paste this
   into Z.ai as the slash-goal.
2. [01-VISION-AND-NON-NEGOTIABLES.md](01-VISION-AND-NON-NEGOTIABLES.md)
3. [02-BASELINE-AND-ARCHITECTURE.md](02-BASELINE-AND-ARCHITECTURE.md)
4. [03-NATURAL-TERRAIN-AND-WATER.md](03-NATURAL-TERRAIN-AND-WATER.md)
5. [04-ORIGINAL-3D-ASSET-FACTORY.md](04-ORIGINAL-3D-ASSET-FACTORY.md)
6. [05-ROADMAP-AND-QUALITY-GATES.md](05-ROADMAP-AND-QUALITY-GATES.md)
7. [prompts/](prompts/) — one bounded execution prompt per milestone.
8. [contracts/rebuild_roadmap.json](contracts/rebuild_roadmap.json) — the
   machine-readable task order and completion evidence.
9. [assets/ORIGINAL-ART-BRIEF.md](assets/ORIGINAL-ART-BRIEF.md) and
   [assets/first_asset_batch.json](assets/first_asset_batch.json) — the exact
   first original GLB assets GLM must make after the baseline is secure.
10. [capability_inventory.json](capability_inventory.json) — the NWR-001
   baseline: what every renderer path ACTUALLY is today, enforced against
   reality by `pc3d_render::inventory` (a false "shipped asset" claim fails
   the suite). Later milestones update rows only by shipping code + proofs.

## The one rule that prevents another wrong turn

Do not call an item complete because it compiles, has unit tests, or produces
a diagnostic PNG. A visual change is complete only when a human can inspect a
windowed capture of the actual game scene, the relevant pixel/semantic tests
pass, and the feature is connected to authoritative game data.

## Working method

Execute the roadmap in order. Finish one milestone, verify it, make a focused
commit according to `AGENTS.md`, update the project bookkeeping, and only then
advance. Do not delete `poorcraft3d/`, its existing gates, saves, or working
renderer. This is a staged replacement of its presentation layer, not an
unbounded rewrite.
