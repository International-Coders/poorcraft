# Observatory Brief

The game cannot be improved by guessing. WT-003 creates an evidence layer so
GLM can inspect what the game actually rendered, what the UI layout actually
was, what meshes and materials were used, what assets were interactive, and
what the player/NPC/machine state was during a proof route.

## Owner Intent

- GLM is authorized to add local debug commands, local MCP-style endpoints,
  local JSON exporters, screenshot tools, wireframe modes, and inspection
  overlays.
- The tools must be deterministic and runnable by automation.
- Captures must be tied to exact build hash, seed, viewport, scene, camera,
  input route, and command.
- Screenshots and JSON must agree. If the JSON says the player entered a
  house, the screenshot/position/volume must prove it.
- Observability must help make the game better, not become an excuse to avoid
  gameplay work.

## Primary Use Cases

- detect broken mouse/menu/input;
- verify the start menu seed preview is deterministic;
- prove a house doorway is enterable;
- prove an NPC talk state opens and is readable;
- prove a forge consumes input and creates output;
- prove generated assets have mesh/material/bounds/anchors;
- prove GPU pass markers exist before AMD/NVIDIA capture claims.
