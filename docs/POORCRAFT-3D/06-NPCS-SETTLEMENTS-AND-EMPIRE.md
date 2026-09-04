# NPCs, Settlements, and Empire

## NPC design goal

NPCs must feel like people embedded in places, not animated markers that
beeline toward coordinates. A character should have a role, schedule, needs,
relationships, knowledge, perceptions, and reasons for changing behavior.

## Minimum NPC model

Each important NPC needs:

- identity, faction, occupation, household, and home;
- schedule and current intent;
- navigation profile and reachable destinations;
- perception events with source, location, time, and confidence;
- memory with decay and importance;
- relationships, trust, fear, obligation, and witnessed history;
- reaction policy for danger, crime, scarcity, visitors, and faction orders;
- work output or service contribution;
- persistence across save/reload and authoritative replication.

## Castle reaction chain

The desired causal chain is:

```text
player action -> NPC perception -> memory/evidence -> report or response
-> local alarm/policy -> guards, citizens, gates, faction relationship
```

NPCs must not react to events they could not see, hear, infer, or learn from a
credible witness. Conversely, an offense inside a castle should be able to
spread through reports and alter later access, patrols, prices, or quests.

## Followers and companions

Companions are useful actors, not teleporting inventory bonuses. They should:

- follow through real terrain and doors;
- avoid hazards and recover from blocked paths;
- fight, assist, wait, retreat, and resume;
- remember major events and respond to player choices;
- work at a settlement when assigned;
- remain coherent in solo and multiplayer sessions.

## Settlement layer

A settlement is a network of homes, workplaces, storage, roads, defenses,
services, and social relationships. NPC jobs should produce visible changes:
stock, repairs, meals, patrols, construction, warnings, and faction reports.

## Empire layer

The player may progress from a personal base to a territorial power through:

- claiming or negotiating control;
- connecting roads, rivers, mines, farms, and markets;
- appointing or persuading local leaders;
- managing defenses and garrisons;
- handling diplomacy, trade, law, rebellion, and rival factions;
- choosing whether expansion is peaceful, commercial, magical, or military.

Empire play must remain connected to the world. A strategic decision should
produce people, structures, routes, supplies, and consequences that can be
visited in first person.

## Simulation LOD

Nearby NPCs receive detailed perception, navigation, and animation intent.
Distant settlements use deterministic aggregate updates and arrival summaries.
When the player returns, the settlement reconciles its summary into visible
state. No distant NPC should consume an unbounded full-frequency simulation.

## NPC decision pipeline

An NPC update should be understandable as a chain:

```text
world event + schedule + needs + remembered knowledge + policy
-> choose intent -> reserve destination/resource -> navigate/act
-> produce visible result -> record memory/report if relevant
```

This model avoids the common alpha failure where an NPC changes animation but
does not have a destination, path, work product, or reason. It also makes
debugging possible: tools can show what an NPC knows, wants, and cannot reach.

## Empire is a consequence layer

The player should not select “Empire Mode” from a menu. Empire emerges once
they hold territory, services, people, obligations, and political legitimacy.
The exact army-control and founding rules remain owner decisions; see the
questionnaire before implementing the strategic layer.

## First living-capital slice

The initial proof needs only one capital, but it must include a gate, guard
policy, homes, jobs, routes, market/service, a local problem, witnessed action,
reporting, and a persistent consequence. This is the vertical slice that tests
whether castles and NPCs are truly one system.

## Capital management contract

The player directly manages one selected main capital. Citizens request food,
homes, materials, security, services, and improvements through characters and
a visual city-oversight panel. The player can solve needs with building,
crafting, trade, magic, machines, policy, workers, or refusal.

Other cities remain meaningful without becoming twenty copies of the same
management screen. Friendly cities can remain their own faction while acting
as allies, clients, puppets, or protectorates. Conquered places can be rebuilt
or delegated. Their degree of autonomy, services, and loyalty is a world
relationship, not a repaint operation.

The full city-growth rules are in `20-SEEDED-HISTORY-AND-CITY-GROWTH.md`.
