# STATE
loop_count: 381
current_milestone: POORCRAFT 3D P3D-305 — bounded conserved reservoirs (gate: YES)
last_done: "loop 381 P3D-305 (P3D-300): GATE DECIDED YES — the dam story needs water that HOLDS, so the minimal bounded volume model is in: hydro::Reservoirs (per-region Reservoir{capacity_kl, volume_kl} in thousand-liter fixed-point; capacity terrain-derived from local elevation range) with fill() (conserved retention, overflow routes DOWNSTREAM through a bounded chain walk, final spill returned) and drain() (never negative). Conservation law test-proven: poured - spilled == total retained across a huge pour; fill/drain round-trip to zero; determinism. 114 pc3d tests green (+2), p3d-smoke OK, root 474 green. Contract at docs/POORCRAFT-3D/contracts/P3D-305.md."
next_task: "P3D-306 — independent flow-consumer query contract and one visible machine proof (docs/POORCRAFT-3D/16-IMPLEMENTATION-WORK-BREAKDOWN.md): the consumer interface (D-007) — a flow-potential query (discharge + slope + wetness at a site) that machines/wheels/fishing/NPCs read WITHOUT weakening the river, plus one visible proof: a waterwheel site rendered/annotated from real records (atlas overlay pattern). Fill the contract first. NOTE: BETA-FOUNDATION track (original game) stays parked at B04."
build: GREEN
tests: 474 passed / 0 failed (root workspace, loop 360) + 112 passed / 0 failed (poorcraft3d workspace, loop 380)
last_screenshot: poorcraft3d/apps/poorcraft3d/shots/flow_map_seed1.png
blockers: "none"
