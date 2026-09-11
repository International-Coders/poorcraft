# Screenshot Playtest Script

This script is the first semantic route GLM should automate.

## Scene

- seed: `semantic_factory_001`;
- viewport: `1501x801` and `1280x720`;
- quality: Mid;
- assets: starter house, NPC villager, forge, chest, resource node, map marker,
  HUD icon sheet.

## Route

1. Spawn outside starter house.
2. Capture beauty, wireframe, and anchor overlay.
3. Walk to door and open it.
4. Enter interior; assert player position is inside the interior volume.
5. Talk to NPC; assert dialogue state appears in sidecar and screenshot.
6. Use forge; insert sample ore/fuel; assert heat/progress/output state.
7. Open chest; assert inventory contract.
8. Harvest resource; assert yield and depleted/respawn state.
9. Open map; assert marker appears with hover label and ownership state.
10. Export final perf sidecar and screenshot digest.

## Pass Criteria

- all interactions report `success: true`;
- every screenshot has nonblank world and visible UI/overlay when required;
- wireframe/bounds overlays match asset ids in JSON;
- no text overlap and no clipped controls;
- route is deterministic for the same seed and build hash.
