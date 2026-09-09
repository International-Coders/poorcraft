# UI Rebuild Spec

The temporary bitmap-font owner overlay must be replaced by a real UI layer.

## Required Screens

- Title screen: logo, Play, New World, Load World, Settings, Quit.
- New World: seed, world name, mode, quality preset, create/back.
- Load World: save slots, timestamp, seed, delete confirmation, load/back.
- Gameplay HUD: bars, hotbar, crosshair, prompts, toast messages.
- Pause menu: Resume, Save, Load, Settings, Quit to Title, Quit to Desktop.
- Settings: mouse sensitivity, invert Y, FOV, quality preset, UI scale, audio
  placeholders, controls summary.
- Error modal: clear message, recover/back/quit choice.

## Architectural Requirements

- UI state must be explicit and serializable enough for screenshot scenes.
- UI rendering must happen after the world pass and before readback.
- Layout must have safe margins at 1280x720, 1280x800, 1920x1080, and 2560x1440.
- Text must never start at pixel 0 or clip into the window edge.
- Buttons need normal, hover, focused, pressed, disabled states.
- Keyboard and mouse navigation must both work.
- Game input must be blocked while title, pause, settings, modal, or save slot
  screens are active.

## First Implementation Preference

Use a simple immediate-mode UI inside `pc3d_render` if that is fastest and
testable. Do not wait for a perfect external UI stack before fixing the owner
experience.
