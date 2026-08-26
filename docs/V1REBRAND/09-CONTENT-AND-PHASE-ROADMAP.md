# Content & Phase Roadmap (P26 → P35)

This turns files `01`–`08` into a concrete queue, numbered to continue
directly from BACKLOG.md's existing P1–P25. Copy each phase into
BACKLOG.md as it's picked up, and check items off the same honest way
existing phases are — unfinished sub-items get `[ ]` and a one-line honest
note, not silently dropped.

## P26 — Rendering & UX stabilization (hard gate, see `02`)

- [ ] Per-vertex lighting / AO on voxel meshes
- [ ] Chunk-border lighting consistency audit
- [ ] Camera/FOV correctness pass (rasterized path)
- [ ] Frame-time target defined + met on a real "low" device
- [ ] Settings menu: key rebinding, quality tiers, audio stubs
- [ ] Save-slot thumbnails
- [ ] Minimap rotation/zoom + waypoint beacons
- [ ] Shared UI language audit across all existing + planned menus

*No phase below starts until P26's exit criteria in `02` are met.*

## P27 — Water Age power

- [ ] Water wheel block + placement rules (adjacent to flowing water)
- [ ] Power-field integration (lowest-tier output)
- [ ] Basic power storage/battery block (needed once intermittent sources
      exist)
- [ ] Vistest proof: a working water-wheel-powered machine

## P28 — Steam Age power

- [ ] Fluid/pipe subsystem groundwork (minimal: water piping only)
- [ ] Boiler block (wood/coal fuel, reuses existing fuel-item pattern)
- [ ] Steam engine block (mid-tier output)
- [ ] Steam/smoke particle effects (only after P26's transparency audit)
- [ ] Vistest proof: boiler → steam engine → machine chain

## P29 — Oil Age power

- [ ] Oil deposit worldgen (desert/swamp-biome-gated)
- [ ] Full pipe/fluid transport (extends P28's groundwork)
- [ ] Oil pump/derrick, refinery, combustion generator
- [ ] Grid visualization overlay (powered/starved machines at a glance)
- [ ] Vistest proof: full extraction → refine → power chain

## P30 — Nuclear tier (capped endgame power)

- [ ] Uranium ore worldgen (rare, deep-only, small veins)
- [ ] Reactor block + fuel-rod processing chain
- [ ] Meltdown failure state (overload/unmaintained reactor) + cleanup
      mechanic
- [ ] Highest-tier power output, explicitly documented as the ceiling in
      `DECISIONS.md`

## P31 — Magic foundation

- [ ] Mana stat + HUD element (matches P26's UI language)
- [ ] Spell-slot system (hotbar-adjacent, not a menu)
- [ ] First bounded spell set (damage / utility-movement / ward / smelt-
      or-light utility)
- [ ] Enchanting minigame (sibling to existing smithing minigame)
- [ ] Wizard NPC: teaches spells, sells reagents, gives lore quests

## P32 — Construction & architecture tools

- [ ] Stairs/slabs/slopes shape system
- [ ] Symmetry/mirror placement tool
- [ ] Blueprint/schematic capture-and-ghost-place tool
- [ ] Scaffolding block
- [ ] Decoration block category (statues, banners, furniture-scale
      objects) as a data-driven mod-style registry
- [ ] Statue-carving interaction

## P33 — "Smart building" tech layer

- [ ] Elevator block (powered, vertical transport)
- [ ] Climate/AC block (cosmetic-plus-minor-perk to start)
- [ ] Computer/screen block (in-world display of tech-tree/chronicle/
      grid-status data)
- [ ] Visible wiring/conduit blocks for the power grid

## P34 — Dragons & top-tier creatures

- [ ] Dragon encounter design (rare, above Null Knight in difficulty)
- [ ] Chronicle integration for dragon fights
- [ ] Elemental creature tied to fluid systems (small roster addition)
- [ ] (Stretch, needs its own technical spike + DECISIONS.md entry)
      Dragon mount/companion

## P35 — Specialization (Paths) system

- [ ] Path standing data model on player-save
- [ ] Path-gated recipe unlocks, extending the existing tech-tree gating
      pattern
- [ ] Four starting Paths: Engineer, Architect, Battlemage, Artisan
- [ ] Respec mechanic (quest or resource sink)
- [ ] Player-to-player trading UI (extends existing villager trading UI)
- [ ] Chronicle integration: Path milestones write saga entries

## Sequencing notes

- P27–P30 (power) and P31 (magic) can proceed in parallel once P26 is
  done — they touch different crates (`lf_game` machines vs. a new spell
  system) and different content, per the layer rules in AGENTS.md.
- P32/P33 (building) can also start in parallel with P27+ — building
  tools don't depend on power tiers existing, though P33's elevator does
  depend on *some* power grid existing (P27 at minimum).
- P35 (Paths) should come last of this batch — it's the system that
  reaches across all the others (gating recipes from P27–P34), so it
  needs those recipes to exist first.
- P34 (dragons) has no hard dependency but pairs naturally with P31
  (magic) since both are Battlemage-Path-flavored content.

## Every phase inherits AGENTS.md's rules

Code + tests + vistest proof, STATE/BACKLOG/CHANGELOG/DEVLOG updated,
runtimes rebuilt and pushed, per the existing mandatory bookkeeping
section. This roadmap changes *what* gets built next, not *how*.
