# Construction & Architecture — Detail for Steps 31–32

## Purpose

The "very tall buildings, castles full of statues, a modern building with
elevators and AC and computers" ask, made concrete. Building is core, not
a side effect of having blocks — this stage gives the player real tools
for it and a payoff layer (smart-building tech) tied to the power grid
built in Stage F.

## Step 31 — Building tools
- **Stairs/slabs/slopes**: a real partial-block shape system, needed for
  anything beyond flat-topped cube builds — towers, modern buildings, and
  castles all need this.
- **Symmetry/mirror placement**: place one side of a structure, mirror it
  automatically — high value for towers and castles specifically.
- **Blueprint/schematic tool**: capture a built structure, place a
  ghost-preview copy elsewhere. Big value for repeating floor layouts in
  a tall building, and for sharing designs on a multiplayer server (ties
  to Stage J/K — a blueprint could plausibly become a Workshop-shareable
  item later, flag this for a future `DECISIONS.md` entry rather than
  building it now).
- **Scaffolding block**: temporary, climbable, easy to bulk-remove —
  needed for building tall safely.

## Step 32 — Decoration and smart-building tech
- **Decoration block registry**: statues, banners, rugs, furniture-scale
  objects, colored/stained glass — built as a data-driven registry the
  same way mod blocks already work, so mod authors (Stage K) can extend
  it without engine changes.
- **Statue-carving interaction**: a dedicated mini-interaction (can reuse
  the smithing-minigame *pattern* — a focused interaction, not just
  "place item from inventory") so building a statue feels like a craft.
- **Elevator block**: vertical transport between floors, drawing power
  from the grid built in Stage F — an Electric-tier-or-later reward that's
  about living somewhere nice, not just producing more.
- **Climate/AC block**: a powered comfort block. Start cosmetic-plus-
  minor-perk (e.g., a small regen or comfort-adjacent bonus indoors)
  rather than a full temperature simulation — a full survival-temperature
  system is explicitly out of scope unless a future `DECISIONS.md` entry
  changes that.
- **Computer/screen block**: an interactive, in-world display block
  showing real live data — tech-tree progress, chronicle log entries
  (Step 21), or the power-grid overlay (Step 25) — rendered on an
  in-world screen instead of only in a menu. This is a genuine showcase
  of "craft-first" done well: a UI panel that's also a placeable object.
  Must show *real* game state, not placeholder text — that's the concrete
  Done check.
- **Visible wiring/conduit blocks**: power/fluid conduits should be
  something a player visibly runs through walls and ceilings on purpose,
  not an invisible network.

## Guardrail
Smart-building tech should read as "a wizard-tower-and-castle world where
one wing got wired for electricity," not "suddenly a different, modern-day
game." Keep material language (stone, timber, copper, iron) consistent
even on "modern" blocks.
