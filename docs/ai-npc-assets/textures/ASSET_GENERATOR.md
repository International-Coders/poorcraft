# Procedural Asset Generator — Reference Document

## Why a generator, not hand-painted art

At this stage of development, the game needs many textures quickly:
CTM strips (47 tiles each × 8 block types = 376 tiles), 6 entity skins,
and various block textures for new biome/faction blocks. A generator that
follows explicit pixel-art rules produces consistent, game-appropriate
assets faster than painting each tile by hand — and because the rules are
encoded in code, the output is reproducible and can be iterated by changing
a parameter, not redrawing every tile.

The generator does NOT produce "AI-looking" art because it follows
pixel-art rules, not neural network latent space exploration. The rules are:
- Defined colour palette (never sample outside the game's palette).
- Explicit pixel-level decisions (edge pixels are darker; interior pixels
  get noise; diagonal gradients are integer-stepped, not smooth).
- No anti-aliasing (pixel art doesn't have it).
- Tile must tile seamlessly (edge pixels match the opposite edge).

## xorshift64 PRNG (implement this in xtask, no external crate)

```rust
pub struct Xorshift64 { state: u64 }

impl Xorshift64 {
    pub fn new(seed: u64) -> Self {
        // Avoid 0 state (xorshift produces all zeros for 0 seed)
        Self { state: if seed == 0 { 0xdeadbeef_cafef00d } else { seed } }
    }
    pub fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
    pub fn next_u8(&mut self) -> u8 { (self.next() & 0xFF) as u8 }
    pub fn next_range(&mut self, min: i32, max: i32) -> i32 {
        // min inclusive, max exclusive
        min + (self.next() % (max - min) as u64) as i32
    }
    pub fn next_f32(&mut self) -> f32 {
        (self.next() & 0xFFFFFF) as f32 / 0xFFFFFF as f32
    }
}
```

## 2D hash noise (no external crates)

```rust
/// Returns a value 0.0..1.0 for integer coordinates (x, y).
/// Deterministic: same (x, y, seed) always returns the same value.
pub fn hash_noise_2d(x: i32, y: i32, seed: u64) -> f32 {
    let h = (x as u64)
        .wrapping_mul(127_1)
        .wrapping_add((y as u64).wrapping_mul(311_7))
        .wrapping_add(seed);
    // Avalanche hash
    let h = h ^ (h >> 30);
    let h = h.wrapping_mul(0xbf58476d1ce4e5b9);
    let h = h ^ (h >> 27);
    let h = h.wrapping_mul(0x94d049bb133111eb);
    let h = h ^ (h >> 31);
    (h & 0xFFFFFF) as f32 / 0xFFFFFF as f32
}
```

## Pixel-art rules (apply to every texture type)

**Rule 1 — No anti-aliasing.** Every pixel is one of a small set of
discrete colours. No blending between colours. No fractional brightness.

**Rule 2 — Limited colour range per tile.** A grass tile should use at
most 6–8 distinct colour values (the base green, 2 darker variants, 2
lighter variants, 1 highlight, 1 shadow). More colours make a tile look
busy. Fewer make it look flat.

**Rule 3 — Edge pixels follow edge rules.** In a tiling texture, the
pixels on each edge must match the opposite edge (top row matches bottom
row, left column matches right column) for the texture to tile seamlessly.
For CTM tiles, only the edges that connect to matching neighbours need
to be seamless; exposed edges are deliberately different.

**Rule 4 — Noise is additive, not the base.** Start with a flat base
colour, then add noise as ±Δ brightness variations. Don't start with
noise and try to push it toward the right colour.

**Rule 5 — Details are sparse.** A grass tile with 2–3 dark pixel "pebbles"
looks realistic. A grass tile with 40% dark pixels looks like static.
Use `rng.next() % 100 < density_percent` to control sparsity.

## Grass CTM strip generation rules (detailed)

The grass CTM strip must produce a meadow-like surface for the interior
tiles (fully surrounded) and clean edges for the exposed tiles.

**Base colour palette for grass:**
```
base:       RGB(90, 138, 42)   — medium grass green
dark1:      RGB(72, 110, 34)   — shadow/depth pixel
dark2:      RGB(58, 88, 27)    — deeper shadow
light1:     RGB(110, 162, 52)  — highlight pixel
light2:     RGB(128, 180, 60)  — bright highlight
blade:      RGB(80, 128, 36)   — grass blade detail pixel
```

**For each of the 47 tiles:**
1. Fill the 16×16 tile with `base`.
2. Apply hash noise: for each pixel (px, py), if
   `hash_noise_2d(tile_x*16+px, tile_y*16+py, seed) < 0.25`, darken
   by one step (base → dark1 → dark2 — don't go darker than dark2).
3. Apply grass blade details: sparse vertical 1-pixel lines (length 2–3px)
   in `blade` colour: `if rng.next() % 100 < 8 { draw_blade_at(px, py) }`.
4. For tiles with exposed N/S/E/W edges (i.e., the bitmask has an open
   direction): draw a 1-pixel border on the exposed edge(s) in `dark2`.
   This gives connected surfaces a subtle edge definition.
5. For the isolated tile (tile 46): draw 1-pixel borders on all 4 edges
   in `dark2`.
6. For interior tiles (tile 0, fully surrounded): add 2–4 randomly placed
   single-pixel `light2` highlights (bright spots = visible sun dappling
   in a large meadow).

**Seamlessness:** Because the base + noise is computed from absolute pixel
coordinates (tile_x*16+px, tile_y*16+py), adjacent tiles that are both
"interior" will have continuous noise across their shared edge, producing
the large-meadow effect you described.

## Entity skin generation rules (detailed)

Villager skin dimensions: 64×32 pixels (standard humanoid layout).
Body regions for a simple box-model character:
```
Head front:  8×8 at (8, 8)
Body front: 8×12 at (20, 20)
Arm front:  4×12 at (44, 20) and (36, 52)
Leg front:  4×12 at (4, 20) and (20, 52)
```

(If the game uses a different skin layout, read the existing entity
rendering code to determine the correct UV regions before generating.)

**Per-faction rules:**
- Body region fill: faction primary colour (from the palette in
  MAIN_MENU_REDESIGN.md from the prior pack).
- Arm/Leg fill: faction primary colour, ×0.9 brightness (slightly darker).
- Head fill: neutral skin tone RGB(196, 149, 106).
- Eyes: 1×1 pixel pupils at (10, 9) and (13, 9) in dark colour (RGB 30, 20, 15).
  Iris: faction accent colour, 1×1 pixel adjacent to pupil.
- Faction symbol: 4×4 pixel pattern stamped at the centre of the body
  front region. Each faction has a defined 4×4 binary pattern:

```
Accord (scale):    ####    Ironborn (anvil):  .##.
                   #..#                       .##.
                   #..#                       ####
                   ####                       .##.

Covenant (flame):  .##.    Free Holds (wheat):.#.#
                   ###.                       .###
                   .###                       .##.
                   ..#.                       ..#.

Ashen (book):      ####    Nameless (chain):  #.#.
                   #..#                       ####
                   ####                       #.#.
                   ####                       ####
```

Symbol pixels use the faction secondary colour (slightly darker than primary).
Background pixels (`.`) are the faction primary colour (same as body fill).

## Output format requirements

All generator output is PNG format, written using the `image` crate
(already in the project's dependency graph via `lf_assets`). If `image`
is not already a dependency of `xtask`, add it.

Output paths:
- CTM strips: `assets/ctm/<block_id>.png`
- Entity skins: `assets/skins/npc/<faction_id>.png`
- Block noise: `assets/textures/<block_id>_generated.png`
  (note the `_generated` suffix — never overwrite a non-generated asset)

The generator always checks if the output path exists before writing.
If the file exists and does NOT have the `_generated` suffix, abort and
print: "Refusing to overwrite existing hand-crafted asset: <path>".
Only `_generated` files can be overwritten by the generator.
