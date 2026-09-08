# NWR-003 — Natural Terrain Spike Prompt

```text
Implement NWR-003 only. Do not replace all terrain yet.

Create an isolated 3x3 normal-landscape test region using the surface-first
terrain contract. Generate a deterministic 16 m patch grid with seam-shared
boundaries, low-poly triangulated height/material surface, collision derived
from the same authoritative data, cached mesh data, and a sparse terrain-delta
edit command. It must look like sloped, faceted natural ground—not staircase
cubes—and it must coexist beside current construction without changing the
construction representation.

Add proof cases for: cross-patch seam continuity, terrain collision vs visible
surface, slope walking, raise/lower/level edit dirtying only local/border
patches, save/load of an edit, and bounded remesh/upload counts. Create a
windowed before/after capture with semantic checks that distinguishes the new
surface from the old cube terrain. Keep all existing visual gates green.

Do not add caves, new assets, shadows, broad terrain migration, or full fluid
simulation. Record CPU/GPU/mesh/upload evidence, follow AGENTS.md completely,
and stop after this single spike is fully proven.
```

