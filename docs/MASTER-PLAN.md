# LOREFORGE master fix plan — "fix all the rest" (approved 2026-08-28)

> Current direction: [`docs/BETA-FOUNDATION/README.md`](BETA-FOUNDATION/README.md)
> is the authoritative product-and-engine brief for the alpha-to-beta effort.
> This older checklist remains valuable implementation history, but it must not
> override the newer water, castle-spacing, NPC, asset, or multiplayer contracts.

Built from a full read of STATE/BACKLOG/AUDIT/STATUS/DECISIONS/DEVLOG and
docs/V1REBRAND, plus code research (loops 329–330). Every item ships with
tests + vistest screenshot proofs, one shippable job per dev-loop pass,
repo green between passes. Execution order (user-confirmed): A→B→C→D→E.
Progress tracking: STATE.md `next_task` + this file's checkboxes.

## Phase A — Timber: Valheim tree felling + deep falling-block animation — ✅ DONE (loop 330, commit ddc39e9)

- [x] A1 horizontal logs: ids 111–120 (X/Z × 5 species), directional mesher
  faces (ring ends along the axis), species-log stone-drop bug fixed.
- [x] A2 pure `lf_game::timber`: find_tree / fall_plan / tree_parts /
  fall_rotation, 8 tests.
- [x] A3 client FallingTree: rigid rotated-cube fall, landing as horizontal
  log row (blocked cells → drops), canopy shatter, TreeCreak/TreeCrash,
  camera shake.
- [x] A4 deep fallers: tumble (deterministic axis), one 0.18 bounce + dust;
  perf gate p50 116.8ms vs 111 baseline (noise).
- [x] Proofs: tree_fall_mid (seeded angles + GPU animation-diff test),
  tree_fall_landed, falling_blocks_deep — judge pass ×3.
- Deferred: remote clients see edits but not the animation; no axe/stripping.

## Phase B — Asset completion

Detailed continuation: `docs/ASSET-RENDERING-PLAN.md` (material maps,
humanoid/item assets, contact shadows, LODs, and proof/perf gates).

- [x] B1 Mod blocks get unique textures: deterministic hash-tinted variant
  layers instead of the single shared `mod` layer 47 (test: two mod ids →
  distinct; asset_catalog extended). Shipped loop 335.
- [x] B2 Entity skin coverage: non-archetype villagers stop wearing block
  textures (outfit skins per job, `rebuild_drop_batch` villager fallback);
  remote players get a real skin (not snow cubes); entity_skins scene claims.
  Shipped loop 338 together with six-part articulated humanoids and item
  impostors.
- [ ] B3 Connected textures completed: ASHEN_MARBLE + ASHEN_BOOKSHELF join
  stone/planks in `connected_variant` (+ lf_voxel's mirrored
  `lf_assets_conn`) with contract tests + side-by-side proof scene.
- [ ] B4 Procedural music/ambient (no asset files): day birds/wind, night
  crickets, cave drone, machine hum by proximity — `volume_music` finally
  consumed; looped synth with seamlessness tests; settings slider live.
- [ ] B5 Decoration texture overrides via mod.toml + mods/README + loader test.
- [ ] B6 Asset-scope hygiene: stale "creative is a stub" comment, audit
  placeholder/coming-soon strings.
- [x] B7 Authored-depth raster path: a linear RGB normal-map array (including
  CTM + dynamic layers) adds cheap per-pixel relief without RT; item sprites
  render as crossed alpha-cutout cards; CTM markers moved out of the real
  atlas range. Shipped loop 338.

## Phase C — World & survival gaps

- [ ] C1 Beds: block+item+icon+recipe; set spawn; skip night when no
  hostiles near; tests + bed_night proof scene.
- [ ] C2 Dawn/dusk light ramp: `sky_light_level` curve replaces the binary
  switch; sky/fog follow; curve pinned by test + dusk scene.
- [ ] C3 Quest/chronicle producers: q4 "collect iron" from furnace output +
  trades (`Collected`); producers or documented cuts for GreatTrade,
  StructureCompleted, VillageFounded, ReachedDepth; one test each.
- [ ] C4 Spawn-or-cut Geode Guardian + Cinder Crawler (cave spawns); hostile
  spawn light-level gating; tests.
- [ ] C5 Radiation scrub tool + suit; reactor meltdown destroys neighbor
  machine entities; tests.
- [ ] C6 Craft queue = real timed batch with progress UI; per-slot armor
  equip restrictions.

## Phase D — Multiplayer honesty

- [ ] D1 `Welcome.seed` adopted: joining clients regenerate terrain from the
  server seed (today only edited blocks sync). Real-UDP two-socket test:
  join → matching terrain sample.
- [ ] D2 Player name entry (persisted in Settings; remove hardcoded "smith"
  at ui.rs start_dedicated/connect path).
- [ ] D3 Client trade-offer SEND UI (protocol v4 + escrow already tested).

## Phase E — Perf, docs, release hygiene

- [ ] E1 F2 screenshot completeness: water, crack decals, particles in the
  offscreen re-render (AUDIT open item) + proof.
- [ ] E2 Live ≥30fps F3 confirmation recorded (DECISIONS Step-9 obligation);
  greedy meshing stays behind its DECISIONS gate.
- [ ] E3 Doc accuracy: RELEASE.md counts, missing DECISIONS entries
  (transparency/sort; RT-renders-shapes-as-cubes), the loop-318 genver.dat
  contradiction, stale BACKLOG checkboxes.
- [ ] E4 Windows cross-build attempt (rustup + mingw) or verified CI matrix
  as the Windows channel.

## Verification ladder (every pass, non-negotiable)

`cargo build --workspace` clean → `cargo test --workspace` green →
`cargo run --release -p xtask -- vistest shots` all green with new scenes'
pixel claims → human-eye/judge pass on new+changed PNGs → `make smoke` →
`make perf` compared to recorded numbers for perf-touching work →
bookkeeping (STATE/BACKLOG/CHANGELOG/DEVLOG, Makefile if commands change) →
`make runtimes` + artifact check → commit → `git push github HEAD`.
