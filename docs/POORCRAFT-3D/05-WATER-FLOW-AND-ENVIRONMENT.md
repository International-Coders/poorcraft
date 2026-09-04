# Water Flow and Environmental Simulation

## Design intent

Water should feel persistent and useful without running a particle simulation
continuously across the entire world. The engine remembers stable flow and
rebuilds only what changed.

## Cached flow representation

A river or channel can be represented by a persistent flow record containing:

- source and destination regions;
- centerline or connected channel segments;
- width, depth, slope, and elevation profile;
- direction and velocity class;
- discharge/capacity in game units such as liters per second;
- local interaction radius;
- terrain and flow revision numbers;
- nearby consumer/query points.

The record is a simulation cache, not a claim of real fluid dynamics. It may
be rebuilt locally when terrain, dams, channels, or sources change. Unchanged
sections remain stable and do not need per-particle work.

## Independent consumers

Machines read flow potential independently. A wheel, pump, boiler, magical
device, or other consumer may query the nearest valid flow segment within its
interaction radius.

The intended game rule is non-destructive access:

```text
river flow remains unchanged
machine output = function(flow potential, machine capacity, local conditions)
```

If a river exposes 10,000 liters per second and a wheel has 100,000 units of
potential, the wheel receives the river's available potential. A second wheel
may read the same river independently. The river is not depleted by these
queries. This is a deliberate, readable gameplay abstraction rather than a
claim of conservation of energy.

Optional balancing systems—maintenance, distance losses, blockages, or power
priority—must be added only if they improve play and remain understandable.

## Local rebuilding

When a player changes terrain:

1. Mark the edited channel and connected boundary segments dirty.
2. Recompute slope, connectivity, and local discharge constraints.
3. Rebuild the affected flow records deterministically.
4. Preserve unaffected upstream/downstream records where valid.
5. Emit an event and update machine queries.

No global water restart should be required for a local edit.

## Environmental extensions

The same bounded-region approach may later support dams, irrigation, steam,
fire, weather, snowmelt, pollution, and magical liquids. These are separate
systems sharing world queries and event contracts; they should not become one
unbounded universal fluid solver.

## Acceptance

The first water milestone is successful when a player can observe a river,
alter a channel, see the local flow cache rebuild, and operate multiple
independent consumers while the river remains visually and mechanically alive.

## Two levels of water state

Use different representations for different problems:

1. **Flow graph** for rivers, streams, canals, and other long-lived channels.
   It stores stable direction and capacity cheaply.
2. **Local volume regions** only where gameplay needs accumulation, a dammed
   basin, a lock, flooding, a container, or a newly dug channel. These use
   fixed-point volume and bounded active cells/patches.

The local model may affect nearby flow records after an edit, but it must not
turn every ocean and river in the world into an always-running cell simulation.

## Visual language

Water should communicate its state without a debug overlay: direction through
surface movement/foam/debris, depth through color and banks, danger through
sound and speed, and usable flow through visible channel shape or a tool
inspection. A calm pond and a powerful river cannot look mechanically
identical.

## Design boundary

“Non-destructive” applies to devices reading river potential. Geometry still
matters: blocking a channel, raising/lowering an outlet, adding a dam, or
diverting flow can change local flow. The game abstraction is that a wheel does
not drain the river's flow merely by operating.

For terrain data and hydrology handoff, see `15-TERRAIN-TECHNICAL-BLUEPRINT.md`.
