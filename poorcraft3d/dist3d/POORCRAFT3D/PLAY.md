# POORCRAFT 3D — the walkable build (NWR-005 caves/water/foundations, 2026-09-08)

Windowed 3D game over the deterministic simulation: natural streamed
surface terrain, original glTF assets, and now SPARSE CAVES, conforming
river water, and terrain-aware foundation checks.

## Play the world (first person)
  ./poorcraft3d --play-slice live
WASD walk, click to look, F places / R removes at your aim, B saves,
L reloads, I shows Bed/Work/Idle boxes, Esc quits.

## NEW this build: caves + water + foundations
  ./poorcraft3d --play-caves
One live window: stand INSIDE a carved cave (enclosing faceted stone,
no daylight leak), fly to a river overlook with water conforming to the
valley, then watch a 6 m dam appear across the strip — only the nearby
water sections remesh (the printout proves the local-work law).

## Other automated windowed proofs
  ./poorcraft3d --play-surface-stream  # streamed surface vista
  ./poorcraft3d --play-surface         # surface spike + edit
  ./poorcraft3d --play-assets          # original GLB tree/rock/house
  ./poorcraft3d --play-slice | --play-city | --play-npcs
  ./poorcraft3d --play-water | --play-terrain | --play-stream
  ./poorcraft3d --play-quality | --play-shot | --play
(PNGs land in a shots/ folder next to the binary.)

## Windowless simulation
  --journey 4242 | --diagnose 2024 | --soak 365 80808
  --atlas 2024 24 | --flow-map 2024 | --validate-assets
