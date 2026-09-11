# NVIDIA Nsight And DLSS Gates

Use NVIDIA tooling for evidence, not as a label.

## Nsight GPU Trace

GPU Trace can show frame timelines, per-queue events, metric graphs, user
markers, synchronization objects, action rows, hardware utilization, and shader
profiling views. POORCRAFT must emit stable markers so traces can be compared.

## Shader Profiling

Shader profiling needs compatible hardware and shader pipeline/debug
information. When shader-level optimization is claimed, export:

- shader/pipeline name;
- pass marker;
- source/debug info status;
- bottleneck note;
- before/after evidence.

## DLSS/Streamline Rules

DLSS-style integration is a later spike. It must not be promised in the current
wgpu path until API/backend support is proven for the target build. Readiness
requires:

- depth;
- motion vectors;
- jitter;
- exposure if needed;
- UI composition plan;
- frame pacing;
- screenshots and metrics.
