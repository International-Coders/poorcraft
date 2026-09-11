# Performance Evidence Rules

Performance claims need comparable data.

## Required Metrics

- build hash;
- GPU vendor and adapter name;
- backend/API;
- scene id;
- seed;
- viewport;
- quality;
- frame count;
- p50, p95, p99 frame time;
- draw calls;
- instances;
- triangles;
- material buckets;
- visible chunks/assets/NPCs;
- marker list;
- screenshot digest.

## Comparison Rules

- compare same scene, seed, viewport, quality, and route;
- keep before/after JSON;
- fail if visual or interaction gates regress;
- report neutral or negative results honestly;
- never average across different content and call it a win.
