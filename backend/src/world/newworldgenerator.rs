use fastrand::Rng;
use glam::{vec3, IVec3, U16Vec3};
use log::info;

use super::{BlockPos, BlockType, Chunk, ChunkPos, WorldGenerator, CHUNK_SIZE};

#[derive(Debug)]
pub struct GeneratorV0 {
    rng: Rng,
}

impl GeneratorV0 {
    pub fn new(rng: Rng) -> Self {
        Self { rng }
    }
}

impl WorldGenerator for GeneratorV0 {
    fn generate(&mut self, chunk_position: ChunkPos) -> Chunk {
        if chunk_position.as_vec().y < -2 || chunk_position.as_vec().y > 2 {
            return Chunk::empty();
        }
        let mut res = Chunk::new(false);
        let amplitude = 10.0;
        let frequency = 0.03;

        let first_block =
            chunk_position.get_center_block_pos().as_vec() - (CHUNK_SIZE as i32 / 2) * IVec3::ONE;
        for block_pos in chunk_position.iter_block_pos() {
            let pos = block_pos.as_vec();
            let offset: U16Vec3 = (pos - first_block).as_u16vec3();
            // info!("{} {} {}", pos.x, pos.y, pos.z);
            let h = (amplitude
                * ((frequency * pos.x as f32).sin() + (frequency * pos.z as f32).sin()))
            .max(-3.0);
            let yf = block_pos.as_vec3().y;
            if yf <= h {
                let block = if h - yf <= 1.0 {
                    BlockType::Grass
                } else if h - yf <= 3.0 {
                    BlockType::Dirt
                } else if h - yf <= 7.0 {
                    BlockType::Stone
                } else {
                    BlockType::Empty
                };
                res.set(offset, block);
            }
        }
        res
    }
}
