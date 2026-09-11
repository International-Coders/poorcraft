# GPU Markers, Timestamps, And Captures

WT-003 starts with neutral instrumentation. Vendor tools come after pass names
and frame data exist.

## wgpu Instrumentation

Use wgpu debug groups and markers around major passes where supported:

- frame;
- terrain;
- sky/atmosphere;
- water;
- asset instances;
- NPCs;
- machines;
- wireframe overlays;
- UI;
- screenshot readback.

Use timestamp queries only when supported by the adapter/features. If not
supported, export `null` with the reason and keep CPU-side timing.

## AMD Capture Notes

AMD Radeon GPU Profiler and Radeon Developer Panel are the target path for
Radeon captures. RGP captures explicit graphics API work and benefits from
named events/markers, stable scenes, and clear before/after performance data.
FSR integration belongs after depth, motion vectors, reactive/transparency
handling, UI composition, and frame pacing are proven.

## NVIDIA Capture Notes

NVIDIA Nsight Graphics GPU Trace is the target path for NVIDIA profiling:
stable markers, queue/timing data, hardware unit utilization, draw/event names,
and shader/pipeline debug info are needed for useful captures. Nsight Aftermath
is the crash-dump path for GPU exceptions on supported D3D12/Vulkan flows.
Streamline/DLSS work is a later integration spike and must be gated by API
support, motion vectors, depth, jitter, UI composition, and frame pacing.

## No-Claim Rule

No AMD/NVIDIA improvement may be claimed unless before/after evidence names:

- build hashes;
- GPU vendor/name;
- API backend;
- scene and seed;
- viewport and quality;
- markers present;
- p50/p95 frame time;
- draw calls, triangles, material buckets;
- screenshot digest.
