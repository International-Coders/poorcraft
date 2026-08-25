# CHANGELOG

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
