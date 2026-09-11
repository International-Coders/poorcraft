# Tool Commands To Build

These are the local tools GLM should add or extend.

## Asset Factory

- `--asset-factory <batch.json> <outdir>`: generate original assets and
  manifests.
- `--asset-register <manifest.json>`: add accepted assets to runtime catalogs.
- `--asset-validate <manifest.json>`: reject missing affordances or proof.
- `--asset-diff <old.json> <new.json>`: show what changed in geometry,
  materials, anchors, collision, nav, and gameplay affordances.

## Inspection

- `--asset-inspect <asset_id> <outdir>`: beauty, wireframe, bounds, anchors,
  material, LOD, perf JSON.
- `--asset-batch-inspect <manifest.json> <outdir>`: same for every asset.
- `--scene-inspect <scene> <outdir>`: camera route, visible assets, culling,
  frame timing, input state, UI layout.
- `--wireframe-scene <scene> <outdir>`: wireframe render with depth.

## Playtest

- `--semantic-playtest <route.json> <outdir>`: scripted walk/use/talk/forge.
- `--interaction-probe <asset_id> <anchor_id>`: prove an anchor reaches a
  runtime action.
- `--nav-probe <asset_id>`: prove player/NPC approach and interior nodes.

## GPU

- `--gpu-markers-audit <scene>`: verify all major passes emit stable markers.
- `--perf-capture <scene> <outdir>`: frame timing, draw calls, instances,
  triangles, material buckets, screenshots.
- `--vendor-capture-plan amd|nvidia <scene> <outdir>`: write capture steps and
  expected marker names for RGP or Nsight.
