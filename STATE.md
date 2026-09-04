# STATE
loop_count: 382
current_milestone: POORCRAFT 3D P3D-306 — flow-consumer query + wheel-site proof
last_done: "loop 382 P3D-306 (P3D-300): the D-007 consumer contract. hydro::FlowPotential{region, discharge, slope_per_mille, wetness, reservoir_kl, viable} via RiverGraph::flow_potential_at — THE pure read: querying 1089 times changed no discharge/reservoir/graph (purity test). best_wheel_site: viable-only candidates (real water AND real slope), maximizes discharge x slope, deterministic; THE visible machine proof: the flow map stamps a white wheel marker at the best site (render_flow_map), marker test pins it at the winning region. THE SLOPE-UNITS BUG caught by the viability test: per-mille slope computed as meters*1000/256000mm truncated to 0 for every plausible drop (correct: meters x 1e6 / mm) — the fix made viable sites exist at all and bumped all three slope formulas. 117 pc3d tests green (+3), p3d-smoke OK, root 474 green. Contract at docs/POORCRAFT-3D/contracts/P3D-306.md (docs/POORCRAFT-3D/contracts/P3D-306.md written alongside flow-map work)."
next_task: "P3D-307 — fishing, irrigation, transport, weather, or magical liquids one at a time against the same interface (docs/POORCRAFT-3D/16-IMPLEMENTATION-WORK-BREAKDOWN.md): the first consumer BUILT on the flow-consumer contract — fishing: fish stocks per river region derived from discharge+wetness, a catch query consuming stock without weakening the river (D-007), and deterministic restock. Fill the contract first. NOTE: BETA-FOUNDATION track (original game) stays parked at B04."
build: GREEN
tests: 474 passed / 0 failed (root workspace, loop 360) + 112 passed / 0 failed (poorcraft3d workspace, loop 380)
last_screenshot: poorcraft3d/apps/poorcraft3d/shots/flow_map_seed1.png
blockers: "none"
