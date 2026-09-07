//! Minimal 5x7 bitmap font for the HUD debug line (no text stack needed).
//!
//! Rasterizes a single line of ASCII into an RGBA8 texture each frame; the
//! renderer uploads it and draws it as one alpha-blended quad. Kept tiny on
//! purpose — it is a debug readout, not a UI framework.

/// 5-bit rows, top to bottom, for each supported glyph.
const GLYPHS: &[(char, [u8; 7])] = &[
    ('0', [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110]),
    ('1', [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
    ('2', [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111]),
    ('3', [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110]),
    ('4', [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010]),
    ('5', [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110]),
    ('6', [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
    ('7', [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000]),
    ('8', [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110]),
    ('9', [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100]),
    ('A', [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
    ('B', [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110]),
    ('C', [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110]),
    ('D', [0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100]),
    ('E', [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111]),
    ('F', [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000]),
    ('G', [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111]),
    ('H', [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
    ('I', [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
    ('J', [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100]),
    ('K', [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001]),
    ('L', [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111]),
    ('M', [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001]),
    ('N', [0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001]),
    ('O', [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
    ('P', [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000]),
    ('Q', [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101]),
    ('R', [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001]),
    ('S', [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110]),
    ('T', [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100]),
    ('U', [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
    ('V', [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100]),
    ('W', [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001]),
    ('X', [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001]),
    ('Y', [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100]),
    ('Z', [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111]),
    (' ', [0; 7]),
    ('.', [0, 0, 0, 0, 0, 0b01100, 0b01100]),
    ('-', [0, 0, 0, 0b01110, 0, 0, 0]),
    (':', [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0]),
    ('/', [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000]),
];

pub const GLYPH_W: u32 = 5;
pub const GLYPH_H: u32 = 7;
/// 1 px gap between glyphs.
pub const ADVANCE: u32 = GLYPH_W + 1;

fn glyph(c: char) -> [u8; 7] {
    let upper = c.to_ascii_uppercase();
    GLYPHS
        .iter()
        .find(|(gc, _)| *gc == upper)
        .map(|(_, rows)| *rows)
        .unwrap_or([0; 7])
}

/// Rasterizes a line into a tightly packed RGBA8 buffer (white, alpha mask).
/// Returns (bytes, width, height); width covers the longest line.
pub fn rasterize_line(line: &str, scale: u32) -> (Vec<u8>, u32, u32) {
    let chars: Vec<char> = line.chars().collect();
    let w = chars.len() as u32 * ADVANCE.saturating_sub(1) * scale + scale;
    let h = GLYPH_H * scale;
    let mut out = vec![0u8; (w * h * 4) as usize];
    for (ci, ch) in chars.iter().enumerate() {
        let rows = glyph(*ch);
        for ry in 0..GLYPH_H {
            let bits = rows[ry as usize];
            for rx in 0..GLYPH_W {
                if bits & (1 << (GLYPH_W - 1 - rx)) != 0 {
                    for py in 0..scale {
                        for px in 0..scale {
                            let x = (ci as u32 * ADVANCE + rx) * scale + px;
                            let y = ry * scale + py;
                            if x < w && y < h {
                                let i = ((y * w + x) * 4) as usize;
                                out[i] = 255;
                                out[i + 1] = 255;
                                out[i + 2] = 255;
                                out[i + 3] = 255;
                            }
                        }
                    }
                }
            }
        }
    }
    (out, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_and_letters_rasterize_with_ink() {
        let (buf, w, h) = rasterize_line("P3D", 1);
        // 3 glyphs x 5 px + 1 px tail = 16 columns, 7 rows.
        assert_eq!((w, h), (16, 7));
        assert!(buf.chunks_exact(4).any(|p| p[0] == 255), "glyphs must have ink");
        assert!(buf.chunks_exact(4).any(|p| p[3] == 0), "gaps must stay empty");
    }

    #[test]
    fn unknown_chars_render_as_space_without_panic() {
        let (a, wa, _) = rasterize_line("A~B", 1);
        let (b, wb, _) = rasterize_line("A B", 1);
        assert_eq!(wa, wb);
        assert_eq!(a, b, "unknown glyph must fall back to space");
    }

    #[test]
    fn scale_enlarges_the_raster() {
        let (_, w1, h1) = rasterize_line("FPS", 1);
        let (_, w2, h2) = rasterize_line("FPS", 2);
        assert_eq!((w2, h2), (w1 * 2, h1 * 2));
    }
}
