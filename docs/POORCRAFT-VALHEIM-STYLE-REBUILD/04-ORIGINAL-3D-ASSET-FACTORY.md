# Original 3D Asset Factory

## A file is not an asset until it is playable

Every asset must have all of the following before it is called complete:

1. An original or suitably licensed source, recorded in the manifest.
2. A source artifact: normally `.blend` plus an exported reproducible Blender
   Python script, or a repository-owned procedural source.
3. A compiled `.glb` (glTF 2.0) under `poorcraft3d/assets/compiled/`.
4. Meters, +Y up, -Z forward, applied transforms, sensible pivot, named nodes,
   material slots, and no missing textures.
5. Triangle budget, LODs, render category, collider, nav/blocking and socket
   metadata in the manifest.
6. An actual `pc3d_render` consumer and a windowed proof scene.
7. A build/validation command that fails on bad paths, unsupported texture
   formats, invalid transforms, missing LODs, or orphaned manifest entries.

## Asset families and early delivery order

| Family | First shipped set | Style/technical rule |
| --- | --- | --- |
| Terrain materials | grass, soil, rock, sand, snow, riverbed, cave stone | Tileable original maps; modest resolution; channel-packed where useful |
| Wilderness | 3 trees, 3 rocks, shrubs/grass clusters, fallen log | Strong silhouette; instanced; 2–3 LODs; cheap collider |
| Settlement | home, workshop, market, wall, tower, gate, keep | Modular sockets; original silhouette; construction-friendly scale |
| Utility | bridge, dock, water wheel, mine props, signs, banner | Anchors/mounts explicit; gameplay path preserved |
| NPCs | resident, worker, guard base + clothing/gear | One low-poly rig format; capsule collision; role readability at distance |
| Effects | water normal/flow, foliage wind, sparks/torch | Small atlas/material effects; no expensive unique simulation |

## Authoring policy

The first pass can be authored with reproducible Blender scripts and restrained
procedural geometry; it must still yield legitimate, inspectable 3D meshes, not
runtime cubes renamed as a castle. A human artist may later replace any source
without changing stable ids, sockets, bounds contracts, collision, or save data.

Use a compact stylized PBR-ish material model (base colour, normal where useful,
roughness/metallic/AO packed as appropriate). Avoid large uncompressed texture
sets. Transcode/compress only with tools available in CI/release; retain
uncompressed source images separately from compiled runtime assets.

## Asset acceptance prompts

The dedicated prompts in `prompts/` require GLM to build the pipeline first,
then produce assets in family-sized batches. It must not attempt all assets in
one run, download unclear assets, or substitute a new TODO list for a compiled
and rendered deliverable.

