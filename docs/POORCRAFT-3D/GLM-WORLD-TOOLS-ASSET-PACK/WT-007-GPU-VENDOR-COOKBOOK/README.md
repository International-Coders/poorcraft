# WT-007 GPU Vendor Cookbook

This sub-pack turns "AMD and NVIDIA improvements" into a measured workflow for
POORCRAFT 3D. It covers neutral wgpu markers, timestamp support, capture
scenes, AMD Radeon GPU Profiler expectations, NVIDIA Nsight GPU Trace
expectations, upscaler prerequisites, shader/debug-info requirements, and
before/after evidence rules.

No future GLM/Z-code pass may claim a vendor optimization unless it produces
the evidence described here.

## Files

- `PROMPT_TO_GLM.txt` - paste this into Z-code for WT-007.
- `00-GPU-COOKBOOK-BRIEF.md` - owner intent and scope.
- `01-WGPU-MARKER-PLAN.md` - marker/timestamp placement.
- `02-AMD-RGP-FSR-GATES.md` - AMD capture and FSR rules.
- `03-NVIDIA-NSIGHT-DLSS-GATES.md` - NVIDIA capture and DLSS rules.
- `04-CAPTURE-SCENES.md` - scenes that must be capturable.
- `05-UPSCALER-PREREQUISITES.md` - motion/depth/UI prerequisites.
- `06-PERF-EVIDENCE-RULES.md` - before/after data rules.
- `07-FAILURE-MODES.md` - ways GPU work becomes fake.
- JSON contracts for vendor toolchain, marker matrix, capture scenes,
  upscaler readiness, before/after evidence, and shader debug info.
