# NWR-002 — Asset Factory Prompt

```text
Implement NWR-002 only. Preserve every existing visual gate.

Build a source-controlled original 3D asset pipeline for pc3d_assets and
pc3d_render. Use glTF 2.0/GLB as runtime exchange format. Add a strict manifest
schema and a validator/build command. It must validate source/compiled paths,
asset ids, original/license provenance, coordinate convention, transforms,
triangle budgets, LODs, collider/nav metadata, material references, consumers,
and proof scenes. Invalid/missing entries must fail named tests.

Create exactly three small original proof assets: a stylized tree, a rock, and
a compact timber-and-stone house. Create them as reproducible Blender-Python or
repository-owned procedural source plus compiled GLB, not as renamed runtime
cubes. Add a real GLB loader and render all three in a live windowed scene with
correct scale, lighting, depth, material binding, and simple declared colliders.
Do not add downloaded or franchise-derived assets. Do not start castle/NPC/full
terrain content yet.

Prove asset validation, loading, missing-file failure, material association,
LOD selection/fallback, and a human-inspected windowed screenshot. Measure
asset memory and draw/triangle counts. Update all mandatory bookkeeping, build
runtimes, commit and push only when green. Stop after NWR-002.
```

