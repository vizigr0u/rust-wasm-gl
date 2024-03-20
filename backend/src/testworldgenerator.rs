use fastrand::Rng;
use glam::{U16Vec3, UVec3};

use crate::chunk::{BlockType, Chunk, ChunkGenerator, CHUNK_SIZE};

#[derive(Debug)]
pub struct TestGenerator {
    pub rng: Rng,
}

fn dirt_with_grass_on_top(world_position: UVec3, rng: &mut Rng) -> Chunk {
    let min_y = 3;
    let max_y = 5;
    let mut res = Chunk::empty(world_position);
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

impl ChunkGenerator for TestGenerator {
    fn generate(&mut self, world_position: &UVec3) -> Chunk {
        let world_position = *world_position;
        match world_position.y {
            0 => Chunk::random(world_position, &mut self.rng),
            1 | 3..=4 => Chunk::plain(world_position, BlockType::Stone),
            2 => Chunk::plain(world_position, BlockType::Lava),
            5 => dirt_with_grass_on_top(world_position, &mut self.rng),
            _ => Chunk::empty(world_position),
        }
    }
}
