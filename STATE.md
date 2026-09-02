# STATE
loop_count: 341
current_milestone: Complete — HUD declutter + inventory-first E screen, researched Minecraft conventions
last_done: "loop 341 HUD pass (user-goal 3, web-researched): info line minimal by default (clock + facing; the dense biome/coords/weather/net/fps/RT readout moved behind F3 per Minecraft's show-nothing HUD convention); E now opens a real inventory screen — armor column (head/chest/legs/feet + off hand) beside a painted kit-block player portrait, 3x9 storage grid, hotbar band with selection frame, craft-by-hand route to the basic workbench via the new UiOpen::HandCraft; furnace + chest restyled from raw egui window chrome into the design-kit panel shell (vignette + framed panel + proper titles). vistest inventory_screen scene (mirrored preview + pixel claims: slot wells/accent/title band) and the mirrored HUD info line synced. Verification: full vistest + workspace tests + smoke (numbers in DEVLOG/CHANGELOG)."
next_task: "User goal pass 3 remainder: convert the remaining pre-kit screens (13 machine windows, trade, companion menu, tech tree) to the kit panel shell (mechanical, copy the furnace conversion); then build-mode HUD (shape picker for slab/stairs, symmetry indicator, ghost preview polish) on the blueprint-ghost overlay hooks; then pass 4 (missing texture patterns audit + fill)."
build: GREEN
tests: 365 passed / 0 failed (loop 341)
last_screenshot: shots/vistest_inventory_screen.png
blockers: "none"
