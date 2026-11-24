// Probe N08 (v13): bank spot via water law on seed 123 near the carve
// (128,127) — check flatness + the material at the bank cells.
use pc3d_world::gen::{WorldGen, cell_material, CellMaterial};
use pc3d_world::coords::RegionCoord;
use pc3d_world::survival::{water_edge_distance_m, CAST_REACH_M};

fn main() {
    let gen = WorldGen::new(123);
    let ground_at = |x: i64, z: i64| gen.effective_surface_mm(x * 1000, z * 1000);
    // Bank ring around the carve at (128,127): try all cells with
    // water_d in (0.5..=3], flat 3x3, and report material.
    let mut found = 0;
    for x in 124..140_i64 {
        for z in 120..136_i64 {
            let Some(d) = water_edge_distance_m(&gen, &[x as f32, 0.0, z as f32]) else {
                continue;
            };
            if !(0.5..=CAST_REACH_M).contains(&d) {
                continue;
            }
            let g = ground_at(x, z);
            let flat = (0..3).all(|i| {
                (0..3).all(|j| {
                    (ground_at(x - 1 + i, z - 1 + j) - g).abs() <= 350
                })
            });
            let region = RegionCoord { x: (x * 1000).div_euclid(256_000) as i32, z: (z * 1000).div_euclid(256_000) as i32 };
            let m = cell_material(gen.biome(region), (g / 1000) * 1000, g);
            found += 1;
            if found <= 12 {
                println!("bank ({x},{z}): d {d:.1} flat {flat} surf {g} mat {m:?}");
            }
        }
    }
    println!("total castable bank cells: {found}");
    let _ = CellMaterial::Air;
}
