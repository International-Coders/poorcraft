# Screenshot, Wireframe, And Overlays

WT-003 requires visual modes that prove what the player would see and what the
engine believes is present.

## Capture Modes

- `beauty`: ordinary game view;
- `ui_only`: UI canvas over transparent or flat background;
- `wireframe`: visible mesh edges, depth-tested where possible;
- `bounds`: asset bounds, collision, nav blockers, door clearances;
- `anchors`: interaction anchors and labels;
- `materials`: material id colors and atlas/texture names;
- `lod`: LOD level colors and switch distances;
- `perf`: pass names and draw/instance counts;
- `comparison`: before/after side-by-side with diff heatmap.

## Pixel Gates

- screenshot is nonblank;
- UI controls are not clipped or overlapping;
- wireframe has enough edge pixels;
- required anchors are visible and labeled;
- collision/bounds overlay surrounds the expected asset;
- diff heatmap is present when comparing;
- captured viewport matches the sidecar;
- image build hash/scene label matches JSON.

## Evidence Naming

Use deterministic names:

`<route>__<scene>__<seed>__<viewport>__<mode>.<png|json>`

The same command with same build and same seed should overwrite or produce an
identical digest unless explicitly told to keep history.
