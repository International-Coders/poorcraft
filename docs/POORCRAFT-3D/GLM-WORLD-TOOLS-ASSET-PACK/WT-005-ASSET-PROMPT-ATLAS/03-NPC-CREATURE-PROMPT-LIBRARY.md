# NPC And Creature Prompt Library

NPCs need bodies, roles, readable silhouettes, dialogue anchors, and state.

## NPC Prompt Families

- villager worker: readable face, simple tunic, role prop, talk anchor, work
  animation hints, schedule tags.
- guard: helmet, shield/spear, faction colors, alert pose, patrol anchor,
  combat target, talk/intimidate state.
- trader: backpack, ledger, sample goods, market anchor, trade UI state,
  price tags.
- builder: hammer, measuring rope, plank bundle, construction anchor, request
  dialogue.
- miner: helmet lamp, pick, dust, ore pouch, mine workspot, hazard dialogue.
- farmer: straw hat, hoe, seed pouch, field anchor, crop request.
- mage: rune staff, crystal belt, talk/teach/ritual anchors, magic glow kept
  subtle and readable.
- faction leader: banner colors, command table, diplomacy dialogue, guard
  escort anchor.

## Creature Prompt Families

- forest boar, wolf, cave crawler, river serpent, ash imp, crystal wisp,
  bandit brute, armored captain, corrupted guardian, regional boss.

## Rig And Animation Rules

- humanoids need idle, walk, talk, work, attack, hurt, flee;
- creatures need idle, move, alert, attack, hurt, death or retreat;
- face/hand/weapon silhouette must survive LOD1;
- dialogue state must be visible in screenshot and export.
