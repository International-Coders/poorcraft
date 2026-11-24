// Probe N08 (v12): where IS the carve relative to the plaza on 123?
// List carved columns' distance from the plaza and check water law ON
// the carved band edges (stand 1-4 m from a carved column).
use pc3d_world::gen::WorldGen;
use pc3d_world::survival::water_edge_distance_m;

fn main() {
    let gen = WorldGen::new(123);
    let (px, pz) = (384_i64, 128_i64);
    let mut carved: Vec<(i64, i64, f32)> = Vec::new();
    for x in 60..700_i64 {
        for z in -100..400_i64 {
            if gen.river_channel(x * 1000, z * 1000).is_some() {
                let d = ((x - px).pow(2) + (z - pz).pow(2)) as f32;
                carved.push((x, z, d.sqrt()));
            }
        }
    }
    carved.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
    println!("carved cols near plaza: {}", carved.len());
    for (x, z, d) in carved.iter().take(8) {
        println!("  carved ({x},{z}) at {d:.0} m from plaza");
    }
    // On the nearest carved column, find a bank spot the law answers.
    if let Some(&(cx, cz, _)) = carved.first() {
        // Walk perpendicular offsets from the carved column.
        for off in 1..8_i64 {
            let bx = cx + off;
            let bz = cz;
            let d = water_edge_distance_m(&gen, &[bx as f32, 0.0, bz as f32]);
            let g = gen.effective_surface_mm(bx * 1000, bz * 1000);
            let ch = gen.river_channel(bx * 1000, bz * 1000);
            println!("bank cand ({bx},{bz}): water_d {d:?} surf {g} chan {ch:?}");
        }
    }
}
