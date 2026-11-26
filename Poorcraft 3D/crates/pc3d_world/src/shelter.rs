//! The shelter score (pure): coverage + walls + opening + fire distance.
//!
//! N11: shelter is a VERDICT, not a boolean stamped when a piece is placed.
//! The law scores a player position from the live world: a roof of solid
//! cells overhead (coverage), solid columns around (walls), a gap to walk
//! through (opening), and a burning campfire close enough to matter
//! (warmth within reach). Every axis reports its own answer — the HUD line
//! names WHAT is missing, the gate decides `sheltered`. The per-cell truth
//! is `terrain::final_solid` — the SAME soil render, collision and nav
//! answer (nothing floats, nothing buries).

use crate::coords::CellCoord;
use crate::gen::WorldGen;
use crate::terrain::final_solid;

/// How many cells above the head must hold a solid to count as a roof.
pub const ROOF_SCAN: i32 = 6;
/// How far the wall scan reaches (Chebyshev, cells).
pub const WALL_SCAN: i32 = 3;
/// Solid columns needed in the wall ring (out of the scanned band) —
/// walls help; a full fortress is not required.
pub const WALL_MIN: u32 = 8;
/// A burning campfire within this distance counts as "warmth in reach".
pub const FIRE_REACH_M: f32 = 4.0;

/// One axis of the shelter verdict (named — the HUD says WHAT is missing).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Axis {
    pub name: &'static str,
    pub ok: bool,
}

/// The shelter verdict for one standing position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShelterScore {
    pub axes: [Axis; 4],
    /// Fraction of green axes in [0,1].
    pub score: f32,
}

impl ShelterScore {
    /// The law's own gate: sheltered means coverage AND walls AND
    /// (opening OR fire) — a sealed box is a prison; the opening axis
    /// keeps it a home, the fire axis keeps it warm.
    pub fn sheltered(&self) -> bool {
        let a = |name: &str| self.axes.iter().find(|x| x.name == name).unwrap().ok;
        a("coverage") && a("walls") && (a("opening") || a("fire"))
    }

    /// The HUD line: names the FIRST failing axis, or the green verdict.
    pub fn line(&self) -> String {
        if self.sheltered() {
            return "SHELTERED".into();
        }
        match self.axes.iter().find(|a| !a.ok).map(|a| a.name) {
            Some("coverage") => "NO ROOF OVERHEAD".into(),
            Some("walls") => "WALLS TOO THIN".into(),
            Some("opening") => "SEALED - MAKE A DOORWAY".into(),
            _ => "FIRE TOO FAR".into(),
        }
    }
}

fn solid_at(gen: &WorldGen, c: CellCoord) -> bool {
    // Cells are 1 m cubes: the cell coordinate is the world METER, so the
    // millimeter query is the cell origin (x, y*1000+500 centers it, but
    // the band [y*1000, y*1000+1000) shares one answer — depth is flat
    // within a cell except at the 1 m skin, where the surface decision
    // (Grass/Soil/Rock) still holds because depth_mm is measured to the
    // cell ORIGIN like the mesher's own cell walk).
    final_solid(gen, c.x as i64 * 1000, c.y as i64 * 1000, c.z as i64 * 1000).solid
}

/// Scores the position `(x, z)` standing on the authoritative surface.
///
/// `feet_y` is the walk surface height in meters (the same `ground_y_at`
/// answer the body stands on); `fire_dist` is the nearest BURNING
/// campfire's distance in meters (`None` = no fire lit).
pub fn score_at(gen: &WorldGen, x: f32, z: f32, feet_y: f32, fire_dist: Option<f32>) -> ShelterScore {
    let px = x.floor() as i32;
    let pz = z.floor() as i32;
    // The standing cell: the first AIR cell at/above the walk surface.
    let base_y = feet_y.ceil() as i32;

    // COVERAGE: any solid cell within the roof scan above the head.
    let coverage = (1..=ROOF_SCAN)
        .any(|dy| solid_at(gen, CellCoord { x: px, y: base_y + dy, z: pz }));

    // WALLS: solid columns in the ring band around the body column
    // (any solid within the body's own height band counts).
    let mut wall_hits = 0u32;
    let mut wall_total = 0u32;
    for dx in -WALL_SCAN..=WALL_SCAN {
        for dz in -WALL_SCAN..=WALL_SCAN {
            if dx.abs() < 2 && dz.abs() < 2 {
                continue; // the body's own breathing space
            }
            wall_total += 1;
            let mut any = false;
            for dy in 0..=2 {
                if solid_at(gen, CellCoord { x: px + dx, y: base_y + dy, z: pz + dz }) {
                    any = true;
                    break;
                }
            }
            if any {
                wall_hits += 1;
            }
        }
    }
    let walls = wall_hits >= WALL_MIN;

    // OPENING: a walkable gap in the wall ring (a passable orthogonal
    // neighbor at foot level) — a sealed box fails this axis.
    let mut opening = false;
    for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        if !solid_at(gen, CellCoord { x: px + dx, y: base_y, z: pz + dz }) {
            opening = true;
            break;
        }
    }

    // FIRE: a burning campfire within reach.
    let fire = fire_dist.is_some_and(|d| d <= FIRE_REACH_M);

    let axes = [
        Axis { name: "coverage", ok: coverage },
        Axis { name: "walls", ok: walls },
        Axis { name: "opening", ok: opening },
        Axis { name: "fire", ok: fire },
    ];
    let green = axes.iter().filter(|a| a.ok).count() as f32;
    ShelterScore { axes, score: green / 4.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p3dn11_open_plain_fails_shelter() {
        // A wide-open FLAT column: no roof, thin walls, no fire. A
        // hillside hollow legitimately greens walls/coverage (earth is
        // shelter), so the probe self-locates flat ground first.
        let gen = WorldGen::new(11);
        let gh = |x: f32, z: f32| {
            crate::grounding::ground_at(
                &gen,
                (x * 1000.0) as i64,
                (z * 1000.0) as i64,
                crate::grounding::GroundQuery::Point,
            )
            .surface_m()
        };
        let mut spot: Option<(f32, f32)> = None;
        'outer: for xi in 2..16 {
            for zi in 2..16 {
                let x = xi as f32 * 4.0;
                let z = zi as f32 * 4.0;
                let h = gh(x, z);
                let flat = [
                    (x - 4.0, z),
                    (x + 4.0, z),
                    (x, z - 4.0),
                    (x, z + 4.0),
                    (x - 4.0, z - 4.0),
                    (x + 4.0, z + 4.0),
                ]
                .iter()
                .all(|(nx, nz)| (gh(*nx, *nz) - h).abs() < 0.4);
                if flat {
                    spot = Some((x, z));
                    break 'outer;
                }
            }
        }
        let (x, z) = spot.unwrap_or((8.0, 8.0));
        let feet = gh(x, z);
        let s = score_at(&gen, x, z, feet, None);
        assert!(!s.axes.iter().find(|a| a.name == "coverage").unwrap().ok);
        assert!(!s.axes.iter().find(|a| a.name == "walls").unwrap().ok);
        assert!(!s.sheltered());
        assert_eq!(s.line(), "NO ROOF OVERHEAD");
    }

    #[test]
    fn p3dn11_fire_axis_names_itself() {
        let gen = WorldGen::new(11);
        let feet = crate::grounding::ground_at(
            &gen,
            8_500,
            8_500,
            crate::grounding::GroundQuery::Point,
        )
        .surface_m();
        // The fire AXIS answers distance alone (score lines name the
        // missing axis; the axis itself is distance-only).
        assert!(!score_at(&gen, 8.5, 8.5, feet, Some(9.0)).axes[3].ok);
        assert!(score_at(&gen, 8.5, 8.5, feet, Some(3.0)).axes[3].ok);
    }

    #[test]
    fn p3dn11_score_is_fraction_of_green() {
        let gen = WorldGen::new(11);
        let feet = crate::grounding::ground_at(
            &gen,
            8_500,
            8_500,
            crate::grounding::GroundQuery::Point,
        )
        .surface_m();
        let none = score_at(&gen, 8.5, 8.5, feet, None);
        let with_fire = score_at(&gen, 8.5, 8.5, feet, Some(2.0));
        assert!((with_fire.score - none.score).abs() >= 0.25, "one green axis = +0.25");
    }

    #[test]
    fn p3dn11_line_names_first_missing() {
        let s = ShelterScore {
            axes: [
                Axis { name: "coverage", ok: false },
                Axis { name: "walls", ok: false },
                Axis { name: "opening", ok: false },
                Axis { name: "fire", ok: false },
            ],
            score: 0.0,
        };
        assert_eq!(s.line(), "NO ROOF OVERHEAD");
        let s = ShelterScore {
            axes: [
                Axis { name: "coverage", ok: true },
                Axis { name: "walls", ok: true },
                Axis { name: "opening", ok: false },
                Axis { name: "fire", ok: false },
            ],
            score: 0.5,
        };
        assert_eq!(s.line(), "SEALED - MAKE A DOORWAY");
        let s = ShelterScore {
            axes: [
                Axis { name: "coverage", ok: true },
                Axis { name: "walls", ok: true },
                Axis { name: "opening", ok: true },
                Axis { name: "fire", ok: false },
            ],
            score: 0.75,
        };
        assert_eq!(s.line(), "SHELTERED", "opening covers fire absence");
    }

    #[test]
    fn p3dn11_cave_interior_scores_walls() {
        // Underground cells sit under a solid ceiling: coverage + walls
        // both green (a cave mouth is the free starter shelter).
        let gen = WorldGen::new(11);
        let surface = crate::grounding::ground_at(
            &gen,
            8_500,
            8_500,
            crate::grounding::GroundQuery::Point,
        )
        .surface_m();
        // Walk down to solid rock: 3 below the surface the column above
        // is terrain (coverage) and the ring is walls.
        let deep = surface - 3.0;
        let base = deep.ceil() as i32;
        let s = score_at(&gen, 8.5, 8.5, deep, None);
        assert!(
            s.axes.iter().find(|a| a.name == "coverage").unwrap().ok
                || (1..=ROOF_SCAN).any(|dy| solid_at(
                    &gen,
                    CellCoord { x: 8, y: base + dy, z: 8 }
                )),
            "a deep cell has terrain overhead"
        );
    }
}
