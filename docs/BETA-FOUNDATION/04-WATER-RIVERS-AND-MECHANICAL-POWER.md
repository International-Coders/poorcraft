# Water, Rivers, and Mechanical Power

## Player promise

Water is a buildable force. A river has direction and strength. Blocking it
raises upstream water; opening a gate releases a surge; narrowing or dropping
a channel changes usable flow; a wheel turns only when water crosses it with
enough discharge and head. The same current can push loose items, swimmers,
and boats when those systems are present.

The target is a deterministic voxel hydrology model, not general-purpose CFD.

## Preserve and replace

Preserve the existing source/flow visuals, edit-triggered queue, water
collision semantics, buckets, worldgen rivers, transparent rendering, and
save compatibility. Replace the source-distance nibble as the sole simulation
truth. It can remain a legacy/render fallback during migration.

Fluid state does not fit safely in `BlockState`: its low nibble already stores
flow level and its high nibble stores construction shape. Introduce sparse,
chunk-partitioned simulation data.

## Simulation model

Recommended fixed-point cell state:

```text
FluidCell {
  kind: Water | Crude | Lava,
  volume: 0..4096,             # fraction of a voxel, fixed point
  velocity_x: i16,
  velocity_y: i16,
  velocity_z: i16,
  pressure: i16,               # bounded local head/constraint term
  flags: SOURCE | OCEAN_BOUNDARY | DIRTY | FALLING
}

FluidRegion {
  chunk,
  cells: sparse map or compact active slab,
  dirty_frontier,
  total_finite_mass,
  version,
}
```

Use integer/fixed-point arithmetic for canonical results. Rendering may
convert to floats. Process cells in a deterministic order independent of hash
map iteration and thread scheduling.

## Boundary types

- Ocean boundary: infinite reservoir at the generated sea surface, simulated
  only near changed/active shores.
- Spring/source: bounded emission rate, not automatic creation of a full new
  source cell every tick.
- Finite water: conserved local volume from buckets, tanks, redirected rivers,
  and isolated pools.
- Drain/void boundary: explicit world or structure rule with measured outflow.
- Frozen boundary: ice stores or blocks water according to biome/temperature
  rules without duplicating mass.

Worldgen rivers provide a stable macro river graph and boundary discharge.
The live solver covers player-modified and visible regions. The engine must
not tick every ocean cell.

## Flow rule

At each fluid tick:

1. Activate dirty cells and a bounded neighbor halo.
2. Compute potential from cell base elevation, fill height, pressure, and
   downward gravity.
3. Propose outflow across open faces based on potential difference, face
   resistance, fluid viscosity, and prior momentum.
4. Scale proposals so total outflow never exceeds available volume.
5. Apply all transfers in a second deterministic phase.
6. Update velocity from net directional flux and damp it by viscosity.
7. Deactivate cells whose volume and flux stay below thresholds for a stable
   period.

This two-phase transfer avoids iteration-order creation or deletion. Downward
flow is strongest, but horizontal flow can retain momentum and carry discharge
through channels.

## Rivers, dams, gates, and flumes

- Generated rivers expose upstream/downstream direction, nominal discharge,
  width class, bed height, source basin, and mouth connection.
- A solid dam blocks face transfer and raises upstream fill until water finds a
  spillway or overtops a bounded edge.
- A sluice/gate block has authoritative open fraction and permitted flow faces.
- A flume is a player-built channel whose floor/walls reduce leakage and whose
  elevation drop creates useful head.
- Culverts and grates declare permeability and resistance.
- A river cannot be "cut" merely because an unloaded chunk is absent; region
  boundaries exchange summarized inflow/outflow.
- Beta does not require terrain erosion. Water may move designated loose
  blocks only through an explicit later rule.

## Mechanical power contract

A wheel has position, axle axis, rotor plane, radius class, immersion range,
angular velocity, inertia, and load. It samples net flux through its rotor
plane, not nearby block identity.

Conceptual output:

```text
hydraulic_power = density * gravity * discharge * effective_head
shaft_power = hydraulic_power * wheel_efficiency
angular_acceleration = (water_torque - machine_load - drag) / inertia
```

Constants are tuned for play, but relationships are binding:

- still water produces no sustained torque;
- backwards flow reverses or brakes the wheel;
- higher discharge or head produces more torque up to the wheel's safe cap;
- an overloaded wheel slows and may stall instead of minting a fixed output;
- removing the channel or closing the gate drains stored rotational energy;
- orientation matters; a parallel current cannot power the wrong rotor plane;
- generated EU comes from measured shaft work through a generator coupling.

Small undershot wheels favor high flow; overshot wheels favor head and flumes.
Beta may ship one wheel type first, but the data contract must allow both.

## Entity interaction

The simulation exposes a `FlowSample` at an AABB:

```text
average_velocity, submerged_fraction, force, dominant_direction, turbulence
```

Players, mobs, NPC movement profiles, boats, and dropped items consume that
sample. Apply bounded acceleration, respect mass/drag, and never teleport an
entity. NPC planners mark unsafe crossings and use bridges unless their
movement profile can swim.

## Rendering and audio

- Surface height comes from volume, with corner heights sampled from neighbors.
- UV or normal motion follows the dominant velocity direction and magnitude.
- Waterfalls, rapids, foam, wheel splash, and calm pools are visually distinct
  at raster quality; path tracing remains optional.
- Audio uses spatial flow intensity: calm lap, stream, rapid, waterfall, wheel
  creak/splash. No per-cell audio source.
- F3 can display region bounds, velocity arrows, volume, flux, active cells,
  solver backlog, wheel torque, and mass error.

## Multiplayer and persistence

- Only the authoritative simulation ticks fluids and wheels.
- Clients receive region baselines plus versioned sparse deltas and wheel
  state; cosmetic surface interpolation is local.
- Block edits that alter fluid boundaries are commands validated by the host.
- Reconnect requests missing region versions rather than replaying the entire
  ocean.
- Saves persist non-generated fluid deviations, sources/gates, active region
  versions, and machine rotational state. Generated untouched water can be
  reconstructed from `WorldIdentity`.

## Performance rules

- Fixed fluid cadence below render rate; begin at 10 Hz and tune from evidence.
- Global and per-region cell budgets with deterministic continuation.
- Active-region sleep and wake on edits, boundary inflow, or nearby motion.
- No full-world scans, no allocation per cell per tick, and no remesh per edit;
  coalesce changed cells by section before meshing/network deltas.
- Benchmark a calm ocean, flowing river, player dam, dam release, and multiple
  loaded wheels.

## Required tests

- finite closed basin conserves mass exactly within the fixed-point contract;
- transfer result is independent of insertion/chunk generation order;
- source emission and drains match declared rates;
- still pool settles and sleeps;
- dam raises upstream level and opening a gate creates downstream flux;
- flume drop increases usable head relative to a level channel;
- still adjacent water does not power a wheel;
- correct current powers the wheel, reverse flow changes torque sign, load can
  stall it, and output is bounded;
- current pushes a loose item and affects a swimmer without tunneling;
- save/load resumes identical state and mass;
- server and client-view replicas converge after dropped/reordered deltas;
- active-region and remesh budgets hold in the stress scene.

## Visual proofs

- `river_source_to_mouth`: macro connectivity, variable width, tributary or
  basin, and credible mouth.
- `water_dam_head`: closed gate with high upstream/low downstream surfaces.
- `water_dam_release`: visible directional surge and foam after opening.
- `water_flume_wheel`: channel crosses the rotor plane and the HUD/F3 proof
  reports nonzero torque and machine work.
- `water_still_wheel`: identical wheel in a pool, visibly stopped, zero output.
- `water_current_push`: deterministic dropped item or test body carried a
  measured distance.

Beta water passes only when changing the channel changes the measured behavior
and the player can understand the result in-world.
