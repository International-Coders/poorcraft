# Visual Polish Principles — Read Before Everything Else

## What "looks AI generated" actually means

When you say the game looks AI generated, you're describing a real,
identifiable aesthetic failure. It's not about the code being AI-written —
it's about the *visual output* having these specific tells:

**The AI aesthetic tells:**
1. **Perfect symmetry.** Everything is centered, everything is equal
   width, every button is the same size. Real design has intentional
   asymmetry that serves a purpose.
2. **Generic defaults.** Round-rect buttons with a blue or grey fill.
   Pure white text. System default font. Nothing chosen, everything
   accepted.
3. **No wear, no history.** Perfect clean textures with no variation.
   Terrain that looks procedurally smooth rather than geologically scarred.
   A world that was never lived in.
4. **Over-legibility.** UI that explains itself with too many words and
   labels. Real game UI trusts the player to learn through play.
5. **Matching everything.** When all elements match, nothing has identity.
   A game where the title screen, the crafting screen, and the inventory
   all feel like they were created by the same function call in a UI
   library.

## The five principles (apply these across ALL sections)

### Principle 1 — Every choice is a design choice

If a value in the code was chosen because it was the easiest to type
(e.g., `width: 200.0`, `color: white`, `margin: 10.0`), it is probably
wrong. Every dimension, color, and spacing value in the LOREFORGE UI must
be chosen because it serves the design.

The LOREFORGE palette (from `menu/MAIN_MENU_REDESIGN.md`) is the only
acceptable color set for UI. Any color not in that palette is a bug.
The spacing system (multiples of 8) is the only acceptable spacing.
Any spacing not a multiple of 8 is a bug.

This sounds rigid but it produces **coherence**, which reads as designed
rather than assembled.

### Principle 2 — Imperfection is a craft decision

A rocky cliffside in a voxel game should not have a mathematically smooth
face. The noise function produces the variation, but the variation should
be *enough* to read as geological. If the terrain looks smooth from 100
blocks away, the noise amplitude or frequency needs adjustment.

A crafting screen where every recipe row is exactly 36px tall is a
database. A crafting screen where rows are sized to their content
(and that content varies slightly by how many materials a recipe uses)
is a workbench.

### Principle 3 — The game world has a voice in the UI

Every UI screen in LOREFORGE is made by people who live in Valdenmoor.
They don't use modern UI conventions (they don't know what those are).
They use **parchment-toned backgrounds, ember-orange highlights, warm
off-white text, iron-brown panel borders.** The game world's aesthetic
extends into the interface.

This means:
- No blue-tinted color schemes (unless in a "cold/ice" context where it's
  intentional).
- No pure white or pure black.
- Panel backgrounds have depth (the dark warm brown palette gives this).
- The accent color (`#c4602a`, ember-orange) is used for emphasis, not
  decoration.

### Principle 4 — Trust the player, don't explain everything

The crafting screen doesn't need a tutorial overlay every time it opens.
The main menu doesn't need a tooltip explaining what "Multiplayer" means.
The biome color grade doesn't need a popup saying "You've entered the
Desert biome."

Complexity is revealed through play, not front-loaded through UI text.
The place for explanation is lore books and the chronicle — not tooltips
and overlays. One exception: new mechanic first-encounter toasts (e.g.
"New recipes unlocked" when picking up an item for the first time) are
acceptable because they're brief and informative without being patronizing.

### Principle 5 — Verify against real human judgment, not test coverage

A vistest PNG passing pixel-analysis proves the renderer worked. It does
not prove the UI looks good. After every visual section, do a real
human-eye look at the full game running in release mode and ask:
"Does this look like a game that someone cared about?"

If the answer is "not yet," find the element that's pulling it toward
generic and change it before marking the section done. This is not
documented anywhere except here — it's a judgment call, and it requires
a human to make it. The DEVLOG.md entry for a visual section must include
the answer to the question "did this pass a real human look?"

## Specific anti-patterns to find and remove

Search the UI code (`lf_client/src/ui.rs`, `lf_client/src/ui_kit.rs`)
for these patterns and fix them:

| Anti-pattern | Replace with |
|---|---|
| `Color32::WHITE` | `Color32::from_hex("#f0ead6")` (text-primary) |
| `Color32::BLACK` | `Color32::from_hex("#1a1410")` (background-deep) |
| `Color32::BLUE` or any blue not in palette | Appropriate palette color |
| `Rounding::same(8.0)` or similar rounding on all corners | `Rounding::ZERO` (sharp corners, LOREFORGE aesthetic) |
| `egui::Button::new(text)` with no custom styling | Styled with the underline-hover pattern |
| Hardcoded pixel dimensions not multiples of 8 | Round to nearest multiple of 8 |
| Any `ui.heading(text)` using default egui styles | Styled heading matching the typography system |
| Any `ui.separator()` with default egui color | Styled with `#4a3f2e` (border/line) |

## Performance note

Every visual improvement must be checked against the low-end device target
(documented in `DECISIONS.md` from prior work). The color-grade post-
process pass, vignette overlay, and particle systems all have GPU cost.
If a new visual feature causes a measurable fps drop on the target device,
find an optimization before shipping — never silently let the low-end
experience degrade.
