# Owner Asset Factory Brief

The current game does not need isolated art. It needs assets that behave like
game objects. WT-002 creates the rules and tools for high-volume generation
without losing gameplay meaning.

## Required Output

Build a pipeline that can create, import, inspect, and test batches of assets:

- buildings: huts, houses, workshops, towers, bridges, docks, walls, gates,
  mines, barns, taverns, shrines, forts, keeps, markets, farms, ports;
- machines: forge, kiln, sawmill, crusher, loom, mill, water wheel, pump,
  boiler, generator, battery, switchboard, drill, rail cart, elevator;
- NPCs: villagers, workers, guards, traders, quest givers, scouts, bandits,
  faction leaders, caravan drivers, miners, builders, farmers, mages;
- interactables: doors, beds, chests, signs, notice boards, campfires, ovens,
  anvils, levers, locks, ladders, ropes, workbenches, shrines, wells;
- world resources: ores, trees, herbs, mushrooms, fish nodes, clay, salt,
  ruins, fossils, crystals, monster nests, treasure caches;
- UI and strategy markers: map icons, quest pins, faction banners, route
  markers, warning signs, economy markers, siege markers, selection rings.

## Non-negotiable

No asset is accepted unless it has gameplay metadata and proof. A good-looking
house with no doorway is rejected. A forge mesh with no forge action is
rejected. An NPC mesh with no talk state is rejected. A beautiful object that
cannot be tested in the game is still fake.
