# POORCRAFT 3D Implementation Work Breakdown

This is a linear work order, not permission to run all tasks at once. Each ID
is one independently testable job. Complete the task contract before work and
keep the tree green between jobs.

## P3D-000 — Project boundary and truth

| ID | Outcome |
|---|---|
| P3D-001 | Create the separate workspace/worktree, new executable name, repository identity, and no-accidental-save-sharing guard. |
| P3D-002 | Define world/save/content/protocol version headers and refusal behavior for unknown versions. |
| P3D-003 | Create deterministic clock, command envelope, event journal, seed streams, and replay hash harness. |
| P3D-004 | Add profiler counters, frame-time capture, memory counters, and a baseline hardware/profile record. |
| P3D-005 | Build the first empty-world runtime and a headless smoke path. |

## P3D-100 — World substrate

| ID | Outcome |
|---|---|
| P3D-101 | Implement world coordinates, macro regions, terrain patches, bounds, and spatial queries. |
| P3D-102 | Implement versioned world header, patch store, atomic save, load, and corrupted-data rejection. |
| P3D-103 | Implement procedural macro elevation, climate, biomes, and deterministic patch regeneration. |
| P3D-104 | Add seed atlas and patch-hash proof tools. |
| P3D-105 | Implement interest management and bounded streaming queues. |
| P3D-106 | Add biome/hydrology constraint tests for mountains, coasts, river corridors, sites, and reproducible seed history. |

## P3D-200 — Hybrid terrain

| ID | Outcome |
|---|---|
| P3D-201 | Prototype and benchmark natural-surface extraction candidates on shared test scenes. |
| P3D-202 | Implement base density/material queries and a single authoritative final-solid query. |
| P3D-203 | Add caves, overhangs, cliffs, and sealed-volume correctness tests. |
| P3D-204 | Add natural-terrain edit operations, patch-local invalidation, save/reload, and compaction. |
| P3D-205 | Add construction overlay, collision, material ownership, and terrain/build priority rules. |
| P3D-206 | Implement LOD rings and seam handling; prove no visual or collision gaps. |
| P3D-207 | Implement terrain debug overlay: patch state, LOD, mesh queue, density, edits, and collision. |

## P3D-300 — Water and environment

| ID | Outcome |
|---|---|
| P3D-301 | Generate a deterministic macro watershed and river graph from terrain. |
| P3D-302 | Define persistent flow records, fixed-point units, revisions, and patch boundary ports. |
| P3D-303 | Implement local dirty-region flow rebuild after terrain/channel edits. |
| P3D-304 | Render direction, width, depth, and current from flow records without particle simulation. |
| P3D-305 | Add local reservoirs/canals/dams only if a bounded volume model is required by play. |
| P3D-306 | Implement independent flow-consumer query contract and one visible machine proof. |
| P3D-307 | Add fishing, irrigation, transport, weather, or magical liquids one at a time against the same interface. |

## P3D-400 — Movement, entities, and NPC foundations

| ID | Outcome |
|---|---|
| P3D-401 | Player controller, terrain collision, climbing/step rules, swimming, and safe spawn. |
| P3D-402 | Entity registry, spatial index, persistence, interest state, and deterministic update ordering. |
| P3D-403 | Local navigation graph from terrain patches, doors, roads, and structure anchors. |
| P3D-404 | NPC identity, roles, needs, schedule, intent, and visible activity model; require reachable Bed, Work, and Idle boxes. |
| P3D-405 | Perception, memory, report/evidence, faction-specific karma baselines, and local reaction model. |
| P3D-406 | Companion follow/wait/assist/recovery behaviors. |
| P3D-407 | Far-settlement aggregate simulation and near/far reconciliation. |

## P3D-500 — Personal gameplay

| ID | Outcome |
|---|---|
| P3D-501 | Inventory, tools, harvesting, construction, durability, and item authority. |
| P3D-502 | Food, fishing, basic survival, and contextual onboarding. |
| P3D-503 | Combat, creatures, loot, and a first dungeon/ruin loop. |
| P3D-504 | First magic path: one learnable, world-facing magical system. |
| P3D-505 | First engineering path: water/steam/valve components with visible networks. |
| P3D-506 | Data-driven recipes/items/content validation and mod boundary decision. |

## P3D-600 — Settlements, factions, and empire

| ID | Outcome |
|---|---|
| P3D-601 | Settlement plan, anchors, roads, homes, workspaces, storage, defenses, and service model. |
| P3D-602 | Castle planner and modular asset manifest; one terrain-aware capital district. |
| P3D-603 | Gates, guards, laws, alarms, witnesses, and faction access consequences. |
| P3D-604 | Production, trade, needs, and economic effects that NPCs visibly perform. |
| P3D-605 | Faction trust, diplomacy, quests, disputes, and territory control. |
| P3D-606 | Player-founded settlement, appointments, policies, and expansion. |
| P3D-607 | Army/garrison/strategic layer after the owner defines direct-control versus delegated-control play. |
| P3D-608 | Build the main-capital oversight panel backed by real hunger, housing, work, supplies, safety, project, and request data. |
| P3D-609 | Implement allied, puppet/protectorate, rival, and conquered-city relationship contracts; puppets grow autonomously until release. |
| P3D-610 | Implement player-built and NPC-commissioned civic projects through the same materials, site, pathing, ownership, save, and request contracts. |
| P3D-611 | Implement first-person war/defense objectives and high-level NPC intent without remote RTS micro-control. |
| P3D-612 | Implement permanent named-NPC death, replacement/training, service loss, and garrison-strength consequences. |
| P3D-613 | Implement personal/civic/faction/ideological karma evidence and test NPC help/refusal/report/leave/rebel outcomes. |
| P3D-614 | Implement ideology-founded player factions with original faction identity and diplomacy/recruitment/policy consequences; let ideology evolve later. |

## P3D-700 — Advanced paths and beta breadth

| ID | Outcome |
|---|---|
| P3D-701 | Valve-era computing: signals, relays/valves, programmable or configured controllers, clear debugging. |
| P3D-702 | Steam, electricity, heat, fluids, and typed machine capabilities. |
| P3D-703 | Nuclear branch with explicit safety, world consequence, and cap. |
| P3D-704 | Dragon and advanced magic branch with world/settlement/faction consequences. |
| P3D-705 | Additional faction/castle kits only after the first capital behaves deeply. |

## P3D-800 — Multiplayer and beta proof

| ID | Outcome |
|---|---|
| P3D-801 | Integrated solo host runs all canonical systems. |
| P3D-802 | Dedicated-server replication, sequence/ack, snapshots, deltas, interest, reconnect, and content handshake. |
| P3D-803 | Steam lobbies/invites/transport and real two-account proof. |
| P3D-804 | Long-running world, save/reload, terrain/water/NPC/castle soak tests. |
| P3D-805 | Complete beta player-journey automation, visual proof, accessibility, and release evidence. |
| P3D-806 | Prove host-selected scale in stages: 4, 16, 32, 64, then 128 players only if server profiling and abuse/permissions gates pass. |

## Stop conditions

Do not advance to more content when the active foundation has a known
determinism, persistence, authority, performance, collision, or visual-seam
defect. Fix the contradiction, then continue.
