# Castles, NPCs, and City Presentation

## Castle silhouette before castle detail

The first rendered capital does not need every faction kit. It needs a place a
player can recognize in one glance: an approach road, gatehouse, walls, tower,
keep or civic core, homes, workshop/market, and visible occupation.

Use the existing simulation’s capital/kit plan as the placement authority. The
renderer consumes modules and anchors from that plan; it must not invent a
second decorative capital that NPCs cannot navigate.

## Module levels

| Level | Visual requirement | Gameplay binding |
|---|---|---|
| Placeholder | original primitive silhouette with material/color | module id, bounds, collision, anchor positions |
| Beta | authored/compiled module with LODs and faction variation | nav portals, ownership, work/service anchors |
| Final | detailed materials, props, animation/light/audio polish | same stable module identity |

## NPC presentation

Start with three visible, role-readable NPCs:

- **Resident**: travels between Bed and Idle boxes.
- **Worker**: travels from Bed to Work box and shows a simple work pose/prop.
- **Guard**: patrols gate/wall route and responds to a high-level defense intent.

Their path comes from the same simulation navigation/anchor state used by the
city. A rendered NPC must never look busy while its simulation has no
destination or work role.

## Bed, Work, Idle box visualization

Boxes are city authoring tools, not permanent neon UI. In build/inspect mode:

- Bed Box shows capacity, occupied beds, quality, and unreachable cells.
- Work Box shows job type, station/storage requirements, current assignment,
  and missing inputs.
- Idle Box shows allowed residents/roles, safety, and social/free-time use.

Outside build/inspect mode they should resolve into believable world places:
homes, workshops, taverns, squares, gardens, docks, wall walks, or player-made
spaces.

## Permanent NPC death must be visible

When a named guard dies, the guard is gone from the wall, the patrol route has
a gap, garrison count drops in the city panel, and any lost service shows its
real missing capability. Do not represent permanent loss only as a number.
