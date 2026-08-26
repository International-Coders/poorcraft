# Reality Audit — How to Verify "Done" Actually Means Done

## Why this file exists

You said it directly: block destruction feedback, lore, and biomes have
all been marked as implemented in this project's own docs at various
points, but don't feel implemented when you actually play. That gap is
the single most important thing to close before anything else gets built,
because every new step in `MASTER_PLAN.md` assumes the ground it's
building on is real. This file is the method for Step 1.

## Method

For every `[x]` checked item currently in `BACKLOG.md`, do all three of
the following before accepting it as real:

1. **Read the code.** Find the actual implementation (not a stub, not a
   data type with no consumer). If a feature's "implementation" is a
   struct that's never rendered or never wired to input, it's not done.
2. **Run it.** Launch the actual game (or the relevant vistest scene) and
   trigger the feature yourself. A passing unit test on internal logic
   does not substitute for the feature being visible/usable in play — a
   block-breaking hardness calculation can have a green test and still
   produce zero visual feedback when you actually mine a block.
3. **Screenshot it.** For anything visual, capture a fresh vistest PNG (or
   an in-game F2 screenshot) showing the feature happening, and pixel-
   check it's not a uniform/blank result the way the P25 audit's own
   pixel-analysis check catches "it rendered one flat color."

Only mark something CONFIRMED if all three pass.

## Specific items to check first (the three you flagged directly)

### Block destruction feedback
- Does breaking a block show a cracking overlay that progresses with
  mining time, or does the block just vanish at 100%?
- Does breaking a block spawn any particle effect at all?
- Is there a break sound?
- If any of these three is missing, this is ACTUALLY-MISSING regardless
  of what BACKLOG.md says about hold-to-mine/hardness/durability being
  done — those are the *simulation*, not the *feedback*, and both need to
  exist.

### Lore
- Can a player actually open and read a lore book in-game right now, or
  is this still the explicitly-deferred stub BACKLOG.md already admits to
  ("lore books readable in-game (deferred)")?
- Does the chronicle system produce anything the player sees *during
  play*, or only a markdown export on save that nobody in-game ever
  reads?
- Do villagers, quests, and structures reference a shared, consistent set
  of named lore (people, places, events) or is each system's "lore" text
  generic and disconnected?

### Biomes
- Take a screenshot of 5+ biomes side by side. Do they look meaningfully
  different (color palette, ground cover, unique features), or do they
  read as "the same terrain shape with a different noise seed"?
- Does each biome have anything placed in it that isn't in every other
  biome?
- Do structures and mobs actually respect biome boundaries, or can a
  desert structure generate in a snow biome?

## Output format

Create `AUDIT.md` at the repo root with one row per previously-checked
BACKLOG item:

```
| Item | Previous status | Audit result | Repro / evidence |
|---|---|---|---|
| Block breaking feedback | [x] "hold-to-mine with hardness..." | ACTUALLY-MISSING | No particle/crack overlay on break; see screenshot attempt in shots/audit_break.png (blank/no overlay) |
| 30-biome world | [x] "all 8 biomes reachable" (note: BACKLOG says 8 here, STATUS says 30 — reconcile which is true) | ... | ... |
```

Note the discrepancy already visible between BACKLOG.md (which says "all 8
biomes reachable" in one line) and STATUS.md (which says "30-biome
world") — resolve which number is actually true as part of this audit;
don't let two of the project's own docs disagree without resolving it.

## After the audit

- Update BACKLOG.md in the same commit: uncheck anything reclassified as
  ACTUALLY-MISSING, and add an honest note the same way existing deferred
  items are noted ("deferred, honestly").
- Every ACTUALLY-MISSING or ACTUALLY-BROKEN row becomes a real fix task in
  Step 2, before any Stage C+ content work begins.
- Keep `AUDIT.md` in the repo permanently as a record — don't delete it
  once fixed. It's useful evidence the next time a "done" claim needs
  checking.
