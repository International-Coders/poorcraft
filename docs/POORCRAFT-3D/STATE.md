# POORCRAFT 3D — Living State

Last updated: 2026-09-29 (VS Phase 1–4 close + subject gates / session persist / packaging)

Root `STATE.md` / `BACKLOG.md` / `CHANGELOG.md` track **LOREFORGE only**.
This file is the living truth for `poorcraft3d/` work.

## Loop

| Field | Value |
| --- | --- |
| Horizon | **H0 Vertical-Slice Beta** (see `24-VERTICAL-SLICE-BETA.md`) |
| Build | libs green; `make p3d-gate-check` **OK** (142 dumps); release binary in both `dist3d` trees; DMG built |
| Last measure | 2026-09-29 — played: dig/water/city/rebuild/playtest/people/gate/journey PASS; fresh DMG; Makefile syncs CARGO_TARGET_DIR → poorcraft3d/target |
| `next_task` | **H1 step 7 — path choice (technology / magic / exploration / political) as a player-facing fork** |

## VS-Beta gates (V1–V6) — current

| # | Gate | Status | Notes |
| --- | --- | --- | --- |
| V1 | New World + Load World | **WORKS** | Scene rebuild on load |
| V2 | Survive | **PARTIAL** | Dig/build/night onboarding; first_catch still unused |
| V3 | Shelter + production | **PARTIAL** | K pack + C craft + forge; machine-part recipes optional |
| V4 | River → machine | **WORKS** | M + FeedBoiler + toast; river `subject_in_frame` on `--play-water` |
| V5 | One capital | **PARTIAL** | Talk shows NEEDS; city subject gate on `--play-city` |
| V6 | Persist alone | **WORKS** | Pose + builds + pack + onboarding + **forge + quests** (`session.bin`) |

## Blockers

- GPU windowed battery / observatory need host GPU (sandbox has no adapter).
- Re-run playtest routes to regenerate exclusive layout dumps (PNGs remain; stale stacked JSON deleted).
- Dual movement laws (render vs world) still open (D8).

## Proof commands (P3D)

```bash
cd poorcraft3d && cargo test -p pc3d_world -p pc3d_save -p pc3d_core --lib
cargo test -p pc3d_save --lib session_store
cargo test -p pc3d_render --lib slice::tests::slice_save_reload
make -C .. p3d-gate-check   # GATE CHECK OK
# GPU host:
make -C .. p3d-visual-gates
make -C .. p3d-dmg
```

## H1 queue (after VS)

1. Path choice (journey step 7)
2. Companion helps in the world (step 8)
3. Witnessed faction effect (step 9)
4. Two-client authoritative co-op
