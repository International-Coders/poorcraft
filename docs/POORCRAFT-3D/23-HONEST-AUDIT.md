# POORCRAFT 3D — Honest Audit

Date: 2026-09-21

This is POORCRAFT 3D's first state file. The repository root's `STATE.md`,
`BACKLOG.md`, and `CHANGELOG.md` track LOREFORGE (loop 472), not this
subproject. Everything below was measured on this host today, not read from a
prior report.

The scorecard is the ten-step beta journey in
`02-DESIGN-PILLARS-AND-PROGRESSION.md` lines 71-80, because that document is
the pack's own definition of the minimum playable game.

## Baseline measured today

| Check | Command | Result |
| --- | --- | --- |
| Build | `cargo check --workspace` | green (warnings only) |
| Tests | `cargo test --workspace` | **693 pass / 0 fail** |
| Observatory | `make p3d-observe` | **FAILS** (`route_pit_wall`), exit 1 |
| Visual gates | `make p3d-visual-gates` | 10/10 "PASS" — but see below |

A note on running the suite: 40 `pc3d_render` tests abort with
`no suitable GPU adapter` (`gpu.rs:28`) when run inside a sandbox with no GPU.
They are not failures. Any GPU or windowed work on this host must run outside
the sandbox.

## The gates cannot fail

`make p3d-visual-gates` reports `ALL VISUAL GATES PASS`. The captures it
blessed include:

- `shots/windowed_city.png` — passes `castle-city` on an
  "opening/pillar delta 0.38" check. The frame is a small grey plate in an
  empty orange void. There is no visible city.
- `shots/windowed_wild_vista.png` — passes over untextured grey terrain with
  what look like canopies detached in the sky. (The grounded-geometry law
  added in Phase 1 audits the real placement authority and reports every
  plant correctly seated, so this one is a rendering/LOD artefact, not a
  placement bug. The law disproved the eyeball reading, which is the point of
  having it.)
- `playtest-a/.../play_interior.png` — captioned "interior" while the camera
  stands outdoors.

The gates assert distinct-colour counts, pixel deltas, and "no row shear".
None of them asserts that the subject of the proof is on screen.

Measured over every `*.layout.json` written by today's run:

- **290 text-on-text rectangle overlaps** across 32 layout dumps.
- **30 dumps have more than one exclusive panel open at once.**

Worst offenders: `dialog_hint` × `interact_hint` (30), `dialog_speaker` ×
`interact_line_1` (27), `forge_hint` × `forge_slots` (23).

Both classes of defect are fully visible in data the harness already writes,
and no gate looks at it.

## Journey scorecard

Legend: WORKS / PARTIAL / IMPOSSIBLE.

### 1. Create a named world and understand the first survival tasks — PARTIAL

NEW WORLD works (seed entry, reroll, quality tier, CREATE) and `create_world`
re-assembles the scene correctly. Health, stamina, and food bars are live, and
the quest journal seeds real tasks.

But `LOAD WORLD` is broken: `load_world` (`app.rs:2528-2562`) swaps
`slice.seed`, `slice.host`, and `slice.player` and never rebuilds
`slice.scene`. Loading a save whose seed differs from the running scene leaves
the player in the wrong world with the right blocks. `create_world`
(`app.rs:2563`) does re-assemble, so the asymmetry is the bug.

### 2. Build a shelter and a first useful production loop — PARTIAL

Build (F), remove (R), and dig (G) all work against the host command path and
are foundation-gated. Digging credits a yield with pack space checked before
the ground breaks.

There is no production loop, because there is no crafting. `Screen` has
exactly six variants — `Title, NewWorld, LoadWorld, Settings, Gameplay, Pause`
(`ui.rs:28-35`). There is no inventory screen; the "pack" is a one-line HUD
string (`items::stock_line`). Hotbar slots 6-9 are permanently empty.
`pc3d_world::craft` has tests and zero callers from the played game.

### 3. Discover a river and understand why it is valuable — PARTIAL

Rivers generate, mesh, and render with flow direction, and a dam edit remeshes
locally. The river is discoverable. Nothing in the played game explains or
uses its value, which is step 4.

### 4. Harness that river with a machine — IMPOSSIBLE

`HostCommand` defines `FeedBoiler`, `FuelReactor`, `SetRods`, `SpawnDragon`,
and `AssaultDragon` (`host.rs:48-70`). All five have **zero** references from
`pc3d_render` and `apps/`. Nothing constructs them, so no machine can ever be
operated.

Worse, and this is the single largest defect found: **the world simulation
never advances during play.** `SoloHost::run_ticks` drives machines, reactors,
settlements, and dragons. In the live loop it is called in exactly two places
— the build arm (`app.rs:3016`) and the remove arm (`app.rs:3157`). World
assembly ends on `run_ticks(0)` (`slice.rs:795`). The simulation only moves
forward in the instant the player places or breaks a block.

### 5. Travel far enough to find a clearly distinct castle or capital — PARTIAL

A settlement kit of 11 GLB modules is assembled and drawn, and the vertical
slice finds a real gate. But `city.rs` uses zero GLB assets — it is procedural
`push_city_box` geometry — and faction kit modules render as generic blocks
with no faction identity. The `castle-city` proof capture shows no city at
all. "Clearly distinct" is not met.

### 6. Meet residents whose jobs, laws, and reactions are visible — PARTIAL

Talk (E) resolves the nearest chest, ore node, marker, forge, or villager, and
villagers have generated names and dialogue lines. NPCs walk with a live crowd
law that yields correctly when two meet head-on.

Jobs are visible only through the `I` debug Bed/Work/Idle boxes, which are a
developer overlay, not player-facing. Laws and reactions are not visible at
all: `perception`, `relationships`, `karma_evidence`, `npc_death`, and
`oversight` have tests and zero callers anywhere. NPCs are procedural boxes
with no character art (`npcs.rs:160-241`).

### 7. Choose a technology, magic, exploration, or political direction — IMPOSSIBLE

No direction is selectable. `magic`, `ley`, `valve_computing`, `engineering`,
and `kits` all have logic and tests and zero callers. The forge is the only
production device reachable, and it is itself broken — see below.

### 8. Recruit a companion and see that companion help — IMPOSSIBLE

`pc3d_world::companion` has tests and zero callers from any binary.

### 9. Affect a faction through a witnessed action — IMPOSSIBLE

`faction` is reachable only from `ideology`, which is reachable only from
`kits`, which has no callers. `perception` and `karma_evidence` likewise have
none. The witnessing chain does not exist in the played game.

### 10. Continue the same world alone or with friends — PARTIAL

Save (B) and load (L) work in-session and survive a reload. Solo continuation
is real, subject to the `LOAD WORLD` scene bug in step 1. Multiplayer is
protocol-only: `P3D-802`/`P3D-803` closed without any two-client runtime
proof.

**Score: 0 of 10 fully working. 6 partial, 4 impossible.**

## Defects found in the played loop

### D1 — The world is frozen (highest severity)

Described under journey step 4. Machines, reactors, settlements, and dragons
are already ticked by `SoloHost`; they are simply never given time. Several
systems that look unwired are in fact wired and starved.

### D2 — Two advertised verbs are unreachable

The forge HUD prints `"G FUEL · H ORE · T TAKE · E CLOSE"` (`ui.rs:1363`) and
`ui.rs` handles all four actions. But `ui_key` (`app.rs:3847`) maps only
`E, J, D, X, G`, digits, and navigation keys. `KeyH` and `KeyT` are absent, so
`Key::Char('h')` and `Key::Char('t')` can never be produced. The ore→smelt→take
chain dead-ends at "load fuel".

A unit test covers the forge keys and passes, because it calls `on_key` with
abstract keys and never exercises `ui_key`. The test is blind to exactly the
layer that is broken.

### D3 — `D` is double-bound

`ui_key` maps `KeyD` to `Key::Char('d')` → `UiAction::DeliverAtSite`
(`ui.rs:2042`), and the handler then falls through to movement
(`app.rs:2826-2842`). Strafing right fires a delivery attempt on every key
press and every OS key repeat.

### D4 — `LOAD WORLD` does not rebuild the scene

Described under journey step 1.

### D5 — Panels stack instead of excluding

`state.dialog`, `state.journal`, `state.interact`, and `state.forge` are four
independent `Option`s (`ui.rs:1209, 1241, 1299, 1330`), each drawn when
`Some`, with nothing closing the others. 30 layout dumps show more than one
open simultaneously.

### D6 — Panels are fixed-size but flow their content

The forge panel hardcodes `panel_h = 150` (`ui.rs:1332`), flows content
downward via `fy`, then anchors its key hint to `panel.bottom()`. `forge_slots`
lands on top of `forge_hint` in 23 dumps.

### D7 — `route_pit_wall` fails

`make p3d-observe` exits 1. The route reports `wounded false` where the law
expects a wound. This is a real, pre-existing failure that the visual gate
battery does not cover.

### D8 — The tested movement law is not the played movement law

`pc3d_world::player` (`MoveInput`, `Player`, `WALK_SPEED`, `SIM_DT`) has zero
references from `pc3d_render` or `apps/`; only `diagnose.rs` and `journey.rs`
use it. The renderer carries a parallel `pc3d_render::player` with its own
`WALK_SPEED`/`SPRINT_SPEED` constants (`player.rs:13-16`). The movement the
tests guarantee and the movement the player feels are different code.

### D9 — The observatory's honesty mechanism is unused

`observe.rs:23-95` declares an `available` / `reason` field per route so a
route can honestly report that an interaction does not exist. All 17 routes
are hardcoded `available: true` with empty reasons.

### D10 — `dist3d/` ships two different builds

`dist3d/POORCRAFT3D.app/Contents/MacOS/poorcraft3d` is 8,686,776 bytes dated
Sep 15. `dist3d/POORCRAFT3D/poorcraft3d` is 7,355,956 bytes dated Sep 8. Both
carry a byte-identical `PLAY.md` dated Sep 10, so the folder copy ships a
binary older than its own instructions.

## What is genuinely good

Worth stating plainly, because the defect list above is long:

- The deterministic core (clock, command envelope, journal, seed streams,
  replay hash) is real and tested.
- Terrain, streaming, LOD, water flow, and local remeshing all work and are
  bounded. The stream walk holds a 24 MB GPU budget with a hard cap on meshes
  and uploads per frame.
- The GLB pipeline genuinely works: 1,324 compiled assets, multi-LOD, named
  sockets, lazily uploaded flora variants with a negative cache.
- The vine grip system (grab, climb, tip release) is a complete, tested,
  three-law mechanic.
- Fall damage, plaza recovery, and the crowd yield law all pass real routes.
- `inventory.rs` is an anti-overclaiming validator that cross-checks claimed
  renderer paths, Makefile gates, and GLB files on disk in both directions.

The problem is not workmanship. It is that the proof harness grew teeth for
rendering and geometry while the player-facing loop grew none, so defects that
only a player would notice went unrecorded.

## Order of work

1. Give the gates teeth, so every later fix is provable.
2. Unfreeze and repair the live loop (D1, D2, D3, D4).
3. Fix the UI structure (D5, D6).
4. Wire dormant systems in journey order.
5. Art and atmosphere.

## After the revival pass (same day)

Phases 0–4 of the revival plan landed in-tree. Gates gained layout / subject /
grounded laws; panels collapsed to `ActivePanel`; flora seats on terrain;
survival/craft, machines (P3D-306 flow), and social (combat/companion/oversight)
reach the live loop; art-pass shipped NPC GLBs with faction kits, retuned
detail atlas, live day/night (`sun_at_phase` + WGSL night dim), and promoted
beta-critical rows off placeholder with in-world proof scenes.

Evidence: `make p3d-daynight` → day mean 194 → night mean 100 (48% darker);
`apps/poorcraft3d/shots/windowed_day.png` + `windowed_night.png`. Shadows were
already PCF-live (NWR-006); day/night + hemisphere ambient close the atmosphere
gap the audit named. Journey scorecard above is the *baseline* measured before
the pass — re-score against a fresh play session when wiring the next loop.
