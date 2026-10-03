# POORCRAFT 3D — Beta Test Plan

The long, defined way to test everything. One command runs the whole
battery; this document says what each stage proves, what may never be
skipped, and how a human repeats the parts a machine cannot.

- **Runner:** `make p3d-beta` (repo root). Builds the release binary,
  then executes every stage below in order, appending one
  `STAGE PASS` / `STAGE FAIL` line per stage to
  `poorcraft3d/apps/poorcraft3d/shots/BETA-REPORT.txt`.
- **Exit code:** 0 only if every stage passed. Any FAIL fails the battery.
- **Duration:** roughly 20–40 minutes on the reference host (M-series Mac,
  Metal). Most stages open a real window and drive themselves.
- **Golden rule (from 24-VERTICAL-SLICE-BETA.md):** the battery passing is
  necessary, not sufficient. A capture proves pixels and laws; the VS soak
  script below is the bar for “a stranger can play it.”

## Stage map

| # | Stage | Kind | Proves |
| --- | --- | --- | --- |
| 1 | static-assets | windowless | every beta-critical GLB/manifest row exists and fits budgets |
| 2 | asset-sidecars | windowless | anchor law: each starter asset's sockets agree with its GLB bytes |
| 3 | layout-laws | windowless | no overlapping text, one panel at a time, no unreachable hint keys, over every `*.layout.json` on disk |
| 4 | headless-smoke | windowless | the empty-world runtime stays alive 5 s with a sane frame p50/p95 and journal |
| 5 | journey-42 | windowless | all 10 journey steps pass on seed 42 (spawn→…→onboarding) |
| 6 | diagnose-2024 | windowless | all 13 system checks pass (spawn, dig, fishing, magic, companion, …) + 3 proof PNGs |
| 7 | soak-30d | windowless | 30 in-game days under a continuous command stream end inside event bounds |
| 8 | seed-atlas | windowless | biome atlas renders; patch hashes agree between the two samplers |
| 9 | terrain-bench | windowless | surface-extraction bake-off table prints sane numbers |
| 10 | terrain-analyze | windowless | per-biome slope/roughness census is physical; relief + slope PNGs |
| 11 | export-data | windowless | worldgen/NPC/machine JSON exports validate |
| 12 | windowed-3d-axes | windowed | 3D projection: face flip + parallax between two poses |
| 13 | terrain-scenes | windowed | hills/cliff/cave terrain renders with probes |
| 14 | stream-walk | windowed | streamed LOD walk holds mesh/upload/GPU-byte budgets |
| 15 | river-water | windowed | transparent river water + dam edit remeshes; river subject in frame |
| 16 | castle-city | windowed | capital + gate render; city subject in frame |
| 17 | npc-cast | windowed | NPC cast at sim positions + inspect boxes |
| 18 | day-night | windowed | noon vs midnight luminance gate (night measurably darker) |
| 19 | quality-tiers | windowed | Low/Mid/High tiers all render + record memory/frame |
| 20 | vertical-slice | windowed | showcase scene assembles city + cave + builds |
| 21 | asset-factory | windowed | tree/rock/house GLB assets draw in-window |
| 22 | surface-edit | windowed | rock→sand edit remeshes the touched section only |
| 23 | caves-water | windowed | cave interior + conforming river water + post-edit refresh |
| 24 | rebuild-slice | windowed | the played rebuild slice: vantage/street/river/gate/cave + save→reload builds survive |
| 25 | npc-people | windowed | plaza/stride/guard/anchor rig proofs + crowd budget |
| 26 | settlement-kit | windowed | settlement overview/street/wheel + socket kit |
| 27 | wilderness | windowed | instanced wilderness: vista/landmark/undergrowth/canopy/low-tier |
| 28 | materials | windowed | shadows/fog/grain/glint/cutout across tiers |
| 29 | surface-stream | windowed | streamed surface vista |
| 30 | ui-states | windowed | 13 owner-facing UI states + pixel checks + layout dumps |
| 31 | seed-preview-ui | windowed | New World seed preview states + pixel report |
| 32 | seed-preview-4242 | windowless | one seed's preview PNG + JSON sidecar |
| 33 | asset-captures | windowed | beauty/wireframe/anchor capture per GLB starter asset |
| 34 | observatory | windowed | ALL 19 observer routes (~61 PNGs, 19 evidence bundles: PNG + layout + runtime_state + perf + verdict + bundle digest), every route's hard assertions green |
| 35 | dig-determinism | windowed | the dig route run TWICE from clean dirs, evidence bundles byte-compared equal |
| 36 | playtest-determinism | windowed | the semantic playtest (house→talk→forge→fall) run TWICE + comparator equal |
| 37 | deck-bench | windowed | 240-frame scripted walk per tier (low/mid/high): frame p50/p95/p99 + CPU-contention probe inside its 1.4× band + DECK-BENCH-REPORT.md |
| 38 | package-dmg | packaging | `.app` + `hdiutil` DMG build with the git sha in the volume name |

## Evidence inventory (what “tested” means afterwards)

- `poorcraft3d/apps/poorcraft3d/shots/BETA-REPORT.txt` — the per-stage ledger.
- ~300 proof PNGs + layout JSONs under
  `poorcraft3d/apps/poorcraft3d/shots/` (observatory bundles, windowed
  captures, UI states, atlases, terrain analysis, asset captures).
- `poorcraft3d/dist3d/poorcraft3d-macos.dmg` — the packaged build the
  battery just tested.
- `docs/POORCRAFT-VALHEIM-STYLE-REBUILD/DECK-BENCH-REPORT.md` — frame
  perf per quality tier.

## The human VS soak (cannot be automated away)

Run after (or before) the battery, from the packaged DMG — this is the
stranger test from `24-VERTICAL-SLICE-BETA.md`:

1. Open `poorcraft3d/dist3d/POORCRAFT3D.app`.
2. NEW WORLD → CREATE → confirm spawn plaza + HUD vitals.
3. Dig (G) → open pack (K) → see the yield line.
4. Craft (C) or forge (E at the plaza forge; H fuel, T ore, T take).
5. Find the river → M → feed the boiler → toast/charge.
6. Talk (E) to a resident → read the NEEDS line.
7. SAVE (B / pause) → QUIT → LOAD WORLD → builds + pack + forge + quests intact.
8. **Fail the soak** if any step needs F3, a debug key, or source reading.

## Regression law

Any change to terrain generation, meshing, streaming, UI layout, or
survival rules must re-run `make p3d-beta` (or at minimum the stages
touching it) before the work is called done. Screenshots are evidence,
not decoration: if a stage's PNG stops agreeing with its probes, the
stage fails — fix the game or fix the probe, never the check.
