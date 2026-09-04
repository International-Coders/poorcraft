# Castles, Factions, and 3D Assets

## Castle purpose

Castles are major places of power. They should be rare enough to feel like
destinations, far enough apart to make travel meaningful, and distinct enough
that a player can recognize them without a label.

## Castle requirements

Each beta capital needs:

- a multi-chunk or equivalent district footprint;
- terrain-aware placement and foundations;
- readable approach roads, gates, walls, and skyline;
- interior circulation for players and NPCs;
- faction-specific architecture and material language;
- homes, work areas, services, storage, defenses, and social spaces;
- navigation anchors, activity markers, and expansion sockets;
- protection rules that do not erase legitimate player edits;
- a modular authored asset plan with collision and LOD data.

Castle spacing should be chosen by world-scale testing, not an arbitrary
minimum distance. The player should experience an expedition between major
capitals, while smaller villages and outposts provide intermediate discovery.

## Faction identity

Factions need differences in:

- values, laws, and political tensions;
- resources, occupations, and services;
- castle proportions, roofs, walls, streets, and interiors;
- colors, materials, symbols, sound, and lighting;
- NPC names, roles, dialogue, and reactions;
- strategic advantages and vulnerabilities.

The six existing core factions should be deep before additional factions are
added. A reviewer must be able to distinguish a realm from its architecture,
behavior, and economy rather than from a text label alone.

## Asset pipeline

Every 3D or voxel asset requires:

- an identifier and manifest row;
- source/provenance information;
- material and texture references;
- collision and interaction metadata;
- navigation/anchor metadata where relevant;
- LOD or impostor behavior;
- at least one real consumer in generation or runtime;
- a visual proof or review path.

Generated assets are not complete when files exist. They are complete when the
runtime consumes them and the result is recognizable, performant, and saved.

## Art direction

Use natural terrain and authored silhouettes to move beyond generic cubes while
retaining readable voxel construction. Selective smoothness is preferable to
making every surface equally polished or equally expensive.

## Capital build order

Build each major capital from a plan before placing detail:

1. Choose a terrain-aware site and approach routes.
2. Reserve districts: gate, keep/civic core, homes, market, work, storage,
   worship/ritual or research, defense, and expansion.
3. Publish navigation portals, activity anchors, service points, ownership,
   and protection boundaries.
4. Assemble faction modules with stable identifiers and LOD/collision data.
5. Spawn residents only after their homes, jobs, and routes exist.
6. Run the capital's economy/alarm/service proof before adding decoration.

## Asset quality gate

An asset is rejected if it is technically present but has no role, no consumer,
no collision/LOD plan where needed, no source/provenance, or no recognizable
in-game silhouette. This protects the project from accumulating generated
files that look impressive in isolation but do not form a playable place.

## Travel target

Capital spacing must be decided from the owner’s intended travel time and
travel tools. Until that answer is given, generation should support sparse
major sites with smaller villages, ruins, bridges, and outposts in between.
