# LOREFORGE ZCode Idle Upgrade Pack

This folder is the durable operating contract for ZCode/GLM idle-time work on
LOREFORGE and its active POORCRAFT 3D rebuild. It replaces one-off “make the
whole game better” prompts with a repeatable loop: every idle invocation reads
the current repository, ships one coherent improvement, proves it, records the
next valuable job, and leaves the project at a green checkpoint.

The project has no terminal design point. An idle run may finish; the game is
never declared permanently finished. When the known queue is empty, the agent
must inspect the playable product, measured performance, portability, system
depth, dependency health, and lore presentation to discover the next evidence-
backed improvement. It must not manufacture churn merely to stay busy.

## What to paste into ZCode

Paste the complete text inside
[`ZCODE-IDLE-TASK-PROMPT.md`](ZCODE-IDLE-TASK-PROMPT.md) into the ZCode
**Idle-time task** instructions field.

Recommended task configuration:

- Task title: `LOREFORGE Perpetual Upgrade Loop`
- Project: `/Users/zari/Desktop/POORCRAFT`
- Permission: a mode that permits repository edits, builds, commits, and the
  project-authorized GitHub push; keep destructive or unrelated access gated.
- Model: GLM-5.3, or the strongest coding model available in that ZCode setup.
- Thought level: high for architecture, persistence, simulation, rendering,
  networking, or dependency migrations; normal is sufficient for bounded
  mechanical follow-ups.

ZCode documents idle-time tasks as unscheduled work queued for spare capacity.
The exact idle window is controlled by ZCode. Keep the computer awake and the
application available according to the current ZCode settings. Requeue or keep
the task enabled according to the ZCode version in use; this repository prompt
is intentionally safe to invoke repeatedly.

## Files in this pack

- [`ZCODE-IDLE-TASK-PROMPT.md`](ZCODE-IDLE-TASK-PROMPT.md) — the paste-ready
  autonomous development instruction.
- [`LOREFORGE-CANON-BIBLE.md`](LOREFORGE-CANON-BIBLE.md) — the consolidated
  world, faction, character, visual, and narrative canon. This is binding for
  lore-facing work.
- [`UPGRADE-DECISION-FRAMEWORK.md`](UPGRADE-DECISION-FRAMEWORK.md) — how to
  choose worthwhile engine, Rust library, performance, and portability jobs
  without dependency churn or speculative rewrites.
- [`autonomous_upgrade_contract.json`](autonomous_upgrade_contract.json) — a
  machine-readable summary of the loop, canon locks, proof gates, and forbidden
  behavior.

## Validation

Run this before enabling the idle task and whenever this pack changes:

```bash
make idle-upgrade-check
```

The validator checks that every required document exists, local links resolve,
the machine contract parses, the six canonical factions and four-era history
remain named, and the prompt still contains the required proof, lore,
performance, portability, commit, and handoff rules.

## Authority and precedence

When sources conflict, use this order:

1. `AGENTS.md` safety, workflow, layer, verification, runtime, and shipping law.
2. Current source code, tests, generated evidence, and actual runtime behavior.
3. `STATE.md`, `BACKLOG.md`, `CHANGELOG.md`, `DEVLOG.md`, and `AUDIT.md` for the
   live queue and honest status.
4. `LOREFORGE-CANON-BIBLE.md` for canonical identity and lore.
5. Accepted decisions in `docs/POORCRAFT-3D/18-DECISION-REGISTER.md` and the
   design constitution.
6. Detailed subsystem packs and implementation contracts.
7. Old plans, prompts, and speculative notes.

Code may prove that a planned mechanic is not implemented. It may not silently
rewrite an established canonical fact. A deliberate canon change requires an
explicit owner decision, a recorded migration, updated data and dialogue, and
tests that prove the change is internally consistent.

