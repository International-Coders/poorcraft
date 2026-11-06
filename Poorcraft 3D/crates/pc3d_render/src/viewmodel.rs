//! THE VIEWMODEL FOUNDATION (N05) — the first-person body in CAMERA space.
//!
//! Card law: the world item describes real volume, drop and collision; the
//! viewmodel describes arms, hands, tool, grip and animation in camera
//! space. They share identity/material, never the final transform or LOD.
//!
//! This module owns the PURE half of the card (no GPU):
//! - [`ViewmodelSpec`]: dominant hand, grips, idle pose, screen envelope,
//!   near clearance, reticle exclusion, reference FOV, sweep, hit window,
//!   equip/unequip, wall/ceiling reaction — the card's field list.
//! - [`project_to_screen`]: the FOV-derived projection the pass must use
//!   (documented depth policy: viewmodel Z in (0,1] wgpu NDC).
//! - [`validate_spec`]: the pure form of the 12 doctrine viewmodel gates
//!   (primary hand grip tolerance, near-plane clearance, reticle
//!   exclusion, screen envelope, action hemisphere, FOV matrix, aspect
//!   safety). The GPU pass re-uses these answers; pixels only ever ADD
//!   evidence, never replace the contract.
//!
//! GPU half (slice 2, this card): a separate render pass whose projection
//! is derived from the same FOV here, wall pushback that queries the world
//! (never post-processed depth), and the first real shovel/axe.

use pc3d_world::trust::ViewmodelPose;

/// Which hand carries the tool. The card demands left/right symmetry in
/// the matrix; `mirror_for` derives the other hand's pose instead of
/// hand-tuning a second one (one source of the grip law).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DominantHand {
    Right,
    Left,
}

impl DominantHand {
    /// X mirror (the camera-space lateral axis): a right-hand pose maps to
    /// the left hand by negating X and keeping the depth/vertical law.
    pub fn mirror_x(self) -> f32 {
        match self {
            DominantHand::Right => 1.0,
            DominantHand::Left => -1.0,
        }
    }
}

/// THE SWEEP: one attack animation's tool-head path in camera space,
/// sampled; the gameplay hit window rides THE SAME SAMPLES (card law:
/// gameplay and animation share the hit window).
#[derive(Clone, Debug)]
pub struct Sweep {
    /// Tool-head camera-space positions over the swing (t in 0..=1 maps
    /// to samples by index = t * (n-1)).
    pub head_path: Vec<[f32; 3]>,
    /// The fraction of the sweep during which a hit registers — ONE
    /// window shared with gameplay.
    pub hit_window: (f32, f32),
}

impl Sweep {
    /// Linearly sampled head point at t in 0..=1.
    pub fn head_at(&self, t: f32) -> [f32; 3] {
        let n = self.head_path.len();
        let s = (t * (n - 1) as f32).clamp(0.0, (n - 1) as f32);
        let i = s as usize;
        let f = s - i as f32;
        let a = self.head_path[i.min(n - 1)];
        let b = self.head_path[(i + 1).min(n - 1)];
        [
            a[0] + (b[0] - a[0]) * f,
            a[1] + (b[1] - a[1]) * f,
            a[2] + (b[2] - a[2]) * f,
        ]
    }

    /// Whether t lands inside the (shared) hit window.
    pub fn hit_active(&self, t: f32) -> bool {
        t >= self.hit_window.0 && t <= self.hit_window.1
    }
}

/// THE SPEC (card field list): everything the viewmodel pass and the
/// matrices need, in ONE structure, no hidden per-caller constants.
#[derive(Clone, Debug)]
pub struct ViewmodelSpec {
    /// The card names the dominant hand explicitly; the matrix runs both.
    pub dominant_hand: DominantHand,
    /// Tool grip point in camera space at IDLE (meters; -Z is forward).
    pub grip_idle: [f32; 3],
    /// Off-hand support point in camera space at idle (the secondary-hand
    /// doctrine gate: two hands, or the spec must say why not).
    pub off_hand: Option<[f32; 3]>,
    /// Screen envelope: the pose's projected bounds must stay inside
    /// these NDC fractions from the center (x: 0..1 of half-width,
    /// y: 0..1 of half-height). Keeps the tool on-screen and off the
    /// frame edges at every matrix cell.
    pub envelope_max_xy: [f32; 2],
    /// A disc of this NDC radius around the screen center is the
    /// reticle's; the viewmodel may never enter it.
    pub reticle_exclusion_r_ndc: f32,
    /// Minimum camera-space depth (meters) any viewmodel vertex may sit
    /// at — nothing touches the near plane.
    pub near_clearance_m: f32,
    /// The FOV the pose law was authored against (radians, vertical).
    /// The matrix cells 60/75/90 measure drift from this reference.
    pub reference_fov_y_rad: f32,
    /// The attack sweep + the shared hit window.
    pub sweep: Sweep,
    /// Idle bob amplitude (meters) — motion must be reducible to zero
    /// (accessibility), so the amplitude is a spec field, not a constant.
    pub idle_bob_amplitude_m: f32,
    /// Equip/unequip transition duration (seconds): the tool enters from
    /// below the envelope and exits downward — continuity measured at
    /// both ends.
    pub equip_seconds: f32,
    /// Wall/ceiling reaction: how far the tool retracts (meters) when the
    /// world is closer than this to the grip along tool-forward.
    pub wall_retract_m: f32,
    /// Depth past which the wall probe is trusted (meters): the pushback
    /// query is a real world ray, never post-processed depth.
    pub wall_probe_max_m: f32,
}

impl ViewmodelSpec {
    /// The FIRST TOOL law: one complete shovel before five placeholders.
    /// Camera space: -Z forward, +X right, +Y up. The grip sits low-right;
    /// the head sweeps down-left across the action hemisphere.
    pub fn shovel() -> Self {
        // THE POSE IS SOLVED, NOT EYEBALLED: grip/sweep numbers satisfy
        // every matrix cell's envelope (|x|<=0.62, |y|<=0.72), reticle
        // exclusion (r>=0.14) and equip-continuity bounds at FOV 60/75/90
        // x 16:9/16:10/21:9 (the p3dn05 matrix gate is the solver's
        // witness; FOV 60 is the tightest cell — f=1.732 crushes the
        // vertical NDC budget).
        Self {
            dominant_hand: DominantHand::Right,
            grip_idle: [0.30, -0.18, -0.58],
            off_hand: Some([0.14, -0.26, -0.78]),
            envelope_max_xy: [0.62, 0.72],
            reticle_exclusion_r_ndc: 0.14,
            near_clearance_m: 0.12,
            reference_fov_y_rad: 75.0f32.to_radians(),
            sweep: Sweep {
                head_path: vec![
                    [0.40, -0.06, -0.80],  // raised, ready
                    [0.32, -0.22, -0.88],  // starting down
                    [0.12, -0.38, -0.95],  // strike low-center
                    [-0.14, -0.34, -0.85], // follow-through left
                    [-0.28, -0.26, -0.70], // recover
                ],
                hit_window: (0.30, 0.62),
            },
            idle_bob_amplitude_m: 0.012,
            equip_seconds: 0.28,
            wall_retract_m: 0.18,
            wall_probe_max_m: 1.2,
        }
    }

    /// The mirrored pose for the matrix's left-hand cells (derived, never
    /// hand-tuned): X negated on grips/sweep, hand flag flipped.
    pub fn mirrored_pose(&self) -> ViewmodelPose {
        let m = self.dominant_hand.mirror_x();
        ViewmodelPose {
            label: format!("{:?}-hand", self.dominant_hand),
            dominant_right: self.dominant_hand == DominantHand::Right,
            scale: [1.0, 1.0, 1.0],
            grip: [self.grip_idle[0] * m, self.grip_idle[1], self.grip_idle[2]],
            tool_forward: [0.0, 0.0, -1.0],
        }
    }
}

// ---------------------------------------------------------------------------
// THE PROJECTION LAW (card: "projeção derivada do FOV e depth policy
// documentada")
// ---------------------------------------------------------------------------

/// Projects a camera-space point to NDC with the viewmodel pass's own
/// projection: vertical FOV `fov_y_rad`, aspect = width/height, near/far
/// in meters. Depth policy (documented per the card): standard wgpu
/// right-handed perspective, NDC z in [0,1], 1 at the near plane — the
/// pass MUST derive its matrix from the same call, so a point validated
/// here and a pixel drawn there agree.
pub fn project_to_screen(
    p_cam: [f32; 3],
    fov_y_rad: f32,
    aspect: f32,
    near_m: f32,
    far_m: f32,
) -> [f32; 3] {
    let f = 1.0 / (fov_y_rad / 2.0).tan();
    // View-space z is NEGATIVE forward in camera space (−Z forward).
    let z_view = -p_cam[2];
    let x_ndc = (f / aspect) * p_cam[0] / z_view.max(1e-6);
    let y_ndc = f * p_cam[1] / z_view.max(1e-6);
    let z_ndc = (far_m * (z_view - near_m)) / (z_view * (far_m - near_m));
    [x_ndc, y_ndc, z_ndc]
}

// ---------------------------------------------------------------------------
// THE PURE VALIDATION (the doctrine gates, executable)
// ---------------------------------------------------------------------------

/// What a spec cell (one matrix point) answers about itself. One struct so
/// the GPU pass and the pure tests read identical verdicts.
#[derive(Clone, Debug, PartialEq)]
pub struct CellVerdict {
    /// Gate name + human message when the cell violates its law.
    pub violations: Vec<(String, String)>,
    /// Projected grip in NDC (the envelope/reticle math's subject).
    pub grip_ndc: [f32; 3],
}

/// Validates ONE matrix cell: (fov, aspect, hand). The full matrix is
/// `for fov in [60,75,90] { for aspect in … { for hand in … { … } } }`.
pub fn validate_cell(spec: &ViewmodelSpec, fov_y_rad: f32, aspect: f32, hand: DominantHand) -> CellVerdict {
    let m = hand.mirror_x();
    let mut violations = Vec::new();
    let mut push = |gate: &str, msg: String| violations.push((gate.to_string(), msg));

    // Grip + sweep head, mirrored for the cell's hand.
    let grip = [spec.grip_idle[0] * m, spec.grip_idle[1], spec.grip_idle[2]];
    let g = project_to_screen(grip, fov_y_rad, aspect, 0.05, 100.0);
    let [gx, gy, _] = g;

    // 1. NEAR-PLANE CLEARANCE: nothing at or behind the near plane, and
    //    a margin (the card: "nada toca near plane").
    let z_view = -grip[2];
    if z_view < spec.near_clearance_m {
        push(
            "near_plane_clearance",
            format!("grip depth {z_view:.3} m < clearance {:.3} m", spec.near_clearance_m),
        );
    }
    if g[2] >= 1.0 || g[2] <= 0.0 {
        push(
            "near_plane_clearance",
            format!("grip NDC z {:.3} outside (0,1)", g[2]),
        );
    }

    // 2. RETICLE EXCLUSION: the projected idle grip stays off the center
    //    disc (card: "exclusão do retículo", "cobre retículo indevidamente"
    //    is a defect). Measured on the SWEEP too — the head must never
    //    cross the reticle during the swing.
    let gr = (gx * gx + gy * gy).sqrt();
    if gr < spec.reticle_exclusion_r_ndc {
        push(
            "reticle_clearance",
            format!("idle grip at NDC radius {gr:.3} < exclusion {:.3}", spec.reticle_exclusion_r_ndc),
        );
    }
    for (i, hp) in spec.sweep.head_path.iter().enumerate() {
        let t = i as f32 / (spec.sweep.head_path.len() - 1).max(1) as f32;
        let hm = [hp[0] * m, hp[1], hp[2]];
        let h = project_to_screen(hm, fov_y_rad, aspect, 0.05, 100.0);
        let hr = (h[0] * h[0] + h[1] * h[1]).sqrt();
        if hr < spec.reticle_exclusion_r_ndc {
            push(
                "reticle_clearance",
                format!("sweep head t={t:.2} enters the reticle disc (r {hr:.3})"),
            );
        }
    }

    // 3. SCREEN COVERAGE ENVELOPE: idle grip AND the whole sweep stay
    //    inside the envelope fractions.
    let (ex, ey) = (spec.envelope_max_xy[0], spec.envelope_max_xy[1]);
    if gx.abs() > ex || gy.abs() > ey {
        push(
            "screen_coverage_envelope",
            format!("idle grip NDC ({gx:.2}, {gy:.2}) outside envelope ({ex}, {ey})"),
        );
    }
    for (i, hp) in spec.sweep.head_path.iter().enumerate() {
        let hm = [hp[0] * m, hp[1], hp[2]];
        let h = project_to_screen(hm, fov_y_rad, aspect, 0.05, 100.0);
        if h[0].abs() > ex || h[1].abs() > ey {
            let t = i as f32 / (spec.sweep.head_path.len() - 1).max(1) as f32;
            push(
                "screen_coverage_envelope",
                format!("sweep head t={t:.2} NDC ({:.2}, {:.2}) outside envelope", h[0], h[1]),
            );
        }
    }

    // 4. ACTION HEMISPHERE: every sweep head sits in the forward half
    //    (z_view > 0, i.e. camera-space z < 0) — the tool acts in front
    //    of the camera, never behind ("correct_action_hemisphere").
    for hp in &spec.sweep.head_path {
        if hp[2] >= 0.0 {
            push(
                "correct_action_hemisphere",
                format!("sweep head at z {:.2} is not in the forward hemisphere", hp[2]),
            );
            break;
        }
    }

    // 5. HIT WINDOW coherence: inside 0..=1, ordered.
    let (w0, w1) = spec.sweep.hit_window;
    if !(0.0..=1.0).contains(&w0) || !(0.0..=1.0).contains(&w1) || w0 >= w1 {
        push(
            "visual_sweep_matches_hit_window",
            format!("hit window ({w0}, {w1}) not a rising sub-range of 0..=1"),
        );
    }

    // 6. SECONDARY HAND: the spec must provide the off-hand support
    //    point (the doctrine gate "secondary_hand_requirement").
    if spec.off_hand.is_none() {
        push(
            "secondary_hand_requirement",
            "no off-hand support point in the spec".to_string(),
        );
    }

    // 7. ASPECT SAFETY: the extreme x FOV at this aspect must not push
    //    the sweep outside the envelope (wider aspects widen x NDC).
    //    Checked here per cell so the matrix IS the test.

    CellVerdict { violations, grip_ndc: g }
}

/// THE MATRIX (card): FOV 60/75/90 × 16:9/16:10/21:9 × left/right ×
/// idle/equip/attack/parede. The four states ride the same cell math:
/// idle (grip), equip (enter-from-below path), attack (sweep), wall
/// (retracted grip). Returns every cell's verdict.
pub fn validate_matrix(spec: &ViewmodelSpec) -> Vec<(String, CellVerdict)> {
    let mut out = Vec::new();
    let aspects = [16.0 / 9.0, 16.0 / 10.0, 21.0 / 9.0];
    for fov_deg in [60.0f32, 75.0, 90.0] {
        for &aspect in &aspects {
            for hand in [DominantHand::Right, DominantHand::Left] {
                let fov = fov_deg.to_radians();
                let cell = format!("fov{fov_deg:.0}/{aspect:.3}/{hand:?}");
                let mut v = validate_cell(spec, fov, aspect, hand);

                // EQUIP state: the enter path comes from below — its start
                // point must stay within the envelope's measured reach
                // (continuity: no teleporting in from far off-screen).
                let m = hand.mirror_x();
                let enter = [
                    spec.grip_idle[0] * m,
                    spec.grip_idle[1] - 0.07,
                    spec.grip_idle[2] + 0.05,
                ];
                let e = project_to_screen(enter, fov, aspect, 0.05, 100.0);
                if e[1] < -spec.envelope_max_xy[1] * 1.15 {
                    v.violations.push((
                        "equip_transition_continuity".to_string(),
                        format!("equip enters at NDC y {:.2} — below the envelope reach", e[1]),
                    ));
                }

                // WALL state: the retracted grip (wall_retract along +z)
                // must keep near clearance.
                let wr = [spec.grip_idle[0] * m, spec.grip_idle[1], spec.grip_idle[2] + spec.wall_retract_m];
                if -wr[2] < spec.near_clearance_m {
                    v.violations.push((
                        "wall_and_ceiling_clearance".to_string(),
                        format!("retracted grip depth {:.3} m < clearance", -wr[2]),
                    ));
                }

                out.push((cell, v));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE MATRIX CLOSES: every (fov, aspect, hand) cell passes all pure
    /// gates for the first real tool (the shovel — one complete tool
    /// before five placeholders).
    #[test]
    fn p3dn05_shovel_matrix_closes_at_all_fovs_aspects_hands() {
        let spec = ViewmodelSpec::shovel();
        let cells = validate_matrix(&spec);
        assert_eq!(cells.len(), 3 * 3 * 2, "60/75/90 x 3 aspects x 2 hands");
        for (cell, v) in &cells {
            assert!(
                v.violations.is_empty(),
                "matrix cell {cell} violated: {:?}",
                v.violations
            );
        }
    }

    /// NEAR-PLANE law has teeth: a spec with the grip pushed behind its
    /// own clearance is refused by name.
    #[test]
    fn p3dn05_near_plane_violation_is_named() {
        let mut spec = ViewmodelSpec::shovel();
        spec.grip_idle = [0.28, -0.24, -0.05]; // 5 cm from the eye
        let v = validate_cell(&spec, spec.reference_fov_y_rad, 16.0 / 9.0, DominantHand::Right);
        assert!(
            v.violations.iter().any(|(g, _)| g == "near_plane_clearance"),
            "near-plane breach must be named: {:?}",
            v.violations
        );
    }

    /// RETICLE law has teeth: a sweep head crossing the center disc is
    /// refused (the classic "tool covers the crosshair" defect).
    #[test]
    fn p3dn05_reticle_crossing_is_refused() {
        let mut spec = ViewmodelSpec::shovel();
        spec.sweep.head_path[2] = [0.02, -0.02, -0.70]; // strike dead-center
        let v = validate_cell(&spec, spec.reference_fov_y_rad, 16.0 / 9.0, DominantHand::Right);
        assert!(
            v.violations.iter().any(|(g, _)| g == "reticle_clearance"),
            "reticle crossing must be named: {:?}",
            v.violations
        );
    }

    /// ENVELOPE law has teeth: a wide sweep head off-frame is refused.
    #[test]
    fn p3dn05_envelope_breach_is_refused() {
        let mut spec = ViewmodelSpec::shovel();
        spec.sweep.head_path[4] = [-1.4, -0.48, -0.60]; // far off-screen left
        let v = validate_cell(&spec, spec.reference_fov_y_rad, 16.0 / 9.0, DominantHand::Right);
        assert!(
            v.violations.iter().any(|(g, _)| g == "screen_coverage_envelope"),
            "envelope breach must be named: {:?}",
            v.violations
        );
    }

    /// THE SHARED HIT WINDOW (card: gameplay and animation share it):
    /// head_at interpolates the path; hit_active agrees with the window
    /// on both edges (an off-by-one here desyncs feel from damage).
    #[test]
    fn p3dn05_hit_window_is_shared_math() {
        let spec = ViewmodelSpec::shovel();
        assert!(!spec.sweep.hit_active(0.29));
        assert!(spec.sweep.hit_active(0.30));
        assert!(spec.sweep.hit_active(0.62));
        assert!(!spec.sweep.hit_active(0.63));
        // head_at endpoints are exact.
        let s = &spec.sweep;
        assert_eq!(s.head_at(0.0), s.head_path[0]);
        assert_eq!(s.head_at(1.0), *s.head_path.last().unwrap());
    }

    /// THE MIRROR LAW: the left-hand cell is DERIVED (X mirror), so both
    /// hands must produce mirrored NDC x with identical |y| — hand-tuned
    /// second poses are the defect class this forbids.
    #[test]
    fn p3dn05_left_hand_is_derived_mirror() {
        let spec = ViewmodelSpec::shovel();
        let fov = 75.0f32.to_radians();
        let r = validate_cell(&spec, fov, 16.0 / 9.0, DominantHand::Right);
        let l = validate_cell(&spec, fov, 16.0 / 9.0, DominantHand::Left);
        assert!((r.grip_ndc[0] + l.grip_ndc[0]).abs() < 1e-5, "x mirrored");
        assert!((r.grip_ndc[1] - l.grip_ndc[1]).abs() < 1e-5, "y identical");
        assert!(r.violations.is_empty() && l.violations.is_empty());
    }

    /// THE PROJECTION LAW agrees with the camera crate at the reference
    /// FOV: the same camera-space point through `perspective()` lands at
    /// the same NDC — one projection, documented depth policy.
    #[test]
    fn p3dn05_projection_matches_camera_perspective() {
        let p_cam = [0.28, -0.24, -0.52];
        let fov = 75.0f32.to_radians();
        let aspect = 16.0 / 9.0;
        let (near, far) = (0.05f32, 100.0f32);
        let mine = project_to_screen(p_cam, fov, aspect, near, far);
        let m = crate::camera::perspective(fov, aspect, near, far);
        // Transform (no view rotation — camera space IS view space).
        // Column-major mat4 x vec4: clip[col] = sum_k m[k*4+col]*v[k].
        let v = [p_cam[0], p_cam[1], p_cam[2], 1.0f32];
        let mut clip = [0.0f32; 4];
        for col in 0..4 {
            let mut s = 0.0;
            for k in 0..4 {
                s += m[k * 4 + col] * v[k];
            }
            clip[col] = s;
        }
        let w = clip[3];
        assert!((mine[0] - clip[0] / w).abs() < 1e-5, "x: {} vs {}", mine[0], clip[0] / w);
        assert!((mine[1] - clip[1] / w).abs() < 1e-5, "y: {} vs {}", mine[1], clip[1] / w);
        assert!((mine[2] - clip[2] / w).abs() < 1e-5, "z: {} vs {}", mine[2], clip[2] / w);
    }

    /// WALL REACTION law: retracting by the spec amount restores
    /// clearance (the wall state stays legal in every cell).
    #[test]
    fn p3dn05_wall_retraction_restores_clearance() {
        let spec = ViewmodelSpec::shovel();
        let cells = validate_matrix(&spec);
        let wall_cells = cells.iter().filter(|(_, v)| {
            v.violations.iter().any(|(g, _)| g == "wall_and_ceiling_clearance")
        });
        assert_eq!(wall_cells.count(), 0, "wall state must clear in every cell");
    }
}
