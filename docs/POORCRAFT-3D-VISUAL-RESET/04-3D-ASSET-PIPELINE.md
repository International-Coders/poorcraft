# 3D Asset Pipeline and Manifest Contract

## Why a manifest comes before asset generation

The project needs assets that become real game objects, not a folder of
untracked generated images. Every critical asset is declared in
`assets/beta_critical_assets.json` and validated against
`assets/asset_manifest.schema.json`.

An asset is incomplete until it has all of:

1. Stable id and category.
2. Original or properly licensed provenance.
3. Source and compiled-file paths, or an explicit procedural placeholder.
4. Material/texture and render-pass information.
5. Collision, navigation, and interaction metadata when relevant.
6. LOD/fallback decision.
7. Runtime consumer(s).
8. A visual proof scene.

## Asset categories

- Terrain materials: grass, soil, rock, sand, snow, riverbed, cave stone.
- Construction blocks: wood, stone, roof, wall, floor, door/window.
- Castle modules: gatehouse, wall, tower, keep, house, workshop, market.
- City markers: Bed Box, Work Box, Idle Box; readable but unobtrusive.
- NPCs: resident, worker, guard; low-poly/voxel placeholders before final rigs.
- Machines: water wheel, boiler, engine/generator, valve console.
- World props: tree, bridge, river dock, mine/cave prop, signs/banners.
- Effects: water foam/current, torch/forge light, selection outline.

## Source policy

- Create original content or use assets whose license permits the intended
  distribution and modification.
- Do not use Warcraft, Heroes of Might and Magic, Minecraft, Skyrim, All the
  Mods, or other game assets as source material.
- An AI-generated image or model still needs provenance, review, a manifest
  row, and an actual consumer.
- Early placeholders may be generated procedurally from cubes, prisms, and
  simple materials, but they must be labelled `placeholder` rather than final.

## 3D model rules

- Preferred exchange format: glTF 2.0 (`.gltf` / `.glb`) for authored props,
  NPCs, and castle modules.
- Construction and terrain are generated meshes, not thousands of separate
  glTF files.
- Model scale is meters and up-axis is documented in the manifest.
- Meshes need reasonable triangle budgets, authored bounds, material slots,
  and LODs where repeated/large.
- Collision may be box/capsule/convex/simple mesh; never assume render mesh is
  safe collision.

## Asset gate

The next agent must implement an asset-manifest validator before importing
anything beyond placeholder primitives. The validator rejects duplicate ids,
missing consumers, absent proof scene, forbidden source policy, invalid LODs,
or a final asset with no compiled/runtime path.
