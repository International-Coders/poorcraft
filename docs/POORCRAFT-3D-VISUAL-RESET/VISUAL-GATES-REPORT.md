# R3DV-012 — Visual Regression & Performance Gates Report

Audit run: 2026-09-07, host darwin 24.6.0 x64 (host-iGPU).
Command: `make p3d-visual-gates` — the full battery re-runs EVERY windowed
proof in sequence and fails loudly on any regression. Raw console capture:
`poorcraft3d/apps/poorcraft3d/shots/gates_report.txt`.

## Result: ALL 9 GATES PASS

| Gate | Scene command | Frame record (this run) | Semantic evidence |
|---|---|---|---|
| windowed-3d-axes | `--play-shot` | p50 9.33 ms · p95 11.10 ms · 119 fps | face-flip + occlusion + parallax probes; live resize recovery |
| terrain-scenes | `--play-terrain` | p50 9.75 ms · p95 11.32 ms · 112 fps | query-derived probes on hill/cliff/cave+overhang |
| stream-walk | `--play-stream` | p50 1.12 ms · p95 2.41 ms · 745 fps steady | 3 waypoints raycast probes; 360 patches (101 full/259 mid) at caps (3 mesh/frame), 24555/24576 KB budget, 77875 frustum-culled draws |
| river-water | `--play-water` | p50 9.63 ms · p95 11.66 ms · 111 fps | control-render transparency blend, along-vs-across current deltas, dam edit remeshed 5/281 sections |
| castle-city | `--play-city` | p50 10.22 ms · p95 19.32 ms · 92 fps | 13 silhouette kinds, wall/tower/roof probes, gate opening delta 0.29 |
| npc-cast | `--play-npcs` | p50 10.00 ms · p95 21.57 ms · 93 fps | torso sky-deltas 0.49/0.75/0.75, inspect boxes frame |
| quality-tiers | `--play-quality` | p50 2.00 ms · p95 3.51 ms · 434 fps | Low/Mid/High: 42/80/102 patches, 2831/5468/6908 KB GPU — tier scaling asserted |
| vertical-slice | `--play-slice` | p50 9.86 ms · p95 10.06 ms · 112 fps | showcase seed 22 (found 2.4 s): city+wheel+water+cast overview, cave interior, first-person build proof |
| asset-manifest | `--validate-assets` | — | 19 beta-critical rows validated; consumer audit maps every row to a real module |

Save/reload proof and the movement/build smoke live in the test suite
(`slice::tests::journey_*`: walk → cave → build through HostCommand →
save → reload pixel-identical) and in the walkable slice
(`make p3d-slice-live`).

## Human review (this audit)

Every regenerated gate PNG was visually inspected during this run and in
each task's original pass: windowed_3d (perspective banner scene),
terrain hill/cliff/cave, stream walk 1–3, water before/after dam, city
overview + gate close-up, NPC town + close-ups + inspect overview,
quality low/mid/high, slice showcase + cave + built. All PASS — no holes,
no seams, no missing layers, roles and silhouettes readable.

## Performance record

- All eight rendered gates hold **≥ 92 fps average** on the host iGPU
  (p95 ≤ 21.6 ms worst case, city+npc close-up loads).
- Streaming steady-state is **~1.1 ms/frame** after load; load work is
  capped at 3 meshes/frame by law.
- Quality tiers scale honestly: 42→80→102 patches (2.8→6.9 MB GPU);
  the full 320 m Lod ring needs ~8 s of frames at the honest cap.
- Showcase seed search: 2.4 s (bounded).

## Known limits (honest, consolidated)

1. Terrain is the culled-cell representation (bake-off-selected); smooth
   dual-contouring is a future quality pass. No carved river beds — water
   strips follow the analytic surface and can be buried by hills between
   cross-sections.
2. Interior lighting is sun-lambert + hemisphere ambient — no shadows,
   AO, or light propagation; caves are lit, not dark.
3. The material atlas is a single procedural noise tile (high tier), not a
   per-material tiled atlas; no authored textures or models yet.
4. NPCs are the showcase trio (blocky prisms, stride offset only — no
   walk animation, no death/garrison-gap visualization, no in-world
   labels). The plan's anchors overlap its own footprints (documented
   data truth); the cast grounds on nearest open cells.
5. Player: no jump; no held-item/hand rendering; fixed dawn sky (no
   day/night); the slice loads a static terrain band around the gate plus
   streaming rather than the full streamed world; the town is a real
   256 m walk from the capital.
6. Kit faction modules render as generic blocks (no faction skins);
   machines are the one wheel silhouette; sea water is not meshed.
7. "POORCRAFT 3D is playable" remains gated on the OWNER's manual pass:
   `make p3d-slice-live` (the machine evidence is complete).

## Regression use

`make p3d-visual-gates` is the standing regression battery — run it after
any renderer/sim change that could affect visuals; the report file and the
gates' exit code are the verdict.
