# Roadmap and Quality Gates

The master task order is in `contracts/rebuild_roadmap.json`. This is the human
overview; GLM must follow both.

| Milestone | Outcome | Required visible proof |
| --- | --- | --- |
| NWR-001 | Baseline audit and safety net | Existing gates pass; new baseline report and screenshot inventory |
| NWR-002 | Asset factory | One original tree, rock, and house GLB validate, load, and render |
| NWR-003 | Natural terrain spike | One 3x3 terrain area is visibly sloped/faceted, collidable, editable, seam-free |
| NWR-004 | Terrain migration | Streamed normal terrain uses the new surface path; construction remains correct |
| NWR-005 | Caves, water, foundations | Cave boundary, terrain-conforming river, wheel, build support proof |
| NWR-006 | Materials and atmosphere | Original material palette, shadows, fog, water and foliage readability |
| NWR-007 | Wilderness assets | Instanced biome props with LOD, collision, perf evidence |
| NWR-008 | Settlement assets | Original castle/town modules replace procedural primitives |
| NWR-009 | NPC presentation | Rig-ready original NPCs with roles and simple animation state |
| NWR-010 | Steam Deck quality | Low/med/high tiers, budgets, gameplay clarity and deck-oriented report |
| NWR-011 | Rebuild vertical slice | Human-walkable natural-world showcase, asset and terrain proofs |

## Gate rules

Every milestone needs: focused tests, old visual regression battery, new
semantic image/readback assertions, at least one human-inspected real windowed
capture, a save/load outcome where state changed, and a bounded-performance
record. If a rendering test catches a defect, repair the defect before adding
scope. A milestone cannot be waved through by an attractive concept image.

## Git and work discipline

Keep the rebuild in the existing `poorcraft3d/` workspace so the baseline and
new renderer share tests. Use small commits. Do not make docs-only commits;
every implementation commit must ship working code and tests as `AGENTS.md`
requires. Preserve unrelated user changes. Push only after the complete required
verification and runtime packaging succeeds.

