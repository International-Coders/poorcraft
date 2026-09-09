# Generated Assets

Date: 2026-09-08

These images were generated with the built-in Codex image generation tool for
POORCRAFT 3D concept direction. They are repository-local so future agents can
use them without relying on transient generation output.

## Files

- `generated/poorcraft3d-logo-concept.png`
  - Prompt family: `logo-brand`
  - Use: title/menu/logo direction
  - Alpha: RGBA PNG, transparent background present
- `generated/poorcraft3d-hud-concept-sheet.png`
  - Prompt family: `ui-mockup`
  - Use: HUD, hotbar, status bars, compass, objective tag, pause-panel direction
  - Alpha: RGBA PNG, transparent pixels present across the sheet

## Art Direction

- Rugged fantasy survival, not a cube-logo clone.
- Forged iron edges with restrained ember highlights.
- Forest green and river blue as secondary accents.
- Transparent smoked-glass panels for HUD surfaces.
- Controls must remain readable on Steam Deck scale.

## Implementation Rule

Do not ship these concept PNGs as the only UI implementation. Convert them into
real runtime UI assets or renderer-native panel primitives, then prove the
result with screenshots and pixel checks.
