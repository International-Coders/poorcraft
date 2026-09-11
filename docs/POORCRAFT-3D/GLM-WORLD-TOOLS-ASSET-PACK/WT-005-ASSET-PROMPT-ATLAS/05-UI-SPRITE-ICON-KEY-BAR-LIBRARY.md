# UI Sprite, Icon, Key, And Bar Library

UI assets are real game assets. They need alpha, states, names, contrast, and
runtime binding rules.

## HUD Bars

- health: normal, low, poisoned, burning, healing;
- stamina: normal, exhausted, sprinting, blocked;
- food: full, hungry, starving;
- mana: charged, depleted, corrupted;
- air: underwater, low air, drowning;
- machine progress: idle, working, blocked, complete.

## Icons

- actions: talk, trade, open, forge, build, harvest, inspect, map, quest,
  sleep, repair, research, cast, fish, mount;
- resources: wood, stone, ore, ingot, coal, food, herb, crystal, gold, mana,
  power, population;
- status: cold, hot, poisoned, wet, rested, overburdened, wanted, blessed,
  cursed, hungry, injured.

## Keys And Controls

- keycaps must be blank shapes plus runtime text;
- labels come from bindings, not baked pixels;
- states: normal, hover, pressed, disabled, conflict, rebinding;
- provide mouse buttons, scroll wheel, controller buttons later.

## Map Markers

- home, spawn, town, quest, mine, cave, ruin, danger, faction, road, port,
  shrine, dungeon, boss, caravan, siege, resource.

## Alpha And Contrast

Every sprite sheet needs transparent background, dark/light backdrop test,
disabled state, focus state, and UI scale proof.
