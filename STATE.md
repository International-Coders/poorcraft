# STATE
loop_count: 308
current_milestone: P28-v1rebrand-gate
last_done: "P28 loop 1: V1REBRAND execution plan committed (docs/V1REBRAND/11-EXECUTION-PLAN.md, phases P28-P39 = roadmap docs at +2 offset per DECISIONS); wind-sway honesty fix — P26's commit claimed foliage wind but shader.wgsl never read the sway attribute (loc 6) or time_sway uniform, so leaves could not animate; vs_main now applies a world-position-phased double sine (max ~0.08 blocks, inside the 0.1 cull margin), proven by a new GPU-rendered two-phase test (foliage_sway_animates_between_frames: phases differ, same-phase control pixel-identical); clouds setting un-no-op'd; unload radius now view_distance+3 (was fixed 8, zero headroom at view 8); Settings-from-title returns to title on Back/Esc instead of dropping into the world; boot now loads the booted slot's player extras (legacy worlds/default was read before boot_slot). 162 tests green; 22/22 vistest; smoke OK"
next_task: "P28 remainder: chunk-border cross-column lighting (3x3 flood + night seam proof), transparency/sort audit entry, xtask perf + frame-time DECISIONS entry, PathTraced quality tier, key rebinding, save thumbnails, minimap rotation/zoom + beacons, UI language audit, connected-surface textures"
build: GREEN
tests: 162 passing
last_screenshot: shots/vistest_foliage_canopy.png
blockers: "none — push to github works again (P25+P26 pushed as 5f7cb4d)"
