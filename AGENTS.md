# AGENTS.md — POORCRAFT repository control plane

This repository now has two physically separated Rust games:

- `poorcraft-antigo/` — preserved LOREFORGE/POORCRAFT legacy workspace.
- `poorcraft-novo/` — active POORCRAFT 3D workspace.

The root holds shared documentation, history, automation, and repository rules.

## Read first
- `STATE.md` — current loop count, milestone, test count, next task
- `BACKLOG.md` — done vs deferred (deferrals are honest, not failures)
- `CHANGELOG.md` — per-loop history (starts with the loop-282 audit)

## Non-negotiable ground rules
1. **No docs-only commits.** Every change ships code and keeps
   the affected workspace green (`make test` for the legacy game,
   `make p3d-test` for POORCRAFT 3D). Screenshot claims must come from the
   appropriate proof harness (`make vistest` or a named `p3d-*` route) —
   pixel-analyze the PNGs; never trust "it rendered".
2. **Update STATE.md / BACKLOG.md / CHANGELOG.md** after real work only.
3. Bugs found by proofs are fixed before committing (history: face-winding
   see-through terrain, unsampled light closure, egui pass encoded after
   texture readback, torches placed in a dead code copy).

## MANDATORY: job bookkeeping (every finished job)
- **Keep the Makefile current** (`Makefile` at the repo root is the .mk
  info file). It documents what can be done and how (`make help`). When you
  add or change a command/target, update the Makefile in the same commit.
  Common targets: build, release, test, run, server, smoke, vistest,
  screenshot, package, runtimes, push.
- **Log every action** in `DEVLOG.md` (the action log): append a dated
  entry per job stating WHAT was done, HOW it was done (files touched,
  approach, commands used), and the verification evidence (test counts,
  smoke result, artifact paths). One entry per job, newest last.
- **Push to GitHub after committing**:
  `git push github HEAD` — the remote is
  `https://github.com/International-Coders/poorcraft.git` (named `github`;
  create it with `git remote add github <url>` if missing, or
  `make push`). If authentication fails, say so explicitly in the final
  report — never claim a push that didn't happen.

## Build & verify
```bash
make build                           # legacy workspace
make test                            # legacy full suite
make p3d-build                       # active POORCRAFT 3D
make p3d-test                        # active POORCRAFT 3D full suite
make run                             # legacy title screen
make vistest                         # legacy proof scenes
make p3d-beta                        # POORCRAFT 3D long visual battery
make help                            # Makefile lists all targets (.mk info)
```
Smoke test rule: launch the release binary in the background, sleep ~12s,
check the process is alive, and stop only that binary. `make smoke` automates it.

## MANDATORY: desktop runtimes after completing a job
When a task is finished and green, ALWAYS produce fresh runtimes for the
affected game before committing: `make runtimes` for the legacy workspace and
`make p3d-dmg` (plus the portable package targets when available) for
POORCRAFT 3D. If cross tooling is unavailable, say so honestly.
Report artifact paths in the final message. If any target genuinely cannot
build on this host, state which and why — never claim an artifact that
isn't on disk. (`make runtimes` automates all of this.)

## Legacy layout (`poorcraft-antigo/`)
- `poorcraft-antigo/crates/lf_engine` — wgpu renderer (SceneResources/MeshBatch, outline,
  atmosphere (clouds/sun/stars/weather), `pathtrace` = compute voxel-DDA
  path tracer + persistent `Pathtracer` for Live RT)
- `poorcraft-antigo/crates/lf_voxel` — blocks (registry.rs is the single source of truth:
  ids, solidity/opacity, mod blocks >=100), meshing (winding matters —
  outward test), light (BFS; y-stride owns the column), World + regions
- `poorcraft-antigo/crates/lf_worldgen` — biome table (30 biomes in biome.rs), trees,
  structures, ore veins, WorldType (Normal/Superflat/Amplified)
- `poorcraft-antigo/crates/lf_game` — survival, items/mining/crafting/smelting, machines
  (generator/E-furnace/crusher/assembler + power field), research eras,
  combat (arrows/XP/armor), player physics
- `poorcraft-antigo/crates/lf_client` — the game shell: input, streaming, block entities,
  `ui.rs` (screens), `ui_kit.rs` (design system: theme/easing/Reveal/
  animated widgets), `net.rs` (UDP), `atmosphere` lives in lf_engine
- `poorcraft-antigo/apps/loreforge` and `poorcraft-antigo/apps/loreforge-server`
- `poorcraft-antigo/xtask`, `poorcraft-antigo/mods/`, `poorcraft-antigo/shots/`

## Current layout (`poorcraft-novo/`)
- `poorcraft-novo/apps/poorcraft3d` — active executable and proof routes.
- `poorcraft-novo/crates/pc3d_*` — core, world, render, save, assets, audio.
- `poorcraft-novo/tools/assetgen` — deterministic GLB factory.
- `poorcraft-novo/docs/CONTINUACAO-GLM` — mandatory current operating doctrine.

## Layer rules
- lf_engine may not depend on gameplay crates; lf_voxel is the substrate
  (worldgen → voxel; game → voxel). Client wires everything.
- Block ids/semantics change in `lf_voxel/src/registry.rs` first, then
  `lf_assets` (texture atlas layer + `texture_index_for_block`), then
  items/drops in `lf_game/src/items.rs`. Keep the catalog consistency test
  green — it catches dangling references.
- Settings live in lf_client `Settings` (persisted inside ClientSave);
  they must actually drive the engine (view_distance → streamer, FOV →
  camera, rt_mode → render path).

## Gotchas
- wgpu 24 + winit 0.30 + egui 0.31 are version-locked; egui-wgpu needs a
  `RenderPass<'static>` (scoped transmute in ui.rs / headless.rs).
- Offscreen UI proofs: the egui pass MUST be encoded before the texture
  readback copy or UI silently vanishes from screenshots.
- Cargo fingerprints occasionally go stale mid-session; if an edit seems
  ignored, clear only the affected package fingerprints inside that workspace.
- `git status` may show `STATE.md` dirty from a prior loop — read before
  overwriting; never clobber `poorcraft-antigo/worlds/` or
  `poorcraft-novo/saves3d/` in tests: use
  `tempfile` like the existing tests do.
- Steam: `lf_steam` feature `steam` is OFF by default (SDK links
  dynamically; falls back to UDP when the client isn't running).
