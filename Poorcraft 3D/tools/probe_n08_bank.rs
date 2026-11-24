// Probe N08 (v6): does a bank cell exist AT ALL on seed 7 near the
// channel at (-256,-256)? Scan a dense 24 m window across the channel
// and record the water law + material of every column in a 1 m grid.
use pc3d_world::gen::{WorldGen, cell_material};
use pc3d_world::coords::RegionCoord;
use pc3d_world::survival::{water_edge_distance_m, CAST_REACH_M};

fn main() {
    let gen = WorldGen::new(7);
    // Channel passes through (-256,-256) diagonally (45° from the walk).
    // Scan the perpendicular band densely.
    let mut n_ok = 0;
    let mut first = None;
    for x in (-300_i64..=-212).step_by(1) {
        for z in (-300_i64..=-212).step_by(1) {
            if gen.river_channel(x * 1000, z * 1000).is_some() {
                continue;
            }
            let Some(d) = water_edge_distance_m(&gen, &[x as f32, 0.0, z as f32]) else {
                continue;
            };
            if !(0.0..=CAST_REACH_M).contains(&d) {
                continue;
            }
            n_ok += 1;
            if first.is_none() {
                first = Some((x, z, d));
            }
        }
    }
    println!("bank-ish columns (any distance ≤ 3): {n_ok}, first {first:?}");

    // What's the surface material at (-250,-250) etc.? (is the bank under
    // water? The channel cut -24 m deep — banks may be underwater.)
    for (x, z) in [(-250i64, -250), (-240, -240), (-230, -230)] {
        let g = gen.effective_surface_mm(x * 1000, z * 1000);
        let region = RegionCoord { x: x.div_euclid(256) as i32, z: z.div_euclid(256) as i32 };
        let m = cell_material(gen.biome(region), g, g);
        let d = water_edge_distance_m(&gen, &[x as f32, 0.0, z as f32]);
        println!("({x},{z}): surface {g} material {m:?} water_d {d:?}");
    }
}
