# GPU Profiling And Upscaling Gates

This file translates current official AMD/NVIDIA guidance into POORCRAFT 3D
rules. It is not a promise to integrate every vendor SDK immediately. It is a
gate: no "AMD/NVIDIA optimization" claim without data.

## Official Sources To Check

- AMD Radeon GPU Profiler manual:
  https://gpuopen.com/manuals/rgp_manual/
- AMD Radeon GPU Profiler product page:
  https://gpuopen.com/rgp/
- AMD FidelityFX Super Resolution 3:
  https://gpuopen.com/fidelityfx-super-resolution-3/
- NVIDIA Nsight Graphics:
  https://developer.nvidia.com/nsight-graphics
- NVIDIA Nsight GPU Trace documentation:
  https://developer.nvidia.com/docs/drive/drive-os/7.0.3/public/nsight/nsight-graphics/AdvancedLearning/index.html
- NVIDIA Streamline:
  https://developer.nvidia.com/rtx/streamline
- NVIDIA DLSS:
  https://developer.nvidia.com/rtx/dlss

## AMD-Informed Gates

AMD RGP is aimed at low-level profiling for DirectX 12, Vulkan, OpenCL, and HIP
applications on Radeon hardware. For POORCRAFT 3D this means GLM must:

- add GPU debug markers around terrain, UI, assets, wireframe, shadows,
  post-processing, and screenshots;
- collect queue, barrier, draw, shader, and cache evidence before optimizing;
- keep captures tied to exact build hash, scene seed, viewport, and quality;
- prefer Vulkan-ready markers and data because wgpu can target Vulkan.

FSR work must start as an optional upscaler path, not a default fix for bad
rendering. FSR 3 frame generation needs correct motion vectors, depth,
reactive/transparency/composition handling, and UI composition. It should be
deferred until the engine has stable temporal inputs.

## NVIDIA-Informed Gates

Nsight Graphics can debug/profile/export frames for Direct3D, Vulkan, OpenGL,
and OpenXR and can inspect events down to pixels. GPU Trace is useful for
hardware-unit utilization, timing, and bottlenecks. For POORCRAFT 3D this
means GLM must:

- add named markers and stable capture scenes;
- compile shaders with debug info when shader-level inspection is needed;
- export per-pass timing and resource names;
- never call a vendor optimization successful without before/after captures.

DLSS/Streamline work is optional and later. Streamline supports DirectX 11/12
today while Vulkan support depends on current vendor state, so do not promise
DLSS in the current wgpu/Vulkan path without a fresh integration spike. DLSS
also depends on correct motion vectors, depth, jitter, mip bias, and placement
early in post-processing.

## POORCRAFT Rule

First build neutral measurement tools. Then add vendor-specific profiles. Then
add upscalers only after motion vectors, depth, UI composition, jitter, and
frame pacing can be proven.
