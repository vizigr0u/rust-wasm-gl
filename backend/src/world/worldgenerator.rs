use fastrand::Rng;
use glam::IVec3;

use super::{Chunk, ChunkPage};

pub trait WorldGenerator {
    fn generate(&mut self, chunk_pos: IVec3) -> Chunk;
}

pub struct EmptyWorldGenerator;

impl WorldGenerator for EmptyWorldGenerator {
    fn generate(&mut self, _chunk_pos: IVec3) -> Chunk {
        Chunk::default()
    }
}

#[derive(Debug)]
pub struct RandomChunkGenerator {
    pub rng: Rng,
}

impl WorldGenerator for RandomChunkGenerator {
    fn generate(&mut self, _chunk_pos: IVec3) -> Chunk {
        Chunk::random(&mut self.rng)
    }
}
