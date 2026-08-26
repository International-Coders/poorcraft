# Lore & Narrative Surfacing — Detail for Steps 20–22

## The actual problem to solve

The game already has quest objectives, villager schedules/trading, named
mobs (Geode Guardian, Cinder Crawler, Null Knight), and a chronicle system
that writes saga events. That's real lore *infrastructure*. What's missing
is lore *surfacing* — a player experiencing that content as a story while
they play, not just triggering data that gets written to a file they may
never open. This is exactly the gap you're describing when you say "the
lore is not implemented."

## Step 20 — Lore books, for real

- BACKLOG.md already honestly lists "lore books readable in-game" as
  deferred. This step finishes it: a real interaction (right-click/use a
  placed or held book item) opens a paginated in-game reading UI, built
  with the Step 12 design system, showing real text loaded from actual
  content files (not a placeholder string).
- Content: write a real, if modest, set of lore book texts tied to the
  world's actual named entities (the Geode Guardian, the Null Knight,
  village history) rather than generic flavor text — this is what makes
  Step 22's cross-referencing possible.

## Step 21 — A live, in-game chronicle screen

- Currently the chronicle exports to `worlds/<name>/chronicle.md` on
  save — useful as a record, invisible during play.
- Add an in-game journal/chronicle screen (a keybind, e.g. matching the
  existing quest-log `J` pattern) that shows the same milestone entries
  live, as they happen, in the same reading-friendly format as the
  exported markdown but rendered in-engine.
- This turns "you did something significant" into an immediate, visible
  payoff instead of a file the player has to go find after the fact.

## Step 22 — One consistent world story across every system

- Pick (or confirm, if some already exist) a small set of named lore
  anchors — a founding event, a named ruin, a recurring antagonist faction
  or figure — and make sure they show up consistently across:
  - Lore book text (Step 20)
  - Villager dialogue/trading flavor text
  - Chronicle milestone templates (Step 21)
  - Quest text for the existing 5-quest starter chain (or its successor)
- The test for this step is concrete: pick 3 named lore entities/events
  and confirm each appears in at least 2 of the 4 systems above. This is
  what makes the world feel like it has one story instead of four
  disconnected flavor-text generators.

## Why this is its own stage, separate from magic/wizards (Stage G)

Lore surfacing is about making *existing* systems (chronicle, quests,
villagers) visible and connected. Stage G's wizards/dragons are *new*
content that will also need lore hooks once it exists — but fixing the
surfacing of what's already built comes first, so new magic-era lore has
a working system to plug into rather than repeating the same "written but
never shown to the player" mistake.
