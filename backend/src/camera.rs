use glam::{Mat4, Quat, Vec3};
use log::{info, warn};

use crate::{
    inputsystem::{HandleInputs, InputEventType},
    time::Time,
};

const DEBUG_INPUTS: bool = false;

const DEFAULT_CAMERA_SPEED: f32 = 0.08;
const MIN_CAMERA_SPEED: f32 = 0.001;
const MAX_CAMERA_SPEED: f32 = 1.0;
const CAMERA_SPEED_FACTOR: f32 = 1.5;

const DEFAULT_MOUSE_SENSITIVITY: f32 = 0.2;
const MAX_MOUSE_SENSITIVITY: f32 = 1.0;
const MIN_MOUSE_SENSITIVITY: f32 = 0.01;
const MOUSE_SENSITIVITY_FACTOR: f32 = 0.5;

const NEAR_VIEW: f32 = 0.1;
const FAR_VIEW: f32 = 300.0;

pub struct Camera {
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,

    pub projection: Mat4,
    pub look_at: Mat4,

    camera_speed: f32,
    mouse_sensitivity: f32,
    right: Vec3,
    velocity: Vec3,
}

impl Camera {
    pub fn new(position: Vec3, forward: Vec3, up: Vec3) -> Self {
        Self {
            position,
            forward,
            up,
            right: forward.cross(up),
            projection: Mat4::perspective_rh_gl(
                f32::to_radians(45.0),
                800.0 / 600.0,
                NEAR_VIEW,
                FAR_VIEW,
            ),
            look_at: Mat4::IDENTITY,
            velocity: Vec3::ZERO,
            camera_speed: DEFAULT_CAMERA_SPEED,
            mouse_sensitivity: DEFAULT_MOUSE_SENSITIVITY,
        }
    }

    pub fn update(&mut self, time: &Time) {
        let velocity = self.forward * self.velocity.z
            + self.up * self.velocity.y
            + self.right * self.velocity.x;
        let velocity = velocity.normalize_or_zero() * self.camera_speed;
        self.position += velocity * (time.delta_time() as f32 / 5.0);
        self.look_at = Mat4::look_to_rh(self.position, self.forward, self.up);
    }
}

impl HandleInputs for Camera {
    fn handle_inputs(&mut self, inputs: &crate::inputsystem::InputState) {
        self.velocity = Vec3::ZERO;
        if inputs.is_key_down("KeyW") {
            self.velocity.z = -1.0;
        }
        if inputs.is_key_down("KeyS") {
            self.velocity.z = 1.0;
        }
        if inputs.is_key_down("KeyA") {
            self.velocity.x = 1.0;
        }
        if inputs.is_key_down("KeyD") {
            self.velocity.x = -1.0;
        }
        if inputs.is_key_down("KeyC") {
            info!(
                "Camera position: {:?}, forward: {:?}, up: {:?}",
                self.position, self.forward, self.up
            );
        }
        for e in inputs.get_events() {
            match e {
                InputEventType::MouseMove(_) => {
                    if inputs.is_mouse_down() {
                        let speed = self.mouse_sensitivity;
                        let d = inputs.get_mouse_delta();
                        let yaw = -d.x * speed.to_radians();
                        let pitch = -d.y * speed.to_radians();

                        let yaw_rotation = Quat::from_rotation_y(yaw);
                        let forward = yaw_rotation.mul_vec3(self.forward).normalize();
                        let right = yaw_rotation.mul_vec3(self.right).normalize();

                        let pitch_rotation = Quat::from_axis_angle(right, pitch);
                        let forward = pitch_rotation.mul_vec3(forward).normalize();

                        self.forward = forward;
                        self.right = right;
                    }
                }
                InputEventType::MouseWheel(e) => {
                    let factor = if e.delta_y() <= 0.0 {
                        CAMERA_SPEED_FACTOR
                    } else {
                        1.0 / CAMERA_SPEED_FACTOR
                    };
                    self.camera_speed =
                        (self.camera_speed * factor).clamp(MIN_CAMERA_SPEED, MAX_CAMERA_SPEED);
                    info!("Camera speed changed to: {}", self.camera_speed);
                }
                _ => (),
            }
        }
    }
}
