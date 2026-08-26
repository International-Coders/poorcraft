# Magic, Lore Creatures & Dragons — Detail for Steps 28–30

## Foundation already in place

Villager schedules, trading, named mobs (Geode Guardian, Cinder Crawler,
Null Knight), quest objectives, and the chronicle system already exist.
Magic extends this lore layer — a wizard should feel like a cousin of
existing villagers, a dragon a bigger cousin of the Null Knight, not an
import from a different game.

## Step 28 — Magic foundation
- **Mana stat + HUD**: a new resource tracked alongside health/hunger/air,
  shown in the same visual language as those existing HUD elements (Step
  12's design system applies here directly).
- **Spell-slot system**: a spell is *found and equipped* like a tool — a
  hotbar-adjacent slot, not a menu-driven ability list. Craft-first, per
  the project's own pattern of making capability something you place/
  carry, not something you toggle in a screen.
- **A bounded first spell set** — ship exactly these four to start, don't
  design an open tree:
  1. A damage spell (offense)
  2. A movement/utility spell (mobility)
  3. A ward/defensive spell
  4. A crafting-adjacent utility spell (e.g. remote-light or a
     quick-smelt effect) that ties magic back into the survival/
     industrial loop instead of keeping it siloed
- **Enchanting minigame**: model it as a sibling to the existing smithing
  minigame (which already has an 8-material system and tool-assembly
  interaction) — imbuing a material into a finished item, not a new kind
  of interaction invented from scratch.

## Step 29 — Wizard NPCs and towers
- Wizard NPCs live in towers (a natural hook into Step 31/32's building
  tools — whether player-built or worldgen-placed the way meadow huts/
  watchtowers/pyramids already are).
- They teach spells, sell reagents, and give lore-linked quests that
  produce chronicle entries the same way other significant events do.

## Step 30 — Dragons and roster additions
- **Rare, high-tier boss** — sits above the Null Knight in difficulty,
  the top of the "magic" content the way Nuclear is the top of "tech."
  Not a common mob; an event.
- **Chronicle integration is mandatory**, not optional — a dragon fight
  must produce a real saga entry (this is also the concrete Done check
  for Step 30 in `MASTER_PLAN.md`).
- **One elemental creature tied to the fluid systems** from Stage F (a
  steam/water-adjacent creature) — a small, deliberate roster addition,
  not a new bestiary.
- **Mount/companion payoff is explicitly a stretch goal**, not part of
  Step 30's Done check — flight interacts with chunk streaming/view
  radius in ways that need a technical spike and a `DECISIONS.md` entry
  before committing to it.

## How magic meets tech (the actual point of the mashup)
A small number of crossover items — an enchanted tool with a durability/
power-adjacent stat, a magic light source needing no fuel, a warding block
protecting machines from mob damage — a few per age, not a whole parallel
crafting tree. Magic must never trivialize the power tiers from Stage F
(no spell that substitutes for a generator); keep magic's power-adjacent
effects small, situational, or short-duration.

## Guardrail
No open-ended magic "classes" beyond the four paths defined in
`09-SPECIALIZATION-PATHS.md` — Battlemage is a path built on top of this
spell system, not a second leveling system layered over it.
