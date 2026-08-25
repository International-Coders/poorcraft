use image::{Rgba, RgbaImage};

/// Canonical texture atlas layer order. Block ids map onto these indices.
pub const TEXTURE_NAMES: [&str; 6] = ["stone", "grass", "dirt", "sand", "mycelium", "snow"];

/// Texture atlas layer for a block id (see lf_voxel::BlockState / lf_worldgen::BlockId).
pub fn texture_index_for_block(block_id: u32) -> u32 {
    match block_id {
        1 => 0, // stone
        2 => 1, // grass
        3 => 2, // dirt
        4 => 3, // sand
        5 => 4, // mycelium
        6 => 5, // snow
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_generation() {
        for name in TEXTURE_NAMES {
            let tex = generate_block_texture(name);
            assert_eq!(tex.width(), 16);
            assert_eq!(tex.height(), 16);
            // fully opaque
            assert!(tex.pixels().all(|p| p.0[3] == 255));
        }
    }

    #[test]
    fn test_atlas_covers_all_blocks() {
        let atlas = generate_atlas();
        assert_eq!(atlas.len(), TEXTURE_NAMES.len());
        // every known block id maps to a valid layer
        for id in 1..=6u32 {
            assert!(texture_index_for_block(id) < atlas.len() as u32);
        }
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
