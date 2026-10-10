//! Real 16x16 item icons as egui textures.
//!
//! Block items reuse their atlas art (`lf_assets::generate_block_texture`);
//! every other item gets sprite art from `lf_assets::generate_item_texture`.
//! Mod items (registered at runtime, ids >= MOD_BLOCK_BASE or `mod:` style)
//! get a deterministic gem icon tinted by an id hash so nothing renders as a
//! bare gray square. The flat-color fallback stays for truly unknown ids.

use std::collections::HashMap;

use lf_game::items::{item_def, registered_mod_items, ItemKind};

/// One icon per item id, built once per egui context (NEAREST keeps the
/// pixel art crisp at any slot size).
pub struct ItemIcons {
    map: HashMap<String, egui::TextureHandle>,
}

fn load(ctx: &egui::Context, id: &str, img: image::RgbaImage) -> egui::TextureHandle {
    let size = [img.width() as usize, img.height() as usize];
    let pixels = img
        .pixels()
        .map(|p| egui::Color32::from_rgba_unmultiplied(p.0[0], p.0[1], p.0[2], p.0[3]))
        .collect();
    ctx.load_texture(format!("item:{}", id), egui::ColorImage { size, pixels }, egui::TextureOptions::NEAREST)
}

/// Deterministic tint for a mod item id (same id -> same color, forever).
fn hash_tint(id: &str) -> egui::Color32 {
    let mut h: u32 = 2166136261;
    for b in id.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    let r = 90 + (h % 140) as u8;
    let g = 90 + ((h >> 7) % 140) as u8;
    let b = 90 + ((h >> 14) % 140) as u8;
    egui::Color32::from_rgb(r, g, b)
}

/// 8x8 gem sprite tinted per mod item (drawn as pixels, no art table needed).
fn gem_image(tint: egui::Color32) -> egui::ColorImage {
    const ART: [&str; 8] = [
        "...dd...",
        "..dDDd..",
        ".dDLDLd.",
        "dDLLDDDd",
        "dDDDDLDd",
        ".dDDDDd.",
        "..dDDd..",
        "...dd...",
    ];
    let dark = egui::Color32::from_rgb(
        (tint.r() as u16 * 2 / 3) as u8,
        (tint.g() as u16 * 2 / 3) as u8,
        (tint.b() as u16 * 2 / 3) as u8,
    );
    let light = egui::Color32::from_rgb(
        (tint.r() as u16 + 255) as u8 / 2,
        (tint.g() as u16 + 255) as u8 / 2,
        (tint.b() as u16 + 255) as u8 / 2,
    );
    let mut pixels = Vec::with_capacity(64);
    for row in ART {
        for ch in row.chars() {
            pixels.push(match ch {
                'd' => dark,
                'D' => tint,
                'L' => light,
                _ => egui::Color32::TRANSPARENT,
            });
        }
    }
    egui::ColorImage { size: [8, 8], pixels }
}

impl ItemIcons {
    pub fn new(ctx: &egui::Context) -> Self {
        let mut map = HashMap::new();
        for def in lf_game::items::items() {
            let img = match def.kind {
                ItemKind::Block(b) => {
                    let layer = lf_assets::texture_index_for_block(b) as usize;
                    lf_assets::generate_block_texture(lf_assets::TEXTURE_NAMES[layer])
                }
                _ => match lf_assets::generate_item_texture(def.id) {
                    Some(img) => img,
                    None => continue,
                },
            };
            map.insert(def.id.to_string(), load(ctx, def.id, img));
        }
        for def in registered_mod_items() {
            if map.contains_key(def.id) {
                continue;
            }
            let image = gem_image(hash_tint(def.id));
            let tex = ctx.load_texture(format!("item:{}", def.id), image, egui::TextureOptions::NEAREST);
            map.insert(def.id.to_string(), tex);
        }
        Self { map }
    }

    pub fn get(&self, item_id: &str) -> Option<&egui::TextureHandle> {
        self.map.get(item_id)
    }

    /// Draw the icon into a rect (uv 0..1), tinted white; caller draws the
    /// flat-color fallback when this returns false.
    pub fn paint(&self, ui: &mut egui::Ui, rect: egui::Rect, item_id: &str) -> bool {
        let Some(tex) = self.get(item_id) else { return false };
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        ui.painter().image(tex.id(), rect, uv, egui::Color32::WHITE);
        true
    }
}

/// Flat fallback color for ids without art (matches the pre-icon look).
pub fn fallback_color(item_id: &str) -> egui::Color32 {
    match item_def(item_id).map(|d| d.kind) {
        Some(ItemKind::Food(_)) => egui::Color32::from_rgb(200, 60, 60),
        Some(ItemKind::Tool(_, _)) => egui::Color32::from_rgb(150, 130, 90),
        _ => egui::Color32::from_gray(160),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_vanilla_item_has_art() {
        let ctx = egui::Context::default();
        let icons = ItemIcons::new(&ctx);
        for def in lf_game::items::items() {
            assert!(icons.get(def.id).is_some(), "no icon for item {}", def.id);
        }
    }

    #[test]
    fn icons_are_16x16_pixel_art() {
        let ctx = egui::Context::default();
        let icons = ItemIcons::new(&ctx);
        for id in ["iron_pickaxe", "apple", "stone", "coal_generator"] {
            let tex = icons.get(id).unwrap();
            assert_eq!(tex.size_vec2(), egui::vec2(16.0, 16.0), "{} wrong size", id);
        }
    }

    /// Loop 329 asset completeness: every registered non-block item must
    /// produce real pixel art — the flat-color fallback is only allowed for
    /// unknown/mod ids, never for anything the registry ships.
    #[test]
    fn every_registered_item_has_art() {
        for def in lf_game::items::items() {
            if matches!(def.kind, ItemKind::Block(_)) {
                // block items reuse their atlas layer
                let layer = lf_assets::texture_index_for_block(match def.kind {
                    ItemKind::Block(b) => b,
                    _ => unreachable!(),
                });
                assert!(layer < lf_assets::TEXTURE_NAMES.len() as u32,
                    "block item {} maps past the atlas", def.id);
                continue;
            }
            let img = lf_assets::generate_item_texture(def.id)
                .unwrap_or_else(|| panic!("no icon art for registered item {}", def.id));
            assert_eq!((img.width(), img.height()), (16, 16), "{} wrong size", def.id);
            assert!(img.pixels().any(|p| p.0[3] > 0), "{} icon fully transparent", def.id);
        }
    }

    /// Loop 329: no two registered items may share one icon — a hand
    /// update that reuses a palette is exactly the bug this catches.
    #[test]
    fn item_icons_are_pairwise_distinct() {
        let mut icons: Vec<(String, image::RgbaImage)> = Vec::new();
        for def in lf_game::items::items() {
            match def.kind {
                ItemKind::Block(b) => {
                    let layer = lf_assets::texture_index_for_block(b) as usize;
                    icons.push((def.id.to_string(),
                        lf_assets::generate_block_texture(lf_assets::TEXTURE_NAMES[layer])));
                }
                _ => {
                    if let Some(img) = lf_assets::generate_item_texture(def.id) {
                        icons.push((def.id.to_string(), img));
                    }
                }
            }
        }
        for i in 0..icons.len() {
            for j in (i + 1)..icons.len() {
                assert_ne!(icons[i].1.as_raw(), icons[j].1.as_raw(),
                    "items '{}' and '{}' share one icon", icons[i].0, icons[j].0);
            }
        }
    }

    #[test]
    fn mod_items_get_stable_gem_icons() {
        let ctx = egui::Context::default();
        assert!(lf_game::items::register_mod_item(
            "icons_test:shard".into(), "Test Shard".into(), ItemKind::Material, 64));
        let icons = ItemIcons::new(&ctx);
        assert!(icons.get("icons_test:shard").is_some(), "mod item must get a gem icon");
        // unknown ids still fall back
        assert!(icons.get("never_registered:item").is_none());
        // tint is deterministic
        let a = hash_tint("icons_test:shard");
        let b = hash_tint("icons_test:shard");
        assert_eq!(a, b);
    }
}
