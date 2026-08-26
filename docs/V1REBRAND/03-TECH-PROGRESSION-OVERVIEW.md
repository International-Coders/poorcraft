# Tech Progression Overview — Stone Age to Capped Post-Industrial

## Purpose

One map of the whole tech arc, so every future phase knows where it sits.
This is the backbone that `04-POWER-AND-AUTOMATION.md` hangs its machines
on, and it plugs directly into the **research eras gated by a tech-tree
screen with live have/need costs**, which already exists in the game.

## The ages (extends the existing research-era system, doesn't replace it)

| Age | Status | Core loop | Signature power source |
|---|---|---|---|
| Stone Age | **Built** | punch tree, wooden/stone tools | none (manual) |
| Iron Age | **Built** | smithing minigame, iron tools/armor | none (manual) |
| Industrial (Electric) tier | **Built** | copper/tin/bauxite/sulfur ores, coal generators → electric furnace/crusher/assembler | coal-fired electric generator |
| **Water Age** (new) | Planned | water wheels feed early machines before coal is available/instead of it | flowing water |
| **Steam Age** (new) | Planned | boilers (wood/coal) → steam engines, precursor/parallel to electric tier | steam pressure |
| **Oil Age** (new) | Planned | drilling, refining, combustion engines, oil-fired power at higher output than coal | crude oil / refined fuel |
| **Nuclear tier** (new, capped endgame) | Planned | uranium ore (rare), reactor, meltdown risk, highest output, deliberately hard to reach | enriched fuel rods |

Water and Steam are **not required prerequisites** to reach Electric — a
player can rush coal generators exactly as today. Water and Steam exist as
*earlier, cheaper, and more manual* alternatives so a player without coal
nearby (or who just wants water wheels because they're fun to build) has a
real path forward. This mirrors the classic automation-mod pattern you
described (water/steam/oil/nuclear as parallel or escalating power tiers)
without forcing a single linear ladder.

## Why Water and Steam slot in as their own age rather than "just another
machine"

- They change *what a build looks like* (a mill on a river vs. a boiler
  room) which is exactly the "very crafty... very tall buildings" appeal
  you described — power infrastructure should be something worth
  building, not a hidden number.
- They give multiplayer servers a natural early split: a river-adjacent
  base can bootstrap early without competing for the same coal/copper
  nodes as an inland base.

## Ore/resource additions this requires

- **Uranium** (Nuclear tier, deliberately rare — see `04` for placement
  rules) alongside the existing copper/tin/bauxite/sulfur.
- **Crude oil** as a new fluid resource (see `04` for the fluid-handling
  approach — pipes, not just solid-block ore veins).
- No new ore needed for Water or Steam — they reuse existing wood/stone/
  iron/copper, which keeps early game accessible.

## Relationship to the existing research-era tech-tree screen

Water Age and Steam Age slot in as **eras the player can unlock in either
order relative to each other**, both gating machines the same way the
current tech tree gates the electric tier (live have/need costs). Oil Age
unlocks after either Steam or Electric (needs refining machinery that
assumes an existing power grid). Nuclear tier unlocks only after Oil Age
*and* a dedicated "reactor safety" research node — it should never be the
fastest path to power, only the highest ceiling.

## Guardrail (ties to Pillar 5 in `01`)

Nuclear is the last tier. Do not add a tier past it without a
`DECISIONS.md` entry. "A little bit plus" than industrial means Nuclear
caps the ladder — it is an endgame power/prestige sink (see `04`'s
meltdown-risk design), not a launchpad into sci-fi content.

## Cross-reference

- Machine specifics for each age: `04-POWER-AND-AUTOMATION.md`
- How magic-track players interact with this ladder without needing to
  fully climb it: `05-MAGIC-LORE-AND-CREATURES.md`
- How specialization changes who gets the *best* version of each age's
  tools: `07-SPECIALIZATION-AND-PROGRESSION-PATHS.md`
