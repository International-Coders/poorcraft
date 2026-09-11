# Semantic Asset Rulebook

## Asset Laws

1. An asset must answer "what can the player do with this?"
2. A structure must expose entry, exit, interior, collision, nav, lighting, and
   at least one reason to visit.
3. A machine must expose inputs, outputs, work state, blocked state, progress,
   fuel/power if relevant, and user feedback.
4. An NPC must expose identity, faction, schedule, talk state, available
   actions, memory hooks, and failure states.
5. A resource node must expose harvest tool, yield, respawn/decay law, biome,
   rarity, and minimap/quest tags.
6. A UI icon must expose semantic id, contrast target, alpha policy, disabled
   state, focus state, and hotkey/text source.
7. A strategy marker must expose map scale, faction ownership, selected state,
   hover label, and fog-of-war visibility.

## Rejection Rules

Reject any asset when:

- it only has a screenshot and no data contract;
- the visual promise contradicts collision or interaction;
- a door is painted on but not enterable;
- an NPC has a body but no talk/follow/combat/trade state;
- a forge has fire but no forging action;
- a chest opens visually but has no inventory contract;
- a bridge looks walkable but nav/collision blocks the player;
- GLM cannot export bounds, anchors, wireframe, screenshot, and material data.

## Acceptance Proof

Every accepted asset needs:

- one beauty screenshot;
- one wireframe screenshot;
- one bounds/anchor overlay screenshot;
- one JSON sidecar with dimensions, anchors, materials, LODs, collision, nav,
  gameplay affordances, provenance, and proof commands;
- one runtime consumer or a clear pending consumer listed in the backlog.
