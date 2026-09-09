# Owner Reality Check

The owner is not asking for another invisible systems milestone. The owner is
saying the game still feels like a broken pre-alpha when launched and played.

Observed baseline from `baseline/current-windowed-slice-showcase.png`:

- Top-left text is clipped by the window edge.
- The HUD is debug text, not a game HUD.
- No health, stamina, food, mana, armor, XP, air, hotbar, item count, or prompt
  layer is visible.
- No title menu is visible in the baseline proof screenshot.
- No panel, button, or save/load screen proves owner-facing navigation.
- Screenshot automation does not yet prove title, pause, gameplay HUD, settings,
  and save-slot screens as separate UI states.

Owner expectation:

- Double-click should feel like a game opening.
- The first screen should be a title/welcome screen with clear choices.
- Playing should show a legible HUD that uses real state.
- Escape should pause. It must not feel like a crash.
- Settings, save, load, quit-to-title, and quit-to-desktop must be discoverable.
- Every UI claim must be backed by screenshots, tests, and data export.

This is the priority until the owner-facing loop feels playable.
