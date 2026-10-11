# RELEASE.md

## LOREFORGE — voxel sandbox RPG in Rust

A complete base game plus the industrial expansion: 30 biomes with
weather/clouds/sun/moon/stars, survival combat (bow, armor, XP), villages
with trading NPCs, an industrial tier (ore veins, coal generators, powered
electric furnaces/crushers/assemblers), research eras with a tech-tree
screen, a compute-shader voxel path tracer, multiplayer, and modding —
proven by 168 tests and 22 pixel-verified screenshot scenes (counts re-verified by the build-pack audit, 2026-08-26).

### Play
```bash
cargo run --release -p loreforge                # play (title screen)
./target/release/loreforge-server               # host multiplayer :25565
cargo run --release -p xtask -- vistest shots   # render all 14 proofs
cargo run --release -p xtask -- package         # portable zip in dist/
cargo test --workspace                          # 168 tests
```

### Controls
WASD move · Space jump · Ctrl sprint · F fly · mouse look · LMB mine/
attack · RMB place/use/eat/trade · 1-9 hotbar · E inventory · J quests ·
K tech tree · T chat · R path-traced capture · F2 screenshot · Esc pause

### Steam
Steam-ready: `steam_appid.txt` (Spacewar 480) for dev, feature-gated
`lf_steam` transport (falls back to UDP without the client), depot +
steamcmd guide in docs/STEAM.md.

### Proofs
shots/vistest_*.png — every pillar verified by pixel analysis. Known
deferred polish listed in BACKLOG.md.
