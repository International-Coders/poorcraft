# POORCRAFT 3D

## Greenfield design and execution handoff

POORCRAFT is the original game. POORCRAFT 3D is a separate greenfield game
and engine effort. The original project remains preserved for later
continuation; this folder describes the new project's identity, boundaries,
and work order.

This is a design and handoff pack, not a claim that the new game is already
implemented. It is written so a future developer or z.ai session can begin
from one coherent source of intent instead of guessing from scattered ideas.

## Document authority

For POORCRAFT 3D work, use this order:

1. The current user-approved direction in this folder.
2. The task contract for the active job.
3. Repository safety and verification rules in `AGENTS.md`.
4. Current source, tests, assets, and runtime evidence.
5. Older POORCRAFT/LOREFORGE documents as historical references only.

No document in this folder proves that a feature exists. A feature is real
only when its implementation, tests, runtime behavior, and evidence agree.

## Pack map

- `00-DESIGN-CONSTITUTION.md` — the non-negotiable center of the game, its
  anti-goals, and the decision filter for every future feature.
- `01-PREFACE-AND-PRODUCT-DIRECTION.md` — approved concept and product
  promise.
- `02-DESIGN-PILLARS-AND-PROGRESSION.md` — the solo-to-empire player journey
  and the rules that keep crafting from becoming the whole game.
- `03-ENGINE-BOUNDARIES-AND-STACK.md` — lightweight engine architecture,
  language strategy, ownership, and performance principles.
- `04-TERRAIN-AND-VOXEL-WORLD.md` — hybrid smooth-ish terrain, caves,
  overhangs, editing, streaming, collision, and LOD.
- `05-WATER-FLOW-AND-ENVIRONMENT.md` — cached flow lines, local rebuilding,
  independent consumers, and future environmental simulation.
- `06-NPCS-SETTLEMENTS-AND-EMPIRE.md` — living NPCs, castles as systems,
  settlements, factions, and empire-scale play.
- `07-TECHNOLOGY-MAGIC-AND-INDUSTRY.md` — parallel progression in magic,
  machines, steam, electricity, valve-era computers, and nuclear systems.
- `08-CASTLES-FACTIONS-AND-ASSETS.md` — distant capitals, realm identity,
  modular 3D/voxel assets, and asset provenance.
- `09-MULTIPLAYER-AND-STEAM.md` — one authoritative world for solo, LAN,
  dedicated, and Steam play.
- `10-BETA-SCOPE-AND-SEQUENCING.md` — engine-first development stages,
  beta gates, and explicit non-goals.
- `11-TASK-CONTRACT-TEMPLATE.md` — the contract every implementation task
  must complete.
- `12-ZAI-EXECUTION-PROMPT.md` — paste-ready operating instructions for z.ai.
- `13-OPEN-DECISIONS.md` — questions to resolve before irreversible format or
  multiplayer commitments.
- `14-PLAYER-STORIES-AND-GAMEPLAY-LOOPS.md` — concrete solo and co-op stories
  from first landing to faction and empire play.
- `15-TERRAIN-TECHNICAL-BLUEPRINT.md` — proposed terrain data model, meshing,
  LOD, editing, collision, hydrology handoff, budgets, and test plan.
- `16-IMPLEMENTATION-WORK-BREAKDOWN.md` — ordered, bounded engine and game
  tasks with the output expected from each one.
- `17-OWNER-VISION-QUESTIONNAIRE.md` — the questions that need the owner's
  answer before the project commits to a specific game shape.
- `18-DECISION-REGISTER.md` — accepted owner decisions, assumptions, and
  decision records that prevent future sessions from losing the game's center.
- `19-ATM-RESEARCH-AND-SANDBOX-BALANCE.md` — research-derived lessons from
  All the Mods 6/7 and original balance rules for POORCRAFT 3D.
- `20-SEEDED-HISTORY-AND-CITY-GROWTH.md` — seed-driven world history, player
  origin choice, capital growth, NPC requests, satellites, and city oversight.
- `21-FIRST-PERSON-WAR-KARMA-AND-SERVER-SCALE.md` — first-person warfare,
  player/NPC construction choices, karma-driven autonomy, ideology-based new
  factions, and staged scaling toward host-selected populations.

## What this project is

POORCRAFT 3D is a voxel sandbox RPG and empire-building game. It combines:

- personal RPG play and faction consequence;
- first-person exploration, building, crafting, combat, fishing, and magic;
- natural terrain with caves, overhangs, and persistent water;
- NPCs that work, remember, perceive, report, follow, and react;
- castles, settlements, economies, diplomacy, armies, and territorial power;
- open-ended industrial or magical progression;
- a solo experience that remains deep without requiring a multiplayer group.

The broad freedom is inspired by the *kind* of breadth found in large
Minecraft modpacks such as All the Mods. It is not a request to copy their
names, code, assets, recipes, interfaces, or progression. Skyrim-like
situated role-playing, Heroes of Might and Magic III-like realm identity, and
Minecraft-like voxel construction are quality references only.

## Approval status

The product preface and the following decisions are approved for planning:

- original game: POORCRAFT;
- new game: POORCRAFT 3D;
- new world/save format, with no compatibility obligation to POORCRAFT;
- wheels and other machines independently read flow potential without
  damaging the river;
- caves and overhangs are desired in the terrain foundation;
- engine work comes before feature breadth, with all systems built against
  shared world contracts;
- language should be selected for the best complete result, not loyalty;
- the industrial branch begins in an electrical-valve/vacuum-tube era.

No implementation work is authorized by this folder alone. Each task must
still be explicitly selected and completed under the repository workflow.
