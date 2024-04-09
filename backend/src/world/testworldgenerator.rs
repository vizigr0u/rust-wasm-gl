use fastrand::Rng;
use glam::{IVec3, U16Vec3};
use log::info;

use super::{BlockType, Chunk, ChunkPos, WorldGenerator, CHUNK_SIZE};

#[derive(Debug)]
pub struct TestGenerator {
    pub rng: Rng,
}

fn dirt_with_grass_on_top(rng: &mut Rng) -> Chunk {
    let min_dirt_height = 1;
    let max_dirt_height = 4;
    let min_grass_y = 14;
    let max_grass_y = 16;
    let mut res = Chunk::new(false);
    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let max_y = rng.u8(min_grass_y..=max_grass_y as u8) as u16;
            let dirt_height = rng.u8(min_dirt_height..max_dirt_height as u8) as u16;
            let min_dirt_y = max_y - 1 - dirt_height;
            for y in 0..max_y {
                let block = if y == max_y - 1 {
                    BlockType::Grass
                } else if y >= min_dirt_y {
                    BlockType::Dirt
                } else {
                    BlockType::Stone
                };
                res.set(U16Vec3::new(x as _, y, z as _), block);
            }
        }
    }
    res
}

impl WorldGenerator for TestGenerator {
    fn generate(&mut self, chunk_position: ChunkPos) -> Chunk {
        match chunk_position.as_vec().y {
            // -5 => Chunk::random(&mut self.rng),
            // -1 | -4 | -2 => Chunk::plain(BlockType::Stone),
            // -3 => Chunk::plain(BlockType::Lava),
            // -2 => Chunk::plain(BlockType::Stone),
            -1 => dirt_with_grass_on_top(&mut self.rng),
            _ => Chunk::empty(),
        }
    }
}
