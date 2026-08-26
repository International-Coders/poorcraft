# Index — poorcraft Vanilla-Complete Build Pack (v2)

## What changed from the first pack

The first roadmap (if you still have it) assumed the existing docs
(BACKLOG.md/STATUS.md) accurately described what's in the game. You've now
told me directly that things marked done in those docs — block breaking
feedback, lore, biomes — don't actually feel implemented when you play.
This pack starts from a different assumption: **the docs are not proof,
the running game is.** File `01` makes that explicit and is step 1 of the
master plan for a reason — nothing else should be trusted or built on top
of until it's re-verified against the actual build.

## The one file you asked for

**`MASTER_PLAN.md`** is the file to hand to your z.ai coding sessions
directly. It has 40 numbered, independently gradeable steps, in build
order, covering: re-auditing what's real, fixing block destruction/engine/
rendering/UI feel, making biomes and lore actually show up in play, then
building out the full vanilla-to-industrial-plus-magic content set, then
Steam lobbies/matchmaking and Steam Workshop mod support. Each step names
its target crate(s), what to build, and a concrete "how do we know it's
done" check — no step is "make it better," every step is checkable.

Feed z.ai one step at a time, in order (dependencies are called out where
they matter), the same way your project already works through BACKLOG.md
phases. When a step is done, the acceptance check should pass a real
`cargo test --workspace` run and a real `vistest` screenshot where visual,
per your own AGENTS.md discipline — not a markdown claim.

## Supporting reference docs

`MASTER_PLAN.md` keeps each step short. These files hold the actual design
detail a step references, so the AI coder isn't guessing at what "make
biomes feel real" means:

| File | Backs which steps |
|---|---|
| `01-REALITY-AUDIT.md` | Step 1 — how to verify claimed-done features |
| `02-DESTRUCTION-ENGINE-RENDERING.md` | Steps 2–11 — block breaking feel, renderer fixes |
| `03-UI-UX-OVERHAUL.md` | Steps 12–15 — menus, HUD, settings |
| `04-BIOMES-AND-WORLD-IDENTITY.md` | Steps 16–19 — making 30 biomes actually read as different |
| `05-LORE-AND-NARRATIVE-SURFACING.md` | Steps 20–22 — lore in front of the player, not just in save files |
| `06-POWER-AND-AUTOMATION.md` | Steps 23–27 — water/steam/oil/electric/nuclear |
| `07-MAGIC-LORE-AND-CREATURES.md` | Steps 28–30 — wizards, spells, dragons |
| `08-CONSTRUCTION-AND-ARCHITECTURE.md` | Steps 31–32 — building tools, statues, smart-building tech |
| `09-SPECIALIZATION-PATHS.md` | Step 33 — the four-path mastery system |
| `10-STEAM-MULTIPLAYER-INTEGRATION.md` | Steps 34–36 — Steamworks lobbies/matchmaking/join |
| `11-STEAM-WORKSHOP-AND-MODDING.md` | Steps 37–39 — Workshop uploads, mod-dev ergonomics |
| `12-RELEASE-READINESS.md` | Step 40 — final pass before handing to friends/Steam |

## Ground rule carried over from your AGENTS.md

No docs-only commits. No claiming a step is done without a passing test
and, for anything visual, a fresh vistest PNG that's been pixel-checked,
not eyeballed. This pack exists to stop the gap between "the markdown says
done" and "the game feels done" — the acceptance checks in `MASTER_PLAN.md`
are written to be impossible to fake with docs alone.
