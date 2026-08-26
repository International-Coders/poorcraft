# Vision & Design Pillars

## One-line pitch

A voxel sandbox where the block you place, the machine you wire up, and the
spell you learn are all the same kind of progress — Minecraft's crafting and
building, an automation game's fuel-and-machine chains (water, steam, oil,
electricity, and a capped nuclear tier), and an RPG's wizards, dragons, and
lore, running on hardware a normal person already owns.

## Who this is for

- People who liked Minecraft but bounced off it once they'd built a house
  and beaten the dragon, and want a reason to keep playing for months.
- People who like automation/factory games (the "many mods" reference —
  tech-tree mods like oil/steam/nuclear power mods) but want it to be a
  *world* they live in, not a spreadsheet.
- People who like RPG sandboxes with lore, wizards, and dragons, but want
  to actually build the tower the wizard lives in, brick by brick.
- Friends the team wants to hand a build to and have them get lost in it
  for a weekend, on whatever laptop they own — not a 4090 requirement.

## Design pillars (the filter every future decision runs through)

1. **Craft-first, not menu-first.** If a system can be a block you place
   and wire up in the world, it should be — not a hidden stat on a
   character sheet. This is why the existing machine/power-field approach
   (`lf_game` generator → E-furnace/crusher/assembler) is the right shape
   to extend, not replace.
2. **One world, three flavors of progress, no wall between them.** A
   player should be able to smelt copper, wire a water wheel to a crusher,
   and learn a fire spell in the same afternoon, in the same save. Tech and
   magic are not separate game modes; see `05-MAGIC-LORE-AND-CREATURES.md`
   for how they touch.
3. **Depth rewards commitment, but never locks anyone out.** A generalist
   can always do everything at a basic level. A player who commits to a
   path gets tools and machines a generalist literally cannot build yet —
   not just faster versions. See `07-SPECIALIZATION-AND-PROGRESSION-PATHS.md`.
4. **Runs on what people actually own.** Every new visual or simulation
   system needs a "low" setting that a mid-range laptop's integrated GPU
   can hold 30+ fps on. The existing compute-shader path tracer is a
   showcase feature, not the baseline — rasterized rendering must always
   look good and run everywhere first.
5. **Stop at "a lot of fun," not "everything."** Nuclear power is the
   ceiling, not a stepping stone to sci-fi content. Dragons and wizards are
   the top of the magic ladder, not the start of a second game. See the
   scope guardrails below — this list is meant to be argued with when a
   feature request threatens to blow past it.
6. **Honest, testable, screenshot-able progress.** Every pillar above
   inherits AGENTS.md's rule: if it doesn't render in a vistest PNG and
   pass a test, it isn't done, no matter how good the markdown sounds.

## Scope guardrails (deliberately excluded, for now)

These exist so future sessions don't scope-creep the game into something
unshippable. Revisit only via `DECISIONS.md`, not by drifting into it:

- **No tech beyond "a bit past industrial."** Electricity, combustion
  engines, and a single capped nuclear reactor tier are the ceiling.
  No space travel, no lasers-as-a-tech-tree, no rockets.
- **No open-ended magic school system.** A focused set of wizard
  archetypes and a bounded spell list (see `05`) beats an infinitely
  deep skill web nobody finishes.
- **No always-online requirement.** Singleplayer and self-hosted/dedicated
  multiplayer (already built) are both first-class; no forced central
  server.
- **No monetization beyond the up-front Steam price** at this stage — see
  `10-STEAM-AND-RELEASE-PLAN.md`. No cosmetic shop, no DLC plan yet.

## Tone

Grounded-but-whimsical. Coal generators and copper wire sit next to a
wizard's tower and a chronicle that writes your saga (already built,
`lf_game` chronicle system) without either one feeling like it wandered in
from a different game. Humor is allowed; grimdark is not the default.

## Definition of "done" for the vision layer

This vision is doing its job when a new phase can be checked against it in
one sentence — e.g. "does the nuclear reactor stay a capped endgame sink,
or does it quietly start a sci-fi tech tree? Pillar 5 says cap it." If a
phase can't be checked against a pillar in one sentence, the pillar needs
sharpening, not the phase abandoning.
