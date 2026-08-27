# BACKLOG — LOREFORGE

Honest status after the P0 audit (see CHANGELOG) and the build-pack Step 1
reality audit (see AUDIT.md, 2026-08-26 — every checked item below was
re-verified against code + a live session; broken claims were fixed or
re-opened). Everything checked below exists in code and is covered by tests
and/or real rendered proofs (`shots/vistest_*.png`).
The former "Evolution Mode" list (~230 items claimed in loops 26–281) contained no
implementations; the genuinely built items are checked here, the rest are planned
below by phase. Its fossil `shots/ev_*.png` "proofs" were removed by the audit.

## Done (verified)

- [x] M1 window opens and clear color (lf_engine)
- [x] M2 chunk data structure + culled meshing (lf_voxel) + texture array
- [x] M3 voxel raycast (DDA) with tests (not yet wired to input — P1)
- [x] M4 terrain noise heightmap + biomes + strata — 30 biomes today
      (the "all 8 biomes" wording was the M4-era count, stale until the
      audit; visual distinctness of the 30 is build-pack Steps 16–19)
- [x] M5 world persistence (region files hold many chunks, atomic writes, P0)
- [x] M6 day/night cycle math + light level constants (propagation is P3)
- [x] M7 survival data types (stats, inventory stacking)
- [x] M8 smithing data model (8 materials, tool assembly, forge minigame;
      audit fixed the forge UI minting one steel ingot per frame — strike
      is now a click and the forge resets after granting)
- [x] M9 mob data model (6 types incl. Null Knight boss)
- [x] M10 mod manifest + block/item data loading (ember_ores, amberium examples)
- [x] M11 protocol codec + UDP echo server binary
- [x] M12 villager schedules + trading (audit split: the Geode Guardian /
      Cinder Crawler mobs in lf_npc are dead data, never spawned — open
      item in AUDIT.md, spawn-or-cut)
- [x] M13 quest data types (objectives, quest log)
- [x] M14 chronicle events + saga/markdown generation
- [x] Depth-buffered renderer with shared GpuScene (P0)
- [x] Real offscreen headless renderer + scene harness (P0)
- [x] xtask vistest/screenshot commands producing real PNGs (P0)
- [x] real item icons + tooltips + recipe book + minimap/world map/waypoints (P22)

## P1 — First-person core
- [x] keyboard/mouse input (WASD, jump, sneak, sprint, mouse look, cursor lock;
      audit fix: sneak was captured but never read — now a 0.45x careful walk)
- [x] player AABB physics (gravity, collision, jump, fly; substepped anti-tunneling; 8 tests)
- [x] camera control (first-person from eye position; crosshair lands with P4 HUD)
- [x] block targeting outline via DDA raycast; break/place with player-overlap check
- [x] hotbar (1–6, scroll) with block placement; F2 in-game screenshots

## P2 — World streaming & terrain
- [x] chunk streaming: background generator thread, nearest-first, wish
      radius follows the view-distance setting (audit fix: was hard-wired
      to 5 so High preset never streamed farther)
- [x] worldgen features: trees (canopy in-chunk), caves (3D noise), coal/iron by depth, water at sea level
- [x] sphere-frustum + distance column culling using mesh bounds
- [x] save/load world (region chunks + player state) with autosave and save-on-exit
- [x] block registry: solid/transparent/targetable; water non-solid, raycast skips it

## P3 — Lighting & atmosphere
- [x] flood-fill sky + block light per column (BFS, opacity-aware; tests for falloff/overhangs/emitters)
- [x] torches/lanterns emit real light (14/15) and are placeable (audit
      fix: the lantern block existed with light 15 but had no item or
      recipe — unplaceable; now craftable iron-over-torch)
- [x] day/night cycle drives sky color, clear color, sky-light factor + distance fog
- [x] water transparency (alpha-blended pass, back-to-front column sort; underwater tint in P7)
- [x] smooth per-vertex lighting/AO (done by P26's visual identity pass;
      this line had been left unchecked and stale until the audit)
- [x] sun/moon/stars/clouds + weather-driven sky/fog (client atmosphere pass)

## P4 — Survival & inventory UI
- [x] egui HUD (crosshair, hearts, hunger, air bubbles, 9-slot hotbar, mining progress, clock)
- [x] inventory screen: click pick/place/swap/merge, right-click split; shift-click quick-move (P22)
- [x] crafting 2x2 in inventory + crafting table 3x3; shaped recipes with translation matching (+8 tests)
- [x] hold-to-mine with hardness, tool speed multipliers, harvest gating (iron needs stone pick), durability that breaks tools
- [x] item drop entities with gravity + magnet pickup + bobbing render
- [x] hunger drain, regen when fed, fall damage, drowning with air, death screen + respawn
- [x] eating (apple); inventory/stats/time saved with the world
- [ ] beds/spawn setting, crack overlay texture (P5/P7 polish)

## P5 — Content catalog
- [x] furnace with smelting state machine (raw iron->ingot, sand->glass; coal/log/planks fuel; ticks while closed) + UI
- [x] chest block entity (27 slots) + UI; contents spill on break
- [x] planks + glass blocks (glass renders in transparent pass)
- [x] iron tool tier (pick/axe/shovel) + wooden/stone/iron swords + all recipes
- [x] block entities persist with the world; catalog consistency test
- [ ] armor, beds/doors/signs, wool/decor variants (fold into later passes)
- [ ] smithing table integration (with P6 combat loot)

## P6 — Mobs & combat
- [x] mob framework + AI (wander/chase/flee, 1-block hops) + spawning rules
      (day/night table, cap 12; the light-level gating is still open)
- [ ] grid A* pathfinding (mobs hop and beeline today)
- [x] combat: cooldown, knockback, armor mitigation, bow/arrows
- [x] XP levels; Null Knight boss data (arena/phases still open)
- [x] villagers wander by schedule + trading UI

## P7 — Structures, weather, sound, menus
- [x] structures: meadow huts (torch/crafting table/furnace), highlands watchtowers, desert pyramids — deterministic (+1 test)
- [x] title screen (Play/Quit) and pause menu with sensitivity/FOV settings
- [x] UI proof screenshots: hud_preview scene renders the real egui HUD offscreen
- [x] weather particles (rain/snow by biome) — sound lands in P17
- [x] world types superflat/amplified with title-screen selection
- [ ] key rebinding, PT-BR (P19)

## P8 — Quests & chronicle live
- [x] quest events from gameplay (collect/craft/kill) advance objectives with progress (+1 test)
- [x] 5-quest starter chain (timber -> planks -> tools -> iron age -> night hunter)
- [x] quest log UI (J) with objectives, progress and the chronicle
- [x] chronicle records live milestones, exports worlds/<name>/chronicle.md on save
      (audit note: 5 of 11 event types have no producer — GreatTrade,
      Discovery, StructureCompleted, VillageFounded, RuneApplied; and q4's
      "collect iron" can't fire from furnace output, only ground pickup)
- [x] quest/chronicle state persists with the world
- [ ] lore books readable in-game (deferred)

## P9 — Multiplayer
- [x] protocol v3: join/leave, 20/s position snapshots, validated block ops, chat (+4 codec tests)
- [x] authoritative-lite lf_server: canonical world + edit history, newcomer replay, chat relay
- [x] two-client local integration test over real UDP (chat + block sync + positions)
- [x] dedicated loreforge-server binary (bind + seed args)
- [x] client join from title screen, remote players rendered, remote edits applied, chat UI (T)
      (audit note: Welcome.seed is ignored — clients generate local-seed
      terrain, only edited blocks sync; connect is hardcoded localhost +
      name "smith"; see P28 + build-pack Steps 34–35)
- [ ] singleplayer routed through integrated server; mob/world sync; server browser (deferred)

## P10 — Mod API real
- [x] runtime registries: mod blocks (stable ids, solidity/opacity/drops), items, recipes, smelting
- [x] smelting.toml parsing fixed; worldgen ore hooks (auto *_ore veins)
- [x] client loads mods/ at boot — smoke log confirms ember_ores + amberium
- [x] mods/README.md; full-pipeline test (parse->register->place->break->smelt)
- [ ] custom mod textures (generic mod layer for now)

## P11 — Performance & release
- [x] light cache with edit invalidation (+test) — and it caught two real
      lighting bugs (unsampled light closure; section-local y offset)
- [x] cargo xtask package: portable dist/ zip (binaries + mods + docs)
- [x] CI release matrix (ubuntu/macOS/windows) uploading artifacts
- [x] honest RELEASE.md with run instructions, controls, features, gaps
- [ ] puffin profiling pass, greedy meshing (deferred; frame times fine at view 5)

## P25 — Correctness & honesty sweep
- [x] server SetBlock validates against the real registry (mod blocks >= 100
      accepted; the old `block <= 18` cap silently dropped every mod edit);
      dedicated server loads mods/ at boot like the client
- [x] `lf_steam/steam` feature actually compiles (steamworks 0.12 optional dep);
      STEAM.md corrected (CI still builds default-feature binaries only)
- [x] generator version stamped into saves (`genver.dat` per world; mismatch
      warns — revisited unedited chunks regenerate, edited chunks are safe)
- [x] lantern block got a real texture layer (was falling through to stone)
- [x] root Cargo.toml dependency table pruned to what is actually used (the
      old table listed 14 deps nothing referenced, with drifted versions)
- [x] mods/README `_ore` auto-registration claim is now real code in
      lf_modapi::apply_mod; misleading `tests/golden` stub removed
- [x] vistest PNGs are pixel-analyzed after rendering (non-uniform, multi-color
      check in lf_vistest::verify_render) — "it rendered" is enforced by code
- [x] the pixel gate immediately caught two real pathtracer bugs that had made
      every raytraced scene (and in-game Live RT) render one flat color since
      P18: the WGSL DDA initialized t_max with a signed numerator (negative
      for negative ray components), and the camera basis was scaled by a
      double `to_radians()` on the already-radians fovy (basis at ~1.4%,
      all rays parallel). Both fixed; RT proofs show real terrain again.

## P26 — Visual identity (per approved direction: hybrid-selective)
- [x] per-face materials: `tex_of(BlockState, Face)` — grass top/side/bottom
      correct on all six faces, log rings on every species' ends (atlas 48
      layers: +grass_top, +log_top, +crack_0..3)
- [x] alpha-cutout foliage: shader discards alpha < 0.5; all six leaf
      textures hole-punched per species (18-29% deterministic holes);
      see-through leaves with reliable depth writes (water/ice unaffected,
      glass pane becomes frame-only cutout)
- [x] foliage wind: GpuVertex sway weight + Env.time; vertex-shader wave
      phased by world position (stable across chunk borders); frozen when
      the particles setting is off (low quality tier)
- [x] smooth lighting: per-vertex AO (classic side/side/corner) + per-corner
      light averaging over the 4 touching cells (was flat per-face)
- [x] mining feedback on the target: stage 0..3 crack decal (inflated cutout
      cube) + debris particles (billboards, gravity, ground stop, cap 128);
      the subtle HUD progress bar stays for accessibility
- [x] mipmaps: 5-level CPU box-filtered chain per layer, mag-nearest /
      min-linear sampling (distance shimmer gone)
- [x] two new proofs: `foliage_canopy`, `mining_feedback` (22 scenes total)
- [ ] connected-surface projection on large man-made materials
      (stone/marble/planks) — the hybrid-selective art direction's second half

## P27 — Camera culling fix
- [x] objects-no-longer-disappear-when-looking-up: column frustum culling
      now uses the exact AABB bounding sphere (sqrt(128 + half_h^2) + sway
      margin) instead of an under-sized axis-only sphere, and the frustum
      planes are normalized (the near plane's raw normal is ~2x unit);
      regression test proves corner-inside => kept across pitches 5-85 deg
      with a pinned pre-fix failure case

## Deferred (P23 notes, honest)
- [ ] console `new`/`load` adopt connected-server seed in multiplayer
  (Welcome.seed now carries it; client terrain is still local-only).
- [ ] second-level autocomplete (command arguments) — first token only.
- [ ] save-slot thumbnails in the picker (renders a map preview per slot).
- [ ] Gamepad/mobile input; key remapping UI.

## Deferred (P22 notes, honest)
- [ ] migrating GameState::menu_reveal (f32 clock) onto ui_kit::Reveal — the
  clock + per-open reset already provides the same behavior; pure churn, kept
  for a rainy day. Reveal stays tested in ui_kit.
- [ ] minimap rotation / zoom controls; in-game waypoint beacon rendering
  (map + minimap pips exist, world-space beams do not).
- [ ] Windows exe runtime: host lacks mingw-w64 (macOS runner); macOS dmg +
  Linux tarball ship instead.

## P28 — V1REBRAND rendering & UX gate (docs/V1REBRAND/02, +2 numbering per DECISIONS)
- [x] execution plan written: docs/V1REBRAND/11-EXECUTION-PLAN.md (P28-P39
      map onto roadmap docs 02-10 with the +2 phase offset)
- [x] wind-sway honesty fix: P26 claimed foliage wind, but shader.wgsl
      vs_main never read the sway attribute (loc 6) or time_sway — leaves
      could not move. Shader now offsets sway-weighted vertices with a
      world-position-phased double sine (max ~0.08 blocks, inside the 0.1
      cull margin). Proof: foliage_sway_animates_between_frames renders the
      canopy at two wind phases through the real GPU pipeline and demands
      pixels differ (same-phase control must be pixel-identical).
- [x] clouds setting un-no-op'd: the toggle now actually clears/rebuilds
      the cloud batch (was rebuilt unconditionally)
- [x] unload radius tracks view distance: UNLOAD_RADIUS=8 const replaced by
      view_distance + UNLOAD_MARGIN(3); column_in_view takes the view
      distance (view 8 previously had zero unload headroom)
- [x] first-launch fix: Settings opened from the title screen now returns
      to the title (Back and Esc via close_settings) instead of dropping
      the player into the world
- [x] boot loads the booted slot's player extras (was: legacy worlds/default
      read before boot_slot(), so slotted players booted with default
      inventory/settings until Play was clicked)
- [ ] chunk-border lighting: cross-column flood (3x3 neighborhood) replacing
      "seams accepted"; night seam vistest + regression test
- [ ] transparency/sort audit documented in DECISIONS (water sort + particle
      rules ahead of Steam-age smoke/steam)
- [ ] frame-time target: xtask perf (p50/p95 ms at Medium) + DECISIONS entry
      (host iGPU is the "low" device)
- [ ] quality tiers: PathTraced preset (Low/Medium/High/Path-Traced)
- [ ] key rebinding (input.rs Action/Keymap + Controls tab + persistence)
- [ ] save-slot thumbnails
- [ ] minimap rotation/zoom + waypoint beacons
- [ ] UI language audit (on-kit quest log/book/console/trade/tech tree,
      HUD text shadows, Theme::MANA)
- [ ] connected-surface textures (stone/marble/planks; carried from P26)

## Build-pack Stage A — reality audit (docs/poorcraft-build-pack, Step 1-2)
- [x] Step 1: AUDIT.md written at repo root — every prior [x] claim
      re-verified (code + live session + captures); BACKLOG corrected in
      the same commit; stale lines fixed (M4 "8 biomes" -> 30, P3
      smooth-AO now checked, M8/M12/P2/P9 caveats added)
- [x] Step 1 fixes shipped with the audit: HUD hidden behind the title
      menu; title orbit camera clamped above ring terrain (was buried on
      hilly worlds -> flat-dark backdrop, repro tool in
      lf_worldgen/examples); render culling uses the render camera's eye;
      streamer wish radius follows view distance; sneak wired to a slow
      walk; smithing strike-per-click + grant-once reset; lantern
      craftable; random_seed() sequence counter; 201 fossil ev_*.png
      "proofs" removed (zero code references — Evolution-era residue)
- [ ] Step 2 remainder: audio engine + break/place sounds (pack Step 4);
      biome visual identity (pack Steps 16-19); spawn-or-cut Geode
      Guardian / Cinder Crawler; q4 Collected from furnace output/trade;
      multiplayer Welcome.seed + address entry; chronicle dead event
      types; dawn/dusk light ramp; F2 re-render includes water/crack

## Fluids & block gravity (user request; P30 Steam-Age fluid groundwork landed early)
- [x] granular blocks fall: registry::has_gravity (sand/red_sand/snow/dirt/
      grass/moss/mycelium — ores deliberately excluded, MC rule); breaking
      support detaches the column into animated FallingBlock entities that
      land through the player edit path (remesh + MP broadcast); settle_
      gravity is the headless twin (tests + vistest)
- [x] water physics: level 0..7 flow states in BlockState flags; event-driven
      cellular sim (fall first, spread with decay, dry up when unsupported);
      flowing surfaces render lowered (stepped water) with step-covering
      side faces; 64-cell tick budget; oceans/lakes are sources
- [x] bucket + water_bucket (craftable from 3 iron; scoop a source /
      pour a source) — first player tool for the fluid system
- [x] vistest proofs: water_flow (aqueduct -> flume -> dam pooling,
      settled through the real sim before meshing) and falling_sand
      (collapsed pile + mid-air faller), 24 scenes total

## Goal-file Sections 0–4 (2026-08-26, loop 311)
- [x] S0 re-audit of the four flagged items (AUDIT.md updated): bottom
      mining bar CONFIRMED and removed; texture stretching NOT
      reproducible in the raster path (per-block quads tile by
      construction — mesh test + visual proof added, greedy-mesh
      precondition recorded); biome grade absence CONFIRMED and fixed;
      mod-load visibility CONFIRMED and fixed
- [x] S2: mining/bow progress = crosshair-centered radial ring
      (ui_kit::paint_mining_reticle + geometry test; hud_preview proof
      shows it mid-break; bottom-of-screen bars fully removed)
- [x] S3: per-biome color grade (shader grade uniform; warm/cool/lush/
      eerie/teal/neutral table; ~0.3s boundary lerp; clear-color mirror;
      GPU hue/sat proof test biome_grade_shifts_midframe_color)
- [x] S4: mods/smoke_test (1 block + 1 item) + [MOD SMOKE TEST] OK boot
      line (client + dedicated server) + CI test on the real folder +
      README pointer
- [x] S1: per-block texture tiling proven (mesh test
      multi_block_walls_tile_per_block_not_stretched + texture_tiling
      scene, AI-verified per-block repetition on a 7-wide wall and floor)
- [x] S5 spot: Live RT decision + greedy-mesh UV precondition in DECISIONS
- [x] STATUS.md rewritten to match verified reality (was stale: 121 tests/
      14 scenes/live-RT-deferred)

## Loop 312 — build-pack Step 3 remainder, Step 4 (audio), Step 7, Step 8, Step 9
- [x] Step 4 AUDIO: new lf_audio crate — procedural PCM one-shots (no
      asset files) per material category (wood/stone/metal/glass/soft),
      break + place variants, rodio playback with silent fallback when no
      output device, 30ms rate limit, driven by the persisted volume
      sliders; client plays on every real break/place; category dispatch
      unit-tested (block_categories_dispatch_correctly etc., 4 tests);
      CI ubuntu jobs install libasound2-dev
- [x] Step 3 remainder: impact pulse — heavy blocks with heavy tools kick
      a short decaying camera shake (break_shake/shake_decay/shake_offset,
      envelope unit-tested; jitters the look target only)
- [x] Step 7: FOV-to-projection verified against hand-computed reference
      values at fov 90 and 60 (projection_matches_reference_values_at_two_
      fovs in lf_engine) — the double-to_radians class is guarded on the
      raster path now
- [x] Step 8: transparency_layers proof scene — water pool behind a glass
      wall with particles on both sides; AI-verified layering (water
      through glass, near particles over the pane, far ones through it)
- [x] Step 9: persistent HeadlessRenderer refactor (device+atlas once —
      the first perf run measured setup, not frames), xtask perf + make
      perf, Medium(radius-5) numbers recorded in DECISIONS (target device
      = this host's iGPU); live >=30fps confirmation pending next play
      session's F3 reading

## Loop 313 — build-pack Steps 13, 14, 15 (settings completeness + picker + minimap)
- [x] Step 13 KEY REBINDING: new lf_client::input (Action x Keymap,
      defaults = the original hardcoded keys); window_event + movement read
      the keymap; Settings > Controls tab with click-then-press capture
      rows; persisted via Settings.keymap_pairs in ClientSave (serde
      default; junk input falls back to defaults). Digits 1-9 + Escape
      stay fixed by design. Persistence test:
      rebind_and_quality_tier_persist_through_client_save
- [x] Step 13 PATH-TRACED TIER: Quality::PathTraced added — Medium raster
      base + RtMode::Live; Low/Medium/High now explicitly set rt Off;
      active tier stored in Settings.quality and shown in the preset row
- [x] Step 14 THUMBNAILS: save_world captures a 256x144 live-view PNG to
      worlds/<slot>/thumb.png (throttled ~2 min so the 30s autosave stays
      cheap); the slot picker lazily loads and shows it beside
      name/type/seed/last-played
- [x] Step 14 FIRST-LAUNCH WALKTHROUGH: documented in DEVLOG (title ->
      Play/New/Load/Multiplayer/Settings, no dead ends; Settings-from-
      title returns to title — fixed in the loop-309 audit; legacy
      worlds/default pre-load removed loop 308)
- [x] Step 15 MINIMAP: rotate-with-view (custom rotated mesh + shared
      marker rotation + N chip rides the rim) + zoom 0.5-3x (Interface
      settings), both persisted
- [x] Step 15 BEACONS: per-color translucent atlas layers
      (waypoint_0..5) + world-space beams rebuilt per frame from the
      player's waypoints and drawn in the transparent pass; proof scene
      waypoint_beacons (AI-verified: three colored translucent beams,
      no artifacts)

## Loop 314 — build-pack Steps 16-19 (biome identity, spawns, weather)
- [x] Step 16 BIOME SURFACES: JUNGLE_GRASS (deep saturated) + SAVANNA_GRASS
      (dry gold) blocks (ids 42/43) with atlas layers; Jungle/Savanna/
      WindsweptSavanna now wear them; Swamp keeps MOSS; MushroomHollow now
      generates on previously-unused MYCELIUM; wildflower cutout plant
      (FLOWER, id 44) sparsely covers FlowerForest — breaking its twin
      with Forest. GENERATOR_VERSION -> 2 (unedited chunks regenerate).
- [x] Step 16 CONTACT SHEET: biome_contact_sheet scene — 30 strips paved
      with each biome's REAL surface+filler from the table; pixel check
      measures exactly 30 distinct quantized strip colors; AI-verified all
      palette families present with no identical groups
- [x] Step 17 EXCLUSIVITY: Tundra gets SpruceSparse conifers (vs dense
      SnowyTaiga); boulder fields are SnowySlope/WindsweptHills/
      WindsweptSavanna's exclusive feature; regression test
      biome_identity_markers_are_distinct enforces pairwise uniqueness
      (surface+filler+tree+structure+exclusive) with only two documented
      families exempt (depth-banded oceans incl. FrozenOcean; coastal
      StonyShore/Mountains) — the test caught two real twins during
      development and both were fixed
- [x] Step 18 SPAWNS: roll_spawn is biome-aware (cold biomes: woolbeasts
      only; temperate: boars + rare woolbeasts); night hostiles global;
      tested (day_spawns_are_biome_appropriate). Structures were already
      biome-gated (loop-309 audit)
- [x] Step 19 WEATHER: cold = the actual biome field (Biome::is_cold via
      the map's generator), not the old surface-block proxy; proofs:
      weather_snow (flakes over a snow field) + weather_dry (clear desert)
      join clouds_weather (rain)

## Loop 315 — P29 Water Age (V1REBRAND doc 04 / build-pack Step 23)
- [x] research prerequisite GRAPH: Era::Water branch (prereq Industrial,
      independent of Electrical — doc 03's either-order rule);
      ResearchState.branches (serde-default, pre-branch saves load);
      can_unlock/unlocked/unlock() with material costs (16 planks + 24
      stone + 4 iron — cheap/early per doc 04); tech-tree screen gains the
      branch card with a live Unlock button; 5 new tests incl. save-compat
- [x] machines: WaterWheel (12 EU/s while water touches it, river-gated,
      free — lowest tier below the coal generator's 20) + BatteryCell
      (4000 EU) + a PURE lf_game::machines::distribute_power (producers
      first, batteries cover gaps = blackout prevention, surplus recharges
      batteries in the 4-block field) — 3 tests
- [x] blocks: WATER_WHEEL (45) + BATTERY (46) through the full content
      pipeline (registry, atlas layers + procedural textures, items,
      Water-era-gated recipes, drops); machine UI panels (spin-up + charge
      bars); client power tick now runs every source through the pure step
- [x] RT palette: ids 42-46 hand-set + a stable hash fallback for all
      future ids so new blocks are never invisible/wrong in path tracing
- [x] proof: water_wheel_power scene (river carved, wheel + battery +
      crusher, the real power step spins the wheel for 30 sim-seconds) —
      AI-verified as a riverside power station

## Loop 316 — P30 Steam Age (V1REBRAND doc 04 / build-pack Step 24)
- [x] Era::Steam branch (prereq Industrial, either-order vs Water —
      tested; 12 iron + 4 gears + 16 coal); tech-tree branch cards now
      list Water AND Steam with live unlock buttons
- [x] machines: Pipe (1000 mB, equal-share between neighbors — no
      pressure sim per DECISIONS), Boiler (fuel via the existing
      fuel_seconds table + water -> steam, idle dissipates), SteamEngine
      (16 EU/s at full steam — wheel 12 < engine 16 < coal 20, asserted);
      4 tests incl. the full boiler->engine->machine chain through
      distribute_power
- [x] blocks PIPE(47)/BOILER(48)/STEAM_ENGINE(49) through the full
      pipeline (atlas layers, procedural textures — flywheel underflow
      caught by the atlas test and fixed, items, Steam-gated recipes,
      drops); client steam pass (pipe equalization, boiler feeds from
      adjacent sources like a pump + pipes, engines drink adjacent
      boilers); steam puffs rise from burning boilers (particles-gated);
      UI panels (pipe water level, boiler fire+steam+fuel, engine output)
- [x] proof: steam_chain scene — water -> pipes -> fueled boiler (pre-run
      through the real machine code) -> engine -> crusher with live puffs;
      AI-verified as a working boiler room (caught + fixed an infinite
      feed loop in the scene itself during rendering)

## Loop 317 — P28 chunk-border lighting + Step 20 lore books
- [x] P28/Step 6 CROSS-COLUMN LIGHTING: compute_column_light now floods
      sky+block light through a 3x3-column volume (48x256x48) and extracts
      the center slice — the P3 "seams accepted" decision is superseded
      (DECISIONS updated); edit invalidation widened to the full 3x3 column
      neighborhood (light travels 15 blocks). Regression:
      torch_light_crosses_chunk_borders (neighbor column gets 13 at the
      border). Proof: night_border_seam scene — torch straddling a border
      at night, measured max adjacent-column brightness step 1.92 (a seam
      would cliff >8). perf bench after the change: p50 47.7ms incl.
      readback+PNG at Medium radius-5 — no regression.
- [x] Step 20 LORE BOOKS: lore/books.toml with three real tomes (Tome of
      the First Forge / Tome of the Null / The River Wardens' Ledger —
      anchored to the existing Smith + Null + river lore threads);
      lf_client::lore loads them at boot; right-click a tome to page
      through an on-kit reader (prev/next, page x of y); tomes are
      Lorekeeper trades; pixel-art icons with a cover gem. Test:
      lore_books_load_from_the_real_file (3 tomes, pages >40 chars,
      item mapping, lore anchors present). Proof: lore_book scene reading
      the REAL file — AI-verified finished reader with readable story
      text.

## Loop 318 — P31 Oil Age + Step 25 power-grid overlay
- [x] OIL worldgen (id 50): biome-gated crude pools replace deep stone
      (y 8..44, desert/swamp only) + rare surface seeps (1/700 columns);
      regression test `oil_is_biome_gated_and_banded` scans 144 chunks
      and asserts every crude block sits in a desert/swamp column.
      Existing worlds keep their generated chunks (no GEN_VERSION marker
      exists in this codebase; oil appears in newly generated terrain —
      same policy as every prior ore addition).
- [x] Pipes v2: fluid typing (FluidKind Water/Crude on separate channels,
      serde-defaulted so existing pipe entities load unchanged); channels
      never mix (test). Boiler drinks Water; pump/refinery move Crude.
- [x] Crude oil fluid sim: step_cell is now fluid-generic — oil creeps
      only OIL_SPREAD=3 cells (sluggish vs water's 7), fall-first, and
      water/oil never convert each other (test). Sand sinks through oil.
- [x] PUMPJACK (51) / REFINERY (52) / COMBUSTION_GENERATOR (53): pump
      lifts 120 mB/s while powered + adjacent to an oil source, feeds
      neighbor pipes; refinery = crude 240 mB + power -> refined fuel +
      tar per 6s batch (exact mass-balance test); combustion burns
      refined_fuel only (45s/unit) at 26 EU/s — steam 16 < coal 20 <
      combustion 26 < nuclear (P32). Full-chain headless test through the
      real distribute_power.
- [x] Research: Era::Oil branch (bincode-appended, saves safe) gated
      Industrial AND (Steam OR Electrical) — meets_prereqs handles the
      either-or edge; cost = 4 refined fuel + 8 iron + 1 frame (you must
      have RUN the chain to earn it); pump/refinery gate at Industrial
      (extraction is iron-age kit), combustion generator at Oil. Tech
      tree shows the third branch card with the either-or hint.
- [x] Items/crafting: pump/refinery/combustion recipes; oil_bucket
      (scoop crude sources or pour; right-click a refinery to feed its
      tank +1000 mB), refined_fuel, tar (P34 construction fodder).
      Bucket arm handles all three buckets.
- [x] Client wiring: BlockEntity Pump/Refinery/Combustion; oil pass
      (refineries drink from adjacent pipes pre-power, pumpjacks lift
      post-power into adjacent pipes); machine UI panels (refinery with
      fuel/tar slots + pour button); spill-on-break for the new
      containers; keymap rebinding-aware.
- [x] Step 25 POWER-GRID OVERLAY: G toggles translucent tint cubes over
      every machine in the power field — green = granted >= 90% of draw,
      red = starved (same ratio rule as the client overlay). Rebuilt
      every 15 frames while on; rides the transparent pass with the
      waypoint beams. Rebindable Action::GridOverlay.
- [x] Proofs: oil_chain (pool -> pumpjack -> pipes -> refinery ->
      combustion -> powered furnace, 200 sim-seconds pre-run through the
      real machine code; AI-verified coherent chain with flare smoke)
      and grid_overlay (green cube on the powered furnace, red on a
      starved crusher out of range; AI-verified both cubes). The scene
      needed an honest bootstrap: one combustion generator (26 EU/s)
      cannot feed three 10 EU/s consumers — a coal generator runs the
      pumpjack while the oil chain spins up, exactly the balance the
      overlay exists to show (DECISIONS entry).
- [ ] Deferred: derrick silhouette reads small at distance (AI feedback)
      — texture polish for a later visual pass; GEN_VERSION-style save
      re-generation marker still doesn't exist in this codebase.

## Loop 319 — P32 Nuclear (the ceiling)
- [x] URANIUM_ORE (id 54) via the deep band: y 8..24, threshold 0.68
      (rare, tiny) in the standard ore pass; drops raw_uranium; smelts to
      uranium_ingot; assembler makes fuel_rod (2 ingots + 1 iron).
- [x] REACTOR (55): 32 EU/s — the top of the ladder (wheel 12 < steam 16
      < coal 20 < combustion 26 < reactor 32; tier test). Heat/output
      curve: fission +4 heat/s, full coolant -5/s (a cooled core holds
      equilibrium — caught by test, constant fixed from 3), passive -0.5/s.
      Auto-SCRAM at 80, unscram below 60, MELTDOWN at 100. Residual decay
      heat +0.8/s while scrammed WITH rods loaded: a scrammed core without
      coolant still melts (test proves the sequence scram -> meltdown).
      Coolant: 60 mB/s from adjacent pipes (water channel) or water blocks.
- [x] Meltdown in the world: apply_meltdown destroys the r=3 sphere,
      crusts up to 14 RADIATION (56) residue blocks through the crater
      (they glow — emission 7 — and damage anyone within ~3 blocks until
      scrubbed), blast debris + camera shake + a chronicle Meltdown event
      (new EventType, bincode-appended).
- [x] Research: Era::Nuclear branch (Nuclear-era gated reactor + fuel
      rod) requiring Oil AND the new reactor_safety certification
      (glass 8 + basic_circuit 2 + book 1, studied in the tech tree).
      reactor_safety is serde-defaulted — old saves load uncertified.
      DECISIONS: nuclear is the ceiling (Pillar 5) — nothing above it.
- [x] Client: nuclear pass (coolant from adjacent water/pipes, tick,
      meltdown applied live, venting steam particles while scrammed),
      reactor UI (heat/coolant/output bars, fuel-rod slot, SCRAM +
      restart buttons with the honest warning), radiation damage in
      survival_tick.
- [x] Proofs: reactor_control (uranium vein in a cut wall, water+pipe
      cooling line, reactor run to thermal equilibrium through the real
      tick code — asserted heat<30/buffer>1000 — furnace+crusher
      consumers; AI-verified core window + green-flecked vein) and
      meltdown_aftermath (crater + ~dozen glowing residue blocks +
      wrecked machines at dusk; AI-verified). Framing lesson recorded:
      scene builders place at world.surface_height but the framing
      convention uses gen.surface_top — new scenes must mirror the
      steam_chain pattern verbatim.
- [ ] Deferred: radiation suit/scrubbing tools; reactor neighbor
      destruction of other machines' block entities (entities in the
      crater are dropped today via generic spill-on-break only for the
      reactor itself).

## Loop 320 — P33 Magic foundation
- [x] MANA: lf_game::magic (MAX_MANA 30, regen 1.5/s) + PlayerStats.mana
      (persisted via ClientSave.mana). HUD bar in Theme::MANA violet under
      the XP bar — appears only once a spell is learned (magic is found,
      not innate). Tests: pool/cost gating, bounded set stability.
- [x] THE BOUNDED FOUR (doc 05): Firebolt (8 mana — a harder-hitting
      arrow with impact sparks), Gale-step (12 — gaze-ray blink up to 8
      blocks, wall-safe), Ward (20 — 5s of full damage absorption while
      the timer runs), Hearthlight (15 — softens ONE raw ore by hand via
      smelting::smelt_result and lights the targeted cell with a
      temporary lumen that burns out after 90s). 3 cast slots on
      Z/X/C (rebinding-aware, Action::Spell1..3), spellbook screen on B
      (on-kit slide panel: mana bar, slot cards, learned list,
      assign/clear).
- [x] SAVE MIGRATION (latent bug found + fixed): bincode EOFs on old
      bytes when fields are added — every past ClientSave field addition
      silently reset old worlds' extras on load. Extras are now JSON
      (serde defaults apply), with a frozen LegacyClientSave bincode
      shape migrating pre-magic worlds. Tests prove the legacy bytes
      fail the current struct (the legacy path is load-bearing), that
      older JSON tolerates missing new fields, and that the spellbook +
      runed tools persist.
- [x] WIZARD: VillagerJob::Wizard (Ysolde, max 2 per world) settles
      towers (spawn marker = the enchanting table), sells all four
      scrolls + reagents. Wizard towers worldgen: 5x5 stone shell, 9
      tall, spiral stair, torch-lit enchanting table on top —
      FlowerForest (1/53) / Highlands (1/97); unit test scans 400 chunks
      (seed 42 -> 3 towers, biome-gated). Scrolls: right-click to learn
      (chronicle Discovery; auto-assign to a free slot).
- [x] ENCHANTING: ENCHANTING_TABLE (57, craftable + towers) opens the
      imbue minigame (ImbueMinigame mirrors ForgeMinigame: channel
      55..75 band, 3 pulses bind, reset guards per-frame minting).
      Runes (rune_of_haste x1.3 mining while held, rune_of_warding +2
      armor while held) bind to the HELD tool (runed_tools map,
      persisted; CustomTool.rune fill + RuneApplied chronicle event —
      the pre-cut hook is proven by test).
- [x] CROSSOVER ITEMS (doc 05): LUMEN_BLOCK (58 — fuelless light-15,
      crafted from glitch dust + glass + torch; also Hearthlight's
      temporary form) and WARDING_PYLON (59 — hostile mobs refuse to
      spawn within ~3 blocks; crafted around a null_shard core). Magic
      that plays along with the machines, not instead of them.
- [x] Proofs: wizard_tower (AI-verified tower + table + torches at
      dusk), spellbook (AI-verified finished screen: title, mana bar,
      three Z/X/C slot cards, four learned spells), spell_effects
      (AI-verified: lumen glow, enchanting table, orange firebolt arc,
      pale ward ring — pixel-checked; dusk dimming caught by scan
      thresholds, scene moved to 0.62 golden hour).
- [ ] Deferred to P33b+: more runes (the enum is the extension point),
      wizard quest hooks (trades teach today), spell targeting beyond
      the crosshair cell.
