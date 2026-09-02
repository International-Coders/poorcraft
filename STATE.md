# STATE
loop_count: 338
current_milestone: Complete — authored-depth raster assets, articulated NPCs, item impostors
last_done: "loop 338 authored-depth assets: every generated base/mod/entity/item/CTM atlas layer now has a linear RGB tangent-space normal map and the raster shader uses it for cheap directional relief without ray tracing; seven villager-job outfits plus a wayfarer network-player skin replace block-textured people; shared six-part humanoids support yaw/gait/crouch for villagers, companions and remote players; non-block drops use their real alpha-cutout inventory art on crossed double-sided cards while blocks stay cubes. Fixed two proof-found pre-existing bugs: CTM sentinel layers 165+ collided with real atlas art (moved to 4096+) and entity push_cube ignored its position. entity_skins now close-proves 8 articulated characters + 8 readable items. New staged plan: docs/ASSET-RENDERING-PLAN.md. 353 tests, 83-scene vistest, smoke green; terrain_vista perf p50 50.2ms / p95 50.6ms."
next_task: "Asset plan Stage 1: per-part humanoid UV atlas + job/faction attachments + deliberate NPC facing + first/third-person local player, with turntable and distance proofs (docs/ASSET-RENDERING-PLAN.md)."
build: GREEN
tests: 353 passed / 0 failed (loop 338)
last_screenshot: shots/vistest_entity_skins.png
blockers: "none"
