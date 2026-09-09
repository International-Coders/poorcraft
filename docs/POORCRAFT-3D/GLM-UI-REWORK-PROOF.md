# GLM UI Rework — Proof Record (UI-001..UI-008)

Date: 2026-09-09
Session: GLM 5.3 / Z-code, executing
`docs/POORCRAFT-3D/GLM-UI-REWORK-PACK/09-IMPLEMENTATION-QUEUE.md`.

## What Shipped

- **UI-001 (screenshotable UI state harness)**: `--ui-shots [outdir]` /
  `make p3d-ui-shots` — one deterministic windowed run that drives every
  owner-facing UI state on the showcase world (seed 22) and captures 11
  scenes with per-scene layout dumps (`*.layout.json`) and in-process pixel
  checks (nonblank, safe margins, no interactive overlap, alpha-blended UI
  presence, required elements with ink, forbidden elements absent,
  focused-state distinctness, live bar-fill fraction).
- **UI-002 (runtime UI draw list)**: `pc3d_render::ui` — `UiState` (explicit
  screen/focus/hover/modal/settings/HUD state, JSON-serializable), a pure
  layout builder (`build(state, w, h) -> DrawList`), a pure CPU painter
  (`paint(list) -> RGBA canvas`), and a pure input reducer
  (`on_key`/`on_click`/`on_mouse_move -> Vec<UiAction>`). The renderer draws
  the canvas as one fullscreen alpha-blended quad after the world pass
  (`Renderer::set_ui_layer`); uploads happen only when the UI changed.
- **UI-003 (real title + pause)**: the bitmap-text owner overlay is gone.
  Title (PLAY/CONTINUE, NEW WORLD, LOAD WORLD, SETTINGS, QUIT) and Pause
  (RESUME, SAVE, LOAD, SETTINGS, QUIT TO TITLE, QUIT TO DESKTOP) are real
  panels with mouse hover/click + arrow-key/Enter navigation and visible
  focus. Escape pauses/resumes and NEVER exits (unit-tested + inspector
  replay proof); quitting is an explicit choice (or Q on a menu, which asks
  first). Game input is blocked under every menu and modal.
- **UI-004 (HUD bars + hotbar)**: health/stamina/food bars (live values —
  stamina drains while walking and recovers at rest, food drifts with
  travel; health stays full until damage systems exist), XP strip, 9-slot
  hotbar where slots 1–5 are the real build palette (selection via 1–9/wheel
  changes what F places), selected-slot ember frame, crosshair, action
  prompt, fading toasts above the hotbar band. The old top-left debug line
  is hidden in owner mode (blank line = no draw) and replaced by the F3
  debug strip (off by default).
- **UI-005 (settings + controls)**: mouse sensitivity, invert Y, FOV,
  UI scale, quality preset — each with [−]/[+] controls and arrow-key
  adjustment, all driving runtime behavior live (sensitivity/invert in the
  look handler, FOV → camera, UI scale → layout, quality → `deck::apply`),
  plus the full keymap summary rendered from the binding table. Settings
  persist in `saves3d/settings.json` (clamped on load).
- **UI-006 (save slot UI)**: LOAD WORLD lists saves3d slots (name, seed
  parsed from `world.p3d`, civil-date mtime); LOAD/DELETE/quit-to-desktop
  ask through a confirmation modal that owns the frame (dim layer; the
  underlying screen is not interactive). Test-only save directories: the
  harness and inspector run against a temp `saves3d` root
  (`WindowConfig::save_root_override`), never owner saves.
- **UI-007 (inspector)**: `--ui-inspect '<json>'` — a local JSON command
  tool driving the live windowed game: `ui_state`, `input_state`,
  `set_screen`, `set_debug_hud_values`, `capture_screenshot`,
  `dump_ui_layout`, `dump_frame_stats`, `dump_visible_world`,
  `dump_mesh_wireframe` (summary-level v1, documented), `replay_input`,
  `wait`. Commands execute through the same action path as real input.
- **UI-008 (polish)**: renderer-native frames everywhere — forged-metal
  panels with bevels + corner studs, ember focus/hover rings, chunky logo
  with gradient letters + backing band, bar sheen, material swatch icons,
  shadow-backed text (readable over bright sky and dark terrain — both
  backgrounds captured). Generated PNG sheets remain declared concept
  references; nothing is baked into art (key labels come from `KEYMAP`).

## How It Was Proven

- Build: `cargo build --release --manifest-path poorcraft3d/Cargo.toml` —
  clean.
- Tests: `cargo test --release --manifest-path poorcraft3d/Cargo.toml` —
  pc3d_render 153 passed (26 new UI layout/paint/input tests + app tests:
  escape laws, modal routing, civil-date formatting, framed-meta seed
  parse, palette law); full p3d workspace suite green (counts in
  DEVLOG/STATE).
- Screenshots: `make p3d-ui-shots` — 11 captures + 11 layout dumps in
  `poorcraft3d/apps/poorcraft3d/shots/` (`ui_*.png`, `ui_*.layout.json`);
  the run's own pixel checks PASS (printed report; also a gate).
- Inspector: the 12-command proof script (including two `replay_input`
  Escape presses) returned `ok: true`, `screen_after=pause` with
  `gameplay_blocked=true`, then `screen_after=gameplay` with the pointer
  re-grabbed — the app ran to its frame budget both times. Capture:
  `shots/inspect_pause.png`.
- Gates: `make p3d-visual-gates` — the battery is now 10 gates
  (`ui-states` added; capability inventory + guardrail updated together).
- Runtime: `make p3d-dmg` rebuilt; no-args launch verified ALIVE on the
  title screen.

## Human Inspection Notes (every screenshot opened and read)

- `ui_title_1280x720` — PASS after fixes. First capture showed the legacy
  top-left debug line ("P3D POS …") behind the menus and low-contrast
  button text; both fixed (owner runs blank the debug line; button text
  scale 3 + darker fill; logo got a dark backing band). Final read: ember
  POORCRAFT letters with the 3D chip over the backing band, five labeled
  buttons (CONTINUE/NEW WORLD/LOAD WORLD/SETTINGS/QUIT), footers inside
  margins, no stray text. "Reads like a real game title screen."
- `ui_title_focus` — same composition with focus on NEW WORLD; pixel diff
  localized to exactly the two button rects (2092 px each).
- `ui_new_world` — panel reads WORLD NAME/WORLD-22, SEED/22, MODE/SURVIVAL
  (CREATIVE SOON — honest placeholder), QUALITY/MID, then REROLL SEED,
  QUALITY −/+, CREATE WORLD, BACK. No clipping.
- `ui_load_world` — two slot rows (ALPHA-SHOT · SEED 22 · date,
  BETA-SHOT · SEED 7 · date) each with LOAD/DELETE buttons inside the
  panel, BACK below. First run showed wrong seeds (18/17): the meta parser
  read the payload-length field — fixed to offset 24 (frame = header 16 +
  len 8) with a regression test.
- `ui_settings` — five setting rows with values and [−]/[+] chips, the
  two-column CONTROLS summary, BACK. Readable over the world.
- `ui_gameplay_hud_default` — bars full, prompt under the crosshair,
  9-slot hotbar with SOIL/GRASS/SAND/ROCK/SNOW labels, slot 1 selected,
  XP strip, no debug text.
- `ui_gameplay_hud_debug_values` — bars read ~45%/30%/70% (fill-fraction
  check 0.38 incl. sheen tolerance), slot 4 carries the ember frame, toast
  strip above the hotbar.
- `ui_pause` — six buttons read cleanly (RESUME … QUIT TO DESKTOP) over
  the bright-sky pose.
- `ui_modal_confirm` — dim layer over the dark-terrain pose, DELETE
  message, DELETE/CANCEL buttons; nothing overlaps (the modal owns the
  frame).
- `ui_debug_inspector` — blue-tinted strip top-right inside margins:
  "P3D 0.1 SEED 22 WORLD REBUILD POS … FPS … BUILT 0".
- `ui_title_deck` (1280×800) — same title composition, safe margins hold
  (the fit-scale law shrinks rather than clips).

## What Failed First (and what it fixed)

1. Screens never switched: script-driven `OpenScreen`/`StartPlaying`
   actions relied on the reducer having set the screen — the action
   executor now sets it (idempotent).
2. Overlap check caught the modal sharing the frame with pause buttons —
   modals now own the frame (dim + modal only).
3. Settings [−]/[+] mini-buttons overlapped rows at UI scale 1.5 — fixed
   heights; New World QUALITY −/+ overlapped each other — halved widths.
4. The vision pass caught the legacy debug line behind the title screen
   (owner runs now blank it; empty line = no draw at all).
5. Load-world seeds read wrong (18/17 instead of 22/7) — framed-meta
   offset fix + test.
6. A stale half-resolution contact sheet read as "NO SAVED WORLDS YET";
   full-resolution crops + the layout dump confirmed the slots were always
   there — no code change, but it showed why inspection must be at native
   resolution.

## Honest Deferrals

- The hotbar's slots 6–9 are reserved-empty (no tools/items in the live
  slice yet); health has no damage source; XP = onboarding fraction
  (first-build marker) — all marked in the UI ("CREATIVE SOON" likewise).
- `dump_mesh_wireframe` is summary-level v1 (counters, not raw triangles).
- Audio placeholders and gamepad/rebinding UIs are not built (no audio or
  binding system in the slice yet); the keymap is a table, not a rebind
  flow.
- Menu world backdrop is a fixed showcase vantage; no per-world thumbnail
  in save slots.
- Steam Deck layout is proven at 1280×800 logical; a real Deck hardware
  pass remains with the deck bench.

## Task Status

All eight queue tasks (UI-001..UI-008) are done per their done_when
criteria; the acceptance gates in `ui_acceptance_gates.json` are covered by
the gate battery + this proof record. Next: the owner's manual play pass
remains the gate before calling the slice playable.
