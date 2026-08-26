# V1REBRAND Execution Plan (P28–P39)

This file is the operational execution plan derived from `00`–`10`. It adds
nothing to the design law; it sequences it, names the files, and records
what each phase must prove. Phases are numbered to continue BACKLOG.md
(P26 visual identity and P27 camera-culling fix already consumed those
numbers), so every roadmap phase below runs at **doc number + 2**:

| This plan | V1REBRAND doc | Topic |
|---|---|---|
| P28 | `02` (P26) | Rendering & UX gate completion — **hard gate** |
| P29 | `04` (P27) | Water Age power |
| P30 | `04` (P28) | Steam Age power + fluid groundwork |
| P31 | `04` (P29) | Oil Age power + grid visualization |
| P32 | `04` (P30) | Nuclear tier (capped ceiling) |
| P33 | `05` (P31) | Magic foundation |
| P34 | `06` (P32) | Construction & architecture tools |
| P35 | `06` (P33) | Smart-building tech layer |
| P36 | `05` (P34) | Dragons & top-tier creatures |
| P37 | `07`/`08` (P35) | Paths & specialization + player trading |
| P38 | — (approved extra) | Procedural audio engine |
| P39 | `10` | Release prep (friend test → Steam page) |

No phase below P29's prerequisites starts until P28's exit criteria (doc
`02`) are met and vistest-proven. Checkpoint **C1** (friends-and-family
build, doc `10`) opens once P28 + P29 are done: package, ship, keep
building, fold feedback into the active phase.

## Per-phase obligations (inherited from AGENTS.md)

Every phase ships code, keeps `cargo test --workspace` green, adds or
extends a vistest scene with pixel analysis, updates
STATE/BACKLOG/CHANGELOG/DEVLOG, syncs the Makefile, rebuilds runtimes
(`make runtimes`), and pushes. New blocks follow the content pipeline in
order: `lf_voxel/src/registry.rs` → `lf_assets` (atlas layer + procedural
texture + pathtrace color) → `lf_game/src/items.rs` (item/drop/recipe) →
`catalog_consistency` green.

## P28 — Rendering & UX gate completion (doc 02)

1. **Wind-sway fix**: `shader.wgsl` vs_main must consume vertex loc 6
   (`sway`) and `uniforms.time_sway` — the attribute and uniform exist but
   were never read, so P26's "wind sway" claim was not yet true in-game.
   Proof: canopy scene rendered at two sway phases must pixel-diff in the
   foliage region.
2. **Chunk-border lighting**: extend `compute_column_light` to flood sky +
   block light across a 3×3 column neighborhood at mesh time; replaces the
   "cross-chunk seams accepted" decision. Proof: night scene with a torch
   straddling a chunk border; seam-luma regression test.
3. **Transparency/sort audit**: document water-column sort + particle
   rules (steam/smoke = alpha-blended billboards in the transparent pass,
   budgeted, `settings.particles`-gated) in DECISIONS.
4. **Frame-time target**: host Intel-Mac iGPU ≈ the "low" device; add
   `xtask perf` (N headless frames at Medium, p50/p95 ms) + Makefile
   target; write the number (≥30 fps / ≤33 ms p95 at Medium, view 5) in
   DECISIONS.
5. **Quality tiers**: `Quality::PathTraced` added (Low/Medium/High/
   Path-Traced dropdown maps to Live RT); fix the `clouds` settings no-op;
   `UNLOAD_RADIUS = view_distance + 3`.
6. **Key rebinding**: new `lf_client/src/input.rs` (`Action` + `Keymap`),
   migrate `window_event` + `PlayerInput` construction + hotbar digits,
   Controls settings tab with capture rows, persisted in `Settings`
   (`#[serde(default)]`), dynamic pause hints.
7. **Save thumbnails**: render a small PNG in `save_world` via
   `render_to_png` → `worlds/<slot>/thumb.png`; shown in `draw_slots`.
8. **Minimap**: rotation + zoom on the corner minimap; waypoint beacons
   as in-world translucent beams (transparent batch) + HUD pips.
9. **First-launch flow**: fix Settings-back-from-title dropping into the
   world; drop the legacy `worlds/default` pre-load; Multiplayer gets
   address/name entry (no hardcoded localhost); dead-end audit.
10. **UI language audit**: on-kit quest log, book, console, trade, tech
    tree; shared screen shell in `ui_kit.rs`; HUD text shadow/outline
    helper for all-biome legibility; add `Theme::MANA` (P33 prep).
11. **Connected-surface textures** (BACKLOG P26 remainder:
    stone/marble/planks).

## P29 — Water Age (doc 04)

1. **Research graph**: `research.rs` linear `Era::next()` → prerequisite
   graph (unlocked set; Water ⊥ Steam; Oil after Steam|Electric; Nuclear
   after Oil + `reactor_safety`). Save migration via serde defaults.
2. **Pathtrace palette**: registry-driven `pathtrace_color(id)` so every
   future block id renders in RT (palette currently stops coloring at 41).
3. **WATER_WHEEL + BATTERY** blocks through the content pipeline,
   Water-era-gated recipes. Wheel = steady lowest-tier output while
   adjacent to water (flow sim deferred — DECISIONS entry). Battery
   buffers field surplus and discharges to starved machines; the power
   tick loop in `lf_client/src/lib.rs` gains a typed source list.
4. Proof: river + wheel + battery + running crusher scene.

## P30 — Steam Age (doc 04)

1. **Fluid groundwork**: `lf_game/src/fluids.rs` — mB units, pipe
   block-entity graph, per-tick BFS equal-share flow, no pressure sim
   (DECISIONS). PIPE block; BUCKET item (scoops a water source).
2. **BOILER** (fuel via `fuel_seconds` + piped/bucketed water → steam
   buffer) and **STEAM_ENGINE** (steam → power between wheel and coal
   generator). Steam/smoke particles per P28 rules. Steam era ⊥ Water.
3. Proof: boiler → engine → machine chain with live particles.

## P31 — Oil Age (doc 04)

1. **Oil worldgen**: biome-gated (desert/swamp) underground pools +
   surface seeps; GENERATOR_VERSION bump.
2. **Pipes v2**: fluid typing (water/crude/refined); OIL_PUMP (powered),
   REFINERY (crude → refined fuel + byproduct), COMBUSTION_GENERATOR
   (top-below-nuclear output). Oil era after Steam|Electric.
3. **Grid visualization**: powered/starved machine tint via outline or
   transparent batch, toggled in-world — craft-first, no spreadsheet.
4. Proof: extraction → refine → power chain + overlay scene.

## P32 — Nuclear tier, capped (doc 04)

1. **URANIUM_ORE** via `register_ore_hook` (deep y 8–24, rare, tiny
   veins) + content pipeline.
2. **Fuel rods**: smelt → ingot → assembler rod. **REACTOR**: highest
   output, heat/output curve, piped cooling water, SCRAM; **meltdown** =
   local destruction + lingering radiation residue blocks (damage until
   cleaned) + chronicle event. Never silently safe.
3. Research: Nuclear after Oil **and** `reactor_safety`. **DECISIONS:
   nuclear is the power ceiling** (Pillar 5).
4. Proof: reactor_power + meltdown_aftermath scenes.

## P33 — Magic foundation (doc 05)

1. **Mana**: `PlayerStats.mana` + regen, ClientSave persistence, HUD bar
   mirroring the XP bar, `Theme::MANA`.
2. **Spell slots** (3, hotbar-adjacent, rebinding-aware) + on-kit
   spellbook. Bounded set of four: firebolt (projectile), gale-step
   (blink), ward (timed absorb), hearthlight (temporary light + hand
   smelt).
3. **Wizard NPC**: `VillagerJob::Wizard` teaches spells, sells reagents,
   gives lore quests; wizard-tower worldgen structure + spawn marker.
4. **Lore books readable** (finishes deferred P8): tomes teach spells.
5. **Enchanting**: ENCHANTING_TABLE + imbue minigame mirroring
   `ForgeMinigame`; fills the pre-cut `CustomTool.rune` +
   `RuneApplied` chronicle event.
6. **Crossover items** (2–3, rare): fuelless magic light block,
   machine-warding block.
7. Proof: wizard_tower, spellbook UI, spell_effects scenes.

## P34 — Construction & architecture (doc 06) — riskiest phase

1. **Shape system**: shape + orientation in `BlockState`'s unused high 8
   bits; `mesh_section` shape dispatch (slab/stairs/slope via
   arbitrary-corner `push_face`); per-shape collision AABBs in player
   physics. Fallback if the spike struggles: slabs-only v1. DECISIONS: RT
   renders shapes as full cubes (documented limitation).
2. **Symmetry/mirror placement**, **blueprint/schematic** (two-corner
   capture → `worlds/<slot>/blueprints/`, ghost preview, paste consumes
   materials), **scaffolding** (climbable, bulk-remove).
3. **Decoration registry**: `lf_modapi` v2 — per-block textures, apply the
   parsed-but-unused `light` field, data-driven decoration packs
   (statues/banners/rugs/furniture).
4. **Statue carving**: chisel minigame mirroring smithing.
5. Proof: build_tools (stairs/slabs + ghost), statue_gallery scenes.

## P35 — Smart building (doc 06)

1. **Conduit/wire blocks**: power-field relays (DECISIONS: unified field +
   relays).
2. **Elevator** (powered vertical ride), **climate/AC block** (cosmetic +
   minor regen perk).
3. **Computer/screen block**: new dynamic-texture path in lf_engine
   (data-change-driven `queue.write_texture` + mip regen); pages cycle on
   interact; shows tech-tree/chronicle/grid status.
4. Proof: modern_wing scene ("one wing wired for electricity").

## P36 — Dragons & top-tier creatures (doc 05)

1. **Dragon entity**: multi-part animated rendering (body/head/tail/
   wings), flight AI (circle/swoop/perch/phases), fire breath, rare
   roost-structure spawn, stats above Null Knight, chronicle `BossSlain`.
2. **Steam/water elemental** (small fluid-adjacent roster addition).
3. **Dragon mount** (approved stretch): technical spike first (flight ×
   chunk streaming: unload margin, look-direction generation priority,
   mesh budget) → DECISIONS entry → implement if the spike holds, honest
   deferral note if not.
4. Proof: dragon_roost, dragon_flight scenes.

## P37 — Paths & specialization (docs 07/08) — last, gates P29–P36 recipes

1. **Path standing**: Engineer/Architect/Battlemage/Artisan on
   ClientSave; accrual hooks (machines run, blocks placed, spells/bosses,
   crafts/enchants); no decay, no lock-in.
2. **Gate generalization**: `Gate::Era | Gate::Path{standing}`; enforce at
   craft-match and placement (today gating is UI-only); professional-tier
   ornate recipes (1–2 per path).
3. **Respec** (resource-sink quest redirecting future gains); chronicle
   path milestones; Paths standing screen.
4. **Player-to-player trading**: protocol v4 (offer/accept/cancel, server
   escrow + ack, PROTOCOL_VERSION bump); trade UI extending the villager
   screen; real-UDP tests. Server-side validation of new blocks follows
   the P25 `SetBlock` pattern; full server-side machine simulation stays
   deferred (honest BACKLOG note).
5. Proof: paths_screen, trade scenes.

## P38 — Procedural audio engine (approved extra)

Synthesized SFX, no asset files (generated like the textures): break/
place/step/UI/cast/machine hum/reactor hum/dragon roar. Dependency choice
(cpal vs rodio) → DECISIONS; consumes the existing volume sliders;
silence-safe fallback keeps tests audio-free. Schedulable any time after
P28; ideally before C1 ships.

## P39 — Release prep (doc 10)

Early Access framing; store copy leading with the pillars; capsule art
brief (water wheel + wizard tower, composited from vistest shots); system
requirements from the P28 perf target; real App ID replaces Spacewar 480
(`docs/STEAM.md` + `steam_appid.txt` updated together, `steam` feature
flips only after); name change pending a separate decision.

## Risks & mitigations

- Shape system (P34) is the biggest structural change → spike first,
  slabs-only fallback.
- Pipes scope creep → strict v1 BFS equal-share, no pressure.
- Research/save refactors → serde defaults + migration tests; existing
  saves keep loading.
- Phase overflow → honest BACKLOG deferrals, never silent drops.
