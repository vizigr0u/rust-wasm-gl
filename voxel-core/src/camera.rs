//! Orbit camera math (no input plumbing).
use glam::{Mat4, Quat, Vec3};

pub const MAX_CAMERA_DISTANCE: f32 = 800.0;
pub const MIN_CAMERA_DISTANCE: f32 = 1.1;
const WHEEL_ZOOM_FACTOR: f32 = 1.15;
const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;
const FOV_Y_DEGREES: f32 = 45.0;
const NEAR_VIEW: f32 = 0.1;
const FAR_VIEW: f32 = 1000.0;

/// Clip-space z convention of the graphics API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipZ {
    /// OpenGL / WebGL2
    NegOneToOne,
    /// WebGPU
    ZeroToOne,
}

#[derive(Debug, Clone)]
pub struct OrbitCamera {
    pub target: Vec3,
    pub offset: Vec3,
    aspect: f32,
}

/// Rotate `offset` up/down by `pitch` radians (positive lowers the camera),
/// keeping elevation within +/- MAX_PITCH so `look_at` with up=Y never degenerates.
pub fn apply_pitch(offset: Vec3, pitch: f32) -> Vec3 {
    let len = offset.length();
    if len <= f32::EPSILON {
        return offset;
    }
    let forward = offset / len;
    let elevation = forward.y.clamp(-1.0, 1.0).asin();
    let new_elevation = (elevation - pitch).clamp(-MAX_PITCH, MAX_PITCH);
    let effective = elevation - new_elevation;
    let horizontal = Vec3::new(forward.x, 0.0, forward.z);
    // Looking straight along Y: no defined right axis, nudge using +Z.
    let horizontal = if horizontal.length_squared() < 1e-12 {
        Vec3::Z
    } else {
        horizontal.normalize()
    };
    let right = Vec3::Y.cross(horizontal);
    Quat::from_axis_angle(right, effective) * offset
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self::new(Vec3::new(0.0, 0.0, 10.0))
    }
}

impl OrbitCamera {
    pub fn new(offset: Vec3) -> Self {
        Self {
            target: Vec3::ZERO,
            offset: offset.clamp_length(MIN_CAMERA_DISTANCE, MAX_CAMERA_DISTANCE),
            aspect: 800.0 / 600.0,
        }
    }

    pub fn aspect(&self) -> f32 {
        self.aspect
    }

    /// Ignores degenerate sizes. Returns true if the aspect changed.
    pub fn set_viewport_size(&mut self, width: u32, height: u32) -> bool {
        if width == 0 || height == 0 {
            return false;
        }
        let aspect = width as f32 / height as f32;
        if (aspect - self.aspect).abs() > f32::EPSILON {
            self.aspect = aspect;
            return true;
        }
        false
    }

    /// Yaw around Y then pitch (radians), pole-clamped.
    pub fn orbit(&mut self, yaw: f32, pitch: f32) {
        let yawed = Quat::from_rotation_y(yaw) * self.offset;
        self.offset = apply_pitch(yawed, pitch);
    }

    /// Positive `dy` zooms out.
    pub fn zoom(&mut self, dy: f32) {
        let factor = if dy >= 0.0 {
            WHEEL_ZOOM_FACTOR
        } else {
            1.0 / WHEEL_ZOOM_FACTOR
        };
        self.offset = (self.offset * factor).clamp_length(MIN_CAMERA_DISTANCE, MAX_CAMERA_DISTANCE);
    }

    pub fn view(&self) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(self.target + self.offset, self.target, Vec3::Y)
    }

    pub fn projection(&self, clip_z: ClipZ) -> Mat4 {
        projection(self.aspect, clip_z)
    }
}

pub fn projection(aspect: f32, clip_z: ClipZ) -> Mat4 {
    let fov = f32::to_radians(FOV_Y_DEGREES);
    match clip_z {
        ClipZ::NegOneToOne => {
            glam::camera::rh::proj::opengl::perspective(fov, aspect, NEAR_VIEW, FAR_VIEW)
        }
        ClipZ::ZeroToOne => {
            glam::camera::rh::proj::directx::perspective(fov, aspect, NEAR_VIEW, FAR_VIEW)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_camera_has_finite_offset() {
        let c = OrbitCamera::default();
        assert!(c.offset.is_finite());
        assert!(c.offset.length() >= MIN_CAMERA_DISTANCE);
    }

    #[test]
    fn set_viewport_size_updates_aspect() {
        let mut c = OrbitCamera::default();
        assert!(c.set_viewport_size(1920, 1080));
        assert!((c.aspect() - 16.0 / 9.0).abs() < 1e-5);
        assert!(!c.set_viewport_size(0, 0));
        assert!((c.aspect() - 16.0 / 9.0).abs() < 1e-5);
    }

    #[test]
    fn pitch_never_crosses_pole() {
        let mut o = Vec3::new(0.0, 1.0, 10.0);
        for _ in 0..50 {
            o = apply_pitch(o, -1.0);
            assert!(o.is_finite());
            assert!(o.y / o.length() <= MAX_PITCH.sin() + 1e-4);
        }
        for _ in 0..100 {
            o = apply_pitch(o, 1.0);
            assert!(o.is_finite());
            assert!(o.y / o.length() >= -MAX_PITCH.sin() - 1e-4);
        }
    }

    #[test]
    fn pitch_preserves_length() {
        let o = Vec3::new(3.0, 4.0, 5.0);
        assert!((apply_pitch(o, 0.3).length() - o.length()).abs() < 1e-3);
    }

    #[test]
    fn zero_to_one_matches_gl_xy_and_remaps_z() {
        let gl = projection(1.5, ClipZ::NegOneToOne);
        let wg = projection(1.5, ClipZ::ZeroToOne);
        for p in [Vec3::new(1.0, 2.0, -5.0), Vec3::new(-3.0, 0.5, -100.0)] {
            let a = gl.project_point3(p);
            let b = wg.project_point3(p);
            assert!((a.x - b.x).abs() < 1e-4 && (a.y - b.y).abs() < 1e-4);
            assert!((b.z - (a.z + 1.0) / 2.0).abs() < 1e-4);
        }
    }

    // ---- A4 snapshots ----
    use crate::testutil::{fmt_mat4, fmt_vec3};

    #[test]
    fn offset_clamped() {
        assert!((OrbitCamera::new(Vec3::splat(0.0001)).offset.length() - MIN_CAMERA_DISTANCE).abs() < 1e-4);
        assert!((OrbitCamera::new(Vec3::splat(1e6)).offset.length() - MAX_CAMERA_DISTANCE).abs() < 1e-2);
        let mut c = OrbitCamera::new(Vec3::new(0.0, 0.0, MAX_CAMERA_DISTANCE));
        c.zoom(1.0);
        assert!((c.offset.length() - MAX_CAMERA_DISTANCE).abs() < 1e-2);
        let mut c = OrbitCamera::new(Vec3::new(0.0, 0.0, MIN_CAMERA_DISTANCE));
        c.zoom(-1.0);
        assert!((c.offset.length() - MIN_CAMERA_DISTANCE).abs() < 1e-4);
    }

    /// Seven cameras looking at the centre of block (0,0,0): 4 faces, top/bottom tilted, 3/4.
    pub fn single_block_cameras() -> [(&'static str, Vec3); 7] {
        [
            ("pos_x", Vec3::new(3.0, 0.0, 0.0)),
            ("neg_x", Vec3::new(-3.0, 0.0, 0.0)),
            ("pos_z", Vec3::new(0.0, 0.0, 3.0)),
            ("neg_z", Vec3::new(0.0, 0.0, -3.0)),
            ("top", Vec3::new(0.3, 3.0, 0.3)),
            ("bottom", Vec3::new(0.3, -3.0, 0.3)),
            ("three_quarter", Vec3::new(2.0, 1.5, 2.0)),
        ]
    }

    #[test]
    fn view_snapshots() {
        let mut out = vec![("default".to_string(), fmt_mat4(OrbitCamera::default().view()))];
        for (name, off) in single_block_cameras() {
            let mut c = OrbitCamera::new(off);
            c.target = Vec3::splat(0.5);
            out.push((name.to_string(), fmt_mat4(c.view())));
        }
        insta::assert_yaml_snapshot!(out);
    }

    #[test]
    fn projection_snapshot_gl() {
        insta::assert_yaml_snapshot!(fmt_mat4(projection(800.0 / 600.0, ClipZ::NegOneToOne)));
    }

    #[test]
    fn orbit_zoom_sequence_snapshot() {
        let mut c = OrbitCamera::default();
        let mut out = vec![fmt_vec3(c.offset)];
        for (yaw, pitch, dy) in [(0.5, 0.2, 1.0), (-1.0, 0.4, -1.0), (2.0, -0.3, 1.0), (0.1, 0.1, -1.0)] {
            c.orbit(yaw, pitch);
            c.zoom(dy);
            out.push(fmt_vec3(c.offset));
        }
        insta::assert_yaml_snapshot!(out);
    }

    #[test]
    fn pole_clamp_snapshot() {
        let mut c = OrbitCamera::new(Vec3::new(0.0, 1.0, 10.0));
        c.orbit(0.0, -10.0);
        let top = c.offset;
        c.orbit(0.0, 20.0);
        insta::assert_yaml_snapshot!((fmt_vec3(top), fmt_vec3(c.offset)));
        assert!(top.is_finite() && c.offset.is_finite());
    }
}
