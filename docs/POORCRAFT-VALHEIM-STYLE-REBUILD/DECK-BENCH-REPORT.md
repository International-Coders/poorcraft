# POORCRAFT 3D — Steam Deck quality report (NWR-010)

- date: unix+1795637948s
- host hardware: Apple host iGPU (documented evidence machine; the contract targets Steam Deck) (documented; Deck numbers are the contract's target, this host is the evidence machine)
- window: 800x500
- runs: 3
- host CPU probe ns (start/end per tier, band x1.4): low 534024/533878, mid 594682/534164, high 533802/533934

## The quality contract

| lever | low | mid | high |
|---|---|---|---|
| terrain LOD rings | [Full] | [Full, Lod] | [Full, Lod, Macro] |
| mesh work / frame | 1 | 2 | 3 |
| GPU byte budget | 8 MB | 24 MB | 48 MB |
| shadow map | off | 1024 | 2048 (16 MB) |
| fog density | 0.0040 | 0.0055 | 0.0070 |
| material detail | 0.00 | 0.55 | 0.85 |
| water glint | 0 | 1 | 1 |
| flora radius | 84 m | 140 m | 180 m |
| grass radius | 22 m | 40 m | 56 m |
| crowd pose rate | 15 Hz | 60 Hz | 60 Hz |
| internal render scale | 1.0 | 1.0 | 1.0 |
| world / interactions | SAME world; anchors, characters, water, edits all visible | same | same |

## Benchmark walk

| tier | frames | p50 ms | p95 ms | p99 ms | worst | avg fps | meshed | GPU KB | flora inst | setl tris | crowd inst |
|---|---|---|---|---|---|---|---|---|---|---|---|
| low | 201 | 28.07 | 31.41 | 65.15 | 123.73 | 34.2 | 201 | 8177 | 463 | 1048 | 92 |
| mid | 201 | 28.64 | 33.57 | 60.74 | 103.86 | 33.8 | 402 | 18654 | 463 | 1048 | 92 |
| high | 201 | 29.28 | 35.28 | 55.31 | 109.74 | 32.8 | 603 | 27359 | 463 | 1048 | 92 |

## Bottleneck notes (measured, not guessed)

- Terrain patch meshing is the CPU spike source: the surface streamer caps it (mesh/frame rows above) and the caps HELD every frame in every tier run (counters in the table).
- The crowd's per-frame pose rebuild allocates small buffers; Low gates it to 15 Hz (the animation-rate lever) — the rig still moves at bench-stable steps.
- Far-detail levers (flora radius, shadow res, grass radius) are the GPU/memory rows; each is a documented contract number, not a hidden switch.
