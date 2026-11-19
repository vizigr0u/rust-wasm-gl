use fastrand::Rng;

use super::{Chunk, ChunkPos};

/// Deterministic RNG for `(seed, pos)`: generation must not depend on load order.
pub fn rng_for(seed: u64, pos: ChunkPos) -> Rng {
    let p = pos.as_vec();
    let mut h = seed ^ 0x9E37_79B9_7F4A_7C15;
    for v in [p.x, p.y, p.z] {
        h = (h ^ (v as u32 as u64)).wrapping_mul(0x0100_0000_01B3);
        h ^= h >> 29;
    }
    Rng::with_seed(h)
}

/// `&self` + pure function of `(seed, pos)`: allows parallel/worker generation.
pub trait WorldGenerator {
    fn generate(&self, chunk_pos: ChunkPos) -> Chunk;
}

/// One grass block at block (0,0,0), everything else air. Test scenes only.
#[derive(Debug, Default)]
pub struct SingleBlockGenerator;

impl WorldGenerator for SingleBlockGenerator {
    fn generate(&self, chunk_pos: ChunkPos) -> Chunk {
        let mut chunk = Chunk::empty();
        if chunk_pos.as_vec() == glam::IVec3::ZERO {
            chunk.set(glam::U16Vec3::ZERO, super::BlockType::Grass);
            chunk.compact();
        }
        chunk
    }
}

pub struct EmptyWorldGenerator;

impl WorldGenerator for EmptyWorldGenerator {
    fn generate(&self, _chunk_pos: ChunkPos) -> Chunk {
        Chunk::default()
    }
}

#[derive(Debug)]
pub struct RandomChunkGenerator {
    pub seed: u64,
}

impl WorldGenerator for RandomChunkGenerator {
    fn generate(&self, chunk_pos: ChunkPos) -> Chunk {
        Chunk::random(&mut rng_for(self.seed, chunk_pos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::IVec3;

    #[test]
    fn single_block_only_in_origin_chunk() {
        let g = SingleBlockGenerator;
        let count = |p: IVec3| {
            g.generate(p.into())
                .iter_blocks()
                .filter(|b| *b != super::super::BlockType::Empty)
                .count()
        };
        assert_eq!(count(IVec3::ZERO), 1);
        assert_eq!(count(IVec3::new(1, 0, 0)), 0);
        assert!(g.generate(IVec3::new(0, 1, 0).into()).is_empty());
    }
}
