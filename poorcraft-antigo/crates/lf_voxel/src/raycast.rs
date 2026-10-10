use glam::{Vec3, IVec3};

use crate::{BlockState, registry};

/// Simple Amanatides & Woo DDA raycast for voxels.
pub fn raycast_voxel<F>(origin: Vec3, direction: Vec3, max_distance: f32, mut check: F) -> Option<(IVec3, IVec3)>
where
    F: FnMut(IVec3) -> bool,
{
    let dir = direction.normalize();
    let mut current = IVec3::new(origin.x.floor() as i32, origin.y.floor() as i32, origin.z.floor() as i32);
    
    let step = IVec3::new(
        if dir.x > 0.0 { 1 } else { -1 },
        if dir.y > 0.0 { 1 } else { -1 },
        if dir.z > 0.0 { 1 } else { -1 },
    );

    let t_delta = Vec3::new(
        if dir.x != 0.0 { (1.0 / dir.x).abs() } else { f32::INFINITY },
        if dir.y != 0.0 { (1.0 / dir.y).abs() } else { f32::INFINITY },
        if dir.z != 0.0 { (1.0 / dir.z).abs() } else { f32::INFINITY },
    );

    let mut next_t = Vec3::new(
        if dir.x > 0.0 { (current.x as f32 + 1.0 - origin.x) * t_delta.x }
        else if dir.x < 0.0 { (origin.x - current.x as f32) * t_delta.x }
        else { f32::INFINITY },
        if dir.y > 0.0 { (current.y as f32 + 1.0 - origin.y) * t_delta.y }
        else if dir.y < 0.0 { (origin.y - current.y as f32) * t_delta.y }
        else { f32::INFINITY },
        if dir.z > 0.0 { (current.z as f32 + 1.0 - origin.z) * t_delta.z }
        else if dir.z < 0.0 { (origin.z - current.z as f32) * t_delta.z }
        else { f32::INFINITY },
    );

    let mut normal = IVec3::ZERO;
    let mut distance = 0.0;

    while distance <= max_distance {
        if check(current) {
            return Some((current, normal));
        }

        if next_t.x < next_t.y && next_t.x < next_t.z {
            distance = next_t.x;
            next_t.x += t_delta.x;
            current.x += step.x;
            normal = IVec3::new(-step.x, 0, 0);
        } else if next_t.y < next_t.z {
            distance = next_t.y;
            next_t.y += t_delta.y;
            current.y += step.y;
            normal = IVec3::new(0, -step.y, 0);
        } else {
            distance = next_t.z;
            next_t.z += t_delta.z;
            current.z += step.z;
            normal = IVec3::new(0, 0, -step.z);
        }
    }

    None
}

/// Ray vs axis-aligned box (slab method). Returns the entry distance and
/// the face normal at the entry, or None when the ray misses.
fn ray_box(origin: Vec3, inv_dir: Vec3, min: Vec3, max: Vec3) -> Option<(f32, IVec3)> {
    let mut tmin = 0.0f32;
    let mut tmax = f32::INFINITY;
    let mut normal = IVec3::ZERO;
    for axis in 0..3 {
        let (o, d, mn, mx) = match axis {
            0 => (origin.x, inv_dir.x, min.x, max.x),
            1 => (origin.y, inv_dir.y, min.y, max.y),
            _ => (origin.z, inv_dir.z, min.z, max.z),
        };
        let (mut t1, mut t2) = ((mn - o) * d, (mx - o) * d);
        let mut face = -1i32;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
            face = 1;
        } else {
            face = -1;
        }
        if t1 > tmin {
            tmin = t1;
            normal = match axis {
                0 => IVec3::new(face, 0, 0),
                1 => IVec3::new(0, face, 0),
                _ => IVec3::new(0, 0, face),
            };
        }
        tmax = tmax.min(t2);
        if tmin > tmax {
            return None;
        }
    }
    Some((tmin, normal))
}

/// Shape-aware pick raycast: walks cells with the same DDA as
/// `raycast_voxel`, but a cell only counts when the ray actually crosses
/// one of its `registry::pick_boxes` — so the empty half of a slab, the
/// gaps of a flower, or the air beside a torch no longer swallow the
/// crosshair (loop 347 hitbox fix).
pub fn raycast_voxel_boxes<F>(origin: Vec3, direction: Vec3, max_distance: f32, mut state_at: F) -> Option<(IVec3, IVec3)>
where
    F: FnMut(IVec3) -> BlockState,
{
    let dir = direction.normalize();
    let inv_dir = Vec3::new(
        if dir.x != 0.0 { 1.0 / dir.x } else { f32::INFINITY },
        if dir.y != 0.0 { 1.0 / dir.y } else { f32::INFINITY },
        if dir.z != 0.0 { 1.0 / dir.z } else { f32::INFINITY },
    );
    // Reuse the cell walk, refining each targetable cell against its
    // pick boxes; a miss keeps the traversal going.
    raycast_voxel(origin, direction, max_distance, |cell| {
        let state = state_at(cell);
        if !registry::is_targetable(state) {
            return false;
        }
        let base = cell.as_vec3();
        for b in registry::pick_boxes(state) {
            let min = base + Vec3::new(b[0], b[1], b[2]);
            let max = base + Vec3::new(b[3], b[4], b[5]);
            if let Some((t, _)) = ray_box(origin, inv_dir, min, max) {
                if t <= max_distance {
                    return true;
                }
            }
        }
        false
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raycast() {
        let hit = raycast_voxel(Vec3::new(0.5, 0.5, -2.0), Vec3::Z, 5.0, |pos| {
            pos == IVec3::new(0, 0, 1)
        });
        assert!(hit.is_some());
        let (pos, normal) = hit.unwrap();
        assert_eq!(pos, IVec3::new(0, 0, 1));
        assert_eq!(normal, IVec3::new(0, 0, -1));
    }

    /// Regression (mob AI LOS): a perfectly axis-aligned ray from an
    /// origin lying exactly on a voxel boundary produced 0 × ∞ = NaN in
    /// the boundary-distance for the idle axes, so `distance` went NaN
    /// and the walk stopped after the first cell — rays were blind along
    /// exactly the lines mobs most often shoot (block-aligned, straight).
    #[test]
    fn axis_aligned_ray_from_boundary_origin_walks_the_line() {
        let mut visited = 0;
        let hit = raycast_voxel(Vec3::new(8.0, 1.9, 0.0), Vec3::new(-1.0, 0.0, 0.0), 6.0, |c| {
            visited += 1;
            c == IVec3::new(4, 1, 0)
        });
        assert!(visited > 4, "ray must walk past the origin cell (visited {})", visited);
        assert_eq!(hit, Some((IVec3::new(4, 1, 0), IVec3::new(1, 0, 0))));
    }

    /// Loop 347: shape-aware picking. A bottom slab is only picked when
    /// the ray actually crosses its lower half; the empty top half lets
    /// the crosshair fall through to whatever is behind.
    #[test]
    fn shaped_raycast_skips_the_empty_half_of_a_slab() {
        use crate::{Shape, registry::block};
        // slab at (2, 0, 0), stone at (4, 0, 0); ray travels +X at eye height
        let world = |c: IVec3| -> BlockState {
            if c.y == 0 && c.z == 0 {
                if c.x == 2 {
                    BlockState(block::PLANKS).with_shape(Shape::SlabBottom)
                } else if c.x == 4 {
                    BlockState(block::STONE)
                } else {
                    BlockState::AIR
                }
            } else {
                BlockState::AIR
            }
        };
        // high ray (y=0.8): over the slab, hits the stone behind it
        let hit = raycast_voxel_boxes(Vec3::new(0.5, 0.8, 0.5), Vec3::new(1.0, 0.0, 0.0), 8.0, world);
        assert_eq!(hit.map(|(c, _)| c), Some(IVec3::new(4, 0, 0)));
        // low ray (y=0.25): crosses the slab's solid half, hits the slab
        let hit = raycast_voxel_boxes(Vec3::new(0.5, 0.25, 0.5), Vec3::new(1.0, 0.0, 0.0), 8.0, world);
        assert_eq!(hit.map(|(c, _)| c), Some(IVec3::new(2, 0, 0)));
    }

    /// Loop 347: a flower's pick box is a small inset box — aiming at the
    /// air around its edges targets the ground instead, exactly like the
    /// visible plant.
    #[test]
    fn shaped_raycast_picks_flowers_only_inside_their_box() {
        use crate::registry::block;
        let world = |c: IVec3| -> BlockState {
            if c == IVec3::new(3, 1, 0) {
                BlockState(block::LAVENDER)
            } else if c.y == 0 && c.z == 0 {
                BlockState(block::GRASS)
            } else {
                BlockState::AIR
            }
        };
        // above the 0.8-tall plant box (top at y=1.8): nothing to hit
        let hit = raycast_voxel_boxes(Vec3::new(0.5, 1.9, 0.5), Vec3::new(1.0, 0.0, 0.0), 8.0, world);
        assert_eq!(hit.map(|(c, _)| c), None, "ray above the plant box hits nothing");
        // low ray straight through the stem area: the flower cell
        let hit = raycast_voxel_boxes(Vec3::new(0.5, 1.4, 0.5), Vec3::new(1.0, -0.08, 0.0).normalize(), 8.0, world);
        assert!(hit.is_some(), "descending ray meets the plant box");
    }
}
