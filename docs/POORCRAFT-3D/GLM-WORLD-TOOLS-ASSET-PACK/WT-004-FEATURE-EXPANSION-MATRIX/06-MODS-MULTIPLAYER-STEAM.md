# Mods, Multiplayer, And Steam

Broad sandbox freedom needs modding and multiplayer rules early.

## Mods And Plugins

- TOML/JSON manifests for blocks, items, recipes, NPC roles, structures,
  quests, biomes, UI strings, and asset packs;
- semantic validation: no item without icon, no block without material, no NPC
  without interaction, no structure without bounds/collision/nav;
- mod load order and conflict reporting;
- safe local tools to inspect loaded mods.

## Multiplayer

- singleplayer and multiplayer should share world rules;
- authoritative server for world edits, inventory, combat, NPC/faction state;
- deterministic seed and structure agreement;
- chat, join/leave, player names, remote player rendering;
- latency and disconnect handling.

## Steam Readiness

- build IDs visible in UI;
- dedicated server binary;
- app id file policy;
- save folders, logs, crash reports, controller readiness later;
- no Steam-only dependency for local singleplayer.

## Proofs

- two clients see same edited block;
- server exposes seed/world metadata;
- mod validation catches bad content;
- Steam-offline fallback works.
