//! Biome table: 46 biomes with surfaces, tree kinds, and classification.

use lf_voxel::BlockState;

/// All 30 surface biomes. Classification uses temperature, humidity,
/// terrain height, and a slow "variant" channel that splits shared
/// climate bands into neighbors (Forest/Birch/Dark Forest etc).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Biome {
    // plains & forests
    Meadow,
    FlowerForest,
    Forest,
    BirchForest,
    DarkForest,
    PaleGarden,
    CherryGrove,
    // cold
    Taiga,
    SnowyTaiga,
    GiantTaiga,
    Tundra,
    IceSpikes,
    SnowySlope,
    SnowyPeaks,
    FrozenOcean,
    // warm & wet
    Jungle,
    Swamp,
    // dry
    Savanna,
    WindsweptSavanna,
    Desert,
    Badlands,
    // shores & oceans
    Beach,
    StonyShore,
    Ocean,
    DeepOcean,
    WarmOcean,
    // heights
    Highlands,
    Mountains,
    WindsweptHills,
    MushroomHollow,
    // lore-and-visuals C1: the volcanic belt (Ironborn home)
    Volcanic,
    // king-quest B: 15 new biomes (variant-channel splits of the climate
    // grid, so each spawns where its lore palette belongs)
    Oasis,
    RedwoodForest,
    Mangrove,
    AspenGrove,
    BaobabFields,
    WillowWetlands,
    PaintedDunes,
    FrostMeadow,
    Emberwood,
    LavenderFields,
    MapleForest,
    PineBarrens,
    SaltFlats,
    FoggyFjord,
    SunflowerPlains,
}

impl Biome {
    /// Every variant, declaration order (contact sheets + tests).
    pub const ALL: [Biome; 46] = [
        Biome::Meadow, Biome::FlowerForest, Biome::Forest, Biome::BirchForest,
        Biome::DarkForest, Biome::PaleGarden, Biome::CherryGrove, Biome::Taiga,
        Biome::SnowyTaiga, Biome::GiantTaiga, Biome::Tundra, Biome::IceSpikes,
        Biome::SnowySlope, Biome::SnowyPeaks, Biome::FrozenOcean, Biome::Jungle,
        Biome::Swamp, Biome::Savanna, Biome::WindsweptSavanna, Biome::Desert,
        Biome::Badlands, Biome::Beach, Biome::StonyShore, Biome::Ocean,
        Biome::DeepOcean, Biome::WarmOcean, Biome::Highlands, Biome::Mountains,
        Biome::WindsweptHills, Biome::MushroomHollow, Biome::Volcanic,
        Biome::Oasis, Biome::RedwoodForest, Biome::Mangrove, Biome::AspenGrove,
        Biome::BaobabFields, Biome::WillowWetlands, Biome::PaintedDunes,
        Biome::FrostMeadow, Biome::Emberwood, Biome::LavenderFields,
        Biome::MapleForest, Biome::PineBarrens, Biome::SaltFlats,
        Biome::FoggyFjord, Biome::SunflowerPlains,
    ];

    pub fn name(self) -> &'static str {
        use Biome::*;
        match self {
            Meadow => "Meadow",
            FlowerForest => "Flower Forest",
            Forest => "Forest",
            BirchForest => "Birch Forest",
            DarkForest => "Dark Forest",
            PaleGarden => "Pale Garden",
            CherryGrove => "Cherry Grove",
            Taiga => "Taiga",
            SnowyTaiga => "Snowy Taiga",
            GiantTaiga => "Giant Taiga",
            Tundra => "Snowy Plains",
            IceSpikes => "Ice Spikes",
            SnowySlope => "Snowy Slopes",
            SnowyPeaks => "Snowy Peaks",
            FrozenOcean => "Frozen Ocean",
            Jungle => "Jungle",
            Swamp => "Swamp",
            Savanna => "Savanna",
            WindsweptSavanna => "Windswept Savanna",
            Desert => "Desert",
            Badlands => "Badlands",
            Beach => "Beach",
            StonyShore => "Stony Shore",
            Ocean => "Ocean",
            DeepOcean => "Deep Ocean",
            WarmOcean => "Warm Ocean",
            Highlands => "Highlands",
            Mountains => "Mountains",
            WindsweptHills => "Windswept Hills",
            MushroomHollow => "Mushroom Hollow",
            Volcanic => "Volcanic",
            Oasis => "Oasis",
            RedwoodForest => "Redwood Forest",
            Mangrove => "Mangrove",
            AspenGrove => "Aspen Grove",
            BaobabFields => "Baobab Fields",
            WillowWetlands => "Willow Wetlands",
            PaintedDunes => "Painted Dunes",
            FrostMeadow => "Frost Meadow",
            Emberwood => "Emberwood",
            LavenderFields => "Lavender Fields",
            MapleForest => "Maple Forest",
            PineBarrens => "Pine Barrens",
            SaltFlats => "Salt Flats",
            FoggyFjord => "Foggy Fjord",
            SunflowerPlains => "Sunflower Plains",
        }
    }

    /// Surface block for this biome (lf_voxel block ids).
    pub fn surface_block(self) -> u32 {
        use lf_voxel::registry::block;
        use Biome::*;
        match self {
            SnowyTaiga | SnowySlope | SnowyPeaks | FrozenOcean => block::SNOW,
            IceSpikes => block::ICE,
            Desert | Beach => block::SAND,
            Badlands => block::RED_SAND,
            Savanna => block::GILDED_GRASS,
            WindsweptSavanna => block::SAVANNA_GRASS,
            Swamp => block::BOG_PEAT,
            Jungle => block::JUNGLE_GRASS,
            MushroomHollow => block::MYCELIUM,
            StonyShore | Mountains => block::STONE,
            Volcanic => block::VOLCANIC_BASALT,
            Ocean | DeepOcean | WarmOcean => block::SAND,
            // Tundra: blueish icy soil, its marker vs the snow family (C1)
            Tundra => block::PERMAFROST,
            // king-quest B identities
            SaltFlats => block::SALT,
            PaintedDunes => block::RED_SAND,
            Emberwood => block::VOLCANIC_BASALT,
            Mangrove | WillowWetlands => block::BOG_PEAT,
            FrostMeadow => block::SNOW,
            _ => block::GRASS,
        }
    }

    /// Sub-surface filler under the surface band.
    pub fn filler_block(self) -> u32 {
        use lf_voxel::registry::block;
        use Biome::*;
        match self {
            Badlands => block::MESA_TERRACOTTA,
            Desert | Beach => block::SAND,
            StonyShore | Mountains | SnowyPeaks | Volcanic | Emberwood => block::STONE,
            SaltFlats | PaintedDunes => block::SAND,
            _ => block::DIRT,
        }
    }

    /// Cold biomes: snow/taiga family — drives snow weather (Step 19) and
    /// cold-adapted passive spawns (Step 18).
    pub fn is_cold(self) -> bool {
        use Biome::*;
        matches!(self, Tundra | SnowyTaiga | GiantTaiga | IceSpikes | SnowySlope
            | SnowyPeaks | FrozenOcean | FrostMeadow | FoggyFjord)
    }

    /// Tree shape used by worldgen (None = treeless).
    pub fn tree_kind(self) -> TreeKind {
        use Biome::*;
        match self {
            Forest | FlowerForest => TreeKind::Oak,
            Meadow => TreeKind::OakSparse,
            BirchForest => TreeKind::Birch,
            DarkForest => TreeKind::DarkOak,
            PaleGarden => TreeKind::Pale,
            CherryGrove => TreeKind::Cherry,
            Taiga | SnowyTaiga => TreeKind::Spruce,
            GiantTaiga => TreeKind::GiantSpruce,
            Jungle => TreeKind::Jungle,
            Swamp => TreeKind::OakSparse,
            Savanna => TreeKind::OakSparse,
            WindsweptSavanna => TreeKind::OakSparse,
            Tundra => TreeKind::SpruceSparse,
            SnowySlope => TreeKind::Spruce,
            // C1: giant mushrooms grew during the Long Winter
            MushroomHollow => TreeKind::Mushroom,
            // king-quest B: the new biomes carry their own species
            Oasis => TreeKind::Palm,
            RedwoodForest => TreeKind::Redwood,
            Mangrove => TreeKind::Mangrove,
            AspenGrove => TreeKind::Aspen,
            BaobabFields => TreeKind::Baobab,
            WillowWetlands => TreeKind::Willow,
            FrostMeadow => TreeKind::SpruceSparse,
            Emberwood => TreeKind::Ember,
            MapleForest => TreeKind::Maple,
            PineBarrens => TreeKind::SpruceSparse,
            FoggyFjord => TreeKind::SpruceSparse,
            SunflowerPlains => TreeKind::OakSparse,
            _ => TreeKind::None,
        }
    }

    /// Above-water ocean surface freezes.
    pub fn freezes(self) -> bool {
        matches!(self, Biome::FrozenOcean | Biome::IceSpikes)
    }

    /// Ground-cover identity (ui-world-craft E3): (density 0..1, feature
    /// blocks). Density is the fraction of surface columns carrying a
    /// feature; which feature wins on a given column is hash-driven. This
    /// is the second half of the 5-second test — palette + one thing no
    /// other biome has.
    pub fn surface_features(self) -> (f32, &'static [u32]) {
        use lf_voxel::registry::block;
        use Biome::*;
        match self {
            // lush temperate band — grass dominates, flowers accent
            // (a meadow that's half red blooms reads as noise, not identity)
            Meadow => (0.15, &[block::TALL_GRASS, block::TALL_GRASS, block::FLOWER]),
            FlowerForest => (0.35, &[block::FLOWER, block::FLOWER, block::TALL_GRASS]),
            Forest | BirchForest => (0.30, &[block::TALL_GRASS, block::TALL_GRASS, block::FLOWER]),
            DarkForest => (0.35, &[block::TALL_GRASS]),
            PaleGarden => (0.15, &[block::DEAD_SHRUB]),
            CherryGrove => (0.30, &[block::TALL_GRASS, block::FLOWER]),
            Jungle => (0.40, &[block::TALL_GRASS, block::TALL_GRASS, block::FLOWER]),
            // wet cold band
            Taiga => (0.12, &[block::DEAD_SHRUB]),
            GiantTaiga => (0.14, &[block::DEAD_SHRUB, block::MOSS]),
            SnowyTaiga => (0.08, &[block::DEAD_SHRUB]),
            Tundra => (0.02, &[block::DEAD_SHRUB]),
            // dry hot band
            Savanna => (0.18, &[block::DRY_GRASS]),
            WindsweptSavanna => (0.12, &[block::DRY_GRASS, block::DEAD_SHRUB]),
            Desert => (0.05, &[block::CACTUS, block::DEAD_SHRUB]),
            Badlands => (0.04, &[block::DEAD_SHRUB]),
            // wet hot band
            Swamp => (0.25, &[block::DEAD_SHRUB, block::TALL_GRASS]),
            MushroomHollow => (0.35, &[block::MUSHROOM_CAP]),
            // heights & shores: bare by nature
            Highlands => (0.04, &[block::FLOWER]),
            Mountains | WindsweptHills => (0.02, &[block::STONE]),
            Volcanic => (0.06, &[block::VOLCANIC_BASALT]),
            Beach | StonyShore | IceSpikes | SnowySlope | SnowyPeaks => (0.0, &[]),
            Ocean | DeepOcean | WarmOcean | FrozenOcean => (0.0, &[]),
            // king-quest B identities
            Oasis => (0.10, &[block::TALL_GRASS, block::CACTUS]),
            RedwoodForest => (0.20, &[block::TALL_GRASS, block::MOSS]),
            Mangrove => (0.30, &[block::DEAD_SHRUB, block::TALL_GRASS]),
            AspenGrove => (0.25, &[block::TALL_GRASS, block::FLOWER]),
            BaobabFields => (0.15, &[block::DRY_GRASS]),
            WillowWetlands => (0.30, &[block::TALL_GRASS, block::DEAD_SHRUB]),
            PaintedDunes => (0.03, &[block::DEAD_SHRUB]),
            FrostMeadow => (0.08, &[block::FLOWER, block::DEAD_SHRUB]),
            Emberwood => (0.05, &[block::DEAD_SHRUB]),
            LavenderFields => (0.45, &[block::LAVENDER, block::LAVENDER, block::TALL_GRASS]),
            MapleForest => (0.20, &[block::TALL_GRASS, block::FLOWER]),
            PineBarrens => (0.10, &[block::DEAD_SHRUB, block::MOSS]),
            SaltFlats => (0.0, &[]),
            FoggyFjord => (0.03, &[block::MOSS]),
            SunflowerPlains => (0.40, &[block::SUNFLOWER, block::SUNFLOWER, block::TALL_GRASS]),
        }
    }
}

/// Distinct tree shapes worldgen can build.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TreeKind {
    None,
    OakSparse,
    Oak,
    /// Sparse wind-bent conifers — Tundra's marker vs the dense SnowyTaiga.
    SpruceSparse,
    Birch,
    DarkOak,
    Spruce,
    GiantSpruce,
    Jungle,
    Cherry,
    Pale,
    /// Giant mushroom: pale trunk, red-cap canopy (MushroomHollow).
    Mushroom,
    // king-quest B: nine new species so each new biome has its own tree
    Palm,
    Acacia,
    Mangrove,
    Redwood,
    Maple,
    Aspen,
    Willow,
    Baobab,
    Ember,
}

impl TreeKind {
    /// (log block, leaves block, trunk height base, canopy radius)
    pub fn blocks(self) -> (u32, u32, i32, i32) {
        use lf_voxel::registry::block;
        match self {
            TreeKind::None => (block::AIR, block::AIR, 0, 0),
            TreeKind::OakSparse | TreeKind::Oak => (block::LOG, block::LEAVES, 4, 2),
            TreeKind::Birch => (block::BIRCH_LOG, block::BIRCH_LEAVES, 6, 2),
            TreeKind::DarkOak => (block::DARK_LOG, block::DARK_LEAVES, 5, 3),
            TreeKind::Spruce | TreeKind::SpruceSparse => (block::SPRUCE_LOG, block::SPRUCE_LEAVES, 6, 2),
            TreeKind::GiantSpruce => (block::SPRUCE_LOG, block::SPRUCE_LEAVES, 12, 3),
            TreeKind::Jungle => (block::LOG, block::LEAVES, 9, 3),
            TreeKind::Cherry => (block::CHERRY_LOG, block::CHERRY_LEAVES, 5, 3),
            TreeKind::Pale => (block::LOG, block::PALE_LEAVES, 5, 2),
            TreeKind::Mushroom => (block::BIRCH_LOG, block::MUSHROOM_CAP, 3, 2),
            TreeKind::Palm => (block::PALM_LOG, block::PALM_LEAVES, 7, 2),
            TreeKind::Acacia => (block::ACACIA_LOG, block::ACACIA_LEAVES, 5, 4),
            TreeKind::Mangrove => (block::MANGROVE_LOG, block::MANGROVE_LEAVES, 6, 3),
            TreeKind::Redwood => (block::REDWOOD_LOG, block::REDWOOD_LEAVES, 13, 3),
            TreeKind::Maple => (block::MAPLE_LOG, block::MAPLE_LEAVES, 5, 3),
            TreeKind::Aspen => (block::BIRCH_LOG, block::ASPEN_LEAVES, 7, 1),
            TreeKind::Willow => (block::LOG, block::WILLOW_LEAVES, 6, 4),
            TreeKind::Baobab => (block::BAOBAB_LOG, block::ACACIA_LEAVES, 6, 5),
            TreeKind::Ember => (block::EMBER_LOG, block::EMBER_LEAVES, 4, 2),
        }
    }

    /// Cone-shaped canopies (spruce) layer differently from blobs.
    pub fn is_conifer(self) -> bool {
        matches!(self, TreeKind::Spruce | TreeKind::GiantSpruce | TreeKind::Redwood)
    }
}

/// Pure biome classification from climate + elevation + variant channel
/// (all inputs roughly [0..1] except height in blocks).
pub fn biome_from(t: f32, h: f32, height: i32, variant: f32) -> Biome {
    use Biome::*;
    let v = variant.clamp(0.0, 0.999);

    // --- high elevations
    if height >= 150 {
        return if t < 0.3 { SnowyPeaks } else { Mountains };
    }
    if height >= 128 {
        return if t < 0.35 { SnowySlope } else if t > 0.75 && h < 0.35 { WindsweptHills } else { Highlands };
    }
    // --- oceans
    if height < 40 {
        return if t < 0.25 { FrozenOcean } else if t > 0.75 { WarmOcean } else { DeepOcean };
    }
    if height < 50 {
        return Ocean;
    }
    // --- shores
    if height <= 53 {
        if t < 0.25 { return if v > 0.5 { FoggyFjord } else { FrozenOcean }; }
        return if t > 0.8 && h < 0.4 { Beach } else if v > 0.75 { StonyShore } else { Beach };
    }

    // --- land climate grid
    if t < 0.2 {
        // frigid
        if h > 0.7 { return if v > 0.7 { GiantTaiga } else { SnowyTaiga }; }
        if v > 0.9 { return FrostMeadow; }
        return if v > 0.78 { IceSpikes } else { Tundra };
    }
    if t < 0.4 {
        // cool: taiga / cold forests
        if h > 0.75 { return if v > 0.6 { Swamp } else { Taiga }; }
        if h > 0.45 {
            return if v > 0.78 { MapleForest }
                else if v > 0.66 { BirchForest }
                else if v > 0.33 { Forest }
                else { Taiga };
        }
        if v > 0.85 { return PineBarrens; }
        return if v > 0.5 { Taiga } else { Tundra };
    }
    if t > 0.75 {
        // hot
        if h < 0.35 {
            // the volcanic belt: the rarest slice of hot dry land (C1);
            // king-quest B splits its siblings off the same variant channel
            if v < 0.05 && height > 64 && height < 150 { return Emberwood; }
            if v < 0.10 && height > 64 && height < 150 { return Volcanic; }
            if v < 0.16 { return Oasis; }
            if v < 0.30 { return PaintedDunes; }
            return if v > 0.82 { Badlands } else if v > 0.66 { WindsweptSavanna } else { Desert };
        }
        if v < 0.12 { return BaobabFields; }
        return Savanna;
    }
    if t > 0.6 && h < 0.45 {
        if v < 0.15 { return SaltFlats; }
        return if v > 0.7 { Savanna } else { Desert };
    }
    // temperate
    if h > 0.8 {
        if v < 0.06 && t > 0.35 { return MushroomHollow; }
        if v < 0.12 && t > 0.35 { return RedwoodForest; }
        return if v > 0.72 { PaleGarden } else if v > 0.36 { DarkForest } else { Forest };
    }
    if h > 0.6 {
        return if v > 0.78 { CherryGrove }
            else if v > 0.72 { WillowWetlands }
            else if v > 0.62 { LavenderFields }
            else if v > 0.5 { FlowerForest }
            else if v > 0.25 { Forest }
            else if v > 0.12 { AspenGrove }
            else { BirchForest };
    }
    if h > 0.35 && t > 0.55 {
        return if v > 0.8 { Jungle } else if v > 0.6 { Swamp } else { Forest };
    }
    if v > 0.6 { return SunflowerPlains; }
    Meadow
}
