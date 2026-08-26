# AGENTS.md — LOREFORGE (POORCRAFT)

Voxel sandbox RPG in Rust (Minecraft-class): wgpu renderer, streamed voxel
worlds, survival/industrial gameplay, egui UI, compute path tracer, UDP +
Steam-ready multiplayer, TOML mods.

## Read first
- `STATE.md` — current loop count, milestone, test count, next task
- `BACKLOG.md` — done vs deferred (deferrals are honest, not failures)
- `CHANGELOG.md` — per-loop history (starts with the loop-282 audit)

## Non-negotiable ground rules
1. **No docs-only commits.** Every change ships code and keeps
   `cargo test --workspace` green. Screenshot claims must come from the
   vistest harness (`cargo run --release -p xtask -- vistest shots`) —
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
cargo build --workspace              # must be clean
cargo test --workspace               # 123 tests currently
cargo run --release -p loreforge     # play (title screen)
cargo run --release -p xtask -- vistest shots          # all proof scenes
cargo run --release -p xtask -- screenshot <scene> <out.png> [seed]
cargo run --release -p xtask -- package                # dist/ zip
make help                            # Makefile lists all targets (.mk info)
```
Smoke test rule: launch the release binary in the background, sleep ~12s,
check the process is alive, `pkill -f target/release/loreforge`.

## MANDATORY: desktop runtimes after completing a job
When a task is finished and green, ALWAYS produce fresh runtimes for the
user to test, then commit:
```bash
cargo build --release -p loreforge -p loreforge-server
# macOS app bundle + dmg (host is darwin):
mkdir -p "dist/loreforge.app/Contents/MacOS" "dist/loreforge.app/Contents/Resources"
cp target/release/loreforge "dist/loreforge.app/Contents/MacOS/"
cp target/release/loreforge-server dist/
printf 'APPLLORE' > "dist/loreforge.app/Contents/Resources/PkgInfo"
cat > "dist/loreforge.app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>CFBundleExecutable</key><string>loreforge</string>
<key>CFBundleIdentifier</key><string>com.loreforge.game</string>
<key>CFBundleName</key><string>LOREFORGE</string></dict></plist>
PLIST
hdiutil create -volname LOREFORGE -srcfolder dist/loreforge.app -ov -format UDZO dist/loreforge-macos.dmg
tar -czf dist/loreforge-linux-x86_64.tar.gz -C target/release loreforge loreforge-server
# Windows exe requires cross: rustup target add x86_64-pc-windows-gnu
#   brew install mingw-w64  (once); then:
#   cargo build --release -p loreforge --target x86_64-pc-windows-gnu
#   cp target/x86_64-pc-windows-gnu/release/loreforge.exe dist/
# If cross tooling is unavailable, ship the linux/macOS artifacts and say so.
ls -la dist/   # verify artifacts exist before reporting done
```
Report artifact paths in the final message. If any target genuinely cannot
build on this host, state which and why — never claim an artifact that
isn't on disk. (`make runtimes` automates all of this.)

## Layout
- `crates/lf_engine` — wgpu renderer (SceneResources/MeshBatch, outline,
  atmosphere (clouds/sun/stars/weather), `pathtrace` = compute voxel-DDA
  path tracer + persistent `Pathtracer` for Live RT)
- `crates/lf_voxel` — blocks (registry.rs is the single source of truth:
  ids, solidity/opacity, mod blocks >=100), meshing (winding matters —
  outward test), light (BFS; y-stride owns the column), World + regions
- `crates/lf_worldgen` — biome table (30 biomes in biome.rs), trees,
  structures, ore veins, WorldType (Normal/Superflat/Amplified)
- `crates/lf_game` — survival, items/mining/crafting/smelting, machines
  (generator/E-furnace/crusher/assembler + power field), research eras,
  combat (arrows/XP/armor), player physics
- `crates/lf_client` — the game shell: input, streaming, block entities,
  `ui.rs` (screens), `ui_kit.rs` (design system: theme/easing/Reveal/
  animated widgets), `net.rs` (UDP), `atmosphere` lives in lf_engine
- `apps/loreforge` (client bin), `apps/loreforge-server` (dedicated UDP)
- `xtask` — vistest/screenshot/package automation
- `mods/` — example TOML mods (README.md documents the API)
- `shots/` — proof PNGs; `docs/STEAM.md`; `steam_appid.txt` (480/Spacewar)

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
  ignored, `rm -rf target/release/.fingerprint/lf_vistest*` and rebuild.
- `git status` may show `STATE.md` dirty from a prior loop — read before
  overwriting; never clobber `worlds/` (player saves) in tests: use
  `tempfile` like the existing tests do.
- Steam: `lf_steam` feature `steam` is OFF by default (SDK links
  dynamically; falls back to UDP when the client isn't running).
