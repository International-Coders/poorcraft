# Power & Automation

## Foundation already in place

`lf_game` already has a real machine + power-field model: coal generators
power electric furnaces, crushers, and assemblers. Every system below is an
**additional power source and additional machine set feeding the same
power-field concept**, not a new parallel simulation. If the existing power
field is a single scalar/graph per network, extend it to carry a power
*type* or keep it unified — that's an implementation decision for
`DECISIONS.md`, but the player-facing rule is fixed: **machines care about
power delivered, not which source produced it**, except where a tier
explicitly requires a minimum grid quality (see Nuclear below).

## Water Age

- **Water wheel**: placed adjacent to flowing water, produces a small,
  steady, free amount of power as long as the water source keeps flowing.
  No fuel to manage — the tradeoff is low output and a location
  requirement (must be built on/near a river or coastline), which is the
  intended "build a mill" fantasy.
- **Water pressure/flow variant (optional stretch)**: a taller wheel or a
  flume built to redirect water for a mild output boost — a build-skill
  reward, not a hard requirement.
- Output tier: **lowest** of all sources. Enough to run one or two early
  machines, not a whole factory. This is the point — it's the accessible
  starter tier, matching Pillar 3 (generalist can always do a basic
  version of everything).

## Steam Age

- **Boiler**: burns wood/coal (reuses existing fuel item logic from the
  current furnace fuel system) to heat water into steam. Needs a water
  source piped or carried in.
- **Steam engine**: consumes steam from an adjacent/piped boiler, outputs
  power at a **higher rate than a single water wheel, lower than a coal
  electric generator** — steam is the "you've committed to a boiler room"
  mid-tier.
- Visual/build identity: pipes, pressure gauges, a boiler that visibly has
  a fire under it and a smoke/steam particle effect (ties to the P26
  transparency/particle audit — don't add these particles before that
  phase's exit criteria are met).

## Oil Age

- **Oil deposit**: a new fluid-bearing underground resource, found in
  specific biomes (desert/swamp-coded, to give it a placement identity
  distinct from ore veins).
- **Oil pump/derrick**: extracts crude oil into a pipe network. This is
  the first system to need real fluid transport (pipes carrying a fluid,
  not just solid items on the existing item/inventory model) — treat pipes
  as their own new subsystem, not a reskin of item transport.
- **Refinery**: converts crude oil into refined fuel (+ minor byproducts,
  optional stretch).
- **Combustion generator**: consumes refined fuel, **higher output than
  steam, comparable to or above a coal electric generator** — this is the
  payoff for building out extraction + refining infrastructure.
- Combustion engines can also directly power new *vehicles* if/when a
  vehicle system is considered — explicitly **out of scope** for the
  current roadmap (see Pillar 5); note it here only so a future
  `DECISIONS.md` entry has context if it comes up.

## Nuclear tier (capped endgame)

- **Uranium ore**: rare, deep-underground only, small vein sizes,
  intentionally scarce so reaching this tier is a real achievement, not a
  fast-track.
- **Reactor**: consumes processed fuel rods, **highest power output in the
  game, by a meaningful margin** — the entire point of climbing this far.
- **Meltdown risk, not a minigame**: an unmaintained or overloaded reactor
  has a real failure state (a localized destructive event, radiation-styled
  block/area effect that requires cleanup) — this is what makes Nuclear
  feel like an achievement with weight rather than "just the next
  generator." It should never be silently safe to spam.
- **Deliberately the last tier.** No further power tiers beyond this
  without a `DECISIONS.md` entry (Pillar 5).

## Cross-cutting automation systems these tiers all need

- **Item transport** (belts or an inserter-analog) — needed once multiple
  machines exist per base; check whether this already has a stub in
  `lf_game`/`lf_client` before designing from scratch.
- **Fluid transport** (pipes) — new, needed starting at Oil Age (water
  piping for Steam Age can reuse a minimal version of this).
- **Power storage** (a battery/accumulator block) so intermittent sources
  (water flow interruption, a reactor scram) don't instantly blackout a
  base — good candidate for a Steam-Age-unlocked block that stays useful
  forever.
- **Grid visualization**: a way to see, in-world or via a HUD overlay,
  which machines are powered vs. starved — directly serves the "craft-
  first, not menu-first" pillar; avoid a spreadsheet-style power screen.

## What this deliberately does not add

- No liquid handling beyond water/oil/steam (no acid, no exotic fluids).
- No power tier between Oil and Nuclear — the jump is meant to feel large
  and earned.
- No automated mob farms or automated combat — automation here is about
  resource/power chains, not replacing the survival/combat loop.
