use glam::IVec3;
use glam::Mat4;
use glam::Quat;
use glam::Vec3;
use glow::HasContext;
use glow::WebTextureKey;

use crate::core::position::ChunkPos;
use crate::core::position::WorldPos;
use crate::core::position::CHUNK_SIZE;
use crate::core::Time;
use crate::core::Transform;
use crate::graphics::TextureType;
use crate::graphics::UniformTypes;
use crate::objects::Camera;

#[derive(Debug)]
pub struct GameObject {
    position: WorldPos,
    player_chunk: ChunkPos,
    rotation: Quat,
    scale: Vec3,
    velocity: Vec3,

    transform: Mat4,
    transform_dirty: bool,
}

impl GameObject {
    pub fn new() -> Self {
        GameObject {
            position: WorldPos::ZERO,
            player_chunk: IVec3::ZERO.into(),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            transform: Mat4::IDENTITY,
            transform_dirty: true,
            velocity: Vec3::ZERO,
        }
    }

    pub fn update(&mut self, time: &Time, player_chunk: ChunkPos) {
        if time.delta_time() > 0.0 && self.velocity != Vec3::ZERO {
            self.position += self.velocity * time.delta_time() as f32 * 0.001;
            self.transform_dirty = true;
        }
        self.set_current_player_chunk(player_chunk);
        if self.transform_dirty {
            let screen_pos = self.position.to_screen_pos(self.player_chunk);
            self.transform =
                Mat4::from_scale_rotation_translation(self.scale, self.rotation, screen_pos);
        }
    }

    fn set_current_player_chunk(&mut self, chunk: ChunkPos) {
        if chunk != self.player_chunk {
            self.player_chunk = chunk;
            self.transform_dirty = true;
        }
    }
}

impl Transform for GameObject {
    fn get_position(&self) -> WorldPos {
        self.position
    }

    fn set_position(&mut self, position: WorldPos) {
        self.position = position;
        self.transform_dirty = true;
    }

    fn get_rotation(&self) -> Quat {
        self.rotation
    }

    fn set_rotation(&mut self, rotation: Quat) {
        self.rotation = rotation;
        self.transform_dirty = true;
    }

    fn set_scale(&mut self, scale: Vec3) {
        self.scale = scale;
        self.transform_dirty = true;
    }

    fn get_scale(&self) -> Vec3 {
        self.scale
    }

    fn get_velocity(&self) -> Vec3 {
        self.velocity
    }

    fn set_velocity(&mut self, velocity: Vec3) {
        self.velocity = velocity;
    }
}
