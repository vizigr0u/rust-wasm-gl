use glam::{UVec3, Vec3};
use rand::{distributions::Distribution, distributions::Standard, Rng};

use crate::mesh::{Mesh, QuadSideData, Side, SideVertices, ToMesh};

pub const BLOCK_SIZE: f32 = 1.0;
pub const CHUNK_SIZE: usize = 8;

#[derive(Clone, Copy)]
pub enum BlockType {
    Empty = 0,
    Grass,
    Dirt,
    Stone,
    Sand,
    Lava,
    Diamond,
    Coal,
    Gold,
}

impl Distribution<BlockType> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> BlockType {
        match rng.gen_range(0..=8) {
            0 => BlockType::Empty,
            1 => BlockType::Grass,
            2 => BlockType::Dirt,
            3 => BlockType::Stone,
            4 => BlockType::Sand,
            5 => BlockType::Lava,
            6 => BlockType::Diamond,
            7 => BlockType::Coal,
            _ => BlockType::Gold,
        }
    }
}

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
                    blocks[x][y][z] = rand::random();
                }
            }
        }
        Chunk {
            blocks,
            world_position: UVec3::ZERO,
        }
    }
}

impl ToMesh for Chunk {
    fn to_mesh(&self) -> Mesh {
        let mut sides: Vec<QuadSideData> = Vec::new();
        let chunk_size = CHUNK_SIZE;
        for x in 0..chunk_size {
            for y in 0..chunk_size {
                for z in 0..chunk_size {
                    let offset = Vec3::new(x as f32, y as f32, z as f32) * BLOCK_SIZE;
                    let (top, side, bottom) = match self.blocks[x][y][z] {
                        BlockType::Empty => continue,
                        t => t.into(),
                    };

                    // todo: don't generate hidden sides
                    sides.push((Side::Top, top, offset));
                    sides.push((Side::Bottom, bottom, offset));
                    sides.push((Side::Front, side, offset));
                    sides.push((Side::Back, side, offset));
                    sides.push((Side::Left, side, offset));
                    sides.push((Side::Right, side, offset));
                }
            }
        }
        Mesh::from_offset_sides(sides, 1.0, &SIDE_VERTICES)
    }
}

type BlockSideTextures = (BlockSideTexture, BlockSideTexture, BlockSideTexture);

impl Into<BlockSideTextures> for BlockType {
    fn into(self) -> BlockSideTextures {
        match self {
            BlockType::Grass => (
                BlockSideTexture::GrassTop,
                BlockSideTexture::GrassSide,
                BlockSideTexture::Dirt,
            ),
            BlockType::Dirt => three_of(BlockSideTexture::Dirt),
            BlockType::Stone => three_of(BlockSideTexture::Cobblestone),
            BlockType::Sand => three_of(BlockSideTexture::Sand),
            BlockType::Lava => three_of(BlockSideTexture::Lava),
            BlockType::Diamond => three_of(BlockSideTexture::Diamond),
            BlockType::Coal => three_of(BlockSideTexture::Coal),
            BlockType::Gold => three_of(BlockSideTexture::Gold),
            BlockType::Empty => three_of(BlockSideTexture::Unknown),
            _ => three_of(BlockSideTexture::Pickaxe),
        }
    }
}

fn three_of(t: BlockSideTexture) -> BlockSideTextures {
    (t, t, t)
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

//   C        D
// A        B

//   G        H
// E        F

const SIDE_VERTICES: [SideVertices; 6] = [
    SideVertices {
        a: C,
        b: A,
        c: B,
        d: D,
    },
    SideVertices {
        a: E,
        b: G,
        c: H,
        d: F,
    },
    SideVertices {
        a: A,
        b: E,
        c: F,
        d: B,
    },
    SideVertices {
        a: D,
        b: H,
        c: G,
        d: C,
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
];

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
