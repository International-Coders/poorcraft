# Owner Alpha Rescue

Date: 2026-09-08

This pass records the owner's current reality check: the game may have many
proven systems, but the owner-facing alpha still felt broken because the
double-click build opened straight into a live slice with no welcome screen,
the mouse affordance was unclear, and Escape immediately closed the app. To a
player, that looks like a crash.

## Shipped UX Contract

- Double-clicking the POORCRAFT 3D app opens the rebuild slice behind a
  centered welcome overlay.
- Enter or left-click starts the owner alpha slice.
- The first click captures the mouse for look control.
- Escape pauses/resumes in owner mode; it must not exit the application.
- Q is the explicit quit command in owner mode.
- While title or pause is open, movement, build/remove commands, and crowd
  animation are blocked.
- Automated proof windows keep the old behavior unless they opt into
  `owner_menu`, so visual-gate frame timing stays stable.

## Immediate Honesty

This is not the final UI stack. It is a rescue shell using the renderer's tiny
bitmap-font HUD path so the owner can play-test without the app feeling hostile.
The real menu, settings, save-slot picker, world creation, and alpha HUD should
land as follow-up work with a proper UI layer.

## Generated Visual Assets

Two transparent PNG concept assets were generated for direction and stored in
the repository:

- `docs/POORCRAFT-3D/assets/generated/poorcraft3d-logo-concept.png`
- `docs/POORCRAFT-3D/assets/generated/poorcraft3d-hud-concept-sheet.png`

They are concept references, not final shipping art. Their role is to give the
next UI/brand pass a concrete target: rugged fantasy survival, forged-metal
panels, transparent alpha overlays, ember highlights, forest/river accents, and
Steam Deck-readable controls.

## Next Required Job

Build the proper alpha front-end:

- title screen with Play, New World, Load, Settings, Quit;
- pause menu with Resume, Save, Settings, Quit to Title, Quit to Desktop;
- save-slot browser for the current `saves3d` path;
- mouse sensitivity and invert-Y settings;
- HUD bar/hotbar implementation based on the generated HUD concept sheet;
- screenshot proof that title, pause, and gameplay HUD render in the live
  window.

This next job should replace the temporary bitmap-font menu, not pile another
temporary layer beside it.
