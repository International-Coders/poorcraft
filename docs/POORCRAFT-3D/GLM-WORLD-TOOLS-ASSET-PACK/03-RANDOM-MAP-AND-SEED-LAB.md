# Random Map And Seed Lab

The seed lab is a tool for the player and for GLM.

## Player-Facing Seed Lab

- Preview a world before creating it.
- Reroll seed.
- Lock seed.
- Copy seed.
- Toggle preview layers.
- Compare two seeds.
- Show why a spawn is safe or unsafe.

## Developer Seed Lab

- Generate seed corpus screenshots.
- Export biome histogram.
- Export height histogram.
- Export spawn safety report.
- Export route candidates: town, cave, water, forest, resource cluster.
- Export diversity score.
- Export screenshot montage.

## Tests

- Same seed is stable.
- Different seed changes at least one preview layer.
- Spawn point is not underwater, not inside rock, not on severe slope.
- Preview metadata agrees with world generation.
- Reroll creates a seed not equal to the current seed.

## Acceptance

Do not call seed generation good until screenshots and JSON reports show the
world differs in ways a player can understand.
