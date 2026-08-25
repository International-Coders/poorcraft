# RELEASE.md

## LOREFORGE — voxel sandbox RPG in Rust

Current state: **P0 honest baseline complete.** The engine renders, the world
generates, persistence works, and a real screenshot pipeline proves it. The
game is not yet playable — input, physics, and gameplay arrive in P1–P4 of the
approved plan (see BACKLOG.md).

### What runs today
```bash
cargo run -p loreforge                                              # windowed demo scene
cargo run -p loreforge -- --headless --scene terrain_vista --out s.png  # real render to PNG
cargo run -p xtask -- vistest shots                                 # render all scenes
cargo test --workspace                                              # 43 tests
cargo run -p loreforge-server                                       # UDP echo server
```

### Verified proof screenshots (real renders, not mockups)
- `shots/vistest_spawn_plains_dawn.png` — dawn over meadow terrain
- `shots/vistest_terrain_vista.png` — noon vista across biomes
- `shots/vistest_night_watch.png` — night scene

### Architecture
- `lf_engine` windowing + wgpu renderer (windowed & offscreen), `lf_voxel`
  sections/meshing/raycast/persistence, `lf_worldgen` seeded noise terrain +
  8 biomes, `lf_game` survival/smithing/mobs data, `lf_npc` villagers,
  `lf_story` quests, `lf_chronicle` saga generation, `lf_modapi` TOML mod
  loading, `lf_protocol` + `loreforge-server` networking, `lf_vistest` +
  `xtask` visual test harness.

### Historical note
Commits before "P0: honest baseline" include ~256 docs-only "Evolution Mode"
commits whose claimed features were never implemented. The CHANGELOG documents
the audit. Trust code + tests + `shots/vistest_*.png`, nothing else.
