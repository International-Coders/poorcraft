# STATE
loop_count: 348
current_milestone: Complete — material-colored voxel lighting and craftable hearth sources
last_done: "loop 348 colored-light pass: block light is now RGB max-composited and attenuated per channel while retaining the existing u32 vertex layout and legacy 0xF0 sky encoding; ordinary torches/lanterns/lava/ember/lumen/radiation now cast material-specific colors, with subtle position-phased warm flicker. Added craftable/placeable Ember Torch, Lumen Torch, and Fireplace blocks with distinct procedural pixel textures, drops, recipes, hit behavior, and server-valid registry IDs. Fixed two proof-discovered production bugs: emitters beneath roofs were never scanned, and skylight flood began on the opaque roof cell and leaked through it. Direct section scanning skips irrelevant sections to recover lighting-build performance. 406 tests, 94/94 vistest, smoke OK, warm perf p50 53.7ms, runtimes rebuilt."
next_task: "First-minute onboarding pass: contextual movement/mining/crafting prompts plus a pinned starter objective, dismissible and persisted, with compact HUD and small-window vistest proofs (carried over from loop 344 planning)."
build: GREEN
tests: 406 passed / 0 failed (loop 348)
last_screenshot: shots/vistest_colored_light_room.png
blockers: "none"
