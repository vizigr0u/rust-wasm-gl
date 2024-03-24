use core::num;

use glam::{Mat4, Quat, Vec3};
use log::info;

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
        Self::new(Vec3::ZERO)
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
            projection: Mat4::perspective_rh_gl(
                f32::to_radians(45.0),
                800.0 / 600.0,
                NEAR_VIEW,
                FAR_VIEW,
            ),
            look_at: Mat4::IDENTITY,
            mouse_sensitivity: DEFAULT_MOUSE_SENSITIVITY,
        }
    }

    pub fn update(&mut self, time: &Time) {
        // let velocity = velocity.normalize_or_zero() * self.camera_speed;
        // self.position += velocity * (time.delta_time() as f32 / 5.0);
        let position = self.target + self.target_offset;
        self.look_at = Mat4::look_at_rh(position, self.target, Vec3::Y);
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
                        let d = inputs.get_mouse_delta();
                        let yaw = -d.x * speed.to_radians();
                        let pitch = -d.y * speed.to_radians();

                        let yaw_rotation = Quat::from_rotation_y(yaw);
                        let forward = self.target_offset.normalize();
                        let pitch_rotation = if forward.dot(Vec3::Y).abs() > 0.9999 {
                            Quat::IDENTITY
                        } else {
                            let right = Vec3::Y.cross(forward).normalize();
                            Quat::from_axis_angle(right, pitch)
                        };
                        self.target_offset = pitch_rotation * yaw_rotation * self.target_offset;
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
