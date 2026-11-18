use fastrand::Rng;
use glam::{U16Vec3, Vec3};
use itertools::iproduct;

use crate::graphics::Side;

use super::{BLOCKS_PER_CHUNK, CHUNK_SIZE};

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

#[derive(Debug, Clone)]
pub struct Chunk {
    pub blocks: [BlockType; BLOCKS_PER_CHUNK],
    is_empty_flag: bool,
}

const OPTIMIZATION_LEVEL: usize = 1;

impl Default for Chunk {
    fn default() -> Self {
        Chunk {
            blocks: [BlockType::Empty; BLOCKS_PER_CHUNK],
            is_empty_flag: true,
        }
    }
}

impl Chunk {
    pub fn new(is_empty: bool) -> Chunk {
        Chunk {
            is_empty_flag: is_empty,
            ..Default::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.is_empty_flag
    }

    pub fn random(rng: &mut Rng) -> Chunk {
        let mut res = Self::new(false);
        for i in 0..BLOCKS_PER_CHUNK {
            res.blocks[i] = Into::<BlockType>::into(rng.u8(..9));
        }
        res
    }

    pub fn empty() -> Chunk {
        Default::default()
    }

    pub fn plain(block: BlockType) -> Chunk {
        let mut res = Self::new(block == BlockType::Empty);
        for i in 0..BLOCKS_PER_CHUNK {
            res.blocks[i] = block;
        }
        res
    }

    // pub fn at(&self, offset: U16Vec3) -> &BlockType {
    //     &self.blocks[chunk_index_from_offset(&offset)]
    // }

    pub fn set(&mut self, offset: U16Vec3, block: BlockType) {
        debug_assert!(offset.x < CHUNK_SIZE as u16);
        debug_assert!(offset.y < CHUNK_SIZE as u16);
        debug_assert!(offset.z < CHUNK_SIZE as u16);
        self.blocks[chunk_index_from_offset(&offset)] = block;
        if block != BlockType::Empty {
            self.is_empty_flag = false;
        }
    }

    // fn at_mut(&mut self, offset: U16Vec3) -> &mut BlockType {
    //     &mut self.blocks[chunk_index_from_offset(&offset)]
    // }

    // pub fn dirt_with_grass_on_top(chunk_pos: IVec3) -> Chunk {
    //     let mut res = Self::plain(chunk_pos, BlockType::Dirt);
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

    const TEST_ONE_CUBE: bool = false;

    const NO_DATA: [Vec<i32>; 6] = [
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ];

    pub fn to_vertex_data(&self) -> [Vec<i32>; 6] {
        let mut result = Self::NO_DATA.clone();
        if self.is_empty() {
            return result;
        }
        if Self::TEST_ONE_CUBE {
            let (top, side, bottom) = BlockType::Grass.into();
            let offset = U16Vec3::new(0, 0, 0);
            add_side_data(&mut result, Side::Top, top, offset);
            add_side_data(&mut result, Side::Bottom, bottom, offset);
            add_side_data(&mut result, Side::Right, side, offset);
            add_side_data(&mut result, Side::Left, side, offset);
            add_side_data(&mut result, Side::Front, side, offset);
            add_side_data(&mut result, Side::Back, side, offset);
        } else {
            for (x, y, z) in iproduct!(0..CHUNK_SIZE, 0..CHUNK_SIZE, 0..CHUNK_SIZE) {
                let offset = U16Vec3::new(x as _, y as _, z as _);
                let (top, side, bottom) = match self.get_block(offset) {
                    BlockType::Empty => continue,
                    t => t.into(),
                };
                {
                    if y == CHUNK_SIZE - 1
                        || self.get_block(offset + U16Vec3::Y) == BlockType::Empty
                    {
                        add_side_data(&mut result, Side::Top, top, offset);
                    }
                    if y == 0 || self.get_block(offset - U16Vec3::Y) == BlockType::Empty {
                        add_side_data(&mut result, Side::Bottom, bottom, offset);
                    }
                    if x == CHUNK_SIZE - 1
                        || self.get_block(offset + U16Vec3::X) == BlockType::Empty
                    {
                        add_side_data(&mut result, Side::Right, side, offset);
                    }
                    if x == 0 || self.get_block(offset - U16Vec3::X) == BlockType::Empty {
                        add_side_data(&mut result, Side::Left, side, offset);
                    }
                    if z == CHUNK_SIZE - 1
                        || self.get_block(offset + U16Vec3::Z) == BlockType::Empty
                    {
                        add_side_data(&mut result, Side::Front, side, offset);
                    }
                    if z == 0 || self.get_block(offset - U16Vec3::Z) == BlockType::Empty {
                        add_side_data(&mut result, Side::Back, side, offset);
                    }
                }
            }
        }
        result
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

pub fn make_base_quad_data() -> [Vec3; 4] {
    SIDE_VERTICES[Side::Bottom as usize].get_quad_triangles_strip()
}

fn add_side_data(
    result: &mut [Vec<i32>; 6],
    side: Side,
    texture: BlockSideTexture,
    offset: U16Vec3,
) {
    result[side as usize].push(make_side_quad_data(side, texture, offset));
}

struct FiveBitsPerInt;
struct FourBitsPerInt;

fn pack_ivec_4bits(v: U16Vec3) -> i32 {
    debug_assert!(v.x < 16);
    debug_assert!(v.y < 16);
    debug_assert!(v.z < 16);
    (v.x | v.y << 4 | v.z << 8) as i32
}

fn pack_ivec_5bits(v: U16Vec3) -> i32 {
    debug_assert!(v.x < 32);
    debug_assert!(v.y < 32);
    debug_assert!(v.z < 32);
    (v.x | v.y << 5 | v.z << 10) as i32
}

fn make_side_quad_data(side: Side, texture: BlockSideTexture, offset: U16Vec3) -> i32 {
    let side = side as i32;
    let texture = texture as i32;
    let pos = offset;
    let mut result: i32 = 0;
    // 32 bit layout
    // -------- ---ttttt tssszzzz yyyyxxxx
    result |= pack_ivec_4bits(pos); // 12 bits zzzz yyyy xxxx
    result |= (side & 7) << 12; // 3 bits sss
    result |= (texture & 63) << 15; // 6 bits tttttt
    result
}

pub fn _generate_mesh_old<I>(sides: I) -> Vec<i32>
where
    I: IntoIterator<Item = ChunkSideData>,
    I::IntoIter: ExactSizeIterator,
{
    let iterator = sides.into_iter();
    let size = iterator.len();
    let mut data = Vec::<i32>::with_capacity(size);
    for (side, texture, offset) in iterator {
        for vert in SIDE_VERTICES[side as usize].get_quad_triangles_strip() {
            let norm = side as i32;
            let pos = vert.as_ivec3() + offset.as_ivec3();
            let mut result: i32 = 0;
            // 32 bit layout
            // -----ttt tttnnnzz zzzzyyyy yyxxxxxx
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

pub struct SideVertices {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
    pub d: Vec3,
}

// enum Corner {
//     TopLeft,
//     TopRight,
//     BotLeft,
//     BotRight,
// }

impl SideVertices {
    fn get_quad_triangles(&self) -> [Vec3; 6] {
        [self.a, self.b, self.c, self.a, self.c, self.d]
    }

    fn get_quad_triangles_strip(&self) -> [Vec3; 4] {
        [self.a, self.b, self.d, self.c]
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
