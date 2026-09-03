# Proper Engine Architecture: Repair by Extraction

## Decision

LOREFORGE should remain one engine made of explicit subsystems, not a pile of
separate "engines" and not a rewrite. Keep the working wgpu renderer, voxel
storage, generation, content registry, and tests. Move simulation authority out
of the client in small, proven cuts until singleplayer and multiplayer execute
the same deterministic rules.

## Target layers

| Layer | Owns | Must not own |
|---|---|---|
| `lf_voxel` | block state, chunks, queries, persistence substrate, meshing inputs | quests, NPC policy, UI, networking |
| `lf_worldgen` | deterministic immutable terrain/features and site candidates | live settlement or fluid ticks |
| structure planner (new `lf_structure` or a focused worldgen module) | pure castle plans, modules, ports, terrain adaptation reports | renderer or client state |
| simulation core (new `lf_sim` or extracted `lf_game` services) | fixed tick, commands, events, fluids, machines, entities, time, save snapshot | window/input/egui/wgpu |
| `lf_npc` | identity, perception, memory, planner, navigation profiles, schedules | voxel storage implementation or UI |
| `lf_lore` / `lf_story` | faction policy, dialogue data, quests, world history | direct mutation of entities or inventory |
| `lf_server` | authoritative world instance, validation, persistence, replication | transport-specific UI |
| `lf_protocol` | versioned commands, snapshots, deltas, channel contracts | UDP or Steam API calls |
| UDP and `lf_steam` adapters | connection, delivery, lobby/invite identity | game-rule decisions |
| `lf_client` | input mapping, prediction/interpolation, presentation, UI, audio requests | canonical fluid/NPC/machine/inventory rules |
| `lf_engine` / `lf_assets` | rendering, GPU resources, asset compilation/lookup | gameplay policy |

The exact new crate boundary can be chosen during the first extraction job.
Semantics matter more than creating crates for their own sake.

## One authoritative simulation

The simulation host owns a `WorldInstance` containing:

```text
WorldIdentity
TickClock
VoxelWorld + protected edits
FluidRegions
BlockEntities + power networks
Players + authoritative inventories/stats
Mobs + NPCs + companions
Settlements + faction knowledge/policy state
Quest/chronicle state
Deterministic event journal
```

Singleplayer starts this host in-process. LAN/dedicated mode runs it in
`loreforge-server`. Steam wraps the same host with another transport. Client UI
sends validated commands; it does not mutate canonical state directly.

## Time, commands, and events

- Use one fixed simulation tick. Rendering interpolates independently.
- Every external mutation is a typed command with actor, tick/sequence,
  payload, and validation result.
- Accepted commands emit stable, idempotent domain events.
- Systems subscribe to events or read snapshots; combat code must not directly
  edit faction standing, UI, and network state in the same branch.
- Replaying an event ID is a no-op. This is required for reconnect and save
  recovery.
- Randomness comes from named deterministic streams `(world seed, system,
  entity/site, tick or event id)`, never wall-clock order.
- Schema versions for saves, generator, protocol, mods, structure grammar, and
  assets remain separate.

## Scheduling and simulation LOD

Every system declares cadence, active scope, and budget:

| System | Near/player-visible | Far/off-screen |
|---|---|---|
| fluids | active dirty regions at a fixed low frequency | static boundary summaries; no ocean-wide ticking |
| NPC locomotion | full path/steering/animation intent | arrival-time and schedule summaries |
| settlement economy | visible job transactions and stock changes | bounded hourly/daily aggregate ticks |
| mobs/combat | full authoritative ticks | sleep/despawn or coarse encounter state |
| power/machines | active networks with dirty propagation | elapsed-time catch-up capped by stored inputs/output |

Budgets are global as well as per-entity. A thousand NPCs must not each receive
a private unbounded A* search. F3 and server diagnostics expose queue depth,
work consumed, cache hit rate, and deferred work.

## Spatial services

Create shared read-only services instead of repeated brute-force scans:

- block and collision query;
- active-chunk and interest management;
- structure ownership and protected-edit query;
- named anchors and portal/street graph;
- nearest role/resource/bed/workplace lookup;
- line of sight and sensory query;
- entity spatial index;
- fluid flux query;
- path request queue and cache.

The client currently scans volumes for workstations and structure markers.
Castle plans should register those anchors directly when placed.

## Persistence contract

- Save from an immutable simulation snapshot or short paused tick boundary.
- Persist canonical state, not render interpolation or temporary UI state.
- Keep legacy migrations and fixtures. Never use a serde default when the old
  meaning is ambiguous.
- Persist stable IDs for entities, settlements, modules, events, commands that
  await acknowledgement, and fluid-region versions.
- Save player edits separately enough that generator or castle upgrades cannot
  overwrite them.
- A load followed by one tick must not duplicate rewards, rumors, NPC spawns,
  settlement stock, or fluid mass.

## Rendering boundary

The renderer consumes immutable frame snapshots:

```text
terrain meshes + fluid surfaces/flow vectors + entity poses + particles + UI model
```

It must not calculate wheel output, decide NPC goals, or settle network state.
Visual animation may interpolate or add cosmetic noise, but collision and
gameplay outcomes remain simulation-owned.

## Failure behavior

- A path request that exhausts its budget returns a typed failure and a safe
  fallback anchor; it never blocks the frame.
- A fluid region that exceeds its budget continues next tick deterministically
  and reports backlog; it never silently deletes water.
- A structure plan with no safe site is rejected or downgraded; it never forces
  a platform through a river or protected edit.
- A client missing authoritative state requests a baseline/resync; it does not
  invent local truth.
- A missing beta-critical asset fails validation before runtime; optional art
  uses a declared fallback.

## Migration sequence

1. Define `TickClock`, command/event IDs, deterministic ordering, and snapshot
   interfaces around existing behavior.
2. Run the current client against an in-process simulation host without visible
   feature change.
3. Move block edits and authoritative inventory transactions first.
4. Move machines/power and fluid ticks.
5. Move mobs, NPCs, companions, quests, reputation, and settlement state.
6. Make UDP and Steam consume the same host interface.
7. Remove client mutation paths only after parity tests pass.

Each cut must remain shippable and save-compatible. Avoid a long-lived second
implementation.

## Engine acceptance gates

- Singleplayer, UDP, and Steam test harnesses run the same deterministic
  command sequence and produce the same final snapshot hash.
- Fixed-tick results are independent of render frame rate and packet arrival
  batching.
- Server state can save, load, reconnect a client, and continue without
  duplicate domain events.
- Every subsystem publishes measured budget counters.
- The client can be treated as untrusted for inventory, damage, block edits,
  trades, machine output, faction changes, and quest rewards.
- Existing visual quality and world saves are preserved through migration.
