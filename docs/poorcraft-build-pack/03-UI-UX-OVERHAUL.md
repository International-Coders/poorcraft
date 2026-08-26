# UI/UX Overhaul — Detail for Steps 12–15

## Why this matters as much as the engine

You said the UI is "not at all fascinating." A voxel-RPG-automation
mashup lives or dies on its screens — inventory, crafting, the tech tree,
and (soon) a spellbook and blueprint screen are where a player spends a
huge fraction of their time. This needs to look designed, not default.

## Step 12 — Design-system pass on ui_kit.rs

- Audit every existing screen (inventory, crafting table, chest, quest
  log, tech tree, pause menu, title screen) against `ui_kit.rs`'s theme,
  easing, and `Reveal` animation primitives. Any screen that doesn't use
  the shared theme/animation system gets brought in line.
- Define (if not already defined) a small, explicit set of color roles —
  e.g. background, panel, accent-primary (used for interactive
  highlights), accent-danger (used for durability/health-critical
  states), text-primary, text-muted — and apply them consistently. Every
  future screen (spellbook, blueprint tool, computer-block display) must
  use these same roles, not invent new ones.
- Consistent corner radius, padding, and reveal-animation timing across
  all panels — a player moving from the inventory to the tech tree
  shouldn't feel like they opened a different application.
- Typography: confirm one font (or a deliberate, small font pairing — one
  display face for headers, one body face) is used everywhere; audit for
  any screen still using an egui default that doesn't match.

## Step 13 — Settings menu completeness

- **Key rebinding**: at minimum, movement (WASD or equivalent), jump,
  sneak, sprint, interact/place, break, inventory, and any newly-added
  binds (spell cast, blueprint tool) from later steps. Store in the
  existing `Settings` struct pattern, persisted in `ClientSave`.
- **Graphics quality tiers**: a Low/Medium/High/Path-Traced dropdown that
  actually maps to real settings — view distance, AO on/off, particle
  density, and whether the path tracer is available at all. This is what
  makes Pillar 4 (runs on low-end hardware) a real, selectable promise
  instead of a single fixed experience.
- **Audio sliders**: master/music/sfx volume, wired to whatever audio
  system Step 4 introduces. Build the UI now even if it only controls one
  category at first — better than adding a new settings screen later.

## Step 14 — Save-slot picker and first-launch flow

- **Thumbnails**: render a small preview image per save slot — a stored
  screenshot taken at last save, or a generated top-down map preview if a
  live screenshot isn't feasible yet.
- **Metadata visible before loading**: world name, seed, last-played
  timestamp, and world type (Normal/Superflat/Amplified) shown on each
  slot.
- **First-launch audit**: literally trace, as if you'd never seen the
  game, every path from the title screen: New World → world-type/seed
  selection → in-world; Load → slot picker → in-world; Multiplayer →
  (after Step 34) lobby list or direct-connect → in-world; Settings → back
  to title with no lost progress. Any screen with no visible way back or
  forward is a bug.

## Step 15 — Minimap and waypoints

- **Rotation**: minimap rotates with player facing (or add a toggle for
  north-locked vs. player-locked, a common QoL option).
- **Zoom**: at least two zoom levels.
- **World-space waypoint beacons**: a vertical beam or marker rendered in
  the 3D world at a waypoint's location, visible from a distance, not just
  a pip on the 2D map/minimap.

## Cross-cutting note for later steps

Every UI added in later stages — the spellbook (Step 28), the blueprint
tool (Step 31), the computer/screen block's in-world display (Step 32),
Steam lobby browser (Step 34) — must be built using the design system
locked in during Step 12, not as one-off screens. If Step 12 is skipped or
rushed, every later UI step inherits the same "doesn't look designed"
problem this stage exists to fix.
