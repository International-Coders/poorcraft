# Mob AI Upgrade — Reference Document

## Current state (from prior sessions)

The game has 6 mob types (standard wander, chase/aggressive, ranged, Geode
Guardian, Cinder Crawler, Null Knight). The base AI uses 1-block hop
pathfinding with basic wander/chase/flee. A* pathfinding is deferred.

## Design principles for this upgrade

**Rule 1: AI should feel intentional, not random.** A mob that wanders
aimlessly with no apparent goal reads as a bug. A mob that walks to the
edge of its territory, stops, looks around (via a pause in movement), then
turns and walks back reads as a creature with a home.

**Rule 2: Complexity scales with mob tier.** Common mobs (wander/chase/
ranged) get the full state machine. Boss mobs (Geode Guardian, Cinder
Crawler, Null Knight) have their own special AI defined separately — do
not apply the generic state machine to bosses.

**Rule 3: Performance over correctness at scale.** A* for every mob every
tick is not acceptable. A* is used for Chase/Investigate state, computed
once when the state is entered, then the cached path is followed. The
path is recomputed only if the target moved > 4 blocks from where it was
when the path was computed.

## Per-mob-type configuration

Each mob archetype in the data files (or registry) gets these fields:
```toml
[mob.wander_mob]
aggro_radius    = 8.0     # blocks — how close player must be to trigger Chase
melee_range     = 1.5     # blocks — how close mob must be to Attack
flee_speed      = 1.3     # multiplier on base movement speed while fleeing
is_passive      = false   # true = never Chase, only Flee when attacked
faction_id      = ""      # blank = no faction, always hostile
group_aggro     = true    # triggers nearby same-type mobs on Chase

[mob.passive_mob]
aggro_radius    = 0.0     # 0 = never aggressively initiates
is_passive      = true
flee_speed      = 1.5

[mob.geode_guardian]
# Boss — handled by separate BossAI, not the generic state machine
use_boss_ai     = true
```

## A* implementation notes

The A* in `lf_game` should be a standalone module `mob_pathfind.rs`
(or added to the existing pathfind.rs if one exists). Key constraints:

- **Max nodes: 256.** If the pathfinder explores 256 nodes without finding
  the goal, return `None`. The mob then falls back to direct 1-block-hop
  movement toward the target.
- **Only cardinal moves + 1-block jump.** No diagonal movement.
- **Terrain awareness:** a block is passable if: it is air or a non-solid
  block, AND the block below it is solid (mob can stand on it), AND the
  block at mob-height + 1 is not solid (mob can fit through the gap).
- **Cache lifetime:** a cached path is valid for 2 seconds or until the
  target moves > 4 blocks, whichever comes first.

## Line-of-sight usage pattern

LOS is cheap (DDA raycast, max 32 blocks). It is called:
- Once per tick per mob in Chase state to check if the target is still
  visible (if not, trigger Investigate transition).
- Once when transitioning from Wander → to check if the player is visible
  before committing to Chase (prevents mobs from "seeing through walls").

LOS is NOT called:
- Every frame (too expensive at mob scale).
- For the Idle or Wander states (mobs that are idle don't actively look
  for players — they only react when the player enters aggro_radius via
  distance check, which is cheaper than LOS).

## Group aggro design

Group aggro creates the danger of mob encounters. Design rules:
- Spread through the group with a 0.5s per-mob delay so they don't all
  turn at exactly the same time (simultaneous pivot looks robotic).
- Group aggro does NOT chain infinitely. Only first-order neighbours
  (mobs within 8 blocks of the originally-aggroed mob) are affected.
- Group aggro does NOT apply if the mob that was originally aggroed is
  a passive mob that fled — fleeing doesn't trigger group combat.

## Boss AI (separate from the generic state machine)

The Geode Guardian, Cinder Crawler, and Null Knight use a separate
`BossPhase` enum instead of `MobBehaviourState`:
```rust
pub enum BossPhase {
    Dormant,          // before player enters the boss's territory
    Awakening { timer: f32 }, // brief animation/warning before combat
    Combat,           // active combat phase, using boss-specific attack patterns
    Enraged { hp_threshold: f32 }, // below a HP threshold, different behaviour
    Defeated,         // death sequence
}
```

Do not implement full boss AI in this prompt — that is a separate task.
Only ensure the generic state machine has a `use_boss_ai` flag that skips
the generic system for boss-typed mobs. Bosses in `Dormant` state should
use a simple "stand still" fallback.
