# STATE
loop_count: 332
current_milestone: Complete — ai-npc-assets Sections A-G (mob AI, NPCs, connected textures, tooling)
last_done: "loop 332 ai-npc-assets: B mob behaviour state machine (7 states, 11 transitions, LOS, faction aggro radius, group aggro, A* pathfinder) + client standing/group wiring; C NPC enriched schedules + activity states + reactions + persistent memory; A black-square hardening (Live-RT invalidation on world load, empty-batch guards, no_black_square scene + black-run assertion on 8 scenes); D --smoke headless logic flag + hardened make smoke + mob_ai_visible/npc_schedule_time scenes; E connected textures (derived 47-tile CTM table, 8 blocks, second strip texture + shader branch, per-tile UVs, connected_texture_uv_3x3 + scene); F xtask gen-texture/gen-ctm/gen-all-textures (deterministic, tested); fixed pre-existing NaN in the voxel raycast (axis-aligned rays from boundary origins died after one cell). 338 tests, 82/82 vistest, smoke (logic+GUI) OK."
next_task: "Master plan Phase B assets (per docs/MASTER-PLAN.md) or master-plan Phase C world/survival; candidate small items queued in BACKLOG loop-332 deferred notes (TOML schedule overrides, hostile-NPC join-the-fight, persistent honored-ack flag)."
build: GREEN
tests: 338 passed / 0 failed (loop 332)
last_screenshot: shots/vistest_connected_textures_grass_3x3.png
blockers: "none"
