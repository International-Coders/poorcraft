// Probe N08 (v8): for the 14 candidate seeds, check the carve gate at
// the watershed scale (RiverGraph 70 + river_channel at midpoints) and
// cave_near_city (find_cave_pocket within ring 3 patches of the plaza).
use pc3d_world::gen::WorldGen;
use pc3d_world::coords::RegionCoord;
use pc3d_world::hydro::{RiverGraph, RIVER_THRESHOLD};
use pc3d_world::river_carve::WATERSHED_HALF;

fn main() {
    for seed in [75u64, 123, 83, 63, 101, 10, 190] {
        let t0 = std::time::Instant::now();
        let gen = WorldGen::new(seed);
        let graph = RiverGraph::new(&gen, WATERSHED_HALF);
        // Carve-gated nearest edge midpoint (same law as river_near_city).
        let mut best_d = f32::INFINITY;
        for x in -WATERSHED_HALF..=WATERSHED_HALF {
            for z in -WATERSHED_HALF..=WATERSHED_HALF {
                let r = RegionCoord { x, z };
                let Some(d) = graph.downstream(r) else { continue };
                if graph.discharge(d) < RIVER_THRESHOLD
                    || graph.discharge(r) < RIVER_THRESHOLD
                {
                    continue;
                }
                let mx = ((r.x as f32 + 0.5) + (d.x as f32 + 0.5)) / 2.0 * 256.0;
                let mz = ((r.z as f32 + 0.5) + (d.z as f32 + 0.5)) / 2.0 * 256.0;
                if gen.river_channel((mx * 1000.0) as i64, (mz * 1000.0) as i64).is_none() {
                    continue;
                }
                let dist = (mx * mx + mz * mz).sqrt();
                if dist < best_d {
                    best_d = dist;
                }
            }
        }
        let t1 = t0.elapsed();
        println!(
            "seed {seed}: carve-gated river at {best_d:.0} m (graph {:?})",
            t1
        );
    }
}
