# NPC AI, Reputation, Memory, and Daily Life

NPCs should appear to pursue understandable goals in the same voxel world as
the player. They do not need unrestricted language-model autonomy. They need
reliable navigation, contextual decisions, local knowledge, memory, visible
activity, and faction-specific consequences.

## Layered NPC model

1. **Identity:** stable ID, name, faction, role, traits, home, workplace,
   commander, relationships, equipment, and voice key.
2. **Needs/duties:** safety, sleep, food, work, social, patrol, trade, ritual,
   travel, quest, and emergency obligations with priorities.
3. **Knowledge:** personally witnessed facts, told rumors, discovered routes,
   known threats, and confidence/age/source for each fact.
4. **Disposition:** faction standing plus personal trust, fear, respect,
   resentment, debt, and morale. Do not compress every emotion to one number.
5. **Planner:** deterministic utility or state-machine selection at a bounded
   cadence, with explicit interruption and recovery rules.
6. **Navigation:** route plan, local steering, collision/hop/drop/door logic,
   reservation, stuck detection, replanning budget, and safe fallback.
7. **Presentation:** gaze, facing, locomotion phase, posture, held prop,
   activity caption when targeted, bark cooldown, and animation state.

## Navigation contract

- A* or equivalent operates on a bounded, cached walkability graph; it may
  step one block, descend safely, use doors/gates, and reject lethal drops.
- Long routes are hierarchical: castle street/portal graph first, local grid
  near the actor. Never search an unbounded voxel volume per frame.
- Work is time-sliced with global and per-NPC node budgets. F3 exposes queue,
  expansions, cache hit rate, replans, and stuck recoveries.
- Dynamic obstacles trigger local avoidance/reservation before full replan.
- Doorways, ladders/stairs when supported, water capability, crowd width, and
  entity collision are part of the movement profile.
- After repeated failure, the NPC picks a safe reachable anchor and visibly
  changes plan. Teleport recovery is debug-only except for distant simulation
  with strict no-player-visible constraints.
- NPC positions and active intent persist safely across save/load.

## Daily life and reaction states

Minimum shared states: sleep, wake, eat, travel, work, trade, socialize,
patrol, guard/challenge, investigate, flee, shelter, fight, assist, mourn,
celebrate, repair, and idle-with-purpose. Each faction replaces or tunes some
activities: archive ritual, forge shift, grove tending, farm muster, grave
procession, contract court, and so on.

The player sees the cause: a guard runs toward an alarm bell, a smith carries
ore to a forge, civilians shelter behind a gate, mourners visit a grave. Avoid
random walking presented as intelligence.

## Moral history: classify before changing standing

Record immutable `MoralEvent`s with actor, target, target category, faction,
location, time/day, intent context, combat state, contract/legal state,
witnesses, evidence, and severity. Required distinctions include:

- killed hostile attacker in self-defense;
- killed enemy soldier during declared conflict;
- assassinated a named political or military target under contract;
- murdered a neutral human civilian;
- killed a faction member after surrender;
- killed undead, infernal, animal, boss, or monster;
- rescued/healed/protected a person;
- stole, trespassed, vandalized, desecrated a grave, broke a pact;
- honored a contract, showed mercy, returned property, aided a settlement;
- used forbidden magic or allied with a realm.

The combat system emits facts; reputation policy interprets them. Do not put
faction opinions inside generic damage code.

## Witness and rumor rules

- Direct witnesses need line of sight or another explicit sensory rule and a
  plausible awareness window.
- Witnesses create knowledge with confidence. They can report to guards,
  rulers, allies, or rumor routes if they survive and can communicate.
- Physical evidence may create suspicion without exact identity; disguises or
  stealth can be added later without rewriting the event model.
- Rumors decay, mutate only by defined rules, and propagate at bounded daily
  ticks. Allied realms exchange some reports; enemies may distrust them.
- The player's chronicle may know what the player did, but NPCs do not gain
  that omniscience.
- UI shows `Witnessed`, `Reported`, or `Suspected` when appropriate.

## How factions interpret a human assassin

Reactions depend on victim, context, witnesses, and realm values:

| Realm | Likely positive interpretation | Likely negative interpretation |
|---|---|---|
| Accord | lawful defense, sanctioned enemy commander | civilian murder, broken surrender, unlicensed assassination |
| Ironborn | removing a proven saboteur, defending workers | killing artisans, disrupting production, cowardly betrayal |
| Ember Covenant | stopping a despoiler, protecting living land | indiscriminate killing, burning groves, hunting sacred creatures |
| Free Holds | defending a household, overthrowing a tyrant | harming villagers, violating hospitality, hired killing for outsiders |
| Ashen Order | preventing destruction of knowledge | killing witnesses/scholars, erasing records, repeated irrational violence |
| Nameless | breaking oppressive authority, surviving by cunning | serving institutions, killing exiles, needless domination |
| Gravebound Court | delivering oathbreakers to death, grave offerings, defeating zealots | destroying bound dead, grave robbery without tribute, wasting useful lives |
| Cinder Host | fulfilled lethal contract, destabilized a rival, accepted blame | broke a bargain, killed infernal assets, acted without exploitable purpose |

Thus a player who repeatedly assassinates humans may gain fear/respect or
conditional access among Gravebound and Cinder Host contacts, especially when
victims oppose them. They do not become the player's friends automatically.
The Gravebound may view uncontrolled murder as waste; the Cinder Host may set
harder bargains and prepare betrayal. Civilian-kill history closes humane
routes, changes prices/dialogue/guards, and must have consequences.

## Response bands

Combine public faction standing with personal disposition and current threat:

- welcome/escort/honor;
- friendly service/quest/discount;
- neutral business;
- guarded/challenge/search/refuse restricted areas;
- fearful avoidance/refuse interaction;
- arrest/expel;
- hostile pursuit, with surrender/retreat rules.

Standing thresholds from existing faction data remain useful, but policy may
require fear, evidence, a warrant, or personal memory. Every transition emits
one reasoned notification and one chronicle entry at most.

## Deep tests

- Same event with no witnesses changes private moral history but not remote
  faction knowledge.
- Witness killed before reporting prevents ordinary rumor propagation unless
  physical evidence is discovered.
- Self-defense against an Accord attacker differs from civilian murder.
- Killing an Accord civilian hurts Accord/allies and can conditionally improve
  Gravebound/Cinder disposition only after information reaches them.
- Killing undead reverses Gravebound response; breaking an infernal contract
  overrides prior ruthless reputation for Cinder Host.
- Save/load preserves events, knowledge sources, dispositions, warrants, and
  active NPC intent without duplicate application.
- Schedule simulation across a full day reaches required anchors; blocked
  doors, destroyed workstations, crowds, cliffs, and alarms recover.
- Performance test caps path expansions and planning time for representative
  castle populations.
- Vistests show work, patrol, challenge, alarm/shelter, friendly greeting,
  hostile refusal, and a reported reputation change in real geometry.

Dialogue is presentation of computed state, not the state itself. Generated or
templated lines must never grant items, alter standing, or complete quests
without a validated game command.
