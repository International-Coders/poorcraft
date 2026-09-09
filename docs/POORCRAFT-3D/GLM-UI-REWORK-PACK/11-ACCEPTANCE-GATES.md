# Acceptance Gates

The POORCRAFT 3D UI cannot be called owner-alpha usable until these gates pass.

## Launch Gate

- Double-click/no-args path opens a title screen.
- Title screen has visible logo and buttons.
- Cursor is visible.
- No clipped text.

## Gameplay Gate

- Play starts the rebuild slice.
- Pointer capture is clear.
- Crosshair, bars, hotbar, prompts, and toasts render.
- Debug text is hidden unless debug mode is on.

## Pause Gate

- Escape pauses and releases the pointer.
- Escape or Resume continues the game.
- Pause blocks movement/build/remove.
- Quit choices are explicit.

## Settings Gate

- Sensitivity, invert Y, FOV, quality, and UI scale are visible.
- Changes affect runtime behavior or are clearly disabled.

## Proof Gate

- Automated screenshot scenes cover all required screens.
- Pixel checks pass.
- Human inspection notes are written.
- Touched tests pass.
- Runtime package is rebuilt after owner-facing changes.
