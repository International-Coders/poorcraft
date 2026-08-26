# Construction & Architecture

## Purpose

This is the "very crafty in a way like Minecraft — very tall buildings,
very fun buildings... a full castle full of statues, or a modern building
with elevators and air conditioning and electricity and computers" ask,
made concrete. Building is a first-class pillar (Pillar 1), not a side
effect of having blocks.

## Building tools (quality-of-life, unlocked early)

- **Symmetry/mirror placement** for builders working on towers/castles —
  place one side, mirror the other.
- **Stairs, slabs, and slopes** as a real shape system (not just full
  cubes) — needed for anything that isn't a Minecraft-style flat-topped
  build, including wizard towers and modern buildings alike.
- **Blueprint/schematic tool**: capture a built structure and place a
  ghost-preview copy elsewhere — huge value for anyone building a "tall
  building" repeatedly (multiple floors of the same layout) and for
  multiplayer servers sharing designs.
- **Scaffolding block**: temporary climbable block for building tall
  structures safely, removes itself or is easily bulk-removed when done.

## Decoration & statuary

- A real decoration-block category (distinct from functional blocks):
  statues, banners, rugs, furniture-scale objects, and stained/colored
  glass variants — this is what turns "a pile of stone blocks" into "a
  castle." Should be data-driven the same way the mod block registry
  already is, so mod authors can add decoration sets without touching
  engine code.
- **Statue crafting**: a dedicated statue-carving interaction (could reuse
  the smithing-minigame *pattern* — a focused mini-interaction rather than
  "place item from inventory") so building a statue feels like a real
  craft, not just a decorative block placement.

## Tall-building support (technical, ties to P26)

- Verify chunk streaming and lighting hold up at height — a 40+ block
  tall tower needs correct light propagation top-to-bottom and shouldn't
  cause streaming stutter. This is a direct dependency on the P26
  rendering fixes (lighting/AO, chunk-border consistency) landing first.
- Consider a soft world-height increase if the current column height is a
  limiting factor for "very tall buildings" — a `DECISIONS.md`-level call
  once P26 profiling data exists.

## "Modern building" tech layer

This is the part of the tech tree that's about *quality of life inside a
building* rather than raw production, and it's what makes the industrial
tier feel like it pays off at home, not just in a factory:

- **Elevator block**: vertical transport between floors, powered by the
  existing/extended power grid (`04`) — a great "electric tier" reward
  that's about living somewhere nice, not just producing more.
- **Air conditioning / climate block**: a powered comfort block — could
  tie into a lightweight comfort/temperature stat if one exists or is
  added, or purely as an ambient/visual + minor gameplay perk (e.g. faster
  regen indoors) if a full temperature system is out of scope for now
  (recommend starting cosmetic-plus-minor-perk, not a full survival
  temperature simulation, per Pillar 5).
- **Computer/screen block**: an interactive decorative block that can
  display something useful and in-fiction — e.g., a readout of the
  player's tech-tree progress, chronicle log, or power-grid status
  (nice tie-in to the "grid visualization" need from `04`) rendered on an
  in-world screen rather than only in a menu. This is a good showcase for
  "craft-first" done well — a UI panel that is also a placeable object.
- **Wiring as a visible, buildable thing**: if the power grid needs
  conduits, make placing them feel like electrical wiring a house, not an
  invisible network — visually distinct wire/pipe blocks that a player
  runs through walls and ceilings on purpose.

## Guardrail

Modern-building tech should read as "a wizard-tower-and-castle world where
one wing got wired for electricity," not "suddenly this is a different,
modern-day game." Keep material/aesthetic language (stone, timber, copper,
iron) consistent even on "modern" blocks — a computer block should look
like it belongs in this world, not like a Windows 95 icon dropped in.

## Cross-reference

- Power sources these blocks draw from: `04-POWER-AND-AUTOMATION.md`
- Wizard towers and worldgen-placed magic structures: `05`
- Rendering prerequisites for tall/complex builds: `02`
