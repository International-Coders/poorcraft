# Inspector Endpoints

These endpoint names mirror WT-003 but are framed as plugin/export surfaces.

## Endpoints

- `pc3d.export.scene`;
- `pc3d.export.ui`;
- `pc3d.export.asset`;
- `pc3d.export.mesh`;
- `pc3d.export.materials`;
- `pc3d.export.player`;
- `pc3d.export.npc`;
- `pc3d.export.machine`;
- `pc3d.export.seed_preview`;
- `pc3d.export.worldgen_sample`;
- `pc3d.export.perf`;
- `pc3d.export.evidence_bundle`;
- `pc3d.validate.mod`;
- `pc3d.validate.asset_batch`;
- `pc3d.validate.evidence_bundle`.

## Endpoint Law

Every endpoint returns version, build hash, command, input args, output paths,
and verdict.
