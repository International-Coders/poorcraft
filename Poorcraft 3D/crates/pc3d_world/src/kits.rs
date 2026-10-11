//! P3D-705: additional faction/castle kits.
//!
//! The first capital behaves deeply (P3D-602 planner, law, oversight,
//! garrison); now each ideology gets its own building vocabulary: two
//! signature modules, a kit manifest, a preferred law, and a
//! kit-aware capital planner that appends the signatures to the core
//! plan on real terrain — same seed + center + ideology → same layout,
//! and different ideologies grow visibly different capitals.

use crate::castle::{plan_capital, ModuleKind, PlacedModule};
use crate::castle_law::LawKind;
use crate::coords::{CellCoord, RegionCoord};
use crate::gen::WorldGen;
use crate::ideology::Ideology;

/// One ideology's faction kit: signature architecture and civic bias.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FactionKit {
    pub ideology: Ideology,
    /// The law this culture enforces first.
    pub preferred_law: LawKind,
    /// Signature castle modules, in placement order.
    pub modules: &'static [ModuleKind],
}

/// The kit table: every ideology has exactly one kit.
pub const KITS: &[FactionKit] = &[
    FactionKit {
        ideology: Ideology::Conquest,
        preferred_law: LawKind::Assault,
        modules: &[ModuleKind::Arsenal, ModuleKind::TrainingYard],
    },
    FactionKit {
        ideology: Ideology::Commerce,
        preferred_law: LawKind::Theft,
        modules: &[ModuleKind::Warehouse, ModuleKind::Mint],
    },
    FactionKit {
        ideology: Ideology::Faith,
        preferred_law: LawKind::Theft,
        modules: &[ModuleKind::Cathedral, ModuleKind::Reliquary],
    },
    FactionKit {
        ideology: Ideology::Liberty,
        preferred_law: LawKind::Trespass,
        modules: &[ModuleKind::Forum, ModuleKind::Watchpost],
    },
    FactionKit {
        ideology: Ideology::Isolation,
        preferred_law: LawKind::Trespass,
        modules: &[ModuleKind::Vault, ModuleKind::Lookout],
    },
];

/// The kit for an ideology.
pub fn kit_for(ideology: Ideology) -> &'static FactionKit {
    KITS.iter()
        .find(|k| k.ideology == ideology)
        .expect("every ideology has a kit")
}

/// Candidate origins around a center, deterministic order: expanding
/// Chebyshev rings, then (dx, dz) ascending. Ring 0 excluded (core).
fn candidate_offsets(radius: i32) -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    for r in 1..=radius {
        for dx in -r..=r {
            for dz in -r..=r {
                if dx.abs().max(dz.abs()) == r {
                    v.push((dx, dz));
                }
            }
        }
    }
    v
}

/// Place a module footprint at (ox, oz) if every cell is unoccupied.
fn place_if_free(
    occupied: &mut std::collections::BTreeSet<(i32, i32)>,
    kind: ModuleKind,
    ox: i32,
    oz: i32,
    y: i32,
) -> Option<PlacedModule> {
    let (fw, fh) = kind.footprint();
    for dx in 0..fw as i32 {
        for dz in 0..fh as i32 {
            if occupied.contains(&(ox + dx, oz + dz)) {
                return None;
            }
        }
    }
    for dx in 0..fw as i32 {
        for dz in 0..fh as i32 {
            occupied.insert((ox + dx, oz + dz));
        }
    }
    Some(PlacedModule {
        kind,
        origin: CellCoord { x: ox, y, z: oz },
    })
}

/// The kit-aware capital planner: the deep core plan from P3D-602 plus
/// each signature module sited on the first free, terrain-valid cells
/// in deterministic ring order. Returns the merged layout.
pub fn plan_capital_kit(
    gen: &WorldGen,
    center: RegionCoord,
    ideology: Ideology,
) -> crate::castle::CastleLayout {
    let kit = kit_for(ideology);
    let mut layout = plan_capital(gen, center);
    let cy = layout.modules.first().map(|m| m.origin.y).unwrap_or(0);

    // Occupied cells from the core plan.
    let mut occupied: std::collections::BTreeSet<(i32, i32)> = std::collections::BTreeSet::new();
    for m in &layout.modules {
        let (fw, fh) = m.kind.footprint();
        for dx in 0..fw as i32 {
            for dz in 0..fh as i32 {
                occupied.insert((m.origin.x + dx, m.origin.z + dz));
            }
        }
    }

    let radius = 14i32;
    for kind in kit.modules {
        'sites: for (dx, dz) in candidate_offsets(radius) {
            let ox = center.origin().x.div_euclid(1000) as i32 + dx;
            let oz = center.origin().z.div_euclid(1000) as i32 + dz;
            let probe = CellCoord {
                x: ox,
                y: cy,
                z: oz,
            };
            if !crate::castle::footprint_fits(gen, probe, kind.footprint().0, kind.footprint().1) {
                continue;
            }
            if let Some(placed) = place_if_free(&mut occupied, *kind, ox, oz, cy) {
                layout.modules.push(placed);
                break 'sites;
            }
        }
    }
    layout
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen() -> WorldGen {
        WorldGen::new(70707)
    }

    /// Every ideology has exactly one kit with two DISTINCT signature
    /// modules; the five kits are pairwise distinct; each maps to a
    /// real law; and kit modules have sane names/footprints.
    #[test]
    fn p3d705_kit_table_is_complete_and_distinct() {
        assert_eq!(KITS.len(), 5, "one kit per ideology");
        let mut seen_kits = std::collections::BTreeSet::new();
        let mut seen_modules = std::collections::BTreeSet::new();
        for k in KITS {
            assert_eq!(k.modules.len(), 2, "two signatures per kit");
            assert_ne!(k.modules[0], k.modules[1], "signatures distinct");
            for m in k.modules {
                assert!(!m.name().is_empty());
                let (fw, fh) = m.footprint();
                assert!(fw > 0 && fh > 0);
                assert!(
                    seen_modules.insert(*m),
                    "a module serves one kit only: {m:?}"
                );
            }
            assert!(seen_kits.insert(k.ideology), "one kit per ideology");
        }
        // Pairwise-distinct kits as module sets.
        let sets: Vec<std::collections::BTreeSet<ModuleKind>> = KITS
            .iter()
            .map(|k| k.modules.iter().copied().collect())
            .collect();
        for i in 0..sets.len() {
            for j in i + 1..sets.len() {
                assert_ne!(sets[i], sets[j], "kits must differ architecturally");
            }
        }
    }

    /// The kit-aware planner is deterministic, places BOTH signature
    /// modules on terrain-valid non-overlapping sites, and keeps the
    /// core plan byte-identical to the untouched P3D-602 planner.
    #[test]
    fn p3d705_kit_capital_plans_deterministically() {
        let g = gen();
        let center = RegionCoord { x: 0, z: 0 };
        let core = plan_capital(&g, center);

        for ideology in [
            Ideology::Conquest,
            Ideology::Commerce,
            Ideology::Faith,
            Ideology::Liberty,
            Ideology::Isolation,
        ] {
            let kit = kit_for(ideology);
            let a = plan_capital_kit(&g, center, ideology);
            let b = plan_capital_kit(&g, center, ideology);
            assert_eq!(a, b, "{ideology:?}: deterministic layout");

            // Core plan is a prefix, unchanged.
            assert_eq!(a.modules.len(), core.modules.len() + 2, "{ideology:?}");
            assert_eq!(&a.modules[..core.modules.len()], &core.modules[..]);

            // Both signatures placed exactly once.
            for want in kit.modules {
                let hits = a.modules.iter().filter(|m| m.kind == *want).count();
                assert_eq!(hits, 1, "{ideology:?}: {want:?} placed once");
            }

            // No overlaps anywhere; y pinned to the core floor.
            let mut occupied = std::collections::BTreeSet::new();
            for m in &a.modules {
                let (fw, fh) = m.kind.footprint();
                for dx in 0..fw as i32 {
                    for dz in 0..fh as i32 {
                        assert!(
                            occupied.insert((m.origin.x + dx, m.origin.z + dz)),
                            "{ideology:?}: overlap at {:?} +({dx},{dz})",
                            m.kind
                        );
                    }
                }
                assert_eq!(m.origin.y, cy_of(&core), "kit module at core floor");
            }
        }
    }

    fn cy_of(core: &crate::castle::CastleLayout) -> i32 {
        core.modules.first().map(|m| m.origin.y).unwrap_or(0)
    }

    /// Different ideologies grow visibly different capitals: the
    /// Conquest capital has an Arsenal and no Mint; the Commerce one
    /// the reverse; Faith raises a cathedral nobody else has.
    #[test]
    fn p3d705_ideologies_read_differently() {
        let g = gen();
        let center = RegionCoord { x: 0, z: 0 };
        let has = |layout: &crate::castle::CastleLayout, k: ModuleKind| {
            layout.modules.iter().any(|m| m.kind == k)
        };
        let conquest = plan_capital_kit(&g, center, Ideology::Conquest);
        let commerce = plan_capital_kit(&g, center, Ideology::Commerce);
        let faith = plan_capital_kit(&g, center, Ideology::Faith);
        assert!(has(&conquest, ModuleKind::Arsenal) && !has(&conquest, ModuleKind::Mint));
        assert!(has(&commerce, ModuleKind::Mint) && !has(&commerce, ModuleKind::Arsenal));
        assert!(has(&faith, ModuleKind::Cathedral));
        assert!(!has(&conquest, ModuleKind::Cathedral));

        // Law preferences: conquest polices assault, commerce theft.
        assert_eq!(kit_for(Ideology::Conquest).preferred_law, LawKind::Assault);
        assert_eq!(kit_for(Ideology::Commerce).preferred_law, LawKind::Theft);
        assert_eq!(kit_for(Ideology::Liberty).preferred_law, LawKind::Trespass);
    }
}
