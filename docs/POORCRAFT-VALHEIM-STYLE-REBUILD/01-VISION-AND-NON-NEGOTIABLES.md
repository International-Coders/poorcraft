# Vision and Non-Negotiables

## Player-visible identity

The player enters a wild, stylized continent. Ground rolls and breaks naturally;
forests have distinct silhouettes; rivers lead to useful places; a distant
fortress looks deliberately built rather than assembled from debug boxes.
Survival, exploration, crafting, settlement power, magic, and engineering
create stories without forcing a fixed campaign.

The artistic north star is **stylized low-poly naturalism**:

- faceted, hand-authored-looking slopes rather than photoreal terrain;
- strong silhouettes and restrained palettes over noisy detail;
- visible material change between soil, grass, rock, sand, snow, wood, and
  worked stone;
- warm, readable settlements against cool, dangerous wilderness;
- atmospheric lighting and fog that improve navigation rather than hide flaws.

## What remains block-based

Construction remains intentional and readable: walls, floors, machinery,
roads, defensive works, and player-created objects retain stable grid/module
placement. A building must snap predictably, have robust collision, save
reliably, and replicate cleanly. Natural terrain must adapt *around* it through
foundation, clearance, and terrain-deformation rules.

## What changes

Natural surface terrain no longer exposes a blanket of cube faces. It becomes
a 16 m x 16 m patch surface with a density/height/material sample grid, a
cached render mesh, simple terrain collision, edit deltas, and LOD. Caves and
overhangs are sparse local volumes where they materially improve the world.

## Hard exclusions

- No copyright-infringing imitation or importing commercial game assets.
- No global marching-cubes/dual-contouring volume as the first replacement.
- No per-frame full-world terrain, water, NPC, or mesh scans.
- No "asset" that is merely a name in a manifest, a procedurally coloured cube,
  or an unconsumed file.
- No breaking changes to authoritative save/world state without an explicit
  migration, version marker, regression test, and documented fallback.
- No visual claim without a human-inspected windowed proof.

