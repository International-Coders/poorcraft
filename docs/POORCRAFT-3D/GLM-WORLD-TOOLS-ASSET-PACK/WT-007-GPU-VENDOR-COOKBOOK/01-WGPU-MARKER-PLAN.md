# wgpu Marker Plan

Use wgpu debug groups and markers as the vendor-neutral base. Markers should be
stable and hierarchical so AMD RGP and NVIDIA Nsight captures can correlate
work across frames.

## Required Marker Tree

- `pc3d.frame`
  - `pc3d.frame.prepare`
  - `pc3d.pass.terrain`
  - `pc3d.pass.sky_atmosphere`
  - `pc3d.pass.water`
  - `pc3d.pass.assets`
  - `pc3d.pass.npcs`
  - `pc3d.pass.machines`
  - `pc3d.pass.wireframe_overlay`
  - `pc3d.pass.ui`
  - `pc3d.pass.screenshot_readback`

## Timestamp Policy

- Query timestamp support at startup.
- If supported, write timestamps around major passes.
- If unsupported, export `gpu_timestamp_ms: null` and `reason`.
- Always export CPU frame timings as a fallback.

## Naming Law

Names must be deterministic. Do not add frame numbers to pass marker names
unless a vendor tool explicitly needs a per-frame suffix in a side channel.
