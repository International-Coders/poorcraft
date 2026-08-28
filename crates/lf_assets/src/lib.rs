use image::{Rgba, RgbaImage};

/// Canonical texture atlas layer order. Block ids map onto these indices.
pub const TEXTURE_NAMES: [&str; 165] = [
    "stone", "grass", "dirt", "sand", "mycelium", "snow",
    "log", "leaves", "coal_ore", "iron_ore", "water", "torch_item", "crafting_table",
    "furnace", "chest", "planks", "glass",
    // wood/biome variants (18-30)
    "birch_log", "spruce_log", "dark_log", "cherry_log",
    "birch_leaves", "spruce_leaves", "dark_leaves", "cherry_leaves", "pale_leaves",
    "red_sand", "terracotta", "moss", "ice",
    // industrial ores (31-34)
    "copper_ore", "tin_ore", "bauxite_ore", "sulfur_ore",
    // machines + benches (34-39)
    "smithing_table", "coal_generator", "electric_furnace", "crusher", "assembler",
    "research_bench",
    "lantern",
    // per-face materials (P26): grass top face, shared log end rings
    "grass_top", "log_top",
    // progressive crack decal stages for the mining overlay
    "crack_0", "crack_1", "crack_2", "crack_3",
    "mod",
    // waypoint beacon column tints (Step 15) — translucent, drawn in the
    // transparent pass as world-space beams
    "waypoint_0", "waypoint_1", "waypoint_2", "waypoint_3", "waypoint_4", "waypoint_5",
    // biome-identity surfaces (Step 16-17)
    "jungle_grass", "savanna_grass", "flower",
    // Water Age machines (P29)
    "water_wheel", "battery",
    // Steam Age machines (P30)
    "pipe", "boiler", "steam_engine",
    // Oil Age (P31): crude fluid + machines, and the power-grid overlay
    // tints (translucent, transparent pass like the waypoint beams)
    "oil", "pump", "refinery", "combustion_generator",
    "grid_ok", "grid_starved",
    // Nuclear tier (P32): deep uranium, the reactor, meltdown residue
    "uranium_ore", "reactor", "radiation",
    // Magic foundation (P33)
    "enchanting_table", "lumen_block", "warding_pylon",
    // Construction (P34)
    "scaffold", "statue",
    // Smart building (P35): relays, ride, climate, and the dynamic screen
    "conduit", "elevator", "ac_unit", "computer", "screen",
    // P36 dragons
    "dragon_scale_block", "dragon_egg",
    // Step 11: connected (edgeless) variants for large flat surfaces
    "stone_conn", "planks_conn",
    // Step 27: the item belt
    "belt",
    // Faction blocks (lore-and-visuals C1, 86-97)
    "accord_stone", "accord_pillar", "ironborn_brick", "ironborn_grate",
    "ember_covenantwood", "ember_glowstone", "freeholds_thatch", "freeholds_daub",
    "ashen_marble", "ashen_bookshelf", "nameless_rotwood", "nameless_scorched",
    // Biome-exclusive blocks (98-105)
    "mushroom_cap", "coral_block", "permafrost", "volcanic_basalt",
    "deep_slate", "mesa_terracotta", "gilded_grass", "bog_peat",
    // Decoration blocks (106-123)
    "carved_oak", "carved_stone", "carved_iron",
    "stained_glass_red", "stained_glass_orange", "stained_glass_yellow",
    "stained_glass_green", "stained_glass_blue", "stained_glass_purple",
    "stained_glass_black", "stained_glass_white",
    "banner_accord", "banner_ironborn", "banner_covenant",
    "banner_freeholds", "banner_ashen", "banner_nameless",
    "lantern_hanging",
    // Villager faction skins (C2, 124-131)
    "villager_accord", "villager_ironborn", "villager_covenant",
    "villager_freeholds", "villager_ashen", "villager_nameless",
    "villager_unmarked", "villager_maren",
    // Companion skins + trust-badge variants (132-143)
    "companion_accord_warden", "companion_ironborn_artisan",
    "companion_covenant_channeler", "companion_freeholds_scout",
    "companion_ashen_scribe", "companion_nameless_rover",
    "companion_accord_warden_trusted", "companion_ironborn_artisan_trusted",
    "companion_covenant_channeler_trusted", "companion_freeholds_scout_trusted",
    "companion_ashen_scribe_trusted", "companion_nameless_rover_trusted",
    // Mob skins (C2 refresh, 144-149)
    "mob_boar", "mob_woolbeast", "mob_glitchling", "mob_stalker",
    "mob_crawler", "mob_null_knight",
    // Biome-tint variants of the common hostiles (150-158)
    "mob_glitchling_desert", "mob_stalker_desert", "mob_crawler_desert",
    "mob_glitchling_snow", "mob_stalker_snow", "mob_crawler_snow",
    "mob_glitchling_swamp", "mob_stalker_swamp", "mob_crawler_swamp",
    // Ambient ember particle core (C4)
    "ember",
    // ui-world-craft D3/E3: lava + biome surface decoration (160-164)
    "tall_grass", "dry_grass", "cactus", "dead_shrub", "lava",
];

/// Atlas layers of the waypoint beacon tints, indexed by waypoint color.
pub const WAYPOINT_LAYERS: [u32; 6] = [48, 49, 50, 51, 52, 53];

/// Atlas layers of the biome-identity grasses (Step 16).
pub const JUNGLE_GRASS_LAYER: u32 = 54;
/// ui-world-craft D3/E3 decoration + lava layers.
pub const TALL_GRASS_LAYER: u32 = 160;
pub const DRY_GRASS_LAYER: u32 = 161;
pub const CACTUS_LAYER: u32 = 162;
pub const DEAD_SHRUB_LAYER: u32 = 163;
pub const LAVA_LAYER: u32 = 164;
pub const SAVANNA_GRASS_LAYER: u32 = 55;
pub const FLOWER_LAYER: u32 = 56;
pub const WATER_WHEEL_LAYER: u32 = 57;
pub const BATTERY_LAYER: u32 = 58;
pub const PIPE_LAYER: u32 = 59;
pub const BOILER_LAYER: u32 = 60;
pub const STEAM_ENGINE_LAYER: u32 = 61;
pub const OIL_LAYER: u32 = 62;
pub const PUMP_LAYER: u32 = 63;
pub const REFINERY_LAYER: u32 = 64;
pub const COMBUSTION_LAYER: u32 = 65;
/// Power-grid overlay tints (Step 25): green = powered, red = starved.
pub const GRID_OK_LAYER: u32 = 66;
pub const GRID_STARVED_LAYER: u32 = 67;
pub const URANIUM_LAYER: u32 = 68;
pub const REACTOR_LAYER: u32 = 69;
pub const RADIATION_LAYER: u32 = 70;
pub const ENCHANTING_LAYER: u32 = 71;
pub const LUMEN_LAYER: u32 = 72;
pub const WARDING_LAYER: u32 = 73;
pub const SCAFFOLD_LAYER: u32 = 74;
pub const STATUE_LAYER: u32 = 75;
pub const CONDUIT_LAYER: u32 = 76;
pub const ELEVATOR_LAYER: u32 = 77;
pub const AC_LAYER: u32 = 78;
pub const COMPUTER_LAYER: u32 = 79;
/// The dynamic screen layer (P35): the client rewrites its pixels when
/// the displayed page or data changes (data-change-driven uploads).
pub const SCREEN_LAYER: u32 = 80;
/// Dragon body/wing/tail tint (a deep ember red the multi-part renderer
/// tints its cubes with).
pub const DRAGON_BODY_LAYER: u32 = 81;
pub const DRAGON_EGG_LAYER: u32 = 82;
/// Step 11: when the neighbor on a face is the SAME block, that face
/// samples the edgeless variant so big surfaces stop gridding.
pub const STONE_CONN_LAYER: u32 = 83;
pub const PLANKS_CONN_LAYER: u32 = 84;

// Faction + biome + decoration block layers (lore-and-visuals C1).
// Vanilla block ids 68..=105 map to layers id+18 (86..=123).
pub const ACCORD_STONE_LAYER: u32 = 86;
pub const ACCORD_PILLAR_LAYER: u32 = 87;
pub const IRONBORN_BRICK_LAYER: u32 = 88;
pub const IRONBORN_GRATE_LAYER: u32 = 89;
pub const EMBER_COVENANTWOOD_LAYER: u32 = 90;
pub const EMBER_GLOWSTONE_LAYER: u32 = 91;
pub const FREEHOLDS_THATCH_LAYER: u32 = 92;
pub const FREEHOLDS_DAUB_LAYER: u32 = 93;
pub const ASHEN_MARBLE_LAYER: u32 = 94;
pub const ASHEN_BOOKSHELF_LAYER: u32 = 95;
pub const NAMELESS_ROTWOOD_LAYER: u32 = 96;
pub const NAMELESS_SCORCHED_LAYER: u32 = 97;
pub const MUSHROOM_CAP_LAYER: u32 = 98;
pub const CORAL_BLOCK_LAYER: u32 = 99;
pub const PERMAFROST_LAYER: u32 = 100;
pub const VOLCANIC_BASALT_LAYER: u32 = 101;
pub const DEEP_SLATE_LAYER: u32 = 102;
pub const MESA_TERRACOTTA_LAYER: u32 = 103;
pub const GILDED_GRASS_LAYER: u32 = 104;
pub const BOG_PEAT_LAYER: u32 = 105;

// Entity skins (C2): villager faction variants, companion skins (+ the
// trust-badge swap at trust >= 50), mob refresh, biome-tint variants.
pub const VILLAGER_ACCORD_LAYER: u32 = 124;
pub const VILLAGER_IRONBORN_LAYER: u32 = 125;
pub const VILLAGER_COVENANT_LAYER: u32 = 126;
pub const VILLAGER_FREEHOLDS_LAYER: u32 = 127;
pub const VILLAGER_ASHEN_LAYER: u32 = 128;
pub const VILLAGER_NAMELESS_LAYER: u32 = 129;
pub const VILLAGER_UNMARKED_LAYER: u32 = 130;
pub const VILLAGER_MAREN_LAYER: u32 = 131;
pub const COMPANION_LAYERS: [(&str, u32); 6] = [
    ("accord_warden", 132),
    ("ironborn_artisan", 133),
    ("covenant_channeler", 134),
    ("freeholds_scout", 135),
    ("ashen_scribe", 136),
    ("nameless_rover", 137),
];
/// The trust-badge variant of a companion layer (trust >= 50).
pub fn trusted_companion_layer(layer: u32) -> u32 {
    if (132..=137).contains(&layer) { layer + 6 } else { layer }
}
pub const MOB_BOAR_LAYER: u32 = 144;
pub const MOB_WOOLBEAST_LAYER: u32 = 145;
pub const MOB_GLITCHLING_LAYER: u32 = 146;
pub const MOB_STALKER_LAYER: u32 = 147;
pub const MOB_CRAWLER_LAYER: u32 = 148;
pub const MOB_NULL_KNIGHT_LAYER: u32 = 149;
/// (desert, snow, swamp) biome-tint layers per common hostile.
pub const MOB_GLITCHLING_TINTS: [u32; 3] = [150, 153, 156];
pub const MOB_STALKER_TINTS: [u32; 3] = [151, 154, 157];
pub const MOB_CRAWLER_TINTS: [u32; 3] = [152, 155, 158];
/// Ambient ember particle core (C4).
pub const EMBER_LAYER: u32 = 159;

/// The connected variant for a block's atlas layer, if it has one.
pub fn connected_variant(layer: u32) -> Option<u32> {
    match layer {
        0 => Some(STONE_CONN_LAYER),      // stone
        15 => Some(PLANKS_CONN_LAYER),    // planks
        _ => None,
    }
}

/// Texture atlas layer for a block id (see lf_voxel::BlockState / lf_worldgen::BlockId).
pub fn texture_index_for_block(block_id: u32) -> u32 {
    match block_id {
        1 => 0, // stone
        2 => 1, // grass
        3 => 2, // dirt
        4 => 3, // sand
        5 => 4, // mycelium
        42 => 54, // jungle grass
        43 => 55, // savanna grass
        44 => 56, // wildflower
        45 => 57, // water wheel
        46 => 58, // battery
        47 => 59, // pipe
        48 => 60, // boiler
        49 => 61, // steam engine
        50 => 62, // crude oil
        51 => 63, // pumpjack
        52 => 64, // refinery
        53 => 65, // combustion generator
        54 => 68, // uranium ore
        55 => 69, // reactor
        56 => 70, // radiation residue
        57 => 71, // enchanting table
        58 => 72, // lumen block
        59 => 73, // warding pylon
        60 => 74, // scaffolding
        61 => 75, // chiseled statue
        62 => 76, // conduit
        63 => 77, // elevator
        64 => 78, // climate unit
        65 => 79, // computer (its FACE shows the dynamic screen layer)
        66 => 82, // dragon egg
        67 => 85, // item belt
        6 => 5, // snow
        7 => 6, // log
        8 => 7, // leaves
        9 => 8, // coal ore
        10 => 9, // iron ore
        11 => 10, // water
        12 => 11, // torch
        13 => 40, // lantern
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
        // lore-and-visuals blocks: ids 68..=105 map consecutively to
        // layers 86..=123 (faction, biome, decoration)
        id @ 68..=105 => id + 18,
        // ui-world-craft: lava + surface decoration (explicit — id+18 would
        // collide with the villager skin layers)
        106 => TALL_GRASS_LAYER,
        107 => DRY_GRASS_LAYER,
        108 => CACTUS_LAYER,
        109 => DEAD_SHRUB_LAYER,
        110 => LAVA_LAYER,
        id if id >= 200 => 47, // mod blocks (registry::MOD_BLOCK_BASE)
        _ => 0,
    }
}

/// Atlas layers for the per-face materials and the crack decal (P26).
pub const GRASS_TOP_LAYER: u32 = 41;
pub const LOG_TOP_LAYER: u32 = 42;
pub const CRACK_LAYERS: [u32; 4] = [43, 44, 45, 46];

/// Per-face material mapping: which atlas layer a block's face uses. Blocks
/// without distinct faces fall through to `texture_index_for_block`.
pub fn texture_index_for_face(block_id: u32, face: lf_voxel::meshing::Face) -> u32 {
    use lf_voxel::meshing::Face;
    use lf_voxel::registry::block;
    let is_log = matches!(block_id, block::LOG | block::BIRCH_LOG | block::SPRUCE_LOG
        | block::DARK_LOG | block::CHERRY_LOG);
    match (block_id, face) {
        (block::GRASS, Face::Top) => GRASS_TOP_LAYER,
        (block::GRASS, Face::Bottom) => texture_index_for_block(block::DIRT),
        (id, Face::Top) | (id, Face::Bottom) if is_log => LOG_TOP_LAYER,
        // golden savanna grass: gold blades on top, plain dirt sides
        (block::GILDED_GRASS, Face::Top) => GILDED_GRASS_LAYER,
        (block::GILDED_GRASS, Face::Bottom) | (block::GILDED_GRASS, Face::Side) => {
            texture_index_for_block(block::DIRT)
        }
        // carved column: plain accord stone caps, fluted face
        (block::ACCORD_PILLAR, Face::Top) | (block::ACCORD_PILLAR, Face::Bottom) => {
            ACCORD_STONE_LAYER
        }
        // bookshelf: marble shelf caps
        (block::ASHEN_BOOKSHELF, Face::Top) | (block::ASHEN_BOOKSHELF, Face::Bottom) => {
            ASHEN_MARBLE_LAYER
        }
        _ => texture_index_for_block(block_id),
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
                // biome-identity grasses (Step 16): same construction as
                // grass but distinct palettes — jungle deep saturated green,
                // savanna dry gold
                "jungle_grass" => {
                    if y < 4 {
                        Rgba([40, 140, 50, 255])
                    } else {
                        let v = 70 + ((x * 7 + y * 5) % 15);
                        Rgba([ch(v), 60, 32, 255])
                    }
                }
                "savanna_grass" => {
                    if y < 4 {
                        Rgba([178, 168, 74, 255])
                    } else {
                        let v = 120 + ((x * 5 + y * 11) % 18);
                        Rgba([ch(v), 96, 42, 255])
                    }
                }
                // wildflower: cutout bloom — green stem pixels plus red
                // petals with transparent gaps (non-solid plant block)
                "flower" => {
                    let stem = x >= 7 && x <= 8 && y >= 8;
                    let petal = (x as i32 - 8).abs() <= 2 && (y as i32 - 6).abs() <= 2
                        && !((x as i32 - 8).abs() == 2 && (y as i32 - 6).abs() == 2);
                    let center = x == 7 && y >= 5 && y <= 6;
                    if petal {
                        Rgba([235, 70, 70, 255])
                    } else if center {
                        Rgba([250, 210, 90, 255])
                    } else if stem {
                        Rgba([60, 150, 60, 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                // Steam Age machines (P30): pipe = copper tube with flanges;
                // boiler = iron drum with a glowing fire door; engine =
                // iron block with piston + flywheel stripe
                "pipe" => {
                    let tube = (x >= 4 && x <= 11);
                    let flange = (x == 4 || x == 5 || x == 10 || x == 11) && y >= 4 && y <= 11;
                    let shine = tube && (y == 5 || y == 6);
                    if flange {
                        Rgba([150, 150, 158, 255])
                    } else if tube && y >= 4 && y <= 11 {
                        if shine { Rgba([235, 150, 90, 255]) } else { Rgba([198, 110, 62, 255]) }
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "boiler" => {
                    let shell = x >= 2 && x <= 13 && y >= 3 && y <= 14;
                    let door = x >= 6 && x <= 9 && y >= 9 && y <= 13;
                    let gauge = x >= 6 && x <= 9 && y >= 4 && y <= 6;
                    let hot = x >= 5 && x <= 10 && y >= 11 && y <= 13;
                    if gauge {
                        Rgba([90, 160, 210, 255]) // water gauge
                    } else if door && hot {
                        Rgba([250, 150, 60, 255]) // fire door glow
                    } else if door {
                        Rgba([70, 60, 55, 255])
                    } else if shell {
                        let v = 140 + ((x * 5 + y * 7) % 20);
                        Rgba([ch(v), ch(v - 6), ch(v - 20), 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "steam_engine" => {
                    let body = x >= 2 && x <= 13 && y >= 5 && y <= 14;
                    let piston = x >= 5 && x <= 10 && y >= 3 && y <= 6;
                    let flywheel = (x as i32 - 11).pow(2) + (y as i32 - 10).pow(2) <= 9 && x >= 8;
                    if piston {
                        Rgba([220, 200, 160, 255])
                    } else if flywheel {
                        Rgba([110, 110, 120, 255])
                    } else if body {
                        let v = 130 + ((x * 3 + y * 9) % 24);
                        Rgba([ch(v), ch(v - 4), ch(v - 16), 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                // Oil Age (P31)
                "oil" => {
                    // full-tile glossy crude: near-black with brown swirls
                    // and a light-streak highlight, mirroring water's style
                    let swirl = ((x * 3 + y * 5) % 11 < 3) || ((x * 7 + y * 2) % 13 < 2);
                    let sheen = (x + y) % 9 == 2 && x > 3 && x < 12;
                    if sheen {
                        Rgba([88, 74, 48, 255])
                    } else if swirl {
                        Rgba([30, 24, 16, 255])
                    } else {
                        Rgba([16, 13, 9, 255])
                    }
                }
                "pump" => {
                    // derrick: steel lattice tower with a walking beam
                    let lattice = x >= 3 && x <= 12 && y >= 2 && y <= 13
                        && (x == 3 || x == 12 || y == 13
                            || (x + y) % 9 == 0 || ((x as i32) - (y as i32)).rem_euclid(9) == 0);
                    let beam = y >= 5 && y <= 7 && x >= 5 && x <= 11;
                    let head = (x as i32 - 5).abs() <= 1 && y <= 4 && y >= 2;
                    if beam {
                        Rgba([210, 160, 70, 255])
                    } else if head {
                        Rgba([180, 130, 60, 255])
                    } else if lattice {
                        let v = 120 + ((x * 5 + y * 11) % 18);
                        Rgba([ch(v - 20), ch(v - 22), ch(v - 16), 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "refinery" => {
                    // two fractioning columns + pipe run + flare stub
                    let col_a = x >= 3 && x <= 5 && y >= 2 && y <= 13;
                    let col_b = x >= 8 && x <= 10 && y >= 4 && y <= 13;
                    let pipes = y >= 10 && y <= 11 && x >= 3 && x <= 13;
                    let flare = x >= 12 && x <= 13 && y >= 1 && y <= 4;
                    if flare {
                        Rgba([250, 170, 60, 255])
                    } else if col_a || col_b {
                        let v = 140 + ((x * 9 + y * 3) % 20);
                        Rgba([ch(v), ch(v - 6), ch(v - 18), 255])
                    } else if pipes {
                        Rgba([198, 110, 62, 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "combustion_generator" => {
                    // engine block with exhaust stack and hot glow grille
                    let body = x >= 2 && x <= 13 && y >= 6 && y <= 14;
                    let stack = x >= 10 && x <= 12 && y >= 2 && y <= 6;
                    let grille = body && y >= 9 && y <= 12 && x >= 4 && x <= 7;
                    if stack {
                        Rgba([120, 120, 128, 255])
                    } else if grille {
                        Rgba([240, 130, 50, 255])
                    } else if body {
                        let v = 135 + ((x * 7 + y * 5) % 22);
                        Rgba([ch(v), ch(v - 8), ch(v - 20), 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "grid_ok" => {
                    // power-grid overlay tint (transparent pass): opaque
                    // wireframe cage over a translucent green fill
                    if x % 4 == 0 || y % 4 == 0 {
                        Rgba([120, 255, 150, 255])
                    } else {
                        Rgba([90, 230, 120, 60])
                    }
                }
                "grid_starved" => {
                    if x % 4 == 0 || y % 4 == 0 {
                        Rgba([255, 110, 100, 255])
                    } else {
                        Rgba([230, 70, 60, 60])
                    }
                }
                // Nuclear tier (P32)
                "uranium_ore" => {
                    // stone matrix with glowing green flecks
                    let fleck = (x * 7 + y * 13) % 23 < 3;
                    if fleck {
                        Rgba([120, 240, 90, 255])
                    } else {
                        let v = 110 + ((x * 5 + y * 7) % 18);
                        Rgba([ch(v), ch(v - 2), ch(v - 4), 255])
                    }
                }
                "reactor" => {
                    // heavy containment vessel: thick steel ring + core window
                    let shell = x >= 1 && x <= 14 && y >= 1 && y <= 14;
                    let ring = (x as i32 - 7).pow(2) + (y as i32 - 7).pow(2);
                    let window = ring <= 9;
                    let bolts = ring >= 36 && ring <= 49 && (x + y) % 3 == 0;
                    if window {
                        // the core glows through the water
                        let pulse = (x * 3 + y * 5) % 7 < 3;
                        if pulse { Rgba([140, 240, 190, 255]) } else { Rgba([40, 150, 110, 255]) }
                    } else if bolts {
                        Rgba([235, 235, 190, 255])
                    } else if shell {
                        let v = 118 + ((x * 11 + y * 3) % 16);
                        Rgba([ch(v), ch(v - 4), ch(v - 12), 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "radiation" => {
                    // sickly glowing residue crust
                    let glow = (x * 5 + y * 3) % 17 < 4;
                    let crust = (x * 3 + y * 9) % 13 < 5;
                    if glow {
                        Rgba([170, 255, 120, 255])
                    } else if crust {
                        Rgba([70, 120, 60, 255])
                    } else {
                        Rgba([34, 60, 34, 255])
                    }
                }
                // Magic foundation (P33)
                "enchanting_table" => {
                    // dark table with a floating rune diamond + corner studs
                    let top = y >= 2 && y <= 6 && x >= 2 && x <= 13;
                    let cloth = y >= 7 && y <= 13 && x >= 3 && x <= 12;
                    let rune = (x as i32 - 8).abs() <= 1 && (y as i32 - 9).abs() <= 1;
                    let stud = (x < 3 || x > 12) && (y < 3 || y > 12) && (x + y) % 2 == 0;
                    if rune {
                        Rgba([160, 120, 240, 255])
                    } else if top {
                        let v = 90 + ((x * 7 + y * 5) % 14);
                        Rgba([ch(v), ch(v - 12), ch(v - 26), 255])
                    } else if cloth {
                        let v = 40 + ((x * 3 + y * 11) % 12);
                        Rgba([ch(v), ch(v - 8), ch(v + 30), 255])
                    } else if stud {
                        Rgba([220, 200, 120, 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "lumen_block" => {
                    // warm fuelless light: bright core, pale casing
                    let core = (x as i32 - 8).pow(2) + (y as i32 - 8).pow(2) <= 16;
                    let ring = (x as i32 - 8).pow(2) + (y as i32 - 8).pow(2) <= 42;
                    if core {
                        Rgba([255, 244, 200, 255])
                    } else if ring {
                        Rgba([230, 214, 168, 255])
                    } else {
                        Rgba([120, 108, 84, 255])
                    }
                }
                "conduit" => {
                    // slim cable with glow pulses: two vertical rails +
                    // cross ties that read as energy flow
                    let rail = x >= 6 && x <= 9 && y >= 1 && y <= 14;
                    let tie = rail && (y % 3 == 1);
                    let pulse = rail && ((x + y * 2) % 7 < 2);
                    if pulse {
                        Rgba([255, 230, 140, 255])
                    } else if tie {
                        Rgba([210, 170, 80, 255])
                    } else if rail {
                        Rgba([90, 80, 70, 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "elevator" => {
                    // platform pad with a lit rim + arrows
                    let pad = x >= 2 && x <= 13 && y >= 10 && y <= 13;
                    let rim = (x == 2 || x == 13 || y == 10) && x >= 2 && x <= 13 && y <= 13;
                    let up_arrow = (x as i32 - 8).abs() + (3 - y as i32).abs() <= 2 && y <= 6;
                    let down_arrow = (x as i32 - 8).abs() + (y as i32 - 9).abs() <= 2 && y >= 7 && y <= 9;
                    if up_arrow || down_arrow {
                        Rgba([140, 230, 235, 255])
                    } else if rim {
                        Rgba([90, 220, 230, 255])
                    } else if pad {
                        let v = 120 + ((x * 7 + y * 3) % 20);
                        Rgba([ch(v), ch(v - 6), ch(v - 16), 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "ac_unit" => {
                    // vented box: slats + a small status LED
                    let box_ = x >= 2 && x <= 13 && y >= 4 && y <= 14;
                    let slat = box_ && (y % 2 == 0) && x >= 4 && x <= 11;
                    let led = x >= 11 && x <= 12 && y >= 5 && y <= 6;
                    if led {
                        Rgba([120, 240, 160, 255])
                    } else if slat {
                        Rgba([60, 62, 70, 255])
                    } else if box_ {
                        let v = 130 + ((x * 5 + y * 7) % 18);
                        Rgba([ch(v), ch(v), ch(v + 10), 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "computer" => {
                    // monitor frame around the dynamic screen area; the
                    // center pixels come from the SCREEN layer at render
                    // time (this texture is the case/stand)
                    let frame = x >= 2 && x <= 13 && y >= 2 && y <= 12;
                    let inner = x >= 4 && x <= 11 && y >= 4 && y <= 10;
                    let stand = x >= 7 && x <= 8 && y >= 13 && y <= 14;
                    if stand {
                        Rgba([90, 90, 100, 255])
                    } else if inner {
                        Rgba([10, 14, 20, 255]) // behind the dynamic face
                    } else if frame {
                        let v = 110 + ((x * 3 + y * 9) % 16);
                        Rgba([ch(v), ch(v - 6), ch(v - 14), 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "dragon_scale_block" => {
                    // ember-red scales: overlapping rows with dark ridges
                    let ridge = (x + y * 3) % 7 < 2;
                    let scale = (x / 2 + y / 2) % 2 == 0;
                    if ridge {
                        Rgba([96, 26, 20, 255])
                    } else if scale {
                        Rgba([176, 46, 34, 255])
                    } else {
                        Rgba([136, 34, 26, 255])
                    }
                }
                "dragon_egg" => {
                    // dark scaled ovoid with ember cracks
                    let dx = (x as i32 - 8).pow(2) * 4 / 9 + (y as i32 - 8).pow(2);
                    let shell = dx <= 42;
                    let crack = (x * 3 + y * 7) % 23 < 3;
                    if shell && crack {
                        Rgba([255, 140, 60, 255])
                    } else if shell {
                        Rgba([44, 22, 20, 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "belt" => {
                    // angled rollers + a moving-load stripe
                    let frame = (x >= 2 && x <= 13) && (y >= 6 && y <= 9);
                    let roller = frame && (x % 3 == 0);
                    let load = frame && (y == 7) && ((x + 2) % 5 < 2);
                    if load {
                        Rgba([240, 200, 90, 255])
                    } else if roller {
                        Rgba([90, 90, 100, 255])
                    } else if frame {
                        Rgba([140, 140, 150, 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "stone_conn" => {
                    let v = 112 + ((x * 5 + y * 7) % 18);
                    Rgba([ch(v), ch(v - 2), ch(v - 4), 255])
                }
                "planks_conn" => {
                    let grain = ((y * 3 + (x / 4) * 5) % 9) < 2;
                    let v = if grain { 148 } else { 172 };
                    Rgba([ch(v), ch(v - 46), ch(v - 92), 255])
                }
                "screen" => {
                    // default face: dark glass with a faint scanline (the
                    // client rewrites these pixels per page)
                    let scan = (y % 3) == 0;
                    if scan {
                        Rgba([26, 36, 48, 255])
                    } else {
                        Rgba([14, 20, 28, 255])
                    }
                }
                "scaffold" => {
                    // ladder-tower: two uprights + rungs + a top platform
                    let upright = (x == 4 || x == 5 || x == 10 || x == 11) && y >= 2 && y <= 13;
                    let rung = x >= 4 && x <= 11 && y >= 3 && y <= 13 && (y % 3 == 0);
                    let platform = y >= 12 && y <= 13 && x >= 3 && x <= 12;
                    if platform {
                        Rgba([ch(150 + ((x * 5 + y * 7) % 18)), ch(118), ch(70), 255])
                    } else if upright {
                        Rgba([128, 92, 52, 255])
                    } else if rung {
                        Rgba([168, 126, 76, 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "statue" => {
                    // a chiseled figure on a plinth (front view)
                    let plinth = y >= 12 && y <= 14 && x >= 3 && x <= 12;
                    let body = x >= 6 && x <= 9 && y >= 5 && y <= 11;
                    let head = (x as i32 - 7).pow(2) + (y as i32 - 3).pow(2) <= 3;
                    let arm_l = y >= 6 && y <= 7 && x == 5;
                    let arm_r = y >= 6 && y <= 7 && x == 10;
                    if head {
                        Rgba([205, 200, 192, 255])
                    } else if body || arm_l || arm_r {
                        let v = 160 + ((x * 7 + y * 3) % 20);
                        Rgba([ch(v), ch(v - 8), ch(v - 20), 255])
                    } else if plinth {
                        let v = 120 + ((x * 11 + y * 5) % 16);
                        Rgba([ch(v), ch(v - 4), ch(v - 14), 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "warding_pylon" => {
                    // obsidian-ish pillar with a cyan ward band
                    let pillar = x >= 5 && x <= 10 && y >= 2 && y <= 14;
                    let band = pillar && y >= 6 && y <= 8;
                    let cap = y >= 0 && y <= 1 && x >= 6 && x <= 9;
                    if band {
                        Rgba([90, 230, 235, 255])
                    } else if cap {
                        Rgba([70, 220, 225, 255])
                    } else if pillar {
                        let v = 38 + ((x * 5 + y * 7) % 10);
                        Rgba([ch(v), ch(v - 4), ch(v + 12), 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                // Water Age machines (P29)
                "water_wheel" => {
                    let dx = (x as i32 - 8).abs();
                    let dy = (y as i32 - 8).abs();
                    let ring = (dx * dx + dy * dy) >= 9 && (dx * dx + dy * dy) <= 64;
                    let spoke = dx <= 1 || dy <= 1;
                    let axle = dx <= 2 && dy <= 2;
                    if axle {
                        Rgba([120, 84, 46, 255])
                    } else if ring && spoke {
                        Rgba([92, 62, 34, 255])
                    } else if ring {
                        // paddles: alternating planks with gaps that read as slats
                        let slat = (x + y) % 5 < 3;
                        let v = if slat { 168 } else { 120 };
                        Rgba([ch(v), ch(v - 40), ch(v - 90), 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "battery" => {
                    let shell = x >= 2 && x <= 13 && y >= 4 && y <= 13;
                    let terminal = (x >= 4 && x <= 6 || x >= 9 && x <= 11) && y >= 2 && y <= 4;
                    let stripe = shell && y >= 6 && y <= 7;
                    if terminal {
                        Rgba([210, 120, 60, 255]) // copper tops
                    } else if stripe {
                        Rgba([240, 200, 90, 255]) // charge stripe
                    } else if shell {
                        let v = 150 + ((x * 7 + y * 3) % 22);
                        Rgba([ch(v), ch(v), ch(v + 8), 255])
                    } else {
                        Rgba([0, 0, 0, 0])
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
                "lantern" => {
                    // iron-framed lantern with a glowing core; the emissive
                    // look matches its light level 15 (lf_voxel light emitters)
                    let border = x <= 1 || x >= 14 || y <= 2 || y >= 13;
                    let bar = y == 6 || y == 7 || x == 5 || x == 10;
                    if border {
                        let v = 60 + ((x * 3 + y * 7) % 15);
                        Rgba([ch(v), ch(v), ch(v + 12), 255]) // dark iron frame
                    } else if bar {
                        Rgba([85, 85, 95, 255]) // iron cross-bars
                    } else {
                        let glow = 190 + ((x * 5 + y * 3) % 50);
                        Rgba([255, ch(glow), ch(glow / 2), 255]) // warm bright core
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
                // waypoint beacon columns (Step 15): solid translucent tint
                // with a soft vertical falloff, one per waypoint color
                "waypoint_0" => beacon_pixel(x, y, [235, 80, 80]),
                "waypoint_1" => beacon_pixel(x, y, [90, 170, 240]),
                "waypoint_2" => beacon_pixel(x, y, [120, 220, 130]),
                "waypoint_3" => beacon_pixel(x, y, [240, 200, 90]),
                "waypoint_4" => beacon_pixel(x, y, [200, 120, 235]),
                "waypoint_5" => beacon_pixel(x, y, [235, 140, 200]),
                "water" => {
                    let v = 40 + ((x * 3 + y * 5) % 14);
                    Rgba([30, ch(60 + v / 2), ch(150 + v / 3), 170])
                }
                "grass_top" => {
                    // full green with a mottled, slightly clumpy lawn look
                    let v = ((x * 5 + y * 3) % 7) + ((x * y) % 5);
                    Rgba([ch(52 + v * 3), ch(128 + v * 4), ch(40 + v * 2), 255])
                }
                "log_top" => {
                    // growth rings: concentric squares around the center
                    let d = ((x as i32 - 8).abs()).max((y as i32 - 8).abs());
                    let ring = if d % 2 == 0 { 176 } else { 120 };
                    let v = ring + ((x * 3 + y * 7) % 12);
                    Rgba([ch(v), ch((v * 3) / 4), ch(v / 2), 255])
                }
                // ---- faction blocks (lore-and-visuals C1) ----------------
                "accord_stone" => {
                    // smooth stone with a faint corner-to-corner inlay groove
                    let groove = x == y || x + y == 15;
                    let v = 130 + ((x * 7 + y * 13) % 12) as i32 - if groove { 22 } else { 0 };
                    Rgba([ch(v as u32), ch((v + 4) as u32), ch((v + 10) as u32), 255])
                }
                "accord_pillar" => {
                    // fluted column face — vertical grooves, lighter edges
                    let flute = matches!(x, 2 | 5 | 8 | 11 | 14);
                    let edge = x <= 1 || x >= 14;
                    let base = if flute { 112 } else if edge { 152 } else { 134 };
                    let v = base + ((x * 3 + y * 11) % 9) as i32;
                    Rgba([ch(v as u32), ch((v + 4) as u32), ch((v + 10) as u32), 255])
                }
                "ironborn_brick" => {
                    let row = y / 4;
                    let joint = y % 4 == 0 || (x + if row % 2 == 1 { 4 } else { 0 }) % 8 == 0;
                    let fleck = pixel_hash(x, y, "ironborn") % 23 == 0;
                    if joint {
                        Rgba([58, 50, 44, 255])
                    } else if fleck {
                        Rgba([150, 138, 140, 255])
                    } else {
                        let v = (x * 5 + y * 9 + row * 17) % 10;
                        Rgba([ch(88 + v), ch(62 + v), ch(44 + v), 255])
                    }
                }
                "ironborn_grate" => {
                    // iron frame with punched round holes (translucent pass)
                    let hole = [(3u32, 3u32), (3, 11), (11, 3), (11, 11), (7, 7)]
                        .iter()
                        .any(|(cx, cy)| (x as i32 - *cx as i32).abs() <= 1 && (y as i32 - *cy as i32).abs() <= 1);
                    if hole {
                        Rgba([0, 0, 0, 0])
                    } else if x <= 1 || x >= 14 || y <= 1 || y >= 14 {
                        Rgba([72, 64, 58, 255])
                    } else {
                        Rgba([92, 82, 74, 170])
                    }
                }
                "ember_covenantwood" => {
                    let plank = y % 5 == 0;
                    let rune = matches!((x, y), (4, 4) | (5, 4) | (4, 6) | (11, 3)
                        | (11, 5) | (12, 4) | (6, 11) | (8, 11) | (7, 12));
                    if rune {
                        Rgba([168, 78, 32, 255])
                    } else if plank {
                        Rgba([24, 20, 17, 255])
                    } else {
                        let v = (x * 7 + y * 5) % 7;
                        Rgba([ch(40 + v), ch(33 + v), ch(28 + v), 255])
                    }
                }
                "ember_glowstone" => {
                    // muted amber, glow-flecked (emits light 8)
                    let v = (x * 3 + y * 7) % 13;
                    let px = if v < 3 { [242, 192, 92] } else if v > 10 { [172, 110, 44] } else { [204, 142, 62] };
                    Rgba([px[0], px[1], px[2], 255])
                }
                "freeholds_thatch" => {
                    let weave = (x + y) % 4 < 2;
                    let v = (x * 5 + y * 3) % 9;
                    if weave {
                        Rgba([ch(172 + v), ch(142 + v), ch(74 + v), 255])
                    } else {
                        Rgba([ch(202 + v), ch(174 + v), ch(100 + v), 255])
                    }
                }
                "freeholds_daub" => {
                    let v = (x * 5 + y * 3) % 11;
                    let blotch = pixel_hash(x, y, "daub") % 17 == 0;
                    if blotch {
                        Rgba([198, 190, 170, 255])
                    } else {
                        Rgba([ch(220 + v / 2), ch(212 + v / 2), ch(194 + v / 2), 255])
                    }
                }
                "ashen_marble" => {
                    let vein = (x as i32 - (y as i32 * 2)).rem_euclid(16) == 0
                        || (x as i32 + (y as i32 * 3)).rem_euclid(16) == 15;
                    let v = (x * 7 + y * 5) % 8;
                    let base = if vein { 150 } else { 204 + v };
                    Rgba([ch(base as u32), ch((base + 2) as u32), ch((base + 4) as u32), 255])
                }
                "ashen_bookshelf" => {
                    let shelf = y % 5 == 4;
                    if shelf {
                        Rgba([188, 190, 194, 255])
                    } else {
                        // book spines: grey / off-white / dark blue / dark red
                        let spine = (x / 3) % 4;
                        let gap = x % 3 == 2;
                        let c = match spine {
                            0 => [118, 120, 126],
                            1 => [208, 204, 196],
                            2 => [58, 68, 110],
                            _ => [112, 52, 52],
                        };
                        if gap { Rgba([84, 84, 88, 255]) } else { Rgba([c[0], c[1], c[2], 255]) }
                    }
                }
                "nameless_rotwood" => {
                    let rot = pixel_hash(x, y, "rotwood") % 13 < 3;
                    let crack = (x + y * 3) % 17 < 1;
                    if crack {
                        Rgba([26, 22, 18, 255])
                    } else if rot {
                        Rgba([54, 46, 38, 255])
                    } else {
                        let v = (x * 3 + y * 11) % 8;
                        Rgba([ch(84 + v), ch(72 + v), ch(58 + v), 255])
                    }
                }
                "nameless_scorched" => {
                    let crack = (x * 7 + y * 5) % 29 < 1;
                    let edge = x <= 1 || x >= 14 || y <= 1 || y >= 14;
                    if crack {
                        Rgba([148, 74, 30, 255])
                    } else if edge {
                        Rgba([88, 84, 82, 255])
                    } else {
                        let v = (x * 5 + y * 9) % 9;
                        Rgba([ch(52 + v), ch(50 + v), ch(48 + v), 255])
                    }
                }
                // ---- biome-exclusive blocks ------------------------------
                "mushroom_cap" => {
                    let spot = [(4u32, 4u32), (12, 7), (7, 12), (13, 13)]
                        .iter()
                        .any(|(cx, cy)| (x as i32 - *cx as i32).abs() <= 1 && (y as i32 - *cy as i32).abs() <= 1);
                    let v = (x * 5 + y * 3) % 10;
                    if spot {
                        Rgba([240, 235, 225, 255])
                    } else {
                        Rgba([ch(198 + v), ch(48 + v / 2), ch(38 + v / 2), 255])
                    }
                }
                "coral_block" => {
                    let mottle = pixel_hash(x, y, "coral") % 7;
                    let px = match mottle {
                        0 | 1 => [216, 104, 84],
                        2 => [246, 162, 122],
                        _ => [235, 132, 100],
                    };
                    Rgba([px[0], px[1], px[2], 255])
                }
                "permafrost" => {
                    let ice = pixel_hash(x, y, "frost") % 11 < 2;
                    let v = (x * 3 + y * 7) % 9;
                    if ice {
                        Rgba([182, 212, 238, 255])
                    } else {
                        Rgba([ch(92 + v), ch(108 + v), ch(128 + v), 255])
                    }
                }
                "volcanic_basalt" => {
                    let heat = (x * 3 + y * 5) % 19 < 1;
                    let v = (x * 7 + y * 3) % 7;
                    if heat {
                        Rgba([206, 92, 38, 255])
                    } else {
                        Rgba([ch(42 + v), ch(40 + v), ch(42 + v), 255])
                    }
                }
                "deep_slate" => {
                    let strata = y % 6 == 0;
                    let v = (x * 5 + y * 11) % 6;
                    let base = if strata { 32 } else { 38 + v };
                    Rgba([ch(base as u32), ch((base + 4) as u32), ch((base + 16) as u32), 255])
                }
                "mesa_terracotta" => {
                    let band = (y / 4) as usize;
                    let palettes = [[214, 120, 70], [196, 96, 60], [224, 140, 88], [182, 82, 54]];
                    let p = palettes[band % 4];
                    let v = (x * 3 + y * 5) % 8;
                    Rgba([ch(p[0] as u32 + v), ch(p[1] as u32 + v), ch(p[2] as u32 + v), 255])
                }
                "gilded_grass" => {
                    // golden dry blades (top face of the savanna grass)
                    let blade = x % 2 == 0;
                    let v = (x * 5 + y * 3) % 8;
                    if (x * 7 + y * 11) % 23 < 1 {
                        Rgba([218, 196, 112, 255])
                    } else if blade {
                        Rgba([ch(188 + v), ch(168 + v), ch(72 + v), 255])
                    } else {
                        Rgba([ch(168 + v), ch(148 + v), ch(58 + v), 255])
                    }
                }
                "bog_peat" => {
                    let root = (x * 5 + y * 3) % 13 < 1;
                    let wet = pixel_hash(x, y, "peat") % 19 < 1;
                    let v = (x * 7 + y * 5) % 7;
                    if root {
                        Rgba([72, 58, 40, 255])
                    } else if wet {
                        Rgba([64, 56, 44, 255])
                    } else {
                        Rgba([ch(46 + v), ch(37 + v), ch(27 + v), 255])
                    }
                }
                // ---- decoration blocks -----------------------------------
                "carved_oak" => {
                    let relief = (x as i32 - 8).abs() + (y as i32 - 8).abs();
                    let groove = relief == 4 || relief == 5;
                    let v = (x * 5 + y * 9) % 9;
                    if groove {
                        Rgba([122, 94, 58, 255])
                    } else {
                        Rgba([ch(170 + v), ch(134 + v), ch(86 + v), 255])
                    }
                }
                "carved_stone" => {
                    let relief = (x as i32 - 8).abs() + (y as i32 - 8).abs();
                    let v = (x * 7 + y * 5) % 10;
                    if relief == 5 {
                        Rgba([96, 98, 104, 255])
                    } else if relief == 6 {
                        Rgba([152, 154, 160, 255])
                    } else {
                        Rgba([ch(122 + v), ch(124 + v), ch(128 + v), 255])
                    }
                }
                "carved_iron" => {
                    let relief = x as i32 == y as i32;
                    let dent = pixel_hash(x, y, "carvediron") % 13;
                    let base = 138 + (dent % 4) * 4;
                    if relief {
                        Rgba([108, 104, 110, 255])
                    } else {
                        Rgba([ch(base as u32), ch((base - 2) as u32), ch((base + 2) as u32), 255])
                    }
                }
                "lantern_hanging" => {
                    // shared lantern art + the chain hook at the top
                    if (x == 7 || x == 8) && y < 4 {
                        Rgba([96, 92, 88, 255])
                    } else if x <= 2 || x >= 13 || y <= 3 || y >= 13 {
                        Rgba([88, 78, 60, 255])
                    } else {
                        let v = (x * 5 + y * 7) % 12;
                        Rgba([ch(250), ch(210 + v), ch(120 + v), 255])
                    }
                }
                name if name.starts_with("stained_glass_") => {
                    let tint = stained_glass_tint(name);
                    if x <= 1 || x >= 14 || y <= 1 || y >= 14 {
                        Rgba([ch(tint[0] / 2 + 40), ch(tint[1] / 2 + 40), ch(tint[2] / 2 + 40), 210])
                    } else {
                        Rgba([tint[0] as u8, tint[1] as u8, tint[2] as u8, 140])
                    }
                }
                name if name.starts_with("banner_") => banner_pixel(x, y, name),
                // ---- entity skins (C2) ------------------------------------
                name if name.starts_with("villager_") => villager_pixel(x, y, name),
                name if name.starts_with("companion_") => companion_pixel(x, y, name),
                name if name.starts_with("mob_") => mob_pixel(x, y, name),
                "ember" => {
                    // solid amber particle core with a bright center
                    let d = ((x as i32 - 8).abs() + (y as i32 - 8).abs()) as u32;
                    if d <= 3 {
                        Rgba([255, 214, 122, 255])
                    } else {
                        Rgba([228, 148, 58, 255])
                    }
                }
                // ui-world-craft E3: ground-cover plants are cutout sprites —
                // grass blades with transparent gaps, like the wildflower
                "tall_grass" => {
                    let blade = |bx: u32, h: u32| x >= bx && x < bx + 1 && y >= 16 - h;
                    let n = (x * 13 + y * 7) % 5;
                    if blade(2, 6) || blade(5, 9) || blade(8, 12) || blade(11, 8) || blade(14, 5)
                        || (x == 3 && n < 2 && y >= 12) || (x == 12 && n < 3 && y >= 11)
                    {
                        let v = 90 + ((x * 7 + y * 11) % 30);
                        Rgba([ch(v - 20), ch(v + 40), 50, 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "dry_grass" => {
                    let blade = |bx: u32, h: u32| x >= bx && x < bx + 1 && y >= 16 - h;
                    if blade(3, 7) || blade(7, 10) || blade(10, 6) || blade(13, 8) {
                        let v = 150 + ((x * 5 + y * 9) % 40);
                        Rgba([ch(v + 30), ch(v), 70, 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "dead_shrub" => {
                    // bare branches: a trunk stub with two offshoots
                    let trunk = x >= 7 && x <= 8 && y >= 6;
                    let arm_l = x >= 4 && x <= 6 && y >= 7 && y <= 8;
                    let arm_r = x >= 9 && x <= 12 && y >= 9 && y <= 10;
                    if trunk || arm_l || arm_r {
                        let v = 90 + ((x * 3 + y * 5) % 25);
                        Rgba([ch(v), ch(v - 20), 40, 255])
                    } else {
                        Rgba([0, 0, 0, 0])
                    }
                }
                "cactus" => {
                    // ribbed green column (solid block: fully opaque), pale
                    // spine dots down the ribs
                    let spine = (x == 5 || x == 10) && y % 5 == 2;
                    let rib = (x + y) % 4 == 0;
                    let edge = x < 2 || x > 13;
                    if spine {
                        Rgba([230, 230, 190, 255])
                    } else if edge {
                        Rgba([36, 80, 36, 255])
                    } else if rib {
                        Rgba([44, 98, 44, 255])
                    } else {
                        let v = 58 + ((x * 3 + y * 5) % 12);
                        Rgba([ch(v), ch(v + 66), ch(v - 4), 255])
                    }
                }
                "lava" => {
                    // slow convection: orange crust with bright cracks
                    let v = (x * x * 3 + y * y * 7) % 23;
                    if v < 4 {
                        Rgba([255, 216, 120, 255])
                    } else if v < 10 {
                        Rgba([240, 130, 40, 255])
                    } else {
                        Rgba([196, 70, 18, 255])
                    }
                }
                name if name.starts_with("crack_") => {
                    // progressive mining cracks on a transparent decal
                    let stage: u32 = name[6..].parse().unwrap_or(0);
                    crack_pixel(x, y, stage)
                }
                _ => {
                    let v = ((x + y) * 8) % 256;
                    Rgba([ch(v), ch(255 - v), 128, 255])
                }
            };
            let mut px = color;
            if is_leaf_name(name) {
                // alpha-cutout foliage: deterministic holes, density varies
                // per species via the name hash
                let h = pixel_hash(x, y, name);
                let density = 18 + (fnv1a(name) % 12) as u32; // 18..29 %
                if h % 100 < density {
                    px.0[3] = 0;
                }
            }
            img.put_pixel(x, y, px);
        }
    }
    img
}

/// Foliage textures punched full of cutout holes.
fn is_leaf_name(name: &str) -> bool {
    name == "leaves" || name.ends_with("_leaves")
}

fn fnv1a(s: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

fn pixel_hash(x: u32, y: u32, name: &str) -> u32 {
    let mut h = fnv1a(name);
    h ^= (x.wrapping_mul(73856093)) ^ (y.wrapping_mul(19349663));
    h = h.wrapping_mul(0x9E3779B1);
    h ^ (h >> 13)
}

/// One pixel of the stage-N crack decal: dark crooked lines radiating from
/// the block center; more/longer cracks as the stage grows.
fn crack_pixel(x: u32, y: u32, stage: u32) -> Rgba<u8> {
    let mut on = false;
    let cracks = 2 + stage;
    for c in 0..cracks {
        // deterministic walk from the center outward
        let mut px = 8i32;
        let mut py = 8i32;
        let mut dir = (c as i32) * 3 + ((stage as i32 * 5) % 7);
        let len = 4 + stage * 3 + (c % 2) as u32;
        for _ in 0..len {
            // step with a jittered direction so cracks look jagged
            let jitter = pixel_hash(px as u32, py as u32, "crack") % 7;
            match (dir + jitter as i32) % 4 {
                0 => px += 1,
                1 => px -= 1,
                2 => py += 1,
                _ => py -= 1,
            }
            if px < 0 || px > 15 || py < 0 || py > 15 { break; }
            if px == x as i32 && py == y as i32 {
                on = true;
            }
        }
    }
    if on {
        let v = 22 + ((x * 11 + y * 17) % 18) as u32;
        Rgba([ch(v), ch(v), ch(v + 6), 255])
    } else {
        Rgba([0, 0, 0, 0])
    }
}

/// The full atlas, one 16x16 layer per entry in TEXTURE_NAMES.
/// One beacon-layer pixel: mostly transparent so stacked quads glow, with
/// a bright core column and an alpha falloff toward the top of the tile
/// (the mesh tiles it vertically; higher repeats read fainter).
fn beacon_pixel(x: u32, y: u32, rgb: [u32; 3]) -> Rgba<u8> {
    let (x, y) = (x as usize, y as usize);
    let dx = (x as i32 - 8).abs();
    let core = dx <= 2;
    let edge = dx <= 4;
    // fade out toward the tile's top so tall beams taper
    let top_fade = 1.0 - (y as f32 / 16.0) * 0.55;
    let base = if core { 255 } else if edge { 90 } else { 20 };
    let alpha = base as f32 * top_fade;
    Rgba([rgb[0] as u8, rgb[1] as u8, rgb[2] as u8, alpha as u8])
}

// ------------------------------------------------------------------
// lore-and-visuals C1/C2 pixel helpers

fn stained_glass_tint(name: &str) -> [u32; 3] {
    match name.strip_prefix("stained_glass_").unwrap_or(name) {
        "red" => [200, 40, 40],
        "orange" => [230, 130, 30],
        "yellow" => [230, 210, 50],
        "green" => [60, 180, 70],
        "blue" => [60, 110, 210],
        "purple" => [150, 70, 200],
        "black" => [30, 30, 35],
        _ => [235, 235, 235], // white
    }
}

/// Faction banner: cutout cloth (faction color + symbol) on a pole, drawn
/// flat like a sign (cross-plant render path).
fn banner_pixel(x: u32, y: u32, name: &str) -> Rgba<u8> {
    let (color, symbol, symbol_color): ([u32; 3], &str, [u32; 3]) = match name {
        "banner_accord" => ([74, 122, 181], "scale", [240, 244, 250]),
        "banner_ironborn" => ([139, 69, 19], "hammer", [235, 228, 220]),
        "banner_covenant" => ([196, 96, 42], "flame", [52, 38, 32]),
        "banner_freeholds" => ([107, 142, 35], "wheat", [240, 232, 200]),
        "banner_ashen" => ([176, 176, 176], "book", [62, 66, 74]),
        _ => ([45, 45, 45], "chain", [140, 140, 140]), // nameless
    };
    // pole at the left, cloth 4..=12 x 2..=13 with a swallowtail cut
    if x == 3 {
        return Rgba([104, 82, 52, 255]);
    }
    if x == 2 && y == 1 {
        return Rgba([104, 82, 52, 255]); // finial
    }
    let in_cloth = (4..=12).contains(&x) && (2..=13).contains(&y)
        && !(y == 13 && x % 2 == 1); // zigzag fly edge
    if !in_cloth {
        return Rgba([0, 0, 0, 0]);
    }
    // symbol glyphs in the cloth middle (6..=10 x 5..=10)
    let sym = match symbol {
        "scale" => (y == 6 && (5..=11).contains(&x)) || x == 8 && (6..=10).contains(&y)
            || (y == 10 && (6..=10).contains(&x)),
        "hammer" => (y == 6 && (6..=10).contains(&x)) || (x == 8 && (7..=10).contains(&y)),
        "flame" => ((x == 8 && (5..=9).contains(&y)) || (x == 7 && (7..=9).contains(&y))
            || (x == 9 && (7..=9).contains(&y))) && !(y == 9 && x != 8),
        "wheat" => (matches!(x, 6 | 8 | 10) && (5..=10).contains(&y)),
        "book" => (y == 8 && (6..=10).contains(&x)) || (y == 9 && (6..=10).contains(&x))
            || (x == 8 && (6..=10).contains(&y)),
        _ => (x == 7 && (6..=8).contains(&y)) || (x == 9 && (8..=10).contains(&y))
            || (y == 8 && (6..=10).contains(&x)), // chain, deliberately broken
    };
    if sym {
        return Rgba([symbol_color[0] as u8, symbol_color[1] as u8, symbol_color[2] as u8, 255]);
    }
    let v = (x * 3 + y * 5) % 7;
    Rgba([ch(color[0] + v), ch(color[1] + v), ch(color[2] + v), 255])
}

/// Humanoid outfit used by villagers and companions: hair, face, robe with
/// faction trim + chest symbol, legs. The same texture wraps every cube
/// face, so it reads as an outfit at glance distance.
fn outfit_pixel(
    x: u32,
    y: u32,
    robe: [u32; 3],
    trim: Option<[u32; 3]>,
    hair: [u32; 3],
    symbol: Option<(&str, [u32; 3])>,
) -> Rgba<u8> {
    let v = (x * 3 + y * 5) % 7;
    if y < 4 {
        return Rgba([ch(hair[0] + v), ch(hair[1] + v), ch(hair[2] + v), 255]);
    }
    if (4..6).contains(&y) {
        // face with eyes
        if y == 5 && matches!(x, 4 | 5 | 10 | 11) {
            return Rgba([38, 38, 58, 255]);
        }
        return Rgba([224, 188, 152, 255]);
    }
    if (12..16).contains(&y) {
        // legs + boots
        return if y == 15 { Rgba([52, 46, 40, 255]) } else { Rgba([70, 62, 54, 255]) };
    }
    // robe band 6..12
    if let Some((kind, sc)) = symbol {
        let sym = match kind {
            "scale" => (y == 8 && (6..=10).contains(&x)) || (x == 8 && (8..=10).contains(&y)),
            "hammer" => (y == 8 && (7..=9).contains(&x)) || (x == 8 && (9..=10).contains(&y)),
            "flame" => x == 8 && (8..=10).contains(&y),
            "wheat" => matches!(x, 7 | 9) && (8..=10).contains(&y),
            "book" => (y == 9 && (6..=10).contains(&x)) || (x == 8 && (8..=10).contains(&y)),
            _ => (x == 7 && (8..=9).contains(&y)) || (x == 9 && y == 9), // chain
        };
        if sym {
            return Rgba([sc[0] as u8, sc[1] as u8, sc[2] as u8, 255]);
        }
    }
    if let Some(t) = trim {
        if y == 6 || y == 11 {
            return Rgba([t[0] as u8, t[1] as u8, t[2] as u8, 255]);
        }
    }
    Rgba([ch(robe[0] + v), ch(robe[1] + v), ch(robe[2] + v), 255])
}

fn villager_pixel(x: u32, y: u32, name: &str) -> Rgba<u8> {
    let hair = [45, 36, 28];
    match name {
        "villager_accord" => outfit_pixel(x, y, [74, 122, 181], Some([236, 240, 246]), hair, Some(("scale", [240, 244, 250]))),
        "villager_ironborn" => outfit_pixel(x, y, [139, 69, 19], Some([58, 40, 26]), hair, Some(("hammer", [235, 228, 220]))),
        "villager_covenant" => outfit_pixel(x, y, [74, 56, 46], Some([196, 96, 42]), hair, Some(("flame", [232, 148, 60]))),
        "villager_freeholds" => outfit_pixel(x, y, [107, 142, 35], Some([210, 190, 140]), hair, Some(("wheat", [240, 232, 200]))),
        "villager_ashen" => outfit_pixel(x, y, [176, 176, 176], Some([236, 233, 226]), hair, Some(("book", [62, 66, 74]))),
        "villager_nameless" => outfit_pixel(x, y, [45, 45, 45], None, hair, Some(("chain", [140, 140, 140]))),
        // The Unmarked: Nameless clothes, ash-grey hair, no symbol anywhere
        "villager_unmarked" => {
            let mut px = outfit_pixel(x, y, [45, 45, 45], None, [172, 172, 172], None);
            if y < 4 {
                px = Rgba([ch(172 + (x * 3 + y * 5) % 9), ch(170 + (x * 3 + y * 5) % 9), ch(168 + (x * 3 + y * 5) % 9), 255]);
            }
            px
        }
        // Archivist Maren Voss: Order robes + the journal under one arm and
        // a decorative hem at the robe's bottom edge
        _ => {
            let mut px = outfit_pixel(x, y, [176, 176, 176], Some([236, 233, 226]), hair, Some(("book", [62, 66, 74])));
            if (12..=14).contains(&x) && (9..=12).contains(&y) {
                px = Rgba([122, 92, 62, 255]); // the journal
            }
            if y == 11 {
                px = Rgba([124, 128, 138, 255]); // hem
            }
            px
        }
    }
}

fn companion_pixel(x: u32, y: u32, name: &str) -> Rgba<u8> {
    let trusted = name.ends_with("_trusted");
    let base = name.trim_end_matches("_trusted");
    let hair = [42, 34, 26];
    let (robe, trim, symbol): ([u32; 3], Option<[u32; 3]>, (&str, [u32; 3])) = match base {
        "companion_accord_warden" => ([86, 108, 148], Some([224, 230, 240]), ("scale", [240, 244, 250])),
        "companion_ironborn_artisan" => ([126, 74, 34], Some([196, 168, 120]), ("hammer", [235, 228, 220])),
        "companion_covenant_channeler" => ([66, 50, 42], Some([214, 116, 48]), ("flame", [238, 160, 70])),
        "companion_freeholds_scout" => ([96, 128, 36], Some([206, 188, 138]), ("wheat", [240, 232, 200])),
        "companion_ashen_scribe" => ([168, 168, 172], Some([236, 233, 226]), ("book", [62, 66, 74])),
        _ => ([52, 52, 54], None, ("chain", [150, 150, 150])),
    };
    let mut px = outfit_pixel(x, y, robe, trim, hair, Some(symbol));
    // archetype detail rows
    match base {
        "companion_accord_warden" if y == 7 => px = Rgba([120, 134, 156, 255]), // pauldrons
        "companion_ironborn_artisan" if (5..=10).contains(&x) && (9..=12).contains(&y) => {
            px = Rgba([148, 104, 58, 255]) // leather apron + tool belt
        }
        "companion_ironborn_artisan" if y == 10 && (5..=10).contains(&x) => px = Rgba([90, 70, 44, 255]),
        "companion_covenant_channeler" if y == 12 => px = Rgba([234, 152, 62, 255]), // glow cuffs
        "companion_freeholds_scout" if y < 3 => px = Rgba([86, 112, 40, 255]), // hood
        "companion_ashen_scribe" if (12..=14).contains(&x) && (9..=12).contains(&y) => {
            px = Rgba([122, 92, 62, 255]) // journal
        }
        "companion_nameless_rover" if (x + y) % 7 == 3 => px = Rgba([70, 70, 74, 255]), // patches
        _ => {}
    }
    // trust badge (>= 50): a small warm gold mark on the chest
    if trusted && (7..=8).contains(&x) && (8..=9).contains(&y) {
        px = Rgba([244, 204, 120, 255]);
    }
    px
}

/// Mob skins: distinct palettes per type; the common hostiles share a
/// pattern helper so biome-tint variants are palette swaps of the same art
/// (accent pixels — eyes/glow — stay constant across variants).
fn mob_pixel(x: u32, y: u32, name: &str) -> Rgba<u8> {
    let v = (x * 5 + y * 3) % 8;
    match name {
        "mob_boar" => {
            if y == 12 && (6..=9).contains(&x) {
                Rgba([182, 134, 104, 255]) // snout
            } else if pixel_hash(x, y, "boar") % 9 < 2 {
                Rgba([104, 72, 48, 255]) // coarse bristle
            } else {
                Rgba([ch(132 + v), ch(92 + v), ch(62 + v), 255])
            }
        }
        "mob_woolbeast" => {
            if y > 12 {
                Rgba([122, 112, 102, 255]) // legs
            } else if pixel_hash(x, y, "wool") % 7 < 2 {
                Rgba([208, 202, 192, 255]) // fleece mottle
            } else {
                Rgba([ch(232 + v / 2), ch(226 + v / 2), ch(216 + v / 2), 255])
            }
        }
        "mob_glitchling" | "mob_glitchling_desert" | "mob_glitchling_snow" | "mob_glitchling_swamp" => {
            let body = match name {
                "mob_glitchling_desert" => [196u32, 172, 110],
                "mob_glitchling_snow" => [206, 222, 238],
                "mob_glitchling_swamp" => [96, 110, 80],
                _ => [70, 180, 140],
            };
            if y % 4 == 1 {
                Rgba([ch(body[0] / 2), ch(body[1] / 2), ch(body[2] / 2), 255]) // scanline
            } else if pixel_hash(x, y, "glitch") % 19 < 1 {
                Rgba([222, 255, 242, 255]) // hot pixels (accent, untinted)
            } else {
                Rgba([ch(body[0] + v), ch(body[1] + v), ch(body[2] + v), 255])
            }
        }
        "mob_stalker" | "mob_stalker_desert" | "mob_stalker_snow" | "mob_stalker_swamp" => {
            let body = match name {
                "mob_stalker_desert" => [186, 158, 106],
                "mob_stalker_snow" => [176, 196, 222],
                "mob_stalker_swamp" => [82, 96, 66],
                _ => [112, 56, 40],
            };
            if y == 3 && matches!(x, 3 | 12) {
                Rgba([250, 240, 180, 255]) // eyes (accent, untinted)
            } else if x % 3 == 0 {
                Rgba([ch((body[0] * 4) / 5), ch((body[1] * 4) / 5), ch((body[2] * 4) / 5), 255])
            } else {
                Rgba([ch(body[0] + v), ch(body[1] + v), ch(body[2] + v), 255])
            }
        }
        "mob_crawler" | "mob_crawler_desert" | "mob_crawler_snow" | "mob_crawler_swamp" => {
            let body = match name {
                "mob_crawler_desert" => [172, 142, 92],
                "mob_crawler_snow" => [168, 184, 208],
                "mob_crawler_swamp" => [76, 88, 62],
                _ => [72, 62, 56],
            };
            // ember glow patches are a heat property — never tinted
            if pixel_hash(x, y, "crawler") % 7 < 2 {
                Rgba([222, 122, 42, 255])
            } else if pixel_hash(x, y, "crawler") % 5 == 0 {
                Rgba([ch(body[0] * 3 / 4), ch(body[1] * 3 / 4), ch(body[2] * 3 / 4), 255])
            } else {
                Rgba([ch(body[0] + v), ch(body[1] + v), ch(body[2] + v), 255])
            }
        }
        _ => {
            // Null Knight: near-black armor with grey void-glow at the joints
            if y == 5 && (6..=9).contains(&x) {
                Rgba([96, 96, 118, 255]) // visor
            } else if x % 5 == 0 && y % 5 == 0 {
                Rgba([142, 142, 154, 255]) // joint glow
            } else {
                Rgba([ch(28 + v), ch(28 + v), ch(34 + v), 255])
            }
        }
    }
}

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
    "bucket", "water_bucket", "oil_bucket", "refined_fuel", "tar",
    "tome_of_the_forge", "tome_of_the_null", "wardens_ledger",
    "bronze_chestplate", "steel_chestplate",
    "bronze_helmet", "bronze_leggings", "bronze_boots",
    "steel_helmet", "steel_leggings", "steel_boots",
    "raw_copper", "copper_ingot", "raw_tin", "tin_ingot",
    "aluminum_ingot", "sulfur", "bronze_ingot", "steel_ingot",
    "copper_wire", "iron_gear", "machine_frame", "basic_circuit",
    "glitch_dust", "null_shard",
    "raw_uranium", "uranium_ingot", "fuel_rod",
    "scroll_of_firebolt", "scroll_of_gale_step", "scroll_of_ward", "scroll_of_hearthlight",
    "rune_of_haste", "rune_of_warding", "chisel", "blueprint",
    "stone_slab", "planks_slab", "stone_stairs", "dragon_scale",
    "precision_gear", "master_blueprint", "battlestaff", "master_chisel",
    "iron_plate", "bog_grass", "torn_archive_page", "anima_crystal",
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
    "..cCCggCCgCc....",
    "..cCCggCCgCc....",
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

/// 'm' = metal wall, 'M' = highlight rim, 'd' = dark base/shadow,
/// 'w'/'W' = water fill + sparkle (water_bucket only).
const BUCKET_ART: [&str; 16] = [
    "................",
    "................",
    "...M........M...",
    "...mM......Mm...",
    "...mMm....mMm...",
    "....mMm..mMm....",
    "....mMmmmmMm....",
    "....mMwwwwMm....",
    "....mMwWWwMm....",
    "....mMwwwwMm....",
    "....dMmmmmMd....",
    ".....dddddd.....",
    "................",
    "................",
    "................",
    "................",
];

const SLFB_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "................",
    "....kSSSSk......",
    "...kSssssSk.....",
    "..kSskkkksSk....",
    "..kSskSSksSk....",
    "..kSskkkksSk....",
    "...kSssssSk.....",
    "....kSSSSk......",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const SLAB_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "..SSSSSSSSSSSS..",
    "..SSSSSSSSSSSS..",
    "..ssssssssssss..",
    "..ssssssssssss..",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const STAIRS_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "......SSSSSSS...",
    "......SSSSSSS...",
    "......SSSSSSS...",
    "......sssssss...",
    "..SSSSssssss....",
    "..SSSSSSSSSS....",
    "..SSSSSSSSSS....",
    "..ssssssssss....",
    "..ssssssssss....",
    "................",
    "................",
    "................",
    "................",
];

const CHISEL_ART: [&str; 16] = [
    "................",
    "..............M.",
    ".............Mm.",
    "............Mm..",
    "...........Mm...",
    "..........Mm....",
    ".........Mm.....",
    "........mm......",
    ".......hh.......",
    "......hh........",
    ".....hh.........",
    "....dd..........",
    "................",
    "................",
    "................",
    "................",
];

const BLUEPRINT_ART: [&str; 16] = [
    "................",
    ".pppppppppppppp.",
    ".pPPPPPPPPPPPPp.",
    ".pPwwwwwwwwwwPp.",
    ".pPwiiiiiiiiwPp.",
    ".pPwiwiiiiwiwPp.",
    ".pPwiiiiiiwiwPp.",
    ".pPwiwiiiiiiwPp.",
    ".pPwiwiiiiwiwPp.",
    ".pPwiiiiiiwiwPp.",
    ".pPwiiiiiiiiwPp.",
    ".pPwwwwwwwwwwPp.",
    ".pPPPPPPPPPPPPp.",
    ".pppppppppppppp.",
    "................",
    "................",
];

const RUNE_ART: [&str; 16] = [
    "................",
    "................",
    "......dGg.......",
    ".....dGggd......",
    "....dGg..gGd....",
    "....dg.GG.gd....",
    "....dg.GG.gd....",
    "....dGg..gGd....",
    ".....dGggd......",
    "......dGg.......",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const SCROLL_ART: [&str; 16] = [
    "................",
    ".....rrrr.......",
    "....rppppr......",
    "...rpssppr......",
    "..rpspppspr.....",
    "..rpspgppr......",
    "..rpspggpr......",
    "..rpsppppr......",
    "..rpssppr.......",
    "...rppppr.......",
    "....rrrr........",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const FUEL_ROD_ART: [&str; 16] = [
    "................",
    ".......M........",
    "......mMm.......",
    ".....mMgMm......",
    ".....mGgGm......",
    ".....mGgGm......",
    ".....mMgMm......",
    ".....mGgGm......",
    ".....mGgGm......",
    ".....mMgMm......",
    ".....mGgGm......",
    ".....mMgMm......",
    "......mMm.......",
    ".......M........",
    "................",
    "................",
];

fn ingot_palette2(base: Rgba<u8>, top: Rgba<u8>, dark: Rgba<u8>) -> RgbaImage {
    paint_sprite(INGOT_ART, |c| match c {
        'I' => base, 'H' => top, 'i' => dark, 'd' => dark, _ => Rgba([0, 0, 0, 0]),
    })
}

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

/// Loop 329 armor set: helmet (brow + dome), leggings (waist + legs),
/// boots (feet) — same m/M palette language as the chestplate.
const HELMET_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "....MMMMMMMM....",
    "...MMmmmmmmMM...",
    "..MMmmmmmmmmMM..",
    "..MmmmmmmmmmmM..",
    "..Mmm......mmM..",
    "..Mm........mM..",
    "..Mm........mM..",
    "..mm........mm..",
    "................",
    "................",
    "................",
    "................",
    "................",
];

const LEGGINGS_ART: [&str; 16] = [
    "................",
    "................",
    "..mmmmmmmmmmmm..",
    "..mmmmmmmmmmmm..",
    "..mmmMMMMMMmmm..",
    "..mmm.mmmm.mmm..",
    "..mm...mm...mm..",
    "..mm...mm...mm..",
    "..mm...mm...mm..",
    "..mm...mm...mm..",
    ".mmm...mm...mmm.",
    ".mm....mm....mm.",
    "................",
    "................",
    "................",
    "................",
];

const BOOTS_ART: [&str; 16] = [
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "................",
    "..mm......mm....",
    "..mm......mm....",
    "..mm......mm....",
    "..mmm....mmm....",
    "..mMMM..mMMM....",
    "..mmmm..mmmm....",
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
        "tome_of_the_forge" => paint_sprite(BOOK_ART, |c| match c {
            'c' => Rgba([110, 60, 30, 255]), 'C' => Rgba([150, 96, 50, 255]),
            'p' => Rgba([240, 190, 110, 255]), 'g' => Rgba([250, 150, 60, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "tome_of_the_null" => paint_sprite(BOOK_ART, |c| match c {
            'c' => Rgba([40, 40, 52, 255]), 'C' => Rgba([70, 70, 90, 255]),
            'p' => Rgba([150, 150, 170, 255]), 'g' => Rgba([120, 220, 235, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "wardens_ledger" => paint_sprite(BOOK_ART, |c| match c {
            'c' => Rgba([50, 70, 90, 255]), 'C' => Rgba([80, 110, 135, 255]),
            'p' => Rgba([210, 225, 235, 255]), 'g' => Rgba([90, 170, 240, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "book" => paint_sprite(BOOK_ART, |c| match c {
            'c' => Rgba([120, 80, 45, 255]), 'C' => Rgba([152, 106, 62, 255]),
            'p' => Rgba([230, 225, 210, 255]), _ => Rgba([0, 0, 0, 0]),
        }),
        "bucket" => paint_sprite(BUCKET_ART, |c| match c {
            'm' => Rgba([198, 198, 206, 255]), 'M' => Rgba([232, 232, 240, 255]),
            'd' => Rgba([140, 140, 150, 255]), _ => Rgba([0, 0, 0, 0]),
        }),
        "water_bucket" => paint_sprite(BUCKET_ART, |c| match c {
            'm' => Rgba([198, 198, 206, 255]), 'M' => Rgba([232, 232, 240, 255]),
            'd' => Rgba([140, 140, 150, 255]),
            'w' => Rgba([64, 120, 200, 255]), 'W' => Rgba([110, 170, 235, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "oil_bucket" => paint_sprite(BUCKET_ART, |c| match c {
            'm' => Rgba([198, 198, 206, 255]), 'M' => Rgba([232, 232, 240, 255]),
            'd' => Rgba([140, 140, 150, 255]),
            'w' => Rgba([28, 22, 14, 255]), 'W' => Rgba([70, 58, 36, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "refined_fuel" => paint_sprite(BUCKET_ART, |c| match c {
            'm' => Rgba([210, 140, 60, 255]), 'M' => Rgba([240, 180, 90, 255]),
            'd' => Rgba([150, 90, 34, 255]),
            'w' => Rgba([250, 170, 60, 255]), 'W' => Rgba([255, 220, 140, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "tar" => paint_sprite(SULFUR_ART, |c| match c {
            'y' => Rgba([26, 22, 20, 255]), 'Y' => Rgba([70, 62, 56, 255]),
            _ => Rgba([0, 0, 0, 0]),
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
        // loop 329: the rest of the kit shares the chestplate palette
        "bronze_helmet" | "steel_helmet" | "bronze_leggings" | "steel_leggings"
        | "bronze_boots" | "steel_boots" => {
            let base = if item_id.starts_with("bronze") {
                (Rgba([205, 127, 50, 255]), Rgba([235, 165, 90, 255]))
            } else {
                (Rgba([172, 182, 198, 255]), Rgba([208, 218, 234, 255]))
            };
            let art = if item_id.ends_with("helmet") {
                HELMET_ART
            } else if item_id.ends_with("leggings") {
                LEGGINGS_ART
            } else {
                BOOTS_ART
            };
            paint_sprite(art, |c| match c {
                'm' => base.0, 'M' => base.1, _ => Rgba([0, 0, 0, 0]),
            })
        }
        "raw_iron" => raw_chunk(Rgba([172, 146, 126, 255]), Rgba([210, 185, 160, 255])),
        // lore-and-visuals materials: flat plate, grass bundle, torn page,
        // and the Anima crystal (Covenant channeler wage)
        "iron_plate" => paint_sprite(INGOT_ART, |c| match c {
            'I' => Rgba([188, 192, 200, 255]), 'i' => Rgba([160, 164, 174, 255]),
            'H' => Rgba([214, 218, 226, 255]), 'd' => Rgba([120, 124, 134, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "bog_grass" => paint_sprite(STICK_ART, |c| match c {
            'h' => Rgba([96, 122, 58, 255]), 'H' => Rgba([130, 158, 74, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "torn_archive_page" => paint_sprite(BLUEPRINT_ART, |c| match c {
            'p' => Rgba([232, 226, 208, 255]), 'P' => Rgba([248, 244, 232, 255]),
            'l' => Rgba([120, 124, 138, 255]), _ => Rgba([0, 0, 0, 0]),
        }),
        "anima_crystal" => paint_sprite(SULFUR_ART, |c| match c {
            'y' => Rgba([214, 128, 44, 255]), 'Y' => Rgba([248, 192, 96, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "raw_uranium" => raw_chunk(Rgba([70, 130, 55, 255]), Rgba([140, 230, 100, 255])),
        "scroll_of_firebolt" => paint_sprite(SCROLL_ART, |c| match c {
            'p' => Rgba([238, 226, 196, 255]), 'r' => Rgba([120, 72, 40, 255]),
            's' => Rgba([60, 130, 220, 255]), 'g' => Rgba([250, 140, 60, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "scroll_of_gale_step" => paint_sprite(SCROLL_ART, |c| match c {
            'p' => Rgba([238, 226, 196, 255]), 'r' => Rgba([120, 72, 40, 255]),
            's' => Rgba([60, 130, 220, 255]), 'g' => Rgba([150, 235, 170, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "scroll_of_ward" => paint_sprite(SCROLL_ART, |c| match c {
            'p' => Rgba([238, 226, 196, 255]), 'r' => Rgba([120, 72, 40, 255]),
            's' => Rgba([60, 130, 220, 255]), 'g' => Rgba([235, 235, 245, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "precision_gear" => paint_sprite(SULFUR_ART, |c| match c {
            'y' => Rgba([210, 210, 220, 255]), 'Y' => Rgba([245, 245, 252, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "master_blueprint" => paint_sprite(BLUEPRINT_ART, |c| match c {
            'p' => Rgba([70, 110, 170, 255]), 'P' => Rgba([110, 150, 210, 255]),
            'w' => Rgba([240, 240, 235, 255]), 'i' => Rgba([255, 215, 90, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "battlestaff" => paint_sprite(CHISEL_ART, |c| match c {
            'm' => Rgba([150, 40, 30, 255]), 'M' => Rgba([255, 140, 60, 255]),
            'h' => Rgba([120, 84, 46, 255]), 'd' => Rgba([104, 76, 43, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "master_chisel" => paint_sprite(CHISEL_ART, |c| match c {
            'm' => Rgba([200, 200, 210, 255]), 'M' => Rgba([240, 240, 248, 255]),
            'h' => Rgba([120, 84, 46, 255]), 'd' => Rgba([104, 76, 43, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "dragon_scale" => paint_sprite(SLFB_ART, |c| match c {
            's' => Rgba([150, 40, 30, 255]), 'S' => Rgba([200, 70, 50, 255]),
            'k' => Rgba([90, 22, 18, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "stone_slab" => paint_sprite(SLAB_ART, |c| match c {
            's' => Rgba([130, 130, 134, 255]), 'S' => Rgba([172, 172, 178, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "planks_slab" => paint_sprite(SLAB_ART, |c| match c {
            's' => Rgba([150, 116, 68, 255]), 'S' => Rgba([186, 148, 92, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "stone_stairs" => paint_sprite(STAIRS_ART, |c| match c {
            's' => Rgba([130, 130, 134, 255]), 'S' => Rgba([172, 172, 178, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "chisel" => paint_sprite(CHISEL_ART, |c| match c {
            'm' => Rgba([200, 200, 210, 255]), 'M' => Rgba([240, 240, 248, 255]),
            'h' => HANDLE, 'd' => HANDLE_DARK,
            _ => Rgba([0, 0, 0, 0]),
        }),
        "blueprint" => paint_sprite(BLUEPRINT_ART, |c| match c {
            'p' => Rgba([70, 110, 170, 255]), 'P' => Rgba([110, 150, 210, 255]),
            'w' => Rgba([240, 240, 235, 255]), 'i' => Rgba([250, 210, 120, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "rune_of_haste" => paint_sprite(RUNE_ART, |c| match c {
            'g' => Rgba([150, 235, 170, 255]), 'G' => Rgba([210, 255, 225, 255]),
            'd' => Rgba([70, 120, 85, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "rune_of_warding" => paint_sprite(RUNE_ART, |c| match c {
            'g' => Rgba([120, 200, 235, 255]), 'G' => Rgba([190, 235, 255, 255]),
            'd' => Rgba([60, 95, 125, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "scroll_of_hearthlight" => paint_sprite(SCROLL_ART, |c| match c {
            'p' => Rgba([238, 226, 196, 255]), 'r' => Rgba([120, 72, 40, 255]),
            's' => Rgba([60, 130, 220, 255]), 'g' => Rgba([255, 220, 130, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
        "uranium_ingot" => ingot_palette2(Rgba([90, 160, 70, 255]), Rgba([140, 230, 105, 255]), Rgba([55, 105, 45, 255])),
        "fuel_rod" => paint_sprite(FUEL_ROD_ART, |c| match c {
            'm' => Rgba([170, 180, 190, 255]), 'M' => Rgba([215, 225, 235, 255]),
            'g' => Rgba([120, 240, 90, 255]), 'G' => Rgba([180, 255, 150, 255]),
            _ => Rgba([0, 0, 0, 0]),
        }),
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
            if name == "water_wheel" || name == "battery" || name == "pipe" || name == "boiler" || name == "steam_engine"
                || name == "pump" || name == "refinery" || name == "combustion_generator"
                || name == "reactor" || name == "enchanting_table" || name == "warding_pylon"
                || name == "scaffold" || name == "statue" || name == "conduit" || name == "elevator" || name == "dragon_egg" || name == "belt"
                || name == "ac_unit" || name == "computer" {
                // machine shells with hollow details
                let solid = tex.pixels().filter(|p| p.0[3] == 255).count();
                assert!(solid > 40, "{} too sparse", name);
                continue;
            }
            if name == "flower" {
                // cutout plant: transparent gaps plus solid stem/petals
                let holes = tex.pixels().filter(|p| p.0[3] == 0).count();
                let solid = tex.pixels().filter(|p| p.0[3] == 255).count();
                assert!(holes > 100 && solid > 20, "flower holes={} solid={}", holes, solid);
                continue;
            }
            if matches!(name, "tall_grass" | "dry_grass" | "dead_shrub") {
                // cutout ground-cover plants (E3): transparent gaps + blades
                let holes = tex.pixels().filter(|p| p.0[3] == 0).count();
                let solid = tex.pixels().filter(|p| p.0[3] == 255).count();
                assert!(holes > 100 && solid > 20, "{} holes={} solid={}", name, holes, solid);
                continue;
            }
            if name == "glass" {
                // frame opaque, pane translucent
                continue;
            }
            if name.starts_with("stained_glass_") {
                // tinted pane, translucent like glass
                assert!(tex.pixels().all(|p| p.0[3] == 140 || p.0[3] == 210), "{} unexpected alpha", name);
                continue;
            }
            if name == "ironborn_grate" {
                // punched holes + translucent iron between them
                let holes = tex.pixels().filter(|p| p.0[3] == 0).count();
                assert!(holes > 20, "grate holes={}", holes);
                assert!(tex.pixels().all(|p| matches!(p.0[3], 0 | 170 | 255)));
                continue;
            }
            if name.starts_with("banner_") {
                // cutout cloth: transparent around the banner, solid on it
                let holes = tex.pixels().filter(|p| p.0[3] == 0).count();
                let solid = tex.pixels().filter(|p| p.0[3] == 255).count();
                assert!(holes > 100 && solid > 30, "{} holes={} solid={}", name, holes, solid);
                continue;
            }
            if name == "ice" {
                // translucent pane everywhere
                assert!(tex.pixels().all(|p| p.0[3] == 200));
                continue;
            }
            if name.starts_with("crack_") || name.starts_with("waypoint_") || name.starts_with("grid_") {
                // decal: mostly transparent, some opaque crack pixels
                let solid = tex.pixels().filter(|p| p.0[3] == 255).count();
                assert!(solid > 4, "{} too sparse", name);
                continue;
            }
            if is_leaf_name(name) {
                // alpha-cutout foliage: holes AND leaf pixels, all-or-nothing
                let holes = tex.pixels().filter(|p| p.0[3] == 0).count();
                let solid = tex.pixels().filter(|p| p.0[3] == 255).count();
                assert!(holes > 20 && solid > 100, "{} holes={} solid={}", name, holes, solid);
                continue;
            }
            if name != "torch_item" {
                let expected_alpha = if name == "water" { 170 } else { 255 };
                assert!(tex.pixels().all(|p| p.0[3] == expected_alpha));
            }
        }
    }

    #[test]
    fn crack_stages_grow() {
        let counts: Vec<usize> = (0..4).map(|s| {
            generate_block_texture(&format!("crack_{}", s)).pixels().filter(|p| p.0[3] == 255).count()
        }).collect();
        assert!(counts[0] < counts[3], "later stages have more crack pixels: {:?}", counts);
    }

    #[test]
    fn per_face_mapping_routes_materials() {
        use lf_voxel::meshing::Face;
        let name_of = |id: u32, f: Face| TEXTURE_NAMES[texture_index_for_face(id, f) as usize];
        // grass: green top, banded side, dirt bottom
        assert_eq!(name_of(2, Face::Top), "grass_top");
        assert_eq!(name_of(2, Face::Side), "grass");
        assert_eq!(name_of(2, Face::Bottom), "dirt");
        // every log species: ring texture on the ends, bark on the sides
        for log_id in [7u32, 19, 20, 21, 22] {
            assert_eq!(name_of(log_id, Face::Top), "log_top", "log {}", log_id);
            assert_eq!(name_of(log_id, Face::Bottom), "log_top", "log {}", log_id);
            assert_ne!(name_of(log_id, Face::Side), "log_top");
        }
        // blocks without distinct faces are unchanged on every face
        assert_eq!(name_of(1, Face::Top), "stone");
        assert_eq!(name_of(1, Face::Bottom), "stone");
        assert_eq!(name_of(41, Face::Side), "research_bench");
        assert_eq!(name_of(200, Face::Top), "mod");
    }

    #[test]
    fn test_atlas_covers_all_blocks() {
        let atlas = generate_atlas();
        assert_eq!(atlas.len(), TEXTURE_NAMES.len());
        // every known block id maps to a valid layer (all vanilla ids + mods)
        for id in 1u32..=lf_voxel::registry::MAX_VANILLA_BLOCK {
            assert!(texture_index_for_block(id) < atlas.len() as u32, "block {} unmapped", id);
        }
        assert!(texture_index_for_block(200) < atlas.len() as u32, "mod blocks unmapped");
        // spot-check the wiring so wood variants and machines stop falling back to stone
        let name = |id: u32| TEXTURE_NAMES[texture_index_for_block(id) as usize];
        assert_eq!(name(19), "birch_log");
        assert_eq!(name(31), "ice");
        assert_eq!(name(32), "copper_ore");
        assert_eq!(name(37), "coal_generator");
        assert_eq!(name(41), "research_bench");
        assert_eq!(name(13), "lantern");
        assert_eq!(name(200), "mod");
        // the lore-and-visuals expansion: every new block hits its own layer
        assert_eq!(name(68), "accord_stone");
        assert_eq!(name(73), "ember_glowstone");
        assert_eq!(name(83), "volcanic_basalt");
        assert_eq!(name(87), "bog_peat");
        assert_eq!(name(98), "stained_glass_white");
        assert_eq!(name(104), "banner_nameless");
        assert_eq!(name(105), "lantern_hanging");
        assert_eq!(name(100), "banner_ironborn", "id 100 is a vanilla banner now");
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
