# Biome Identity — Reference Document

## The climate grid (how biomes are placed)

Biomes are selected from a 2D table of Temperature × Moisture. Both axes
are determined by noise maps, with altitude modifying temperature.

```
Temperature axis (0.0 = freezing, 1.0 = scorching):
  base_temp = noise_temp(x, z, seed) mapped to 0..1
  altitude_temp_penalty = (height - sea_level).max(0) as f32 / 120.0
  effective_temp = (base_temp - altitude_temp_penalty).clamp(0.0, 1.0)

Moisture axis (0.0 = arid, 1.0 = saturated):
  moisture = noise_moisture(x, z, seed) mapped to 0..1
  (moisture is NOT affected by altitude — moisture is about rainfall, not
  elevation per se)
```

## Biome selection table

| Temperature → | Freezing (0–0.2) | Cold (0.2–0.4) | Temperate (0.4–0.6) | Warm (0.6–0.8) | Hot (0.8–1.0) |
|---|---|---|---|---|---|
| **Arid (0–0.2)** | Tundra | Highland | Plains | Savanna | Desert |
| **Dry (0.2–0.4)** | Snowy Hills | Taiga | Oak Forest | Dry Forest | Badlands |
| **Normal (0.4–0.6)** | Permafrost | Spruce Forest | Temperate Forest | Dense Forest | Mesa |
| **Wet (0.6–0.8)** | Snowy Tundra | Snowy Forest | Meadow | Jungle (placeholder) | Volcanic |
| **Saturated (0.8–1.0)** | Ice Flats | Deep Taiga | Mushroom Forest | Swamp | Bog |

This table guarantees no climate-impossible adjacency (no desert next to
tundra without a plausible transition) because adjacent cells in the table
are always plausible climate neighbors.

Special biomes placed by terrain, not climate:
- **Ocean**: any area below sea level
- **Beach**: 1–3 block band at sea level, adjacent to ocean
- **Deep Cave**: underground, below y=30 (not a surface biome)
- **River**: follows the river system from worldgen

## Per-biome identity spec

Each biome below has: a surface block, a tree/feature type, a surface
feature density, a color-grade spec, and a fog color.

### Tundra
- Surface: `permafrost` block
- Features: sparse dead shrubs (2% density), occasional ice boulders
- Density: 0.02
- Grade: hue=–5, sat=0.85, warm=–0.09, bright=1.05 (cold, slightly bright)
- Fog: pale blue-grey (#c8d4e0)
- Unique: snow particle overlay (ambient falling snow, not rain)

### Highland
- Surface: stone (bare rock peaks), with thin soil/grass patches
- Features: exposed rock outcrops (the `volcanic_basalt` or a grey stone
  cluster), alpine flowers at lower heights
- Density: 0.04
- Grade: hue=+2, sat=0.9, warm=–0.03, bright=1.03
- Fog: clear, slightly grey (#d8dce0)

### Plains
- Surface: grass (standard)
- Features: tall grass (25% density), occasional single oak
- Density: 0.25
- Grade: hue=+2, sat=1.0, warm=+0.01, bright=1.02
- Fog: slightly warm pale blue (#d4e0d4)
- This is the "default" biome — everything else should differ from this

### Savanna
- Surface: `gilded_grass` (golden tint)
- Features: acacia-style flat-top trees (sparse, 5% density), dry grass tufts
- Density: 0.18
- Grade: hue=+8, sat=0.9, warm=+0.07, bright=1.06 (warm, dry)
- Fog: warm tan-yellow (#e8d8a0)

### Desert
- Surface: sand (top), sandstone (2 blocks deep)
- Features: cactus (3% density), dead shrubs (1%)
- Density: 0.04
- Grade: hue=+10, sat=0.82, warm=+0.09, bright=1.08 (very warm, slightly desaturated)
- Fog: dusty orange-tan (#e0c888)
- Unique: no rain events

### Oak Forest
- Surface: grass
- Features: oak trees (20% density), shrubs, mushrooms
- Density: 0.35
- Grade: hue=+5, sat=1.05, warm=+0.02, bright=0.98
- Fog: green-tinted (#c0d4b8)

### Temperate Forest
- Surface: grass, some patches of moss
- Features: mixed oak/birch trees (25% density), ferns
- Density: 0.4
- Grade: hue=+3, sat=1.08, warm=0.0, bright=0.96
- Fog: neutral green (#bcd0bc)

### Meadow
- Surface: grass (lush green)
- Features: flowers (12% density), tall grass, occasional willow-style tree
- Density: 0.45
- Grade: hue=+5, sat=1.15, warm=+0.03, bright=1.05 (lush, vivid green)
- Fog: bright green-blue (#b4d8b4)
- This is the "beautiful" biome — should look immediately inviting

### Swamp
- Surface: mud/swamp grass (dark surface block — use existing dark grass or `bog_peat`)
- Features: mangrove-style trees (droopy, over water), lily pads on water surfaces
- Density: 0.3
- Grade: hue=+14, sat=1.2, warm=–0.04, bright=0.88 (murky, green-heavy)
- Fog: dense green-grey (#788870)
- Unique: frequent rain events, water patches on surface

### Bog
- Surface: `bog_peat`
- Features: dead trees (no leaves), cattail plants, fog patches
- Density: 0.12
- Grade: hue=+16, sat=1.1, warm=–0.06, bright=0.84 (dark, murky)
- Fog: very dense, dark olive-green (#606858)

### Mushroom Forest
- Surface: mycelium (if exists, else dark grass)
- Features: giant mushroom caps as "trees" (using `mushroom_cap` block),
  smaller mushrooms as ground cover
- Density: 0.35
- Grade: hue=+18, sat=1.25, warm=+0.04, bright=0.92 (vivid purple-warm)
- Fog: warm purple-grey (#907880)
- Unique: no hostile mob spawns (matches Minecraft's rule — feels safe and
  magical)

### Snowy Forest / Snowy Hills / Snowy Tundra
- Surface: snow layer on top of normal surface block
- Features: spruce trees (snow-capped tips)
- Density: varies by subtype (forest=0.3, hills=0.15, tundra=0.04)
- Grade: hue=–6, sat=0.88, warm=–0.1, bright=1.08 (cold, blue-white)
- Fog: pale cold blue (#c0cce0)
- Unique: snow weather always, ice on standing water

### Volcanic
- Surface: `volcanic_basalt`
- Features: basalt spikes (4% density), lava pools on surface (rare)
- Density: 0.06
- Grade: hue=+4, sat=0.78, warm=+0.08, bright=0.85 (dark, slightly warm)
- Fog: dark grey-orange (#786858)
- Unique: ember particles rising from hot blocks

### Mesa / Badlands / Taiga / Deep Taiga
Define analogously to the above — each gets a distinct grade, fog,
and at least one unique surface block. The pattern is the same:
warm/cool push, saturation, a signature feature, and a fog color that
tells a climate story.

## Transition zone implementation

At any biome boundary, the surface block and feature placement blend
across a 4–8 block wide transition band. The blend width is jittered by
noise (so it's not a mathematically perfect straight line):
```rust
let blend_width = 4.0 + noise_jitter(x, z, seed) * 4.0;  // 4–8 blocks
let d = distance_to_biome_boundary(x, z);
let blend_factor = smoothstep(0.0, blend_width, d);
// blend_factor = 0 at boundary center, 1 well inside biome A
// use blend_factor to mix between biome A and B surface/features
```

## The 5-second test checklist

For every biome, answer these questions:
1. Can a player name the biome from 5 seconds of looking at it? (Y/N)
2. Does the fog color feel like the climate? (Y/N)
3. Is there at least one thing in this biome that doesn't appear in any
   other biome? (Y/N)
4. Does the color grade distinguish it from the adjacent biomes? (Y/N)

Any "N" answer means the biome needs more work before the section is done.
Log the answers for each biome in DEVLOG.md as part of the job evidence.
