# CHANGELOG

## 2026-09-03 — authoritative host owns block edits (loop 359, B03 slice 1)

- **The client no longer edits the world directly — at all.** New pure
  `lf_game::host::SimHost`: systems queue block commands (monotonic id +
  sim tick + `EditKind` reason) and the host applies them to the voxel
  world in canonical (tick, id) order, recording one domain event per
  outcome. Replayed ids are rejected (a packet-loss duplicate applies
  once, ever); out-of-loaded-space edits are rejected **with events**,
  never silence.
- **All 16 runtime `world.set_block` sites migrated** through two
  funnels: `host_set_block` (same-frame apply + remesh + network
  broadcast — mining, placing, scaffold columns, symmetry mirrors, tree
  felling, paste builds, lumen blocks, hearth expiry, crater residue,
  support-removal fallers) and `apply_remote_block_update` (multiplayer
  BlockUpdate: recorded by the host but never re-broadcast — the echo
  loop is structurally impossible now).
- **The contract is test-enforced in both directions**: host unit tests
  prove idempotence, rejection-with-events, total ordering, and
  event-sourced history; and a new client **self-audit test**
  `include_str!`s its own source and rejects any `.set_block(` outside
  the two funnels or test code — reintroducing a direct edit fails the
  suite, exactly B03's "direct client mutation is test-rejected".
- **Multiplayer fix shipped by the seam**: player mining never broadcast
  to the server before (the server never learned mined blocks); the
  funnel's uniform broadcast now covers every local edit. Save
  compatibility untouched — the host keeps no persisted state this
  slice.
- 470 tests (+4 host, +1 self-audit), smoke OK (the headless
  onboarding/craft journey runs through the host), 107/107 vistest
  freshly rendered, runtimes rebuilt. Slice 2 (craft/inventory
  transactions through the host) closes B03 and Stage A.

## 2026-09-03 — deterministic tick, command IDs, domain events (loop 358, B02)

- **The simulation now has a clock that render cadence cannot bend.** New
  pure `lf_game::sim`: `TickClock` converts the client's variable
  real-frame dt into whole 60 Hz sim ticks through an integer-microsecond
  accumulator with an 8-tick shed cap (a one-second freeze fires exactly
  the cap and drops the backlog — deterministic, never a death spiral).
  `advance()` returns the inclusive RANGE of ticks to execute: the
  original count-returning API was a real design flaw the cadence proof
  caught — looping `0..count` collapses multi-tick frames onto the last
  tick number and silently skips the others.
- **Commands and events get total order and stable fingerprints.**
  `CommandEnvelope` (monotonic id + issuing tick) with `canonical_batch`
  ordering by (tick, id), duplicates keeping their earliest occurrence —
  batching and reordering are provably inert. `EventLog` records
  immutable domain events under a dense monotone seq, hashed FNV-1a over
  (tick, seq, kind, payload) — no randomized `DefaultHasher`.
- **The done-when is enforced, not claimed**: snapshot-hash tests run a
  representative command-driven sim under uniform 60 fps, 3-tick, and
  mixed 1/2-tick frame partitions over the same 600 ticks — identical
  state and event-log hashes; one-by-one vs single-batch delivery hash
  identical; jitter streams replay identically and track wall time within
  2 ticks; overload shedding is deterministic; hashes are
  perturbation-sensitive.
- **Fixed a real pre-existing race the new tests exposed**: crafting's
  mod-recipe tests guarded the global registry with private mutexes (and
  `mod_recipes_match` didn't lock at all), so
  `mod_fingerprint_tracks_the_mod_set` could fail under load — all three
  now share one `MOD_REGISTRY_LOCK`, stress-verified at
  `--test-threads=8`.
- 465 tests (+6 sim), smoke OK, runtimes rebuilt. Client wiring is
  deliberately untouched — B03 migrates block edits and inventory/crafting
  through commands behind the integrated host.

## 2026-09-03 — runtime truth dashboard (loop 357, B01)

- **Stage A of the beta-foundation roadmap opens with honesty
  infrastructure.** New `xtask/src/truth.rs` publishes a machine-readable
  dashboard of what the engine actually is: 21 systems across four
  ownership classes (ServerAuthoritative / RelayOnly / ClientLocal /
  DeterministicGenerator), schema versions (protocol v4, generator v7),
  scene and test counts, optional live perf, and seedlab fold-through —
  written to `target/truth_report.json` by `xtask truth` / `make truth`.
- **The dashboard cannot lie about authority.** Every
  `ServerAuthoritative` row must cite the server crate AND carry marker
  strings checked against the live `lf_server` source at compile time
  (`include_str!`) — claims without implementations fail, and losing an
  implementation (e.g. someone rips out SetBlock handling) fails too.
  The 13 systems the loop-356 audit names client-simulated are pinned
  `ClientLocal`: relabeling one requires deleting a line from
  `KNOWN_CLIENT_ONLY` — a reviewable contract change, never silent.
  Evidence paths and audit-list coverage are validated at runtime in
  `build_report` as well.
- Truthful baseline recorded: server authority is exactly
  `session_and_peers` + `block_edits`; everything else is relay
  (presence/chat/trade/UDP) or client-local until B03 starts the
  migration. Also committed the `docs/BETA-FOUNDATION/` goal pack and
  the MASTER-PLAN deprecation pointer.
- 459 tests (+6 truth-contract tests), smoke OK. Tooling-only job —
  dist binaries unchanged, runtimes not rebuilt.

## 2026-09-03 — the biome identity contract (loop 356, N07)

- **Forty-six biomes, and now a law: none may look like another.** Every
  biome carries an identity row — surface, filler, tree silhouette,
  ground-cover density and feature set, freeze, cold — and a test
  enforces that no two biomes share the whole visible tuple. The scan
  found one surviving clone family: the oceans, which now have three
  distinct floors (murky dirt Ocean, tropical-sand Warm Ocean, pale-stone
  Deep Ocean). Generator version bumped to v7 for the change.
- **Ground cover has a confetti ceiling.** Feature density is capped at
  0.35 by test (Jungle, Lavender Fields, and Sunflower Plains were over);
  identity reads from negative space, not from filling every cell.
- **The proofs photograph the identity itself.** The biome contact sheet
  now grows each biome's signature tree over its real surface and
  ground-cover features — 46 strips of "what this place is" — and the
  new `biome_transitions` scene shows four intentional boundary pairs
  (desert/savanna, snowy plains/taiga, meadow/forest, swamp/jungle), each
  joined by a dithered mixing band rather than a hard seam. Both
  image-reviewed; 107/107 visual scenes.
- **Proof:** 453 tests, the 64-seed lab still passing its diversity
  floors on v7, smoke green, runtimes rebuilt. Per-biome fog/sky grading
  and gameplay resource rows are honestly deferred (fog needs an engine
  input the atmosphere doesn't have yet).

## 2026-09-03 — every seed knows where you wake up (loop 355, N06)

- **Spawn is selected, not assumed.** A deterministic spiral search picks
  the actual spawn: dry land above the sea, a land biome, off rivers, no
  tree in the cell, no kingdom on top — and it reports the nearest wood.
  Ocean-centered seeds used to drop the player into the water at (0,0);
  the seed laboratory caught exactly that (0 of 64 origin spawns were
  valid) and now scores the real selection instead of a proxy. New
  worlds and loads place the player — and the respawn point — there,
  with an arrival hint naming the biome and wood distance.
- **The seeds render their evidence.** Three new proof scenes:
  `seed_atlas_8` paints eight labeled maps straight from the generator's
  biome and height fields at the laboratory's macro scale, gated by a
  cross-panel disagreement metric; `seed_same_control` regenerates half
  the world from the same seed and requires cell-identical columns plus
  a seamless render; `spawn_quality_8` lists eight find_spawn verdicts
  with every safety invariant asserted before a pixel draws. Building
  the atlas gate exposed (and fixed) an inverted-range bug in its own
  sampler where every panel but the first sampled zero pixels.
- **Measured, again.** The 64-seed lab passes its diversity floors by
  wide margins on generator v6 (height L1 p05 0.090 vs 0.020 floor;
  biome JS p05 0.214 vs 0.025), so this pass repaired the one real
  defect the metrics named — spawn — and left terrain shape alone.
- **Proof:** 451 tests (+1: spawn safety/reachability across a 16-seed
  spread + determinism of the selection), 106/106 visual scenes, all new
  scenes image-reviewed at 0.95–0.98 confidence, smoke green, runtimes
  rebuilt. True multi-viewport panorama atlases and the river/biome
  transition scenes are honestly deferred (N07).

## 2026-09-03 — world identity and the seed laboratory (loop 354, N05)

- **A world has a name that means something.** The canonical
  `WorldIdentity` — seed, generator version, world type, and a
  fingerprint of everything modded that can touch generation — is
  stamped into `identity.dat` before the first chunk generates, restored
  exactly on load, and shown on F3. A save from an older generator now
  tells you plainly that unedited areas may have changed, and modded
  recipe/ore-hook sets are part of the world's provenance.
- **Same seed, same world — proven, not promised.** The new seed
  laboratory hashes height and biome lattices with order-independent
  combiners: the same identity is bit-identical across instances
  (negative, ±1M, and ±i32::MAX coordinates included), and generating
  chunks in any order produces identical columns.
- **Different seeds, different worlds — measured.** 64 fixed seeds are
  sampled into a machine-readable report (`make seedlab` →
  target/seedlab_report.json): height-field distances, biome
  Jensen–Shannon distances, water/river/cave fractions, surface
  composition, and kingdom placement, with calibrated diversity floors
  the reduced 12-seed corpus also enforces in the test suite. Generator
  v6 measures height L1 p05 = 0.090 (floor 0.020) and biome JS p05 =
  0.214 (floor 0.025) — comfortably diverse; N06 now has numbers to
  repair against instead of opinions.
- **Multiplayer shares one world.** The Welcome packet always carried the
  server's seed; the client now adopts it as its world identity and
  restarts streaming when it differs, instead of silently generating a
  private world.
- **Proof:** 450 tests (+12: seed-text rules, hash stability, identity
  round-trip, version policy, channel decorrelation, corpus diversity,
  bit-identity, order independence, JS behavior, mod fingerprints),
  smoke green, runtimes rebuilt. The rendered seed atlases are N06's
  evidence by design.

## 2026-09-03 — the HUD speaks context (loop 353, N04)

- **What you can do, right now, is written beside the crosshair.** A
  prompt shows the live keymap chip, the verb, and the target — "E Trade
  — Mara", "LMB Hold to mine — Iron Ore", "RMB Place — blocked by
  player" — with hostile standing barring the door in red. Priority
  resolves companion > villager > functional block > mine > place, and
  rebinding the key changes the chip.
- **Combat reads at a glance.** A red arc flashes around the crosshair
  from the direction damage came (world-true: it stays put while you
  turn), an attack-readiness ring sweeps while the swing cools down, and
  one priority-resolved danger line (drowning > critical health >
  starving > threats) sits above the hotbar on a dark plate — a fix the
  image review forced after catching the warning drowning in busy
  terrain.
- **The world reacts visibly.** Standing changes now toast with the
  faction crest, the signed delta, the reason ("quest fulfilled",
  "destroyed their structure", "a gift well received"…), and the new
  threshold title when a band is crossed; entering a kingdom announces
  its name and whether its gates are barred to you, once per visit.
- **Proof:** 103/103 visual scenes (four new: hud_contextual at
  1280×800, hud_contextual_small at 640×420, hud_danger, hud_reputation,
  all image-reviewed), 438 tests (+3: prompt copy/keymap chips, transient
  fade/caps, strict danger priority), smoke green, runtimes rebuilt.

## 2026-09-03 — the modal workbench and input recovery (loop 352, N03)

- **The workbench is a modal now, not a transparency.** Strongly opaque
  framed panels sit over a world scrim, and the survival HUD (hearts,
  hotbar, XP, minimap) is no longer drawn beneath any container/station
  screen — the audit's "duplicate HUD through the workbench" failure is
  gone. The zone layout is pure shared math (`workbench_layout`),
  unit-tested to never overlap or clip at 640×420, 800×600, or 1280×800,
  and the proof scenes sample those same rectangles.
- **Small windows get a drill-down, not small text.** Under 700px the
  workbench collapses to category chips + one pane (recipe list, or the
  detail with a ← back link) over a single-row hotbar strip.
- **Discovery is earned and searchable.** A text search plus filter chips
  (All / Can make / New / ★ Favorites — favorites persist with the save)
  and station chips (Any / Craft / Smelt / Alloy / Crush) narrow the
  list; partial-ingredient rows now carry the amber ~ mark.
- **One key in, one key out.** The rebindable inventory key (E) now
  closes every container/station screen — hand craft, workbench,
  furnace, chest, machine — matching Escape; the contract is a pure
  function tested across every screen variant. Enter fires the primary
  craft action exactly once, and only while the search box doesn't own
  the keyboard.
- **Proof:** the rebuilt `crafting_workbench` scene plus three new ones —
  `crafting_workbench_small` (drill-down at 640×420),
  `crafting_missing_ingredients` (owned/needed marks + the exact reason
  line), and `crafting_queue` (working / blocked-with-reason / queued) —
  all reviewed by image recognition at 0.96–0.97 confidence. 99/99
  visual scenes, 435 tests, smoke green, runtimes rebuilt. Era filter
  chip, substitutions, and queue pause are honestly deferred.

## 2026-09-03 — transactional crafting and a real queue (loop 351, N02)

- **Crafting cannot void or mint anymore.** All craft execution now flows
  through one transactional engine in `lf_game::crafting`: it validates
  every ingredient against real inventory counts, proves the output FITS
  before touching anything, then consumes and grants exactly. A blocked
  craft — short one log or one slot of room — consumes nothing and says
  precisely why ("missing log (need 3, have 1)", "no room — need space
  for 4 more (free 2)"). The old grant loop silently lost outputs past
  the 255-per-insert boundary; the new one batches with zero loss, and a
  regression test crafts 256 planks in one action to prove it.
- **Craft All is integer-safe.** `max_batches` computes the largest batch
  count the inventory supports right now — limited by every ingredient
  count AND by output room — and the button labels the number it will
  make. Rapid double-crafts share the same atomic path: each call
  re-verifies, so materials for one craft can only ever produce one.
- **The queue is real.** "Add to Queue" used to push into a list nothing
  consumed. Now one job completes per 1.25 seconds of play through the
  same engine, the queue strip shows the head's live state (working /
  blocked with the exact reason) with per-job cancel, and the documented
  rule is honest: enqueue reserves nothing, consumption happens only at
  completion, cancel is free. The queue persists across save/load in the
  same ClientSave shape, and a job whose recipe vanished (mod unloaded)
  is dropped with a message instead of spinning forever.
- **Proof:** eight new engine tests (exact consume/grant, both blocked
  paths leave the inventory untouched, >255 outputs, integer-safe
  batches, rapid double-craft, a registered mod recipe executing
  transactionally, reason copy) plus three client tests (queue status
  running/blocked, catalog lookup + unknown outputs, queue save
  round-trip). 433 tests total, smoke green, runtimes rebuilt. The
  visual workbench pass (modal layouts, queue-strip proofs, input
  recovery) is N03.

## 2026-09-03 — first-minute onboarding and the nightly-beta goal pack (loop 350, N01)

- **The first five minutes teach the game.** A persisted tutorial state
  machine (Move → Look → Gather → Craft → Build) advances only on real
  gameplay facts: 3 blocks of horizontal walking, 1.6 radians of camera
  travel, a natural-material drop (log/dirt/stone/sand) reaching the
  inventory, a hand-crafted output, and a solid-block placement — torches
  and flowers are not shelter, a picked-up sword is not mining, and
  out-of-order events never skip steps. Vertical fall does not complete
  "move"; wild camera swinging does not complete it either.
- **The HUD says what to do next.** A compact top-center tutorial card
  shows the verb, keycap chips drawn from the live keymap (rebind
  Inventory to O and the card says O), an n/5 step chip, and a click-✕
  dismiss; beneath it a pinned objective line tracks the first incomplete
  starter quest and its progress ("Punch a Tree · oak log 1/3"). Both are
  painted by shared painters used verbatim by the new vistest proofs, so
  the proof pixels are the in-game pixels. Prompts pause behind modal
  screens, never block input, and skip creative mode.
- **It persists honestly.** Tutorial state rides ClientSave with serde
  defaults (old saves load as a fresh Move tutorial; the legacy bincode
  shape migrates), a new world resets it, and Gameplay settings gained
  "Show first-minute hints" plus "Restart tutorial".
- **The nightly-beta goal pack landed** (`docs/NIGHTLY-BETA/`, 14
  documents: beta gates, HUD/crafting spec, seed diversity contract,
  castle/faction strategy, NPC moral-history model, asset bible, vision
  protocol, performance/hygiene rules, the N01–N24 job queue, data
  contracts, and the morning-report template) with the
  `xtask night-plan-check` validator and `make night-plan-check` target
  that keep it verifiable.
- **Proof:** 12 new tests (state-machine transitions, keymap-adaptive
  copy, serde round-trip, pinned-objective chain, rect non-collision at
  640×420/800×600/1280×800/1600×900) and two new visual scenes
  `hud_onboarding` (1280×800) and `hud_small_onboarding` (640×420,
  zero-overlap assertions), both reviewed by image recognition at 0.97+
  confidence. `cargo test --workspace` is 422/0, vistest is 96/96, smoke
  is green, and fresh runtimes were rebuilt.

## 2026-09-03 — real sound-effect bank via ElevenLabs (loop 349)

- **The game now sounds real.** All 33 sound events play generated MP3
  samples instead of procedural beeps: block break/place across the five
  material families, per-material footsteps, UI clicks, eating, hurt, XP,
  tree creak and crash — plus twelve events that were silent until now:
  entering water, bow release, an arrow sticking into terrain, the melee
  whoosh, flesh hits and creature deaths (melee and arrow paths), the
  mount dragon's roar, item pickups, crafting success, chest lids, the
  forge anvil, and the player's death sting.
- **The bank is generated, committed, and self-contained.**
  `tools/gen_sounds.py` (wrapped by `make sounds`) is the generator with
  the full 33-prompt manifest; it talks to the ElevenLabs Sound Effects
  API with a key from the environment only, and caches aggressively —
  files that exist are never regenerated. The MP3s live in
  `assets/sounds/` and are embedded into the binary via `include_bytes`,
  so a missing file is a compile error and the catalog cannot drift from
  the code. Total free-tier spend: 620 of 10,000 monthly characters.
- **Decode is defensive.** Samples decode through rodio/symphonia at
  boot, downmix to mono, trim head/tail padding with a peak-relative
  threshold (so percussive events don't feel laggy), reject near-silent
  files outright, and normalize to a common playing level. The original
  synthesizer stays as a deterministic fallback for any event whose
  sample is missing — and as the reference the tests hold to.
- **Quality was measured, not assumed.** Every generation's true
  peak/RMS was inspected; eight first-pass files (footsteps, ui click,
  glass place, arrow hit) came back near-silent. The pattern —
  "footstep/stomp" prompts master quietly, impact textures don't — was
  root-caused and exactly those prompts rewritten (the winning wooden
  step is a knuckle knock on a board) and regenerated.

## 2026-09-03 — colored light, fireplaces, and material torches (loop 348)

- **Block light is genuinely RGB.** The voxel BFS propagates red, green, and
  blue independently, loses one level per channel/cell, max-blends overlapping
  sources, crosses chunk borders, and smooths every channel at mesh corners.
  The engine packs RGB into previously unused bits of the existing `u32`
  vertex-light field, preserving the old `0xF0` full-sky encoding and adding
  no vertex bandwidth. Neutral skylight wins in daylight; colored sources tint
  dark spaces. Warm sources breathe with a subtle position-phased flicker.
- **Materials now determine the light.** Ordinary torches and lanterns are
  warm gold, lava and Ember Glowstone amber, Lumen blocks cyan, and radiation
  green. New craftable/placeable **Ember Torch**, **Lumen Torch**, and
  **Fireplace** blocks complete the registry → texture atlas → item/drop →
  crafting → server-validation pipeline. Their procedural pixel textures read
  as an ember crystal, cyan crystal, and stone/log hearth rather than recolored
  copies.
- **Indoor lighting is reliable.** The visual proof exposed that the old
  combined skylight/source scan stopped at the first roof, so no enclosed
  emitter was ever discovered. Splitting source discovery from the sky pour
  fixed it. A second regression proved the skylight flood had been seeded on
  the opaque roof itself; it now starts at the lowest transparent cell above
  it, eliminating roof leaks while keeping overhang and shaft spill. Direct
  chunk-section scanning skips irrelevant sections and avoids per-voxel hash
  lookups.
- **Proof:** CPU tests cover packing compatibility, palettes, attenuation,
  blending, cross-chunk travel, sealed rooms, and the corrected sky frontier;
  a direct GPU test proves three independent packed channels reach the shader.
  The new `colored_light_room` scene pixel-checks warm, cyan, and green pools.
  `cargo test --workspace` is 406/0, all 94/94 visual scenes pass, smoke is
  green, and the warm readback+PNG benchmark is p50 53.7ms / p95 58.0ms.

## 2026-09-02 — packed normal/AO materials and hero textures (loop 346)

- **Terrain has authored character without abandoning voxel art.** Stone now
  reads as mineral plates with fissures and quartz, dirt as clumps with pores
  and pebbles, sand as wind-rippled grains, planks as joined boards with grain
  and knots, and coal/iron as connected veins instead of scattered noise.
  Grass gained clustered blades on top and roots/soil clumps on its sides.
- **Normal maps and ambient occlusion are one reliable material contract.**
  Each linear RGBA material layer stores a tangent-space normal in RGB and
  micro ambient occlusion in alpha. Procedural and mod textures get a
  deterministic Sobel/horizon-derived fallback; authored layers can enter
  through `new_with_material_maps`. Transparent edges do not grow fake bevels,
  AO has a readability floor, CTM derivatives are generated tile-by-tile, and
  normal mip levels are decoded, averaged, and renormalized instead of treating
  vectors as colors.
- **The renderer uses it cheaply.** The raster shader combines micro-AO with
  existing block-corner AO and lights the packed normal from the visible sun.
  Normal and AO share the previous normal-map lookup, so there is no additional
  atlas or texture fetch. Runtime layer replacement regenerates the complete
  material mip chain.
- **Proof:** four focused asset tests, a GPU test that independently proves
  authored normal and AO channels reach the fragment shader, and a new
  `material_gallery` raking-light scene with grass/sand/wood/iron/cavity pixel
  claims. `cargo test --workspace` is 387/0 and all 93/93 visual scenes pass.
  Warm A/B perf is p50 102.9ms for this build versus 104.0ms for the exact
  loop-345 source snapshot in the same readback+PNG harness (no median
  regression; p95 remains host-noisy).

## 2026-09-02 — kingdoms, walking NPCs, and the kingdom compass (loop 345)

- **NPCs actually walk now.** The old movement loop only committed a step
  when the next cell was air *and* the cell below it was solid — a one-block
  bump, a one-block dip, or any obstacle froze an NPC forever (the "some NPCs
  are not walking" bug), and hamlet villagers were additionally anchored at
  the world origin (8, 64, 8) by the default schedule instead of their
  hamlet. The new `lf_npc::locomotion` module owns real traversal: one-block
  step-up with head clearance, descent of up to 3 blocks, cliff refusal,
  accelerating gravity that cannot tunnel through ledges, and a stuck-reflex
  that sidesteps around walls/trees after 20 blocked ticks (per-NPC side
  bias so walls get rounded instead of ping-ponged). `update_villagers`
  drives it; idle NPCs shuffle around home, guards patrol a four-post
  circuit at dusk, and panicking NPCs flee directly away from the player.
- **Kingdoms exist.** One deterministic kingdom per 12x12-chunk region
  (hash-ordered candidate chunks on flat eligible grassland, named from a
  16-realm pool — `Kingdom of Elderfall`, `Thornmere`, `Goldhelm`, ...),
  generated as a full citadel: crenellated curtain walls with four torch
  towers, a gated south wall flying royal banners, a two-storey keep whose
  THRONE is the settle marker, two houses with hearths, a stone-ringed
  well, market stalls with a stock chest, and an irrigated farm plot.
  Three new blocks flow the whole registry→atlas→items pipeline (THRONE,
  BANNER_KINGDOM, KINGDOM_BRICK) and are mineable/lootable. First sight of
  a throne settles a six-NPC court — the new **Monarch** job (Queen Ilsa,
  royal trades: statuary and blueprints for ingots and books), two wall
  guards, a farmer, trader, and smith, all homed at the citadel — records
  the kingdom in the persisted save, writes a chronicle entry, and crowns
  it on the world map.
- **The Kingdom Compass.** Craft one from any wood block over an iron
  ingot (any trunk species or planks). Held, it draws a gold-rimmed compass
  dial under the crosshair whose red needle swings toward the nearest
  kingdom with the realm's name and distance — deterministic from the seed,
  so it works from the moment you spawn.
- **Proof**: `npc_walkers` ticks the real locomotion across a step lane, a
  dug lane, and a flat lane (arrival asserts) and claims each walker's
  pixels; `kingdom_citadel` plants the citadel and claims royal gold,
  purple banners/throne, and the ashlar wall band; `kingdom_compass_hud`
  renders through the real client paint function and claims case, rim,
  needle, and needle direction. 92/92 vistest scenes green.

## 2026-09-02 — clear sky + sun-tracked voxel lighting (loop 344)

- **The sun is visible again**: new authored 16x16 pixel-art sun, crescent
  moon, and star atlas layers render on celestial quads that bypass terrain
  distance fog and color grading. Terrain and clouds can still occlude them,
  but the performance fog can no longer erase the unreachable sky.
- **Light follows what players see**: raster face/normal-map relief now reads
  the same `sun_direction(time)` vector used to place the sun, so the brighter
  side of blocks moves from east to west during the day without adding a
  shadow-map pass or distant geometry cost. The Live RT path keeps its real
  cast shadows.
- **Night timing fixed**: stars are emitted while the sun is below the horizon,
  correcting the previous inverted condition that produced stars at noon.
- **Proof**: `sun_visibility` renders the authored sun at 420 blocks while
  terrain fog ends at 48; a GPU regression proves east and west sun positions
  materially change more than 500 terrain pixels. The complete harness is
  89/89 and the workspace is 371/371 tests green.

## 2026-09-01 — HUD pass finished: kit everywhere + building HUD (loop 343)

- **Zero alpha chrome left**: the 13 machine windows, trade, companion
  menu, tech tree, lore book, and smithing forge all converted to the
  design-kit panel shell through a shared `kit_shell` helper (vignette +
  dark wash + framed panel + title + scroll) — the whole UI now speaks
  the workbench/settings/inventory design language.
- **Building HUD**: a strip floats above the hotbar while you hold a
  block (or while symmetry is live) with placement-shape chips — pick
  BLOCK / SLAB / STAIRS by clicking or pressing R, and any held solid
  block places as a bottom slab or a yaw-facing staircase (slab onto
  matching slab still merges into a full cube). The symmetry chip shows
  the live mirror plane; click it or press V to toggle. The world-space
  symmetry wall and ghost overlays render as before.
- **Proof**: `build_hud` vistest scene with chip-rect pixel claims
  (accent-selected chip, olive symmetry chip, dark unselected chips) and
  an lf_game test locking the shape math (slab bottom, all four stair
  facings, air/water refusal, cycle order, slab merge).

# CHANGELOG

## 2026-09-01 — missing texture patterns: bark + soil (loop 342)

- **Every log species now has bark**: a flatness audit (luminance stddev
  across the whole generated atlas) found eight logs rendered as pure
  noise next to palm/redwood/ember's structured bark. Oak gained grain
  streaks, spruce scaly chips, dark wood deep furrows, cherry horizontal
  lenticels, acacia exfoliating plates, mangrove fibrous strands, maple
  pale strips, baobab wide smooth bands — all in their existing palettes.
- **Soil clumps**: dirt gets chunky clumps and rare pale pebbles, red
  sand wind ripples. The remaining "flat" textures are the authentic ones
  (water, snow, sand, stained glass, waypoint beams).
- **Regression-locked**: `bark_and_soil_keep_their_patterns` enforces
  variance floors per texture, so a future generator change that
  flattens bark back to noise fails the test suite.

# CHANGELOG

## 2026-09-01 — HUD declutter, inventory-first E screen, kit restyles (loop 341)

- **Minimal HUD by default** (researched Minecraft convention: the
  survival screen shows nothing until F3): the top-left info line is now
  clock + facing; biome, coordinates, weather, net, FPS, and RT moved
  behind the F3 debug toggle.
- **E opens an inventory screen**, not a crafting list: armor column
  (head/chest/legs/feet + off hand) beside a painted kit-block player
  portrait, the 3×9 storage grid, the hotbar with its selection frame,
  shift-click quick-move and right-click split — plus a "craft by hand"
  route into the basic workbench (full recipes stay at crafting tables).
- **Furnace and chest wear the design kit**: the last two alpha-chrome
  container screens converted to the vignette + framed-panel shell with
  proper titles, matching the workbench/settings/journal family.
- **Proof**: `inventory_screen` vistest scene (mirrored preview; pixel
  claims on the slot-well grid, the kit-accent portrait/selection, and
  the title band) and the mirrored HUD info line synced with the
  declutter.

# CHANGELOG

## 2026-09-01 — GMod-style physics item drops (loop 340)

- **Mined blocks become props**: breaking a block (or harvesting a crop,
  looting a chest, felling a tree) spawns a rigid item prop with gravity,
  axis-separated collision, and restitution — it bounces off the floor
  AND walls, tumbles while it moves, slides a couple of blocks GMod-style,
  settles onto its nearest flat face, and sleeps. The old 2-block magnet
  vacuum is gone: items now litter the floor until you walk into them.
- **Carry at range**: hold right-click while aiming at a prop (up to 6
  blocks, walls block the grab) to pin it to your view ray with a soft
  spring — swing the camera and it follows; release to throw it with its
  momentum. Walk right up to a carried or resting prop to pocket it.
- **Stacks grow**: same-item props resting within touch distance merge up
  to five per stack, and the rendered cube grows with the count until a
  full 5-stack is exactly one block wide (non-block items scale their
  sprite impostors by the same rule).
- **Physics lives in lf_game::props** (`PropBody`, `step_prop`,
  `prop_half`, `merged_counts`) with unit tests: fall/bounce/rest on the
  block top, fast throws rebound off walls while slow pushes stop touching
  them, held props freeze and sleeping props skip the step.
- **Proof**: the `item_physics` vistest scene runs the real physics (three
  stacks stepped 600 ticks to sleep at sizes 1/3/5, a prop caught
  mid-fall after 14 ticks, a thrown prop slid into a wall) with pixel
  claims — the three ground silhouettes must strictly grow, one cube must
  be airborne above the ground line, and the wall must stand tall. 365
  tests green; 86/86 vistest scenes; smoke green.

## 2026-09-01 — mob animations: legs, hurt flashes, death topples (loop 339)

- **Animals walk like animals**: chicken/wolf/dog/bear — and the formerly
  single-cube boar and woolbeast — are articulated multi-part bodies whose
  legs swing in diagonal trot pairs around hip pivots, driven by a
  distance-based walk cycle (`gait_phase` advances with speed so legs never
  moonwalk; `gait_amp` eases in/out so strides start and stop) and the whole
  assembly yaws to the mob's facing (rate-limited turns instead of snaps).
  Wolves wag their tails at rest, woolbeasts lower their heads to graze,
  chickens peck while idle.
- **Hits read as hits**: every mob skin (21 layers incl. biome tints) got a
  red-multiplied hurt copy appended to the atlas; while `hurt_flash` decays
  the renderer flickers the mob onto the hurt layer plus a flinch squash.
- **Deaths are visible**: mobs topple over (~0.5s ease-out fall around the
  feet), rest ~1s as physics corpses (gravity + friction, no AI), then are
  removed — loot still pops out at the kill. Nameless raiders now walk as
  six-part humanoids and topple like the villagers; cube mobs tumble their
  single cube. Fixed two proof-found pre-existing bugs: firebolt kills
  never removed the mob (immortal corpse), and mobs that fell below y=-10
  ticked forever.
- **NPCs face where they go**: villagers turn smoothly toward their
  walking direction (`yaw`/`walk_phase` on `Villager`, replacing the
  id-hash fake facing), and remote players' gaits are estimated from
  position deltas so they visibly walk.
- **Engine primitives**: `cuboid_part_faces` (the extracted
  yaw+pitch-around-pivot cuboid the humanoid builder uses, now shared by
  animals; bit-for-bit refactor proven by test) and `topple_faces`
  (Rodrigues rotation of assembled part faces around a world pivot — the
  vistest scene caught it initially rotating around the world origin and
  teleporting corpses away; fixed with a non-zero-pivot test).
- **Proof**: `mob_anim` (four wolves at stride phases 0/90/180/270° on a
  sand stage — the pixel claim requires their silhouettes to differ in
  width, a frozen-leg detector) and `mob_hurt_death` (red-tint count,
  toppled-corpse-low / fallen-raider-low / standing-raider-tall windows).
  360 tests green; 85/85 vistest scenes; smoke green.

## 2026-09-02 — authored-depth raster assets, articulated NPCs, item impostors (loop 338)

- **Normal-mapped raster materials**: every generated base, mod, entity,
  item, and connected-texture layer now gets a linear RGB tangent-space
  normal map. The default raster shader reconstructs a tangent frame and
  applies cheap directional relief, so grooves, pixels, and bevels respond
  to viewable light without requiring the optional path tracer.
- **Characters are people, not blocks**: seven villager-job outfits and a
  neutral remote-player skin feed a shared six-part humanoid builder. NPCs,
  companions, and network players now have heads, torsos, independent arms
  and legs, yaw, walking gait, and crouch posture.
- **World items use their real art**: every registered item sprite is in the
  scene atlas; non-block drops render as crossed, double-sided alpha-cutout
  cards while block drops keep their compact cube silhouette.
- **Proof-found fixes**: CTM sentinel indices formerly began at 165 and
  collided with real tree/biome/skin atlas layers; they now occupy 4096+.
  The client entity helper also ignored requested cube positions, collapsing
  entity parts at the origin; it now translates every face correctly.
- **Plan + proof**: added `docs/ASSET-RENDERING-PLAN.md`, a five-stage path
  through per-part skins, attachments, hero item meshes, authored material
  channels, cheap contact/projected shadows, and LOD/performance budgets.
  `entity_skins` is a close lineup of eight articulated characters and eight
  readable item silhouettes with pixel assertions.
- **Verification**: workspace build green; 353 tests passed; 83/83 GPU proof
  scenes passed after correcting the CTM assertion to sample its projected
  edge; manual PNG inspection clean; smoke green; `terrain_vista` benchmark
  p50 50.2ms / p95 50.6ms (29 warm frames, ~20 FPS).

## 2026-08-30 — smart HUD, personalized font, Minecraft controls (loop 337)
- **Smart HUD (never overlaps)**: `kit::hud_layout(w, h)` is the single
  pure source of HUD geometry — info line capped away from the minimap,
  companion tiles ending above the chat band, chat above the hotbar
  band — with a disjointness test proving zero overlap at 640x360,
  800x600, 1280x720 and 1920x1080. The live HUD regions (chat, companion
  tiles, info-line width, minimap anchor) are re-anchored to the computed
  layout, so window size can no longer produce overlapping widgets.
- **Personalized font**: `kit::install_font` promotes the embedded Hack
  monospace over the entire UI (proportional + monospace families,
  1.06 scale, baseline nudge) — a chunky, technical LOREFORGE voice —
  installed once per session to keep the glyph atlas stable.
- **Minecraft controls**: SHIFT sprints, CTRL crouches (FlyDown moved to
  CTRL too), and **crouching edge-locks**: while sneaking on the ground,
  per-axis movement that would leave the supporting block is cancelled —
  you can hold the edge all day and never fall. Sneaking lowers the eye
  by 0.28, slows to 45%, and sprinting runs at 5.6 vs 4.3 walk.
- **Tests**: crouch edge-lock (sneaker holds a floating ledge 600 ticks;
  a non-sneaking walker falls off — proving the platform test real),
  sneak/sprint speed ratios, keymap defaults, HUD disjointness at four
  window sizes. 349 tests, hud_small vistest scene, smoke green.

## 2026-08-29 — king-quest: 50 mods, 15 biomes, animals, the Accord Bastion, vassal workers (loop 334)
- **50 community mods** (`mods/`): ores & metals, food & farming, magic,
  building & decoration packs — 88 blocks (with worldgen ore veins and
  light emitters), 79 items including tools with damage/durability, 10
  smelting recipes. A load-all contract test pins parsing, unique fnv1a
  block ids, and the ore/light/smelting minimums.
- **15 new biomes**: Oasis, Redwood Forest, Mangrove, Aspen Grove, Baobab
  Fields, Willow Wetlands, Painted Dunes, Frost Meadow, Emberwood,
  Lavender Fields, Maple Forest, Pine Barrens, Salt Flats, Foggy Fjord,
  Sunflower Plains — 18 new blocks (logs/leaves/plants/salt) with their
  own atlas art, **9 new tree species**, per-biome ground cover, and
  climate-grid classification (variant-channel splits, reachability-
  tested). Structure gates extended so the new biomes spawn huts,
  embassies, farms and roads.
- **Animals**: chickens (temperate days), wolves (cold nights), bears
  (deep forests), dogs (settlements) — multi-part cube layouts with a
  walk wobble, four new skins, own spawn rules and combat behaviour.
- **The Accord Bastion**: a walled mini-city in the accord meadowlands —
  perimeter walls with merlons, a south gate, four timber houses, a
  two-storey stone keep flying the Accord banner (NPCs settle it), accord
  pillars and torch-lit roads — plus frontier wooden watchtowers in the
  new forests and sun-baked ruins in the new deserts, each biome-gated
  (a biome may or may not carry its structure, by seeded hash).
- **The Vassal system**: at Honored (+75) standing, sneak-use a villager
  to swear them in — Smiths/Lorekeepers mine, Guards/Traders/Bards
  lumber, Farmers farm. Each in-game day vassals stack the yield for
  their liege; sneak-use again to collect. State rides the villager JSON
  save, all rules pure and deterministic.
- **Steam (honest pass)**: Workshop items installed in `workshop/` now
  load in the client and dedicated server through the same mod pipeline
  (`lf_steam::workshop` has real consumers); `lf_steam --features steam`
  compiles. Steam P2P/achievements stay deferred — no Steam client, SDK
  runtime or real AppID exists on this host (dev AppID is Valve's 480).
- **Suite**: 344 tests (was 338), full vistest suite, smoke green.

## 2026-08-29 — the in-game black screen: root cause found and fixed (loop 333)
- **The bug**: after starting a single-player game the view showed a giant
  static black rectangle over the world (HUD still drew on top). Reproduced
  live with a new `loreforge --autostart` debug harness (boots straight
  into a fresh world through the exact menu code path) and macOS screen
  captures, then bisected the frame with draw toggles and instrumented
  batch geometry: the drop_batch carried an entity cube near the world
  origin rendered with `MeshBatch::new`'s **identity view_proj** — six
  per-frame batches (sky/cloud/weather/drop/crack/particle) never got
  `update_camera` in the live render loop, so any geometry within ±1 unit
  of the origin (right where the player spawns) landed inside the clip
  volume and filled the screen with black. The same bug made the sun,
  moon, stars, clouds, item drops, mob cubes, crack decals and particles
  invisible in live play (they only ever rendered in headless vistest,
  which updates all cameras — why proofs never caught it).
- **The fix**: `update_camera` for all six batches every frame. Verified
  with real screen captures at t=15s and t=23s of a fresh world: fully
  rendered terrain, river, sky and HUD, no black rectangle — and the
  sky bodies/clouds/drops now actually visible in live play.

## 2026-08-28 — ai-npc-assets: mob AI state machine, living NPCs, connected textures, generator (loop 332, Sections A-G)
- **Mob AI (B)**: mobs got a real behaviour machine — Idle / Wander /
  Chase / Attack / Flee / Investigate / Disengage with all 11 spec
  transitions (lost-sight investigation, 30s disengage, flee-while-seen).
  DDA line-of-sight (cached per tick, 32-block cap) gates aggro and flee;
  faction standing widens or calms the aggro radius (+100 standing =
  ignore unless attacked); group aggro recruits nearby same-type mobs
  with a 0.5s reaction delay (first-order only, ≤5 pack); A*
  pathfinding (`lf_game::mob_pathfind`, cardinal + 1-up steps, 256-node
  cap, 2s cache) drives Chase/Investigate with direct-steer fallback.
  Client ticks mobs with the player's faction standing and propagates
  group pings. En route: fixed a pre-existing engine bug — axis-aligned
  rays from block-boundary origins produced NaN in the DDA and stopped
  after one cell (rays were blind along exactly the lines mobs shoot).
- **NPC behaviour (C)**: the canonical enriched day (sleep / eat / work /
  socialize / return-home) now drives movement, an `NpcActivityState`
  that bends the render pose (sleeping lies low, working bobs), and
  dialogue posture (sleeping NPCs only murmur; no trade). Reactions:
  structure damage call-outs, combat panic (flee 10s), gifts (use an
  item on a villager: +2 standing, thanks, memory), companion-quit
  comments, and a once-per-crossing "+75 honoured" acknowledgement. NPCs
  remember the last two interactions per world (5-day window) and open
  with memory lines; trades and completed quests write the memory.
- **Black-square hardening (A)**: Live-RT pathtracer + displayed image
  are invalidated on world transitions (a stale frame of the previous
  world covered the new one), and empty column meshes never register a
  draw batch. `no_black_square` scene + pure-black run-length assertion
  on eight daytime gameplay scenes.
- **Testing (D)**: `loreforge --smoke` runs 300 headless logic ticks
  (superflat worldgen seed 42, passive+hostile mob AI, NPC schedule, a
  planks craft, a block mine) with exit-code + log-pattern checks;
  `make smoke` = logic smoke + 12s GUI liveness. New scenes:
  `mob_ai_visible` (real 120-tick mob sim, moved-distance claim),
  `npc_schedule_time` (midday = Work slot), `no_black_square`,
  `connected_textures_grass_3x3`.
- **Connected textures (E)**: top faces of grass, sand, water, snow,
  bog peat, permafrost, accord stone and ashen marble pick one of 47
  strip tiles by an 8-neighbour bitmask (standard CTM corner rule,
  derived table), so fields read as one surface with edge definition.
  The strips live in a second 192×512 texture bound alongside the atlas;
  the mesher bakes per-tile UVs behind marker layer ids and the shader
  reroutes them. Proofs: `connected_texture_uv_3x3` (centre of a 3×3 =
  interior tile, isolated block = bordered tile) + a vistest scene with
  a dark-ring pixel claim.
- **Asset generator (F)**: `xtask gen-texture` (grass/stone CTM strips,
  faction entity skins, block noise — seeded xorshift64 + integer hash
  noise, bit-identical per seed), `gen-ctm <block>`, `gen-all-textures`
  (skips existing). `asset_generator_grass_output` pins seed-42
  determinism and the no-pure-black/white pixel-art rule.
- **Suite**: 338 tests (was 322), 82/82 vistest scenes, headless+GUI
  smoke green.

## 2026-08-28 — plants render as crosses + seed-field contract + opaque surface (loop 331)
- **Ground plants are Minecraft-style crosses**: flower / tall grass / dry
  grass / dead shrub render as two diagonal cutout quads (each emitted
  twice so backface culling keeps both sides), lit by their own cell with
  foliage wind sway — no more solid green cubes. Proof `plants_cross`
  pixel-claims plant colors AND sky visible above the band (a cube would
  block it); mesher test pins 16 verts / 24 indices inside the cell.
- **Seed field contract**: the Create-a-Game seed field is now one tested
  helper (`slots::parse_seed_field`): trimmed number = literal, empty =
  fresh random, any other text = deterministic hash (same words → same
  world forever). World-level proof: `seed_comparison` renders half a
  scene from seed A and half from seed B through the same generator and
  fails if the halves look alike; `same_seed_same_world_...` pins
  reproducibility.
- **Black-box fix (compositor)**: the surface now requests
  `CompositeAlphaMode::Opaque` — with a premultiplied/inherited mode the
  desktop behind the window blended through pixels whose alpha was not 1
  (water, ice, unlit regions), which read as a dark box while playing.

## 2026-08-28 — timber: Valheim tree felling + deep falling blocks (loop 330, master-plan Phase A)
- **Tree felling**: chopping a trunk now fells the whole tree. The new pure
  `lf_game::timber` system identifies the standing tree from the break
  (`find_tree`: same-species trunk column, canopy scan, 24-block cap, lone
  placed logs never fall), removes trunk + canopy (every cell broadcast),
  and animates a rigid fall — angular acceleration around the stump hinge,
  rendered as rotated cubes via the shared `tree_parts` layout fn (the
  dragon_parts idiom; client + proofs run the same code). On impact the
  `fall_plan` lands the trunk as **horizontal log blocks** along the fall
  direction (cells blocked by terrain convert to drops), the canopy
  shatters into debris, a crash sound plays and the camera shakes with
  tree size. Two new procedural sounds: TreeCreak (wobbling low creak) on
  the tip and TreeCrash (noise burst over a 70 Hz thud) on impact.
- **Horizontal logs**: 10 new vanilla blocks (ids 111-120: X/Z variants of
  the five trunk species). The mesher's `Face` enum gained directional
  variants (West/East/North/South) so a lying log's cut ends show the
  ring texture along its axis — all other faces stay bark. Drops map back
  to the species' log item.
- **Bug fix found on the way**: birch/spruce/dark/cherry logs had no items —
  breaking them fell through to the stone fallback. Each species now drops
  its own log item and saws into planks.
- **Deep falling-block animation**: granular fallers tumble while falling
  (deterministic per-block axis + angular velocity, fibonacci-hashed from
  the cell), fast first impacts bounce once (restitution 0.18) with an
  impact dust puff, then settle. Physics stays the cheap scalar drop;
  rotation is render-only CPU quads in the existing per-frame batch.
  Perf bench after the change: terrain_vista p50 116.8 ms / p95 158.6 ms
  (baseline 111/156 — within run variance; the rotated path only runs
  while fallers/trees are airborne).
- **Proofs**: tree_fall_mid (oak caught mid-rotation at a seeded angle;
  the tree_fall_animates_between_frames GPU test renders two angles and
  demands pixel differences with a deterministic same-angle control),
  tree_fall_landed (the real fall_plan applied headless: horizontal log
  row with visible ring ends), falling_blocks_deep (three independently
  tumbling cubes). All with pixel claims (bark/ring/leaf counts).

## 2026-08-28 — assets-and-menus: complete asset set, centered UIs, creative mode (loop 329)
- Menus centered, everywhere: the New World and Multiplayer panels were
  anchored top-left (a fresh top_down egui cursor starts at 0,0) — now both
  sit on `ui_kit::centered_panel_rect` (clamped, both axes); pause, settings,
  Load World, spellbook, imbue, carve and paths vertically center via
  `ui_kit::center_vertically` (the pause screen's fixed 18% guess is gone).
  `ui_kit::apply_kit_style` pushes the LOREFORGE palette into egui's global
  style so every plain widget and `egui::Window` screen (trade, furnace,
  chest, tech tree, machine panels, smithing) wears the kit instead of
  egui's cool-blue defaults.
- Quest log redesigned: the raw default-styled window pinned top-left is now
  a centered Journal panel with tabs (Quests n / Chronicle), faction chips,
  per-objective progress bars, standing rewards and completion tinting.
- Multiplayer screen developed: styled host-world rows (selection accent,
  seed readouts, empty-state line), clearer sections, honest friends copy.
- Window-size robustness is a proven contract: `centered_panel_rect` is
  unit-tested (symmetry + clamping on small screens) and three new vistest
  scenes render a real-helper menu panel at 640x420, 800x600 and 1280x800
  with a largest-connected-component pixel claim that the panel bounding box
  is symmetric in both axes at every size (menus_centered_small /
  menus_centered / menus_centered_wide).
- Asset completion: the armor set is real — bronze/steel helmet, leggings
  and boots (items, pixel-art icons, Bronze-era recipes, research gates);
  the inventory's four trailing armor slots are honored (worn_armor_points
  sums 36..=39; bronze kit 10 pts, steel 17) and drawn as a labeled
  head/chest/legs/feet row with a live armor readout in the workbench strip.
- Audio completion: new procedural Sfx set (ui click, eat crunch, hurt
  thud, xp chime, per-material footsteps) in lf_audio with bounded/decaying/
  distinct synth tests; wired live — screen transitions click, eating
  crunches, damage thuds, level-ups chime, ground travel steps by material.
- Asset testing: every registered item must generate real art and every
  icon pair must differ (registry-derived tests, not a hand-list); the new
  asset_catalog vistest scene renders ALL ~190 registered item icons through
  the real ItemIcons with a per-cell non-uniform pixel claim.
- Creative mode plays like creative: no damage, no hunger drain, infinite
  items (consume_selected no-ops), instant mining, F toggles flight
  (survival lost the old ungated debug fly) — five pure gates on GameMode
  with tests; the New World screen note now describes the real behavior.
- Journal/workbench/new-world/multiplayer proof replicas now draw through
  the client kit's real layout helper + style, and the workbench replica
  mirrors the client's vignette+wash treatment (judge-flagged contrast).

## 2026-08-27 — ui-world-craft: title identity, world creation, terrain rivers, workbench (loop 328)
- Title screen rebuilt on the LOREFORGE palette (parchment/ember/iron-brown
  across every screen via the ui_kit Theme): logotype top-left with
  "Build. Rule. Endure.", underline-on-hover link column, radial vignette,
  version + preview-seed display bottom-right. The title world is now
  seeded from the game version (Fibonacci-hash mix — every release shows a
  different recognizable world) and orbits on a 90s elliptical path with a
  57.3s altitude oscillation; the preview generates in memory and never
  writes to worlds/.
- World creation flow: New World screen (name, visible seed with reroll,
  world type / game mode / difficulty segment toggles, Back + Create
  World), Load World picker with seed-rendered cached thumbnails,
  world-type glyphs, difficulty + last-played metadata and a real delete
  confirmation; Multiplayer screen with working Direct Connect, Host World
  (dedicated server spawn) and an honest Steam-lobby stub. Worlds save
  difficulty (Peaceful blocks hostile spawns; Easy/Normal/Hard scale mob
  damage and hunger pace), game mode (Creative saved as a flag) and
  creation metadata; old slot metas upgrade in place.
- Terrain rework (gen v4): two-layer continental terrain — flat lowlands
  hugging the sea, ridged highlands, ocean shelf — measured at a 0.409
  mean flat fraction across 5 seeds vs 0.275 mountains. Rivers meander
  through the whole lowland (OpenSimplex2 zero-crossings, hard highland
  cutoff, 3-7 blocks wide widening toward the coast, carved to a bed that
  actually fills with water). Caves: surface-breach ramp, deep-slate biome
  below y=30, lava lakes below y=10, stalactites/stalagmites. Structures
  (huts to faction camps) adapt to terrain: no floating floors, filled
  platforms on slopes, underwater sites refused.
- Biome identity: every biome scatters its own ground cover at its own
  density (tall grass/flower meadows, flower-dominant forests, cactus and
  dead shrubs in the deserts, dry savanna grass, mushroom/volcanic
  signatures) with five new decoration blocks + lava; transition bands
  interleave both biomes' covers via the climate dither.
- Crafting workbench: the 3x3 grid is gone. A three-zone workbench
  (category sidebar with craftable counts, recipe list sorted
  craftable-first with locked rows hidden, detail panel with flavor text,
  real have/need counts, batch quantity and Craft / Add to Queue) opens
  from the inventory (basics) or a crafting table (everything). Recipes
  are EARNED: a base survival set is visible from minute one, era-tagged
  recipes surface when the era lands, and picking up any ingredient for
  the first time unlocks its recipes with an amber toast. The set
  persists in the save.

## 2026-08-27 — lore-and-visuals: factions, companions, skins (loop 327)
- Six-faction standing system (–100..+100, Nameless start –50) loaded from
  lore/factions.toml; threshold titles, rivals-drift on honored crossings,
  standing events (quest ±, trade, structure destroy/discover) all
  data-driven via the new lf_lore crate. Chronicle entries reference the
  canonical world events (Era/Year dates from lore/world_events.toml).
- 12 faction quests (2 per faction) from lore/quests_factions.toml with
  narrative text from the faction docs; new quest mechanics: tagged Reach
  (road markers / ember formations / new biomes), Break, Place, Interact,
  and any-food Collect. Quest log shows them; faction NPCs grant them on
  first contact.
- Hireable companions: hire at standing ≥75 with a fee (trade UI), up to
  3 active; trust (0-100) and morale (0-100) with the documented event
  table, daily wages paid at sunrise (unpaid → morale loss → quit at 0,
  "word gets around" −5 faction), dismiss returns them to their schedule
  with trust remembered. Command menu on interact (follow/stay/rest/mine/
  chop/haul/guard/pay/dismiss); 2-4 block follow AI that defends the
  player against the mob that hit them; contextual dialogue lines from
  lore/dialogue.toml; trust badge on the skin at ≥50.
- 38 new blocks (12 faction-themed, 8 biome-exclusive, 18 decoration
  including 8 stained glasses and 6 banners) with procedural 16x16
  textures, drops, and 24 recipes; MOD_BLOCK_BASE 100→200 (DECISIONS).
- Volcanic biome (31st) + biome surface identity updates (gilded savanna
  grass, bog peat, permafrost tundra, mesa terracotta badlands), deep
  slate depth band, coral heads, natural ember-glowstone formations,
  accord road markers.
- Six faction structures (embassy, forge camp, grove shrine, longhouse,
  library, nameless camp) placed deterministically in their home biomes;
  banner markers settle the faction's NPCs (incl. named The Unmarked,
  Maren Voss, Dag Holtz) and drive discovery chronicle + map icons.
- Entity visual identity: 6 faction villager skins + 2 named NPC skins,
  6 companion skins with trust-badge variants, 6 mob skins with distinct
  silhouettes, 9 biome-tint variants of the common hostiles, Nameless
  Raider mob (food+loot drops).
- Map: faction territory tint (30% blend over terrain shading) on the
  minimap and world map + faction-color structure icons. HUD: faction
  standing widget (bottom-right, pulse on change) + companion status
  tiles (top-left). Trade gates: hostile standing refuses trade,
  friendly gets a 10% discount. Ambient ember particles for
  ember_glowstone. 13 new vistest scenes, all pixel-verified.


## 2026-08-26 — P27: fix "objects disappear when looking up" (frustum culling, loop 307)
- The chunk-column frustum test approximated each 16x16xH column with a
  sphere of radius `max(half_h, 11.4)`. That covers the footprint only
  along its axes — the true corner distance is sqrt(128 + half_h^2)
  (~13.6 flat ground, ~17.7 for a 20-tall column), so when the bottom
  frustum plane swept up with the view, columns still poking into the
  frame were wrongly culled: terrain and objects vanished as pitch rose
  (and tall columns vanished even near level pitch). The raycast and FOV
  were innocent: pitch is clamped to 89 deg and the look_at basis stays
  non-degenerate.
- Fix: exact AABB bounding sphere with a 0.1 sway margin (wind-animated
  leaves never culled at the margin), and the Gribb-Hartmann frustum
  planes are normalized before the distance test so the world-unit radius
  means what it says (the raw near-plane normal is ~2x unit length).
- Regression test `looking_up_does_not_cull_visible_columns`: pitches
  5-85 deg x four eye heights x a column grid x five column heights — any
  AABB corner projecting inside the frustum requires the column kept —
  plus the pinned pre-fix failure (pitch 5 deg, tall column at the frame
  edge). Verified the test fails against the old formula.

## 2026-08-26 — P26: visual identity — per-face materials, cutout leaves + wind, smooth AO, mining cracks/particles, mipmaps (loop 306)
- Per-face materials: meshing's texture callback is now
  `(BlockState, Face::{Top, Bottom, Side})`. Grass finally renders a green
  top, banded side and dirt bottom (it previously painted a fake green band
  on all six faces); every log species gets growth-ring end textures. New
  atlas layers (grass_top, log_top, crack_0..3) bring the array to 48.
- Alpha cutout: the fragment shader discards alpha < 0.5 and the six leaf
  textures are deterministically hole-punched per species. Foliage is now
  see-through with reliable depth writes — water (0.67) and ice (0.78) sit
  above the threshold and are unaffected; the glass pane becomes a
  frame-only cutout, Minecraft-style.
- Wind: vertices carry a sway weight (leaf family = 1.0) and Env.time drives
  a vertex-shader wave whose phase derives from world position, so animation
  is continuous across chunk borders and stable while moving. Frozen when
  the particles setting is off (low quality tier).
- Smooth lighting: per-vertex ambient occlusion from the classic
  side/side/corner rule and per-corner light averaging over the four cells
  touching each corner (both were flat per-face before). get_block now
  handles diagonal cross-section lookups safely (approximates as air) —
  corner sampling used to overflow section indexing.
- Mining feedback: a stage 0..3 crack decal (slightly inflated cutout cube
  on the targeted block) plus debris particles — small camera-facing
  billboards sampling the block's texture, with gravity, a simple ground
  stop and a 128-particle cap. The subtle HUD progress bar stays for
  accessibility.
- Mipmaps: a 5-level CPU box-filtered chain per atlas layer with
  mag-nearest / min-linear+mipmap-linear sampling; distance shimmer is gone
  without losing the pixel-art look up close.
- New proof scenes: `foliage_canopy` (cutout + AO + log rings close-up) and
  `mining_feedback` (crack decal + debris on a stone column). The mining
  scene taught a lesson: frame scene cameras against the terrain AT the eye
  — a buried camera sees straight through backfaces.

## 2026-08-26 — P25: correctness & honesty sweep + the pathtracer was flat-color broken (loop 305)
- Server SetBlock validates against the real registry now
  (`lf_voxel::registry::is_known_block`): vanilla ids <= 41 plus registered
  mod blocks (>= 100). The old `block <= 18` cap silently dropped every mod
  block edit in multiplayer. The dedicated server loads `mods/` at boot so
  the ids exist server-side; new UDP integration test covers accept/reject.
- `lf_steam/steam` compiles for real: steamworks 0.12 as an optional dep
  (`steam = ["dep:steamworks"]`), verified with
  `cargo check -p lf_steam --features steam`. STEAM.md corrected — CI
  builds default-feature binaries only. The feature-off default is untouched.
- Generator versioning: `lf_worldgen::GENERATOR_VERSION` + `genver.dat` per
  world (client slots and the dedicated server). A mismatch warns loudly:
  unedited chunks regenerate from the seed on revisit, edited chunks are
  always safe on disk. Pre-P25 worlds upgrade silently on first load.
- Lantern (block 13) got its own procedural atlas layer (41 -> 42 layers);
  it previously fell through to the stone texture.
- Root `[workspace.dependencies]` now lists only the deps actually used
  (21, correct versions incl. winit 0.30 / egui 0.31 / fastnoise-lite 1.1);
  the old table declared 14 deps nothing referenced.
- `lf_modapi::apply_mod` auto-registers `*_ore` blocks as worldgen veins
  (y 8..50, id-derived noise offset clear of vanilla) — mods/README now
  tells the truth. Stale BACKLOG entries corrected (P6 mobs/combat and the
  P3 sky line were marked undone but shipped and tested); tests/golden stub
  removed.
- **vistest PNGs are pixel-analyzed after rendering** (`verify_render`:
  >= 16 distinct colors, real luma variance, sane size) — enforced in code,
  not narrative. First run caught two real, long-standing pathtracer bugs:
  every raytraced scene since P18 (and Live RT / R-key captures in-game)
  rendered ONE flat color. (1) The WGSL DDA initialized `t_max` with a
  signed numerator divided by abs(dir) — negative for negative ray
  components — so that axis always won and rays marched off into the void
  (or straight down into the emissive floor). (2) The CPU camera basis was
  scaled by `camera.fovy.to_radians().tan()` — but `fovy` is already
  radians, so the basis came out at ~1.4% and every ray was parallel.
  Both fixed; raytraced_shadows shows real terrain with fog gradient
  (1697 colors), raytraced_night shows emissive glow over dark ground
  (294 colors).
- Server UDP tests use a deadline-based `drain_until` instead of fixed
  sleeps (chunk generation on first SetBlock made 200 ms pumps flaky under
  parallel test load).

## 2026-08-26 — P24: THE input fix — every key/mouse handler was unreachable (loop 304)
- Root cause (found with a synthetic-input harness driving the real binary
  with macOS keystrokes): a stray `_ => {}` wildcard arm sat in the middle
  of the `match event` in `window_event` — inserted back in P4 — so every
  handler after it (Focused, KeyboardInput, MouseInput, MouseWheel) was
  UNREACHABLE dead code. rustc only warns about unreachable patterns, and
  the warning was buried: keyboard and mouse input never worked through
  the event handler in ANY release; menus worked only because egui gets
  events through a different path. Fix: wildcard removed; lf_client now
  `#![deny(unreachable_patterns)]` so this class of bug cannot compile
  silently again.
- Verified empirically on the fixed build via synthetic input: E toggles
  inventory, holding W walks (position trace 1.2,-0.8 -> 5.1,-4.9), M
  toggles map, ` opens the console, a typed `fly` command executes, Esc
  returns to play, clicks reach the mining path.
- Kept (behind LOREFORGE_DEBUG_INPUT / F3): per-event input trace
  (event/ui_open/egui-consumed), 1Hz tick summary with frame_ms.

## 2026-08-26 — P23: urgent fixes — input, console, seeds, biomes, slots, scaling (loop 303)
- Input defenses: `close_ui` clears a stale chat input (an invisible Chat
  screen forced every frame would eat all keys and clicks); Escape closes
  the Pause menu; if the OS refuses the cursor grab the game still enters
  input mode (mouse-look via raw motion, clicks keep working); the click
  that re-captures the cursor also passes through instead of being eaten.
  New F3 / LOREFORGE_DEBUG_INPUT overlay shows ui_open/cursor_locked/
  playing/keys/health for live diagnosis.
- Developer console (`` ` `` or `/`): 20 commands — help, time set
  (sunrise/day/noon/sunset/night/ticks), give, tp, seed, weather, fly,
  heal, feed, kill, spawn, clear, waypoint add/list/remove, say, fps, rt,
  save, slots, load <slot>, new <type> [name]. TAB cycles autocomplete,
  arrows walk history, Esc closes; command parsing is a pure, unit-tested
  function.
- Real random seeds: each world owns a seed (`seed.dat`), generated from
  OS entropy for new worlds; WorldGen exposes it, noise channels hash it
  with splitmix64 (u64->i32 truncation no longer collides); switching
  worlds restarts the streamer with the new seed (latent bug: the worker
  kept its old WorldGen forever); the dedicated server persists its seed
  and sends the true value in Welcome.
- Natural biome transitions: fractal (3-octave) climate noise at lower
  frequency with a contrast stretch, domain warping (±34 blocks) so biome
  borders follow organic curves, and fine dithering (±0.045) that turns
  straight threshold lines into dithered transition bands of mixed surface
  blocks. `biome_from` stays pure; biome-coverage test samples wider.
- Multiple save slots: each world lives in `worlds/<slot>/` with
  `meta.dat` (name, type, seed, updated). Title menu reordered (Play —
  <slot>, New World submenu, Load Game, Multiplayer, Settings, Quit);
  slot picker with Load / Delete-with-confirm / Create (name + type);
  pause menu gains Save Now, Load Game, Quit to Title. `load_world()`
  reloads a slot mid-session; the pre-slot `worlds/default` auto-migrates
  to "World 1" keeping its chunks and seed (verified live). Slot meta,
  seed persistence and migration are tempfile-tested.
- DPI/proportional UI: egui zoom = user scale × native display density ×
  viewport factor (720p reference, clamped) — text, slots, panels and the
  minimap scale with both pixel density and window size; macOS bundle
  declares NSHighResolutionCapable.
- Harness: plain egui `Area`s never render in the two-pass headless
  harness (only windows materialize) — previews converted to frameless
  windows; new `console_preview` proof scene. Tests 140 -> 149.

## 2026-08-26 — P22: UI overhaul — icons, tooltips, recipe book, map suite (loop 302)
- Real pixel-art item icons in every slot (hotbar, inventory, crafting grid,
  chests, furnaces, machines, trades, cursor, tooltips): `lf_assets`
  sprite generator for all non-block items (tools per tier, ingots, raw
  ores, food, armor, industrial parts) + deterministic gem icons for mod
  items; block items reuse atlas art.
- Texture atlas 18 -> 41 layers: wood-variant logs/leaves, red sand,
  terracotta, moss, ice, copper/tin/bauxite/sulfur ores and all six
  machine/bench blocks now render in-world with their own art (they
  previously fell back to stone).
- Tooltip system: icon + display name, tool tier/damage/speed, food value,
  armor points, fuel seconds, "smelts into", crusher input, era-requirement
  badge, stack size; recipe-book entries preview their pattern as a
  mini-grid on hover.
- Crafting screen redesign + recipe book panel: unified catalog merging
  crafting (vanilla + mods), smelting, assembler alloys and crusher
  recipes; search box, station tabs, craftable-only filter; ingredient
  icons with have/need coloring; click auto-fills the grid from the
  inventory (returning grid contents first); "needs table" and era-lock
  badges.
- Shift-click quick-move everywhere: inventory storage <-> hotbar, chest
  <-> inventory, furnace/machine slots <-> inventory.
- Map suite: top-right minimap (terrain/biome colors, entity dots,
  waypoint pips, player arrow, north marker, toggle in settings) and the
  full M-key world map — pan/zoom, fog of war, explored-but-unloaded
  approximation dimmed, spawn marker, waypoint manager (add/rename/
  recolor/delete, persisted in ClientSave), cursor coords + biome,
  chunk grid at high zoom. HUD info line gains compass facing + biome.
- HUD polish: icon hotbar with pulsing selection glow and fading item
  name on switch, armor points, XP bar mirroring the hotbar width with
  level chip + gain flash, dynamic crosshair (expands while mining,
  hit-marker on attacks), hurt vignette + low-health pulse, redesigned
  death screen with run stats. Settings gains an Interface tab (minimap
  toggle, UI scale driving egui zoom).
- Fixes: `crafting::recipes()` leaked a Vec per call (now a OnceLock
  singleton), `Inventory::add_item` ignored per-item max_stack, crusher
  catalog listed `iron_ore` (a block id, not an item), and the vistest
  harness never rendered egui windows (fresh contexts need a warmup pass
  for window areas, whose font-atlas texture delta must be threaded to
  the renderer — the pre-P22 trade/tech proof shots had silently empty
  windows).
- Proofs: 3 new scenes (crafting_ui, map_screen, minimap_hud), all 19
  scenes re-rendered and pixel-verified (panels present, icons/text
  visible). Tests 123 -> 140.

## 2026-08-25 — P21: menus, animations, HUD & real settings (loop 301)
- ui_kit: theme, easing (+tests), Reveal stagger, animated menu buttons
  (hover glow, press spring, accent bar), slide panels, toggles, sliders,
  section headers, painted vector heart/hunger glyphs.
- Title screen: pulsing logo, staggered buttons, live orbiting world
  background behind the menu; pause menu as an animated slide-in panel.
- Settings screen with Video/Audio/Gameplay tabs and quality presets,
  persisted with the world; every knob drives the engine live (view
  distance feeds the streamer, FOV the camera, invert-Y the mouse,
  clouds/particles gate the atmosphere batches).
- Ray tracing settings made real: Off / Captures(R) / **Live** — a
  persistent Pathtracer reuses GPU resources and traces every frame at a
  configurable internal scale, shown fullscreen beneath the HUD.
- HUD rebuilt: painted hearts/hunger, XP bar with level chip, hotbar with
  hover tooltips, info line (clock, weather, net, FPS, RT flag).
- Proofs: menu_preview (dark-panel 20% + logo light 2.3% in the center),
  settings_preview (window clearly visible). Tests 119 -> 123.

## 2026-08-25 — P20: final consolidation (loop 300)
- All 14 vistest proof scenes render and pixel-verify: biome_montage,
  clouds_weather, first_person_view, hud_preview, industrial_machines,
  night_watch, raytraced_night (100% emissive), raytraced_shadows,
  spawn_plains_dawn, tech_tree, terrain_features, terrain_vista,
  torchlit_night, village_trading.
- STATE/STATUS/BACKLOG/RELEASE reflect exactly what exists.
- 121 tests green; game smoke-tested end to end.

## 2026-08-25 — P19: Steam readiness (loop 299)
- lf_steam crate: feature-gated Steamworks binding (off by default — the
  SDK links dynamically and CI lacks the client); preferred_transport()
  reports Steam only after a successful init, else UDP fallback (+2 tests).
- steam_appid.txt (Spacewar 480) for dev testing; title screen shows the
  active transport; docs/STEAM.md covers the dev loop, feature flag, depot
  layout and steamcmd upload.
- Tests 119 -> 121.

## 2026-08-25 — P18: compute voxel path tracer (loop 298)
- lf_engine pathtrace: WGSL compute tracer — DDA primary rays through a
  128x64x128 block clip texture, jittered soft sun shadows, one-bounce GI
  (sky + emissive torches/lanterns), fog, 2x2 supersampling (portable
  write-only storage; read-write accumulation unsupported on this adapter).
- Rust: build_voxel_texture_data from any World; pathtrace_to_image with
  f16 decode -> PNG; headless scene integration.
- Client: R key path-traces the current view in-game and saves
  shots/rt_frame_N.png.
- Proofs: raytraced_shadows (varied terrain lighting, luminance
  transitions), raytraced_night (100% warm emissive coverage with a
  lantern floor in view). Fixed along the way: uniform member order
  mismatch, stale cargo fingerprints masking edits, torch placement living
  in a dead code copy.

## 2026-08-25 — P15+P16: industrial machines & research (loop 297)
- worldgen: copper/tin/bauxite/sulfur veins by depth (+generation test).
- lf_game machines: Generator (EU buffer), ElectricFurnace (2x speed),
  Crusher (ore doubling), Assembler (bronze/steel/circuits/frames) with
  power draw (+5 state-machine tests).
- Client: machine blocks/entities with slots+progress UIs, 4-block power
  field, machines tick while closed, spill on break, persist.
- Research: eras with material costs, ResearchState advance consuming
  inventory (+3 tests); era-gated crafting shows locked recipes with
  requirements; research bench advances eras on RMB.
- Tech tree screen (K): era columns with done/current/locked states, live
  have/need cost colors, and a next-step hint line.
- Bug fixed: headless egui proofs encoded the UI pass after the texture
  readback — UI screenshots silently lacked their UI. Reordered; tech_tree
  proof now shows the panel (verified by pixel analysis).
- Proofs: industrial_machines, tech_tree. Tests 110 -> 119.

## 2026-08-25 — P14: combat & survival completion (loop 296)
- lf_game combat.rs: Arrow projectiles (gravity + solid-hit), XP curve
  (7+3*level) with carry-over levels, armor mitigation (flat, min 1)
  (+3 tests).
- Items: bow/arrow, bronze & steel chestplates (Armor kind), smithing
  table block (+texture); industrial material items (copper/tin/aluminum/
  sulfur/bronze/steel ingots, wire, gear, machine frame, basic circuit)
  with recipes — catalog consistency kept green throughout.
- Client: hold-RMB bow charging with HUD bar, arrows fly and damage mobs,
  XP bar in HUD, worn armor reduces damage, RMB on the smithing table opens
  the forge minigame UI (bellows pump + orange-zone strikes produce steel).
- Tests 107 -> 110.

## 2026-08-25 — P13: NPCs & villages (loop 295)
- lf_npc: trade_offers(job) tables for all six jobs (+coherence test).
- worldgen: hamlets gain dirt paths and lamp torches.
- Client: villagers spawn when hamlet chunks load (deterministic job/name,
  persisted), day-wander/night-rest schedule, RMB opens the trade screen
  with live have/need counts and affordability colors; lore book item opens
  a reading window showing the world chronicle; job-tinted villager cubes.
- Proofs: village_trading scene with a real egui trade panel (verified by
  pixel analysis). Tests 106 -> 107.

## 2026-08-25 — P12: world & atmosphere completion (loop 294)
- Biomes: data-driven table, 8 -> 30 (variant channel splits climate bands;
  census test verifies all 30 occur in a sampled world; per-biome surfaces,
  tree species, freezing oceans cap with ice).
- Blocks: birch/spruce/dark/cherry logs + leaves, pale leaves, red sand,
  terracotta (banded), moss, packed ice (+textures); Badlands strata.
- Trees: per-species shapes (conifer cones, tall jungle, wide cherry/dark
  canopies); >=3 species generation test.
- Atmosphere (lf_engine::atmosphere): drifting cloud layer (transparent
  pass), sun/moon billboards with celestial rotation, night stars,
  underwater fog/tint, weather cycle with rain/snow particles and storm
  sky darkening. Cloud winding bug caught by pixel-proof (visible from
  below only) — fixed.
- World types: Normal/Superflat/Amplified (superflat = flat, no caves/
  ores/structures); title-screen new-world buttons; type persists.
- Proofs: biome_montage (19 green hue buckets = species variety),
  clouds_weather (8.6% white cloud pixels). Tests 104 -> 106.

## 2026-08-25 — P11: performance & release; base game complete (loop 293)
- World light cache with invalidation on edits (+test) — writing the test
  exposed two real bugs: the mesher never actually sampled the light
  closure (per-face light was hardcoded 15 since P3) and section-local y
  indexed the world-height light array. Both fixed; renders now show true
  dynamic range (shadowed overhangs, torch pools, dark nights).
- xtask package: portable dist/ zip with release binaries, mods and docs.
- CI: release matrix (ubuntu/macOS/windows) builds and uploads artifacts.
- RELEASE.md rewritten honestly (how to run, controls, verified features,
  known gaps).
- Tests 103 -> 104. P0-P11 of the base-game plan are complete.

## 2026-08-25 — P10: mod API real (loop 292)
- lf_voxel registry: runtime mod blocks (MOD_BLOCK_BASE + fnv ids) consulted
  by name/is_solid/is_opaque; registration test.
- lf_game: register_mod_item / register_mod_recipe / register_mod_smelt;
  block_drop owned Strings + mod drops.
- lf_worldgen: register_ore_hook + generate consults hooks (+test).
- lf_modapi: smelting.toml parsed in load_mod (was silently dropped);
  apply_mod/load_mods_dir wire mods into the live registries; full-pipeline
  test: parse -> register -> place modded block -> break -> drop -> smelt.
- lf_assets: generic "mod" texture layer; client loads mods/ at boot
  (smoke log: loaded 2 mods).
- mods/README.md documents the mod surface.
- Tests 96 -> 103.

## 2026-08-25 — P9: multiplayer (loop 291)
- lf_protocol v3: ClientMessage/ServerMessage gameplay set with framed
  codec (+4 round-trip and rejection tests).
- lf_server (real now): UDP authoritative-lite server — canonical world
  (worldgen on demand + validated edits + history replay to newcomers),
  20/s player snapshots, chat relay, join/leave roster; a two-client
  integration test runs over real localhost UDP and passes.
- loreforge-server binary: dedicated hosting (bind addr + seed args).
- lf_client net.rs: connect from the title screen, state send, remote
  block edits applied with remesh, remote players as cubes, chat overlay
  with T input; local edits replicate to the server.
- Tests 92 -> 96.

## 2026-08-25 — P8: quests & chronicle live (loop 290)
- lf_story: QuestEvent (Collected/Crafted/Killed/ReachedDepth) advancing
  objectives with progress counters; starter_quests() 5-quest chain (+1 test).
- Client: events wired to pickups, crafting, mob kills; quest log UI on J
  showing objectives/progress/chronicle; chronicle milestones (first logs,
  first blood, Null Knight slain, deaths, quest completions) exported to
  worlds/<name>/chronicle.md on save; state persists in the save.
- Tests 91 -> 92; game smoke-tested.

## 2026-08-25 — P7: structures, menus, UI proofs (loop 289)
- worldgen structures: meadow huts, highlands watchtowers, desert pyramids
  (deterministic per-chunk placement, in-chunk footprints, +1 test).
- Client: title screen and pause menu; Esc opens pause; settings sliders
  (mouse sensitivity, FOV); quit saves the world.
- lf_engine headless: optional egui overlay in render_to_png; vistest
  hud_preview scene draws the real HUD (hearts/hunger/hotbar/crosshair/
  inventory) — honest UI proof shots at last.
- Tests 90 -> 91; game smoke-tested from the title screen.

## 2026-08-25 — P6: mobs & combat (loop 288)
- lf_game mobs rewritten as a live framework: MobEntity with AI/physics
  update (wander, flee-on-hit, chase, melee with cooldown), MobType stats
  table + drops, roll_spawn day/night table (+5 tests).
- Client: mob spawn cycle (every 2s, cap 12, surface-only, despawn 80),
  crosshair mob attack (tool damage table, knockback, hurt flash), mob
  cubes render with the drop batch, mobs/kills persist in the save.
- Items: porkchop/mutton food, glitch_dust/null_shard materials.
- Tests 87 -> 90; game smoke-tested with mobs active.

## 2026-08-25 — P5: content catalog core (loop 287)
- Blocks: furnace, chest, planks, glass (transparent); matching textures.
- lf_game: smelting module (Furnace state machine with fuel/burn/progress,
  +4 tests), iron tier + swords in the item table, tool_damage table,
  recipes for furnace/chest/iron tools/swords.
- Client: block entities (furnace/chest) with persistence, RMB opens their
  screens, furnaces tick while closed, containers spill contents when
  broken, furnace UI (input/fuel/output + flame + progress), chest UI.
- Catalog consistency test: recipe outputs/ingredients, smelt outputs,
  block drops, and block items all resolve.
- Tests 82 -> 87; game smoke-tested with containers active.

## 2026-08-25 — P4: survival & inventory UI (loop 286)
- Migrated winit 0.29 -> 0.30 (ApplicationHandler) and adopted egui 0.31
  (matches wgpu 24; the 0.29 stack was incompatible).
- lf_game: items registry (block items, tools with tiers, food, materials),
  mining rules (hardness, tool multipliers, harvest gating, break times),
  shaped crafting with translation-aware matching (+13 tests total).
- Blocks: crafting table; torch item texture; CRAFTING_TABLE id 14.
- Client: egui HUD + inventory/crafting/death screens with full stack
  interactions (pick/place/swap/merge/split); hold-to-mine with progress;
  tool durability; item drops (gravity, magnet pickup, bobbing cubes);
  hunger/regen/fall-damage/drowning/death+respawn; RMB context (open table,
  eat, place); inventory/stats/time persisted in player_extras.dat.
- Tests 69 -> 82; game smoke-tested with UI active; all scenes render.

## 2026-08-25 — P3: lighting & atmosphere (loop 285)
- lf_voxel light.rs: per-column flood-fill sky + block light (BFS with -1
  falloff, opacity-aware, 15-level); emitter table (torch=14, lantern=15);
  index-stride bug class caught by tests (y must own the largest stride).
- Mesher samples light from the exposed cell per face (packed sky<<4|block
  in the vertex light attribute; was hardcoded 15).
- Shader: brightness = max(sky*day, block*0.92) with ambient AO and distance
  fog blending to the sky color; uniforms carry camera pos + day + fog.
- Water: separate alpha-blended pipeline (no depth write), water faces split
  from opaque mesh, columns sorted back-to-front; water texture alpha 170.
- Torch/lantern blocks (non-solid, non-opaque, targetable).
- Client: 20-minute day/night cycle (lf_game::TimeOfDay) driving sky clear
  color, day factor and fog; torch in the 9-slot hotbar; deeper night
  constants (starlight 0.12, night sky mix 0.15).
- vistest: torchlit_night scene (torch grid on terrain at night); scene sky
  math now reuses lf_game::TimeOfDay.
- Tests 66 → 69; all 6 scenes render; game smoke-tested with lighting.

## 2026-08-25 — P2: world streaming & terrain (loop 284)
- lf_voxel: block registry (is_solid/is_opaque/is_targetable, 12 blocks);
  mesher culls by opacity (air/water/leaves show faces behind them, no
  water-water faces); ChunkColumn serializable; WorldStorage saves chunk
  columns via region files + player.dat (+round-trip tests).
- lf_worldgen: trees on meadows (deterministic hash placement, canopy kept
  in-chunk), 3D-noise caves, coal (<y96) and iron (<y48) ores in stone,
  water fills to sea level; 4 feature tests over real generated chunks.
- lf_assets: log/leaves/coal/iron/water textures (11-layer atlas).
- lf_client: background chunk streamer (worker thread, nearest-first,
  view radius 5, unload radius 8 with save-before-drop), sphere-frustum
  column culling from mesh bounds, world persistence with 30s autosave and
  save on exit, player position/look restored from save, hotbar 8 slots.
- vistest: terrain_features scene; renders verify trees (~7% canopy pixels)
  and water (~20% water pixels) visible.
- Tests 58 → 66. Game smoke-tested 20s with streaming active.

## 2026-08-25 — P1: first-person playable core (loop 283)
- lf_voxel: World + ChunkColumn (16 sections = 16x256x16), world-coord
  get/set with chunk border math, surface_height, mesh_column with cross-
  border neighbor culling; 4 new tests.
- lf_worldgen: generate_chunk fills a ChunkColumn directly; surface_top()
  helper (surface band tops out at height+3, standing surface height+4).
- lf_game: Player with AABB physics — gravity, jumping, sprint/sneak/walk
  speeds, fly mode, axis-separated collision with substepping so long falls
  never tunnel; 8 physics tests (landing, walls, jump arc, ceiling, fly,
  tunneling, look dir).
- lf_engine: scene split into SceneResources (shared pipeline + atlas) and
  MeshBatch (per-column drawable); OutlineScene line pipeline renders the
  targeted block outline.
- lf_client (replaces dummy): the game shell — winit input with cursor
  lock, player update loop, DDA raycast targeting with outline, instant
  break / face-adjacent place (with player-overlap rejection), 6-slot
  hotbar (digits + scroll, shown in window title), F2 screenshots via the
  offscreen renderer, per-column GPU remesh on edit.
- vistest: first_person_view scene with vista-seeking camera (moderate-drop
  selection, constrained to the meshed area); scenes use World pipeline and
  radius-3 meshes.
- Tests 43 → 57. Proofs: shots/vistest_first_person_view.png (eye-height
  view over real terrain), plus re-rendered dawn/vista/night scenes.

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

## Loop 308 — P28 (V1REBRAND gate) begins: sway honesty fix + 4 latent UX bugs
- Committed docs/V1REBRAND/11-EXECUTION-PLAN.md: the full P28-P39 execution
  plan mapping roadmap docs 02-10 onto BACKLOG numbers at +2 offset
  (DECISIONS entry added; P26/P27 were already taken by visual identity and
  the camera-culling fix).
- Wind-sway honesty fix: P26's commit message claimed foliage wind sway, but
  shader.wgsl's vs_main never consumed the sway vertex attribute (loc 6) or
  the time_sway uniform — the attribute, uniform, mesher weights, client
  clock, and cull margin all existed; only the shader math was missing, so
  leaves never moved. vs_main now applies a world-position-phased double
  sine (amplitudes 0.055/0.045/0.02, combined max ~0.08 blocks, inside the
  0.1 sway margin of the P27 cull). New proof test
  foliage_sway_animates_between_frames renders the canopy at two wind
  phases through the real GPU pipeline: frames must differ (>0.1% of
  pixels), same-phase control must be pixel-identical. The test necessarily
  fails on the old shader (it had zero time dependence).
- Clouds setting was a no-op: cloud_batch was rebuilt unconditionally; now
  gated on settings.clouds (cleared when off).
- UNLOAD_RADIUS (fixed 8) -> view_distance + UNLOAD_MARGIN(3); the distance
  cull in column_in_view now takes the view distance (view 8 previously had
  zero unload headroom). P27 regression updated to pass view=5 explicitly.
- First-launch fix: Settings opened from the title screen now returns to
  the title via Back and Esc (close_settings); previously both dropped the
  player straight into the world.
- Boot now loads the booted slot's player_extras (settings/inventory/etc.)
  instead of reading legacy worlds/default before boot_slot() — slotted
  players previously booted with defaults until clicking Play.
- 162 tests green (was 161), 22/22 vistest scenes, release smoke OK.

## Loop 309 — build-pack Stage A: reality audit (Step 1) + 8 fixes
- Executed docs/poorcraft-build-pack Step 1 per 01-REALITY-AUDIT.md: every
  [x] claim in BACKLOG verified across code, a live release-build session
  (title captured twice, in-world session observed, log inspected), and
  fresh vistest renders. AUDIT.md at repo root records CONFIRMED vs
  ACTUALLY-BROKEN/MISSING per claim; BACKLOG corrected in the same commit.
- Verdicts on the three user-flagged areas: destruction feedback CONFIRMED
  (crack decal + block-textured debris traced through the real mining
  path) except break SOUND (ACTUALLY-MISSING — no audio system exists);
  lore machinery CONFIRMED but shallow (chronicle readable in play via J
  and the book; 5/11 event types never fire; no dialogue; no named
  places); biomes ACTUALLY-BROKEN as an experience (17-18 of 30 are
  worldgen twins; one untinted grass texture; global fog; MYCELIUM unused;
  biome_montage scene shows one vista, not a montage).
- Fixed with the audit (each with a test): HUD rendered behind the title
  menu (hud_visible gate); title orbit camera buried in ring terrain ->
  flat-dark backdrop (title_eye_y clamp + audit_title_camera.rs repro
  tool: World_5 had 12/64 orbit points under higher ground); render
  culling/water sort used the player eye instead of the render camera;
  streamer wish radius hardwired to 5 so view-distance settings never
  streamed farther (sync_wish); sneak captured but never read (0.45x
  careful walk); smithing UI called strike() and granted a steel ingot
  every frame (Strike button + ForgeMinigame::reset); lantern block had
  no item/recipe (craftable iron-over-torch); random_seed() could collide
  within one clock tick (sequence counter). 201 fossil shots/ev_*.png
  removed (Evolution-era residue; zero code references — several name
  creatures that don't exist). RELEASE.md counts corrected (168 tests /
  22 scenes).
- 168 tests green, 22/22 vistest; the user's own live play session served
  as the smoke check (their process was left running untouched).

## Loop 310 — block gravity + water physics (user request)
- Granular blocks no longer float: registry::has_gravity (sand, red_sand,
  snow, dirt, grass, moss, mycelium — ores excluded, embedded in stone per
  the Minecraft rule). Breaking support detaches the whole column into
  animated FallingBlock entities (rendered with the block's own texture,
  water-damped sinking, landing re-places through the same remesh +
  network-broadcast path as a player edit, crushing nothing v1; a landing
  into an occupied cell drops the item instead).
- Water physics: event-driven cellular simulation in lf_game::fluids —
  flow level 0 (source) .. 7 rides in BlockState's unused flag nibble;
  water falls first, then spreads horizontally with decay, and unsupported
  flow dries up (scooping a source recedes its puddle — test-proven).
  Edits enqueue the cell + 6 neighbors; a 64-cell tick budget bounds frame
  cost. Worldgen oceans/lakes are sources, so nothing changes until
  disturbed. Mesher renders flowing water as stepped, lowered surfaces
  with step-covering side faces (no slits between levels).
- Bucket + water_bucket items (craftable: 3 iron ingots in a V; pixel-art
  icons) — scoop a source (right-click it with an empty bucket) or pour
  one (right-click a face with a full bucket): the player-facing tool for
  the fluid system and the P30 Steam-Age groundwork.
- Proofs: 6 new unit tests + 2 new vistest scenes — water_flow (aqueduct
  pours down a flume and pools at a dam, settled through the real sim
  before meshing; AI-verified stepped surfaces + pooling) and falling_sand
  (column collapsed into a dug pocket via the real gravity settle, plus a
  mid-air faller cube; AI-verified pile + floating block). 24 scenes total.
- 174 tests green, 24/24 vistest; runtimes rebuilt; pushed.

## Loop 311 — goal Sections 0–4: re-audit + the four feel fixes
- S0: re-verified the four flagged items before touching them (AUDIT.md
  "Goal-file re-audit" section). Verdicts: bottom mining bar CONFIRMED;
  texture stretching NOT reproducible in the raster path; biome grade
  absent CONFIRMED; mod-load visibility absent CONFIRMED.
- S2 destruction feel: removed the mining/bow egui::ProgressBar pair from
  the bottom HUD panel (the reported "mar") entirely; progress renders as
  a crosshair-centered radial ring (faint track + clockwise accent arc;
  bow charge in the ok role). Geometry unit-tested
  (reticle_arc_spans_progress_from_top); hud_preview renders it mid-break.
  Crack decal + debris particles unchanged.
- S3 biome grade: shader.wgsl gains a grade uniform (tint multiply +
  saturation pull toward luma) applied after lighting and fog; lf_client
  biome_grade table (desert/badlands/savanna warm, snow family cool +
  desaturated, swamp/jungle lush, hollow eerie pale, oceans teal,
  temperate neutral); ~0.3s exponential lerp across boundaries; clear
  color mirrors the grade so the sky shifts with the world. GPU proof:
  biome_grade_shifts_midframe_color (same scene, warm vs cold: hue moves
  ~10.7deg, saturation ~0.10).
- S4: mods/smoke_test (one block, one item) with lf_modapi::smoke_line ->
  "[MOD SMOKE TEST] OK" boot line on both client and dedicated server; CI
  test loads the real mods/smoke_test folder and asserts both
  registrations; mods/README.md points to it as the first sanity check.
- S1: proved per-block texture tiling instead of assuming it — mesh test
  multi_block_walls_tile_per_block_not_stretched (two blocks = two quads,
  seam vertex, UVs exactly 0..1) + texture_tiling scene (7-wide plank
  wall + stone floor, AI-verified per-block repetition, no smearing).
  DECISIONS: greedy meshing blocked on the UV-repeat invariant; Live RT
  ships (live + capture) — capture-only no longer the model.
- STATUS.md rewritten to match verified reality (the old one claimed 121
  tests / 14 scenes / live-RT-deferred).
- 178 tests green; 25/25 vistest scenes; runtimes rebuilt; pushed.

## Loop 312 — audio engine + impact shake + FOV/transparency/perf proofs
- lf_audio crate (rodio): procedural break/place sounds per material
  category with silent fallback; wired into real break/place; sliders now
  actually drive playback (the settings label no longer says "when it
  lands"). 4 dispatch/synth tests. CI ubuntu installs alsa headers.
- Step 3 impact pulse: short decaying screen shake on heavy breaks
  (envelope tested), applied to the camera target only.
- Step 7: FOV reference test at 90/60 degrees guards the double-radians
  bug class on the raster path.
- Step 8: transparency_layers scene (water behind glass, particles both
  sides) — AI-verified correct layering.
- Step 9: headless.rs refactored into a persistent HeadlessRenderer (the
  naive perf loop measured 774ms of per-frame SETUP); xtask perf + make
  perf at Medium radius-5: p50 111 / p95 156 / min 77 ms incl. readback +
  PNG encode; DECISIONS names this host's iGPU as the low-end target.
- 184 tests green; 26/26 vistest scenes; runtimes rebuilt; pushed.

## Loop 313 — Steps 13/14/15: settings completeness, thumbnails, minimap
- Key rebinding: lf_client::input Keymap (Action-keyed, name-serialized,
  junk-safe load), Controls tab with capture rows, movement + UI keys
  rebound live, persisted in ClientSave; PathTraced quality tier drives
  RtMode::Live; both round-trip tested.
- Save-slot thumbnails: throttled live-view capture at save time, shown in
  the picker.
- Minimap: rotation with the view (rotated texture mesh + marker/N-chip
  rotation) and 0.5-3x zoom, persisted; world-space waypoint beacons
  (six tint layers, transparent pass, per-frame rebuild) with the
  waypoint_beacons proof scene.
- 187 tests green; 27/27 vistest; SMOKE OK with the live boot log showing
  "[MOD SMOKE TEST] OK — smoke_test mod loaded successfully" (3 mods).

## Loop 314 — Steps 16-19: biomes finally read as different places
- New identity surfaces: jungle grass, savanna grass (gold), mycelium
  hollow, moss swamp, FlowerForest wildflowers; Tundra spruce-sparse vs
  SnowyTaiga dense; boulder fields on the three windswept/snow-slope
  biomes. Generator v2. Contact sheet measures 30/30 distinct strip
  colors; pairwise identity regression test with two documented families
  exempt (it caught two real twins while being written — both fixed).
- Biome-aware day spawns (woolbeast= cold, boar= temperate) with test;
  weather coldness from the biome field; weather_snow + weather_dry
  proof scenes.
- 188 tests green; 30/30 vistest scenes; runtimes rebuilt; pushed.

## Loop 315 — P29 Water Age
- Research is a graph now: the Water branch (Industrial prereq,
  independent of Electrical), unlockable from the tech-tree screen with
  material costs; pre-branch saves load unchanged (serde default).
- Machines: WaterWheel (12 EU/s free while touching water) + Battery
  (4000 EU) and a pure, tested distribute_power (producers → batteries
  cover gaps → surplus recharges) that the client tick and the vistest
  scene both run. Water Wheel + Battery blocks craftable (Water-era
  gated) with UI panels. RT palette covers new ids + stable fallback for
  future ones.
- 194 tests green; 31/31 vistest (water_wheel_power proof); runtimes
  rebuilt; pushed.

## Loop 316 — P30 Steam Age
- Steam branch research (independent of Water, tested); Pipe/Boiler/
  SteamEngine machines (equal-share pipes, fuel+water->steam boiler,
  16 EU/s engine) with 4 tests including the full chain; blocks through
  the content pipeline with UI panels and burning-boiler steam puffs.
- 199 tests green; 32/32 vistest (steam_chain proof, AI-verified); smoke
  OK; runtimes rebuilt; pushed.

## Loop 317 — cross-column lighting + lore books
- Light engine floods a 3x3-column neighborhood: chunk borders no longer
  seam (regression test + night_border_seam proof with a measured 1.92
  max brightness step; perf unchanged at p50 47.7ms).
- Lore books: three tomes in lore/books.toml (the Smith / Null / river
  warden threads), on-kit paginated reader, Lorekeeper trades, icons;
  file-load test + AI-verified lore_book proof scene.
- 201 tests green; 34/34 vistest; smoke OK; runtimes rebuilt; pushed.

## Loop 318 — Oil Age (P31) + power-grid overlay (Step 25)
- Crude oil in worldgen (desert/swamp pools + surface seeps), typed
  pipes, pumpjack/refinery/combustion generator, Oil research branch
  (Steam-or-Electrical either-or), oil buckets, tar byproduct.
- G toggles the power-grid overlay: green/red tint cubes over machines.
- 209 tests green (+8); 36/36 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.

## Loop 319 — Nuclear tier, capped (P32)
- Uranium (deep rare band), fuel rods, the 32 EU/s reactor with a real
  heat curve (equilibrium cooling, auto-SCRAM, residual decay heat,
  meltdown with glowing radiation residue + chronicle event), the
  reactor_safety certification gating Era::Nuclear, reactor UI with the
  big red button.
- 213 tests green (+4); 38/38 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.

## Loop 320 — Magic foundation (P33)
- Mana + HUD, the bounded four spells with 3 rebinding-aware slots and a
  spellbook screen, wizard NPC + rare tower worldgen, scroll learning,
  the enchanting imbue minigame with real rune effects on held tools,
  and the two crossover blocks (fuelless lumen light, mob-warding
  pylon). Extras saves moved to JSON with a legacy bincode migration —
  fixing the latent silent-reset on every old field addition.
- 223 tests green (+10); 41/41 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.

## Loop 321 — Construction (P34)
- The shape system (slabs/stairs in BlockState bits, shaped meshing,
  fractional collision), shaped placement with slab merge, climbable
  bulk-removable scaffolding, build symmetry (V), blueprint
  capture/ghost/paste with material bills, chisel statue carving, and
  the modapi light fix + decor_pack. 233 tests green (+10); 42/42
  vistest scenes; smoke OK; runtimes rebuilt; pushed.

## Loop 322 — Smart building (P35)
- Conduit-relayed power distribution (4-hop field chains), the physics
  elevator, climate comfort regen, and the computer screen with the
  engine's first dynamic-texture path (data-change-gated atlas layer
  rewrites showing research/chronicle/grid readouts).
- 238 tests green (+5); 43/43 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.

## Loop 323 — Dragons (P36)
- The roost boss end-to-end: circle/swoop/perch flight AI (tested),
  multi-part sine-animated rendering from a shared layout fn, fire
  breath, mountain roost worldgen (gated, tested), BossSlain saga, and
  the approved dragon mount after the streaming spike held.
- 243 tests green (+5); 45/45 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.

## Loop 324 — Paths & specialization (P37)
- Four play-accrued paths with chronicle milestones, the generalized
  era|path gate enforced at crafting AND placement (fixing the
  branch-era bench lock bug), respec with double-accrual focus, the
  ornate tier, and protocol v4 player trading with server escrow and a
  real-UDP test.
- 248 tests green (+5); 47/47 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.

## Loop 325 — Finish line, part 1 (Steps 34-39)
- Transport-neutral lobbies + the invite flow (tested), Workshop UGC
  scanning (tested), the full mods/README authoring guide, and
  `xtask new-mod` scaffolding (live-verified + loader-tested).
- 253 tests green (+5); smoke OK; runtimes rebuilt; pushed.

## Loop 326 — P28 leftovers + Step 40
- Connected stone/planks surfaces, HUD text shadows + the on-kit audit,
  in-play chronicle toasts + the cross-system lore-anchor test, and the
  item belt backbone (tested pure push + client pass). Final honesty
  pass recorded in BACKLOG + STATUS.
- 256 tests green (+3); 47/47 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.

## Loop 347 — Hitboxes, walls, wheels & castle siting
- User bug hunt: flowers no longer cull the ground under them or form
  invisible solid hitboxes (`is_plant`-driven solidity/opaque), picking
  and the selection outline follow real block shapes (`pick_boxes` +
  shape-aware raycast), animals collide with walls (axis-separated AABB
  physics with hop step-up; dragons clamp to terrain), LMB near mobs no
  longer throttles creative breaking to 2/s (crosshair LOS filter), the
  wheel switches every notch instantly (multi-step + no set_title +
  same-frame UI), a caption above the hotbar names the looked-at block,
  and kingdom citadels site properly (dense validation, border margin,
  spawn clearance, hillside carving, clean courtyard; gen v6).
- 399 tests green (+12); 93/93 vistest scenes; smoke OK; runtimes
  rebuilt; pushed.
