# Engine Boundaries and Stack

## Primary decision

Build one lightweight engine with explicit subsystems. Do not create a
collection of unrelated mini-engines and do not begin by rewriting every
working component from POORCRAFT. Reuse ideas only after checking their
contracts; POORCRAFT 3D has a new format and may make incompatible choices.

## Preferred architecture

```text
platform/input/audio
        |
renderer + presentation
        |
client shell ---- commands ----> authoritative simulation host
                                      |
             terrain / fluids / entities / NPCs / machines / factions
                                      |
                         persistence + replication + events
```

The same simulation host must run in-process for solo play and inside a
dedicated server for multiplayer. The client presents state and submits
commands; it does not own canonical world mutations.

## Subsystem ownership

| Layer | Responsibility |
|---|---|
| Platform | window, input, timing, filesystem, audio device |
| Renderer | GPU resources, visibility, materials, lighting, post effects |
| World substrate | terrain data, region storage, edits, collision queries |
| World generation | deterministic terrain, biomes, sites, resources |
| Simulation | fixed ticks, commands, events, fluids, machines, entities |
| NPC/settlement | perception, memory, navigation, jobs, policies, alarms |
| Content | data-driven items, recipes, factions, dialogue, structures |
| Networking | versioned messages, interest, delivery, reconnect |
| Tools | asset validation, profiling, world inspection, proof scenes |

No renderer module should decide faction standing. No UI module should grant
items. No transport adapter should decide whether a river is blocked.

## Language strategy

The language choice should optimize the complete product: correctness,
iteration speed, portability, debugging, tooling, and performance together.

Recommended default:

- Rust for engine, simulation, networking, persistence, tools, and gameplay.
- Portable GPU shaders through the selected graphics abstraction.
- C++ only behind a narrow, measured boundary if profiling or a required
  library proves it necessary.
- C for small platform/ABI boundaries, not as a second gameplay language.

Do not split code across languages before a benchmark demonstrates a need. A
well-designed Rust data-oriented system is preferable to a premature FFI
boundary that multiplies build, debugging, and portability risk.

## Performance principles

- Stream regions by distance and visibility.
- Use adaptive detail: simple data far away, editable detail near players.
- Keep simulation fixed-step and budgeted.
- Separate visual meshes, collision meshes, and interaction data.
- Use dirty queues and cached summaries instead of constant global scans.
- Record counters for mesh work, fluid work, path requests, entity ticks,
  memory, and network bytes.
- Make raster rendering the baseline; high-end effects are optional.
- Benchmark representative low-end hardware before adding complexity.

## New format policy

POORCRAFT 3D uses a new world, save, content, and network format. Do not
silently load POORCRAFT saves as if they were compatible. If an importer is
ever desired, it is a separate, explicit migration tool with its own tests.

## Data ownership rules

Every persistent value needs one owner and one serialization path. For example:

| Data | Canonical owner | Reader examples |
|---|---|---|
| Terrain edits | world substrate / simulation host | renderer, collision, water, navigation |
| Flow records | environmental simulation | water renderer, machines, fishing, NPCs |
| Inventory/rewards | simulation host | client UI, crafting, trade |
| NPC knowledge | NPC/settlement simulation | dialogue, guards, faction policy |
| Castle plan/anchors | structure system | renderer, navigation, jobs, protection |
| Faction policy | world/faction simulation | guards, markets, quests, strategic map |

No two systems should independently serialize competing versions of the same
fact. That is how solo and multiplayer worlds diverge.

## Foreign-language gate

Rust remains the default until a profiling report identifies a hot subsystem
and demonstrates that an alternative implementation would improve the complete
experience enough to justify its cost. Any C or C++ addition must have a narrow
C-compatible interface, deterministic test vectors, memory ownership rules,
cross-platform build proof, and a Rust fallback or failure mode. “It might be
faster” is not enough.

The full terrain implementation contract is in
`15-TERRAIN-TECHNICAL-BLUEPRINT.md`.
