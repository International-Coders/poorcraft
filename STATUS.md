# LOREFORGE — Status (Step 40 honesty pass, loop 326)

## Shipped and proven
Every V1REBRAND phase (P28-P39) and every build-pack step (1-40) has
code, tests, and rendered proof:

- Rendering/UX: cross-column lighting (no chunk seams), sway, per-face
  materials, AO, transparency rules, connected stone/planks surfaces,
  HUD text shadows, radial reticle, biome grading, perf target (p50
  ~48ms at Medium on this iGPU).
- Gameplay ages: Water -> Steam -> Oil -> Nuclear (the tested ceiling:
  meltdowns leave glowing residue that hurts until scrubbed).
- Magic: mana, four bounded spells, spellbook, wizard towers, enchanting
  imbue with real rune effects, lumen/warding crossovers.
- Construction: shape system (slabs/stairs + fractional collision),
  symmetry, blueprints with material bills, scaffolding, chisel statues.
- Smart building: conduit-relayed power, elevators, climate, live
  computer screens (the engine's dynamic-texture path).
- Dragons: flight AI, multi-part rendering, breath, roosts, the
  user-approved mount (spike audit in DECISIONS).
- Paths: four standings with milestones, generalized gates enforced at
  craft AND placement, respec, protocol-v4 escrowed trading (real-UDP
  tested).
- Platform: lobbies/invites model (tested), Workshop UGC scanning,
  mods/README authoring guide, `xtask new-mod` scaffolding.
- Evidence: 256 tests, 47 vistest scenes (pixel/AI verified), smoke
  every loop, runtimes in dist/.

## Honest limits (not hidden, not shipped)
- Steam: the `steam` feature arms are unverified without the Steam SDK;
  UDP is the default, tested transport; the lobby model is transport-
  neutral and tested.
- Trade: receiving/applying offers is wired client-side; the SENDING UI
  is not (protocol + server escrow + real-UDP test fully cover the
  deliverable).
- Deferred niceties: roost loot chests, breath igniting blocks,
  blueprint rotation, slopes beyond stairs, more runes/spells (bounded
  by design), belt extraction direction UI.
- Perf: measured on this Intel iGPU host only.
