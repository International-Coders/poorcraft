use pc3d_world::gen::WorldGen;
use pc3d_world::nav::NavPatch;
use pc3d_world::terrain::SceneSpec;
use pc3d_world::coords::CellCoord;
#[test]
fn probe_nav_diag() {
    let (seed, patch) = SceneSpec::SmoothHills.patch();
    let gen = WorldGen::new(seed);
    let nav = NavPatch::from_gen(&gen, patch);
    let o = patch.origin();
    let ax = o.x.div_euclid(1000) as i32;
    let az = o.z.div_euclid(1000) as i32;
    let from = CellCoord { x: ax + 2, y: 0, z: az + 2 };
    let to = CellCoord { x: ax + 13, y: 0, z: az + 13 };
    let r = nav.path(from, to);
    println!("path len: {:?}", r.as_ref().map(|p| p.len()));
    assert!(r.is_some());
}
