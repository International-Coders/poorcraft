# Release Readiness — Detail for Step 40

## Purpose

This is the final honesty check for this entire pack — the same
discipline Step 1 applied to the *existing* codebase, now applied to
everything built across Steps 2–39. The goal isn't a finished, bug-free
game (you've said explicitly you expect bugs and don't need a complete
project) — the goal is that whatever ships is **actually there when
tested**, matching what the docs claim, with no gap like the one that
prompted this whole pack.

## The full-loop playtest

Before rewriting STATUS.md/BACKLOG.md, personally play (or have z.ai run
and screenshot, but ideally a human plays this one) one continuous session
covering:

1. **New world creation** through the improved first-launch flow
   (Step 14).
2. **Core survival loop** with the Stage B destruction/rendering fixes
   visibly present (crack overlay, particles, sound, AO).
3. **At least one full power age** end-to-end — extract/generate fuel,
   power a machine, see output (Water Age at minimum; Steam/Oil/Nuclear if
   time allows).
4. **One magic interaction** — learn a spell from a wizard, cast it, see
   a real effect.
5. **One build project** using the new tools — place stairs/slabs, use
   the blueprint tool, place at least one decoration/statue.
6. **One multiplayer session joined via Steam lobby** (Step 34–36), with
   a second player/account, exchanging chat and seeing each other's block
   edits sync.
7. **One lore touchpoint** — read a lore book, open the live chronicle
   screen, see a consistent named reference across two systems (Step 22's
   test).

## Output

Rewrite `STATUS.md` and `BACKLOG.md` to reflect exactly what was
personally verified in that session — nothing marked done that wasn't
just witnessed working. Where something built in Steps 2–39 doesn't fully
work yet, mark it deferred with the same honest, specific style the
project already uses ("A* pathfinding (mobs hop and beeline today)") —
a specific, truthful gap note, not a vague caveat and not silence.

## Why this step exists at all

Everything in this pack was written because a previous round of "done"
claims didn't hold up under play. Step 40 is the mechanism that keeps that
from happening again at the end of *this* round — it's not optional
polish, it's the check that makes the other 39 steps trustworthy.
