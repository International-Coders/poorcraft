# IDEAS-600 — 300 Missing Features + 300 Upgrades

*Generated 2026-09-01 at loop 337 from a full screenshot test run + visual audit of all
83 vistest scenes + a code inventory of the current build. Every idea is grounded in
something the screenshots show or the code confirms.*

## How this was produced (methodology)

1. **Test run**: `cargo run --release -p xtask -- vistest shots` re-rendered all **83
   proof scenes** to `shots/vistest_*.png` (83/83 `[ok]`, exit 0). Six extra-seed
   variety shots of gameplay scenes were added (`shots/extra_*.png`, seeds 777–55555).
2. **Visual analysis**: every one of the 83 PNGs was read and analyzed (3 parallel
   review passes covering the full set, plus manual reads of the 8 most load-bearing
   shots). Findings per scene fed the grounding tags below.
3. **Code inventory**: crates were audited for implemented systems (46 biomes, ~139
   vanilla blocks + 100 mod blocks, 159 items, 13 mob types, villagers/companions/
   vassals, 6 factions, quests, spells, 8-era research, 14 machines, UDP+Steam net,
   TOML mods) and for known gaps (BACKLOG deferrals, ROADMAP-100 phases).
4. **Dedup rules**: ideas tagged `extends R##` build on a ROADMAP-100 step and go
   **beyond** it; the roadmap's own steps are not re-listed. `gap` marks a BACKLOG
   deferral or code-level known-missing item. `shot:` cites the screenshot that
   motivates the idea.

**Tag format**: `[effort S|M|L / impact H|M|L]` followed by optional grounding tags.

**A visual-quality caveat from the audit**: the dominant white/gray "X" sprites
carpeting outdoor shots are the cross-quad **ground-cover plants** at extreme density
(see `plants_cross`, `biome_contact_sheet`), not mobs — but they read as noise at any
distance, which is why a ground-cover art/density pass is the top visual win. Vistest
scenes are deterministic reconstructions (real worldgen/mesher/renderer/mob sim), so
scene *composition* issues (floating platforms, empty sky) are noted separately in the
appendix from game-content issues.

---

# PART 1 — 300 MISSING FEATURES (M001–M300)

## World & Nature (M001–M020)

- **M001 Seasons cycle** — four seasons re-grade biome palettes, change crop growth speed and mob spawn tables, and are visible from the menu orbit. [L/H]
- **M002 Fishing** — craftable rod, fish species per biome and water depth, cooking recipes, fishing minigame. [S/H]
- **M003 Lightning** — storms strike the tallest nearby block: fire ignition, glass fuse-sand, charged-mob variants. [S/M]
- **M004 Wind system** — a global wind vector drives particle drift, tree-sway amplitude, smoke bend and future sailing. [M/M]
- **M005 Natural disasters** — rare earthquakes (tunnel cracks), spreading wildfires, multi-day droughts with crop wither. [L/M]
- **M006 Erosion & regrowth** — exposed dirt slowly greens over; riverbanks crumble under water flow. [L/L]
- **M007 Water currents** — rivers push swimmers, boats and floating items downhill with visible flow arrows. [M/M]
- **M008 Temperature & clothing** — biome heat/cold drains comfort; parkas and desert wraps as clothing slots. [M/M]
- **M009 Moon phases** — an 8-phase moon modulating night brightness and hostile spawn rate. [S/M]
- **M010 Aquifers & flooded caves** — underground lakes with blind-fish and air pockets as cave landmarks. [M/M]
- **M011 Volcano biome** — rare mountain with lava vents, basalt columns, sulfur ore. [L/M]
- **M012 Geysers & hot springs** — geothermal vents (damage + power potential) and comfort-buff soaking pools. [S/M]
- **M013 Beehives & beekeeping** — wild hives in flowering biomes, honeycomb harvest, tameable apiary (extends R21–30 farming). [M/H]
- **M014 Ambient insects** — butterflies by day, fireflies in marshes at night, purely atmospheric but density-tunable. [S/M]
- **M015 Aurora** — cold-biome night sky event, cheap shader win with big atmosphere payoff. [S/M]
- **M016 Rainbows** — post-rain arc tied to a luck charm trigger. [S/L]
- **M017 Tides** — coastal water level oscillation exposing tidal pools with unique loot. [M/L]
- **M018 Meteor showers** — announced night event; fallen-star blocks spawn and can be mined before despawning. [M/M]
- **M019 Falling leaves & pollen** — ambient particles under canopies; strengthens the existing GPU foliage sway. [S/M]
- **M020 Snow drifts** — accumulating thin snow layers on surfaces in cold biomes (pairs with U184). [M/M]

## Creatures & Wildlife (M021–M040)

- **M021 Taming** — feed wolves/boars to tame; sit/stay/follow pet commands. [M/H]
- **M022 Animal breeding** — feed two animals to produce an offspring that grows through stages (extends R27's chicken cycle to all species). [M/H]
- **M023 Fish entities** — visible fish schools in rivers/oceans, catchable, bucketable. [M/M]
- **M024 Birds** — small flying ambience that lands, chirps and drops feathers; nest blocks in trees. [M/M]
- **M025 Land mounts** — a rideable horse-class animal with saddle, reins and speed tiers (the dragon is currently the only mount). [L/H]
- **M026 Frogs & pond life** — marsh critters that snap at flies; poison ingredient source. [S/L]
- **M027 Rabbits & squirrels** — small fleeing critters with hide drops, burrow holes in plains. [S/M]
- **M028 Animal aging** — young→adult→elder size scaling affecting drops and speed. [M/L]
- **M029 Predator–prey simulation** — wolves hunt boars; carcasses attract scavengers; ecology depth. [M/M]
- **M030 Herd migration** — animal groups drift seasonally between biomes (needs M001). [M/L]
- **M031 Nests & eggs** — collectible wild eggs that hatch near a warm hearth. [S/M]
- **M032 Guard animals** — trained dogs assigned to a settlement patrol slot. [M/M]
- **M033 Pet accessories** — collar naming, pet bed, follow-distance toggle. [S/M]
- **M034 Legendary species** — one shy rare animal per biome class; trophy mounting on defeat-by-capture. [M/M]
- **M035 Locust swarms** — jungle/marsh hazard events that strip crops (bridges farming). [S/L]
- **M036 Pack-mule pets** — a haul animal companion increasing carry capacity (distinct from hirelings). [M/M]
- **M037 Zoo & pen detection** — enclosures with named animals register as a zoo granting small charm buffs. [M/L]
- **M038 Wildlife photography** — photo-mode achievements tied to sighting each species (pairs with M290 bestiary). [M/L]
- **M039 Bestiary** — auto-filling illustrated encyclopedia of every species seen, with habitat notes. [S/H]
- **M040 Troughs & feed** — placeable feed stations keeping penned animals fed and breeding-ready. [S/M]

## NPCs & Settlements (M041–M060)

- **M041 Wandering merchants** — traveling traders with rare rotating stock appearing on roads. [M/H]
- **M042 Trade caravans** — pack-train convoys between settlements; escort them or raid them. [L/H]
- **M043 Marriage & households** — NPCs pair up, merge homes, share schedules. [L/L]
- **M044 Children** — village kids who play, then grow into working adults across in-game days. [L/M]
- **M045 Bounty board** — settlement job board with timed contracts (hunt, deliver, build). [M/H]
- **M046 Town hall** — an upgradable settlement building that unlocks services (bank stall, caravan origin, quests). [M/M]
- **M047 Festivals** — periodic village feasts: music, food, discounts, mini-contests. [M/M]
- **M048 NPC adventurers** — independent heroes who delve dungeons and compete for your fame. [L/M]
- **M049 Messenger service** — paid item delivery between settlements; arrivals appear in the chronicle. [M/M]
- **M050 Hermit sages** — lone wilderness NPCs holding unique knowledge, maps or trades. [S/M]
- **M051 Black market** — nameless-faction underground trade in "hot" goods with standing risk. [M/M]
- **M052 Skill trainers** — pay NPCs for shortcut lessons (spell hints, perk discounts). [M/M]
- **M053 Gossip network** — your deeds travel between settlements and change prices/standings (pairs with U030). [M/H]
- **M054 Settler recruitment** — convince NPC pairs to found a new hamlet where you designate. [M/M]
- **M055 Named faction leaders** — unique leader NPCs with audience mechanics and edicts. [M/H]
- **M056 Refugees** — world events spawn refugee groups seeking shelter; house them for standing. [M/M]
- **M057 Settlement watch** — hire stationable guards beyond your 4-companion cap. [M/M]
- **M058 Pilgrims** — travelers visit shrines; bless or rob them with standing consequences. [S/M]
- **M059 Rival company** — an NPC-run industrial firm that buys ore, undercuts shops and can be outcompeted. [L/M]
- **M060 Census screen** — per-settlement roster of residents with jobs, mood and needs. [S/M]

## Combat & Enemies (M061–M080)

- **M061 Ranged enemies** — archer/slinger mobs that keep distance and use line-of-sight (all 13 mob types are melee today). [M/H]
- **M062 Shields & parry** — offhand block slot; timed parry staggers attackers. [M/H]
- **M063 Dodge roll** — i-frame dash on a stamina cost; the single biggest combat-feel gap. [S/H]
- **M064 Status effects** — poison, burn, freeze, bleed with HUD icon row (pairs with U139). [M/H]
- **M065 Throwable weapons** — spears, throwing axes, tar jars, crude bombs. [M/M]
- **M066 Placeable traps** — spike strips, tripwires, bear traps for base defense. [M/M]
- **M067 Mounted combat** — attack/breathe from dragon-back; melee from land mounts (pairs with M025). [M/M]
- **M068 Positional damage** — backstab/critical-hit multipliers with directional hit feedback. [S/M]
- **M069 Elite affixes** — rare prefix mobs (Burning, Frozen, Ancient) with auras and better loot. [S/M]
- **M070 Enemy camps** — handcrafted bandit dens with a leader, chests and patrol routes (the nameless camp is a bare block ring — shot: `nameless_camp`). [M/M]
- **M071 Night ambushes** — coordinated hunting parties that flank and retreat, scaling with your gear. [M/M]
- **M072 Arena challenges** — claimable arena sites running wave trials for rewards. [M/M]
- **M073 Bounty targets** — named rare mobs spawned from bounty papers. [M/M]
- **M074 Grappling hook** — traversal tool that also yanks light enemies. [M/M]
- **M075 Weapon archetypes** — halberd reach, dagger speed, whip range, hammer stagger (only sword/bow exist). [M/H]
- **M076 Unarmed path** — brawler skill line making bare-fist viable. [M/L]
- **M077 Combat stances** — toggleable offense/defense stances with tradeoffs. [M/M]
- **M078 Adrenaline meter** — builds on hits, spends on a burst ability. [M/L]
- **M079 War horns** — rally/buff instruments for siege play (bridges R71–80 armies). [S/M]
- **M080 Duels** — consensual PvP sparring with coin wagers. [S/M]

## Bosses & Dungeons (M081–M100) — extends R61–70

- **M081 World boss events** — a roaming server-wide boss announced in chat and on the map. [L/M]
- **M082 Mechanic arenas** — boss rooms with pillars, pressure plates and phase geometry. [L/H]
- **M083 Underwater dungeon** — flooded vault navigated via air pockets. [M/M]
- **M084 Sky-island dungeon** — cloud ruins reachable by flight or towering builds. [M/M]
- **M085 Maze dungeon** — shifting hedge/stone maze with a minotaur-class guardian. [M/M]
- **M086 Puzzle rooms** — rune/plate logic puzzles gating loot rooms. [M/M]
- **M087 Dungeon keys** — locked doors, key items, a boss key per tier. [S/M]
- **M088 Crypts & graveyards** — undead-flavored surface mini-dungeons with epitaph lore. [M/M]
- **M089 Haunted mansion** — above-ground dungeon with ghostly resident NPCs. [M/M]
- **M090 Abandoned mine rails** — rideable cart segments through a dungeon. [M/L]
- **M091 Lava fortress** — deep bastion tier with fire-trap corridors (beyond R67 depth tiers). [M/M]
- **M092 Treasure maps** — dungeon-found maps leading to buried caches. [M/M]
- **M093 Boss trophies** — mountable heads/relics granting small permanent buffs. [S/M]
- **M094 Secret rooms** — suspicious-wall hideaways with rare loot. [S/M]
- **M095 Dungeon escort quests** — escort a scholar in and out alive (beyond R68 clear-quests). [M/M]
- **M096 Boss summon effigies** — craftable re-fight items for farmed uniques. [S/M]
- **M097 Party-scaled replays** — dungeon difficulty scaling with group size. [M/L]
- **M098 Trap rooms** — dart walls, collapsing floors, swinging logs. [M/M]
- **M099 Dungeon guardian spirit** — a neutral NPC offering riddles and wagers inside dungeons. [M/M]
- **M100 Dungeon repopulation** — cleared dungeons refill after N in-game days. [S/M]

## Magic & Arcane (M101–M120) — extends the 4-spell baseline

- **M101 New spell schools** — ice (freeze), storm (chain lightning), stone (wall). [M/H]
- **M102 Ritual magic** — multi-item circles for big cast effects, multiplayer ceremonies. [M/M]
- **M103 Summoning** — familiar pets (wisp, ember-imp) with upkeep costs. [M/M]
- **M104 Spell crafting** — combine runes into hybrid spells (extends the 2 existing runes). [M/H]
- **M105 Mana economy items** — mana potions, regen-enchanted gear. [S/M]
- **M106 Staff/wand tiers** — cast-focus items with gem slots changing spell behavior. [M/M]
- **M107 Curses** — cursed items and shrines requiring lifting quests. [M/M]
- **M108 Ley lines** — worldgen mana nodes that amplify spells cast nearby. [M/M]
- **M109 Wizard guild** — joinable arcane order with rank quests (beyond the single tower archetype). [M/M]
- **M110 Portal network** — linked teleport pads with fuel cost. [M/H]
- **M111 Divination** — a scrying spell that pings ores/structures onto the map. [S/M]
- **M112 Weather magic** — clear-skies and summon-rain rituals (ties into weather system). [S/M]
- **M113 Illusions** — decoy clones that distract mob AI. [M/L]
- **M114 Enchant families** — protection/fortune/haste enchant categories on gear (extends the imbue minigame). [M/H]
- **M115 Updraft columns** — anti-gravity zones for building and mobility. [M/L]
- **M116 Dungeon-exclusive tomes** — rare spells only found in dungeon loot (bridges R61–70). [S/M]
- **M117 Mana beasts** — creatures visible only while a ward is active. [M/L]
- **M118 Astral projection** — risky out-of-body scouting mode. [M/L]
- **M119 Rune dictionary** — collectible glyphs decoding ancient texts (bridges M209). [M/M]
- **M120 Ward totems** — placeable protection fields against undead-class enemies. [S/M]

## Technology & Machines (M121–M140) — extends the 14-machine baseline

- **M121 Logic circuits** — AND/OR/NOT/XOR blocks with wiring (extends the G-key power grid). [M/H]
- **M122 Sensors** — daylight and motion sensors feeding logic circuits. [S/M]
- **M123 Sorting machine** — filtered item routing into chests. [M/H]
- **M124 Quarry** — multiblock auto-miner with an animated frame. [M/H]
- **M125 Field harvester** — crop automation for farms (extends R21–30 farming). [M/M]
- **M126 Trains & freight rails** — rideable carts, powered rails, cargo wagons. [L/H]
- **M127 Pneumatic tubes** — capsule item transport with visible pods. [M/M]
- **M128 Worker drones** — flying haulers for factory logistics. [M/M]
- **M129 Solar arrays** — clean daytime generation tier. [M/M]
- **M130 Wind turbines** — large windmill variant whose output follows the wind (M004). [M/M]
- **M131 Wireless power** — short-range tesla transfer coils. [M/L]
- **M132 Battery tiers** — higher-capacity storage beyond the single BatteryCell. [S/M]
- **M133 Pollution** — combustion/reactor output fouls nearby air/water until scrubbed. [M/M]
- **M134 Waste & recycling** — byproducts needing disposal or a recycler machine. [M/L]
- **M135 Programmable computer** — scriptable monitor/alarm programs on the existing Computer block. [M/M]
- **M136 Remote access terminal** — open any machine's UI from a linked terminal (pairs with U232). [M/M]
- **M137 Mass production lines** — blueprint-driven assembler batching (extends the blueprint system). [M/M]
- **M138 Refrigeration** — food-preserving machine (ties into R30 spoilage switches). [M/M]
- **M139 Hot-fluid pipes** — lava/crude transport with heat loss over distance. [M/M]
- **M140 Factory planning overlay** — ghost-link machines into visible production lines (extends the grid overlay). [M/L]

## Farming & Food (M141–M160) — extends R21–30

- **M141 Irrigation** — water-channel moisture that speeds crop growth. [S/M]
- **M142 Sprinklers** — machine-assisted watering covering a radius. [M/M]
- **M143 Greenhouses** — glass domes enabling off-season and wrong-biome crops. [M/M]
- **M144 Fruit orchards** — apple/cherry trees with multi-harvest seasons. [M/M]
- **M145 Vineyard & brewery** — grapes into wine, grain into mead, casks that age. [M/M]
- **M146 Mushroom caverns** — dark-farm fungi varieties in caves. [S/M]
- **M147 Paddy & kelp crops** — underwater rice/kelp farming. [M/M]
- **M148 Composter** — converts waste to fertilizer. [S/M]
- **M149 Fertilizer tiers** — bone/ash/compost growth boosts with different profiles. [S/M]
- **M150 Seasonal crops** — crops with preferred seasons (requires M001). [M/L]
- **M151 Scarecrows** — pest-prevention radius for crop plots. [S/L]
- **M152 Dairy** — woolbeast milking, butter and cheese crafting chains. [S/M]
- **M153 Cooking breadth** — soups, stews, pies beyond the oven basics of R25. [M/M]
- **M154 Food buffs** — well-fed tiers granting regen/stamina by meal quality. [M/H]
- **M155 Preservation** — smoking/salting to extend shelf life once spoilage exists. [M/M]
- **M156 Feast table** — shared meal giving a group buff (bridges co-op). [S/M]
- **M157 Beeswax products** — candles and polish from M013 hives. [S/M]
- **M158 Crop quality grades** — star-rated harvests from care and soil. [M/M]
- **M159 Market day** — weekly event where food sells at a premium. [S/M]
- **M160 Orchard automation** — vassal farmers also tend trees (extends R28). [M/L]

## Building & Mechanisms (M161–M180)

- **M161 Paint & dye** — recolor wood/stone families at a dye station. [M/H]
- **M162 Carpets & rugs** — thin decor layers with pattern variants. [S/M]
- **M163 Curtains & hanging banners** — cloth decor with wind sway. [S/M]
- **M164 Chandeliers** — multi-light fixtures for great halls. [S/M]
- **M165 Drawbridges** — collapsible plate spanning moats/gates. [M/M]
- **M166 Portcullis & gates** — sliding bars with lever/remote control. [M/M]
- **M167 Pistons & pushers** — block-moving mechanisms for contraptions. [L/H]
- **M168 Ropes & ziplines** — traversal lines between anchor points. [M/M]
- **M169 Glider & parachute** — fall-arrest travel tools. [M/H]
- **M170 Boats** — watercraft with chest storage for river trade (pairs with M007). [M/H]
- **M171 Wrench** — rotate slabs/stairs, pick block variants. [S/M]
- **M172 Custom portals** — player-framed teleport gates (beyond the M110 prefab pads). [M/M]
- **M173 Mosaic tiles** — pattern-stamped decorative blocks. [S/L]
- **M174 Street lamps** — post+lantern prefabs with dawn/dusk sensors. [S/M]
- **M175 Chimneys** — hearth smoke vents (visible from village shots' perspective). [S/M]
- **M176 Fences & low walls** — half-height barrier set for yards. [S/M]
- **M177 Furniture kit 2** — shelves, cabinets, wardrobes (extends R33 chair/table set). [M/M]
- **M178 Mirror blocks** — reflective surfaces the path tracer already renders well. [M/L]
- **M179 Garden kit** — hedges, topiary, flowerbeds for estates. [S/M]
- **M180 Building rating** — an aesthetics score granting coziness-style buffs (extends R35). [M/L]

## Economy & Trade (M181–M200) — extends R11–20

- **M181 Auction house** — global listing board with listing fees (extends R15 player shops). [M/H]
- **M182 Supply & demand** — prices drift with local scarcity per settlement. [M/M]
- **M183 Trade contracts** — timed deliver-X-to-Y jobs paying coins. [M/M]
- **M184 Banking** — deposits with interest, loans with collateral risk. [M/M]
- **M185 Protection taxes** — pay a faction for territory safety discounts. [M/L]
- **M186 Trade routes** — recurring caravan escort jobs (bridges M042). [M/M]
- **M187 Toll gates** — player-built tolls on road chokepoints. [S/L]
- **M188 Rentable stalls** — market points in settlements for your goods. [S/M]
- **M189 Pawn & salvage** — sell used tools; disassemble gear into parts. [M/M]
- **M190 Repair economy** — anvil repairs cost materials + coins, feeding sinks. [S/M]
- **M191 Insurance** — coverage subscriptions refunding lost inventory. [M/L]
- **M192 Rarity resale** — ornate-tier items gain provenance and resale value. [M/L]
- **M193 Investment deeds** — fund NPC mines/farms for dividends (extends R17 claimable mines). [M/M]
- **M194 Price rumors** — NPC gossip reveals market swings before they happen. [S/M]
- **M195 Tariffs & smuggling** — faction borders tax goods; secret routes avoid it. [M/L]
- **M196 Currency denominations** — copper/silver/gold tiers (extends R11 coin). [S/M]
- **M197 Mint** — press raw gold into coins at a machine. [S/M]
- **M198 Counterfeits** — black-market coins that traders can flag. [M/L]
- **M199 Trade treaties** — standing-gated faction price agreements. [M/L]
- **M200 Shop catalog** — link all R16 player shops into one searchable network. [M/M]

## Quests, Story & Lore (M201–M220)

- **M201 Branching main story** — moral forks in the crown path (extends R1–10 win condition). [L/H]
- **M202 Karma** — choices shift multiple faction standings at once. [M/H]
- **M203 Faction war arc** — ironborn-vs-accord storyline with map consequences (bridges R74 war score). [L/M]
- **M204 Investigation quests** — clue-gathering mysteries with deduction UI. [M/M]
- **M205 Riddle doors** — lore-gated puzzle entrances. [S/M]
- **M206 Treasure hunts** — hand-drawn map chains to buried rewards. [M/M]
- **M207 Dream sequences** — sleep-triggered narrative vignettes foreshadowing bosses. [M/L]
- **M208 Prophecy chain** — the chronicle saga starts predicting events. [M/M]
- **M209 Ancient language** — learnable glyph vocabulary that decodes tomes. [M/L]
- **M210 Side-character arcs** — five named NPCs with multi-stage personal stories. [M/M]
- **M211 Rival** — a mirror character whose power tracks your progress. [M/M]
- **M212 Romance** — companion relationship arcs with gifts and endings. [M/L]
- **M213 Legacy** — descendants inherit part of your wealth/perks across lives. [L/L]
- **M214 Collectible sets** — artifact sets (torn archive pages exist) with set bonuses. [S/M]
- **M215 Museum hall** — donate finds to an exhibit with completion rewards. [M/M]
- **M216 Faction epilogues** — victory-screen variants per patron faction (extends R6). [M/M]
- **M217 NG+ narrative** — New Game+ exclusive dialogue and quests (extends R7). [M/M]
- **M218 Envoy quests** — letter deliveries between faction leaders. [S/M]
- **M219 Theater troupe** — NPCs re-enact your chronicle events as plays. [M/L]
- **M220 Lore codex** — searchable encyclopedia of every discovered term. [S/M]

## Multiplayer & Social (M221–M240) — extends R81–90

- **M221 Server browser** — public lobby list beyond direct-connect (the current screen is a stub — shot: `multiplayer_screen`). [M/H]
- **M222 Parties** — party invites, teleports, shared XP option. [M/H]
- **M223 Guilds** — player clans with tags, halls and treasuries. [M/M]
- **M224 Shared bank** — guild storage vaults with permission tiers. [S/M]
- **M225 Chat channels** — local/proximity and party tabs (only global exists). [S/M]
- **M226 Emote wheel** — radial quick-emotes (extends R94 chat emotes). [S/M]
- **M227 Spectator mode** — free-fly observation for events/streams. [S/M]
- **M228 PvP arenas** — dedicated dueling grounds (bridges M080). [M/M]
- **M229 Team minigames** — capture-the-flag and race maps. [L/L]
- **M230 Co-op scaling** — mob HP/loot scale with player count (bridges R81). [M/M]
- **M231 Shared annotations** — co-op drawn map pins visible to all. [S/M]
- **M232 House tours** — visit flags and ratings on notable builds. [M/L]
- **M233 Event calendar** — scheduled server events (festivals, boss spawns). [M/M]
- **M234 Leaderboards** — per-world mining/building/combat boards. [S/M]
- **M235 Moderation kit** — kick/ban/mute plus rollback tools. [M/M]
- **M236 Server-side validation 2** — speed/reach anti-cheat on top of validated ops. [M/M]
- **M237 Guest permissions** — per-visitor build-zone whitelists. [M/M]
- **M238 In-world galleries** — frames displaying shared screenshots. [S/L]
- **M239 Persistent profiles** — player identity across servers. [M/L]
- **M240 Streamer mode** — hide coordinates/personal info for broadcast. [S/L]

## UI/UX & Accessibility (M241–M260)

- **M241 Colorblind modes** — palette swaps for faction/status colors. [S/H]
- **M242 Large-print UI** — font scaling presets beyond the single UI scale slider. [S/M]
- **M243 Reduced motion** — disable camera shake, sway and reveal animations. [S/H]
- **M244 Photosensitivity mode** — cap flash intensity (reactor alarms, spell bursts). [S/M]
- **M245 Caption log** — every sound cue and bark captioned in a readable feed. [S/M]
- **M246 Contextual hints** — first-time prompts per mechanic (extends R96 tutorial chain). [M/H]
- **M247 In-game help search** — F1 codex answering "how do I…". [S/M]
- **M248 Recipe pinning** — pin favorite recipes beside the hotbar. [S/H]
- **M249 Inventory sort & filters** — one-key sort, category filter (shots show only manual shuffling). [S/H]
- **M250 Comparison tooltips** — equip A vs B stat diff on hover. [S/M]
- **M251 Death recap** — what killed you, with a tip. [S/M]
- **M252 World pings** — place colored 3D beacons other players see (extends R94 map pings). [M/M]
- **M253 HUD editor** — drag/resize the smart-HUD regions. [M/M]
- **M254 Keybind conflict detection** — warn on overlapping binds (26 rebindable keys today). [S/M]
- **M255 Console autocomplete 2** — second-level suggestions (backlog gap). [S/L]
- **M256 Gamepad support** — full controller scheme with UI navigation (backlog gap). [M/H]
- **M257 Localization** — string-table extraction + PT-BR first (backlog gap). [M/H]
- **M258 Text-to-speech** — optional reading of chat/journal for accessibility. [M/L]
- **M259 One-handed mode** — remappable action clusters. [M/L]
- **M260 Save visibility** — autosave indicator + manual save key. [S/M]

## Audio & Presentation (M261–M280) — extends R41–50 music

- **M261 Biome ambience** — wind/birds/crickets loop layers per biome (audio is 100% one-shots today; the Music slider drives nothing). [M/H]
- **M262 Weather audio** — rain intensity layers, roof patter, thunder rolls. [M/M]
- **M263 Creature voices** — distinct calls per mob type. [M/M]
- **M264 Audible NPC chatter** — bark lines spoken as muffled voice-like synth. [M/M]
- **M265 Cave reverb** — echo scaled to enclosure size. [M/L]
- **M266 Playable instruments** — a bard lute that grants audience buffs (ties the Bard job). [M/M]
- **M267 Boss themes** — proximity-triggered music layers (extends R48 night/combat variants). [M/M]
- **M268 Faction leitmotifs** — each settlement hums its faction's theme. [M/M]
- **M269 Combat stingers** — victory/defeat audio cues. [S/M]
- **M270 Title & credits music** — menu identity track (extends R46). [S/M]
- **M271 Audio category sliders** — ambience/footsteps/UI beyond the current 3 sliders. [S/M]
- **M272 Positional audio** — pan and attenuate sources by direction (rodio backend exists). [M/H]
- **M273 Underwater muffle** — low-pass filter when submerged. [S/M]
- **M274 Machine ambience** — per-machine hum loops (pairs with U230). [M/M]
- **M275 Wind audio** — gusts that visibly bend trees. [S/M]
- **M276 Hearth ambience** — fire crackle in furnished homes (extends R35 coziness). [S/M]
- **M277 Crowd murmur** — settlement daytime ambience bed. [S/M]
- **M278 Mood-mapped music** — extend R47's scale-based generator across a weather/biome matrix. [M/M]
- **M279 Music box** — craftable in-world player recording discovered themes. [M/L]
- **M280 Per-event mute matrix** — silence any individual sound. [S/L]

## Modding, Tools & Meta (M281–M300)

- **M281 Mod manager screen** — enable/disable and load-order UI in-game (packs load but are unmanaged at runtime). [M/H]
- **M282 Mod browser** — in-game Workshop catalog with ratings (UGC scanning exists; browsing doesn't). [M/M]
- **M283 Custom mob API** — data-driven mob definitions for mod packs (blocks/items/recipes are moddable; creatures aren't). [L/H]
- **M284 Custom quest API** — quest packs as TOML for modders. [M/H]
- **M285 Custom biome API** — biome TOML with palettes/spawn tables. [L/M]
- **M286 Custom structure API** — stamped building hooks for mods. [M/M]
- **M287 Resource packs** — override base textures/sounds without editing code. [M/H]
- **M288 Translation packs** — community locale files riding the M257 string tables. [M/M]
- **M289 Replay recorder** — capture input/state and replay with free camera. [L/L]
- **M290 Photo mode** — free camera, filters, framing overlay for screenshots. [M/H]
- **M291 Statistics screen** — playtime, blocks mined, kills, distance, deaths. [S/M]
- **M292 Hardcore mode** — permadeath world flag with tombstone export. [S/M]
- **M293 Challenge presets** — skyblock/one-biome/island worldgen presets. [M/M]
- **M294 Screenshot gallery** — in-game browser for F2 captures. [S/M]
- **M295 Cloud saves** — Steam remote-storage sync (gated on partner AppID like R91). [M/M]
- **M296 Guided demo world** — a curated tour save for new players. [M/M]
- **M297 Dev commentary mode** — hotspot notes explaining each system in-world. [M/L]
- **M298 Crash reporter** — bundle logs + save into a zip on panic. [S/M]
- **M299 Roadmap panel** — in-game ROADMAP-100 progress view (extends R98 dist checklist). [S/L]
- **M300 Mod sandbox flag** — world option enabling cheats/hot-reload for mod testing. [S/M]

---

# PART 2 — 300 UPGRADES (U001–U300)

## NPC & Villager upgrades (U001–U050)

- **U001 Branching dialogue trees** — player choice nodes; today every conversation is one state-conditional line from `dialogue.toml` rendered as an italic quote. [M/H · gap]
- **U002 Dedicated dialogue screen** — full-screen conversation instead of a quote strip atop the trade window. [M/H]
- **U003 NPC portraits** — rendered bust of whomever you're talking to. [S/M]
- **U004 Villager pathfinding** — villagers walk by deterministic lerp with no A*; reuse `mob_pathfind` so they stop ghosting through terrain. [M/H · gap]
- **U005 Door & obstacle handling** — NPCs open doors and never wall-stick on their schedule routes. [M/M]
- **U006 Nameplates & job icons** — floating name + job glyph over every villager (in-world shots show anonymous pale cubes). [S/H · shot: village_trading]
- **U007 Activity bubbles** — a working/sleeping/commuting icon over the head; `npc_schedule_time` shows no visible difference between schedule states. [S/M · shot: npc_schedule_time]
- **U008 Proper NPC bodies** — multi-part voxel bodies with a walk cycle; villagers are single textured cubes today. [M/H · shot: entity_skins]
- **U009 Sleeping animation** — NPCs actually lie in their beds during the Sleep slot. [S/M]
- **U010 Sitting** — NPCs sit at tables/benches for Eat and Socialize. [S/M]
- **U011 Eating animation** — hold food, consume particles at the table anchor. [S/M]
- **U012 Per-archetype schedule overrides** — parse the schedule overrides already present in `lore/npcs.toml` (documented gap). [S/M · gap]
- **U013 Weather-aware schedules** — rain sends NPCs indoors instead of along open-air anchors. [M/M]
- **U014 Guard patrols & defense** — guards walk real routes and fight hostiles that enter the settlement (hostile-faction NPCs don't join fights today). [M/H · gap]
- **U015 Dropped-item gifts** — NPCs pick up gifts thrown at them; gifting is hotbar-only today. [S/M · gap]
- **U016 Persistent standing ack** — the "+75 standing" greeting is session-state; persist it. [S/L · gap]
- **U017 Deterministic unique names** — replace first-settled-wins NPC naming with stable per-world names. [S/L · gap]
- **U018 Rotating trade stock** — daily limited quantities, restock cycles, per-NPC variety beyond static per-job tables. [M/H]
- **U019 NPC wealth simulation** — traders accumulate coins and prices react to their purse (bridges R12). [M/M]
- **U020 Haggling minigame** — quick charm check for better rates on big trades. [M/M]
- **U021 NPC skill growth** — a smith who forges daily produces higher-quality goods. [M/M]
- **U022 Relationship graph** — NPCs form friend/rival pairs you can observe chatting. [M/M]
- **U023 Memory depth** — beyond the last 2 interactions: a gift ledger and visible grudge decay. [M/M]
- **U024 Quest-giver indicators** — "!" markers above NPCs holding available quests. [S/H]
- **U025 Turn-in highlights** — pulsing marker on the NPC who completes your active quest. [S/M]
- **U026 Personal favors** — NPCs ask small inline requests ("3 berries for the hearth"). [M/M]
- **U027 Context-conditioned lines** — dialogue variants for rain, night, and nearby hostiles. [S/M]
- **U028 Dialogue history** — scrollable log of past conversations per NPC. [S/L]
- **U029 Rumor lines** — NPCs mention your recent deeds in greeting (extends the existing memory-greeting system). [M/M]
- **U030 Bard performs** — the Bard job actually plays audible music during Socialize. [M/M]
- **U031 Lorekeeper chronicles** — discovered events get written into buyable book pages. [M/M]
- **U032 Healer job** — an NPC who treats injured villagers and companions. [M/L]
- **U033 Children NPCs** — small-sized residents who grow into jobs (bridges M044). [M/L]
- **U034 Elders** — retired villagers who trade stories and hints instead of goods. [S/L]
- **U035 Fear variety** — panic responses beyond flee: hide indoors, grab weapons, rally to guards. [M/M]
- **U036 Faction clothing tinting** — stronger in-world faction colors on clothing (skins exist but read faint in shots). [S/M · shot: entity_skins]
- **U037 Hauling visuals** — NPCs visibly carry item cubes while working. [S/M]
- **U038 Station animations** — hammer sparks at the forge, sawdust at the bench. [S/M]
- **U039 Follow-me command** — lead a villager to a location by example. [M/M]
- **U040 Home/work reassignment** — relocate an NPC's bed and workstation. [M/M]
- **U041 Where-is-NPC overlay** — find residents on map/minimap. [S/M]
- **U042 Opinions panel** — inspect what each NPC thinks of you and why. [S/M]
- **U043 Audience with leaders** — appointment mechanics for M055 leader NPCs. [M/M]
- **U044 NPC machine operators** — late-game villagers staff furnaces/assemblers for you. [M/L]
- **U045 Visible farm work** — when R21 farming lands, make villager farmers till and harvest live crops. [M/M]
- **U046 Cart commuting** — NPCs travel between hamlets by cart (bridges M042). [M/L]
- **U047 Co-op NPC sync** — all players see the same villagers (extends R84). [L/H]
- **U048 Vassal depth** — loyalty meter, wage negotiation and duties beyond the flat daily yield (BACKLOG calls vassal mechanics flat). [M/M · gap]
- **U049 Accessible dialogue** — larger text option and speaker tags for readability. [S/L]
- **U050 NPC barks audible** — synthesized muffled voices for the existing bark lines (bridges M264). [M/M]

## Creature & Mob upgrades (U051–U080)

- **U051 Multi-part bodies for all mobs** — replace the single-cube/flat-quad entity reads with proper multi-cube bodies (close-ups read as "paper planes" — shot: `mob_ai_visible`, `water_flow`). [M/H · shot: mob_ai_visible]
- **U052 Animation cycles** — limb swings for walk/attack beyond the sine wobble. [M/H]
- **U053 World-space health bars & nameplates** — target info above mobs; shots show zero entity readouts. [S/H]
- **U054 Hit reactions** — damage flash + stagger so hits register visually. [S/M]
- **U055 Death animations** — topple or dissolve with particles instead of instant despawn. [S/M]
- **U056 Per-species ability kits** — Glitchling blink, Stalker short cloak, Crawler web spit — variety beyond shared FSM chase. [M/H]
- **U057 Pack tactics** — flanking arcs and wolf howl coordination on aggro. [M/M]
- **U058 Fear of fire** — hostiles avoid torchlight beyond the warding pylon radius. [S/M]
- **U059 Water behavior** — swim/breach instead of walking the riverbed. [M/M]
- **U060 Cliff-aware chases** — drop-height penalty in the A* cost so mobs don't leap off ledges. [S/M]
- **U061 Mob allegiances** — nameless raiders arrive with glitchling escorts; mob factions. [M/M]
- **U062 Elite variants** — rare aura mobs with prefix modifiers (pairs with M069). [S/M]
- **U063 Biome subspecies** — snow wolf, dune boar with stat/skin variants (tint system exists). [M/M]
- **U064 Spawn density & light gating** — tame the ground-cover-look crowding and add the documented light-level spawn gate. [S/H · gap]
- **U065 Ambient behaviors** — graze, drink at rivers, scratch — herds that look alive in shots. [M/M]
- **U066 Animal sleep cycles** — wildlife beds down at night; predators stir at dusk. [M/M]
- **U067 Loot tables 2.0** — rare drops, trophies, species-specific materials. [M/M]
- **U068 Dragon anatomy** — head/neck/wings/tail model; currently an unreadable multi-cube blob (shot: `dragon_flight`, `dragon_roost`). [M/H · shot: dragon_flight]
- **U069 Dragon flight feel** — banking tilt and wing beats (wing-tilt is a documented deferral). [M/M · gap]
- **U070 Fire-breath ignition** — dragon breath spreads fire to blocks. [M/M · gap]
- **U071 Roost hoards** — loot chests at dragon roosts (documented deferral). [S/M · gap]
- **U072 Mounted dragon combat** — attack/breathe while riding. [M/M]
- **U073 NullKnight boss brain** — real phases and an arena instead of FSM-with-more-HP (documented; supports R1–10). [M/H · gap]
- **U074 Wake the dead code** — spawn GeodeGuardian and CinderCrawler as R65/66 dungeon minibosses (both exist, tested, never spawned). [M/H · extends R65]
- **U075 Spawner cage feedback** — spawn burst FX and a remaining-count display (bridges R63). [S/M]
- **U076 Dog utility** — bark alerts on hostiles, fetch items, settle defense. [M/M]
- **U077 Bear territory** — cave-guarding bears with warning roars before attack. [S/M]
- **U078 Nesting boxes** — chickens lay visibly at nests (bridges R26 egg cycle). [S/M]
- **U079 Collision & steering polish** — mobs stop clipping through doorways; simple separation steering in crowds. [M/M]
- **U080 Entity tick LOD** — distant mobs tick at reduced rate (perf enabler for density). [M/M]

## Companion upgrades (U081–U105)

- **U081 Proper companion bodies** — replace the 0.46-radius cube with a real model (hirelings are the game's faces — they deserve it). [M/H]
- **U082 Visible equipment** — armor/tools you give a companion render on them. [M/M]
- **U083 Implement the Craft command** — the button exists but is disabled ("recipes soon"): let companions craft from blueprints. [M/H · gap]
- **U084 Smarter haul** — target any chest by crosshair with pathfinding (current chest-targeting is simplified). [M/M · gap]
- **U085 Companion leveling** — XP from tasks; a third task-rotation slot at high level. [M/M]
- **U086 Specialty perks** — miners dig faster, guards draw aggro, haulers move stacks. [M/M]
- **U087 Combat AI 2.0** — proactive targeting, retreat at low HP, rough cover use (they only revenge-attack today). [M/M]
- **U088 Field healing** — bandage command restores companion HP. [S/M]
- **U089 Downed state** — a rescue timer instead of instant morale collapse. [M/M]
- **U090 Command wheel** — radial menu replacing the two-column list (shot: `companion_commands` shows unlabeled gray blobs). [S/M · shot: companion_commands]
- **U091 Formations** — line/wedge follow for multiple companions. [S/L]
- **U092 Companion banter** — they reference each other in barks. [S/L]
- **U093 Wage autopay** — a designated chest pays daily wages automatically. [S/M]
- **U094 Renaming** — custom companion names on HUD tiles and nameplates. [S/M]
- **U095 Portrait HUD tiles** — mini face render instead of the faction-colored initial. [S/M]
- **U096 Inspect screen** — stats, task history and trust breakdown panel. [S/M]
- **U097 Patrol routes** — assign guard waypoints (beyond "guard this point"). [M/M]
- **U098 Send-to-location** — order a companion to any spot and stay. [S/M]
- **U099 Camp site** — companions rest at a placed camp, healing faster. [M/L]
- **U100 Loyalty quests** — a personal quest chain per companion unlocking trust ≥75 rewards. [M/M]
- **U101 Bark expansion + captions** — more contextual lines plus a subtitle toggle. [S/M]
- **U102 Companion mounts** — ride horses alongside you (needs M025). [M/L]
- **U103 Co-op companion sync** — hirelings visible to all players (extends R84). [M/M]
- **U104 Return warmth** — re-hire dialogue that references shared history. [S/L]
- **U105 Cargo UI 2.0** — drag-drop management and auto-sort for companion cargo. [S/M]

## Player, Combat & Feel upgrades (U106–U130)

- **U106 First-person hand & held item** — no arm or tool is visible in any shot; the biggest presence gap. [M/H · shot: first_person_view]
- **U107 Swing/place animations** — tool arcs and block-place bob. [M/M]
- **U108 Third-person camera** — toggle with a player model (entirely absent). [M/H]
- **U109 Player model** — voxel body visible in third person and to other players. [M/M]
- **U110 Armor visuals & slot rules** — per-slot equip restrictions (documented gap) plus worn-armor rendering. [M/M · gap]
- **U111 Durability rings on hotbar** — circular wear indicators on tool icons. [S/H]
- **U112 Damage numbers** — optional floating combat text. [S/M]
- **U113 Hit-stop** — 2–3 frame freeze on heavy impacts for punch. [S/L]
- **U114 Bow trajectory dots** — arc preview while charging (the radial charge ring exists). [S/M]
- **U115 Quiver UI** — ammo selection and counts for arrow types. [S/M]
- **U116 Fall roll** — crouch-timing reduces fall damage (crouch already exists). [S/M]
- **U117 Exhaustion feedback** — breathing vignette at low hunger. [S/M]
- **U118 Air meter visibility** — show breath bubbles before drowning; shots at the waterline show none. [S/H · shot: hud_preview]
- **U119 Respawn chooser** — pick among bed/waypoints on death (beds arrive with R32). [S/M]
- **U120 Camera-shake slider** — intensity control for the existing impact shake. [S/L]
- **U121 Placement ghost** — translucent block preview with validity coloring. [S/H]
- **U122 Selection wireframe & cracks** — outline the targeted block and show break progress on it (shot: `mining_feedback` shows neither). [S/H · shot: mining_feedback]
- **U123 XP curve & level rewards** — smoother curve feeding the R51–60 perk points. [M/M]
- **U124 Hunger saturation** — hidden fullness stat so quality food matters. [M/M]
- **U125 Cosmetic layers** — capes/hats earned from achievements (bridges R41–50). [M/L]
- **U126 Gesture emotes** — animated character emotes behind the M226 wheel. [M/L]
- **U127 Build assist** — hold-to-extend line/column placement. [S/M]
- **U128 Mount UI** — saddle inventory and stamina bar while riding. [M/L]
- **U129 Break tint** — the targeted block darkens as it cracks. [S/M]
- **U130 Landing dust** — impact particles scaled by fall speed. [S/L]

## UI screen upgrades (U131–U170)

- **U131 Opaque panel pass** — the tech tree, settings, trade, companion and crafting panels are translucent and illegible over bright terrain; give every screen a solid or 90%-dark backdrop. [S/H · shot: tech_tree, settings_preview]
- **U132 Unified tooltips** — hover explanations everywhere; the multiplayer screen renders tofu-box checkbox glyphs today. [S/H · shot: multiplayer_screen]
- **U133 Title screen polish** — logo art, a Continue button, changelog panel; the title is plain white text (shot: `menu_preview`). [M/M]
- **U134 New World extras** — structures toggle, world-preview thumbnail, world-type tooltips (difficulty exists; depth doesn't). [M/M · shot: new_world_screen]
- **U135 World-slot manager** — sort/search saves, backup button (thumbnails exist). [S/M]
- **U136 Settings slider relabeling** — named sliders with filled bars instead of bare "0.0 / 5.00" numbers (shot: `settings_preview`). [S/H]
- **U137 Settings search + profiles** — find settings fast; save video/control presets. [S/M]
- **U138 Always-on armor bar** — armor exists in combat but the HUD shows no armor row in any shot. [S/H · shot: hud_preview]
- **U139 Status-effect icon row** — buffs/debuffs beside the hearts (needs M064). [M/M]
- **U140 Clock/weather widget** — promote the top-left debug line to a styled time/weather readout (shot: `minimap_hud`). [S/M]
- **U141 Quest tracker** — active-quest objectives pinned on the HUD; the journal is the only place to see progress. [M/H · shot: journal]
- **U142 Minimap frame & player arrow** — opaque circular mask, arrow, north marker; it's ghost-transparent with no orientation cue (shot: `minimap_hud`). [S/H]
- **U143 Minimap layers** — toggles for mob/waypoint/structure dots. [S/M]
- **U144 Hotbar icon art pass** — wire the 157 existing item icons into every slot (the workbench renders every recipe as the same blank tan square — shot: `crafting_workbench`). [S/H · shot: crafting_workbench]
- **U145 Hotbar QoL** — drag to rearrange, extended bag row. [S/M]
- **U146 Inventory sort & search** — one-key sort plus filter field. [S/H]
- **U147 Shift-click transfers** — quick-move between inventory/chest/machine slots. [S/M]
- **U148 Recipe search & pins** — find recipes by name, pin favorites (pairs M248). [S/M]
- **U149 Ingredient highlighting** — required items glow in inventory when a recipe is selected. [S/M]
- **U150 Craft queue progress** — per-item progress on queued icons. [S/M]
- **U151 Lock reasons on recipes** — "requires Bronze Age" instead of silent absence. [S/M]
- **U152 Furnace presets & gauges** — save loadouts, fuel gauge, output lock. [S/M]
- **U153 Machine UI graphs** — progress/heat sparklines and status LEDs; reactor/machine panels are numbers-only (shot: `reactor_control`). [M/M · shot: reactor_control]
- **U154 Chest QoL** — auto-stack, category tabs, cross-chest search. [M/M]
- **U155 Journal depth** — rewards, giver, flavor text, expandable cards, track button; the panel is three bars in a half-empty frame (shot: `journal`). [M/H · shot: journal]
- **U156 Chronicle filters** — filter by event type and wire the 5 dead event producers (documented gap). [M/M · gap]
- **U157 Map cleanup** — soften the unexplored chunk-checkerboard, cursor coordinates, biome legend, grid toggle (shot: `map_screen`). [M/H · shot: map_screen]
- **U158 Typed map pins** — icons per pin kind; quest/faction pins on the map. [M/M]
- **U159 Paint the territories** — faction-map legend colors aren't actually rendered on the map (shot: `faction_map`). [M/H · shot: faction_map]
- **U160 Waypoint beacon rework** — beam width, glow, top icon and distance label; beacons are 1-px hairlines (shot: `waypoint_beacons`). [S/M · shot: waypoint_beacons]
- **U161 Trade icons & display names** — show item art and names, not raw ids like `iron_ingot x4` (shot: `trade_p2p`). [S/H · shot: trade_p2p]
- **U162 Trade compare view** — side-by-side offer values with a hold-to-confirm. [M/M]
- **U163 Companion menu redesign** — portrait, labeled icon buttons, morale explanations (shot: `companion_commands` is gray-on-gray and unreadable). [M/H · shot: companion_commands]
- **U164 Spellbook icons & stats** — spell icons, damage/cooldown/cost table (shot: `spellbook` is text-only cards). [S/M · shot: spellbook]
- **U165 Paths screen fix & depth** — unclip the Architect card (it's cut off), add icons and milestone breakdowns (shot: `paths_screen`). [S/M · shot: paths_screen]
- **U166 Tech-tree visualization** — a real node graph with connecting lines, era headers and search; today it's invisible boxes (shot: `tech_tree`). [M/H · shot: tech_tree]
- **U167 Lore book art** — page illustrations, illuminated headers, a modal dim — 70% of that frame is empty sky (shot: `lore_book`). [M/M · shot: lore_book]
- **U168 Console polish** — contrast fix, per-command help, colored output (shot: `console_preview`). [S/M]
- **U169 Multiplayer screen depth** — saved servers, player list, ping display; Friends is an explicit grayed stub (shot: `multiplayer_screen`). [M/M]
- **U170 Death recap screen** — cause of death, tips, dropped-item marker (bridges R97 graves). [S/M]

## Renderer & visual upgrades (U171–U195)

- **U171 Dawn/dusk light ramp** — golden-hour grading; the "dawn" spawn shot is indistinguishable from noon (documented deferral; shot: `spawn_plains_dawn`). [M/H · gap]
- **U172 Visible sun/moon/stars** — night skies render as flat gray with no celestial bodies in any night shot (day/night sky code exists — make it read). [S/M · shot: night_watch]
- **U173 Cloud shadows & parallax** — clouds are flat blobs with no ground shadow (shot: `clouds_weather`). [M/M]
- **U174 Fog overhaul** — height/distance curves and biome tint; fog bleaches every horizon to white-blue (shots: `biome_ground_cover`, `terrain_vista`). [M/H]
- **U175 Water transparency & depth fade** — fix opaque banded water (shots: `water_flow`, `transparency_layers`). [M/H]
- **U176 Shore blending** — wet-sand edge and foam rim at waterlines. [M/M]
- **U177 Glass alpha fix** — glass renders as blown-out white frames (shot: `transparency_layers`). [S/M]
- **U178 Leaf texture pass** — varied hole pattern and color jitter instead of moth-eaten regular holes (shot: `foliage_canopy`). [S/M]
- **U179 Ground-cover art & density** — replace the white X-quads carpeting every biome with per-biome tufts/flowers at sane density — the single biggest visual win across all 83 shots. [M/H · shot: biome_contact_sheet]
- **U180 Grass-side overlay subtlety** — soften the harsh green stripe banding that reads as a texture bug (shot: `grid_overlay`, `texture_tiling`). [S/M]
- **U181 AO strength option** — corner occlusion is nearly invisible in shots; add a strength setting. [S/M]
- **U182 Entity blob shadows** — ground shadows under mobs/NPCs (shots show none). [S/M]
- **U183 Void-edge fog mask** — hide the unloaded-world wall that cuts through vistas (shots: `terrain_vista`, `extra_terrain_vista`). [S/M]
- **U184 Particle pass** — real rain sheets with splashes, dust motes, fireflies; rain is sparse dotted dashes (shot: `clouds_weather`). [M/M]
- **U185 Snow accumulation rendering** — thin white surface layers instead of one flat quad (shot: `weather_snow`). [M/M]
- **U186 Crack-decal set** — a wider progression of break decals (F2 captures are documented to miss water/cracks too). [S/M · gap]
- **U187 Beacon beam rendering** — wide additive beams with base crystals (pairs U160). [S/M]
- **U188 Emissive/bloom pass** — lamps, lava and reactor cores glow (shots are uniformly flat). [M/M]
- **U189 Water reflections option** — screen-space sky reflection in raster mode (the path tracer already does RT). [M/L]
- **U190 Path-trace quality modes** — denoising and sample presets; PT support for water/glass. [M/M]
- **U191 Quality auto-detect** — measure fps and suggest a preset. [S/M]
- **U192 Entity mesh LOD** — simpler far-entity meshes (pairs U080). [M/M]
- **U193 Sky gradient & horizon haze** — layered sky colors replace the flat blue void (nearly every shot). [S/M]
- **U194 Weather transitions** — rain/snow fade in and out with cloud buildup. [S/M]
- **U195 Camera flourishes** — subtle head-bob option, screenshot FOV ease. [S/L]

## Worldgen upgrades (U196–U215)

- **U196 Biome transition blending** — dithered/graded edges instead of hard striped dirt walls (shot: `biome_contact_sheet`). [M/H]
- **U197 River character** — varying width, reeds, gravel shallows; rivers are uniform flat ribbons (shot: `river_valley`). [M/M]
- **U198 Beach/shore zones** — sand strips at waterlines instead of hard grass-water cuts. [S/M]
- **U199 Snowline blending** — gradual snow by altitude, not patch rectangles (shot: `river_valley`). [S/M]
- **U200 Village layout generator** — paths, wells and farm plots between houses; hamlets are sparse sheds (shot: `village_trading`, `preview_orbit_a`). [M/H]
- **U201 Structure density options** — worldgen settings for ruin/hamlet frequency. [S/M]
- **U202 Surface props** — boulders, fallen logs, stumps for natural feel. [S/M]
- **U203 Cave variety** — crystal caverns, flooded grottos, fossil layers beyond the current cave set. [M/M]
- **U204 Ore vein shapes** — cluster/streak/bed forms per ore type. [S/M]
- **U205 Tree species silhouettes** — a unique shape per species (documented: one shape today). [M/H · gap]
- **U206 Mountain features** — snowcaps, overhangs, scree slopes. [M/M]
- **U207 Archipelago world type** — island-heavy generation preset. [M/M]
- **U208 Amplified playability pass** — buildable ledges and fewer sheer walls. [S/M]
- **U209 Superflat layer editor** — custom layer-stack UI for the existing superflat type. [M/L]
- **U210 Menu preview world quality** — curate the scenic orbit so the title screen doesn't show ground-cover noise (shot: `preview_orbit_a/b/c`). [S/M]
- **U211 Spawn-point logic** — spawn near trees/water with shelter, not a featureless lawn (shot: `spawn_plains_dawn`). [S/M]
- **U212 Territory shape variance** — faction territories with size/shape generation rules (map shows uniform blotches). [M/L]
- **U213 Roost variety** — biome-flavored dragon roosts (desert spire, forest crag). [M/L]
- **U214 Ancient roads** — weathered paths linking ruins and hamlets. [M/M]
- **U215 Geothermal worldgen** — hot springs and vents as features (bridges M012). [S/L]

## Machine & tech upgrades (U216–U235)

- **U216 Machine animation pass** — spinning wheels, pistons, fans; the steam/oil chains read as powered-off decorations (shot: `steam_chain`, `industrial_machines`). [M/H · shot: steam_chain]
- **U217 Running FX** — steam puffs, smoke and sparks while machines work. [S/H]
- **U218 Connector visuals** — pipes, cables and conduit stretching between blocks (conduit stretching is a documented deferral). [M/H · gap]
- **U219 Status LEDs on blocks** — colored light state visible from across the room. [S/M]
- **U220 Heat/radiation overlay** — extend the G-key power overlay with heat and radiation views (meltdown residue gives no HUD feedback — shot: `meltdown_aftermath`). [S/M · shot: meltdown_aftermath]
- **U221 Reactor UX** — gauges, alarm states and configurable auto-SCRAM (shot: `reactor_control` shows no control UI). [M/H · shot: reactor_control]
- **U222 Machine upgrade slots** — speed/capacity modules per machine. [M/M]
- **U223 Belt motion & item skins** — visible items riding moving belts. [M/M]
- **U224 Elevator polish** — multi-floor selector and door animation (documented deferral). [M/M · gap]
- **U225 Computer programs** — scriptable monitors/alarms on the existing Computer block (its dynamic atlas screen exists). [M/M]
- **U226 Grid diagnostics** — per-line load readout and breaker blocks. [M/M]
- **U227 Pipe filters** — extract-only-X valves for fluid/item routing. [M/M]
- **U228 Assembler recipe breadth** — more automation recipes per era. [M/M]
- **U229 Auto-refuel** — generators pull fuel from an adjacent chest. [S/M]
- **U230 Machine audio identity** — a distinct hum per machine type (pairs M274). [M/M]
- **U231 Logic-signal inputs** — machines accept signals from M121 circuits. [M/M]
- **U232 Remote machine monitoring** — open machine panels from the computer terminal (pairs M136). [M/M]
- **U233 Wear & repair kits** — machines degrade with use; repair-kit item. [M/L]
- **U234 Machine placement preview** — ghost with power-link preview before placing. [S/M]
- **U235 Vistest recomposition** — reshoot `water_wheel_power` (subject not in frame) and `oil_chain` (infrastructure cropped); scenes should prove what they're named for. [S/L · shot: water_wheel_power]

## Audio upgrades (U236–U250)

- **U236 Per-block footsteps** — surface variety beyond the 5 material categories. [S/M]
- **U237 Creature calls** — a voice per mob type (pairs M263). [M/M]
- **U238 Spoken barks** — muffled voice synthesis for NPC/companion lines (pairs M264/U050). [M/M]
- **U239 UI sound set** — hover, click and error tones for the whole kit. [S/M]
- **U240 Craft/build feedback** — success chimes and per-material placement thuds. [S/M]
- **U241 Fluid audio** — water/lava flow loops near active fluids. [M/M]
- **U242 Storm layers** — thunder and rain intensity audio (pairs M003). [M/M]
- **U243 Cave reverb zones** — echo by enclosure (pairs M265). [M/L]
- **U244 Wind & foliage rustle** — audible gusts tied to M004 wind. [S/M]
- **U245 Achievement/level stingers** — unlock jingles feeding R44 toasts. [S/M]
- **U246 Boss proximity music** — tension layer when a boss is near (pairs M267). [M/M]
- **U247 Volume categories** — separate footstep/ambience/UI sliders (pairs M271). [S/M]
- **U248 Positional panning** — direction-based attenuation through rodio (pairs M272). [M/H]
- **U249 Underwater muffle** — low-pass when submerged (pairs M273). [S/M]
- **U250 Per-event mute matrix** — silence any individual sound (pairs M280). [S/L]

## Multiplayer upgrades (U251–U265)

- **U251 Remote player identity** — skins and nameplates; remote players are anonymous cubes today. [M/H]
- **U252 Remote limb sync** — pitch and swing animation for other players. [M/M]
- **U253 Peer-trade client UI** — the protocol's TradeOffer/Accept/Resolve messages exist but no client ever sends them (documented). [M/H · gap]
- **U254 Mob sync polish** — health bars and animation state over the R82 position sync. [M/M]
- **U255 Co-op NPC visibility** — villagers/companions shared between players (extends R84). [L/H]
- **U256 Machine state sync** — co-op machine progress/inventory shared (roadmap covers mobs/quests, not machines). [M/M]
- **U257 3D pings** — world-space beacons from map pings (extends R94). [S/M]
- **U258 Chat modes** — local radius and party channels (pairs M225). [S/M]
- **U259 Player list** — score/ping overlay tab. [S/M]
- **U260 Reconnect flow** — resume after a drop without duplicate entities. [M/M]
- **U261 Server MOTD/rules** — a join screen with server info. [S/M]
- **U262 Edit rollback** — per-player block revert for moderation. [M/M]
- **U263 Seed adoption on join** — apply `Welcome.seed` instead of rendering local-seed terrain (documented backlog gap). [S/H · gap]
- **U264 Connect settings** — persist address/player name instead of hardcoded localhost/"smith". [S/M · gap]
- **U265 Shared gallery** — a co-op screenshot wall (pairs M238). [S/L]

## Modding & platform upgrades (U266–U285)

- **U266 TOML mob definitions** — data-driven creatures for mod packs (blocks/items/recipes are moddable; mobs aren't). [L/H]
- **U267 TOML quest hooks** — quest packs for modders (pairs M284). [M/H]
- **U268 Biome/structure mod hooks** — worldgen modding beyond the existing ore hooks. [L/M]
- **U269 Resource-pack overrides** — swap base textures/sounds via packs. [M/H]
- **U270 Locale file loading** — community translations riding M257's string tables. [M/M]
- **U271 Mod dependencies & load order** — resolve and report missing/conflicting mods. [M/M]
- **U272 Hot-reload** — reload mod TOML packs without restarting the client. [M/M]
- **U273 Per-mod profiling** — attribute meshing/render cost to mods (bridges R92 mesh budget guard). [M/M]
- **U274 Per-block CTM PNGs** — runtime per-block connected textures instead of one strip (documented limitation). [M/M · gap]
- **U275 gen-all-textures registry** — placeholder registry so batch texture generation covers every block (documented gap). [S/L · gap]
- **U276 Gamepad input** — full controller mapping (documented deferral; pairs M256). [M/H · gap]
- **U277 Display modes** — borderless window and DPI-scale settings. [S/M]
- **U278 Save snapshots** — rolling save versions with a rollback picker (genver.dat exists; extend it). [M/M]
- **U279 Autosave options** — interval setting and background-thread saves. [S/M]
- **U280 Steam cloud saves** — remote-storage sync (gated on partner AppID; pairs M295). [M/L]
- **U281 Crash bundle** — auto-zip logs+save on panic (pairs M298). [S/M]
- **U282 Mod conflict checker** — report duplicate ids/recipe collisions at load. [M/M]
- **U283 Workshop upload pipeline** — one-command pack publishing (UGC scan exists; publishing doesn't). [M/M]
- **U284 API docs generator** — generate mod API docs from the registry (README is hand-maintained). [S/L]
- **U285 Handheld input profiles** — preset layouts for Steam Deck-class devices (pairs U276). [M/L]

## Performance & engine upgrades (U286–U300)

- **U286 Greedy meshing** — merge coplanar faces (documented deferral; frame times are fine but mods will grow worlds). [L/M · gap]
- **U287 Mesh-build budget** — timeslice chunk meshing per frame to kill hitching during streaming. [M/M]
- **U288 Parallel light BFS** — multi-threaded column lighting propagation. [M/M]
- **U289 Entity update LOD** — reduced tick rate for distant mobs (pairs U080). [M/M]
- **U290 Particle budget** — pooled particles with a cap and distance LOD. [S/M]
- **U291 Structure occlusion culling** — visibility tests for cave/interior geometry. [L/M]
- **U292 Region-file compaction** — garbage-collect dead sectors on save. [M/L]
- **U293 Startup-time pass** — lazy atlas build and deferred UI construction. [M/M]
- **U294 Chunk memory pool** — reuse chunk allocations across stream-in/out. [M/L]
- **U295 Flamegraph profiling** — integrate the puffin profiler (documented deferral). [S/M · gap]
- **U296 Snapshot delta compression** — smaller network updates at 20 Hz. [M/M]
- **U297 Server tick scaling** — keep the dedicated server responsive at 8+ players. [L/M]
- **U298 CI perf gate** — regression thresholds on the existing bench suite. [S/M]
- **U299 GPU capture harness** — capture hooks in vistest for renderer debugging. [M/L]
- **U300 Determinism checker** — same-seed world-hash test guarding save migrations (bridges genver policy gap). [S/M · gap]

---

# Top 25 quick wins (S-effort / H-impact, ordered by payoff)

1. **U131** opaque panel pass — fixes the worst readability failures in one sweep.
2. **U144** hotbar/recipe icon art pass — 157 icons exist; wire them in.
3. **U179** ground-cover art & density — de-noises nearly every outdoor shot.
4. **U122** selection wireframe + break cracks on the targeted block.
5. **U121** placement ghost with validity color.
6. **U171** dawn/dusk light ramp.
7. **U174** fog overhaul (distance curve + biome tint).
8. **U175** water transparency & depth fade.
9. **U142** minimap frame, player arrow, north marker.
10. **U138** always-on armor bar; **U118** visible air meter.
11. **U140** clock/weather widget; **U141** quest tracker.
12. **U064** spawn density + light-level gating.
13. **U006** NPC nameplates & job icons; **U007** activity bubbles.
14. **U053** mob nameplates & health bars.
15. **U024/U025** quest-giver/turn-in indicators.
16. **U160** beacon beam rework (width/glow/labels).
17. **U161** trade icons & display names.
18. **U136** settings slider relabeling.
19. **U253** peer-trade send UI (protocol already exists).
20. **U263** seed adoption on join; **U264** connect settings persistence.
21. **U012** per-archetype schedule overrides; **U015** dropped-item gifts; **U016** standing-ack persistence.
22. **U111** hotbar durability rings.
23. **U063 seed variety extras / U211 spawn-point logic** (both tiny worldgen tweaks with big first-impression payoff).
24. **U235** reshoot water_wheel/oil_chain vistest framing.
25. **U132** unified tooltips (kills the tofu-glyph class of bugs).

# Appendix — audit findings worth acting on beyond the idea list

**Cross-cutting visual findings** (from reading all 83 PNGs):
- The white/gray X-quads covering terrain are **cross-quad ground-cover plants at extreme density** (`plants_cross` confirms the render path), not mobs; at distance they read as static noise in ~40 of 83 shots. U179 + U064 address art and density separately.
- Most menu/UI scenes render panels **translucent over bright terrain** — `tech_tree`, `settings_preview`, `village_trading`, `companion_commands`, `crafting_ui`, `multiplayer_screen`, `paths_screen`, `console_preview` are all partially unreadable. U131 is one systemic fix.
- Night/dawn shots are **indistinguishable from noon or flat gray** — no visible sun/moon/stars, no golden-hour ramp (`night_watch`, `spawn_plains_dawn`, `raytraced_night` is 70% blown-out sky).
- Water renders as **opaque banded blue** in every appearance despite the transparency system (`water_flow`, `river_valley`, `transparency_layers`).
- Machines are **static blocks with no connectors** — no steam puffs, no pipes between blocks (`steam_chain`, `oil_chain`, `modern_wing`).

**Vistest scene composition bugs found by the audit** (fix the scenes, not the game):
- `water_wheel_power` — the water wheel is not in frame at all.
- `oil_chain` — infrastructure is cropped at the top; subject missing.
- `seed_comparison` — shows only ONE seed; the side-by-side framing fails.
- `paths_screen` — the Architect card is clipped off the right edge with no scroll.
- `companion_follow` / `faction_hud` — the HUD replica appears **drawn twice** (doubled hearts/XP/hotbar rows); verify whether the replica draws both variants or the layout stacks.
- `entity_skins` — subjects are a few pixels wide on a huge empty platform; the contact sheet doesn't let you evaluate the skins it exists to prove.
- `raytraced_shadows` — camera points at the canopy underside; no ground/shadow visible, so it demonstrates nothing.
- `raytraced_night` — 70% of the frame is a blown-out flat yellow sky; terrain unreadable.
- Faction structures (embassy, library, longhouse, shrine, camp) are roofless/hollow shells floating on test platforms with no doors, props or interior — fine as block showcases, misleading as "structure" proof; a furnished-interior pass (M070-adjacent, U200) would make these scenes honest.

**Code-level "dead or unwired" inventory** the ideas draw from: GeodeGuardian/CinderCrawler never spawned; 5 of 11 chronicle events have no producer; Escort/Defend quest objectives parsed but never fired; companion Craft command disabled; peer-trade protocol messages never sent by any client; villager schedules ignore per-archetype TOML overrides; `+75` standing ack is session-only; NPC unique names are first-settled-wins; CTM covers top faces only; one tree shape per species; elevator door animation and conduit stretching deferred; music slider drives nothing; F2 screenshots omit water and cracks; light-level spawn gating absent; remote players are anonymous cubes; client renders local-seed terrain ignoring `Welcome.seed`.

