# Visual Polish Pass

Everything in this file improves how the game looks and feels. All of it
should be verified with real vistest PNGs per AGENTS.md. Work through
these in the order listed — some depend on earlier items.

## 1. Block ambient occlusion (if not yet done — verify first)

Run a vistest scene with a corner/overhang and pixel-analyze for non-
uniform vertex-level shading. If AO is not actually present in the
rendered output:

Implement vertex-AO corner-darkening in `lf_voxel`'s mesher:
- For each vertex of each face, sample up to 3 neighboring block
  solidities that touch that corner (the standard "3-neighbor AO"
  technique used in Minecraft-style renderers).
- Compute an AO factor: 0 neighbors solid = 1.0 (no darkening);
  1 solid = 0.8; 2 solid = 0.6; 3 solid = 0.4.
- Multiply the vertex's light value by the AO factor.
- This is computed at mesh-build time, not per-frame in the shader.
- If greedy meshing is also active, AO values must survive face-merging
  correctly — per-vertex, not per-merged-quad.

**Verify:** a vistest scene with a corner overhang shows visible AO
darkening; pixel-analysis confirms non-uniform vertex light values.

## 2. Biome color grading (if not yet done — verify first)

Run a vistest scene in at least two different biomes, pixel-sample the
mid-frame area (not sky), and confirm measurably different average hue
between them. If the color grade is not actually varying:

Add a full-screen color-grade post-process pass to `lf_engine`:
- Runs after the main scene render, before egui UI.
- Takes the current biome's grade parameters as uniforms.
- Applies a simple color matrix or per-channel shift (hue rotation +
  saturation scale + warm/cool tint).
- Smooth interpolation between biomes as the player walks: blend
  current grade toward target grade at a rate of ~20% per second
  (so a full transition takes ~5 seconds, not an instant cut).

Add per-biome grade parameters to `lf_worldgen/src/biome.rs`:
```rust
pub struct BiomeGrade {
    pub hue_shift: f32,       // degrees, –30.0 to +30.0
    pub saturation: f32,      // 0.5 (desaturated) to 1.5 (vivid)
    pub warm_cool: f32,       // –0.1 (cool) to +0.1 (warm)
    pub brightness: f32,      // 0.9 (darker) to 1.1 (slightly brighter)
}
```

Starter values per biome type:
- Temperate/meadow: hue_shift=0, sat=1.0, warm_cool=0.0, brightness=1.0
- Desert/badlands: hue_shift=+8, sat=0.85, warm_cool=+0.08, brightness=1.05
- Snow/tundra: hue_shift=–5, sat=0.9, warm_cool=–0.08, brightness=1.05
- Swamp/bog: hue_shift=+12, sat=1.2, warm_cool=–0.03, brightness=0.92
- Volcanic: hue_shift=+5, sat=0.8, warm_cool=+0.06, brightness=0.88
- Mushroom forest: hue_shift=+15, sat=1.3, warm_cool=+0.02, brightness=0.95
- Deep cave: hue_shift=–8, sat=0.7, warm_cool=–0.06, brightness=0.85
- Highland forest: hue_shift=+5, sat=1.1, warm_cool=+0.01, brightness=1.02
- (Define values for all 30 biomes; unlisted biomes get the temperate default)

**Verify:** side-by-side vistest captures of desert vs. snow biome show
a clearly different mid-frame color cast.

## 3. Particle system improvements

### Break particles
- Sample 3–5 dominant colors from the broken block's texture at break time.
- Spawn 8–15 particles per break, each using one of those sampled colors.
- Burst velocity: random outward cone from break point, initial speed
  ~4 blocks/second, gravity-pulled at the existing physics gravity rate.
- Lifetime: 0.6–1.0 seconds (random per particle).
- Size: 2–4 pixels at original scale, shrink to 0 at end of lifetime.
- **Must have zero residual mark on the ground after lifetime expires.**
  This was previously reported as a bug — confirm it's fixed.
- The crack-overlay (in-progress mining texture on block face) must
  disappear at the exact moment of break/particle spawn.

### Ember particles (new, for ember_glowstone block)
- Ambient particle emitter, not triggered by break.
- 2–3 small rising spark particles per second per ember_glowstone block.
- Color: amber-orange (#c4602a range), slightly randomized per particle.
- Rise velocity: ~0.3 blocks/second upward, slight random horizontal drift.
- Lifetime: 1.5–2.5 seconds.
- Size: 1–2 pixels, fade to 0 at end of lifetime.
- Uses the existing transparent-sorted pass.

### Rain/snow correctness
- Verify rain only falls in rain biomes, snow only in snow biomes.
- Verify weather particles do not z-fight with the new block particles
  (they should be in the same sorted pass).

## 4. Block-break radial progress indicator (if not yet done — verify first)

The previous build reportedly had a "mar on the bottom" from mining
progress display. The intended design:

- Remove any mining-progress element that appears at the bottom of the
  screen (check `ui.rs`/HUD rendering for a bottom-anchored mining bar
  or overlay — find and remove it fully).
- Add a radial/pie-fill progress indicator centered on the crosshair:
  a circular arc fills clockwise from 0 to 360 degrees as mining progresses
  from 0.0 to 1.0.
- Implement in the existing `ui.rs` HUD rendering using the egui
  painter (`ui.painter().arc()` or equivalent egui primitive).
- Use a semi-transparent white or accent color matching `ui_kit.rs`
  theme — not a garish indicator.
- The radial appears only while actively mining (mouse held, block
  targeted) and disappears immediately on release or block break.

**Verify:** mid-mining vistest scene shows radial on crosshair, nothing
at the bottom of the screen.

## 5. Faction territory map shading

- Each chunk's controlling faction is determined by its biome: if the
  biome is in a faction's home-biome list (from faction TOML data), it
  is that faction's territory. Biomes not in any faction list = unclaimed.
- On the minimap (already exists), shade territory chunks with a
  semi-transparent overlay (alpha ~40%) of the controlling faction's color.
- This layer must composite correctly with the existing minimap height-
  shading so both are visible — territory color on top, height shading
  underneath, both readable.

**Verify:** vistest of the minimap/world map screen with a world that has
at least two different faction territories visible in different colors.

## 6. Companion and faction HUD elements

These use the `ui_kit.rs` design system (same corner radius, colors, and
`Reveal` animation as every other screen):

### Faction standing widget
Position: bottom-right of screen, above the hotbar.
Contents:
- Faction small icon (from UI skin manifest).
- Faction name (short).
- Standing bar: filled left-to-right, color interpolated from red
  (–100) through grey (0) to warm gold (+100).
- Standing number.
Visible only when the player is in faction territory (biome-matched) or
within 30 blocks of a faction structure.
Animates with a brief scale-pulse (using `Reveal`) when standing changes.

### Companion status tile
Position: top-left of screen, one tile per active companion (max 3).
Each tile contains:
- Companion archetype initial letter in faction color.
- Trust bar (thin, below the tile).
- Morale bar (thin, below trust bar).
- State label (FOLLOW / GUARD / REST / WORK).
Clicking a companion tile (if UI input supports it) opens the companion
command menu.

**Verify:** vistest proof showing both HUD elements present and correctly
styled — a single screenshot that shows the faction standing widget and
at least one companion tile simultaneously.

## 7. Lighting — chunk border and torch consistency

Run a vistest scene with a torch placed at a chunk boundary. Pixel-
analyze for any visible seam in light falloff. If seams exist, the BFS
light propagation in `lf_voxel`'s light module is not correctly reading
neighbor chunk state at boundaries — fix the boundary-sampling in the BFS
initialization step.

**Verify:** no visible seam in a chunk-border vistest scene.

## 8. World map faction icons

For each worldgen-placed faction structure (the 6 structure types from
Section C3 of the main prompt), add a small map icon at the structure's
world coordinates — a faction-colored dot or the faction's small UI icon
(from the skin manifest) rendered on the minimap and world map at the
correct position. Follows the same pattern as any existing waypoint
rendering.

**Verify:** a world map vistest scene showing at least two faction
structure icons at correct positions.
