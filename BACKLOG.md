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
- [ ] smooth per-vertex lighting/AO (deferred to P11 polish; flat per-face now)
- [ ] sun/moon/stars/clouds (deferred to P7 sky pass)

## P4 — Survival & inventory UI
- [ ] egui HUD (health/hunger/hotbar)
- [ ] inventory screen with drag/drop, shift-click
- [ ] crafting 2x2 + table 3x3, recipe matching
- [ ] tool durability, mining speed/harvest levels, breaking overlay
- [ ] item drop entities + pickup
- [ ] eating, fall/drown damage, death/respawn, beds

## P5 — Content catalog
- [ ] data-driven block/item registries (full catalog: stones, woods, ores, decor)
- [ ] functional blocks (furnace, chest, doors, beds, signs)
- [ ] full tool/armor tiers + recipes + smelting with fuel
- [ ] smithing table integration (existing forge minigame)

## P6 — Mobs & combat
- [ ] mob framework + spawning rules (day/night, light level)
- [ ] AI: wander/chase/flee + grid A* pathfinding
- [ ] combat: cooldown, knockback, armor mitigation, bow/arrows
- [ ] XP orbs + levels; Null Knight boss fight
- [ ] villagers wander by schedule + trading UI

## P7 — Structures, weather, sound, menus
- [ ] structure generator (villages, ruins, crypts, watchtowers, pyramid)
- [ ] world types (normal/superflat/amplified)
- [ ] weather (rain/snow particles + sounds)
- [ ] kira audio: steps/blocks/mobs/ambience/music (synthesized)
- [ ] title screen, pause, settings, key rebinding, EN + PT-BR

## P8 — Quests & chronicle live
- [ ] quest triggers from gameplay (collect/kill/reach/craft)
- [ ] quest log UI + tracking; chronicle records live events; lore books

## P9 — Multiplayer
- [ ] expand protocol (join/leave, snapshots, block ops, chat, inventory)
- [ ] lf_server authoritative sim; singleplayer = integrated server
- [ ] dedicated server + direct IP join; 2–8 players verified locally

## P10 — Mod API real
- [ ] runtime registration of modded blocks/items/recipes
- [ ] worldgen hooks (ember_ores/amberium generate); fix smelting.toml parsing
- [ ] modding docs

## P11 — Performance & release
- [ ] profiling (puffin), chunk gen/mesh budgets
- [ ] release packaging (macOS .app/.dmg, Windows .exe, Linux .deb) in CI
- [ ] honest RELEASE.md
