# The Nameless

**Full name:** The Nameless (they refuse any formal name — using one is
itself considered a capitulation to the compact-world's logic)  
**Ideology:** All compacts fail, all authority corrodes — freedom through
rejection of every organized system  
**Alignment:** Hostile (starting standing –50)  
**Home biomes:** Ruins, scorched zones, swamp (derelict areas)  
**Color:** #2d2d2d (near-black, ash)  
**Symbol:** A broken chain  

## Who they are

The Nameless are not simply bandits, though they raid and pillage. They
have a philosophy, and understanding it is what separates the better
Nameless NPCs from generic mob-enemies. Their philosophy, stated plainly:
every compact in history has eventually been used by those who wrote it
to control those who didn't. The Accord was supposed to protect everyone
and now argues about who owns what land. The Ironborn Guild was supposed
to be merit-based and now the old forge families run it. The Covenant was
supposed to heal the world and now it keeps its best magic secret. All
of it, eventually, becomes the same thing. So: don't sign, don't join,
don't let them give you a name and a role.

The problem with this philosophy in practice is that people still need
to eat, and raiding is how many Nameless survive. There is a genuine
tension between the "true-believer" Nameless (ideologically coherent,
usually older, often more dangerous) and the "recruited desperate"
Nameless (people who had no other option, not true believers, the ones
most likely to be persuadable by a player who offers them something).

## Can the player improve standing with the Nameless?

Yes, but it is the hardest faction to befriend and the most interesting
to do so. Standing with the Nameless can be raised by:

- Completing missions for Nameless NPCs that don't require fighting other
  Nameless (smuggling a package, finding a person, destroying a specific
  Accord marker or document).
- Demonstrating that the player also defies authority — the Nameless
  notice standing drops with the Accord and treat that favorably.
- A specific questline (two quests, below) that requires the player to
  engage with their philosophy, not just complete tasks.

The player can never reach "honored" standing with the Nameless and
simultaneously hold honored standing with the Accord — the standing-
relationship table in FACTIONS_OVERVIEW.md ensures this.

## Two starter quests for the Nameless (available only at standing ≥ –30)

**Quest ID: `nameless_q1_accord_marker`**  
Title: "Unmarked"  
Issuing NPC: A Nameless Drifter found in a `nameless_camp` — not hostile
if the player approaches without weapons drawn and standing is ≥ –30  
Objective type: `Break` (destroy 3 Accord boundary markers — the same
freeholds_daub marker blocks, relabeled as "Accord Survey Markers" for
this quest's context)  
Narrative: "They put their markers on everything. This field, that road,
this hill. It was here before they were. Break three of their stakes —
I don't care which — and come back. No one else will pay you to do this.
We will."  
Reward: +20 Nameless, –10 Accord, 8 food items (Nameless pay in food,
not crafted goods — they don't have forges).  

**Quest ID: `nameless_q2_the_philosophy`**  
Title: "What We Decided"  
Issuing NPC: A Nameless Elder, a named NPC called "The Unmarked" (one
per world, placed in the largest `nameless_camp` in the world's most
remote ruin biome)  
Objective type: `Reach` (find The Unmarked) + dialogue sequence (5
dialogue choices, each affecting standing differently — the player is
"interviewed" about their views on the factions)  
Narrative: This quest has no combat and no collection. It is a
conversation. The Unmarked asks the player why they've been helping
the Nameless. The player's dialogue choices determine the standing
outcome: thoughtful engagement earns more standing than simple agreement.  
Reward: Variable standing (+5 to +25 Nameless depending on dialogue
choices), a named lore item ("The Unmarked's Account" — a readable book
giving the Nameless perspective on the Ruin), a unique broken-chain
pendant item (cosmetic).

## Companion type available from this faction (at standing ≥ +75)

NOTE: reaching +75 with the Nameless while staying positive with any
Lawful faction is almost impossible — this companion is for players who
have made a specific political choice. The standing system will naturally
enforce this.

**Nameless Rover** — lean NPC in dark, patched clothing, no faction
symbol. Skill set: stealth (reduces mob aggro radius in a small area
around the player), lockpicking (opens Nameless camp chests that would
otherwise require a key item), combat (knife-fast, low armor). Daily
wage: 6 food items — they won't accept Accord coin equivalents.
