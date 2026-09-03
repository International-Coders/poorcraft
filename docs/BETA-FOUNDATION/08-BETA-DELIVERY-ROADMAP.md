# Beta Delivery Roadmap

## Operating rule

One job is one green, reviewable commit with code, tests, visual proof when
visible, bookkeeping, fresh runtimes when game code changes, and push. Split a
job that cannot be completed and proved in one pass. This document defines
order and gates; `STATE.md` names the exact next job.

The existing N01–N07 work remains valid. The old N08–N24 queue is superseded
where it conflicts with the order below, especially for simulation authority,
castle distance, water force, six-realm beta scope, and multiplayer depth.

## Stage A — Freeze truth and create the simulation seam

### B01 — Runtime truth dashboard

Refresh the source-backed audit as machine-readable diagnostics: active
systems, schema versions, simulation ownership, scene/test counts, known client
authority, and current performance. Add no features. Done when a test prevents
the dashboard from claiming server authority for a client-only system.

### B02 — Fixed tick, command IDs, and domain events

Introduce deterministic tick/order primitives around existing behavior with
snapshot-hash tests. No visible redesign. Done when render cadence and command
batching do not alter a representative simulation result.

### B03 — Integrated singleplayer host

Run a local authoritative host behind the client and migrate block edits plus
inventory/crafting transactions through commands. Keep existing saves loading.
Done when direct client mutation is test-rejected for migrated systems and the
beta onboarding/craft journey still passes.

## Stage B — Water that performs work

### B04 — Sparse conserved fluid state

Add fixed-point volume, two-phase deterministic transfer, active regions,
legacy migration, mass/determinism tests, and debug counters. Preserve current
water visuals until the state is stable.

### B05 — Fluid rendering and current forces

Drive corner surface height and motion from volume/velocity; expose bounded
flow samples to players, mobs, NPCs, and props. Ship calm/rapid/waterfall and
current-push proofs.

### B06 — River boundaries, dams, gates, and flumes

Connect live regions to generated river direction/discharge. Add finite gates,
head building, release, and channel rules with unloaded-boundary tests.

### B07 — Mechanical wheel simulation

Replace `has_water` power with oriented flux/head/torque, inertia, load, stall,
and EU coupling. Ship powered/still/reverse/overload tests and proofs.

### B08 — Authoritative fluids and machines

Move fluid, wheel, pipe, steam, machine, power, and transaction state into the
host. Replicate baselines/deltas and prove two clients converge after packet
loss/reconnect.

## Stage C — Rare, authored realm seats

### B09 — Macro realm geography and spacing

Replace the 12×12-chunk kingdom lattice with deterministic macro-province
candidate selection. Calibrate capital/town/hamlet distances over 64+ seeds and
ship a map atlas showing sparse destinations.

### B10 — Structure planner and placement report

Create pure multi-chunk plans, module ports, roads, terrain adaptation,
protected edits, water impact, nav/activity anchors, and 128-site properties.
Do not add faction breadth yet.

### B11 — Asset manifest and structure compiler

Add `assets/manifest.toml`, validation, authored voxel-module compilation,
optional glTF metadata, fallbacks, provenance, and catalog closure. Import one
small diagnostic kit before a full castle.

### B12 — Accord capital vertical slice

Ship one complete civic capital: architecture, road, gate, interiors, ruler,
guards, work, market, beds, water, alarm anchors, state, proofs, and save/load.
This is the contract template, not a final universal footprint.

## Stage D — People who inhabit the place

### B13 — Hierarchical navigation service

Use castle portals/street graph plus bounded local A*, doors/gates,
reservations, crowd handling, failure recovery, persistence, and F3 budgets.

### B14 — NPC anchors, needs, and completed work

Replace shared-home/default-day shortcuts with per-role schedules, real
destinations, visible props/actions, and stock/resource effects. Prove a full
day in the Accord capital.

### B15 — Perception, moral events, and knowledge

Implement sight/sound/contact/evidence, immutable moral events, personal
knowledge, reporting, confidence/age, idempotence, and persistence.

### B16 — Castle alarm and faction policy

Connect crime/threat knowledge to guard investigation, gate state, civilian
shelter, surrender/escort, warrants, and reasoned public/personal disposition.

### B17 — Companion navigation and life

Ship formation follow, wait/guard, assist, role-valid work/haul, route failure
communication, unseen catch-up, relationship consequences, save/load, and
co-op authority.

## Stage E — Six realms, depth first

Each job reuses shared systems but must ship a different grammar, economy,
law, activity set, services, and recognition proof:

- B18 — Ironborn forge-fort.
- B19 — Ember Covenant living citadel.
- B20 — Free Holds hillfort.
- B21 — Ashen Order archive city.
- B22 — Nameless sunken refuge.

After B22, run a faction-difference audit. Rework any pair the reviewer cannot
distinguish without labels. Do not add new realms before it passes.

## Stage F — Multiplayer productization

### B23 — Full authoritative world migration

Move remaining player survival, combat, mobs, drops, NPCs, quests, reputation,
companions, and settlement state to the shared host. Remove parallel client
authority with parity tests.

### B24 — Versioned replication and reconnect

Add channels, sequence/ack/resync, baselines/deltas, interest management,
content handshake, duplicate suppression, and transaction recovery.

### B25 — Steam lobby and invite UI integration

Wire the existing Steamworks transport into the real host/client UI path;
handle initialization, overlay, invite, join, mismatch, loading, leave, and
direct-connect fallback honestly.

### B26 — Two-account Steam proof

Run the external matrix on two accounts/machines with the real mixed-system
journey. Fix discovered code defects. Record external prerequisites separately
from game defects.

## Stage G — Candidate hardening

### B27 — Asset and animation closure

Complete beta-critical castle kits, NPC roles, held props, water/machine
motion, LODs, fallbacks, provenance, and actual-scale recognition. No orphaned
generated files.

### B28 — Performance and soak

Profile fluid regions, castle population, navigation, server replication,
save/autosave, rendering, memory, and stalls. Enforce calibrated budgets and
run long travel/castle/dam/co-op soak tests.

### B29 — Deterministic beta journey

Automate the product journey from world creation through river power, castle
faction action, witnessed offense, companion travel, co-op join, save/reload,
and reconnect. Pair state assertions with representative visual scenes.

### B30 — Beta candidate release

Run full build/tests/vistest/image review/smoke/perf/runtimes, legacy save and
protocol migration, two-account checklist, known issues, accessibility and
first-time play. The first failing gate determines `ALPHA`, `PRE-BETA`, or
`BETA CANDIDATE` honestly.

## Milestone gates

| Gate | Minimum outcome |
|---|---|
| Simulation foundation | B01–B03: local host owns blocks and inventory |
| Physical-world slice | B04–B08: a real river powers one authoritative machine |
| Living-capital slice | B09–B17: one distant capital and its people work end to end |
| Realm beta breadth | B18–B22: six original realms pass difference tests |
| Co-op beta | B23–B26: same living world over UDP and real Steam session |
| Beta candidate | B27–B30: assets, performance, journey, release evidence green |

## Stop-the-line failures

Data loss, client/server divergence, fluid mass creation, nondeterministic
replay, castle overwrite of protected edits, inaccessible required anchors,
unbounded path/fluid work, remote NPC omniscience, duplicated transactions,
missing asset provenance, screenshot contradiction, or any red required check
stops the active job before more breadth is added.
