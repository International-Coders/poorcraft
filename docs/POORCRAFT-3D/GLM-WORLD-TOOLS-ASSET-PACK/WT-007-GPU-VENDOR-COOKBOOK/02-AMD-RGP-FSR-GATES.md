# AMD RGP And FSR Gates

Use AMD tooling for measured captures, not slogans.

## AMD Capture Path

- Radeon Developer Panel launches/captures supported explicit API workloads.
- Radeon GPU Profiler analyzes frame captures.
- Markers should name terrain, assets, UI, overlays, readback, and post passes.
- Capture bundles must include scene seed, viewport, quality, and build hash.

## What To Inspect

- barrier/synchronization cost;
- pass durations;
- draw/dispatch counts;
- occupancy or cache behavior where available;
- expensive readbacks;
- UI pass visibility;
- overdraw or material bucket inflation.

## FSR Rules

FSR-style upscaling is allowed only after:

- render resolution and output resolution are explicit;
- depth is correct;
- motion vectors exist and are validated;
- jitter is controlled;
- transparency/reactive/composition needs are documented;
- UI is composed at the right stage;
- frame pacing is measured.

Do not use upscaling to hide broken rendering or unreadable UI.
