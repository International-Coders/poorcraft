# Failure Modes

These are known ways an AI session can produce fake progress.

- Shipping docs but no code.
- Adding assets but never drawing them in runtime.
- Claiming the game was played without screenshot artifacts.
- Claiming screenshots are good without opening them.
- Treating debug text as a HUD.
- Letting text clip at window edges.
- Letting Escape exit the app in owner mode.
- Hiding mouse capture behavior.
- Building a menu that does not block gameplay input.
- Baking key labels into generated PNGs.
- Making bars that do not read game state.
- Testing only one resolution.
- Letting GPU proof scenes skip the owner-facing UI path.
- Weakening tests after they expose a real UI problem.
- Forgetting to update the bookkeeping docs.

When one of these happens, stop feature work and fix the proof loop.
