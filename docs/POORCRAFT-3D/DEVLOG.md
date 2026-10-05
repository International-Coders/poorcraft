# POORCRAFT 3D Development Log

This is the index and contract for the current game's development history.
Entries are stored by month so the log can grow without turning one file into
an unreadable multi-megabyte document.

## Monthly logs

- `devlog/2026-10.md` — playability-reset audit, packaged-build playtest, and
  workspace consolidation

## Entry contract

Append one entry per completed work card, newest last:

```text
## YYYY-MM-DD — CARD-ID: outcome

Problem:
Change:
Files:
Verification:
Runtime evidence:
Performance:
Known debt:
Next card:
Commit/package:
```

Rules:

1. State what a player can observe before listing implementation detail.
2. Copy command results and counts from the current checkout; do not carry a
   previous entry's green claim forward.
3. Link or name the evidence files that were inspected.
4. Record failures and deferred work plainly.
5. Name exactly one next card so another session can resume without guessing.
6. Start a new `devlog/YYYY-MM.md` file each month and add it to this index.

Root `DEVLOG.md` remains the historical LOREFORGE log. POORCRAFT 3D work is
recorded here.
