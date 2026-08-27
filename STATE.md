# STATE
loop_count: 310
current_milestone: fluids + block gravity (user request; P30 groundwork early)
last_done: "Block gravity + water physics (user ask: 'sand and grass DO NOT FLOAT, they fall' + 'work on the water physics'). Gravity: registry::has_gravity marks granular blocks (sand/red_sand/snow/dirt/grass/moss/mycelium; ores excluded — embedded in stone, MC rule); breaking support cascades the column into animated FallingBlock entities (own texture, water-damped, land through the same remesh+broadcast path as player edits); pure settle_gravity powers tests/vistest. Water: event-driven cellular sim in lf_game::fluids — flow level 0(source)..7 stored in BlockState's flag nibble, fall-first then decaying horizontal spread, unsupported flow dries up (bucket scooping recedes puddles); mesher renders flowing water as stepped lowered surfaces with step-covering sides; 64-cell tick budget keeps frame cost bounded; bucket/water_bucket craftable (3 iron) to scoop and pour sources. 6 new tests (spread decay, recede, fall+pool, column collapse, water displacement, lowered-surface meshing); 2 new vistest proofs (water_flow: aqueduct->flume->dam pooling settled through the real sim; falling_sand: collapsed pile + mid-air faller) — both visually verified. 174 tests green; 24/24 vistest; user's live session still running (their play = smoke; no pkill)"
next_task: "build-pack Step 2 remainder: audio engine + break/place sounds (Step 4), biome identity (16-19), Geode/Cinder spawn-or-cut, q4 Collected fix, Welcome.seed; then P28 gate remainder"
build: GREEN
tests: 174 passing
last_screenshot: shots/audit_water_flow.png
blockers: "none"
