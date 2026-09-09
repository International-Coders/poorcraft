# Baseline Screenshot Notes

Inspected image:

- `baseline/current-windowed-slice-showcase.png`

Attempted fresh capture:

- Command: `make p3d-rebuild OUTDIR=poorcraft3d/apps/poorcraft3d/shots/glm_ui_baseline`
- Result: build completed and the route seed was found, but the process stalled
  before writing PNGs and was interrupted. Future work should harden the
  owner-facing screenshot path so GLM can always capture title/pause/gameplay
  UI states without manual window interaction.

Visual notes from the existing baseline:

- The visible text starts at the top-left edge and is clipped.
- There is no real HUD composition.
- No bars, hotbar, crosshair, button frame, title menu, prompt glyph, toast, or
  settings surface is visible.
- The world render exists, but it is not presented inside a usable game shell.
- This screenshot is a failure baseline for UI work, even if it was previously
  acceptable as a renderer/world proof.
