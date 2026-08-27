# STATUS — what is actually verified working

Human-facing status, kept separate from STATE.md (the session/loop log).
Everything listed here was verified against the running build — code +
tests + rendered proofs, and where the player sees it, a live session or
AI-verified screenshot. AUDIT.md at the repo root holds the full per-claim
evidence trail (last full re-audits: 2026-08-26, loops 309–311; this file
replaced a stale version claiming 121 tests / 14 scenes and "live RT
deferred" — both no longer true).

## Working (verified in play / by rendered proof)

- First-person core: WASD + mouse-look + cursor lock, jump/sprint/sneak
  (sneak = careful 0.45x walk), AABB physics with anti-tunneling, DDA
  targeting outline, break/place with player-overlap check, 1–9 hotbar +
  scroll, F2 screenshots (offscreen re-render — no water/crack in the
  capture itself).
- Survival: hold-to-mine with hardness/tool tiers/harvest gating/durability,
  hunger/regen/fall damage/drowning/death+respawn, eating, item drops with
  magnet pickup, furnaces/chests (contents spill), crafting 2x2 + 3x3,
  smithing minigame (click-to-strike, grants once).
- Rendering: per-vertex AO + smoothed corner light, per-face materials,
  alpha-cutout leaves with real wind sway (GPU two-phase proof), mipmaps,
  crack decal + block-textured debris on mining, mining/bow progress as a
  crosshair-centered radial ring (no bottom-of-screen bar), per-block
  texture tiling on multi-block surfaces (mesh-test + visual proof),
  per-biome color grade with smooth boundary blending (GPU hue/sat proof).
- Physics: granular blocks fall (sand/red_sand/snow/dirt/grass/moss/
  mycelium — animated, land through the player edit path); water flows
  (source + 7 flow levels, fall-first spread, dries when unsupported),
  stepped flowing surfaces; bucket scoops/pours sources.
- World: 30 biomes (17 still share a worldgen twin — see gaps), biome-gated
  structures (hut/watchtower/pyramid), weather rain/snow, sun/moon/stars/
  clouds, day/night, superflat/amplified world types.
- Content: 6 mobs + Null Knight boss, villagers with trading, quests (J) +
  chronicle readable in play (also exported on save), research eras + tech
  tree (K), machines (coal generator → E-furnace/crusher/assembler),
  world map + minimap + waypoints (M), recipe book, lantern + torch.
- Multiplayer: dedicated UDP server (validated block ops, edit replay),
  client join (hardcoded localhost + name — address entry still open),
  chat, remote players/edits. Mods: 2 example mods + smoke_test (boot line
  `[MOD SMOKE TEST] OK`), runtime block/item/recipe/smelting/ore-vein
  registration, CI-enforced full pipeline.
- Path tracing: opt-in Live RT (live-updating path-traced view) + R-key
  captures — both working (DECISIONS entry recorded).
- Packaging: macOS dmg + Linux tarball runtimes, portable zip; CI
  (test/build/vistest/release matrix). No Windows exe on this host
  (mingw missing). 178 tests, 25 vistest proof scenes.

## Known-not-working / honest gaps (details in BACKLOG.md deferred notes)

- No audio at all (break/place sounds are the top feel gap — build-pack
  Step 4).
- Biome *texture* identity is thin under the new grade: untinted grass, no
  biome fog colors, no per-biome decorations — deeper palette work pending.
- Chunk-border light seams (per-column BFS accepted for now); connected-
  surface textures pending; key rebinding, save thumbnails, minimap
  rotation/zoom + beacons pending (P28 remainder).
- Multiplayer terrain desync (Welcome.seed ignored), lore depth (5 of 11
  chronicle event types never fire; no villager dialogue; lore-book
  *content* beyond the saga is the deferred stub — the book UI itself
  works), Geode Guardian/Cinder Crawler are dead data, q4 "collect iron"
  only counts ground pickups.
- Greedy meshing stays deferred until its UV-repeat invariant is provable
  on merged quads (DECISIONS precondition).

Last full play-verification of this list: 2026-08-26 (live sessions during
loops 309–311 + 178-test / 25-scene automated suites).
