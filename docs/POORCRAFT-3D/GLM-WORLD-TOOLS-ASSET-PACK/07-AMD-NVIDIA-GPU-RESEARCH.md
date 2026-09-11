# AMD And NVIDIA GPU Research

This file summarizes official-source research for future GPU tooling and
upscaling/profiling work. It is not permission to add vendor SDKs blindly; it is
a plan for instrumentation and optional platform-specific integrations.

## AMD Sources

- AMD Radeon GPU Profiler documentation:
  `https://gpuopen.com/manuals/rgp_manual/`
- Radeon GPU Profiler quick start:
  `https://gpuopen.com/manuals/rgp_manual/quickstart/`
- AMD GPUOpen RGP product page:
  `https://gpuopen.com/rgp/`
- AMD FidelityFX Super Resolution 3:
  `https://gpuopen.com/fidelityfx-super-resolution-3/`
- FidelityFX SDK FSR documentation:
  `https://gpuopen.com/manuals/fidelityfx_sdk/techniques/super-resolution-interpolation/`

Relevant implications:

- RGP is the AMD frame/profile viewer for DirectX 12, Vulkan, OpenCL, and HIP
  applications on RDNA hardware.
- AMD's workflow uses Radeon Developer Panel / CLI to generate captures and RGP
  to inspect profiles.
- RGP exposes queues, barriers, event timing, wavefront occupancy, instruction
  timing, pipeline state, and user debug markers.
- POORCRAFT 3D should emit clear debug marker regions: world, UI, terrain,
  water, settlement, NPCs, postprocess, upload, readback.
- FSR 3 requires explicit UI handling. AMD documents multiple UI composition
  options; future integration must preserve UI/HUD readability and should not
  let generated/interpolated frames distort menus.

## NVIDIA Sources

- NVIDIA Nsight Aftermath SDK:
  `https://developer.nvidia.com/nsight-aftermath`
- Nsight Aftermath getting started:
  `https://developer.nvidia.com/nsight-aftermath/getting-started`
- NVIDIA Streamline DLSS programming guide:
  `https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideDLSS.md`
- NVIDIA Streamline DLSS Frame Generation guide:
  `https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideDLSS_G.md`
- NVIDIA DLSS 4 Streamline integration article:
  `https://developer.nvidia.com/blog/how-to-integrate-nvidia-dlss-4-into-your-game-with-nvidia-streamline/`

Relevant implications:

- Nsight Aftermath can create GPU mini-dumps for D3D12/Vulkan GPU exceptions or
  TDR-style failures, with marker and pipeline context.
- POORCRAFT 3D should design crash marker strings now, even if full Aftermath
  integration is platform-gated later.
- Streamline/DLSS integration requires correct resource tagging and support
  checks.
- NVIDIA guidance for frame generation calls out HUD-less and UI color/alpha
  buffers, disabling frame generation in menus/loading/paused states, and UI
  recomposition considerations.

## Immediate Vendor-Neutral Work

Before adding AMD/NVIDIA SDKs, GLM should build:

- GPU marker naming convention.
- Frame stats JSON.
- Capture metadata sidecars.
- UI alpha/color buffer concept in renderer architecture.
- Pause/menu/upscaler-disable state tracking.
- Wireframe/mesh dump for visual debugging.
- Per-pass timings and draw counts.

These help both AMD and NVIDIA workflows and are safe on the current wgpu path.
