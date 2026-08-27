# Factions Overview

Six factions. Every one has a coherent internal logic. The player's
relationship with each is tracked as a standing integer (–100 to +100).

## Quick-reference table

| ID | Name | Ideology | Alignment | Home biomes | Color | Symbol |
|---|---|---|---|---|---|---|
| `accord` | The Accord | Order, law, coalition | `lawful` | Plains, meadow, temperate forest | `#4a7ab5` (blue-grey) | A balanced scale |
| `ironborn` | The Ironborn | Industry, craft, pragmatism | `lawful` | Mountains, badlands, volcanic | `#8b4513` (iron-brown) | A hammer over an anvil |
| `ember_covenant` | The Ember Covenant | Nature, magic, balance | `neutral` | Highland forest, mushroom, bog | `#c4602a` (ember-orange) | A flame inside a circle of leaves |
| `free_holds` | The Free Holds | Independence, tradition, land | `neutral` | Savanna, coastal, plains (rural) | `#6b8e23` (earthy green) | Three bound wheat stalks |
| `ashen_order` | The Ashen Order | Knowledge, neutrality, record | `neutral` | Marble highlands, deep cave | `#b0b0b0` (pale grey) | An open book |
| `nameless` | The Nameless | Rejection of all compacts | `hostile` | Ruins, scorched zones, swamp | `#2d2d2d` (near-black) | A broken chain |

## Standing system mechanics (applies to all factions)

- Starting standing: 0 for all `lawful`/`neutral` factions; –50 for `hostile`.
- Range: –100 (kill-on-sight) to +100 (faction champion).
- Standing changes are logged in the player's chronicle automatically.
- Standing thresholds (applied consistently across all factions):
  - ≤ –75: NPCs flee or attack on sight. Faction structures become
    hostile territory. Quest-givers refuse to speak.
  - –74 to –30: NPCs are cold, refuse trading, give hostile dialogue.
  - –29 to +29: Neutral. NPCs follow their normal schedule. Standard
    trading available.
  - +30 to +74: Friendly. Bonus trade discounts (10%). Exclusive faction
    dialogue unlocked. Some faction quests become available.
  - ≥ +75: Honored. Exclusive faction items unlocked. Companion hire
    available from this faction. Chronicle marks the player with a
    faction title.

## Standing change events (standard, applies to all factions)

| Event | Change |
|---|---|
| Complete a faction quest | +15 |
| Fail/abandon a faction quest | –10 |
| Trade 10+ items with a faction NPC | +2 |
| Attack a faction NPC (non-combat) | –20 |
| Kill a faction NPC | –35 |
| Destroy a block in a faction structure | –5 |
| Build using faction-theme blocks near a faction hub | +3 |
| Discover a faction structure for the first time | +5 |
| An opposing faction's standing crosses +75 | –10 with this faction |

The last row creates real tension: becoming an Accord champion makes the
Nameless hate you more, and it nudges the Free Holds slightly colder
(they distrust the Accord). This is a design choice — factions should
feel like real politics, not independent reputation meters.

## Faction relationships with each other

| | Accord | Ironborn | Covenant | Free Holds | Ashen Order | Nameless |
|---|---|---|---|---|---|---|
| **Accord** | — | Allied | Tense | Cold | Neutral | Enemy |
| **Ironborn** | Allied | — | Wary | Neutral | Neutral | Enemy |
| **Covenant** | Tense | Wary | — | Friendly | Friendly | Cold |
| **Free Holds** | Cold | Neutral | Friendly | — | Friendly | Wary |
| **Ashen Order** | Neutral | Neutral | Friendly | Friendly | — | Cold |
| **Nameless** | Enemy | Enemy | Cold | Wary | Cold | — |

These relationships affect NPC dialogue and which factions' quests are
mutually exclusive (e.g. a player cannot do a quest for the Accord that
harms the Ironborn without paying a standing cost with the Ironborn —
the factions will notice).
