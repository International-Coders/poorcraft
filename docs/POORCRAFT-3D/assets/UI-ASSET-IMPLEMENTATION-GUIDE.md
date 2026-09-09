# UI Asset Implementation Guide

Date: 2026-09-09

This guide turns the generated concept sheets into a practical POORCRAFT 3D UI
pipeline. The generated PNGs are direction and slice sources; the engine should
draw exact state, text, bindings, cooldowns, and fill amounts itself.

## Runtime Contract

- Store runtime-ready UI images under `poorcraft3d/assets/ui/` once the asset
  loader exists. Keep generated concept sheets in `docs/POORCRAFT-3D/assets/generated/`.
- Use `ui_asset_manifest.json` as the first naming map. It is allowed to point
  at concept sheets until real sliced runtime files exist.
- Render bars as engine primitives whenever possible: frame image + clipped
  fill region + numeric or icon overlay.
- Render key labels in-engine. Generated key text is a visual reference only.
- Use nine-slice scaling for panels, buttons, tooltips, hotbar slots, and modal
  frames so menus can resize without stretching corners.
- Every asset imported into runtime must preserve alpha and pass a screenshot
  proof on at least `1280x720`, `1920x1080`, and Steam Deck `1280x800`.

## HUD Bars

Use `generated/poorcraft3d-hud-bars-pack.png` for visual style:

- `health`: red heart bar, always visible in gameplay.
- `stamina`: green lightning bar, visible while moving, jumping, fighting, or
  building.
- `food`: amber meat bar, visible near health until survival tuning is final.
- `armor`: steel shield bar or pips, visible when armor exists.
- `mana`: blue spiral bar, hidden until the first magic system is unlocked.
- `air`: cyan wind/water bar, visible only underwater or in low-air volumes.
- `experience`: gold star progress strip near the hotbar.
- `temperature`: split heat/cold exposure bar, visible in hazardous biomes.
- `karma_corruption`: purple bar for faction/war/magic consequences, hidden
  until consequences exist.

Implementation note: bar fill should be a clipped rectangle or signed distance
shape controlled by game state, not a fixed full PNG. The art can provide the
frame, icon medallion, texture, and color ramp.

## Keys And Prompts

Use `generated/poorcraft3d-key-glyphs-pack.png` as a style guide for keycaps
and mouse buttons. Runtime labels must come from the active binding map:

- `WASD`: move.
- `Mouse`: look after capture.
- `Left Mouse`: interact, start, place, or confirm based on mode.
- `Right Mouse`: alternate interact or future block/tool action.
- `Wheel`: cycle hotbar or zoom/map scale.
- `Space`: jump.
- `Shift`: sprint/sneak depending on control mode.
- `Tab`: inventory or character sheet.
- `E`: interact/use.
- `F`: build/place in the current owner slice.
- `R`: remove in the current owner slice.
- `B`: save snapshot in the current owner slice.
- `L`: load snapshot in the current owner slice.
- `I`: inspect cell/object in the current owner slice.
- `M`: map/realm view once available.
- `C`: character/skills once available.
- `Esc`: pause/resume in owner mode.
- `Q`: explicit quit in owner mode until a full quit-to-title flow exists.

The current owner alpha menu should keep text prompts until a proper UI layer
lands. The first real implementation should draw an empty keycap frame and
place crisp bitmap/vector text on top.

## Action Icons

Use `generated/poorcraft3d-action-icons-pack.png` for action vocabulary:

- Core slice now: build/place, mine/remove, save, load, inspect, settings.
- Survival next: chop, pickaxe, torch, campfire, bed, backpack.
- RPG next: sword, bow, shield, stealth eye, magic spell, karma scales.
- Systems next: crafting hammer, furnace/smelter, research book, settlement
  flag, recruit/follow, trade, quest/journal, map.

Every action icon should have a disabled, normal, hover, pressed, and cooldown
state. For the first pass, shader tinting is acceptable; do not create five
separate generated variants per icon.

## Resource Icons

Use `generated/poorcraft3d-resource-icons-pack.png` for item presentation:

- Natural: wood, stone, clay, sand, herbs, berries, meat, leather, bone, water,
  resin.
- Ore: coal, copper, iron, gold, crystal.
- Industrial/magic: ember core, mana dust, gear, pipe, wire, valve, cell,
  steam pressure, circuit rune, ancient relic.
- Economy/settlement: coin, town supply crate.

Inventory icons should be normalized into square transparent cells with optical
padding. Avoid embedding item counts in the PNG; counts must be rendered by the
UI.

## Panels, Slots, And Menus

Use `generated/poorcraft3d-panel-frames-pack.png` for UI structure:

- Title: large frame with logo, Play, New World, Load, Settings, Quit.
- Pause: medium frame with Resume, Save, Settings, Quit to Title, Quit.
- Tooltip: small frame with 8-12 px safe padding after scaling.
- Hotbar: unselected slot, selected slot, and XP strip.
- Inventory: repeated square slot with item icon and count overlay.
- Minimap/compass: circular frame with markers from the faction/strategy pack.
- Toast: narrow strip for save/load/objective feedback.
- Dialogue: wide strip for NPC speech once NPCs have names and requests.

Nine-slice panel corners should never blur. If a frame is too detailed for
nine-slice, recreate it as renderer-native rectangles, bevels, and small corner
sprites.

## Faction And Strategy Markers

Use `generated/poorcraft3d-faction-strategy-pack.png` for the HOMM3 side of the
game:

- Factions: forest clan, mountain forge, river league, crypt, arcane tower,
  desert nomads, snowhold, swamp alchemists.
- Map locations: neutral village, player kingdom, raider camp, dungeon, mine,
  lumber camp, farm, watchtower, castle, shrine, portal, road sign.
- Realm states: town growth, diplomacy, war, peace.

These icons should first ship as minimap/world-map markers. A later realm view
can reuse them at larger size with tooltips and faction reputation colors.

## First Implementation Sequence

1. Add a small `UiAssetCatalog` in `pc3d_render` that loads PNG metadata and
   names from `ui_asset_manifest.json`.
2. Add a `UiDrawList` with quads for image, nine-slice panel, filled bar,
   bitmap text, and tinted icon.
3. Replace the temporary owner menu text panel with a title frame, real buttons,
   and exact key glyph prompts.
4. Add a live gameplay HUD: health/stamina/food bars, hotbar, crosshair, and
   save/load toast.
5. Add screenshot scenes for title, pause, and gameplay HUD. Pixel-check alpha
   overlays and text legibility before marking the UI pass done.
