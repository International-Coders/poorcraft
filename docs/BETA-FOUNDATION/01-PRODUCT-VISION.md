# Product Vision: A Living Voxel Realm RPG

## One-line pitch

LOREFORGE is a buildable voxel-world RPG where rivers power machines, distant
faction capitals live by their own laws, companions remember the road, and the
world keeps running consistently whether played alone or with friends.

## The intended blend

The three references contribute different qualities:

- A Skyrim-like RPG layer means places, organizations, named characters,
  conflicting values, discoverable stories, personal advancement, and choices
  that change how people treat the player.
- A Heroes of Might and Magic III-like strategy layer means distant realm
  seats with instantly readable identity, local economies, garrisons,
  recruitment, resource geography, and consequences visible on the map.
- A Minecraft-like foundation means first-person voxel building, mining,
  crafting, persistent terrain, understandable blocks, and player-authored
  structures.

The combination is the goal, not imitation of any one source.

## What "Minecraft-ish, but less Minecraft-ish" means

Keep the grammar players understand: one-meter-ish blocks, mining, placement,
survival, crafting, caves, generated terrain, and limitless construction.
Move the identity away from a clone through:

- larger-scale landforms and hydrology that explain where people settle;
- architecture made from faction-specific proportions, rooflines, supports,
  streets, landmarks, interiors, and props—not colored cubes alone;
- authored materials and selective 3D silhouettes layered onto voxel terrain;
- inhabitants with roles, memory, work, relationships, laws, and emergencies;
- physical production chains whose placement matters in the world;
- a named history surfaced through quests, ruins, NPC knowledge, and change;
- a restrained LOREFORGE UI, soundscape, palette, and terminology;
- a strategic map that records discovery, routes, control, danger, and trade.

The game should be recognizable as a voxel sandbox in one glance and
recognizable as LOREFORGE in the next.

## Design pillars

### 1. The world explains the mechanics

Water turns a wheel because measurable flow crosses the wheel, not because a
blue block is adjacent. A castle exists where terrain, water, defense, and
routes make sense. A trader has stock because production and travel supplied
it. Cause and effect must be visible enough for a player to learn by looking.

### 2. Factions are societies, not reputation bars

Each realm has values, laws, services, enemies, architecture, occupations,
resources, and internal disagreements. Standing is one public summary.
Personal trust, fear, evidence, warrants, promises, and witnessed history
decide actual reactions.

### 3. Place is progression

Progress is not only an inventory tier. Finding a river, restoring a mill,
earning gate access, opening a trade route, recruiting a specialist, securing
a mine, and upgrading a district all change what the player can do.

### 4. Build first; menus support the world

Power, storage, defense, crafting, and settlement upgrades should exist as
blocks, machines, people, routes, and visible construction. Menus clarify and
control them but do not replace them with disconnected spreadsheets.

### 5. One simulation, solo or together

Singleplayer uses an integrated authoritative server. Dedicated UDP and Steam
sessions use the same commands, validation, ticks, persistence, and state.
Transport changes how bytes travel, never what the game rules mean.

### 6. Depth before breadth

One castle whose gate, market, ruler, workers, guards, law, economy, and alarm
all work is more valuable than eight empty skins. One river that visibly
drives a mill is more valuable than many decorative water blocks.

### 7. Ordinary hardware is a first-class platform

Raster remains the baseline. Simulation is sparse, budgeted, deterministic,
and level-of-detail aware. Optional path tracing and high-detail assets may
enhance the experience but never define minimum playability.

## The core player loop

1. Survive and learn the immediate landscape.
2. Gather, craft, build shelter, and establish a first production loop.
3. Follow rivers, roads, rumors, and landmarks into a wider region.
4. Discover a distant settlement or capital with a unique law and need.
5. Trade, work, investigate, fight, negotiate, or break that law.
6. Build relationships and recruit people whose abilities alter play.
7. Harness geography—water, ore, farmland, routes, magic sites—to automate.
8. Affect regional safety, production, allegiance, and conflict.
9. Return home to expand a lived-in base whose history is visible.

## Beta player journey

A fresh beta save must support this coherent journey without debug knowledge:

- create a named deterministic world and spawn safely;
- complete contextual onboarding, gather, craft, build, save, and reload;
- discover a natural river and construct or restore a working water wheel;
- see water flow and machine output change when the channel is blocked,
  diverted, raised, or released;
- travel far enough that finding a major castle feels like an expedition;
- enter through a readable gate and find residents performing useful roles;
- complete a faction task, change a relationship, and understand why;
- commit a witnessed offense and see local alarm, witness reporting, and later
  policy consequences without remote omniscience;
- recruit a companion who follows, assists, waits, recovers, and remembers;
- host or join the same world and observe the same blocks, fluids, machines,
  NPCs, mobs, inventories, quests, and settlement state;
- leave and return with no duplicated rewards, lost intent, or corrupt save.

## Tone and presentation

Grounded, weathered, hopeful fantasy with industrial weight. Magic is old and
physical; machinery is noisy and situated; political conflict has human
reasons. Humor is welcome, but the world should not read as a collection of
memes or generic generated fantasy text.

## Scope boundaries for beta

- The six existing factions are the beta core. Gravebound Court and Cinder
  Host remain post-beta candidates until the six existing realms are deep.
- Beta needs one complete capital grammar per core realm, not every imagined
  upgrade tier, creature tier, siege, or diplomacy arc.
- Local water must support flow, force, wheels, dams, flumes, and entity push.
  Full Navier–Stokes, sediment transport, destructive erosion, and oceans
  simulated cell-by-cell are outside beta.
- NPCs need reliable authored/gameplay dialogue and systemic reactions, not a
  live language model in the runtime.
- Steam lobbies, invites, networking, and builds are in scope; a real App ID,
  store approval, cloud saves, achievements, and public matchmaking depend on
  external Steamworks access and can be gated honestly.
- Nuclear remains the technological ceiling. No space or sci-fi ladder.

## Product decision test

A proposed feature belongs before beta only if it strengthens at least one
step of the beta journey, uses the shared simulation, has bounded runtime cost,
and can be proven behaviorally. Otherwise it goes to the post-beta backlog.
