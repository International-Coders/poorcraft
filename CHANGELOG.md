# CHANGELOG

## 2026-08-26 — P22: UI overhaul — icons, tooltips, recipe book, map suite (loop 302)
- Real pixel-art item icons in every slot (hotbar, inventory, crafting grid,
  chests, furnaces, machines, trades, cursor, tooltips): `lf_assets`
  sprite generator for all non-block items (tools per tier, ingots, raw
  ores, food, armor, industrial parts) + deterministic gem icons for mod
  items; block items reuse atlas art.
- Texture atlas 18 -> 41 layers: wood-variant logs/leaves, red sand,
  terracotta, moss, ice, copper/tin/bauxite/sulfur ores and all six
  machine/bench blocks now render in-world with their own art (they
  previously fell back to stone).
- Tooltip system: icon + display name, tool tier/damage/speed, food value,
  armor points, fuel seconds, "smelts into", crusher input, era-requirement
  badge, stack size; recipe-book entries preview their pattern as a
  mini-grid on hover.
- Crafting screen redesign + recipe book panel: unified catalog merging
  crafting (vanilla + mods), smelting, assembler alloys and crusher
  recipes; search box, station tabs, craftable-only filter; ingredient
  icons with have/need coloring; click auto-fills the grid from the
  inventory (returning grid contents first); "needs table" and era-lock
  badges.
- Shift-click quick-move everywhere: inventory storage <-> hotbar, chest
  <-> inventory, furnace/machine slots <-> inventory.
- Map suite: top-right minimap (terrain/biome colors, entity dots,
  waypoint pips, player arrow, north marker, toggle in settings) and the
  full M-key world map — pan/zoom, fog of war, explored-but-unloaded
  approximation dimmed, spawn marker, waypoint manager (add/rename/
  recolor/delete, persisted in ClientSave), cursor coords + biome,
  chunk grid at high zoom. HUD info line gains compass facing + biome.
- HUD polish: icon hotbar with pulsing selection glow and fading item
  name on switch, armor points, XP bar mirroring the hotbar width with
  level chip + gain flash, dynamic crosshair (expands while mining,
  hit-marker on attacks), hurt vignette + low-health pulse, redesigned
  death screen with run stats. Settings gains an Interface tab (minimap
  toggle, UI scale driving egui zoom).
- Fixes: `crafting::recipes()` leaked a Vec per call (now a OnceLock
  singleton), `Inventory::add_item` ignored per-item max_stack, crusher
  catalog listed `iron_ore` (a block id, not an item), and the vistest
  harness never rendered egui windows (fresh contexts need a warmup pass
  for window areas, whose font-atlas texture delta must be threaded to
  the renderer — the pre-P22 trade/tech proof shots had silently empty
  windows).
- Proofs: 3 new scenes (crafting_ui, map_screen, minimap_hud), all 19
  scenes re-rendered and pixel-verified (panels present, icons/text
  visible). Tests 123 -> 140.

## 2026-08-25 — P21: menus, animations, HUD & real settings (loop 301)
- ui_kit: theme, easing (+tests), Reveal stagger, animated menu buttons
  (hover glow, press spring, accent bar), slide panels, toggles, sliders,
  section headers, painted vector heart/hunger glyphs.
- Title screen: pulsing logo, staggered buttons, live orbiting world
  background behind the menu; pause menu as an animated slide-in panel.
- Settings screen with Video/Audio/Gameplay tabs and quality presets,
  persisted with the world; every knob drives the engine live (view
  distance feeds the streamer, FOV the camera, invert-Y the mouse,
  clouds/particles gate the atmosphere batches).
- Ray tracing settings made real: Off / Captures(R) / **Live** — a
  persistent Pathtracer reuses GPU resources and traces every frame at a
  configurable internal scale, shown fullscreen beneath the HUD.
- HUD rebuilt: painted hearts/hunger, XP bar with level chip, hotbar with
  hover tooltips, info line (clock, weather, net, FPS, RT flag).
- Proofs: menu_preview (dark-panel 20% + logo light 2.3% in the center),
  settings_preview (window clearly visible). Tests 119 -> 123.

## 2026-08-25 — P20: final consolidation (loop 300)
- All 14 vistest proof scenes render and pixel-verify: biome_montage,
  clouds_weather, first_person_view, hud_preview, industrial_machines,
  night_watch, raytraced_night (100% emissive), raytraced_shadows,
  spawn_plains_dawn, tech_tree, terrain_features, terrain_vista,
  torchlit_night, village_trading.
- STATE/STATUS/BACKLOG/RELEASE reflect exactly what exists.
- 121 tests green; game smoke-tested end to end.

## 2026-08-25 — P19: Steam readiness (loop 299)
- lf_steam crate: feature-gated Steamworks binding (off by default — the
  SDK links dynamically and CI lacks the client); preferred_transport()
  reports Steam only after a successful init, else UDP fallback (+2 tests).
- steam_appid.txt (Spacewar 480) for dev testing; title screen shows the
  active transport; docs/STEAM.md covers the dev loop, feature flag, depot
  layout and steamcmd upload.
- Tests 119 -> 121.

## 2026-08-25 — P18: compute voxel path tracer (loop 298)
- lf_engine pathtrace: WGSL compute tracer — DDA primary rays through a
  128x64x128 block clip texture, jittered soft sun shadows, one-bounce GI
  (sky + emissive torches/lanterns), fog, 2x2 supersampling (portable
  write-only storage; read-write accumulation unsupported on this adapter).
- Rust: build_voxel_texture_data from any World; pathtrace_to_image with
  f16 decode -> PNG; headless scene integration.
- Client: R key path-traces the current view in-game and saves
  shots/rt_frame_N.png.
- Proofs: raytraced_shadows (varied terrain lighting, luminance
  transitions), raytraced_night (100% warm emissive coverage with a
  lantern floor in view). Fixed along the way: uniform member order
  mismatch, stale cargo fingerprints masking edits, torch placement living
  in a dead code copy.

## 2026-08-25 — P15+P16: industrial machines & research (loop 297)
- worldgen: copper/tin/bauxite/sulfur veins by depth (+generation test).
- lf_game machines: Generator (EU buffer), ElectricFurnace (2x speed),
  Crusher (ore doubling), Assembler (bronze/steel/circuits/frames) with
  power draw (+5 state-machine tests).
- Client: machine blocks/entities with slots+progress UIs, 4-block power
  field, machines tick while closed, spill on break, persist.
- Research: eras with material costs, ResearchState advance consuming
  inventory (+3 tests); era-gated crafting shows locked recipes with
  requirements; research bench advances eras on RMB.
- Tech tree screen (K): era columns with done/current/locked states, live
  have/need cost colors, and a next-step hint line.
- Bug fixed: headless egui proofs encoded the UI pass after the texture
  readback — UI screenshots silently lacked their UI. Reordered; tech_tree
  proof now shows the panel (verified by pixel analysis).
- Proofs: industrial_machines, tech_tree. Tests 110 -> 119.

## 2026-08-25 — P14: combat & survival completion (loop 296)
- lf_game combat.rs: Arrow projectiles (gravity + solid-hit), XP curve
  (7+3*level) with carry-over levels, armor mitigation (flat, min 1)
  (+3 tests).
- Items: bow/arrow, bronze & steel chestplates (Armor kind), smithing
  table block (+texture); industrial material items (copper/tin/aluminum/
  sulfur/bronze/steel ingots, wire, gear, machine frame, basic circuit)
  with recipes — catalog consistency kept green throughout.
- Client: hold-RMB bow charging with HUD bar, arrows fly and damage mobs,
  XP bar in HUD, worn armor reduces damage, RMB on the smithing table opens
  the forge minigame UI (bellows pump + orange-zone strikes produce steel).
- Tests 107 -> 110.

## 2026-08-25 — P13: NPCs & villages (loop 295)
- lf_npc: trade_offers(job) tables for all six jobs (+coherence test).
- worldgen: hamlets gain dirt paths and lamp torches.
- Client: villagers spawn when hamlet chunks load (deterministic job/name,
  persisted), day-wander/night-rest schedule, RMB opens the trade screen
  with live have/need counts and affordability colors; lore book item opens
  a reading window showing the world chronicle; job-tinted villager cubes.
- Proofs: village_trading scene with a real egui trade panel (verified by
  pixel analysis). Tests 106 -> 107.

## 2026-08-25 — P12: world & atmosphere completion (loop 294)
- Biomes: data-driven table, 8 -> 30 (variant channel splits climate bands;
  census test verifies all 30 occur in a sampled world; per-biome surfaces,
  tree species, freezing oceans cap with ice).
- Blocks: birch/spruce/dark/cherry logs + leaves, pale leaves, red sand,
  terracotta (banded), moss, packed ice (+textures); Badlands strata.
- Trees: per-species shapes (conifer cones, tall jungle, wide cherry/dark
  canopies); >=3 species generation test.
- Atmosphere (lf_engine::atmosphere): drifting cloud layer (transparent
  pass), sun/moon billboards with celestial rotation, night stars,
  underwater fog/tint, weather cycle with rain/snow particles and storm
  sky darkening. Cloud winding bug caught by pixel-proof (visible from
  below only) — fixed.
- World types: Normal/Superflat/Amplified (superflat = flat, no caves/
  ores/structures); title-screen new-world buttons; type persists.
- Proofs: biome_montage (19 green hue buckets = species variety),
  clouds_weather (8.6% white cloud pixels). Tests 104 -> 106.

## 2026-08-25 — P11: performance & release; base game complete (loop 293)
- World light cache with invalidation on edits (+test) — writing the test
  exposed two real bugs: the mesher never actually sampled the light
  closure (per-face light was hardcoded 15 since P3) and section-local y
  indexed the world-height light array. Both fixed; renders now show true
  dynamic range (shadowed overhangs, torch pools, dark nights).
- xtask package: portable dist/ zip with release binaries, mods and docs.
- CI: release matrix (ubuntu/macOS/windows) builds and uploads artifacts.
- RELEASE.md rewritten honestly (how to run, controls, verified features,
  known gaps).
- Tests 103 -> 104. P0-P11 of the base-game plan are complete.

## 2026-08-25 — P10: mod API real (loop 292)
- lf_voxel registry: runtime mod blocks (MOD_BLOCK_BASE + fnv ids) consulted
  by name/is_solid/is_opaque; registration test.
- lf_game: register_mod_item / register_mod_recipe / register_mod_smelt;
  block_drop owned Strings + mod drops.
- lf_worldgen: register_ore_hook + generate consults hooks (+test).
- lf_modapi: smelting.toml parsed in load_mod (was silently dropped);
  apply_mod/load_mods_dir wire mods into the live registries; full-pipeline
  test: parse -> register -> place modded block -> break -> drop -> smelt.
- lf_assets: generic "mod" texture layer; client loads mods/ at boot
  (smoke log: loaded 2 mods).
- mods/README.md documents the mod surface.
- Tests 96 -> 103.

## 2026-08-25 — P9: multiplayer (loop 291)
- lf_protocol v3: ClientMessage/ServerMessage gameplay set with framed
  codec (+4 round-trip and rejection tests).
- lf_server (real now): UDP authoritative-lite server — canonical world
  (worldgen on demand + validated edits + history replay to newcomers),
  20/s player snapshots, chat relay, join/leave roster; a two-client
  integration test runs over real localhost UDP and passes.
- loreforge-server binary: dedicated hosting (bind addr + seed args).
- lf_client net.rs: connect from the title screen, state send, remote
  block edits applied with remesh, remote players as cubes, chat overlay
  with T input; local edits replicate to the server.
- Tests 92 -> 96.

## 2026-08-25 — P8: quests & chronicle live (loop 290)
- lf_story: QuestEvent (Collected/Crafted/Killed/ReachedDepth) advancing
  objectives with progress counters; starter_quests() 5-quest chain (+1 test).
- Client: events wired to pickups, crafting, mob kills; quest log UI on J
  showing objectives/progress/chronicle; chronicle milestones (first logs,
  first blood, Null Knight slain, deaths, quest completions) exported to
  worlds/<name>/chronicle.md on save; state persists in the save.
- Tests 91 -> 92; game smoke-tested.

## 2026-08-25 — P7: structures, menus, UI proofs (loop 289)
- worldgen structures: meadow huts, highlands watchtowers, desert pyramids
  (deterministic per-chunk placement, in-chunk footprints, +1 test).
- Client: title screen and pause menu; Esc opens pause; settings sliders
  (mouse sensitivity, FOV); quit saves the world.
- lf_engine headless: optional egui overlay in render_to_png; vistest
  hud_preview scene draws the real HUD (hearts/hunger/hotbar/crosshair/
  inventory) — honest UI proof shots at last.
- Tests 90 -> 91; game smoke-tested from the title screen.

## 2026-08-25 — P6: mobs & combat (loop 288)
- lf_game mobs rewritten as a live framework: MobEntity with AI/physics
  update (wander, flee-on-hit, chase, melee with cooldown), MobType stats
  table + drops, roll_spawn day/night table (+5 tests).
- Client: mob spawn cycle (every 2s, cap 12, surface-only, despawn 80),
  crosshair mob attack (tool damage table, knockback, hurt flash), mob
  cubes render with the drop batch, mobs/kills persist in the save.
- Items: porkchop/mutton food, glitch_dust/null_shard materials.
- Tests 87 -> 90; game smoke-tested with mobs active.

## 2026-08-25 — P5: content catalog core (loop 287)
- Blocks: furnace, chest, planks, glass (transparent); matching textures.
- lf_game: smelting module (Furnace state machine with fuel/burn/progress,
  +4 tests), iron tier + swords in the item table, tool_damage table,
  recipes for furnace/chest/iron tools/swords.
- Client: block entities (furnace/chest) with persistence, RMB opens their
  screens, furnaces tick while closed, containers spill contents when
  broken, furnace UI (input/fuel/output + flame + progress), chest UI.
- Catalog consistency test: recipe outputs/ingredients, smelt outputs,
  block drops, and block items all resolve.
- Tests 82 -> 87; game smoke-tested with containers active.

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
