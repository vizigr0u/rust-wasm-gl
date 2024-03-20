use fastrand::Rng;
use glam::{U16Vec3, UVec3};

use crate::mesh::Side;

pub const CHUNK_SIZE: usize = 8;
pub const BLOCKS_PER_CHUNK: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

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

impl Into<BlockType> for u8 {
    fn into(self) -> BlockType {
        match self {
            0 => BlockType::Empty,
            1 => BlockType::Grass,
            2 => BlockType::Dirt,
            3 => BlockType::Stone,
            4 => BlockType::Sand,
            5 => BlockType::Lava,
            6 => BlockType::Diamond,
            7 => BlockType::Coal,
            8 => BlockType::Gold,
            _ => BlockType::Empty,
        }
    }
}

pub trait ChunkGenerator {
    fn generate(&mut self, pos: &UVec3) -> Chunk;
}

pub struct EmptyChunkGenerator;

impl ChunkGenerator for EmptyChunkGenerator {
    fn generate(&mut self, _pos: &UVec3) -> Chunk {
        Chunk::default()
    }
}

#[derive(Debug)]
pub struct RandomChunkGenerator {
    pub rng: Rng,
}

impl ChunkGenerator for RandomChunkGenerator {
    fn generate(&mut self, pos: &UVec3) -> Chunk {
        Chunk::random(*pos, &mut self.rng)
    }
}

#[derive(Debug)]
pub struct Chunk {
    pub blocks: [BlockType; BLOCKS_PER_CHUNK],
    pub world_position: UVec3,
}

const OPTIMIZATION_LEVEL: usize = 1;

impl Default for Chunk {
    fn default() -> Self {
        Chunk {
            blocks: [BlockType::Empty; BLOCKS_PER_CHUNK],
            world_position: UVec3::ZERO,
        }
    }
}

impl Chunk {
    pub fn new(world_position: UVec3) -> Chunk {
        Chunk {
            world_position,
            ..Default::default()
        }
    }

    pub fn random(world_position: UVec3, rng: &mut Rng) -> Chunk {
        let mut res = Self::new(world_position);
        for i in 0..BLOCKS_PER_CHUNK {
            res.blocks[i] = Into::<BlockType>::into(rng.u8(..9));
        }
        res
    }

    pub fn empty(world_position: UVec3) -> Chunk {
        Self::plain(world_position, BlockType::Empty)
    }

    pub fn plain(world_position: UVec3, block: BlockType) -> Chunk {
        let mut res = Self::new(world_position);
        for i in 0..BLOCKS_PER_CHUNK {
            res.blocks[i] = block;
        }
        res
    }

    // pub fn at(&self, offset: U16Vec3) -> &BlockType {
    //     &self.blocks[chunk_index_from_offset(&offset)]
    // }

    pub fn set(&mut self, offset: U16Vec3, block: BlockType) {
        self.blocks[chunk_index_from_offset(&offset)] = block;
    }

    // fn at_mut(&mut self, offset: U16Vec3) -> &mut BlockType {
    //     &mut self.blocks[chunk_index_from_offset(&offset)]
    // }

    // pub fn dirt_with_grass_on_top(world_position: UVec3) -> Chunk {
    //     let mut res = Self::plain(world_position, BlockType::Dirt);
    //     for x in 0..CHUNK_SIZE {
    //         for z in 0..CHUNK_SIZE {
    //             *res.at_mut(U16Vec3::new(x as _, CHUNK_SIZE as u16 - 1, z as _)) = BlockType::Grass;
    //         }
    //     }
    //     res
    // }

    pub fn get_block(&self, offset: U16Vec3) -> BlockType {
        self.blocks[chunk_index_from_offset(&offset)]
    }

    pub fn to_vertex_data(&self) -> Vec<i32> {
        let mut sides: Vec<ChunkSideData> = Vec::new();
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let offset = U16Vec3::new(x as _, y as _, z as _);
                    let (top, side, bottom) = match self.get_block(offset) {
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
                        if y == CHUNK_SIZE - 1
                            || self.get_block(offset + U16Vec3::Y) == BlockType::Empty
                        {
                            sides.push((Side::Top, top, offset));
                        }
                        if y == 0 || self.get_block(offset - U16Vec3::Y) == BlockType::Empty {
                            sides.push((Side::Bottom, bottom, offset));
                        }
                        if x == CHUNK_SIZE - 1
                            || self.get_block(offset + U16Vec3::X) == BlockType::Empty
                        {
                            sides.push((Side::Right, side, offset));
                        }
                        if x == 0 || self.get_block(offset - U16Vec3::X) == BlockType::Empty {
                            sides.push((Side::Left, side, offset));
                        }
                        if z == CHUNK_SIZE - 1
                            || self.get_block(offset + U16Vec3::Z) == BlockType::Empty
                        {
                            sides.push((Side::Front, side, offset));
                        }
                        if z == 0 || self.get_block(offset - U16Vec3::Z) == BlockType::Empty {
                            sides.push((Side::Back, side, offset));
                        }
                    }
                }
            }
        }
        generate_mesh(sides)
    }
}

// #[inline(always)]
// fn offset_from_chunk_index(i: usize) -> U16Vec3 {
//     U16Vec3::new(
//         (i % CHUNK_SIZE) as _,
//         (i / CHUNK_SIZE) as _,
//         (i / (CHUNK_SIZE * CHUNK_SIZE)) as _,
//     )
// }

#[inline(always)]
fn chunk_index_from_offset(offset: &U16Vec3) -> usize {
    offset.x as usize + offset.y as usize * CHUNK_SIZE + offset.z as usize * CHUNK_SIZE * CHUNK_SIZE
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
            // _ => three_of(BlockSideTexture::Pickaxe),
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

// enum Corner {
//     TopLeft,
//     TopRight,
//     BotLeft,
//     BotRight,
// }

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
