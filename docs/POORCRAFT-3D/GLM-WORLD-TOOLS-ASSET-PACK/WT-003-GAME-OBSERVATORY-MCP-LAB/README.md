# WT-003 Game Observatory MCP Lab

This sub-pack tells GLM/Z-code to build the tools that let it see the game it
is changing. It covers local MCP-style inspection, screenshot capture,
wireframe capture, mesh/material/asset dumps, runtime state exports, input
replay, route proofs, and GPU marker evidence.

The goal is a feedback loop:

1. run the game or a headless scene;
2. drive deterministic input;
3. capture screenshots, wireframes, overlays, metrics, and JSON state;
4. compare the results to acceptance contracts;
5. fix the game when evidence disagrees.

No future GLM pass may claim a UI, asset, seed preview, NPC, machine, or GPU
optimization works without evidence from this style of observatory.

## Files

- `PROMPT_TO_GLM.txt` - paste this into Z-code for WT-003.
- `00-OBSERVATORY-BRIEF.md` - exact owner intent.
- `01-MCP-TOOLS-SPEC.md` - local tool surface GLM should expose.
- `02-RUNTIME-STATE-EXPORTS.md` - game state JSON requirements.
- `03-SCREENSHOT-WIREFRAME-OVERLAYS.md` - visual evidence modes.
- `04-INPUT-REPLAY-AND-ROUTES.md` - deterministic route driver.
- `05-ASSET-MESH-MATERIAL-DUMPS.md` - asset introspection rules.
- `06-GPU-MARKERS-TIMESTAMPS-AND-CAPTURES.md` - marker/timestamp contract.
- `07-COMPARISON-AND-REGRESSION-GATES.md` - before/after proof rules.
- `08-SECURITY-AND-SCOPE.md` - local-only constraints.
- `09-IMPLEMENTATION-SLICES.md` - ordered Z-code slices.
- `10-ACCEPTANCE-CHECKLIST.md` - done criteria.
- `11-FAILURE-MODES.md` - ways observability becomes fake.
- JSON contracts for MCP tools, runtime exports, screenshot scenes, wireframe
  exports, asset dumps, input routes, GPU markers, regression checks, and an
  evidence bundle schema.
