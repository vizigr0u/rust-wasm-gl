use glam::{U16Vec3, UVec3, Vec3};
use log::info;
use rand::{distributions::Distribution, distributions::Standard, Rng};

use crate::mesh::{
    uv_definitions, Mesh, QuadSideData, Side, ToMesh, VertexAttrType, VertexDataType, SIDE_NORMS,
};

pub const CHUNK_SIZE: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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

#[derive(Debug)]
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

const OPTIMIZATION_LEVEL: usize = 1;

impl Chunk {
    pub fn to_vertex_data(&self) -> Vec<i32> {
        let mut sides: Vec<ChunkSideData> = Vec::new();
        let chunk_size = CHUNK_SIZE;
        for x in 0..chunk_size {
            for y in 0..chunk_size {
                for z in 0..chunk_size {
                    let offset = U16Vec3::new(x as _, y as _, z as _);
                    let (top, side, bottom) = match self.blocks[x][y][z] {
                        BlockType::Empty => continue,
                        t => t.into(),
                    };

                    if OPTIMIZATION_LEVEL < 1 {
                        // todo: don't generate hidden sides
                        sides.push((Side::Top, top, offset));
                        sides.push((Side::Bottom, bottom, offset));
                        sides.push((Side::Front, side, offset));
                        sides.push((Side::Back, side, offset));
                        sides.push((Side::Left, side, offset));
                        sides.push((Side::Right, side, offset));
                    } else {
                        if y == chunk_size - 1 || self.blocks[x][y + 1][z] == BlockType::Empty {
                            sides.push((Side::Top, top, offset));
                        }
                        if y == 0 || self.blocks[x][y - 1][z] == BlockType::Empty {
                            sides.push((Side::Bottom, bottom, offset));
                        }
                        if x == chunk_size - 1 || self.blocks[x + 1][y][z] == BlockType::Empty {
                            sides.push((Side::Right, side, offset));
                        }
                        if x == 0 || self.blocks[x - 1][y][z] == BlockType::Empty {
                            sides.push((Side::Left, side, offset));
                        }
                        if z == chunk_size - 1 || self.blocks[x][y][z + 1] == BlockType::Empty {
                            sides.push((Side::Front, side, offset));
                        }
                        if z == 0 || self.blocks[x][y][z - 1] == BlockType::Empty {
                            sides.push((Side::Back, side, offset));
                        }
                    }
                }
            }
        }
        generate_mesh(sides)
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

#[derive(Clone, Copy, Debug)]
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

pub type ChunkSideData = (Side, BlockSideTexture, U16Vec3);

pub fn generate_mesh<I>(sides: I) -> Vec<i32>
where
    I: IntoIterator<Item = ChunkSideData>,
    I::IntoIter: ExactSizeIterator,
{
    let iterator = sides.into_iter();
    let size = iterator.len();
    let mut data = Vec::<i32>::with_capacity(size);
    for (side, texture, offset) in iterator {
        for vert in SIDE_VERTICES[side as usize].get_quad_triangles() {
            let norm = side as i32;
            let pos = vert + offset;
            let mut result: i32 = 0;
            result |= (pos.x & 63) as i32;
            result |= (pos.y as i32 & 63) << 6;
            result |= (pos.z as i32 & 63) << 12;
            result |= (norm & 7) << 18;
            result |= (texture as i32 & 63) << 21;
            // {
            //     let data = result;
            //     let x: i32 = data & 63;
            //     let y: i32 = (data >> 6) & 63;
            //     let z: i32 = (data >> 12) & 63;
            //     let face: i32 = (data >> 18) & 7;
            //     let depth: i32 = (data >> 21) & 63;

            //     info!(
            //         "x:{}, y:{}, z:{}, norm:{norm}, texture:{} -> result: {result:b}\nx:{x}, y:{y}, z:{z}, norm:{face}, texture:{depth}",
            //         pos.x, pos.y, pos.z, texture as u32
            //     );
            // }
            data.push(result);
        }
    }
    data
}

//   C        D
// A        B

//   G        H
// E        F

const A: U16Vec3 = U16Vec3 { x: 0, y: 1, z: 1 };
const B: U16Vec3 = U16Vec3 { x: 1, y: 1, z: 1 };
const C: U16Vec3 = U16Vec3 { x: 0, y: 1, z: 0 };
const D: U16Vec3 = U16Vec3 { x: 1, y: 1, z: 0 };
const E: U16Vec3 = U16Vec3 { x: 0, y: 0, z: 1 };
const F: U16Vec3 = U16Vec3 { x: 1, y: 0, z: 1 };
const G: U16Vec3 = U16Vec3 { x: 0, y: 0, z: 0 };
const H: U16Vec3 = U16Vec3 { x: 1, y: 0, z: 0 };

pub struct SideVertices {
    pub a: U16Vec3,
    pub b: U16Vec3,
    pub c: U16Vec3,
    pub d: U16Vec3,
}

enum Corner {
    TopLeft,
    TopRight,
    BotLeft,
    BotRight,
}

impl SideVertices {
    fn get_quad_triangles(&self) -> [U16Vec3; 6] {
        [self.a, self.b, self.c, self.a, self.c, self.d]
    }
}

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
