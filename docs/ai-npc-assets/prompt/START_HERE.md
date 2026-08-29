# LOREFORGE — AI/NPC Upgrade, Black-Square Fix, Texture Tooling & Connected Skins
## z.ai Prompt — paste this entire file

Codebase: Rust voxel RPG. Crate layout (from AGENTS.md):
- `crates/lf_engine` — wgpu 24 renderer, SceneResources/MeshBatch, atmosphere, compute path tracer
- `crates/lf_voxel` — block registry, meshing, BFS light, World + regions
- `crates/lf_worldgen` — 30-biome worldgen, trees, structures, ore veins
- `crates/lf_game` — survival, items, crafting, machines, combat, research eras
- `crates/lf_client` — input, streaming, block entities, ui.rs, ui_kit.rs, net.rs
- `apps/loreforge` (client), `apps/loreforge-server` (dedicated UDP)
- `xtask` — vistest/screenshot/package

Stack: wgpu 24, winit 0.30, egui 0.31 (version-locked, do not upgrade).
Known gotchas: egui pass MUST be encoded before texture readback or UI vanishes
from screenshots. `RenderPass<'static>` via scoped transmute in ui.rs/headless.rs.

All AGENTS.md rules apply. Cargo test --workspace currently passes 123 tests — it
must stay green after every section. Read AGENTS.md before touching any code.
Work top to bottom. Verify each section before moving to the next.

Reference files in this folder (read each before its section):
- `rendering/BLACK_SQUARE_FIX.md` — before Section A
- `ai/MOB_AI_UPGRADE.md` — before Section B  
- `npc/NPC_BEHAVIOUR_UPGRADE.md` — before Section C
- `testing/TESTING_UPGRADE.md` — before Section D
- `textures/CONNECTED_TEXTURES.md` — before Section E
- `textures/ASSET_GENERATOR.md` — before Section F

---

## SECTION A — Fix the black square in the viewport

Read `rendering/BLACK_SQUARE_FIX.md` before writing any code.

A persistent black square appears in the player's view during gameplay.
This is a rendering artifact. Find and fix its root cause.

### A1 — Identify the source

The black square is almost certainly one of these four causes (check in
this order — they go from most likely to least):

**Cause 1 — Stale chunk mesh batch with uninitialized/zero-cleared vertex
buffer.** When a chunk is queued for upload but its mesh generation returns
empty, the batch entry may still be drawn with a zero-filled vertex buffer,
producing a black quad where the chunk should be. Check `lf_engine`'s
`MeshBatch` and `SceneResources` — look for any path where a mesh upload
happens with `vertex_count == 0` but the draw call still fires.

**Cause 2 — egui debug/overlay rect.** egui's default debug features
include painting rectangles around certain panels or the cursor interaction
area. If a debug layer was left enabled, it renders as a solid rect.
Search `lf_client/src/ui.rs` for `debug_paint_pointer_pos`,
`style.debug`, `visuals.debug` — disable all debug paint options.

**Cause 3 — Skybox/atmosphere rendering order.** If the atmosphere quad
(sky, fog, clouds) is rendered AFTER an opaque mesh pass but BEFORE the
egui pass and uses `Load` instead of `Clear` for the depth attachment,
a previous-frame depth stencil can mask geometry, leaving a black area.
Check the render pass descriptors in `lf_engine` for any pass using
`LoadOp::Load` on the depth attachment where `LoadOp::Clear` is correct.

**Cause 4 — Path tracer persistent buffer.** The compute path tracer
(`Pathtracer`) maintains a persistent accumulation buffer. If this buffer
is not properly cleared on scene change (load new world, title screen
transition), the stale buffer may be composited as a black patch over
the new frame. Check `lf_engine/src/pathtrace.rs` (or similar) for
buffer invalidation on scene transitions.

### A2 — Implement the fix

After identifying the cause, implement a targeted fix. Do not refactor
surrounding code as part of this fix — change only what is necessary to
eliminate the black square. The fix must:

- Be accompanied by a new vistest scene or an assertion in an existing
  scene that would have caught this bug (i.e., the fix must be testable).
- Not regress any of the 123 existing tests.
- Include a comment in the fixed code: `// fix: black-square artifact —
  [brief description of what was wrong]` so future readers know why this
  code is structured the way it is.

**Verify:** run `cargo run --release -p xtask -- vistest shots` and confirm
no scene produces a frame with a large solid-black rectangular region that
is not the sky or expected dark background. Add a pixel-analysis assertion
to the vistest that fails if any frame contains a rectangular region of
pure black (0, 0, 0) pixels larger than 64×64 that is not in the sky area.

---

## SECTION B — Mob AI upgrade

Read `ai/MOB_AI_UPGRADE.md` before writing any code.

The existing mob AI in `lf_game` uses basic wander/chase/flee behaviour
with 1-block hop pathfinding. This section upgrades it to be more
interesting and believable without becoming expensive.

### B1 — Behaviour state machine (replace the current ad-hoc state)

Replace any existing boolean/enum mob state with a proper
`MobBehaviourState` enum in `lf_game/src/mob.rs` (or wherever mobs live):

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum MobBehaviourState {
    /// Default: exploring territory, no target
    Wander { timer: f32, next_pos: Option<BlockPos> },
    /// Target acquired, moving toward it
    Chase { target_entity: EntityId, aggro_timer: f32 },
    /// Taking damage, moving away from threat
    Flee { threat_pos: Vec3, flee_timer: f32 },
    /// Cannot see target, searching last known position
    Investigate { last_known: BlockPos, search_timer: f32 },
    /// In combat range, executing attack
    Attack { target_entity: EntityId, cooldown: f32 },
    /// After long combat with no kill, mob disengages
    Disengage { cooldown: f32 },
    /// Mob is idle (passive mobs, or between behaviours)
    Idle { timer: f32 },
}
```

State transitions (implement all of these):

```
Idle → Wander: after idle_timer expires (random 3–8s)
Wander → Chase: player enters aggro_radius AND mob is hostile AND has line of sight
Wander → Flee: mob takes damage AND is a passive mob
Chase → Attack: distance to target ≤ melee_range (1.5 blocks)
Chase → Investigate: target leaves line of sight for > 2s
Chase → Disengage: chase has lasted > 30s without a kill
Attack → Chase: after attack cooldown, target moved out of melee_range
Investigate → Chase: target re-enters line of sight during investigation
Investigate → Wander: search_timer expires (random 8–15s) without finding target
Flee → Wander: flee_timer expires AND threat is no longer in sight
Disengage → Idle: cooldown expires
```

### B2 — Line-of-sight check (cheap raycast)

Add a `has_line_of_sight(from: Vec3, to: Vec3, world: &World) -> bool`
function in `lf_game`. This is a DDA voxel raycast along the vector from
mob eyes to target eyes — check each block along the ray; return false if
any solid block is hit before the target. Maximum check distance: 32 blocks.
Cache the result per mob per tick (don't recompute every frame).

Line of sight is required for:
- Triggering Chase from Wander (mob must see the player).
- Maintaining Chase (lose sight → Investigate).
- Fleeing (passive mob must see the threat to flee; if it loses sight, it
  transitions to Wander even if not at flee_timer end).

### B3 — Faction-aware aggression

Mobs have a `faction_id: Option<String>` field (using the faction system
from the prior lore pack). Mob aggression toward the player is modulated
by the player's faction standing:

```rust
fn effective_aggro_radius(base_radius: f32, standing: i32) -> f32 {
    // Hostile faction: standing drives radius. At +75 (honored), peaceful.
    // At -75 (enemy), extended radius.
    let standing_factor = 1.0 - (standing as f32 / 100.0).clamp(-1.0, 1.0);
    // standing_factor: 0.0 at +100 standing (no aggro), 2.0 at -100 standing
    base_radius * standing_factor
}
```

If `effective_aggro_radius == 0.0` (standing is very high), the mob
ignores the player entirely unless attacked. Implement this in the
Wander → Chase transition check.

### B4 — Group behaviour (simple flocking for same-type mobs)

When 2+ mobs of the same type are within 8 blocks of each other and one
enters Chase or Attack state, the others enter Chase state toward the same
target with a 0.5s delay. This makes mob encounters feel like a real
threat rather than mobs taking turns.

Limit: only trigger group aggro if the group size is ≤ 5. Beyond 5 mobs,
performance and player experience both degrade.

### B5 — Improved pathfinding (A* for short distances)

The current 1-block hop is fine for simple movement but produces mobs
that get stuck behind single blocks. Add A* pathfinding for paths up to
16 blocks long:

```rust
/// Returns a path of block positions from start to goal, or None if no
/// path found within max_nodes explored.
pub fn find_path(
    start: BlockPos,
    goal: BlockPos,
    world: &World,
    max_nodes: usize,  // hard cap at 256
) -> Option<Vec<BlockPos>>
```

Heuristic: Manhattan distance. Neighbours: 4 cardinal directions + 1-block
up/down jumps (same as current). Diagonal movement is NOT allowed (it
produces diagonal floating that looks wrong in a voxel world).

Use this for Chase and Investigate states. Continue using the simple hop
for Wander (A* is overkill for random wandering).

**Verify:** a new test `mob_ai_state_transitions` that simulates a mob
stepping through each state transition with mock world/player state and
asserts the resulting `MobBehaviourState` is correct. A separate test
`mob_pathfinding_basic` that verifies A* finds a correct path around a
single-block obstacle. Both must pass as part of the 123+N test suite.

---

## SECTION C — NPC behaviour upgrade

Read `npc/NPC_BEHAVIOUR_UPGRADE.md` before writing any code.

NPCs (villagers, faction NPCs, companions from the lore pack) share a
schedule system. This section makes them feel more alive.

### C1 — Schedule enrichment

The existing schedule has villagers following a time-of-day routine.
Add the following schedule slots to every NPC's day (TOML-driven, per
the existing NPC archetype data pattern):

```toml
[[schedule_slot]]
time_start = 0.0    # 0.0–1.0 = midnight–midnight
time_end   = 0.25   # midnight to 6am
activity   = "sleep"
location   = "bed"  # the nearest bed block in their structure

[[schedule_slot]]
time_start = 0.25
time_end   = 0.35
activity   = "eat"
location   = "table"  # the nearest crafting table or chest

[[schedule_slot]]
time_start = 0.35
time_end   = 0.75
activity   = "work"
location   = "workstation"  # faction-specific: forge = Ironborn, shrine = Covenant, etc.

[[schedule_slot]]
time_start = 0.75
time_end   = 0.85
activity   = "socialize"
location   = "gather"  # wander within 8 blocks of their structure's centre

[[schedule_slot]]
time_start = 0.85
time_end   = 1.0
activity   = "return_home"
location   = "door"
```

Activities affect: movement target (NPC pathfinds to the slot's location),
animation state (a flag on the entity for idle/work/eat/sleep), and the
dialogue they give when interacted with during that activity.

### C2 — Contextual idle animations (visual state flags)

Add an `NpcActivityState` enum to the NPC entity:

```rust
pub enum NpcActivityState {
    Walking,
    Idle,         // standing, looking around (random head-turn timer)
    Working,      // at workstation — slight bob animation
    Eating,       // at table — cup/bowl raise animation (if model supports it)
    Sleeping,     // horizontal / eyes closed (use a prone model offset)
    Socializing,  // walking slowly between nearby NPCs
}
```

These states drive the visual appearance (which model offset/rotation is
applied during rendering) and the dialogue posture (an NPC who is
sleeping gives a drowsy one-liner before going back to sleep; one who is
working gives a quick response without stopping work).

### C3 — Reaction events

NPCs react to the following world events when they occur near the NPC
(within 24 blocks):

| Event | NPC reaction |
|---|---|
| Player breaks a block in the NPC's faction structure | NPC turns toward the player; standing penalty already handles the economics; add a short dialogue line: "[Name]: Hey! Watch what you're doing." |
| Combat starts near NPC | NPC pathfinds away from combat (minimum 16 blocks) and enters a Fleeing sub-state for 10s. Hostile faction NPCs join the fight instead. |
| Player gives an NPC a gift item (new: drop item on NPC) | NPC picks up the item, standing +2, contextual thank-you dialogue line. |
| A companion's morale hits 0 near another NPC of same faction | That NPC gives a one-line reaction: "I see [Companion] has had enough. Can't say I'm surprised." |
| Player reaches +75 standing with a faction | All NPCs of that faction give a brief acknowledgement on next interaction: "The [title] walks among us." |

Reactions are one-line chat messages using the existing chat/UI system —
no new UI required.

### C4 — NPC memory (per-world, not per-session)

NPCs remember the last two significant interactions with the player
(stored in the world save alongside mob/entity state):

```rust
pub struct NpcMemory {
    pub last_interaction: Option<InteractionRecord>,
    pub prior_interaction: Option<InteractionRecord>,
}

pub struct InteractionRecord {
    pub event: NpcEvent,  // enum: Gifted, Traded, QuestGiven, QuestCompleted, Attacked, Dismissed(companion)
    pub day: u32,         // in-game day number
}
```

On interaction, NPCs reference their memory in the opening line when
relevant. Examples:
- "Back again? Good. I have more work." (if last_interaction was a
  completed quest)
- "Still carrying those items I gave you?" (if player last traded)
- "I remember what you did to our [building/NPC]. Be careful." (if
  player's last interaction was an attack that didn't cross –30 standing)

Memory is only referenced if the prior interaction was within the last
5 in-game days. After that, the NPC treats the player as a stranger again.

**Verify:** a test `npc_schedule_activity` that steps the in-game time
and confirms an NPC's activity state changes correctly at each schedule
boundary. A test `npc_memory_persistence` that creates an NPC, records
an interaction, serializes world state, reloads it, and confirms the
memory survived.

---

## SECTION D — Testing upgrade

Read `testing/TESTING_UPGRADE.md` before writing any code.

### D1 — Expand the vistest scene suite

The existing `xtask` vistest runs a fixed set of scenes. Add these new
scenes that specifically cover the systems this prompt introduces:

Add the following scene definitions to `xtask/src/main.rs` (or wherever
the scene list lives):

```rust
VistestScene {
    name: "no_black_square",
    seed: 12345,
    camera: /* slightly above terrain, facing horizon */,
    description: "verifies no large black rectangle in gameplay view",
    assertions: vec![
        // fail if any 64×64 region in the centre 80% of the frame
        // is more than 95% pure black (0,0,0) pixels
        Assertion::NoBlackRect { min_size: 64, region: CentrePercent(80), threshold: 0.95 },
    ],
},
VistestScene {
    name: "connected_textures_grass_3x3",
    seed: 99999,
    camera: /* looking down at a 3×3 flat grass surface from above */,
    description: "verifies connected texture UV covers 3×3 as one large tile",
    assertions: vec![
        // The 3×3 area must NOT show a repeating 1×1 grid pattern
        // (i.e., the centre pixel of each block-face must differ from
        //  the equivalent centre pixel of its neighbour by < 8 grey value)
        Assertion::ConnectedSurface { blocks: 3, axis: TopDown },
    ],
},
VistestScene {
    name: "mob_ai_visible",
    seed: 77777,
    description: "spawns a mob, steps AI 120 ticks, verifies it moved",
    // headless AI-tick test, no visual assertion needed — just confirm
    // the mob entity's position changed from its spawn position
    assertions: vec![
        Assertion::EntityMoved { entity_type: "mob", min_distance: 1.0 },
    ],
},
VistestScene {
    name: "npc_schedule_time",
    seed: 11111,
    description: "sets time to 0.5 (midday), verifies NPCs are in Work state",
    assertions: vec![
        Assertion::NpcActivity { expected: NpcActivityState::Working },
    ],
},
```

If the `Assertion` enum or `VistestScene` struct doesn't exist in its
exact form, adapt to the actual structure in `xtask/src/main.rs` —
the scene concepts must be implemented even if the API shape differs.

### D2 — Smoke test hardening

The current smoke test (launch binary, sleep 12s, check alive) is too
coarse. Upgrade it:

```bash
# New smoke.sh (or integrate into Makefile's smoke target):
#!/usr/bin/env bash
set -e

BINARY="target/release/loreforge"
LOG="smoke_run.log"

# Launch headless with a test world flag (add --smoke flag to the binary
# that generates a tiny world and exits after 5s of game ticks without
# a window — see D3 below)
timeout 30 "$BINARY" --smoke > "$LOG" 2>&1 &
PID=$!
sleep 20
if ! kill -0 $PID 2>/dev/null; then
  echo "SMOKE FAIL: binary exited early"
  cat "$LOG"
  exit 1
fi
kill $PID
wait $PID 2>/dev/null || true

# Check log for known error patterns
if grep -qE "(PANIC|thread.*panicked|ERROR.*wgpu|vulkan.*error)" "$LOG"; then
  echo "SMOKE FAIL: error pattern found in log"
  grep -E "(PANIC|thread.*panicked|ERROR.*wgpu|vulkan.*error)" "$LOG"
  exit 1
fi

echo "SMOKE PASS"
```

### D3 — Headless smoke mode (`--smoke` flag on the binary)

Add a `--smoke` command-line argument to `apps/loreforge/src/main.rs`.
When `--smoke` is passed:
- Create a world with seed=42 (Superflat, Peaceful difficulty).
- Run 300 game ticks (about 5 seconds of simulated time).
- During those ticks: spawn one mob, step its AI, spawn one NPC, step
  its schedule, perform one crafting operation.
- Exit with code 0 if all operations completed without panic.
- Exit with code 1 if any operation panicked or produced an error.
- No window is opened. No GPU is required (use the existing headless path
  if one exists, or add a `cfg(feature = "headless")` flag).

This makes the smoke test genuinely useful — it exercises real game logic
in a reproducible, fast, windowed-display-free way.

### D4 — Test count target

After completing all sections in this prompt, the test suite should have
at minimum 123 + 8 new tests = 131 tests. The new tests are:

1. `mob_ai_state_transitions`
2. `mob_pathfinding_basic`
3. `mob_los_check`
4. `mob_group_aggro`
5. `npc_schedule_activity`
6. `npc_memory_persistence`
7. `connected_texture_uv_3x3`
8. `asset_generator_grass_output`

Write each test before the implementation it tests (TDD for this section).
A test that doesn't test anything real (always passes regardless of
implementation) is worse than no test — write real assertions.

**Verify:** `cargo test --workspace` reports ≥ 131 tests passing, 0 failing.

---

## SECTION E — Connected textures (neighbour-aware UV mapping)

Read `textures/CONNECTED_TEXTURES.md` before writing any code.

This is the "stretch a skin over a 3×3 field of grass instead of tiling
1×1" feature you described. The correct name is **connected textures** —
a system where a block's rendered face UV depends on which adjacent blocks
are the same type, so large surfaces of the same material render as a
single large texture region rather than a repeating 1×1 tile.

### E1 — Why this works at 16×16 or 32×32

At 16×16 pixels per face, a 3×3 area of the same block type is 48×48 pixels.
The texture can be designed with features that span the full 48×48 (a
large central motif, subtle long-range gradients, occasional detail
elements) that would be invisible in a 1-tile-repeat pattern. This is
what makes large grass fields look like a meadow rather than a tiled floor.

### E2 — The neighbour bitmask

For each block face being meshed, compute an 8-bit neighbour bitmask
(for top faces; other faces use a 4-bit bitmask for the 4 cardinal
neighbours in the face plane):

```
Bit layout for a top face (looking down):
  NW=7  N=6  NE=5
   W=4       E=3
  SW=2  S=1  SE=0

bitmask = 0b_NW_N_NE_W_E_SW_S_SE
```

Two blocks are "same type" for connectivity if they share the same block
ID. `AIR` never connects to anything. Do not connect across block
boundaries (a grass block does not connect through a stone block to another
grass block on the other side).

For diagonal bits (NW, NE, SW, SE): only set the diagonal bit if BOTH
of its cardinal neighbours are also set (i.e., NW is only set if both N
and W are set). This is the standard CTM rule that prevents corner
artifacts.

```rust
fn top_face_bitmask(x: i32, y: i32, z: i32, block_id: u8, world: &World) -> u8 {
    let same = |dx: i32, dz: i32| -> bool {
        world.block_at(x + dx, y, z + dz).map(|b| b == block_id).unwrap_or(false)
    };
    let n = same(0, -1);
    let s = same(0,  1);
    let e = same(1,  0);
    let w = same(-1, 0);
    let ne = n && e && same(1, -1);
    let nw = n && w && same(-1, -1);
    let se = s && e && same(1,  1);
    let sw = s && w && same(-1,  1);
    (nw as u8) << 7 | (n as u8) << 6 | (ne as u8) << 5
    | (w as u8) << 4                 | (e as u8) << 3
    | (sw as u8) << 2 | (s as u8) << 1 | (se as u8)
}
```

### E3 — CTM UV selection (the 47-tile method)

There are 47 meaningful bitmask configurations (out of 256, many are
equivalent). The standard connected-texture tileset has 47 tiles arranged
in a 12×4 or 8×6 texture atlas strip. For LOREFORGE's 16×16 tiles, the
CTM strip for a block type is stored as a separate asset at
`assets/ctm/<block_id>.png` (a 48×64 image containing 12 tiles wide ×
4 tiles tall, each tile 16×16, covering the 47 configurations).

Map bitmask → tile index using the standard CTM lookup table:

```rust
/// Standard 47-tile CTM bitmask to tile-index lookup.
/// tile_index is 0..46, laid out row-major in the 12-wide strip.
pub fn ctm_tile_index(bitmask: u8) -> u8 {
    // This table maps all 256 bitmask values to one of the 47 tiles.
    // Values derived from the standard Minecraft CTM spec.
    CTM_TABLE[bitmask as usize]
}

const CTM_TABLE: [u8; 256] = [
    // bitmask 0 (isolated) → tile 0 (fully surrounded, looks centred)
    // ... full 256-entry table in textures/CONNECTED_TEXTURES.md
    // For now, implement the minimal version:
    // 0 neighbours = tile 46 (isolated)
    // all 8 neighbours = tile 0 (interior, fully connected)
    // use linear interpolation for intermediate cases
    // The full table is in the reference file.
    46, /* 0x00 - isolated */
    // ... see reference file for complete table
    0,  /* 0xFF - fully surrounded */
];
```

The full 256-entry lookup table is in `textures/CONNECTED_TEXTURES.md`.

### E4 — Integration with the mesher

The mesher in `lf_voxel` currently assigns UV coordinates based only on
block ID and face direction. Extend this:

1. For block types that have a CTM strip (`has_ctm(block_id) -> bool`),
   call `top_face_bitmask` (or the equivalent for side/bottom faces) and
   look up the CTM tile index.
2. Use the tile index to compute UV coordinates within the CTM strip texture
   rather than the main atlas texture.
3. The CTM strip is a separate texture bound alongside the main atlas —
   add it as a second texture binding in `lf_engine`'s SceneResources if
   not already present.

**Opt-in per block type.** Only blocks that have a CTM asset get connected
textures. All other blocks continue to use the standard atlas UV.
Start with grass as the first block to get CTM treatment, since it was
specifically called out.

### E5 — CTM-enabled block list (initial set)

Add CTM assets for these block types in this order (grass first, verify it
works, then the rest):

1. `grass` (top face only — side faces keep the standard dirt-side texture)
2. `sand`
3. `water` (top face)
4. `snow`
5. `bog_peat` (from the biome blocks added previously)
6. `permafrost`
7. `accord_stone` (large surfaces of this should read as architectural stone)
8. `ashen_marble`

CTM is NOT applied to: stone, dirt, wood planks, ores (too small to benefit),
or any block that's not expected to appear in large flat surfaces.

### E6 — CTM asset generator stub (bootstrapping the textures)

Since the CTM strip textures (`assets/ctm/*.png`) don't exist yet,
add a command to `xtask` that generates a placeholder CTM strip for a
given block type from its existing 1×1 atlas tile:

```
cargo run --release -p xtask -- gen-ctm <block_id>
```

This command:
1. Reads the block's existing 16×16 atlas tile.
2. Creates a 192×64 image (12 tiles × 4 tiles, each 16×16).
3. Fills all 47 tile positions with the original tile (so the block looks
   exactly the same as before — no regression).
4. Saves to `assets/ctm/<block_id>.png`.

This placeholder ensures the CTM system works before the art assets are
designed. The actual distinct CTM tile designs (the meadow motif, the
subtle gradients) are applied by a human or the asset generator in
Section F.

**Verify:** a test `connected_texture_uv_3x3` that places a 3×3 grass
surface, calls `top_face_bitmask` on the centre block, and asserts the
result is `0xFF` (all 8 neighbours present). A vistest scene
`connected_textures_grass_3x3` that confirms the centre block's UV
coordinates differ from a standalone grass block's UV coordinates.

---

## SECTION F — Procedural asset generator tool

Read `textures/ASSET_GENERATOR.md` before writing any code.

This is a new `xtask` subcommand that generates pixel-art texture assets
programmatically. The goal is to produce consistent, game-appropriate
16×16 or 32×32 textures without a human artist having to paint every
tile — but without them looking AI-generated (random noise with no rules
looks worse than a hand-painted tile; the tool must follow explicit
pixel-art rules).

### F1 — `xtask gen-texture` command

```
cargo run --release -p xtask -- gen-texture <type> <output.png> [--size 16|32] [--seed N] [--palette <name>]
```

Arguments:
- `type` — what kind of texture to generate (see type list below)
- `output.png` — path to write the PNG
- `--size` — 16 (default) or 32
- `--seed` — random seed for reproducible output
- `--palette` — named colour palette from the LOREFORGE palette system

Texture types and their generation rules:

**`grass-ctm-strip`** — generates a full 192×64 CTM strip for grass.
Rules:
- Base colour: #5a8a2a (medium grass green).
- Each tile gets subtle variation: ±10% brightness per-tile, seeded by
  tile index × seed.
- Interior tiles (bitmask = fully surrounded) get a few randomly-placed
  darker green pixels (1–3 per tile) to simulate depth/variation.
- Edge tiles get a 1-pixel darker border on their exposed edges only.
- Corner tiles get a 1-pixel rounded-corner detail.
- Result: tiles tile seamlessly and produce a field that reads as one
  large meadow surface, not a grid.

**`stone-ctm-strip`** — CTM strip for stone/accord_stone.
Rules:
- Base colour: #6a6a6a (mid grey).
- Subtle noise pattern (Perlin-ish using the seed, hand-coded — do NOT
  use external noise crates in xtask; implement a simple 2D hash noise):
  `noise(x, y) = fract(sin(x * 127.1 + y * 311.7) * 43758.5)`
- Interior tiles show subtle cracking detail (2–4 randomly placed 1-pixel
  dark lines of length 2–4px, oriented randomly).
- Edge tiles have a 1-pixel shadow on the interior side of exposed edges.

**`entity-skin`** — generates a flat entity skin texture.
Arguments: `--faction <id>` (accord/ironborn/covenant/freeholds/ashen/nameless)
Rules:
- Body region: faction primary colour (from the LOREFORGE colour palette).
- Clothing detail region: faction accent colour.
- Face region: neutral warm skin tone (#c4956a), with 2-pixel-wide eyes
  in a dark colour (faction-coloured iris: 1×1 pixel each).
- Faction symbol stamp: a 4×4 pixel pattern (defined per faction in the
  asset generator code) centered on the chest face region.

**`block-noise`** — a simple noisy block texture.
Arguments: `--base-color <hex>` `--variation 0..50` (brightness variation %)
Rules:
- Fill with base colour.
- Apply per-pixel brightness variation using the hash noise above.
- Apply 1-pixel darker edge (all 4 sides) for a slight "block" look.

### F2 — Batch generation

```
cargo run --release -p xtask -- gen-all-textures
```

Runs the generator for:
- All 8 CTM block types from E5 (using placeholder or styled tiles).
- All 6 faction entity skins.
- Any block-noise textures for blocks that still have placeholder colours.

Produces a summary log: which assets were generated, their output paths,
and any assets that already existed and were skipped (do not overwrite
existing hand-crafted assets — check for the file first).

### F3 — Deterministic output requirement

Given the same `--seed` and `--type`, `gen-texture` must produce bit-
identical output every time, on any machine. This means:
- No floating-point variance (use integer arithmetic everywhere in the
  generator).
- No system time, no `rand::random()`, no OS-seeded RNG — use a seeded
  PRNG explicitly (implement a simple xorshift64 in the xtask source
  rather than pulling in `rand`).

**Verify:** a test `asset_generator_grass_output` that runs the grass CTM
strip generator with seed=42 twice and asserts the output bytes are
identical. A visual inspection step in DEVLOG.md: generate the grass CTM
strip with seed=42, view it, and write one sentence confirming it looks
like a plausible grass texture (no pure white, no pure black, variation
is visible).

---

## SECTION G — Honest wrap-up

### G1 — Full test suite
`cargo test --workspace` must report ≥ 131 tests, 0 failing.

### G2 — Vistest suite
`cargo run --release -p xtask -- vistest shots` — every scene must pass its
assertions. The `no_black_square` scene must pass. The
`connected_textures_grass_3x3` scene must show different UV coordinates
for the centre block vs. an isolated block.

### G3 — Smoke test
`make smoke` (using the upgraded D3 headless flag) must exit 0.

### G4 — Build runtimes and push
Per AGENTS.md mandatory desktop runtimes section: build release binaries,
produce macOS `.dmg` and Linux `.tar.gz` artifacts. Report artifact paths.
Then `git push github HEAD`.

### G5 — Honest BACKLOG.md and DEVLOG.md
One DEVLOG.md entry covering all sections: what was done, what files were
touched, what the test evidence shows, what is honestly deferred with a
specific reason and no vague language. Update BACKLOG.md — uncheck anything
not fully completed.

Honest deferral is not failure. "CTM strip art for 6 of 8 blocks completed;
sand and permafrost use placeholder tiles pending visual review" is a good
DEVLOG entry. "All tasks complete" when sand still has a placeholder tile
is not.
