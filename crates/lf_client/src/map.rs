//! World map + minimap + waypoints.
//!
//! Chunks are rendered once into 16x16 cached tiles: loaded chunks are
//! colored from their real top blocks (player edits included), explored-
//! but-unloaded chunks are approximated from the seed-known WorldGen and
//! dimmed, and unexplored area stays dark fog. Tiles are composited into an
//! egui texture (the proven live-RT upload path) for both the corner
//! minimap and the full M-key map screen.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use egui::{Color32, Pos2, Rect, Vec2};
use lf_voxel::registry::block;
use lf_voxel::world::World;
use lf_worldgen::{Biome, Seed, WorldGen};

use crate::ui_kit::Theme;
use crate::GameState;

/// Map color for a top block id (variant woods/leaves included).
pub fn block_map_color(id: u32) -> Color32 {
    let c = |r: u32, g: u32, b: u32| Color32::from_rgb(r as u8, g as u8, b as u8);
    match id {
        block::GRASS => c(96, 160, 62),
        block::DIRT => c(134, 96, 67),
        block::STONE => c(128, 128, 130),
        block::SAND => c(219, 207, 163),
        block::MYCELIUM => c(140, 130, 160),
        block::SNOW => c(240, 246, 246),
        block::LOG => c(102, 81, 50),
        block::LEAVES => c(58, 118, 40),
        block::WATER => c(52, 98, 178),
        block::BIRCH_LOG => c(196, 190, 172),
        block::SPRUCE_LOG => c(70, 52, 32),
        block::DARK_LOG => c(55, 42, 28),
        block::CHERRY_LOG => c(140, 85, 70),
        block::BIRCH_LEAVES => c(112, 162, 92),
        block::SPRUCE_LEAVES => c(45, 95, 60),
        block::DARK_LEAVES => c(35, 70, 40),
        block::CHERRY_LEAVES => c(220, 150, 165),
        block::PALE_LEAVES => c(160, 160, 150),
        block::RED_SAND => c(190, 110, 60),
        block::TERRACOTTA => c(172, 100, 70),
        block::MOSS => c(80, 120, 60),
        block::ICE => c(170, 210, 235),
        block::COAL_ORE => c(96, 96, 100),
        block::IRON_ORE => c(152, 140, 128),
        block::COPPER_ORE => c(162, 112, 82),
        block::TIN_ORE => c(152, 152, 155),
        block::BAUXITE_ORE => c(162, 122, 96),
        block::SULFUR_ORE => c(172, 166, 92),
        block::CRAFTING_TABLE | block::PLANKS => c(170, 130, 80),
        block::FURNACE => c(120, 120, 122),
        block::CHEST => c(172, 132, 76),
        block::GLASS => c(200, 230, 235),
        block::SMITHING_TABLE => c(110, 108, 112),
        block::COAL_GENERATOR => c(128, 118, 108),
        block::ELECTRIC_FURNACE => c(120, 128, 148),
        block::CRUSHER => c(112, 118, 128),
        block::ASSEMBLER => c(150, 138, 100),
        block::RESEARCH_BENCH => c(96, 148, 142),
        id if id >= lf_voxel::registry::MOD_BLOCK_BASE => c(150, 96, 196),
        _ => c(120, 120, 120),
    }
}

/// Cartographic palette for the 30 biomes (approx tiles + biome labels).
pub fn biome_color(b: Biome) -> Color32 {
    use Biome::*;
    let c = |r: u32, g: u32, b: u32| Color32::from_rgb(r as u8, g as u8, b as u8);
    match b {
        Meadow => c(120, 178, 90),
        FlowerForest => c(140, 185, 95),
        Forest => c(78, 140, 66),
        BirchForest => c(148, 168, 104),
        DarkForest => c(48, 100, 55),
        PaleGarden => c(150, 150, 140),
        CherryGrove => c(214, 150, 165),
        Taiga => c(70, 120, 85),
        SnowyTaiga => c(150, 175, 165),
        GiantTaiga => c(55, 105, 75),
        Tundra => c(200, 215, 215),
        IceSpikes => c(185, 220, 235),
        SnowySlope => c(215, 225, 228),
        SnowyPeaks => c(235, 240, 242),
        FrozenOcean => c(140, 175, 200),
        Jungle => c(50, 135, 60),
        Swamp => c(85, 105, 70),
        Savanna => c(175, 170, 95),
        WindsweptSavanna => c(165, 158, 92),
        Desert => c(228, 208, 140),
        Badlands => c(190, 115, 65),
        Beach => c(222, 210, 165),
        StonyShore => c(140, 140, 138),
        Ocean => c(55, 95, 165),
        DeepOcean => c(35, 62, 130),
        WarmOcean => c(60, 140, 175),
        Highlands => c(125, 145, 105),
        Mountains => c(130, 128, 125),
        WindsweptHills => c(145, 150, 120),
        MushroomHollow => c(150, 120, 145),
        Volcanic => c(72, 60, 58),
        Oasis => c(196, 186, 120),
        RedwoodForest => c(96, 66, 48),
        Mangrove => c(70, 104, 74),
        AspenGrove => c(188, 186, 130),
        BaobabFields => c(178, 158, 84),
        WillowWetlands => c(88, 118, 84),
        PaintedDunes => c(206, 142, 96),
        FrostMeadow => c(206, 216, 222),
        Emberwood => c(88, 62, 52),
        LavenderFields => c(150, 128, 196),
        MapleForest => c(178, 96, 58),
        PineBarrens => c(92, 112, 92),
        SaltFlats => c(232, 230, 222),
        FoggyFjord => c(128, 142, 148),
        SunflowerPlains => c(198, 186, 78),
    }
}

const FOG: Color32 = Color32::from_rgb(20, 24, 32);
const EXPLORED_DIM: f32 = 0.62;
/// Tiles older than this get recomputed (throttles without staleness).
const TILE_TTL: Duration = Duration::from_secs(4);

/// Persisted player marker.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Waypoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub name: String,
    pub color_idx: usize,
}

pub const WAYPOINT_COLORS: [Color32; 6] = [
    Color32::from_rgb(240, 200, 120), // gold
    Color32::from_rgb(120, 210, 130), // green
    Color32::from_rgb(110, 220, 255), // cyan
    Color32::from_rgb(240, 130, 120), // red
    Color32::from_rgb(220, 140, 240), // violet
    Color32::from_rgb(255, 255, 255), // white
];

/// A2: blend the faction color over the terrain tile — alpha ~0.30, light
/// enough that hillshade still reads, strong enough to claim the region.
fn apply_territory_tint(mut px: Vec<Color32>, tint: Option<Color32>) -> Vec<Color32> {
    if let Some(t) = tint {
        for p in px.iter_mut() {
            let a = 0.30;
            let blended = [
                (p.r() as f32 * (1.0 - a) + t.r() as f32 * a) as u8,
                (p.g() as f32 * (1.0 - a) + t.g() as f32 * a) as u8,
                (p.b() as f32 * (1.0 - a) + t.b() as f32 * a) as u8,
                p.a(),
            ];
            *p = Color32::from_rgba_unmultiplied(blended[0], blended[1], blended[2], blended[3]);
        }
    }
    px
}

fn shade(c: Color32, f: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (c.r() as f32 * f).clamp(0.0, 255.0) as u8,
        (c.g() as f32 * f).clamp(0.0, 255.0) as u8,
        (c.b() as f32 * f).clamp(0.0, 255.0) as u8,
        c.a(),
    )
}

#[derive(Clone, Copy, PartialEq)]
enum TileSource {
    /// Colored from actual world top blocks.
    Loaded,
    /// Explored but not loaded: seed-derived approximation, drawn dimmed.
    Approx,
}

struct Tile {
    px: Vec<Color32>, // 16*16
    source: TileSource,
    /// Territory tint (faction color) applied over the terrain shading.
    tint: Option<Color32>,
}

/// Per-client map cache + view state.
pub struct MapState {
    gen: WorldGen,
    tiles: HashMap<(i32, i32), Tile>,
    tiles_version: u64,
    minimap_tex: Option<egui::TextureHandle>,
    map_tex: Option<egui::TextureHandle>,
    /// (center, zoom, wh, tiles_version) the full-map texture was built for.
    map_view: Option<((f32, f32), f32, (usize, usize), u64)>,
    last_center_chunk: (i32, i32),
    last_refresh: Instant,
    /// Full-map view: pixels per block, and whether it tracks the player.
    pub zoom: f32,
    pub center: (f32, f32),
    pub following: bool,
    /// lore-and-visuals A2: biome -> faction territory color. Set by the
    /// client from lore data; tiles built while empty stay untinted.
    pub faction_tints: HashMap<Biome, Color32>,
    /// lore-and-visuals D3: discovered structure icons (x, z, faction
    /// color), drawn like waypoints on both map surfaces.
    pub structure_icons: Vec<(f32, f32, Color32)>,
}

impl MapState {
    pub fn new(world_type: lf_worldgen::WorldType, seed: u64) -> Self {
        Self {
            gen: WorldGen::with_type(Seed(seed), world_type),
            tiles: HashMap::new(),
            tiles_version: 0,
            minimap_tex: None,
            map_tex: None,
            map_view: None,
            last_center_chunk: (i32::MAX, i32::MAX),
            last_refresh: Instant::now() - TILE_TTL,
            zoom: 2.0,
            center: (0.0, 0.0),
            following: true,
            faction_tints: HashMap::new(),
            structure_icons: Vec::new(),
        }
    }

    /// Territory tint for a chunk (faction color, chunk-center biome).
    fn territory_tint(&self, cx: i32, cz: i32) -> Option<Color32> {
        let biome = self.gen.biome(cx * 16 + 8, cz * 16 + 8);
        self.faction_tints.get(&biome).copied()
    }

    pub fn biome_at(&self, x: i32, z: i32) -> Biome {
        self.gen.biome(x, z)
    }

    /// True when the player crossed into a new chunk or the TTL expired.
    fn due(&mut self, center_chunk: (i32, i32)) -> bool {
        if center_chunk != self.last_center_chunk || self.last_refresh.elapsed() > TILE_TTL {
            self.last_center_chunk = center_chunk;
            self.last_refresh = Instant::now();
            true
        } else {
            false
        }
    }

    /// Recompute stale/new tiles around the player. Cheap when nothing
    /// moved: cached tiles are reused; dirty/edited chunks are rebuilt.
    pub fn refresh(&mut self, world: &World, loaded: &HashSet<(i32, i32)>, saved: &HashSet<(i32, i32)>,
                   dirty: &HashSet<(i32, i32)>, center_chunk: (i32, i32), radius: i32) {
        if !self.due(center_chunk) {
            return;
        }
        let mut bumped = false;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let pos = (center_chunk.0 + dx, center_chunk.1 + dz);
                if loaded.contains(&pos) {
                    let tint = self.territory_tint(pos.0, pos.1);
                    let stale = !matches!(self.tiles.get(&pos), Some(t) if t.source == TileSource::Loaded && t.tint == tint)
                        || dirty.contains(&pos);
                    if stale {
                        let px = tile_from_world(world, pos.0, pos.1);
                        let px = apply_territory_tint(px, tint);
                        self.tiles.insert(pos, Tile { px, source: TileSource::Loaded, tint });
                        bumped = true;
                    }
                } else if saved.contains(&pos) {
                    let tint = self.territory_tint(pos.0, pos.1);
                    let stale = !matches!(self.tiles.get(&pos), Some(t) if t.source == TileSource::Approx && t.tint == tint);
                    if stale {
                        let px = tile_from_gen(&self.gen, pos.0, pos.1);
                        let px = apply_territory_tint(px, tint);
                        self.tiles.insert(pos, Tile { px, source: TileSource::Approx, tint });
                        bumped = true;
                    }
                } else if self.tiles.remove(&pos).is_some() {
                    bumped = true;
                }
            }
        }
        // prune far tiles so the cache stays bounded
        let keep = radius + 8;
        let before = self.tiles.len();
        self.tiles.retain(|(x, z), _| (x - center_chunk.0).abs() <= keep && (z - center_chunk.1).abs() <= keep);
        if self.tiles.len() != before {
            bumped = true;
        }
        if bumped {
            self.tiles_version += 1;
        }
    }

    fn dim_approx(source: TileSource, c: Color32) -> Color32 {
        match source {
            TileSource::Loaded => c,
            TileSource::Approx => shade(c, EXPLORED_DIM),
        }
    }

    /// Sample the composited color at a world block position (fog if unknown).
    fn sample(&self, x: i32, z: i32) -> Color32 {
        let (cx, lx) = (x.div_euclid(16), x.rem_euclid(16) as usize);
        let (cz, lz) = (z.div_euclid(16), z.rem_euclid(16) as usize);
        match self.tiles.get(&(cx, cz)) {
            Some(t) => Self::dim_approx(t.source, t.px[lz * 16 + lx]),
            None => FOG,
        }
    }

    /// Build/refresh the region texture around `center` (world xz) at
    /// `px_per_block`, reusing the handle like the live-RT path.
    fn composite(&mut self, ctx: &egui::Context, name: &str, existing: Option<egui::TextureHandle>,
                 wh: (usize, usize), center: (f32, f32), px_per_block: f32) -> egui::TextureHandle {
        let (w, h) = wh;
        let mut pixels = Vec::with_capacity(w * h);
        let step = 1.0 / px_per_block;
        let x0 = center.0 - w as f32 / (2.0 * px_per_block);
        let z0 = center.1 - h as f32 / (2.0 * px_per_block);
        let mut wz = z0;
        for _ in 0..h {
            let mut wx = x0;
            for _ in 0..w {
                pixels.push(self.sample(wx.floor() as i32, wz.floor() as i32));
                wx += step;
            }
            wz += step;
        }
        let image = egui::ColorImage { size: [w, h], pixels };
        match existing {
            Some(mut t) => {
                t.set(image, egui::TextureOptions::NEAREST);
                t
            }
            None => ctx.load_texture(name, image, egui::TextureOptions::NEAREST),
        }
    }

    /// World-space position of a pixel offset in a composited view.
    fn pixel_to_world(center: (f32, f32), wh: (usize, usize), px_per_block: f32, p: Vec2) -> (f32, f32) {
        let step = 1.0 / px_per_block;
        (
            center.0 + (p.x - wh.0 as f32 / 2.0) * step,
            center.1 + (p.y - wh.1 as f32 / 2.0) * step,
        )
    }
}

/// Color a chunk tile from the loaded world: per column the first non-air
/// block from the top (water counts — oceans read as water), with simple
/// west-neighbor hillshading. Sections whose palette is only air are
/// skipped so the scan stays cheap.
fn tile_from_world(world: &World, cx: i32, cz: i32) -> Vec<Color32> {
    let Some(col) = world.chunk(cx, cz) else {
        return vec![FOG; 256];
    };
    let mut heights = [0i32; 256];
    let mut colors = [FOG; 256];
    for lz in 0..16usize {
        for lx in 0..16usize {
            let mut found = None;
            'scan: for s in (0..col.sections.len()).rev() {
                let sec = &col.sections[s];
                if sec.palette.len() == 1 && sec.palette[0].id() == block::AIR {
                    continue;
                }
                for y in (0..16usize).rev() {
                    let id = col.get(lx, s * 16 + y, lz).id();
                    if id != block::AIR {
                        found = Some((id, (s * 16 + y) as i32));
                        break 'scan;
                    }
                }
            }
            if let Some((id, y)) = found {
                let idx = lz * 16 + lx;
                heights[idx] = y;
                colors[idx] = block_map_color(id);
            }
        }
    }
    let mut out = Vec::with_capacity(256);
    for lz in 0..16usize {
        for lx in 0..16usize {
            let idx = lz * 16 + lx;
            let west = if lx > 0 { heights[idx - 1] } else { heights[idx] };
            let f = (1.0 + (heights[idx] - west) as f32 * 0.035).clamp(0.62, 1.30);
            out.push(shade(colors[idx], f));
        }
    }
    out
}

/// Approximate tile for explored-but-unloaded chunks: biome color + height
/// shading sampled on a 4x4 lattice (keeps noise calls at 16/chunk).
fn tile_from_gen(gen: &WorldGen, cx: i32, cz: i32) -> Vec<Color32> {
    const N: usize = 4;
    let mut lattice_h = [[0i32; N]; N];
    let mut lattice_c = [[Color32::default(); N]; N];
    for gz in 0..N {
        for gx in 0..N {
            let wx = cx * 16 + (gx * 16 / N) as i32;
            let wz = cz * 16 + (gz * 16 / N) as i32;
            lattice_h[gz][gx] = gen.height(wx, wz);
            lattice_c[gz][gx] = biome_color(gen.biome(wx, wz));
        }
    }
    let mut out = Vec::with_capacity(256);
    for lz in 0..16 {
        for lx in 0..16 {
            let gx = (lx * N / 16).min(N - 1);
            let gz = (lz * N / 16).min(N - 1);
            let h = lattice_h[gz][gx];
            let west = if gx > 0 { lattice_h[gz][gx - 1] } else { h };
            let mut f = 1.0 + (h - west) as f32 * 0.035;
            if h <= lf_worldgen::SEA_LEVEL {
                f = 1.0;
            }
            out.push(shade(lattice_c[gz][gx], f.clamp(0.62, 1.30)));
        }
    }
    out
}

// ------------------------------------------------------------------
// Minimap (corner HUD widget)

const MINIMAP_PX: usize = 172;

impl GameState {
    /// Top-right minimap: terrain, entity dots, waypoints, player arrow.
    pub fn draw_minimap(&mut self, ctx: &egui::Context) {
        if !self.settings.show_minimap {
            return;
        }
        let p = self.player.position;
        let center_chunk = (p.x.div_euclid(16.0) as i32, p.z.div_euclid(16.0) as i32);
        let loaded: HashSet<(i32, i32)> = self.batches.keys().copied().collect();
        self.map.refresh(&self.world, &loaded, &self.saved_set, &self.dirty, center_chunk, 6);
        let center = (p.x, p.z);
        let zoom = self.settings.minimap_zoom.clamp(0.5, 3.0);
        let existing = self.map.minimap_tex.take();
        let tex = self.map.composite(ctx, "minimap", existing, (MINIMAP_PX, MINIMAP_PX), center, zoom);
        self.map.minimap_tex = Some(tex.clone());
        // Step 15 rotation: facing direction maps to screen-up; texture and
        // markers share the same rotation so they stay aligned.
        let look = self.player.look_dir();
        let ang = look.x.atan2(look.z);
        let rotate = self.settings.rotate_minimap;
        // rotation by (ang + PI) sends the facing vector (sin a, cos a) to (0, -1)
        let (rc, rs) = ((ang + std::f32::consts::PI).cos(), (ang + std::f32::consts::PI).sin());

        egui::Area::new(egui::Id::new("minimap"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 34.0))
            .show(ctx, |ui| {
                let size = MINIMAP_PX as f32;
                let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
                let paint = ui.painter();
                paint.rect_filled(rect, 8.0, FOG);
                if rotate {
                    // rotated texture draw: custom mesh with UVs rotated by
                    // the inverse angle around the texture center
                    let inv = -(ang + std::f32::consts::PI);
                    let (ic, is) = (inv.cos(), inv.sin());
                    let corners = [
                        (rect.left_top(), Vec2::new(-0.5, -0.5)),
                        (rect.right_top(), Vec2::new(0.5, -0.5)),
                        (rect.right_bottom(), Vec2::new(0.5, 0.5)),
                        (rect.left_bottom(), Vec2::new(-0.5, 0.5)),
                    ];
                    let mut verts = Vec::with_capacity(4);
                    for (pos, off) in corners {
                        let uv = Pos2::new(0.5 + off.x * ic - off.y * is, 0.5 + off.x * is + off.y * ic);
                        verts.push(egui::epaint::Vertex { pos, uv, color: Color32::WHITE });
                    }
                    let mut mesh = egui::Mesh::default();
                    mesh.vertices = verts;
                    mesh.indices = vec![0, 1, 2, 0, 2, 3];
                    mesh.texture_id = tex.id();
                    paint.add(egui::Shape::Mesh(std::sync::Arc::new(mesh)));
                } else {
                    let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
                    paint.image(tex.id(), rect, uv, Color32::WHITE);
                }
                paint.rect_stroke(rect, 8.0, egui::Stroke::new(2.0, Theme::ACCENT_DIM), egui::StrokeKind::Middle);
                // north chip: pinned at the top when north-up, riding the
                // rotated rim when the map spins
                let north_pos = if rotate {
                    let half = size / 2.0 - 8.0;
                    rect.center() + Vec2::new(-0.0 * rc - (-half) * rs, -0.0 * rs + (-half) * rc)
                } else {
                    rect.center_top() + Vec2::new(0.0, 6.0)
                };
                paint.rect_filled(Rect::from_center_size(north_pos, egui::vec2(18.0, 12.0)), 3.0, Theme::BG);
                paint.text(north_pos, egui::Align2::CENTER_CENTER, "N",
                    egui::FontId::proportional(10.0), Theme::ACCENT);

                let to_screen = |wx: f32, wz: f32| -> Pos2 {
                    let dx = (wx - center.0) * zoom;
                    let dz = (wz - center.1) * zoom;
                    if rotate {
                        Pos2::new(
                            rect.center().x + dx * rc - dz * rs,
                            rect.center().y + dx * rs + dz * rc,
                        )
                    } else {
                        Pos2::new(rect.left() + dx + size / 2.0, rect.top() + dz + size / 2.0)
                    }
                };
                for mob in &self.mobs {
                    let c = if mob.mob_type.is_hostile() { Theme::BAD } else { Theme::TEXT_DIM };
                    paint.circle_filled(to_screen(mob.position.x, mob.position.z), 2.0, c);
                }
                for v in &self.villagers {
                    paint.circle_filled(to_screen(v.position[0], v.position[2]), 2.0, Theme::OK);
                }
                // lore-and-visuals D3: discovered faction structures — a
                // small faction-color diamond at their world position
                for (ix, iz, col) in &self.map.structure_icons {
                    let mut pos = to_screen(*ix, *iz);
                    pos.x = pos.x.clamp(rect.left() + 6.0, rect.right() - 6.0);
                    pos.y = pos.y.clamp(rect.top() + 6.0, rect.bottom() - 6.0);
                    let r = 4.0;
                    let diamond = vec![
                        pos + Vec2::new(0.0, -r),
                        pos + Vec2::new(r, 0.0),
                        pos + Vec2::new(0.0, r),
                        pos + Vec2::new(-r, 0.0),
                    ];
                    paint.add(egui::Shape::convex_polygon(diamond, *col, egui::Stroke::new(1.0, Theme::BG)));
                }
                // waypoint pips (clamped to the edge when off-map)
                for wp in &self.waypoints {
                    let mut pos = to_screen(wp.x, wp.z);
                    pos.x = pos.x.clamp(rect.left() + 6.0, rect.right() - 6.0);
                    pos.y = pos.y.clamp(rect.top() + 6.0, rect.bottom() - 6.0);
                    let col = WAYPOINT_COLORS[wp.color_idx % WAYPOINT_COLORS.len()];
                    paint.circle_filled(pos, 3.5, col);
                    paint.circle_stroke(pos, 3.5, egui::Stroke::new(1.0, Theme::BG));
                }
                // player arrow: points along the look direction, or straight
                // up when the map itself rotates with the player
                let c = rect.center();
                let dir = if rotate { Vec2::new(0.0, -1.0) } else { Vec2::new(ang.sin(), ang.cos()) };
                let tip = c + dir * 7.0;
                let left = c + Vec2::new(-dir.y, dir.x) * 4.0 - dir * 4.0;
                let right = c - Vec2::new(-dir.y, dir.x) * 4.0 - dir * 4.0;
                paint.add(egui::Shape::convex_polygon(vec![tip, left, right], Color32::WHITE, egui::Stroke::new(1.0, Theme::BG)));
                if response.hovered() {
                    let b = self.map.biome_at(p.x as i32, p.z as i32);
                    paint.text(rect.center_bottom() + Vec2::new(0.0, 14.0), egui::Align2::CENTER_CENTER,
                        format!("{:.0}, {:.0} · {}", p.x, p.z, b.name()),
                        egui::FontId::proportional(11.0), Theme::TEXT);
                }
            });
    }

    /// Full-screen world map (M). Pan by dragging, zoom with the wheel,
    /// manage waypoints on the right.
    pub fn draw_map_screen(&mut self, ctx: &egui::Context) {
        let p = self.player.position;
        let center_chunk = (p.x.div_euclid(16.0) as i32, p.z.div_euclid(16.0) as i32);
        let loaded: HashSet<(i32, i32)> = self.batches.keys().copied().collect();
        self.map.refresh(&self.world, &loaded, &self.saved_set, &self.dirty, center_chunk, 10);

        let reveal = crate::ui_kit::ease_out_cubic((self.menu_reveal / 0.35).clamp(0.0, 1.0));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::from_black_alpha((150.0 * reveal) as u8)))
            .show(ctx, |ui| {
                crate::ui_kit::slide_panel(ui, reveal, |ui| {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        // ---- map canvas ----
                        let avail = ui.available_size();
                        let canvas = egui::vec2((avail.x - 250.0).max(320.0), (avail.y - 12.0).max(240.0));
                        let (rect, response) = ui.allocate_exact_size(canvas, egui::Sense::click_and_drag());
                        let wh = (rect.width() as usize, rect.height() as usize);
                        let zoom = self.map.zoom;
                        let scroll = ui.ctx().input(|i| i.raw_scroll_delta.y);
                        if scroll != 0.0 {
                            self.map.zoom = (self.map.zoom * (1.0 + scroll * 0.001)).clamp(0.5, 6.0);
                        }
                        if response.dragged() {
                            let delta = response.drag_delta();
                            if delta != Vec2::ZERO {
                                let step = 1.0 / zoom;
                                self.map.center.0 -= delta.x * step;
                                self.map.center.1 -= delta.y * step;
                                self.map.following = false;
                            }
                        }
                        if self.map.following {
                            self.map.center = (p.x, p.z);
                        }
                        // composite only when the view actually changed
                        let view = (self.map.center, self.map.zoom, wh, self.map.tiles_version);
                        if self.map.map_view.as_ref() != Some(&view) {
                            let existing = self.map.map_tex.take();
                            let t = self.map.composite(ctx, "world_map", existing, wh, self.map.center, self.map.zoom);
                            self.map.map_tex = Some(t);
                            self.map.map_view = Some(view);
                        }
                        let paint = ui.painter();
                        paint.rect_filled(rect, 6.0, FOG);
                        if let Some(tex) = &self.map.map_tex {
                            let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
                            paint.image(tex.id(), rect, uv, Color32::WHITE);
                        }
                        paint.rect_stroke(rect, 6.0, egui::Stroke::new(1.5, Theme::ACCENT_DIM), egui::StrokeKind::Middle);

                        // chunk grid at high zoom
                        if zoom >= 2.0 {
                            let step = 16.0 * zoom;
                            let alpha = ((zoom - 2.0) / 4.0).clamp(0.0, 1.0) * 0.25;
                            let grid_col = Color32::from_rgba_unmultiplied(255, 255, 255, (alpha * 255.0) as u8);
                            let off_x = (rect.left() + (self.map.center.0 * zoom)) % step;
                            let off_x = if off_x > 0.0 { step - off_x } else { 0.0 };
                            let mut x = rect.left() + off_x;
                            while x < rect.right() {
                                paint.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], egui::Stroke::new(1.0, grid_col));
                                x += step;
                            }
                            let off_y = (rect.top() + (self.map.center.1 * zoom)) % step;
                            let off_y = if off_y > 0.0 { step - off_y } else { 0.0 };
                            let mut y = rect.top() + off_y;
                            while y < rect.bottom() {
                                paint.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], egui::Stroke::new(1.0, grid_col));
                                y += step;
                            }
                        }

                        let map_center = self.map.center;
                        let to_screen = move |wx: f32, wz: f32| -> Pos2 {
                            Pos2::new(
                                rect.left() + (wx - map_center.0) * zoom + rect.width() / 2.0,
                                rect.top() + (wz - map_center.1) * zoom + rect.height() / 2.0,
                            )
                        };
                        // spawn marker (diamond)
                        let sp = to_screen(self.spawn_point.x, self.spawn_point.z);
                        paint.add(egui::Shape::convex_polygon(
                            vec![sp + Vec2::new(0.0, -6.0), sp + Vec2::new(6.0, 0.0), sp + Vec2::new(0.0, 6.0), sp + Vec2::new(-6.0, 0.0)],
                            Color32::from_rgb(240, 120, 140),
                            egui::Stroke::new(1.5, Theme::BG),
                        ));
                        // lore-and-visuals D3: faction structure icons
                        for (ix, iz, col) in &self.map.structure_icons {
                            let pos = to_screen(*ix, *iz);
                            if rect.contains(pos) {
                                let r = 5.0;
                                paint.add(egui::Shape::convex_polygon(
                                    vec![pos + Vec2::new(0.0, -r), pos + Vec2::new(r, 0.0),
                                         pos + Vec2::new(0.0, r), pos + Vec2::new(-r, 0.0)],
                                    *col,
                                    egui::Stroke::new(1.5, Theme::BG),
                                ));
                            }
                        }
                        // waypoints
                        for wp in self.waypoints.iter() {
                            let pos = to_screen(wp.x, wp.z);
                            if rect.contains(pos) {
                                let col = WAYPOINT_COLORS[wp.color_idx % WAYPOINT_COLORS.len()];
                                paint.circle_filled(pos, 5.0, col);
                                paint.circle_stroke(pos, 5.0, egui::Stroke::new(1.5, Theme::BG));
                                let dist = ((wp.x - p.x).powi(2) + (wp.z - p.z).powi(2)).sqrt();
                                paint.text(pos + Vec2::new(0.0, -12.0), egui::Align2::CENTER_CENTER,
                                    format!("{} · {:.0}m", wp.name, dist),
                                    egui::FontId::proportional(11.0), Theme::TEXT);
                            }
                        }
                        // player arrow
                        let look = self.player.look_dir();
                        let ang = look.x.atan2(look.z);
                        let c = to_screen(p.x, p.z);
                        let dir = Vec2::new(ang.sin(), ang.cos());
                        let scale = 2.0 + zoom * 1.5;
                        let tip = c + dir * (4.0 + scale);
                        let left = c + Vec2::new(-dir.y, dir.x) * (3.0 + scale * 0.6) - dir * (2.0 + scale * 0.5);
                        let right = c - Vec2::new(-dir.y, dir.x) * (3.0 + scale * 0.6) - dir * (2.0 + scale * 0.5);
                        paint.add(egui::Shape::convex_polygon(vec![tip, left, right], Color32::WHITE, egui::Stroke::new(1.0, Theme::BG)));
                        // compass + cursor readout + legend
                        paint.text(rect.center_top() + Vec2::new(0.0, 12.0), egui::Align2::CENTER_CENTER, "N",
                            egui::FontId::proportional(13.0), Theme::ACCENT);
                        if let Some(cursor) = response.hover_pos() {
                            let local = cursor - rect.min;
                            let (wx, wz) = MapState::pixel_to_world(self.map.center, wh, zoom, local);
                            let b = self.map.biome_at(wx.floor() as i32, wz.floor() as i32);
                            paint.rect_filled(Rect::from_min_size(rect.left_bottom(), egui::vec2(240.0, 20.0)), 3.0, Color32::from_black_alpha(170));
                            paint.text(rect.left_bottom() + Vec2::new(8.0, 10.0), egui::Align2::LEFT_CENTER,
                                format!("{:.0}, {:.0} · {}", wx, wz, b.name()),
                                egui::FontId::proportional(11.0), Theme::TEXT);
                        }
                        paint.text(rect.right_bottom() + Vec2::new(-160.0, -10.0), egui::Align2::LEFT_CENTER,
                            "drag pan · wheel zoom · M close", egui::FontId::proportional(10.0), Theme::TEXT_DIM);

                        // ---- waypoint manager (right column) ----
                        ui.vertical(|ui| {
                            ui.add_space(4.0);
                            crate::ui_kit::section_header(ui, "Waypoints", 1.0);
                            ui.add_space(10.0);
                            if crate::ui_kit::menu_button(ui, &format!("+ Marker at {:.0},{:.0}", p.x, p.z), 1.0, true) {
                                let n = self.waypoints.len() + 1;
                                self.waypoints.push(Waypoint {
                                    x: p.x, y: p.y, z: p.z,
                                    name: format!("Marker {}", n),
                                    color_idx: n % WAYPOINT_COLORS.len(),
                                });
                            }
                            ui.add_space(6.0);
                            let mut delete = None;
                            let max_h = ui.available_height() - 46.0;
                            egui::ScrollArea::vertical().max_height(max_h).show(ui, |ui| {
                                for (i, wp) in self.waypoints.iter_mut().enumerate() {
                                    egui::Frame::new()
                                        .fill(Color32::from_black_alpha(100))
                                        .corner_radius(6.0)
                                        .inner_margin(6.0)
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                let col = WAYPOINT_COLORS[wp.color_idx % WAYPOINT_COLORS.len()];
                                                let (dot, resp) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::click());
                                                ui.painter().circle_filled(dot.center(), 5.0, col);
                                                if resp.clicked() {
                                                    wp.color_idx = (wp.color_idx + 1) % WAYPOINT_COLORS.len();
                                                }
                                                let dist = ((wp.x - p.x).powi(2) + (wp.z - p.z).powi(2)).sqrt();
                                                ui.vertical(|ui| {
                                                    let mut name = wp.name.clone();
                                                    ui.set_width(120.0);
                                                    if ui.text_edit_singleline(&mut name).changed() {
                                                        wp.name = name;
                                                    }
                                                    ui.label(egui::RichText::new(format!("{:.0}m", dist)).small().color(Theme::TEXT_DIM));
                                                });
                                                if ui.small_button("×").clicked() {
                                                    delete = Some(i);
                                                }
                                            });
                                        });
                                }
                            });
                            if let Some(i) = delete {
                                self.waypoints.remove(i);
                            }
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("zoom").small().color(Theme::TEXT_DIM));
                                ui.add(egui::Slider::new(&mut self.map.zoom, 0.5..=6.0));
                            });
                            if ui.button("Center on player").clicked() {
                                self.map.following = true;
                            }
                        });
                    });
                });
            });
    }
}

/// Compass facing label for the HUD info line.
pub fn compass_facing(yaw: f32) -> &'static str {
    let deg = yaw.to_degrees().rem_euclid(360.0);
    match (deg / 45.0).round() as i32 % 8 {
        0 => "S", 1 => "SW", 2 => "W", 3 => "NW", 4 => "N", 5 => "NE", 6 => "E", _ => "SE",
    }
}

/// A seed-derived top-down thumbnail (ui-world-craft C2): a grid of
/// WorldGen-approximated map tiles rendered top-down, RGBA8, row-major.
/// Pure generation — no GPU, no live world — so the Load World screen can
/// cache one to `thumbnail.png` per slot on first open.
pub fn seed_thumbnail_rgba(seed: u64, world_type: lf_worldgen::WorldType, chunks_per_side: usize) -> Vec<u8> {
    let gen = WorldGen::with_type(Seed(seed), world_type);
    let side = chunks_per_side as i32;
    let half = side / 2;
    let mut out = Vec::with_capacity(16 * 16 * chunks_per_side * chunks_per_side * 4);
    for cz in -half..=(side - 1 - half) {
        for cx in -half..=(side - 1 - half) {
            for px in tile_from_gen(&gen, cx, cz) {
                let [r, g, b, a] = px.to_array();
                out.extend_from_slice(&[r, g, b, a]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use lf_voxel::BlockState;
    const TEST_SEED: u64 = 12345;

    #[test]
    fn biome_palette_covers_all_30() {
        use Biome::*;
        for b in [Meadow, FlowerForest, Forest, BirchForest, DarkForest, PaleGarden, CherryGrove,
            Taiga, SnowyTaiga, GiantTaiga, Tundra, IceSpikes, SnowySlope, SnowyPeaks, FrozenOcean,
            Jungle, Swamp, Savanna, WindsweptSavanna, Desert, Badlands, Beach, StonyShore, Ocean,
            DeepOcean, WarmOcean, Highlands, Mountains, WindsweptHills, MushroomHollow] {
            let c = biome_color(b);
            assert_ne!(c, Color32::default(), "{} uncolored", b.name());
        }
    }

    #[test]
    fn loaded_tile_colors_follow_top_blocks() {
        let mut world = World::new();
        let gen = WorldGen::new(Seed(TEST_SEED));
        world.chunks.insert((0, 0), gen.generate_chunk(0, 0));
        let tile = tile_from_world(&world, 0, 0);
        assert_eq!(tile.len(), 256);
        assert!(tile.iter().any(|c| *c != FOG), "tile should not be pure fog");
        // editing the top block changes the tile (player edits are respected)
        world.set_block(0, 250, 0, BlockState(block::CHEST));
        let tile2 = tile_from_world(&world, 0, 0);
        assert_eq!(tile2[0], block_map_color(block::CHEST), "placed chest must recolor the tile");
    }

    #[test]
    fn approx_tile_is_deterministic() {
        let gen = WorldGen::new(Seed(TEST_SEED));
        let a = tile_from_gen(&gen, 3, -7);
        let b = tile_from_gen(&gen, 3, -7);
        assert_eq!(a, b, "same chunk must render identically");
        assert!(a.iter().any(|c| *c != FOG));
    }

    #[test]
    fn composite_and_world_pixel_roundtrip() {
        let ctx = egui::Context::default();
        let mut map = MapState::new(lf_worldgen::WorldType::Normal, TEST_SEED);
        let mut world = World::new();
        let gen = WorldGen::new(Seed(TEST_SEED));
        for cx in -1..=1 {
            for cz in -1..=1 {
                world.chunks.insert((cx, cz), gen.generate_chunk(cx, cz));
            }
        }
        let loaded: HashSet<(i32, i32)> = world.chunks.keys().copied().collect();
        map.last_center_chunk = (99, 99); // bypass the throttle
        map.refresh(&world, &loaded, &loaded, &HashSet::new(), (0, 0), 2);
        assert!(!map.tiles.is_empty());
        let tex = map.composite(&ctx, "test_map", None, (32, 32), (0.0, 0.0), 1.0);
        assert_eq!(tex.size_vec2(), egui::vec2(32.0, 32.0));
        let (wx, wz) = MapState::pixel_to_world((0.0, 0.0), (32, 32), 1.0, Vec2::new(16.0, 16.0));
        assert!((wx - 0.0).abs() < 0.01 && (wz - 0.0).abs() < 0.01);
        assert_ne!(map.sample(wx.floor() as i32, wz.floor() as i32), FOG, "center of a loaded world must not be fog");
    }

    #[test]
    fn compass_covers_facings() {
        for yaw in [0.0f32, 0.785, 3.14, -3.14, 6.28] {
            let f = compass_facing(yaw);
            assert!(f.len() <= 2);
        }
    }

    /// ui-world-craft C2: seed thumbnails are pure generation, sized right,
    /// deterministic, and distinct between seeds.
    #[test]
    fn seed_thumbnail_is_deterministic_and_distinct() {
        let side = 4usize;
        let a = seed_thumbnail_rgba(TEST_SEED, lf_worldgen::WorldType::Normal, side);
        let b = seed_thumbnail_rgba(TEST_SEED, lf_worldgen::WorldType::Normal, side);
        assert_eq!(a.len(), b.len());
        assert_eq!(a.len(), 16 * 16 * side * side * 4, "16px tiles, RGBA");
        assert!(a.chunks(4).any(|px| px != [20, 24, 32, 255]), "not pure fog");
        // different seeds look different (statistically: some pixel differs)
        let c = seed_thumbnail_rgba(TEST_SEED + 1, lf_worldgen::WorldType::Normal, side);
        assert_ne!(a, c, "different seeds must render different worlds");
    }
}
