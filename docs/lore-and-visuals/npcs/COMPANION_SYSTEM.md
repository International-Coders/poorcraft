# Companion System Design

## The core feel

The Elder Scrolls comparison you made is apt: companions should feel like
real individuals with a faction identity and a relationship to the player
that grows from genuine shared history, not just a hired-hand economy.
The mechanics below are designed to produce that feel without requiring
voiced dialogue or cinematic cutscenes — just text, behavior, and the
existing chronicle system doing the heavy lifting.

## The relationship triangle: trust, morale, wages

Three variables govern the companion relationship:

### Trust (0–100)
How much the companion personally believes in the player.
- Starts at 0 on hire.
- High trust unlocks better cooperation, extra task slots, shared recipes.
- Trust changes slowly — it's the long-term relationship meter.
- Does NOT reset on dismiss. If a companion is dismissed and re-hired,
  trust picks up where it left off (minus 10 for the friction of returning).

### Morale (0–100)
How the companion feels right now.
- Starts at 50 on hire.
- High morale means they work better, comment positively, take on
  Working tasks readily.
- Low morale means they're sluggish, complain, and eventually refuse
  Working tasks.
- Zero morale triggers a quit.
- Morale recovers in Resting state at +5/minute real time.

### Wages (daily)
- Defined per archetype in the NPC data files.
- Paid automatically from the player's inventory at sunrise each
  in-game day. The game checks for the wage items in the first matching
  slots of the player's inventory.
- If the player can't pay: morale –10, a warning chat message from the
  companion, a chronicle entry noting the unpaid day.
- If the player pays early (via the "Pay now" command): trust +2.

## How trust grows (complete list)

| Event | Trust change |
|---|---|
| Player pays on time | +1 |
| Player pays early | +2 |
| Player completes a quest while companion is Following | +3 |
| Player defends companion from mob that would have killed them | +5 |
| Player gives companion food item (for morale) | +2 (trust cap: once/day) |
| Player uses companion for a Working task and it completes | +1 |
| Companion at zero morale (quit event) | –15 |
| Player attacks NPC of companion's faction | –10 |
| Player standing with companion's faction drops below –30 | –8 |

## How morale changes (complete list)

| Event | Morale change |
|---|---|
| Resting state | +5/min |
| Working task completed | +5 |
| Combat — player survives a fight | +3 |
| Combat — companion takes heavy damage | –8 |
| Player kills a named NPC of companion's faction | –20 |
| 2+ in-game days of continuous Working without a Resting break | –5/day |
| Wages unpaid | –10 |
| Being given food (the "gift" interaction) | +8 |
| Player morale (future design note) | mirrored at 50% weight |

## Capacity and stacking

- Maximum 3 active companions at once.
- All 3 can be in different states simultaneously (one Following, one
  Guarding the base, one Working the furnace).
- A 4th hire attempt shows: "You already have three companions — they
  need your attention as much as your coin."

## The quit event

When morale hits 0:
1. The companion stops whatever they're doing.
2. A chat message appears: "[Name] says: 'I've had enough. Find someone
   else.' [Name] has left your service."
3. A chronicle entry is written: "Your companion [Name] departed, their
   spirit worn through."
4. The companion returns to their faction's normal NPC schedule.
5. Trust drops by 15. Standing with their faction drops by 5 (word
   gets around).
6. They can be re-hired after one full in-game day, if standing is
   sufficient, but they'll remember what happened — their opening
   dialogue on re-hire is different.

## The dismiss event (player-initiated)

When the player chooses "Dismiss":
1. Chat message: "[Name] says: 'Understood. I'll make my own way.' They
   return to [faction] territory."
2. Chronicle: "You parted with [Name] at [location]. The road ahead is
   your own again."
3. Trust –0 (deliberate dismissal is respected, not punished).
4. Standing with their faction: no change.
5. Re-hire available immediately if standing is sufficient.

## Wage table (reference for NPC data files)

| Archetype | Daily wage |
|---|---|
| Accord Warden | 8 iron ingots or equivalent |
| Ironborn Artisan | 6 iron ingots or 3 coal |
| Covenant Channeler | 4 food items + 1 Anima crystal |
| Free Holds Scout | 5 food items |
| Ashen Scribe | 3 paper + 2 ink |
| Nameless Rover | 6 food items (no Accord equivalents) |

"Equivalent value" means the game checks the combined item value in the
existing item-value system — if iron ingots aren't available, the player
can substitute other items of matching total value.
