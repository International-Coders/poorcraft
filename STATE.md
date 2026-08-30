# STATE
loop_count: 337
current_milestone: Complete — smart HUD + personalized font + Minecraft controls
last_done: "loop 337 menus-and-controls: (1) SMART HUD — kit::hud_layout(w,h) computes every HUD region (info line capped away from the minimap, companion tiles ending above chat, chat above the hotbar band) with a disjointness test proving zero overlap at 640x360/800x600/1280x720/1920x1080, and the live HUD regions (chat, companion tiles, info line width, minimap) re-anchored to the computed layout; (2) personalized font — kit::install_font promotes the embedded Hack monospace over the whole UI with a 1.06 scale tweak (installed once, not per frame); (3) Minecraft controls — SHIFT sprints, CTRL crouches (defaults swapped, FlyDown=CTRL), and crouching edge-locks: movement that would leave the supporting block is cancelled per axis while sneaking on the ground (tested: sneaker holds the 3x3 ledge 600 ticks, walker falls; sneak slower than walk, sprint faster); sneak lowers the eye 0.28. hud_small vistest scene (640x360) pixel-proves hotbar/minimap/info bands hold. 349 tests, vistest suite, smoke green."
next_task: "Deferred: bigger UI overhaul (custom pixel TTF font is the next step — needs a font asset), more HUD polish; or master-plan Phase C (docs/MASTER-PLAN.md)."
build: GREEN
tests: 349 passed / 0 failed (loop 337)
last_screenshot: shots/vistest_hud_small.png
blockers: "none"
