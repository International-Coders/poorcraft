use image::{Rgba, RgbaImage};

/// Canonical texture atlas layer order. Block ids map onto these indices.
pub const TEXTURE_NAMES: [&str; 41] = [
    "stone", "grass", "dirt", "sand", "mycelium", "snow",
    "log", "leaves", "coal_ore", "iron_ore", "water", "torch_item", "crafting_table",
    "furnace", "chest", "planks", "glass",
    // wood/biome variants (18-30)
    "birch_log", "spruce_log", "dark_log", "cherry_log",
    "birch_leaves", "spruce_leaves", "dark_leaves", "cherry_leaves", "pale_leaves",
    "red_sand", "terracotta", "moss", "ice",
    // industrial ores (31-34)
    "copper_ore", "tin_ore", "bauxite_ore", "sulfur_ore",
    // machines + benches (35-40)
    "smithing_table", "coal_generator", "electric_furnace", "crusher", "assembler",
    "research_bench",
    "mod",
];

/// Texture atlas layer for a block id (see lf_voxel::BlockState / lf_worldgen::BlockId).
pub fn texture_index_for_block(block_id: u32) -> u32 {
    match block_id {
        1 => 0, // stone
        2 => 1, // grass
        3 => 2, // dirt
        4 => 3, // sand
        5 => 4, // mycelium
        6 => 5, // snow
        7 => 6, // log
        8 => 7, // leaves
        9 => 8, // coal ore
        10 => 9, // iron ore
        11 => 10, // water
        12 => 11, // torch
        14 => 12, // crafting table
        15 => 13, // furnace
        16 => 14, // chest
        17 => 15, // planks
        18 => 16, // glass
        19 => 17, // birch log
        20 => 18, // spruce log
        21 => 19, // dark log
        22 => 20, // cherry log
        23 => 21, // birch leaves
        24 => 22, // spruce leaves
        25 => 23, // dark leaves
        26 => 24, // cherry leaves
        27 => 25, // pale leaves
        28 => 26, // red sand
        29 => 27, // terracotta
        30 => 28, // moss
        31 => 29, // ice
        32 => 30, // copper ore
        33 => 31, // tin ore
        34 => 32, // bauxite ore
        35 => 33, // sulfur ore
        36 => 34, // smithing table
        37 => 35, // coal generator
        38 => 36, // electric furnace
        39 => 37, // crusher
        40 => 38, // assembler
        41 => 39, // research bench
        id if id >= 100 => 40, // mod blocks (registry::MOD_BLOCK_BASE)
        _ => 0,
    }
}

/// Channel helper: clamp to 0..=255 (all math in u32, cast at the end).
fn ch(v: u32) -> u8 {
    v.min(255) as u8
}

/// Generates a procedural 16x16 pixel-art texture for a block (e.g. stone, grass, dirt).
pub fn generate_block_texture(name: &str) -> RgbaImage {
    let mut img = RgbaImage::new(16, 16);
    for x in 0u32..16 {
        for y in 0u32..16 {
            let color = match name {
                "stone" => {
                    let v = 120 + ((x * 7 + y * 13) % 20);
                    Rgba([ch(v), ch(v), ch(v), 255])
                }
                "grass" => {
                    if y < 4 {
                        Rgba([80, 160, 60, 255])
                    } else {
                        let v = 110 + ((x * 5 + y * 9) % 15);
                        Rgba([ch(v), 90, 50, 255])
                    }
                }
                "dirt" => {
                    let v = 100 + ((x * 11 + y * 7) % 25);
                    Rgba([ch(v), 80, 40, 255])
                }
                "sand" => {
                    let v = 210 + ((x * 3 + y * 5) % 12);
                    Rgba([ch(v), ch(200 - (x % 3) * 4), 150, 255])
                }
                "mycelium" => {
                    if y < 4 {
                        let v = 130 + ((x * 7 + y * 3) % 30);
                        Rgba([ch(v), ch(v - 20), ch(v + 30), 255])
                    } else {
                        let v = 90 + ((x * 13 + y * 11) % 20);
                        Rgba([ch(v), ch(v - 30), ch(v - 10), 255])
                    }
                }
                "snow" => {
                    let v = 235 + ((x + y * 3) % 5);
                    Rgba([ch(v), ch(v.min(250)), ch(v.min(252)), 255])
                }
                "log" => {
                    let v = 90 + ((x * 9 + y * 5) % 18);
                    let edge = if x == 0 || x == 15 || y == 0 || y == 15 { 12 } else { 0 };
                    Rgba([ch(v - 20 + edge), ch(v - 45 + edge), ch(v - 60 + edge), 255])
                }
                "leaves" => {
                    let v = 40 + ((x * 13 + y * 7) % 40);
                    Rgba([ch(v / 3), ch(v + 60), ch(v / 4), 255])
                }
                "coal_ore" => {
                    let speck = (x * 7 + y * 11) % 29 < 7;
                    if speck {
                        Rgba([30, 30, 34, 255])
                    } else {
                        let v = 120 + ((x * 7 + y * 13) % 20);
                        Rgba([ch(v), ch(v), ch(v), 255])
                    }
                }
                "iron_ore" => {
                    let speck = (x * 5 + y * 13) % 31 < 7;
                    if speck {
                        Rgba([216, 175, 147, 255])
                    } else {
                        let v = 120 + ((x * 7 + y * 13) % 20);
                        Rgba([ch(v), ch(v), ch(v), 255])
                    }
                }
                "torch_item" => {
                    // mostly transparent with a bright flame tip and a stick
                    let stick = x == 7 || x == 8;
                    let flame = stick && y >= 4 && y <= 7;
                    if flame {
                        Rgba([255, 220, 120, 255])
                    } else if stick && y > 7 {
                        Rgba([120, 90, 50, 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "crafting_table" => {
                    let v = 140 + ((x * 5 + y * 11) % 20);
                    let grid_line = x == 5 || x == 10 || y == 5 || y == 10;
                    if grid_line {
                        Rgba([90, 60, 30, 255])
                    } else {
                        Rgba([ch(v - 20), ch(v - 70), ch(v - 100), 255])
                    }
                }
                "furnace" => {
                    let mouth = (4..12).contains(&x) && (6..13).contains(&y);
                    if mouth {
                        let glow: u32 = if (x + y) % 3 == 0 { 90 } else { 0 };
                        Rgba([ch(120 + glow), ch(60 + glow / 2), 30, 255])
                    } else {
                        let v = 110 + ((x * 7 + y * 13) % 20);
                        Rgba([ch(v), ch(v), ch(v), 255])
                    }
                }
                "chest" => {
                    let band = y == 7 || y == 8;
                    let latch = (7..=8).contains(&x) && (6..=10).contains(&y);
                    if latch {
                        Rgba([180, 170, 120, 255])
                    } else if band {
                        Rgba([90, 60, 30, 255])
                    } else {
                        let v = 150 + ((x * 5 + y * 9) % 15);
                        Rgba([ch(v - 10), ch(v - 60), ch(v - 95), 255])
                    }
                }
                "planks" => {
                    let v = 160 + ((x * 3 + y * 7) % 12);
                    let seam = y % 4 == 3 || (y % 8 < 4 && x == 8) || (y % 8 >= 4 && x == 3);
                    if seam {
                        Rgba([120, 90, 55, 255])
                    } else {
                        Rgba([ch(v - 20), ch(v - 70), ch(v - 100), 255])
                    }
                }
                "glass" => {
                    let frame = x == 0 || x == 15 || y == 0 || y == 15;
                    if frame {
                        Rgba([200, 220, 225, 255])
                    } else {
                        Rgba([220, 240, 245, 60])
                    }
                }
                "birch_log" => {
                    let v = 200 + ((x * 3 + y * 5) % 10);
                    let fleck = (x * 7 + y * 3) % 11 == 0;
                    if fleck { Rgba([70, 70, 65, 255]) } else { Rgba([ch(v), ch(v - 8), ch(v - 30), 255]) }
                }
                "spruce_log" => {
                    let v = 70 + ((x * 9 + y * 5) % 16);
                    Rgba([ch(v), ch(v - 20), ch(v - 35), 255])
                }
                "dark_log" => {
                    let v = 55 + ((x * 5 + y * 11) % 14);
                    Rgba([ch(v), ch(v - 15), ch(v - 25), 255])
                }
                "cherry_log" => {
                    let v = 150 + ((x * 3 + y * 7) % 12);
                    Rgba([ch(v - 30), ch(v - 60), ch(v - 75), 255])
                }
                "birch_leaves" => {
                    let v = 50 + ((x * 13 + y * 7) % 35);
                    Rgba([ch(v + 60), ch(v + 110), ch(v + 30), 255])
                }
                "spruce_leaves" => {
                    let v = 35 + ((x * 11 + y * 5) % 30);
                    Rgba([ch(v / 3), ch(v + 40), ch(v / 3), 255])
                }
                "dark_leaves" => {
                    let v = 30 + ((x * 7 + y * 13) % 25);
                    Rgba([ch(v / 4), ch(v / 2), ch(v / 5), 255])
                }
                "cherry_leaves" => {
                    let v = 180 + ((x * 5 + y * 11) % 40);
                    Rgba([ch(v), ch(v - 40), ch(v - 70), 255])
                }
                "pale_leaves" => {
                    let v = 120 + ((x * 7 + y * 5) % 35);
                    Rgba([ch(v), ch(v - 5), ch(v - 15), 255])
                }
                "red_sand" => {
                    let v = 180 + ((x * 3 + y * 5) % 12);
                    Rgba([ch(v), ch(v - 60), ch(v - 90), 255])
                }
                "terracotta" => {
                    let band = (y / 4) % 3;
                    let base = match band { 0 => 190, 1 => 160, _ => 175 };
                    let v = base + ((x * 5 + y * 3) % 8);
                    Rgba([ch(v), ch(v - 70), ch(v - 110), 255])
                }
                "moss" => {
                    let v = 60 + ((x * 7 + y * 13) % 40);
                    Rgba([ch(v / 3), ch(v + 20), ch(v / 4), 255])
                }
                "ice" => {
                    let v = 190 + ((x * 3 + y * 7) % 20);
                    Rgba([ch(v - 40), ch(v - 20), ch(v + 30), 200])
                }
                "coal_generator" => {
                    let v = 90 + ((x * 7 + y * 5) % 18);
                    let vent = (4..8).contains(&x) && (4..8).contains(&y);
                    if vent { Rgba([60, 60, 65, 255]) } else { Rgba([ch(v + 40), ch(v), ch(v), 255]) }
                }
                "electric_furnace" => {
                    let v = 110 + ((x * 5 + y * 9) % 15);
                    let coil = (5..=10).contains(&x) && (y == 5 || y == 10);
                    if coil { Rgba([220, 140, 60, 255]) } else { Rgba([ch(v), ch(v), ch(v + 20), 255]) }
                }
                "crusher" => {
                    let v = 100 + ((x * 11 + y * 3) % 16);
                    let jaws = x == 7 || x == 8;
                    if jaws { Rgba([200, 200, 210, 255]) } else { Rgba([ch(v - 20), ch(v - 10), ch(v), 255]) }
                }
                "assembler" => {
                    let v = 120 + ((x * 3 + y * 7) % 14);
                    let arm = (6..=9).contains(&x) && (6..=9).contains(&y);
                    if arm { Rgba([240, 190, 70, 255]) } else { Rgba([ch(v - 30), ch(v - 10), ch(v), 255]) }
                }
                "research_bench" => {
                    let v = 130 + ((x * 5 + y * 3) % 12);
                    let grid = (3..=12).contains(&x) && (3..=12).contains(&y) && (x + y) % 4 == 0;
                    if grid { Rgba([90, 200, 190, 255]) } else { Rgba([ch(v - 40), ch(v - 25), ch(v - 10), 255]) }
                }
                "smithing_table" => {
                    let v = 120 + ((x * 5 + y * 11) % 15);
                    let anvil = (5..11).contains(&x) && (5..11).contains(&y);
                    if anvil {
                        Rgba([ch(v + 60), ch(v + 55), ch(v + 50), 255])
                    } else {
                        Rgba([ch(v - 10), ch(v - 50), ch(v - 75), 255])
                    }
                }
                "copper_ore" => {
                    let speck = (x * 5 + y * 13) % 31 < 7;
                    if speck {
                        Rgba([198, 110, 62, 255])
                    } else {
                        let v = 120 + ((x * 7 + y * 13) % 20);
                        Rgba([ch(v), ch(v), ch(v), 255])
                    }
                }
                "tin_ore" => {
                    let speck = (x * 9 + y * 7) % 29 < 6;
                    if speck {
                        Rgba([205, 205, 215, 255])
                    } else {
                        let v = 120 + ((x * 7 + y * 13) % 20);
                        Rgba([ch(v), ch(v), ch(v), 255])
                    }
                }
                "bauxite_ore" => {
                    let speck = (x * 11 + y * 5) % 31 < 7;
                    if speck {
                        Rgba([190, 130, 100, 255])
                    } else {
                        let v = 120 + ((x * 7 + y * 13) % 20);
                        Rgba([ch(v), ch(v), ch(v), 255])
                    }
                }
                "sulfur_ore" => {
                    let speck = (x * 7 + y * 9) % 29 < 7;
                    if speck {
                        Rgba([220, 210, 90, 255])
                    } else {
                        let v = 120 + ((x * 7 + y * 13) % 20);
                        Rgba([ch(v), ch(v), ch(v), 255])
                    }
                }
                "mod" => {
                    let v = 100 + ((x * 5 + y * 11) % 40);
                    let band = (x + y) % 8 < 2;
                    if band {
                        Rgba([ch(v + 60), ch(v), ch(v + 90), 255])
                    } else {
                        Rgba([ch(v), ch(v - 30), ch(v + 40), 255])
                    }
                }
                "water" => {
                    let v = 40 + ((x * 3 + y * 5) % 14);
                    Rgba([30, ch(60 + v / 2), ch(150 + v / 3), 170])
                }
                _ => {
                    let v = ((x + y) * 8) % 256;
                    Rgba([ch(v), ch(255 - v), 128, 255])
                }
            };
            img.put_pixel(x, y, color);
        }
    }
    img
}

/// The full atlas, one 16x16 layer per entry in TEXTURE_NAMES.
pub fn generate_atlas() -> Vec<RgbaImage> {
    TEXTURE_NAMES.iter().map(|n| generate_block_texture(n)).collect()
}

// ------------------------------------------------------------------
// Item sprites: 16x16 pixel art for non-block items.
// '.' = transparent; every other char maps through a per-item palette.

/// Every item id `generate_item_texture` knows how to draw (block items reuse
/// their atlas texture instead, so they are not listed here).
pub const ITEM_TEXTURE_IDS: &[&str] = &[
    "stick", "coal", "raw_iron", "iron_ingot",
    "wooden_pickaxe", "stone_pickaxe", "iron_pickaxe",
    "wooden_axe", "stone_axe", "iron_axe",
    "wooden_shovel", "stone_shovel", "iron_shovel",
    "wooden_sword", "stone_sword", "iron_sword",
    "apple", "porkchop", "mutton", "book", "bow", "arrow",
    "bronze_chestplate", "steel_chestplate",
    "raw_copper", "copper_ingot", "raw_tin", "tin_ingot",
    "aluminum_ingot", "sulfur", "bronze_ingot", "steel_ingot",
    "copper_wire", "iron_gear", "machine_frame", "basic_circuit",
    "glitch_dust", "null_shard",
];

fn paint_sprite(art: [&str; 16], colors: impl Fn(char) -> Rgba<u8>) -> RgbaImage {
    let mut img = RgbaImage::new(16, 16);
    for (y, row) in art.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch != '.' {
                img.put_pixel(x as u32, y as u32, colors(ch));
            }
        }
    }
    img
}

/// (base, highlight) colors for a tool head by tier (0 wood, 1 stone, 2 iron).
fn tool_head_palette(tier: u8) -> (Rgba<u8>, Rgba<u8>) {
    match tier {
        0 => (Rgba([168, 133, 80, 255]), Rgba([198, 162, 104, 255])),
        1 => (Rgba([138, 138, 142, 255]), Rgba([176, 176, 182, 255])),
        _ => (Rgba([204, 204, 214, 255]), Rgba([236, 236, 244, 255])),
    }
}

const HANDLE: Rgba<u8> = Rgba([146, 109, 62, 255]);
const HANDLE_DARK: Rgba<u8> = Rgba([104, 76, 43, 255]);

/// Ingot palette by material: (base, top-light, bottom-dark).
fn ingot_palette(name: &str) -> (Rgba<u8>, Rgba<u8>, Rgba<u8>) {
    match name {
        "iron_ingot" => (Rgba([200, 200, 208, 255]), Rgba([236, 236, 242, 255]), Rgba([150, 150, 160, 255])),
        "copper_ingot" => (Rgba([198, 110, 62, 255]), Rgba([232, 150, 96, 255]), Rgba([150, 78, 42, 255])),
        "tin_ingot" => (Rgba([190, 198, 206, 255]), Rgba([228, 234, 240, 255]), Rgba([140, 150, 162, 255])),
        "aluminum_ingot" => (Rgba([210, 218, 226, 255]), Rgba([240, 246, 250, 255]), Rgba([158, 168, 180, 255])),
        "bronze_ingot" => (Rgba([205, 127, 50, 255]), Rgba([235, 165, 90, 255]), Rgba([160, 92, 32, 255])),
        "steel_ingot" => (Rgba([170, 180, 195, 255]), Rgba([205, 215, 230, 255]), Rgba([120, 130, 148, 255])),
        _ => (Rgba([180, 180, 180, 255]), Rgba([220, 220, 220, 255]), Rgba([130, 130, 130, 255])),
    }
}

const PICKAXE_ART: [&str; 16] = [
    "................",
    "...mMMMMMMMMm...",
    "..mmm..hH..mmm..",
    "..mm...hH...mm..",
    ".mm....hH....mm.",
    ".m.....hH.....m.",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    "......hHHHh.....",
    "................",
];

const AXE_ART: [&str; 16] = [
    "................",
    "........hH......",
    "...mmmm.hH......",
    "..mmMMm.hH......",
    "..mmMM..hH......",
    "..mmm...hH......",
    "........hH......",
    "........hH......",
    "........hH......",
    "........hH......",
    "........hH......",
    "........hH......",
    "........hH......",
    "........hH......",
    ".......hHHHh....",
    "................",
];

const SHOVEL_ART: [&str; 16] = [
    "................",
    "......mmmm......",
    ".....mmMMmm.....",
    ".....mmMMmm.....",
    ".....mmmmmm.....",
    "......mmmm......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    "......hHHHh.....",
    "................",
];

const SWORD_ART: [&str; 16] = [
    "................",
    ".......mm.......",
    ".......Mm.......",
    ".......Mm.......",
    ".......Mm.......",
    ".......Mm.......",
    ".......Mm.......",
    ".......Mm.......",
    "....gggMmggg....",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    "......ppppp.....",
    "................",
];

const BOW_ART: [&str; 16] = [
    "................",
    ".........bb.....",
    ".......bb..s....",
    "......b....s....",
    ".....b.....s....",
    ".....b.....s....",
    "....b......s....",
    "....b......s....",
    "....b......s....",
    "....b......s....",
    ".....b.....s....",
    ".....b.....s....",
    "......b....s....",
    ".......bb..s....",
    ".........bb.....",
    "................",
];

const ARROW_ART: [&str; 16] = [
    "................",
    ".......mm.......",
    "......mmm.......",
    ".......mm.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    ".......hH.......",
    "..f....hH....f..",
    "..ff...hH...ff..",
    "...ff..hH..ff...",
    ".....ffhHff.....",
    "................",
];

const STICK_ART: [&str; 16] = [
    "................",
    "................",
    "..........hH....",
    ".........hH.....",
    ".........hH.....",
    "........hH......",
    "........hH......",
    ".......hH.......",
    ".......hH.......",
    "......hH........",
    "......hH........",
    ".....hH.........",
    ".....hH.........",
    "................",
    "................",
    "................",
];

const INGOT_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "................",
    "....IIIIIIII....",
    "...HIIIIIIIIH...",
    "...IiiiiiiiiI...",
    "...iiiiiiiiii...",
    "...iiiiiiiiii...",
    "...iiiiiiiiii...",
    "....dddddddd....",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const RAW_CHUNK_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "................",
    ".....oooo.......",
    "...ooOooooo.....",
    "..oooooooOoo....",
    "..oOooooooo.....",
    "..ooooooOoo.....",
    "...oooOoooo.....",
    "....oooooo......",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const APPLE_ART: [&str; 16] = [
    "................",
    "................",
    "........s.......",
    "....ll..s.......",
    "...rrrrrrrr.....",
    "..rrRRrrrrrr....",
    "..rRRrrrrrrr....",
    "..rrrrrrrrrr....",
    "..rrrrrrrrrr....",
    "..rrrrrrrrrr....",
    "...rrrrrrrr.....",
    "....rr.rr.......",
    "................",
    "................",
    "................",
    "................",
];

const MEAT_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "......pppp......",
    "....ppPPpppp....",
    "...ppPPpppppp...",
    "...pppppppppp...",
    "...ppppppppp....",
    "....ppppppb.....",
    ".....pppp.bb....",
    "..........bb....",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const BOOK_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "..cccccccccc....",
    "..cCCCCCCCCc....",
    "..cCCCCCCCCc....",
    "..cCCCCCCCCc....",
    "..cCCCCCCCCc....",
    "..cCCCCCCCCc....",
    "..cccccccccc....",
    "...pppppppp.....",
    "...pppppppp.....",
    "................",
    "................",
    "................",
    "................",
];

const CHESTPLATE_ART: [&str; 16] = [
    "................",
    "................",
    "..MM........MM..",
    ".mMMm......mMMm.",
    ".mmmmmmmmmmmmmm.",
    ".mmm..mmmm..mmm.",
    ".mmmm.mmmm.mmmm.",
    ".mmm..mmmm..mmm.",
    ".mmm..mmmm..mmm.",
    ".mmm........mmm.",
    ".mm..........mm.",
    ".mm..........mm.",
    "................",
    "................",
    "................",
    "................",
];

const WIRE_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "....cccccccc....",
    "...cccCCCCccc...",
    "...cC......Cc...",
    "...cC..cc..Cc...",
    "...cC..cc..Ccc..",
    "...cC......Cc...",
    "...cccCCCCccc...",
    "....cccccccc....",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const GEAR_ART: [&str; 16] = [
    "................",
    "................",
    ".......gg.......",
    "....g..gg..g....",
    "...gg.gggg.gg...",
    "...gggGGGGggg...",
    "..ggGGg..gGGgg..",
    "..ggGG....GGgg..",
    "..ggGG....GGgg..",
    "..ggGGg..gGGgg..",
    "...gggGGGGggg...",
    "...gg.gggg.gg...",
    "....g..gg..g....",
    ".......gg.......",
    "................",
    "................",
];

const FRAME_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "..mmmmmmmmmmmm..",
    "..mM........Mm..",
    "..mM.mmmmmm.Mm..",
    "..mM.m....m.Mm..",
    "..mM.m....m.Mm..",
    "..mM.m....m.Mm..",
    "..mM.m....m.Mm..",
    "..mM.mmmmmm.Mm..",
    "..mM........Mm..",
    "..mmmmmmmmmmmm..",
    "................",
    "................",
    "................",
];

const CIRCUIT_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "..BBBBBBBBBBBB..",
    "..Btt.BBBB.ttB..",
    "..Bt.BccccB.tB..",
    "..BB.BccccB.BB..",
    "..Bt.BccccB.tB..",
    "..BB.BccccB.BB..",
    "..Bt.BccccB.tB..",
    "..BB.BBBBBB.BB..",
    "..BBBBBBBBBBBB..",
    "................",
    "................",
    "................",
    "................",
];

const SULFUR_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "......y...y.....",
    ".....yYy.yYy....",
    ".....yYy.yYy....",
    "....yYYYyYYYy...",
    "....yYYYyYYYy...",
    "...yYYYYYYYYYy..",
    "....yyyyyyyyy...",
    ".....yyyyyyy....",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const GLITCH_DUST_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "....c...m...c...",
    "...m..cmc..m....",
    "....cmmmmc.c....",
    "...cmmmmmmmmc...",
    "..cmmmmmmmmmmc..",
    "...mmmmmmmmmm...",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const NULL_SHARD_ART: [&str; 16] = [
    "................",
    "................",
    ".......n........",
    "......nNn.......",
    "......nNn.......",
    ".....nNNnn......",
    ".....nNNnn......",
    "....nNNNnnn.....",
    "....nNNNnnn.....",
    "...nNNNNnnnn....",
    "...nNNNNnnnn....",
    "..nnNNNnnnnn....",
    "..nnnnnnnnnn....",
    "................",
    "................",
    "................",
];

const COAL_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "................",
    ".....cccc.......",
    "...cccccccc.....",
    "..cccCcccccc....",
    "..ccccccCccc....",
    "..cCcccccccc....",
    "...cccccccc.....",
    "....cccccc......",
    "................",
    "................",
    "................",
    "................",
    "................",
];

/// Generates 16x16 pixel art for a non-block item id (see ITEM_TEXTURE_IDS).
/// Block items are not handled here — they reuse their atlas texture.
pub fn generate_item_texture(item_id: &str) -> Option<RgbaImage> {
    // tools: shared art per kind, head palette per tier
    let tool = match item_id {
        "wooden_pickaxe" => Some(("pickaxe", 0u8)),
        "stone_pickaxe" => Some(("pickaxe", 1)),
        "iron_pickaxe" => Some(("pickaxe", 2)),
        "wooden_axe" => Some(("axe", 0)),
        "stone_axe" => Some(("axe", 1)),
        "iron_axe" => Some(("axe", 2)),
        "wooden_shovel" => Some(("shovel", 0)),
        "stone_shovel" => Some(("shovel", 1)),
        "iron_shovel" => Some(("shovel", 2)),
        "wooden_sword" => Some(("sword", 0)),
        "stone_sword" => Some(("sword", 1)),
        "iron_sword" => Some(("sword", 2)),
        _ => None,
    };
    if let Some((kind, tier)) = tool {
        let (m, mm) = tool_head_palette(tier);
        let art = match kind {
            "pickaxe" => PICKAXE_ART,
            "axe" => AXE_ART,
            "shovel" => SHOVEL_ART,
            _ => SWORD_ART,
        };
        // swords: guard/pommel stay fixed, blade takes the tier palette
        if kind == "sword" {
            return Some(paint_sprite(art, |c| match c {
                'm' => m, 'M' => mm,
                'g' => Rgba([120, 96, 52, 255]),
                'p' => Rgba([190, 160, 90, 255]),
                'h' => HANDLE, 'H' => HANDLE_DARK,
                _ => Rgba([0, 0, 0, 0]),
            }));
        }
        return Some(paint_sprite(art, |c| match c {
            'm' => m, 'M' => mm,
            'h' => HANDLE, 'H' => HANDLE_DARK,
            _ => Rgba([0, 0, 0, 0]),
        }));
    }
    let img = match item_id {
        "stick" => paint_sprite(STICK_ART, |c| match c {
            'h' => HANDLE, 'H' => HANDLE_DARK, _ => Rgba([0, 0, 0, 0]),
        }),
        "coal" => paint_sprite(COAL_ART, |c| match c {
            'c' => Rgba([38, 38, 42, 255]), 'C' => Rgba([90, 90, 96, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "bow" => paint_sprite(BOW_ART, |c| match c {
            'b' => HANDLE, 's' => Rgba([225, 222, 210, 255]), _ => Rgba([0, 0, 0, 0]),
        }),
        "arrow" => paint_sprite(ARROW_ART, |c| match c {
            'm' => Rgba([200, 200, 210, 255]), 'f' => Rgba([225, 222, 210, 255]),
            'h' => HANDLE, 'H' => HANDLE_DARK, _ => Rgba([0, 0, 0, 0]),
        }),
        "apple" => paint_sprite(APPLE_ART, |c| match c {
            'r' => Rgba([200, 40, 40, 255]), 'R' => Rgba([240, 90, 80, 255]),
            's' => Rgba([100, 72, 40, 255]), 'l' => Rgba([80, 150, 60, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "porkchop" => paint_sprite(MEAT_ART, |c| match c {
            'p' => Rgba([235, 150, 150, 255]), 'P' => Rgba([250, 190, 185, 255]),
            'b' => Rgba([235, 225, 210, 255]), _ => Rgba([0, 0, 0, 0]),
        }),
        "mutton" => paint_sprite(MEAT_ART, |c| match c {
            'p' => Rgba([190, 90, 80, 255]), 'P' => Rgba([222, 130, 110, 255]),
            'b' => Rgba([235, 225, 210, 255]), _ => Rgba([0, 0, 0, 0]),
        }),
        "book" => paint_sprite(BOOK_ART, |c| match c {
            'c' => Rgba([120, 80, 45, 255]), 'C' => Rgba([152, 106, 62, 255]),
            'p' => Rgba([230, 225, 210, 255]), _ => Rgba([0, 0, 0, 0]),
        }),
        "bronze_chestplate" | "steel_chestplate" => {
            let base = if item_id.starts_with("bronze") {
                (Rgba([205, 127, 50, 255]), Rgba([235, 165, 90, 255]))
            } else {
                (Rgba([172, 182, 198, 255]), Rgba([208, 218, 234, 255]))
            };
            paint_sprite(CHESTPLATE_ART, |c| match c {
                'm' => base.0, 'M' => base.1, _ => Rgba([0, 0, 0, 0]),
            })
        }
        "raw_iron" => raw_chunk(Rgba([172, 146, 126, 255]), Rgba([210, 185, 160, 255])),
        "raw_copper" => raw_chunk(Rgba([160, 98, 74, 255]), Rgba([200, 140, 100, 255])),
        "raw_tin" => raw_chunk(Rgba([150, 148, 145, 255]), Rgba([192, 192, 196, 255])),
        "sulfur" => paint_sprite(SULFUR_ART, |c| match c {
            'y' => Rgba([210, 195, 70, 255]), 'Y' => Rgba([240, 230, 120, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "glitch_dust" => paint_sprite(GLITCH_DUST_ART, |c| match c {
            'm' => Rgba([220, 80, 200, 255]), 'c' => Rgba([80, 220, 230, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "null_shard" => paint_sprite(NULL_SHARD_ART, |c| match c {
            'n' => Rgba([60, 40, 90, 255]), 'N' => Rgba([130, 90, 190, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "copper_wire" => paint_sprite(WIRE_ART, |c| match c {
            'c' => Rgba([198, 110, 62, 255]), 'C' => Rgba([232, 150, 96, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "iron_gear" => paint_sprite(GEAR_ART, |c| match c {
            'g' => Rgba([176, 178, 186, 255]), 'G' => Rgba([214, 216, 224, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "machine_frame" => paint_sprite(FRAME_ART, |c| match c {
            'm' => Rgba([120, 122, 130, 255]), 'M' => Rgba([160, 162, 172, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "basic_circuit" => paint_sprite(CIRCUIT_ART, |c| match c {
            'B' => Rgba([30, 90, 50, 255]), 't' => Rgba([220, 180, 80, 255]),
            'c' => Rgba([50, 52, 58, 255]), _ => Rgba([0, 0, 0, 0]),
        }),
        _ => {
            // ingots share art with per-material palettes
            if item_id.ends_with("_ingot") {
                let (i, ii, d) = ingot_palette(item_id);
                paint_sprite(INGOT_ART, |c| match c {
                    'i' => i, 'I' => ii, 'H' => d, 'd' => d, _ => Rgba([0, 0, 0, 0]),
                })
            } else {
                return None;
            }
        }
    };
    Some(img)
}

fn raw_chunk(base: Rgba<u8>, highlight: Rgba<u8>) -> RgbaImage {
    paint_sprite(RAW_CHUNK_ART, |c| match c {
        'o' => base, 'O' => highlight, _ => Rgba([0, 0, 0, 0]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_generation() {
        for name in TEXTURE_NAMES {
            let tex = generate_block_texture(name);
            assert_eq!(tex.width(), 16);
            assert_eq!(tex.height(), 16);
            // fully opaque except water (transparent pass)
            if name == "glass" {
                // frame opaque, pane translucent
                continue;
            }
            if name == "ice" {
                // translucent pane everywhere
                assert!(tex.pixels().all(|p| p.0[3] == 200));
                continue;
            }
            if name != "torch_item" {
                let expected_alpha = if name == "water" { 170 } else { 255 };
                assert!(tex.pixels().all(|p| p.0[3] == expected_alpha));
            }
        }
    }

    #[test]
    fn test_atlas_covers_all_blocks() {
        let atlas = generate_atlas();
        assert_eq!(atlas.len(), TEXTURE_NAMES.len());
        // every known block id maps to a valid layer (all vanilla ids 1..=41 + mods)
        for id in 1u32..=41 {
            assert!(texture_index_for_block(id) < atlas.len() as u32, "block {} unmapped", id);
        }
        assert!(texture_index_for_block(100) < atlas.len() as u32, "mod blocks unmapped");
        // spot-check the wiring so wood variants and machines stop falling back to stone
        let name = |id: u32| TEXTURE_NAMES[texture_index_for_block(id) as usize];
        assert_eq!(name(19), "birch_log");
        assert_eq!(name(31), "ice");
        assert_eq!(name(32), "copper_ore");
        assert_eq!(name(37), "coal_generator");
        assert_eq!(name(41), "research_bench");
        assert_eq!(name(100), "mod");
    }

    #[test]
    fn item_textures_generate_nonempty() {
        for id in ITEM_TEXTURE_IDS {
            let tex = generate_item_texture(id).unwrap_or_else(|| panic!("no art for {}", id));
            assert_eq!(tex.width(), 16);
            assert_eq!(tex.height(), 16);
            assert!(tex.pixels().any(|p| p.0[3] > 0), "{} fully transparent", id);
        }
        assert!(generate_item_texture("stone").is_none(), "block items use atlas art");
        assert!(generate_item_texture("modded:thing").is_none());
    }

    #[test]
    fn item_textures_differ_by_material() {
        let iron = generate_item_texture("iron_ingot").unwrap();
        let copper = generate_item_texture("copper_ingot").unwrap();
        assert_ne!(iron.as_raw(), copper.as_raw());
        let wood = generate_item_texture("wooden_pickaxe").unwrap();
        let stone = generate_item_texture("stone_pickaxe").unwrap();
        assert_ne!(wood.as_raw(), stone.as_raw());
    }

    #[test]
    fn textures_differ_from_each_other() {
        let atlas = generate_atlas();
        for i in 0..atlas.len() {
            for j in (i + 1)..atlas.len() {
                assert_ne!(atlas[i].as_raw(), atlas[j].as_raw(),
                    "textures {} and {} are identical", TEXTURE_NAMES[i], TEXTURE_NAMES[j]);
            }
        }
    }
}
