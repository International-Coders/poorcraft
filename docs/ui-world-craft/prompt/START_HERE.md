# LOREFORGE — Main Menu, World Generation & Crafting Revamp
## z.ai Prompt — paste this entire file as one job

You are working on a Rust voxel RPG (LOREFORGE / poorcraft). The codebase
uses: wgpu renderer in `crates/lf_engine`, voxel world in `crates/lf_voxel`,
world generation in `crates/lf_worldgen`, game logic in `crates/lf_game`,
client shell (input, UI, screens) in `crates/lf_client`. The main binary is
`apps/loreforge`. UI uses egui 0.31 via `ui_kit.rs` (design system with
theme/easing/Reveal animations). Read AGENTS.md before touching any code.

All existing AGENTS.md rules apply: no docs-only commits, `cargo test
--workspace` must stay green, visual claims need real vistest PNGs with
pixel-analysis, DEVLOG.md gets a dated entry per completed job.

**The core complaint driving this entire prompt:** the game currently looks
like it was made by AI without human design decisions. That must end.
Every system below has explicit design rules that encode a human aesthetic
decision. Follow them precisely — do not substitute "cleaner," "simpler,"
or "more consistent" alternatives. Generic = wrong here.

Read the reference files in this folder before coding each section:
- `menu/MAIN_MENU_REDESIGN.md` — before Section A
- `menu/VERSION_SEED_WORLD.md` — before Section B
- `menu/WORLD_CREATION_FLOW.md` — before Section C
- `worldgen/WORLDGEN_REWORK.md` — before Section D
- `worldgen/BIOME_IDENTITY.md` — before Section E
- `crafting/CRAFTING_REVAMP.md` — before Section F
- `visuals/POLISH_PRINCIPLES.md` — read this first, before anything else

Work top to bottom. Do not mark a section done unless its Verify check
passes on the real running build.

---

## READ THIS FIRST — POLISH_PRINCIPLES.md summary (mandatory)

Before writing a single line of UI, world, or crafting code, internalize
these five rules. They apply to every section below.

**Rule 1 — Asymmetry is not a bug.** Buttons that are all the same width,
menus where every row is identical height, worlds where every mountain is
the same shape — these are AI tells. Intentional slight variation (a
button label that's slightly wider so its padding differs, a mountain that
leans slightly) reads as hand-crafted.

**Rule 2 — Every UI element has a reason to exist at exactly that size and
position.** If you can't state the reason, move it until you can. "It
fits in the grid" is not a reason.

**Rule 3 — The world should look like something geological, not
algorithmic.** River valleys exist because water cuts through the lowest
path, not because noise function A happened to be low here. Biomes should
tell a climate story (cold highlands, hot lowlands, wet coasts) even if
the actual generation is still noise-based.

**Rule 4 — Crafting is a conversation, not a form.** A form asks you to
fill in fields. A conversation shows you what you can make, asks you how
many, and confirms. The current "looks AI generated" crafting is a form.
The replacement should feel like talking to a workbench.

**Rule 5 — The name LOREFORGE appears on screen. "poorcraft" does not.**
Anywhere "poorcraft" appears in the UI, in window titles, in menu text, in
the worldgen seed display — replace it with LOREFORGE (all caps, always).

---

## SECTION A — Main Menu: Identity & Layout Redesign

Read `menu/MAIN_MENU_REDESIGN.md` fully before writing any code.

### A1 — Remove "poorcraft" everywhere, install LOREFORGE identity

Search `lf_client/src/ui.rs` (and any other UI file) for every occurrence
of "poorcraft," "Poorcraft," or "POORCRAFT" and replace with "LOREFORGE."
Also fix the window title (set in the main binary or the wgpu surface
creation — find it, change it to "LOREFORGE").

The LOREFORGE logotype rules (codified in `menu/MAIN_MENU_REDESIGN.md`):
- All caps, always.
- Rendered at the top-third of the title screen, not center.
- Large — the logotype should be approximately 1/6 of the screen height.
- Color: warm off-white (#f0ead6), NOT pure white.
- Below the logotype, a single-line subtitle in smaller text, muted warm
  grey: "Build. Rule. Endure." (this is the tagline — do not change it).
- No drop shadow. No glow. No outline. The logotype stands on its own.

**Verify:** a vistest screenshot of the title screen shows "LOREFORGE" in
the correct position and color. `grep -r "poorcraft" crates/lf_client/src/`
returns zero results in UI/display code (config/internal variable names
are fine, external-display strings are not).

### A2 — Background and button layout redesign

The title screen background is a live rotating voxel world (this already
exists — keep it). The overlay is what needs redesign.

**Overlay design:**
- A subtle dark vignette around all four edges (not a solid frame — a
  gradient fade from the world render to a dark edge). Implemented as a
  full-screen quad with a radial-gradient alpha texture or a shader vignette
  pass. The center of the screen must remain fully visible/unobscured.
- No solid-color background panel behind the buttons. The buttons float
  directly over the world render with the vignette providing enough contrast.

**Button layout (left-aligned column, NOT centered):**
The current centered button layout reads as a generic template. Move the
main menu buttons to a left-aligned column positioned at 10% from the
left edge and vertically centered (not quite — sit the column at 55–70%
of screen height so the logotype has visual room above). Buttons:
1. New World
2. Load World
3. Multiplayer
4. Settings
5. Quit

Button visual rules (from `menu/MAIN_MENU_REDESIGN.md`):
- No rounded rect with full fill. Instead: a left-aligned text label with
  a thin underline that animates in width from 0 to full on hover.
- Text: warm off-white (#f0ead6), same family as the logotype.
- On hover: the text shifts slightly right (+4px) with a 120ms ease-out.
- On press: the text shifts back 2px for a tactile feel.
- Active/selected state: the underline stays at full width, text is
  slightly brighter (#fff8ee).
- No drop shadow on buttons. Ever.
- Button height is NOT uniform — it is determined by text size plus fixed
  vertical padding. Since all labels are short this will be nearly uniform,
  but do not explicitly enforce equal height.

**Verify:** a vistest screenshot showing the left-aligned button column,
the vignette effect, and the LOREFORGE logotype together.

### A3 — Version display

In the bottom-right corner of the title screen (always visible, never
obscured by UI):
- The version string: `v{MAJOR}.{MINOR}.{PATCH}` read from `Cargo.toml`
  at compile time via `env!("CARGO_PKG_VERSION")`.
- Text style: small, muted warm grey (#8a7f6e), 0.6× the body text size.
- Format: `LOREFORGE v0.x.x` on one line.
- This same version string feeds the preview world seed (see Section B).

**Verify:** a vistest screenshot showing the version string in the
bottom-right, correct text and color.

---

## SECTION B — Version-Seeded Preview World

Read `menu/VERSION_SEED_WORLD.md` fully before writing any code.

### B1 — Derive the preview world seed from the game version

The rotating background world on the title screen must use a seed derived
deterministically from the game version string. Specifically:

```rust
fn preview_world_seed_from_version(version: &str) -> u64 {
    // version is "MAJOR.MINOR.PATCH" e.g. "0.4.2"
    // Parse into three u16 components, pack into a u64
    let parts: Vec<u64> = version
        .split('.')
        .map(|s| s.parse::<u64>().unwrap_or(0))
        .collect();
    let major = parts.get(0).copied().unwrap_or(0);
    let minor = parts.get(1).copied().unwrap_or(0);
    let patch = parts.get(2).copied().unwrap_or(0);
    // A stable, non-trivial mixing: use a multiply-add hash so that
    // v0.4.1 and v0.4.2 produce visually different worlds
    major.wrapping_mul(1_000_000)
        .wrapping_add(minor.wrapping_mul(1_000))
        .wrapping_add(patch)
        .wrapping_mul(0x9e3779b97f4a7c15) // Fibonacci hashing constant
}
```

Replace the current title-screen preview world seed (whatever constant or
random seed it currently uses) with `preview_world_seed_from_version(
env!("CARGO_PKG_VERSION"))`.

**Why this matters for players:** every game version will show a different,
recognizable preview world. Players will learn to associate "that snowy
valley was v0.3" with a version, which creates a tangible sense of the
game's history.

### B2 — Preview world camera path

The camera rotating over the preview world should not feel mechanical.
Replace any constant-radius circular orbit with:
- A slow, slightly elliptical orbit (x-radius ≠ z-radius, ratio ~1.3:1).
- A very gentle, long-period altitude oscillation (period ~60 seconds,
  amplitude ±8 blocks) so the camera slowly rises and falls as it orbits.
- Camera always looks at a point slightly offset from the world center
  (offset by ~20 blocks in X) so the view is never perfectly symmetric.
- Rotation speed: one full orbit every 90 seconds (slow enough to be
  scenic, not dizzying).

These parameters are constants in the title-screen code. Name them clearly
(`PREVIEW_ORBIT_PERIOD_SECS`, `PREVIEW_ORBIT_X_RADIUS`, etc.) so they're
tunable in one place.

**Verify:** a 3-frame vistest sequence (t=0, t=30s, t=60s) showing
visibly different camera angles with no perfectly symmetric framing.

---

## SECTION C — World Creation Flow

Read `menu/WORLD_CREATION_FLOW.md` fully before writing any code.

### C1 — New World screen

Currently selecting "New World" goes directly into a world with a default
seed. This must be replaced with a proper world-creation screen. The screen
uses the `ui_kit.rs` design system (same visual language as everything else
— do not invent new widget styles).

The screen is a single panel, not tabbed. It contains:

**World Name** — a text input. Default: `"World {N}"` where N is the
count of existing worlds + 1. Placeholder text: `"Name your world..."`.

**Seed** — a text input that accepts either:
- A string (hashed to u64 the same way as the existing seed system).
- A number (parsed directly as u64).
Default: a random visible seed generated fresh each time this screen opens
(show the number, not blank — the player can see what random seed they'd
get and change it if they want). A "🎲 Roll" button next to the seed input
regenerates a random seed.

**World Type** — three clearly labeled options, presented as a horizontal
set of toggle buttons (exactly three, equal visual weight, one selected at
a time):
- `Normal` — standard terrain generation
- `Superflat` — flat world, good for building
- `Amplified` — exaggerated terrain height, dramatic landscapes

**Difficulty** — four options, same toggle-button style:
- `Peaceful` — no hostile mobs
- `Easy` — mobs exist, reduced damage
- `Normal` — standard
- `Hard` — increased mob difficulty, stricter hunger

**Game Mode** (new, not yet in the game — add as a data field on the world
save but don't gate content behind it yet):
- `Survival` — the main mode
- `Creative` — unlimited blocks, no hunger/health (stub: same as Survival
  for now but saved to the world so the toggle is real)

After all fields are set, a single "Create World" button at the bottom.
Left of it, a "Back" text-link (same hover-underline style as the main
menu buttons, not a separate button widget).

**Verify:** a vistest screenshot of the New World screen showing all five
field groups rendered correctly. A test confirms creating a world with a
specific seed actually uses that seed.

### C2 — Load World screen

The existing load screen (if any) needs the same polish treatment:
- Each world slot shows: world name, world type icon (a tiny symbol per
  type: mountain = Normal, flat line = Superflat, tall peak = Amplified),
  difficulty, last-played date, and a small rendered thumbnail (the same
  preview world technique from Section B — render the world's seed into a
  tiny top-down thumbnail using the existing screenshot/vistest mechanism).
- Thumbnails are generated once on first load and cached to the world's
  save folder as `thumbnail.png`.
- If no thumbnail exists yet, show a world-type-appropriate placeholder
  color tile.
- Delete button per slot (with a confirm prompt — "Delete 'World Name'?
  This cannot be undone." Yes/No).

**Verify:** vistest screenshot of the Load World screen showing at least
two slots with distinct world names and their metadata.

### C3 — Multiplayer screen

Currently the multiplayer screen (if it exists) is a stub. Give it the
minimum viable structure that matches the visual language:
- A "Direct Connect" section: IP address text input + Port input + Connect
  button.
- A "Host World" section: selects from existing world saves (same slot
  list as Load World), then Start Server button.
- Friends lobby list placeholder (leave as "Steam lobby integration
  coming soon" — honest stub, don't fake it).

**Verify:** vistest screenshot showing the multiplayer screen structure.

---

## SECTION D — World Generation Rework

Read `worldgen/WORLDGEN_REWORK.md` fully before writing any code. This is
the largest technical section.

### D1 — Fix the mountain over-dominance

The current worldgen produces too many mountains because the continental
noise that determines elevation baseline is not properly counterbalanced by
flat lowland regions. Fix by restructuring the terrain height formula in
`lf_worldgen`:

Current problem (likely): a single octave-noise height value is used
everywhere, producing a near-continuous mountain landscape.

Required approach — **two-layer terrain:**
```
base_height = sea_level + continental_offset
continental_offset = lerp(lowland_height, highland_height, continental_factor)

where:
  continental_factor = smoothstep(0.3, 0.7, noise_continental(x, z))
  lowland_height = noise_detail(x, z) * 8.0   // ±8 blocks variation on flat
  highland_height = noise_detail(x, z) * 40.0  // ±40 blocks on mountains
```

This produces genuine flat lowlands where `continental_factor < 0.3`, a
transition zone, and mountains only where `continental_factor > 0.7`. The
ratio of flat:mountain land should be approximately 60:40 — more flat
than mountain, because flat land is where players build.

Additional fix — **ridge noise for mountains only:**
In the highland zone (continental_factor > 0.7), apply a ridge-noise
function (absolute value of noise, inverted and sharpened) to produce
distinct mountain ridgelines rather than rolling hills:
```
ridge = 1.0 - abs(noise_mountain(x, z))
ridge = ridge.powf(2.5)  // sharpen
mountain_height += ridge * 30.0  // sharp ridges add up to 30 blocks
```

### D2 — Rivers and valleys

Rivers should cut through the terrain following the gradient of the height
map, not placed randomly. A simple but effective approach:
- Sample a flow direction from a dedicated low-frequency noise map.
- Carve a river channel: wherever the height difference between adjacent
  columns exceeds a threshold AND the flow noise is low, lower the column
  to water level and mark it as river.
- River width: 3–7 blocks, determined by how far downstream the river
  has flowed (rivers widen toward the coast).
- Rivers connect to ocean at sea level. They do not float in the sky or
  cut through mountains above a configurable max altitude.

### D3 — Caves — quality over quantity

The current cave system (if it exists) likely produces too-uniform tunnels.
If caves exist, audit and apply:
- Worm caves: the existing approach is probably fine. Verify they don't
  produce caves that breach the surface too often above y=50 (visible
  surface holes should be rare — caves are underground features).
- Add **cave biomes**: below y=30, the block type transitions. Deep cave
  biome uses `deep_slate` (from the skin manifest in the previous pack).
  Lava lakes appear below y=10.
- Stalactites/stalagmites: spike downward from cave ceilings and up from
  cave floors (simple: check if a solid block has 3+ solid horizontal
  neighbors and has air below it → has a 10% chance to extend a 1–4 block
  spike downward into the air space).

### D4 — Biome placement: climate logic

Read `worldgen/BIOME_IDENTITY.md` for the full biome list. Biome placement
currently doesn't follow a climate model, which is why "hot desert next to
snowy tundra" happens. Fix by adding two climate axes:

**Temperature axis** (derived from latitude-equivalent noise + altitude):
- High altitude → cold, regardless of latitude noise.
- Low noise value = cold zone → snow/tundra/highland biomes.
- High noise value = hot zone → desert/savanna/mesa biomes.
- Middle = temperate → forest/plains/meadow biomes.

**Moisture axis** (separate noise map):
- High moisture + temperate → swamp, bog, mushroom forest.
- High moisture + cold → taiga, snowy forest.
- High moisture + hot → jungle-adjacent (dense highland forest in this
  game's vocabulary).
- Low moisture + hot → desert, badlands.
- Low moisture + temperate → plains, savanna.

The biome lookup becomes a 2D table (temperature × moisture) instead of a
single noise value, producing geographically coherent biome clusters.
Adjacent biomes should always be plausible climate neighbors.

### D5 — Structures respect terrain (don't float or bury)

Every structure placed by worldgen (faction camps, huts, libraries, etc.)
must be placed with terrain adaptation:
- Find the highest solid block at the structure's center column. Place the
  structure's ground floor at that Y level.
- If the ground varies by more than 4 blocks across the structure's
  footprint, fill below with the appropriate biome ground block (stone or
  dirt) to create a leveled platform. Do not just place the structure in
  the air.
- Never place a structure whose footprint is more than 50% underwater.

**Verify (all of D):** a vistest seed that was previously all-mountains
must now show clear lowland areas, a river, and at least two visually
distinct biome zones in the same frame. Capture a top-down map screenshot
and a ground-level screenshot for the same world seed.

---

## SECTION E — Biome Visual Identity

Read `worldgen/BIOME_IDENTITY.md` fully before coding.

### E1 — Every biome must pass the "5-second test"

Looking at a biome for 5 seconds, a player should be able to name it
without seeing a label. This requires each biome to have at minimum:
1. A distinct ground-color palette (handled by the biome color-grade pass
   from prior work — verify it's real).
2. A unique surface block or ground-cover detail (e.g. savanna has patches
   of dry grass tufts as a decoration block, mushroom forest has giant
   mushroom caps as trees).
3. A skybox/fog color that matches the biome's climate identity.

For each biome that currently fails this test (look at the vistest
contact-sheet from prior work), add at minimum the surface-detail element.
The color grade should already handle palette — if it's not working,
fix that first (see prior MEGA_PROMPT Section 1.2).

### E2 — Transition zones

At biome boundaries, blend the surface blocks and decoration over a
4–8 block wide transition zone (not a hard edge). In the transition zone:
- Surface block alternates between the two biomes' surface blocks with
  a noise-driven probability (near biome A: 90% A block, near biome B:
  90% B block, midpoint: 50/50).
- Tree/decoration features from both biomes can appear.
- The color grade blends smoothly (already designed — confirm it applies
  at boundaries).

### E3 — Biome-specific surface decoration density

Each biome entry in the biome table (`lf_worldgen/src/biome.rs`) gets a
`surface_feature_density: f32` (0.0–1.0) and a list of `surface_features`
(decoration blocks placed on the top surface). Examples:
- Dense forest: density 0.4, features [small_oak, shrub, mushroom]
- Desert: density 0.05, features [cactus, dead_shrub]
- Plains: density 0.15, features [tall_grass, flower]
- Volcanic: density 0.02, features [volcanic_basalt_spike]

**Verify:** a vistest contact-sheet of 6 different biomes showing clearly
different ground decoration density and block variety.

---

## SECTION F — Crafting System Revamp

Read `crafting/CRAFTING_REVAMP.md` fully before writing any code.

This is the most UX-sensitive section. The current crafting interface
"looks AI generated" — meaning it probably shows a grid of recipe slots,
uniform sizing, no personality, no guidance. The replacement must feel
like a real game interface with intentional design decisions.

### F1 — Crafting screen layout redesign

The new crafting screen (opens from the crafting table block or from the
inventory's crafting section) has three visual zones:

**Zone 1 — Category sidebar (left, narrow)**
A vertical list of crafting categories with icons. Categories:
- 🪵 Materials (raw → processed: planks, stone, ingots)
- ⚔️ Tools & Weapons
- 🏗️ Building Blocks
- 🍖 Food & Farming
- ⚙️ Machines & Power
- ✨ Magic & Enchanting
- 🛡️ Armor & Gear
- 🎨 Decoration

The sidebar is narrow (fixed width, fits a small icon + label). The
selected category has a subtle left-border highlight (accent color from
`ui_kit.rs` theme) — NOT a filled background highlight, which looks like
a default list widget.

**Zone 2 — Recipe list (center)**
A scrollable list of all recipes in the selected category that the player
can currently see. Each row shows:
- Item icon (16×16 texture tile, rendered inline).
- Item name.
- Required materials (inline, small): e.g. "2× Oak Log → 8× Plank" shown
  as a short inline summary, not a separate column.
- A green checkmark if all materials are in the player's inventory, a
  grey dotmark if some are missing.
- Locked recipes (prerequisites not met) appear greyed-out and below all
  available recipes. They show "Locked — needs [prerequisite]" in muted
  text. They do NOT show the full recipe.

**Zone 3 — Crafting detail panel (right)**
Clicking a recipe opens its detail in the right panel:
- Large item icon (48×48 or 64×64, rendered from the same texture).
- Item name, large.
- Description: 1–2 lines of flavor text. This is not an instruction —
  it describes what the item IS in the world (e.g. "Iron ingots, smelted
  from ore in a furnace. The Ironborn build their reputation on these.").
- Ingredients list: each ingredient on its own line, showing:
  `[icon] N× Ingredient Name — [have X / need N]` with color coding
  (green = have enough, orange = partial, red = missing entirely).
- Output: `→ N× [icon] Item Name`
- A quantity selector: `[-] [4] [+]` that scales the ingredients display
  in real time. The player can craft in batches.
- A single "Craft" button. If any ingredient is missing, the button is
  disabled and shows "Missing materials" instead of "Craft."
- A secondary "Add to Queue" button (placeholder for now — adds to a
  simple queue displayed as a small badge below the crafting screen).

**Zone sizing:** sidebar ~15% of screen width, recipe list ~40%, detail
panel ~45%. These are approximate — let the content drive the layout.

### F2 — Remove the 3×3 grid metaphor

The 3×3 grid crafting interface (Minecraft-style) should be either hidden
or removed entirely. Shaped crafting (where position matters) is the main
reason for the grid — if the current game uses shaped crafting, transition
all recipes to "ingredient list" style (unordered, quantity-based).
If shaped crafting is already not used, remove any grid UI that exists.

The new system is: you select a recipe, you have the ingredients or you
don't, you click Craft. Position does not matter.

### F3 — Recipe discovery system

Players should not see all recipes from the start. Recipe visibility
follows this logic (stored in `ClientSave`):
- **Always visible:** basic survival recipes (planks from logs, stone tools,
  torches, basic food).
- **Unlocked by finding materials:** when the player picks up a new item
  type for the first time, all recipes that use that item as a primary
  ingredient become visible. A brief toast notification: "New recipes
  unlocked: [N] recipes" appears in the HUD.
- **Unlocked by research eras:** recipes tied to the existing research-era
  system become visible when that era is unlocked (this already exists —
  wire it into the visibility system).
- **Faction-locked recipes:** visible only at the appropriate faction
  standing threshold (see the faction system from the previous pack).

**Verify (all of F):** a vistest screenshot of the full crafting screen
showing all three zones, with a selected recipe in the detail panel showing
real ingredient counts from a real player inventory state. A test confirms
the recipe-visibility system correctly hides/shows recipes based on
inventory state.

---

## SECTION G — Integration and Honest Review

### G1 — Run the full test suite
`cargo test --workspace` must pass. Fix any failures introduced by
Sections A–F before proceeding.

### G2 — Vistest suite
Run `cargo run --release -p xtask -- vistest shots` and pixel-analyze
every new scene added for this job. For each failure, fix and re-run.
Do not commit a failing vistest.

### G3 — Produce fresh runtimes and push
Per AGENTS.md mandatory desktop runtimes section: build release binaries,
package the macOS app bundle and Linux tarball (Windows if cross-tooling
is available), verify artifacts exist with `ls -la dist/`, then
`git push github HEAD`.

### G4 — Honest BACKLOG.md and DEVLOG.md update
Write one DEVLOG.md entry covering all of Sections A–F: what was done,
what files were touched, what the vistest evidence shows, and what is
honestly deferred with a specific reason. Update BACKLOG.md accordingly —
uncheck anything not actually completed. The project's own prior history
of overclaiming "done" is the reason this step is explicitly listed.
