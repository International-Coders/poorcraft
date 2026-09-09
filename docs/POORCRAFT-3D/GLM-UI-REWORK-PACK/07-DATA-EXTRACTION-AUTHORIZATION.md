# Data Extraction Authorization

The owner authorizes agents to extract runtime data from POORCRAFT 3D for the
purpose of making the game understandable, testable, and visually better.

Allowed exports:

- UI tree and rectangles.
- Draw order.
- Text bounds.
- Active screen.
- Input capture state.
- Key binding map.
- Bar values and hotbar state.
- Camera pose.
- Player pose.
- Visible terrain chunks or surface patches.
- Mesh/wireframe summaries.
- Object instance counts.
- NPC/crowd state summaries.
- Save/load paths used during tests.
- Frame time, GPU memory estimates, draw calls, triangle counts.
- Screenshot file paths and image statistics.

Allowed implementation methods:

- local debug modules;
- temporary CLI commands;
- local MCP-style command server;
- JSON/CSV exports;
- renderer debug overlays;
- mod/plugin hooks;
- test-only save directories.

Restrictions:

- Do not upload private saves or screenshots to external services.
- Do not weaken tests to make proofs pass.
- Do not leave debug UI visible by default in owner gameplay.
