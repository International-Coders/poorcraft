use image::{Rgba, RgbaImage};

/// Generates a procedural 16x16 pixel-art texture for blocks (e.g. stone, grass, dirt).
pub fn generate_block_texture(name: &str) -> RgbaImage {
    let mut img = RgbaImage::new(16, 16);
    for x in 0..16 {
        for y in 0..16 {
            let color = match name {
                "stone" => {
                    let v = 120 + ((x * 7 + y * 13) % 20) as u8;
                    Rgba([v, v, v, 255])
                }
                "grass" => {
                    if y < 4 {
                        Rgba([80, 160, 60, 255])
                    } else {
                        let v = 110 + ((x * 5 + y * 9) % 15) as u8;
                        Rgba([v, 90, 50, 255])
                    }
                }
                "dirt" => {
                    let v = 100 + ((x * 11 + y * 7) % 25) as u8;
                    Rgba([v, 80, 40, 255])
                }
                _ => {
                    let v = ((x + y) * 8) as u8;
                    Rgba([v, 255 - v, 128, 255])
                }
            };
            img.put_pixel(x, y, color);
        }
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_generation() {
        let tex = generate_block_texture("stone");
        assert_eq!(tex.width(), 16);
        assert_eq!(tex.height(), 16);
    }
}
