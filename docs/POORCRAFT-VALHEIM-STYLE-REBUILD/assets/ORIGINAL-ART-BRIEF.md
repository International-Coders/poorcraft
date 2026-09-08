# Original Art Brief: First Asset Batch

This is a production brief, not a request to reproduce another game's art.

## Style grammar

- Shapes are chunky but not cubic: visible planes, tapered trunks, irregular
  stone, pitched roofs, handmade asymmetry, and intentional silhouette breaks.
- Use modest texture detail. Form, palette, lighting, and silhouette must read
  before a player can see fine texture resolution.
- Default palette: moss/lichen greens, warm earth, slate blue-grey stone,
  weathered brown timber, muted linen, oxidised dark metal. Reserve saturated
  colours for banners, magical interactions, selected flowers, and UI.
- Assets are grounded in meters. Avoid heroic objects that are too large for
  player collision, doors, roads, or believable settlement footprints.

## First original assets

| Id | Brief | Runtime requirements |
| --- | --- | --- |
| `prop.tree_ash` | Tall ash tree with a subtly crooked trunk, three broad branch tiers, sparse rounded leaf clusters, exposed roots | 2–3 LODs, trunk collider, foliage cutout material, pivot at ground |
| `prop.rock_granite` | Low broad granite outcrop with an angled fracture seam and a small moss patch | 2 LODs, static convex/box collider, nonuniform silhouette |
| `module.house_croft` | One-storey timber-and-stone croft: stone plinth, framed walls, steep roof, small chimney, front-door and road sockets | 2 LODs, doorway/nav opening, simple collider, named sockets |

The models must not deliberately evoke a recognisable commercial-game asset.
They should be generated from a repository-owned Blender script or be manually
modelled and accompanied by a source `.blend`. Sources, exports, texture inputs
and compiled GLBs must all be referenced by the manifest.

## Later batches

Wilderness assets precede town kits. Town kits precede characters. This ordering
matters: a beautiful castle floating above cubic wilderness is not a successful
rebuild, and a detailed NPC cannot compensate for missing terrain/asset scale.

