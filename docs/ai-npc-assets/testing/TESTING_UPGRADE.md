# Testing Upgrade — Reference Document

## Current state

The project has 123 tests (`cargo test --workspace`). The smoke test
launches the binary, waits 12 seconds, and checks if the process is alive.
The vistest harness runs a fixed set of scenes and pixel-analyzes PNGs.

## Why the current smoke test is insufficient

"The binary didn't crash in 12 seconds" tells you almost nothing. It doesn't
tell you:
- Whether the game can create a world.
- Whether mob AI runs without panicking.
- Whether the crafting system can complete a recipe.
- Whether chunk streaming works at all.

A binary that shows a blank title screen for 12 seconds "passes" the
current smoke test. The upgraded smoke test must actually exercise game
logic.

## Headless mode design (`--smoke` flag)

The `--smoke` flag makes the game run without a window or GPU. This is
possible because:
- World generation, game logic, mob AI, NPC scheduling, and crafting are
  all CPU-side systems in `lf_game` and `lf_voxel`.
- The renderer (`lf_engine`) is only needed for the visual output.
- The existing vistest/headless.rs in `xtask` already runs parts of the
  engine headlessly — use that pattern.

The smoke mode checklist (all must complete without panic):
1. Generate a Superflat world with seed=42.
2. Spawn the player entity.
3. Run 300 game ticks (simulated time, not real time — just call
   `game.tick(dt)` in a loop with `dt = 1.0/20.0`).
4. During those ticks, as events: spawn 1 passive mob + 1 hostile mob,
   step their AI for all 300 ticks.
5. Spawn 1 NPC (faction: accord), step their schedule for the 300 ticks.
6. Perform 1 crafting operation: craft planks from oak logs (a basic recipe
   that should always be available).
7. Mine 1 block: remove a block from the Superflat world at (0, 63, 0).
8. Print a summary: "SMOKE PASS: world_gen=OK, ai_ticks=300, npc_ticks=300,
   craft=OK, mine=OK" and exit with code 0.

Any panic at any step exits with code 1 and prints the panic message.

## Vistest assertion API

If the current vistest doesn't have a typed `Assertion` enum, add one.
Minimum required assertions for the new scenes:

```rust
pub enum Assertion {
    /// Fail if any contiguous black region (all channels < threshold_value)
    /// larger than min_size×min_size exists in the centre region_percent of the frame.
    NoBlackRect {
        min_size: u32,          // 64 recommended
        region_percent: u32,    // 80 = centre 80% of frame
        threshold_value: u8,    // 8 recommended (near-black, not pure sky dark)
    },
    /// Fail if the surface of same-block-type N×N area shows a repeating 1×1
    /// tile pattern (i.e., connected textures are working if this passes).
    ConnectedSurface {
        block_count: u32,  // 3 for a 3×3 area
        axis: ViewAxis,
    },
    /// Fail if no entity of the given type moved at least min_distance blocks
    /// from its spawn position.
    EntityMoved {
        entity_type: &'static str,
        min_distance: f32,
    },
    /// Check that the pixel-analysis result matches the expected color range.
    PixelColorRange {
        region: PixelRegion,
        min_rgb: [u8; 3],
        max_rgb: [u8; 3],
    },
}
```

## Test-naming convention

New tests follow the existing naming convention (underscore_separated,
descriptive). Each test file has a clear module docstring. Each test
function has a one-line comment explaining what failure means.

Example:
```rust
#[test]
fn mob_ai_state_transitions() {
    // If this fails, the mob state machine has a broken transition.
    // Check MobBehaviourState transitions in lf_game/src/mob.rs.
    let mut mob = Mob::new(MobArchetype::Hostile);
    let player_pos = Vec3::new(4.0, 64.0, 0.0);
    mob.tick_ai(&MockWorld::flat(), player_pos, 1.0 / 20.0);
    // At aggro_radius=8, player at distance 4 should trigger Chase
    assert_eq!(mob.state, MobBehaviourState::Chase { .. });
}
```

## Test quality rules

A test must fail when the implementation is wrong, not just when the
test is misconfigured. Before each new test, check: "If I delete the
implementation code this test is checking, does the test fail?" If not,
the test is checking nothing and must be rewritten.

Specific antipatterns to avoid:
- `assert!(true)` — always passes, tests nothing.
- Testing only that a function exists/compiles — does not test behaviour.
- Tests that construct the expected result from the same code as the
  implementation (circular verification).
