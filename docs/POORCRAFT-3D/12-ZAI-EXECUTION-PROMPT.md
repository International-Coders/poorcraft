# z.ai Execution Prompt

Use the following prompt when starting a POORCRAFT 3D implementation session.

```text
You are working on POORCRAFT 3D, a new greenfield voxel RPG and
empire-building game. POORCRAFT is the original game and must remain
preserved for later continuation. POORCRAFT 3D uses a brand-new world/save/
content/network format and must not silently claim compatibility.

Read AGENTS.md and every file in docs/POORCRAFT-3D/ before changing code.
Inspect the current source, tests, assets, git status, and runtime evidence.
Treat implementation and tests as the source of truth; do not claim a feature
because it is described in Markdown.

The product combines original voxel construction, natural smooth-ish terrain,
caves, overhangs, persistent cached water flow, personal RPG progression,
magic, valve-era electrical technology, machines, fishing, factions,
castles, NPC societies, settlements, and empire management. It is inspired by
the breadth of large Minecraft modpacks, Skyrim-like situated role-playing,
and Heroes of Might and Magic III-like realm identity, but copy no protected
names, code, assets, layouts, text, or progression.

Work engine-first and one bounded task at a time. Use shared world contracts.
The simulation host owns canonical state; the renderer/client presents it and
sends commands. Solo play runs the same host in-process that dedicated and
Steam multiplayer use behind transport adapters.

Terrain should use a hybrid representation: streamed regions, natural
height/density fields, adaptive volumetric detail for caves and overhangs,
and explicit edits for voxel construction. A 16x16 meter patch may be a
streaming/editing unit, but it must not force every surface to be equally
blocky or equally expensive.

Water is a persistent cached flow network, not a world-wide particle loop.
Store channel geometry, direction, slope, discharge, capacity, revision, and
interaction radius. Rebuild dirty connected sections after terrain edits;
preserve unaffected sections. Wheels, pumps, boilers, and magical devices read
flow potential independently. They do not consume or weaken the river unless a
future approved design explicitly changes that rule.

NPCs must perceive, remember, navigate, work, report, follow, and react based
on credible knowledge. Castles are living multi-scale places with gates,
services, jobs, laws, defenses, faction identity, and consequences. Do not
solve missing world behavior with more decorative assets.

Before implementation, fill docs/POORCRAFT-3D/11-TASK-CONTRACT-TEMPLATE.md for
the selected task. Preserve unrelated work. Add tests with the behavior.
Run the repository verification ladder required by AGENTS.md. Inspect every
changed visual proof. Record honest limitations and stop at a green task
boundary. Never make a broad “make the whole game beta” diff.
```

## Required reading order for a new z.ai session

1. `00-DESIGN-CONSTITUTION.md`
2. `01-PREFACE-AND-PRODUCT-DIRECTION.md`
3. `14-PLAYER-STORIES-AND-GAMEPLAY-LOOPS.md`
4. the relevant subsystem contract, especially
   `15-TERRAIN-TECHNICAL-BLUEPRINT.md` for terrain work
5. `16-IMPLEMENTATION-WORK-BREAKDOWN.md`
6. `11-TASK-CONTRACT-TEMPLATE.md`
7. `17-OWNER-VISION-QUESTIONNAIRE.md` for unresolved decisions

Do not resolve an open owner decision by silently inventing a preference. Mark
the task blocked at the exact decision, offer the smallest viable options, and
continue only on work that is unaffected.

## Flash versus full-model use

Use a faster model for bounded inventory, call-site mapping, repetitive tests,
validators, and verification. Use the strongest available model for engine
boundaries, persistence, deterministic simulation, terrain formats, NPC
authority, and multiplayer. Do not let model speed choose architecture.
