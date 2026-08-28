# Crafting System Revamp — Reference Document

## Why the current crafting "looks AI generated"

AI-generated UIs have these tells, and the current crafting almost
certainly has several of them:
- Every element is the same size as every other element of the same type.
- Information is displayed in a grid or table because that's the default.
- Buttons are labeled with verbs ("Craft," "Close") with no personality.
- Empty states (no recipe selected) show either nothing or a placeholder
  text like "Select a recipe."
- There's no sense of the game world in the crafting UI — it's a pure
  database interface floating over the game.

The fix is not to add decoration. It is to make design decisions that a
human would make — choices that reflect the game's identity, that have
a reason beyond "it's the default."

## The "workbench conversation" design principle

A workbench is a physical object. When you stand at a workbench in a real
workshop, you:
1. See what you can make (your materials suggest possibilities).
2. Pick up the thing you're going to make next.
3. Check if you have what you need.
4. Make it.

The crafting interface should mirror this flow — not a database query
interface where you search for a recipe by name and fill in quantities.
The interface should start from **what the player has** and surface
**what they can make with it**.

## Layout specification

### The three zones (mandatory, not optional)

The `crafting/` UI is a full-screen overlay (same vignette background
as the world creation screen, same panel styling from the LOREFORGE
palette — see `menu/MAIN_MENU_REDESIGN.md` for the palette).

```
┌──────────────────────────────────────────────────────────────────┐
│  CRAFTING                           [close — press E or Esc]     │
├──────────┬───────────────────────┬───────────────────────────────┤
│          │                       │                               │
│ CATEGORY │   RECIPE LIST         │   DETAIL PANEL                │
│ SIDEBAR  │   (scrollable)        │   (selected recipe)           │
│          │                       │                               │
│ 15%      │   40%                 │   45%                         │
│          │                       │                               │
├──────────┴───────────────────────┴───────────────────────────────┤
│  [player inventory — bottom strip, non-scrollable]               │
└──────────────────────────────────────────────────────────────────┘
```

The player inventory at the bottom is always visible while the crafting
screen is open. The player should be able to see at a glance what they
have, which affects the recipe list's "can craft" highlighting.

### Zone 1 — Category sidebar

Categories (in order):
- `Materials` — raw → processed (ore, ingots, planks, stone)
- `Tools` — pickaxes, axes, swords, bows
- `Building` — all placeable blocks (includes faction blocks once unlocked)
- `Food` — all food items and farming
- `Machines` — generators, furnaces, machines (unlocks with eras)
- `Magic` — spells, enchanting, reagents (unlocks with magic system)
- `Armor` — all wearable gear
- `Deco` — decoration blocks, banners, statues

Each category entry:
- A small icon (8×8 or 16×16 from the texture atlas — use an
  appropriate item as the category icon, e.g. an ingot for Materials).
- A short label.
- Shows a count badge: how many recipes in this category are
  currently craftable (green number) vs. total visible (grey total).
  Example: `Materials (3/12)` meaning 3 can be crafted now out of 12
  visible.
- Selected state: left accent border (2px, `#c4602a`), label in
  `#f0ead6`. Unselected: label in `#8a7f6e`.
- On hover: label brightens, no other change.

### Zone 2 — Recipe list

Within the selected category, recipes are shown in this order:
1. **Can craft now** (all materials available) — sorted by the item
   level/tier (stone tools before iron before steel, etc.)
2. **Partially available** (some but not all materials in inventory)
3. **Visible but can't craft** (none of the materials available)
4. **Locked** (prerequisite not met — greyed out, last, show lock icon)

Each recipe row contains:
- `[16×16 icon]  Item Name   N× needed materials  ✓ or ·`
- The materials summary is abbreviated: "2× log, 3× stone" — not a full
  breakdown, just the top 2–3 materials by quantity.
- The checkmark `✓` is `#6b8e23` (success green) if fully craftable.
- The dotmark `·` is `#8a7f6e` (muted) if not.
- Locked recipes: `[🔒]  Item Name   Needs: [prerequisite name]`
  in `#4a4438` (text-disabled).

Row height is NOT uniform. A recipe with 3 materials to show
in the summary will be slightly taller than one with 1. This is correct.

Clicking a row opens it in the detail panel.

### Zone 3 — Detail panel

When no recipe is selected:
- Show the category's "greeting text" — a 2-line flavor statement for
  each category. Example for Materials: "The foundation of everything
  you'll build. Iron doesn't apologize for being iron." This text is
  stored as data (per-category strings), not hardcoded in the renderer.
- Below the flavor text: "Select a recipe to see details."
  in `text-muted`.

When a recipe IS selected:

```
[64×64 item icon, rendered from texture atlas]

IRON INGOT                        (item name, 20pt, text-primary)
Materials                         (category, 12pt, text-muted)

The Ironborn have smelted this    (flavor text, 12pt, text-muted,
in their forges since Era I.      italic if possible, 2 lines max)
─────────────────────────────────
INGREDIENTS

[icon] 1× Iron Ore        ✓ Have 14
[icon] 1× Coal            ✓ Have 6
                           
─────────────────────────────────
PRODUCES

→ [icon] 1× Iron Ingot

─────────────────────────────────
QUANTITY

          [-]  [  1  ]  [+]
    Needs: 1× Iron Ore, 1× Coal

─────────────────────────────────

            [ Craft 1 ]
```

Styling details:
- Item name: 20pt, `#f0ead6`
- Category label: 12pt, `#8a7f6e`
- Flavor text: 12pt, `#8a7f6e` (same muted color, set it apart with
  a brief horizontal rule above and below)
- Dividers (the `────` lines): 1px `#4a3f2e`
- Ingredient rows: icon + name left-aligned, "✓ Have N" right-aligned
  in the same row. Green if sufficient, orange if partial, red if none.
- Quantity selector: the `[-]` and `[+]` are the same hover-underline
  interaction as navigation elements — NOT visible bordered boxes. The
  count `[1]` is a text input that the player can type into directly.
  When quantity changes, the "Needs:" line below updates instantly.
- "Craft N" button: at the bottom of the detail panel. If craftable:
  `#c4602a` accent color underline permanently visible (action button).
  If NOT craftable: label changes to "Missing materials" in `#4a4438`,
  no underline, no hover response.

### Bottom inventory strip

The player's hotbar + inventory, condensed into a 2-row strip at the
bottom of the crafting screen. Item counts are shown on each slot.
Mousing over a slot highlights it and shows a tooltip with the item
name. Items relevant to the currently selected recipe are subtly
highlighted (a faint accent-color border on matching inventory slots).
This is the "workbench conversation" — you see your materials while
you look at what you could make.

## Recipe visibility system

This is a new system, not currently in the game. Implement it as a
`HashSet<RecipeId>` in `ClientSave` (persisted):

```rust
pub struct RecipeBook {
    pub known_recipes: HashSet<String>,  // recipe IDs
}
```

Initialization: populate with all "always visible" recipes at world
creation. Always-visible set (hardcode as constants):
- All planks-from-logs recipes
- Basic stone tools (pickaxe, axe, shovel, sword)
- Torches, sticks, crafting table
- Basic food processing (bread, cooked meat)
- Chest, furnace
- Basic armor (leather/cloth)

New recipe unlock rules (add to existing pickup/collect event handling):
- When the player picks up item type X for the first time: unlock all
  recipes where X is a *primary* ingredient (the ingredient with the
  highest quantity or the one listed first in the recipe data).
- Emit a HUD toast: `"Recipes unlocked: [N] new recipes available"`
  in the `warning` amber color (it's a positive notification, but amber
  is more visible than green). The toast uses the existing toast system
  if one exists; otherwise add a simple queued-toast HUD element.
- Research era unlocks: hook into the existing era-unlock event to call
  `unlock_era_recipes(era_id)`, which adds all recipes tagged with that
  era to the known set.

## Recipe data format

Each recipe in the TOML/data files gets one new optional field:
```toml
[[recipe]]
id = "iron_ingot"
category = "materials"
output_item = "iron_ingot"
output_count = 1
flavor_text = "The Ironborn have smelted this in their forges since Era I."
always_visible = false
unlock_on_pickup = ["iron_ore"]  # unlocked when player first picks up iron_ore
unlock_by_era = ""               # blank = no era lock
unlock_by_faction_standing = ""  # blank = no faction lock
ingredients = [
  { item = "iron_ore", count = 1 },
  { item = "coal", count = 1 },
]
```

The `flavor_text` is mandatory for all recipes (even if a generic one).
Recipes without flavor text look like a database, not a crafted world.
Generic fallbacks if a specific one hasn't been written:
- Materials: "A foundation of the world's industry."
- Tools: "Built to last, if you maintain it."
- Building: "A block that can become anything."
- Food: "Sustains the body. Simple as that."
- Machines: "The Ironborn's contribution to the age."
- Magic: "Anima, shaped by intention."
- Armor: "Between you and the world."
- Deco: "Because places should feel like places."

## What the crafting screen does NOT have

- A search bar (the category/sort system replaces the need for search
  at the current recipe count; add search only when the recipe count
  exceeds ~200 items).
- A 3×3 grid (the grid metaphor is the Minecraft metaphor; this game's
  crafting has its own metaphor).
- Tabs within a category (categories are already a top-level filter;
  nested tabs double the navigation work for minimal benefit).
- An animation on craft (a particle or visual on the inventory is fine,
  but the crafting screen itself should not play an animation — it's a
  tool, not a ceremony, for the first few hundred hours of crafting the
  same items).
