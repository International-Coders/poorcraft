// Probe N08 (v11): seed 123's carve near the plaza — the edge midpoint
// is (0,0), so the channel crosses the plaza itself. Walk the channel
// line: where does river_channel fire near plaza (384,128)? And why
// does water_edge_distance answer None at the plaza?
use pc3d_world::gen::WorldGen;
use pc3d_world::survival::water_edge_distance_m;

fn main() {
    let gen = WorldGen::new(123);
    let mut carved: Vec<(i64, i64)> = Vec::new();
    for x in -100..=100_i64 {
        for z in 40..220_i64 {
            if gen.river_channel(x * 1000, z * 1000).is_some() {
                carved.push((x, z));
            }
        }
    }
    println!("carved cols in window: {}", carved.len());
    for (x, z) in carved.iter().take(10) {
        println!("  carved at ({x},{z})");
    }
    // water law at plaza + offsets
    for (dx, dz) in [(0i64, 0i64), (10, 0), (-10, 0), (0, 10), (0, -10), (30, 30), (50, -50)] {
        let x = 384 + dx;
        let z = 128 + dz;
        let d = water_edge_distance_m(&gen, &[x as f32, 0.0, z as f32]);
        let ch = gen.river_channel(x * 1000, z * 1000);
        println!("({x},{z}): water_d {d:?} channel {ch:?}");
    }
}
