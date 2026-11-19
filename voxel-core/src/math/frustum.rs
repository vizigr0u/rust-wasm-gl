//! View frustum from a view-projection matrix, plus a conservative AABB test.
use glam::{Mat4, Vec3, Vec4};

/// Six inward-pointing planes (`xyz` = unit normal, `w` = distance).
#[derive(Debug, Clone, Copy)]
pub struct Frustum {
    planes: [Vec4; 6],
}

impl Frustum {
    /// Gribb-Hartmann extraction for WebGPU clip space (x, y in -1..1, z in 0..1).
    pub fn from_view_proj(m: Mat4) -> Self {
        let (r0, r1, r2, r3) = (m.row(0), m.row(1), m.row(2), m.row(3));
        let raw = [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2];
        let planes = raw.map(|p| {
            let len = p.truncate().length();
            if len > f32::EPSILON {
                p / len
            } else {
                p
            }
        });
        Self { planes }
    }

    /// False only when the box is fully outside one plane. A few false positives
    /// near the frustum corners are fine.
    pub fn intersects_aabb(&self, min: Vec3, max: Vec3) -> bool {
        self.planes.iter().all(|p| {
            let n = p.truncate();
            let far_corner = Vec3::select(n.cmpge(Vec3::ZERO), max, min);
            n.dot(far_corner) + p.w >= 0.0
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Eye at origin looking +X, 90 degree fov, near 1, far 100.
    fn frustum() -> Frustum {
        let view = glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::X, Vec3::Y);
        let proj = glam::camera::rh::proj::directx::perspective(
            std::f32::consts::FRAC_PI_2,
            1.0,
            1.0,
            100.0,
        );
        Frustum::from_view_proj(proj * view)
    }

    fn cube(c: Vec3, half: f32) -> (Vec3, Vec3) {
        (c - Vec3::splat(half), c + Vec3::splat(half))
    }

    fn visible(c: Vec3, half: f32) -> bool {
        let (a, b) = cube(c, half);
        frustum().intersects_aabb(a, b)
    }

    #[test]
    fn in_front_visible_behind_culled() {
        assert!(visible(Vec3::new(10.0, 0.0, 0.0), 1.0));
        assert!(!visible(Vec3::new(-10.0, 0.0, 0.0), 1.0));
    }

    #[test]
    fn sides_culled() {
        assert!(!visible(Vec3::new(10.0, 0.0, 30.0), 1.0));
        assert!(!visible(Vec3::new(10.0, 0.0, -30.0), 1.0));
        assert!(!visible(Vec3::new(10.0, 30.0, 0.0), 1.0));
        assert!(!visible(Vec3::new(10.0, -30.0, 0.0), 1.0));
    }

    #[test]
    fn straddling_near_and_containing_eye_visible() {
        assert!(visible(Vec3::new(1.0, 0.0, 0.0), 0.5));
        assert!(visible(Vec3::ZERO, 2.0));
    }

    #[test]
    fn beyond_far_culled() {
        assert!(visible(Vec3::new(98.0, 0.0, 0.0), 1.0));
        assert!(!visible(Vec3::new(120.0, 0.0, 0.0), 1.0));
    }

    #[test]
    fn pitched_up_culls_box_below() {
        let view = glam::camera::rh::view::look_at_mat4(
            Vec3::ZERO,
            Vec3::new(1.0, 1.0, 0.0).normalize(),
            Vec3::Y,
        );
        let proj = glam::camera::rh::proj::directx::perspective(
            std::f32::consts::FRAC_PI_2,
            1.0,
            1.0,
            100.0,
        );
        let f = Frustum::from_view_proj(proj * view);
        let (a, b) = cube(Vec3::new(10.0, -10.0, 0.0), 1.0);
        assert!(!f.intersects_aabb(a, b));
        let (a, b) = cube(Vec3::new(10.0, 10.0, 0.0), 1.0);
        assert!(f.intersects_aabb(a, b));
    }
}
