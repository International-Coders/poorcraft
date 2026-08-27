# NPC Roster — Named Archetypes

Every NPC archetype in this file can be placed by worldgen in its faction's
structures, or found wandering in its faction's home biomes. "Named"
archetypes have a fixed name and unique dialogue; generic archetypes
are name-randomized per world.

## Named NPCs (one per world, fixed)

### The Unmarked
**Faction:** The Nameless  
**Location:** The largest `nameless_camp` in the world's most remote ruin biome.  
**Role:** Elder, philosopher, quest-giver for `nameless_q2_the_philosophy`.  
**Dialogue seed:** Speaks in measured, non-violent language. Never
threatens. Asks questions rather than giving speeches. The player who
expects a bandit lord finds something more unsettling.  
**Visual:** Older NPC skin variant, Nameless near-black clothes, no visible
weapons. Ash-grey hair.

### Archivist Maren Voss
**Faction:** The Ashen Order  
**Location:** The Ashen Archive (the largest `ashen_library` structure
in the world).  
**Role:** The Order's primary contact, quest-giver for both Ashen quests.  
**Dialogue seed:** Precise, carefully neutral language. Never says the
player did well or poorly — says their actions "are noted in the record."
Does have opinions; just expresses them as observations.  
**Visual:** Pale-grey robes, shorter build, carries a journal item.

### Foreman Dag Holtz
**Faction:** The Ironborn  
**Location:** The largest `ironborn_forge_camp` structure.  
**Role:** Ironborn primary contact, both Ironborn quests.  
**Dialogue seed:** Direct, evaluative. Compliments are a single word.
Criticism is a full sentence. Not unkind — just working-class efficient.  
**Visual:** Ironborn-brown leather apron, stocky, forge-soot on face
(texture detail on the NPC skin's face region).

---

## Generic hireable archetypes (name-randomized, many per world)

### Accord Herald
**Faction:** The Accord  
**Role:** Quest-giver, trader (sells Accord building blocks, maps).  
**Hireable as:** Accord Warden (at standing ≥ +75)  
**Spawn:** 1–2 per `accord_embassy` structure.  
**Skill set when hired:** Combat, Accord block crafting.  
**Opening hire dialogue:** "The Accord pays well and asks little — follow
the law and keep to your contract. Can you do that?"

### Ironborn Artisan
**Faction:** The Ironborn  
**Role:** Trader (sells iron/ore items, crafting recipes), uses the
forge_camp's furnace autonomously when not hired.  
**Hireable as:** Ironborn Artisan  
**Spawn:** 1 per `ironborn_forge_camp`.  
**Skill set when hired:** Smelting, crafting, mining.  
**Opening hire dialogue:** "I work for coin and good iron. Don't waste
either and we'll get along."

### Covenant Herbalist
**Faction:** The Ember Covenant  
**Role:** Trades potions, herb items, magic-reagent materials.  
**Hireable as:** Covenant Channeler  
**Spawn:** 1–2 per `covenant_grove_shrine`.  
**Skill set when hired:** Herb gathering, minor healing/ward support.  
**Opening hire dialogue:** "The world is not a place to move through
alone. Walk with me if you want, wanderer."

### Free Holds Elder
**Faction:** The Free Holds  
**Role:** Quest-giver. Does not trade (Free Holds barter, they don't
run shops — use the existing trade UI but configured as barter exchange).  
**Hireable as:** Free Holds Scout  
**Spawn:** 1 per `freeholds_longhouse` (the Elder is the longhouse's
named occupant; a Scout is a separate generic NPC nearby).  
**Skill set when hired:** Ranged combat, foraging, tracking.  
**Opening hire dialogue:** "You've proven you're not just another
Accord messenger. I'll walk with you — for a fair share of what we find."

### Ashen Archivist (generic)
**Faction:** The Ashen Order  
**Role:** Trader (lore books, paper, ink, maps). Reads found lore books
aloud to the player if they bring one to the archivist (triggers the
lore-book reading UI from the player's perspective).  
**Hireable as:** Ashen Scribe  
**Spawn:** 1 per `ashen_library`.  
**Skill set when hired:** Item identification (flavor text), chronicle
extended by dictation, paper/ink crafting.  
**Opening hire dialogue:** "I record events. Traveling with you would
produce... considerably more of them."

### Nameless Drifter
**Faction:** The Nameless  
**Role:** Quest-giver (only if standing ≥ –30), hostile below that.  
**Hireable as:** Nameless Rover (only at standing ≥ +75 with Nameless)  
**Spawn:** 2–4 per `nameless_camp`. Hostile by default; NPC attitude
changes at standing thresholds.  
**Skill set when hired:** Stealth, lockpick, knife combat.  
**Opening hire dialogue:** "I don't work for anyone. But I'll walk next
to you — for now."
