# BACKLOG — LOREFORGE

Honest status after the P0 audit (see CHANGELOG). Everything checked below exists
in code and is covered by tests and/or real rendered proofs (`shots/vistest_*.png`).
The former "Evolution Mode" list (~230 items claimed in loops 26–281) contained no
implementations; the genuinely built items are checked here, the rest are planned
below by phase.

## Done (verified)

- [x] M1 window opens and clear color (lf_engine)
- [x] M2 chunk data structure + culled meshing (lf_voxel) + texture array
- [x] M3 voxel raycast (DDA) with tests (not yet wired to input — P1)
- [x] M4 terrain noise heightmap + biomes + strata (all 8 biomes reachable, P0)
- [x] M5 world persistence (region files hold many chunks, atomic writes, P0)
- [x] M6 day/night cycle math + light level constants (propagation is P3)
- [x] M7 survival data types (stats, inventory stacking)
- [x] M8 smithing data model (8 materials, tool assembly, forge minigame)
- [x] M9 mob data model (6 types incl. Null Knight boss)
- [x] M10 mod manifest + block/item data loading (ember_ores, amberium examples)
- [x] M11 protocol codec + UDP echo server binary
- [x] M12 villager schedules + Geode Guardian / Cinder Crawler mobs
- [x] M13 quest data types (objectives, quest log)
- [x] M14 chronicle events + saga/markdown generation
- [x] Depth-buffered renderer with shared GpuScene (P0)
- [x] Real offscreen headless renderer + scene harness (P0)
- [x] xtask vistest/screenshot commands producing real PNGs (P0)
- [x] real item icons + tooltips + recipe book + minimap/world map/waypoints (P22)

## P1 — First-person core
- [x] keyboard/mouse input (WASD, jump, sneak, sprint, mouse look, cursor lock)
- [x] player AABB physics (gravity, collision, jump, fly; substepped anti-tunneling; 8 tests)
- [x] camera control (first-person from eye position; crosshair lands with P4 HUD)
- [x] block targeting outline via DDA raycast; break/place with player-overlap check
- [x] hotbar (1–6, scroll) with block placement; F2 in-game screenshots

## P2 — World streaming & terrain
- [x] chunk streaming: background generator thread, view radius 5, unload radius 8, nearest-first
- [x] worldgen features: trees (canopy in-chunk), caves (3D noise), coal/iron by depth, water at sea level
- [x] sphere-frustum + distance column culling using mesh bounds
- [x] save/load world (region chunks + player state) with autosave and save-on-exit
- [x] block registry: solid/transparent/targetable; water non-solid, raycast skips it

## P3 — Lighting & atmosphere
- [x] flood-fill sky + block light per column (BFS, opacity-aware; tests for falloff/overhangs/emitters)
- [x] torches/lanterns emit real light (14/15) and are placeable
- [x] day/night cycle drives sky color, clear color, sky-light factor + distance fog
- [x] water transparency (alpha-blended pass, back-to-front column sort; underwater tint in P7)
- [ ] smooth per-vertex lighting/AO (deferred to visual-identity polish; flat per-face now)
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
- [x] quest/chronicle state persists with the world
- [ ] lore books readable in-game (deferred)

## P9 — Multiplayer
- [x] protocol v3: join/leave, 20/s position snapshots, validated block ops, chat (+4 codec tests)
- [x] authoritative-lite lf_server: canonical world + edit history, newcomer replay, chat relay
- [x] two-client local integration test over real UDP (chat + block sync + positions)
- [x] dedicated loreforge-server binary (bind + seed args)
- [x] client join from title screen, remote players rendered, remote edits applied, chat UI (T)
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
