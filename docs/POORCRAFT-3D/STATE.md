# POORCRAFT 3D — Living State

Last updated: 2026-10-03 (beta battery + engine perf + mapgen quality + route-repair passes)

Root `STATE.md` / `BACKLOG.md` / `CHANGELOG.md` track **LOREFORGE only**.
This file is the living truth for `poorcraft3d/` work.

## Loop

| Field | Value |
| --- | --- |
| Horizon | **H0 Vertical-Slice Beta** (see `24-VERTICAL-SLICE-BETA.md`) |
| Build | libs green (pc3d_world 277, pc3d_render 255); `make p3d-beta` = the full beta battery (38 stages, one report) |
| Last measure | 2026-10-03 (branch beta0.2-wide-world) — W1.1 dials UP (rings 192/640/2048 m, climate ×2, layout v2; suites green, deck p50 6.62 ms flat); W1.2 far horizon (slab dead, daynight 49% held); W1.3 realm map (6 D-026 capitals, planner-true); W3.1 career fork CLOSED (Engineering/Mysteries, choose-once, gated recipes, mana at Mysteries, route PASS); world 289 / render 256 / save 20 green; 20-route observatory |
| `next_task` | **W2.3 — wire the six faction kits into city.rs (the V5 closer)** (Beta 0.2 plan; W3.1 the career fork CLOSED — V21/W10 WORKS; far horizon W07 WORKS; realm map W11 WORKS; next: W1.2 impostor plates, W3.2 machines, W4.1 UI catalog) |

## VS-Beta gates (V1–V6) — current

| # | Gate | Status | Notes |
| --- | --- | --- | --- |
| V1 | New World + Load World | **WORKS** | Scene rebuild on load |
| V2 | Survive | **WORKS** | Dig/build/night onboarding; dig route films readable captures (VIS-201) |
| V3 | Shelter + production | **WORKS** | K pack + C craft + forge; forge route proves the full closed loop (chest pick → ore node → smelt → take) |
| V4 | River → machine | **WORKS** | M + FeedBoiler + toast; machine_chain route charges from the city river |
| V5 | One capital | **PARTIAL** | Talk shows NEEDS; city reads as a place but draws procedural silhouettes (GLB kit un wired to city.rs) |
| V6 | Persist alone | **WORKS** | Pose + builds + pack + onboarding + forge + quests (`session.bin`) |

## Beta test procedure

`make p3d-beta` — the long, defined way to test everything
(38 stages: static truth → headless determinism → 22 windowed captures →
observatory → determinism pairs → deck bench → DMG). Documented in
`BETA-TEST-PLAN.md`; the per-stage ledger lands in
`poorcraft3d/apps/poorcraft3d/shots/BETA-REPORT.txt`.

## Engine + world facts (measured 2026-10-03)

- `pc3d_world::cache::GenCache` memoizes the generator (value-identity law
  tested; ocean provably invariant). Streaming surface mesh: cold 622 →
  ~330 µs/patch, warm steady state 45-58 µs/patch.
- Mapgen: warped 4-octave fbm + ridged mountain lift; per-column ground
  biome (border dither + altitude snow); scenes/tests SEEK their ground.
- Worldgen cost honestly higher: regenerate 288 → 462 µs/patch.

## Blockers

- None blocking VS. Named deferrals: rivers never carve the height
  function (water strips can meet slopes awkwardly — the RiverCarve
  design is queued); city GLB kit still unwired; far-view reads as a
  floating slab at ring edge (fog/sky mismatch).

## H1 queue (after VS)

1. Path choice (journey step 7)
2. Companion helps in the world (step 8)
3. Witnessed faction effect (step 9)
4. Two-client authoritative co-op
5. RiverCarve: hydrology-carved valleys under the water strips
