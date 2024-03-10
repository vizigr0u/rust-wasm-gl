use glam::{Mat4, Vec3};
use log::{info, warn};

use crate::{inputsystem::InputEventType, time::Time};

const DEBUG_INPUTS: bool = false;

const CAMERA_SPEED: f32 = 0.08;

pub struct Camera {
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,

    pub projection: Mat4,
    pub look_at: Mat4,

    right: Vec3,
    velocity: Vec3,
    clicked: bool,
}

impl Camera {
    pub fn new(position: Vec3, forward: Vec3, up: Vec3) -> Self {
        Self {
            position,
            forward,
            up,
            right: forward.cross(up),
            projection: Mat4::perspective_rh_gl(f32::to_radians(45.0), 800.0 / 600.0, 0.1, 100.0),
            look_at: Mat4::IDENTITY,
            velocity: Vec3::ZERO,
            clicked: false,
        }
    }

    pub fn update(&mut self, time: &Time) {
        let velocity = self.forward * self.velocity.z + self.right * self.velocity.x;
        let velocity = velocity.normalize_or_zero() * CAMERA_SPEED;
        self.position += velocity * (time.delta_time() as f32 / 5.0);
        self.look_at = Mat4::look_to_rh(self.position, self.forward, self.up);
    }

    pub fn handle_input(&mut self, ev: &InputEventType) {
        match ev {
            InputEventType::MouseDown(e) => {
                if DEBUG_INPUTS {
                    warn!("MOUSE DOWN {}, {}", e.offset_x(), e.offset_y())
                }
                self.clicked = true;
            }
            InputEventType::MouseUp(e) => {
                if DEBUG_INPUTS {
                    warn!("MOUSE UP {}, {}", e.offset_x(), e.offset_y())
                }
                self.clicked = false;
            }
            InputEventType::MouseMove(e) => {
                if DEBUG_INPUTS {
                    info!("MOUSE MOVE {}, {}", e.offset_x(), e.offset_y())
                }
            }
            InputEventType::KeyDown(e) => {
                if DEBUG_INPUTS {
                    warn!("KEY DOWN: {}", e.code())
                }
                match e.code().as_str() {
                    "KeyW" => self.velocity.z = -1.0,
                    "KeyS" => self.velocity.z = 1.0,
                    "KeyA" => self.velocity.x = 1.0,
                    "KeyD" => self.velocity.x = -1.0,
                    _ => {}
                }
            }
            InputEventType::KeyUp(e) => {
                if DEBUG_INPUTS {
                    warn!("KEY UP: {}", e.code())
                }
                match e.code().as_str() {
                    "KeyW" => {
                        if self.velocity.z < 0.0 {
                            self.velocity.z = 0.0;
                        }
                    }
                    "KeyS" => {
                        if self.velocity.z > 0.0 {
                            self.velocity.z = 0.0;
                        }
                    }
                    "KeyA" => {
                        if self.velocity.x > 0.0 {
                            self.velocity.x = 0.0;
                        }
                    }
                    "KeyD" => {
                        if self.velocity.x < 0.0 {
                            self.velocity.x = 0.0;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
