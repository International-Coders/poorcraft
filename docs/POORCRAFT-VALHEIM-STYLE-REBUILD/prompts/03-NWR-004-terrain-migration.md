# NWR-004 — Terrain Migration Prompt

```text
Implement NWR-004 only after NWR-003 is conclusively green.

Migrate ordinary streamed natural terrain from culled cube faces to the proven
surface patch path. Preserve seed determinism, distance/interest management,
authoritative query boundaries, construction overlays, save compatibility, and
per-frame work limits. Add far/mid/near terrain LOD with seam-safe transitions;
it must not pop holes or use a full-world mesh rebuild. Keep a narrow legacy
fallback behind an explicit versioned world/renderer policy while migrations
are tested; do not silently invalidate saved worlds.

Prove multi-biome landscape readability, horizon terrain, streaming walk,
teleport recovery, editing, construction contact, save/load, LOD seams, GPU
memory caps, and low/medium/high behavior using real windowed captures and
semantic assertions. Do not begin cave volumes, city assets, or NPC art here.
```

