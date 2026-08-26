# Rendering & UX Stabilization (Phase P26 — do this before new content)

## Why this comes first

You said it plainly: there are real problems with how the game looks on
screen and how the menus work, and that has to get fixed before Stone Age
through Industrial+ content gets built on top of it. Shipping new eras on
top of a shaky renderer just means more content that looks broken. This
phase is a **hard gate**: P27 onward (power, magic, building) should not
start until the exit criteria below are met and proven with vistest PNGs,
per AGENTS.md's existing pixel-analysis discipline.

## Known starting points (from BACKLOG.md, carried over honestly)

- Smooth per-vertex lighting/AO is explicitly deferred ("flat per-face
  now") — this is very likely a chunk of the "looks wrong" feedback and
  should be the first rendering item picked up.
- Live RT view toggle doesn't exist yet (R captures a screenshot instead
  of toggling a live view) — if path tracing is part of what looks
  inconsistent, decide in this phase whether Live RT ships or path tracing
  stays a captured-showcase-only feature per Pillar 4 (runs everywhere).
- Minimap rotation/zoom and waypoint beams are unfinished — menu/HUD
  polish item.
- Key rebinding doesn't exist — settings menu gap.
- Save-slot thumbnails don't exist — menu polish gap.

## Rendering fix checklist

- [ ] **Per-vertex lighting / ambient occlusion** on voxel meshes — replace
      flat per-face light with vertex-AO (cheap corner-darkening based on
      neighbor solidity is enough; this is the single biggest "looks like
      a tech demo" fix for a voxel game).
- [ ] **Lighting consistency audit**: confirm torches/lanterns, sky light,
      and the day/night factor all compose correctly at chunk borders
      (a common voxel-engine bug class — seams at chunk edges).
- [ ] **Camera & FOV correctness pass**: re-verify the FOV math after the
      P25 audit already found and fixed a `to_radians()` double-conversion
      bug in the path tracer — check the rasterized camera path for the
      same class of bug.
- [ ] **Transparency/sorting audit**: water and glass already use a
      back-to-front sort; verify it holds up with the new content this
      roadmap adds (steam/smoke particles, magic effects) before those
      ship.
- [ ] **Frame-time budget on a "low" target device**: pick a real
      integer-GPU laptop spec, profile against it, and write the target
      fps in `DECISIONS.md`. Pillar 4 depends on this being a number, not
      a feeling.
- [ ] **Greedy meshing** (already noted as deferred, "fine at view 5") —
      revisit once new block variety from later phases increases mesh
      complexity per chunk.

## Menu & UX fix checklist

- [ ] **Settings menu completeness**: key rebinding, graphics quality tiers
      (a literal "Low / Medium / High / Path-Traced" dropdown mapping to
      Pillar 4's guardrail), audio volume sliders (sound doesn't exist yet
      per BACKLOG — stub the UI now so P-numbered sound work later doesn't
      need a new settings screen).
- [ ] **Save-slot picker polish**: thumbnails per save, world name/seed/
      last-played visible before loading.
- [ ] **Minimap/waypoint completion**: rotation, zoom, and world-space
      waypoint beacons (map + minimap pips already exist).
- [ ] **First-launch flow**: a brand-new player should hit a title screen
      that clearly offers "New World," "Load," "Multiplayer," and
      "Settings" with no dead ends — audit this against a literal
      first-time user, not a dev who knows the shortcuts.
- [ ] **In-game menu consistency**: inventory, crafting, chest, quest log,
      tech-tree, and (new, from later phases) spellbook and blueprint
      screens should share one `ui_kit.rs` visual language — same corner
      radius, same reveal animation, same color roles — so the RPG/tech
      mashup doesn't look like two different games glued together.
- [ ] **Readability at a glance**: HUD elements (hearts, hunger, air,
      hotbar, mana once added in P28) need to be legible against every
      biome's sky/fog color, not just the biome they were designed against.

## Exit criteria for P26

- A fresh vistest run shows no flat-shaded/AO-less terrain in any scene.
- Settings menu can rebind at least movement + interact keys and persists
  them in `ClientSave`, matching existing `Settings` persistence pattern.
- A person who has never seen the game can get from title screen into a
  world and back to the settings menu with no explanation.
- Frame-time target for the chosen "low" device is written down and met
  at "Medium" quality in a vistest-adjacent perf check.
- All of the above pass `cargo test --workspace` + a fresh
  `vistest shots` pixel-analysis run, committed with real evidence in
  DEVLOG.md, per AGENTS.md.
