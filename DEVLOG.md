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
