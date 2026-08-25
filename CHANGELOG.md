# CHANGELOG

## 2026-08-25 — P4: survival & inventory UI (loop 286)
- Migrated winit 0.29 -> 0.30 (ApplicationHandler) and adopted egui 0.31
  (matches wgpu 24; the 0.29 stack was incompatible).
- lf_game: items registry (block items, tools with tiers, food, materials),
  mining rules (hardness, tool multipliers, harvest gating, break times),
  shaped crafting with translation-aware matching (+13 tests total).
- Blocks: crafting table; torch item texture; CRAFTING_TABLE id 14.
- Client: egui HUD + inventory/crafting/death screens with full stack
  interactions (pick/place/swap/merge/split); hold-to-mine with progress;
  tool durability; item drops (gravity, magnet pickup, bobbing cubes);
  hunger/regen/fall-damage/drowning/death+respawn; RMB context (open table,
  eat, place); inventory/stats/time persisted in player_extras.dat.
- Tests 69 -> 82; game smoke-tested with UI active; all scenes render.

## 2026-08-25 — P3: lighting & atmosphere (loop 285)
- lf_voxel light.rs: per-column flood-fill sky + block light (BFS with -1
  falloff, opacity-aware, 15-level); emitter table (torch=14, lantern=15);
  index-stride bug class caught by tests (y must own the largest stride).
- Mesher samples light from the exposed cell per face (packed sky<<4|block
  in the vertex light attribute; was hardcoded 15).
- Shader: brightness = max(sky*day, block*0.92) with ambient AO and distance
  fog blending to the sky color; uniforms carry camera pos + day + fog.
- Water: separate alpha-blended pipeline (no depth write), water faces split
  from opaque mesh, columns sorted back-to-front; water texture alpha 170.
- Torch/lantern blocks (non-solid, non-opaque, targetable).
- Client: 20-minute day/night cycle (lf_game::TimeOfDay) driving sky clear
  color, day factor and fog; torch in the 9-slot hotbar; deeper night
  constants (starlight 0.12, night sky mix 0.15).
- vistest: torchlit_night scene (torch grid on terrain at night); scene sky
  math now reuses lf_game::TimeOfDay.
- Tests 66 → 69; all 6 scenes render; game smoke-tested with lighting.

## 2026-08-25 — P2: world streaming & terrain (loop 284)
- lf_voxel: block registry (is_solid/is_opaque/is_targetable, 12 blocks);
  mesher culls by opacity (air/water/leaves show faces behind them, no
  water-water faces); ChunkColumn serializable; WorldStorage saves chunk
  columns via region files + player.dat (+round-trip tests).
- lf_worldgen: trees on meadows (deterministic hash placement, canopy kept
  in-chunk), 3D-noise caves, coal (<y96) and iron (<y48) ores in stone,
  water fills to sea level; 4 feature tests over real generated chunks.
- lf_assets: log/leaves/coal/iron/water textures (11-layer atlas).
- lf_client: background chunk streamer (worker thread, nearest-first,
  view radius 5, unload radius 8 with save-before-drop), sphere-frustum
  column culling from mesh bounds, world persistence with 30s autosave and
  save on exit, player position/look restored from save, hotbar 8 slots.
- vistest: terrain_features scene; renders verify trees (~7% canopy pixels)
  and water (~20% water pixels) visible.
- Tests 58 → 66. Game smoke-tested 20s with streaming active.

## 2026-08-25 — P1: first-person playable core (loop 283)
- lf_voxel: World + ChunkColumn (16 sections = 16x256x16), world-coord
  get/set with chunk border math, surface_height, mesh_column with cross-
  border neighbor culling; 4 new tests.
- lf_worldgen: generate_chunk fills a ChunkColumn directly; surface_top()
  helper (surface band tops out at height+3, standing surface height+4).
- lf_game: Player with AABB physics — gravity, jumping, sprint/sneak/walk
  speeds, fly mode, axis-separated collision with substepping so long falls
  never tunnel; 8 physics tests (landing, walls, jump arc, ceiling, fly,
  tunneling, look dir).
- lf_engine: scene split into SceneResources (shared pipeline + atlas) and
  MeshBatch (per-column drawable); OutlineScene line pipeline renders the
  targeted block outline.
- lf_client (replaces dummy): the game shell — winit input with cursor
  lock, player update loop, DDA raycast targeting with outline, instant
  break / face-adjacent place (with player-overlap rejection), 6-slot
  hotbar (digits + scroll, shown in window title), F2 screenshots via the
  offscreen renderer, per-column GPU remesh on edit.
- vistest: first_person_view scene with vista-seeking camera (moderate-drop
  selection, constrained to the meshed area); scenes use World pipeline and
  radius-3 meshes.
- Tests 43 → 57. Proofs: shots/vistest_first_person_view.png (eye-height
  view over real terrain), plus re-rendered dawn/vista/night scenes.

## 2026-08-25 — P0: honest baseline (loop 282)
- AUDIT: found that loops 26–281 (~256 "Evolution Mode" loops) changed only
  BACKLOG.md/STATE.md — no code, data, or tests. All their claimed features
  (Nether, Ender Dragon, shaders, ~230 items) were never implemented. Docs
  reset to reflect reality; backlog re-planned as P1–P11.
- Fixed: region storage no longer overwrites neighbor chunks in the same
  region (region files now hold all chunks keyed by (x,z), atomic tmp+rename
  writes). 4 regression tests added.
- Fixed: worldgen biome selection now uses elevation; all 8 biomes are
  reachable (verified by a sampled world-sweep test). Height range stretched
  to 24..176 so deep oceans and peaks exist. SEA_LEVEL = 62 constant added.
- Renderer: added depth buffer (Depth32Float) to the pipeline; extracted
  shared GpuScene (pipeline + buffers + texture array) used by windowed app
  and headless path alike; Camera moved to lf_engine::camera.
- New: lf_engine::headless — offscreen wgpu render to PNG (real GPU output,
  256-byte-aligned readback, sRGB-correct).
- New: lf_vistest — real scene registry; scenes build actual terrain from
  lf_worldgen (seeded, deterministic), mesh it with lf_voxel, render via
  lf_engine. Unit tests: registry uniqueness, non-empty deterministic meshes,
  seed sensitivity, unknown-scene error.
- New: xtask `vistest` and `screenshot` commands render real PNGs;
  `cargo run -p loreforge -- --headless --scene NAME [--seed N] --out PATH`
  now genuinely renders (CI's old command shape made real).
- CI: vistest job now runs the real harness and uploads PNG artifacts.
- Meshing: per-block texture atlas indices emitted by the mesher (was:
  everything hardwired to grass); added meshing unit tests (face counts,
  culling, per-block tex_index).
- lf_assets: 6 procedural textures (stone/grass/dirt/sand/mycelium/snow),
  atlas + block-id→layer mapping, overflow-safe color math, tests.
- Tests: 27 → 43 passing across the workspace.

## Loops 1–25 (original milestones — real work)
- loop 1: initialized STATE, BACKLOG, DECISIONS, CHANGELOG, and shots dir. build green.
- loop 2: added m1 window screenshot proof to shots/m1_window.png. build green.
- loop 3: implemented M2 textured chunk rendering; saved shots/m2_chunk.png. build green.
- loop 4: implemented DDA voxel raycast in lf_voxel with unit tests; saved shots/m3_breakplace.png. build green.
- loop 5: implemented lf_worldgen noise heightmap, biomes, and strata with tests passing; saved shots/m4_terrain.png. build green.
- loop 6: implemented region storage persistence and chunk round-trip test; saved shots/m5_save_load.png. build green.
- loop 7: implemented TimeOfDay day/night cycle, sky color transition, LightEngine with torch block light; saved shots/m6_night.png. build green.
- loop 8: implemented survival core player stats and inventory item stacking; saved shots/m7_survival.png. build green.
- loop 9: implemented medieval smithing system with 8 materials (wood to adamantine), tool parts, assembly, forge minigame; saved shots/m8_forge.png. build green.
- loop 10: implemented mobs (Boar, Woolbeast, Glitchling, Stalker, Crawler, Null Knight boss) and combat system; saved shots/m9_boss.png. build green.
- loop 11: implemented lf_modapi mod loader for TOML manifests and datapacks with ember_ores example mod; saved shots/m10_mod.png. build green.
- loop 12: implemented protocol codec (handshake/login/chat/messages) and dedicated loreforge-server binary; captured shots/m11_two_players.png. build green.
- loop 13: implemented villagers with VillagerJob, VillagerSchedule, utility AI two-tier dialogue system (data-driven + optional LLM fallback); captured shots/m12_village.png. build green.
- loop 14: implemented story mode quests, objective types, and quest log tests; captured shots/m13_quests.png proof. build green.
- loops 15–25: chronicle engine, quests, milestone proofs, amberium mod, crystal/obsidian content, Geode Guardian, Cinder Crawler (all with code + tests).
