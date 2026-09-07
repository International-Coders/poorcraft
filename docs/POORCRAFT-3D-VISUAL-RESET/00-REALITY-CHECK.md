# Reality Check: What Z.AI Built and What Is Still Missing

Audit date: 2026-09-06.

## What exists in `poorcraft3d/`

- Rust workspaces `pc3d_core`, `pc3d_world`, `pc3d_save`, and the
  `poorcraft3d` executable.
- Deterministic seed/world identity, command/event flow, save framing, and
  simulation host.
- Terrain-cell generation, caves/overhang logic, construction overlay, LOD
  bookkeeping, river/flow records, NPC/city/faction state, machines, magic,
  nuclear systems, dragons, replication contracts, and tests.
- CLI diagnostics that write small 2D atlas, flow-map, and debug-overlay PNGs.
- A claimed 273 P3D unit tests and a headless journey/soak harness.

This work can remain as the simulation substrate. It must not be discarded
merely because it is not visual.

## What does not exist

The audit found the following concrete gaps:

| Required for a 3D game | Current evidence |
|---|---|
| window/event loop | absent from the P3D workspace dependencies |
| GPU renderer | absent: no `wgpu`/renderer crate in `poorcraft3d/Cargo.toml` |
| shader sources | zero `.wgsl`, `.glsl`, vertex, or fragment files |
| real terrain mesh | absent: terrain is cells/data; mesh queue is bookkeeping |
| first-person rendered scene | absent: executable is a CLI/headless simulator |
| 3D model format/loader | absent: zero `.gltf`, `.glb`, or `.obj` assets/loaders |
| material/texture asset pipeline | absent: current P3D PNGs are diagnostic maps only |
| actual water surface | absent: flow map is a 2D proof, not a water mesh |
| rendered NPC/castle/city | absent |
| visual save/reload proof | absent |

`poorcraft3d/dist3d/POORCRAFT3D/PLAY.md` explicitly says the windowed renderer
client “does not exist yet.” The current debug proof is therefore evidence of
simulation state, not evidence that the game looks or plays like a 3D voxel
world.

## Diagnosis

The prior roadmap completed **simulation-first**, but it stopped before the
project became a visual game. That is why it feels like Z.AI “did something
else.” It built a rule engine and called it a finished 3D roadmap.

## Preserve / replace

| Preserve as source of truth | Replace or add next |
|---|---|
| `pc3d_core` deterministic identity/clock/events | `pc3d_render` window, GPU, camera, render graph |
| `pc3d_world` terrain/query/flow/settlement state | actual terrain, construction, water, and entity meshes |
| `pc3d_save` formats | visual-world save/reload integration |
| existing tests/journey as simulation evidence | scene tests, screenshot tests, manual playable proofs |
| LOD/stream queues as scheduling inputs | real background mesh upload and GPU lifetime management |

## Reset decision

From this point, the next complete milestone is not “another system.” It is a
small but honest playable 3D slice. The renderer reads P3D state; it does not
fork the simulation or create a second fake world in the client.
