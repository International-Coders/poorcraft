# Visual Reset Execution Queue

Execute exactly one task at a time, in order. A task cannot be skipped because
the old simulation roadmap says it is complete.

## STATUS — the queue's own record (update after every task; evidence lives in DEVLOG.md and tasks/render_execution_queue.json)

| Task | Status | One-line evidence |
|---|---|---|
| R3DV-001 windowed renderer bootstrap | DONE 2026-09-06 | winit window + wgpu surface + nonuniform swapchain screenshot; 9 tests; visual PASS |
| R3DV-002 first-person camera + 3D primitive | DONE 2026-09-07 | perspective camera in P3D meters, depth, sun lighting, mouse-look/WASD; face-flip + occlusion + 19.5% parallax proofs |
| R3DV-003 asset manifest validator | DONE 2026-09-07 | pc3d_assets validates beta_critical_assets.json (19 rows); duplicate/brand/LOD/final-path rejections tested (13 tests) |
| R3DV-004 construction cell mesh | DONE 2026-09-07 | culled faces from the host overlay via read-only binding; host-command rock→sand edit remeshed exactly 1 patch; before/after windowed PNGs, pixel flip 0.22, live F/R mode |
| R3DV-005 natural terrain mesh prototype | DONE 2026-09-07 | culled faces straight from final_solid (exact cross-patch culling, ~7 ms/patch); hill slope + cliff material separation + cave interior + overhang underside all probe-verified from the live window; bake-off reproduced (heightfield family wins) |
| R3DV-006 streamed terrain, LOD, real mesh work | DONE 2026-09-07 | TerrainStreamer consumes the world's interest rings/BoundedQueue/LOD bands: caps held every frame (teleport vista = 57 frames at cap 2), GPU budget with farthest-first eviction, frustum culling (77,875 culled draws in the walk), top-shell identical across LODs (no cracks); 3-waypoint windowed walk, raycast probes PASS |
| R3DV-007 river and water mesh | DONE 2026-09-07 | transparent strips from FlowRecords (direction/width/depth/speed all record-driven); control-render transparency proof; along-vs-across current deltas; dam edit remeshed 5/281 sections with the far course pixel-identical; windowed before/after PNGs inspected PASS |
| R3DV-008 castle/city module renderer | DONE 2026-09-07 | silhouettes from plan_capital + the settlement plan (town one region east so the authorities never collide): gate/wall/tower/house/workshop all probe-verified (gate delta 0.29, roofs from above where rings occlude walls); collision = footprint union, nav anchors = ports/beds/work grounded on local terrain; materials from the pc3d_assets registry (manifest-coverage-tested); windowed city + gate PNGs inspected PASS |
| R3DV-009 NPC renderer and city anchors | DONE 2026-09-07 | resident/worker/guard ARE NpcBrains bound to grounded plan anchors, stepped on the real NavPatch; presence proven by control-diff at sim positions (0.10–0.31), resident torso color pixel-exact; guard spear + worker tool only while Working; Bed/Work/Idle inspect frames from anchor materials; 5 windowed captures inspected PASS |
| R3DV-010 materials, lighting, quality tiers | NEXT | — |
| R3DV-011..R3DV-012 | BLOCKED (queue order) | — |

Reset rule in force: "POORCRAFT 3D is playable" stays forbidden until the
R3DV-011 vertical slice passes with human review (see README.md).

## Stage R0 — Make a window and prove it is real

### R3DV-001 — Windowed renderer bootstrap

Create `pc3d_render`, add `winit`/`wgpu`, open a resizable window, render a
clear scene, handle surface resize/loss, and add a screenshot test. Done only
when a nonuniform GPU screenshot comes from the windowed renderer.

### R3DV-002 — First-person camera and scene primitive

Render an indexed cube/ground primitive with depth, directional light, mouse
look, keyboard movement, resize, and a simple HUD/debug line. Camera position
uses P3D world coordinates.

### R3DV-003 — Asset manifest validator

Implement `pc3d_assets` to parse and validate the supplied JSON schema/data.
Validate source policy, ids, material paths/procedural placeholders, consumers,
proof scenes, collision, and LOD fields. Add invalid-manifest tests.

## Stage R1 — Render actual terrain and building state

### R3DV-004 — Construction cell mesh

Generate culled/greedy block faces from the existing construction overlay.
Place/remove through the host command path and prove a changed GPU mesh appears
without direct client world mutation.

### R3DV-005 — Natural terrain mesh prototype

Use the existing terrain query to render hills, cliff, cave, and overhang. Run
the documented surface-extraction comparison and select the real algorithm
from evidence. Add visual/collision consistency tests.

### R3DV-006 — Streamed terrain, LOD, and real mesh work

Consume existing LOD/stream queues to mesh/upload visible patches with bounded
per-frame work, culling, invalidation, and seam handling. Show actual queue,
GPU-memory, and uploaded-patch counters.

### R3DV-007 — River and water mesh

Generate transparent water from actual flow records; show current direction,
depth/width, and local update after a channel edit. Add a visual proof distinct
from a 2D flow map.

## Stage R2 — Make a living place visible

### R3DV-008 — Castle/city module renderer

Render original placeholder castle modules from existing capital/kit plans,
with bounds/collision/navigation anchors. Prove a gate, wall, tower, house,
and workshop silhouette.

### R3DV-009 — NPC renderer and city anchors

Render resident/worker/guard entities using their actual simulated locations,
paths, roles, and Bed/Work/Idle boxes. Provide inspect-mode labels/overlays.

### R3DV-010 — Materials, lighting, and asset quality tiers

Add atlas/material binding, sunlight/ambient, water transparency, cutouts,
low/medium/high quality tiers, and memory limits. All beta-critical asset rows
must have a real consumer.

## Stage R3 — Honest playable proof

### R3DV-011 — First 3D vertical slice

Assemble the deterministic showcase seed in `01-3D-VERTICAL-SLICE.md`. A human
can walk, cave, river, build, city, NPC, inspect-box, save, and reload.

### R3DV-012 — Visual regression and performance gates

Add real windowed screenshot scenes with semantic pixel checks, manually review
them, run low/mid/high hardware settings, save/reload proof, and a short
movement/build smoke. Only after this task may other systems receive new
visual content.

## Hard stop

Do not work on another pure simulation feature, new faction, new machine,
advanced magic, or multiplayer scale claim before R3DV-012 has a green
windowed visual slice. Fix renderer/mesh/asset failures first.
