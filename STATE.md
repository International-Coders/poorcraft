# STATE
loop_count: 351
current_milestone: Complete — transactional crafting + real craft queue (nightly-beta N02)
last_done: "loop 351 craft correctness (N02): new lf_game::crafting transactional engine — execute() validates every ingredient against real inventory counts, PROVES output capacity, then consumes and grants exactly (batched past the u8 add_item boundary with zero loss; the old grant loop silently dropped outputs past 255), typed CraftOutcome/CraftBlock reasons (missing item with need/have, no-room with needed/free), max_batches integer-safe craft-all limited by materials AND room, Inventory count_of/free_capacity/remove_count helpers; a blocked craft consumes NOTHING. The client craft buttons + new Craft All run the engine with exact missing-item lines; the placeholder queue became REAL — enqueue reserves nothing (documented rule), one engine-verified job completes per 1.25s of play, blocked jobs show live reasons in the queue strip, cancel is free, the queue persists via the unchanged ClientSave shape, unknown recipes drop honestly; quest/audio/tutorial events fire exactly once per completed craft. 8 engine tests + 3 client tests (queue status, catalog lookup, save round-trip). 433 tests total, smoke OK, runtimes rebuilt."
next_task: "N03 — workbench visual hierarchy and input recovery (docs/NIGHTLY-BETA/10-OVERNIGHT-JOB-QUEUE.md): modal normal/compact layouts with world scrim, ingredient ownership/disabled reasons on the transactional engine, stable focus, E/Escape recovery integration tests, and the crafting_workbench/_small/missing_ingredients/queue vistest proofs from 03-HUD-AND-CRAFTING.md."
build: GREEN
tests: 433 passed / 0 failed (loop 351)
last_screenshot: shots/vistest_hud_onboarding.png
blockers: "none"
