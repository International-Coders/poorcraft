# LOREFORGE Canon Bible

## Purpose and authority

This is the consolidated narrative constitution for LOREFORGE and the active
POORCRAFT 3D rebuild. It gathers the established world history, cosmology,
factions, NPC principles, quests, settlement logic, visual identity, and owner
direction that were previously distributed across repository documents, TOML
data, implementation packs, and earlier project conversations.

Use this bible to prevent an autonomous coding agent from turning the game
into a generic voxel sandbox, a copy of another fantasy property, a disconnected
factory simulator, or a sequence of technically impressive systems with no
place in Valdenmoor.

This document does not make every described mechanic already implemented.
Current code and tests remain the authority for implementation status. This
document is the authority for canonical identity and intended expression.

### Canon categories

- **Locked canon:** a fact that must not change without an explicit owner
  decision and a recorded migration.
- **Design law:** a binding rule for how systems express the world.
- **Established direction:** approved product intent whose exact balance may
  evolve through tests and play.
- **Planned expression:** a lore-consistent mechanic or scene that may not yet
  exist. Never claim it is implemented merely because it appears here.
- **Mystery:** intentionally incomplete information. Do not resolve it through
  narration, tooltips, omniscient dialogue, or accidental data naming.

## Identity

### Project identity

LOREFORGE is the enduring game identity. POORCRAFT is the historical repository
and prototype name. POORCRAFT 3D is the active greenfield-format rebuild within
the same creative project. Code and release branding may continue to use these
names while the owner decides final public branding, but the game world and
canon remain original.

### The game in one sentence

LOREFORGE is a first-person voxel fantasy-industrial sandbox RPG where a
vulnerable wanderer shapes a persistent world and may grow into a builder,
explorer, mage, engineer, merchant, faction champion, rebel, founder, ruler,
ally, or enemy of living settlements and empires.

### Product promise

Start with one character. Read the land. Survive. Build useful things. Meet
people with needs and beliefs. Choose paths of power. Change places, economies,
relationships, and institutions. Decide what kind of history the world
remembers.

### Originality lock

Minecraft, Valheim, Skyrim, Heroes of Might and Magic III, large modpacks, and
other games describe desired qualities only: construction freedom, atmosphere,
situated role-playing, systemic breadth, realm identity, and long-term
specialization. They are not content sources.

Do not copy protected or distinctive names, lore, characters, factions, unit
rosters, castle layouts, quests, UI composition, icons, textures, silhouettes,
models, music, dialogue, code, recipes, or progression. Every shipped asset and
piece of expression must be original or carry documented compatible provenance
and licensing.

## The player

### Identity lock

The player has no fixed origin, ancestry, race, gender, allegiance, class,
religion, or chosen destiny. The world initially knows them as a newcomer,
wanderer, traveler, or a name they choose. Titles are earned from actions and
relationships.

No prophecy makes the player important. They become important because they
survive, learn, build, keep or break promises, help or harm people, create
institutions, redirect resources, and leave visible history.

### Freedom of path

The player may become, combine, abandon, or revisit identities such as:

- hunter, fisher, farmer, cook, herbalist, or wilderness survivor;
- miner, smith, builder, architect, craftsperson, or merchant;
- explorer, cartographer, ruin-delver, scholar, or chronicler;
- mage, ritualist, enchanter, dragon-binder, or magical-industrial hybrid;
- engineer of waterworks, mills, steam, electricity, machines, control
  networks, early computers, or hazardous high-energy systems;
- companion leader, caravan organizer, diplomat, spy, mercenary, rebel, or
  faction champion;
- settlement founder, castle builder, lawgiver, protector, conqueror, or
  empire ruler;
- deliberate outsider who refuses faction membership and empire play.

These are world behaviors, not a cosmetic class picker. Progress unlocks new
decisions and responsibilities, not only stronger numbers.

### Three connected scales

1. **Personal:** survival, movement, combat, tools, crafting, building,
   exploration, magic, relationships, and first-person consequence.
2. **Settlement:** homes, work, food, workshops, farms, defenses, services,
   production, laws, alarms, social spaces, local identity, and needs.
3. **Realm:** castles, territories, economies, roads, armies, diplomacy,
   strategic resources, faction conflict, satellite cities, and war.

The scales must feed one another. A river powers a mill. The mill feeds a town.
The town supplies a castle. The castle changes law and security. NPCs operate
and judge these systems. The realm remembers what the player did.

## The realm of Valdenmoor

### World premise

Valdenmoor is a recovered but unsettled realm shaped by rivers, mountains,
forests, coasts, bogs, caves, volcanic land, ruins, trade roads, old industrial
works, Anima concentrations, and the political afterlife of catastrophe.

Every seed creates different geography and starting conditions while
preserving the same deep history and faction identities. The seed contains old
history: faction distribution, ruins, resources, routes, hazards, sites, and
political tension. The player authors the new history.

### Geographic law

World features must follow shared causes rather than appear as independent
decoration:

- mountains vary with climate, geology, biome, and seed;
- coasts form plausible beaches, marshes, forests, cliffs, plains, and harbors;
- rivers create wet corridors, travel routes, mills, farms, fisheries, flood
  risks, settlement sites, and political value;
- caves, mines, ruins, magical sites, and resources fit terrain history;
- roads connect places that exchange goods, authority, labor, or conflict;
- capitals and castles require defensible terrain, routes, water, resources,
  and faction logic;
- player edits persist separately from deterministic generation.

### Named places — locked canon

- **Ashenmoor:** the ancient river-crossing settlement, rebuilt seat of The
  Accord, and symbolic center of coalition politics.
- **The Great Smelter:** the Ashborn masterwork built into the cliff above
  Ashenmoor in E1Y214. Part ruin, part pilgrimage site, and central to Ironborn
  memory.
- **The Ember Sanctum:** the first Covenant shrine deep in the highland forest.
- **The Ashen Archive:** the marble highland library founded in E3Y55, with
  copies and competing accounts from every faction.
- **The Free Holds:** both the collective identity and the scattered independent
  settlements that refused central authority.
- **The Sundering Scar:** the crater, rift, or geographic wound associated with
  E2Y1. It is contested and not fully understood.
- **The Nameless Warrens:** derelict camps, tunnels, ruins, and hidden routes
  used by The Nameless.

New places are welcome when they fit seeded geography, faction culture, the
chronology, and gameplay. Do not casually rename these anchors.

## Cosmology

### Anima — locked canon

Magic is not divine intervention or an exception to nature. The Ember
Covenant calls the underlying phenomenon **Anima**: a material energetic flow
associated with living processes, stone resonance, and heat.

Anima can be sensed widely but shaped deliberately only through knowledge,
practice, tools, materials, and exertion. Mages are trained, not born as chosen
ones. Using magic costs energy, attention, reagents, time, bodily endurance, or
prepared infrastructure.

Magic concentrates, accelerates, redirects, stores, resonates with, or
dissipates natural forces. It does not create unlimited matter or consequence-
free power from nothing.

Examples:

- a fire effect concentrates ambient heat until ignition;
- a movement effect is an Anima-assisted kinetic push, not unexplained
  teleportation;
- a ward disperses concentrated Anima and may be less effective against a
  mundane blade;
- a crafting spell shapes or heats matter by performing work that a furnace or
  tool could also perform, with different costs;
- enchanting saturates a prepared material with controlled Anima after the
  ordinary craft exists.

### Magic and industry

Magic and machinery are parallel ways to direct energy and matter. They are
not metaphysical enemies. This creates conflict and collaboration:

- Ironborn engineers value repeatability, throughput, repair, and visible
  contracts;
- Covenant practitioners value ecological balance, resonance, stewardship,
  and consequences invisible to crude measurements;
- hybrid systems can use forged components, Anima treatment, water power,
  control networks, wards, or magical sensors;
- neither path should invalidate the other or become an isolated tech tree.

An enchanted tool still needs a real tool. A ritual may need an engineered
site. A machine may depend on a material safely prepared with Anima. The player
can build magical industry or technologically supported magic.

### The Old Powers

The Ashen Order uses **Old Powers** for ancient concentrations, entities, or
forces of Anima that predate recorded human history. They are not established
as gods. They left no reliable dogma and no complete record of intentional
communication.

Known expressions include:

- **Geode formations:** Anima crystallized in stone. A Geode Guardian forms as
  a natural defensive construct around major concentrations; it is not evil.
- **The Null:** an area or force where Anima appears absent or drained. The Null
  Knight is not established as undead; it may be an Anima-vacuum protector, a
  suit around absence, or a wound made active.
- **Dragon resonance:** dragons are vast living Anima concentrators. Their
  breath releases energy that superheats air and is distinct from ordinary
  combustion. Dragonfire may resist ordinary fire defenses while interacting
  with Anima wards.

### The Ruin mystery — locked ambiguity

The exact cause of the Ruin and Sundering is a long-term mystery. No ordinary
narrator, quest log, loading tip, faction book, item description, or NPC may
state one faction's theory as settled truth.

The Ashen Order's strongest private hypothesis is that an Ashborn mine reached
a mountain-scale concentration of Anima. Mining destabilized it. Early mages
attempted to redirect the release and partially succeeded, dispersing the event
into a multi-decade ecological disruption rather than one explosion. This is a
hypothesis, not public fact.

The player may assemble fragments from an Order text, Ironborn oral history,
Covenant inscription, terrain evidence, and contradictory testimony. Discovery
should increase understanding while preserving room for doubt.

## The Four Eras

All formal dates use `Era X, Year Y`. The game begins in **Era IV, Year 1**.

### Era I — The Age of First Flame

Approximately eight centuries before the player, two root cultures arrived
from opposite coasts:

- the **Alder-kin**, builders, farmers, and river settlers whose descendants
  influenced The Accord and The Free Holds;
- the **Ashborn**, miners, smiths, and volcanic-coast dwellers whose craft
  tradition precedes The Ironborn.

They cooperated and competed across central Valdenmoor.

Locked events:

- **E1Y12 — The Ashenmoor Compact:** the first formal trade agreement at the
  crossing of two rivers; the settlement of Ashenmoor emerged there.
- **E1Y214 — The Great Smelter of Ashenmoor:** an Ashborn industrial forge
  built into the cliff, still partially standing.
- **E1Y301 — The First Mages:** highland observers formalized knowledge of heat,
  growth, and stone resonance, beginning the tradition later called magic.

### Era II — The Age of Ruin

Approximately five centuries before the player, ecological and political
collapse reshaped the realm.

Locked events:

- **E2Y1 — The Sundering:** catastrophe destroyed Ashenmoor's upper city and
  scattered the root peoples. Accounts conflict.
- **E2Y80 — The Long Winter:** a multi-decade cold period. Mushroom forests
  expanded dramatically; the Covenant interprets this as healing.
- **E2Y200 — The Wandering Century:** fixed governance failed and nomadic life
  dominated. The Nameless identity began among people rejecting every compact.
- **E2Y340 — The Ember Covenant Founded:** survivors and scholars created a
  formal order and built the first highland shrines.
- **E2Y410 — The Ironborn Guild Chartered:** Ashborn remnants deliberately
  reorganized around craft and charter rather than blood or ethnicity.

### Era III — The Age of Rebuild

Approximately two centuries before the player, roads, settlements, institutions,
magic, industry, and political claims returned.

Locked events:

- **E3Y1 — The Accord of Ashenmoor:** Alder-descended representatives, the
  Ironborn Guild, and the Ember Covenant formed a governance compact. The Free
  Holds refused to sign.
- **E3Y55 — The Ashen Archive Founded:** scholars seeking a record independent
  of faction propaganda established the Ashen Order's library.
- **E3Y120 — The Nameless Raids:** raids threatened rebuilding; The Accord's
  military answer hardened both Accord authority and Nameless counter-identity.
- **E3Y180 — The Great Map:** the Ashen Order completed the first comprehensive
  map of the known realm.
- **E3Y220 — The Coal Compact:** The Accord and The Ironborn agreed that coal
  and later hazardous strategic extraction required joint authorization,
  preserving Ironborn political veto power.

### Era IV — The Age of Reckoning

The player arrives in E4Y1. The realm is stable enough to build and travel but
tense enough that choices matter:

- Accord authority depends on strained relationships rather than unquestioned
  sovereignty;
- The Free Holds resist new territorial and administrative pressure;
- The Ember Covenant has recovered a fragment of pre-Ruin knowledge and is
  divided over disclosure;
- The Ironborn defend their independence and face renewed questions about
  Ashborn responsibility;
- The Ashen Order gathers evidence while trying to preserve access;
- The Nameless have a new organizer whose identity remains intentionally
  unknown.

The age has no mandatory main plot. The player's chronicle becomes its history.

## The six foundational factions

### Relationship law

Every faction has an internally coherent ideology, constituency, memory, and
reason to believe it protects something important. Political conflict should
produce hard choices without flattening everyone into heroic or evil skins.

Standing ranges from -100 to +100. The Nameless begin at -50; other foundational
factions begin neutral unless a seeded or player-history rule says otherwise.
Actions can affect multiple factions because politics is relational.

Threshold intent:

- at -75 or below, territory and NPCs may become actively hostile;
- from -74 through -30, access and trade are cold or refused;
- from -29 through +29, ordinary neutral relations apply;
- from +30 through +74, friendship, discounts, dialogue, and quests expand;
- at +75 and above, honored titles, exclusive opportunities, and companion
  recruitment may unlock.

Exact numeric balance may evolve. The political meaning may not vanish.

### The Accord

- **ID:** `accord`
- **Full name:** The Accord of Ashenmoor
- **Values:** order, coalition, civic infrastructure, negotiated law
- **Alignment descriptor:** lawful
- **Home expression:** plains, meadows, temperate forests, roads, embassies,
  administrative stone, guarded crossings
- **Color:** muted blue-grey `#4a7ab5`
- **Symbol:** balanced scale

The Accord is Valdenmoor's closest central government, but it is not an
unquestioned empire. It claims to organize mutual recognition, roads, defense,
standards, and trade rather than own all land. Its authority exists because
other groups continue to honor agreements.

Its virtue is coordination after catastrophe. Its danger is converting a
voluntary compact into paper sovereignty over people who never consented. It
uses unaffiliated travelers because an official visit is political while a
newcomer's journey can look like ordinary help.

Accord play should involve roads, surveying, civic works, negotiation, public
order, disputed land, reports, guards, law, and the cost of enforcement.

Canonical starter quests:

- `accord_q1_road_survey` — **The Broken Road:** survey disrupted trade-road
  markers and report passability.
- `accord_q2_nameless_camp` — **The Camp at Miller's Crossing:** remove raiders
  near a politically sensitive boundary, gaining Accord trust and some Free
  Holds resentment.

Companion expression: **Accord Warden**, a blue-clad guard whose competence and
contract matter more than decorative armor.

### The Ironborn

- **ID:** `ironborn`
- **Full name:** The Ironborn Guild
- **Values:** industry, craft, pragmatic merit, maintainable results
- **Alignment descriptor:** lawful
- **Home expression:** mountains, badlands, volcanic land, forge camps, quarry
  roads, metal, warm dark leather, repaired ruins
- **Color:** iron-brown `#8b4513`
- **Symbol:** hammer over anvil

The Ironborn are a professional identity, not an ethnicity. Anyone can join by
learning the craft and honoring the charter. This is a source of pride and a
defense against old accusations directed at the Ashborn.

Their virtue is competence, openness to earned membership, and infrastructure.
Their danger is becoming a family-dominated guild that mistakes output for
justice and treats environmental or magical risk as someone else's concern.

The Great Smelter is their near-sacred historical site. Their public position
rejects inherited guilt for the Ruin; private memories are less certain. The
Coal Compact makes them a political partner with veto power, not an Accord
subordinate.

Canonical starter quests:

- `ironborn_q1_ore_survey` — **The Vein Finder:** demonstrate actual mining
  ability by bringing raw ore.
- `ironborn_q2_machine_parts` — **Parts for the Press:** manufacture clean
  plates and gain access to a practical production recipe.

Companion expression: **Ironborn Artisan**, able to mine and operate real
production systems when supplied, routed, paid, and safe.

### The Ember Covenant

- **ID:** `ember_covenant`
- **Full name:** The Ember Covenant
- **Values:** Anima balance, ecological healing, stewardship, teachable magic
- **Alignment descriptor:** neutral
- **Home expression:** highland forests, mushroom hollows, bogs, living wood,
  tended stones, shrines, ember light among leaves
- **Color:** ember-orange `#c4602a`
- **Symbol:** flame inside a circle of leaves

The Covenant is a knowledge and stewardship order, not a religion, even when
its rituals and places resemble worship. It studies Anima, tends Ruin wounds,
protects bogs and mushroom forests, and teaches controlled magic.

Its virtue is long-term ecological attention and a non-extractive understanding
of power. Its danger is paternal secrecy: deciding that only the Covenant is
responsible enough to know or use important discoveries.

In Era IV, members disagree over a rediscovered pre-Ruin fragment. Some favor
careful secrecy; others believe hoarding repeats the failures they criticize.

Canonical starter quests:

- `covenant_q1_healing_herbs` — **What the Bog Gives:** gather healing plants
  through knowledge of a feared ecosystem.
- `covenant_q2_ember_stone` — **Tend the Ember Stone:** visit natural Anima
  formations and place crafted signs of care rather than strip-mine them.

Companion expression: **Covenant Channeler**, providing wards, gathering,
healing knowledge, and contextual lore rather than unlimited spell damage.

### The Free Holds

- **ID:** `free_holds`
- **Full name:** The Free Holds
- **Values:** land independence, local consent, tradition, practical mutual aid
- **Alignment descriptor:** neutral
- **Home expression:** savanna, coast, open rural plains, farms, longhouses,
  boundary stones, timber, thatch, local variation
- **Color:** earthy green `#6b8e23`
- **Symbol:** three bound wheat stalks

The Free Holds are a shared refusal more than a government. Communities use the
name because they did not sign the Accord of Ashenmoor or because they later
left Accord structures after a grievance.

Their virtue is local knowledge, consent, resilience, and defense against
distant bureaucracy. Their danger is fragmentation: an inability to coordinate
against threats, protect outsiders consistently, or distinguish independence
from the power of local families.

They trade with The Ironborn, often cooperate with Covenant stewards, and
respect Ashen records more than Accord declarations. They do not accept orders.

Canonical starter quests:

- `freeholds_q1_harvest` — **Hands on the Harvest:** prove usefulness by
  bringing food, not papers or titles.
- `freeholds_q2_boundary_markers` — **Our Ground, Our Markers:** craft and place
  physical claims against Accord cartography.

Companion expression: **Free Holds Scout**, a fast bow user, tracker, and
forager whose knowledge comes from moving through land.

### The Ashen Order

- **ID:** `ashen_order`
- **Full name:** The Ashen Order
- **Values:** accurate record, evidence, access, procedural neutrality
- **Alignment descriptor:** neutral
- **Home expression:** marble highlands, deep caves, libraries, survey markers,
  maps, redundant archives, pale stone and cloth
- **Color:** pale archival grey `#b0b0b0`
- **Symbol:** open book

The Ashen Order consists of archivists, scholars, and cartographers. It seeks
truth above faction interest and guards neutrality because access to competing
records depends on trust.

Its virtue is method, preservation, and willingness to show conflicting
evidence. Its danger is confusing documentation with innocence: recording harm
without intervening can become a moral choice disguised as procedure.

The Order is the strongest route into the Ruin mystery, but even its scholars
distinguish hypothesis, source, inference, and fact.

Canonical starter quests:

- `ashen_q1_document_recovery` — **The Missing Page:** recover an E3Y180 survey
  fragment and preserve its provenance.
- `ashen_q2_biome_survey` — **Three New Corners:** update the Great Map through
  firsthand observations of unfamiliar biomes.

Companion expression: **Ashen Scribe**, identifying objects, reading texts,
recording travel, and extending the chronicle with sourced knowledge.

### The Nameless

- **ID:** `nameless`
- **Full name:** The Nameless; imposing a formal replacement name contradicts
  their philosophy
- **Values:** rejection of compacts, imposed roles, institutional permanence
- **Alignment descriptor:** hostile at the opening, not metaphysically evil
- **Home expression:** ruins, scorched zones, swamps, hidden camps, tunnels,
  repurposed debris, erased marks
- **Color:** near-black ash `#2d2d2d`
- **Symbol:** broken chain

The Nameless believe every compact becomes a tool used by its authors and
heirs against those who did not write it. They point to Accord land claims,
Ironborn forge families, and Covenant secrecy as proof that reform always
hardens into control.

Their insight is that institutions accumulate power and rewrite consent. Their
failure is that people still need food, care, defense, and durable cooperation;
raiding transfers the cost of their freedom to others.

There is tension between philosophical true believers and desperate recruits.
The latter may be hungry, displaced, coerced, or persuadable. The faction must
not become a homogeneous bandit species.

Canonical starter quests, available only after enough trust:

- `nameless_q1_accord_marker` — **Unmarked:** destroy Accord survey markers,
  gaining Nameless standing and losing Accord standing.
- `nameless_q2_the_philosophy` — **What We Decided:** speak with The Unmarked;
  thoughtful answers matter more than automatic agreement.

Companion expression: **Nameless Rover**, a fast stealth and lock-oriented
fighter who walks beside the player without pretending to become property.

### Inter-faction relationships

- Accord and Ironborn are allied through practical dependence.
- Accord and Covenant are tense.
- Accord and Free Holds are cold.
- Accord and Ashen Order are formally neutral.
- Accord and Nameless are enemies.
- Ironborn and Covenant are wary.
- Ironborn and Free Holds are neutral trading partners.
- Ironborn and Ashen Order are neutral.
- Ironborn and Nameless are enemies.
- Covenant and Free Holds are friendly.
- Covenant and Ashen Order are friendly.
- Covenant and Nameless are cold.
- Free Holds and Ashen Order are friendly.
- Free Holds and Nameless are wary.
- Ashen Order and Nameless are cold.

These relations create consequences. Honored status with one faction can cool a
rival. Quests that harm another faction must say so and change world behavior,
not only an invisible meter.

## People, NPCs, and companions

### Personhood law

NPCs are not scenery, omniscient quest dispensers, or units detached from the
first-person world. A credible person has a place, schedule, needs, reachable
anchors, knowledge limits, relationships, role, equipment, fears, and visible
responses.

NPC decisions may use:

- what they personally saw;
- what a trusted witness reported;
- civic knowledge made public;
- faction ideology and law;
- personal relationships and memories;
- hunger, safety, wages, morale, loyalty, and opportunity;
- paths, doors, terrain, workstations, storage, and real distances.

They must not know a remote crime instantly, react to client-only state, or
teleport between abstract schedules when the world says the path is blocked.

### Named figures — locked identities

- **The Unmarked:** Nameless elder and philosopher associated with the quest
  `nameless_q2_the_philosophy`. One per world.
- **Archivist Maren Voss:** primary Ashen Order contact at the Ashen Archive.
- **Foreman Dag Holtz:** Ironborn foreman at the largest forge camp.
- **The Smith:** a legendary unnamed woman in *Tome of the First Forge* who
  taught early families to work red-hill iron and river wheels. Her maxim that
  a machine is a promise links craft, maintenance, and responsibility.

Generic NPCs draw names from faction-appropriate pools and remain individuals.
Important deaths are persistent. A dead named NPC removes skills, relationships,
services, work, and possibly military strength; the city must visibly adapt.

### Companion contract

Companions are hired people, not inventory items. Up to three active companions
may follow, guard, work, rest, or act through role-specific behavior. They need
wages, morale, trust, safety, routes, and credible commands.

Trust grows through kept agreements, timely payment, shared danger, completed
work, food, and respect for the companion's people. It falls through unpaid
wages, abuse, attacks on their faction, broken commitments, or exhausting work.
Morale recovers through rest and support and falls through harm, overwork,
losses, hunger, and betrayal.

A companion at zero morale can quit, return to their faction schedule, create a
chronicle event, and remember the history if rehired. A respectful dismissal is
not betrayal.

The six companion archetypes are Accord Warden, Ironborn Artisan, Covenant
Channeler, Free Holds Scout, Ashen Scribe, and Nameless Rover. Their exact
balance may change; their faction purpose must remain recognizable.

### Dialogue voice

Dialogue is situated, concise, and aware of standing, role, local place, active
quests, witnessed actions, and knowledge. Factions speak from ideology rather
than exposition dumps.

Voice tendencies:

- Accord: civic, procedural, diplomatic, sometimes quietly coercive.
- Ironborn: direct, craft-centered, skeptical of talk without demonstrated work.
- Covenant: observant, ecological, warm but capable of guarded secrecy.
- Free Holds: local, practical, hospitable on their own terms, suspicious of
  distant claims.
- Ashen Order: precise, dry, source-conscious, deliberately qualified.
- Nameless: anti-institutional, challenging, sparse, resistant to labels.

Avoid modern meme language, generic medieval formality, encyclopedic lore
delivery, or identical hostility lines with a faction noun swapped.

## Settlements, castles, and empire

### Living-place law

A settlement or castle is not complete when walls render. It needs connected
physical and social functions:

- entrances, gates, roads, bridges, docks, and reachable interiors;
- homes, beds, storage, food, water, sanitation at the chosen abstraction;
- workshops, farms, markets, services, and real work;
- residents, guards, visitors, schedules, and social spaces;
- ownership, permissions, laws, alarms, crimes, and access decisions;
- resource inflow, production, maintenance, scarcity, and requests;
- faction architecture, symbols, materials, and historical reason;
- defense, danger, repair, politics, and chronicle consequence.

### One directly managed capital

The player may build or adopt one capital managed in detail. It has a physical
city and an oversight interface. The interface summarizes reality; it does not
replace it. A housing warning points to people and a district. An approved
project creates a site, material need, worker task, construction state, and
world result.

NPC-built civic projects and player-built projects share material, terrain,
ownership, navigation, save, and visual rules. Commissioning never spawns a
finished building from a menu without visible logistics and work.

### Bed, Work, and Idle anchors

NPC use of player-created spaces is taught through three real reachable areas:

- **Bed Box:** home, sleep, recovery, household capacity, safe return.
- **Work Box:** station, tools, storage, job type, shift activity.
- **Idle Box:** square, tavern, garden, dock, wall walk, rest, food, social life.

These anchors must validate collision, route, capacity, ownership, and safety.
They are not invisible teleport points.

### Satellite cities

Other cities retain identities and degrees of autonomy:

- allies exchange aid, trade, defense, and influence;
- protectorates or puppets grow with support and pressure while retaining
  local culture;
- conquered cities become delegated political entities rather than another
  spreadsheet clone of the capital;
- rivals continue pursuing their own needs and faction goals;
- release, rebellion, legitimacy, and negotiated independence remain possible.

### Player-created factions

The player may eventually found a faction by declaring an ideological
foundation drawn from values alive in Valdenmoor. This choice affects law,
labor, magic, technology, war, outsiders, recruitment, architecture,
relationships, and internal disagreement. It is not a color picker.

## First-person conflict, law, and karma

War, raids, sieges, defense, and expeditions happen in the first-person world.
The player can issue high-level intentions—hold a bridge, defend a gate, escort
a caravan, patrol a road, protect workers—but real NPCs follow real routes and
consume real time, morale, equipment, information, and supplies.

There is no universal omniscient good/evil meter. Judgment has four interacting
layers:

1. personal judgment from an NPC's experience;
2. civic standing from a city's known events and promises;
3. faction moral baseline from ideology and law;
4. ideological alignment between player choices and declared values.

Help, refusal, reporting, departure, betrayal, rebellion, and recruitment come
from evidence and relationships. They are never random punishment.

## Water, industry, technology, and risk

### Water law

Water is geography, travel, habitat, hazard, irrigation, food, defense, power,
and political infrastructure. The simulation uses bounded persistent flow data
rather than trying to update every water particle everywhere.

Flow should expose channel geometry, direction, slope, discharge, capacity,
revision, and interaction reach. Terrain edits dirty affected connected
sections. Unaffected rivers remain cached. Rendering conforms to the same
authoritative flow information used by wheels, pumps, navigation, fishing, and
settlements.

Water power can be a readable abstraction: several devices may read available
flow without visibly draining the river unless an explicitly approved later
design changes the rule. The abstraction must remain stable in solo and
multiplayer.

### Industrial progression

Industry begins with tools, heat, water, and craft, then can grow through mills,
presses, boilers, steam, electrical networks, automation, valves, mechanical or
early programmable control, and hazardous high-energy systems.

Every machine is a promise: supply it, route it, maintain it, understand its
failure, and it creates useful world consequences. Neglect, overload, scarcity,
pollution, fire, flooding, radiation, or political control can matter when the
system is deep enough to teach those risks clearly.

Machines should create goods, services, routes, labor changes, defense,
settlement capacity, or political choices—not only faster counters.

## Creatures and threats

Established threat concepts include ordinary wildlife and hostile creatures,
Geode Guardians, Cinder Crawlers, Glitchlings, Stalkers, Crawlers, dragons, and
the Null Knight. Implementation status varies and names found only in old code
must be verified before being treated as active content.

### Null expression

The Null is a wound or absence, not a generic undead element. Null creatures
should feel like borrowed shape, erased story, cold discontinuity, or energy
vacuum. The Null Knight is a major mystery and boss candidate, not simply a
skeleton in armor.

### Dragon expression

Dragons are living concentrations of Anima with ecology, territory, needs, and
political consequences. They may become threats, negotiated powers, research
subjects, allies, or foundations for a realm path. Do not reduce them to flying
loot containers or copy a familiar fantasy roster.

## Quests and the chronicle

### Quest law

Quests reveal needs and conflicts but do not replace sandbox freedom. An
objective must correspond to real world action: travel to a place, move goods,
build, craft, break, fight, speak, discover, repair, escort, protect, or decide.

Faction quests express ideology. Rewards and consequences are authoritative,
persistent, and visible. A quest that harms another faction changes standing,
access, dialogue, territory behavior, or later opportunities.

### Chronicle law

The chronicle records significant player-created history: meetings, discoveries,
construction, trades, promises, faction thresholds, hires, departures, deaths,
settlements, wars, machine milestones, magical discoveries, and consequences.

It distinguishes old history from new history. Dates and named events remain
accurate. Chronicle text uses earned names and titles and never assigns the
player a hidden canonical origin.

The chronicle is not a raw event log. It tells a readable story grounded in
authoritative events and cannot invent actions that did not occur.

## Canonical books

### Tome of the First Forge

This text remembers The Smith, her teaching about red-hill iron and river
wheels, and the principle that machines require care. It connects craft,
responsibility, and the chronicle tradition.

### Tome of the Null

This book describes winter glitches, borrowed-shape creatures, the empty Null
Knight, and the theory that the Null is a wound in the world's story. It is a
source inside the world, not an omniscient answer.

### The River Wardens' Ledger

This ledger remembers mill keepers, seasonal flood risk, boilers, the marriage
of fire and water, and the disappearance of the wardens. It makes water power a
human tradition rather than a disconnected tech unlock.

Future books need an identifiable author, institution, or provenance and may
be biased or incomplete. A book is a source, not automatically truth.

## Visual and material identity

### Desired feeling

LOREFORGE should feel like an original low-poly/faceted natural world with
readable voxel construction, atmospheric depth, rugged craft, living places,
and a visual record of use. It should move away from an entire landscape made
of obvious staircase cubes without losing the clarity of building and digging.

### Environment

- natural hills read as sloped or faceted land;
- caves and overhangs have volume, darkness, routes, and geology;
- rivers conform to terrain and reveal direction and use;
- forests, rocks, plants, ruins, and roads form biome and history ensembles;
- sun, fog, clouds, weather, shadows, grain, and material response are bounded
  by low-spec performance;
- settlements use architecture and wear patterns that show faction, labor,
  age, climate, and resource access.

### Faction visual grammar

- Accord: blue-grey institutional stone, measured symmetry, scales, roads,
  gates, posted law, maintained civic structures.
- Ironborn: iron-brown leather, soot, metal, repaired masonry, cranes, rails,
  forge light, pragmatic modularity.
- Ember Covenant: ember light, dark living wood, leaf circles, tended stones,
  organic paths, shrines integrated with ecosystems.
- Free Holds: earthy green, timber, thatch, farms, longhouses, hand-built local
  variation, boundary marks, open working yards.
- Ashen Order: pale stone, marble, shelves, maps, survey instruments, protected
  archives, quiet geometric clarity.
- Nameless: ash-black repurposed material, broken marks, hidden paths, erased or
  inverted institutional symbols, camps shaped by scarcity and mobility.

Visual grammar must not make every member uniform. Role, wealth, climate,
repair, age, and personal variation remain visible.

### Asset law

An asset is not complete because an image or GLB exists. It requires:

- original or licensed provenance and source/build path;
- stable identifier and coordinate/material conventions;
- appropriate texture/material budget and quality tiers;
- LOD or instancing policy;
- collision, navigation, sockets, and interaction anchors;
- runtime consumer and semantics;
- proof in the actual renderer and relevant gameplay route;
- portability and package-size consideration.

## Architecture as lore protection

Lore survives only when systems have clear authority:

- faction standing and ideology belong to authoritative world simulation;
- dialogue reads knowledge and standing but does not invent them;
- UI submits commands and presents results; it never grants rewards;
- the chronicle records accepted events from the host;
- seed generation creates starting history; player edits create persistent new
  history;
- renderer materials express faction/world state but never decide it;
- multiplayer clients cannot privately diverge on laws, quests, inventories,
  water, machines, NPCs, or terrain.

This is why engine correctness is a narrative requirement, not only a technical
one.

## Rules for extending canon

### Allowed without changing locked canon

- add a local place whose name and history fit geography and chronology;
- add an NPC with faction-appropriate but individual beliefs;
- add a quest that exposes an existing need, contradiction, or relationship;
- add a craft, machine, spell, creature, or resource grounded in world rules;
- add a faction internal debate without erasing its core ideology;
- add evidence about a mystery without resolving it;
- add architecture and visual variants with recorded provenance;
- add a player-created institution whose consequences emerge from choices.

### Requires explicit owner approval

- changing the realm name or project identity;
- changing the four-era chronology or a locked date;
- resolving the Ruin as fact;
- redefining Anima as divine, limitless, or hereditary chosen-one magic;
- deleting, renaming, merging, or replacing a foundational faction;
- assigning a mandatory player origin, destiny, class, or allegiance;
- converting the game from first-person sandbox play to menu/RTS primacy;
- importing another property's distinctive expression;
- making a major retcon that invalidates shipped quests, dialogue, saves, or
  player chronicles.

### Canon change procedure

When the owner explicitly approves a locked-canon change:

1. Write a decision entry with the old fact, new fact, reason, and migration.
2. Search code, docs, data, dialogue, quests, art manifests, tests, saves, and
   proof scenes for every affected reference.
3. Update the canon bible and primary source files in the same job.
4. Add validation preventing mixed old/new identity.
5. Migrate persistent data explicitly or state incompatibility honestly.
6. Run narrative consistency, gameplay, visual, and package proof.
7. Record the change in CHANGELOG.md and DEVLOG.md.

## Lore impact template for every job

Use this short block in implementation plans and DEVLOG entries:

```text
LORE IMPACT
- Canon touched: [exact entities or none]
- Locked facts preserved: [facts]
- World expression: [how the change appears in Valdenmoor]
- Data/dialogue/save migration: [details or none]
- Originality/provenance: [details or not applicable]
```

An optimization may legitimately say “Canon touched: none,” but it must still
avoid removing visual or simulation behavior that makes the world legible.

## Source map

This bible consolidates and does not erase the more detailed source material:

- `docs/lore-and-visuals/lore/WORLD_HISTORY.md`
- `docs/lore-and-visuals/lore/COSMOLOGY.md`
- `docs/lore-and-visuals/factions/FACTIONS_OVERVIEW.md`
- `docs/lore-and-visuals/factions/THE_*.md`
- `docs/lore-and-visuals/npcs/NPC_ROSTER.md`
- `docs/lore-and-visuals/npcs/COMPANION_SYSTEM.md`
- `docs/lore-and-visuals/npcs/DIALOGUE_FRAMEWORK.md`
- `lore/factions.toml`
- `lore/npcs.toml`
- `lore/dialogue.toml`
- `lore/quests_factions.toml`
- `lore/books.toml`
- `lore/world_events.toml`
- `docs/POORCRAFT-3D/00-DESIGN-CONSTITUTION.md`
- `docs/POORCRAFT-3D/01-PREFACE-AND-PRODUCT-DIRECTION.md`
- `docs/POORCRAFT-3D/02-DESIGN-PILLARS-AND-PROGRESSION.md`
- `docs/POORCRAFT-3D/05-WATER-FLOW-AND-ENVIRONMENT.md`
- `docs/POORCRAFT-3D/06-NPCS-SETTLEMENTS-AND-EMPIRE.md`
- `docs/POORCRAFT-3D/07-TECHNOLOGY-MAGIC-AND-INDUSTRY.md`
- `docs/POORCRAFT-3D/08-CASTLES-FACTIONS-AND-ASSETS.md`
- `docs/POORCRAFT-3D/20-SEEDED-HISTORY-AND-CITY-GROWTH.md`
- `docs/POORCRAFT-3D/21-FIRST-PERSON-WAR-KARMA-AND-SERVER-SCALE.md`

If a detail is absent here, consult the most specific current source and verify
it against code/data before treating it as active. If two sources conflict,
apply the precedence in this pack's README and record the conflict rather than
silently choosing whichever text is most convenient.
