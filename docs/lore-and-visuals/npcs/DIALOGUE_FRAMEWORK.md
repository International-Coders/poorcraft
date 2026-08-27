# Dialogue Framework

## How dialogue works in this game (no branching engine needed yet)

The existing villager trading UI already supports an interaction model:
look at an NPC, press interact, get a screen. This framework extends that
same screen with a simple, linear dialogue layer before or instead of the
trade menu, depending on the NPC type and the player's current standing.

There is no branching dialogue tree required for the base implementation.
Dialogue is state-conditional — the NPC's current text is selected based
on the player's current standing with their faction, the NPC's role,
and any active/completed quests involving that NPC. This is simpler to
implement, still produces dramatically different conversations depending
on what the player has done, and it's honest about the game's current
scope.

## Dialogue node structure (TOML, matches the existing mod/quest pattern)

```toml
[[dialogue_node]]
npc_archetype = "accord_herald"
condition = "standing_accord < -30"
text = "You're not welcome here. Leave, before the Accord makes you."
action = "close"  # closes the interaction menu

[[dialogue_node]]
npc_archetype = "accord_herald"
condition = "standing_accord >= -30 and standing_accord < 30"
text = "What can the Accord do for you, traveler?"
action = "open_menu"  # opens the standard trade/quest menu

[[dialogue_node]]
npc_archetype = "accord_herald"
condition = "standing_accord >= 30 and standing_accord < 75"
text = "Ah, a friend of the Accord. What do you need?"
action = "open_menu"

[[dialogue_node]]
npc_archetype = "accord_herald"
condition = "standing_accord >= 75"
text = "Champion. The Accord is grateful for your service. What do you need?"
action = "open_menu_plus"  # opens the menu with exclusive faction items added
```

## Companion contextual dialogue (chat messages during play)

Companions occasionally generate a short line of text in the existing
chat UI, triggered by environmental or state conditions. These are
pulled from a TOML list per archetype:

```toml
[[companion_line]]
archetype = "accord_warden"
condition = "biome = volcanic"
text = "I don't like the heat here. The Ironborn like this place — that says something."

[[companion_line]]
archetype = "accord_warden"
condition = "standing_accord > 75"
text = "The Accord would be glad to know you're out here."

[[companion_line]]
archetype = "ironborn_artisan"
condition = "near_machine"
text = "Decent setup. Could run hotter, but decent."

[[companion_line]]
archetype = "covenant_channeler"
condition = "biome = mushroom_forest"
text = "These grew during the Long Winter. The world made something beautiful out of the Ruin. Worth remembering."

[[companion_line]]
archetype = "free_holds_scout"
condition = "structure_discovered = accord_embassy"
text = "Accord post. Don't sign anything they put in front of you."

[[companion_line]]
archetype = "nameless_rover"
condition = "standing_accord < -50"
text = "You're burning your Accord bridges. Good. They were holding you back."

[[companion_line]]
archetype = "ashen_scribe"
condition = "lore_book_found"
text = "That document — may I read it? I'll tell you what it says."

[[companion_line]]
archetype = "any"
condition = "morale < 30"
text = "I need to rest. Give me an hour."

[[companion_line]]
archetype = "any"
condition = "morale = 0"
text = "I've had enough. I'm going."
```

## Chronicle integration points

Every dialogue interaction that changes standing, completes a quest, or
involves a named NPC should produce a chronicle entry. The chronicle
entry templates:

- On first meeting a named NPC:
  `"You met [Name] of [Faction] at [Location]."`
- On completing a faction quest:
  `"You completed '[Quest Title]' for [Faction]. [Standing change phrase]."`
  Where standing change phrase = "They are grateful." (positive) or
  "The cost was worth it." (neutral) or "The relationship is strained."
  (negative to another faction).
- On a companion hire:
  `"You hired [Name/Archetype] of [Faction] at [Location]. Their wage
  is [wage]. The road is less empty."`
- On a companion quit:
  `"[Name/Archetype] left your service at [Location]. Their patience
  ran out."`
- On standing threshold crossed:
  `"[Faction] now regards you as [threshold title]. The world notices
  who you stand with."`

## Threshold titles (used in standing-crossed chronicle entries)

| Standing | Accord title | Ironborn title | Covenant title | Free Holds title | Ashen title | Nameless title |
|---|---|---|---|---|---|---|
| ≥ +75 | Accord Champion | Guild Master | Covenant Elder | Free Holds Friend | Order Scholar | The Recognized |
| ≥ +50 | Accord Ally | Guild Member | Covenant Initiate | Hold Keeper | Order Reader | The Trusted |
| ≥ +30 | Accord Friend | Guild Contact | Covenant Acquaintance | Free Traveler | Order Visitor | Known Name |
| ≤ –30 | Accord Suspect | Guild Blacklisted | Covenant Wary | Free Holds Cold | Order Flagged | The Known |
| ≤ –75 | Accord Enemy | Guild Banned | Covenant Shunned | Free Holds Hostile | Order Refused | One of Us |
