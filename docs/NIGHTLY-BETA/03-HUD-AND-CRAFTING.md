# HUD, Inventory, and Crafting Specification

The UI should feel like a finished voxel RPG: quiet while exploring, decisive
when danger or interaction matters, and fully readable without F3. Preserve
the existing `ui_kit` visual language and improve its hierarchy instead of
introducing a second theme.

## HUD information architecture

### Always visible during survival play

- centered crosshair with target affordance and mining/attack state;
- health, hunger, air only when relevant, experience progress, and hotbar;
- selected item name briefly after selection;
- equipped-stack count and durability with color plus shape, never color only;
- one compact objective line until the player dismisses or completes it.

### Contextual layers

- Interaction prompt beside the crosshair: action key, verb, target name, and
  blocked reason. Examples: `E Trade — Mara`, `Hold LMB Mine — Iron Ore`,
  `RMB Place — blocked by player`, `E Enter — gate barred (Hostile)`.
- Combat: hit direction, cooldown/readiness, damage/heal feedback, threatened
  state, and boss/elite identity. It fades completely after the event.
- Reputation: a small faction crest, signed standing delta, reason, and current
  threshold when a witnessed action changes standing. Never expose private
  debug arithmetic by default.
- Settlement: realm and place name on entry, safety/hostility state, optional
  garrison alert, and kingdom-compass direction when held.
- Survival warnings: low health, starvation, drowning, temperature/status
  effects when implemented. Priority prevents warnings stacking illegibly.
- Build mode: shape/mirror controls only while holding a placeable block.

### Debug only

Coordinates, biome ID, FPS/frame time, network state, pathfinding counters,
seed hashes, and render mode remain under F3. Debug telemetry must never be
required to play.

## First-minute onboarding

Implement a persisted tutorial state machine, not timed text spam:

1. Move: completes after real displacement from WASD/controller input.
2. Look: completes after yaw/pitch delta.
3. Gather: points at a valid reachable natural block and completes after its
   drop reaches inventory.
4. Craft: opens hand crafting, highlights planks, and completes after output.
5. Build shelter: place a qualifying solid block and pin the starter quest.

Prompts pause while a modal screen is open, do not repeat after completion,
can be dismissed or reset in settings, and adapt to the active keymap. Tests
drive the pure state transitions; small-window proofs show no overlap.

## Workbench redesign

The current workbench exposes world, HUD, armor strip, inventory, recipes, and
actions at once. Beta behavior uses a deliberate modal composition:

- opaque or strongly dimmed panel surface with a world scrim;
- no duplicate survival HUD beneath the modal except an optional faint hotbar;
- left: searchable categories and known/locked counts;
- center: recipe list with icon, name, craftable count, era/knowledge lock;
- right: selected recipe, output preview, short purpose text, exact ingredient
  rows, owned/needed amounts, substitutions, station, time/power requirements;
- bottom/right primary action: `Craft 1`, `Craft All`, or `Queue`; one action
  owns Enter/Space and has a clear disabled reason;
- compact inventory access for ingredient movement without drawing unrelated
  armor UI over the recipe browser;
- queue strip with output, remaining count/time, pause/cancel, blocked reason,
  and where output will go;
- recipe discovery filters: craftable, new, favorites, station, era, and text;
- consistent close behavior: E or Escape closes to play and restores cursor
  lock; no invisible focus owner survives.

At 640×420, switch to a two-pane drill-down layout instead of shrinking text.
At wide sizes, cap line length and panel width. Long translated names truncate
with tooltip or wrap without shifting the action off-screen.

## Crafting correctness contract

- Crafting is transactional: validate, reserve/consume exact ingredients,
  produce exact output, then emit quest/audio/toast events once.
- Failed output insertion does not consume ingredients; queued jobs persist or
  return ingredients according to a documented rule.
- Batch craft uses integer-safe maximum counts and cannot duplicate through
  rapid clicks, screen close, save/reload, or queue cancellation.
- Recipe book visibility remains earned; locked rows say how to discover them
  without leaking hidden lore rewards.
- Hand crafting, workbench, furnace, machines, smithing, and mod recipes share
  reusable ingredient/status widgets but keep distinct mechanics.
- Mouse, keyboard, and future controller navigation use stable focus order.

## Required proofs

- `hud_onboarding`: first prompt and pinned objective at 1280×800.
- `hud_small_onboarding`: same state at 640×420 with zero rectangle overlap.
- `hud_combat_reputation`: combat warning and one faction delta, priority-safe.
- `crafting_workbench`: revised normal layout with world scrim and no duplicate
  HUD collision.
- `crafting_workbench_small`: two-pane compact layout.
- `crafting_missing_ingredients`: readable disabled reason and owned/needed
  values.
- `crafting_queue`: active, blocked, and completed queue states.
- Input integration test: E/Escape recovery from every modal UI variant.
- Transaction/property tests: repeated craft, full inventory, cancellation,
  save/load, mod recipe, and event-exactly-once cases.

Z.ai review must answer concrete questions: What is the primary action? Which
ingredient is missing? Is the world visually subordinate? Is any text or slot
covered? Can health/hunger be distinguished? If the model cannot answer from
the image with high confidence, the scene fails.
