# POORCRAFT 3D — THE REBUILD SLICE (play-test DMG, 2026-09-10)

## WHICH BUILD AM I RUNNING? (read this first)
  The title screen subtitle shows BUILD <version> <git-hash> (e.g.
  BUILD 0.11.1 a1b2c3d), and the DMG volume itself is named
  POORCRAFT3D-<git-hash>. If the subtitle says an older hash than the
  volume you think you opened — or Finder shows two POORCRAFT volumes —
  you are playing a STALE mount: eject every POORCRAFT3D-* volume in
  Finder (the eject button next to each) and open the new DMG again.
  Local dev builds (cargo) say "dev".

## Just play
  Double-click POORCRAFT3D.app — it opens the TITLE SCREEN over the
  walkable rebuild slice. (Or from Terminal: open POORCRAFT3D.app)
NOTE: saves and settings land in the build folder's saves3d/ on this
machine.

## The real menus (GLM UI rework)
  Title screen: PLAY (or CONTINUE), NEW WORLD, LOAD WORLD, SETTINGS,
  QUIT. Click a button, or move with the arrow keys and press Enter.
  ESC never exits the game — it pauses. Quitting is always an explicit
  menu choice (or Q on the title/pause screen, which asks first).

- PLAY enters the world and captures the mouse; the first click in the
  world also captures it. ESC pauses (mouse released), ESC or RESUME
  continues.
- Pause menu: RESUME, SAVE, LOAD, SETTINGS, QUIT TO TITLE, QUIT TO
  DESKTOP.
- LOAD WORLD lists your saves3d slots with seed + date; loading and
  deleting both ask for confirmation first.
- SETTINGS: mouse sensitivity, invert Y, FOV, UI scale, quality preset
  (all live), and the full controls summary. Settings persist in
  saves3d/settings.json.
- NEW WORLD: type a seed (digits), reroll, pick a quality tier, CREATE.

## Gameplay HUD + keys
  Health/stamina/food bars (live values — stamina drains while you
  walk, food drifts down with travel), XP strip, 9-slot hotbar (slots
  1-5 are build materials; 1-9 or the wheel selects — F builds with the
  SELECTED material), crosshair, action prompt, fading toasts
  (SAVED/LOADED/PLACED/...). F3 toggles the debug strip (off by
  default).

  WASD walk the surface · mouse look · F build on INSPECTED ground
  (rejections name the reason) · R remove · B save · L reload · I
  inspect Bed/Work/Idle boxes · ESC pause.

## The automated proofs
  ./poorcraft3d --play-rebuild            # the 5-stop route proof
  ./poorcraft3d --play-rebuild live       # the interactive slice
  ./poorcraft3d --ui-shots                # 13 UI state screenshots
                                          #   + pixel checks + layout dumps
                                          #   + the row-shear (diagonal-cut) law
  ./poorcraft3d --ui-inspect '<json>'     # the local JSON inspector
  ./poorcraft3d --deck-bench 3 shots low|mid|high
  ./poorcraft3d --deck-bench 3 shots report

## Other automated windowed proofs
  ./poorcraft3d --play-people | --play-settlement | --play-wilderness
  ./poorcraft3d --play-materials | --play-caves | --play-surface-stream
  ./poorcraft3d --play-surface | --play-assets | --play-slice
  ./poorcraft3d --play-slice live | --play-city | --play-npcs
  ./poorcraft3d --play-water | --play-terrain | --play-stream
  ./poorcraft3d --play-quality | --play-shot | --play

## Windowless simulation
  --journey 4242 | --diagnose 2024 | --soak 365 80808
  --atlas 2024 24 | --flow-map 2024 | --validate-assets
