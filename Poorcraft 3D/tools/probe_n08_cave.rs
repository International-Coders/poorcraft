// Probe N08 (v9): seeds 75/123 have carved rivers AT the plaza. Check
// the OTHER find_showcase gate: cave_near_city (ring 3 patch scan for a
// corridor pocket). Ring 3 = 49 patches; time it.
use pc3d_world::gen::WorldGen;
use pc3d_world::coords::{PatchCoord, RegionCoord};
use pc3d_render::terrain::{find_cave_pocket, pocket_has_corridor};

fn main() {
    for seed in [75u64, 123, 83, 63, 10] {
        let gen = WorldGen::new(seed);
        // city_scene's plaza is deterministic; approximate center: the
        // cave scan in slice.rs anchors at (center*16+8) with the center
        // region (0,0) → patch (8,1,8). Use the same anchor.
        let pc = (8, 8);
        let t0 = std::time::Instant::now();
        let mut found = None;
        'outer: for ring in 0..=3i32 {
            for dx in -ring..=ring {
                for dz in -ring..=ring {
                    if dx.abs() != ring && dz.abs() != ring {
                        continue;
                    }
                    let patch = PatchCoord { x: pc.0 + dx, y: 1, z: pc.1 + dz };
                    if let Some(pocket) = find_cave_pocket(&gen, patch) {
                        let (air, wall, dir) = pocket;
                        if pocket_has_corridor(&gen, air, dir) {
                            found = Some((air, wall, dir));
                            break 'outer;
                        }
                    }
                }
            }
        }
        println!(
            "seed {seed}: cave {} ({:?})",
            found.map_or("NONE".to_string(), |(a, _, d)| format!(
                "at ({},{},{}) dir {:?}",
                a.x, a.y, a.z, d
            )),
            t0.elapsed()
        );
        let _ = RegionCoord { x: 0, z: 0 };
    }
}
