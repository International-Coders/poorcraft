# Biomes & World Identity — Detail for Steps 16–19

## The actual problem to solve

Whether the number is 8 (per BACKLOG.md's line) or 30 (per STATUS.md's
claim — reconcile this in Step 1's audit), the complaint you raised is
about *feel*, not count: biomes existing in a lookup table doesn't mean a
player experiences them as different places. This stage makes each biome
a distinct place, not a palette-swapped noise function.

## Step 16 — Visual identity audit and fix

For every biome in the table:
- **Foliage/terrain color**: grass, leaves, and any biome-tinted blocks
  should have a genuinely distinct color per biome — not a single global
  green reused everywhere with only the ground texture changing.
- **Fog/sky color**: the existing atmosphere system already varies sky
  color with day/night; extend/verify it also has a biome-specific fog
  tint (a desert's dusty haze should not look like a swamp's green murk).
- **Contact sheet proof**: produce one vistest scene (or a batch of
  individual captures stitched together) showing all biomes side by
  side. Any two biomes that look near-identical in this comparison get a
  real palette or texture fix.

## Step 17 — Biome-exclusive features

Each biome needs at least one thing that only it has:
- A unique tree/plant type or a unique variant of an existing one (a
  desert cactus, a swamp mangrove-style tree, a highlands pine).
- A unique ground-cover or rock-formation block (desert has dunes/mesa
  striping, highlands has exposed rock outcrops, etc.).
- This doesn't require dozens of new assets — even a recolored/resized
  variant of an existing tree with a different placement rule counts, as
  long as it reads as "this belongs here and only here."

## Step 18 — Structure and mob placement correctness

- Cross-reference the biome table against structure placement (meadow
  huts, highlands watchtowers, desert pyramids) and mob spawn rules.
- Fix any case where a structure or mob can place in a biome it doesn't
  make sense in — this is a straightforward correctness bug, not a
  judgment call, once the biome table and structure/mob tables are
  compared directly.
- Consider (optional, note in `DECISIONS.md` if deferred) adding at least
  one new structure tied to a biome that currently has none, so every
  biome has *something* built in it somewhere in the world, not just
  terrain.

## Step 19 — Weather correctness per biome

- Confirm rain only falls in biomes that should get rain, snow only in
  cold biomes, and dry biomes get neither (or a sandstorm-style particle
  variant if that's judged worth adding — optional stretch, not required
  for Step 19's Done check).
- Vistest proof needed for at least one biome per weather category
  (rain/snow/none) showing the correct particle behavior.

## Why this order (16 → 17 → 18 → 19)

Fix what's visually wrong first (16) since it's the fastest way to make
biomes *look* different, then add exclusive content (17) so they also
*play* different, then fix placement correctness (18) so the world's
internal logic is consistent, then verify weather (19) as the final
atmosphere layer on top of a now-solid foundation.
