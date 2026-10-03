# POORCRAFT 3D — System Gap Matrix

Priority: **P0** = blocks VS fun · **P1** = VS polish · **P2** = Full Beta · **later** = H2 ambition.
Call status: from the **played** path, not crate existence.
Updated: 2026-10-03 after the beta battery + engine perf + mapgen + route-repair passes.

| System | File anchors | Play call status | VS pri | Gap |
| --- | --- | --- | --- | --- |
| Terrain / streaming | `pc3d_world` gen, `surface_stream`, `cache` | Wired | P1 | LOD/art (far view slab edge); warm mesh 10.7–13.8× (PERF-101); rivers never carve the height fn (RiverCarve queued) |
| Water / rivers | `hydro`, `water` render | Wired | P1 | Subject gate on `--play-water`; spawn-band rivers restored on seed 3 |
| Dig / build | `app.rs`, HostCommand Build | Wired | P1 | Onboarding marks tree/build |
| Inventory / pack | items, **K pack panel** | Wired | P1 | Select/use beyond eat still thin |
| Craft | `craft.rs`, C key | Wired | P1 | No machine-part recipes (ok) |
| Forge | `forge`, H/T via `ui_key` | Wired | P1 | **Persists** in `session.bin` |
| Machines / boiler | FeedBoiler, M panel | Wired | P1 | Host machines not in save |
| Capital / settlement | city, oversight O | Partial | P1 | City subject gate present |
| NPC Talk | `dialog.rs`, E | Wired | P1 | Needs shown; laws still thin |
| Combat | P melee, creatures | Optional | P2 | Catalog thin — fine for VS |
| Companion | N key | Partial | later | Exists; not VS gate |
| Magic / ley / valve | world crates | Not play career | later | H1/H2 |
| Factions / karma | faction, perception | Thin | later | Witness chain H1 |
| Quests / journal | J panel | Wired | P1 | **Persists** in `session.bin` |
| Survival onboarding | `Onboarding` | Partial | P1 | tree/build/night; catch not |
| Save / load | player_store + session_store | Wired | P1 | V6 WORKS |
| Net / Steam | docs 09 | Out of VS | later | H2 |
| Visual gates | `subject_in_frame`, gate-check, `make p3d-beta` | Wired | P1 | 38-stage battery; observatory 19/19 PASS (forge/social/dig repaired VIS-201/202) |
| Packaging | `dist3d`, PLAY.md, DMG | Wired | P1 | D10 closed (identical bins + DMG) |

## Remaining work order

1. H1 step 7 path choice (STATE `next_task`)
2. Optional VS polish: first_catch, machine-part recipes
3. GPU host: regenerate playtest layout dumps; full visual battery
