# NPC Behaviour Upgrade — Reference Document

## Current state

Villager NPCs have schedules (time-of-day routines) and trading. The
faction and companion systems were designed in the prior lore pack.
This document is about making NPCs feel like they inhabit the world, not
just stand in front of a shop waiting to be clicked.

## The goal in one sentence

An NPC who is sleeping should look asleep. An NPC who is working should
be doing something at their workstation. An NPC who last saw you steal
from them should remember that.

## Schedule implementation details

### Finding the location for each activity

NPCs need to pathfind to "the nearest bed," "the nearest chest," etc.
This requires a spatial lookup in the world's block entity data. Since
the faction structures (ashen_library, ironborn_forge_camp, etc.) are
placed by worldgen with predictable block compositions, NPCs in those
structures can be pre-assigned a workstation position at spawn time
(stored in their entity data).

Implementation approach:
1. When an NPC is spawned in a faction structure, find the nearest block
   of the "workstation" type for their archetype (Ironborn → furnace,
   Covenant → ember_glowstone altar, Ashen → bookshelf-adjacent).
2. Store that position in `NpcEntity.workstation_pos: Option<BlockPos>`.
3. During `Work` schedule activity, pathfind to `workstation_pos`.
4. For `Bed` and `Table`, find the nearest bed block and crafting table
   respectively within 16 blocks of the NPC's home structure centre.

### Pathfinding for NPCs

NPCs use the same A* pathfinder from Section B (mob_pathfind.rs). They
are not mobs, but they use the same terrain traversal logic. NPC paths
are computed less frequently than mob paths (recompute once per schedule
slot transition, not per-tick unless the path becomes invalid).

### Activity transitions

Schedule transitions are not instantaneous. When the NPC's schedule slot
changes:
1. Finish any current movement (don't teleport).
2. Set `NpcActivityState` to the new slot's activity.
3. Begin pathfinding to the new slot's location.
4. While en-route: show `Walking` animation state.
5. Upon arrival (within 1 block of target): show the activity animation state.

## NPC memory technical details

Memory is stored per-NPC in the world save, in the existing entity
serialization format (match whatever serde/bincode approach the game uses).

The `InteractionRecord` needs an `NpcEvent` enum:
```rust
pub enum NpcEvent {
    PlayerTraded,
    QuestGiven { quest_id: String },
    QuestCompleted { quest_id: String },
    QuestFailed { quest_id: String },
    PlayerAttackedNpc,
    PlayerDestroyedBlock,
    PlayerGiftedItem { item_id: String },
    CompanionDismissed,
    CompanionQuit,
}
```

Memory reference in dialogue is only triggered when:
- The player initiates interaction (right-click the NPC).
- There is a `last_interaction` record.
- The record is within 5 in-game days of the current day.
- The event is one of: `QuestCompleted`, `PlayerAttackedNpc`, `PlayerTraded`.

For other events, the NPC reacts at the moment the event occurs (the
chat message system from Section C3 handles that) but does not reference
it in future greetings.

## Dialogue posture rules

During each activity, the NPC's dialogue opening line changes:

```toml
[[npc_dialogue.activity_posture]]
activity = "sleep"
opening = "Mmh...? It's the middle of the night. Come back when the sun's up."
# NPC does not open the trade/quest menu in this state — dialogue only.

[[npc_dialogue.activity_posture]]
activity = "work"
opening = "I'm busy. What do you need?"
# Opens trade/quest menu after the opening line.

[[npc_dialogue.activity_posture]]
activity = "eat"
opening = "Can't it wait until after I've eaten? ...Fine."
# Opens trade/quest menu, but standing-change for interruption: -1 (too small
# to matter, but adds up if the player interrupts every meal).

[[npc_dialogue.activity_posture]]
activity = "socialize"
opening = "Ah, good timing. I was just thinking about [faction topic]."
# Faction-specific topic is drawn from the faction's lore data.
```

## Reaction event implementation

Reactions are triggered from event dispatch in `lf_game`. The existing
event system (whatever emits mining events, combat events, etc.) needs
these new event types:

```rust
pub enum WorldEvent {
    // existing events...
    BlockDestroyedInFactionStructure { pos: BlockPos, faction_id: String, player_id: EntityId },
    CombatStartedNear { pos: Vec3, attacker: EntityId, target: EntityId },
    PlayerDroppedItemOnNpc { item_id: String, npc_entity: EntityId },
    CompanionMoraleZero { companion_entity: EntityId, faction_id: String },
    FactionStandingThresholdCrossed { faction_id: String, threshold: i32 },
}
```

Each NPC within 24 blocks of the event position receives the event and
responds with the appropriate chat message and state change. NPCs farther
than 24 blocks do not receive the event (saves iteration cost).

## The "alive world" test

After implementing these systems, walk through an NPC settlement at three
different times of day (simulated via time-advancement in a test world):
- 3am: NPCs should be at their beds, in Sleeping state.
- Noon: NPCs should be at their workstations, in Working state.
- 7pm: NPCs should be socializing (walking between nearby NPCs), in
  Socializing state.

Write this observation as a paragraph in DEVLOG.md. It is the most
important verification for this section — not a unit test, but a human
observation that the world feels inhabited.
