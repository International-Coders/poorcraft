# Implementation Slices

## Slice 1: Evidence Bundle Types

- Define shared JSON structs or builders for capture metadata.
- Add unit tests for required fields.

## Slice 2: Screenshot And State Command

- Add one headless command that writes beauty PNG and runtime JSON.
- Prove title/new-world route can capture.

## Slice 3: Wireframe And Overlay Command

- Add wireframe, bounds, and anchor overlays.
- Pixel-check edge/anchor visibility.

## Slice 4: Asset Dump Command

- Export mesh/material/bounds/anchor JSON for one asset.
- Reject missing material ids and non-finite bounds.

## Slice 5: Input Replay

- Implement title, seed preview, Escape pause, house entry, NPC talk, and forge
  route shapes even if some routes initially fail honestly.

## Slice 6: GPU Marker Audit

- Add debug group/marker names to major render passes.
- Export marker audit JSON and optional timestamps.

## Slice 7: Regression Comparator

- Compare evidence bundles and fail when expected fields disappear or images
  become blank/clipped.
