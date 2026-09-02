# STATE
loop_count: 344
current_milestone: Complete — clear sky, visible pixel sun, and sun-tracked raster relief
last_done: "loop 344 visual sky pass: added authored 16x16 pixel-art sun, crescent moon, and star atlas layers; celestial quads carry an atmosphere marker that bypasses distance fog and terrain color grading while still respecting depth, so fog can hide unrendered distant terrain without blanketing the unreachable sky. Raster normal/face relief now uses the exact same public sun_direction(time) vector that positions the visible sun, making highlights move coherently through the day at negligible cost; the existing path tracer retains real cast shadows. Corrected stars to appear while the sun is below the horizon. Added asset/atmosphere unit locks, a GPU east-vs-west shading regression, and a sun_visibility proof with fog_far=48 versus a 420-block celestial distance. 371 tests, 89/89 vistest, smoke green; terrain_vista perf p50 63.6ms / p95 80.7ms."
next_task: "First-minute onboarding pass: contextual movement/mining/crafting prompts plus a pinned starter objective, dismissible and persisted, with compact HUD and small-window vistest proofs."
build: GREEN
tests: 371 passed / 0 failed (loop 344)
last_screenshot: shots/vistest_sun_visibility.png
blockers: "none"
