# Input Replay And Routes

Input replay is how GLM proves the game can actually be operated.

## Route Format

Each route step declares:

- `kind`: key, mouse_move, mouse_click, wait, assert, capture, inspect;
- `target`: optional UI element or world anchor;
- `duration_ms`;
- `expected_state`;
- `failure_message`.

## Required Routes

- `route_title_mouse`: move over buttons, click New World, click Back;
- `route_new_world_seed`: type seed, randomize, reroll, capture preview;
- `route_escape_pause`: press Escape, assert pause menu, resume, no crash;
- `route_house_entry`: open door, cross threshold, assert inside volume;
- `route_npc_talk`: focus NPC, talk, assert dialogue text/state;
- `route_forge_use`: add input/fuel, start forging, assert progress/output;
- `route_asset_inspect`: cycle beauty/wireframe/bounds/anchors/materials;
- `route_gpu_markers`: render representative scene and audit markers.

## Route Law

A route failure is a game bug or a test bug. Do not hide it by weakening the
route. Fix the route only when the route was invalid for the intended behavior.
