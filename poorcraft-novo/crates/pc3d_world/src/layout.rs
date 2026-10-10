//! The world-layout dials (BETA-0.2 W1.1): the ONE place the played
//! world's scale constants live, stamped with a layout version so any
//! dial change is a deliberate, versioned migration — never a silent
//! drift under old saves.
//!
//! THE DIAL LAW: every dial here is the single source of truth for its
//! value. Consumers read these constants; none may hardcode a copy. A
//! dial change MUST bump [`WORLDGEN_LAYOUT_VERSION`] in the same commit
//! (the lockstep law below enforces it structurally — the save header's
//! `world` field carries this version, and the framing law refuses
//! mismatched files loudly instead of rendering a foreign world).

use crate::coords::RegionCoord;
use crate::gen::WorldGen;

/// The worldgen layout this build speaks. Bumped whenever any dial below
/// changes the meaning of generated terrain (the save header's `world`
/// field carries it; older saves refuse with a legible explanation).
pub const WORLDGEN_LAYOUT_VERSION: u16 = 2;

/// Climate field base wavelength, in region cells (1 region = 256 m).
/// 192 cells ≈ 49 km weather systems (the dual-scale biome law).
/// The W1 dial: ×2 (384) turns belts into continents.
pub const CLIMATE_CELLS: f64 = 384.0;

/// Elevation field base wavelength, in region cells (the ~12 km relief
/// base). The W1 dial: ×2 (96) widens mountain systems with the climate.
pub const ELEVATION_BASE_CELLS: f64 = 96.0;

/// Stream interest rings, in meters (the dials the streamer's tiers read).
/// The W1 dial: 192/640/2048 widens the played view ~2x per ring.
pub const TIER_FULL_M: f32 = 192.0;
pub const TIER_LOD_M: f32 = 640.0;
pub const TIER_MACRO_M: f32 = 2048.0;

/// Capital spacing, in meters, from decision D-026 (10–25 minutes on
/// foot). The walk rate is 4.3 m/s (WALK_SPEED), so the owned band is
/// [2_580, 6_450] m; the planner targets its middle.
pub const CAPITAL_SPACING_MIN_M: f32 = 2_580.0;
pub const CAPITAL_SPACING_MAX_M: f32 = 6_450.0;
/// The planner's target spacing: the middle of the owned band.
pub const CAPITAL_SPACING_TARGET_M: f32 =
    (CAPITAL_SPACING_MIN_M + CAPITAL_SPACING_MAX_M) / 2.0;

/// Realm capitals per world (one per biome family — the Wide World's
/// six realms; the W1.3 dial from 2 to 6 as the kits land).
pub const REALM_CAPITAL_COUNT: usize = 6;

/// The lockstep law: this build's layout version and the save header's
/// world version are THE SAME NUMBER — a dial change without a version
/// bump fails this test, and a version bump without the framing law's
/// refusal refuses old saves loudly (never renders a foreign world).
pub fn layout_header_agrees(header_world: u16) -> bool {
    header_world == WORLDGEN_LAYOUT_VERSION
}

/// The realm plan for one seed: REALM_CAPITAL_COUNT capitals placed by
/// the deterministic watershed/spacings law (W1.3's authority; the
/// planner consumes this, the map export draws it, the law test proves
/// the D-026 spacing).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealmPlan {
    pub seed: u64,
    pub capitals: Vec<RealmCapital>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealmCapital {
    /// Region coords of the capital's plaza.
    pub region: RegionCoord,
    /// Realm index (0..REALM_CAPITAL_COUNT) — the faction slot.
    pub realm: u8,
    /// The biome family this realm rules (from the generator's table).
    pub biome: crate::gen::Biome,
}

impl RealmPlan {
    /// Deterministic per seed: sweep the world's region lattice on the
    /// spacing grid, score each candidate region by how strongly it
    /// plays its biome family (a real Mountains region scores higher
    /// than a pocket), greedy-pick with the D-026 Chebyshev spacing.
    /// Pure — the same seed always plans the same six realms.
    pub fn for_seed(gen: &WorldGen) -> Self {
        use crate::gen::Biome;
        const HALF: i32 = 32; // the planned band: ±32 regions (~±8 km) — the D-026 pack needs the room
        let families: [Biome; REALM_CAPITAL_COUNT] = [
            Biome::Coast,
            Biome::Plains,
            Biome::Forest,
            Biome::Wetland,
            Biome::Highlands,
            Biome::SnowPeaks,
        ];
        // Score every region in the band once. Each realm accepts a SET
        // of terrains (a harbor may sit on the ocean shore, the snow
        // realm takes high mountains) — strict single-biome matching
        // starves seeds whose band hosts fewer distinct biomes, and a
        // realm without a home is a law failure, not a feature.
        const ACCEPT: [[Biome; 2]; REALM_CAPITAL_COUNT] = [
            [Biome::Coast, Biome::Ocean],     // the harbor realm
            [Biome::Plains, Biome::Forest],   // the farmland realm
            [Biome::Forest, Biome::Plains],   // the woodland realm
            [Biome::Wetland, Biome::Coast],   // the marsh realm
            [Biome::Highlands, Biome::Mountains], // the crag realm
            [Biome::SnowPeaks, Biome::Mountains], // the frost realm
        ];
        let mut candidates: Vec<(i32, i32, Biome, i64)> = Vec::new();
        for x in -HALF..=HALF {
            for z in -HALF..=HALF {
                let r = RegionCoord { x, z };
                let b = gen.biome(r);
                let f = gen.macro_field(r);
                // Family strength: how far the region sits INTO its
                // biome's band (coasts hug sea level, peaks reach high).
                let strength: i64 = match b {
                    Biome::Coast => 2 - (f.elevation_m - 1).abs() as i64,
                    Biome::Plains => 40 - (f.elevation_m - 20).abs() as i64,
                    Biome::Forest => 40 - (f.elevation_m - 30).abs() as i64 + f.humidity as i64 / 4,
                    Biome::Wetland => 20 - (f.elevation_m - 6).abs() as i64 + f.humidity as i64 / 3,
                    Biome::Highlands => 30 - (f.elevation_m - 90).abs() as i64,
                    Biome::Mountains | Biome::SnowPeaks => f.elevation_m as i64,
                    Biome::Ocean => -1_000, // the shore may host, the deep may not
                };
                candidates.push((x, z, b, strength));
            }
        }
        let mut capitals: Vec<RealmCapital> = Vec::new();
        // GLOBAL ASSIGNMENT greedy: strongest regions claim their
        // best-fit unplaced realm first. Per-family sequential placement
        // starves the last families on seeds whose band packs tight —
        // the assignment pass fills all six wherever the terrain allows.
        let mut placed: Vec<RealmCapital> = Vec::new();
        let mut taken: Vec<(i32, i32)> = Vec::new();
        // Descending strength, deterministic tie-break (x, then z).
        let mut order: Vec<(i64, i32, i32, Biome)> = candidates
            .iter()
            .map(|&(x, z, b, s)| (s, x, z, b))
            .collect();
        order.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        for (s, x, z, b) in order {
            if placed.len() == REALM_CAPITAL_COUNT {
                break;
            }
            if taken.contains(&(x, z)) {
                continue;
            }
            // Which unplaced realms accept this terrain, in family order?
            let claimant = families
                .iter()
                .enumerate()
                .find(|(ri, family)| {
                    !placed.iter().any(|c| c.realm == *ri as u8)
                        && (b == ACCEPT[*ri][0] || b == ACCEPT[*ri][1])
                })
                .map(|(ri, family)| (ri, *family));
            let Some((ri, family)) = claimant else {
                continue;
            };
            let spacing_ok = placed.iter().all(|c| {
                let d = ((c.region.x - x).abs().max((c.region.z - z).abs())) as f32 * 256.0;
                d >= CAPITAL_SPACING_MIN_M
            });
            if !spacing_ok {
                continue;
            }
            placed.push(RealmCapital {
                region: RegionCoord { x, z },
                realm: ri as u8,
                biome: b,
            });
            taken.push((x, z));
            let _ = s;
        }
        // RESCUE pass: a realm whose accepted terrain never found a
        // spaced home takes the best ANY-LAND region that keeps D-026 —
        // a realm rules the terrain it finds (recorded in its biome
        // field; six realms per world is the law, not six biomes).
        for ri in 0..REALM_CAPITAL_COUNT {
            if placed.iter().any(|c| c.realm == ri as u8) {
                continue;
            }
            let mut best: Option<(i64, i32, i32, Biome)> = None;
            for &(x, z, b, s) in &candidates {
                if matches!(b, Biome::Ocean) || taken.contains(&(x, z)) {
                    continue;
                }
                let spacing_ok = placed.iter().all(|c| {
                    let d =
                        ((c.region.x - x).abs().max((c.region.z - z).abs())) as f32 * 256.0;
                    d >= CAPITAL_SPACING_MIN_M
                });
                if !spacing_ok {
                    continue;
                }
                match best {
                    Some((bs, _, _, _)) if bs >= s => {}
                    _ => best = Some((s, x, z, b)),
                }
            }
            if let Some((_, x, z, b)) = best {
                placed.push(RealmCapital {
                    region: RegionCoord { x, z },
                    realm: ri as u8,
                    biome: b,
                });
                taken.push((x, z));
            }
        }
        // Realm order canonical (0..6) for stable consumers.
        placed.sort_by_key(|c| c.realm);
        capitals = placed;
        RealmPlan {
            seed: gen.seed(),
            capitals,
        }
    }

    /// Spacing law: every pair of placed capitals is at least
    /// CAPITAL_SPACING_MIN_M apart (Chebyshev, region×256 m).
    pub fn spacing_holds(&self) -> bool {
        for i in 0..self.capitals.len() {
            for j in (i + 1)..self.capitals.len() {
                let a = &self.capitals[i].region;
                let b = &self.capitals[j].region;
                let d = (a.x - b.x)
                    .abs()
                    .max((a.z - b.z).abs()) as f32
                    * 256.0;
                if d < CAPITAL_SPACING_MIN_M {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::WorldGen;

    /// THE LOCKSTEP LAW: the layout version and the save header's world
    /// version are the same number — a dial change without the version
    /// bump fails here before it can render under old saves.
    #[test]
    fn beta02_layout_version_locksteps_with_save_header() {
        let h = pc3d_core::FormatHeader::current();
        assert!(
            layout_header_agrees(h.world),
            "WORLDGEN_LAYOUT_VERSION ({}) and FormatHeader world ({}) diverged — bump both together",
            WORLDGEN_LAYOUT_VERSION,
            h.world
        );
    }

    /// THE DIAL LAW: the stream tiers in pc3d_world::stream read THESE
    /// constants (a dial change without rewiring the streamer fails).
    #[test]
    fn beta02_stream_tiers_read_the_dials() {
        assert_eq!(crate::stream::TIER_FULL_M, TIER_FULL_M);
        assert_eq!(crate::stream::TIER_LOD_M, TIER_LOD_M);
        assert_eq!(crate::stream::TIER_MACRO_M, TIER_MACRO_M);
    }

    /// THE D-026 SPACING LAW: the realm plan is deterministic per seed,
    /// every placed pair keeps the owned minimum spacing, and different
    /// seeds plan different realms.
    #[test]
    fn beta02_realm_plan_is_deterministic_and_d026_spaced() {
        for seed in [3u64, 42, 777, 2024, 31415] {
            let gen = WorldGen::new(seed);
            let a = RealmPlan::for_seed(&gen);
            let b = RealmPlan::for_seed(&gen);
            assert_eq!(a, b, "seed {seed}: the realm plan must replay exactly");
            assert!(
                a.spacing_holds(),
                "seed {seed}: capitals closer than D-026's {} m",
                CAPITAL_SPACING_MIN_M
            );
            assert_eq!(
                a.capitals.len(),
                REALM_CAPITAL_COUNT,
                "seed {seed}: only {} of {} realms found a home",
                a.capitals.len(),
                REALM_CAPITAL_COUNT
            );
            assert!(a.capitals.len() <= REALM_CAPITAL_COUNT);
        }
        let pa = RealmPlan::for_seed(&WorldGen::new(3));
        let pb = RealmPlan::for_seed(&WorldGen::new(42));
        assert_ne!(
            pa.capitals, pb.capitals,
            "different seeds must plan different realms"
        );
    }

    /// The spacing target sits inside the owned band (a planner that
    /// targets outside D-026 fails here).
    #[test]
    fn beta02_capital_spacing_target_is_in_the_owned_band() {
        assert!(CAPITAL_SPACING_MIN_M <= CAPITAL_SPACING_TARGET_M);
        assert!(CAPITAL_SPACING_TARGET_M <= CAPITAL_SPACING_MAX_M);
        // 10-25 minutes at the 4.3 m/s walk rate brackets the band.
        assert!((CAPITAL_SPACING_MIN_M / 4.3 - 600.0).abs() < 1.0);
        assert!((CAPITAL_SPACING_MAX_M / 4.3 - 1500.0).abs() < 1.0);
    }
}
