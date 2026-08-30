# LOREFORGE — The 100-Step Roadmap: "Simple Winnable Wins"

Research basis: a deep comparison against Heroes of Might and Magic V
(faction towns, creature dwellings + weekly growth, initiative combat,
hero skills + artifacts, mine capture, recruit chains), Skyrim (18
use-based skills, interlocking quest/dungeon flow, crafting power
curve, discover→clear→loot→level loop), and Minecraft (explicit
progression ladder wood→iron→diamond→nether→boss, achievements,
respawnable dragon) — measured against the LOREFORGE inventory:

**Already strong**: 46 biomes, 50 community mods, 6 factions + quests,
vassal economy, machines/tech eras, magic (4 spells), boss dragon +
NullKnight, Accord Bastion city, animals, schedules, map/waypoints,
multiplayer (UDP + Steam transport behind a feature).

**The gaps this roadmap closes**: no win condition, no currency, no
farming, no doors/beds/furniture, no achievements or music, no skill
perks, no underground dungeon content, two bosses unspawnable, no mob
sync in multiplayer, act-2+ quests unwritten.

Each step is a small, testable win — one dev-loop pass. Every pass
keeps `cargo test --workspace` + vistest + smoke green.

## Phase 1 — The Win Condition (1–10)
- [ ] 1. Track `bosses_slain` counter (dragon + null knight) in ClientSave
- [ ] 2. Crown of the Vale item drops from the first dragon kill
- [ ] 3. Throne block recipe (gold + stone + banners)
- [ ] 4. Throne placement marks the "seat of power" position
- [ ] 5. Act-2 quest "Claim the Crown" (kill dragon → take crown)
- [ ] 6. Act-3 quest "Found the Bastion" (throne in a walled city)
- [ ] 7. Victory screen: credits overlay when the throne seats on the Bastion
- [ ] 8. New Game+ flag: world continues, bosses respawn stronger
- [ ] 9. Chronicle records the victory as the era's end
- [ ] 10. Vistest scene `victory_throne` proving the overlay renders

## Phase 2 — Currency & Economy (11–20)
- [ ] 11. Gold coin item (palette-ruled disc art)
- [ ] 12. Night mobs drop coins (small chance)
- [ ] 13. Coin loot in ruin/pyramid/bastion chests
- [ ] 14. Villager trades accept coins as universal payment
- [ ] 15. Coin purse HUD chip near the hotbar
- [ ] 16. Player shop block (buy/sell UI)
- [ ] 17. Claimable abandoned mines → weekly coin income
- [ ] 18. Vassal wages payable in coins (toggle)
- [ ] 19. Fast-travel waypoint activation costs coins (a sink)
- [ ] 20. Economy round-trip test (earn → spend → persist)

## Phase 3 — Farming & Food (21–30)
- [ ] 21. Farmland block (hoe tills dirt)
- [ ] 22. Seed items per crop (wheat, berry, pumpkin)
- [ ] 23. Crop growth stages (random-tick, tested)
- [ ] 24. Harvest → food + seeds back
- [ ] 25. Oven cooking recipes (bread, stew, pie)
- [ ] 26. Chicken eggs hatch chicks (life cycle)
- [ ] 27. Woolbeast shearing → wool → bed
- [ ] 28. Vassal Farmers tend crops
- [ ] 29. Food spoilage OFF by default (design note)
- [ ] 30. Vistest farm scene with growth stages

## Phase 4 — Home & Hearth (31–40)
- [ ] 31. Door block (two halves, opens on use)
- [ ] 32. Bed block: sleep skips the night
- [ ] 33. Bed sets the respawn point (tested)
- [ ] 34. Chair/table/furniture set
- [ ] 35. Reinforced chest (blast-proof variant)
- [ ] 36. Coziness score (light + furniture) → regen buff
- [ ] 37. Blueprint library for saved rooms
- [ ] 38. Vassal Carpenter builds from blueprints
- [ ] 39. Home map marker auto-set at the bed
- [ ] 40. Vistest furnished-house scene

## Phase 5 — Achievements & Music (41–50)
- [ ] 41. Achievement registry (id, title, condition fn)
- [ ] 42. 10 starter achievements (first tree, first iron, first kill…)
- [ ] 43. Unlock toast via the existing toast system
- [ ] 44. Achievement panel in the journal
- [ ] 45. Unlocked set persists in ClientSave
- [ ] 46. 30 more achievements tracing the 100-step ladder
- [ ] 47. Procedural day-music generator (scale-based calm loop)
- [ ] 48. Night/combat/boss variants crossfaded by time + threat
- [ ] 49. Music volume slider drives real gain (fix the dead slider)
- [ ] 50. Tests: unlock-once; music switches by state

## Phase 6 — Skill Tree & Perks (51–60)
- [ ] 51. Perk struct + trees (Sword / Bow / Build)
- [ ] 52. Perk points from XP levels (1 per level)
- [ ] 53. Perk panel UI (tree grid, click to unlock)
- [ ] 54. 15 perks: damage/speed/reach/hunger (5 each)
- [ ] 55. Use-based skill XP hooks (mining feeds Builder)
- [ ] 56. Path points unlock their perk branch
- [ ] 57. Perk set saves and loads
- [ ] 58. Test: a perk unlock changes stats
- [ ] 59. Vistest perk-tree scene
- [ ] 60. Mob HP rebalance around perks

## Phase 7 — Underground Dungeons (61–70)
- [ ] 61. Dungeon room stamp (corridors + rooms, stone brick)
- [ ] 62. Surface entrances (stairs down)
- [ ] 63. Loot rooms (coin/gear chests)
- [ ] 64. Spawner cages spawning glitchlings until broken
- [ ] 65. Miniboss: GeodeGuardian awakens as a dungeon boss
- [ ] 66. Miniboss: CinderCrawler in the deep tiers
- [ ] 67. Depth tiers (stone → deep slate → lava)
- [ ] 68. Dungeon symbols on the map
- [ ] 69. Radiant "clear the dungeon" quests
- [ ] 70. Vistest dungeon interior scene

## Phase 8 — HoMM Layer: Armies & Raids (71–80)
- [ ] 71. Unit stack items (followers as army chips)
- [ ] 72. Army panel: 6 stacks follow the banner
- [ ] 73. Barracks weekly stack growth
- [ ] 74. Night raids march toward player bases
- [ ] 75. Vassals man walls in sieges (auto-defense)
- [ ] 76. Initiative-lite auto-resolve battle screen
- [ ] 77. Capturable map mines (HoMM-style)
- [ ] 78. Faction war score moves with raids
- [ ] 79. Raid defense feeds the Bastion victory quest
- [ ] 80. Pure army math test (stacks beat/loss)

## Phase 9 — Multiplayer Co-op (81–90)
- [ ] 81. Protocol v5: mob spawn/despawn + drop sync
- [ ] 82. Host-authoritative mob AI ticks
- [ ] 83. Client mob rendering from network state
- [ ] 84. Shared vassal economy in co-op
- [ ] 85. Co-op quest progress sync
- [ ] 86. Socket-pair local test: host+client exchange, no accounts
- [ ] 87. Two-account live test recipe + probe flags
- [ ] 88. Steam lobby join flows into the real game connect
- [ ] 89. Map ping markers + chat emotes
- [ ] 90. Two-process block-edit sync test over a socket pair

## Phase 10 — Release Polish (91–100)
- [ ] 91. Steam achievements bridge (code ready; gated on partner AppID)
- [ ] 92. Overlay hints in the pause menu (launch through Steam)
- [ ] 93. Mesh budget guard for mod-heavy worlds
- [ ] 94. Rebinding UI covers Sprint/Crouch explicitly
- [ ] 95. First-10-minutes tutorial quest chain
- [ ] 96. Death screen + grave marker with inventory recovery
- [ ] 97. World-gen presets (peaceful / standard / hard raids)
- [ ] 98. Vistest suite at 90+ scenes
- [ ] 99. dist artifacts embed the roadmap checklist
- [ ] 100. Release notes generated from CHANGELOG

## Progress
- Phases complete: 1 / 10 (Phase 9 step 86 early-shipped)
- Steps complete: 1 / 100
- (Update this section every pass.)
