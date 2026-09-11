# GPU Cookbook Brief

GPU work starts with observability. POORCRAFT 3D should support vendor tools,
but it must not pretend a vendor SDK is a magic fix for bad UI, broken input,
or missing gameplay.

## Scope

- neutral wgpu pass markers;
- timestamp query capability checks;
- capture scenes with deterministic seeds;
- AMD Radeon GPU Profiler/Radeon Developer Panel capture planning;
- NVIDIA Nsight Graphics GPU Trace planning;
- upscaler readiness gates for FSR/DLSS-style paths;
- before/after performance evidence.

## No-claim Rule

An optimization is not real until a bundle records:

- build hash before and after;
- GPU vendor/name;
- backend/API;
- scene, seed, viewport, quality;
- screenshots;
- marker audit;
- p50/p95 frame time;
- draw calls, triangles, material buckets;
- what changed and what did not.
