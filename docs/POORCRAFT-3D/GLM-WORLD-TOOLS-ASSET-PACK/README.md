# GLM World Tools And Asset Pack

Date: 2026-09-11

This is the second drop-in GLM 5.3 / Z-code handoff pack. The first pack,
`GLM-UI-REWORK-PACK`, forces the UI to become screenshotable and playable. This
pack pushes the next layer: asset generation, start menu/world seed tools, GPU
profiling hooks, wireframe/data extraction, and gameplay-semantic asset rules.

The rule is simple: assets are not decoration. A house needs a door and an
interior affordance. An NPC needs a talk anchor and dialogue state. A forge
needs a forge interaction, heat state, recipe surface, and proof. GLM must build
from a game perspective.

## Start Here

1. Read `ZCODE_WORLD_TOOLS_PROMPT.txt`.
2. Read `00-OWNER-DEMANDS.md`.
3. Execute `zcode_world_tools_task_queue.json` in order.
4. Use `interactive_object_contracts.json` and `asset_generation_backlog.json`
   as machine-readable work contracts.
5. Do not generate assets that cannot be inspected, entered, used, talked to,
   harvested, opened, crafted with, or otherwise justified by gameplay.

## First Concrete Job

Start with `WT-001-SEED-PREVIEW-HARNESS/`. It turns the broad world-tool rules
into one implementation-ready Z-code task: build a real New World seed preview
screen with seed entry, random/reroll, deterministic map preview, metadata
export, screenshot proof, and stability/change tests.

Then use `WT-002-SEMANTIC-ASSET-FACTORY-LAB/`. It turns "create way more
assets" into a gameplay-semantic asset factory with doors, interiors, NPC talk
states, forge actions, screenshot/wireframe/anchor exports, playtest routes,
and AMD/NVIDIA-informed capture gates.

Use `WT-003-GAME-OBSERVATORY-MCP-LAB/` before claiming runtime success. It
forces local MCP-style tools, input replay, screenshots, wireframes, overlays,
asset dumps, runtime state JSON, GPU markers, and regression verdicts.

Use `WT-004-FEATURE-EXPANSION-MATRIX/` to pick future gameplay breadth. It
breaks worldgen, NPCs, factions, settlements, industry, magic, survival,
combat, exploration, UI/HUD, mods, multiplayer, Steam, tools, and proof gates
into vertical slices.

Use `WT-005-ASSET-PROMPT-ATLAS/` to generate lots of original assets and UI
sprites. It provides prompt libraries, batch rules, glTF/Blender import
requirements, UI icon/key/bar contracts, and acceptance gates.

Use `WT-006-ZCODE-CONTINUOUS-RUNBOOK/` to keep long Z-code sessions moving. It
defines task order, proof-first development, failure recovery, evidence
bundles, sprint cards, bookkeeping, commit, and push discipline.

## Folder Map

- `00-OWNER-DEMANDS.md` — plain owner intent.
- `01-ZCODE-RULES.md` — mandatory GLM behavior.
- `02-START-MENU-SEED-WORLDGEN.md` — start menu, random map, seed lab, preview.
- `03-RANDOM-MAP-AND-SEED-LAB.md` — deeper seeded world preview rules.
- `04-GAMEPLAY-SEMANTIC-ASSET-LAWS.md` — house door, NPC talk, forge forge.
- `05-ASSET-BRAINSTORM-CATALOG.md` — broad asset brainstorm by gameplay system.
- `06-ASSET-GENERATION-PROMPTS.md` — prompts and acceptance rules.
- `07-AMD-NVIDIA-GPU-RESEARCH.md` — official-source vendor tooling notes.
- `08-GPU-PROFILING-AND-UPSCALING-SPEC.md` — practical implementation plan.
- `09-MCP-INSPECTOR-EXPANSION.md` — wiremesh, asset, gameplay, perf inspection.
- `10-WIREFRAME-AND-ASSET-INTROSPECTION.md` — extraction formats and gates.
- `11-TOOLS-TO-BUILD.md` — concrete local tools GLM may add.
- `12-VALIDATION-MATRIX.md` — screenshots, tests, metrics, gameplay probes.
- `13-BRAINSTORM-NOT-MENTIONED.md` — additional systems the owner did not name.
- `14-NO-FAKE-ASSETS.txt` — short rules to keep beside the editor.
- `WT-001-SEED-PREVIEW-HARNESS/` — implementation-ready first task for the
  start-menu seed preview harness.
- `WT-002-SEMANTIC-ASSET-FACTORY-LAB/` — implementation-ready second task for
  high-volume playable assets, inspection tools, and GPU capture discipline.
- `WT-003-GAME-OBSERVATORY-MCP-LAB/` — implementation-ready observability task
  for MCP-style inspection, screenshots, wireframes, state exports, input
  replay, GPU markers, and regression comparison.
- `WT-004-FEATURE-EXPANSION-MATRIX/` — implementation-ready feature matrix for
  broad gameplay systems, vertical slices, and proof gates.
- `WT-005-ASSET-PROMPT-ATLAS/` — implementation-ready prompt atlas for
  high-volume original 3D assets, UI sprites, import contracts, and batch
  acceptance.
- `WT-006-ZCODE-CONTINUOUS-RUNBOOK/` — implementation-ready runbook for
  continuous Z-code work, proof gates, failure recovery, evidence bundles, and
  green checkpoints.

## Machine Contracts

- `world_tools_manifest.json`
- `zcode_world_tools_task_queue.json`
- `asset_generation_backlog.json`
- `asset_prompt_matrix.json`
- `interactive_object_contracts.json`
- `start_menu_worldgen_contract.json`
- `gpu_vendor_tooling_contract.json`
- `screenshot_wiremesh_capture_contract.json`
- `semantic_asset_tags.schema.json`
- `tooling_backlog.json`
- `WT-003-GAME-OBSERVATORY-MCP-LAB/observatory_manifest.json`
- `WT-004-FEATURE-EXPANSION-MATRIX/feature_expansion_manifest.json`
- `WT-005-ASSET-PROMPT-ATLAS/asset_prompt_atlas_manifest.json`
- `WT-006-ZCODE-CONTINUOUS-RUNBOOK/zcode_runbook_manifest.json`

## Relationship To The Existing Packs

- Use `GLM-UI-REWORK-PACK` for UI proof discipline.
- Use `assets/UI-ASSET-IMPLEMENTATION-GUIDE.md` for existing generated UI art.
- Use this folder for world objects, start menu world generation, GPU tooling,
  gameplay affordances, asset semantics, and inspector expansion.
