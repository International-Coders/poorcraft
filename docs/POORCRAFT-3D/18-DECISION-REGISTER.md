# POORCRAFT 3D Decision Register

This is the durable memory of decisions made by the owner. Update it whenever
an answer changes an implementation boundary, data format, progression rule,
or player expectation. A future model must read this file before proposing a
conflicting plan.

## Accepted decisions

| ID | Decision | Status | Consequence |
|---|---|---|---|
| D-001 | POORCRAFT is the original game; POORCRAFT 3D is a new project. | Accepted | Preserve the old project; use separate identity and workspace. |
| D-002 | POORCRAFT 3D uses a brand-new format. | Accepted | No implicit save/content/protocol compatibility with POORCRAFT. |
| D-003 | The goal is a lightweight proprietary engine, not Unity/Unreal plus voxel plugins. | Accepted | Build explicit, testable engine subsystems; profile before foreign-language work. |
| D-004 | Terrain should be natural and smooth-ish, but still lightweight and buildable. | Accepted | Use hybrid terrain/construction representation; avoid uniform high-detail simulation. |
| D-005 | Caves and overhangs are wanted from the terrain foundation. | Accepted | A heightmap-only terrain design is insufficient. |
| D-006 | Water should remember stable flow and rebuild locally after changes. | Accepted | Use cached flow records plus bounded local volume simulation, not global particles. |
| D-007 | Water consumers are independent and do not reduce river flow merely by operating. | Accepted | Devices query flow potential non-destructively; geometry may still reroute water. |
| D-008 | Water wheels are one example, not the core purpose of the game. | Accepted | Build a general environmental/machine interface; do not center the roadmap on wheels. |
| D-009 | The game needs broad optional paths comparable in spirit to large Minecraft modpacks. | Accepted | Magic, technology, exploration, factions, and empire must cross-connect. |
| D-010 | The game needs Skyrim-like RPG/faction presence and Heroes-like castle/empire play. | Accepted | NPCs, settlements, castles, diplomacy, and territory are core—not decoration. |
| D-011 | Solo play must be deeply satisfying. | Accepted | Design a full solo authority and long-term progression path before relying on co-op. |
| D-012 | Technology includes electrical valves/vacuum-tube computers. | Accepted | Industrial content starts in a distinctive valve-era aesthetic, not generic modern tech. |
| D-013 | The best implementation language should be chosen by product outcome. | Accepted | Rust is default; C/C++ require measured justification and narrow boundaries. |
| D-014 | Each seed supplies the old world; the player creates the important new history. | Accepted | No mandatory faction campaign; independent, merchant, magical, industrial, and ruler paths are valid. |
| D-015 | The opening pleasure is choosing the best place to start, like reading a Minecraft world. | Accepted | Start safely, then let the player choose a meaningful base site from seed-specific geography. |
| D-016 | Terrain geography is seed- and biome-driven: mountains, coasts, plains, forests, rivers, wetlands, and dense river forests form coherent regions. | Accepted | World generation must use biome/hydrology constraints, not disconnected random decoration. |
| D-017 | No progression path is categorically strongest. | Accepted | Balance through capability, materials, infrastructure, risk, and social/world consequence. |
| D-018 | Open-core nuclear systems can be powerful but make nearby areas hazardous without shielding and protection. | Accepted | Nuclear gameplay must include radiation, siting, protection, and civic/ecological consequences. |
| D-019 | The player directly manages one chosen main city through physical construction plus a visual needs/oversight panel. | Accepted | City UI reflects real population, hunger, housing, supplies, work, safety, and projects. |
| D-020 | NPCs communicate needs through requests for material, homes, improvements, safety, and services. | Accepted | Requests are soft opportunity threads with visible world outcomes, not mandatory quest rails. |
| D-021 | Friendly cities can be autonomous puppet/protectorate states that keep their own faction identity; expansion beyond the main city is delegated. | Accepted | Empire uses relationships and influence rather than direct micromanagement of every city. |
| D-022 | First-beta priorities are empire/city systems, broad crafting and block construction, castles, NPCs, systems management, and biome/world quality. | Accepted | Beta scope must protect these pillars before optional catalog expansion. |
| D-023 | The game is an open sandbox with many long-term activity angles, not a single activity loop. | Accepted | The roadmap must keep magic, machines, underground, construction, trade, factions, and city growth connected. |
| D-024 | Wars and major danger are experienced in first person. | Accepted | Leadership uses high-level intents and real NPC logistics, never a remote RTS replacement for battlefield play. |
| D-025 | Civic structures may be player-built or commissioned to NPC builders; personal builds remain fully player-created. | Accepted | City projects use the same world/material/ownership rules in both paths. |
| D-026 | Major capitals should be roughly 10–25 minutes apart on foot, while exact placement remains seed-driven. | Accepted | Generator needs travel-time-aware route/terrain constraints and intermediate discoveries. |
| D-027 | A player-created faction chooses an ideology from existing faction values; later factions may expand the ideological palette. | Accepted | Ideology affects diplomacy, NPC expectations, law, architecture, recruitment, and city management. |
| D-028 | NPC autonomy—help, refusal, leaving, rebellion, betrayal—depends on a karma system. | Accepted | Build personal, civic, and ideological evidence systems before final behavioral scoring. |
| D-029 | The host chooses population limits; up to 128 players is an allowed target only when the server can sustain it. | Accepted | Scale must be staged and measured; no universal 128-player claim. |
| D-030 | Each faction has its own karma baseline. | Accepted | NPC/city reactions use faction values plus personal/civic knowledge, not one global morality score. |
| D-031 | A player starts a faction with one chosen ideology and evolves it later through play. | Accepted | Ideology data must support initial choice, later change, and consequences. |
| D-032 | A conquered city grows as an autonomous puppet state until the player releases it. | Accepted | It has delegated city growth and its own identity rather than becoming a second fully managed capital. |
| D-033 | The player must designate Bed, Work, and Idle boxes for city NPC life. | Accepted | NPC residency, jobs, schedules, construction validation, and navigation depend on three physical reachable anchors. |
| D-034 | Named NPC death is permanent and weakens the city/garrison. | Accepted | Death persistence, skill/service loss, replacement, and military readiness are core city consequences. |

## Proposed defaults pending owner confirmation

| ID | Proposal | Why it is useful | Owner answer needed |
|---|---|---|---|
| P-001 | 16 x 16 x 16 meter terrain patches with one-meter base cells. | Matches the stated subdivision idea and bounds rebuild work. | Confirm or change desired physical scale. |
| P-002 | 256 x 256 meter macro regions. | Gives a stable unit for climate, watersheds, and site placement. | Confirm after world-scale/travel answer. |
| P-003 | Major-site placement is determined by the seed within coherent terrain/route/resource constraints. | Preserves world variety while preventing nonsensical capitals. | Accepted with a 10–25 minute foot-travel target. |
| P-004 | First playable vertical slice includes terrain, water, building, one NPC settlement, one magic or industry branch, and solo save/load. | Proves the central loop without trying to ship all systems at once. | Confirm the minimum implementation slice after beta priorities are decomposed. |
| P-005 | City oversight is a visual panel linked to a physical, directly managed capital. | Matches the approved management vision. | Define which metrics/actions are exposed first. |
| P-006 | Strategic map/overhead planning role. | Affects UI, travel, army, and satellite interaction. | Decide how much management occurs outside first person. |
| P-007 | Exact karma values and moral axes. | Determines which actions cause trust, fear, loyalty, rebellion, or ideological conflict. | Define the actions/values the world should judge. |
| P-008 | Original names and final identities for faction archetypes. | Keeps the desired breadth while avoiding a one-for-one external faction recreation. | Approve original faction name/identity direction. |

## Decision record template

Copy this section for each new settled choice:

```text
### D-XXX — Short decision title

Date:
Owner statement:
Decision:
Why:
Affected documents/systems:
What this rules out:
Implementation task that makes it real:
Evidence after implementation:
```

## Rule for AI sessions

An AI may propose alternatives, risks, or smaller prototypes. It may not
silently replace an Accepted decision. If an Accepted decision becomes too
expensive or technically contradictory, the AI must explain the evidence and
ask the owner whether to revise the decision register.
