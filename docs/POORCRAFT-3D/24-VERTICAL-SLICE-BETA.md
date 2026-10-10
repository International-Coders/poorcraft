# POORCRAFT 3D — Vertical-Slice Beta Contract

**Now:** a stranger finishes one focused solo session without debug keys or reading source.
**Not now:** companion recruitment as a gate, karma/faction witnessing, magic/ley careers, multiplayer/Steam, nuclear, dragons-as-campaign, empire depth.

Maps to a tightened reading of journey steps 1–6 + 10 in `02-DESIGN-PILLARS-AND-PROGRESSION.md` (defer 7–9 to Full Beta / H1).

## V1–V6 gates

| # | Must work in the played game | Done when |
| --- | --- | --- |
| V1 | New World + Load World | Correct seed/scene; load rebuilds terrain |
| V2 | Survive | Health/stamina/food matter; dig yields go into a readable pack; death/recovery clear |
| V3 | Shelter + production | Build/remove; **inventory + craft** produce a tool or machine part the player uses |
| V4 | River value | Find a river; one machine clearly consumes flow and produces a local benefit |
| V5 | One capital | Settlement reads as a place; talk shows job/role without F3-only boxes as the only signal |
| V6 | Persist alone | Save → quit → load → same builds, inventory, and world state |

## Explicit non-goals (VS)

- Companion as a required journey gate
- Witnessed faction / karma chain
- Magic / ley / valve / nuclear as selectable careers
- Multiplayer, Steam productization, 128-player scale
- Creature catalog expansion, dragons as campaign

## Fun bar — first 20 minutes

1. Title → New World (or Play) → spawn in a real place (not empty void).
2. See vitals move; dig or open a chest; **see the pack change in a readable UI**.
3. Craft or forge something that unlocks the next verb (pick → dig stone, or feed a machine).
4. Find water; open machines (M); feed boiler; see charge/toast.
5. Walk to the capital; Talk (E); read a job/need in player language.
6. Save (B / menu); quit; Load; still have builds + pack.

## Proof commands

| Gate | Proof |
| --- | --- |
| V1 | Manual New/Load; or journey/rebuild routes that call `load_world` |
| V2 | `make p3d-dig`, `make p3d-recovery`; HUD vitals in live play |
| V3 | Craft (C) + pack panel; forge H/T via `ui_key` reachability tests |
| V4 | Machines panel + FeedBoiler; machine toast; water proof |
| V5 | `make p3d-city` / settlement + Talk in live; subject_in_frame on city proof |
| V6 | Save/load round-trip test (construction + inventory); menu Save/Load |

Also: `make p3d-gate-check` (layout exclusivity), `make p3d-observe` (GPU routes).

## VS soak / playtest script (Phase 4)

1. Open `poorcraft-novo/dist3d/POORCRAFT3D.app` (or DMG volume `POORCRAFT3D-<git>`).
2. NEW WORLD → CREATE → confirm spawn plaza + HUD vitals.
3. Dig (G) → open pack (K) → see yield; craft (C) or forge (E near plaza).
4. Find river → M → feed boiler → toast/charge.
5. Talk (E) to a resident → read NEEDS line.
6. SAVE (B / pause) → QUIT TO TITLE → LOAD WORLD → pack + builds + forge/quests intact.
7. Fail the soak if any step needs F3, debug keys, or source reading.

## Breadth safeguard

No nuclear / armies / Steam productization while any of V1–V6 fail.
Constitution + D-022 pillars stay; VS is a **subset**, not a rewrite of identity.
