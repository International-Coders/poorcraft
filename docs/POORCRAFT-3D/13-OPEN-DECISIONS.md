# Open Decisions Before Irreversible Work

These are deliberately left open. They should be answered before committing
to a save schema, network protocol, or expensive terrain representation.

## World scale

- What hardware and resolution define the minimum supported target?
- How large should a typical world be before streaming becomes the primary
  constraint?
- How far apart should major capitals be relative to ordinary travel time?

## Terrain detail

- Is the first terrain prototype based on a density field, adaptive octree,
  dual contouring, marching cubes, or another meshing method?
- Which materials remain faceted/block-readable rather than smooth?
- How much underground editing is required in the first playable slice?

## Simulation

- What are the fixed tick rate and per-system budgets?
- Which environmental systems need persistent state versus deterministic
  regeneration?
- Which machine capabilities are typed separately from generic power?

## Progression and empire

- Is empire management always available, or unlocked after a personal arc?
- Can a solo player directly command armies, appoint NPC leaders, or both?
- What counts as a meaningful beta end state: a capital, a realm, or a
  multi-realm empire?

## Multiplayer

- What is the expected player count for the first beta?
- Is host migration required, or is reconnect to a persistent host sufficient?
- Which Steam features are in the first public test and which require external
  account/store setup?

## Decision rule

Choose the smallest design that preserves the product promise, can be tested
deterministically, and does not prevent later expansion. Record accepted
answers beside the implementation task that makes them real.

## Owner interview reference

The full question set is in `17-OWNER-VISION-QUESTIONNAIRE.md`. This file is a
technical reminder list; the questionnaire is designed for plain-language
answers about the desired player experience. Do not treat an unanswered item
as permission to make an irreversible design choice.
