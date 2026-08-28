# World Generation Rework — Reference Document

## The core problem

Too many mountains. Not enough flat land. No climate logic. Structures
floating or buried. Rivers absent or wrong. Biomes placed without geographic
reasoning. This document specifies the terrain generation architecture that
fixes all of these.

## Terrain generation pipeline (new architecture)

The pipeline below replaces or extends whatever is currently in
`lf_worldgen`. Each step is a pure function of `(x, z, seed)` → height
or type value, with no mutable global state. This is important for
correctness with the existing chunked streaming world.

### Step 1 — Continental map

```rust
/// Returns a value 0.0..1.0 where:
/// < 0.3 = ocean/lowland
/// 0.3..0.7 = transition hills  
/// > 0.7 = highland/mountain zone
fn continental_factor(x: f64, z: f64, seed: u64) -> f32 {
    // Low-frequency noise — very large features (continent-scale)
    let freq = 1.0 / 1200.0;  // one "continent" per ~1200 blocks
    let raw = noise_fbm(x * freq, z * freq, seed ^ 0x01, octaves=4, lacunarity=2.0, gain=0.5);
    smoothstep(0.2, 0.8, (raw + 1.0) * 0.5)  // remap –1..1 to 0..1 with smooth edges
}
```

Tuning: the `1200.0` frequency value controls how large continental features
are. Smaller = more, smaller mountains; larger = fewer, bigger features. Do
not go below 600 (produces tiny fragmented features) or above 2400 (features
so large they're not visible at play scale).

### Step 2 — Detail noise (fine terrain within each zone)

```rust
fn detail_noise(x: f64, z: f64, seed: u64) -> f32 {
    // Mid-frequency noise for local terrain variation
    let freq = 1.0 / 80.0;
    noise_fbm(x * freq, z * freq, seed ^ 0x02, octaves=6, lacunarity=1.9, gain=0.55)
    // Returns –1.0..1.0
}
```

### Step 3 — Ridge noise (mountain ridgelines only)

```rust
fn ridge_noise(x: f64, z: f64, seed: u64) -> f32 {
    let freq = 1.0 / 120.0;
    let raw = noise_fbm(x * freq, z * freq, seed ^ 0x03, octaves=4, lacunarity=2.1, gain=0.45);
    // Ridge transform: invert absolute value, sharpen
    let ridge = 1.0 - raw.abs();
    ridge.powf(2.5)  // sharpen peaks
}
```

### Step 4 — Combine into final terrain height

```rust
fn terrain_height(x: f64, z: f64, seed: u64) -> i32 {
    let cf = continental_factor(x, z, seed);
    let detail = detail_noise(x, z, seed);
    let ridge = ridge_noise(x, z, seed);

    let sea_level: f32 = 64.0;
    
    // Lowland zone (cf < 0.3): ±8 blocks variation around sea level
    let lowland = sea_level + detail * 8.0;
    
    // Highland zone (cf > 0.7): mountains with ridgelines
    let highland = sea_level + 30.0 + detail * 25.0 + ridge * 35.0;
    // Max highland peak: sea_level + 30 + 25 + 35 = sea_level + 90
    // i.e., mountains can reach ~154 blocks high (sea level 64 + 90)
    
    // Smooth lerp between zones
    let height = lerp(lowland, highland, cf.clamp(0.0, 1.0));
    
    height.round() as i32
}
```

### Step 5 — Ocean/water fill

Anywhere `terrain_height < sea_level (64)`: fill from terrain_height+1
to sea_level with water blocks. The terrain itself is stone/sand below.

### Step 6 — River system

Rivers require a separate flow map, computed once per region and cached:

```rust
/// Returns a river factor 0.0..1.0 where 1.0 = center of river channel
fn river_factor(x: f64, z: f64, seed: u64) -> f32 {
    // Low-frequency river path noise (meanders, not straight lines)
    let freq = 1.0 / 400.0;
    let flow = noise_simplex(x * freq, z * freq, seed ^ 0x10);
    
    // A river exists where flow noise is near 0 (the valley between
    // two noise peaks) AND continental factor is in lowland range
    let cf = continental_factor(x, z, seed);
    if cf > 0.55 { return 0.0; }  // no rivers in highlands
    
    let river_proximity = 1.0 - (flow.abs() / 0.08).clamp(0.0, 1.0);
    river_proximity * (1.0 - cf / 0.55)  // stronger in lowlands
}

/// Modifies terrain height for rivers
fn apply_river_carving(base_height: i32, river_f: f32, x: f64, z: f64, seed: u64) -> i32 {
    if river_f < 0.1 { return base_height; }
    
    // River channel: carve down to near sea level
    let river_bed = 62;  // 2 below sea level, becomes water
    let carve_depth = lerp(base_height as f32, river_bed as f32, river_f);
    carve_depth.round() as i32
}
```

### Step 7 — Surface block selection

Based on height, biome, and proximity to transitions:
- Above `highland_snow_line` (height > sea_level + 60): snow/ice surface
- Mountain rock zone (height > sea_level + 30 AND cf > 0.7): stone surface
- Normal terrain: biome surface block (see biome table)
- Beach zone (height within 3 of sea_level AND not river): sand
- River banks: gravel or sand, 1–2 blocks wide adjacent to river

## Caves

### Worm cave generation
- 3D Perlin/Simplex noise, threshold-based: if `cave_noise(x, y, z) > 0.72`
  the block is air.
- Cave noise uses separate frequency for horizontal (wider tunnels) vs.
  vertical (limiting tunnel height).
- Surface breach prevention: multiply cave openness by 0 below y=48,
  smooth ramp from 0 to 1 between y=48 and y=56. Caves rarely breach above
  y=56 (visible surface holes become very rare).
- Cave biome: below y=30, the stone type transitions to `deep_slate`.
  Below y=10: lava lakes (lava_source blocks filling depressions).

### Stalactites / stalagmites (simple pass after cave generation)
Iterate over cave-ceiling and cave-floor blocks:
- Stalactite: a stone block with air below it and 2+ solid horizontal
  neighbors → 15% chance to extend a spike of 1–4 blocks downward.
- Stalagmite: a stone block with air above it and 2+ solid horizontal
  neighbors → 10% chance to extend 1–3 blocks upward.
- Use the block type of the ceiling/floor stone for the spike material.

## Flat land proportion validation

After implementing the above, generate 5 test seeds (any 5 values) and
for each, count the fraction of surface blocks at height within ±6 of
sea level (64). This fraction should be ≥ 40%. If it's consistently
below 40%, increase the lowland zone width by reducing the
`smoothstep(0.2, 0.8...)` range (try `smoothstep(0.35, 0.75...)`).

Write the actual measured flat-land percentage for 3 seeds into DEVLOG.md
as part of the job evidence.

## Performance requirements

Terrain generation is already run per-chunk on a background thread (the
existing streaming architecture). The above pipeline adds more noise
samples per column than the current approach likely uses. Profile with:
```
cargo run --release -p xtask -- vistest shots
```
and check if chunk generation time increases significantly. If it does,
the continent/detail/ridge noises can be sampled at lower octave counts
(reduce octaves by 1 each until performance is acceptable). Do not
sacrifice correctness for performance if the measurements show acceptable
frame pacing.
