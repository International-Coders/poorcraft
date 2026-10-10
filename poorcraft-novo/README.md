# POORCRAFT 3D

POORCRAFT 3D is the current Rust game and engine workspace. It is the
Minecraft/Valheim-style survival sandbox with Skyrim-like role-playing and
Heroes-like realms, settlements, and long-term empire play.

The repository root contains the older LOREFORGE/POORCRAFT codebase under
`../poorcraft-antigo/`. It is preserved as history and is not the
implementation target for POORCRAFT 3D work.

## Start here

- Mandatory current GLM/QA doctrine:
  `docs/CONTINUACAO-GLM/README.md`
- Workspace-specific agent rules: `AGENTS.md`
- Current product and implementation plan:
  `../docs/POORCRAFT-3D/CURRENT-PLAN.md`
- Current measured state: `../docs/POORCRAFT-3D/STATE.md`
- POORCRAFT 3D development log: `../docs/POORCRAFT-3D/DEVLOG.md`
- Design decisions that remain binding:
  `../docs/POORCRAFT-3D/18-DECISION-REGISTER.md`
- Historical Beta 0.2 expansion plan:
  `../docs/POORCRAFT-3D/28-BETA-0.2-PLAN.md`

The active priority is the playability reset: fix the spawn and river, replace
letter-per-action controls with selected tools and mouse actions, make the
first crafting loop obvious, and add a proper developer console. Asset and
realm expansion resumes after the first-session gates are green.

## Workspace map

| Path | Purpose | Keep in Git? |
| --- | --- | --- |
| `apps/poorcraft3d/` | Executable and deterministic screenshot routes | Source: yes; generated route output: curated only |
| `crates/pc3d_core/` | Identity, clocks, commands, replay, format headers | Yes |
| `crates/pc3d_world/` | Terrain, water, items, crafting, NPCs, cities, progression | Yes |
| `crates/pc3d_render/` | Renderer, player input, UI, streaming, scene assembly | Yes |
| `crates/pc3d_save/` | Save/session persistence | Yes |
| `crates/pc3d_assets/` | Asset catalogs and validators | Yes |
| `assets/compiled/` | Runtime GLB assets | Yes when consumed by the game |
| `tools/assetgen/` | Deterministic asset generation | Yes |
| `dist3d/` | Test application, DMG, portable package | Curated release artifacts only |
| `target/` | Cargo build cache | No; regenerate with Cargo |
| `saves3d/` | Local player worlds/settings | No |
| `shots/` and `apps/poorcraft3d/shots/` | Visual evidence | Keep named release baselines; do not use as a scratch directory |

## Common commands

Run these from the repository root (`POORCRAFT/`):

```sh
make p3d-test
make p3d-doctrine-check
make p3d-rebuild OUTDIR=/tmp/p3d-rebuild
make p3d-water OUTDIR=/tmp/p3d-water
make p3d-ui-shots OUTDIR=/tmp/p3d-ui
make p3d-rebuild-live
make p3d-dmg
make p3d-clean
```

`make p3d-clean` removes only regenerable Cargo output. It does not touch
source, saves, screenshots, assets, or release packages.

## Working rule

One work card at a time. Each completed card must leave the workspace tests
green, produce the named runtime evidence, update `STATE.md`, and append a
concise entry to the monthly POORCRAFT 3D development log. A test that passes
only because it checks pixels or counters without checking the visible subject
does not prove the feature.
