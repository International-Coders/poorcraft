# Asset Bible and Complete Beta Manifest

This file defines every asset family the beta needs and the metadata each
asset must carry. It is a manifest specification, not permission to generate
thousands of disconnected PNGs. Assets land only with a real consumer, proof,
and catalog test.

## Art direction

- Original stylized voxel/pixel fantasy-industrial art with strong silhouettes,
  limited palettes, crisp nearest-neighbor edges, and readable material scale.
- Surfaces use intentional clusters, seams, wear, and construction logic;
  uniform random noise is not finished texture work.
- Factions share world lighting/material rules but differ in silhouette,
  rhythm, palette, symbols, roofline, entrances, props, and emissive language.
- Icons must be identifiable at actual hotbar size, not only enlarged.
- Alpha means intentional cutout/transparency. No accidental dark/opaque
  backgrounds, halos, antialias blur, or unlicensed embedded references.
- Never copy or trace Heroes of Might and Magic or another game's assets.

## Required manifest row

Every generated or hand-authored asset has:

```text
id | type | path | consumer | dimensions/layout | palette | alpha mode |
authored/generated | generator+seed | source/license | animation set |
LOD/fallback | proof scene | status | reviewer/date
```

The canonical machine-readable form is described in `11-DATA-CONTRACTS.md`.
Catalog validation fails on a missing file, duplicate ID, invalid dimensions,
unknown consumer, absent license/source, missing fallback, or beta asset with
no proof.

## Existing assets to audit first

- Procedural block/item atlas and texture mappings in `lf_assets`.
- Eight tracked CTM strips under `assets/ctm/`.
- Six 64×32 faction NPC skins under `assets/skins/npc/`.
- Current asset catalog proof with 164 entries.
- Generated sounds or other dirty assets present at execution time; preserve
  ownership and licensing metadata before adopting them.
- Old/manual screenshots are evidence files, not source art.

## Environment and block families

For every biome material used in beta:

- top/side/bottom textures where faces differ;
- connected-surface set only where large construction surfaces need it;
- normal/AO or the project's current material auxiliary data;
- damaged/mining feedback compatibility;
- slab/stair/log-end routing where applicable;
- inventory icon crop and map color;
- wet/snow/seasonal variants only when the renderer and gameplay consume them.

Required environment families: soils, grasses, sands, snows/ice, stones/ores,
woods/leaves, water/lava/special fluids, cave growth, crops/food plants,
roads/paths, ruins, industrial materials, magic materials, and faction kits.

## Castle kit per realm

Each of the eight realms needs a coherent minimum kit. One texture may serve
multiple shapes through face routing, but every role must read distinctly:

1. foundation;
2. primary wall;
3. secondary wall/trim;
4. floor/path;
5. roof;
6. pillar/buttress;
7. window/grate;
8. door and gate states;
9. banner/sign with original symbol;
10. light/emissive fixture;
11. workstation or dwelling marker;
12. statue/landmark material;
13. damaged/ruined treatment;
14. map color and inventory icons.

That is a role matrix of 8 realms × 14 roles = 112 verified role entries,
not necessarily 112 unique image files. Reuse must be declared; recolor-only
reuse cannot satisfy primary silhouette roles.

## NPC skin and equipment roster per realm

Minimum readable roles:

1. ruler/commander;
2. guard;
3. scout/ranger;
4. crafter/worker;
5. trader;
6. scholar/priest/ritualist;
7. civilian/farmer/caretaker;
8. hero/companion.

Eight realms × eight roles = 64 role entries. Variants can share a base skin
only when equipment, palette, or silhouette still distinguishes the role.
Each role declares head/body/limb UVs, carried props, faction symbol placement,
skin tone or nonhuman material, and portrait/icon crop.

Required humanoid animation states: idle with at least two subtle variants,
walk, run, work/interaction, talk/greet, alert, attack, block/cast where valid,
hurt, flee, death/downed, sit, and sleep. Current procedural limb swing can be
the base, but state timing, facing, prop attachment, and transition tests must
be explicit.

## Creature and dwelling roster

Do not begin with seven unimplemented tiers per faction. Beta minimum is four
playable creature families per realm, each with a base and elite identity:

- common defender/worker;
- mobile scout/ranged role;
- specialist support/control role;
- rare signature large/elite role.

For 8 realms × 4 families, track 32 base creature entries plus elite material/
equipment variants. Each entry needs world model/skin, icon, portrait crop,
hit/death feedback, shadow/footprint, dwelling prop/sign, drops, audio key,
movement profile, and animations appropriate to anatomy. A creature does not
ship because an icon exists; it ships when behavior and rendering consume it.

Neutral beta families also need entries: common passive animals, common night
hostiles, biome guardians, bosses already in data, and any spawn-or-cut types
such as Geode Guardian and Cinder Crawler.

## HUD and menu asset inventory

- health, hunger, air, mana/status when implemented, XP, armor/durability;
- crosshair states, hit directions, interaction key chips, blocked reasons;
- eight faction crests and standing/fear/respect markers;
- objective, quest, discovery, rumor/witness, crime/warrant, settlement alert;
- crafting categories, station types, ingredient states, favorite/new/locked,
  queue states, cancellation, output-full, time/power;
- map landmarks, castles, dwellings, resource sites, roads, danger, companion;
- currency/resources used by castle economy;
- input glyph source that can adapt to key rebinding;
- accessibility alternatives: text/shape in addition to color.

Prefer existing vector painter/code-native glyphs when they stay crisp and
themeable. Do not create bitmap files for shapes the UI kit already draws well.

## Asset production pipeline

1. Create/approve a manifest row and consuming code path.
2. Generate a rough original asset with deterministic seed or author it.
3. Run structural checks: dimensions, color/alpha, tile seams, UV occupancy,
   palette size, transparent border, and file size.
4. Render it through the real atlas/model/UI, never judge only the source PNG.
5. Use Z.ai recognition at real gameplay scale and an enlarged diagnostic.
6. Fix silhouette/material/readability issues; record reviewer/date/status.
7. Add the file, catalog mapping, behavioral data, proof, and tests in one job.

Generated filenames use stable IDs and never overwrite hand-authored files.
The generator version and seed are recorded. Regeneration writes to a staging
path, produces a visual diff, and requires acceptance before replacement.

## Asset acceptance tests

- Catalog closure: block registry → atlas → items/drops/recipes and back.
- No two beta-critical icons are near-identical at 16×16 perceptual scale.
- Tile edge/CTM and alpha-mode assertions.
- Faction palette is present but not the only distinguishing feature.
- Entity UV occupancy and no transparent required face.
- Animation frame/state coverage and transition validity.
- Asset catalog page renders every item with ID labels available in debug.
- Castle material gallery and NPC role lineup for every realm.
- Z.ai can identify role/realm from unlabeled crops above a calibrated success
  threshold; confusion is logged and corrected, not hidden by labels.

## Licensing and provenance

Every asset is `project-authored`, `procedurally-generated`, or has a specific
compatible third-party license and source record. "AI generated" is not a
license. Prompts must request original LOREFORGE work and exclude living-artist
imitation and protected franchise replicas. Secrets/API keys never enter the
repository, logs, manifests, or screenshots.
