# Export Schemas

Use stable, plain formats first.

## JSON

Primary format for state:

- scene state;
- UI layout;
- asset manifest;
- mesh/material dump;
- player/NPC/machine state;
- seed preview;
- worldgen sample;
- performance report;
- route transcript;
- verdict.

## PNG

Primary format for visuals:

- beauty screenshot;
- UI-only screenshot;
- wireframe screenshot;
- bounds/anchor overlay;
- material/LOD overlay;
- diff heatmap.

## CSV

Allowed for frame-time series and long metric tables.

## OBJ/GLB

Allowed for debug mesh dumps. Runtime import still needs semantic sidecar,
validator report, and asset manifest entry.

## Trace/Metric/Log

Allowed as local files. Use OpenTelemetry concepts for naming and structure
only when useful; do not require an external collector by default.
