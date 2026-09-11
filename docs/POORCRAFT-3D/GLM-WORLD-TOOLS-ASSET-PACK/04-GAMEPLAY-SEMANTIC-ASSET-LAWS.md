# Gameplay Semantic Asset Laws

These laws turn art into gameplay.

## House Law

A house asset must have:

- door socket;
- interior or transition behavior;
- collision that leaves the doorway open;
- floor/entry nav point;
- light/shelter metadata;
- ownership/settlement tag;
- screenshot from outside and inside/threshold.

## NPC Law

An NPC asset must have:

- name;
- role;
- talk prompt;
- dialogue state;
- schedule;
- faction/settlement relation;
- inspect dump;
- screenshot close-up and in-world proof.

## Forge Law

A forge asset must have:

- heat state;
- input slot;
- output slot;
- recipe or smithing action;
- animation/spark placeholder;
- sound placeholder;
- UI prompt;
- test proving one valid forge action.

## Door Law

A door must open, close, block when closed, pass when open, and serialize state.

## Chest Law

A chest must open, store items, persist contents, and spill or lock according to
design.

## Workstation Law

Every workstation must have:

- physical model;
- use prompt;
- UI screen or action;
- recipe list or behavior;
- blocked-state message;
- proof command.

## Asset Refusal Rule

If an asset lacks its semantic law, GLM must mark it `visual_only` and must not
claim it is a gameplay asset.
