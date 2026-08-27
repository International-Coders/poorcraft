# LOREFORGE — Lore, Factions, NPCs & Visuals Design Pack

Drop this entire folder into `.zcode/plans/` (or `docs/lore/`) in the
repo. The **starter prompt** is `prompt/START_HERE.md` — paste it directly
into z.ai to begin. Every other file in this folder is a reference the AI
coder reads *during* a job when that system is being built.

## Why this pack exists

The existing game has real quest infrastructure (5-quest chain, quest log
`J`), villager NPCs with schedules and trading, a chronicle/saga system,
and 30 biomes. What it doesn't yet have is:

- A **world with political identity** — factions, opposing ideologies,
  territories — so the world feels alive with history, not just terrain.
- **Hireable companion NPCs** that grow with the player and have a
  relationship economy (trust, loyalty, wages, morale).
- **More and better textures/skins** — block faces, entity skins, mob
  skins, NPC visual identity — with correct per-block UV tiling.
- **Visual polish passes** that make the game feel distinctive rather
  than generic.

## Folder map

| File | What it covers |
|---|---|
| `prompt/START_HERE.md` | **The z.ai starter prompt** — paste this first |
| `lore/WORLD_HISTORY.md` | The world's founding lore, age timeline, named events |
| `lore/COSMOLOGY.md` | How magic, the old gods, and the age of ruin fit together |
| `factions/FACTIONS_OVERVIEW.md` | All six factions, their ideologies, territories, symbols |
| `factions/THE_ACCORD.md` | The ruling coalition (Thaumor-equivalent) — detail |
| `factions/THE_IRONBORN.md` | The industrial/guild faction — detail |
| `factions/THE_EMBER_COVENANT.md` | The magic/druid faction — detail |
| `factions/THE_FREE_HOLDS.md` | The independent traditionalist faction — detail |
| `factions/THE_ASHEN_ORDER.md` | The neutral scholar/knowledge faction — detail |
| `factions/THE_NAMELESS.md` | The antagonist/bandit faction — detail |
| `npcs/COMPANION_SYSTEM.md` | Hireable companions: trust, wages, morale, commands |
| `npcs/NPC_ROSTER.md` | Named NPC archetypes, faction ties, dialogue seeds |
| `npcs/DIALOGUE_FRAMEWORK.md` | How dialogue trees wire into the existing quest/chronicle system |
| `skins/SKIN_MANIFEST.md` | Every new texture needed: blocks, entities, NPCs, UI |
| `skins/BLOCK_SKIN_SPEC.md` | UV tiling rules + per-biome color-grade integration |
| `skins/ENTITY_SKIN_SPEC.md` | Mob, companion, and villager visual specs |
| `visuals/VISUAL_POLISH_PASS.md` | Renderer improvements: lighting, particles, post-FX, HUD |
