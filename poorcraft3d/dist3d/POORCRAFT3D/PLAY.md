# POORCRAFT 3D — the walkable build (NWR-008 settlement kit, 2026-09-08)

Windowed 3D game over the deterministic simulation: streamed surface
terrain, caves, rivers, atmosphere, wilderness — and now SETTLEMENTS
as an original modular kit: socket-chained walls, a walkable gate
arch, watchtowers, the banner-topped keep, thatched crofts, market
stalls, docks and water wheels, all placed by the world's own plans.

## Play the world (first person)
  ./poorcraft3d --play-slice live
WASD walk, click to look, F places / R removes at your aim, B saves,
L reloads, I shows Bed/Work/Idle boxes, Esc quits.

## NEW this build: the settlement proof
  ./poorcraft3d --play-settlement
The capital and its town from the authoritative plans as kit modules —
overview, the street among the crofts, and the river works.

## Other automated windowed proofs
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
