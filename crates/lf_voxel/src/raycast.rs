use glam::{Vec3, IVec3};

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
}
