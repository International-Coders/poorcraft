# Magic, Lore & Creatures

## Foundation already in place

The game already has villager schedules, trading, a Geode Guardian, a
Cinder Crawler, a Null Knight boss, quest objectives, and a chronicle that
writes a saga as milestones happen. Magic is an **extension of this lore
layer**, not a new game bolted on — a wizard NPC should feel like a cousin
of the existing villagers, and a dragon should feel like a bigger cousin of
the Null Knight, not an import from a different game.

## Magic system shape

- **Mana as a resource**, tracked alongside the existing stats (health/
  hunger/air) — a new HUD element in the same visual language as the
  existing hearts/hunger/air bubbles (P26 UX pass should leave room for
  this).
- **Spells learned, not crafted from a menu**: a spell is taught by a
  wizard NPC, found in a lore book (the existing lore-book concept, still
  marked deferred/unread in BACKLOG — this is a good reason to finish it),
  or unlocked via a quest, then bound to a hotbar-like "spell slot" the
  same way a tool occupies a hotbar slot. This keeps magic craft-first per
  Pillar 1 — you *find and place* your capability, you don't scroll a
  skill tree.
- **A bounded spell list, not an open tree** (Pillar 5 guardrail):
  ship a focused first set — e.g. a damage spell, a utility/movement spell,
  a defensive/ward spell, and one crafting-adjacent utility spell (a
  smelt/light spell tying magic back into the survival loop) — rather than
  designing an infinite spell system up front. Expand later only with a
  `DECISIONS.md` entry once the first set is proven fun.
- **Enchanting reuses the existing smithing-minigame pattern**: smithing
  already has an 8-material system and a minigame for tool assembly —
  enchanting an item should feel like a sibling minigame (imbuing a
  material into a finished tool/weapon), not a wholly new interaction
  model.

## Wizard NPCs

- Live in **towers**, which is a direct hook into
  `06-CONSTRUCTION-AND-ARCHITECTURE.md` — wizard towers are the kind of
  tall, ornate structure the building system should make satisfying to
  construct (whether player-built as a home base or worldgen-placed as a
  structure like the existing meadow huts/watchtowers/pyramids).
- Teach spells, sell rare reagents, and give lore-flavored quests that
  feed the existing chronicle system, so a wizard's questline reads like
  a saga entry the same way current quests do.

## Dragons

- **Rare, high-tier bosses/encounters**, not a common mob. They sit above
  the Null Knight in the existing mob-difficulty ladder as the top of the
  "magic" content, mirroring how the Nuclear reactor sits at the top of
  the "tech" ladder in `04` — both are endgame-flavored achievements, not
  everyday content.
- **Optional mount/companion payoff** for defeating or bonding with one —
  flagged here as a stretch goal requiring its own `DECISIONS.md` entry
  before implementation, since flight interacts with chunk streaming/view
  radius in ways that need a real technical spike first.
- Dragon encounters should produce chronicle entries the same way the
  Null Knight fight presumably will — big fights are lore, automatically.

## How magic meets tech (the actual point of the mashup)

- A handful of **crossover items**: e.g., an enchanted tool that also has
  a durability/power stat, or a magic-infused block that can be wired into
  a power grid for a unique effect (a torch-replacement magic light source
  that needs no fuel, a warding block that protects machines from mob
  damage). These should be **rare, a few per age**, not a whole parallel
  crafting tree — the goal is a spark of "oh, that's clever," not a second
  automation game.
- Magic does **not** replace or trivialize the power tiers in `04` — a
  spell that "just gives infinite power" would undercut the entire point
  of building a water wheel or a reactor. Keep magic's power-adjacent
  effects small, situational, or short-duration.

## Creature roster additions (beyond dragons)

- Fill out the existing 6-mob roster with a small number of magic-flavored
  additions (an elemental tied to the new fluid systems in `04`, e.g., a
  steam/water-adjacent creature) rather than a large new bestiary —
  Pillar 5 applies to creatures too.

## Guardrail

No open-ended magic "classes" beyond the specialization paths defined in
`07-SPECIALIZATION-AND-PROGRESSION-PATHS.md` (Battlemage is a path, not a
separate leveling system layered on top of it).
