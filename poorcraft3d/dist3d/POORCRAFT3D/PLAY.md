# POORCRAFT 3D — the walkable build (NWR-009 the people, 2026-09-08)

Windowed 3D game over the deterministic simulation: streamed terrain,
caves, rivers, atmosphere, wilderness, settlements — and now PEOPLE:
limbed low-poly residents, workers, and guards with role gear,
animated by the world's own schedule (walk strides, work strokes,
sleep), drawn as instanced crowds.

## Play the world (first person)
  ./poorcraft3d --play-slice live
WASD walk, click to look, F places / R removes at your aim, B saves,
L reloads, I shows Bed/Work/Idle boxes, Esc quits.

## NEW this build: the people proof
  ./poorcraft3d --play-people
The plaza crowd animating (two stride frames), the guard close-up
(helm + spear), and the anchor-inspect view.

## Other automated windowed proofs
  ./poorcraft3d --play-settlement       # the modular kit town
  ./poorcraft3d --play-wilderness       # instanced trees/rocks/grass
  ./poorcraft3d --play-materials        # atmosphere tiers + cutout
  ./poorcraft3d --play-caves            # cave interior + river + dam
  ./poorcraft3d --play-surface-stream   # streamed surface vista
  ./poorcraft3d --play-surface | --play-assets
  ./poorcraft3d --play-slice | --play-city | --play-npcs
  ./poorcraft3d --play-water | --play-terrain | --play-stream
  ./poorcraft3d --play-quality | --play-shot | --play
(PNGs land in a shots/ folder next to the binary.)

## Windowless simulation
  --journey 4242 | --diagnose 2024 | --soak 365 80808
  --atlas 2024 24 | --flow-map 2024 | --validate-assets
