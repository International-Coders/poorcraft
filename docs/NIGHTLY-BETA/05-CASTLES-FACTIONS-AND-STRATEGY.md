# Castles, Factions, and Strategy Layer

The target is the pleasure of discovering and affecting powerful faction
towns inside a voxel survival RPG. Use faction-town strategy games as systemic
inspiration—distinct settlements, dwellings, recruitment, resources,
commanders, and map consequences—but keep every LOREFORGE name, layout, unit,
texture, icon, line, and mechanic implementation original.

## Realm roster

Deepen the six existing realms and add two only after the shared faction data
contract can support them:

| ID | Realm fantasy | Castle grammar | Strategic value |
|---|---|---|---|
| `accord` | human law and coalition | blue-gray curtain walls, civic court, balanced gatehouse | roads, diplomacy, disciplined guard |
| `ironborn` | mountain industry | terraced forge-fort, cranes, hot channels, heavy buttresses | ore refining, machines, siege craft |
| `ember_covenant` | living fire and nature | ringed grove citadel, roots, ember glass, open canopy | alchemy, magic, beasts, restoration |
| `free_holds` | independent agrarian clans | timber-and-stone hillfort, long halls, farms, palisades | food, mounts, scouts, local trade |
| `ashen_order` | memory and neutral knowledge | pale stepped archive, observatory, sealed vaults | lore, maps, enchantment, research |
| `nameless` | exile, death, broken compacts | sunken necropolis, bone/rotwood, broken verticals | undead pacts, curses, forbidden salvage |
| `gravebound_court` | ordered undead continuity | tomb-city terraces, ossuary towers, cold lantern processional ways | death magic, revenant retainers, grave resources |
| `cinder_host` | infernal contracts and conquest | basalt caldera fortress, brass seals, chained bridges, magma vents | bargains, destructive magic, elite warbands |

`gravebound_court` and `cinder_host` are original expansion factions. They
must not be thin reskins of Nameless or Covenant; reconcile overlaps in lore
and gameplay before shipping them.

## Castle spatial contract

Every major castle is generated from a deterministic layout plan before any
voxel is written. The plan contains:

- terrain-integrated outer boundary and supported foundation;
- one reachable main gate and at least one service/sally route where suitable;
- road connection to a local path network;
- keep/ruler anchor, garrison, market, crafting/work district, storage,
  housing/rest anchors, food/water source, and faction signature landmark;
- navigation graph nodes for gates, doors, stairs, streets, workplaces,
  beds, defensive posts, safe zones, and creature dwellings;
- expansion sockets so upgrades add districts without replacing player edits;
- defensive silhouette that responds to slope and biome rather than a fixed
  box stamped onto every world;
- protected structure ownership metadata and repair rules.

Use a grammar of modules with constraints, not a single blueprint. A module
declares bounds, ports, foundation rule, roof rule, allowed rotations, required
neighbors, faction materials, nav anchors, NPC roles, and proof tags.

## Settlement simulation

Castles must do something after generation:

- population and jobs are tied to actual beds/workplaces/dwellings;
- market stock responds to local production and safe routes;
- gates open/close by schedule, alarm, faction standing, and siege state;
- guards patrol posts, investigate witnessed crimes, raise alarms, escort or
  challenge the player, and return to duty;
- workers perform short visible work loops whose result changes stock or a
  production counter without simulating every distant animation;
- damage persists; repair consumes resources and respects player edits;
- named rulers/commanders expose policy, quests, garrison, and allegiance;
- discovery, alliance, hostility, capture, liberation, and destruction enter
  the chronicle.

## Strategy systems adapted to real-time voxel play

- Resource sites: mines, farms, lumber, arcane wells, grave fields, and infernal
  vents produce at bounded daily/weekly ticks while controlled and connected.
- Creature dwellings: each castle has faction-specific recruit pools with
  transparent replenishment; units are companions/garrison squads, not a
  separate unintegrated menu army.
- Commander/hero: a named NPC with traits, equipment, relationship, and orders;
  they can defend, patrol, escort, explore, or lead a raid using the same world
  AI and collision rules.
- Garrison: defenders occupy real posts, have alert states, and cannot spawn
  endlessly. Casualties and reinforcement are persistent.
- Castle upgrades: player-delivered resources unlock real module sockets and
  services. Construction is visible in-world and never erases protected edits.
- Territory: influence is an overlay on discovered map cells used for patrol,
  prices, events, and diplomacy—not permission to rewrite terrain.
- Conflict: raids and sieges are bounded events with preparation, goals,
  retreat, and aftermath. Distant simulation uses summaries; nearby entities
  obey real physics and navigation.

## Faction difference test

For each realm, a player should answer from an unlabeled screenshot and a
five-minute visit:

1. Who built this?
2. What do they value?
3. What can I do here that I cannot do elsewhere?
4. What behavior will make them trust or fear me?
5. What resource, creature, or service connects this place to progression?

If two realms have the same footprint, NPC roles, shop list, and recolored
walls, neither is finished.

## Placement and generation tests

- Property-test at least 128 castle sites over varied seeds and world types.
- Required assertions: supported ratio, maximum foundation depth, gate clear,
  path connected, no required room buried, no critical block in water/lava,
  nav anchors reachable, no protected spawn overlap, bounds respected, module
  ports joined, and deterministic layout hash.
- Render at least eight seeds per castle grammar during development; retain a
  representative proof set after stabilization.
- Test regeneration around saved player edits and generator-version changes.
- Test alarm/gate/garrison state persistence and restart.

## Visual acceptance

The camera must include surrounding terrain, entrance, road, and skyline; a
cropped floating diorama hides placement bugs. Z.ai reports silhouette,
material identity, terrain seam, entrance readability, repeated modules,
empty/dead space, NPC activity, and visible clipping. Any severe pedestal,
sheared river, floating wall, inaccessible gate, or identical faction grammar
is a failure.
