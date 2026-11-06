//! N05 GPU proofs: the viewmodel pass draws the equipped tool into a real
//! offscreen frame. The pure matrix proves the CONTRACT; these prove the
//! PIXELS (the card: pixels + semantic state + declared camera).
//!
//! Detection is a REFERENCE-DIFF: the same renderer captures the empty
//! frame first; any changed pixel cluster at the projected grip IS the
//! tool (the sky gradient would false-positive a palette probe, so the
//! empty frame is the control — the repo's own cross-frame law).

use pc3d_render::renderer::Renderer;
use pc3d_render::viewmodel::{project_to_screen, DominantHand, ViewmodelSpec};

const W: u32 = 320;
const H: u32 = 240;

fn offscreen() -> Renderer {
    let mut r = Renderer::offscreen(W, H);
    r.set_pose(pc3d_render::camera::CameraPose::new([0.0, 2.0, 0.0], 0.0, 0.0));
    r
}

/// The grip's expected pixel for this frame size (the reference FOV the
/// spec was solved against).
fn grip_pixel(spec: &ViewmodelSpec, w: u32, h: u32) -> (i32, i32) {
    let m = spec.dominant_hand.mirror_x();
    let g = project_to_screen(
        [spec.grip_idle[0] * m, spec.grip_idle[1], spec.grip_idle[2]],
        spec.reference_fov_y_rad,
        w as f32 / h as f32,
        0.05,
        100.0,
    );
    (
        ((g[0] * 0.5 + 0.5) * w as f32) as i32,
        ((-g[1] * 0.5 + 0.5) * h as f32) as i32,
    )
}

/// Pixels changed (any channel > 10) between two frames inside the tool
/// window centered on the projected grip.
fn changed_px_near_grip(a: &[u8], b: &[u8], cx: i32, cy: i32) -> usize {
    let mut n = 0;
    for y in (cy - 48).max(0)..(cy + 48).min(H as i32) {
        for x in (cx - 48).max(0)..(cx + 48).min(W as i32) {
            let i = ((y as u32 * W + x as u32) * 4) as usize;
            if (a[i] as i32 - b[i] as i32).abs() > 10
                || (a[i + 1] as i32 - b[i + 1] as i32).abs() > 10
                || (a[i + 2] as i32 - b[i + 2] as i32).abs() > 10
            {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn p3dn05_equipped_tool_renders_at_the_projected_grip() {
    let mut r = offscreen();
    let (rep, empty) = r.capture_png(std::path::Path::new("/tmp/p3dn05_empty.png"), &[]);
    let _ = rep;
    r.equip_viewmodel(ViewmodelSpec::shovel())
        .expect("shovel spec must pass its own matrix gate");
    let (_, eq) = r.capture_png(std::path::Path::new("/tmp/p3dn05_idle.png"), &[]);
    let (cx, cy) = grip_pixel(&ViewmodelSpec::shovel(), W, H);
    let n = changed_px_near_grip(&empty, &eq, cx, cy);
    assert!(
        n > 400,
        "tool pixels must appear at the projected grip: {n} changed"
    );
}

#[test]
fn p3dn05_unequip_clears_the_tool() {
    let mut r = offscreen();
    let (_, empty) = r.capture_png(std::path::Path::new("/tmp/p3dn05_un_empty.png"), &[]);
    r.equip_viewmodel(ViewmodelSpec::shovel()).unwrap();
    let (_, eq) = r.capture_png(std::path::Path::new("/tmp/p3dn05_un_eq.png"), &[]);
    let (cx, cy) = grip_pixel(&ViewmodelSpec::shovel(), W, H);
    let eq_n = changed_px_near_grip(&empty, &eq, cx, cy);
    assert!(eq_n > 400, "equipped frame must show the tool: {eq_n}");
    r.unequip_viewmodel();
    let (_, un) = r.capture_png(std::path::Path::new("/tmp/p3dn05_un_off.png"), &[]);
    let un_n = changed_px_near_grip(&empty, &un, cx, cy);
    assert!(
        un_n <= eq_n / 10,
        "unequip must clear the tool: equipped {eq_n} vs unequipped {un_n} (vs empty)"
    );
}

#[test]
fn p3dn05_sweep_moves_the_tool() {
    let mut r = offscreen();
    let (_, empty) = r.capture_png(std::path::Path::new("/tmp/p3dn05_sw_empty.png"), &[]);
    r.equip_viewmodel(ViewmodelSpec::shovel()).unwrap();
    r.viewmodel_attack();
    let mut frames = Vec::new();
    for i in 0..10 {
        let (_, f) = r.capture_png(
            std::path::Path::new(&format!("/tmp/p3dn05_sw_{i}.png")),
            &[],
        );
        frames.push(f);
    }
    let (cx, cy) = grip_pixel(&ViewmodelSpec::shovel(), W, H);
    // Early frame: the tool is visibly there. Mid vs first: the head
    // travels (the silhouette changes inside the action window).
    let early = changed_px_near_grip(&empty, &frames[0], cx, cy);
    let mid_vs_first = changed_px_near_grip(&frames[0], &frames[6], cx, cy);
    assert!(early > 300, "tool visible during the sweep: {early}");
    assert!(
        mid_vs_first > 60,
        "the sweep must move the silhouette: {mid_vs_first} changed by frame 6"
    );
}

#[test]
fn p3dn05_refused_spec_cannot_equip() {
    let mut r = offscreen();
    // Dead-center grip: blocks the reticle disc in every cell.
    let mut bad = ViewmodelSpec::shovel();
    bad.grip_idle = [0.0, 0.0, -0.60];
    let err = r.equip_viewmodel(bad);
    assert!(
        err.is_err(),
        "a reticle-blocking spec must be refused at equip"
    );
    let msg = err.unwrap_err();
    assert!(msg.contains("reticle_clearance"), "named law: {msg}");
    // And nothing mounted: the frame matches the empty control.
    let (_, empty) = r.capture_png(std::path::Path::new("/tmp/p3dn05_ref_empty.png"), &[]);
    let (_, after) = r.capture_png(std::path::Path::new("/tmp/p3dn05_ref_after.png"), &[]);
    let d = pc3d_render::scene::pixel_difference_fraction(&empty, &after);
    assert!(
        d < 0.001,
        "a refused equip must leave the frame unchanged: {d:.5}"
    );
}

#[test]
fn p3dn05_left_hand_mirrors_in_frame() {
    let mut r = offscreen();
    let (_, empty) = r.capture_png(std::path::Path::new("/tmp/p3dn05_lh_empty.png"), &[]);
    let mut spec = ViewmodelSpec::shovel();
    spec.dominant_hand = DominantHand::Left;
    r.equip_viewmodel(spec).expect("left-hand spec must also close");
    let (_, eq) = r.capture_png(std::path::Path::new("/tmp/p3dn05_lh_eq.png"), &[]);
    let (cx, cy) = grip_pixel(&{
        let mut s = ViewmodelSpec::shovel();
        s.dominant_hand = DominantHand::Left;
        s
    }, W, H);
    assert!(cx < (W as i32) / 2, "left-hand grip projects left: {cx}");
    let n = changed_px_near_grip(&empty, &eq, cx, cy);
    assert!(n > 400, "left-hand tool visible at its grip: {n} changed");
    // And NOT at the right-hand position (the mirror is honest).
    let (rx, ry) = grip_pixel(&ViewmodelSpec::shovel(), W, H);
    assert!(rx > (W as i32) / 2, "right-hand grip projects right: {rx}");
    let wrong_side = changed_px_near_grip(&empty, &eq, rx, ry);
    assert!(
        wrong_side < n / 4,
        "no tool mass at the mirrored-away position: left {n} vs right {wrong_side}"
    );
}
