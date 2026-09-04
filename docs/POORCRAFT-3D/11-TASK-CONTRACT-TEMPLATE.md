# POORCRAFT 3D Task Contract

Every implementation task should begin with a filled copy of this contract.
It prevents z.ai sessions from broadening scope or treating prose as proof.

## Identity

- Task ID:
- Title:
- Stage/gate:
- Owner/model:
- Date:

## Goal

State one observable capability in one or two sentences.

## Current truth

List the relevant source, tests, assets, and known limitations. Include file
paths and revision/schema versions where applicable.

## Scope

### Must change

-

### Explicit non-goals

-

## Contracts and invariants

- deterministic behavior:
- authority/ownership:
- save/load behavior:
- network behavior:
- performance budget:
- player-visible result:

## Verification

- unit/integration tests:
- persistence/replay tests:
- visual proof scene and semantic assertion:
- smoke or multiplayer test:
- profiling/counter evidence:

## Done when

The task is complete only when the implementation, tests, runtime evidence,
and documentation agree. A screenshot alone is not proof. A passing compile
alone is not proof. Any discovered contradiction becomes a fix or an explicit
deferral before the task closes.

## Handoff

- files changed:
- commands run:
- evidence paths:
- known limitations:
- next task:

## Required design check

Before a task starts, write one sentence for each item:

- Which player story does this improve?
- Which two game layers does it connect?
- What must a player be able to see or do when it works?
- What user-approved decision does it depend on?

If the task cannot answer these, it is probably implementation activity without
a clear product purpose. Return it to the idea list or clarify it with the
owner first.
