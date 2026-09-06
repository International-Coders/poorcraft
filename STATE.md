# STATE
loop_count: 402
current_milestone: POORCRAFT 3D P3D-611 — war/defense objectives + NPC war intents
last_done="loop 402 P3D-611 (P3D-600): war objectives are high-level NPC intents. pc3d_world::war: WarObjective (DefendGate/HoldWall/AttackTarget/Retreat), WarAssignment{entity, objective, path, leg, arrived}, assign_npcs (nav paths to objective cell), advance (consume one leg per tick, arrived when path exhausted). Tests: assign 2 NPCs to DefendGate, advance until all arrived, deterministic. 223 pc3d tests green (+1), p3d-smoke OK, root 474 green. Contract at docs/POORCRAFT-3D/contracts/P3D-611.md."
next_task: "P3D-612 — permanent named-NPC death, replacement/training, service loss (docs/POORCRAFT-3D/16-IMPLEMENTATION-WORK-BREAKDOWN.md): named NPCs have identity (name, role, skills), death is permanent (removed from registry), replacement NPCs are generated with lower skills, and service loss (e.g. blacksmith dies = no more crafting) is tracked. Fill the contract first. NOTE: BETA-FOUNDATION track (original game) stays parked at B04."
build: GREEN
tests: 474 passed / 0 failed (root workspace, loop 360) + 222 passed / 0 failed (poorcraft3d workspace, loop 401)
last_screenshot: poorcraft3d/apps/poorcraft3d/shots/diagnose_debug_overlay_seed2024.png
blockers: "none"
