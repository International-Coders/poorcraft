# Start Menu, Seed, And World Generation Spec

The owner wants a start menu that can generate a random map like the old game,
with seed generation and meaningful preview.

## Required Start Menu

- Build hash/version visible.
- Play current world.
- New World.
- Load World.
- Settings.
- Tools/Debug, hidden behind a deliberate toggle.
- Quit.

## New World Screen

- World name.
- Seed text field.
- Random seed button.
- Dice/reroll button.
- World type selector: Normal, Wild, Mountain, Island, Superflat test, Debug.
- Preview map.
- Preview camera orbit.
- Biome summary.
- Spawn safety summary.
- Estimated features: town, river, cave, forest, mountain, coast, landmark.
- Create button disabled until preview validates.

## Seed Rules

- Empty seed creates a random seed and displays it.
- Text seed hashes stably.
- Numeric seed parses exactly.
- Same seed + same world type must reproduce preview and world.
- Different seeds must visibly differ.
- Seed metadata must be saved with the world slot.

## Preview Rules

The preview must prove more than colors:

- topographic height;
- water/rivers/coasts;
- biome areas;
- spawn point;
- nearest settlement/castle/landmark if present;
- danger or resource hints once systems exist.

The preview may be low-resolution, but it must be deterministic and screenshoted.
