use glam::{IVec3, Quat, Vec3};

use super::position::{WorldPos, CHUNK_SIZE};

pub trait Transform {
    fn get_position(&self) -> WorldPos;
    fn set_position(&mut self, position: WorldPos);
    fn get_rotation(&self) -> Quat;
    fn set_rotation(&mut self, rotation: Quat);
    fn get_scale(&self) -> Vec3;
    fn set_scale(&mut self, scale: Vec3);
    fn get_velocity(&self) -> Vec3;
    fn set_velocity(&mut self, velocity: Vec3);

    fn get_block_position(&self) -> IVec3 {
        self.get_position().offset().as_ivec3()
            + self.get_position().chunk_pos().as_vec() * CHUNK_SIZE as i32
    }

    fn get_chunk_position(&self) -> IVec3 {
        self.get_block_position() / CHUNK_SIZE as i32
    }
}
