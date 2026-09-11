# Security And Scope

The observatory is local developer tooling. Keep it boring and safe.

## Allowed

- local CLI commands;
- local MCP-style endpoints bound to loopback;
- deterministic JSON/PNG/OBJ/GLB exports;
- temporary debug overlays behind developer flags;
- build-hash and scene metadata in files.

## Not Allowed

- network listening on public interfaces;
- uploading captures without explicit owner request;
- reading unrelated user files;
- storing secrets in sidecars;
- changing gameplay outcomes just to make a route pass;
- disabling collision/input/UI to fake success.

## File Hygiene

Write under explicit output directories such as:

- `poorcraft3d/apps/poorcraft3d/shots/observatory/`;
- `poorcraft3d/target/observatory/`;
- `docs/POORCRAFT-3D/GLM-WORLD-TOOLS-ASSET-PACK/evidence/`.

Never write into player save worlds during tests unless the test owns a temp
world.
