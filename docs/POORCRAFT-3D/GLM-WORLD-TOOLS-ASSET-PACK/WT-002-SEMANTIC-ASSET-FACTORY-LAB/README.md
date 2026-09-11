# WT-002 Semantic Asset Factory Lab

This sub-pack turns "make way more assets" into a repeatable Z-code factory
with game semantics, capture evidence, GPU profiling notes, and import rules.

The goal is not a prettier pile of props. The goal is a playable asset system:

- every building has enterable space, doorway clearance, collision, lighting,
  nav, and interaction anchors;
- every NPC has talk/follow/trade/hostile states as appropriate;
- every machine has inputs, outputs, state, animation hints, and proof that the
  interaction exists;
- every generated asset has screenshots, wireframe, bounds, material tags,
  LOD/collision metadata, and ownership/provenance notes;
- every tool that claims visual improvement writes data GLM can inspect.

## Files

- `PROMPT_TO_GLM.txt` - paste this into Z-code for WT-002.
- `00-OWNER-ASSET-FACTORY-BRIEF.md` - exact job scope.
- `01-SEMANTIC-ASSET-RULEBOOK.md` - laws for playable assets.
- `02-ASSET-CATALOG-BRAINSTORM.md` - broad brainstorm beyond the named cases.
- `03-INTERACTION-ANCHOR-SPECS.md` - anchors for doors, talk, forge, loot, etc.
- `04-INSPECTOR-WIREFRAME-CAPTURE.md` - required capture/export tools.
- `05-MATERIAL-LOD-COLLISION-RULES.md` - runtime import constraints.
- `06-GPU-PROFILING-UPSCALING-GATES.md` - AMD/NVIDIA-informed profiling gates.
- `07-TOOL-COMMANDS-TO-BUILD.md` - concrete CLI/MCP-style tools.
- `08-ZCODE-IMPLEMENTATION-SLICES.md` - ordered implementation slices.
- `09-ACCEPTANCE-CHECKLIST.md` - done criteria.
- `10-FAILURE-MODES.md` - ways this work becomes fake.
- `11-SCREENSHOT-PLAYTEST-SCRIPT.md` - mandatory proof route.
- JSON contracts for the manifest, queue, catalog, affordances, inspector
  exports, GPU captures, tool commands, playtest evidence, and budgets.
