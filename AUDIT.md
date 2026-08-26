# AUDIT — Reality Audit of BACKLOG "Done" Claims (build-pack Step 1)

Method per `docs/poorcraft-build-pack/01-REALITY-AUDIT.md`: every `[x]`
item in BACKLOG.md was (1) read in code (impl + test + consumer, not
stubs), (2) run — a live session of the release build on this host
(title screen captured twice, an in-world session observed, smoke log
inspected) plus the vistest harness, and (3) screenshot where visual
(`shots/audit_*.png`, fresh `shots/vistest_*.png`). Audit date:
2026-08-26, loop 309, commit range: post-P28-loop-1 (65e8672).

Verdicts: **CONFIRMED** (all three legs pass), **PARTIAL** (real but a
named piece is missing/weak — annotated, not silently accepted),
**ACTUALLY-BROKEN / ACTUALLY-MISSING** (the claim was not true in play).
Items found broken are either **FIXED in this commit** or listed under
"Open findings → Step 2" at the bottom. BACKLOG.md was corrected in the
same commit.

## The three user-flagged areas first

### Block destruction feedback
| Aspect | Verdict | Evidence |
|---|---|---|
| Crack overlay during hold-to-mine | CONFIRMED | `rebuild_crack_batch` runs every frame during real mining (crates/lf_client/src/lib.rs tick→render path), stage 0..3 decals from `CRACK_LAYERS`; vistest `mining_feedback` scene pixel-checked |
| Break particles using the block's texture | CONFIRMED | `spawn_break_particles` → `texture_index_for_block(block_id)` with random sub-tile UVs; cap 128; gravity + ground stop |
| Break/place **sound** | **ACTUALLY-MISSING** | No audio system exists anywhere in the workspace (no cpal/rodio/kira dep; the settings sliders literally say "the audio engine consumes them when it lands", ui.rs:1441). Never claimed done in BACKLOG, but it is the missing third pillar of destruction feedback → build-pack Step 4 |

### Lore
| Aspect | Verdict | Evidence |
|---|---|---|
| Quests advance from real gameplay | CONFIRMED | `QuestLog::record_event` fed from pickup/craft/kill; test `events_advance_and_complete_quests` |
| Chronicle visible DURING play | CONFIRMED (weaker than it could be) | Quest log (J) renders the live saga via `SagaGenerator::export_markdown` (ui.rs draw_quest_log); `Book` item also opens it. Complaint root cause: zero discoverability (only a pause-menu hint mentions J), thin 2-sentence prose, and 5 of 11 chronicle event types never fire (GreatTrade/Discovery/StructureCompleted/VillageFounded/RuneApplied/ItemCrafted have no producers) |
| Lore books readable in-game | PARTIAL (was already marked deferred) | The book UI works and shows the real chronicle; the deferred part (paginated lore-book *content* beyond the saga) remains open, honestly marked |
| Consistent named lore across systems | PARTIAL | Two real threads exist ("The Smith", glitch/Null arc: mob→drop→quest→boss→trade cross-references). Villager names appear only in trade-window titles; no dialogue system; no named places; GeodeGuardian/CinderCrawler doc-comments reference biomes that do not exist |

### Biomes
| Aspect | Verdict | Evidence |
|---|---|---|
| 30 biomes exist and are reachable | CONFIRMED | `Biome` enum has exactly 30 variants (biome.rs:9-46); tests `every_biome_variant_reachable` + `all_biomes_appear_across_sampled_world`. BACKLOG M4's "all 8 biomes reachable" was **stale text** (the M4-era count; corrected in this commit — there is no STATUS.md; STATE.md never said 30, the V1REBRAND docs do, correctly) |
| Biomes look meaningfully different | **ACTUALLY-BROKEN (as an experience claim)** | 17–18 of 30 biomes are worldgen-identical to at least one sibling (same surface+filler+tree+freezes, no unique feature): Meadow/Savanna/WindsweptSavanna; Forest/FlowerForest; 3 oceans; SnowyTaiga/Tundra/SnowySlope; Desert/Beach; StonyShore/Mountains; WindsweptHills/MushroomHollow. 14 biomes share one untinted GRASS texture (no per-biome tint exists); fog/sky is global, never biome-dependent (`env()` has no biome input); MYCELIUM block exists but is never placed; the 30-way color identity only exists on the **map screen** (`biome_color`), not in the world. The `biome_montage` vistest scene is a single vista (2–3 biomes), not a montage — it cannot prove the claim its name makes. → build-pack Steps 16–19 |

## Milestones (M1–M14 + done block)

| Item | Verdict | Notes |
|---|---|---|
| M1 window/clear | CONFIRMED | lf_engine app.rs |
| M2 chunk meshing + texture array | CONFIRMED | meshing.rs + scene.rs array; border-cull test |
| M3 DDA raycast | CONFIRMED | raycast.rs + test; wired to input since P1 |
| M4 terrain/biomes/strata | PARTIAL | real; distinctness broken (above); BACKLOG text corrected 8→30 |
| M5 region persistence | CONFIRMED | atomic tmp+rename, round-trip tests |
| M6 day/night math | CONFIRMED | Note found while auditing the dark title: `sky_light_level()` is a hard binary day/night switch (0.8–1.0 day, 0.12 night) — no dawn/dusk ramp; flagged for Step 6's lighting pass |
| M7 survival types | CONFIRMED | stacking tests |
| M8 smithing | PARTIAL→FIXED | system real and reachable; but the forge UI called `strike()` **every frame** and granted a steel ingot **every frame** once hot — fixed in this commit (Strike button + `ForgeMinigame::reset`, test `forge_reset_starts_a_fresh_workpiece`) |
| M9 mob model incl. Null Knight | CONFIRMED | data + actually spawned (2% night roll); arena/phases honestly open |
| M10 mod loading | CONFIRMED | ember_ores + amberium load at boot (live smoke log shows both) |
| M11 protocol/UDP | CONFIRMED | superseded: v3 + authoritative server, not just echo |
| M12 villagers + Geode/Cinder mobs | **PARTIAL** | villager schedules/trading real; **GeodeGuardian + CinderCrawler are dead data** — zero references outside lf_npc's own tests, and their lore names biomes that don't exist. BACKLOG corrected; Step 2+ decides: spawn them or cut them |
| M13 quest types | CONFIRMED | dead arms noted (Reach/Interact/Escort/Defend + ReachedDepth never produced) |
| M14 chronicle | CONFIRMED | export-on-save confirmed in the live session (worlds/<slot>/chronicle.md) |
| P0 renderer/headless/vistest | CONFIRMED | 22 scenes, pixel gate enforced |
| P22 icons/tooltips/recipe book/maps | CONFIRMED | all wired to real data |

## P1–P11 phase items

| Item | Verdict | Notes |
|---|---|---|
| P1 input incl. sneak | PARTIAL→FIXED | sneak was captured but **never read** by physics (hollow claim); fixed in this commit (0.45× walk speed + test `sneaking_walks_at_half_speed`) |
| P1 physics (8 tests) | CONFIRMED | actually 7 tests in player.rs + 1 spawn test in lf_client; count nit |
| P1 camera/targeting/hotbar/F2 | CONFIRMED | nuance: F2 is an offscreen re-render (no water, no crack/particles) — noted for Step 3 |
| P2 streaming "view radius 5" | **PARTIAL→FIXED** | worker wish radius was **hard-wired to 5** — the view-distance setting (and the High preset) never streamed farther; fixed in this commit (`sync_wish` + tests) |
| P2 worldgen/culling/save/registry | CONFIRMED | all four verified incl. tests |
| P3 light BFS | CONFIRMED | cross-column seams still accepted (DECISIONS P3) → P28 remainder |
| P3 torches/lanterns placeable | **PARTIAL→FIXED** | torch real; **lantern was unplaceable** (no item, no recipe, console give rejected it) despite the block existing with light 15; fixed in this commit (item + iron-over-torch recipe + drop + test) |
| P3 day/night, water pass, sky/weather | CONFIRMED | star-fade value computed but unused (nit, Step 11 scope) |
| P4 HUD/inventory/crafting/mining/drops/survival/eating | CONFIRMED | HUD also confirmed live in the observed in-world session (hearts/hunger/XP/hotbar/minimap/info line all present) |
| P5 furnace/chest/planks-glass/iron/persistence | CONFIRMED | block-entity persistence lacks a dedicated roundtrip test (noted) |
| P6 mobs/combat/XP/villagers | CONFIRMED | caveats: NullKnight arena/phases open (admitted); "wander by schedule" is generous (schedule only gates rest) |
| P7 structures biome-gated | CONFIRMED | hut=Meadow-only, tower=Highlands-only, pyramid=Desert-only via chunk-center biome match — a swamp pyramid cannot generate |
| P7 title/pause, hud_preview, weather, world types | CONFIRMED | weather "by biome" is a surface-block proxy (snow/ice under player), noted for Step 19; vistest UI scenes use faithful local re-implementations, not the live draw path (noted) |
| P8 quests/quest-log/chronicle/persistence | CONFIRMED | chronicle depth caveats above; quest bug found: **q4 "collect iron ingot" can't complete the intended way** — furnace output and trade grants never emit `Collected` (only ground-drop pickup does) → Step 2 |
| P9 multiplayer | PARTIAL | protocol/server/test/dedicated/client-join all real; but **Welcome.seed is ignored** (each client generates its own local-seed terrain — same server, different worlds; only edited blocks sync) and the title connect is hardcoded `127.0.0.1:25565` + name "smith" → already P28/V1REBRAND backlog + Steps 34–35 |
| P10 mod API | CONFIRMED | full-pipeline test real |
| P11 perf/release | CONFIRMED | RELEASE.md said "121 tests / 14 scenes" — stale, corrected in this commit (168 / 22) |

## P25–P28 (recent honesty sweeps)

| Item | Verdict | Notes |
|---|---|---|
| P25 all 8 items | CONFIRMED | re-verified (validation, genver, lantern texture — the *texture* was real; the *item* was the gap fixed today) |
| P26 visual identity (7 items) | CONFIRMED | sway now genuinely in-shader since loop 308 (GPU two-phase test); crack/debris confirmed in real play path |
| P27 culling fix | CONFIRMED | regression tests re-run green |
| P28 loop-1 items | CONFIRMED | this audit re-exercised them |

## New bugs found BY this audit (not pre-claimed)

| Finding | Verdict | Status |
|---|---|---|
| Gameplay HUD renders under the title menu (hearts/hotbar/info line visible behind the buttons — live-captured in shots/audit_title.png) | ACTUALLY-BROKEN | **FIXED** this commit (`hud_visible` gate + test) |
| Title screen backdrop renders flat-dark: the orbit camera's fixed spawn+14 eye is **buried inside ring terrain** on hilly worlds (World_5: 12/64 orbit points under higher ground; tool `crates/lf_worldgen/examples/audit_title_camera.rs`; captures shots/audit_title.png, audit_title_later.png mean RGB ≈38) | ACTUALLY-BROKEN | **FIXED** this commit (`title_eye_y` clamp + test); the vistest menu_preview never caught it because it picks its own camera |
| render() culled/sorted with the player eye while rendering from the title orbit camera | ACTUALLY-BROKEN (title-only visual) | **FIXED** this commit (cull/sort now use `camera.eye`) |
| `random_seed()` could return identical values for two calls in one clock tick (test flake observed during the audit run) | flaky-by-construction | **FIXED** this commit (sequence counter mixed in) |
| `shots/ev_*.png` — 201 tracked fossil "proof" PNGs from the disavowed Evolution-era list; several name creatures that exist nowhere in code (voidserpent, allay, axolotl, breeze — zero references) | evidence hygiene | **REMOVED** this commit (history preserves them); only registry-driven `vistest_*` shots are evidence |

## Open findings → Step 2 (not fixed in this commit)

1. **Break/place/mine audio + the whole audio engine** — build-pack Step 4
   (kira per BACKLOG's note). The single biggest feel gap in the flagged
   area.
2. **Biome visual identity** (tint, biome fog, unique features, montage
   contact-sheet scene) — Steps 16–19; the worldgen twins table above is
   the work list.
3. **GeodeGuardian / CinderCrawler dead data** — spawn them somewhere real
   or delete the structs (their lore references nonexistent biomes).
4. **Quest q4 completion path** — furnace-output and trade grants should
   emit `Collected` (or q4 should count ingots from any source).
5. **Multiplayer terrain desync** — consume `Welcome.seed`; address/name
   entry (already in P28/V1REBRAND backlog; Steps 34–35 for lobbies).
6. **Chronicle depth** — fire the five dead event types; richer prose;
   discoverability of J/book (Steps 20–22).
7. **Dawn/dusk light ramp** — binary day/night switch (Step 6 scope).
8. **F2 re-render omissions** (water/crack/particles) — fold into Step 3.

## Live-session evidence index

- `shots/audit_title.png` — title screen at first launch (menu complete;
  HUD-behind-menu bug visible; dark backdrop = buried orbit camera)
- `shots/audit_title_later.png` — second capture, still dark (different
  orbit angle), pixel stats in DEVLOG
- `shots/audit_inworld.png` — live in-world session (underground + torch:
  block-light rendering confirmed working; full HUD present)
- `/tmp/loreforge_audit.log` (session log excerpt quoted in DEVLOG):
  mods load, villager settles, autosave every 30 s, slot switching works
- `crates/lf_worldgen/examples/audit_title_camera.rs` — orbit-burial
  repro tool for any seed (kept as a regression aid)
