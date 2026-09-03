# Castles, Factions, and 3D Assets

## Player promise

A major castle is an expedition destination, a political center, and a working
settlement—not a one-chunk structure encountered every few minutes. From an
unlabeled skyline, the player can identify who built it. From a visit, the
player can explain what it produces, whom it protects, what its laws are, and
how their actions changed it.

## Realm scope

The beta core is the six existing original realms: Accord, Ironborn, Ember
Covenant, Free Holds, Ashen Order, and Nameless. Each needs one capital grammar
and a smaller-site grammar. Additional Gravebound/Cinder realms remain
post-beta until this core is deep enough to prevent recolor-only factions.

## World spacing and hierarchy

Replace "one kingdom candidate per 12×12-chunk region" with a macro realm map.
Initial calibration targets:

- major capitals: at least 1,024 blocks center-to-center, target median
  1,400–2,000 blocks;
- nearest major capital from a safe spawn: normally 650–1,600 blocks, revealed
  through rumor/map/compass rather than visible on the horizon;
- towns/forts: at least 384 blocks from another same-tier site and normally
  256+ blocks from a capital unless they are an intentional satellite;
- hamlets/resource sites may be closer, but must not form structure spam;
- opposing capitals cannot share the same immediate biome basin or appear
  wall-to-wall across a region seam.

These are starting contracts to calibrate across the 64-seed corpus. A
deterministic Poisson-disc or best-candidate selection over macro provinces is
preferred. Candidate scoring includes travel network, water, slope, resources,
biome affinity, defensive position, spawn distance, other realms, and protected
player edits. If no site qualifies, skip or downgrade it.

## Site classes

- Capital: 96×96 to 192×192 block planned bounds, multi-district, rare.
- Town/fort: 48×48 to 96×96, one economic specialty and garrison.
- Hamlet/outpost: 16×16 to 48×48, local service or route function.
- Resource site/dwelling: mine, farm, grove, archive, camp, creature dwelling.

Bounds are planning envelopes, not rectangles that must be flattened. Modules
terrace, bridge, retain, or step with terrain.

## Pure plan before voxel writes

Every site is produced as a deterministic `CastlePlan` before mutation:

```text
site identity + realm + grammar version
terrain/water/resource summary
district polygons and elevation bands
modules and oriented bounds
ports/connections/roads
entrances, doors, stairs, bridges
navigation and activity anchors
beds, workplaces, storage, markets, posts, safe zones
water supply and drainage
NPC role requirements and population capacity
expansion sockets
protected cells and ownership
layout hash and rejection reasons
```

Planning, terrain adaptation, placement, and settlement activation are
separate stages. Placement returns a report with changed cells, support depth,
river impact, entrance clearance, anchor reachability, protected-edit status,
and any downgraded module.

## Six capital identities

### Accord civic bastion

Layered curtain walls, formal gate court, civic hall, barracks, market, road
milestones, balanced axes softened by later organic growth. Gameplay: law,
permits, diplomacy, disciplined guards, safe routes.

### Ironborn forge-fort

Terraced mountain massing, buttresses, cranes, ore lifts, hot channels, foundry
courts, smoke stacks, rail/haul routes. Gameplay: ore processing, engineering,
contracts, production hazards.

### Ember Covenant living citadel

Ring paths, large preserved trees, root-supported platforms, water gardens,
ember glass, shrines and alchemy clearings with a porous defensive boundary.
Gameplay: restoration, magic, ecology, creatures, sanctions for land damage.

### Free Holds hillfort

Palisade and stone rings following contours, long hall, clustered homes,
granaries, farms, livestock courts, wells, informal gates and hospitality
spaces. Gameplay: food, mounts, local autonomy, household relationships.

### Ashen Order archive city

Stepped pale terraces, archive halls, observatory, bridge stacks, sealed vault,
reading courts, controlled sight lines and restricted wings. Gameplay: maps,
lore, research, evidence, access permissions.

### Nameless sunken refuge

Broken verticals, concealed entries, reused ruins, tunnels, suspended salvage,
rotwood, scorched stone, false routes and communal fire. Gameplay: stealth,
salvage, exile networks, conditional refuge—not a generic evil castle.

## Settlement function

Every capital contains real anchors and state for:

- gate and at least one secondary exit;
- ruler/leader, public audience area, law board, and restricted space;
- guard posts, alarm source, muster/shelter zones, finite garrison;
- market stalls and stock stores linked to production or routes;
- workplaces whose visible activity changes counters or inventory;
- beds/homes and food/water capacity tied to population;
- faction service unavailable elsewhere;
- signature landmark and map silhouette;
- damage, repair, ownership, and safe expansion sockets;
- discoverable history and at least one local conflict.

Castle simulation uses near/far LOD. Nearby residents act in full geometry;
far settlements advance bounded economic, route, population, and alarm
summaries. Distant simulation never manufactures visible NPCs from nothing.

## 3D asset strategy

Voxel terrain remains code-native. Castle art gains a consumer-led modular
asset workflow:

### Voxel architecture modules

Use MagicaVoxel `.vox` or another explicitly selected voxel authoring source
for walls, gates, towers, rooms, roofs, bridges, props, and landmarks. An
`xtask` compiler converts source assets into a versioned project-native
structure format containing palette roles and occupied cells. Runtime should
not parse editor formats on the hot path.

Each module has a sidecar/manifest row declaring:

```text
id, realm, role, source path, compiled path, author/origin/license
bounds, pivot, allowed rotations/mirroring
material-role substitutions
connection ports and required neighbors
foundation/roof/water rules
collision and occlusion class
nav portals and activity anchors
LOD/fallback, proof scene, review status
```

### Mesh assets

Use glTF 2.0 only for silhouettes that voxel blocks cannot express well:
mechanical rotors, cranes, banners/cloth rigs, statues, furniture, hero props,
and later animated creatures. Meshes require explicit scale, pivot, collision,
material, texture, LOD, and fallback metadata. A missing mesh must not remove a
gameplay doorway or workstation.

### Textures and materials

Keep the existing nearest-neighbor block atlas and packed normal/AO material
path for construction surfaces. Faction identity must use proportions,
silhouette, rhythm, roofline, entrances, props, and light—not palette alone.

## Asset pipeline

1. Approve a manifest row tied to a real module or renderer consumer.
2. Author/generate original source in a staging location with provenance.
3. Validate dimensions, scale, palette/material, alpha, pivots, ports, anchors,
   collision, and license.
4. Compile to the runtime format deterministically.
5. Place through the real castle planner on multiple terrain types.
6. Render at gameplay scale and diagnostic close-up.
7. Review silhouette, seams, entrances, interior clearance, repetition, and
   faction recognition; fix failures.
8. Land source, compiled asset, manifest, consumer, tests, and proofs together.

No asset is complete because it exists in `assets/`. No AI-generated file is
complete without project rights/provenance and an in-game consumer.

## Required tests

- 128+ site corpus per active grammar: deterministic plan hash, spacing,
  supported ratio, foundation depth, river impact, gate/road connection,
  required rooms, port joins, bounds, and nav reachability;
- hostile terrain candidates reject rather than force a platform;
- regeneration preserves protected player edits;
- every required realm-role manifest entry resolves to source, compiled asset,
  fallback, consumer, and proof;
- module rotation/mirroring preserves ports, collision, and nav anchors;
- every resident role has a reachable bed, workplace/post, food/water access,
  and emergency safe anchor;
- damage/repair, gate/alarm, stock, and population survive save/load;
- realm recognition from unlabeled skyline and material/role crops exceeds a
  calibrated threshold.

## Visual proofs

Capture terrain, skyline, entrance, road, water relationship, and visible life;
never crop away the foundation. Keep one eight-seed siting atlas and one day/
night/activity set per capital grammar. A pedestal diorama may test individual
modules but cannot prove a castle site.

## Beta completion

A realm castle passes only when architecture, services, residents, law,
economy, navigation, alarm response, persistence, and recognition all work in
the same placed structure. Six shallow shells do not satisfy the beta gate.
