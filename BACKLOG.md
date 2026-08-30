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

## ui-world-craft pack (loop 328)

- [x] LOREFORGE title identity: logotype + tagline, vignette, left link
      column, version/seed display; palette unified across ui_kit
- [x] version-seeded preview world (in-memory, never saved) + 90s
      elliptical orbit camera with altitude oscillation
- [x] New World screen (name/seed/reroll/type/mode/difficulty) with real
      difficulty gameplay (spawn gating, damage, hunger)
- [x] Load World picker: seed-rendered cached thumbnails, world-type
      glyphs, metadata, delete confirmation
- [x] Multiplayer screen: direct connect, host world, lobby stub
- [x] two-layer terrain: flat lowlands vs ridged highlands (measured
      0.41 flat fraction across seeds) + ocean shelf
- [x] rivers: meandering lowland channels that actually hold water
- [x] caves: breach ramp, deep slate < y30, lava lakes < y10,
      stalactites/stalagmites
- [x] structures terrain-adapted (support fill, slope platforms,
      underwater refusal) — huts through faction camps
- [x] per-biome ground cover with densities + transition interleaving
      (5 new plant/lava blocks)
- [x] crafting workbench: three zones, batch craft, Add to Queue, earned
      recipe visibility (always-visible / era / first-pickup) with toasts

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
- [x] crafting: the 3x3 grid was replaced by the ui-world-craft workbench
      (three zones, earned recipe visibility, batch craft) — the shaped
      matching engine stays in lf_game for mods/tests; the grid UI is gone
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
- [x] armor (loop 329): full bronze/steel kit — helmet/leggings/boots items,
      icons, Bronze-era recipes; the inventory's four armor slots are
      honored (worn_armor_points sums 36..=39) and drawn in the workbench
      strip with a live readout. Beds/doors/signs, wool/decor variants still open.
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
- [x] loop 329 menu pass: every panel centered (new world + multiplayer were
      top-left anchored), global kit theming for egui windows/widgets,
      Journal quest-log redesign, multiplayer screen developed, resize
      robustness pixel-proven at 640x420 / 800x600 / 1280x800
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

## Loop 321 — P34 Construction
- [x] SHAPE SYSTEM: Shape (Cube/SlabBottom/SlabTop/Stair x4) in
      BlockState's high flag nibble (bits 28..31 — fluid levels keep the
      low nibble; shape 0 = plain cube so every old save is untouched).
      mesh_section emits shaped geometry on its own path (1-2 boxes,
      exactly the exterior faces, no coincident interior quads, culled
      against opaque full cubes, AO/smoothed-light blended); the plain
      cube path is untouched. Physics: intersects_solid resolves the
      player AABB against registry::collision_boxes (slab = half plane,
      stair = slab + back box). Tests: meshing (half-box top, stair =
      11 exterior faces, winding outward, culling), physics (slab at
      half height, stair open/rise halves).
- [x] SHAPED PLACEMENT: stone/planks slabs + stone stairs items with
      shaped_placement() (stairs orient by yaw, tested across quadrants);
      placing a slab onto a matching bottom slab merges into a full cube
      (slab_merge, tested).
- [x] SCAFFOLDING (60): climbable (hold jump to rise, sneak to descend
      — physics hook), breaking one removes the connected column above
      and refunds every block.
- [x] SYMMETRY (V): a mirror plane at the player's x; place AND break
      mirror across it; the plane renders as a translucent wall (overlay
      batch). Rebindable Action::Symmetry.
- [x] BLUEPRINTS: two-corner capture (16^3 clamp) -> bincode file under
      worlds/<slot>/blueprints/; holding the blueprint shows the ghost
      (translucent cubes where it would paste, capped at 600); paste
      places into air cells only and consumes the exact per-block bill
      (drop-table-derived). lf_game::construction with capture/file/bill
      tests.
- [x] STATUE CARVING (61): chisel + CarveMinigame (detail 65..85 band,
      3 taps, per-frame-mint reset — mirroring forge/imbue); a completed
      carve turns the targeted stone into a Chiseled Statue; chronicle
      Discovery event.
- [x] DECORATION REGISTRY v2 / MODAPI LIGHT: BlockDef.light was parsed
      and dropped — it now flows through ModBlockDef.light into
      emission() (mod blocks emit their declared light). New
      mods/decor_pack example (glowing banner light 12, plinth, rug) +
      test loading the real folder and asserting the emission.
- [x] Proof: build_tools scene (slab staircase, oriented stairs,
      scaffold tower, statue, green ghost cubes) — AI-verified all four
      construction elements; pixel art for every new item (icons test).
- [ ] Deferred: slopes/arbitrary-corner shapes beyond stairs (the
      Shape enum is the extension point), decoration texture overrides
      (decor blocks currently use the mod texture slot), blueprint
      rotation on paste.

## Loop 322 — P35 Smart building
- [x] CONDUITS (62): power-field relays — distribute_power_relayed runs
      the same three-phase field but reachability hops through conduit
      chains (BFS, <=4 hops of POWER_RANGE each; unified field + relays
      per DECISIONS). Tests: 10 blocks bridged by two conduits, broken
      chains and the 4-hop cap rejected, the relaying distribute test.
- [x] ELEVATOR (63): powered-by-field vertical ride — jump on a platform
      launches physics-exactly to the next platform up (velocity from
      the height), sneak descends. next_elevator_y shaft tests.
- [x] CLIMATE UNIT (64): a unit with a producer in its range within 4
      blocks of the player regenerates health on a cadence (climate_
      comfort tests: unpowered does nothing, producer near the unit
      comforts, too far doesn't).
- [x] COMPUTER SCREEN (65): the dynamic texture path — SceneResources::
      write_atlas_layer rewrites one atlas layer (mips regenerated) at
      runtime; the screen block shows live data as a styled 16x16
      readout (page 1 research pips, page 2 chronicle rows, page 3 the
      green/red grid split) rewritten only when a data signature changes
      (hash-gated upload). Right-click cycles pages. compose_screen_face
      unit test reads the pixels back.
- [x] Proof: modern_wing ("one wing wired for electricity") — glass
      wall, slab mezzanine, the generator feeding upper machines ONLY
      through the conduit chain (asserted in-scene), elevator shaft,
      climate unit, computer; AI-verified all five elements.
- [ ] Deferred: screen text glyphs (the readout is styled pips/bars),
      elevator door animation, conduit visual connection stretching.

## Loop 323 — P36 Dragons
- [x] FLIGHT AI (lf_game::dragons): circle/swoop/perch state machine —
      Circling holds the 14-block ring at roost+8, close players (<20)
      provoke a SWOOP_TIME dive that re-provokes while the threat
      lingers and releases when they retreat; Perched breathes fire on a
      3s period at <7 blocks and launches back into the ring when
      cornered. 3 AI tests pin all of this.
- [x] MULTI-PART RENDERING: dragon_parts(t, yaw) — body, head (forward,
      bobbing), two wings (sine flap), three tail segments (sway) — one
      shared layout fn used by BOTH the client entity batch and the
      vistest proofs (the proof shows the real assembly). Parts test
      asserts flap amplitude, head-lead, tail-trail, yaw rotation.
- [x] FIRE BREATH: perched dragons damage the player in range with ember
      particles streaming from the mouth; breath gated by the AI test.
- [x] ROOSTS: stone crag + egg clutch (DRAGON_EGG 66, ember-cracked
      texture) in Mountains (1/89) / SnowyPeaks (1/101); 400-chunk
      gating test (seed 99 -> 2 clutches). try_settle_dragons: one
      dragon per roost (marker = egg), max two alive, Discovery
      chronicle on settling.
- [x] BOSS: MobType::Dragon (400 HP, 18 dmg, size 2.2 — above the Null
      Knight), drops dragon_scale + iron, BossSlain saga event on melee
      AND projectile kills ("the dragon of the peaks falls — the saga
      turns a page").
- [x] MOUNT (user-approved spike, DECISIONS entry): the flight x
      streaming audit held (ring 14 << view+3 margins; 8 part-cubes is
      mesh noise) — bare-hand right-click bonds the ride, the rider
      tracks the dragon each tick, sneak dismounts.
- [x] Proofs: dragon_roost (AI-verified: crag + cracked eggs + the full
      multi-part assembly readable as a dragon) and dragon_flight
      (pixel-verified: 8116 body-red px + 228 white-hot breath px on
      the shared-assembly mid-flap pose).
- [ ] Deferred: dragon roost loot chests, wing-tilt banking in the
      layout, breath setting blocks alight.

## Loop 324 — P37 Paths & specialization
- [x] PATHS (lf_game::paths): Engineer/Architect/Battlemage/Artisan
      standings on ClientSave (serde-defaulted, JSON extras) — no decay,
      no lock-in. Accrual events: MachineRan->Engineer (cadence-sampled
      in the power loop), BlockPlaced->Architect (every placement),
      SpellCast/BossSlain->Battlemage, ItemCrafted/ItemEnchanted->
      Artisan; tier crossings (25/step) write chronicle milestones.
      Tests: event->path mapping + weights, tier crossing, respec.
- [x] GATE GENERALIZATION + CRAFT/PLACE ENFORCEMENT: Gate::Era|Path|
      Open with passes() over research.unlocked (fixing the REAL
      branch-era bench bug — boilers/pipes were uncraftable because the
      grid compared against the MAINLINE era) + path standing. Enforced
      at the craft grid (locked veil + gate label) AND at placement
      (refuse + hint; it was UI-only before). Ornate tier: precision_
      gear / master_blueprint / battlestaff / master_chisel (recipes +
      icons) gated at 25 path standing.
- [x] RESPEC: pay 8 iron + 1 null_shard, standings reset, the focused
      path accrues double (tested). Paths screen on P: four cards with
      standing bars, tiers, focus buttons, respec note.
- [x] PROTOCOL v4 TRADING: PROTOCOL_VERSION 4; TradeOffer/Accept/Cancel
      client messages + TradeOffered/TradeResolved server messages
      (bincode round-trip test). Server escrow: offers registered +
      validated (target online), accept delivers items to BOTH sides,
      cancel/decline frees both. REAL-UDP test in lf_server (two
      sockets: offer->receive, accept->both deliveries, cancel->both
      freed). Client applies TradeResolved to the inventory and shows
      offers as hints.
- [x] Proofs: paths_screen (AI-verified: four cards, bars, tier text,
      violet focused Battlemage, respec note) and trade_p2p (the
      escrowed offer panel, rendered + verified).
- [ ] Deferred: client-side trade-offer SEND UI (receive/apply is wired;
  server + protocol + tests fully cover the trading deliverable),
  ornate-item gameplay effects beyond the tier-3 tools.

## Loop 325 — Finish line: Steps 34-39
- [x] STEPS 34-36 (lobbies/P2P/invites): lf_steam::lobbies — a
      transport-neutral lobby model (create/join/membership churn/leave)
      where UDP lobby codes ARE the host address, plus the full invite
      flow (mint/receive/accept/decline). 3 tests. The Steamworks-armed
      mapping stays behind the `steam` feature (off by default; UDP
      fallback unchanged), per the existing DECISIONS entry.
- [x] STEP 37 (Workshop UGC): lf_steam::workshop — WorkshopItem +
      scan_installed(): UGC folders with mod.toml load identically to
      bundled mods (Steam subscriptions land in the same shape); test
      scans a temp dir, ignores non-mods, tolerates a missing dir.
- [x] STEP 38 (mods/README rewrite): full authoring guide — quick-start
      scaffold, manifest reference, blocks (with the light field that
      now truly emits), items/smelting, decoration packs, UGC/Workshop
      install, multiplayer determinism, gate interactions, testing.
- [x] STEP 39 (xtask new-mod): `cargo run -p xtask -- new-mod <id>
      [--name]` scaffolds manifest + example block/item, refuses
      overwrites; `make new-mod id=... name=...` added. Verified live
      (scaffold + duplicate refusal) + the scaffold shape parses and
      registers through the real loader (lf_modapi test).
- [ ] Deferred: client title-screen lobby UI wiring (the model + UDP
      codes are done; Steam feature-on arms unverified without the SDK).

## Loop 326 — P28 leftovers + Step 40 (the honesty pass)
- [x] STEP 11 (connected surfaces): stone + planks faces against the
      SAME block sample edgeless variants (atlas 85->86 with stone_conn/
      planks_conn; the mesher picks per-face from the live neighbor;
      contract test).
- [x] STEP 12 (HUD legibility): text_shadowed helper (hard shadow) on
      the air gauge and the new chronicle toast; the on-kit audit
      conclusion: quest log / book / console / trade / tech tree / map /
      spellbook / imbue / carve / paths all use the ui_kit slide-panel
      system (built P22-P37 on the same kit).
- [x] STEPS 21-22 (chronicle + lore surfacing): chronicle_event now
      toasts milestones across the HUD while playing (fading, 4s), and
      the cross-system anchor test proves the Smith / Null / river
      threads span books + tome items + the Lorekeeper's trades.
- [x] STEP 27 (item belt backbone): BELT block + recipe; belts hold a
      stack and push one item per 1.5s into the first adjacent machine
      input that accepts it (furnace/e-furnace/crusher/assembler A+B/
      boiler fuel) — pure belt_push tested, client pass wired, stacks
      spill on break.

## Step 40 — the final honesty pass
The full evidence trail, restated plainly:
- 256 tests green across the workspace; 47/47 vistest proof scenes
  (every gameplay system has a rendered, pixel- or AI-verified proof).
- Smoke: the release binary boots and stays alive (12s) every loop.
- KNOWN HONEST LIMITS (also in STATUS.md): the Steamworks `steam`
  feature arms are written but unverified without the Steam SDK (UDP is
  the default + tested transport; the lobby model is transport-neutral
  and tested); the client-side trade-offer SEND UI is unwired (receive/
  apply is; protocol + server escrow + real-UDP test fully cover the
  trading deliverable); dragon roosts/loot chests, breath ignition, and
  blueprint rotation are deferred; connected textures cover exactly
  stone + planks; the perf target is met on this iGPU host only.

## lore-and-visuals build (2026-08-27, loop 327)

Done (verified — tests + vistest PNGs listed in DEVLOG):

- [x] A1 lore data layer: lf_lore + lore/*.toml (factions, world events,
      NPC roster, dialogue); standing in ClientSave; Nameless start −50
- [x] A2 faction territory tint on minimap + world map (30% blend,
      height shading still reads); unclaimed biomes untinted
- [x] A3/C4 faction standing HUD widget (name, symbol, colored bar,
      standing number, pulse on change via faction_pulse) — bottom-right
- [x] A4 twelve faction quests load, fire their objective types (incl.
      new Break/Place/Interact/Reach-tag/any-food events), completing
      moves standing (+15 issuer, documented ripples)
- [x] B1 companion model + serde round-trip (trust/morale/wage/state/
      tasks/cargo all persist in ClientSave)
- [x] B2 hire flow: standing ≥75 gate, fee deduction, villager→companion
      transition, chronicle entry; 4th hire refused with the doc line
- [x] B3 command menu on interact (follow/stay/rest/mine/chop/haul/
      guard/pay-now/dismiss) + trust/morale readout; low morale refuses
      work ("I need rest.")
- [x] B4 follow AI: 2-4 block standoff (never clings), defends against
      the player's attacker, working tasks break real blocks into cargo,
      contextual dialogue lines in chat
- [x] B5 morale-zero quit (chronicle + faction −5 + trust memory −15);
      unpaid wages −10 morale/day with warning; pay-now +2 trust
- [x] C1 38 new blocks with distinct non-stretched textures (contact
      sheet vistest_faction_blocks.png); catalog/recipe tests green
- [x] C2 6 villager faction skins (+2 named NPC skins), 6 companion
      skins + trust-badge variants at ≥50, 6 mob skins with distinct
      silhouettes, 9 biome-tint variants (vistest_entity_skins.png)
- [x] C3 six faction structures in home biomes (determinism + biome
      gating tested) with banner markers settling faction NPCs
      (vistest_<structure>.png x6, NPC cube in frame)
- [x] C4 companion HUD tiles; ember particles (vistest_ember_glow.png);
      AO verified present (mesher+shader); biome grade verified per
      biome incl. Volcanic (automated vistest grade test)
- [x] D1 standing-driven NPC behavior: ≤−30 refuses trade + hostile
      dialogue line; ≥+50 friendly pricing (10% discount)
- [x] D2 chronicle integration: standing titles on threshold crossings,
      companion hired/dismissed/quit, quest completions, structure
      discoveries — with world-event references by name + Era/Year
- [x] D3 map structure icons (faction-color diamonds) + territory tint
      re-verified with structures placed (vistest_faction_map.png)

Deferred (honest notes):

- [ ] The Unmarked's 5-choice dialogue interview (nameless_q2 completes
      via interaction; the variable-outcome interview tree is future
      dialogue work — the quest itself is playable)
- [ ] Ashen library's "readable lore book" is the chest + existing tome
      system; no library-exclusive book text written yet
- [ ] Companion Craft command is stubbed in the menu (recipes they know
      exist in the roster data; autonomous crafting is future work)
- [ ] Haul moves cargo to the companion's cargo-clearing behavior; chest
      targeting is simplified (nearest-chest pathing not implemented)
- [ ] Nameless camp chest loot is spawn-table based (raiders drop
      torn_archive_page); the chest itself initializes empty
- [ ] Named-NPC uniqueness ("one per world, largest camp") is
      first-settled-wins, not largest-camp-search

## Loop 330 — Phase A timber (master fix plan)

- [x] Valheim-style tree felling: `lf_game::timber` (find_tree / fall_plan /
  tree_parts, all pure + tested), client FallingTree entity with rigid
  rotated-cube animation around the stump hinge, landing as horizontal log
  blocks (ids 111-120, X/Z per species) with directional mesher faces so
  ring ends face along the log, canopy shatter + TreeCreak/TreeCrash
  sounds + camera shake. Proofs: tree_fall_mid (seeded angles +
  GPU animation-diff test), tree_fall_landed, falling_blocks_deep.
- [x] Deep falling-block animation: per-faller tumble (deterministic
  fibonacci-hashed axis), one 0.18-restitution bounce with dust, scalar
  physics untouched; perf gate p50 116.8ms vs 111 baseline (noise).
- [x] Fixed en route: birch/spruce/dark/cherry logs had no items (breaking
  them dropped stone) — four species log items + planks recipes now exist.
- [ ] Deferred (honest): remote clients see the fell result (block edits)
  but not the fall animation (the breaking client runs the entity);
  horizontal logs from player placement orient by face but there is no
  axe/stripping variant; giant-spruce falls render up to ~70 cubes (still
  noise vs chunk meshes, noted in DEVLOG).

## Loop 329 deferred (honest)
- [ ] beds/spawn setting, doors/signs, wool decor (P5 leftovers, untouched)
- [ ] music/ambient audio: the Music volume slider still drives nothing
  (the loop-329 Sfx set covers ui/body/movement feedback only)
- [ ] vistest UI proofs for the title-flow screens render kit-driven
  replicas (real layout helper + real ItemIcons), not the literal
  GameState::draw_* screens — pixel-testing the real screens needs a
  windowless GameState constructor (GameState::new requires a winit
  window + surface today)
- [ ] multiplayer connect still hardcodes the player name "smith"
- [ ] armor has no per-slot equip restrictions (any piece in any armor slot)

## Loop 332 — ai-npc-assets (Sections A-G, docs/ai-npc-assets/)

- [x] A black-square artifact: root causes addressed — the compositor
  alpha fix shipped in loop 331 (CompositeAlphaMode::Opaque), plus Live-RT
  invalidation on world transitions (stale voxel clip + stale egui image
  covered the viewport after load_world) and empty-column-batch guards at
  all three upload sites. `no_black_square` scene + pure-black run-length
  assertion on 8 daytime gameplay scenes.
- [x] B mob AI: `MobBehaviourState` machine (Idle/Wander/Chase/Attack/
  Flee/Investigate/Disengage, all 11 transitions), DDA line-of-sight
  (cached per tick, 32-block cap), faction standing modulating aggro
  radius (`effective_aggro_radius`, +100 = ignore unless attacked), group
  aggro (first-order neighbours, 0.5s reaction, ≤5 pack, no chains), A*
  pathfinding in `lf_game::mob_pathfind` (cardinal + 1-up jumps, 256-node
  cap, cached 2s / goal-drift invalidation, direct-steer fallback).
  Client wired: standing lookup + group propagation per frame.
- [x] C NPC behaviour: enriched 5-slot day (sleep/eat/work/socialize/
  return) with locations, `NpcActivityState` driving movement + render
  pose + dialogue posture (sleeping NPCs refuse trade), reaction lines
  (structure damage, combat panic, gifts, companion quit, +75 ack), NPC
  memory (last two interactions, 5-day window, greeting references)
  persisted via the ClientSave villager JSON. Gift = use-item-on-villager.
- [x] D testing: vistest scenes `mob_ai_visible` (real 120-tick mob sim,
  world-state assert), `npc_schedule_time` (midday = Work slot);
  `--smoke` headless flag (300 ticks: worldgen seed 42 superflat, 1
  passive + 1 hostile mob AI, NPC schedule, planks craft, block mine —
  exit-code + log-pattern checked); `make smoke` now runs the logic
  smoke AND the 12s GUI liveness check.
- [x] E connected textures: neighbour bitmask (corner rule), derived
  47-tile CTM table (const-evaluated, bijective, 0xFF→0 / 0x00→46), strip
  art generated per block with exposed-edge shading + interior dapple,
  second texture binding (192×512 strip atlas) + shader branch on CTM
  markers, mesher computes the bitmask (with diagonal neighbour sections)
  and bakes per-tile UVs. All 8 E5 blocks. Tests: `connected_texture_
  uv_3x3` (centre = interior tile rect, isolated = tile 46 rect) + table
  bijection test + vistest scene `connected_textures_grass_3x3`.
- [x] F asset generator: `xtask gen-texture` (grass-ctm-strip /
  stone-ctm-strip / entity-skin / block-noise, seeded xorshift64 +
  integer hash noise), `gen-ctm <block>`, `gen-all-textures` (skip
  existing). Deterministic: `asset_generator_grass_output` pins seed-42
  bit-identity; no pure black/white in output.
- [x] G wrap-up: full suite + vistest + smoke green; runtimes rebuilt.

Deferred (honest notes):

- [ ] E uses a single 192×512 strip TEXTURE + shader branch instead of the
  reference doc's per-block separate files (assets/ctm/*.png are exports
  for human review, not runtime-loaded) — the runtime has no PNG loader;
  the export/import split is documented in DEVLOG.
- [ ] CTM applies to top faces only (E5 grass-side stays dirt-side by
  spec); side/bottom faces of accord_stone walls are not connected.
- [ ] NPC schedule is the canonical table in lf_npc (+ per-slot client
  resolution); per-archetype TOML schedule overrides in lore/npcs.toml
  are not parsed yet (the TOML path exists for dialogue/quests only).
- [ ] "Hostile faction NPCs join the fight" (C3) is not implemented —
  there is no hostile-faction villager roster to react; only flee/panic.
- [ ] Gift flow consumes from the hotbar slot via right-click; there is
  no dropped-item-pickup path for NPCs.
- [ ] The honored "+75 acknowledgement" flag is session-state (lost on
  save/reload); memory itself persists.
- [ ] gen-all-textures covers the 8 strips + 6 skins; block-noise has no
  placeholder-block registry to drive batch generation (on-demand only).
- [ ] NullKnight keeps the generic behaviour machine (freezing it with no
  boss AI would be a regression); `use_boss_ai` gates dragons only.

## Loop 334 — king-quest mega-loop

- [x] 50 community mods (88 blocks / 79 items / 10 smelts; ores reach
  worldgen, lights reach the light engine, tools carry damage/durability)
  with a load-all contract test.
- [x] 15 new biomes with 18 new blocks, 9 new tree species, per-biome
  ground cover, climate-grid classification, extended structure gates,
  and the 46-strip contact sheet.
- [x] 4 animals (chicken, wolf, dog, bear): multi-part cube rendering,
  skins, spawn rules, combat behaviour.
- [x] The Accord Bastion walled city + frontier watchtowers + desert
  ruins, biome-gated by seeded hash (a biome may or may not carry one).
- [x] The Vassal system: recruit at Honored standing, daily deterministic
  yields, collect from the vassal, persistent on the villager save.
- [x] Steam honest pass: Workshop UGC dir loads in client + server;
  `lf_steam --features steam` compiles.

Deferred (honest notes):

- [ ] Steam P2P transport, overlay and achievements: no Steam client,
  SDK runtime or real AppID on this host (dev AppID 480). The transport
  selector compiles but is not exercised end-to-end.
- [ ] Multi-chunk city sprawl: the Accord Bastion is a single-chunk
  walled town (the structure system is per-chunk); a sprawling capital
  needs a cross-chunk placement system.
- [ ] Unique block art per mod pack (mod blocks share the generic
  mod-block atlas layer today) — the largest remaining gap vs the
  300-asset ask; ~200 new discrete assets shipped this loop.
- [ ] More tree shape variants per biome (9 new species shipped; one
  shape each) and vassal loyalty/wage mechanics (flat deterministic
  yields today).
- [ ] Villager TOML schedule overrides per archetype (canonical enriched
  schedule is the single default table).

## Loop 335 — asset-gap closure

- [x] Unique generated 16x16 atlas art per mod block (100 blocks),
  deterministic per namespaced id, palette-ruled (3+ colors, no pure
  black/white), pairwise-distinct (tested), one atlas layer per block.
- [x] 7 ring-top layers for the new tree species (per-face routing) and
  12 packs gained a signature block (mod blocks 88 -> 100).
- [x] Fixed the loop-B atlas drift (hand-counted layer constants were
  +4 off; all king-quest layers now derive from layer_of(name)) and
  raised max_texture_array_layers to 512 (the atlas is 294 deep).
- [x] Asset ledger: 320 new discrete assets across loops 334-335 — the
  300 target is cleared.
- [ ] Steam P2P/overlay/achievements remain BLOCKED on having a Steam
  client, an SDK runtime and a real AppID (documented in loop 334).

## Loop 335b — Steam exercised live

- [x] Steamworks end-to-end exercised on this host with the real client:
  init, Steam ID, stats request, matchmaking lobby create/leave, and
  live transport selection (preferred_transport() -> SteamP2p). Probe:
  `cargo run -p lf_steam --features steam --example steam_probe`.
- [x] Client boot logs the selected transport + feature wiring.
- [ ] Overlay activation: needs launching the game THROUGH the Steam
  client (works for non-Steam games too) — user-side step, not code.
- [ ] Game achievements/leaderboards: need a real partner AppID
  (current dev AppID is Valve's 480/Spacewar).
- [ ] ISteamNetworkingSockets as the in-game multiplayer transport
  (replacing UDP): the binding/init/selection are proven; only the
  socket swap remains.
