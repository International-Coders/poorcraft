---
name: dev-loop
description: Run the LOREFORGE (POORCRAFT) self-driving development loop — orient from STATE.md / BACKLOG.md / CHANGELOG.md / DEVLOG.md, autonomously pick, plan and implement the next feature, keep cargo test --workspace + vistest + smoke green, update the bookkeeping docs, ship runtimes, commit and push, then loop straight into the next job. Use whenever the user wants to continue or keep building on this project, run a dev loop, plan and upgrade the game, work autonomously on new features, asks "what should we build next (and build it)", or says things like "keep going" / "next loop" — even without the word "loop". Also use to just plan the next job when the user asks what's next.
---

# LOREFORGE dev-loop

You are running the project's self-driving development loop. Each pass ships
exactly one **job**: a coherent, tested, verifiable feature or fix that leaves
`main` green. `AGENTS.md` is law; this skill is the engine that keeps the
project moving without re-deriving the workflow every session.

One pass = **Orient → Pick → Plan → Implement → Verify → Bookkeep → Ship**.
Run passes back-to-back until a stopping condition fires (see *Loop control*).
Between passes the repo is always green and pushed, so an interrupt loses
nothing.

## 1. Orient (read before anything else)

Never start from memory — the docs are the state machine:

1. `STATE.md` — `loop_count`, `current_milestone`, `last_done`, `next_task`,
   `blockers`. `next_task` is the default queue.
2. Tail of `CHANGELOG.md` and the newest `DEVLOG.md` entry — what just
   shipped and what was **HONESTLY DEFERRED** (deferrals are pre-validated
   work, already scoped by the previous loop).
3. `BACKLOG.md` — unchecked items; `AUDIT.md` — open items (e.g. dead data
   that needs spawn-or-cut).
4. `git log --oneline -8` and `git status` — confirm the tree matches the
   docs. If STATE.md is dirty from a prior loop, read it before overwriting.

If `blockers` is non-none, resolve or clear it first.

## 2. Pick the next job

Priority order — the first source that yields a task wins:

0. **The user's explicit ask**, if this pass was started with a named
   feature. It wins over everything below.
1. `STATE.md` → `next_task`, unless it says `NONE`.
2. The newest **HONESTLY DEFERRED** notes (DEVLOG tail / BACKLOG deferred
   section) — e.g. Creative-mode behavior, per-biome fog curves, faction
   recipe gates.
3. Open `AUDIT.md` items.
4. Derive: find the gap a player would hit next. Read `docs/`,
   `mods/README.md`, `lore/`, click through `crates/lf_client/src/ui.rs`
   screens, and ask "what is stubbed, unreadable, or missing after the last
   milestone?" Prefer depth over novelty — finish systems, don't stack
   orphans.

**Scope rule:** one job = one shippable unit (a pack of tightly related small
items is fine — the ui-world-craft pack is the model). If it cannot land,
verify, and ship in one pass, split it: ship part 1, and write the remainder
into `STATE.md` → `next_task` with enough detail that a fresh session can
pick it up cold.

## 3. Plan (short, concrete, then go)

Before writing code, state in a few lines: the goal; which crates/files
change (respect the **Layout** and **Layer rules** in AGENTS.md — lf_engine
never depends on gameplay crates); which tests get added or rewritten; which
vistest scene(s) prove it visually; which docs get updated. Blocks/items flow
one way: `lf_voxel/src/registry.rs` → `lf_assets` (atlas +
`texture_index_for_block`) → `lf_game/src/items.rs`; the catalog consistency
test must stay green. Do not ask permission for the plan — the loop is
authorized; just build.

## 4. Implement

- Read a neighboring module first and match its idioms — this codebase has
  strong conventions (e.g. BFS light, y-stride columns, ui_kit Reveal
  animations).
- Write tests alongside the code, not after. Every behavior claim gets a
  test; every visual claim gets a vistest scene with a pixel assertion.
- Run `cargo test --workspace` after each coherent unit, not only at the
  end — fail fast, fix before moving on.
- Gotchas that will bite (from AGENTS.md): wgpu 24 / winit 0.30 / egui 0.31
  are version-locked; the egui pass must be encoded before texture readback
  or UI vanishes from screenshots; if an edit seems ignored, stale
  fingerprint — `rm -rf target/release/.fingerprint/lf_vistest*`; tests never
  touch `worlds/` — use `tempfile`.

## 5. Verify (constantly, in this ladder)

Never trust "it rendered" or "it compiles" — every claim comes from a command
run in this session:

1. `cargo build --workspace` — clean.
2. `cargo test --workspace` — all green; report the count.
3. Visual work: `cargo run --release -p xtask -- vistest shots` — every
   scene's pixel claim passes; then actually look at the new/changed PNGs
   (human-eye pass — two river bugs were caught by looking after the pixel
   assertions were green).
4. Binary: `make smoke` (launch ~12s, process alive, pkill).
5. Perf-sensitive work: `make perf` and compare against the previous p50/p95.

**Bugs found by proofs are fixed before committing.** That rule killed the
face-winding see-through terrain and the dead-code torch placements; it is
non-negotiable.

## 6. Bookkeep (only after real, verified work)

- `STATE.md`: `loop_count` + 1, new `current_milestone`, `last_done` (one
  dense honest paragraph), `next_task` (the next real task, or NONE with
  pointers to deferred notes), `build: GREEN`, fresh test count.
- `CHANGELOG.md`: entry at top.
- `BACKLOG.md`: tick what shipped, add new deferrals as unchecked items.
- `DEVLOG.md`: one dated entry, newest last — WHAT, HOW (files touched,
  approach, commands), VERIFICATION (test counts, vistest score, smoke,
  artifact paths), and **HONESTLY DEFERRED**.
- `Makefile`: keep in sync whenever commands/targets changed.

## 7. Ship

1. `make runtimes` (macOS .app + .dmg, Linux tarball, Windows exe when mingw
   exists) — then `ls -la dist/` and verify the artifacts are on disk. If a
   target genuinely cannot build here, say which and why; never claim an
   artifact that isn't there.
2. Commit — message names the loop and the concrete shipped features, like
   the existing history. No docs-only progress claims: code ships every job.
3. `git push github HEAD` (or `make push`). If authentication fails, say so
   explicitly in the report — never claim a push that didn't happen.
4. Final report: what shipped, evidence (tests / vistest / smoke), artifact
   paths, what was deferred, what `next_task` now points at.

If the job changed no game code (pure tooling), skip `make runtimes` (the
dist/ binaries are unchanged) but still log + commit + push honestly.

## Loop control

- If the user asked for N loops or named a goal, stop when it's met.
- Otherwise keep passing while real tasks remain and the context/time budget
  allows — but only ever stop **between** passes, at a green, pushed,
  bookkept checkpoint.
- Running low on context mid-pass? Finish the current job or shrink it to
  what's shippable, then set `STATE.md` → `next_task` with precise notes and
  stop cleanly. Never abandon a half-done feature on main.
- Never: leave tests red, force-push, delete or overwrite `worlds/`, claim
  unverified pixels, or silently drop a deferral.
