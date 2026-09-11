# Telemetry, Traces, Metrics, And Logs

Telemetry can help GLM understand cause and effect, but it must stay simple
and local first.

## Signals

- traces: route or frame operations with nested spans;
- metrics: frame time, draw calls, triangles, memory, chunk count, asset count;
- logs: important events, warnings, validation failures, route failures.

## Naming

Use stable names:

- `pc3d.route.seed_preview`;
- `pc3d.ui.click`;
- `pc3d.asset.inspect`;
- `pc3d.worldgen.preview`;
- `pc3d.machine.forge`;
- `pc3d.gpu.pass.terrain`;
- `pc3d.export.bundle`.

## Local-first Rule

No external telemetry backend is required. A local JSONL or OTLP-like file is
enough for first implementation. External exporters require explicit owner
approval.
