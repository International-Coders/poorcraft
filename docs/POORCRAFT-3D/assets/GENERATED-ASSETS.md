# Generated Assets

Date: 2026-09-09

These images were generated with the built-in Codex image generation tool for
POORCRAFT 3D concept direction. They are repository-local so future agents can
use them without relying on transient generation output.

## Files

- `generated/poorcraft3d-logo-concept.png`
  - Prompt family: `logo-brand`
  - Use: title/menu/logo direction
  - Alpha audit: `1774x887`, alpha `(0,255)`, transparent `755724/1573538`, semi-transparent `817680`
- `generated/poorcraft3d-hud-concept-sheet.png`
  - Prompt family: `ui-mockup`
  - Use: first HUD composition direction: hotbar, compass, status, objective, pause-panel mood
  - Alpha audit: `1536x1024`, alpha `(0,254)`, transparent `972039/1572864`, semi-transparent `600825`
- `generated/poorcraft3d-hud-bars-pack.png`
  - Prompt family: `hud-bars`
  - Use: individual survival/magic/progression bar styling
  - Alpha audit: `1254x1254`, alpha `(0,255)`, transparent `582864/1572516`, semi-transparent `987674`
- `generated/poorcraft3d-key-glyphs-pack.png`
  - Prompt family: `control-glyphs`
  - Use: keycap and mouse prompt styling
  - Alpha audit: `1254x1254`, alpha `(0,255)`, transparent `543948/1572516`, semi-transparent `1026848`
- `generated/poorcraft3d-action-icons-pack.png`
  - Prompt family: `action-icons`
  - Use: HUD, hotbar, inventory, and menu action icon styling
  - Alpha audit: `1254x1254`, alpha `(0,255)`, transparent `787368/1572516`, semi-transparent `784948`
- `generated/poorcraft3d-resource-icons-pack.png`
  - Prompt family: `resource-icons`
  - Use: inventory and crafting resource styling
  - Alpha audit: `1254x1254`, alpha `(0,255)`, transparent `706805/1572516`, semi-transparent `864469`
- `generated/poorcraft3d-panel-frames-pack.png`
  - Prompt family: `panel-frames`
  - Use: title screen, pause menu, tooltip, inventory slot, hotbar slot, minimap, toast, button states
  - Alpha audit: `1254x1254`, alpha `(0,255)`, transparent `402549/1572516`, semi-transparent `1168788`
- `generated/poorcraft3d-faction-strategy-pack.png`
  - Prompt family: `faction-strategy-icons`
  - Use: minimap, realm screen, settlement markers, faction banners, diplomacy/war/peace UI
  - Alpha audit: `1254x1254`, alpha `(0,255)`, transparent `747933/1572516`, semi-transparent `823174`

## Art Direction

- Rugged fantasy survival, not a cube-logo clone.
- Forged iron edges with restrained ember highlights.
- Forest green and river blue as secondary accents.
- Transparent smoked-glass panels for HUD surfaces.
- Controls must remain readable on Steam Deck scale.
- Key prompts must be exact. Use generated keycap art as a style reference or
  empty frame, then render labels in-engine with the real control binding.
- Strategy markers should read at minimap size before they are used in a realm
  view.

## Implementation Rule

Do not ship these concept PNGs as the only UI implementation. Convert them into
real runtime UI assets or renderer-native panel primitives, then prove the
result with screenshots and pixel checks.

Use `ui_asset_manifest.json` and `UI-ASSET-IMPLEMENTATION-GUIDE.md` as the
runtime handoff contract for slicing, naming, and drawing the assets.
