# MASTER PLAN — poorcraft, Vanilla-Complete Build (40 Steps)

Hand this file to z.ai one step at a time, in order. Each step names its
target crate(s), what to build, and a **Done when** check that requires
running code and a real test/screenshot — not a doc update. Detail docs
referenced per step (e.g. `see 02`) live alongside this file; read them
before starting that step.

This is not a finished game today. That's expected. What's not acceptable
is a step being marked done when it isn't — every "Done when" below is
written so that can't happen quietly.

---

## STAGE A — Trust nothing, verify first

### Step 1 — Reality audit of every "done" claim
Play the current build yourself (or have z.ai launch it headlessly plus
a vistest run) and check every checked-off `[x]` item in BACKLOG.md
against what actually happens on screen and in a fresh save. See
`01-REALITY-AUDIT.md` for the exact checklist and method.
**Done when:** a new `AUDIT.md` exists at repo root listing every
previously-checked item as CONFIRMED or ACTUALLY-BROKEN/MISSING, with the
specific repro steps for anything broken. BACKLOG.md is corrected to match
reality (unchecking anything that isn't real) in the same commit.

### Step 2 — Fix or re-open everything Step 1 found broken
Before any new content work, every item Step 1 reclassified as broken
gets its own fix, test, and vistest proof — including block breaking,
biome variety, and lore surfacing specifically, since those are the three
you called out directly.
**Done when:** AUDIT.md has zero ACTUALLY-BROKEN/MISSING rows left for
anything that was previously claimed done.

---

## STAGE B — Block destruction, engine, and rendering feel

See `02-DESTRUCTION-ENGINE-RENDERING.md` for full detail on every step
below.

### Step 3 — Real block-break visual feedback
Cracking overlay that progresses with mining time (texture already
mentioned as a gap), block-specific particle burst on break using the
block's own texture, and a short screen-shake/impact pulse on heavy tools.
**Done when:** breaking any of 5 different block types in a vistest scene
produces visibly distinct particle-colored debris matching that block's
texture, captured in a PNG.

### Step 4 — Block-break audio and haptic-equivalent feedback
Distinct break/place sound per material category (wood, stone, metal,
glass) — first real use of the sound system if it still doesn't exist;
if sound infrastructure isn't there yet, build the minimal player (kira,
per BACKLOG's existing note) as part of this step, not a separate one.
**Done when:** a test confirms the correct sound asset key is dispatched
per block category on break/place.

### Step 5 — Per-vertex lighting / ambient occlusion
Replace flat per-face lighting with vertex-AO corner-darkening based on
neighbor solidity.
**Done when:** a vistest scene with a corner/overhang shows visible AO
darkening, and the pixel-analysis check confirms non-uniform per-vertex
values (not just non-uniform per-face).

### Step 6 — Chunk-border lighting consistency
Audit and fix any seam/mismatch in torch light, sky light, or day-night
factor across chunk boundaries.
**Done when:** a vistest scene straddling a chunk border shows continuous
lighting with no visible seam line.

### Step 7 — Camera and FOV correctness pass (rasterized path)
Re-check the rasterized camera for the same class of bug the P25 audit
found and fixed in the path tracer (double radians conversion).
**Done when:** a test verifies FOV-to-projection-matrix math against a
known-correct reference value at 2+ FOV settings.

### Step 8 — Transparency and particle sort audit
Confirm water/glass back-to-front sort still holds with the new break
particles (Step 3) and any smoke/steam particles added later (Step 24).
**Done when:** a vistest scene with overlapping water, glass, and
particles renders with correct visual layering, pixel-checked.

### Step 9 — Frame-time budget on a defined low-end device
Pick a real integer-GPU laptop spec, write it into DECISIONS.md, profile
against it.
**Done when:** DECISIONS.md has a named target device and measured fps at
"Medium" quality settings, logged in DEVLOG.md.

### Step 10 — Greedy meshing
Revisit now that break particles and future content increase per-chunk
mesh complexity.
**Done when:** a benchmark test shows reduced triangle count for a
representative chunk vs. the current culled-but-not-greedy mesher, with
no visual regression in vistest.

### Step 11 — Live RT view toggle (or explicit cut)
Decide and implement: either a real live-updating ray-traced view (not
just an R-key screenshot capture), or explicitly cut path tracing to
screenshot-only and document why in DECISIONS.md.
**Done when:** either a vistest scene shows Live RT updating across two
consecutive frames with camera movement, or DECISIONS.md has a dated
entry explaining path tracing stays capture-only and why.

---

## STAGE C — UI/UX overhaul

See `03-UI-UX-OVERHAUL.md` for full detail.

### Step 12 — Design-system pass on ui_kit.rs
One consistent visual language (corner radius, color roles, reveal
animation timing) applied to every existing screen (inventory, crafting,
chest, quest log, tech tree, pause, title).
**Done when:** a vistest proof exists per screen showing the shared
visual language, and a checklist in the detail doc is fully checked.

### Step 13 — Settings menu completeness
Key rebinding (movement + interact minimum), graphics quality tiers
(Low/Medium/High/Path-Traced dropdown), audio volume sliders.
**Done when:** rebinding a key and a quality tier both persist in
ClientSave across a save/reload, verified by test.

### Step 14 — Save-slot picker and first-launch flow polish
Save-slot thumbnails, world name/seed/last-played visible pre-load; audit
title screen for dead ends for a first-time player.
**Done when:** a vistest proof shows the save-slot picker with a real
thumbnail, and a written first-launch walkthrough in DEVLOG.md confirms no
dead end from title screen to in-world and back.

### Step 15 — Minimap and waypoint completion
Rotation, zoom, and world-space waypoint beacons (pips already exist).
**Done when:** a vistest proof shows a rotated/zoomed minimap and a
world-space beacon rendered in the 3D scene.

---

## STAGE D — Biomes and world identity

See `04-BIOMES-AND-WORLD-IDENTITY.md` for full detail.

### Step 16 — Biome visual identity audit
For all 30 claimed biomes, verify each has a genuinely distinct palette
(foliage/terrain color, fog color), not just a different noise seed
producing the same textures.
**Done when:** a vistest contact-sheet scene (or 30 individual captures)
shows all 30 biomes side by side with visibly distinct color identity;
any biome that fails gets a real texture/palette fix, not a note.

### Step 17 — Biome-specific worldgen features
Each biome should have at least one feature unique to it (a tree type, a
rock formation, a ground-cover block) beyond the base terrain shape.
**Done when:** a test enumerates all 30 biomes and confirms each has at
least one biome-exclusive placed feature/block in generated chunks.

### Step 18 — Biome-appropriate mob and structure placement
Cross-check that mobs, structures (meadow huts, watchtowers, pyramids),
and the biome table agree with each other — a desert pyramid in a swamp
biome is a bug, not flavor.
**Done when:** a test confirms structure/mob spawn tables only place
biome-appropriate content, with any existing mismatches from Step 1's
audit fixed.

### Step 19 — Weather and atmosphere per biome
Confirm rain/snow particles and fog actually vary correctly by biome (not
just globally).
**Done when:** vistest proofs for a cold biome (snow), a wet biome (rain),
and a dry biome (neither) show correct, distinct weather.

---

## STAGE E — Lore surfaced to the player

See `05-LORE-AND-NARRATIVE-SURFACING.md` for full detail. This is about
making lore something the player *sees*, not just something the chronicle
system writes to a file.

### Step 20 — Lore books readable in-game
Finish the previously-deferred lore book reading UI — a real in-world
interaction (open book, read paginated text) not a stub.
**Done when:** a vistest proof shows an open lore-book UI with real text
rendered, and a test confirms book content loads from real data files.

### Step 21 — Chronicle visible during play, not just on export
Add an in-game journal/chronicle screen (keybind) showing the player's
own saga entries as they happen, not only exported to
`worlds/<name>/chronicle.md` on save.
**Done when:** a vistest proof shows the chronicle screen with at least
one real milestone entry generated during that same play session.

### Step 22 — World lore delivered through villagers and structures
Villager dialogue/trading and structure placement (watchtowers, pyramids)
should reference the same lore names/events the chronicle and lore books
use — one consistent world story, not disconnected flavor text.
**Done when:** a test or content audit confirms at least 3 named lore
entities/events appear in 2+ of: lore books, villager dialogue, chronicle
templates, quest text.

---

## STAGE F — Power and automation ages

See `06-POWER-AND-AUTOMATION.md` for full detail on each tier.

### Step 23 — Water Age
Water wheel block, power-field integration at lowest tier, a basic power
storage/battery block.
**Done when:** a vistest proof shows a water-wheel-powered machine
running, plus a test for power-field integration.

### Step 24 — Steam Age
Minimal water-pipe subsystem, boiler (wood/coal fuel), steam engine
(mid-tier output), steam/smoke particles (only after Step 8 is done).
**Done when:** a vistest proof shows a full boiler → steam engine →
machine chain running.

### Step 25 — Oil Age
Oil deposit worldgen (desert/swamp-gated), full pipe/fluid transport, oil
pump/derrick, refinery, combustion generator, power-grid visualization
overlay.
**Done when:** a vistest proof shows extraction → refine → power chain
end to end, plus the grid overlay distinguishing powered vs. starved
machines.

### Step 26 — Nuclear tier (capped endgame)
Rare deep-only uranium ore, reactor + fuel-rod processing, real meltdown
failure state with cleanup, highest power output in the game.
**Done when:** a test demonstrates both successful power generation and a
triggered meltdown event with a real in-world consequence (not just a
log message).

### Step 27 — Item/fluid transport backbone
Belts or an inserter-analog for items; confirm pipes (Step 25) generalize
across Steam/Oil/Nuclear needs rather than being reimplemented per tier.
**Done when:** a test shows the same transport primitives feeding at
least two different machine types from two different ages.

---

## STAGE G — Magic, lore creatures, dragons

See `07-MAGIC-LORE-AND-CREATURES.md` for full detail.

### Step 28 — Magic foundation
Mana stat + HUD element, spell-slot system (hotbar-adjacent), a bounded
first spell set (damage / movement / ward / utility), enchanting minigame
sibling to the existing smithing minigame.
**Done when:** a vistest proof shows the mana HUD and a cast spell
producing a visible effect; a test confirms enchant application changes
item stats.

### Step 29 — Wizard NPCs and towers
Wizard NPC that teaches spells, sells reagents, gives lore-linked quests;
tower structure for worldgen or player building.
**Done when:** a vistest proof shows a wizard NPC and tower rendered; a
test confirms a spell can be learned from the NPC interaction.

### Step 30 — Dragons and creature roster additions
Rare high-tier dragon encounter above the Null Knight in difficulty, with
chronicle integration; one fluid-tied elemental creature.
**Done when:** a vistest proof shows the dragon rendered and a test
confirms the fight produces a chronicle entry.

---

## STAGE H — Construction and architecture

See `08-CONSTRUCTION-AND-ARCHITECTURE.md` for full detail.

### Step 31 — Building tools
Stairs/slabs/slopes shape system, symmetry/mirror placement, blueprint/
schematic capture-and-ghost-place, scaffolding block.
**Done when:** a vistest proof shows a stair/slab shape placed correctly
and a blueprint ghost-preview rendered before confirming placement.

### Step 32 — Decoration and smart-building tech
Statue-carving interaction, decoration block registry (data-driven like
mod blocks), elevator block, climate/AC block, computer/screen block
showing live tech-tree/chronicle/grid-status data, visible wiring blocks.
**Done when:** a vistest proof shows a statue, an elevator moving a
player between floors, and a computer block rendering live data from the
actual game state (not placeholder text).

---

## STAGE I — Specialization paths

See `09-SPECIALIZATION-PATHS.md` for full detail.

### Step 33 — Four-path mastery system
Path standing data model, Path-gated recipes extending the existing
tech-tree gating pattern, four starting paths (Engineer/Architect/
Battlemage/Artisan), respec mechanic, player-to-player trading UI.
**Done when:** a test shows a player unlocking a Path-gated recipe after
crossing a standing threshold, and that a generalist who never commits
still has full access to the base-tier recipe set.

---

## STAGE J — Steam multiplayer integration

See `10-STEAM-MULTIPLAYER-INTEGRATION.md` for full detail. This is the
"Steam Space Force" (Steamworks/Spacewar) multiplayer upgrade you asked
for: real lobbies and easy joining, not just the existing raw UDP + P2P
transport option.

### Step 34 — Steamworks lobby creation and discovery
Use the Steamworks SDK (already a feature-gated optional dep per
`lf_steam`) to create a lobby when hosting, and list joinable friend
lobbies from the title/multiplayer screen.
**Done when:** with the `steam` feature enabled and two Steam accounts
(or a documented local test method), one client can see and join the
other's lobby from the UI without typing an IP.

### Step 35 — Steam P2P as the default transport when in a Steam lobby
When joined via a Steam lobby, traffic should route over Steam's P2P
networking (already an existing feature-gated transport option) rather
than requiring manual UDP port-forwarding; UDP stays available for
dedicated/self-hosted servers.
**Done when:** a two-client test joins via lobby and exchanges chat/block
sync purely over the Steam transport path, with the existing UDP
integration test still passing unchanged for the dedicated-server case.

### Step 36 — Friend invite and "Join Game" flow
Steam overlay "invite to game" and "join friend's game" support so a
friend can join with one click from their Steam friends list.
**Done when:** a documented manual test confirms a Steam overlay invite
successfully drops the invited friend into the same lobby/world.

---

## STAGE K — Steam Workshop and mod-friendliness

See `11-STEAM-WORKSHOP-AND-MODDING.md` for full detail. This covers both
of your asks: mods shareable via Steam Workshop, and mods being easy for
an AI coding session or a person to write.

### Step 37 — Workshop upload/download pipeline
Package an existing `mods/` folder entry as a Workshop item (using
Steamworks UGC APIs) and support subscribing to and loading Workshop mods
alongside local `mods/`.
**Done when:** a test/manual-test round-trip confirms a packaged mod
uploads to Workshop (or a local UGC sandbox if public upload isn't
testable in CI) and a fresh client can subscribe and have it appear in
the loaded-mods smoke log the same way `ember_ores`/`amberium` do today.

### Step 38 — Mod manifest and API documentation rewrite
Rewrite `mods/README.md` as a complete, example-driven reference: every
field in the TOML schema (blocks/items/recipes/smelting/ore veins) with a
minimal working example per field, plus a "common mistakes" section aimed
squarely at an AI coding assistant writing a mod from scratch with no
other context.
**Done when:** a person or AI with zero prior repo context can follow only
`mods/README.md` and produce a loadable mod that passes the existing
full-pipeline test (parse → register → place → break → smelt) on the
first try.

### Step 39 — Mod scaffolding tool
`xtask new-mod <name>` (or equivalent) that generates a valid starter mod
folder with a working example block/item/recipe, so both a person and an
AI session start from a known-good template instead of an empty folder.
**Done when:** running the scaffold command produces a mod that loads
without errors on first try, verified by test.

---

## STAGE L — Release readiness

See `12-RELEASE-READINESS.md` for full detail.

### Step 40 — Full-loop playtest and honest gap list
Play a full session from new-world creation through at least one full
power age, one magic interaction, one build project, and one multiplayer
join-via-Steam-lobby session. Write an honest, current BACKLOG.md and
STATUS.md reflecting exactly what works — the same discipline as Step 1,
now applied to everything built in Steps 2–39.
**Done when:** STATUS.md and BACKLOG.md are rewritten to match a real,
fresh playtest, with zero items marked done that weren't just personally
verified in that session.

---

## Sequencing notes

- Stage A must complete before anything else starts.
- Stages B, C, D, E can run in parallel with each other once Stage A is
  done — they touch different systems.
- Stage F (power) and Stage G (magic) can run in parallel once Stages B–E
  are done, since new content shouldn't be judged against a shaky
  rendering/UI baseline.
- Stage H (construction) can start alongside F/G.
- Stage I (paths) depends on F, G, and H having real recipes/content to
  gate — do it last among the content stages.
- Stage J (Steam multiplayer) and Stage K (Workshop/modding) can start any
  time after Stage A, in parallel with everything else — they're
  infrastructure, not gated on content.
- Stage L is always last.
