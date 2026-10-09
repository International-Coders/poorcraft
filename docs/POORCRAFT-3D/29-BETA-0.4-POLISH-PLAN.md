# POORCRAFT 3D — Beta 0.4 Plan: "Feel & Finish" — 180 phases, 6 workstreams

Status: ACTIVE (owner direction 2026-10-05: "make the game the best game
ever — the features are too indie; at least 30 phases for EACH step with
multiple tests and verifications").
Builds on `28-BETA-0.2-PLAN.md` (world/content breadth continues in
parallel where the battery allows) and the playability reset
(`POORCRAFT3D-CURRENT-PLAN.md` — its control-grammar and honesty gates
stay law).

## The bar

"Too indie" is a specific, fixable set of missing surfaces. Beta 0.4
closes them in six workstreams of exactly 30 phases each — 180 gated
phases. A phase is CLOSED only when: code lands, its law test passes,
its battery stage stays green, and its capture/evidence updates. Phases
are numbered and never skipped (a deferred phase stays OPEN in
`PHASE-TRACKER.md`); three consecutive failed attempts on a phase move
it to the honest-deferred list with the reason.

| Workstream | The indie tell it kills | Proof home |
| --- | --- | --- |
| W-A Game Feel & Feedback | The world doesn't react to you | `make p3d-feel-gates` |
| W-B Audio | A silent game | `make p3d-audio-gates` |
| W-C Visual Fidelity | Flat lighting, no atmosphere | `make p3d-look-gates` |
| W-D UI/UX Polish | Static text-box menus | `make p3d-ui-gates` |
| W-E Living World | A world that ignores time | `make p3d-world-gates` |
| W-F Content Depth | Five items and three buildings | `make p3d-content-gates` |

Every workstream ALSO re-runs `make p3d-beta` at its phase 30 — polish
never buys a regression.

---

## W-A — GAME FEEL & FEEDBACK (30 phases)

The player must FEEL every action within one frame.

| # | Phase | Verification |
| --- | --- | --- |
| A1 | HUD damage flash: red vignette pulse on health loss (HUD overlay element, law: flash visible for 2 frames after a drop, gone after 30) | ui law + capture diff |
| A2 | Health-bar damage easing: bars animate to new values (0.15 s ease), never snap | ui law (bar frac sampled at t0/t+dt) |
| A3 | Hit feedback on melee: target flash + 4 px camera nudge on landed hit | route_pit_wall extended |
| A4 | Dig feedback: the dug cell puffs (6-particle burst, 0.4 s) | capture + particle count gate |
| A5 | Toast polish: slide-in from the right + fade (already fades; add 6 px slide) | ui law (toast x at t0 vs t+0.1 s) |
| A6 | Item-pickup bounce: hotbar slot scales 1.15→1.0 over 8 frames on count change | ui law |
| A7 | Walk-head-bob (0.05 m sine at step cadence; OFF in menus) | route capture y-sampling law |
| A8 | Landing thud camera drop (0.08 m, recovers 0.2 s) | fall route extended |
| A9 | Selected-slot underline slide (the ring glides between slots) | ui law |
| A10 | Crosshair context morph: dot → ring when a target is in reach | ui law + capture |
| A11 | Objective strip typewriter reveal on new objective (12 frames) | ui law |
| A12 | Bar segment ticks (25% marks) — readable fractions | ui capture |
| A13 | Low-health heartbeat pulse on the health bar (< 25%) | ui law |
| A14 | Crafting success flash on the crafted slot | ui law |
| A15 | Panel open/close 4-frame scale-in (0.96→1.0) on all panels | ui law per panel |
| A16 | Fountain/wheel splash particles at the river machine | route_machine_chain extended |
| A17 | Snow-footprint tint in the snow realm (material-aware steps) | capture probe |
| A18 | Wind gusts sway peaks (flora wind param gust envelope) | capture diff over 40 frames |
| A19 | Toast icon prefix per kind (save/dig/error) | ui law |
| A20 | Day-phase tint on the HUD bars (slight warm at dusk) | capture probe |
| A21 | Forge ember particles while heating | route_forge_use extended |
| A22 | Chest-open lid angle animation (GLB socket rotate 0.6 rad, 10 frames) | capture diff |
| A23 | Firefly particles at night in wetland | night capture probe |
| A24 | Sun god-rays at dawn/dusk (screen-space radial streaks, strength 0.25) | capture probe |
| A25 | Footstep dust puffs on sprint | capture probe |
| A26 | Realm-banner cloth sway on capital gates | city route extended |
| A27 | Rain ripple rings on water during rain (W-E dependency) | world gate |
| A28 | Fish shadow ripples in rivers (W-F fish dependency) | world gate |
| A29 | Compressor: all feedback effects scale with ui "effects" setting (off/low/high) | settings law |
| A30 | W-A phase-30 gate: full battery + `p3d-feel-gates` + side-by-side before/after contact sheet | battery |

## W-B — AUDIO (30 phases)

A silent game is the loudest indie tell. Foundation: a `pc3d_audio`
crate (cpal output stream + a f32 mixer; NO audio files — every sound is
a deterministic procedural synth, seeded like everything else in this
repo).

| # | Phase | Verification |
| --- | --- | --- |
| B1 | `pc3d_audio` crate: AudioHost (output stream, graceful no-device fallback), Mixer (f32 bus, master gain), 48 kHz stereo | crate law: mixer length/NaN checks |
| B2 | Synth kernel: envelope (attack/decay), noise, sine, filtered noise — deterministic from seed | sample-buffer law tests |
| B3 | SFX set 1: footstep grass (filtered noise burst 60 ms), footstep stone (click), jump/land | buffer law per sound |
| B4 | SFX set 2: dig thunk (100 ms low burst), build place (double click), remove crumble | buffer law |
| B5 | SFX set 3: forge whoosh, ore clink, bar-ready chime, eat crunch, toast tick | buffer law |
| B6 | SFX set 4: UI open/close/click/deny (deny = low buzz) | buffer law |
| B7 | Ambient bed: wind loop (slow LFO on filtered noise, seamless loop law) | loop-seam law |
| B8 | Hook: footsteps at walk cadence (material-aware from the D2 surface law) | audio gate: step events counted per 10 m walk |
| B9 | Hook: dig/build/remove sounds on the real actions | route_dig audio-event log |
| B10 | Hook: forge ore/fuel/take sounds | route_forge_use audio log |
| B11 | Hook: eat + toast tick + UI panel open/deny | ui law + audio log |
| B12 | Ambient: wind gain follows weather + night crickets (high sine chirps) at night | audio log over a day cycle |
| B13 | Combat: swing whoosh, hit impact, creature hurt | route_social audio log |
| B14 | Water: river loop gain by distance to the strip | distance law |
| B15 | Fire crackle at the forge while heating | forge route audio log |
| B16 | Master/SFX/Ambient volume settings in Settings (persisted, drive the mixer — the "settings drive the engine" law) | settings law |
| B17 | Mute-on-pause (mixer ducks to 0 under menus; ambient returns on resume) | mixer law |
| B18 | Audio follows the camera: 3D-ish panning + distance gain for world sounds (simple stereo pan law) | pan law test |
| B19 | SFX set 5: companion follow/whistle, chest open creak, coin clink (economy) | buffer law |
| B20 | Quest: accept chime + complete fanfare (4-note arpeggio) | buffer law |
| B21 | Career: choose-path sting per career (Engineering = metal; Mysteries = airy fifth) | buffer law |
| B22 | Footsteps per realm material (snow crunch, sand soft, wood knock on build floors) | material law |
| B23 | Rain sound (W-E rain dependency): filtered noise bed + droplet ticks | audio log in rain |
| B24 | Dragon/creature distant growl set (W-F dependency, placeholder synth) | buffer law |
| B25 | Music foundation: a 2-layer generative ambient pad (seeded chord bed by realm, cross-fades at realm borders) | cross-fade law |
| B26 | Music: exploration intensity envelope (calm ↔ tension by creature proximity) | envelope law |
| B27 | Audio occlusion: sounds behind terrain roll off (cheap: LOS check at half distance) | occlusion law |
| B28 | Performance: the mixer shares the frame budget; zero-allocation steady state | deck bench unchanged ±5% |
| B29 | Settings UI row: volumes + mute + "audio test" button in Settings | ui law |
| B30 | W-B gate: `p3d-audio-gates` (every sound's buffer law + hook logs + settings) + full battery | battery |

## W-C — VISUAL FIDELITY (30 phases)

| # | Phase | Verification |
| --- | --- | --- |
| C1 | Post-process pipeline (full-screen pass after the world): foundation + identity LUT | capture unchanged at neutral |
| C2 | Vignette (subtle, 0.12) | capture probe |
| C3 | Film grain (animated, strength 0.03) | capture probe |
| C4 | Color grade LUT per day-phase (warm noon, amber dusk, blue night) | capture probes ×3 |
| C5 | Bloom on emissive (forge fire, glowcaps, lanterns) | capture probe |
| C6 | Tonemap (ACES-ish) instead of raw srgb | capture probe |
| C7 | Water reflections: screen-space sun streak on rivers | capture probe |
| C8 | Soft shadows: PCF radius by quality tier | shadow capture |
| C9 | Grass density/tint per biome dress (W2.2 dependency) | wilderness capture |
| C10 | Fog height function (valleys misty at dawn) | dawn capture |
| C11 | Cloud layer (2D noise clouds drifting with the wind) | capture diff |
| C12 | Stars + moon at night (replaces flat night sky) | night capture |
| C13 | Lightning flash during storms (W-E rain dependency) | storm capture |
| C14 | Realm banner colors on capital kit materials | city capture |
| C15 | NPC outline highlight on talk-target (subtle rim) | npc capture |
| C16 | Selection ring on the aimed interactable | ui capture |
| C17 | Ghost preview for build pieces (translucent green/red by validity) | build route |
| C18 | Damage decals (dark splats on hit creatures) | combat capture |
| C19 | Footprint decals on sand/snow | capture |
| C20 | Water depth tint (shallow→deep gradient) | water capture |
| C21 | Cave ambience lighting (cold blue fill in interiors) | cave capture |
| C22 | Forge fire light source (point-light approx in the shader) | forge capture |
| C23 | Lantern posts in settlements glow at night | night city capture |
| C24 | Snow cap blending by altitude (smooth, not dithered) | mountain capture |
| C25 | Tree impostor billboards for the macro ring (W1.2 plates successor) | far capture |
| C26 | Screen-space rain streaks (W-E dependency) | storm capture |
| C27 | Melee swing arc trail | combat capture |
| C28 | LOD pop fade (0.2 s crossfade instead of pop) | capture diff at LOD border |
| C29 | Effects quality setting drives C2/C3/C5/C24 strength | settings law |
| C30 | W-C gate: `p3d-look-gates` (before/after contact sheet + battery) | battery |

## W-D — UI/UX POLISH (30 phases)

| # | Phase | Verification |
| --- | --- | --- |
| D01 | ui_kit for P3D: theme tokens + easing fns (port from LOREFORGE) | ui law |
| D02 | Animated panel transitions (fade+slide 6 frames) | ui law |
| D03 | Button hover lift + press sink (2 px, 3 frames) | ui law |
| D04 | Tooltip system (hover any interactive element → name + hint) | ui law |
| D05 | Rebindable keys screen (list, press-to-bind, conflict deny) | ui law + round-trip |
| D06 | Keybind-driven prompts everywhere (the guide's "never baked" law) | ui law |
| D07 | Inventory grid with drag/drop + stack split (half/right-click) | ui law |
| D08 | Item detail card (name, kind, stats, flavor line) | ui law |
| D09 | Craft queue (batch craft N, progress bar) | ui law |
| D10 | Recipe cards per the P4 spec (owned/required counts, station, purpose) | ui law |
| D11 | Minimap corner (realm map crop by player region, N-rotating) | ui law |
| D12 | Compass strip with realm capital bearings | ui law |
| D13 | Notification feed (right side: quest/claim/trade events) | ui law |
| D14 | Dialog portraits (NPC GLB head render into the panel corner) | ui capture |
| D15 | Dialog choice buttons (2-3 options; the interview deepens later) | ui law |
| D16 | Damage numbers (floating, rise+fade at hit) | combat capture |
| D17 | Settings: audio rows (B29) + effects rows (A29/C29) + keybinds (D05) on pages | ui law |
| D18 | Save-slot browser with world minimap crops + playtime | ui law |
| D19 | Loading screen with realm-map background + tips | ui capture |
| D20 | Death screen (fade, "you fell", respawn button — replaces the instant plaza teleport) | route_plaza_recovery extended |
| D21 | Tutorial toast sequence tied to Onboarding (one per step) | ui law |
| D22 | HUD scale presets + safe-area check at 5 resolutions | ui law ×5 |
| D23 | Colorblind-safe bar palette (deuteranopia LUT check) | capture probe |
| D24 | Quest tracker on the right edge (active quest + progress) | ui law |
| D25 | Shop/trade panel (wallet, offer rows, escrow line) | economy law + ui |
| D26 | Codex screen (items seen, realms discovered, quests done) | ui law |
| D27 | Photo mode (pause world, free camera, hide HUD, save PNG) | capture law |
| D28 | Text speed + font-size setting | ui law |
| D29 | Controller navigation of all menus (dpad focus walk) | input law |
| D30 | W-D gate: `p3d-ui-gates` + battery | battery |

## W-E — LIVING WORLD (30 phases)

| # | Phase | Verification |
| --- | --- | --- |
| E01 | Weather state machine (clear/cloudy/rain/storm, seeded Markov per day) | world law |
| E02 | Rain particles + splash (C26/A27 consumers) | world gate |
| E03 | Storm: wind gust envelope + lightning flash + thunder (B23) | storm gate |
| E04 | Weather affects gameplay: rain fills flow slightly, wet ground tint | flow law |
| E05 | Seasons原型: day-phase temperature curve per realm (snow realm colder) | temp law |
| E06 | Fireflies (A23) + birds by day (ambient cast, no gameplay) | capture |
| E07 | Fish schools visible in rivers (W-F fish) | capture |
| E08 | RiverCarve W1.4: valleys carved under river edges | carve law |
| E09 | Waterfalls at steep drops (particle column) | capture |
| E10 | Lake generation (wide river nodes become ponds) | atlas law |
| E11 | Cave entrances visible from outside (mouth dressing) | capture |
| E12 | Ore veins visible as surface outcrops near cave mouths | capture |
| E13 | Day-length per realm (snow realms shorter days) | time law |
| E14 | Region discovery popups ("DISCOVERED: THORNWOOD") | ui law |
| E15 | Roads between capitals (the planner's A-line, dressed) | realm-map law |
| E16 | Signposts at crossroads (readable with E) | interact law |
| E17 | Campfires at roads (warm light + cook station) | capture + interact |
| E18 | Migrating deer herds (ambient, W2.4 deer) | capture |
| E19 | NPC schedules use buildings per weather (rain → indoors) | npc law |
| E20 | Region-name mapping (seeded names per region, shown on discover) | name law |
| E21 | Capital districts grow visible over prosperity (kit variants) | city law |
| E22 | Guard patrols on capital walls (nav loop) | npc law |
| E23 | Market stalls on plaza days (calendar) | capture |
| E24 | Chirp/bird ambience by biome (B12 consumer) | audio log |
| E25 | Fog-of-war on the realm map (visited regions lit) | map law |
| E26 | Fast travel: capital → discovered capital (loading screen D19) | route law |
| E27 | World edge: the planned band gets a natural barrier (ocean ring) | atlas law |
| E28 | Deterministic ruins (5 seeded ruin kits in wilderness) | capture |
| E29 | Weather persistence in the session save | save law |
| E30 | W-E gate: `p3d-world-gates` + battery | battery |

## W-F — CONTENT DEPTH (30 phases)

| # | Phase | Verification |
| --- | --- | --- |
| F01 | Item catalog to 40 (tools tier 3, 6 foods, 4 cloth, armor pieces) | catalog law |
| F02 | Recipes to 60 (data-driven TOML, P3D-506 boundary) | recipe law |
| F03 | Armor: 4 slots, craftable, damage reduction law | combat law |
| F04 | Melee weapons: sword/club/spear (reach/damage table) | combat law |
| F05 | Bows + arrows (projectile arc, ammo) | combat route |
| F06 | Creature combat: 12 kinds (W2.4) with drop tables | spawn law |
| F07 | Fishing: rod, cast/wait/reel minigame, 6 fish by realm | fishing route |
| F08 | Cooking: campfire recipes (raw→cooked value chain) | cook law |
| F09 | Farming: hoe → tilled soil, 4 crops with growth ticks | farm route |
| F10 | Seeds from grass/harvest (renewable loop) | drop law |
| F11 | Workbench station (P4): personal vs station recipes | craft law |
| F12 | Furnace station (smelt ore → bars offline with fuel) | smelt law |
| F13 | Chest storage (placeable, per-chest inventory, persists) | chest law |
| F14 | Doors + windows (construction pieces with open/close) | build law |
| F15 | Multi-piece structures (foundation walls, roofs snap) | build law |
| F16 | Shelter law: inside covered cells at night = rested buff | survival law |
| F17 | Beds: sleep to dawn if sheltered + no creatures near | sleep route |
| F18 | Torch placeable (light source C22 consumer) | capture |
| F19 | Ladders (climb placement, vine-grip reuse) | climb law |
| F20 | Realm banner crafting (faction identity item) | craft law |
| F21 | Trade goods (economy: sell ore/fish/crops for credits at market) | trade law |
| F22 | NPC needs → requests (deliver quests generated from needs) | quest law |
| F23 | Companion inventory + equip (shares the loot rules) | companion law |
| F24 | Magic path spell 1-3: ward, light, bolt (mana spend, S06 live) | career law |
| F25 | Engineering path machine 1-3: mill, pump, crane | machine law |
| F26 | Valve computing starter: lever + lamp (B29 the light) | valve law |
| F27 | Realm-specific recipe variants (6× the banner + a dish) | realm law |
| F28 | Rare drops (1/64 sigil shard; 8 shards = a ward charge) | drop law |
| F29 | Codex auto-fills from discoveries (D26 consumer) | codex law |
| F30 | W-F gate: `p3d-content-gates` + battery | battery |

---

## Execution order & rules

Sequential per workstream (phases within one build on each other);
workstreams interleave W-B → W-A → W-D → W-C → W-E → W-F so audio and
feel land first (the loudest indie tells). Each session executes 3-8
phases, always: code + law test + battery stage green + commit + push.
`PHASE-TRACKER.md` (same folder) is the living checklist — updated every
session, never hand-waved.

Everything stays inside the constitution: depth before catalog size,
performance is part of the fantasy (every workstream's phase 30 re-runs
the battery), and the breadth safeguard still gates multiplayer/armies/
nuclear delivery.
