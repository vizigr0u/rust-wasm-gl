use fastrand::Rng;
use glam::{IVec3, U16Vec3};
use log::info;

use super::{BlockType, Chunk, WorldGenerator, CHUNK_SIZE};

#[derive(Debug)]
pub struct TestGenerator {
    pub rng: Rng,
}

fn dirt_with_grass_on_top(rng: &mut Rng) -> Chunk {
    let min_y = 14;
    let max_y = 16;
    let mut res = Chunk::new(false);
    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let max_y = rng.u8(min_y..max_y as u8) as u16;
            for y in 0..max_y {
                let block = if y == max_y - 1 {
                    BlockType::Grass
                } else {
                    BlockType::Dirt
                };
                res.set(U16Vec3::new(x as _, y, z as _), block);
            }
        }
    }
    res
}

impl WorldGenerator for TestGenerator {
    fn generate(&mut self, chunk_position: IVec3) -> Chunk {
        match chunk_position.y {
            // -5 => Chunk::random(&mut self.rng),
            // -1 | -4 | -2 => Chunk::plain(BlockType::Stone),
            // -3 => Chunk::plain(BlockType::Lava),
            -1 => Chunk::plain(BlockType::Stone),
            0 => dirt_with_grass_on_top(&mut self.rng),
            _ => Chunk::empty(),
        }
    }
}
