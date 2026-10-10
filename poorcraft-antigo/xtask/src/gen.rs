//! Section F (ai-npc-assets): procedural asset generator. Deterministic
//! pixel-art output — integer arithmetic only (xorshift64 + an avalanche
//! hash), no system time, no OS RNG. Same seed + type => bit-identical
//! bytes on any machine.

use image::RgbaImage;

/// Seeded PRNG (ai-npc-assets/ASSET_GENERATOR.md). Never pull `rand`.
pub struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    pub fn new(seed: u64) -> Self {
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
    /// min inclusive, max exclusive
    pub fn next_range(&mut self, min: i32, max: i32) -> i32 {
        min + (self.next() % (max - min) as u64) as i32
    }
}

/// Deterministic 2D hash noise in 0.0..1.0 for integer coordinates.
pub fn hash_noise_2d(x: i32, y: i32, seed: u64) -> f32 {
    let h = (x as u64)
        .wrapping_mul(127_1)
        .wrapping_add((y as u64).wrapping_mul(311_7))
        .wrapping_add(seed);
    let h = h ^ (h >> 30);
    let h = h.wrapping_mul(0xbf58476d1ce4e5b9);
    let h = h ^ (h >> 27);
    let h = h.wrapping_mul(0x94d049bb133111eb);
    let h = h ^ (h >> 31);
    (h & 0xFFFFFF) as f32 / 0xFFFFFF as f32
}

fn clamp8(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// F1 grass-ctm-strip: the full 192x512 strip for grass (and every other
/// CTM block — the rules are per-block). Delegates to the game's own
/// generator so exported art and in-game art can never drift apart.
pub fn grass_ctm_strip(seed: u64) -> RgbaImage {
    lf_assets::generate_ctm_strip_atlas_seeded(seed)
}

/// F1 stone-ctm-strip: mid-grey with hash-noise grain and sparse 1px
/// cracks; exposed tile edges get an interior-side shadow (same CTM rect
/// layout as the grass strip).
pub fn stone_ctm_strip(seed: u64) -> RgbaImage {
    let mut img = RgbaImage::new(192, 512);
    for (b, _name) in lf_assets::CTM_BLOCKS.iter().enumerate() {
        for tile in 0..47u8 {
            let mut tile_img = RgbaImage::new(16, 16);
            let base = [106u8, 106, 106];
            for y in 0..16 {
                for x in 0..16 {
                    let gx = (b as i32 * 64) + x as i32;
                    let gy = tile as i32 * 16 + y as i32;
                    let n = hash_noise_2d(gx, gy, seed) - 0.5; // -0.5..0.5
                    let d = (n * 18.0) as i32; // ±9 brightness
                    tile_img.put_pixel(x as u32, y as u32, image::Rgba([
                        clamp8(base[0] as i32 + d),
                        clamp8(base[1] as i32 + d),
                        clamp8(base[2] as i32 + d),
                        255,
                    ]));
                }
            }
            let mask = lf_assets::ctm_tile_mask(tile);
            // sparse cracks: 2-4 short dark 1px lines, xorshift-placed
            let mut rng = Xorshift64::new(seed ^ ((tile as u64) << 8) ^ (b as u64) << 16);
            let cracks = rng.next_range(2, 5);
            for _ in 0..cracks {
                let (mut cx, mut cy) = (rng.next_range(1, 15), rng.next_range(1, 15));
                let len = rng.next_range(2, 5);
                let horiz = rng.next_range(0, 2) == 0;
                for _ in 0..len {
                    if cx <= 0 || cx >= 15 || cy <= 0 || cy >= 15 {
                        break;
                    }
                    tile_img.put_pixel(cx as u32, cy as u32,
                        image::Rgba([74, 74, 78, 255]));
                    if horiz { cx += 1; } else { cy += 1; }
                }
            }
            shade_exposed_edges(&mut tile_img, mask);
            let col = (tile % 12) as i32 * 16;
            let row = (b as i32 * 4) + (tile as i32) / 12;
            image::imageops::replace(&mut img, &tile_img, col as i64, row as i64);
        }
    }
    img
}

/// Darken the tile's exposed-edge border (interior side shadow) to match
/// the in-game CTM edge rules.
fn shade_exposed_edges(tile: &mut RgbaImage, mask: u8) {
    let f = |p: &mut image::Rgba<u8>| {
        let d = p.0;
        *p = image::Rgba([((d[0] as u16) * 8 / 10) as u8, ((d[1] as u16) * 8 / 10) as u8, ((d[2] as u16) * 8 / 10) as u8, d[3]]);
    };
    let bit_n = 1 << 6;
    let bit_e = 1 << 3;
    let bit_s = 1 << 1;
    let bit_w = 1 << 4;
    if mask & bit_n == 0 {
        for x in 0..16 { f(tile.get_pixel_mut(x, 0)); }
    }
    if mask & bit_s == 0 {
        for x in 0..16 { f(tile.get_pixel_mut(x, 15)); }
    }
    if mask & bit_w == 0 {
        for y in 0..16 { f(tile.get_pixel_mut(0, y)); }
    }
    if mask & bit_e == 0 {
        for y in 0..16 { f(tile.get_pixel_mut(15, y)); }
    }
}

/// Faction identity: (primary, accent, secondary) RGB + a 4x4 symbol stamp
/// (rows of bits, MSB first).
pub struct FactionSkin {
    pub id: &'static str,
    pub primary: [u8; 3],
    pub accent: [u8; 3],
    pub secondary: [u8; 3],
    pub symbol: [u8; 4],
}

pub const FACTIONS: [FactionSkin; 6] = [
    FactionSkin { id: "accord", primary: [96, 110, 140], accent: [210, 215, 228], secondary: [58, 68, 92], symbol: [0b1111, 0b1001, 0b1001, 0b1111] },
    FactionSkin { id: "ironborn", primary: [122, 74, 48], accent: [232, 160, 60], secondary: [82, 48, 30], symbol: [0b0110, 0b0110, 0b1111, 0b0110] },
    FactionSkin { id: "covenant", primary: [140, 48, 36], accent: [244, 150, 60], secondary: [96, 30, 22], symbol: [0b0110, 0b1110, 0b0111, 0b0010] },
    FactionSkin { id: "free_holds", primary: [150, 128, 62], accent: [226, 204, 120], secondary: [104, 88, 40], symbol: [0b0101, 0b0111, 0b0110, 0b0010] },
    FactionSkin { id: "ashen_order", primary: [140, 138, 132], accent: [230, 228, 220], secondary: [96, 94, 90], symbol: [0b1111, 0b1001, 0b1111, 0b1111] },
    FactionSkin { id: "nameless", primary: [38, 38, 42], accent: [120, 118, 128], secondary: [22, 22, 26], symbol: [0b1010, 0b1111, 0b1010, 0b1111] },
];

/// F1 entity-skin: 64x32 humanoid layout — skin head, faction body/limbs,
/// faction-iris eyes, 4x4 faction symbol stamped centre-chest.
pub fn entity_skin(faction_idx: usize, seed: u64) -> RgbaImage {
    let f = &FACTIONS[faction_idx % FACTIONS.len()];
    let mut img = RgbaImage::new(64, 32);
    let fill = |img: &mut RgbaImage, x0: u32, y0: u32, w: u32, h: u32, c: [u8; 3]| {
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                img.put_pixel(x, y, image::Rgba([c[0], c[1], c[2], 255]));
            }
        }
    };
    let skin = [196u8, 149, 106]; // neutral warm skin tone
    // head front 8x8 at (8,8)
    fill(&mut img, 8, 8, 8, 8, skin);
    // body front 8x12 at (20,20)
    fill(&mut img, 20, 20, 8, 12, f.primary);
    // arms 4x12 at (44,20) and (36,52->wrap: use (52,4)) — classic layout
    // uses mirrored regions; keep the two visible front faces
    let limb = [((f.primary[0] as u16) * 9 / 10) as u8, ((f.primary[1] as u16) * 9 / 10) as u8, ((f.primary[2] as u16) * 9 / 10) as u8];
    fill(&mut img, 44, 20, 4, 12, limb);
    fill(&mut img, 36, 52 % 32, 4, 12.min(32 - 52 % 32), limb);
    // legs 4x12 at (4,20) and (20,52->wrap)
    fill(&mut img, 4, 20, 4, 12, limb);
    // clothing detail: accent belt across the body
    fill(&mut img, 20, 25, 8, 2, f.accent);
    // eyes: 2px-wide dark with a faction-iris pixel each
    for (ex, iris) in [(10usize, 11usize), (13, 12)] {
        img.put_pixel(ex as u32, 12, image::Rgba([30, 20, 15, 255]));
        img.put_pixel(ex as u32 + 1, 12, image::Rgba([30, 20, 15, 255]));
        img.put_pixel(iris as u32, 13, image::Rgba([f.accent[0], f.accent[1], f.accent[2], 255]));
    }
    // faction symbol 4x4 stamped centre-chest in the secondary colour
    for (row, bits) in f.symbol.iter().enumerate() {
        for col in 0..4 {
            if bits & (1 << (3 - col)) != 0 {
                img.put_pixel(22 + col as u32, 22 + row as u32,
                    image::Rgba([f.secondary[0], f.secondary[1], f.secondary[2], 255]));
            }
        }
    }
    // deterministic single-pixel wear marks (Rule 5: sparse detail)
    let mut rng = Xorshift64::new(seed ^ (faction_idx as u64) << 20);
    for _ in 0..4 {
        let x = rng.next_range(20, 28) as u32;
        let y = rng.next_range(27, 32) as u32;
        img.put_pixel(x, y, image::Rgba([f.secondary[0], f.secondary[1], f.secondary[2], 255]));
    }
    img
}

/// F1 block-noise: flat base colour, per-pixel brightness variation from
/// the hash noise, 1px darker edge on all four sides.
pub fn block_noise(base: [u8; 3], variation: i32, size: u32, seed: u64) -> RgbaImage {
    let mut img = RgbaImage::new(size, size);
    for y in 0..size {
        for x in 0..size {
            let n = hash_noise_2d(x as i32, y as i32, seed) - 0.5;
            let d = (n * 2.0) * variation as f32;
            img.put_pixel(x, y, image::Rgba([
                clamp8(base[0] as i32 + d as i32),
                clamp8(base[1] as i32 + d as i32),
                clamp8(base[2] as i32 + d as i32),
                255,
            ]));
        }
    }
    let edge = |p: &mut image::Rgba<u8>| {
        let d = p.0;
        *p = image::Rgba([((d[0] as u16) * 9 / 10) as u8, ((d[1] as u16) * 9 / 10) as u8, ((d[2] as u16) * 9 / 10) as u8, d[3]]);
    };
    for i in 0..size {
        edge(img.get_pixel_mut(i, 0));
        edge(img.get_pixel_mut(i, size - 1));
        edge(img.get_pixel_mut(0, i));
        edge(img.get_pixel_mut(size - 1, i));
    }
    img
}

pub fn parse_hex(s: &str) -> Result<[u8; 3], String> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return Err(format!("--base-color wants RRGGBA hex, got {:?}", s));
    }
    let byte = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string());
    Ok([byte(0)?, byte(2)?, byte(4)?])
}

/// F2: generate every asset set under `root` (the repo root), refusing to
/// overwrite existing hand-crafted files. Returns the summary lines.
pub fn gen_all(root: &std::path::Path) -> Vec<String> {
    let mut log = Vec::new();
    // 8 CTM strips (styled, seed 0 = the in-game art)
    for block in lf_assets::CTM_BLOCKS.iter() {
        let path = root.join(format!("assets/ctm/{}.png", block.art));
        gen_to_png(&path, &mut log, || grass_ctm_strip(0));
    }
    // 6 faction entity skins
    for (i, f) in FACTIONS.iter().enumerate() {
        let path = root.join(format!("assets/skins/npc/{}.png", f.id));
        gen_to_png(&path, &mut log, || entity_skin(i, 42));
    }
    log
}

/// Write helper enforcing the "never overwrite hand-crafted assets" rule:
/// existing files are skipped and reported; only new paths are written.
fn gen_to_png(path: &std::path::Path, log: &mut Vec<String>, make: impl FnOnce() -> RgbaImage) {
    let p = path;
    if p.exists() {
        log.push(format!("SKIP (exists): {}", path.display()));
        return;
    }
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match make().save(p) {
        Ok(()) => log.push(format!("wrote {}", path.display())),
        Err(e) => log.push(format!("FAILED {}: {}", path.display(), e)),
    }
}
