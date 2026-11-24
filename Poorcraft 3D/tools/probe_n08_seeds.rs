// Probe N08 (v7): fast watershed scan — replicate RiverGraph::build's
// elevation math (macro_field per region) WITHOUT carving, to find the
// seeds whose discharge field puts RIVER_THRESHOLD edges near the city
// center (0,0 region). Also count cave presence cheaply later.
use pc3d_world::gen::WorldGen;
use pc3d_world::coords::RegionCoord;
use pc3d_world::hydro::RIVER_THRESHOLD;
use std::collections::BTreeMap;

fn main() {
    let half = 40_i32; // scan window smaller than the true 70 — RELATIVE
    // ranking still holds; a channel crossing within ±40 regions of the
    // city qualifies. Carve gate checked after for winners.
    let mut hits: Vec<(u64, f32)> = Vec::new();
    for seed in 3..200 {
        let gen = WorldGen::new(seed);
        // Elevation lattice.
        let mut elevation: BTreeMap<(i32, i32), i32> = BTreeMap::new();
        for x in -half..=half {
            for z in -half..=half {
                elevation.insert(
                    (x, z),
                    gen.macro_field(RegionCoord { x, z }).elevation_m,
                );
            }
        }
        // Steepest descent.
        let neigh = [(-1,0),(1,0),(0,-1),(0,1),(-1,-1),(-1,1),(1,-1),(1,1)];
        let mut downstream: BTreeMap<(i32,i32), Option<(i32,i32)>> = BTreeMap::new();
        for x in -half..=half {
            for z in -half..=half {
                let e = elevation[&(x,z)];
                let mut best = None;
                let mut best_e = e;
                for (dx,dz) in neigh {
                    let nk = (x+dx, z+dz);
                    if let Some(&ne) = elevation.get(&nk) {
                        if ne < best_e || (ne == best_e && best.is_some() && (nk.0, nk.1) < best.unwrap()) {
                            best_e = ne;
                            best = Some(nk);
                        }
                    }
                }
                downstream.insert((x,z), best);
            }
        }
        // Discharge descending order.
        let mut order: Vec<(i32,i32)> = elevation.keys().copied().collect();
        order.sort_by(|a,b| elevation[b].cmp(&elevation[a]).then(a.cmp(b)));
        let mut discharge: BTreeMap<(i32,i32), u64> = BTreeMap::new();
        for k in &order { discharge.insert(*k, 0); }
        for k in &order {
            *discharge.entry(*k).or_insert(0) += 1;
            let d_here = *discharge.get(k).unwrap();
            if let Some(Some(down)) = downstream.get(k) {
                *discharge.entry(*down).or_insert(0) += d_here;
            }
        }
        // Nearest carved-candidate river edge midpoint to city center
        // (city center region (0,0) → center pixel (128,128)).
        let mut best_d = f32::INFINITY;
        for x in -half..=half {
            for z in -half..=half {
                if let Some(Some(d)) = downstream.get(&(x,z)) {
                    if discharge[&(x,z)] >= RIVER_THRESHOLD && discharge[d] >= RIVER_THRESHOLD {
                        let mx = ((x as f32 + 0.5) + (d.0 as f32 + 0.5)) / 2.0 * 256.0;
                        let mz = ((z as f32 + 0.5) + (d.1 as f32 + 0.5)) / 2.0 * 256.0;
                        let dist = (mx*mx + mz*mz).sqrt();
                        if dist < best_d { best_d = dist; }
                    }
                }
            }
        }
        if best_d <= 400.0 {
            hits.push((seed, best_d));
            println!("seed {seed}: river edge at {best_d:.0} m (pre-carve check)");
        }
    }
    println!("---- {} seeds with near rivers (of 197)", hits.len());
}
