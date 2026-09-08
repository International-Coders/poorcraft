# NWR-010 — Steam-Deck Quality Prompt

```text
Implement NWR-010 only. Audit and tune the completed natural terrain, asset,
lighting, foliage, city, NPC, and water paths for Steam Deck-oriented settings.
Create transparent Low/Medium/High configuration contracts. Low must preserve
gameplay readability and essential interaction visibility; it may reduce terrain
distance, shadow resolution/distance, foliage density, animation update rate,
and internal render scale. It may not swap in a different game world.

Add a reproducible benchmark walk and a documented report containing hardware,
resolution, frame-time percentiles, CPU/GPU time when obtainable, mesh/upload
work, draw calls, triangles, texture/VRAM usage, and visible quality settings.
Find and fix actual bottlenecks before adding arbitrary optimizations. Prove
all quality settings still pass core semantic visual gates.
```

