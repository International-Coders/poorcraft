# Specialization & Progression Paths

## The ask, stated plainly

Depending on the path a player chooses — and this plays out differently in
singleplayer vs. multiplayer — committing to that path should make
continuing in it *more* rewarding: eventually granting professional-grade
tools a beginner or a generalist simply cannot have yet. But it should stay
fun, and progressing in *other* fields should still be possible if the
player wants, just less optimal than committing. This document is the
concrete design for that.

## The core mechanic: Paths, not classes

A **Path** is a track of mastery in one of the game's pillars. Proposed
starting set (expand only via `DECISIONS.md`, per Pillar 5):

- **Engineer** — automation, machines, power grids (`04`)
- **Architect** — construction, decoration, large-scale building (`06`)
- **Battlemage** — combat + magic (`05`), the RPG-adventuring path
- **Artisan** — smithing, enchanting, crafting quality/yield

A player is **never locked into one Path at character creation**. Instead:

- Every player can use every basic tool/machine/spell in the game from the
  start (Pillar 3: generalist floor is real, not token).
- Doing Path-relevant actions repeatedly (running machines for Engineer,
  building large structures for Architect, spellcasting/boss fights for
  Battlemage, crafting/enchanting for Artisan) builds **Path standing** in
  that track — a visible, in-fiction progress measure (a mastery rank tied
  to the chronicle/quest system, not a hidden XP bar).
- Crossing a standing threshold unlocks **tier-gated recipes**: advanced
  machines, tools, spells, or building pieces that are recipe-locked behind
  that Path's standing, the same way the current tech tree gates recipes
  behind researched eras. Mechanically this is "the tech tree, but personal
  and Path-shaped" rather than a whole new system — extend the existing
  research-era gating pattern instead of inventing a parallel one.
- **A generalist who spreads effort across all four Paths still finishes
  the base game** — they just reach the *professional-tier* tools later,
  or not at all if they never commit enough to any one Path. This is the
  "more of a strength if you continue going" ask: commitment compounds,
  it doesn't gate the fun parts.

## Why this isn't a punishment system

- No Path standing ever decays for *not* using another Path — a Battlemage
  who never touches Engineering isn't penalized, they just won't unlock
  Engineer's professional tier. Committing to one path costs nothing;
  it's a use-it-to-grow-it system, not a spend-it-or-lose-it one.
- The generalist floor (every basic tool/spell/machine available to
  everyone) means a new player, or a player who just wants to build a
  castle for a weekend without "picking a class," never hits a hard wall.
- Respec is cheap-but-not-free: a quest or resource sink that lets a
  player redirect future standing gains, so a player who commits to
  Engineer and later wants to pivot to Battlemage isn't stuck — see also
  `08` for how this plays out with a shared-world party.

## Singleplayer framing

- Path standing is entirely personal. A singleplayer player is optimizing
  their own pace against their own goals — the system exists to give long
  personal projects (a factory, a castle, a wizard's saga) a sense of
  earned mastery, tied into the existing chronicle system so *this
  character's* Path story is literally written into their saga log.

## Multiplayer framing

- On a shared server, Paths become a natural **division of labor**: one
  player runs the power grid (Engineer), one builds the castle
  (Architect), one handles the dragon fights (Battlemage), one keeps
  everyone's gear enchanted (Artisan) — see `08` for the economy/trading
  implications.
- This is where "more locked in" has real teeth: a server's *best*
  reactor design or *best* enchant should realistically come from whoever
  committed to that Path, giving every player on a team a reason to
  specialize rather than everyone doing everything at a mediocre level.
- Still never mandatory — a solo-flavored player on a multiplayer server
  can ignore Paths and use the generalist floor exactly as in
  singleplayer; Paths are an opportunity, not a server rule.

## Implementation notes for later phases

- Model Path standing as data on the player-save (same persistence layer
  as stats/inventory), not a new save format.
- Gate new recipes the same way `ClientSave`/tech-tree gating already
  works — this is additive to an existing pattern, not a new one.
- Tier-gated professional tools should look visibly different/more
  ornate than their generalist equivalent (ties to Pillar 1: the
  commitment is craft-visible, not a stat tooltip).

## Guardrail

Four Paths, not a sprawling class list (Pillar 5). If a fifth Path is
requested later, it needs a `DECISIONS.md` entry explaining what
distinct professional-tier content it unlocks that the existing four
don't already cover.
