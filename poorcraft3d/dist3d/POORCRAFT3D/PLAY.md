# POORCRAFT 3D — the walkable build (NWR-010 Deck contract, 2026-09-08)

Windowed 3D game over the deterministic simulation: streamed terrain,
caves, rivers, atmosphere, wilderness, settlements, people — now with
a TRANSPARENT Low/Medium/High quality contract and a measured
benchmark (167/149/146 fps on the evidence machine).

## Play the world (first person)
  ./poorcraft3d --play-slice live
WASD walk, click to look, F places / R removes at your aim, B saves,
L reloads, I shows Bed/Work/Idle boxes, Esc quits.

## NEW this build: the Deck benchmark
  ./poorcraft3d --deck-bench 3 shots low
  ./poorcraft3d --deck-bench 3 shots mid
  ./poorcraft3d --deck-bench 3 shots high
  ./poorcraft3d --deck-bench 3 shots report
The full-stack walk per tier + the documented report (contract table,
measured percentiles, work counters, bottleneck notes).

## Other automated windowed proofs
  ./poorcraft3d --play-people          # the NPC rig crowd
  ./poorcraft3d --play-settlement      # the modular kit town
  ./poorcraft3d --play-wilderness      # instanced trees/rocks/grass
  ./poorcraft3d --play-materials       # atmosphere tiers + cutout
  ./poorcraft3d --play-caves           # cave interior + river + dam
  ./poorcraft3d --play-surface-stream  # streamed surface vista
  ./poorcraft3d --play-surface | --play-assets
  ./poorcraft3d --play-slice | --play-city | --play-npcs
  ./poorcraft3d --play-water | --play-terrain | --play-stream
  ./poorcraft3d --play-quality | --play-shot | --play
(PNGs land in a shots/ folder next to the binary.)

## Windowless simulation
  --journey 4242 | --diagnose 2024 | --soak 365 80808
  --atlas 2024 24 | --flow-map 2024 | --validate-assets
