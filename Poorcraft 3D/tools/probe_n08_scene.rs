// Probe N08 (v10): full city_scene + river_near_city + cave_near_city
// on seed 123 — the exact find_showcase body — then the route's own
// finders (grass dig spot, bank spot) at the plaza.
use pc3d_render::city::city_scene;
use pc3d_world::coords::RegionCoord;

fn main() {
    let t0 = std::time::Instant::now();
    let (gen, center, _layout, plan) = city_scene(123, RegionCoord { x: 0, z: 0 });
    println!("city_scene ok ({:?}), plaza {:?}", t0.elapsed(), plan.plaza);
    let t1 = std::time::Instant::now();
    let river = river_near(&gen, center);
    println!("river near: {:?} ({:?})", river, t1.elapsed());
    let t2 = std::time::Instant::now();
    let cave = cave_near(&gen, center);
    println!("cave near: {:?} ({:?})", cave, t2.elapsed());
    // The route finders (copied laws).
    if let Some((graph, _table, edge)) = river {
        let _ = graph;
        let mx = ((edge.0 .0 as f32 + 0.5) + (edge.1 .0 as f32 + 0.5)) / 2.0 * 256.0;
        let mz = ((edge.0 .1 as f32 + 0.5) + (edge.1 .1 as f32 + 0.5)) / 2.0 * 256.0;
        println!("river edge midpoint ({mx:.0},{mz:.0})");
    }
    // Grass dig spot near plaza (bare-hand Grass law): reuse the app's
    // own finder logic inline (3x3 flat, grass material).
    use pc3d_world::gen::cell_material;
    let plaza = plan.plaza;
    let ground_at = |x: f32, z: f32| -> f32 {
        gen.effective_surface_mm((x * 1000.0) as i64, (z * 1000.0) as i64) as f32 / 1000.0
    };
    let mut grass = None;
    'g: for ring in 4..40u32 {
        for k in 0..(ring * 8).max(1) {
            let a = k as f32 / (ring * 8).max(1) as f32 * std::f32::consts::TAU;
            let cx = plaza.x as f32 + 0.5 + ring as f32 * a.cos();
            let cz = plaza.z as f32 + 0.5 + ring as f32 * a.sin();
            let sx = cx.floor() as f32 + 0.5;
            let sz = cz.floor() as f32 + 0.5;
            let g = ground_at(sx, sz);
            let flat = (0..3).all(|i| {
                (0..3).all(|j| {
                    (ground_at(sx - 1.0 + i as f32, sz - 1.0 + j as f32) - g).abs() <= 0.35
                })
            });
            if !flat {
                continue;
            }
            let region = RegionCoord {
                x: (sx as i64 * 1000).div_euclid(256_000) as i32,
                z: (sz as i64 * 1000).div_euclid(256_000) as i32,
            };
            let m = cell_material(
                gen.biome(region),
                (g.floor() as i64) * 1000,
                (g * 1000.0) as i64,
            );
            if m == pc3d_world::gen::CellMaterial::Grass {
                grass = Some((sx, sz));
                break 'g;
            }
        }
    }
    println!("grass dig spot: {grass:?}");
    // Bank spot via the water law, wide rings.
    use pc3d_world::survival::{water_edge_distance_m, CAST_REACH_M};
    let mut bank = None;
    'b: for ring_m in [24.0f32, 64.0, 120.0, 200.0, 300.0, 400.0] {
        for k in 0..48 {
            let a = k as f32 / 48.0 * std::f32::consts::TAU;
            let cx = plaza.x as f32 + 0.5 + ring_m * a.cos();
            let cz = plaza.z as f32 + 0.5 + ring_m * a.sin();
            let sx = cx.floor() as f32 + 0.5;
            let sz = cz.floor() as f32 + 0.5;
            match water_edge_distance_m(&gen, &[sx, 0.0, sz]) {
                Some(d) if (0.5..=CAST_REACH_M).contains(&d) => {}
                _ => continue,
            }
            let g = ground_at(sx, sz);
            let flat = (0..3).all(|i| {
                (0..3).all(|j| {
                    (ground_at(sx - 1.0 + i as f32, sz - 1.0 + j as f32) - g).abs() <= 0.35
                })
            });
            if !flat {
                continue;
            }
            bank = Some((sx, sz, g));
            break 'b;
        }
    }
    println!("bank spot: {bank:?}");
}

// Inlined river_near_city (private in slice.rs) — same body.
fn river_near(
    gen: &pc3d_world::gen::WorldGen,
    center: RegionCoord,
) -> Option<(pc3d_world::hydro::RiverGraph, pc3d_world::flow::FlowTable, ((i32, i32), (i32, i32)))> {
    let half = pc3d_world::river_carve::WATERSHED_HALF;
    let graph = pc3d_world::hydro::RiverGraph::new(gen, half);
    let (ccx, ccz) = (
        center.x as f32 * 256.0 + 128.0,
        center.z as f32 * 256.0 + 128.0,
    );
    let mut best: Option<(((i32, i32), (i32, i32)), f32)> = None;
    for x in -half..=half {
        for z in -half..=half {
            let r = RegionCoord { x, z };
            let Some(d) = graph.downstream(r) else { continue };
            if graph.discharge(d) < pc3d_world::hydro::RIVER_THRESHOLD
                || graph.discharge(r) < pc3d_world::hydro::RIVER_THRESHOLD
            {
                continue;
            }
            let mx = ((r.x as f32 + 0.5) + (d.x as f32 + 0.5)) / 2.0 * 256.0;
            let mz = ((r.z as f32 + 0.5) + (d.z as f32 + 0.5)) / 2.0 * 256.0;
            if gen
                .river_channel((mx * 1000.0) as i64, (mz * 1000.0) as i64)
                .is_none()
            {
                continue;
            }
            let dist = ((mx - ccx).powi(2) + (mz - ccz).powi(2)).sqrt();
            if best.as_ref().map(|(_, b)| dist < *b).unwrap_or(true) {
                best = Some((((r.x, r.z), (d.x, d.z)), dist));
            }
        }
    }
    let (edge, dist) = best?;
    if dist > 450.0 {
        return None;
    }
    let table = pc3d_world::flow::FlowTable::from_graph(&graph);
    Some((graph, table, edge))
}

fn cave_near(
    gen: &pc3d_world::gen::WorldGen,
    center: RegionCoord,
) -> Option<(pc3d_world::coords::CellCoord, pc3d_world::coords::CellCoord, [i32; 3])> {
    let pc = (center.x * 16 + 8, center.z * 16 + 8);
    for ring in 0..=3i32 {
        for dx in -ring..=ring {
            for dz in -ring..=ring {
                if dx.abs() != ring && dz.abs() != ring {
                    continue;
                }
                let patch = pc3d_world::coords::PatchCoord {
                    x: pc.0 + dx,
                    y: 1,
                    z: pc.1 + dz,
                };
                if let Some(pocket) = pc3d_render::terrain::find_cave_pocket(gen, patch) {
                    let (air, wall, dir) = pocket;
                    if pc3d_render::terrain::pocket_has_corridor(gen, air, dir) {
                        return Some((air, wall, dir));
                    }
                }
            }
        }
    }
    None
}
