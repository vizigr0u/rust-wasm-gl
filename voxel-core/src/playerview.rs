//! The player's own view direction and frustum, independent of the orbit debug camera.
use glam::{Mat4, Vec3};

use crate::math::Frustum;

pub const PLAYER_FOV_Y_DEGREES: f32 = 70.0;
pub const PLAYER_NEAR: f32 = 0.1;
/// 15 chunks of 16 blocks.
pub const PLAYER_FAR: f32 = 240.0;
pub const LOOK_SPEED_DEG_PER_S: f32 = 90.0;
const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;

#[derive(Debug, Clone)]
pub struct PlayerView {
    yaw: f32,
    pitch: f32,
    aspect: f32,
}

impl Default for PlayerView {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerView {
    /// Looks along +X (yaw 0), level.
    pub fn new() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            aspect: 800.0 / 600.0,
        }
    }

    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(cy * cp, sp, sy * cp)
    }

    /// Horizontal right axis (yaw only).
    pub fn right(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        Vec3::new(-sy, 0.0, cy)
    }

    /// Radians. Positive yaw turns toward +Z, positive pitch looks up.
    pub fn rotate(&mut self, d_yaw: f32, d_pitch: f32) {
        self.yaw = (self.yaw + d_yaw).rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch + d_pitch).clamp(-MAX_PITCH, MAX_PITCH);
    }

    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Ignores degenerate sizes.
    pub fn set_viewport_size(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.aspect = width as f32 / height as f32;
        }
    }

    pub fn view(&self, eye: Vec3) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(eye, eye + self.forward(), Vec3::Y)
    }

    pub fn projection(&self) -> Mat4 {
        glam::camera::rh::proj::directx::perspective(
            PLAYER_FOV_Y_DEGREES.to_radians(),
            self.aspect,
            PLAYER_NEAR,
            PLAYER_FAR,
        )
    }

    pub fn view_proj(&self, eye: Vec3) -> Mat4 {
        self.projection() * self.view(eye)
    }

    pub fn frustum(&self, eye: Vec3) -> Frustum {
        Frustum::from_view_proj(self.view_proj(eye))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_forward_is_plus_x() {
        let f = PlayerView::new().forward();
        assert!((f - Vec3::X).length() < 1e-6);
    }

    #[test]
    fn pitch_is_clamped() {
        let mut v = PlayerView::new();
        v.rotate(0.0, 10.0);
        assert!(v.pitch() <= MAX_PITCH + 1e-6);
        v.rotate(0.0, -20.0);
        assert!(v.pitch() >= -MAX_PITCH - 1e-6);
    }

    #[test]
    fn right_is_horizontal_and_orthogonal() {
        let mut v = PlayerView::new();
        v.rotate(0.7, 0.5);
        assert_eq!(v.right().y, 0.0);
        assert!(v.right().dot(v.forward()).abs() < 1e-5);
        assert!((v.right().length() - 1.0).abs() < 1e-5);
        // default: forward +X, right +Z (matches the old D = +Z axis)
        assert!((PlayerView::new().right() - Vec3::Z).length() < 1e-6);
    }

    #[test]
    fn forward_point_projects_to_ndc_center() {
        let mut v = PlayerView::new();
        v.rotate(1.2, 0.4);
        let eye = Vec3::new(3.0, 4.0, 5.0);
        let p = v.view_proj(eye) * (eye + v.forward() * 10.0).extend(1.0);
        assert!((p.x / p.w).abs() < 1e-4 && (p.y / p.w).abs() < 1e-4);
    }

    #[test]
    fn frustum_sees_ahead_not_behind() {
        let v = PlayerView::new();
        let f = v.frustum(Vec3::ZERO);
        assert!(f.intersects_aabb(Vec3::new(20.0, -1.0, -1.0), Vec3::new(22.0, 1.0, 1.0)));
        assert!(!f.intersects_aabb(Vec3::new(-22.0, -1.0, -1.0), Vec3::new(-20.0, 1.0, 1.0)));
    }
}
