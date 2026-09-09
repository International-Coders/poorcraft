# HUD And Hotbar Spec

The HUD must be a live game surface, not a debug text line.

## Always Visible In Gameplay

- Crosshair centered.
- Hotbar at bottom center with 9 slots.
- Selected hotbar slot highlighted.
- Current action prompt near the crosshair or hotbar.
- Save/load/toast strip that fades and never covers the hotbar.

## Status Bars

- Health: left-bottom or left-top cluster.
- Stamina: near health, drains while sprinting/actions once state exists.
- Food: visible near health, allowed to be placeholder-backed until survival
  state exists in POORCRAFT 3D.
- XP/progress: thin strip near hotbar.
- Mana, air, armor, temperature, karma/corruption: hidden until state exists,
  but layout reserves a place and screenshot scenes may use debug state.

## Debug Text

Debug text must move behind a debug toggle or inspector overlay. It must not be
the owner-facing HUD.

## Proof

Required screenshots:

- gameplay HUD at default state;
- gameplay HUD with non-full health/stamina/food debug values;
- selected hotbar slot changed;
- toast visible;
- debug overlay on/off comparison.
