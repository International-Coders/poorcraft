//! N03 RED — the voxel authority does not know rivers exist.
//!
//! The doctrine (05-MUNDO-RIOS-SPAWN): "RiverCarve cria leito sob toda
//! superfície visível" — bed and surface must live in the SAME authority
//! render, collision and nav consume (`final_solid`). Today the river is
//! ONLY a render-side strip (`pc3d_render::world_features::ConformingWater`)
//! whose heights follow the un-carved analytic surface: the voxel column at
//! a river channel carries NO Water cell — collision/nav see solid land
//! where the player should wade, and the blue strip floats over ground.
//!
//! This probe prints the measured evidence for the RED state. It exits 1
//! while the defect exists and 0 once RiverCarve lands (then it is folded
//! into a real gate test).

use pc3d_world::coords::RegionCoord;
use pc3d_world::hydro::{RiverGraph, RIVER_THRESHOLD};
use pc3d_world::terrain::final_solid;
use pc3d_world::gen::WorldGen;

fn main() {
    let seed = 4242u64;
    let gen = WorldGen::new(seed);
    let graph = RiverGraph::new(&gen, 24);

    // Find the nearest river region to the origin (same rule the render
    // proofs use: downstream exists and carries threshold discharge).
    let mut river: Option<(RegionCoord, i32)> = None;
    for x in -12..=12 {
        for z in -12..=12 {
            let r = RegionCoord { x, z };
            let Some(down) = graph.downstream(r) else { continue };
            if graph.discharge(down) >= RIVER_THRESHOLD {
                let d = x.abs() + z.abs();
                if river.as_ref().map(|(_, b)| d < *b).unwrap_or(true) {
                    river = Some((r, d));
                }
            }
        }
    }
    let Some((r, dist)) = river else {
        println!("RED inconclusive: no river region within 12 of origin (seed {seed})");
        std::process::exit(2);
    };
    println!("river region {:?} (manhattan {dist}) discharge {}", r, graph.discharge(RegionCoord { x: r.x, z: r.z }));

    // Walk the channel center column: the flow record's own region center.
    let cx = (r.x as i64 * 256 + 128) * 1000;
    let cz = (r.z as i64 * 256 + 128) * 1000;
    let surface_mm = gen.effective_surface_mm(cx, cz);
    let surface_m = surface_mm as f32 / 1000.0;
    println!("channel center ({}, {}) analytic surface {:.2} m", cx, cz, surface_m);

    // Count Water cells in the top 6 m of the column (what a wading
    // player's legs would occupy).
    let mut water_cells = 0usize;
    let mut solid_at_surface = false;
    for y in 0..=(surface_m as i64 * 1000 / 1000) + 6 {
        let a = final_solid(&gen, cx, y * 1000, cz);
        if matches!(a.material, pc3d_world::gen::CellMaterial::Water) {
            water_cells += 1;
        }
        if y * 1000 <= surface_mm && (y + 1) * 1000 > surface_mm {
            solid_at_surface = a.solid;
        }
    }

    println!("water cells in top 6 m of the channel column: {water_cells}");
    println!("solid ground at the analytic surface: {solid_at_surface}");
    if water_cells == 0 {
        println!("RED: the voxel authority has NO river channel — the strip floats over land.");
        std::process::exit(1);
    }
    println!("GREEN: the channel is carved in the voxel authority.");
}
