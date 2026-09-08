# POORCRAFT 3D — the walkable build (NWR-006 materials & atmosphere, 2026-09-08)

Windowed 3D game over the deterministic simulation: natural streamed
surface terrain, caves, conforming rivers — now with SUN SHADOWS,
distance fog, material grain, water glint, and cutout foliage.

## Play the world (first person)
  ./poorcraft3d --play-slice live
WASD walk, click to look, F places / R removes at your aim, B saves,
L reloads, I shows Bed/Work/Idle boxes, Esc quits.

## NEW this build: the atmosphere proof
  ./poorcraft3d --play-materials
One live window through four looks at the same scene: the Mid tier
(shadows + fog + grain + glint), the legacy flat control, the High
tier, and a walk into the cutout foliage — cast shadows behind the
plants, haze swallowing the far hills, a sun glint on the water.

## Other automated windowed proofs
  ./poorcraft3d --play-caves          # cave interior + river + dam edit
  ./poorcraft3d --play-surface-stream # streamed surface vista
  ./poorcraft3d --play-surface | --play-assets
  ./poorcraft3d --play-slice | --play-city | --play-npcs
  ./poorcraft3d --play-water | --play-terrain | --play-stream
  ./poorcraft3d --play-quality | --play-shot | --play
(PNGs land in a shots/ folder next to the binary.)

## Windowless simulation
  --journey 4242 | --diagnose 2024 | --soak 365 80808
  --atlas 2024 24 | --flow-map 2024 | --validate-assets
