# LOREFORGE asset + rendering plan

This plan turns the game's strong procedural catalog into a readable voxel-RPG
art pipeline without making ray tracing mandatory. It uses three different
tools for three different scales:

- **Alpha cutout / impostor**: transparent sprite cards for thin items and
  vegetation. This is the useful part of the “chroma key” idea; transparent
  pixels replace a literal keyed background color.
- **Normal map**: the colorful RGB image that stores which way each surface
  pixel points. Raster lighting uses it to fake grooves, bevels, cloth folds,
  and tiny self-shadowing without adding geometry.
- **Real voxel geometry**: reserved for silhouettes that must hold from every
  angle—heads, limbs, armor, large tools, machines, and bosses.

Live RT remains an optional presentation mode. The normal/impostor path is the
default and must remain playable on ordinary integrated GPUs.

## Stage 0 — foundation (shipped loop 338)

- [x] Linear-space normal-map array for every base, mod, entity, and item
  layer, plus the CTM strip and data-driven dynamic atlas rewrites.
- [x] Cheap directional relief term in the raster shader.
- [x] CTM marker namespace moved to 4096+ so it cannot collide with real atlas
  layers (the former 165+ markers collided with tree/biome/skin art).
- [x] Seven ordinary-villager job skins and a neutral network-player skin.
- [x] Shared six-part humanoid geometry with yaw, gait, and crouch controls.
- [x] Crossed, double-sided alpha-cutout world drops using the exact inventory
  sprites; block drops keep small cubes.
- [x] `entity_skins` is now a close proof shot with articulated people and
  eight recognizable item silhouettes.

## Stage 1 — character art grammar

- [ ] Replace “one 16×16 image wrapped over every body part” with a compact
  per-part UV atlas: face/hair, torso front/back, sleeves, legs, and boots.
- [ ] Add reusable geometry attachments (hat, hood, apron, backpack, shield,
  staff) selected by job/faction instead of baking every distinction into RGB.
- [ ] Give NPC state a facing direction so schedules and conversations orient
  bodies deliberately; preserve deterministic idle poses for proofs.
- [ ] Add first-person hands and a third-person local-player model using the
  same skin contract as remote players.
- [ ] Proof: front/side/back character turntable plus readable silhouettes at
  6, 16, and 32 blocks.

## Stage 2 — item presentation

- [ ] Add hand anchors and per-item pose metadata (grip, rotation, scale).
- [ ] Use simple 3D voxel meshes for “hero silhouette” classes—swords,
  pickaxes, bows, shields—while flat food/materials remain impostors.
- [ ] Add a two-level distance rule: hero mesh nearby, crossed sprite at range.
- [ ] Put real item sprites on conveyor belts and in machine input/output slots.
- [ ] Proof: dropped/held/belt item matrix from four camera angles; no generic
  stone/log fallbacks anywhere in the registered catalog.

## Stage 3 — authored materials

- [ ] Allow explicit `*_normal.png` overrides for signature blocks and mod
  packs; generated maps remain the guaranteed fallback.
- [ ] Add one packed material map: roughness, metalness, emissive strength, and
  optional height. Keep it one extra lookup, not four textures.
- [ ] Author a small high-impact set first: carved stone/oak/iron, ores,
  bricks, machine panels, wet blocks, magic blocks, and armor.
- [ ] Add parallax/height sampling only for the nearest opaque blocks and only
  at High/Ultra quality; never apply it to distant terrain or cutout sprites.
- [ ] Proof: fixed-camera material lineup under morning/noon/night light plus a
  shader-off reference image proving that the map changes visible relief.

## Stage 4 — cheap shadows and depth cues

- [ ] Add soft ellipse/capsule contact shadows under humanoids, mobs, and
  dropped items, scaled by body/item bounds and faded by height.
- [ ] Add sun-facing projected sprite shadows for a curated set of thin items
  and foliage. These are masks, not expensive geometry or rays.
- [ ] Add screen-space ambient contact only if the GPU budget survives Stage 5;
  keep the current baked voxel AO as the baseline.
- [ ] Proof: shadow silhouettes follow entity/item motion and do not draw
  through water, walls, or steep terrain.

## Stage 5 — performance and LOD contract

- [ ] Quality presets explicitly control normal strength, parallax distance,
  contact-shadow count, and hero-item mesh distance.
- [ ] Entity mesh LOD: articulated bodies near, simplified bodies mid-range,
  billboard impostors only at long range.
- [ ] Atlas memory is reported in F3; startup refuses to exceed the adapter's
  array-layer limit and names the offending mod pack.
- [ ] Every material/character pass runs `make perf`; target is no more than a
  10% p95 regression on the reference scene and no new per-frame texture
  uploads.

## Definition of done for every stage

Code + contract tests, a deterministic vistest scene with pixel assertions,
human inspection of its PNG, workspace build/tests, smoke, perf for shader or
LOD work, fresh runtimes, bookkeeping, commit, and GitHub push. Art claims are
never accepted from catalog counts alone.
