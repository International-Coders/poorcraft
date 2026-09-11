# Implementation Brief

WT-001 ships a New World seed preview harness.

## User Value

The player should be able to start the game, choose New World, type or reroll a
seed, see a map preview, understand spawn safety, and create the world.

## Developer Value

GLM should be able to generate deterministic seed-preview screenshots and JSON
metadata for proof, regression tests, and future map tools.

## In Scope

- Seed input and random/reroll seed behavior.
- World type selector.
- Preview map data.
- Preview map render or screenshot scene.
- Biome summary.
- Spawn safety summary.
- JSON sidecar export.
- Tests and screenshot gates.

## Out Of Scope For WT-001

- Full world creation persistence UI polish.
- Multiplayer world setup.
- Full 3D orbit preview.
- All future resource/danger overlays.

Leave hooks for those, but do not block WT-001 on them.
