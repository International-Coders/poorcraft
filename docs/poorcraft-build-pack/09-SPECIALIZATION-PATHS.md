# Specialization Paths — Detail for Step 33

## The mechanic

Committing to a path should make continuing in it more rewarding —
eventually granting professional-grade tools a generalist can't have yet
— while never locking out other fields, just making them less optimal
than committing. This plays out differently in singleplayer (personal
mastery, tied to the chronicle) vs. multiplayer (natural division of
labor on a server).

## Four starting paths (fixed set; expand only via `DECISIONS.md`)
- **Engineer** — automation, machines, power grids (Stage F)
- **Architect** — construction, decoration, large-scale building (Stage H)
- **Battlemage** — combat + magic (Stage G)
- **Artisan** — smithing, enchanting, crafting quality/yield

## How it works
- Every player can use every basic tool/machine/spell from the start —
  the generalist floor is real, not token.
- Path-relevant actions (running machines for Engineer, building large
  structures for Architect, spellcasting/boss fights for Battlemage,
  crafting/enchanting for Artisan) build **Path standing** — a visible,
  in-fiction progress measure tied to the chronicle system, not a hidden
  XP bar.
- Crossing a standing threshold unlocks tier-gated recipes: advanced
  machines/tools/spells/building pieces, gated the same way the existing
  tech tree gates recipes behind researched eras — extend that pattern,
  don't invent a parallel one.
- A generalist spreading effort across all four paths still finishes the
  base game; they just reach professional-tier tools later, or not at
  all if they never commit enough. Commitment compounds, it doesn't gate
  the fun parts.
- **No decay**: Path standing never decays for not using another path.
  Using a path grows it; not using one costs nothing.
- **Cheap-but-not-free respec**: a quest or resource sink to redirect
  future standing gains, so committing early doesn't feel like a
  permanent trap.

## Multiplayer framing
On a shared server, paths become a natural division of labor — one player
runs the grid (Engineer), one builds the castle (Architect), one handles
dragon fights (Battlemage), one keeps gear enchanted (Artisan). A server's
best reactor design or best enchant should realistically come from
whoever committed to that path — real incentive to specialize instead of
everyone doing everything at a mediocre level. Still never mandatory — a
solo-flavored player on a shared server can use the generalist floor
exactly as in singleplayer.

## Implementation notes
- Model Path standing as data on the player-save, same persistence layer
  as stats/inventory — not a new save format.
- Gate new recipes the same way `ClientSave`/tech-tree gating already
  works.
- Tier-gated professional tools should look visibly more ornate than
  their generalist equivalent — the commitment should be craft-visible,
  not a stat-tooltip difference.

## Concrete Done check (from MASTER_PLAN.md Step 33)
A test shows a player unlocking a Path-gated recipe after crossing a
standing threshold, and confirms a generalist who never commits still has
full access to the base-tier recipe set across all four paths' domains.

## Guardrail
Four paths, not a sprawling class list. A fifth path needs a
`DECISIONS.md` entry justifying content the existing four don't already
cover.
