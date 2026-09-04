# Beta Scope and Sequencing

## Development rule

Build the engine and shared contracts first, then add systems linearly. Do
not jump between isolated content features while the world, NPC, and authority
foundations are still ambiguous.

## Stages

### Stage 0 — Project boundary

Create a separate POORCRAFT 3D workspace or worktree, new format identifiers,
build targets, profiling baseline, and a minimal runtime shell. Preserve the
original POORCRAFT project.

### Stage 1 — Engine foundation

Implement timing, input, rendering, asset loading, world coordinates, region
streaming, persistence interfaces, diagnostics, and a deterministic test host.

### Stage 2 — Terrain foundation

Implement the hybrid terrain representation, caves, overhangs, collision,
adaptive meshing, edits, save/reload, and visibility/LOD budgets.

### Stage 3 — Environmental simulation

Add cached water flow, local rebuilds, channels, dams, machine queries,
weather, and bounded environmental events.

### Stage 4 — Entity and NPC foundation

Add players, creatures, navigation, perception, memory, schedules, jobs,
followers, settlement anchors, and authoritative persistence.

### Stage 5 — Personal gameplay

Add building, crafting, tools, combat, fishing, gathering, survival, magic,
and initial machines against the shared contracts.

### Stage 6 — Settlements and realms

Add castles, faction identities, roads, economies, laws, diplomacy, garrisons,
and empire-scale decisions.

### Stage 7 — Multiplayer productization

Move every canonical system behind the host, add replication/reconnect, wire
Steam lobbies and invites, and run the real mixed-system journey with two
accounts.

### Stage 8 — Beta hardening

Complete critical assets, animation, performance, save migration policy,
accessibility, onboarding, soak tests, visual proofs, packaging, and known
issues.

## Beta gates

| Gate | Done when |
|---|---|
| Engine | A small world streams, renders, saves, reloads, and profiles cleanly |
| Terrain | Natural surface, caves, overhangs, edits, and collision work together |
| Environment | A changed river rebuilds locally and supports independent consumers |
| Living world | One castle has working residents, jobs, navigation, and reactions |
| Progression | A solo player can pursue at least two distinct deep paths |
| Empire | Settlement choices affect territory, resources, and faction relations |
| Multiplayer | Solo and shared sessions use one authoritative simulation |
| Beta candidate | The complete player journey passes state, visual, performance, and release checks |

## Explicit non-goals for the first beta

- full physical fluid dynamics;
- every imagined faction, machine, spell, creature, or dimension;
- a live language model controlling NPC dialogue at runtime;
- public matchmaking or Steam store promises without external readiness;
- replacing working systems merely to use a different language;
- unlimited distant simulation at player-level detail.

The project remains alpha until the complete journey works, regardless of
feature count or screenshot quality.

## How a stage becomes implementation

Stages are not giant tickets. Select the next P3D task from
`16-IMPLEMENTATION-WORK-BREAKDOWN.md`, fill the contract in
`11-TASK-CONTRACT-TEMPLATE.md`, implement it with tests, and close it only
with evidence. A stage is complete when all of its tasks meet their stated
gates—not when a model says the code “looks done.”

## Breadth safeguard

Do not start advanced nuclear systems, large armies, many factions, dozens of
dragons, or complex Steam features while the terrain/world authority/NPC
foundation is not reliable. The purpose of the sequence is not to make the
project less ambitious; it is to ensure the ambitious parts can actually
connect and run.

## Approved beta pillars

The beta must protect these priorities:

1. Seeded biomes and a natural, editable voxel world.
2. Broad block/building/crafting capability, including castle construction.
3. NPCs, city needs, systems management, and one directly managed capital.
4. Factions, castles, empire relationships, and meaningful player independence.
5. Multiple viable long-term activity paths: magic, machines, underground,
   exploration, trade, and settlement growth.

This does not mean every possible machine, dragon, faction, or city mechanic
ships in beta. It means no beta plan may cut the listed pillars down to a
decorative demo.
