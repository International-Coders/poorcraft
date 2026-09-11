# WT-008 Data Extraction Plugin Lab

This sub-pack formalizes the owner's authorization: GLM/Z-code may create
local mods, plugins, debug endpoints, and exporters to extract data from
POORCRAFT 3D so it can understand the game it is changing.

The goal is local, deterministic extraction:

- asset manifests;
- mesh and wireframe data;
- materials and textures;
- UI layout and input focus;
- worldgen/seed preview data;
- player/NPC/machine state;
- performance counters;
- traces/metrics/logs;
- screenshot and evidence bundles.

No exporter may hide a failing feature. Extraction exists to make the game
better and more testable.

## Files

- `PROMPT_TO_GLM.txt` - paste this into Z-code for WT-008.
- `00-PLUGIN-BRIEF.md` - scope and owner authorization.
- `01-LOCAL-PLUGIN-ARCHITECTURE.md` - safe local plugin shapes.
- `02-EXPORT-SCHEMAS.md` - JSON/CSV/PNG/GLB/OBJ/trace export rules.
- `03-TELEMETRY-TRACE-METRIC-LOGS.md` - local telemetry rules.
- `04-MOD-SAMPLE-PACKS.md` - test mods/plugins GLM may create.
- `05-INSPECTOR-ENDPOINTS.md` - local MCP-style endpoint list.
- `06-DATA-SECURITY-SCOPE.md` - local-only safety constraints.
- `07-ACCEPTANCE-CHECKLIST.md` - done criteria.
- `08-FAILURE-MODES.md` - ways extraction becomes fake or unsafe.
- JSON contracts for plugin manifest, exporters, endpoint surface, telemetry,
  mod sample packs, safety, and evidence output.
