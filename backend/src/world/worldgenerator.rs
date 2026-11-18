use fastrand::Rng;

use super::{Chunk, ChunkPos};

pub trait WorldGenerator {
    fn generate(&mut self, chunk_pos: ChunkPos) -> Chunk;
}

pub struct EmptyWorldGenerator;

impl WorldGenerator for EmptyWorldGenerator {
    fn generate(&mut self, _chunk_pos: ChunkPos) -> Chunk {
        Chunk::default()
    }
}

#[derive(Debug)]
pub struct RandomChunkGenerator {
    pub rng: Rng,
}

impl WorldGenerator for RandomChunkGenerator {
    fn generate(&mut self, _chunk_pos: ChunkPos) -> Chunk {
        Chunk::random(&mut self.rng)
    }
}
