# LOREFORGE Beta Foundation

This folder is the durable statement of what LOREFORGE is becoming. It was
written after reviewing the repository's 89 Markdown files (16,856 lines), the
current lore data, the active state/backlog/history, and the implemented water,
worldgen, NPC, renderer, asset, protocol, server, and Steam paths at loop 356.

The project is a broad, functioning alpha. The beta effort is not a rewrite and
it is not a content-count race. It is an integration pass that turns existing
features into a coherent game: flowing water must carry measurable energy;
castles must be rare, distant, distinctive places; residents must understand
and react to what happens around them; and multiplayer must run the same
authoritative simulation as singleplayer.

## Authority and document order

When documents disagree, use this order:

1. `AGENTS.md` for repository safety, verification, bookkeeping, and shipping.
2. `STATE.md` for the next executable job and the latest proven counts.
3. This folder for current product direction and engine contracts.
4. `docs/NIGHTLY-BETA/` for detailed acceptance criteria that this folder has
   not explicitly superseded.
5. `DECISIONS.md`, source code, tests, and current runtime evidence for what is
   actually implemented.
6. `BACKLOG.md` and `docs/ROADMAP-100.md` as inventories, not promises.
7. `docs/V1REBRAND/`, `docs/poorcraft-build-pack/`, `docs/ui-world-craft/`,
   `docs/lore-and-visuals/`, and `docs/ai-npc-assets/` as historical design and
   implementation references.

No old `[x]` overrides a failing current build or a contradiction in source.
No item in this folder is "done" merely because it is documented.

## Pack map

- [`01-PRODUCT-VISION.md`](01-PRODUCT-VISION.md) — the player fantasy, design
  pillars, core loop, and meaning of "less Minecraft-ish."
- [`02-CURRENT-ENGINE-AUDIT.md`](02-CURRENT-ENGINE-AUDIT.md) — source-grounded
  alpha diagnosis and the working foundations that must be preserved.
- [`03-PROPER-ENGINE-ARCHITECTURE.md`](03-PROPER-ENGINE-ARCHITECTURE.md) — how
  to repair the engine without replacing it with another monolith.
- [`04-WATER-RIVERS-AND-MECHANICAL-POWER.md`](04-WATER-RIVERS-AND-MECHANICAL-POWER.md)
  — conserved local water, current forces, dams, flumes, and water-wheel torque.
- [`05-CASTLES-FACTIONS-AND-3D-ASSETS.md`](05-CASTLES-FACTIONS-AND-3D-ASSETS.md)
  — realm spacing, terrain-aware castle grammar, settlement gameplay, and the
  asset pipeline.
- [`06-NPC-LIFE-REACTION-AND-FOLLOWERS.md`](06-NPC-LIFE-REACTION-AND-FOLLOWERS.md)
  — navigation, perception, memory, castle reactions, and companion behavior.
- [`07-MULTIPLAYER-AND-STEAM-AUTHORITY.md`](07-MULTIPLAYER-AND-STEAM-AUTHORITY.md)
  — one authoritative simulation behind local, UDP, and Steam transports.
- [`08-BETA-DELIVERY-ROADMAP.md`](08-BETA-DELIVERY-ROADMAP.md) — ordered,
  independently shippable jobs and beta gates.
- [`09-GLM-EXECUTION-GUIDE.md`](09-GLM-EXECUTION-GUIDE.md) — token-conscious
  instructions for GLM 5.3 and GLM 5.3 Flash sessions.
- [`10-DOCUMENT-AUDIT-AND-MIGRATION.md`](10-DOCUMENT-AUDIT-AND-MIGRATION.md) —
  what the older packs contribute and how to prevent plan drift.

## The beta promise

A beta player can learn the game without external instructions, establish a
home, harness a real river, meet a distant realm whose people and architecture
are unmistakable, form or damage relationships through witnessed action,
travel with a useful companion, and continue the same save alone or with
friends. Save/reload, world identity, NPC intent, fluid state, machines, and
multiplayer authority survive without duplication or silent loss.

The label remains alpha or pre-beta until the end-to-end gate in
`08-BETA-DELIVERY-ROADMAP.md` passes. Breadth, screenshots, and test counts are
evidence inputs—not substitutes for that journey.

## Originality rule

The desired feeling may draw from Skyrim's situated faction role-playing,
Heroes of Might and Magic III's strongly differentiated realm strategy, and
Minecraft's legible voxel building. LOREFORGE must not copy names, layouts,
quests, creatures, visual assets, UI, text, music, progression tables, or code.
The references describe qualities: political consequence, strategic place
identity, and a readable buildable world. All shipped expression is original.
