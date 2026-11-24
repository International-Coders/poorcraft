# POORCRAFT 3D — Living State

Last updated: 2026-11-23 (N06 owned hotbar closed: wear/break/selection one authority)

Root `STATE.md` / `BACKLOG.md` / `CHANGELOG.md` are now the split repository's
control plane and preserve the LOREFORGE history. This file is the living truth
for `Poorcraft 3D/` work (folder renamed from `poorcraft-novo/` on 2026-10-10).

## Loop

| Field | Value |
| --- | --- |
| Horizon | **H0 Vertical-Slice Beta** (see `24-VERTICAL-SLICE-BETA.md`) |
| Build | **817/817 tests green** (assets 38, audio 8, core 32, render 270, save 38, world 431); `make p3d-beta` **38/38** (loop-479 baseline), 339 PNGs, one report |
| Last measure | 2026-11-23 — N06 owned hotbar closed: `ToolWear` durability (break → empty hand + toast), owned-selection law (empty slots refuse/skip), wear save trailer — loop 482; `route_hotbar_wear` PASS with mutation RED proof; doctrine promoted to N07 |
| `next_task` | **N07 — ActionResolver mouse-first** from `Poorcraft 3D/docs/CONTINUACAO-GLM/14-PLANO-N06-N11-PRIMEIRA-HORA.md`: Left/right/E/Tab replace the letter-key system verbs with the binding map's prompts |

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
`Poorcraft 3D/apps/poorcraft3d/shots/BETA-REPORT.txt`.

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
