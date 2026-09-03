# GLM 5.3 / GLM 5.3 Flash Execution Guide

## Purpose

Use the available models as disciplined implementation workers, not as a
reason to expand job scope. The repository, tests, and rendered evidence remain
the source of truth.

## Model selection

Use GLM 5.3 for work with architectural or persistence risk:

- simulation-host extraction and authority migration;
- deterministic fluid transfer and cross-chunk boundaries;
- castle planning algorithms and format design;
- hierarchical pathfinding, knowledge, policy, and save migrations;
- protocol reliability, transactions, reconnect, and Steam integration;
- diagnosis after a proof contradicts the implementation.

Use GLM 5.3 Flash for bounded, well-specified work:

- repository inventory and call-site mapping;
- implementing isolated validators, manifest rows, adapters, and UI plumbing;
- generating repetitive tests from an already-approved contract;
- running verification, inspecting logs, and preparing concise failure notes;
- mechanical documentation updates after real code ships.

Do not let Flash choose architecture merely because it is faster. Do not spend
the full model on broad rereads that a targeted inventory can answer.

## Token-conscious reading protocol

The first architecture session should read:

1. `AGENTS.md`, `STATE.md`, and this entire folder;
2. the relevant current source and tests;
3. the matching `docs/NIGHTLY-BETA/` subsystem file;
4. only the relevant sections of BACKLOG/CHANGELOG/DEVLOG by heading/search;
5. historical packs only when the document audit points to a useful detail.

Subsequent jobs should not reread all 16,856 historical Markdown lines. Read
the authority chain, the newest history entry, the active job contract, and
neighboring source. Use `rg` to locate prior decisions and contradictions.

Suggested context allocation:

- 10% orientation and dirty-tree audit;
- 10% concrete plan and risk identification;
- 45% implementation and local iteration;
- 25% tests, visual proof, runtime, and failure repair;
- 10% diff review, bookkeeping, commit, push, and handoff.

If context becomes tight, shrink the current job at a coherent boundary. Never
weaken tests, skip persistence, or combine jobs to make the queue advance.

## Paste-ready goal prompt

```text
Take LOREFORGE from its current alpha toward the beta defined in
docs/BETA-FOUNDATION/. Work exactly one job at a time from
docs/BETA-FOUNDATION/08-BETA-DELIVERY-ROADMAP.md, using STATE.md as the exact
current pointer.

Before changing anything, read AGENTS.md, STATE.md, every file in
docs/BETA-FOUNDATION/, the relevant docs/NIGHTLY-BETA subsystem specification,
and the current source/tests for the job. Inspect git status and preserve all
unrelated work. Search older histories by relevant heading instead of spending
context rereading every old log.

Preserve the working renderer, voxel storage, world identity, saves, content
pipeline, and proofs. Repair by extracting deterministic services into one
authoritative simulation used by singleplayer, UDP, and Steam. Do not create a
parallel game implementation in the client. Do not broaden the job.

For each job, state a short plan, implement code and tests together, run the
proportional verification ladder, inspect every changed PNG, fix every proof-
discovered defect, update STATE/BACKLOG/CHANGELOG/DEVLOG and Makefile if
commands changed, build fresh runtimes for completed game-code work, commit
only the job, and push github HEAD. Never make a docs-only progress commit.

Core outcomes are binding: conserved directional water that creates measured
wheel torque; major capitals that are sparse, multi-chunk, terrain-integrated,
faction-distinct, and asset-manifest-driven; NPCs that navigate, perceive,
remember, report, work, and react inside castles; useful living companions;
and one authoritative multiplayer world with real Steam lobby/invite flow.

Use only original LOREFORGE expression. Skyrim, Heroes of Might and Magic III,
and Minecraft are quality references, never sources to copy.

Stop only between green pushed jobs. Report the first failing beta gate
honestly; do not call the game beta because individual systems or screenshots
exist.
```

## Per-job prompt template

```text
Execute job <ID and title> from
docs/BETA-FOUNDATION/08-BETA-DELIVERY-ROADMAP.md.

Goal:
Current source truth:
Files/crates likely involved:
Behavioral invariants:
Persistence/network compatibility:
Required tests:
Required vistest scenes and semantic questions:
Performance budget/counters:
Explicit non-goals:

Follow AGENTS.md and the Beta Foundation authority chain. Preserve unrelated
changes. If source contradicts this prompt, report the contradiction and use
the higher-authority contract; do not guess silently.
```

## Review prompt for a second session

```text
Review the current job as an adversarial beta gate. Do not implement new
features. Read the job contract, diff, tests, and changed rendered proofs.
Find behavior that is client-only, nondeterministic, non-persistent,
unbounded, unreachable, visually contradicted, or falsely claimed. For every
finding, provide file/line evidence, a reproduction, severity, and the smallest
test that would catch it. If no actionable finding remains, say which gates you
actually verified and which require external/human evidence.
```

## Usage discipline

- Prefer one full-model architecture/implementation pass plus a Flash
  verification pass over several overlapping broad prompts.
- Give each model one bounded job and explicit non-goals.
- Never ask a model to "make the whole game beta" in one diff.
- Do not paste generated summaries back into the repository unless they change
  a decision or acceptance contract.
- Logs should record commands and evidence, not model chain-of-thought.
- Generated assets require manifest rows, provenance, consumers, and rendered
  review regardless of which model produced them.
