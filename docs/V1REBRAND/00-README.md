# Game Design Roadmap — Index

This folder is the **future-facing design spec** for the project currently
named "poorcraft" (codename LOREFORGE internally, per AGENTS.md). It sits
alongside, and does not replace, the existing operational docs:

- `STATE.md` — current loop/milestone/test count (unchanged process)
- `BACKLOG.md` — done vs. deferred, phase-numbered P1–P25 (unchanged process)
- `CHANGELOG.md` — per-loop history (unchanged process)
- `AGENTS.md` — ground rules for the AI coding sessions (unchanged process)
- `DECISIONS.md` — locked-in technical decisions (unchanged process)

**This folder answers a different question than those files.** They answer
"what exists and what's next in the queue." This folder answers "what is
this game actually trying to be, all the way through, so every future queue
item points the same direction."

## How to use this with z.ai / your AI coding sessions

1. Read `01-VISION-AND-PILLARS.md` first, every session, the way AGENTS.md
   is read first. It is the thing that should never silently drift.
2. Treat files `02` through `08` as **design law** for their subsystem —
   when BACKLOG.md gets a new phase entry for power, magic, building, or
   progression, it must be checked against the matching file here before
   it's marked done.
3. `09-CONTENT-AND-PHASE-ROADMAP.md` is the bridge document: it takes
   everything in this folder and turns it into the next 10 phases
   (P26–P35), in the same checklist format as BACKLOG.md, so it can be
   copy-pasted into BACKLOG.md as work is picked up.
4. `10-STEAM-AND-RELEASE-PLAN.md` is the business/release layer — read it
   before touching STEAM.md, steam_appid.txt, or pricing decisions.

## Working name

The repo, binary, and internal docs currently say "poorcraft" / "LOREFORGE."
Nothing in this folder assumes a final name — every doc below refers to
"the game." When the name changes, only `DECISIONS.md` and branding assets
need to change; none of these design docs reference the old name.

## Non-negotiables carried over from AGENTS.md

- No docs-only commits for actual feature work — these files describe
  *what to build*, not a substitute for building it.
- Every phase below is expected to ship code, tests, and a vistest proof,
  exactly like P1–P25.
- Honesty discipline applies here too: if a phase in `09` turns out to be
  too big or wrong once you're in the code, say so in BACKLOG.md the same
  way P6's A* pathfinding is honestly marked deferred — don't quietly claim
  it's done.

## File list

| File | Covers |
|---|---|
| `01-VISION-AND-PILLARS.md` | What the game is, who it's for, tone, scope guardrails |
| `02-RENDERING-AND-UX-STABILIZATION.md` | Fix-first pass: graphics, camera, menus, UI — before new content |
| `03-TECH-PROGRESSION-OVERVIEW.md` | The age-by-age arc, Stone Age → capped post-industrial |
| `04-POWER-AND-AUTOMATION.md` | Water wheels, steam, oil, electric grid, nuclear cap-tier |
| `05-MAGIC-LORE-AND-CREATURES.md` | Wizards, spells, dragons, lore layer, how it meets tech |
| `06-CONSTRUCTION-AND-ARCHITECTURE.md` | Building tools, tall structures, statues, "smart building" tech |
| `07-SPECIALIZATION-AND-PROGRESSION-PATHS.md` | The path/specialization system, SP vs MP framing |
| `08-MULTIPLAYER-ECONOMY-AND-SERVERS.md` | How specialization plays out with other people |
| `09-CONTENT-AND-PHASE-ROADMAP.md` | Turns 01–08 into a concrete phase queue (P26+) |
| `10-STEAM-AND-RELEASE-PLAN.md` | Pricing, friends-test plan, store page checklist |
