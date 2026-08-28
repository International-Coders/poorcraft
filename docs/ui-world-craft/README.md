# UI, World & Crafting Polish Pack

Drop this folder into `.zcode/plans/` in the repo.
The single prompt to paste into z.ai is `prompt/START_HERE.md`.
All other files are reference documents the AI reads during that job.

## What this pack covers

| Problem you described | File that fixes it |
|---|---|
| Main menu wrong name, wrong style, "AI-looking" | `menu/MAIN_MENU_REDESIGN.md` |
| Rotating preview world tied to version seed | `menu/VERSION_SEED_WORLD.md` |
| No world-creation flow (mode, difficulty, seed) | `menu/WORLD_CREATION_FLOW.md` |
| Map generation too mountain-heavy, not interesting | `worldgen/WORLDGEN_REWORK.md` |
| Biomes not visually distinct enough | `worldgen/BIOME_IDENTITY.md` |
| Crafting system looks AI-generated, not user-friendly | `crafting/CRAFTING_REVAMP.md` |
| Visual polish — game should not look AI-made | `visuals/POLISH_PRINCIPLES.md` |

## The one thing every file here agrees on

"AI-generated" in visual output means: generic, symmetric, predictable,
over-clean, no history, no wear, no personality. The antidote is not
random noise — it is **intentional imperfection, consistent visual rules,
and human decision-making embedded into the design system**. Every section
below describes exactly what those rules are for its subsystem.
