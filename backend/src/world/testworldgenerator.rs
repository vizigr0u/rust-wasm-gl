use fastrand::Rng;
use glam::{IVec3, U16Vec3};
use log::info;

use super::{BlockType, Chunk, ChunkGenerator, CHUNK_SIZE};

#[derive(Debug)]
pub struct TestGenerator {
    pub rng: Rng,
}

fn dirt_with_grass_on_top(world_position: IVec3, rng: &mut Rng) -> Chunk {
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
    fn generate(&mut self, world_position: &IVec3) -> Chunk {
        info!("Generating chunk at {:?}", world_position);
        let world_position = *world_position;
        match world_position.y {
            -6 => Chunk::random(world_position, &mut self.rng),
            -2 | -5 | -3 => Chunk::plain(world_position, BlockType::Stone),
            -4 => Chunk::plain(world_position, BlockType::Lava),
            -1 => dirt_with_grass_on_top(world_position, &mut self.rng),
            _ => Chunk::empty(world_position),
        }
    }
}
