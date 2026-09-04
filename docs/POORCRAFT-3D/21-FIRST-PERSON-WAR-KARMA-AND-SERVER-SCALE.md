# First-Person War, Karma, Construction, and Server Scale

## First-person is the battlefield

War, defense, raids, expeditions, sieges, and dangerous territory must be
experienced in first person. The player walks, rides, flies, builds defenses,
commands their own character, enters castles, and sees the consequences on the
ground.

This does not forbid leadership. The player may set high-level intent—defend a
gate, escort a caravan, hold a bridge, patrol a road, protect workers,
evacuate civilians, or assemble a garrison—but orders are carried out by real
NPCs using real routes, supplies, morale, and limitations. This is not an RTS
where the player remotely clicks every soldier across an abstract map.

## Construction freedom and civic projects

Personal structures belong to the player’s creative freedom. They can build
their own house, workshop, fortress, laboratory, magical sanctuary, mine,
machine network, or strange experiment in the world, subject only to ordinary
materials, physics/permissions, and multiplayer rules.

City construction has two valid paths:

| Path | Player action | NPC action |
|---|---|---|
| Player-built civic project | The player designs and places the building or improvements themselves. | NPCs move in, work there, maintain it, or use it when valid. |
| Commissioned civic project | The player approves a need/project, selects a site/plan, and supplies or arranges materials. | Builders transport, construct, pause when blocked, request missing inputs, and create the visible result. |

Both paths must use the same material, terrain, ownership, navigation, and
save rules. Commissioning is not a menu that spawns a finished building; it is
a visible work project. Player freedom is never reduced because NPC construction
exists.

## Karma and NPC autonomy

“Karma” should not be a single universal good/evil number that makes every NPC
respond identically. Each faction owns its own moral baseline: it decides what
actions it praises, fears, tolerates, or treats as betrayal. Cities and people
then interpret that baseline through their own experience. The model has four
layers:

| Layer | Meaning |
|---|---|
| Personal judgment | What this NPC believes the player did to them, their people, and their values. |
| Civic standing | What a city knows or publicly believes about the player’s conduct, promises, service, and harm. |
| Faction karma base | The faction’s ideological baseline for judging conduct, law, magic, work, outsiders, and war. |
| Ideological alignment | Whether the player’s choices fit the faction/city’s declared values. |

An NPC’s decision to help, refuse, leave, betray, rebel, report a crime, or
found a rival group depends on their known evidence, relationships, needs,
fear, loyalty, and these karma layers. It must never be random punishment or
omniscient moral scoring.

The owner still needs to define the final actions each faction judges. Until
then, implement the event/memory foundation and per-faction baseline data, not
a universal final score formula.

## Player-created factions and ideology

The player may found a new faction. It must choose an ideological foundation
drawn from the world’s existing faction values, and may later combine or evolve
those values through play. This makes a new realm legible to NPCs and other
factions: people know what laws, services, rituals, war policies, labor rules,
and ambitions the new faction claims.

The ideology system is not a cosmetic color picker. It influences recruitment,
NPC expectations, city requests, diplomacy, permitted technologies/magic,
public karma, architecture, and internal conflict.

## Originality and fantasy archetypes

POORCRAFT 3D can use common fantasy archetypes: undead societies, fire-aligned
realms, knights, forests, elves, dark elves, dragons, necromancy, magic,
industrial states, and many more. The project must express them originally.

Do not ship a one-for-one recreation of a specific Heroes of Might and Magic
faction package or use its brand/identity as POORCRAFT content: that includes
game branding, distinctive faction combinations, names, lore, unit rosters,
castle layouts, artwork, UI, music, and progression. Ubisoft identifies Might
& Magic and Heroes as its brands and currently markets Heroes products. This is
production-risk guidance, not legal advice; obtain specialist IP advice before
reusing any exact name or close expression.

The practical creative rule is: **retain the broad fantasy role, create a new
POORCRAFT ideology, name, history, visual language, settlement grammar, and
gameplay purpose.**

References checked: [Ubisoft’s Might & Magic brand page](https://www.ubisoft.com/en-us/company/about-us/our-brands/might-and-magic),
[Ubisoft’s Heroes franchise page](https://www.ubisoft.com/en-us/franchises/might-and-magic),
and [Ubisoft’s trademark notice on a Heroes product](https://store.ubisoft.com/es/heroes-of-might-and-magic-6---gold-edition/56c4948688a7e300458b4784.html).

## Host-selected population scale

The host chooses the desired maximum player count, subject to the actual server
hardware and configured simulation/view-distance budget. A world may be private
solo, a small co-op, or configured toward 128 players. The engine must not
promise that 128 players works on every machine.

Scale is proven in stages:

| Stage | Target | Required evidence |
|---|---:|---|
| S1 | 1–4 players | Full gameplay authority and reconnect proof |
| S2 | 8–16 players | Interest management, NPC/terrain/water correctness, latency/loss tests |
| S3 | 32 players | Soak test, network and CPU budgets, meaningful construction/activity density |
| S4 | 64 players | Dedicated-host profiling and degraded-client validation |
| S5 | 128 players | Hardware profile, long soak, abuse/permissions test, server-admin controls, and honest published requirements |

Players outside an entity/terrain interest radius receive compact summaries;
the server remains authoritative. Player count must never cause nearby NPCs,
water, city needs, or faction state to become client-local guesses.
