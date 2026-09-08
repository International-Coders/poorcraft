# NWR-011 — Rebuild slice manual review checklist

Run `poorcraft3d --play-rebuild live` (or `make p3d-rebuild` for the
automated route captures) and check each row. Automated evidence:
`--play-rebuild` captures + probes, `make p3d-deck-bench` tiers.

| # | check | where |
|---|---|---|
| 1 | Distant vantage: town + keep + banner readable in haze, no seams | vantage capture |
| 2 | Player height: trees/rocks/grass around, ground feels solid | gate/street captures |
| 3 | River + water wheel visible; banks occlude water | river capture |
| 4 | Cave mouth dark interior, no daylight leak | cave capture |
| 5 | People: roles readable (guard spear/helm, worker tool), they move | street capture / live |
| 6 | F on flat ground builds; on steep ground prints FOUNDATION REJECTED + reason | live |
| 7 | B then L: buildings and player position return | live |
| 8 | I shows Bed/Work/Idle frames | live |
| 9 | WASD walks the SURFACE (feels underfoot, edits visible later) | live |
| 10 | Frame pacing stays smooth at each deck tier | deck bench |

OWNER GATE: the rebuild slice is COMPLETE when a human owner passes this
checklist in person; until then it is not described as a commercial
beta release.
