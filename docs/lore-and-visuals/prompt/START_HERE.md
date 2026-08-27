# LOREFORGE — Lore, Factions, Companions & Visual Pass
## z.ai Starter Prompt — paste this entire file as one job

This prompt adds three large, interrelated systems to the game and
expands the visual layer significantly. Work through the sections in
order. Do not mark any section done in BACKLOG.md / STATE.md unless its
**Verify** check passes on the real running build — the project has a
history of docs claims outrunning real implementations, and these systems
are too interconnected to build on false assumptions.

All existing AGENTS.md rules apply in full: no docs-only commits, cargo
test --workspace must stay green, visual claims need real vistest PNGs
with pixel-analysis, DEVLOG.md gets a dated entry per job.

---

## SECTION A — World Lore & Faction System

Read `lore/WORLD_HISTORY.md`, `lore/COSMOLOGY.md`, and all six files in
`factions/` before writing any code. Every name, event, and symbol defined
there is canonical — do not invent substitutes.

### A1 — Lore data layer (`crates/lf_game` or a new `crates/lf_lore`)

Create a TOML-driven lore data layer following the same pattern as the
existing mod registry (TOML files parsed at boot, runtime structs, no
hardcoded strings in engine code). It needs to represent:

- **World events** — named historical events with a date (in-world era
  + year), a short description, and a list of faction IDs they involve.
  These feed the existing chronicle system: any in-game event matching a
  world-event's trigger conditions should produce a chronicle entry
  referencing the world-event by name, so the player's saga reads like
  it's happening inside a real history.
- **Factions** — ID, full name, short name, ideology tag (one of:
  `coalition`, `industrial`, `arcane`, `traditionalist`, `scholar`,
  `outlaw`), home biome list, alignment (`lawful`/`neutral`/`hostile`),
  color (for map/minimap territory rendering), and symbol string (for HUD
  faction-status display).
- **Faction standing** — a per-player, per-faction integer (–100 to +100,
  starting at 0 for all neutral factions, –50 for `The Nameless`). Stored
  in `ClientSave` alongside the existing stats/inventory. Standing changes
  when the player: trades with a faction's NPCs, completes or fails a
  faction quest, attacks a faction member, or destroys/builds in a
  faction's territory.

Exact faction definitions are in `factions/FACTIONS_OVERVIEW.md`. All six
must be in the data files; the `hostile` faction (`The Nameless`) starts
at –50 standing for all players.

**Verify:** a test loads all six faction TOML entries, asserts their IDs,
alignments, and home biome lists are present and non-empty, and confirms
standing starts at the correct values in a fresh `ClientSave`.

### A2 — Faction territory on the map

Each faction controls a set of biomes (defined in the faction data from
A1). On the world map (already exists with minimap/waypoints), shade each
visible chunk's minimap pixel with a subtle tint of its controlling
faction's color — light enough not to obscure terrain height shading,
strong enough to read as "this region belongs to someone." A chunk with
no controlling faction gets no tint.

**Verify:** a vistest scene showing the world map with two or more faction
territories visibly tinted in different colors, pixel-analysis confirming
non-uniform faction-tinted vs. un-tinted regions.

### A3 — Faction standing HUD element

A small, unobtrusive faction-status indicator in the HUD (bottom-right,
or wherever the existing HUD has room — check `ui.rs` for the current
HUD layout before placing it). Shows the name and standing value of the
faction whose territory the player is currently in (or the nearest
faction hub if not in any territory), colored by standing polarity
(positive: warm, neutral: grey, negative: red). Uses the existing
`ui_kit.rs` design system.

**Verify:** a vistest scene showing the HUD with the faction indicator
present, non-empty, and correctly colored.

### A4 — Faction-aware quest generation

Extend the existing quest system (`lf_game` quest data types, the 5-quest
starter chain, `lf_client` quest log `J`) so that:

- Each existing quest (or any new quest going forward) can be tagged with
  a faction ID as its "issuing faction."
- Completing a faction quest raises standing with that faction by a
  defined amount. Failing or abandoning it lowers standing.
- New faction-specific quests (at least 2 per faction = 12 new quests
  total across 6 factions) are added following the existing quest TOML/
  data pattern. Each quest must have a real objective (collect/craft/kill/
  reach, matching the existing objective types) and a narrative description
  tying it to the faction's ideology and the world lore from A1.

Full quest narratives for each faction are in the `factions/` detail
files. Use those as the source — do not write generic placeholder text.

**Verify:** a test confirms all 12 new quests load, parse, and fire their
correct objective types; a second test confirms completing one quest
changes standing by the expected amount in `ClientSave`.

---

## SECTION B — Hireable Companion NPCs

Read `npcs/COMPANION_SYSTEM.md`, `npcs/NPC_ROSTER.md`, and
`npcs/DIALOGUE_FRAMEWORK.md` before writing any code.

### B1 — Companion data model (`crates/lf_game`)

Add a `Companion` struct (alongside the existing mob/villager data models)
with:

- `npc_archetype_id: String` — references the NPC roster from the data
  files.
- `faction_id: Option<String>` — the faction this companion belongs to;
  determines their starting dialogue posture and what faction standing is
  needed to hire them (see B2).
- `trust: i32` — 0 to 100, starts at 0 on hire. Increases through: being
  paid on time, completing shared quests, the player taking damage
  defending the companion. Decreases through: being commanded too long
  without rest, the player attacking faction members the companion cares
  about, going unpaid.
- `morale: i32` — 0 to 100, starts at 50. Affected by: combat outcomes,
  food given, player behavior near companion's faction members.
- `daily_wage: u32` — in the existing currency/item economy. Paid
  automatically from the player's inventory at sunrise each in-game day.
  If the player can't pay, morale drops and a warning message appears.
- `state: CompanionState` — enum: `Idle`, `Following`, `Guarding(pos)`,
  `Working(task)`, `Resting`.
- `assigned_task: Option<CompanionTask>` — what the companion is doing
  when in `Working` state. `CompanionTask` is an enum covering: `Mine`,
  `Chop`, `Haul(src, dst)`, `Craft(recipe_id)`, `Guard(area)`.

Companion state persists in the world save alongside the mob state.

**Verify:** a test creates a companion, serializes it, deserializes it,
and confirms all fields survive the round-trip correctly.

### B2 — Companion hiring flow

- Certain villager NPCs (flagged in their NPC archetype data) are
  "hireable" — they display a "Hire" option in the existing trading UI
  when the player has sufficient standing with their faction (threshold
  defined per archetype in the data files).
- On hire: a one-time hire fee is deducted, the NPC transitions from
  villager-schedule behavior to `Companion` behavior (`Following` by
  default), and a confirmation chronicle entry is written (e.g. "You
  hired Mira Stonehands of the Ironborn. She will serve for coin.").
- A player can have up to 3 active companions at once (expandable via a
  `DECISIONS.md` entry if needed later). Attempting to hire a 4th gives a
  clear UI message.

**Verify:** a test simulates the hire flow (sufficient standing, fee
deduction, companion state transition, chronicle entry written).

### B3 — Companion commands (UI and keyboard shortcuts)

A companion command wheel (or a simple menu on a keybind — check the
existing input map before choosing; don't conflict with existing binds)
that appears when looking at an active companion and pressing interact.
Commands available from the start:

- **Follow me** → `Following` state.
- **Stay here** → `Guarding(current_pos)` state.
- **Rest** → `Resting` state (companion stops, morale slowly recovers).
- **Mine this** → `Working(Mine)`, targeting the block the player is
  looking at.
- **Chop this** → `Working(Chop)`, targeting the nearest tree in a small
  radius.
- **Haul to chest** → `Working(Haul)`, moves items from the companion's
  inventory to the nearest accessible chest.
- **Craft** → opens a small menu of recipes the companion knows (based on
  archetype skills from the NPC roster).
- **Pay now** → immediately pays the daily wage from player inventory,
  increases trust by a small amount.
- **Dismiss** → companion returns to their pre-hire schedule; trust resets
  to 0 but they can be re-hired later.

Uses the `ui_kit.rs` design system.

**Verify:** a vistest proof showing the command wheel/menu rendered over a
companion; a test confirming each command transitions companion state
correctly.

### B4 — Companion AI behavior

Companions in `Following` state use the existing mob wander/chase/flee AI
as a base — the same 1-block hop pathfinding, same collision rules — but
with these overrides:

- They maintain a 2–4 block follow distance from the player (not zero-
  distance clinging).
- They attack any mob that attacks the player (reuse existing combat logic
  from lf_game combat: cooldown, knockback, their own damage stat from the
  NPC archetype data).
- They avoid attacking faction members unless their trust is ≥ 70 and the
  player explicitly commands it.
- In `Working(Mine)` or `Working(Chop)` state, they pathfind to the
  target block and "use" the appropriate tool from their inventory
  (companions spawn with a starter tool set per archetype).
- They occasionally generate a short contextual voiced line (a chat
  message in the existing chat/UI system, not audio — sound is a separate
  deferred item) — e.g. commenting on the biome, the faction standing, or
  their morale level. These lines are pulled from the dialogue data
  defined in `npcs/DIALOGUE_FRAMEWORK.md`.

**Verify:** a test places a companion in Following state with a mock
player position, steps the AI, and confirms the companion moves toward
the player without overshooting to 0 distance.

### B5 — Trust and morale consequences

- **High trust (≥ 75):** companion unlocks a second `CompanionTask` slot
  (can be told to do two things in rotation), shares crafting recipes they
  know with the player's recipe book, and generates rarer contextual
  dialogue.
- **Low morale (≤ 20):** companion refuses `Working` commands ("I need
  rest"), moves to `Resting` state if not already there.
- **Zero morale:** companion quits — transitions back to villager-schedule
  behavior, chronicle entry written, standing with their faction drops
  slightly (word gets around). They can be re-hired after a full in-game
  day if standing is still sufficient.
- **Wage unpaid for 2+ consecutive in-game days:** morale drops 10/day,
  plus a journal/chronicle warning.

**Verify:** a test steps through the morale-zero quit path, confirming the
companion state transition and chronicle entry.

---

## SECTION C — Skins, Textures & Visual Identity

Read `skins/SKIN_MANIFEST.md`, `skins/BLOCK_SKIN_SPEC.md`, and
`skins/ENTITY_SKIN_SPEC.md` before writing any art or modifying the
texture atlas.

### C1 — Block texture atlas expansion

The existing texture atlas is in `lf_assets` (`texture_index_for_block`
maps block IDs to atlas layers). The texture tiling bug (textures
stretching across multi-block surfaces instead of repeating) should
already be addressed by prior work; if it hasn't been, fix it as part of
this section — the UV repeat rule is in `skins/BLOCK_SKIN_SPEC.md`.

Add the following new block texture categories, each with a properly-
tiled, non-stretching texture:

**Faction-themed blocks** (used in faction structures and worldgen):
- `accord_stone` — smooth, carved stone with faint geometric inlay pattern
- `accord_pillar` — tall carved column face
- `ironborn_brick` — dark iron-flecked brick, industrial seams
- `ironborn_grate` — metal grate texture (semi-transparent pass like glass)
- `ember_covenantwood` — dark timber with carved runic symbols
- `ember_glowstone` — warm amber self-illuminating block (emits light 8)
- `freeholds_thatch` — rough woven thatch, warm tan
- `freeholds_daub` — pale plaster/daub with rough texture
- `ashen_marble` — pale grey polished stone with subtle veining
- `ashen_bookshelf` — books-on-shelf face tile (matches existing shelf
  pattern if one exists, or new)
- `nameless_rotwood` — decaying dark wood with cracks
- `nameless_scorched` — scorched/charred stone

**Environmental/biome-exclusive blocks** (filling gaps in the 30-biome
world, one new block per biome group):
- `mushroom_cap` — large mushroom cap face (red with white spots),
  mushroom forest biome
- `coral_block` — warm pink-orange with irregular coral texture, reef biome
- `permafrost` — blueish icy soil, tundra biome
- `volcanic_basalt` — dark rough basalt with faint heat-crack lines,
  volcanic biome
- `deep_slate` — very dark blue-grey smooth stone, deep cave biome
- `mesa_terracotta` — warm orange-red layered terracotta, mesa biome
- `gilded_grass` — golden-tinted savanna grass, savanna biome
- `bog_peat` — dark wet peat surface, bog/swamp variant

**Decoration blocks** (building/crafting tier, connects to Section 9 of
the prior MEGA_PROMPT):
- `carved_oak`, `carved_stone`, `carved_iron` — three levels of carved
  decorative block for statues and ornamental walls
- `stained_glass_*` — 8 color variants (reuse the existing glass
  transparent pass, new tint colors: red, orange, yellow, green, blue,
  purple, black, white)
- `banner_*` — 6 faction-colored banner faces (can be 1-block banners
  rendered as a flat quad like a sign, not a 3D object for now)
- `lantern_hanging` — the existing lantern but intended for ceiling/
  chain mounting (can share the lantern texture; different placement rule)

All textures are 16×16 pixel art in the existing atlas style. For each
new block: add a texture constant to `lf_assets`, add a block ID to
`lf_voxel/src/registry.rs`, add solidity/opacity/light rules, add drop
items/crafting recipes in `lf_game/src/items.rs`. Keep the catalog
consistency test green throughout.

**Verify:** a vistest contact-sheet scene showing all new blocks rendered
side-by-side; pixel-analysis confirms each has a distinct, non-uniform,
non-stretched texture.

### C2 — Entity and NPC visual identity

The existing mobs (Geode Guardian, Cinder Crawler, Null Knight, villagers)
need visual differentiation that matches their faction and biome identity.
This section adds or updates entity skins per the specs in
`skins/ENTITY_SKIN_SPEC.md`:

**Faction-variant villager skins** — each villager NPC can belong to a
faction (flagged in their archetype data from B1/B2). Faction-aligned
villagers should wear visible faction colors/symbols in their rendered
skin. At minimum, 6 villager skin variants (one per faction), each with
a distinct primary color on their clothing/gear layer. The player should
be able to identify at a glance which faction a villager belongs to.

**Companion skins** — each companion archetype (defined in
`npcs/NPC_ROSTER.md`) has its own skin, distinct from generic villagers.
Companions that have joined the player carry a visible "trust level"
visual cue — at trust ≥ 50, a small visual marker (a pin, badge, or
clothing detail) appears on the companion's skin to signal loyalty.

**Mob visual refresh** — the 6 existing mob types should each have a
clearly distinct silhouette and color scheme so a player can identify them
at a glance across biomes. Audit the current mobs: if any two share a
near-identical color palette, differentiate them.

**Biome-variant mob tinting** — certain common mobs (the wander/chase
type, not the unique bosses) should have a subtle biome-tinted variant:
desert mobs run a warm sandy palette, snow biome mobs run a blue-white
palette, swamp mobs run a muddy green palette. This is a palette swap on
the same base skin, not new geometry.

**Verify:** a vistest contact-sheet scene showing all 6 faction villager
skins side by side, all 6 mob types with distinct silhouettes, and at
least 3 biome-tint mob variants; pixel-analysis confirms they are
visually distinct.

### C3 — Faction structure worldgen

Each faction needs at least one worldgen-placed structure in its home
biomes, using the new faction-themed blocks from C1. Follow the existing
structure system in `lf_worldgen` (meadow huts, watchtowers, pyramids are
the reference — deterministic placement keyed to chunk seed):

- **The Accord** → `accord_embassy`: a small walled courtyard with an
  accord_stone gatehouse and 1–2 accord_pillar corners, containing an
  Accord-aligned NPC.
- **The Ironborn** → `ironborn_forge_camp`: a compact industrial camp with
  ironborn_brick walls, ironborn_grate windows, a working furnace, and an
  Ironborn NPC.
- **The Ember Covenant** → `covenant_grove_shrine`: a ring of ember_covenantwood
  posts around a central ember_glowstone altar, with a Covenant NPC.
- **The Free Holds** → `freeholds_longhouse`: a freeholds_thatch/daub
  building, slightly larger than a meadow hut, with a Free Holds NPC inside.
- **The Ashen Order** → `ashen_library`: a small ashen_marble building
  with an ashen_bookshelf interior, containing an Ashen Order NPC and a
  readable lore book.
- **The Nameless** → `nameless_camp`: a derelict camp of nameless_rotwood
  and nameless_scorched blocks, with Nameless-aligned hostile NPCs and a
  loot chest.

All structures must pass the existing structure determinism test pattern.

**Verify:** a vistest scene per structure showing the building rendered
correctly in its home biome with its faction NPC present.

### C4 — Expanded HUD and visual polish

These are renderer/HUD improvements that don't require Section A or B but
benefit from them:

**Faction standing widget** (from A3 — must use the new design system):
- Show a small colored bar (faction color, per faction data) + standing
  number in the corner while in faction territory.
- Animate it (brief pulse) when standing changes, using the existing
  `ui_kit::Reveal` animation primitive.

**Companion status HUD** (from Section B):
- While a companion is active, show a small companion portrait-tile
  (their faction color + archetype icon letter — not a full portrait art
  asset, just a colored initial tile) with a miniature trust bar and
  morale bar below it. Uses `ui_kit.rs`.

**Biome color grade** (carried forward from prior work, verify it's real):
- Each of the 30 biomes should have a distinct color-grade entry
  (hue shift + saturation + warm/cool push) in the biome table
  (`lf_worldgen/src/biome.rs`). If this was implemented in a prior loop,
  verify it's actually varying across biomes by running the vistest
  comparison. If it's still not real, implement it now: a full-screen
  post-process pass in `lf_engine`, per-biome grade data in the biome
  table, smooth interpolation at biome boundaries.

**Block ambient-occlusion** (carried forward):
- Per-vertex AO must be implemented and present in every vistest scene.
  If it was done in a prior loop, confirm it visually in the shots. If not,
  implement it now — vertex-AO corner-darkening in `lf_voxel`'s mesher.

**Particle improvements**:
- Break particles now sample the broken block's texture color (not a flat
  grey); particles have a short outward burst + gravity + despawn within
  1 second. No residual mark on the ground.
- Add an `ember` particle for the ember_glowstone block: small floating
  orange sparks rising from the block surface, ambient (not on break),
  2–3 particles/second, using the existing transparent/sorted pass.

**Verify each** with a vistest proof. The companion HUD and faction widget
need separate screenshots.

---

## SECTION D — Wiring Everything Together (integration pass)

After Sections A, B, C are all individually verified:

### D1 — Faction standing affects NPC behavior
- Faction-aligned NPCs (villagers, companions, quest-givers) should
  react to the player's standing with their faction. At standing ≤ –30,
  a faction's NPCs refuse to trade and display a hostile dialogue line.
  At standing ≥ 50, they offer a bonus trade or an exclusive faction item.
  Uses the dialogue framework from `npcs/DIALOGUE_FRAMEWORK.md`.

### D2 — Chronicle integration for faction events
- Every significant faction event (standing threshold crossed, companion
  hired/dismissed/quit, faction quest completed, faction structure
  discovered for the first time) should produce a chronicle entry.
  Reference the world-event names from `lore/WORLD_HISTORY.md` in these
  entries where appropriate — e.g. "You have earned the trust of the
  Ironborn, whose founders forged the Great Smelter of Ashenmoor in the
  Third Era."

### D3 — Faction standing affects the world map
- Re-verify Section A2's faction territory tint is visible in the map
  after the new faction structures from C3 are placed. The structures
  themselves should appear as map icons (a small faction-color dot) at
  their world coordinates on the minimap.

### D4 — Honest final pass
Produce one fresh `cargo test --workspace` run after all sections are
done, resolve any failures, and do a real vistest capture of:
- A world with at least two faction territories visibly tinted on the map
- A companion following the player with the companion HUD visible
- A faction NPC inside a faction structure
- The biome color grade changing between two biomes
- The new block types in a structure

Write a DEVLOG.md entry with the screenshot paths as evidence, update
BACKLOG.md honestly (including anything that's deferred with a specific
note), and push.
