# STATUS.md

## Current Milestone: P0 — Honest Baseline

**Progress**: 100%

### What is actually in the codebase (verified by audit, loop 282)
- 13 lf_* crates + 2 apps + xtask; ~2,600 lines of Rust; 43 unit tests passing
- Working: windowed wgpu renderer (depth-tested), culled-face mesher with
  per-block textures, palette-compressed voxel sections, DDA raycast,
  seeded noise worldgen (8 reachable biomes, elevation-aware), multi-chunk
  region persistence (zstd + atomic writes), survival/smithing/mob/quest/
  chronicle data models, TOML mod loading (2 example mods), protocol codec +
  UDP echo server, **real offscreen headless renderer + scene harness**
- Real proof screenshots: `shots/vistest_*.png` (rendered from actual
  worldgen data through the actual renderer)

### What does NOT exist yet (despite old claims)
- Player input/physics (no walking, no break/place in-game)
- World streaming, trees, caves, ores in-world
- Light propagation, transparency
- Any UI, audio, AI, crafting logic, multiplayer session
- The entire former "Evolution" feature list — re-planned below

### Roadmap (approved full-base-game plan)
- [x] P0 honest baseline + real screenshot pipeline
- [ ] P1 first-person core (input, physics, break/place, hotbar)
- [ ] P2 world streaming & terrain
- [ ] P3 lighting & atmosphere
- [ ] P4 survival & inventory UI
- [ ] P5 content catalog
- [ ] P6 mobs & combat
- [ ] P7 structures, weather, sound, menus
- [ ] P8 quests & chronicle live
- [ ] P9 multiplayer
- [ ] P10 mod API real
- [ ] P11 performance & release

### Ground rules (enforced from P0 on)
1. Every loop must change code and keep tests green — no docs-only commits.
2. Every claim needs a real screenshot from the in-repo render harness.
3. STATE/BACKLOG updated only to what is verifiably done.

### Blockers
None
