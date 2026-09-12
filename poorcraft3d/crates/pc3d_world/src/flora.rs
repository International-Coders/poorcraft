//! Wilderness placement — the authoritative WHAT-GROWS-WHERE (NWR-007).
//!
//! Pure and deterministic: every plant decision is a hash of (seed,
//! slot) gated by the biome and surface of the ONE generator. The
//! renderer turns kinds into instanced meshes; nothing here knows about
//! GPUs. Slots are a 4 m grid; trees reserve the even-diagonal subgrid
//! (>= 8 m apart by construction); one LANDMARK (standing stone) may
//! exist per region where the hash and the ground allow it.

use crate::coords::RegionCoord;
use crate::gen::{Biome, WorldGen};

/// The slot grid: 4 m between neighbors.
pub const SLOT_M: i64 = 4;

/// One wilderness kind. The renderer maps each to an instanced GLB
/// (trees/rocks/log/landmark) or a cutout card field (grass).
///
/// The thousand-asset expansion families (loop 439's 1000-GLB batch)
/// land here as kinds: each names the GLB family on disk that the
/// streamer instances (`flora.<family>_vNN.glb`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlantKind {
    /// Conical evergreen (montane).
    TreePine,
    /// Broad billowing canopy.
    TreeBroadleaf,
    /// Slender pale trunk, airy crown.
    TreeBirch,
    /// Rounded boulder.
    RockBoulder,
    /// Tall fractured spire.
    RockSpire,
    /// Flat slab.
    RockSlab,
    /// Low leafy blob.
    Shrub,
    /// Grass tuft cards (wind-animated).
    Grass,
    /// Fallen log.
    Log,
    // --- thousand-asset expansion families (in the wild since this table
    // --- learned them; every one draws its own GLB variant batch) ---
    /// Arched fronds (forest floor).
    Fern,
    /// Blooming stems (meadow accents).
    Flower,
    /// Pale-stemmed mushroom cluster (damp shade).
    Mushroom,
    /// Luminous mushroom (Anima-touched shade).
    Glowcap,
    /// Tall waterside blades.
    Reed,
    /// Cut trunk with root flares (forestry trace).
    Stump,
    /// Exposed root tendrils.
    Root,
    /// Ground thorn tangle.
    Bramble,
    /// Dark thorny bush.
    Thornbush,
    /// Pebble scatter.
    Pebble,
    /// Flat wet-rimmed stone.
    PuddleStone,
    /// Moss-covered boulder.
    MossRock,
    /// Bare gnarled standing trunk.
    DeadTree,
    /// Standing dead trunk, broken top.
    Snag,
    /// Ruin column (old-world fragment).
    RockColumn,
    /// Stacked trail stones.
    Cairn,
    /// Natural rock arch (the opening is the point).
    ArchRock,
    /// Quartz shard cluster (stone-resonant Anima).
    Crystal,
    /// Black volcanic shard cluster.
    Obsidian,
    /// Ice spike cluster.
    IceShard,
}

impl PlantKind {
    /// Every kind, one place — the renderer's kind table and the laws
    /// iterate THIS so world and picture can never drift apart.
    pub const ALL: [PlantKind; 29] = [
        PlantKind::TreePine,
        PlantKind::TreeBroadleaf,
        PlantKind::TreeBirch,
        PlantKind::RockBoulder,
        PlantKind::RockSpire,
        PlantKind::RockSlab,
        PlantKind::Shrub,
        PlantKind::Grass,
        PlantKind::Log,
        PlantKind::Fern,
        PlantKind::Flower,
        PlantKind::Mushroom,
        PlantKind::Glowcap,
        PlantKind::Reed,
        PlantKind::Stump,
        PlantKind::Root,
        PlantKind::Bramble,
        PlantKind::Thornbush,
        PlantKind::Pebble,
        PlantKind::PuddleStone,
        PlantKind::MossRock,
        PlantKind::DeadTree,
        PlantKind::Snag,
        PlantKind::RockColumn,
        PlantKind::Cairn,
        PlantKind::ArchRock,
        PlantKind::Crystal,
        PlantKind::Obsidian,
        PlantKind::IceShard,
    ];

    /// Tree-class kinds reserve the even-diagonal subgrid and refuse
    /// steep ground — anything trunk-sized, living or dead.
    pub fn is_tree(self) -> bool {
        matches!(
            self,
            PlantKind::TreePine
                | PlantKind::TreeBroadleaf
                | PlantKind::TreeBirch
                | PlantKind::DeadTree
                | PlantKind::Snag
        )
    }

    /// Solid for movement: trunks, rocks, logs, and the upright
    /// expansion masses (columns, cairns, crystals, shards). Ground
    /// flora (ferns, blooms, reeds, roots, stumps you step over,
    /// brambles you push through) is walkable — the simple-collision
    /// contract: what blocks is what looks unpassable at chest height.
    /// The rock ARCH is walkable because its opening is its purpose
    /// (a trunk column would seal the very thing an arch offers).
    pub fn blocks_movement(self) -> bool {
        !matches!(
            self,
            PlantKind::Shrub
                | PlantKind::Grass
                | PlantKind::Fern
                | PlantKind::Flower
                | PlantKind::Mushroom
                | PlantKind::Glowcap
                | PlantKind::Reed
                | PlantKind::Stump
                | PlantKind::Root
                | PlantKind::Bramble
                | PlantKind::Thornbush
                | PlantKind::Pebble
                | PlantKind::PuddleStone
                | PlantKind::ArchRock
        )
    }

    /// Sways in the wind (cards and airy growth; rigid wood and stone
    /// stand still).
    pub fn wind(self) -> f32 {
        match self {
            PlantKind::Grass => 1.0,
            PlantKind::Reed => 0.6,
            PlantKind::Flower => 0.45,
            PlantKind::Shrub => 0.35,
            PlantKind::Fern => 0.3,
            PlantKind::TreeBirch => 0.15,
            PlantKind::Bramble | PlantKind::Thornbush => 0.1,
            _ => 0.0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            PlantKind::TreePine => "flora.tree_pine",
            PlantKind::TreeBroadleaf => "flora.tree_broadleaf",
            PlantKind::TreeBirch => "flora.tree_birch",
            PlantKind::RockBoulder => "flora.rock_boulder",
            PlantKind::RockSpire => "flora.rock_spire",
            PlantKind::RockSlab => "flora.rock_slab",
            PlantKind::Shrub => "flora.shrub",
            PlantKind::Grass => "flora.grass_tuft",
            PlantKind::Log => "flora.log_fallen",
            PlantKind::Fern => "flora.fern",
            PlantKind::Flower => "flora.flower",
            PlantKind::Mushroom => "flora.mushroom",
            PlantKind::Glowcap => "flora.glowcap",
            PlantKind::Reed => "flora.reed",
            PlantKind::Stump => "flora.stump",
            PlantKind::Root => "flora.root",
            PlantKind::Bramble => "flora.bramble",
            PlantKind::Thornbush => "flora.thornbush",
            PlantKind::Pebble => "flora.pebble",
            PlantKind::PuddleStone => "flora.puddle_stone",
            PlantKind::MossRock => "flora.moss_rock",
            PlantKind::DeadTree => "flora.deadtree",
            PlantKind::Snag => "flora.snag",
            PlantKind::RockColumn => "flora.column",
            PlantKind::Cairn => "flora.cairn",
            PlantKind::ArchRock => "flora.arch_rock",
            PlantKind::Crystal => "flora.crystal",
            PlantKind::Obsidian => "flora.obsidian",
            PlantKind::IceShard => "flora.ice_shard",
        }
    }
}

/// A placed plant: kind + variant (render-side scale/rotation jitter).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plant {
    pub kind: PlantKind,
    pub variant: u8,
}

fn fnv_mix(seed: u64, words: [u64; 3]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in seed.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    for word in words {
        for b in word.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    h
}

fn unit(seed: u64, words: [u64; 3]) -> f32 {
    ((fnv_mix(seed, words) >> 11) as f32) / ((1u64 << 53) as f32)
}

/// Density table per biome: (kind, p per 4 m slot). Densities are the
/// gameplay-facing wilderness character — Forest thick with trees,
/// Plains open grassland, heights rocky — and every number here is what
/// the placement tests assert against.
///
/// The expansion families ride the same tables: forest floors get the
/// undergrowth layer (ferns, blooms, fungi, glowcaps, deadfall, roots),
/// wetlands get reeds and bog snags, heights get the ruin-and-resonance
/// layer (columns, cairns, arches, crystals, obsidian), snowpeaks ice.
fn biome_table(b: Biome) -> &'static [(PlantKind, f32)] {
    match b {
        Biome::Ocean => &[],
        Biome::Coast => &[
            (PlantKind::Grass, 0.10),
            (PlantKind::Shrub, 0.02),
            (PlantKind::Log, 0.02),
            (PlantKind::RockBoulder, 0.01),
            (PlantKind::Reed, 0.012),
            (PlantKind::Pebble, 0.02),
            (PlantKind::PuddleStone, 0.008),
            (PlantKind::Root, 0.004),
        ],
        Biome::Plains => &[
            (PlantKind::Grass, 0.30),
            (PlantKind::Shrub, 0.05),
            (PlantKind::TreeBirch, 0.02),
            (PlantKind::RockBoulder, 0.01),
            (PlantKind::Flower, 0.02),
            (PlantKind::Pebble, 0.012),
            (PlantKind::PuddleStone, 0.004),
            (PlantKind::Thornbush, 0.004),
        ],
        Biome::Forest => &[
            (PlantKind::TreeBroadleaf, 0.10),
            (PlantKind::TreePine, 0.06),
            (PlantKind::TreeBirch, 0.05),
            (PlantKind::Shrub, 0.07),
            (PlantKind::Grass, 0.16),
            (PlantKind::Log, 0.012),
            (PlantKind::RockBoulder, 0.008),
            (PlantKind::Fern, 0.04),
            (PlantKind::Flower, 0.025),
            (PlantKind::Mushroom, 0.02),
            (PlantKind::Glowcap, 0.006),
            (PlantKind::DeadTree, 0.010),
            (PlantKind::Snag, 0.006),
            (PlantKind::Stump, 0.008),
            (PlantKind::Root, 0.010),
            (PlantKind::MossRock, 0.010),
            (PlantKind::Bramble, 0.006),
            (PlantKind::Thornbush, 0.004),
        ],
        Biome::Wetland => &[
            (PlantKind::Grass, 0.12),
            (PlantKind::Shrub, 0.04),
            (PlantKind::Log, 0.02),
            (PlantKind::TreeBirch, 0.02),
            (PlantKind::Reed, 0.08),
            (PlantKind::Fern, 0.02),
            (PlantKind::Mushroom, 0.008),
            (PlantKind::Snag, 0.008),
            (PlantKind::MossRock, 0.008),
            (PlantKind::PuddleStone, 0.006),
        ],
        Biome::Highlands => &[
            (PlantKind::TreePine, 0.06),
            (PlantKind::Grass, 0.10),
            (PlantKind::RockBoulder, 0.05),
            (PlantKind::RockSpire, 0.02),
            (PlantKind::Shrub, 0.03),
            (PlantKind::Fern, 0.012),
            (PlantKind::MossRock, 0.015),
            (PlantKind::Cairn, 0.004),
            (PlantKind::RockColumn, 0.003),
            (PlantKind::Pebble, 0.01),
            (PlantKind::Bramble, 0.006),
            (PlantKind::Thornbush, 0.004),
            (PlantKind::ArchRock, 0.002),
        ],
        Biome::Mountains => &[
            (PlantKind::RockBoulder, 0.07),
            (PlantKind::RockSpire, 0.03),
            (PlantKind::RockSlab, 0.02),
            (PlantKind::TreePine, 0.012),
            (PlantKind::RockColumn, 0.005),
            (PlantKind::Cairn, 0.004),
            (PlantKind::ArchRock, 0.004),
            (PlantKind::Crystal, 0.003),
            (PlantKind::Obsidian, 0.002),
            (PlantKind::MossRock, 0.006),
            (PlantKind::Pebble, 0.008),
        ],
        Biome::SnowPeaks => &[
            (PlantKind::RockBoulder, 0.03),
            (PlantKind::RockSpire, 0.012),
            (PlantKind::IceShard, 0.012),
            (PlantKind::Crystal, 0.002),
            (PlantKind::Cairn, 0.002),
            (PlantKind::Pebble, 0.004),
            (PlantKind::ArchRock, 0.0015),
        ],
    }
}

/// The slot key: 4 m cells, world meters divided by SLOT_M.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SlotCoord {
    pub x: i32,
    pub z: i32,
}

impl SlotCoord {
    pub fn from_world_mm(wx: i64, wz: i64) -> Self {
        Self {
            x: (wx.div_euclid(SLOT_M * 1000)) as i32,
            z: (wz.div_euclid(SLOT_M * 1000)) as i32,
        }
    }

    /// Slot center in world meters (the plant pivot).
    pub fn center_m(self) -> [f32; 2] {
        [
            self.x as f32 * SLOT_M as f32 + SLOT_M as f32 * 0.5,
            self.z as f32 * SLOT_M as f32 + SLOT_M as f32 * 0.5,
        ]
    }
}

fn region_of(gen: &WorldGen, slot: SlotCoord) -> RegionCoord {
    // Region 256 m; the slot's macro field is sampled at region
    // resolution like every other biome consumer.
    let wx = slot.x as i64 * SLOT_M * 1000;
    let wz = slot.z as i64 * SLOT_M * 1000;
    RegionCoord {
        x: wx.div_euclid(crate::scales::REGION_MM) as i32,
        z: wz.div_euclid(crate::scales::REGION_MM) as i32,
    }
}

/// Ground slope along +X at a slot (m per 4 m) — the cheap steepness
/// gate every planter shares.
fn slope_x(gen: &WorldGen, slot: SlotCoord) -> f32 {
    let [cx, cz] = slot.center_m();
    let a = gen.effective_surface_mm((cx * 1000.0) as i64, (cz * 1000.0) as i64) as f32;
    let b = gen.effective_surface_mm(((cx + 4.0) * 1000.0) as i64, (cz * 1000.0) as i64) as f32;
    (b - a) / 1000.0 / SLOT_M as f32
}

/// The plant at a 4 m slot, if any. Deterministic in (seed, slot):
/// hash gates by the biome density table; trees additionally reserve
/// the even-diagonal subgrid (8 m minimum spacing); trees and the
/// landmark refuse steep ground (rocks do not).
pub fn plant_at(gen: &WorldGen, slot: SlotCoord) -> Option<Plant> {
    let biome = gen.biome(region_of(gen, slot));
    let table = biome_table(biome);
    if table.is_empty() {
        return None; // Ocean: nothing grows or sits
    }
    let r = unit(gen.hash_seed(), [slot.x as u64, slot.z as u64, 0x77]);
    let mut acc = 0.0f32;
    for (kind, p) in table {
        acc += p;
        if r < acc {
            if kind.is_tree() {
                // Even-diagonal subgrid: tree-sized trunks (living or
                // dead) at least 8 m apart.
                if (slot.x + slot.z).rem_euclid(2) != 0 {
                    return None;
                }
                // Trees refuse steep slopes (rocks hold them).
                if slope_x(gen, slot).abs() > 0.9 {
                    return None;
                }
            }
            let v = unit(gen.hash_seed(), [slot.x as u64, slot.z as u64, 0x78]);
            return Some(Plant {
                kind: *kind,
                variant: (v * 4.0) as u8,
            });
        }
    }
    None
}

/// Deterministic render-side jitter for a slot: (dx, dz within +-0.8 m,
/// scale 0.8..1.25). The renderer derives placement variation from THIS
/// so the world and the picture can never disagree.
pub fn jitter(gen: &WorldGen, slot: SlotCoord) -> [f32; 3] {
    let a = unit(gen.hash_seed(), [slot.x as u64, slot.z as u64, 0x79]);
    let b = unit(gen.hash_seed(), [slot.x as u64, slot.z as u64, 0x7A]);
    let c = unit(gen.hash_seed(), [slot.x as u64, slot.z as u64, 0x7B]);
    [(a - 0.5) * 1.6, (b - 0.5) * 1.6, 0.8 + c * 0.45]
}

/// The region's LANDMARK (standing stone) position in world meters, if
/// the region earns one: a hash gate (~18%), land biome, gentle ground.
/// One per region at most — the wilderness' fixed point of reference.
pub fn landmark_at(gen: &WorldGen, region: RegionCoord) -> Option<[f32; 2]> {
    let biome = gen.biome(region);
    if !matches!(biome, Biome::Plains | Biome::Forest | Biome::Highlands) {
        return None;
    }
    let r = unit(gen.hash_seed(), [region.x as u64, region.z as u64, 0xAA11]);
    if r >= 0.18 {
        return None;
    }
    // Deterministic search for a gentle spot near the region center.
    let base = region.origin();
    let cx = (base.x + crate::scales::REGION_MM / 2) as f32 / 1000.0;
    let cz = (base.z + crate::scales::REGION_MM / 2) as f32 / 1000.0;
    for k in 0..24i32 {
        let a = unit(
            gen.hash_seed(),
            [region.x as u64, region.z as u64, 0xAA12 + k as u64],
        );
        let b = unit(
            gen.hash_seed(),
            [region.x as u64, region.z as u64, 0xAA40 + k as u64],
        );
        let x = cx + (a - 0.5) * 160.0;
        let z = cz + (b - 0.5) * 160.0;
        let h0 = gen.effective_surface_mm((x * 1000.0) as i64, (z * 1000.0) as i64);
        let hx = gen.effective_surface_mm(((x + 6.0) * 1000.0) as i64, (z * 1000.0) as i64);
        let hz = gen.effective_surface_mm((x * 1000.0) as i64, ((z + 6.0) * 1000.0) as i64);
        let slope = ((hx - h0).abs().max((hz - h0).abs()) as f32) / 6000.0;
        if slope < 0.35 {
            return Some([x, z]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::WorldGen;

    fn scan(gen: &WorldGen, x0: i32, z0: i32, x1: i32, z1: i32) -> Vec<(SlotCoord, Plant)> {
        let mut out = Vec::new();
        for x in x0..x1 {
            for z in z0..z1 {
                if let Some(p) = plant_at(gen, SlotCoord { x, z }) {
                    out.push((SlotCoord { x, z }, p));
                }
            }
        }
        out
    }

    #[test]
    fn placement_is_deterministic_and_seed_varied() {
        let a = WorldGen::new(4242);
        let b = WorldGen::new(4242);
        let c = WorldGen::new(7);
        let s = |g: &WorldGen| scan(g, -40, -40, 40, 40);
        assert_eq!(s(&a), s(&b), "same seed -> identical wilderness");
        assert_ne!(s(&a), s(&c), "different seed -> different wilderness");
    }

    #[test]
    fn trees_keep_their_spacing_subgrid() {
        let g = WorldGen::new(4242);
        for (slot, p) in scan(&g, -60, -60, 60, 60) {
            if p.kind.is_tree() {
                assert_eq!(
                    (slot.x + slot.z).rem_euclid(2),
                    0,
                    "tree-sized trunks (living or dead) only on the even diagonal (>= 8 m apart)"
                );
            }
        }
    }

    #[test]
    fn biome_character_holds_over_a_big_sample() {
        // SEARCH for character biomes (a seed's spawn can be one huge
        // lowland — the first sweep assumed Forest nearby and proved
        // nothing), then sample each found region's own slots and
        // assert the character laws on real counts.
        let g = WorldGen::new(2024);
        let mut found: std::collections::BTreeMap<Biome, RegionCoord> = Default::default();
        'search: for r in 0..96i32 {
            for rx in -r..=r {
                for rz in -r..=r {
                    if rx.abs() != r && rz.abs() != r {
                        continue; // ring perimeter only
                    }
                    let region = RegionCoord { x: rx, z: rz };
                    let b = g.biome(region);
                    found.entry(b).or_insert(region);
                    if found.contains_key(&Biome::Forest)
                        && found.contains_key(&Biome::Plains)
                        && found.contains_key(&Biome::Mountains)
                    {
                        break 'search;
                    }
                }
            }
        }
        let sample = |region: RegionCoord| -> (usize, usize, usize) {
            let mut t = (0, 0, 0);
            for (_, p) in scan(
                &g,
                region.x * 64,
                region.z * 64,
                region.x * 64 + 64,
                region.z * 64 + 64,
            ) {
                match p.kind {
                    PlantKind::TreePine | PlantKind::TreeBroadleaf | PlantKind::TreeBirch => {
                        t.0 += 1
                    }
                    PlantKind::Grass => t.1 += 1,
                    PlantKind::RockBoulder | PlantKind::RockSpire | PlantKind::RockSlab => t.2 += 1,
                    _ => {}
                }
            }
            t
        };
        let forest = sample(*found.get(&Biome::Forest).expect("a Forest region exists"));
        let plains = sample(*found.get(&Biome::Plains).expect("a Plains region exists"));
        println!("biome samples: forest {forest:?} plains {plains:?} found {found:?}");
        assert!(
            forest.0 > plains.0 + 20,
            "Forest is treed, Plains is not ({forest:?} vs {plains:?})"
        );
        assert!(plains.1 > plains.0 * 10, "Plains is grassland");
        if let Some(mr) = found.get(&Biome::Mountains) {
            let m = sample(*mr);
            assert!(m.2 > 0, "Mountains rocky ({m:?})");
            assert!(m.1 == 0, "no grass in the Mountains ({m:?})");
        }
        if let Some(or) = found.get(&Biome::Ocean) {
            assert_eq!(sample(*or), (0, 0, 0), "nothing grows in the Ocean");
        }
    }

    #[test]
    fn collision_contract_is_what_it_looks_like() {
        assert!(PlantKind::TreePine.blocks_movement());
        assert!(PlantKind::RockBoulder.blocks_movement());
        assert!(PlantKind::Log.blocks_movement());
        assert!(!PlantKind::Grass.blocks_movement());
        assert!(!PlantKind::Shrub.blocks_movement());
        // The expansion classes: upright masses block, ground flora
        // doesn't, and the arch stays passable through its opening.
        assert!(PlantKind::DeadTree.blocks_movement());
        assert!(PlantKind::RockColumn.blocks_movement());
        assert!(PlantKind::Crystal.blocks_movement());
        assert!(PlantKind::IceShard.blocks_movement());
        assert!(!PlantKind::Fern.blocks_movement());
        assert!(!PlantKind::Reed.blocks_movement());
        assert!(!PlantKind::Stump.blocks_movement());
        assert!(!PlantKind::Pebble.blocks_movement());
        assert!(!PlantKind::ArchRock.blocks_movement());
        // Wind: grass most, rocks none.
        assert!(PlantKind::Grass.wind() > PlantKind::Shrub.wind());
        assert_eq!(PlantKind::RockBoulder.wind(), 0.0);
        assert_eq!(PlantKind::RockColumn.wind(), 0.0);
        assert!(PlantKind::Reed.wind() > PlantKind::Fern.wind());
    }

    /// THE EXPANSION CENSUS LAW: every family named in a biome table is
    /// LIVE — over a whole sampled region of its biome it actually
    /// grows. A table entry that never fires is a dead catalog row (the
    /// "unconsumed asset" failure mode this wiring exists to close).
    #[test]
    fn expansion_families_grow_in_their_biomes() {
        let g = WorldGen::new(2024);
        // Search outward for one region of every land biome (the same
        // ring scan the gen.rs character law uses).
        let mut by: std::collections::BTreeMap<Biome, RegionCoord> = Default::default();
        'find: for ring in 0..200i32 {
            for dx in -ring..=ring {
                for dz in -ring..=ring {
                    if dx.abs() != ring && dz.abs() != ring {
                        continue;
                    }
                    let reg = RegionCoord { x: dx, z: dz };
                    by.entry(g.biome(reg)).or_insert(reg);
                    if by.len() >= 7 {
                        break 'find;
                    }
                }
            }
        }
        assert!(by.contains_key(&Biome::Forest), "a Forest region exists");
        assert!(by.contains_key(&Biome::Plains), "a Plains region exists");
        // Sample each found biome's region fully; count kinds.
        let mut seen: std::collections::BTreeMap<PlantKind, usize> = Default::default();
        let mut sampled: Vec<Biome> = Vec::new();
        for (b, reg) in &by {
            if matches!(b, Biome::Ocean) {
                continue;
            }
            sampled.push(*b);
            for dx in 0..64i32 {
                for dz in 0..64i32 {
                    let slot = SlotCoord {
                        x: reg.x * 64 + dx,
                        z: reg.z * 64 + dz,
                    };
                    if let Some(p) = plant_at(&g, slot) {
                        *seen.entry(p.kind).or_default() += 1;
                    }
                }
            }
        }
        println!("census over {sampled:?}: {seen:?}");
        // Every entry of every SAMPLED biome's table fired at least once
        // in the census — a table row that never fires is a dead catalog
        // entry (the "unconsumed asset" failure mode this wiring closes).
        for b in &sampled {
            for (kind, _) in biome_table(*b) {
                assert!(
                    seen.get(kind).is_some_and(|n| *n > 0),
                    "{kind:?} is named in {b:?}'s table but never grew in the census"
                );
            }
        }
        // Character accents, the world-logic point of the expansion:
        // reeds belong to wet ground, ice to the peaks, resonance
        // (crystal) to the heights, undergrowth to the forest.
        if sampled.contains(&Biome::Wetland) {
            assert!(seen.contains_key(&PlantKind::Reed), "wetlands grow reeds");
        }
        if sampled.contains(&Biome::SnowPeaks) {
            assert!(
                seen.contains_key(&PlantKind::IceShard),
                "snowpeaks grow ice shards"
            );
        }
        if sampled.contains(&Biome::Mountains) {
            assert!(
                seen.contains_key(&PlantKind::Crystal),
                "mountains grow crystals"
            );
        }
        for k in [
            PlantKind::Fern,
            PlantKind::Flower,
            PlantKind::Mushroom,
            PlantKind::DeadTree,
        ] {
            assert!(seen.contains_key(&k), "forest undergrowth lives ({k:?})");
        }
    }

    #[test]
    fn landmarks_are_region_deterministic_and_rare() {
        let g = WorldGen::new(999);
        for r in [
            RegionCoord { x: -3, z: -3 },
            RegionCoord { x: 0, z: 0 },
            RegionCoord { x: 2, z: -1 },
            RegionCoord { x: 5, z: 4 },
        ] {
            assert_eq!(landmark_at(&g, r), landmark_at(&g, r), "deterministic");
        }
        let mut with = 0;
        let mut land = 0;
        for rx in -8..8i32 {
            for rz in -8..8i32 {
                land += 1;
                if landmark_at(&g, RegionCoord { x: rx, z: rz }).is_some() {
                    with += 1;
                }
            }
        }
        println!("landmarks: {with}/{land} regions");
        assert!(with > 0, "some regions earn a landmark");
        assert!(with < land / 3, "landmarks stay rare (< 1/3 of regions)");
    }
}
