# POORCRAFT 3D — THE REBUILD SLICE (play-test DMG, 2026-09-08)

## Just play
  Double-click POORCRAFT3D.app — it opens straight into the walkable
  world. (Or from Terminal: open POORCRAFT3D.app)
NOTE: saves (the B key) land in the build folder's saves3d/ on this
machine; quit with Esc.


The full natural-world rebuild on one walkable route: streamed surface
terrain, atmosphere, wilderness, the settlement kit, living people,
rivers, caves — and YOU on the ground.

## Play the rebuild slice (first person)
  ./poorcraft3d --play-rebuild live
WASD WALKS THE SURFACE, click to look, F builds on INSPECTED ground
(rejections name the reason), R removes, B saves, L reloads,
I shows Bed/Work/Idle boxes, Esc quits.

## The automated route proof
  ./poorcraft3d --play-rebuild
Five captures: the distant vantage, the town street, the river +
wheel, the cave mouth, and the gate at player height (+ the
construction save/reload proof).

## The quality contract + benchmark
  ./poorcraft3d --deck-bench 3 shots low|mid|high
  ./poorcraft3d --deck-bench 3 shots report

## Other automated windowed proofs
  ./poorcraft3d --play-people | --play-settlement | --play-wilderness
  ./poorcraft3d --play-materials | --play-caves | --play-surface-stream
  ./poorcraft3d --play-surface | --play-assets | --play-slice
  ./poorcraft3d --play-slice live | --play-city | --play-npcs
  ./poorcraft3d --play-water | --play-terrain | --play-stream
  ./poorcraft3d --play-quality | --play-shot | --play

## Windowless simulation
  --journey 4242 | --diagnose 2024 | --soak 365 80808
  --atlas 2024 24 | --flow-map 2024 | --validate-assets
