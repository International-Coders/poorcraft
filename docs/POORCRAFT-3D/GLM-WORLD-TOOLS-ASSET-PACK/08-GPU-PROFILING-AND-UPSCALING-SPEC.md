# GPU Profiling And Upscaling Spec

POORCRAFT 3D should be ready for AMD/NVIDIA performance work without turning the
engine into a vendor-locked mess.

## Phase 1: Vendor-Neutral Instrumentation

- Add named GPU/CPU pass markers.
- Export frame stats JSON.
- Export pass timings when available.
- Export draw counts, instance counts, triangle counts.
- Dump UI draw lists and text bounds.
- Dump visible world object counts.
- Add screenshot sidecar metadata.

## Phase 2: AMD Workflow

- Add capture instructions for Radeon Developer Panel CLI.
- Name markers for RGP readability.
- Add RGP capture checklist to docs.
- Add shader/pipeline labels where the API permits.
- Investigate Radeon GPU Analyzer for shader assembly/perf inspection.
- Investigate Radeon Memory Visualizer if memory pressure becomes a problem.

## Phase 3: NVIDIA Workflow

- Add marker strings suitable for Nsight Graphics and Aftermath.
- Add crash dump integration plan for future D3D12/Vulkan backend.
- Track shader binary/source paths for crash inspection.
- Investigate Streamline only after the renderer has stable resource lifetime
  and platform abstractions.

## Phase 4: Upscaling / Frame Generation

Do not start with frame generation. Start with architecture:

- render scene without UI;
- render UI alpha/color separately;
- composite final UI;
- expose pause/menu/loading state;
- expose motion vectors/depth only when real;
- add UI scale and accessibility controls.

Frame generation must be disabled in menus, loading screens, scene transitions,
and paused states until proven safe. UI must remain readable in native,
upscaled, and generated-frame modes.
