
use glam::{Mat4, Quat, Vec3};

use crate::core::{HandleInputs, InputEventType, InputState, Time};

// const DEBUG_INPUTS: bool = false;

// const DEFAULT_CAMERA_SPEED: f32 = 0.08;
// const MIN_CAMERA_SPEED: f32 = 0.001;
// const MAX_CAMERA_SPEED: f32 = 1.0;
// const CAMERA_SPEED_FACTOR: f32 = 1.5;

const DEFAULT_MOUSE_SENSITIVITY: f32 = 0.2;
const MAX_CAMERA_DISTANCE: f32 = 800.0;
const MIN_CAMERA_DISTANCE: f32 = 1.1;
const WHEEL_ZOOM_FACTOR: f32 = 1.15;
// const MAX_MOUSE_SENSITIVITY: f32 = 1.0;
// const MIN_MOUSE_SENSITIVITY: f32 = 0.01;
// const MOUSE_SENSITIVITY_FACTOR: f32 = 0.5;

const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;

const NEAR_VIEW: f32 = 0.1;
const FAR_VIEW: f32 = 1000.0;

#[derive(Debug)]
pub struct Camera {
    // pub position: Vec3,
    // pub forward: Vec3,
    // pub up: Vec3,
    pub target_offset: Vec3,
    pub target: Vec3,
    pub projection: Mat4,
    pub look_at: Mat4,
    mouse_sensitivity: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(Vec3::new(0.0, 0.0, 10.0))
    }
}

/// Rotate `offset` up/down by `pitch` radians (positive lowers the camera),
/// keeping elevation within +/- MAX_PITCH so `look_at` with up=Y never degenerates.
fn apply_pitch(offset: Vec3, pitch: f32) -> Vec3 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_camera_has_finite_offset() {
        let c = Camera::default();
        assert!(c.target_offset.is_finite());
        assert!(c.target_offset.length() >= MIN_CAMERA_DISTANCE);
    }

    #[test]
    fn pitch_never_crosses_pole() {
        let mut o = Vec3::new(0.0, 1.0, 10.0);
        for _ in 0..50 {
            o = apply_pitch(o, -1.0); // keep raising
            assert!(o.is_finite());
            assert!(o.y / o.length() <= MAX_PITCH.sin() + 1e-4);
        }
        for _ in 0..100 {
            o = apply_pitch(o, 1.0); // keep lowering
            assert!(o.is_finite());
            assert!(o.y / o.length() >= -MAX_PITCH.sin() - 1e-4);
        }
    }

    #[test]
    fn pitch_preserves_length() {
        let o = Vec3::new(3.0, 4.0, 5.0);
        assert!((apply_pitch(o, 0.3).length() - o.length()).abs() < 1e-3);
    }
}

impl Camera {
    pub fn new(target_offset: Vec3) -> Self {
        Self {
            target: Vec3::ZERO,
            target_offset: Vec3::clamp_length(
                target_offset,
                MIN_CAMERA_DISTANCE,
                MAX_CAMERA_DISTANCE,
            ),
            projection: glam::camera::rh::proj::opengl::perspective(
                f32::to_radians(45.0),
                800.0 / 600.0,
                NEAR_VIEW,
                FAR_VIEW,
            ),
            look_at: Mat4::IDENTITY,
            mouse_sensitivity: DEFAULT_MOUSE_SENSITIVITY,
        }
    }

    pub fn update(&mut self, _time: &Time) {
        // let velocity = velocity.normalize_or_zero() * self.camera_speed;
        // self.position += velocity * (time.delta_time() as f32 / 5.0);
        let position = self.target + self.target_offset;
        self.look_at = glam::camera::rh::view::look_at_mat4(position, self.target, Vec3::Y);
    }
}

impl HandleInputs for Camera {
    fn handle_inputs(&mut self, inputs: &InputState) {
        // self.velocity = Vec3::ZERO;
        // if inputs.is_key_down("KeyW") {
        //     self.velocity.z = -1.0;
        // }
        // if inputs.is_key_down("KeyS") {
        //     self.velocity.z = 1.0;
        // }
        // if inputs.is_key_down("KeyA") {
        //     self.velocity.x = 1.0;
        // }
        // if inputs.is_key_down("KeyD") {
        //     self.velocity.x = -1.0;
        // }
        for e in inputs.get_events() {
            match e {
                InputEventType::MouseMove(e) => {
                    if e.buttons() % 2 == 1 {
                        let speed = self.mouse_sensitivity;
                        let yaw = -(e.movement_x() as f32) * speed.to_radians();
                        let pitch = -(e.movement_y() as f32) * speed.to_radians();

                        let yaw_rotation = Quat::from_rotation_y(yaw);
                        // Yaw first, then pitch with the elevation clamped so the
                        // camera never reaches or crosses the pole.
                        let yawed = yaw_rotation * self.target_offset;
                        self.target_offset = apply_pitch(yawed, pitch);
                    }
                }
                InputEventType::MouseWheel(e) => {
                    let factor = if e.delta_y() >= 0.0 {
                        WHEEL_ZOOM_FACTOR
                    } else {
                        1.0 / WHEEL_ZOOM_FACTOR
                    };
                    self.target_offset = Vec3::clamp_length(
                        self.target_offset * factor,
                        MIN_CAMERA_DISTANCE,
                        MAX_CAMERA_DISTANCE,
                    );
                    // info!("Camera offset changed to: {}", self.target_offset);
                }
                _ => (),
            }
        }
    }
}
