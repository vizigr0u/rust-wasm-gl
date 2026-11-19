use glam::{Mat4, Vec3};
use voxel_core::camera::OrbitCamera;

use crate::core::{HandleInputs, InputEventType, InputState, Time};

const DEFAULT_MOUSE_SENSITIVITY: f32 = 0.2;

/// Thin web wrapper: input plumbing around the pure `OrbitCamera`.
#[derive(Debug)]
pub struct Camera {
    pub orbit: OrbitCamera,
    pub projection: Mat4,
    pub look_at: Mat4,
    mouse_sensitivity: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(Vec3::new(0.0, 0.0, 10.0))
    }
}

impl Camera {
    pub fn new(target_offset: Vec3) -> Self {
        let orbit = OrbitCamera::new(target_offset);
        Self {
            projection: orbit.projection(crate::render::CLIP_Z),
            look_at: Mat4::IDENTITY,
            orbit,
            mouse_sensitivity: DEFAULT_MOUSE_SENSITIVITY,
        }
    }

    /// Rebuild the projection for a new drawing-buffer size. Ignores degenerate sizes.
    pub fn set_viewport_size(&mut self, width: u32, height: u32) {
        if self.orbit.set_viewport_size(width, height) {
            self.projection = self.orbit.projection(crate::render::CLIP_Z);
        }
    }

    pub fn update(&mut self, _time: &Time) {
        self.look_at = self.orbit.view();
    }
}

impl HandleInputs for Camera {
    fn handle_inputs(&mut self, inputs: &InputState) {
        for e in inputs.get_events() {
            match e {
                InputEventType::MouseMove(e) if e.buttons() % 2 == 1 => {
                    let speed = self.mouse_sensitivity.to_radians();
                    self.orbit.orbit(
                        -(e.movement_x() as f32) * speed,
                        -(e.movement_y() as f32) * speed,
                    );
                }
                InputEventType::MouseWheel(e) => self.orbit.zoom(e.delta_y() as f32),
                _ => (),
            }
        }
    }
}
