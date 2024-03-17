use glam::{UVec3, Vec3};
use rand::{distributions::Distribution, distributions::Standard, Rng};

use crate::mesh::{Mesh, QuadSideData, Side, SideVertices};

pub const BLOCK_SIZE: f32 = 1.0;

#[derive(Clone, Copy)]
enum BlockType {
    Empty = 0,
    Grass,
    Dirt,
    Stone,
    Sand,
    Lava,
}

impl Distribution<BlockType> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BlockType {
        match rng.gen_range(0..=5) {
            0 => BlockType::Empty,
            1 => BlockType::Grass,
            2 => BlockType::Dirt,
            3 => BlockType::Stone,
            4 => BlockType::Sand,
            _ => BlockType::Lava,
        }
    }
}

const CHUNK_SIZE: usize = 16;

pub struct Chunk {
    pub blocks: [[[BlockType; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
    pub world_position: UVec3,
}

impl Distribution<Chunk> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Chunk {
        let mut blocks = [[[BlockType::Empty; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    blocks[x][y] = rand::random();
                }
            }
        }
        Chunk {
            blocks,
            world_position: UVec3::ZERO,
        }
    }
}

impl Into<Mesh> for Chunk {
    fn into(self) -> Mesh {
        let mut sides: Vec<QuadSideData> = Vec::new();
        let chunk_size = 8;
        for x in 0..chunk_size {
            for y in 0..chunk_size {
                for z in 0..chunk_size {
                    let offset = Vec3::new(x as f32, y as f32, z as f32) * BLOCK_SIZE;
                    let tex = match self.blocks[x][y][z] {
                        BlockType::Empty => continue,
                        BlockType::Grass => BlockSideTexture::GrassTop,
                        BlockType::Dirt => BlockSideTexture::Dirt,
                        BlockType::Stone => BlockSideTexture::Cobblestone,
                        BlockType::Sand => BlockSideTexture::Sand,
                        BlockType::Lava => BlockSideTexture::Lava,
                    };

                    // todo: don't generate hidden sides
                    sides.push((Side::Top, tex, offset));
                    sides.push((Side::Bottom, tex, offset));
                    sides.push((Side::Front, tex, offset));
                    sides.push((Side::Back, tex, offset));
                    sides.push((Side::Left, tex, offset));
                    sides.push((Side::Right, tex, offset));
                }
            }
        }
        Mesh::from_offset_sides(sides, 1.0, &SIDE_VERTICES)
    }
}

#[derive(Clone, Copy)]
pub enum BlockSideTexture {
    Unknown = 0,
    GrassSide,
    Cobblestone,
    RedStone,
    TreeBark,
    Sand,
    Dirt,
    Pickaxe,
    TreeCenter,
    GrassTop,
    Coal,
    Lava,
    Diamond,
    Iron,
    Gold,
    Dirt2,
}

const SIDE_VERTICES: [SideVertices; 6] = [
    SideVertices {
        a: C,
        b: A,
        c: B,
        d: D,
    },
    SideVertices {
        a: A,
        b: E,
        c: F,
        d: B,
    },
    SideVertices {
        a: E,
        b: G,
        c: H,
        d: F,
    },
    SideVertices {
        a: B,
        b: F,
        c: H,
        d: D,
    },
    SideVertices {
        a: C,
        b: G,
        c: E,
        d: A,
    },
    SideVertices {
        a: D,
        b: H,
        c: G,
        d: C,
    },
];

//   C        D
// A        B

//   G        H
// E        F

const A: Vec3 = Vec3 {
    x: 0.0,
    y: 1.0,
    z: 1.0,
};
const B: Vec3 = Vec3 {
    x: 1.0,
    y: 1.0,
    z: 1.0,
};
const C: Vec3 = Vec3 {
    x: 0.0,
    y: 1.0,
    z: 0.0,
};
const D: Vec3 = Vec3 {
    x: 1.0,
    y: 1.0,
    z: 0.0,
};
const E: Vec3 = Vec3 {
    x: 0.0,
    y: 0.0,
    z: 1.0,
};
const F: Vec3 = Vec3 {
    x: 1.0,
    y: 0.0,
    z: 1.0,
};
const G: Vec3 = Vec3 {
    x: 0.0,
    y: 0.0,
    z: 0.0,
};
const H: Vec3 = Vec3 {
    x: 1.0,
    y: 0.0,
    z: 0.0,
};
