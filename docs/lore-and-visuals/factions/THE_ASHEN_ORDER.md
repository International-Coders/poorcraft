# The Ashen Order

**Full name:** The Ashen Order  
**Ideology:** Knowledge, accurate record, studied neutrality — truth above
all faction interest  
**Alignment:** Neutral  
**Home biomes:** Marble highlands, deep cave  
**Color:** #b0b0b0 (pale grey, archival)  
**Symbol:** An open book  

## Who they are

The Ashen Order are the world's scholars, archivists, and cartographers.
They are genuinely neutral — not because they lack opinions, but because
they have decided that having opinions publicly is bad for the record.
An archivist who publicly sides with the Accord will be given Accord
documents and denied Nameless ones. The Order's power is access, and
access requires trust from all sides.

Their library, the Ashen Archive (E3Y55), holds copies of documents from
every faction. The Nameless occasionally burn their local copy; the Order
keeps multiple.

They are the faction most likely to help a curious player piece together
the real cause of the Ruin — not because they want the player to know,
but because the correct procedure when a credible researcher asks is to
give them the relevant document and note the access in the log.

## Two starter quests for the Ashen Order

**Quest ID: `ashen_q1_document_recovery`**  
Title: "The Missing Page"  
Issuing NPC: An Ashen Archivist in an `ashen_library` structure  
Objective type: `Collect` (find a "Torn Archive Page" item, placed as a
worldgen loot item in chests in Nameless camp structures)  
Narrative: "A page from our Third Era survey was taken — probably by
Nameless, probably meaninglessly. But it's a real document and we'd like
it back. It describes a mountain survey from E3Y180. If you happen across
it in your travels, we'd appreciate its return."  
Reward: +15 Ashen Order, the page becomes a readable lore book (triggers
the lore-book system), access to the library's reading collection, a
quest-specific chronicle entry linking the page's content to the Ruin lore.

**Quest ID: `ashen_q2_biome_survey`**  
Title: "Three New Corners"  
Issuing NPC: Same Archivist  
Objective type: `Reach` (visit 3 different biomes the player hasn't been
to before, as detected by biome-visit tracking in the chronicle system)  
Narrative: "Our maps are comprehensive but not current. We know the biomes
exist; we don't have recent accounts of what's in them. Walk into three
places you haven't been, and come back. Your observations, even informal
ones, help us."  
Reward: +15 Ashen Order, each visited biome gets a minimap label visible
from then on, a written chronicle entry per biome visited (auto-generated
from biome table data), 1 lore book from the library's collection.

## Companion type available from this faction (at standing ≥ +75)

**Ashen Scribe** — a tall, pale-grey-robed NPC carrying a journal.
Skill set: lore (reads any found lore book to the player as rich dialogue,
providing chronicle entries; identifies unknown items — a "what is this"
command returns flavor text from the NPC's knowledge), crafting of paper/
ink items, can extend the chronicle with the player's dictated notes.
Daily wage: 3 paper items + 2 ink (new craft items from the book-and-
writing recipe set).
