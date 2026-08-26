# STATE
loop_count: 307
current_milestone: P27-camera-culling-fix
last_done: "P27 camera fix (user report: objects disappear when looking up): the chunk-column frustum cull approximated each 16x16xH column with a sphere of radius max(half_h, 11.4) — that only covers the footprint along its axes, while the true corner distance is sqrt(128 + half_h^2) (~13.6 for flat ground, ~17.7 for a 20-tall column). When the bottom frustum plane swept up with the view, columns still poking into the frame edge were wrongly culled by 2-6 blocks — terrain/objects vanishing at high pitch (and tall columns even near level pitch; not the raycast or FOV, both are fine — pitch is clamped to 89 and look_at stays non-degenerate). Fix: exact AABB bounding sphere + 0.1 sway margin, and the Gribb-Hartmann planes are now normalized so the world-unit radius means what it says (near-plane normal was ~2x unit). Regression test scans pitches 5-85 deg, four eye heights, five column heights around the camera asserting corner-inside-frustum => kept, plus the pinned pre-fix failure (pitch 5, tall column at the frame edge); verified the test fails on the old formula and passes on the new one. 161 tests green; 22/22 vistest; smoke OK"
next_task: "user playtest (look up/down — culling fixed); then choose: connected-surface textures on large man-made materials, audio engine, or multiplayer hardening"
build: GREEN
tests: 161 passing
last_screenshot: shots/vistest_foliage_canopy.png
blockers: "none — push to github works again (P25+P26 pushed as 5f7cb4d)"
