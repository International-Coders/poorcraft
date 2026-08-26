# Power & Automation — Detail for Steps 23–27

## Foundation already in place

`lf_game` already has coal generators powering electric furnaces,
crushers, and assemblers through a power-field model. Every tier below
extends that same model with a new source and machine set — machines care
about power delivered, not which source produced it, except where a tier
explicitly needs a minimum grid quality (Nuclear).

## Step 23 — Water Age
- **Water wheel**: placed adjacent to flowing water, produces a small,
  steady, free amount of power with no fuel management. Lowest output
  tier of any source — the accessible starter, matching "generalist can
  always do a basic version of everything."
- **Battery/power-storage block**: needed now because water flow can be
  interrupted (a nearby edit, a frozen/dried source) — smooths supply
  before this becomes a bigger problem at Steam/Oil scale.

## Step 24 — Steam Age
- **Water piping (minimal)**: enough fluid-transport plumbing to feed a
  boiler; this is the seed of the fuller pipe system Oil Age needs.
- **Boiler**: burns wood/coal (reuse the existing furnace fuel-item logic)
  to produce steam.
- **Steam engine**: consumes steam, outputs power above a single water
  wheel, below a coal electric generator — the "you've committed to a
  boiler room" mid-tier.
- **Visual identity**: pipes, a visible firebox, and steam/smoke particles
  — only add these particles after Step 8's transparency/sort audit is
  done, so they don't reintroduce a rendering bug.

## Step 25 — Oil Age
- **Oil deposit**: new fluid resource, desert/swamp-biome-gated for a
  distinct worldgen identity from ore veins.
- **Full pipe/fluid transport**: extends Step 24's minimal water piping
  into a real system — oil pump/derrick extracts into pipes, a refinery
  converts crude oil to refined fuel.
- **Combustion generator**: consumes refined fuel, output higher than
  steam, comparable to or above a coal electric generator — payoff for
  building extraction + refining infrastructure.
- **Grid visualization**: an in-world or HUD overlay showing which
  machines are powered vs. starved (feeds directly into the Step 32
  computer/screen block, which can display this same data).

## Step 26 — Nuclear tier (capped, endgame)
- **Uranium ore**: rare, deep-underground only, small veins — reaching
  this tier should be a real achievement.
- **Reactor**: consumes processed fuel rods, highest power output in the
  game by a meaningful margin.
- **Meltdown risk**: an unmaintained/overloaded reactor has a real failure
  state — a localized destructive event with an area effect requiring
  cleanup, not a silently-safe machine to spam. This is what gives the
  tier weight.
- **This is the last power tier.** Do not add anything above it without a
  written `DECISIONS.md` entry explaining what new gameplay it enables
  that Nuclear doesn't already cover.

## Step 27 — Shared transport backbone
- Confirm belts/inserter-analog (items) and pipes (fluids, from Step 25)
  are genuinely shared primitives feeding multiple machine types across
  ages, not reimplemented per-tier. A test showing the same transport
  block feeding, say, both a Steam-Age boiler and an Oil-Age refinery is
  the concrete proof this was done right.

## What this deliberately does not add
No liquid types beyond water/oil/steam. No power tier between Oil and
Nuclear. No automated combat or automated mob farms — automation here is
resource/power chains, not a replacement for the survival/combat loop.
