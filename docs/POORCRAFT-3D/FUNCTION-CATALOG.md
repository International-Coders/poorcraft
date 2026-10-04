# POORCRAFT 3D — Function Catalog

The numbered meter of what the PLAYED game can do (the BETA-0.2 plan's
W3.0). Every row: a player-reachable function, its input surface, and the
proof that gates it. The law (pc3d_world `catalog` law test): every row
carries an id and a proof reference; the battery fails if a row's proof
is red. **New functions MUST add a row in the same commit** — a shipped
function without a row is an overclaim, a row without a function is a
lie; both fail.

Status vocabulary: `WORKS` (proof green in the battery) · `PARTIAL`
(function exists, proof names a gap) · `PLANNED` (Beta 0.2 row, no code
yet — the row exists so the meter counts the promise).

## Verbs (world interaction)

| id | function | input | status | proof |
| --- | --- | --- | --- | --- |
| V01 | Move / sprint / jump | WASD, Shift, Space | WORKS | battery: vertical-slice; route_vine_climb |
| V02 | Look (mouse) | mouse | WORKS | battery: windowed-3d-axes |
| V03 | Dig one step (yield credited) | G | WORKS | observatory route_dig; battery dig-determinism |
| V04 | Build (placement, foundation-gated) | F | WORKS | battery: rebuild-slice; route_pit_wall |
| V05 | Remove block | R | WORKS | battery: surface-edit |
| V06 | Talk / interact (zone priority) | E | WORKS | observatory route_npc_talk |
| V07 | Melee attack | P | WORKS | observatory route_social (slain ≥ 1) |
| V08 | Recruit / command companion | N | PARTIAL | route_social (follows/waits; assist = W3.4) |
| V09 | Open oversight panel | O | WORKS | observatory route_social (panel shown) |
| V10 | Forge: fuel / ore / take | E→H/T/T | WORKS | observatory route_forge_use (bars ≥ 1) |
| V11 | Machines panel + feed boiler | M | WORKS | observatory route_machine_chain (charge > 0) |
| V12 | Craft menu | C | WORKS | battery: vertical-slice |
| V13 | Pack panel | K | WORKS | battery: ui-states (K reachable) |
| V14 | Quest journal + accept | J, ↓↑, Enter | WORKS | observatory route_semantic_playtest |
| V15 | Save / load (builds+pack+forge+quests) | B / L | WORKS | battery: rebuild-slice (save/reload round-trip) |
| V16 | Pause / resume (never exits) | Esc | WORKS | battery: ui-states |
| V17 | Sleep (skip night, shelter law) | — | PLANNED | W3.3 route_sleep |
| V18 | Fish (river consumer) | — | PLANNED | W3.3 route_fishing; first_catch onboarding |
| V19 | Cook (campfire) | — | PLANNED | W3.3 route_cook |
| V20 | Farm (till/plant/harvest) | — | PLANNED | W3.3 route_farm |
| V21 | Choose a path (career fork) | — | PLANNED | W3.1 route_path_choice |
| V22 | Realm map screen | — | PLANNED | W3.5 ui_realm_map |
| V23 | Ride / fast travel between capitals | — | PLANNED | post-0.2 |
| V24 | Lockpick / chest tiers | — | PLANNED | post-0.2 |

## Survival state

| id | function | status | proof |
| --- | --- | --- | --- |
| S01 | Health (fall/creature wounds) | WORKS | route_plaza_recovery (exact 0.5 law) |
| S02 | Stamina (sprint cost) | WORKS | battery: vertical-slice |
| S03 | Food drain + eating | WORKS | pc3d_world survival laws |
| S04 | XP / levels | WORKS | HUD xp_strip |
| S05 | Armor bar (live) | PLANNED | W3.3 (bar reserved today) |
| S06 | Mana bar (live) | PLANNED | W3.1 mysteries path |
| S07 | Air bar (swim/dive) | PLANNED | post-0.2 |
| S08 | Temperature bar | PLANNED | post-0.2 |
| S09 | Karma/corruption bar | PLANNED | W3.4 (witness chain lands) |

## Machines & economy

| id | function | status | proof |
| --- | --- | --- | --- |
| M01 | Boiler (flow consumer) | WORKS | route_machine_chain |
| M02 | Water wheel (flow potential) | PARTIAL | machines laws; route W3.2 |
| M03 | Mill / crusher / loom / pump / kiln / wind turbine / alembic | PLANNED | W3.2 (machines 2→10) |
| M04 | Trade/escrow with residents | PARTIAL | dialog needs; economy crate thin |
| M05 | Data-driven recipes (TOML) | PLANNED | W5.4 mods door |

## World & progression

| id | function | status | proof |
| --- | --- | --- | --- |
| W01 | Seeded unbounded terrain (8 biomes + pockets) | WORKS | battery: seed-atlas; gen laws 277 |
| W02 | Caves (3D carve, sealed) | WORKS | battery: caves-water; p3d203 |
| W03 | Rivers (flow graph + conforming water) | WORKS | battery: river-water |
| W04 | Terraced cliffs | WORKS | battery: terrain-scenes (cliff seek) |
| W05 | Realm capitals (6, D-026 spaced) | PARTIAL | layout.rs RealmPlan laws (planner done; city render = W1.3) |
| W06 | RiverCarve (valleys under rivers) | PLANNED | W1.4 |
| W07 | Far horizon (no slab edge) | PLANNED | W1.2 far_horizon gate |
| W08 | Day/night + weather | WORKS | battery: day-night |
| W09 | Faction trust (witnessed) | PARTIAL | route_social witnesses; reaction = W3.4 |
| W10 | Two deep career paths | PLANNED | W3.1 (the declared next_task) |

## Shell (screens/menus/HUD)

| id | function | status | proof |
| --- | --- | --- | --- |
| U01 | Title / New World / Load / Settings / Pause / Gameplay | WORKS | battery: ui-states |
| U02 | New World seed preview + reroll | WORKS | battery: seed-preview-ui |
| U03 | Modals (delete/load/quit) with confirm | WORKS | ui laws |
| U04 | 9-slot hotbar (select/use) | PARTIAL | HUD slots; icon render = W4.3 |
| U05 | 9 status bars | PARTIAL | 3 live (health/stamina/food); W4.3 completes |
| U06 | Icon catalog (UiAssetCatalog) | PLANNED | W4.1 |
| U07 | Save-slot browser | PLANNED | W4.4 |
| U08 | Compass + minimap | PLANNED | W4.3 |
| U09 | Settings actually drive engine (sens/FOV/quality) | WORKS | ui laws (every field drives or is disabled) |
| U10 | Debug text behind toggle | PLANNED | W4.3 |

## Count

Rows: 24 verbs + 9 survival + 5 machines + 10 world + 10 shell = **58** (WORKS 29 · PARTIAL 7 · PLANNED 22). Beta 0.2 closes at >= 250 rows with WORKS >= 200 (the plan's W3.0 meter).
