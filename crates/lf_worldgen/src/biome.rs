//! Biome table: 30 biomes with surfaces, tree kinds, and classification.

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
}

impl Biome {
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
        }
    }

    /// Surface block for this biome (lf_voxel block ids).
    pub fn surface_block(self) -> u32 {
        use lf_voxel::registry::block;
        use Biome::*;
        match self {
            Tundra | SnowyTaiga | SnowySlope | SnowyPeaks | FrozenOcean => block::SNOW,
            IceSpikes => block::ICE,
            Desert | Beach => block::SAND,
            Badlands => block::RED_SAND,
            Savanna | WindsweptSavanna => block::GRASS,
            Swamp => block::MOSS,
            StonyShore | Mountains => block::STONE,
            Ocean | DeepOcean | WarmOcean => block::SAND,
            _ => block::GRASS,
        }
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
            Tundra | SnowySlope => TreeKind::Spruce,
            _ => TreeKind::None,
        }
    }

    /// Above-water ocean surface freezes.
    pub fn freezes(self) -> bool {
        matches!(self, Biome::FrozenOcean | Biome::IceSpikes)
    }
}

/// Distinct tree shapes worldgen can build.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TreeKind {
    None,
    OakSparse,
    Oak,
    Birch,
    DarkOak,
    Spruce,
    GiantSpruce,
    Jungle,
    Cherry,
    Pale,
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
            TreeKind::Spruce => (block::SPRUCE_LOG, block::SPRUCE_LEAVES, 6, 2),
            TreeKind::GiantSpruce => (block::SPRUCE_LOG, block::SPRUCE_LEAVES, 12, 3),
            TreeKind::Jungle => (block::LOG, block::LEAVES, 9, 3),
            TreeKind::Cherry => (block::CHERRY_LOG, block::CHERRY_LEAVES, 5, 3),
            TreeKind::Pale => (block::LOG, block::PALE_LEAVES, 5, 2),
        }
    }

    /// Cone-shaped canopies (spruce) layer differently from blobs.
    pub fn is_conifer(self) -> bool {
        matches!(self, TreeKind::Spruce | TreeKind::GiantSpruce)
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
        return if t < 0.25 { FrozenOcean } else if t > 0.8 && h < 0.4 { Beach } else if v > 0.75 { StonyShore } else { Beach };
    }

    // --- land climate grid
    if t < 0.2 {
        // frigid
        if h > 0.7 { return if v > 0.7 { GiantTaiga } else { SnowyTaiga }; }
        return if v > 0.78 { IceSpikes } else { Tundra };
    }
    if t < 0.4 {
        // cool: taiga / cold forests
        if h > 0.75 { return if v > 0.6 { Swamp } else { Taiga }; }
        if h > 0.45 { return if v > 0.66 { BirchForest } else if v > 0.33 { Forest } else { Taiga }; }
        return if v > 0.5 { Taiga } else { Tundra };
    }
    if t > 0.75 {
        // hot
        if h < 0.35 {
            return if v > 0.82 { Badlands } else if v > 0.66 { WindsweptSavanna } else { Desert };
        }
        return Savanna;
    }
    if t > 0.6 && h < 0.45 {
        return if v > 0.7 { Savanna } else { Desert };
    }
    // temperate
    if h > 0.8 {
        if v < 0.12 && t > 0.35 { return MushroomHollow; }
        return if v > 0.72 { PaleGarden } else if v > 0.36 { DarkForest } else { Forest };
    }
    if h > 0.6 {
        return if v > 0.78 { CherryGrove } else if v > 0.5 { FlowerForest } else if v > 0.25 { Forest } else { BirchForest };
    }
    if h > 0.35 && t > 0.55 {
        return if v > 0.8 { Jungle } else if v > 0.6 { Swamp } else { Forest };
    }
    Meadow
}
