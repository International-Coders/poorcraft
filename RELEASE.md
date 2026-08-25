# RELEASE.md

## LOREFORGE — voxel sandbox RPG in Rust

P0-P11 of the base-game plan are complete: this is a playable survival
sandbox with mining, crafting, smelting, mobs, quests, lighting, day/night,
structures, multiplayer and mods — all proven by tests and real renders.

### Play
```bash
cargo run --release -p loreforge            # play (title screen)
./target/release/loreforge-server           # host multiplayer (default :25565)
cargo run -p loreforge -- --headless --scene terrain_vista --out s.png  # render
cargo run --release -p xtask -- vistest shots    # all proof scenes
cargo run --release -p xtask -- package     # portable zip in dist/
cargo test --workspace                      # 104 tests
```

### Controls
WASD move · Space jump · Ctrl sprint · Shift sneak/descend · F fly ·
mouse look · LMB mine/attack (hold) · RMB place/use/eat · 1-9 hotbar ·
E inventory · J quests · T chat (multiplayer) · F2 screenshot · Esc pause

### What's inside (all verified by tests + real screenshots)
- Engine: wgpu renderer with depth, fog, day/night light factors, water
  alpha pass, block outline, offscreen proof-render pipeline
- World: streamed chunks (radius 5), 8 biomes, trees, caves, ores, water,
  structures (huts/watchtowers/pyramids), flood-fill lighting with torch
  emitters, save/load with autosave
- Survival: mining with tool tiers/harvest gating/durability, crafting
  (2x2 + table), furnace smelting, chests, hunger/health/fall/drown,
  death + respawn, item drops with magnet pickup
- Mobs: day/night spawning, chase/flee AI, melee combat with knockback,
  Null Knight boss
- Quests & chronicle: 5-quest chain driven by real gameplay events, saga
  exported to worlds/<name>/chronicle.md
- Multiplayer: dedicated UDP server, chat, block sync, remote players
  (two-client integration test in lf_server)
- Mods: runtime block/item/recipe/smelting registration + ore veins;
  examples in mods/

### Proof screenshots (real renders, not mockups)
shots/vistest_*.png — dawn/noon/night vistas, first-person view, terrain
features (trees/water), torchlit night, HUD proof via the egui overlay.

### Honest gaps (tracked in BACKLOG.md)
Sound/music, weather, PT-BR localization, beds/doors, armor, server-side
mob sync, smooth per-vertex lighting. The old pre-audit history is in
CHANGELOG.md.
