# Current Engine Audit: Why the Game Still Reads as Alpha

Audit basis: loop 356, commit `5c0d445`, `STATE.md` reporting 453 passing
tests, 107/107 vistest scenes, generator version 7, and no blocker. This is a
design audit, not a new runtime verification run.

## What must be preserved

LOREFORGE already has a real Rust workspace, wgpu renderer, streamed/persisted
voxel world, deterministic world generation, 46 biome identities, lighting,
weather, survival, crafting, machines, multiple technology eras, magic,
combat, dragons, factions, companions, maps, lore, mods, UDP networking,
Steamworks adapters, extensive tests, and a GPU proof harness. The renderer and
world substrate are assets. Replacing them wholesale would discard years of
integration knowledge encoded in tests and bug fixes.

The beta problem is depth and authority: several systems exist as convincing
first implementations but do not yet share enough state or causal behavior.

## Water and mechanical power

Implemented truth:

- Water and oil occupy ordinary `BlockState`s. The low state nibble stores
  `0..7` distance from a source; the high nibble is already used by block shape.
- `lf_game::fluids::step_cell` falls first, spreads to four horizontal cells,
  and dries unsupported flow. The client processes 64 queued cells per tick.
- Water surface height is derived only from that source-distance level.
- `WaterWheel::tick` receives a Boolean `has_water`. The client sets it when
  any side or lower neighbor is water, then generates a fixed 12 EU/s.
- Boiler-adjacent water currently supplies water without consuming a finite
  local mass.

Why it reads as alpha: there is no stored volume, velocity, discharge,
pressure/head, direction, conservation report, current force, or torque.
Still water and a fast channel are mechanically equivalent. Dams cannot build
head, flumes cannot increase usable fall, and a wheel cannot slow when flow is
starved. The existing cellular spread is valuable compatibility behavior, but
it is not yet a hydrology or mechanical-power engine.

## Castles and faction geography

Implemented truth:

- `KINGDOM_REGION` is 12 chunks, or 192 blocks. Each region can choose one
  candidate citadel chunk.
- Candidate chunks keep a two-chunk region-edge margin, so neighboring realms
  can still be only 64 blocks apart in the worst arrangement.
- Spawn clearance is 160 blocks.
- A kingdom citadel is a fixed one-chunk `16×16` build: perimeter wall,
  towers, a keep, two small houses, well, market, and farm.
- Terrain checks are far better than the original five-point test, but the
  builder still clears and normalizes an entire chunk around one base plane.
- The same royal grammar is used regardless of the six faction identities.

Why it reads as alpha: a major political center can appear too soon after the
last, its footprint is smaller than a convincing village district, and its
architecture is voxel code rather than a reusable authored module set. There
is no prebuilt multi-chunk plan, road graph, district economy, gate state,
interior navigation contract, expansion socket, or faction-specific capital
grammar. The kingdom proof can show a castle, but it cannot yet prove a living
realm.

## NPCs, reactions, and followers

Implemented truth:

- Villagers have jobs, a default daily schedule, visible activity states,
  direct locomotion with step-up, safe descent, gravity, cliff refusal, and a
  deterministic sidestep reflex.
- Most non-work schedule locations resolve to one home point. Work scans only
  for a nearby furnace in the current client helper.
- The update loop steers directly toward home/work; guards use a four-point
  trigonometric patrol. It does not use the mob A* module or a castle graph.
- Memory holds the last two interaction records and forgets after five days.
- Reactions cover a handful of nearby events and largely surface as one-line
  hints. Public faction standing remains the dominant behavioral input.
- The ordinary villager spawner caps the active list at 12; a kingdom creates
  six court residents around one shared home anchor.
- Companions have useful trust/morale/command foundations, but follow/work
  behavior and co-op synchronization remain incomplete.

Why it reads as alpha: scheduled poses exist without reliable destinations,
route planning, doors, reservations, crowds, perception, local knowledge,
settlement alarms, household relationships, or work products. NPCs react to a
small event radius rather than to what they personally saw, were told, or can
infer. A castle therefore contains moving characters but not yet citizens.

## Multiplayer and Steam

Implemented truth:

- Protocol v4 carries hello, player position, block edits, chat, and peer trade
  messages over a compact bincode frame.
- `lf_server` is authoritative-lite for generated blocks and edit history. It
  relays player snapshots and trade decisions over UDP.
- World identity adoption was repaired: clients now accept the server seed.
- The server does not own survival, inventory, drops, machines, fluids, mobs,
  NPCs, quests, faction knowledge, or settlement simulation.
- Trade item removal/grant is still finalized by clients, so "escrow" is not
  complete server-owned inventory authority.
- `lf_steam` has lobby and ISteamNetworkingSockets implementations and example
  probes, but the normal game client/server path still needs one shared
  transport interface and a real two-account test. Default builds remain UDP.
- Many messages are sent on unreliable transport without application-level
  sequencing, acknowledgement, resync, or channel semantics.

Why it reads as alpha: two players can share blocks and presence while seeing
different living worlds. A client can locally advance a machine, water cell,
mob, NPC, or quest without a canonical server event. Steam is a tested
technology path, not yet a complete player-facing multiplayer product.

## Assets and visual identity

Implemented truth:

- A procedural block/item atlas, CTM strips, packed normal/AO material maps,
  faction skins, articulated humanoid geometry, audio samples, and code-native
  UI painters exist.
- Castle geometry is written directly as voxel placement code. There is no
  validated `assets/manifest.toml` and no compiled library of modular castle
  structures with ports, collision, navigation, and activity markers.
- Existing visual packs contain good art-direction rules, but older manifests
  describe intended files more often than an enforced consumer chain.

Why it reads as alpha: individual assets exist without one source-of-truth
manifest, authored structure workflow, LOD contract, or faction castle kit.
The next step is not indiscriminate image generation; it is a consumer-led 3D
asset pipeline.

## Engine-architecture pressure

`lf_client::GameState` currently coordinates input, UI, world streaming,
fluids, block entities, power, NPCs, mobs, drops, audio, saving, and network
events. That made the alpha playable quickly, but it puts authoritative game
rules inside the presentation client. Adding deeper water, residents, castles,
and co-op there would duplicate logic and produce nondeterministic peers.

The correct repair is staged extraction into deterministic services and a
shared simulation host. It is not a ground-up renderer or voxel rewrite.

## Honest beta diagnosis

The game is feature-rich but interaction-poor in its most important new
fantasies. Beta work must prioritize these causal chains:

```text
river geometry -> measured flow -> wheel torque -> machine work
castle plan -> nav/activity anchors -> residents -> economy/alarm -> faction play
player action -> perception/evidence -> memory/report -> policy -> reaction
player command -> authoritative simulation -> persistent event -> every peer
asset manifest -> compiled consumer -> rendered proof -> runtime fallback
```

Until those chains work end to end, adding more blocks, biomes, factions, or
screens will make the alpha wider without making it more complete.
