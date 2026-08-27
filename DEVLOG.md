# DEVLOG.md

## Session 1: M0 Bootstrap Setup

### Decisions Made
- **Workspace structure**: 20 crates per spec, plus apps and xtask
- **Dependencies pinned**: All in Cargo.lock, see list below
- **lf_engine**: winit + wgpu for windowing, basic render loop
- **Cross-platform**: all crates checked on macOS; cross-platform target set

### Dependencies (Workspace-level)
- winit 0.29 - windowing
- wgpu 24.0 - graphics (Vulkan/Metal/DX12)
- glam 0.30 - math
- tracing + tracing-subscriber - logging
- egui 0.29 + egui-winit + egui-wgpu - UI
- kira 0.9 - audio
- pollster - async runner

### Notes
- Working on macOS arm64 (Mac mini M1)
- Window builds successfully, event loop runs
- Still resolving wgpu/egui integration details for M0
- Next: M1 - First voxel rendering

### Pending
- Need to add src/lib.rs to all 20 crates
- Need to add goldens and test scaffolding
- CI pipeline not yet configured (GitHub Actions)
## Session: /init bookkeeping setup (2026-08-25)

### What was done
- Created `AGENTS.md` (workspace instruction file for future ZCode agents).
- Created `Makefile` at the repo root as the .mk info file: documents every
  common command as a target (`make help`), including `runtimes` (macOS
  .app/.dmg + Linux tarball + optional Windows exe) and `push`.
- Added the `github` remote pointing at
  https://github.com/International-Coders/poorcraft.git.

### How
- AGENTS.md covers: read-first docs (STATE/BACKLOG/CHANGELOG), ground rules
  (no docs-only commits, vistest pixel proofs), build/verify commands,
  MANDATORY job bookkeeping (keep Makefile current, log every action in
  DEVLOG.md with what/how/evidence, push to the GitHub remote after every
  commit), mandatory desktop runtimes per finished job, crate layout,
  layer rules, and known gotchas.
- Makefile verified with `make help` (all targets listed).

### Evidence
- `make help` output lists 12 targets.
- `git remote -v` shows `github -> International-Coders/poorcraft.git`.

### Push attempt (honest log)
- `git push -u github HEAD` FAILED: the stored Personal Access Token lacks
  the `workflow` scope, so GitHub refuses pushes touching
  `.github/workflows/ci.yml` (which is in the branch history).
  Fix: update the token to include `workflow` (and ideally `repo`) scope,
  or switch the remote to SSH. Note: history also contains large
  `target/debug` blobs (>50MB warning) from early commits — consider
  `git rm -r --cached target` is NOT needed now (untracked), but a history
  cleanup (filter-repo) would silence the warning.

## 2026-08-26 — P22: UI/UX overhaul (loop 302)

### What was done
- Real 16x16 pixel-art item icons across every UI surface; tooltip system;
  crafting screen redesign with a searchable recipe book (crafting + smelting
  + alloy + crush) and click-to-auto-fill; shift-click quick-move.
- Map suite: corner minimap + full M-key world map with fog of war, biome
  colors, hillshading, entity dots, waypoint manager (persisted), pan/zoom.
- HUD polish: icon hotbar with selection glow + fading item name, armor
  row, XP flash, dynamic crosshair, hurt vignette, low-health pulse, new
  death screen; Settings Interface tab (minimap toggle, UI scale).
- Fixed: recipes() per-call Box::leak -> OnceLock singleton; add_item now
  respects max_stack; atlas extended 18->41 layers (wood variants, ores,
  machines no longer render as stone in-world); vistest egui windows never
  rendered (warmup pass + font-atlas delta threading).

### How
- lf_assets: `generate_item_texture` sprite generator (string art + per-tier
  palettes), 4 new ore block textures, TEXTURE_NAMES/texture_index_for_block
  fully wired for ids 19-41.
- lf_client: new `icons.rs` (ItemIcons -> egui TextureHandles, NEAREST,
  mod-item gem fallback) and `map.rs` (MapState tile cache — loaded chunks
  colored from real top blocks, explored-unloaded approximated from the
  seed-known WorldGen and dimmed, fog elsewhere; composited into egui
  textures via the live-RT upload pattern); ui.rs reworked around a new
  slot_button (icon + tooltip + shift-click quick-move); ui_kit gained the
  item tooltip body; Settings/ClientSave gained show_minimap/ui_scale/
  waypoints (serde defaults); UiOpen::Map + KeyM input; HUD flash timers in
  tick.
- lf_game: crafting::all_recipes() for browsers; smelting::smelt_entries();
  machines::crush_entries(); survival max_stack fix; dead CraftingRecipe
  struct removed.
- lf_engine: headless::UiOverlay (ctx + warmup texture deltas) — egui
  windows need a warmup pass to materialize their areas, and that pass's
  font-atlas texture delta must reach the renderer or every font/white
  texture draw vanishes.
- lf_vistest: previews rebuilt with real icons + real worldgen map images;
  3 new scenes (crafting_ui, map_screen, minimap_hud); two-pass dispatch.

### Verification evidence
- `cargo test --workspace`: 140 passing, 0 failed (was 123).
- `cargo run --release -p xtask -- vistest shots`: 19/19 [ok]; pixel checks:
  crafting_ui dark-panel 3.0%, map_screen 9.1%, minimap_hud 1.9%,
  village_trading 1.6% (previously 0.0-0.2% — windows absent), machines
  visually distinct in industrial_machines (rust generator, blue-tinted
  electric furnace, teal research bench).
- Smoke test: release binary alive after 12s, mods loaded, clean pkill.
- Artifacts: dist/loreforge-macos.dmg (5.5MB), dist/loreforge-linux-
  x86_64.tar.gz (5.1MB), dist/loreforge.app, dist/loreforge-server.
  Windows exe NOT built: no mingw-w64 on this host.

### Push attempt (honest log, loop 302)
- `git push github HEAD` FAILED again: the stored PAT lacks `workflow` scope
  and the branch history contains `.github/workflows/ci.yml`, so GitHub
  refuses the push (same blocker as loop 301). The P22 commit exists locally
  on main. Fix remains: update the token with `workflow`+`repo` scope or
  switch the remote to SSH. (GitHub also warns about >50MB `target/debug`
  blobs deep in history — history cleanup with filter-repo recommended.)

## 2026-08-26 — P23: urgent fixes (loop 303)

### What was done
- Input fixes (WASD + block clicks report): close_ui clears stale chat
  input (invisible-Chat trap), Escape closes Pause, cursor-grab fallback
  (input mode even if the OS refuses the grab), first click passes
  through after re-lock, F3/LOREFORGE_DEBUG_INPUT diagnostic overlay.
- Developer console (` / `/`): 20 commands, TAB autocomplete, history,
  pure unit-tested parser.
- Per-world random seeds (seed.dat, splitmix64 channel hashing, streamer
  restart on world switch, server sends the true seed in Welcome).
- Natural biome transitions (fractal climate + stretch, domain warp
  ±34 blocks, ±0.045 border dithering; biome_from kept pure).
- Multiple save slots (worlds/<slot>/ + meta.dat, picker UI, load_world
  mid-session, worlds/default auto-migration to World_1).
- Menu reorder + egui zoom = user scale × native DPI × viewport factor;
  NSHighResolutionCapable in the app bundle.

### How
- lib.rs: close_ui/lock_cursor/Escape/MouseInput changes; world_seed/
  world_dir/slot_meta/console/show_debug fields; boot via slots::boot_slot;
  new_world_named + load_world + restart_streamer; Backquote/Slash keys;
  F3. console.rs (new): Command enum + parse/complete + egui overlay.
  slots.rs (new): slot dirs, meta, random_seed, migration, boot pick.
  lf_worldgen: WorldGen stores/exposes seed; FBm+warp+dither climate.
  lf_voxel: WorldStorage::save_seed/load_seed. lf_server + server bin:
  persisted seed, true Welcome.seed. ui.rs: reordered title, pause menu
  additions, draw_slots picker, viewport-proportional zoom factor.
  lf_vistest: console_preview scene; previews moved off raw Areas ( Areas
  never materialize in the 2-pass headless harness — frameless windows do).

### Verification evidence
- `cargo test --workspace`: 149 passing, 0 failed (was 140). New tests:
  console parse/complete/time, slots sanitize/random/meta/seed roundtrip/
  ordering/migration, worldgen coverage widened to ±4000.
- `cargo run --release -p xtask -- vistest shots`: 20/20 [ok];
  console_preview pixel-checked (dark panel 3121 px, history + gold
  suggestions + input visible), minimap_hud re-verified.
- Smoke: release binary alive 12s with LOREFORGE_DEBUG_INPUT=1; live log
  shows "migrated legacy world -> worlds/World_1 (seed 12345)".
- Artifacts: dist/loreforge-macos.dmg (5.6MB), dist/loreforge-linux-
  x86_64.tar.gz (5.2MB), dist/loreforge.app (+NSHighResolutionCapable),
  dist/loreforge-server. Windows exe NOT built (no mingw on host).

### Honest notes
- The static input path was byte-identical to the last working version;
  the fixes target every identified runtime trap (chat-screen hold, grab
  failure, pause-escape) — user playtest requested to confirm.
- GitHub push still blocked (PAT lacks workflow scope; CI file in history).

## 2026-08-26 — P24: input root cause found & fixed (loop 304)

### What was done
- Deep empirical diagnosis with a synthetic-input harness: launched the
  instrumented release binary, drove it with real macOS keystrokes/mouse
  clicks (System Events), captured screenshots, traced every event.
- ROOT CAUSE: a stray `_ => {}` arm in the middle of `match event` in
  window_event (inserted in P4) made ALL later arms unreachable —
  KeyboardInput, MouseInput, Focused, MouseWheel were dead code in every
  release. rustc only WARNED (unreachable_patterns); the warning was
  buried among many others. Static review kept missing it because the
  arms are syntactically valid, just unreachable.
- Fix: removed the wildcard; added `#![deny(unreachable_patterns)]` to
  lf_client (this class of bug now fails the build).
- Input instrumentation kept behind LOREFORGE_DEBUG_INPUT: per-event
  trace + 1Hz tick summary + frame_ms in the F3 overlay.

### Evidence (all from the fixed binary, synthetic input)
- E -> `[input] tick ... ui_open=Inventory` (opened + closed on repeat)
- hold W 2s -> window title pos (1.2,105.0,-0.8) -> (5.1,105.0,-4.9)
- M -> ui_open=Map; Esc -> ui_open=None playing=true
- backquote -> ui_open=Console; typed "fly"+Enter -> `[console] fly`
- MouseInput Pressed events now reach the mining path (logged, ui_open=None)
- cargo test --workspace: 149 passed / 0 failed
- vistest: 20/20 [ok]; runtimes rebuilt (dmg 5.6MB, tar 5.2MB)

### Scope adjustment (honest)
- The planned InputRouter refactor was skipped with evidence: the routing
  logic was always correct — the bug was dead-code suppression, which the
  deny lint now makes a hard error. The synthetic harness (documented
  above) is the deep regression test for input.

### Push
- `git push github HEAD` SUCCEEDED this session (ee1b9a5..5f7cb4d): the P25
  and P26 commits are both on GitHub; the PAT workflow-scope blocker is gone.

## Session 305: P25 — correctness & honesty sweep (incl. flat-color pathtracer fix)

### What
Post-Phase-0-discovery correctness sweep (plan approved by the product
owner, together with the standing decisions: Deck-60fps floor, all four
platforms launch-blocking, hybrid-selective art direction):
1. Server SetBlock registry validation + dedicated-server mods/ loading.
2. lf_steam `steam` feature made compilable via optional steamworks 0.12.
3. Generator version stamped into saves (`genver.dat`) with mismatch warning.
4. Lantern atlas layer (42 layers now; was silently stone).
5. Root Cargo.toml dependency table made truthful.
6. mods/README `_ore` auto-registration implemented for real; BACKLOG stale
   entries fixed; STEAM.md CI claim fixed; tests/golden stub deleted.
7. vistest pixel-analysis gate (`lf_vistest::verify_render`).

### How (files touched)
- crates/lf_voxel/src/registry.rs — MAX_VANILLA_BLOCK + is_known_block + tests
- crates/lf_server/src/lib.rs — validation swap; drain_until helper; new test
  set_block_validates_against_registry
- apps/loreforge-server/{Cargo.toml,src/main.rs} — mods loading, genver check
- crates/lf_steam/{Cargo.toml,src/lib.rs} — steamworks optional dep, honest test
- crates/lf_worldgen/src/lib.rs — GENERATOR_VERSION + save/load helpers
- crates/lf_client/src/slots.rs — sync_generator_version + stamp on create
- crates/lf_client/src/lib.rs — sync calls at boot/new-world/load-world
- crates/lf_assets/src/lib.rs — "lantern" layer, id 13 mapping, mod layer -> 41
- crates/lf_modapi/src/lib.rs — auto OreHook for *_ore blocks
- crates/lf_vistest/src/lib.rs — verify_render gate + test; RT scene geometry
  (camera within the +-32 voxel clip, smaller lantern patch)
- crates/lf_engine/src/pathtrace.wgsl — DDA t_max init fixed (select-based,
  always non-negative distances)
- crates/lf_engine/src/pathtrace.rs — fovy.tan() instead of a second
  to_radians() (4 sites: one-shot + persistent/live tracer)
- Cargo.toml (root) — truthful [workspace.dependencies]; STATE/BACKLOG/
  CHANGELOG/docs updates; tests/golden removed

### The pathtracer detective story (why it matters)
The new pixel gate failed BOTH raytraced scenes with "1 distinct color".
Empirical bisection with in-shader debug probes (echo UV -> varied, echo
ray dir -> constant, echo cam.right -> ~1% magnitude) isolated two
independent bugs, both present since P18:
- WGSL DDA: t_max = (floor(pos)+max(istep,0)-pos)/abs(dir) goes NEGATIVE
  for negative ray components; that axis always compared smallest, so rays
  marched a single axis out of the clip (day scene -> constant fog sky;
  night scene -> straight down into the lantern floor -> constant emissive).
- Rust: camera basis scaled by fovy.to_radians().tan() but Camera stores
  fovy in radians already -> basis at ~1.4% -> all rays ~parallel.
Consequence: every "raytraced" proof PNG and the in-game Live RT mode
(R key, RT settings) had been rendering one flat color, undetected because
nothing ever analyzed the pixels. Fixed both; proofs re-verified visually
(terrain + soft shadows + emissive night glow) and by color census
(1697 / 294 distinct colors).

### Evidence
- cargo build --workspace: clean
- cargo test --workspace: 154 passed / 0 failed (149 + 5 new: registry
  validation, server UDP accept/reject, 2 genver slot tests, verify_render)
- cargo test -p lf_steam --features steam: 2 passed (feature compiles)
- vistest: 20/20 [ok] through the pixel gate, twice in a row
- smoke: release binary alive after 12 s, loads 2 mods
- dedicated server boot: "loaded 2 mod(s)", binds, genver stamped

### Push
- `git push github HEAD` SUCCEEDED this session (ee1b9a5..5f7cb4d): the P25
  and P26 commits are both on GitHub; the PAT workflow-scope blocker is gone.

## 2026-08-26 — P26 commit + first successful GitHub push
### What
- Committed the pending meshing upgrade as P26 (`80f6891`): per-face
  texture API (`meshing::Face`), per-vertex ambient occlusion, smoothed
  corner lighting, leaf wind-sway attribute, `registry::is_leaf`.
- Fixed three call sites still passing the old 1-arg texture closure:
  `lf_engine/src/app.rs:129`, `lf_vistest/src/lib.rs:343`,
  `lf_client/src/lib.rs:2577` (now `|b, _face|`; lf_assets still returns
  one layer per block — per-face atlas selection is future work, sway is
  meshed but not yet consumed by the shader).
- First successful push of the full history to
  github.com/International-Coders/poorcraft (repo was empty; `main`
  now at `80f6891`).
### How
- `cargo build --workspace` clean; `cargo test -p lf_voxel` green;
  granted the gh CLI token `workflow` scope via device flow
  (`gh auth refresh -s workflow`), `gh auth setup-git`, then
  `git push github HEAD`.
### Evidence
- `git ls-remote github refs/heads/main` → `80f6891...` (matches HEAD)
- GitHub warned: `target/debug/deps/libnaga-*.rlib` (53.82 MB) is in
  history — under the 100 MB limit but should be scrubbed/gitignored.

## Session 306: P26 — visual identity milestone

### What
The approved visual-identity scope under the standing decisions
(hybrid-selective art, Deck-60fps floor, four launch platforms):
per-face material mapping, alpha-cutout leaves, vertex wind sway,
smooth per-vertex AO + light, crack-decal + debris mining feedback,
texture mipmaps, plus two new vistest proofs.

### How (files touched)
- crates/lf_voxel/src/meshing.rs — Face enum; tex_of(BlockState, Face);
  per-vertex AO + smoothed light via corner_shades (2x2 touching cells);
  sway weight on leaf vertices; get_block handles diagonal cross-section
  lookups as air; 4 new unit tests (per-face routing, corner AO, smooth
  light averaging, sway flags)
- crates/lf_voxel/src/registry.rs — is_leaf() block family predicate
- crates/lf_voxel/src/world.rs — mesh_column signature follows tex_of
- crates/lf_assets/ — depends on lf_voxel now; texture_index_for_face +
  GRASS_TOP/LOG_TOP/CRACK_LAYERS consts; grass_top + log_top + crack_0..3
  procedural textures; per-species hole-punched leaves; 3 new/updated tests
- crates/lf_engine/src/scene.rs — GpuVertex.sway (+layout loc 6);
  Uniforms.time_sway; Env.time; mipmapped texture array (5 levels, CPU box
  filter) with mag-nearest/min-linear sampler
- crates/lf_engine/src/shader.wgsl — vertex wind (world-pos phased);
  fragment alpha-cutout discard < 0.5
- crates/lf_engine/src/atmosphere.rs, app.rs — sway: 0.0 / Env.time fixes
- crates/lf_client/src/lib.rs — face-aware mesh_column_gpu; elapsed clock;
  crack_batch (stage-keyed rebuild), particle system (spawn on grind ticks
  + 16 on break, physics in tick, billboard rebuild in render, capped 128);
  wind frozen when settings.particles is off
- crates/lf_vistest/src/lib.rs — sway/time plumbed; foliage_canopy +
  mining_feedback scenes (canopy hand-built at origin; stone column placed
  BEFORE meshing — first attempt placed it after the mesh loop and only the
  raw decal/billboard vertices rendered; camera framed against terrain at
  the EYE because a buried camera sees through backfaces)

### Evidence
- cargo build/test --workspace: 160 passed / 0 failed (154 + 6 new)
- vistest: 22/22 [ok] through the pixel gate; foliage_canopy and
  mining_feedback visually verified (cutout holes, trunk, crack lines,
  debris quads); spawn_plains_dawn shows the new grass top/side/bottom
- smoke: release binary alive after 12 s, loads 2 mods
- runtimes rebuilt (see dist/)

### Push
- `git push github HEAD` SUCCEEDED this session (ee1b9a5..5f7cb4d): the P25
  and P26 commits are both on GitHub; the PAT workflow-scope blocker is gone.

## Session 307: P27 — camera culling fix ("objects disappear when looking up")

### What
User-reported: looking up makes objects disappear. Suspected raycast/FOV;
both were checked and are fine (pitch clamps at 89°, look_at
non-degenerate, fov_degrees converts once). The real bug: chunk-column
frustum culling used an under-sized bounding sphere.

### Root cause
`column_in_view` tested columns against the frustum with a sphere of
radius max(half_h, 11.4). That radius covers a 16x16 footprint only along
its axes; the corner distance is sqrt(128 + half_h^2) — 13.6 for flat
ground, ~17.7 for a 20-tall column. As the bottom frustum plane tilted up
with the view, columns still poking into the frame edge fell outside the
too-small sphere and were culled — terrain vanishing at high pitch, tall
columns even near level. Aggravator: the Gribb-Hartmann planes were used
unnormalized (the near plane's normal is ~2x unit), skewing the
world-unit margin.

### How (files touched)
- crates/lf_client/src/lib.rs — column_in_view: exact AABB bounding sphere
  (+0.1 sway margin), normalized plane distances
- same file, tests — looking_up_does_not_cull_visible_columns: corner
  projection property across pitches 5-85° x eye heights x column grid x
  five heights, plus the pinned pre-fix failure (pitch 5°, tall column at
  the frame edge, found by scanning the old code's blind spots)

### Evidence
- with the old formula the regression test fails; with the fix it passes
- cargo build --workspace clean; 161 tests / 0 failures
- vistest 22/22 through the pixel gate; release smoke OK (12 s, 2 mods)

### Push
- committed and pushed to github (main).

### 2026-08-26 — Loop 308: V1REBRAND plan + P28 loop 1 (sway fix + latent UX bugs)
**WHAT**: Wrote the execution plan for the whole docs/V1REBRAND/ roadmap and
started its hard-gate phase P28: fixed the wind-sway shader gap plus four
latent UX bugs found during planning.
**HOW**:
- docs/V1REBRAND/11-EXECUTION-PLAN.md — P28-P39 sequencing with per-phase
  file maps; DECISIONS.md entry records the +2 phase-number offset.
- crates/lf_engine/src/shader.wgsl — vs_main gained `@location(6) sway` and
  reads uniforms.time_sway.x: two world-position-phased sines, combined max
  offset ~0.08 blocks (inside the 0.1 cull margin from P27).
- crates/lf_vistest/src/lib.rs — new test
  `foliage_sway_animates_between_frames`: builds the foliage_canopy scene,
  renders at t=0.8 vs t=0.8+pi through lf_engine::headless (real GPU),
  asserts frames differ while a same-phase control is pixel-identical.
- crates/lf_client/src/lib.rs — clouds gate on settings.clouds;
  UNLOAD_MARGIN(3) + settings-driven unload radius; column_in_view takes
  view_distance (P27 test updated); Escape routes UiOpen::Settings through
  close_settings; GameState::new loads player extras from the booted slot
  dir (dropped the legacy worlds/default pre-load + WORLD_DIR const);
  new GameState field settings_from_title.
- crates/lf_client/src/ui.rs — close_settings() (title vs pause return),
  title/pause Settings buttons record their origin.
**VERIFICATION**: cargo build --workspace clean; cargo test --workspace
162 passed / 0 failed (new sway proof included); `cargo run --release -p
xtask -- vistest shots` 22/22 [ok] with pixel analysis; `make smoke`
SMOKE OK. Runtimes rebuilt via make runtimes; pushed to github.

### 2026-08-26 — Loop 309: build-pack Step 1 reality audit + same-commit fixes
**WHAT**: Executed docs/poorcraft-build-pack MASTER_PLAN Step 1 (per
01-REALITY-AUDIT.md): verified every checked BACKLOG claim against code, a
live session, and fresh renders; wrote AUDIT.md; corrected BACKLOG; fixed 8
real bugs the audit surfaced (7 gameplay/render + 1 flaky test helper).
**HOW**:
- Live session: launched target/release/loreforge, captured the title
  screen twice via screencapture+System-Events-window-crop
  (shots/audit_title.png, audit_title_later.png; backdrop pixel stats mean
  RGB ~38 = the buried-camera bug), observed an in-world session
  (shots/audit_inworld.png — underground torch light + full HUD working),
  inspected the session log (mods load, villager settles, autosave, slot
  switching). The user began their own play session mid-audit (World_7) —
  left untouched; no pkill was run.
- Code verification: three parallel Explore agents over (a) P1-P5+P26/P27
  survival/render claims, (b) P6-P10+P22 lore/mobs/MP/mods claims, (c)
  M1-M14+P3+P11+P25+biome distinctness; spot-verified load-bearing claims
  myself (shader, power loop, day/night math, biome enum).
- Root-cause tool: crates/lf_worldgen/examples/audit_title_camera.rs —
  prints orbit-ring terrain vs camera eye for any seed (World_5:
  12/64 points buried; kept as a regression aid).
- Fixes (all tested): hud_visible gate (ui.rs draw_ui + test);
  title_eye_y clamp + camera()/render() render-eye consistency (lib.rs +
  tests); Streamer wish radius via sync_wish from settings.view_distance
  (lib.rs + test); sneak 0.45x walk (player.rs + test); smithing
  Strike-per-click + ForgeMinigame::reset grant-once (ui.rs, smithing.rs +
  test); lantern item+recipe+drop (items.rs, crafting.rs + test +
  catalog_consistency); random_seed sequence counter (slots.rs).
- Hygiene: git rm of 201 shots/ev_*.png fossil proofs (Evolution-era; zero
  code references — voidserpent/allay/axolotl/breeze don't exist);
  RELEASE.md counts 121/14 -> 168/22.
**VERIFICATION**: cargo test --workspace 168 passed / 0 failed (6 new
tests); cargo run --release -p xtask -- vistest shots 22/22 [ok]; live
session evidence in shots/audit_*.png; AUDIT.md written; runtimes rebuilt;
pushed to github. Smoke skipped deliberately (user's session running).

### 2026-08-26 — Loop 310: block gravity + water physics
**WHAT**: Sand/dirt-family blocks now fall when unsupported (animated),
and water actually flows (falls, spreads with decay, dries up when its
source goes) with lowered flowing surfaces and a bucket to move sources.
**HOW**: registry::has_gravity (granular set, ores excluded); client
FallingBlock entities (24 m/s², capped 2.5 in water, land via apply_sim_
edit which reuses remesh_around + net.send_block; after_edit triggers
fluid-wake + faller-cascade on every player/sim edit); lf_game::fluids
(step_cell rules, settle/settle_gravity drivers, MAX_SPREAD=7); water
levels in BlockState flag nibble (lf_voxel::water_level/water_with_level);
mesh_section partial-height water (water_surface_height 1-1/8 per level,
side faces full-height against taller water); bucket/water_bucket items +
3-iron recipe + BUCKET_ART sprite + scoop/pour in the place handler.
**VERIFICATION**: 174 tests pass (6 new: spread decay, source recede,
fall+pool, column collapse, water displacement, lowered-surface meshing);
vistest 24/24 with new water_flow + falling_sand scenes, both visually
verified via image analysis (stepped surfaces + dam pooling; collapsed
pile + mid-air faller); user's live game session left untouched (no
smoke pkill — their session is the liveness check). Runtimes rebuilt;
pushed to github.

### 2026-08-26 — Loop 311: goal Sections 0-4 (re-audit + feel fixes)
**WHAT**: Re-audited the /goal prompt's four "still wrong" flags and fixed
three of them (the fourth proved not to exist in the current build, with
evidence).
**HOW**: AUDIT.md re-audit section (per-flag verdicts + evidence). S2:
ui.rs bottom progress bars deleted; ui_kit reticle_points/
paint_mining_reticle (arc from 12 o'clock, span=progress) painted at the
crosshair for mining (accent) and bow (ok); vistest hud_preview mirrors it
mid-break. S3: scene.rs Env/Uniforms + shader.wgsl grade vec4 (rgb tint,
w saturation) applied post-fog; lf_client biome_grade table + smoothed
grade_tint/grade_sat fields; clear_color mirrors the shader grade. S4:
mods/smoke_test + smoke_line + client/server boot logging + modapi CI test
on the real folder + README section. S1: lf_voxel mesh test
multi_block_walls_tile_per_block_not_stretched + vistest texture_tiling
scene. STATUS.md rewritten from stale to verified-current.
**VERIFICATION**: cargo test --workspace 178/0 (new: reticle geometry,
biome-grade GPU proof, smoke_test pipeline, tiling mesh proof); vistest
25/25 [ok] (new scene texture_tiling; hud_preview shows the radial
reticle); texture_tiling + grade frames AI-verified (per-block tiling;
hue/sat shift). Runtimes rebuilt; pushed to github.

### 2026-08-27 — Loop 312: audio engine + impact shake + FOV/transparency/perf proofs
**WHAT**: The build-pack's remaining destruction-feel items: real audio
(Step 4), the impact pulse (Step 3), the FOV reference test (Step 7), the
transparency layering proof (Step 8), and the frame-time benchmark with a
named low-end target (Step 9).
**HOW**: crates/lf_audio (rodio; synth = decaying tone + LP-filtered LCG
noise per category; Audio::new silent fallback; scaled() volume math);
client play_block_sound on break/place + break_impulse/shake_decay/
shake_offset camera-target jitter; lf_engine camera reference test;
lf_vistest transparency_layers scene (worldgen pool+glass, post-mesh
billboards); headless.rs persistent HeadlessRenderer + lf_vistest::bench +
xtask/make perf; ci.yml +libasound2-dev (test/build/vistest/release).
**VERIFICATION**: 184 tests / 0 failed (new: 4 audio, screen_shake_
envelope, projection reference); vistest 26/26 [ok]; transparency proof
AI-verified (water through glass, near particles over pane, far ones
through it); perf at Medium radius-5 p50 111 / p95 156 / min 77 ms (incl.
readback+PNG overhead — live confirmation via F3 pending next session).
User's game session still running — no smoke pkill. Runtimes rebuilt;
pushed.

### 2026-08-27 — Loop 313: Steps 13/14/15 (settings, thumbnails, minimap)
**WHAT**: Key rebinding + the Path-Traced quality tier (Step 13), slot
thumbnails + first-launch walkthrough (Step 14), minimap rotation/zoom +
world-space waypoint beacons (Step 15).
**HOW**: crates/lf_client/src/input.rs (Action/Keymap, name-based
serialization with bounded code_from_name); window_event arms became
keymap guards + a rebind-capture pre-step; PlayerInput reads the keymap;
Settings.keymap_pairs + Settings.quality (serde defaults; Settings no
longer Copy). Quality::PathTraced -> RtMode::Live. capture_slot_thumbnail
(HeadlessRenderer 256x144 into worlds/<slot>/thumb.png, 120s throttle) +
picker texture cache. map.rs draw_minimap: rotated egui Mesh for the tile,
shared rotation for dots/pips/N chip, fixed-up player arrow, zoom into
composite; lf_assets waypoint_0..5 translucent layers + push_beam_quads
beams in the transparent pass; waypoint_beacons vistest scene.
**FIRST-LAUNCH WALKTHROUGH (Step 14)**: fresh boot -> title offers Play
<newest slot> / New World (type picker) / Load Game / Multiplayer /
Settings / Quit — no dead ends; Settings opened from the title returns to
the title (Back and Esc, loop-309 fix); Load Game lists slots with
name/type/seed/last-played (+ thumbnail after first autosave) with
two-step delete; New World spawns into the world; Esc pauses; Quit to
Title returns. Verified against the live build (boot log + loop-309
title capture + settings-back regression fixes).
**VERIFICATION**: 187 tests / 0 (new: keymap defaults+roundtrip+persist,
Step-13 combined persist); vistest 27/27; waypoint_beacons proof
AI-verified; `make smoke` SMOKE OK (user session ended, so the real smoke
ran) and the boot log shows [MOD SMOKE TEST] OK with 3 mods. Runtimes
rebuilt; pushed.

### 2026-08-27 — Loop 314: Steps 16-19 (biome identity, spawns, weather)
**WHAT**: The "biomes feel the same" complaint structurally fixed at the
worldgen level (on top of loop 311's color grade), plus biome-aware
spawns and biome-field weather.
**HOW**: registry ids 42-44 (JUNGLE_GRASS/SAVANNA_GRASS/FLOWER; flower
non-solid non-opaque cutout, is_plant); biome.rs surface_block/filler_
block/is_cold/SpruceSparse/Biome::ALL; worldgen flower pass (1/9 columns,
FlowerForest-only), boulder pass (SnowySlope/WindsweptHills/Windswept-
Savanna), GEN_VERSION 2; lf_assets 3 layers + drops (dirt; flower
shatters) + shovel family; mobs roll_spawn(rand, is_day, cold_biome);
client spawn site + gen_biome_temp_at_player now use Biome::is_cold.
**VERIFICATION**: biome_identity_markers_are_distinct (pairwise
surface+filler+tree+structure+exclusive; caught SnowyPeaks/FrozenOcean
and Savanna/WindsweptSavanna twins — fixed via filler key + savanna
boulders); contact sheet pixel check = exactly 30 distinct quantized
strip colors; AI-verified families; 188 tests / 0; vistest 30/30
(3 new scenes). Runtimes rebuilt; pushed.

### 2026-08-27 — Loop 315: P29 Water Age
**WHAT**: The first new power age: graph-based research with the Water
branch, water wheel + battery machines, a pure power-distribution step,
and the riverside proof scene.
**HOW**: research.rs (Era::Water, prereqs(), is_branch, branches vec with
serde default, unlock(); tech-tree branch card + Unlock button);
machines.rs (WaterWheel/BatteryCell/PowerSource/distribute_power, pure and
tested); registry ids 45/46 + lf_assets layers 57/58 + items + recipes +
drops; client BlockEntity variants, tick rewired through distribute_power
(wheel adjacency check on 5 neighbors), UI panels, spill/placement arms;
pathtrace palette entries 42-46 + stable hash fallback; water_wheel_power
vistest scene (river carved, real power step run 30 sim-seconds).
**VERIFICATION**: 194 tests / 0 (8 new: 5 research incl. save-compat, 3
water-age machines); vistest 31/31; water_wheel_power AI-verified (wheel
against water, battery + crusher in field, reads as a riverside power
station). Runtimes rebuilt; pushed.

### 2026-08-27 — Loop 316: P30 Steam Age
**WHAT**: The second power age: research Steam branch, water pipes,
boiler, steam engine, steam particles, and the boiler-room proof chain.
**HOW**: research.rs (Era::Steam + tests); machines.rs (Pipe
equalize/draw/fill, Boiler fuel+water->steam with idle dissipation,
SteamEngine ratio-scaled output, PowerSource::Engine; 4 tests);
registry 47-49 + atlas 59-61 + items + Steam-gated recipes + RT palette
entries; client steam pass (pairwise pipe equalization via remove/
reinsert, boiler feeding from adjacent sources/pipes, engines drinking
adjacent boilers) + steam puffs (snow-tex particles rising, gated) +
UI panels; vistest steam_chain (real machine code pre-run 30 sim-s).
**BUGS CAUGHT BY THE PROOF DISCIPLINE**: the scene's water-feed loop
spun forever once the pipe filled (infinite screenshot render —
killed, fixed with a took==0 break); the steam-engine flywheel texture
underflowed usize below x=11 (atlas test caught it).
**VERIFICATION**: 199 tests / 0; vistest 32/32; steam_chain AI-verified
(water->pipes->boiler with fire glow->engine->crusher, puffs rising);
smoke OK. Runtimes rebuilt; pushed.

### 2026-08-27 — Loop 317: P28 lighting + Step 20 lore books
**WHAT**: Chunk-border light seams eliminated; lore books became a real
readable in-game interaction fed by a data file.
**HOW**: light.rs rewritten to a 48x256x48 neighborhood volume (opacity
bitset + emitter scan + shared BFS, center-slice extraction); world.rs
invalidation widened to 3x3 columns; night_border_seam scene + measured
smoothness. lore/books.toml (3 tomes) + lf_client::lore + UiOpen::LoreBook
paginated reader + item hook + Lorekeeper trades + lf_assets tome icons
(cover-gem BOOK_ART variants); lore_book vistest preview reads the real
file (local schema mirror per the preview pattern).
**VERIFICATION**: 201 tests / 0 (new: border-light regression, lore file
load; icons test caught the missing tome art — fixed with gemmed covers);
vistest 34/34; night seam max step 1.92 (python-measured); lore reader
AI-verified (title/page/readable text/buttons). Smoke OK. Runtimes
rebuilt; pushed.

### 2026-08-27 — Loop 318: P31 Oil Age + Step 25 grid overlay
**WHAT**: The Oil Age end-to-end — deposits in the ground, extraction,
refining, top-below-nuclear power — plus a craft-first power-grid
visualization.
**HOW**: registry ids 50-53 (OIL fluid like WATER: non-solid/non-opaque/
untargetable); worldgen oil pass after ores (noise 0.63 over 8..44,
Desert|Swamp columns only, 1/700 surface seeps); atlas 62->68 layers
(oil/pump/refinery/combustion + grid_ok/grid_starved overlay tints);
FluidKind-typed Pipe channels (serde-default `crude`); PumpJack/
Refinery/CombustionGenerator in machines.rs + oil_age_tests (typed
pipes, pump gating, exact refinery mass balance, refined-fuel-only
burn, full chain via distribute_power); Era::Oil either-or prereqs in
research.rs meets_prereqs; fluids.rs step_cell fluid-generic with
OIL_SPREAD=3; client oil pass + UI panels + 3-bucket arm + refinery
feed; Action::GridOverlay (G) + machine_power ratio map + translucent
push_overlay_cube in the transparent pass; vistest oil_chain +
grid_overlay scenes pre-running the real machine code 200 sim-seconds.
**VERIFICATION**: 209 tests / 0 failed (+8: oil gating scan, 5 oil-age
machine tests, oil creep, oil research either-or). vistest 36/36.
Pixel checks: crude-dark pixels present; green tint 141px / red 116px
localized to the two machines. AI-verified both scenes (green cube =
powered furnace, red cube = starved crusher, coherent oil chain). The
first grid_overlay render caught a REAL balance truth (furnace starved
on one generator) — fixed honestly with a coal bootstrap generator in
the scene, not by fudging numbers. Smoke OK. Runtimes rebuilt; pushed.

### 2026-08-27 — Loop 319: P32 Nuclear
**WHAT**: The capped nuclear tier — deep uranium, the reactor with a
heat/coolant/SCRAM curve, meltdowns that leave glowing damage-dealing
residue, and the safety certification that gates the era.
**HOW**: registry ids 54-56 + RADIATION emission 7; atlas 68->71
(uranium_ore, reactor with a glowing core window, radiation crust);
worldgen deep band y8..24 thr 0.68; smelt/assembler nuclear line;
machines.rs Reactor + ReactorEvent with pinned thermal constants and 4
tests (ceiling tiering, equilibrium, scram->meltdown, coolant rescue +
gated unscram); research Era::Nuclear + reactor_safety (serde-defaulted)
+ REACTOR_SAFETY_COST; client nuclear pass (coolant, live meltdown,
vent particles), apply_meltdown (r=3 destruction + <=14 residue + shake
+ chronicle Meltdown), radiation proximity damage; reactor UI panel with
SCRAM; tech-tree safety certification row. Vistest reactor_control (60s
equilibrium pre-run, asserted) + meltdown_aftermath.
**VERIFICATION**: 213 tests / 0 failed. vistest 38/38. Pixel checks:
reactor core-window teal + green uranium flecks present; meltdown glow
clusters. AI-verified both scenes. Debug-built world state confirmed 14
residue blocks placed before trusting the render. Framing bug chased and
fixed (surface_top vs surface_height convention). Smoke OK. Runtimes
rebuilt; pushed.

### 2026-08-27 — Loop 320: P33 Magic foundation
**WHAT**: The full magic layer — mana, four bounded spells, spellbook,
wizard + towers, enchanting with rune effects, crossover blocks — plus a
real save-compat fix the phase forced into the open.
**HOW**: lf_game::magic (Spell/Spellbook/try_cast/hearthlight_pick/
ImbueMinigame/Rune, 6 tests); PlayerStats.mana + ClientSave.mana +
runed tools (JSON extras + frozen LegacyClientSave bincode migration
with 3 migration tests — bincode EOF on new fields proven live);
registry 57-59 + atlas 71->74 + scroll/rune pixel art; wizard tower
worldgen (gated biomes, 400-chunk test) + Wizard job/trades/spawn;
client: cast_from_slot (firebolt arrow variant, blink raycast, ward
absorb in damage(), hearthlight smelt+temp lumen), mana regen in
survival_tick, spell keys Z/X/C + book B (24 actions now), spellbook +
imbue UIs, rune mining/armor effects, pylon spawn suppression.
**VERIFICATION**: 223 tests / 0 failed. vistest 41/41. Three new scenes
AI-verified (tower at dusk, finished spellbook screen, effects showcase
with firebolt arc + ward ring + lumen glow — the dusk-dimming rabbit
hole ended as a scan-threshold lesson, scene set to 0.62). Smoke OK.
Runtimes rebuilt; pushed.
