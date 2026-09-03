# STATE
loop_count: 349
current_milestone: Complete — real sampled sound bank across 33 events
last_done: "loop 349 sound bank: 33 sound effects generated with the ElevenLabs SFX API (free-tier key, 620/10,000 chars spent) — tools/gen_sounds.py is the cache-aware generator (make sounds; key via env, never committed), the MP3s are committed in assets/sounds/ and embedded via include_bytes so the catalog cannot drift. lf_audio decodes them at boot (rodio/symphonia → mono, peak-relative silence trim, near-silence rejection, 0.85 normalization) and prefers the samples with the old synthesizer kept as per-event fallback; 12 new Sfx events wired in the client (splash edge, bow, arrow stick, melee swing/hit/death, mob hit/death, dragon-mount roar, pickup, craft done, chest open, anvil clang, death sting). Quiet-mastering quality issue root-caused (footstep-type prompts came back near-silent; impact-texture prompts did not) and the 8 affected files regenerated. 410 tests (+4), 94/94 vistest, smoke OK, runtimes rebuilt."
next_task: "First-minute onboarding pass: contextual movement/mining/crafting prompts plus a pinned starter objective, dismissible and persisted, with compact HUD and small-window vistest proofs (carried over from loop 344 planning)."
build: GREEN
tests: 410 passed / 0 failed (loop 349)
last_screenshot: shots/vistest_colored_light_room.png
blockers: "none"
