# Controls And Mouse Spec

The owner alpha must make mouse capture and keyboard behavior obvious.

## Required Behavior

- Title screen: cursor visible, movement disabled.
- Play click or Enter: start game and capture pointer.
- Gameplay click: capture pointer if released.
- Escape in gameplay: pause, release pointer, keep app open.
- Escape in pause: resume, capture pointer.
- Q: explicit quit only in owner mode until the real quit menu exists.
- Alt-F4 / Cmd-Q behavior may follow platform norms but must not be the primary
  exit path.

## Binding Source

All key prompts must come from the active binding map. Generated key art is only
frame/style reference.

## Required Bindings

- WASD: move.
- Mouse: look.
- Space: jump.
- Shift: sprint or sneak, chosen by control mode.
- Left mouse: confirm/interact/place.
- Right mouse: alternate interact/remove/tool alt.
- Wheel or 1-9: hotbar selection.
- E: interact.
- F: current owner-slice build/place.
- R: current owner-slice remove.
- B: save snapshot.
- L: load snapshot.
- I: inspect.
- M: map once available.
- Tab: inventory once available.
- Esc: pause/resume.
- Q: explicit quit.

## Proof

Automated input replay should prove Escape does not exit and that gameplay input
is blocked while menus are open.
