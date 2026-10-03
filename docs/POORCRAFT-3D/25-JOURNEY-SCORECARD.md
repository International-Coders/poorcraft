# POORCRAFT 3D — Journey Scorecard (living)

Measured: **2026-10-03** (beta battery + engine perf + mapgen + route-repair passes). Previous: 2026-09-29.
Baseline (do not erase): `23-HONEST-AUDIT.md` (2026-09-21 pre-revival).

Legend: **WORKS** / **PARTIAL** / **IMPOSSIBLE**.

Columns: Designed · Code exists · Wired to play · Fun · Proof.

## VS-Beta (V1–V6) — product bar

| Gate | Status | Designed | Code | Wired | Fun | Proof |
| --- | --- | --- | --- | --- | --- | --- |
| V1 New/Load | **WORKS** | Y | Y | Y | Y | load rebuilds scene |
| V2 Survive | **WORKS** | Y | Y | Y | Y | dig/recovery; dig route films readable captures (VIS-201) |
| V3 Shelter+prod | **WORKS** | Y | Y | Y | Y | K pack + C craft + forge; forge route = full closed loop (pick→ore→smelt→take) |
| V4 River machine | **WORKS** | Y | Y | Y | Y | M + FeedBoiler; river subject gate |
| V5 Capital | **PARTIAL** | Y | Y | Y | ~ | Talk NEEDS; city subject gate; city still procedural silhouettes (GLB kit unwired) |
| V6 Persist | **WORKS** | Y | Y | Y | Y | pack+onboarding+forge+quests |

## Full intentional beta (10 steps)

| Step | Status | VS map | Wired to play | Notes |
| --- | --- | --- | --- | --- |
| 1 Named world + first tasks | PARTIAL | V1/V2 | Y | Journal + onboarding marks |
| 2 Shelter + production | PARTIAL | V3 | Y | Pack panel K + craft/forge |
| 3 Discover river value | PARTIAL | V4 teach | Y | Machines panel at river |
| 4 Harness river | **WORKS** | V4 | Y | Was IMPOSSIBLE pre-revival |
| 5 Distinct capital | PARTIAL | V5 | ~ | Subject gate on city proof |
| 6 Residents jobs/laws | PARTIAL | V5 | Y | E shows role+activity+need |
| 7 Two deep paths | **later** | H1 | N | **next_task** |
| 8 Companion helps | later | H1 | ~ | N exists; not a VS gate |
| 9 Witnessed faction | later | H1 | ~ | Karma hooks exist |
| 10 Persist alone | **WORKS** | V6 | Y | session.bin forge+quests |

## Revival delta vs Sep 21 audit (D1–D10)

| Defect | Sep 21 | Now |
| --- | --- | --- |
| D1 world never ticks | BROKEN | **FIXED** |
| D2 forge H/T not in ui_key | BROKEN | **FIXED** |
| D3 D double-bind delivery | BROKEN | **FIXED** — delivery is V |
| D4 LOAD no scene rebuild | BROKEN | **FIXED** |
| D5 stacked panels | BROKEN | **FIXED in code**; stale dumps removed; `p3d-gate-check` OK |
| D6 craft unreachable | BROKEN | **FIXED** |
| D7 FeedBoiler unused | BROKEN | **FIXED** |
| D8 dual movement laws | open | Still two player modules |
| D9 observe honesty unused | open | Routes still `available: true` |
| D10 dual dist3d binaries | BROKEN | **FIXED** — identical release binaries + fresh DMG |

## Highest-severity open gaps (ordered)

1. H1 step 7 path choice (STATE `next_task`)
2. first_catch onboarding from a play fishing verb
3. Craft machine-part recipes (optional)
4. Re-dump playtest layout JSON on GPU host after ActivePanel exclusivity

Update this file whenever a VS gate status changes; keep `STATE.md` `next_task` in sync.
