# POORCRAFT 3D — the walkable build (NWR-007 wilderness, 2026-09-08)

Windowed 3D game over the deterministic simulation: natural streamed
surface terrain, caves, rivers, atmosphere (shadows/fog/grain/glint) —
and now a LIVING WILDERNESS: instanced trees, rocks, shrubs, fallen
logs, wind-animated grass, and standing-stone landmarks, all placed
deterministically by the world's own biome rules.

## Play the world (first person)
  ./poorcraft3d --play-slice live
WASD walk, click to look, F places / R removes at your aim, B saves,
L reloads, I shows Bed/Work/Idle boxes, Esc quits.

## NEW this build: the wilderness proof
  ./poorcraft3d --play-wilderness
Four looks: the empty control, the same view with the wilderness
(instances + wind grass under the atmosphere), the standing-stone
landmark, and the Steam-Deck low tier.

## Other automated windowed proofs
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
