# World Generation, Seeds, and Biome Identity

The seed must determine a world, not merely rename one noise field. The goal
is strict reproducibility for one generator version and meaningful diversity
between seeds at world, region, biome, river, cave, resource, and structure
levels.

## Seed contract

Define a canonical `WorldIdentity`:

```text
(seed_u64, generator_version, world_type, enabled_worldgen_mod_fingerprint)
```

- Numeric and word seeds parse to a stable `u64` on every supported platform.
- Empty seed creates a random nonconstant value and writes it to slot metadata
  before generation begins.
- Reroll changes both displayed text and the identity consumed by `WorldGen`.
- Save/load retains identity exactly; multiplayer Welcome adopts server
  identity before local terrain generation.
- Each independent feature family uses a documented salted channel derived
  from the full 64-bit seed. Never truncate all entropy to the same small seed.
- Generator changes that alter untouched chunks increment
  `GENERATOR_VERSION`; edited chunks and saves remain protected.

## Deep determinism tests

For each world type and a fixed seed corpus:

- hash height, biome, surface block, river mask, cave mask, ore samples,
  structures, and kingdom sites over a fixed coordinate lattice;
- same process run twice produces bit-identical hashes;
- serialize/load and rerun produces the same hashes;
- generation order does not matter: shuffled chunk order equals sequential;
- thread count does not matter;
- negative and large coordinates are covered;
- save generator-version mismatch behavior is explicit and tested.

Golden hashes should be few and intentional; most tests compare invariants so
legitimate generator upgrades do not require hundreds of blind snapshots.

## Different-seed diversity tests

Use at least 64 fixed seeds plus the random-seed UI sequence. Record a machine-
readable report. Initial thresholds must be calibrated from the existing
generator and then tightened based on visible quality, but the suite must catch
the failure mode "every seed looks nearly the same."

Measure:

- pairwise normalized difference of sampled height fields;
- biome histogram and Jensen-Shannon distance between seed pairs;
- river topology hash and water coverage;
- positions/types of the first structures and nearest kingdom;
- cave entrance and ore-cluster samples;
- spawn panorama perceptual/pixel hashes from at least eight seeds;
- rare-biome occurrence over a large enough search radius;
- correlations between feature channels, rejecting accidental lockstep.

A different numerical seed alone is not a pass. Require most seed pairs to
cross calibrated height and biome distances, and require rendered panoramas to
be materially different while same-seed controls remain identical.

## Macro world shape

Build diversity in layers:

1. continental land/ocean shelf and large elevation provinces;
2. climate provinces with temperature and humidity gradients;
3. warped ridges, valleys, drainage, and river networks;
4. local erosion/detail and biome-edge blending;
5. caves, resources, vegetation, landmarks, ruins, and settlements;
6. per-seed rare events and named regional identity.

Avoid independent confetti noise for every layer. Rivers follow plausible low
routes and meet larger water; settlements respond to water, slope, resources,
roads, enemies, and faction values; biome transitions preserve landmarks.

## Biome identity matrix

Every biome needs a data-backed identity with:

- climate range, elevation/slope affinity, rarity, and transition neighbors;
- top/filler/stone palette and water/sky/fog treatment;
- vegetation silhouette, density budget, and at least one negative-space rule;
- resource and food availability;
- ambient sound and weather behavior when audio is available;
- structures/encounters it enables or excludes;
- traversal or survival consequence;
- screenshot viewpoint and expected visual identifiers.

Thirty names are not thirty biomes if twenty-seven render as green grass plus
random flowers. Consolidate indistinguishable variants or deepen them. Do not
solve identity by filling every surface cell; quiet areas make landmarks read.

## Spawn and exploration quality

- Spawn is safe, breathable, above valid ground, not inside foliage/water/lava
  or a structure, and has reachable wood/stone/food within calibrated radii.
- The nearest major settlement is discoverable but not visible immediately;
  the existing 160-block castle clearance is a floor to verify, not magic.
- Early exploration exposes at least two meaningful terrain/resource choices.
- Maps reveal discovered data only; a kingdom compass points without teleport
  or global omniscience.

## Structure siting contract

Candidate evaluation is separate from building. Score a dense footprint for
slope, support, flood risk, river obstruction, road access, biome affinity,
spawn clearance, region borders, other landmarks, and protected edits. The
winning candidate is deterministic for the seed. If none qualifies, skip or
choose a documented fallback—never force a floating platform.

All builders return a placement report: changed bounds, support depth, entrance
positions, nav anchors, protected cells, and failure reason. Tests validate
every required room and entrance after terrain adaptation.

## Visual proofs

- `seed_atlas_8`: eight labeled panoramas, fixed camera, same world type.
- `seed_same_control`: two same-seed renders proven pixel-identical.
- `biome_contact_sheet`: climate families framed separately, not one wide green
  scene; each cell includes only a small nonintrusive label.
- `biome_transitions`: at least four intentional boundary pairs.
- `river_source_to_mouth`: connected route through representative terrain.
- `spawn_quality_8`: eight spawns with automatic safety/resource assertions.
- `castle_siting_8`: castles from eight seeds with terrain and road visible.

Z.ai must classify the dominant climate/terrain of each unlabeled crop and
explain which visual cues it used. Repeated confusion between two biomes is
evidence to merge or redesign them, not to add bigger labels.
