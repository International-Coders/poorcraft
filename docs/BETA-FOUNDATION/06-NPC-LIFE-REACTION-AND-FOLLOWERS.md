# NPC Life, Castle Reactions, and Followers

## Player promise

NPCs visibly pursue purposes, understand nearby events through perception,
remember important interactions, communicate knowledge, and react according to
personal and faction values. Followers travel like companions rather than
tethered props. Castle populations coordinate through alarms, roles, routes,
and shared places.

The runtime does not require an LLM. Authored and templated dialogue presents
validated simulation state; it never invents rewards or authority.

## NPC state model

```text
Identity: stable id, name, realm, role, traits, household, relationships
Anchors: home, bed, workplace, seat, post, leader, safe zone, route portals
Body: position, movement profile, health, equipment, pose, locomotion
Needs: safety, duty, sleep, food, social, work, travel, recovery
Intent: goal, target, reason, priority, start/deadline, interrupt policy
Knowledge: fact/event id, source, confidence, age, location, subject
Disposition: trust, fear, respect, resentment, debt, morale, public standing
Legal state: permissions, suspicion, warrant, surrender/escort status
Memory: durable high-value events plus bounded decaying summaries
```

Public faction standing is an input, not the whole mind.

## Decision architecture

1. Perception produces observations at a bounded cadence.
2. Observations become personal knowledge or immediate threats.
3. The settlement blackboard publishes alarms, closures, muster orders, known
   hazards, and work shortages to permitted roles.
4. A deterministic utility planner scores eligible goals.
5. Intent creates a route request and an activity/action plan.
6. Locomotion follows the route with local avoidance and animation state.
7. Completion emits a domain event and updates needs, stock, memory, or duty.
8. Failure selects a typed recovery, not endless wall pushing.

Decisions need human-readable reason codes for F3, targeted captions, tests,
and bug reports.

## Navigation

- Castle plans register a high-level graph of gates, streets, doors, stairs,
  bridges, rooms, posts, and safe zones.
- Long travel routes over that graph first; local bounded voxel A* connects the
  actor to the next portal/anchor.
- Movement profiles declare width, height, step, drop, doors, ladders, water,
  hazards, and preferred costs.
- A shared time-sliced path service owns global/per-NPC expansion budgets,
  cache versions, priority, and cancellation.
- Door and gate use is an action with permission/state, not walking through a
  solid block.
- Local steering and short reservations prevent two actors from owning the
  same doorway. Yield priority considers emergency, guard duty, cargo, and age
  only when the design data declares it.
- Stuck recovery sequence: local detour, reservation release, graph replan,
  alternate safe anchor, visible wait/call-for-help. Teleport only when far
  outside every player's interest area and explicitly logged.

The current direct-steer/sidestep locomotion remains the final actuator and
fallback while the route layer is introduced.

## Castle life

Minimum understandable loops:

- resident wakes, eats, travels, works, socializes, and sleeps at real anchors;
- worker carries an input/prop, performs an animation, and changes real stock;
- trader's stock reflects settlement production and bounded restock;
- guard changes posts, challenges restricted entry, investigates, escorts,
  raises an alarm, fights, and returns to duty;
- civilian seeks a safe zone during attack rather than running randomly;
- healer assists injured residents; mourner/celebration states respond to
  meaningful settlement events;
- leader follows audience/access rules and can issue policy/quest commands;
- destroyed beds/workplaces/gates cause visible reassignment or shortage.

Role and realm tune these loops: a forge shift, archive ritual, grove tending,
farm muster, or salvage watch must not be the same animation with a new color.

## Perception, witnesses, and knowledge

Perception channels are explicit:

- sight: distance, field of view, line of sight, light/visibility;
- sound: event loudness, obstruction approximation, alertness;
- contact: damage, collision, direct interaction;
- report: speaker, route, confidence, trust, faction relationship;
- evidence: body, damaged owned block, missing item, opened container.

A `MoralEvent` records what actually happened. A `KnowledgeRecord` records
what one NPC believes about it. Remote factions do not receive an event until
a witness/evidence/report route reaches them. Killing a witness can stop a
normal report; discovered evidence may create later suspicion.

Knowledge is bounded by priority, age, and aggregation. NPCs need not remember
every footstep. Durable memories include violence, rescue, theft, major gift,
contract, oath, quest outcome, relationship milestone, and settlement change.

## Reaction ladder inside a castle

### Suspicious anomaly

Nearby NPC looks, pauses, comments or investigates. A guard may be notified.

### Crime witnessed

Witness records the event, reacts by role/personality, and attempts to report.
The player receives a concise `Witnessed` indication when appropriate.

### Local alarm

Alarm blackboard closes or controls gates, assigns guard posts/pursuit,
redirects civilians to safe anchors, suspends normal work/trade, and gives the
leader/garrison a shared threat target with an expiry/reassessment rule.

### Resolution

The event can end through escape, surrender, fine, escort, combat outcome,
proof of innocence, leader decision, or alarm timeout. Residents return in
staggered, role-aware order. Casualties, damage, warrants, and memories remain.

### Later consequence

Reported facts feed realm policy. The same killing may be self-defense,
sanctioned war, murder, oath fulfillment, or contract breach. Outputs include
standing, trust, fear, respect, resentment, warrant, price/access, and dialogue
posture—with a reason the player can inspect.

## Dialogue

- Lines are selected from identity, intent, activity, disposition, knowledge,
  settlement state, weather, and recent events.
- Dialogue requests a validated command such as trade, hire, give quest,
  surrender, pay fine, or report fact. Text itself cannot mutate state.
- Repeated barks have per-NPC and settlement-level cooldowns.
- Important information survives as journal/rumor entries rather than being
  lost in transient chat.
- Names and voices remain stable per world seed and entity identity.

## Followers and companions

Followers use the same navigation, perception, combat, and authority as other
NPCs, plus a command intent layer:

- Follow: maintain a configurable trailing slot, use formation offsets for
  multiple companions, avoid occupying the player's body/doorway.
- Wait/guard: hold a reachable anchor, respond within a radius and return.
- Assist: defend the player or designated ally without attacking neutrals.
- Work/haul/gather: only when the role supports it, with real source,
  destination, capacity, and completion event.
- Regroup: if path distance grows, choose a route; far/off-screen catch-up may
  relocate only to a safe unseen anchor and is logged.
- Travel: mount/swim/door capabilities follow the movement profile; incapable
  followers communicate the blockage instead of disappearing.
- Relationship: trust/morale/debt/wages and memories affect compliance,
  dialogue, initiative, and departure.
- Downed/recovery behavior is preferred to sudden deletion.

## Population LOD

- Near: embodied AI, perception, route following, animation, combat.
- Mid: route-graph progress and reduced perception cadence.
- Far: household/job/schedule/economy summary ticks with stable identities.
- Activation materializes the summarized NPC at a valid anchor consistent with
  schedule and state, never in view or inside geometry.
- Population capacity comes from beds, food, work, and safety; it is not an
  arbitrary client-wide limit of 12.

## Multiplayer and persistence

The host owns NPC decisions, paths, damage, inventories, knowledge, alarms,
relationships, and companion commands. Clients receive spawn/despawn,
transform/pose snapshots, intent/activity summaries, speech/events, and state
relevant to their UI. Save/load preserves stable identity and does not respawn
a second court at the same throne.

## Required tests

- every essential role reaches bed, work/post, food/water, gate, and safe zone
  in each castle grammar;
- full-day simulation produces completed activities, not only state labels;
- locked/blocked/destroyed doors and workplaces trigger bounded recovery;
- a gate crowd resolves without overlap or permanent deadlock;
- unwitnessed crime stays private; witnessed crime reaches only plausible
  recipients; intercepted report behaves correctly; evidence can create later
  suspicion;
- self-defense, murder, contract killing, theft, rescue, and desecration yield
  different policy outputs with reasons;
- alarm reassigns guards/civilians/work and resolves without freezing the town;
- follower keeps formation, waits, guards, crosses doors/slopes, reports an
  impossible route, and catches up only under allowed unseen conditions;
- save/load preserves intent, path destination, memory, alarm, warrant, and
  relationship without duplicate application;
- representative capital population stays within planner/path budgets;
- two clients observe the same NPC action and outcome.

## Visual proofs

- `castle_workday`: several roles at distinct meaningful tasks with props.
- `castle_gate_crowd`: bidirectional traffic and reservations.
- `castle_alarm`: guards muster, civilians shelter, gate changes state.
- `npc_witness_report`: visible witness-to-guard information route.
- `npc_memory_return`: later greeting/posture reflects a prior event.
- `companion_long_follow`: route, obstacle, wait, and regroup sequence.

An NPC pass fails if characters merely move. The proof must expose purpose,
cause, and recovery.
