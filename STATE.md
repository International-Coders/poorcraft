# STATE
loop_count: 400
current_milestone: POORCRAFT 3D P3D-609 — allied/puppet/protectorate/rival/conquered relationships
last_done: "loop 400 P3D-609 (P3D-600): relationships between factions are real. pc3d_world::relationships: RelationshipKind (Allied/Puppet/Protectorate/Rival/Conquered) with per-kind autonomy (100/30/60/100/20), tribute_rate (0/500/200/0/800 basis points), and growth_pct (5/2/3/1/1 percent/day); RelationshipSystem — establish, release (puppet/protectorate/conquered -> Allied), change_kind, simulate_day (population grows by growth_pct, tribute accumulates by tribute_rate), collect_tribute (clears owed, returns total). Tests: kinds have distinct properties, puppet grows faster than rival, tribute accumulates and is collectible, release grants independence, determinism across 20 days. 219 pc3d tests green (+5), p3d-smoke OK, root 474 green. Contract at docs/POORCRAFT-3D/contracts/P3D-609.md."
next_task: "P3D-610 — player-built and NPC-commissioned civic projects through the same materials/site/pathing/ownership/save/request contracts (docs/POORCRAFT-3D/16-IMPLEMENTATION-WORK-BREAKDOWN.md). Building on P3D-608 Project. Fill the contract first. NOTE: BETA-FOUNDATION track (original game) stays parked at B04."
build: GREEN
tests: 474 passed / 0 failed (root workspace, loop 360) + 219 passed / 0 failed (poorcraft3d workspace, loop 400)
last_screenshot: poorcraft3d/apps/poorcraft3d/shots/diagnose_debug_overlay_seed2024.png
blockers: "none"
