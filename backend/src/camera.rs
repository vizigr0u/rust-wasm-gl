use glam::{Mat4, Vec3};

pub struct Camera {
    pub position: Vec3,
    pub direction: Vec3,
    pub up: Vec3,

    pub projection: Mat4,
    pub look_at: Mat4,
}
