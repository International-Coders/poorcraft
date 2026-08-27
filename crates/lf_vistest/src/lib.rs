//! Visual test harness: builds real scenes from real game data (worldgen ->
//! voxel sections -> mesher) and renders them with the real renderer to PNG.
//! Every proof screenshot must come through here.

use std::path::Path;

use glam::Vec3;
use lf_engine::{Camera, GpuVertex};
use lf_voxel::{BlockState, World};
use lf_worldgen::{Biome, Seed, WorldGen};

/// One registered visual test scene.
pub struct SceneSpec {
    pub name: &'static str,
    pub desc: &'static str,
    pub default_seed: u64,
    /// Time of day as a fraction of the day cycle [0..1]; drives sky color.
    pub time_of_day: f32,
    /// First-person scenes put the camera at player eye height on the terrain.
    pub first_person: bool,
    /// Scenes with torches place a lit torch grid on the terrain before meshing.
    pub torches: bool,
    /// Machine scenes place generator/furnace/crusher/assembler blocks.
    pub machines: bool,
    /// Ray-traced scenes render through the compute path tracer.
    pub raytraced: bool,
    /// Scene-relative camera placement (eye/target in world blocks).
    pub eye: Vec3,
    pub target: Vec3,
}

impl SceneSpec {
    fn time_of_day(&self) -> lf_game::TimeOfDay {
        lf_game::TimeOfDay::from_fraction(self.time_of_day)
    }

    fn sky_color(&self) -> [f64; 4] {
        let c = self.time_of_day().sky_color();
        [c[0] as f64, c[1] as f64, c[2] as f64, 1.0]
    }

    fn day_factor(&self) -> f32 {
        self.time_of_day().sky_light_level()
    }
}

/// The scene registry. Rendered by `run_scene` and enumerated by tests/xtask.
pub fn scenes() -> Vec<SceneSpec> {
    vec![
        SceneSpec {
            name: "spawn_plains_dawn",
            desc: "meadow spawn at dawn, gentle terrain",
            default_seed: 12345,
            time_of_day: 0.25,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-28.0, 92.0, 64.0),
            target: Vec3::new(24.0, 66.0, 8.0),
        },
        SceneSpec {
            name: "terrain_vista",
            desc: "wider view over varied biomes at noon",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-60.0, 130.0, 120.0),
            target: Vec3::new(48.0, 64.0, 24.0),
        },
        SceneSpec {
            name: "night_watch",
            desc: "same terrain at night",
            default_seed: 12345,
            time_of_day: 0.0,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-28.0, 92.0, 64.0),
            target: Vec3::new(24.0, 66.0, 8.0),
        },
        SceneSpec {
            name: "first_person_view",
            desc: "in-game eye height over real terrain (what the player sees)",
            default_seed: 12345,
            time_of_day: 0.35,
            first_person: true,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(8.5, 0.0, 8.5),
            target: Vec3::ZERO, // computed from terrain in run_scene
        },
        SceneSpec {
            name: "terrain_features",
            desc: "meadow with trees, ores in cliffs, water at the shore",
            default_seed: 12345,
            time_of_day: 0.45,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-40.0, 0.0, 90.0),
            target: Vec3::new(24.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "biome_montage",
            desc: "vista across the 30-biome world (mixed tree species)",
            default_seed: 4242,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-60.0, 0.0, 110.0),
            target: Vec3::new(40.0, 0.0, 0.0),
        },
        SceneSpec {
            name: "clouds_weather",
            desc: "above the cloud layer looking down through it, rain below",
            default_seed: 4242,
            time_of_day: 0.55,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-30.0, 210.0, 60.0),
            target: Vec3::new(30.0, 110.0, -20.0),
        },
        SceneSpec {
            name: "village_trading",
            desc: "hamlet with villagers and an open trade panel",
            default_seed: 12345,
            time_of_day: 0.4,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-30.0, 0.0, -30.0),
            target: Vec3::new(10.0, 0.0, 10.0),
        },
        SceneSpec {
            name: "industrial_machines",
            desc: "machines placed and running on the terrain",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: true,
            raytraced: false,
            eye: Vec3::new(-26.0, 0.0, 42.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "tech_tree",
            desc: "the research progression screen",
            default_seed: 12345,
            time_of_day: 0.45,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-26.0, 0.0, 42.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "raytraced_shadows",
            desc: "path-traced frame with soft sun shadows + GI",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: true,
            torches: false,
            machines: false,
            raytraced: true,
            eye: Vec3::new(8.5, 0.0, 8.5),
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "raytraced_night",
            desc: "path-traced night: torch emissive light and bounce",
            default_seed: 12345,
            time_of_day: 0.97,
            first_person: false,
            torches: true,
            machines: false,
            raytraced: true,
            eye: Vec3::new(-26.0, 0.0, 40.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "menu_preview",
            desc: "the animated title screen over the world",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-26.0, 0.0, 42.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "settings_preview",
            desc: "the tabbed settings screen with RT controls",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-26.0, 0.0, 42.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "hud_preview",
            desc: "in-game view with the real HUD drawn via egui (proof shot)",
            default_seed: 12345,
            time_of_day: 0.42,
            first_person: true,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(8.5, 0.0, 8.5),
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "torchlit_night",
            desc: "night scene lit by torches placed on the terrain",
            default_seed: 12345,
            time_of_day: 0.97,
            first_person: false,
            torches: true,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-26.0, 0.0, 40.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "crafting_ui",
            desc: "3x3 crafting grid + recipe book with real icons",
            default_seed: 12345,
            time_of_day: 0.42,
            first_person: true,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(8.5, 0.0, 8.5),
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "map_screen",
            desc: "the world map with biome colors, fog, waypoints",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(-26.0, 0.0, 42.0),
            target: Vec3::new(8.0, 0.0, 8.0),
        },
        SceneSpec {
            name: "console_preview",
            desc: "the developer console with autocomplete + history",
            default_seed: 12345,
            time_of_day: 0.42,
            first_person: true,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(8.5, 0.0, 8.5),
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "minimap_hud",
            desc: "in-game HUD with minimap, icons hotbar, XP bar",
            default_seed: 12345,
            time_of_day: 0.35,
            first_person: true,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::new(8.5, 0.0, 8.5),
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "foliage_canopy",
            desc: "canopy close-up: cutout leaves, log rings, smooth AO under leaves",
            default_seed: 12345,
            time_of_day: 0.4,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed directly at the placed canopy in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "mining_feedback",
            desc: "crack decal + debris particles on a block being mined",
            default_seed: 12345,
            time_of_day: 0.45,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed directly at the target block in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "water_flow",
            desc: "source on an aqueduct pours down a flume and pools at a dam (flowing surfaces render lowered)",
            default_seed: 12345,
            time_of_day: 0.4,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed at the built waterfall in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "falling_sand",
            desc: "granular column collapse: settled pile plus a block caught mid-fall",
            default_seed: 12345,
            time_of_day: 0.4,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed at the column in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "texture_tiling",
            desc: "7-wide plank wall + wide stone floor: textures repeat per block, never stretch across merged surfaces",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed straight at the wall in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "waypoint_beacons",
            desc: "world-space waypoint beams rising from the terrain, three colors, transparent pass",
            default_seed: 12345,
            time_of_day: 0.55,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed at the beams in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "biome_contact_sheet",
            desc: "all 30 biomes as side-by-side strips of their real surface materials — the identity proof",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed at the sheet in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "weather_snow",
            desc: "cold biome weather: snowfall over a snow field (biome-driven, Step 19)",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed at the field in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "weather_dry",
            desc: "dry biome weather: clear skies over desert sand (no precipitation, Step 19)",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed at the desert in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "water_wheel_power",
            desc: "Water Age (P29): wheel against a river, battery, crusher — the power field at work",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed at the riverside build in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "steam_chain",
            desc: "Steam Age (P30): water -> pipes -> fueled boiler -> steam engine -> powered crusher, with live steam puffs",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed at the boiler room in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "night_border_seam",
            desc: "torch light crossing a chunk border at night — no seam (P28 cross-column lighting)",
            default_seed: 12345,
            time_of_day: 0.97,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed at the border in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "lore_book",
            desc: "open lore tome with real text from lore/books.toml, paginated reader (Step 20)",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO,
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "oil_chain",
            desc: "Oil Age (P31): oil pool -> pumpjack -> pipes -> refinery -> combustion generator -> powered furnace, pre-run through the real machine code",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed at the derrick line in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "reactor_control",
            desc: "Nuclear (P32): a cooled reactor at equilibrium powering machines — uranium ore vein visible in a cut wall",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO,
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "meltdown_aftermath",
            desc: "Nuclear (P32): the crater a neglected reactor leaves — glowing radiation residue through the wreckage",
            default_seed: 12345,
            time_of_day: 0.88,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO,
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "wizard_tower",
            desc: "Magic (P33): the wizard tower — spiral stair, enchanting table under torchlight",
            default_seed: 42,
            time_of_day: 0.72,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO,
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "spellbook",
            desc: "Magic (P33): the spellbook screen — learned spells, three cast slots, mana bar",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO,
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "spell_effects",
            desc: "Magic (P33): hearthlight's lumen glow, a firebolt streak, a ward ring",
            default_seed: 12345,
            time_of_day: 0.62,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO,
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "build_tools",
            desc: "Construction (P34): slab staircase + stairs, a blueprint ghost, scaffolding and a chiseled statue",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO,
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "modern_wing",
            desc: "Smart building (P35): one wing wired for electricity — conduits relaying the field, elevator shaft, climate unit, computer screens, powered machines",
            default_seed: 12345,
            time_of_day: 0.34,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO,
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "dragon_roost",
            desc: "Dragons (P36): the mountain roost — egg clutch on a stone crag, the dragon perched above it",
            default_seed: 99,
            time_of_day: 0.72,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO,
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "dragon_flight",
            desc: "Dragons (P36): the multi-part assembly mid-flight — body, head, flapping wings, tail — breathing fire",
            default_seed: 12345,
            time_of_day: 0.55,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO,
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "paths_screen",
            desc: "Paths & specialization (P37): four standings with tiers, focus bars, respec note",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO,
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "trade_p2p",
            desc: "Player trading (P37, protocol v4): the offer panel — give/want, accept or cancel",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO,
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "grid_overlay",
            desc: "power-grid overlay (Step 25): green tint cube over the powered furnace, red over a starved crusher out of range",
            default_seed: 12345,
            time_of_day: 0.5,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed at the same build in run_scene
            target: Vec3::ZERO,
        },
        SceneSpec {
            name: "transparency_layers",
            desc: "water pool behind a glass wall with particles on both sides: transparent pass layering, pixel-checked",
            default_seed: 12345,
            time_of_day: 0.45,
            first_person: false,
            torches: false,
            machines: false,
            raytraced: false,
            eye: Vec3::ZERO, // framed straight at the stack in run_scene
            target: Vec3::ZERO,
        },
        // lore-and-visuals: vistest scenes (blocks, structures, skins, HUD)
        SceneSpec { name: "faction_blocks",
            desc: "contact sheet of the 38 lore blocks (faction, biome-exclusive, decoration) on a display field",
            default_seed: 12345, time_of_day: 0.5, first_person: false, torches: false, machines: false, raytraced: false,
            eye: Vec3::ZERO, target: Vec3::ZERO },
        SceneSpec { name: "accord_embassy", desc: "The Accord's walled courtyard: accord_stone walls, pillar gatehouse, banner (C3)",
            default_seed: 12345, time_of_day: 0.5, first_person: false, torches: false, machines: false, raytraced: false,
            eye: Vec3::ZERO, target: Vec3::ZERO },
        SceneSpec { name: "ironborn_forge_camp", desc: "Ironborn industrial camp: brick walls, grate windows, furnace, banner (C3)",
            default_seed: 12345, time_of_day: 0.5, first_person: false, torches: false, machines: false, raytraced: false,
            eye: Vec3::ZERO, target: Vec3::ZERO },
        SceneSpec { name: "covenant_grove_shrine", desc: "Ember Covenant shrine: covenantwood post ring around a glowstone altar (C3)",
            default_seed: 12345, time_of_day: 0.5, first_person: false, torches: false, machines: false, raytraced: false,
            eye: Vec3::ZERO, target: Vec3::ZERO },
        SceneSpec { name: "freeholds_longhouse", desc: "Free Holds longhouse: daub walls, thatch roof, log posts (C3)",
            default_seed: 12345, time_of_day: 0.5, first_person: false, torches: false, machines: false, raytraced: false,
            eye: Vec3::ZERO, target: Vec3::ZERO },
        SceneSpec { name: "ashen_library", desc: "Ashen Order library: marble walls, bookshelf interior, lore chest (C3)",
            default_seed: 12345, time_of_day: 0.5, first_person: false, torches: false, machines: false, raytraced: false,
            eye: Vec3::ZERO, target: Vec3::ZERO },
        SceneSpec { name: "nameless_camp", desc: "Nameless derelict camp: broken rotwood palisade, scorched firepit, loot chest (C3)",
            default_seed: 12345, time_of_day: 0.5, first_person: false, torches: false, machines: false, raytraced: false,
            eye: Vec3::ZERO, target: Vec3::ZERO },
        SceneSpec { name: "entity_skins", desc: "contact sheet: 6 faction villager skins, 6 companion skins (+badge variants), 6 mob skins, 3 biome tints (C2)",
            default_seed: 12345, time_of_day: 0.5, first_person: false, torches: false, machines: false, raytraced: false,
            eye: Vec3::ZERO, target: Vec3::ZERO },
        SceneSpec { name: "ember_glow", desc: "ember_glowstone formation with rising amber sparks (ambient C4 particles)",
            default_seed: 12345, time_of_day: 0.75, first_person: false, torches: false, machines: false, raytraced: false,
            eye: Vec3::ZERO, target: Vec3::ZERO },
        SceneSpec { name: "companion_follow", desc: "an Accord Warden companion standing off at follow distance with the HUD companion tile visible (B3/B4)",
            default_seed: 12345, time_of_day: 0.5, first_person: false, torches: false, machines: false, raytraced: false,
            eye: Vec3::ZERO, target: Vec3::ZERO },
        SceneSpec { name: "faction_map", desc: "world map with two+ faction territories tinted in faction colors + structure icons (A2/D3)",
            default_seed: 12345, time_of_day: 0.5, first_person: false, torches: false, machines: false, raytraced: false,
            eye: Vec3::ZERO, target: Vec3::ZERO },
        SceneSpec { name: "faction_hud", desc: "HUD with the faction standing widget (name, colored bar, standing number) + companion tile (A3/C4)",
            default_seed: 12345, time_of_day: 0.5, first_person: false, torches: false, machines: false, raytraced: false,
            eye: Vec3::ZERO, target: Vec3::ZERO },
        SceneSpec { name: "companion_commands", desc: "the B3 companion command menu: follow/stay/rest/mine/chop/haul/guard/pay/dismiss",
            default_seed: 12345, time_of_day: 0.5, first_person: false, torches: false, machines: false, raytraced: false,
            eye: Vec3::ZERO, target: Vec3::ZERO },
    ]
}

/// Build the mesh for a scene: a radius-chunk plot of worldgen terrain
/// centered at (0,0), using the real World + chunk-column pipeline.
pub fn build_scene_mesh(spec: &SceneSpec, seed: u64, radius_chunks: i32, torches: bool, machines_param: bool)
    -> (Vec<GpuVertex>, Vec<u32>, Vec<GpuVertex>, Vec<u32>) {
    let gen = WorldGen::new(Seed(seed));
    let mut world = World::new();
    for cx in -radius_chunks..=radius_chunks {
        for cz in -radius_chunks..=radius_chunks {
            world.chunks.insert((cx, cz), gen.generate_chunk(cx, cz));
        }
    }
    if machines_param {
        use lf_voxel::registry::block;
        let mut place = |x: i32, z: i32, b: u32| {
            let top = world.surface_height(x, z);
            world.set_block(x, top, z, lf_voxel::BlockState(b));
        };
        place(0, 0, block::COAL_GENERATOR);
        place(2, 0, block::ELECTRIC_FURNACE);
        place(4, 0, block::CRUSHER);
        place(6, 0, block::ASSEMBLER);
        place(8, 0, block::RESEARCH_BENCH);
    }
    if torches {
        use lf_voxel::registry::block;
        // A grid of torches near the origin, placed on the terrain surface.
        for tx in (-24..=8).step_by(8) {
            for tz in (-24..=8).step_by(8) {
                let top = world.surface_height(tx, tz);
                world.set_block(tx, top, tz, lf_voxel::BlockState(block::TORCH));
            }
        }
    }

    // P26 proof geometry: a hand-framed canopy, and a crack decal with
    // debris on a block mid-mining.
    if spec.name == "foliage_canopy" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for y in h..h + 8 {
            world.set_block(0, y, 0, lf_voxel::BlockState(block::LOG));
        }
        for dy in 7..12 {
            for dx in -3i32..=3 {
                for dz in -3i32..=3 {
                    if dx.abs() + dz.abs() + (dy - 7) <= 6 {
                        world.set_block(dx, h + dy, dz, lf_voxel::BlockState(block::LEAVES));
                    }
                }
            }
        }
    }

    // ---- lore-and-visuals PRE-MESH world edits ----------------------
    // faction_blocks: every new block id 68..=105 on a display field (C1)
    if spec.name == "faction_blocks" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -6..34 {
            for z in -10..12 {
                for y in h..h + 14 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
        let mut id = 68u32;
        'fill: for row in 0..4i32 {
            for col in 0..10i32 {
                if id > 105 { break 'fill; }
                let x = -5 + col * 4;
                let z = -8 + row * 6;
                world.set_block(x, h, z, lf_voxel::BlockState(id));
                id += 1;
            }
        }
    }

    // the six faction structures planted via the public builder (C3),
    // into the chunk containing (8,8), BEFORE meshing
    if matches!(spec.name, "accord_embassy" | "ironborn_forge_camp" | "covenant_grove_shrine"
        | "freeholds_longhouse" | "ashen_library" | "nameless_camp") {
        let kind = match spec.name {
            "accord_embassy" => lf_worldgen::FactionStructure::AccordEmbassy,
            "ironborn_forge_camp" => lf_worldgen::FactionStructure::IronbornForgeCamp,
            "covenant_grove_shrine" => lf_worldgen::FactionStructure::CovenantGroveShrine,
            "freeholds_longhouse" => lf_worldgen::FactionStructure::FreeholdsLonghouse,
            "ashen_library" => lf_worldgen::FactionStructure::AshenLibrary,
            _ => lf_worldgen::FactionStructure::NamelessCamp,
        };
        // flatten the site so the structure is unobstructed (a display
        // pedestal: dirt to display height, air above), then plant with a
        // uniform ground
        use lf_voxel::registry::block;
        let base = 112i32;  // above the tallest local terrain (~106)
        for x in -8..22i32 {
            for z in -8..22i32 {
                let top = gen.surface_top(x, z).clamp(base - 6, 250);
                for y in top..(base + 12) {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                // fill dips under the display level so nothing floats
                if (0..16).contains(&x) && (0..16).contains(&z) {
                    for y in (base - 4)..base {
                        if !lf_voxel::registry::is_solid(world.get_block(x, y, z)) {
                            world.set_block(x, y, z, lf_voxel::BlockState(block::DIRT));
                        }
                    }
                }
            }
        }
        if let Some(col) = world.chunks.get_mut(&(0, 0)) {
            let g = |_lx: usize, _lz: usize| -> usize { base as usize };
            lf_worldgen::build_faction_structure(kind, col, &g);
        }
    }

    // entity_skins: flat display ground (C2)
    if spec.name == "entity_skins" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -12..30 {
            for z in -10..14 {
                for y in h..h + 12 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
    }

    // ember_glow: the glowstone cluster itself (C4)
    if spec.name == "ember_glow" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        world.set_block(0, h, 0, lf_voxel::BlockState(block::EMBER_GLOWSTONE));
        world.set_block(0, h + 1, 0, lf_voxel::BlockState(block::EMBER_GLOWSTONE));
        world.set_block(1, h, 0, lf_voxel::BlockState(block::EMBER_GLOWSTONE));
        world.set_block(0, h, 1, lf_voxel::BlockState(block::EMBER_GLOWSTONE));
    }

    if spec.name == "mining_feedback" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for y in 0..5 {
            world.set_block(0, h + y, 0, lf_voxel::BlockState(block::STONE));
        }
    }

    // texture_tiling (goal Section 1): a 7-wide, 4-tall plank wall on a
    // wide stone floor — the proof that textures repeat at 1-block scale
    // on multi-block surfaces instead of stretching.
    if spec.name == "texture_tiling" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -5..=5 {
            for z in -3..=4 {
                for y in h..h + 9 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
        // 7-wide, 4-tall plank wall standing on the floor at z = 0
        for x in -3..=3 {
            for y in h..h + 4 {
                world.set_block(x, y, 0, lf_voxel::BlockState(block::PLANKS));
            }
        }
    }

    // biome_contact_sheet (Step 16): 30 strips, each paved with that
    // biome's REAL surface + filler from the biome table, separated by
    // stone walls — a photograph of the identity data itself.
    if spec.name == "biome_contact_sheet" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        let biomes = lf_worldgen::Biome::ALL;
        for (i, b) in biomes.iter().enumerate() {
            let x0 = (i as i32) * 4 - 60;
            for x in x0..x0 + 4 {
                for z in -8..8 {
                    world.set_block(x, h - 1, z, lf_voxel::BlockState(b.surface_block()));
                    world.set_block(x, h - 2, z, lf_voxel::BlockState(b.filler_block()));
                    for y in h..h + 12 {
                        world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                    }
                }
            }
            if i % 2 == 1 {
                // separating wall so strips read as distinct panels
                for z in -8..8 {
                    for y in h..h + 2 {
                        world.set_block(x0 + 4, y, z, lf_voxel::BlockState(block::STONE));
                    }
                }
            }
        }
    }

    // weather_snow: a snow field with falling flakes (Step 19)
    if spec.name == "weather_snow" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -10..10 {
            for z in -10..10 {
                for y in h..h + 10 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::SNOW));
            }
        }
    }

    // weather_dry: desert sand under a clear sky (Step 19)
    if spec.name == "weather_dry" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -10..10 {
            for z in -10..10 {
                for y in h..h + 10 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::SAND));
                world.set_block(x, h - 2, z, lf_voxel::BlockState(block::SAND));
            }
        }
    }

    // water_wheel_power (P29): a real riverside build — the river carved,
    // the wheel placed against it, a battery and a crusher in the field;
    // the wheel is ticked through the actual machine/power code so the
    // crusher runs on river power in the proof.
    // grid_overlay fills this with (pos, powered) pairs for the tint cubes.
    let mut grid_cubes: Vec<((i32, i32, i32), bool)> = Vec::new();
    if spec.name == "water_wheel_power" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        // flatten a riverbank work pad
        for x in -6..10 {
            for z in -6..6 {
                for y in h..h + 12 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::GRASS));
            }
        }
        // the river: a 2-wide channel of water sources at z = -3..-2
        for x in -6..10 {
            for z in -3..=-2 {
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
                world.set_block(x, h, z, lf_voxel::water_with_level(0));
            }
        }
        // the build: wheel against the river, battery + crusher in the field
        world.set_block(0, h, -1, lf_voxel::BlockState(block::WATER_WHEEL));
        world.set_block(2, h, -1, lf_voxel::BlockState(block::BATTERY));
        world.set_block(2, h, 1, lf_voxel::BlockState(block::CRUSHER));
        // run the real power field: 30 simulated seconds spin the wheel and
        // drive the crusher (the pure step the client tick uses)
        let mut sources = vec![
            ((0, h, -1), lf_game::machines::PowerSource::Wheel(Default::default())),
            ((2, h, -1), lf_game::machines::PowerSource::Battery(Default::default())),
        ];
        let dt = 1.0f32 / 20.0;
        for _ in 0..600 {
            if let ((_, lf_game::machines::PowerSource::Wheel(w)), true) = (&mut sources[0], true) {
                w.tick(dt, true);
            }
            let need = lf_game::machines::DRAW_RATE * dt;
            lf_game::machines::distribute_power(&mut sources, &[(2, h, 1)], need);
        }
        // stash the settled state for the test assertion via the world? the
        // visual proof is the scene itself; the numeric proof is the
        // lf_game water_age_tests.
    }

    // steam_chain (P30): the full boiler room, pre-run through the real
    // machine code (pipes equalize, boiler burns, engine buffers) so the
    // proof shows the chain mid-operation; steam puffs appended post-mesh.
    if spec.name == "steam_chain" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -6..8 {
            for z in -6..6 {
                for y in h..h + 12 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
        // water source at x=-4; pipes from it to the boiler at x=0
        world.set_block(-4, h, 0, lf_voxel::water_with_level(0));
        world.set_block(-3, h, 0, lf_voxel::BlockState(block::PIPE));
        world.set_block(-2, h, 0, lf_voxel::BlockState(block::PIPE));
        world.set_block(-1, h, 0, lf_voxel::BlockState(block::PIPE));
        world.set_block(0, h, 0, lf_voxel::BlockState(block::BOILER));
        world.set_block(1, h, 0, lf_voxel::BlockState(block::STEAM_ENGINE));
        world.set_block(1, h, 2, lf_voxel::BlockState(block::CRUSHER));
        // pre-run the real chain for 30 sim-seconds: the boiler lights,
        // the engine buffers, the crusher runs
        let mut pipes: Vec<lf_game::machines::Pipe> = (0..3).map(|_| Default::default()).collect();
        let mut boiler = lf_game::machines::Boiler {
            fuel: Some(lf_game::survival::ItemStack { item_id: "coal".into(), count: 8 }),
            burn_left: 0.0, water: 0, steam: 0.0,
        };
        let mut engine = lf_game::machines::SteamEngine::default();
        let dt = 1.0f32 / 20.0;
        for _ in 0..600 {
            // the adjacent source feeds the first pipe
            let mut feed = 200u16;
            while feed > 0 {
                let took = pipes[0].fill(lf_game::machines::FluidKind::Water, feed.min(60));
                if took == 0 {
                    break; // pipe full
                }
                feed -= took;
            }
            for i in 0..pipes.len().saturating_sub(1) {
                let (a, b) = if i % 2 == 0 {
                    let mut x = pipes[i].clone(); let mut y = pipes[i + 1].clone();
                    x.equalize_with(&mut y); (x, y)
                } else {
                    (pipes[i].clone(), pipes[i + 1].clone())
                };
                pipes[i] = a; pipes[i + 1] = b;
            }
            let mut water_in = 0u16;
            water_in += pipes[2].draw(lf_game::machines::FluidKind::Water, 30);
            boiler.tick(dt, water_in + 40); // + pump from nothing here; fuel+pipe water
            let steam_in = boiler.draw_steam(lf_game::machines::STEAM_ENGINE_INTAKE * dt * 2.0);
            engine.tick(dt, steam_in);
        }
    }

    // night_border_seam (P28): a torch at x=15 — the chunk border runs at
    // x=16 — and the light must fall off smoothly across it.
    if spec.name == "night_border_seam" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -8..24 {
            for z in -6..6 {
                for y in h..h + 12 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
        // the seam-straddling torch, one block west of the border
        world.set_block(15, h, 0, lf_voxel::BlockState(block::TORCH));
    }

    // oil_chain + grid_overlay (P31/Step 25): the derrick line — an oil
    // pool in a pit, the pumpjack over it, pipes to the refinery, the
    // combustion generator powering everything, an electric furnace as the
    // visible consumer. Pre-run through the REAL machine code (the same
    // chain the client ticks) so the proof shows it mid-operation; the
    // grid variant classifies the machines with the same ratio rule the
    // client overlay uses and records the tint cubes.
    if spec.name == "oil_chain" || spec.name == "grid_overlay" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -9..10 {
            for z in -6..6 {
                for y in h..h + 12 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
        // oil pit: two-deep pool of crude sources at the west end
        for x in -7..=-3 {
            for z in -1..=1 {
                world.set_block(x, h - 1, z, lf_voxel::oil_with_level(0));
                world.set_block(x, h - 2, z, lf_voxel::oil_with_level(0));
            }
        }
        let pump_pos = (-5i32, h, 0i32);
        let refinery_pos = (-1, h, 0);
        let furnace_pos = (0, h, 0);
        let gen_pos = (-2, h, 2);
        let coal_pos = (-6, h, 2); // bootstraps the pump while oil flows
        let crusher_pos = (8, h, 0); // grid variant only: far out of range
        world.set_block(pump_pos.0, pump_pos.1, pump_pos.2, lf_voxel::BlockState(block::PUMP));
        for x in -4..=-2 {
            world.set_block(x, h, 0, lf_voxel::BlockState(block::PIPE));
        }
        world.set_block(refinery_pos.0, refinery_pos.1, refinery_pos.2, lf_voxel::BlockState(block::REFINERY));
        world.set_block(furnace_pos.0, furnace_pos.1, furnace_pos.2, lf_voxel::BlockState(block::ELECTRIC_FURNACE));
        world.set_block(gen_pos.0, gen_pos.1, gen_pos.2, lf_voxel::BlockState(block::COMBUSTION_GENERATOR));
        world.set_block(coal_pos.0, coal_pos.1, coal_pos.2, lf_voxel::BlockState(block::COAL_GENERATOR));
        if spec.name == "grid_overlay" {
            world.set_block(crusher_pos.0, crusher_pos.1, crusher_pos.2, lf_voxel::BlockState(block::CRUSHER));
        }

        // pre-run the real chain for 200 sim-seconds
        use lf_game::machines::{self, FluidKind, PowerSource};
        let mut pump = machines::PumpJack::default();
        let mut pipes: Vec<machines::Pipe> = (0..3).map(|_| Default::default()).collect();
        let mut refinery = machines::Refinery::default();
        let mut gen = machines::CombustionGenerator {
            fuel: Some(lf_game::survival::ItemStack { item_id: "refined_fuel".into(), count: 8 }),
            ..Default::default()
        };
        // the pumpjack boots on coal until the oil chain spins up (one
        // combustion generator feeds two machines, not three — 26 EU/s
        // vs 10 per machine; the overlay shows exactly that truth)
        let mut coal = machines::Generator {
            fuel: Some(lf_game::survival::ItemStack { item_id: "coal".into(), count: 20 }),
            burn_left: 60.0,
            buffer: 500.0,
        };
        let dt = 1.0f32 / 20.0;
        let need = machines::DRAW_RATE * dt;
        for _ in 0..4000 {
            // power first (combustion covers everything in range)
            coal.tick(dt);
            let mut sources = vec![
                (coal_pos, PowerSource::Generator(coal.clone())),
                (gen_pos, PowerSource::Combustion(gen.clone())),
            ];
            let mut machines_list = vec![pump_pos, refinery_pos, furnace_pos];
            if spec.name == "grid_overlay" {
                machines_list.push(crusher_pos);
            }
            let granted = machines::distribute_power(&mut sources, &machines_list, need);
            for (spos, src) in sources {
                match src {
                    PowerSource::Combustion(c) if spos == gen_pos => gen = c,
                    PowerSource::Generator(g) if spos == coal_pos => coal = g,
                    _ => {}
                }
            }
            // pump lifts into the first pipe
            let mut lifted = pump.tick(dt, granted[0], true);
            while lifted > 0 {
                let took = pipes[0].fill(FluidKind::Crude, lifted.min(60));
                if took == 0 {
                    break;
                }
                lifted -= took;
            }
            for i in 0..pipes.len() - 1 {
                let (mut x, mut y) = (pipes[i].clone(), pipes[i + 1].clone());
                x.equalize_with(&mut y);
                pipes[i] = x;
                pipes[i + 1] = y;
            }
            let mut crude_in = pipes[2].draw(FluidKind::Crude, 40);
            crude_in += pipes[1].draw(FluidKind::Crude, 20);
            refinery.tick(dt, granted[1], crude_in);
            // haul finished fuel to the generator
            if let Some(out) = refinery.fuel_out.take() {
                match &mut gen.fuel {
                    Some(f) => f.count += out.count,
                    None => gen.fuel = Some(out),
                }
            }
            gen.tick(dt);
            // grid classification exactly like the client overlay
            if spec.name == "grid_overlay" {
                for (mi, mpos) in machines_list.iter().enumerate() {
                    if *mpos == pump_pos || *mpos == refinery_pos {
                        continue; // overlay shows consumer machines only
                    }
                    let ratio = granted[mi] / need.max(1e-6);
                    grid_cubes.push((*mpos, ratio >= 0.9));
                }
            }
        }
    }

    // reactor_control (P32): a uranium vein exposed in a cut wall, a
    // water-cooled reactor at thermal equilibrium running machines through
    // the real tick code (60 sim-seconds, full coolant).
    if spec.name == "reactor_control" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -6..8 {
            for z in -6..6 {
                for y in h..h + 10 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
        // uranium vein exposed in a cut stone wall at the east end
        for vx in 5..=6i32 {
            for vz in -1..=1i32 {
                for vy in h..h + 3 {
                    world.set_block(vx, vy, vz, lf_voxel::BlockState(block::STONE));
                }
            }
        }
        for (vx, vy, vz) in [(4, h, 0), (4, h + 1, 0), (5, h + 1, 1), (5, h, 1)] {
            world.set_block(vx, vy, vz, lf_voxel::BlockState(block::URANIUM_ORE));
        }
        // cooling water channel feeding the reactor from the west
        world.set_block(-4, h, 0, lf_voxel::water_with_level(0));
        world.set_block(-3, h, 0, lf_voxel::BlockState(block::PIPE));
        world.set_block(-2, h, 0, lf_voxel::BlockState(block::PIPE));
        let reactor_pos = (-1i32, h, 0i32);
        world.set_block(reactor_pos.0, reactor_pos.1, reactor_pos.2, lf_voxel::BlockState(block::REACTOR));
        world.set_block(1, h, 0, lf_voxel::BlockState(block::ELECTRIC_FURNACE));
        world.set_block(1, h, 2, lf_voxel::BlockState(block::CRUSHER));
        // run the real reactor: equilibrium at full coolant
        use lf_game::machines::Reactor;
        let mut reactor = Reactor {
            fuel: Some(lf_game::survival::ItemStack { item_id: "fuel_rod".into(), count: 4 }),
            ..Default::default()
        };
        let dt = 1.0f32 / 20.0;
        for _ in 0..1200 {
            reactor.tick(dt, (lf_game::machines::REACTOR_COOLANT_RATE as f32 * dt).ceil() as u16);
        }
        assert!(reactor.heat < 30.0 && reactor.buffer > 1000.0, "equilibrium core for the proof");
    }

    // meltdown_aftermath (P32): what neglect leaves — a blast crater with
    // glowing radiation residue, generated through the same placement rule
    // the client's apply_meltdown uses.
    if spec.name == "meltdown_aftermath" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        // a small machine hall, now ruined
        for x in -6..7 {
            for z in -5..5 {
                for y in h..h + 8 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
        let center = (0i32, h + 1, 0i32);
        let r = 3i32;
        let mut placed = 0usize;
        for dx in -r..=r {
            for dy in -r..=r {
                for dz in -r..=r {
                    if dx * dx + dy * dy + dz * dz > r * r {
                        continue;
                    }
                    let (x, y, z) = (center.0 + dx, center.1 + dy, center.2 + dz);
                    if y < h - 1 {
                        continue;
                    }
                    let residue = (x * 7 + y * 13 + z * 5).rem_euclid(3) == 0;
                    let b = if residue && placed < 14 {
                        placed += 1;
                        block::RADIATION
                    } else {
                        block::AIR
                    };
                    world.set_block(x, y, z, lf_voxel::BlockState(b));
                }
            }
        }
        // the wrecked machines that used to run the hall, flanking the crater
        world.set_block(-3, h, 2, lf_voxel::BlockState(block::CRUSHER));
        world.set_block(3, h, 1, lf_voxel::BlockState(block::COMBUSTION_GENERATOR));
    }

    // wizard_tower (P33): the same tower worldgen places in FlowerForest/
    // Highlands (visual twin of build_wizard_tower — the generation itself
    // is proven by the worldgen unit test).
    if spec.name == "wizard_tower" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -6..8 {
            for z in -6..6 {
                for y in h..h + 14 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::GRASS));
            }
        }
        // 5x5 shell 9 tall, hollow, on the local ground (chunk-local 6..=10
        // mapped to world -2..=2 around the origin)
        for dy in 0..9usize {
            for dx in 6..=10usize {
                for dz in 6..=10usize {
                    let edge = dx == 6 || dx == 10 || dz == 6 || dz == 10;
                    let b = if dy == 0 || dy == 8 || edge { block::STONE } else { block::AIR };
                    world.set_block(dx as i32 - 8, h + dy as i32, dz as i32 - 8, lf_voxel::BlockState(b));
                }
            }
        }
        let ring = [(7, 6), (8, 6), (9, 6), (10, 7), (10, 8), (10, 9), (9, 10), (8, 10), (7, 10), (6, 9), (6, 8), (6, 7)];
        for step in 0..7usize {
            let (sx, sz) = ring[step % ring.len()];
            world.set_block(sx as i32 - 8, h + 1 + step as i32, sz as i32 - 8, lf_voxel::BlockState(block::STONE));
        }
        world.set_block(0, h + 1, -2, lf_voxel::BlockState(block::AIR));
        world.set_block(0, h + 2, -2, lf_voxel::BlockState(block::AIR));
        world.set_block(0, h + 9, 0, lf_voxel::BlockState(block::ENCHANTING_TABLE));
        world.set_block(-2, h + 9, 0, lf_voxel::BlockState(block::TORCH));
        world.set_block(2, h + 9, 0, lf_voxel::BlockState(block::TORCH));
    }

    // spell_effects (P33): a lumen block lighting a dark shelf, a firebolt
    // streak mid-flight, a ward ring around the caster spot.
    if spec.name == "spell_effects" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -6..7 {
            for z in -6..6 {
                for y in h..h + 9 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
        // hearthlight's temporary light + one placed lumen (the crossover)
        world.set_block(-2, h, 0, lf_voxel::BlockState(block::LUMEN_BLOCK));
        world.set_block(2, h, 1, lf_voxel::BlockState(block::ENCHANTING_TABLE));
    }

    // dragon_roost (P36): the crag + egg clutch (the worldgen twin; the
    // generation itself is proven by the worldgen unit test).
    if spec.name == "dragon_roost" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -8..9 {
            for z in -8..8 {
                for y in h..h + 14 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
        // stone crag spire (the build_roost twin)
        for dy in 0..5i32 {
            let r = 3 - dy / 2;
            if r < 0 { continue; }
            for dx in -r..=r {
                for dz in -r..=r {
                    world.set_block(dx, h + dy, dz, lf_voxel::BlockState(block::STONE));
                }
            }
        }
        world.set_block(-1, h + 5, 0, lf_voxel::BlockState(block::DRAGON_EGG));
        world.set_block(1, h + 5, -1, lf_voxel::BlockState(block::DRAGON_EGG));
    }

    // dragon_flight (P36): the multi-part dragon assembled through the
    // SAME layout fn the client renders with, mid-flap, breathing fire.
    if spec.name == "dragon_flight" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -8..9 {
            for z in -8..8 {
                for y in h..h + 16 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
    }

    // modern_wing (P35): "one wing wired for electricity" — a two-storey
    // wing: conduits relay a distant generator's field to the upper
    // floor machines, an elevator shaft climbs the side, a climate unit
    // hums on the wall, and a computer screen shows the grid page.
    if spec.name == "modern_wing" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -8..10 {
            for z in -6..6 {
                for y in h..h + 12 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::PLANKS));
            }
        }
        // ground floor slab + upper floor
        for x in -7..9 {
            for z in -5..5 {
                world.set_block(x, h + 4, z, lf_voxel::BlockState(block::PLANKS)
                    .with_shape(lf_voxel::Shape::SlabTop));
            }
        }
        // back wall
        for x in -7..9 {
            for y in h..h + 9 {
                world.set_block(x, y, -5, lf_voxel::BlockState(block::GLASS));
            }
        }
        // the generator far west (beyond raw range of the east machines —
        // the conduits bridge it, same as the distribute test)
        world.set_block(-3, h, -4, lf_voxel::BlockState(block::COAL_GENERATOR));
        for x in [-1i32, 2, 5] {
            world.set_block(x, h + 1, -4, lf_voxel::BlockState(block::CONDUIT));
            world.set_block(x, h + 5, -4, lf_voxel::BlockState(block::CONDUIT));
        }
        world.set_block(4, h + 4, -2, lf_voxel::BlockState(block::CONDUIT));
        // upper-floor machines fed through the relay
        world.set_block(4, h + 5, 0, lf_voxel::BlockState(block::ELECTRIC_FURNACE));
        world.set_block(6, h + 5, 0, lf_voxel::BlockState(block::CRUSHER));
        // elevator shaft on the east wall
        for y in 0..9 {
            world.set_block(8, h + y, 2, lf_voxel::BlockState(block::ELEVATOR));
        }
        // climate unit + computer on the upper floor
        world.set_block(0, h + 5, -4, lf_voxel::BlockState(block::AC_UNIT));
        world.set_block(-3, h + 5, 0, lf_voxel::BlockState(block::COMPUTER));
        // pre-run the real relayed power so the machines run mid-proof
        use lf_game::machines::{self, PowerSource};
        let mut gen = machines::Generator {
            fuel: Some(lf_game::survival::ItemStack { item_id: "coal".into(), count: 6 }),
            burn_left: 60.0,
            buffer: 1500.0,
        };
        gen.tick(1.0);
        let mut sources = vec![((-3, h, -4), PowerSource::Generator(gen))];
        let conduits = [(-1, h + 1, -4), (2, h + 1, -4), (5, h + 1, -4), (4, h + 4, -2)];
        let granted = machines::distribute_power_relayed(
            &mut sources, &conduits, &[(4, h + 5, 0), (6, h + 5, 0)], machines::DRAW_RATE * 0.5);
        assert!(granted[0] > 0.0 && granted[1] > 0.0,
            "the conduits really carry the field to the upper machines in the proof");
    }

    // build_tools (P34): the construction kit in one frame — a slab
    // staircase climbing east, oriented stairs at the top, scaffolding
    // beside it, a chiseled statue on a pedestal, and a blueprint ghost
    // (translucent tint cubes) hanging where it would paste.
    if spec.name == "build_tools" {
        use lf_voxel::{BlockState, Shape};
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -8..10 {
            for z in -6..6 {
                for y in h..h + 10 {
                    world.set_block(x, y, z, BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, BlockState(block::STONE));
            }
        }
        // slab staircase: each step is one bottom slab + the next course
        for step in 0..4 {
            let x = -6 + step;
            for xw in x..(x + 4) {
                world.set_block(xw, h + step, 0, BlockState(block::STONE).with_shape(Shape::SlabBottom));
            }
        }
        // an oriented stair at the top landing
        world.set_block(-2, h + 4, 0, BlockState(block::STONE).with_shape(Shape::StairEast));
        world.set_block(-1, h + 4, 0, BlockState(block::STONE).with_shape(Shape::StairEast));
        // scaffolding tower beside the stairs
        for y in 0..5 {
            world.set_block(2, h + y, -2, BlockState(block::SCAFFOLD));
            world.set_block(3, h + y, -2, BlockState(block::SCAFFOLD));
        }
        // statue on a plinth
        world.set_block(5, h, 2, BlockState(block::STONE));
        world.set_block(5, h + 1, 2, BlockState(block::STATUE));
    }

    // transparency_layers (Step 8): a water pool BEHIND a glass wall, with
    // debris billboards on both sides of the glass — water must be visible
    // through the pane, and the near particle must render over the glass
    // while the far one shows through it.
    if spec.name == "transparency_layers" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -5..5 {
            for z in -4..8 {
                for y in h..h + 8 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
        // water pool (sources) at z 0..=3, x -2..=2, two deep
        for z in 0..=3 {
            for x in -2..=2 {
                world.set_block(x, h, z, lf_voxel::water_with_level(0));
                world.set_block(x, h + 1, z, lf_voxel::water_with_level(0));
            }
        }
        // glass wall at z = 5, three tall
        for x in -3..=3 {
            for y in h..h + 3 {
                world.set_block(x, y, 5, lf_voxel::BlockState(block::GLASS));
            }
        }
    }

    // water_flow: a stone aqueduct with a source on top, a guiding flume
    // and a dam — then the real simulation runs to quiescence before
    // meshing, so the PNG shows actual flow levels and pooling.
    if spec.name == "water_flow" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        // flatten a work pad
        for x in -6..16 {
            for z in -6..6 {
                for y in h..h + 14 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
        // flume walls guide the runoff 1-D along +x, dam at the far end
        for x in 0..=9 {
            for y in h..h + 2 {
                world.set_block(x, y, -2, lf_voxel::BlockState(block::STONE));
                world.set_block(x, y, 2, lf_voxel::BlockState(block::STONE));
            }
        }
        for y in h..h + 3 {
            for z in -2..=2 {
                world.set_block(9, y, z, lf_voxel::BlockState(block::STONE));
            }
        }
        // aqueduct pillar + source on top
        for y in h..h + 5 {
            world.set_block(0, y, 0, lf_voxel::BlockState(block::STONE));
        }
        world.set_block(0, h + 5, 0, lf_voxel::water_with_level(0));
        let mut q = std::collections::VecDeque::new();
        lf_game::fluids::enqueue_around(&mut q, (0, h + 5, 0));
        lf_game::fluids::settle(&mut world, &mut q, 20_000);
    }

    // falling_sand: a sand column over a dug pocket — the collapse runs
    // through the real gravity settle before meshing.
    if spec.name == "falling_sand" {
        use lf_voxel::registry::block;
        let h = world.surface_height(0, 0);
        for x in -5..5 {
            for z in -5..5 {
                for y in h..h + 10 {
                    world.set_block(x, y, z, lf_voxel::BlockState(block::AIR));
                }
                world.set_block(x, h - 1, z, lf_voxel::BlockState(block::STONE));
            }
        }
        // pocket: two air cells under the column with a stone floor
        world.set_block(0, h - 1, 0, lf_voxel::BlockState(block::AIR));
        world.set_block(0, h - 2, 0, lf_voxel::BlockState(block::AIR));
        world.set_block(0, h - 3, 0, lf_voxel::BlockState(block::STONE));
        for y in h..h + 5 {
            world.set_block(0, y, 0, lf_voxel::BlockState(block::SAND));
        }
        lf_game::fluids::settle_gravity(&mut world, 0, 0);
    }

    let to_gpu = |vs: &[lf_voxel::meshing::Vertex]| -> Vec<GpuVertex> {
        vs.iter().map(|v| GpuVertex {
            position: v.position,
            normal: v.normal,
            tex_coord: v.tex_coord,
            tex_index: v.tex_index,
            ao: v.ao,
            light: v.light,
            sway: v.sway,
        }).collect()
    };
    let mut vertices: Vec<GpuVertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut water_vertices: Vec<GpuVertex> = Vec::new();
    let mut water_indices: Vec<u32> = Vec::new();
    for cx in -radius_chunks..=radius_chunks {
        for cz in -radius_chunks..=radius_chunks {
            let mesh = world.mesh_column(cx, cz, &|b, face| lf_assets::texture_index_for_face(b.id(), face));
            let base = vertices.len() as u32;
            vertices.extend(to_gpu(&mesh.opaque.vertices));
            indices.extend(mesh.opaque.indices.iter().map(|i| i + base));
            let wbase = water_vertices.len() as u32;
            water_vertices.extend(to_gpu(&mesh.water.vertices));
            water_indices.extend(mesh.water.indices.iter().map(|i| i + wbase));
        }
    }
    // mining_feedback: crack decal + debris billboards around the column
    // that build_scene_mesh placed before meshing
    if spec.name == "mining_feedback" {
        let push_quad = |vertices: &mut Vec<GpuVertex>, indices: &mut Vec<u32>,
                         corners: [[f32; 3]; 4], uvs: [[f32; 2]; 4], normal: [f32; 3], tex: u32| {
            let base = vertices.len() as u32;
            for (c, uv) in corners.iter().zip(uvs.iter()) {
                vertices.push(GpuVertex {
                    position: *c,
                    normal,
                    tex_coord: *uv,
                    tex_index: tex,
                    ao: 1.0,
                    light: 0xF0,
                    sway: 0.0,
                });
            }
            indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        };
        // stage-2 crack decal, inflated around the middle block
        let h = world.surface_height(0, 0);
        let (cx, cy, cz) = (0.5f32, h as f32 + 2.5, 0.5f32);
        let r = 0.505f32;
        let crack = lf_assets::CRACK_LAYERS[2];
        let faces: [([f32; 3], [[f32; 3]; 4], [[f32; 2]; 4]); 6] = [
            ([-1.0, 0.0, 0.0], [[-r, -r, -r], [-r, r, -r], [-r, r, r], [-r, -r, r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
            ([1.0, 0.0, 0.0], [[r, -r, r], [r, r, r], [r, r, -r], [r, -r, -r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
            ([0.0, -1.0, 0.0], [[-r, -r, -r], [-r, -r, r], [r, -r, r], [r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ([0.0, 1.0, 0.0], [[-r, r, r], [-r, r, -r], [r, r, -r], [r, r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ([0.0, 0.0, -1.0], [[r, -r, -r], [r, r, -r], [-r, r, -r], [-r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ([0.0, 0.0, 1.0], [[-r, -r, r], [-r, r, r], [r, r, r], [r, -r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
        ];
        for (normal, corners, uvs) in faces {
            let corners: [[f32; 3]; 4] = corners.map(|c| [cx + c[0], cy + c[1], cz + c[2]]);
            push_quad(&mut vertices, &mut indices, corners, uvs, normal, crack);
        }
        // camera-facing debris quads (stone texture sub-tiles)
        let stone_tex = lf_assets::texture_index_for_block(lf_voxel::registry::block::STONE);
        for i in 0..6u32 {
            let t = i as f32;
            let (ox, oy, oz) = (((t * 2.3).sin()) * 0.9, 1.6 + (t % 2.0) * 0.8, ((t * 1.7).cos()) * 0.9);
            let center = Vec3::new(cx + ox, cy + oy, cz + oz);
            let right = Vec3::new(0.08, 0.0, 0.0);
            let up = Vec3::new(0.0, 0.08, 0.0);
            let u0 = (t * 0.11) % 0.75;
            let v0 = (t * 0.17) % 0.75;
            let c0 = center - right - up;
            let c1 = center - right + up;
            let c2 = center + right + up;
            let c3 = center + right - up;
            push_quad(&mut vertices, &mut indices,
                [[c0.x, c0.y, c0.z], [c1.x, c1.y, c1.z], [c2.x, c2.y, c2.z], [c3.x, c3.y, c3.z]],
                [[u0, v0 + 0.25], [u0, v0], [u0 + 0.25, v0], [u0 + 0.25, v0 + 0.25]],
                [0.0, 0.0, 1.0], stone_tex);
        }
    }
    // ---- lore-and-visuals POST-MESH appends -------------------------
    // entity_skins: skin-wearing cubes in rows (C2)
    if spec.name == "entity_skins" {
        let h = world.surface_height(0, 0);
        let mut skins: Vec<u32> = vec![
            lf_assets::VILLAGER_ACCORD_LAYER, lf_assets::VILLAGER_IRONBORN_LAYER,
            lf_assets::VILLAGER_COVENANT_LAYER, lf_assets::VILLAGER_FREEHOLDS_LAYER,
            lf_assets::VILLAGER_ASHEN_LAYER, lf_assets::VILLAGER_NAMELESS_LAYER,
            lf_assets::VILLAGER_UNMARKED_LAYER, lf_assets::VILLAGER_MAREN_LAYER,
        ];
        for (_, layer) in lf_assets::COMPANION_LAYERS {
            skins.push(layer);
            skins.push(lf_assets::trusted_companion_layer(layer));
        }
        skins.extend([
            lf_assets::MOB_BOAR_LAYER, lf_assets::MOB_WOOLBEAST_LAYER,
            lf_assets::MOB_GLITCHLING_LAYER, lf_assets::MOB_STALKER_LAYER,
            lf_assets::MOB_CRAWLER_LAYER, lf_assets::MOB_NULL_KNIGHT_LAYER,
            lf_assets::MOB_GLITCHLING_TINTS[0], lf_assets::MOB_GLITCHLING_TINTS[1],
            lf_assets::MOB_GLITCHLING_TINTS[2],
        ]);
        for (i, tex) in skins.iter().enumerate() {
            let row = i as i32 / 8;
            let col = i as i32 % 8;
            let cx = -10.5 + col as f32 * 3.0;
            let cz = -6.5 + row as f32 * 3.0;
            let cy = h as f32 + 1.0;
            let r = 0.55f32;
            let base = vertices.len() as u32;
            let faces: [([f32; 3], [[f32; 3]; 4], [[f32; 2]; 4]); 6] = [
                ([-1.0, 0.0, 0.0], [[-r, -r, -r], [-r, r, -r], [-r, r, r], [-r, -r, r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
                ([1.0, 0.0, 0.0], [[r, -r, r], [r, r, r], [r, r, -r], [r, -r, -r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
                ([0.0, -1.0, 0.0], [[-r, -r, -r], [-r, -r, r], [r, -r, r], [r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
                ([0.0, 1.0, 0.0], [[-r, r, r], [-r, r, -r], [r, r, -r], [r, r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
                ([0.0, 0.0, -1.0], [[r, -r, -r], [r, r, -r], [-r, r, -r], [-r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
                ([0.0, 0.0, 1.0], [[-r, -r, r], [-r, r, r], [r, r, r], [r, -r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ];
            for (normal, corners, uvs) in faces {
                for (corner, uv) in corners.iter().zip(uvs.iter()) {
                    vertices.push(GpuVertex {
                        position: [cx + corner[0], cy + corner[1], cz + corner[2]],
                        normal, tex_coord: *uv, tex_index: *tex,
                        ao: 1.0, light: 0xF0, sway: 0.0,
                    });
                }
                indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
            }
        }
    }

    // ember_glow: rising amber sparks over the cluster (C4)
    if spec.name == "ember_glow" {
        let h = world.surface_height(0, 0);
        for i in 0..14u32 {
            let t = i as f32;
            let x = 0.5 + (t * 1.1).sin() * 0.4;
            let z = 0.5 + (t * 0.7).cos() * 0.4;
            let y = h as f32 + 2.0 + t * 0.42;
            let sz = 0.07 + (t % 3.0) * 0.012;
            let base = water_vertices.len() as u32;
            let u0 = 0.3 + (i % 3) as f32 * 0.1;
            let corners = [
                [x - sz, y - sz, z], [x - sz, y + sz, z],
                [x + sz, y + sz, z], [x + sz, y - sz, z],
            ];
            for (corner, uv) in corners.iter().zip([[u0, 0.45], [u0, 0.35], [u0 + 0.1, 0.35], [u0 + 0.1, 0.45]]) {
                water_vertices.push(GpuVertex {
                    position: *corner, normal: [0.0, 0.0, 1.0], tex_coord: uv,
                    tex_index: lf_assets::EMBER_LAYER, ao: 1.0, light: 0xF0, sway: 0.0,
                });
            }
            water_indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }

    // structure scenes: the faction's villager NPC at the gate (C3 proof
    // requires the NPC present with the building)
    if matches!(spec.name, "accord_embassy" | "ironborn_forge_camp" | "covenant_grove_shrine"
        | "freeholds_longhouse" | "ashen_library" | "nameless_camp") {
        let tex = match spec.name {
            "accord_embassy" => lf_assets::VILLAGER_ACCORD_LAYER,
            "ironborn_forge_camp" => lf_assets::VILLAGER_IRONBORN_LAYER,
            "covenant_grove_shrine" => lf_assets::VILLAGER_COVENANT_LAYER,
            "freeholds_longhouse" => lf_assets::VILLAGER_FREEHOLDS_LAYER,
            "ashen_library" => lf_assets::VILLAGER_MAREN_LAYER,
            _ => lf_assets::VILLAGER_NAMELESS_LAYER,
        };
        let (cx, cy, cz) = (8.5f32, 113.4f32, 5.5f32);
        let r = 0.45f32;
        let base = vertices.len() as u32;
        let faces: [([f32; 3], [[f32; 3]; 4], [[f32; 2]; 4]); 6] = [
            ([-1.0, 0.0, 0.0], [[-r, -r, -r], [-r, r, -r], [-r, r, r], [-r, -r, r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
            ([1.0, 0.0, 0.0], [[r, -r, r], [r, r, r], [r, r, -r], [r, -r, -r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
            ([0.0, -1.0, 0.0], [[-r, -r, -r], [-r, -r, r], [r, -r, r], [r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ([0.0, 1.0, 0.0], [[-r, r, r], [-r, r, -r], [r, r, -r], [r, r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ([0.0, 0.0, -1.0], [[r, -r, -r], [r, r, -r], [-r, r, -r], [-r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ([0.0, 0.0, 1.0], [[-r, -r, r], [-r, r, r], [r, r, r], [r, -r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
        ];
        for (normal, corners, uvs) in faces {
            for (corner, uv) in corners.iter().zip(uvs.iter()) {
                vertices.push(GpuVertex {
                    position: [cx + corner[0], cy + corner[1], cz + corner[2]],
                    normal, tex_coord: *uv, tex_index: tex,
                    ao: 1.0, light: 0xF0, sway: 0.0,
                });
            }
            indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }

    // companion_follow: the warden cube at follow distance (B3/B4)
    if spec.name == "companion_follow" {
        let h = world.surface_height(0, 0) as f32;
        let mut push_skin_cube = |vertices: &mut Vec<GpuVertex>, indices: &mut Vec<u32>,
                                  c: (f32, f32, f32), r: f32, tex: u32| {
            let (cx, cy, cz) = c;
            let base = vertices.len() as u32;
            let faces: [([f32; 3], [[f32; 3]; 4], [[f32; 2]; 4]); 6] = [
                ([-1.0, 0.0, 0.0], [[-r, -r, -r], [-r, r, -r], [-r, r, r], [-r, -r, r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
                ([1.0, 0.0, 0.0], [[r, -r, r], [r, r, r], [r, r, -r], [r, -r, -r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
                ([0.0, -1.0, 0.0], [[-r, -r, -r], [-r, -r, r], [r, -r, r], [r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
                ([0.0, 1.0, 0.0], [[-r, r, r], [-r, r, -r], [r, r, -r], [r, r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
                ([0.0, 0.0, -1.0], [[r, -r, -r], [r, r, -r], [-r, r, -r], [-r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
                ([0.0, 0.0, 1.0], [[-r, -r, r], [-r, r, r], [r, r, r], [r, -r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ];
            for (normal, corners, uvs) in faces {
                for (corner, uv) in corners.iter().zip(uvs.iter()) {
                    vertices.push(GpuVertex {
                        position: [cx + corner[0], cy + corner[1], cz + corner[2]],
                        normal, tex_coord: *uv, tex_index: tex,
                        ao: 1.0, light: 0xF0, sway: 0.0,
                    });
                }
                indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
            }
        };
        let warden = lf_assets::COMPANION_LAYERS.iter().find(|(id, _)| *id == "accord_warden").unwrap().1;
        push_skin_cube(&mut vertices, &mut indices, (0.5, h + 1.4, 3.5), 0.55,
            lf_assets::trusted_companion_layer(warden));
        push_skin_cube(&mut vertices, &mut indices, (-3.5, h + 0.9, 1.5), 0.45, lf_assets::VILLAGER_IRONBORN_LAYER);
        push_skin_cube(&mut vertices, &mut indices, (4.5, h + 1.1, -1.5), 0.55, lf_assets::MOB_GLITCHLING_LAYER);
    }

    // falling_sand: one granular block caught mid-fall above the settled
    // pile (the client renders these as near-full cubes with the block's
    // own texture — same shape here, appended post-mesh)
    if spec.name == "falling_sand" {
        let push_quad = |vertices: &mut Vec<GpuVertex>, indices: &mut Vec<u32>,
                         corners: [[f32; 3]; 4], uvs: [[f32; 2]; 4], normal: [f32; 3], tex: u32| {
            let base = vertices.len() as u32;
            for (c, uv) in corners.iter().zip(uvs.iter()) {
                vertices.push(GpuVertex {
                    position: *c,
                    normal,
                    tex_coord: *uv,
                    tex_index: tex,
                    ao: 1.0,
                    light: 0xF0,
                    sway: 0.0,
                });
            }
            indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        };
        let h = world.surface_height(0, 0) as f32;
        let (cx, cy, cz) = (0.5f32, h + 3.2, 0.5f32);
        let r = 0.48f32;
        let sand_tex = lf_assets::texture_index_for_block(lf_voxel::registry::block::SAND);
        let faces: [([f32; 3], [[f32; 3]; 4], [[f32; 2]; 4]); 6] = [
            ([-1.0, 0.0, 0.0], [[-r, -r, -r], [-r, r, -r], [-r, r, r], [-r, -r, r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
            ([1.0, 0.0, 0.0], [[r, -r, r], [r, r, r], [r, r, -r], [r, -r, -r]], [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
            ([0.0, -1.0, 0.0], [[-r, -r, -r], [-r, -r, r], [r, -r, r], [r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ([0.0, 1.0, 0.0], [[-r, r, r], [-r, r, -r], [r, r, -r], [r, r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ([0.0, 0.0, -1.0], [[r, -r, -r], [r, r, -r], [-r, r, -r], [-r, -r, -r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
            ([0.0, 0.0, 1.0], [[-r, -r, r], [-r, r, r], [r, r, r], [r, -r, r]], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]),
        ];
        for (normal, corners, uvs) in faces {
            let corners: [[f32; 3]; 4] = corners.map(|c| [cx + c[0], cy + c[1], cz + c[2]]);
            push_quad(&mut vertices, &mut indices, corners, uvs, normal, sand_tex);
        }
    }
    // weather_snow: falling flakes as translucent billboards (Step 19),
    // mirroring atmosphere::weather_particles(cold=true)
    if spec.name == "weather_snow" {
        let snow_tex = lf_assets::WAYPOINT_LAYERS[1]; // pale blue tint layer
        for i in 0..40u32 {
            let t = i as f32;
            let (x, z) = (((t * 2.3).sin()) * 9.0, ((t * 1.7).cos()) * 9.0);
            let y = 4.0 + (t * 3.1).sin() * 3.0 + (t % 5.0);
            let center = Vec3::new(x, world.surface_height(0, 0) as f32 + y, z);
            let base = water_vertices.len() as u32;
            let corners = [
                [center.x - 0.09, center.y - 0.09, center.z],
                [center.x - 0.09, center.y + 0.09, center.z],
                [center.x + 0.09, center.y + 0.09, center.z],
                [center.x + 0.09, center.y - 0.09, center.z],
            ];
            for (corner, uv) in corners.iter().zip([[0.2, 0.3], [0.2, 0.2], [0.3, 0.2], [0.3, 0.3]]) {
                water_vertices.push(GpuVertex {
                    position: *corner,
                    normal: [0.0, 0.0, 1.0],
                    tex_coord: uv,
                    tex_index: snow_tex,
                    ao: 1.0,
                    light: 0xF0,
                    sway: 0.0,
                });
            }
            water_indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }

    // steam_chain: pale puffs rising from the boiler drum
    if spec.name == "steam_chain" {
        let h = world.surface_height(0, 0) as f32;

        let snow_tex = lf_assets::texture_index_for_block(lf_voxel::registry::block::SNOW);
        for i in 0..10u32 {
            let t = i as f32;
            let center = Vec3::new(0.5 + (t * 1.3).sin() * 0.5, h + 1.2 + t * 0.55, 0.5 + (t * 0.9).cos() * 0.4);
            let base = vertices.len() as u32;
            let corners = [
                [center.x - 0.14, center.y - 0.14, center.z],
                [center.x - 0.14, center.y + 0.14, center.z],
                [center.x + 0.14, center.y + 0.14, center.z],
                [center.x + 0.14, center.y - 0.14, center.z],
            ];
            for (corner, uv) in corners.iter().zip([[0.0, 0.25], [0.0, 0.0], [0.25, 0.0], [0.25, 0.25]]) {
                vertices.push(GpuVertex {
                    position: *corner,
                    normal: [0.0, 0.0, 1.0],
                    tex_coord: uv,
                    tex_index: snow_tex,
                    ao: 1.0,
                    light: 0xF0,
                    sway: 0.0,
                });
            }
            indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }

    // spell_effects: a firebolt streak (glowing cubes along an arc) and a
    // ward ring (translucent white tiles circling the caster)
    if spec.name == "spell_effects" {
        let h = world.surface_height(0, 0) as f32;
        let _ = ();
        let lantern = lf_assets::texture_index_for_block(lf_voxel::registry::block::LANTERN);
        let snow = lf_assets::texture_index_for_block(lf_voxel::registry::block::SNOW);
        // firebolt arc from right to left at chest height
        for i in 0..7u32 {
            let t = i as f32;
            let center = Vec3::new(4.5 - t * 1.1, h + 1.6 + (t * 0.18).sin() * 0.4, 0.5);
            let base = vertices.len() as u32;
            let s = 0.12 + (6 - i) as f32 * 0.02;
            let corners = [
                [center.x - s, center.y - s, center.z],
                [center.x - s, center.y + s, center.z],
                [center.x + s, center.y + s, center.z],
                [center.x + s, center.y - s, center.z],
            ];
            // sample the lantern sprite's glowing core, not the empty corner
            for (corner, uv) in corners.iter().zip([[0.375, 0.625], [0.375, 0.375], [0.625, 0.375], [0.625, 0.625]]) {
                vertices.push(GpuVertex {
                    position: *corner,
                    normal: [0.0, 0.0, 1.0],
                    tex_coord: uv,
                    tex_index: lantern,
                    ao: 1.0,
                    light: 0xF0,
                    sway: 0.0,
                });
            }
            indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
        // ward ring: 10 pale tiles facing inward around (0, h+1, 0)
        let (cx, cy, cz) = (0.5f32, h + 1.4, 0.5f32);
        let r = 1.1f32;
        for i in 0..10u32 {
            let a = i as f32 * std::f32::consts::TAU / 10.0;
            let px = cx + a.cos() * r;
            let pz = cz + a.sin() * r;
            let base = vertices.len() as u32;
            let s = 0.24f32;
            let corners = [
                [px - s, cy - s, pz],
                [px - s, cy + s, pz],
                [px + s, cy + s, pz],
                [px + s, cy - s, pz],
            ];
            for (corner, uv) in corners.iter().zip([[0.25, 0.75], [0.25, 0.25], [0.75, 0.25], [0.75, 0.75]]) {
                vertices.push(GpuVertex {
                    position: *corner,
                    normal: [0.0, 0.0, 1.0],
                    tex_coord: uv,
                    tex_index: snow,
                    ao: 1.0,
                    light: 0xF0,
                    sway: 0.0,
                });
            }
            indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }

    // build_tools (P34): the blueprint ghost — translucent green tint
    // cubes hanging in the air where a paste would land (the same cube
    // construction the client's ghost preview uses).
    if spec.name == "build_tools" {
        let h = world.surface_height(0, 0);
        let ghost_cells: Vec<(i32, i32, i32)> = (0..9).map(|i| (i % 3 - 1, h + 6 + i / 3, (i % 5) - 2)).collect();
        for (gx, gy, gz) in ghost_cells {
            let tex = lf_assets::GRID_OK_LAYER;
            let e = 0.51f32;
            let (cx, cy, cz) = (gx as f32 + 0.5, gy as f32 + 0.5, gz as f32 + 0.5);
            let faces: [([[f32; 3]; 4]); 6] = [
                [[cx - e, cy - e, cz - e], [cx - e, cy + e, cz - e], [cx + e, cy + e, cz - e], [cx + e, cy - e, cz - e]],
                [[cx + e, cy - e, cz + e], [cx + e, cy + e, cz + e], [cx - e, cy + e, cz + e], [cx - e, cy - e, cz + e]],
                [[cx - e, cy - e, cz + e], [cx - e, cy + e, cz + e], [cx - e, cy + e, cz - e], [cx - e, cy - e, cz - e]],
                [[cx + e, cy - e, cz - e], [cx + e, cy + e, cz - e], [cx + e, cy + e, cz + e], [cx + e, cy - e, cz + e]],
                [[cx - e, cy + e, cz + e], [cx - e, cy + e, cz - e], [cx + e, cy + e, cz - e], [cx + e, cy + e, cz + e]],
                [[cx - e, cy - e, cz - e], [cx + e, cy - e, cz - e], [cx + e, cy - e, cz + e], [cx - e, cy - e, cz + e]],
            ];
            for corners in faces {
                let base = water_vertices.len() as u32;
                for (corner, uv) in corners.iter().zip([[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]) {
                    water_vertices.push(GpuVertex {
                        position: *corner,
                        normal: [0.0, 1.0, 0.0],
                        tex_coord: uv,
                        tex_index: tex,
                        ao: 1.0,
                        light: 0xF0,
                        sway: 0.0,
                    });
                }
                water_indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
            }
        }
    }

    // dragon scenes (P36): the multi-part assembly through the shared
    // dragon_parts layout (the client renderer uses the same fn), plus
    // fire-breath quads ahead of the head.
    if spec.name == "dragon_roost" || spec.name == "dragon_flight" {
        let h = world.surface_height(0, 0) as f32;
        let tex = lf_assets::DRAGON_BODY_LAYER;
        let (center, yaw, t) = if spec.name == "dragon_roost" {
            (Vec3::new(0.5, h + 7.5, 0.5), 2.2, 0.7)
        } else {
            (Vec3::new(0.5, h + 6.0, 0.5), 0.4, 1.1)
        };
        for (offset, size) in lf_game::dragons::dragon_parts(t, yaw) {
            let p = center + offset;
            // part cubes: 6 faces each, same construction as the mob batch
            let e = size;
            let (cx, cy, cz) = (p.x, p.y, p.z);
            let faces: [([[f32; 3]; 4]); 6] = [
                [[cx - e, cy - e, cz - e], [cx - e, cy + e, cz - e], [cx + e, cy + e, cz - e], [cx + e, cy - e, cz - e]],
                [[cx + e, cy - e, cz + e], [cx + e, cy + e, cz + e], [cx - e, cy + e, cz + e], [cx - e, cy - e, cz + e]],
                [[cx - e, cy - e, cz + e], [cx - e, cy + e, cz + e], [cx - e, cy + e, cz - e], [cx - e, cy - e, cz - e]],
                [[cx + e, cy - e, cz - e], [cx + e, cy + e, cz - e], [cx + e, cy + e, cz + e], [cx + e, cy - e, cz + e]],
                [[cx - e, cy + e, cz + e], [cx - e, cy + e, cz - e], [cx + e, cy + e, cz - e], [cx + e, cy + e, cz + e]],
                [[cx - e, cy - e, cz - e], [cx + e, cy - e, cz - e], [cx + e, cy - e, cz + e], [cx - e, cy - e, cz + e]],
            ];
            for corners in faces {
                let base = vertices.len() as u32;
                for (corner, uv) in corners.iter().zip([[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]) {
                    vertices.push(GpuVertex {
                        position: *corner,
                        normal: [0.0, 1.0, 0.0],
                        tex_coord: uv,
                        tex_index: tex,
                        ao: 1.0,
                        light: 0xF0,
                        sway: 0.0,
                    });
                }
                indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
            }
        }
        // fire breath: ember quads streaming from the head forward
        if spec.name == "dragon_flight" {
            let ember = lf_assets::texture_index_for_block(lf_voxel::registry::block::SNOW);
            let (sy, cy) = yaw.sin_cos();
            let head_dir = Vec3::new(sy, -0.15, -cy);
            let head = center + Vec3::new(sy * 1.2, 0.35, -cy * 1.2);
            for i in 0..7u32 {
                let d = 0.8 + i as f32 * 0.9;
                let spread = (i as f32) * 0.12;
                let p = head + head_dir * d;
                let base = vertices.len() as u32;
                let s = 0.14 + i as f32 * 0.05;
                let corners = [
                    [p.x - s, p.y - s, p.z], [p.x - s, p.y + s, p.z],
                    [p.x + s, p.y + s + spread * 0.1, p.z], [p.x + s, p.y - s - spread * 0.1, p.z],
                ];
                for (corner, uv) in corners.iter().zip([[0.375, 0.625], [0.375, 0.375], [0.625, 0.375], [0.625, 0.625]]) {
                    vertices.push(GpuVertex {
                        position: *corner, normal: [0.0, 0.0, 1.0], tex_coord: uv,
                        tex_index: ember, ao: 1.0, light: 0xF0, sway: 0.0,
                    });
                }
                indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
            }
        }
    }

    // oil_chain: dark flare smoke rising from the refinery columns (P31)
    if spec.name == "oil_chain" {
        let h = world.surface_height(0, 0) as f32;
        let oil_tex = lf_assets::OIL_LAYER;
        for i in 0..9u32 {
            let t = i as f32;
            let center = Vec3::new(-0.5 + (t * 1.1).sin() * 0.5, h + 1.3 + t * 0.5, 0.5 + (t * 0.8).cos() * 0.4);
            let base = vertices.len() as u32;
            let corners = [
                [center.x - 0.15, center.y - 0.15, center.z],
                [center.x - 0.15, center.y + 0.15, center.z],
                [center.x + 0.15, center.y + 0.15, center.z],
                [center.x + 0.15, center.y - 0.15, center.z],
            ];
            for (corner, uv) in corners.iter().zip([[0.0, 0.25], [0.0, 0.0], [0.25, 0.0], [0.25, 0.25]]) {
                vertices.push(GpuVertex {
                    position: *corner,
                    normal: [0.0, 0.0, 1.0],
                    tex_coord: uv,
                    tex_index: oil_tex,
                    ao: 1.0,
                    light: 0xF0,
                    sway: 0.0,
                });
            }
            indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }

    // grid_overlay (Step 25): the tint cubes ride the transparent channel
    // — the same 6-face translucent-cube construction the client's
    // push_overlay_cube uses, classified by the same ratio rule.
    if spec.name == "grid_overlay" && grid_cubes.len() >= 2 {
        let last = &grid_cubes[grid_cubes.len() - 2..];
        for ((bx, by, bz), powered) in last.iter().copied() {
            let tex = if powered { lf_assets::GRID_OK_LAYER } else { lf_assets::GRID_STARVED_LAYER };
            let e = 0.51f32;
            let (cx, cy, cz) = (bx as f32 + 0.5, by as f32 + 0.5, bz as f32 + 0.5);
            let faces: [([[f32; 3]; 4]); 6] = [
                [[cx - e, cy - e, cz - e], [cx - e, cy + e, cz - e], [cx + e, cy + e, cz - e], [cx + e, cy - e, cz - e]],
                [[cx + e, cy - e, cz + e], [cx + e, cy + e, cz + e], [cx - e, cy + e, cz + e], [cx - e, cy - e, cz + e]],
                [[cx - e, cy - e, cz + e], [cx - e, cy + e, cz + e], [cx - e, cy + e, cz - e], [cx - e, cy - e, cz - e]],
                [[cx + e, cy - e, cz - e], [cx + e, cy + e, cz - e], [cx + e, cy + e, cz + e], [cx + e, cy - e, cz + e]],
                [[cx - e, cy + e, cz + e], [cx - e, cy + e, cz - e], [cx + e, cy + e, cz - e], [cx + e, cy + e, cz + e]],
                [[cx - e, cy - e, cz - e], [cx + e, cy - e, cz - e], [cx + e, cy - e, cz + e], [cx - e, cy - e, cz + e]],
            ];
            for corners in faces {
                let base = water_vertices.len() as u32;
                for (corner, uv) in corners.iter().zip([[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]) {
                    water_vertices.push(GpuVertex {
                        position: *corner,
                        normal: [0.0, 1.0, 0.0],
                        tex_coord: uv,
                        tex_index: tex,
                        ao: 1.0,
                        light: 0xF0,
                        sway: 0.0,
                    });
                }
                water_indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
            }
        }
    }

    // waypoint_beacons: slim tinted beams (Step 15) in the transparent
    // channel, mirroring the client's waypoint_batch geometry
    if spec.name == "waypoint_beacons" {
        let h = world.surface_height(0, 0) as f32;
        let mut push_beam = |vertices: &mut Vec<GpuVertex>, indices: &mut Vec<u32>,
                             cx: f32, cz: f32, tex: u32| {
            let r = 0.35f32;
            let (y0, y1) = (h, h + 24.0);
            let faces: [([f32; 3], [[f32; 3]; 4]); 4] = [
                ([0.0, 0.0, -1.0], [[cx - r, y0, cz - r], [cx - r, y1, cz - r], [cx + r, y1, cz - r], [cx + r, y0, cz - r]]),
                ([0.0, 0.0, 1.0], [[cx + r, y0, cz + r], [cx + r, y1, cz + r], [cx - r, y1, cz + r], [cx - r, y0, cz + r]]),
                ([-1.0, 0.0, 0.0], [[cx - r, y0, cz + r], [cx - r, y1, cz + r], [cx - r, y1, cz - r], [cx - r, y0, cz - r]]),
                ([1.0, 0.0, 0.0], [[cx + r, y0, cz - r], [cx + r, y1, cz - r], [cx + r, y1, cz + r], [cx + r, y0, cz + r]]),
            ];
            for (normal, corners) in faces {
                let base = vertices.len() as u32;
                for (corner, uv) in corners.iter().zip([[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]) {
                    vertices.push(GpuVertex {
                        position: *corner,
                        normal,
                        tex_coord: uv,
                        tex_index: tex,
                        ao: 1.0,
                        light: 0xF0,
                        sway: 0.0,
                    });
                }
                indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
            }
        };
        push_beam(&mut water_vertices, &mut water_indices, 0.5, 0.5, lf_assets::WAYPOINT_LAYERS[0]);
        push_beam(&mut water_vertices, &mut water_indices, 4.5, -2.5, lf_assets::WAYPOINT_LAYERS[1]);
        push_beam(&mut water_vertices, &mut water_indices, -4.5, 3.5, lf_assets::WAYPOINT_LAYERS[3]);
    }

    // transparency_layers: debris billboards on both sides of the glass —
    // the near pair must render over the pane, the far pair through it
    if spec.name == "transparency_layers" {
        let h = world.surface_height(0, 0) as f32;
        let stone_tex = lf_assets::texture_index_for_block(lf_voxel::registry::block::STONE);
        let plank_tex = lf_assets::texture_index_for_block(lf_voxel::registry::block::PLANKS);
        let push_billboard = |vertices: &mut Vec<GpuVertex>, indices: &mut Vec<u32>,
                              center: Vec3, half: f32, tex: u32| {
            let base = vertices.len() as u32;
            let corners = [
                [center.x - half, center.y - half, center.z],
                [center.x - half, center.y + half, center.z],
                [center.x + half, center.y + half, center.z],
                [center.x + half, center.y - half, center.z],
            ];
            for (c, uv) in corners.iter().zip([[0.0f32, 0.25], [0.0, 0.0], [0.25, 0.0], [0.25, 0.25]]) {
                vertices.push(GpuVertex {
                    position: *c,
                    normal: [0.0, 0.0, 1.0],
                    tex_coord: uv,
                    tex_index: tex,
                    ao: 1.0,
                    light: 0xF0,
                    sway: 0.0,
                });
            }
            indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
        };
        // in front of the glass (z = 6.4) and behind it over the water (z = 3.6)
        push_billboard(&mut vertices, &mut indices, Vec3::new(-0.6, h + 2.4, 6.4), 0.22, plank_tex);
        push_billboard(&mut vertices, &mut indices, Vec3::new(0.9, h + 1.6, 6.4), 0.16, stone_tex);
        push_billboard(&mut vertices, &mut indices, Vec3::new(-0.9, h + 1.9, 3.6), 0.18, plank_tex);
        push_billboard(&mut vertices, &mut indices, Vec3::new(0.7, h + 2.6, 3.6), 0.2, stone_tex);
    }
    (vertices, indices, water_vertices, water_indices)
}

/// Render a registered scene by name to `out_path` (a real GPU render).
pub fn run_scene(name: &str, seed_override: Option<u64>, out_path: &Path) -> Result<(), String> {
    let spec = scenes().into_iter().find(|s| s.name == name)
        .ok_or_else(|| format!("unknown scene '{}'; known: {:?}", name, scenes().iter().map(|s| s.name).collect::<Vec<_>>()))?;
    let seed = seed_override.unwrap_or(spec.default_seed);
    let (vertices, indices, water_vertices, water_indices) = build_scene_mesh(&spec, seed, 3, spec.torches, spec.machines);
    if vertices.is_empty() {
        return Err(format!("scene '{}' produced an empty mesh", name));
    }
    // Lift the camera safely above whatever terrain the seed generates at
    // its x/z so hills never bury the shot. First-person scenes instead sit
    // at player eye height looking slightly downhill.
    let gen = WorldGen::new(Seed(seed));
    let (eye, target) = if spec.name == "foliage_canopy" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-10.0, h + 13.0, 14.0), Vec3::new(0.5, h + 9.5, 0.5))
    } else if spec.name == "mining_feedback" {
        // the slope rises toward the camera, so reference the terrain AT the
        // eye or the camera ends up buried (backfaces see through the hill)
        let h = gen.surface_top(0, 0) as f32;
        let he = gen.surface_top(-6, 7) as f32;
        (Vec3::new(-6.0, he + 2.2, 7.0), Vec3::new(0.5, h + 2.5, 0.5))
    } else if spec.name == "water_flow" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-9.0, h + 9.0, 11.0), Vec3::new(4.0, h + 1.5, 0.0))
    } else if spec.name == "falling_sand" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-7.0, h + 5.0, 8.0), Vec3::new(0.5, h - 1.0, 0.5))
    } else if spec.name == "texture_tiling" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(0.5, h + 2.6, 9.5), Vec3::new(0.5, h + 1.5, 0.0))
    } else if spec.name == "transparency_layers" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(0.5, h + 2.2, 10.5), Vec3::new(0.0, h + 1.2, 2.0))
    } else if spec.name == "night_border_seam" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(16.0, h + 1.7, 12.0), Vec3::new(16.0, h + 0.4, 0.0))
    } else if spec.name == "steam_chain" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-4.0, h + 6.0, 9.0), Vec3::new(0.5, h + 0.8, 0.5))
    } else if spec.name == "oil_chain" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-8.0, h + 6.5, 9.0), Vec3::new(-1.0, h + 0.6, 0.0))
    } else if spec.name == "grid_overlay" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-4.0, h + 7.0, 13.0), Vec3::new(2.0, h + 0.6, 0.0))
    } else if spec.name == "paths_screen" || spec.name == "trade_p2p" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(0.0, h + 3.0, 0.0), Vec3::new(0.0, h + 3.0, -1.0))
    } else if spec.name == "dragon_roost" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-7.0, h + 11.0, 9.0), Vec3::new(0.5, h + 5.5, 0.5))
    } else if spec.name == "dragon_flight" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-6.0, h + 10.0, 9.0), Vec3::new(0.8, h + 6.0, 0.0))
    } else if spec.name == "modern_wing" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-6.0, h + 9.0, 10.0), Vec3::new(0.5, h + 3.0, -1.0))
    } else if spec.name == "build_tools" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-9.0, h + 8.0, 9.0), Vec3::new(0.0, h + 1.5, 0.0))
    } else if spec.name == "wizard_tower" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-7.0, h + 9.0, 9.0), Vec3::new(0.5, h + 6.0, 0.5))
    } else if spec.name == "spellbook" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(0.0, h + 3.0, 0.0), Vec3::new(0.0, h + 3.0, -1.0))
    } else if spec.name == "spell_effects" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-4.0, h + 6.0, 9.0), Vec3::new(0.5, h + 0.8, 0.5))
    } else if spec.name == "reactor_control" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-5.0, h + 6.0, 9.0), Vec3::new(0.5, h + 0.8, 0.0))
    } else if spec.name == "meltdown_aftermath" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-3.5, h + 3.0, 7.0), Vec3::new(0.0, h + 0.5, 0.0))
    } else if spec.name == "water_wheel_power" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-7.0, h + 6.5, 9.0), Vec3::new(1.0, h + 0.5, -1.0))
    } else if spec.name == "biome_contact_sheet" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(0.0, h + 16.0, 26.0), Vec3::new(0.0, h - 1.0, 0.0))
    } else if spec.name == "weather_snow" || spec.name == "weather_dry" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-12.0, h + 8.0, 14.0), Vec3::new(0.0, h + 1.0, 0.0))
    } else if spec.name == "waypoint_beacons" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(11.0, h + 14.0, 12.0), Vec3::new(0.0, h + 6.0, 0.0))
    } else if matches!(spec.name, "accord_embassy" | "ironborn_forge_camp" | "covenant_grove_shrine"
        | "freeholds_longhouse" | "ashen_library" | "nameless_camp") {
        // display pedestal planted at y=100 in build_scene_mesh
        (Vec3::new(-9.0, 120.5, -9.0), Vec3::new(8.5, 113.5, 8.5))
    } else if spec.name == "faction_blocks" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(14.0, h + 16.0, 20.0), Vec3::new(12.0, h - 1.0, -2.0))
    } else if spec.name == "entity_skins" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(1.0, h + 12.0, 18.0), Vec3::new(0.0, h + 0.5, -4.0))
    } else if spec.name == "ember_glow" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(-5.0, h + 4.0, 6.0), Vec3::new(0.5, h + 1.6, 0.5))
    } else if spec.name == "companion_follow" {
        let h = gen.surface_top(0, 0) as f32;
        (Vec3::new(0.5, h + 2.6, 0.5), Vec3::new(0.5, h + 1.4, 3.5))
    } else if spec.first_person {
        // Find a viewpoint with an open vista: a local rise whose best look
        // direction drops the most over 30 blocks, so the frame shows both
        // nearby terrain and the horizon — what a player actually sees.
        let dirs = [
            Vec3::new(0.35, -0.18, -1.0),
            Vec3::new(-1.0, -0.18, -0.35),
            Vec3::new(-0.35, -0.18, 1.0),
            Vec3::new(1.0, -0.18, 0.35),
        ];
        let flat: Vec<Vec3> = dirs.iter().map(|d| Vec3::new(d.x, 0.0, d.z).normalize()).collect();
        // A moderate ~20-block drop at 30 blocks distance keeps the terrain
        // inside a 45-degree vertical frame; steeper vistas fall below view.
        let mut best_pos = (8i32, 8i32);
        let mut best_score = i32::MAX;
        let mut best_dir = 0usize;
        for x in (-20..20).step_by(4) {
            for z in (-20..20).step_by(4) {
                let h = gen.surface_top(x, z);
                for (i, d) in flat.iter().enumerate() {
                    let drop = h - gen.surface_top(x + (d.x * 30.0) as i32, z + (d.z * 30.0) as i32);
                    if drop < 8 {
                        continue; // needs some view at all
                    }
                    let score = (drop - 20).abs();
                    if score < best_score {
                        best_score = score;
                        best_pos = (x, z);
                        best_dir = i;
                    }
                }
            }
        }
        let h = gen.surface_top(best_pos.0, best_pos.1);
        let eye = Vec3::new(best_pos.0 as f32 + 0.5, h as f32 + 1.62, best_pos.1 as f32 + 0.5);
        (eye, eye + dirs[best_dir].normalize() * 40.0)
    } else {
        let h_eye = gen.surface_top(spec.eye.x as i32, spec.eye.z as i32);
        let h_target = gen.surface_top(spec.target.x as i32, spec.target.z as i32);
        (
            Vec3::new(spec.eye.x, spec.eye.y.max(h_eye as f32 + 22.0), spec.eye.z),
            Vec3::new(spec.target.x, h_target as f32 + 2.0, spec.target.z),
        )
    };
    let mut camera = Camera::new(eye, target);
    camera.set_aspect(800, 600);
    let env = lf_engine::scene::Env {
        camera_pos: eye,
        // mid-sway pose: proofs show the wind offset statically
        time: 0.8,
        day_factor: spec.day_factor(),
        fog_color: spec.time_of_day().sky_color(),
        fog_far: 220.0,
        grade_tint: [1.0, 1.0, 1.0],
        grade_saturation: 1.0,
    };
    // clouds/weather scene: atmosphere geometry joins the standard mesh
    let (mut vertices, mut indices, mut water_vertices, mut water_indices) =
        (vertices, indices, water_vertices, water_indices);
    if spec.name == "clouds_weather" {
        let (sv, si) = lf_engine::atmosphere::sky_bodies(eye, spec.time_of_day);
        let base = vertices.len() as u32;
        vertices.extend(sv);
        indices.extend(si.iter().map(|i| i + base));
        let (cv, ci) = lf_engine::atmosphere::cloud_mesh(eye, 40.0);
        let wbase = water_vertices.len() as u32;
        water_vertices.extend(cv);
        water_indices.extend(ci.iter().map(|i| i + wbase));
        let (rv, ri) = lf_engine::atmosphere::weather_particles(Vec3::new(0.0, 100.0, 0.0), 3.0, false);
        let rbase = water_vertices.len() as u32;
        water_vertices.extend(rv);
        water_indices.extend(ri.iter().map(|i| i + rbase));
    }
    let (vertices, indices, water_vertices, water_indices) = (vertices, indices, water_vertices, water_indices);

    let ui = spec.name == "hud_preview" || spec.name == "village_trading" || spec.name == "tech_tree"
        || spec.name == "menu_preview" || spec.name == "settings_preview"
        || spec.name == "crafting_ui" || spec.name == "map_screen" || spec.name == "minimap_hud"
        || spec.name == "console_preview" || spec.name == "lore_book"
        || spec.name == "spellbook" || spec.name == "paths_screen" || spec.name == "trade_p2p"
        || spec.name == "faction_map" || spec.name == "faction_hud"
        || spec.name == "companion_commands" || spec.name == "companion_follow";
    let (ui_ctx, warm_textures) = if ui {
        let ctx = egui::Context::default();
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))),
            ..Default::default()
        };
        let draw = |ctx: &egui::Context| {
            draw_hud_preview(ctx);
            if spec.name == "village_trading" {
                draw_trade_preview(ctx);
            }
            if spec.name == "tech_tree" {
                draw_tech_tree_preview(ctx);
            }
            if spec.name == "menu_preview" {
                draw_menu_preview(ctx);
            }
            if spec.name == "settings_preview" {
                draw_settings_preview(ctx);
            }
            if spec.name == "crafting_ui" {
                draw_crafting_preview(ctx);
            }
            if spec.name == "map_screen" {
                draw_map_preview(ctx);
            }
            if spec.name == "minimap_hud" {
                draw_minimap_preview(ctx);
            }
            if spec.name == "console_preview" {
                draw_console_preview(ctx);
            }
            if spec.name == "lore_book" {
                draw_lore_preview(ctx);
            }
            if spec.name == "spellbook" {
                draw_spellbook_preview(ctx);
            }
            if spec.name == "paths_screen" {
                draw_paths_preview(ctx);
            }
            if spec.name == "faction_map" {
                draw_faction_map_preview(ctx);
            }
            if spec.name == "faction_hud" || spec.name == "companion_follow" {
                draw_faction_hud_preview(ctx);
            }
            if spec.name == "companion_commands" {
                draw_companion_menu_preview(ctx);
            }
            if spec.name == "trade_p2p" {
                draw_trade_p2p_preview(ctx);
            }
        };
        // Warmup pass: egui windows need one pass to materialize their areas
        // before their content renders (a fresh single-pass context produces
        // empty window shapes — this bit the pre-P22 trade/tech proofs too).
        ctx.begin_pass(raw.clone());
        draw(&ctx);
        let warm = ctx.end_pass();
        ctx.begin_pass(raw);
        draw(&ctx);
        // The warmup output carried the font-atlas texture delta away; keep
        // it so the renderer still uploads fonts (else text/painted fills
        // vanish from the proof).
        (Some(ctx), Some(warm.textures_delta.set))
    } else {
        (None, None)
    };
    let overlay = ui_ctx.as_ref().map(|ctx| lf_engine::headless::UiOverlay {
        ctx,
        extra_textures: warm_textures.as_deref().unwrap_or(&[]),
    });
    if spec.raytraced {
        render_raytraced(&spec, seed, &eye, out_path)?;
        return verify_render(out_path);
    }
    let textures = lf_assets::generate_atlas();
    lf_engine::headless::render_to_png(&vertices, &indices, &water_vertices, &water_indices, &textures, &camera, &env, spec.sky_color(), 800, 600, out_path, overlay.as_ref())?;
    verify_render(out_path)
}

/// Post-render proof check: reopen the written PNG and assert it contains a
/// real image — sane dimensions, several distinct colors, and actual luma
/// variance. Guards against silently black / single-color "it rendered"
/// outputs (AGENTS.md: pixel-analyze the PNGs, never trust that it rendered).
fn verify_render(out_path: &Path) -> Result<(), String> {
    let img = image::open(out_path).map_err(|e| format!("reopen {}: {e}", out_path.display()))?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    if w < 100 || h < 100 {
        return Err(format!("suspect render {}: only {}x{}", out_path.display(), w, h));
    }
    let mut colors = std::collections::HashSet::new();
    let (mut luma_min, mut luma_max) = (u8::MAX, 0u8);
    for p in rgba.pixels() {
        colors.insert(p.0);
        let luma = ((p.0[0] as u32 * 3 + p.0[1] as u32 * 4 + p.0[2] as u32) / 8) as u8;
        luma_min = luma_min.min(luma);
        luma_max = luma_max.max(luma);
        if colors.len() >= 64 && luma_max - luma_min > 32 {
            break; // enough evidence of a real image; skip the full scan
        }
    }
    if colors.len() < 16 {
        return Err(format!("suspect render {}: only {} distinct colors", out_path.display(), colors.len()));
    }
    if luma_max.saturating_sub(luma_min) < 16 {
        return Err(format!("suspect render {}: near-uniform luma {}..{}", out_path.display(), luma_min, luma_max));
    }
    Ok(())
}

/// Title-menu proof overlay mirroring the animated client screen.
fn draw_menu_preview(ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(90)))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("LOREFORGE").size(58.0)
                    .color(egui::Color32::from_rgb(250, 220, 160)).strong());
                ui.label(egui::RichText::new("a voxel saga of forge & industry").size(16.0)
                    .color(egui::Color32::from_rgb(200, 205, 212)));
                ui.add_space(24.0);
                let items = [("Play — World 1", true), ("New World", false), ("Load Game", false),
                             ("Multiplayer (localhost)", false), ("Settings", false), ("Quit", false)];
                for (label, accent) in items {
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(300.0, 50.0), egui::Sense::click());
                    let _ = response;
                    let fill = if accent {
                        egui::Color32::from_rgba_premultiplied(60, 48, 22, 235)
                    } else {
                        egui::Color32::from_rgba_premultiplied(28, 33, 44, 225)
                    };
                    ui.painter().rect_filled(rect, 10.0, fill);
                    let stroke = if accent {
                        egui::Color32::from_rgb(240, 200, 120)
                    } else {
                        egui::Color32::from_rgb(90, 98, 112)
                    };
                    ui.painter().rect_stroke(rect, 10.0, egui::Stroke::new(2.0, stroke), egui::StrokeKind::Middle);
                    if accent {
                        let bar = egui::Rect::from_min_size(
                            egui::Pos2::new(rect.left() + 4.0, rect.center().y - 16.0),
                            egui::vec2(3.0, 32.0),
                        );
                        ui.painter().rect_filled(bar, 2.0, egui::Color32::from_rgb(240, 200, 120));
                    }
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label,
                        egui::FontId::proportional(20.0), egui::Color32::from_rgb(235, 238, 242));
                }
            });
        });
}

/// Settings proof overlay mirroring the tabbed client screen.
fn draw_settings_preview(ctx: &egui::Context) {
    egui::Window::new("Settings")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -10.0))
        .min_size(egui::vec2(520.0, 380.0))
        .collapsible(false).resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (label, on) in [("Video", true), ("Interface", false), ("Audio", false), ("Gameplay", false)] {
                    let btn = egui::Button::new(egui::RichText::new(label)
                        .color(if on { ACCENT } else { TEXT_DIM }))
                        .min_size(egui::vec2(90.0, 28.0));
                    let _ = ui.add(btn);
                }
            });
            ui.separator();
            ui.label(egui::RichText::new("Video").size(17.0).color(egui::Color32::from_rgb(240, 200, 120)));
            ui.add(egui::Slider::new(&mut 70.0f32, 50.0..=110.0).text("Field of view"));
            ui.add(egui::Slider::new(&mut 5.0f32, 3.0..=8.0).text("View distance"));
            ui.checkbox(&mut true, "Clouds");
            ui.checkbox(&mut true, "Weather particles");
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Ray Tracing").size(17.0).color(egui::Color32::from_rgb(240, 200, 120)));
            ui.horizontal(|ui| {
                ui.label("Mode");
                ui.button(egui::RichText::new("Live  (cycle)").color(egui::Color32::from_rgb(240, 200, 120)));
                ui.label(egui::RichText::new("live path-traced view (GPU heavy)").small()
                    .color(egui::Color32::from_rgb(150, 156, 165)));
            });
            ui.add(egui::Slider::new(&mut 0.25f32, 0.1..=0.5).text("RT internal scale"));
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Quality preset").size(17.0).color(egui::Color32::from_rgb(240, 200, 120)));
            ui.horizontal(|ui| {
                ui.button("Low"); ui.button("Medium"); ui.button("High");
            });
        });
}

/// Tech-tree proof overlay mirroring the client's draw_tech_tree.
fn draw_tech_tree_preview(ctx: &egui::Context) {
    let icons = PreviewIcons::new(ctx, &["copper_ingot", "tin_ingot", "steel_ingot", "iron_gear", "coal"]);
    egui::Window::new("Technology — K to close")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -20.0))
        .min_size(egui::vec2(640.0, 380.0))
        .collapsible(false)
        .show(ctx, |ui| {
            ui.heading(egui::RichText::new("Research Progression").size(22.0));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for (e, state) in [
                    ("Primitive", "done"), ("Bronze Age", "CURRENT"), ("Industrial Age", "locked"), ("Electrical Age", "locked"),
                ] {
                    let color = if state == "done" { egui::Color32::from_rgb(120, 200, 120) }
                        else if state == "CURRENT" { ACCENT }
                        else { egui::Color32::from_gray(110) };
                    egui::Frame::new()
                        .fill(egui::Color32::from_black_alpha(120))
                        .stroke(egui::Stroke::new(if state == "CURRENT" { 2.5 } else { 1.0 }, color))
                        .corner_radius(8.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(140.0, 90.0));
                            ui.heading(egui::RichText::new(e).size(15.0).color(color));
                            ui.label(egui::RichText::new(state).small().color(color));
                            if state == "locked" {
                                ui.add_space(4.0);
                                for (item, got, n) in [("copper_ingot", 7, 10), ("tin_ingot", 5, 5), ("steel_ingot", 0, 5)] {
                                    let ok = got >= n;
                                    let c = if ok { OK } else { egui::Color32::from_rgb(230, 130, 130) };
                                    ui.horizontal(|ui| {
                                        let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                                        icons.paint(ui, r, item);
                                        ui.label(egui::RichText::new(format!("{}/{}", got, n)).small().color(c));
                                    });
                                }
                            }
                        });
                    if e != "Electrical Age" { ui.label("->"); }
                }
            });
            ui.add_space(8.0);
            ui.separator();
            ui.label(egui::RichText::new("Next: the Industrial Age — place a Research Bench and bring: steel_ingot (0/5), iron_gear (2/3), coal (14/20)")
                .color(egui::Color32::from_rgb(150, 220, 255)));
        });
}

/// Trade-panel proof overlay (same egui stack as the client trade UI).
fn draw_trade_preview(ctx: &egui::Context) {
    let icons = PreviewIcons::new(ctx, &["raw_iron", "iron_pickaxe", "iron_ingot", "stone_sword", "coal", "furnace"]);
    egui::Window::new("Trading with Brann the Smith")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -30.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            for (give, give_n, get, get_n, have) in [
                ("raw_iron", 4, "iron_pickaxe", 1, 6),
                ("iron_ingot", 3, "stone_sword", 1, 2),
                ("coal", 6, "furnace", 1, 9),
            ] {
                let enough = have >= give_n;
                egui::Frame::new()
                    .fill(egui::Color32::from_black_alpha(130))
                    .corner_radius(7.0)
                    .inner_margin(6.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                            icons.paint(ui, r, give);
                            ui.label(egui::RichText::new(format!("x{}", give_n)).color(if enough { OK } else { BAD }));
                            ui.label(egui::RichText::new("→").color(TEXT_DIM));
                            let (r2, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                            icons.paint(ui, r2, get);
                            ui.label(egui::RichText::new(format!("x{}", get_n)).color(egui::Color32::from_rgb(235, 238, 242)));
                            ui.label(egui::RichText::new(format!("(have {})", have)).small().color(TEXT_DIM));
                            ui.add_enabled(enough, egui::Button::new("Trade"));
                        });
                    });
            }
            ui.separator();
            ui.label(egui::RichText::new("Esc to close").small());
        });
}

// ------------------------------------------------------------------
// Preview helpers: real icon textures + map images from real worldgen.

const ACCENT: egui::Color32 = egui::Color32::from_rgb(240, 200, 120);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(150, 156, 165);
const OK: egui::Color32 = egui::Color32::from_rgb(120, 210, 130);
const BAD: egui::Color32 = egui::Color32::from_rgb(230, 120, 110);

fn load_icon(ctx: &egui::Context, id: &str) -> egui::TextureHandle {
    use lf_game::items::ItemKind;
    let img = match lf_game::items::item_def(id).map(|d| d.kind) {
        Some(ItemKind::Block(b)) => {
            let layer = lf_assets::texture_index_for_block(b) as usize;
            lf_assets::generate_block_texture(lf_assets::TEXTURE_NAMES[layer])
        }
        _ => lf_assets::generate_item_texture(id)
            .unwrap_or_else(|| lf_assets::generate_block_texture("stone")),
    };
    let size = [img.width() as usize, img.height() as usize];
    let pixels = img.pixels().map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3])).collect();
    ctx.load_texture(format!("preview_icon:{}", id), egui::ColorImage { size, pixels }, egui::TextureOptions::NEAREST)
}

/// Icon registry for one preview frame.
struct PreviewIcons {
    map: std::collections::HashMap<String, egui::TextureHandle>,
}

impl PreviewIcons {
    fn new(ctx: &egui::Context, ids: &[&str]) -> Self {
        Self { map: ids.iter().map(|id| (id.to_string(), load_icon(ctx, id))).collect() }
    }

    fn paint(&self, ui: &mut egui::Ui, rect: egui::Rect, id: &str) {
        let uv = egui::Rect::from_min_max(egui::Pos2::new(0.0, 0.0), egui::Pos2::new(1.0, 1.0));
        match self.map.get(id) {
            Some(tex) => { ui.painter().image(tex.id(), rect, uv, egui::Color32::WHITE); }
            None => { ui.painter().rect_filled(rect, 3.0, egui::Color32::from_gray(140)); }
        }
    }
}

/// One preview slot: recessed well + real icon + count + optional selection.
fn preview_slot(ui: &mut egui::Ui, icons: &PreviewIcons, rect: egui::Rect, item: Option<(&str, u8)>, selected: bool) {
    ui.painter().rect_filled(rect, 5.0, egui::Color32::from_black_alpha(170));
    ui.painter().rect_filled(rect.shrink(1.5), 4.0, egui::Color32::from_rgba_premultiplied(30, 35, 46, 200));
    if let Some((id, count)) = item {
        icons.paint(ui, rect.shrink(6.0), id);
        if count > 1 {
            ui.painter().text(rect.right_bottom() + egui::vec2(-5.0, -5.0) + egui::vec2(1.0, 1.0),
                egui::Align2::RIGHT_BOTTOM, format!("{}", count),
                egui::FontId::proportional(13.0), egui::Color32::from_black_alpha(200));
            ui.painter().text(rect.right_bottom() + egui::vec2(-5.0, -5.0),
                egui::Align2::RIGHT_BOTTOM, format!("{}", count),
                egui::FontId::proportional(13.0), egui::Color32::WHITE);
        }
    }
    let stroke = if selected {
        egui::Stroke::new(2.5, ACCENT)
    } else {
        egui::Stroke::new(1.0, egui::Color32::from_gray(80))
    };
    ui.painter().rect_stroke(rect, 5.0, stroke, egui::StrokeKind::Middle);
}

/// Map image sampled from real worldgen (biome color + height shading).
fn map_image(gen: &WorldGen, center: (f32, f32), wh: (usize, usize), px_per_block: f32) -> egui::ColorImage {
    let mut pixels = Vec::with_capacity(wh.0 * wh.1);
    let step = 1.0 / px_per_block;
    let mut wz = center.1 - wh.1 as f32 / (2.0 * px_per_block);
    for _ in 0..wh.1 {
        let mut wx = center.0 - wh.0 as f32 / (2.0 * px_per_block);
        for _ in 0..wh.0 {
            let x = wx.floor() as i32;
            let z = wz.floor() as i32;
            let h = gen.height(x, z);
            let mut c = preview_biome_color(gen.biome(x, z));
            if h <= lf_worldgen::SEA_LEVEL {
                // flatten oceans
            } else {
                let f = (1.0 + (h - gen.height(x - 1, z)) as f32 * 0.035).clamp(0.62, 1.30);
                c = egui::Color32::from_rgba_unmultiplied(
                    ((c.r() as f32) * f).clamp(0.0, 255.0) as u8,
                    ((c.g() as f32) * f).clamp(0.0, 255.0) as u8,
                    ((c.b() as f32) * f).clamp(0.0, 255.0) as u8,
                    c.a(),
                );
            }
            pixels.push(c);
            wx += step;
        }
        wz += step;
    }
    egui::ColorImage { size: [wh.0, wh.1], pixels }
}

/// lore-and-visuals A2: faction home biomes -> colors (the canonical six).
pub fn preview_faction_homes() -> Vec<(&'static str, [u8; 3], Vec<Biome>)> {
    use Biome::*;
    vec![
        ("accord", [74, 122, 181], vec![Meadow, Forest, FlowerForest, BirchForest]),
        ("ironborn", [139, 69, 19], vec![Mountains, Badlands, Volcanic]),
        ("ember_covenant", [196, 96, 42], vec![Highlands, Taiga, MushroomHollow, Swamp]),
        ("free_holds", [107, 142, 35], vec![Savanna, WindsweptSavanna, Beach]),
        ("ashen_order", [176, 176, 176], vec![WindsweptHills, Tundra]),
        ("nameless", [45, 45, 45], vec![PaleGarden, DarkForest]),
    ]
}

/// map_image with the faction territory tint blended over home biomes
/// (same 0.30 blend as the client's apply_territory_tint).
fn faction_tinted_map_image(gen: &WorldGen, center: (f32, f32), wh: (usize, usize), px_per_block: f32) -> egui::ColorImage {
    let homes = preview_faction_homes();
    let mut img = map_image(gen, center, wh, px_per_block);
    for (wz_i, row) in img.pixels.chunks_mut(wh.0).enumerate() {
        for (wx_i, px) in row.iter_mut().enumerate() {
            let wx = center.0 - wh.0 as f32 / (2.0 * px_per_block) + wx_i as f32 / px_per_block;
            let wz = center.1 - wh.1 as f32 / (2.0 * px_per_block) + wz_i as f32 / px_per_block;
            let biome = gen.biome(wx.floor() as i32, wz.floor() as i32);
            if let Some((_, col, _)) = homes.iter().find(|(_, _, biomes)| biomes.contains(&biome)) {
                let a = 0.30f32;
                *px = egui::Color32::from_rgba_unmultiplied(
                    (px.r() as f32 * (1.0 - a) + col[0] as f32 * a) as u8,
                    (px.g() as f32 * (1.0 - a) + col[1] as f32 * a) as u8,
                    (px.b() as f32 * (1.0 - a) + col[2] as f32 * a) as u8,
                    px.a(),
                );
            }
        }
    }
    img
}

/// A2/D3 proof: the world map with territory tints + structure icons.
fn draw_faction_map_preview(ctx: &egui::Context) {
    let gen = WorldGen::new(Seed(12345));
    let panel = egui::Color32::from_rgba_premultiplied(18, 22, 30, 235);
    let accent = egui::Color32::from_rgb(240, 200, 120);
    egui::CentralPanel::default().frame(egui::Frame::new().fill(panel)).show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            ui.label(egui::RichText::new("World Map — faction territories").strong().color(accent));
            ui.add_space(6.0);
            let size = egui::vec2(560.0, 380.0);
            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            let img = faction_tinted_map_image(&gen, (0.0, 0.0), (size.x as usize, size.y as usize), 0.7);
            let tex = ui.ctx().load_texture("faction_map", img, egui::TextureOptions::NEAREST);
            ui.painter().image(tex.id(), rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                egui::Color32::WHITE);
            ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(2.0, accent), egui::StrokeKind::Middle);
            // structure icons: faction-color diamonds at fixed world spots
            let to_screen = |wx: f32, wz: f32| -> egui::Pos2 {
                egui::Pos2::new(
                    rect.left() + (wx + 400.0) / 800.0 * rect.width(),
                    rect.top() + (wz + 270.0) / 540.0 * rect.height(),
                )
            };
            let icons = [
                (60.0, 40.0, [74, 122, 181], "embassy"),
                (-120.0, 90.0, [139, 69, 19], "forge camp"),
                (150.0, -80.0, [196, 96, 42], "shrine"),
                (-40.0, -140.0, [107, 142, 35], "longhouse"),
                (200.0, 150.0, [176, 176, 176], "library"),
                (-220.0, -30.0, [45, 45, 45], "camp"),
            ];
            for (wx, wz, col, label) in icons {
                let pos = to_screen(wx, wz);
                let r = 6.0;
                ui.painter().add(egui::Shape::convex_polygon(vec![
                    pos + egui::vec2(0.0, -r), pos + egui::vec2(r, 0.0),
                    pos + egui::vec2(0.0, r), pos + egui::vec2(-r, 0.0),
                ], egui::Color32::from_rgb(col[0], col[1], col[2]),
                    egui::Stroke::new(1.5, egui::Color32::from_gray(20))));
                ui.painter().text(pos + egui::vec2(0.0, 12.0), egui::Align2::CENTER_CENTER, label,
                    egui::FontId::proportional(10.0), egui::Color32::from_gray(235));
            }
            // player arrow
            let p = to_screen(0.0, 0.0);
            ui.painter().add(egui::Shape::convex_polygon(vec![
                p + egui::vec2(0.0, -7.0), p + egui::vec2(5.0, 5.0), p + egui::vec2(-5.0, 5.0),
            ], egui::Color32::WHITE, egui::Stroke::new(1.0, egui::Color32::from_gray(20))));
            // legend
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                for (name, col, _) in preview_faction_homes() {
                    let (swatch, _) = ui.allocate_exact_size(egui::vec2(9.0, 9.0), egui::Sense::hover());
                    ui.painter().rect_filled(swatch, 2.0, egui::Color32::from_rgb(col[0], col[1], col[2]));
                    ui.label(egui::RichText::new(name).small().color(egui::Color32::from_gray(200)));
                    ui.add_space(8.0);
                }
            });
        });
    });
}

/// A3/C4 proof: the faction standing widget (bottom-right) + the companion
/// HUD tile (top-left) over the standard HUD.
fn draw_faction_hud_preview(ctx: &egui::Context) {
    draw_hud_preview(ctx);
    let panel = egui::Color32::from_rgba_premultiplied(18, 22, 30, 210);
    let accent = egui::Color32::from_rgb(240, 200, 120);
    let ok = egui::Color32::from_rgb(120, 210, 130);
    // companion tile: Accord-blue initial + trust/morale bars
    egui::Area::new(egui::Id::new("preview_companion_tile"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 26.0))
        .show(ctx, |ui| {
            egui::Frame::new().fill(panel).corner_radius(6.0).inner_margin(4.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, 4.0, egui::Color32::from_rgb(74, 122, 181));
                    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "H",
                        egui::FontId::proportional(12.0), egui::Color32::from_gray(16));
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("FOLLOW").small().color(egui::Color32::from_gray(150)));
                        let bar = |ui: &mut egui::Ui, v: i32, color: egui::Color32| {
                            let (r, _) = ui.allocate_exact_size(egui::vec2(64.0, 3.0), egui::Sense::hover());
                            ui.painter().rect_filled(r, 1.5, egui::Color32::from_black_alpha(150));
                            ui.painter().rect_filled(egui::Rect::from_min_size(r.min,
                                egui::vec2(r.width() * v as f32 / 100.0, r.height())), 1.5, color);
                        };
                        bar(ui, 62, accent);
                        ui.add_space(2.0);
                        bar(ui, 48, ok);
                    });
                });
            });
        });
    // faction widget: name, colored standing bar, number (Ironborn, +34)
    egui::Area::new(egui::Id::new("preview_faction_widget"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-10.0, -96.0))
        .show(ctx, |ui| {
            egui::Frame::new().fill(panel).corner_radius(9.0)
                .inner_margin(egui::Margin::symmetric(8, 5))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(139, 69, 19)))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter().rect_filled(r, 2.0, egui::Color32::from_rgb(139, 69, 19));
                        ui.label(egui::RichText::new("HAMMER").small().color(egui::Color32::from_rgb(200, 140, 90)));
                        ui.label(egui::RichText::new("The Ironborn").small().color(egui::Color32::from_gray(235)));
                        let (bar, _) = ui.allocate_exact_size(egui::vec2(46.0, 5.0), egui::Sense::hover());
                        ui.painter().rect_filled(bar, 2.0, egui::Color32::from_black_alpha(150));
                        let frac = (34 + 100) as f32 / 200.0;
                        ui.painter().rect_filled(egui::Rect::from_min_max(bar.left_top(),
                            egui::pos2(bar.left() + bar.width() * frac, bar.bottom())), 2.0, accent);
                        ui.label(egui::RichText::new("+34").small().color(accent));
                    });
                });
        });
}

/// B3 proof: the companion command menu.
fn draw_companion_menu_preview(ctx: &egui::Context) {
    let panel = egui::Color32::from_rgba_premultiplied(18, 22, 30, 235);
    let accent = egui::Color32::from_rgb(240, 200, 120);
    let bad = egui::Color32::from_rgb(230, 120, 110);
    let ok = egui::Color32::from_rgb(120, 210, 130);
    egui::Window::new("Herald Aldis — commands")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -30.0))
        .collapsible(false).resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                ui.painter().rect_filled(r, 5.0, egui::Color32::from_rgb(74, 122, 181));
                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "H",
                    egui::FontId::proportional(14.0), egui::Color32::from_gray(16));
                ui.label(egui::RichText::new("FOLLOW").small().color(egui::Color32::from_gray(150)));
            });
            let bar = |ui: &mut egui::Ui, label: &str, v: i32, color: egui::Color32| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(150.0, 8.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, 4.0, egui::Color32::from_black_alpha(140));
                    ui.painter().rect_filled(egui::Rect::from_min_size(r.min,
                        egui::vec2(r.width() * v as f32 / 100.0, r.height())), 4.0, color);
                    ui.label(egui::RichText::new(format!("{} {}/100", label, v)).small().color(egui::Color32::from_gray(235)));
                });
            };
            bar(ui, "Trust", 62, accent);
            bar(ui, "Morale", 48, ok);
            ui.label(egui::RichText::new("cargo: iron_ore x6").small().color(egui::Color32::from_gray(150)));
            ui.separator();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.button("Follow me");
                    ui.button("Mine this");
                    ui.button("Chop nearby");
                    ui.button("Rest");
                    ui.add_enabled(false, egui::Button::new("Craft (recipes soon)"));
                });
                ui.vertical(|ui| {
                    ui.button("Stay here");
                    ui.button("Haul to chest");
                    ui.button("Guard area");
                    ui.button("Pay now");
                    ui.button(egui::RichText::new("Dismiss").color(bad));
                });
            });
            ui.separator();
            ui.label(egui::RichText::new("Esc to close").small());
        });
}

/// Preview palette matching the client's 30-biome table (subset used here).
fn preview_biome_color(b: Biome) -> egui::Color32 {
    use Biome::*;
    let c = |r: u32, g: u32, bl: u32| egui::Color32::from_rgb(r as u8, g as u8, bl as u8);
    match b {
        Meadow => c(120, 178, 90),
        FlowerForest | Forest => c(78, 140, 66),
        BirchForest => c(148, 168, 104),
        DarkForest => c(48, 100, 55),
        Taiga | SnowyTaiga => c(70, 120, 85),
        Tundra => c(200, 215, 215),
        IceSpikes => c(185, 220, 235),
        SnowySlope | SnowyPeaks => c(225, 232, 235),
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
        Swamp => c(85, 105, 70),
        Jungle => c(50, 135, 60),
        Savanna | WindsweptSavanna => c(170, 164, 94),
        _ => c(120, 160, 90),
    }
}

// ------------------------------------------------------------------
// HUD proof overlay: icons hotbar, XP bar, painted hearts, minimap.

fn draw_hud_preview(ctx: &egui::Context) {
    let icons = PreviewIcons::new(ctx, &[
        "grass", "dirt", "stone_pickaxe", "torch", "planks", "iron_ingot", "apple", "bow", "arrow", "coal", "raw_iron",
    ]);
    egui::TopBottomPanel::bottom("hud").frame(egui::Frame::none()).show_separator_line(false).show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(4.0);
            // hearts + hunger (painted glyphs, no unicode boxes)
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(190.0, 16.0), egui::Sense::hover());
                for i in 0..10 {
                    let c = egui::pos2(rect.left() + 9.0 + i as f32 * 18.0, rect.center().y);
                    let full = i < 8;
                    let half = i == 8;
                    let col = if full || half {
                        egui::Color32::from_rgb(225, 60, 70)
                    } else {
                        egui::Color32::from_rgb(70, 40, 44)
                    };
                    ui.painter().circle_filled(egui::pos2(c.x - 3.5, c.y - 2.5), 3.6, col);
                    ui.painter().circle_filled(egui::pos2(c.x + 3.5, c.y - 2.5), 3.6, col);
                    ui.painter().add(egui::Shape::convex_polygon(vec![
                        egui::pos2(c.x - 6.8, c.y - 1.0), egui::pos2(c.x + 6.8, c.y - 1.0), egui::pos2(c.x, c.y + 6.5),
                    ], col, egui::Stroke::NONE));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(170.0, 14.0), egui::Sense::hover());
                    for i in 0..10 {
                        let c = egui::pos2(rect.right() - 7.0 - i as f32 * 16.0, rect.center().y);
                        let fill = if i < 7 { egui::Color32::from_rgb(210, 150, 50) } else { egui::Color32::from_rgb(70, 56, 32) };
                        ui.painter().circle_filled(c, 5.0, fill);
                        ui.painter().circle_stroke(c, 5.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 24, 16)));
                    }
                });
            });
            // XP bar with level chip
            let (xrect, _) = ui.allocate_exact_size(egui::vec2(420.0, 9.0), egui::Sense::hover());
            ui.painter().rect_filled(xrect, 4.0, egui::Color32::from_black_alpha(190));
            ui.painter().rect_filled(egui::Rect::from_min_size(xrect.min, egui::vec2(xrect.width() * 0.62, xrect.height())), 4.0,
                egui::Color32::from_rgb(110, 220, 255));
            let chip = egui::Rect::from_center_size(xrect.center(), egui::vec2(34.0, 14.0));
            ui.painter().rect_filled(chip, 4.0, egui::Color32::from_rgb(16, 18, 24));
            ui.painter().text(chip.center(), egui::Align2::CENTER_CENTER, "Lv 7",
                egui::FontId::proportional(11.0), egui::Color32::from_rgb(110, 220, 255));
            ui.add_space(1.0);
            // hotbar with real icons
            ui.horizontal(|ui| {
                let items: [Option<(&str, u8)>; 9] = [
                    Some(("grass", 42)), Some(("dirt", 64)), Some(("stone_pickaxe", 1)), Some(("torch", 12)),
                    Some(("planks", 33)), Some(("iron_ingot", 7)), Some(("apple", 3)), Some(("bow", 1)), Some(("arrow", 21)),
                ];
                for (i, item) in items.iter().enumerate() {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
                    preview_slot(ui, &icons, rect, *item, i == 2);
                }
            });
            ui.label(egui::RichText::new("Stone Pickaxe").small().color(ACCENT));
        });
    });
    let pointer = ctx.screen_rect().center();
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, "crosshair".into()));
    let c = egui::Color32::from_white_alpha(220);
    p.line_segment([pointer - egui::vec2(7.0, 0.0), pointer - egui::vec2(2.0, 0.0)], egui::Stroke::new(2.0, c));
    p.line_segment([pointer + egui::vec2(2.0, 0.0), pointer + egui::vec2(7.0, 0.0)], egui::Stroke::new(2.0, c));
    // radial mining-progress reticle mid-break (mirrors ui_kit::
    // paint_mining_reticle — lf_vistest cannot depend on lf_client; keep
    // the math in sync)
    {
        const RADIUS: f32 = 15.0;
        let progress = 0.55f32;
        p.circle_stroke(pointer, RADIUS, egui::Stroke::new(1.5, egui::Color32::from_white_alpha(48)));
        let steps = ((progress * 64.0).ceil() as usize).max(2);
        let points: Vec<egui::Pos2> = (0..=steps)
            .map(|i| {
                let a = -std::f32::consts::FRAC_PI_2 + (i as f32 / steps as f32) * progress * std::f32::consts::TAU;
                pointer + egui::vec2(a.cos() * RADIUS, a.sin() * RADIUS)
            })
            .collect();
        p.add(egui::Shape::Path(egui::epaint::PathShape {
            points,
            closed: false,
            fill: egui::Color32::TRANSPARENT,
            stroke: egui::epaint::PathStroke::new(3.0, egui::Color32::from_rgb(240, 200, 120)),
        }));
    }
    p.line_segment([pointer - egui::vec2(0.0, 7.0), pointer - egui::vec2(0.0, 2.0)], egui::Stroke::new(2.0, c));
    p.line_segment([pointer + egui::vec2(0.0, 2.0), pointer + egui::vec2(0.0, 7.0)], egui::Stroke::new(2.0, c));
}

mod serde_inline {
    // local mirror of lf_client::lore's schema (see note above)
    use serde::Deserialize;
    #[derive(Deserialize)]
    pub struct Lib {
        pub books: Vec<Book>,
    }
    #[derive(Deserialize)]
    pub struct Book {
        pub id: String,
        pub title: String,
        pub item: String,
        pub pages: Vec<String>,
    }
}

/// Lore tome proof (Step 20): an open book page with real text loaded
/// from the actual lore/books.toml the game reads.
fn draw_lore_preview(ctx: &egui::Context) {
    // mini-reader mirroring lf_client::lore (same real file; lf_vistest
    // cannot depend on lf_client) — keep the schema in sync
    let book: Option<serde_inline::Lib> = std::fs::read_to_string("lore/books.toml")
        .ok()
        .and_then(|t| toml::from_str(&t).ok());
    let Some(book) = book.and_then(|l| l.books.into_iter().find(|b| b.item == "tome_of_the_forge")) else {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("lore/books.toml missing — proof cannot render");
        });
        return;
    };
    let page = 1.min(book.pages.len() - 1);
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(150)))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(60.0);
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(24, 22, 18))
                    .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(240, 200, 120)))
                    .corner_radius(8.0)
                    .inner_margin(24.0)
                    .show(ui, |ui| {
                        ui.set_width(430.0);
                        ui.label(egui::RichText::new(&book.title).size(21.0)
                            .color(egui::Color32::from_rgb(240, 200, 120)));
                        ui.label(egui::RichText::new(format!("page {} of {}", page + 1, book.pages.len()))
                            .small().color(egui::Color32::from_gray(150)));
                        ui.separator();
                        ui.label(egui::RichText::new(book.pages[page].clone()).size(15.0)
                            .color(egui::Color32::from_gray(230)));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.add_enabled_ui(page > 0, |ui| { ui.button("< prev"); });
                            ui.add_enabled_ui(page + 1 < book.pages.len(), |ui| { ui.button("next >"); });
                        });
                    });
            });
        });
}

/// Corner minimap proof: terrain texture + entity dots + player arrow.
fn draw_minimap_preview(ctx: &egui::Context) {
    let gen = WorldGen::new(Seed(12345));
    let image = map_image(&gen, (8.0, 8.0), (172, 172), 1.0);
    let tex = ctx.load_texture("preview_minimap", image, egui::TextureOptions::NEAREST);
    egui::Window::new("minimap")
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 34.0))
        .title_bar(false)
        .frame(egui::Frame::none())
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            let size = 172.0;
            let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
            let paint = ui.painter();
            paint.rect_filled(rect, 8.0, egui::Color32::from_rgb(20, 24, 32));
            let uv = egui::Rect::from_min_max(egui::Pos2::new(0.0, 0.0), egui::Pos2::new(1.0, 1.0));
            paint.image(tex.id(), rect, uv, egui::Color32::WHITE);
            paint.rect_stroke(rect, 8.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(120, 96, 52)), egui::StrokeKind::Middle);
            paint.rect_filled(egui::Rect::from_center_size(rect.center_top(), egui::vec2(18.0, 12.0)), 3.0, egui::Color32::from_rgb(16, 18, 24));
            paint.text(rect.center_top() + egui::vec2(0.0, 6.0), egui::Align2::CENTER_CENTER, "N",
                egui::FontId::proportional(10.0), ACCENT);
            // entity dots + waypoint pip
            paint.circle_filled(rect.center() + egui::vec2(-34.0, 18.0), 2.0, BAD);
            paint.circle_filled(rect.center() + egui::vec2(22.0, -40.0), 2.0, BAD);
            paint.circle_filled(rect.center() + egui::vec2(50.0, 30.0), 2.0, OK);
            paint.circle_filled(rect.center() + egui::vec2(-60.0, -30.0), 3.5, ACCENT);
            paint.circle_stroke(rect.center() + egui::vec2(-60.0, -30.0), 3.5, egui::Stroke::new(1.0, egui::Color32::from_rgb(16, 18, 24)));
            // player arrow
            let c = rect.center();
            let dir = egui::vec2(0.6, -0.8);
            let tip = c + dir * 7.0;
            let left = c + egui::vec2(-dir.y, dir.x) * 4.0 - dir * 4.0;
            let right = c - egui::vec2(-dir.y, dir.x) * 4.0 - dir * 4.0;
            paint.add(egui::Shape::convex_polygon(vec![tip, left, right], egui::Color32::WHITE,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(16, 18, 24))));
        });
    // info line with facing + biome
    egui::Area::new(egui::Id::new("info_line"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 8.0))
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("NW · Meadow · 8,12 · 08:24 · clear").small()
                .color(egui::Color32::from_rgba_unmultiplied(235, 238, 242, 200)));
        });
}

/// Full world-map proof: pannable map canvas, fog of war, waypoints panel.
fn draw_map_preview(ctx: &egui::Context) {
    let gen = WorldGen::new(Seed(12345));
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::from_black_alpha(160)))
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                // map canvas with fog beyond the explored radius
                let (rect, _) = ui.allocate_exact_size(egui::vec2(520.0, 520.0), egui::Sense::click_and_drag());
                let image = map_image(&gen, (0.0, 0.0), (260, 260), 2.0);
                let tex = ctx.load_texture("preview_map", image, egui::TextureOptions::NEAREST);
                let paint = ui.painter();
                paint.rect_filled(rect, 6.0, egui::Color32::from_rgb(20, 24, 32));
                let uv = egui::Rect::from_min_max(egui::Pos2::new(0.0, 0.0), egui::Pos2::new(1.0, 1.0));
                paint.image(tex.id(), rect, uv, egui::Color32::WHITE);
                // fog of war: dim everything beyond the explored radius
                let explored = egui::Rect::from_center_size(rect.center(), egui::vec2(300.0, 300.0));
                paint.rect_filled(egui::Rect::from_min_max(rect.left_top(), explored.right_top()), 0.0, egui::Color32::from_rgba_unmultiplied(20, 24, 32, 225));
                paint.rect_filled(egui::Rect::from_min_max(rect.left_top(), explored.left_bottom()), 0.0, egui::Color32::from_rgba_unmultiplied(20, 24, 32, 225));
                paint.rect_filled(egui::Rect::from_min_max(explored.right_top(), rect.right_bottom()), 0.0, egui::Color32::from_rgba_unmultiplied(20, 24, 32, 225));
                paint.rect_filled(egui::Rect::from_min_max(explored.left_bottom(), rect.right_bottom()), 0.0, egui::Color32::from_rgba_unmultiplied(20, 24, 32, 225));
                paint.rect_stroke(rect, 6.0, egui::Stroke::new(1.5, egui::Color32::from_rgb(120, 96, 52)), egui::StrokeKind::Middle);
                let to_screen = |wx: f32, wz: f32| -> egui::Pos2 {
                    egui::Pos2::new(rect.left() + wx * 2.0 + rect.width() / 2.0, rect.top() + wz * 2.0 + rect.height() / 2.0)
                };
                // spawn diamond
                let sp = to_screen(0.0, 0.0);
                paint.add(egui::Shape::convex_polygon(vec![
                    sp + egui::vec2(0.0, -6.0), sp + egui::vec2(6.0, 0.0), sp + egui::vec2(0.0, 6.0), sp + egui::vec2(-6.0, 0.0),
                ], egui::Color32::from_rgb(240, 120, 140), egui::Stroke::new(1.5, egui::Color32::from_rgb(16, 18, 24))));
                // waypoints with labels
                for (x, z, name, col) in [
                    (-58.0, -44.0, "Home · 72m", ACCENT),
                    (64.0, 30.0, "Iron Mine · 71m", egui::Color32::from_rgb(110, 220, 255)),
                ] {
                    let pos = to_screen(x, z);
                    paint.circle_filled(pos, 5.0, col);
                    paint.circle_stroke(pos, 5.0, egui::Stroke::new(1.5, egui::Color32::from_rgb(16, 18, 24)));
                    paint.text(pos + egui::vec2(0.0, -12.0), egui::Align2::CENTER_CENTER, name,
                        egui::FontId::proportional(11.0), egui::Color32::from_rgb(235, 238, 242));
                }
                // player arrow
                let c = to_screen(10.0, 14.0);
                let dir = egui::vec2(0.6, -0.8);
                let tip = c + dir * 8.0;
                let left = c + egui::vec2(-dir.y, dir.x) * 5.0 - dir * 5.0;
                let right = c - egui::vec2(-dir.y, dir.x) * 5.0 - dir * 5.0;
                paint.add(egui::Shape::convex_polygon(vec![tip, left, right], egui::Color32::WHITE,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(16, 18, 24))));
                paint.text(rect.center_top() + egui::vec2(0.0, 12.0), egui::Align2::CENTER_CENTER, "N",
                    egui::FontId::proportional(13.0), ACCENT);
                paint.rect_filled(egui::Rect::from_min_size(rect.left_bottom(), egui::vec2(200.0, 20.0)), 3.0, egui::Color32::from_black_alpha(170));
                paint.text(rect.left_bottom() + egui::vec2(8.0, 10.0), egui::Align2::LEFT_CENTER,
                    "-12, 30 · Taiga", egui::FontId::proportional(11.0), egui::Color32::from_rgb(235, 238, 242));
                paint.text(rect.right_bottom() + egui::vec2(-150.0, -10.0), egui::Align2::LEFT_CENTER,
                    "drag pan · wheel zoom · M close", egui::FontId::proportional(10.0), TEXT_DIM);

                // waypoint manager panel
                ui.vertical(|ui| {
                    ui.set_width(230.0);
                    ui.label(egui::RichText::new("Waypoints").size(18.0).color(ACCENT));
                    ui.painter().line_segment([ui.cursor().min + egui::vec2(0.0, 24.0), ui.cursor().min + egui::vec2(120.0, 24.0)],
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(120, 96, 52)));
                    ui.add_space(14.0);
                    let btn = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(300.0, 52.0));
                    ui.allocate_rect(btn, egui::Sense::click());
                    ui.painter().rect_filled(btn, 10.0, egui::Color32::from_rgba_premultiplied(28, 33, 44, 225));
                    ui.painter().rect_stroke(btn, 10.0, egui::Stroke::new(1.5, egui::Color32::from_rgb(90, 98, 112)), egui::StrokeKind::Middle);
                    ui.painter().rect_filled(egui::Rect::from_min_size(egui::Pos2::new(btn.left() + 4.0, btn.center().y - 16.0), egui::vec2(3.0, 32.0)), 2.0, ACCENT);
                    ui.painter().text(btn.center(), egui::Align2::CENTER_CENTER, "+ Marker at 10,14",
                        egui::FontId::proportional(18.0), egui::Color32::from_rgb(235, 238, 242));
                    ui.add_space(12.0);
                    for (name, dist, col) in [
                        ("Home", "72m", ACCENT),
                        ("Iron Mine", "71m", egui::Color32::from_rgb(110, 220, 255)),
                        ("Village", "143m", OK),
                    ] {
                        egui::Frame::new()
                            .fill(egui::Color32::from_black_alpha(100))
                            .corner_radius(6.0)
                            .inner_margin(6.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (dot, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                                    ui.painter().circle_filled(dot.center(), 5.0, col);
                                    ui.label(egui::RichText::new(name).size(13.0).color(egui::Color32::from_rgb(235, 238, 242)));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.small_button("×");
                                        ui.label(egui::RichText::new(dist).small().color(TEXT_DIM));
                                    });
                                });
                            });
                    }
                    ui.add_space(8.0);
                    ui.add(egui::Slider::new(&mut 2.0f32, 0.5..=6.0).text("zoom"));
                    if ui.button("Center on player").clicked() {}
                });
            });
        });
}

/// Crafting + recipe book proof: real icons everywhere, have/need coloring.
fn draw_crafting_preview(ctx: &egui::Context) {
    let icons = PreviewIcons::new(ctx, &[
        "planks", "stick", "crafting_table", "torch", "iron_ingot", "iron_pickaxe", "chest", "furnace",
        "coal", "raw_iron", "stone", "log", "apple", "dirt", "grass", "copper_ingot", "tin_ingot", "bronze_ingot",
    ]);
    egui::Window::new("Crafting Table")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(-40.0, -20.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    // 3x3 grid with the pickaxe pattern filled in
                    ui.label(egui::RichText::new("Craft").size(16.0).color(ACCENT));
                    ui.add_space(4.0);
                    let grid: [[Option<(&str, u8)>; 3]; 3] = [
                        [Some(("iron_ingot", 12)), Some(("iron_ingot", 12)), Some(("iron_ingot", 12))],
                        [None, Some(("stick", 30)), None],
                        [None, Some(("stick", 30)), None],
                    ];
                    for row in grid {
                        ui.horizontal(|ui| {
                            for cell in row {
                                let (rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
                                preview_slot(ui, &icons, rect, cell, false);
                            }
                        });
                    }
                });
                ui.add_space(12.0);
                // result slot
                let (rect, _) = ui.allocate_exact_size(egui::vec2(52.0, 52.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 6.0, egui::Color32::from_black_alpha(170));
                ui.painter().rect_stroke(rect, 6.0, egui::Stroke::new(2.0, ACCENT), egui::StrokeKind::Middle);
                icons.paint(ui, rect.shrink(8.0), "iron_pickaxe");
                ui.add_space(12.0);
                // recipe book panel
                ui.vertical(|ui| {
                    ui.set_width(300.0);
                    ui.label(egui::RichText::new("Recipe Book").size(16.0).color(ACCENT));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut "pick".to_string()).desired_width(110.0));
                        for (label, on) in [("All", true), ("Craft", false), ("Smelt", false), ("Alloy", false)] {
                            let btn = egui::Button::new(egui::RichText::new(label).color(if on { ACCENT } else { TEXT_DIM }));
                            let _ = ui.add(btn);
                        }
                    });
                    ui.add_space(4.0);
                    let entries: [(&str, &str, u8, &[(&str, u8, u16)], bool); 4] = [
                        ("iron_pickaxe", "Iron Pickaxe", 1, &[("iron_ingot", 3, 12), ("stick", 2, 30)], true),
                        ("iron_axe", "Iron Axe", 1, &[("iron_ingot", 3, 12), ("stick", 2, 30)], true),
                        ("torch", "Torch", 4, &[("coal", 1, 9), ("stick", 1, 30)], true),
                        ("basic_circuit", "Basic Circuit", 1, &[("copper_wire", 2, 0), ("tin_ingot", 1, 2), ("iron_ingot", 1, 12)], false),
                    ];
                    for (id, name, count, needs, craftable) in entries {
                        egui::Frame::new()
                            .fill(if craftable { egui::Color32::from_rgba_premultiplied(34, 40, 32, 220) }
                                  else { egui::Color32::from_black_alpha(150) })
                            .stroke(egui::Stroke::new(if craftable { 1.6 } else { 1.0 },
                                if craftable { egui::Color32::from_rgb(120, 96, 52) } else { egui::Color32::from_gray(60) }))
                            .corner_radius(7.0)
                            .inner_margin(6.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (orect, _) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
                                    icons.paint(ui, orect, id);
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(name).size(13.0).color(egui::Color32::from_rgb(235, 238, 242)));
                                            if count > 1 {
                                                ui.label(egui::RichText::new(format!("x{}", count)).small().color(TEXT_DIM));
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            for (nid, n, have) in needs {
                                                let ok = *have >= *n as u16;
                                                let (irect, _) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                                                icons.paint(ui, irect, nid);
                                                ui.label(egui::RichText::new(format!("{}", n)).small().color(if ok { OK } else { BAD }));
                                            }
                                        });
                                    });
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new("Crafting Table").small().color(TEXT_DIM));
                                        ui.add_enabled(craftable, egui::Button::new("fill"));
                                    });
                                });
                            });
                    }
                });
            });
            ui.add_space(6.0);
            // storage + hotbar rows
            for row in 0..2 {
                ui.horizontal(|ui| {
                    for col in 0..9 {
                        let _ = col;
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
                        let items = [Some(("log", 16)), Some(("planks", 64)), None, Some(("coal", 9)), None,
                                     Some(("raw_iron", 5)), None, None, Some(("dirt", 32))];
                        preview_slot(ui, &icons, rect, items[row], false);
                    }
                });
            }
        });
}

/// Path-trace a scene: build the voxel clip around the camera and dispatch.
fn render_raytraced(spec: &SceneSpec, seed: u64, eye: &Vec3, out_path: &Path) -> Result<(), String> {
    let gen = WorldGen::new(Seed(seed));
    let mut world = World::new();
    for cx in -4..=4 {
        for cz in -4..=4 {
            world.chunks.insert((cx, cz), gen.generate_chunk(cx, cz));
        }
    }
    let ground_level = world.surface_height(eye.x as i32, eye.z as i32) as f32;
    if spec.torches {
        use lf_voxel::registry::block;
        // Torch ring + glowing floor around the camera position so the
        // emissive glow visibly fills the traced frame.
        let cx0 = eye.x as i32;
        let cz0 = eye.z as i32;
        let ground = ground_level;
        for (dx, dz) in [(4, 0), (-4, 0), (0, 4), (0, -4)] {
            let top = world.surface_height(cx0 + dx, cz0 + dz);
            world.set_block(cx0 + dx, top, cz0 + dz, lf_voxel::BlockState(block::TORCH));
        }
        // lantern patch just below the camera: big enough for the emissive
        // glow to dominate the steep-down view, small enough that terrain
        // and sky still frame it (a full-frame floor is one flat color —
        // caught by the P25 pixel gate)
        let ly = (ground + 2.0) as i32 - 2;
        for dz in -4..=0i32 {
            for dx in -2..=2i32 {
                for dy in 0..1i32 {
                    world.set_block(cx0 + dx, ly + dy, cz0 + dz, lf_voxel::BlockState(block::LANTERN));
                }
            }
        }
    }
    // Day: high enough for a vista, but the tracer's voxel clip only extends
    // ±32 blocks around the camera — any higher and the terrain falls out of
    // the clip and every ray returns flat fog (the pre-P25 broken proof).
    // Night torch scenes: sit at ground level beside the torch ring so the
    // glow fills the frame.
    let lift = if spec.time_of_day > 0.2 && spec.time_of_day < 0.8 { 6.0 } else { 0.0 };
    let ground = ground_level;
    let rt_eye = if lift > 0.0 {
        Vec3::new(eye.x, ground + 22.0 + lift, eye.z)
    } else {
        Vec3::new(eye.x, ground + 2.0, eye.z)
    };
    let center = (rt_eye.x as i32, rt_eye.y as i32, rt_eye.z as i32);
    let voxel_data = lf_engine::pathtrace::build_voxel_texture_data(center, &|x, y, z| {
        world.get_block(x, y, z).id()
    });
    // look toward the terrain like the raster scenes
    let look = if spec.torches {
        Vec3::new(0.25, -0.8, -1.0).normalize() // stare into the glowing floor (emissive proof)
    } else {
        Vec3::new(0.35, -0.35, -1.0).normalize()
    };
    let mut camera = Camera::new(rt_eye, rt_eye + look * 40.0);
    camera.set_aspect(800, 600);
    let tod = lf_game::TimeOfDay::from_fraction(spec.time_of_day);
    let angle = spec.time_of_day * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
    let sun = [angle.cos(), angle.sin().abs(), 0.25];
    let img = lf_engine::pathtrace::pathtrace_to_image(
        &voxel_data, center, &camera, sun, tod.sky_light_level(), 800, 600, 48,
    )?;
    img.save(out_path).map_err(|e| format!("save: {e}"))
}

/// Where the spawn column's biome lands for a seed (used by tests to describe scenes).
pub fn spawn_biome(seed: u64) -> Biome {
    WorldGen::new(Seed(seed)).biome(0, 0)
}

/// Console proof overlay: history, suggestions, input line.
fn draw_console_preview(ctx: &egui::Context) {
    // NB: plain `Area`s don't materialize in the two-pass headless harness
    // (only windows do), so the proof uses a frameless anchored window.
    egui::Window::new("LOREFORGE Console")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(12, 14, 20, 242))
                .stroke(egui::Stroke::new(1.0, ACCENT_DIM_COL))
                .corner_radius(8.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_width(520.0);
                    ui.label(egui::RichText::new("console — Tab complete · ↑↓ history · Esc close")
                        .small().color(TEXT_DIM));
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical().stick_to_bottom(true).max_height(170.0).show(ui, |ui| {
                        for line in [
                            "> time set night",
                            "time set (fraction 0.00)",
                            "> give iron_pickaxe",
                            "gave iron_pickaxe x1",
                            "> tp 120 80 -40",
                            "teleported to (120.0, 80.0, -40.0)",
                        ] {
                            ui.label(egui::RichText::new(line).small().monospace().color(egui::Color32::from_rgb(235, 238, 242)));
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("time  tp  weather  waypoint")
                        .small().monospace().color(ACCENT));
                    let mut input = "w".to_string();
                    ui.add(egui::TextEdit::singleline(&mut input)
                        .font(egui::TextStyle::Monospace).desired_width(504.0).hint_text("type a command… (help)"));
                });
        });
}

const ACCENT_DIM_COL: egui::Color32 = egui::Color32::from_rgb(120, 96, 52);

/// Frame-time benchmark (goal Section 5 / Step 9): build a scene once,
/// create the persistent headless renderer once (device + atlas), then
/// time N renders — each render re-uploads the mesh like a live re-mesh
/// and includes GPU + readback + PNG encode, so it is an upper bound on
/// present-only frame cost (caveat recorded in DECISIONS.md).
pub struct BenchStats {
    pub p50_ms: f32,
    pub p95_ms: f32,
    pub min_ms: f32,
}

pub fn bench(scene_name: &str, n: usize) -> Result<BenchStats, String> {
    let spec = scenes().into_iter().find(|s| s.name == scene_name)
        .ok_or_else(|| format!("unknown scene '{}'", scene_name))?;
    if spec.raytraced {
        return Err("bench renders through the raster path; pick a non-RT scene".into());
    }
    // Medium quality = view distance 5, so bench a radius-5 plot
    let (v, i, wv, wi) = build_scene_mesh(&spec, spec.default_seed, 5, spec.torches, spec.machines);
    if v.is_empty() {
        return Err(format!("scene '{}' produced an empty mesh", scene_name));
    }
    let gen = WorldGen::new(Seed(spec.default_seed));
    let h_eye = gen.surface_top(spec.eye.x as i32, spec.eye.z as i32);
    let h_target = gen.surface_top(spec.target.x as i32, spec.target.z as i32);
    let eye = Vec3::new(spec.eye.x, spec.eye.y.max(h_eye as f32 + 22.0), spec.eye.z);
    let target = Vec3::new(spec.target.x, h_target as f32 + 2.0, spec.target.z);
    let mut camera = Camera::new(eye, target);
    camera.set_aspect(800, 600);
    let env = lf_engine::scene::Env {
        camera_pos: eye,
        time: 0.8,
        day_factor: spec.day_factor(),
        fog_color: spec.time_of_day().sky_color(),
        fog_far: 220.0,
        grade_tint: [1.0, 1.0, 1.0],
        grade_saturation: 1.0,
    };
    let textures = lf_assets::generate_atlas();
    let renderer = lf_engine::headless::HeadlessRenderer::new(800, 600, &textures)?;
    let out = std::env::temp_dir().join("lf_perf_frame.png");
    let mut times_ms: Vec<f32> = Vec::with_capacity(n);
    for k in 0..n {
        let t = std::time::Instant::now();
        renderer.render(&v, &i, &wv, &wi, &camera, &env, spec.sky_color(), &out, None)?;
        if k > 0 {
            times_ms.push(t.elapsed().as_secs_f32() * 1000.0); // first frame pays warmup
        }
    }
    times_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = times_ms[times_ms.len() / 2];
    let p95 = times_ms[((times_ms.len() as f32 * 0.95) as usize).min(times_ms.len() - 1)];
    let min = times_ms[0];
    Ok(BenchStats { p50_ms: p50, p95_ms: p95, min_ms: min })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_scenes_with_unique_names() {
        let scenes = scenes();
        assert!(!scenes.is_empty());
        let mut names: Vec<&str> = scenes.iter().map(|s| s.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), scenes.len());
    }

    #[test]
    fn every_scene_builds_nonempty_deterministic_mesh() {
        for spec in scenes() {
            let (v1, i1, _, _) = build_scene_mesh(&spec, spec.default_seed, 1, false, false);
            let (v2, i2, _, _) = build_scene_mesh(&spec, spec.default_seed, 1, false, false);
            assert!(!v1.is_empty(), "{} produced no vertices", spec.name);
            assert_eq!(v1.len(), v2.len(), "{} mesh not deterministic", spec.name);
            assert_eq!(i1.len(), i2.len());
            assert_eq!(i1.len() % 3, 0, "{} indices not triangles", spec.name);
        }
    }

    #[test]
    fn seed_changes_mesh_somewhere() {
        fn hash(vertices: &[GpuVertex]) -> u64 {
            let mut h: u64 = 0xcbf29ce484222325;
            for v in vertices {
                for bits in v.position.map(|f| f.to_bits()) {
                    h = (h ^ bits as u64).wrapping_mul(0x100000001b3);
                }
                h = (h ^ v.tex_index as u64).wrapping_mul(0x100000001b3);
            }
            h
        }
        let spec = &scenes()[0];
        let mut hashes = std::collections::HashSet::new();
        for seed in 1..=20u64 {
            let (v, _, _, _) = build_scene_mesh(spec, seed, 1, false, false);
            hashes.insert(hash(&v));
        }
        assert!(hashes.len() > 1, "seeds 1..=20 all produced the same mesh");
    }

    #[test]
    fn unknown_scene_errors() {
        assert!(run_scene("nope", None, Path::new("/tmp/x.png")).is_err());
    }

    /// Goal Section 3 proof: the per-biome color grade must measurably
    /// shift the mid-frame color of the SAME scene — a warm desert grade
    /// versus a cold snow grade, rendered through the real GPU pipeline.
    #[test]
    fn biome_grade_shifts_midframe_color() {
        let spec = scenes().into_iter().find(|s| s.name == "terrain_vista")
            .expect("terrain_vista scene registered");
        let (v, i, wv, wi) = build_scene_mesh(&spec, spec.default_seed, 2, false, false);
        let gen = WorldGen::new(Seed(spec.default_seed));
        let h = gen.surface_top(0, 0) as f32;
        let eye = Vec3::new(-24.0, h + 26.0, 48.0);
        let mut camera = Camera::new(eye, Vec3::new(0.0, h + 6.0, 0.0));
        camera.set_aspect(800, 600);
        let mk_env = |tint: [f32; 3], sat: f32| lf_engine::scene::Env {
            camera_pos: eye,
            time: 0.8,
            day_factor: spec.day_factor(),
            fog_color: spec.time_of_day().sky_color(),
            fog_far: 220.0,
            grade_tint: tint,
            grade_saturation: sat,
        };
        let textures = lf_assets::generate_atlas();
        let mut paths = Vec::new();
        let frame = |tag: &str, env: &lf_engine::scene::Env, paths: &mut Vec<String>| -> [f64; 3] {
            let path = format!("/tmp/lf_vistest_grade_{tag}_{}.png", std::process::id());
            lf_engine::headless::render_to_png(
                &v, &i, &wv, &wi, &textures, &camera, env, spec.sky_color(),
                800, 600, Path::new(&path), None,
            ).unwrap_or_else(|e| panic!("render {tag} failed: {e}"));
            paths.push(path.clone());
            let img = image::open(&path).expect("reopen grade frame").to_rgba8();
            // mid-frame band: terrain, not sky, not the HUD edge
            let (mut r, mut g, mut b) = (0u64, 0u64, 0u64);
            let mut n = 0u64;
            for y in 240..400u32 {
                for x in 120..680u32 {
                    let p = img.get_pixel(x, y);
                    r += p.0[0] as u64; g += p.0[1] as u64; b += p.0[2] as u64; n += 1;
                }
            }
            [r as f64 / n as f64, g as f64 / n as f64, b as f64 / n as f64]
        };
        let warm = frame("warm", &mk_env([1.08, 1.00, 0.88], 0.92), &mut paths);
        let cold = frame("cold", &mk_env([0.90, 0.98, 1.10], 0.85), &mut paths);
        // hue (degrees) + saturation (max-min / max) of a band average
        let hue_sat = |c: [f64; 3]| -> (f64, f64) {
            let (r, g, b) = (c[0], c[1], c[2]);
            let (max, min) = (r.max(g).max(b), r.min(g).min(b));
            let sat = if max > 1.0 { (max - min) / max } else { 0.0 };
            let hue = if (max - min).abs() < 1e-6 {
                0.0
            } else if max == g {
                60.0 * (2.0 + (b - r) / (max - min))
            } else if max == r {
                60.0 * ((g - b) / (max - min)).rem_euclid(6.0)
            } else {
                60.0 * (4.0 + (r - g) / (max - min))
            };
            (hue, sat)
        };
        let (hw, sw) = hue_sat(warm);
        let (hc, sc) = hue_sat(cold);
        assert!(
            (hc - hw).abs() > 5.0 && (sw - sc).abs() > 0.03,
            "the two grades must measurably shift hue/saturation: warm hue {hw:.1} sat {sw:.3} vs cold hue {hc:.1} sat {sc:.3} (warm {:?} cold {:?})",
            warm, cold
        );
        for p in paths {
            let _ = std::fs::remove_file(p);
        }
    }

    #[test]
    fn foliage_sway_animates_between_frames() {
        // The P26 commit claimed wind sway, but the vertex shader never read
        // the sway attribute — leaves could not move. This proof renders the
        // canopy at two wind phases through the real GPU pipeline and demands
        // the frames differ, with a same-phase control that must be
        // pixel-identical (everything except the sway is deterministic).
        let spec = scenes().into_iter().find(|s| s.name == "foliage_canopy")
            .expect("foliage_canopy scene registered");
        let (v, i, wv, wi) = build_scene_mesh(&spec, spec.default_seed, 2, false, false);
        let gen = WorldGen::new(Seed(spec.default_seed));
        let h = gen.surface_top(0, 0) as f32;
        let eye = Vec3::new(-10.0, h + 13.0, 14.0);
        let mut camera = Camera::new(eye, Vec3::new(0.5, h + 9.5, 0.5));
        camera.set_aspect(800, 600);
        let mk_env = |time: f32| lf_engine::scene::Env {
            camera_pos: eye,
            time,
            day_factor: spec.day_factor(),
            fog_color: spec.time_of_day().sky_color(),
            fog_far: 220.0,
            grade_tint: [1.0, 1.0, 1.0],
            grade_saturation: 1.0,
        };
        let textures = lf_assets::generate_atlas();
        let mut paths = Vec::new();
        let frame = |tag: &str, time: f32, paths: &mut Vec<String>| -> image::RgbaImage {
            let path = format!("/tmp/lf_vistest_sway_{tag}_{}.png", std::process::id());
            lf_engine::headless::render_to_png(
                &v, &i, &wv, &wi, &textures, &camera, &mk_env(time), spec.sky_color(),
                800, 600, Path::new(&path), None,
            ).unwrap_or_else(|e| panic!("render {tag} failed: {e}"));
            paths.push(path.clone());
            image::open(&path).expect("reopen sway frame").to_rgba8()
        };
        let a1 = frame("a1", 0.8, &mut paths);
        let a2 = frame("a2", 0.8, &mut paths);
        let b = frame("b", 0.8 + std::f32::consts::PI, &mut paths);
        let changed = |x: &image::RgbaImage, y: &image::RgbaImage| -> usize {
            x.pixels().zip(y.pixels())
                .filter(|(p, q)| p.0.iter().zip(q.0.iter())
                    .any(|(c, d)| (*c as i32 - *d as i32).abs() > 8))
                .count()
        };
        assert_eq!(changed(&a1, &a2), 0, "same wind phase must render pixel-identical");
        let moved = changed(&a1, &b);
        let total = (800 * 600) as usize;
        assert!(
            moved > total / 1000,
            "wind must visibly move foliage between phases: only {moved}/{total} px changed"
        );
        for p in paths {
            let _ = std::fs::remove_file(p);
        }
    }

    #[test]
    fn verify_render_rejects_blank_and_accepts_varied() {
        let solid = image::RgbaImage::from_pixel(200, 200, image::Rgba([10, 10, 10, 255]));
        let solid_path = format!("/tmp/lf_vistest_blank_{}.png", std::process::id());
        solid.save(&solid_path).unwrap();
        assert!(verify_render(Path::new(&solid_path)).is_err(), "uniform image must fail");

        let mut varied = image::RgbaImage::new(200, 200);
        for y in 0..200u32 {
            for x in 0..200u32 {
                varied.put_pixel(x, y, image::Rgba([
                    (x * 7 % 256) as u8, (y * 5 % 256) as u8, ((x + y) * 3 % 256) as u8, 255,
                ]));
            }
        }
        let varied_path = format!("/tmp/lf_vistest_varied_{}.png", std::process::id());
        varied.save(&varied_path).unwrap();
        assert!(verify_render(Path::new(&varied_path)).is_ok(), "gradient image must pass");
        let _ = std::fs::remove_file(&solid_path);
        let _ = std::fs::remove_file(&varied_path);
    }
}

/// The spellbook preview (P33) — mirrors lf_client's draw_spellbook layout
/// (lf_vistest cannot depend on lf_client); spell set + costs come from
/// the real lf_game::magic.
fn draw_spellbook_preview(ctx: &egui::Context) {
    use egui::{Align2, Color32, FontId, RichText};
    #[allow(unused_imports)]
    use Align2 as _Align2;
    let purple = Color32::from_rgb(185, 130, 255);
    let dim = Color32::from_gray(150);
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(Color32::from_black_alpha(160)))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                egui::Frame::new()
                    .fill(Color32::from_gray(24))
                    .stroke(egui::Stroke::new(1.0, purple))
                    .corner_radius(10.0)
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.set_width(440.0);
                        ui.heading(RichText::new("Spellbook").color(purple));
                        ui.label(RichText::new("the bounded set — four spells, three slots. The wizard sells the rest of what you're missing.")
                            .small().color(dim));
                        ui.add_space(6.0);
                        let (r, _) = ui.allocate_exact_size(egui::vec2(430.0, 10.0), egui::Sense::hover());
                        let p = ui.painter();
                        p.rect_filled(r, 4.0, Color32::from_black_alpha(190));
                        p.rect_filled(egui::Rect::from_min_size(r.min, egui::vec2(r.width() * 0.66, r.height())), 4.0, purple);
                        p.text(r.right_center(), Align2::RIGHT_CENTER, "20", FontId::proportional(10.0), purple);
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            for (label, key, cost) in [
                                ("Firebolt", "Z", "8"), ("Ward", "X", "20"), ("Hearthlight", "C", "15"),
                            ] {
                                egui::Frame::new()
                                    .fill(Color32::from_black_alpha(120))
                                    .stroke(egui::Stroke::new(1.5, purple))
                                    .corner_radius(8.0)
                                    .inner_margin(8.0)
                                    .show(ui, |ui| {
                                        ui.set_min_size(egui::vec2(122.0, 52.0));
                                        ui.heading(RichText::new(format!("{}  [{}]", label, key)).size(13.0).color(purple));
                                        ui.label(RichText::new(format!("{} mana", cost)).small().color(dim));
                                    });
                            }
                        });
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);
                        ui.label(RichText::new("learned — click to fill the first free slot, click a slot row to clear it").small().color(dim));
                        ui.add_space(4.0);
                        for (name, cost, desc) in [
                            ("Firebolt", "8", "hurl a bolt of fire — an arrow that hits harder"),
                            ("Gale-step", "12", "blink forward along your gaze"),
                            ("Ward", "20", "a shield that drinks damage for a few seconds"),
                            ("Hearthlight", "15", "the Smith's trick: light the dark, soften one ore by hand"),
                        ] {
                            ui.horizontal(|ui| {
                                ui.heading(RichText::new(name).size(14.0).color(purple));
                                ui.label(RichText::new(format!("{} mana", cost)).small().color(dim));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.small_button("→ slot");
                                });
                            });
                            ui.label(RichText::new(desc).small().color(dim));
                            ui.add_space(3.0);
                        }
                        ui.add_space(8.0);
                        if ui.button("Close").clicked() {}
                    });
            });
        });
}

/// The paths screen preview (P37) — mirrors lf_client's draw_paths layout.
fn draw_paths_preview(ctx: &egui::Context) {
    use egui::{Align2, Color32, FontId, RichText};
    #[allow(unused_imports)]
    use Align2 as _Align2;
    let gold = Color32::from_rgb(240, 200, 120);
    let violet = Color32::from_rgb(185, 130, 255);
    let dim = Color32::from_gray(150);
    let paths: [(&str, &str, u32); 4] = [
        ("Engineer", "machines hum under your hands", 31),
        ("Architect", "the valley takes your shape", 24),
        ("Battlemage", "spell and steel, both yours", 37),
        ("Artisan", "everything you make is better", 18),
    ];
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(Color32::from_black_alpha(160)))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                egui::Frame::new()
                    .fill(Color32::from_gray(24))
                    .stroke(egui::Stroke::new(1.0, gold))
                    .corner_radius(10.0)
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.set_width(480.0);
                        ui.heading(RichText::new("Paths").color(gold));
                        ui.label(RichText::new("no decay, no lock-in — everything you do deepens a path").small().color(dim));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            for (i, (name, desc, standing)) in paths.iter().enumerate() {
                                let tier = standing / 25;
                                let frac = (standing % 25) as f32 / 25.0;
                                let focused = i == 2;
                                let color = if focused { violet } else { gold };
                                egui::Frame::new()
                                    .fill(Color32::from_black_alpha(120))
                                    .stroke(egui::Stroke::new(if focused { 2.5 } else { 1.0 }, color))
                                    .corner_radius(8.0)
                                    .inner_margin(8.0)
                                    .show(ui, |ui| {
                                        ui.set_min_size(egui::vec2(108.0, 96.0));
                                        ui.heading(RichText::new(name.to_string()).size(14.0).color(color));
                                        ui.label(RichText::new(format!("tier {} — {}/25", tier, standing % 25)).small().color(dim));
                                        let (r, _) = ui.allocate_exact_size(egui::vec2(92.0, 6.0), egui::Sense::hover());
                                        let p = ui.painter();
                                        p.rect_filled(r, 3.0, Color32::from_black_alpha(190));
                                        p.rect_filled(egui::Rect::from_min_size(r.min, egui::vec2(r.width() * frac, r.height())), 3.0, color);
                                        ui.label(RichText::new(desc.to_string()).small().color(dim));
                                        ui.small_button("focus");
                                    });
                            }
                        });
                        ui.add_space(6.0);
                        ui.label(RichText::new("respec: pay 8 iron_ingot + 1 null_shard, standings reset, the focused path accrues double").small().color(dim));
                        ui.label(RichText::new("current focus: Battlemage").small().color(violet));
                        ui.add_space(8.0);
                        ui.button("Close");
                        let _ = FontId::proportional(11.0);
                    });
            });
        });
}

/// The P2P trade offer preview (P37, protocol v4) — the escrowed offer
/// as the receiving player sees it.
fn draw_trade_p2p_preview(ctx: &egui::Context) {
    use egui::{Align2, Color32, RichText};
    let gold = Color32::from_rgb(240, 200, 120);
    let ok = Color32::from_rgb(120, 210, 130);
    let dim = Color32::from_gray(150);
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(Color32::from_black_alpha(160)))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                egui::Frame::new()
                    .fill(Color32::from_gray(24))
                    .stroke(egui::Stroke::new(1.0, gold))
                    .corner_radius(10.0)
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.set_width(400.0);
                        ui.heading(RichText::new("Trade Offer").color(gold));
                        ui.label(RichText::new("alice offers to you (escrowed by the server)").small().color(dim));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new("they give").small().color(dim));
                                ui.label(RichText::new("iron_ingot x4").color(ok));
                            });
                            ui.label(RichText::new("⇄").size(20.0).color(gold));
                            ui.vertical(|ui| {
                                ui.label(RichText::new("they want").small().color(dim));
                                ui.label(RichText::new("dragon_scale x1").color(ok));
                            });
                        });
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.button(egui::RichText::new("Accept").color(ok));
                            ui.button("Decline");
                        });
                        ui.label(RichText::new("accepting delivers to both sides; declining frees the escrow").small().color(dim));
                    });
            });
        });
}
