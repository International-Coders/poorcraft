# The Accord

**Full name:** The Accord of Ashenmoor  
**Ideology:** Order, coalition, law above identity  
**Alignment:** Lawful  
**Home biomes:** Plains, meadow, temperate forest  
**Color:** #4a7ab5 (muted blue-grey, like institutional stone)  
**Symbol:** A balanced scale carved in accord_stone  

## Who they are

The Accord is the closest thing Valdenmoor has to a central government.
They are not an empire — they do not claim to own the land, only to
organize it. Their founding document (the Accord of Ashenmoor, E3Y1) is
a compact of mutual recognition between peoples, not a conquest.

This is also their weakness: their authority is entirely relational.
They have power because the Ironborn respects the Coal Compact (E3Y220),
the Covenant respects the Accord's role in rebuilding infrastructure,
and most merchants prefer a stable road network to independent banditry.
When any of those relationships strain — as they are starting to in Era
IV — the Accord's authority becomes visible as the fiction it partly is.

## What they want from the player

- The player represents an opportunity: a newcomer with no existing
  faction loyalties who can go places and do things an Accord official
  cannot (because an official showing up is a political act; an
  "independent traveler" showing up is just commerce).
- Early Accord quests are framed as civic help: map a new area, deliver
  a message to a Free Holds settlement, clear a Nameless camp near a
  road. None of these are ethically complex at the start.
- Later Accord quests involve real moral weight: suppressing Free Holds
  settlers who are technically squatting on "Accord-recognized land,"
  intercepting Ember Covenant communications, or deciding whether to
  report an Ironborn violation of the Coal Compact.

## Two starter quests for the Accord (implement in the quest data files)

**Quest ID: `accord_q1_road_survey`**  
Title: "The Broken Road"  
Issuing NPC: An Accord Herald in an `accord_embassy` structure  
Objective type: `Reach` (a set of waypoints along a trade road, marking
them as "surveyed")  
Narrative: "The road between Ashenmoor and the eastern settlements has
been disrupted. We need someone to walk it and confirm which sections
are passable. No fighting required — we simply need eyes on the ground."  
Reward: 50 standing (+15 with the Accord), 3 iron ingots, a waypoint
set to Ashenmoor.  
Standing consequence: +15 Accord, no change to others.

**Quest ID: `accord_q2_nameless_camp`**  
Title: "The Camp at Miller's Crossing"  
Issuing NPC: Same Accord Herald  
Objective type: `Kill` (defeat 4 Nameless mobs) + `Reach` (return to
the Herald)  
Narrative: "A Nameless camp has established itself near Miller's Crossing,
raiding grain shipments. The Free Holds won't act — it's too close to
their territory and they distrust us. I need someone to remove it."  
Reward: +15 Accord, –5 Free Holds (they find out you did Accord dirty
work near their territory), 1 iron sword, 5 food items.  
Standing consequence: +15 Accord, –5 Free Holds.

## Companion type available from this faction (at standing ≥ +75)

**Accord Warden** — a medium-build NPC in Accord-blue clothing, carries
a sword and shield. Skill set: combat (guard duty), can craft Accord-tier
building blocks. Daily wage: 8 iron ingots or equivalent value.
