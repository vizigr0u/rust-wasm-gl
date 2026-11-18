use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::{Add, AddAssign},
};

use glam::{ivec2, ivec3, IVec2, IVec3, Vec3};
use itertools::iproduct;

pub const CHUNK_SIZE: usize = 16;
pub const BLOCKS_PER_CHUNK: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

pub const TOTAL_CHUNK_HEIGHT: i32 = 320 / CHUNK_SIZE as i32;
pub const MIN_CHUNK_Y: i32 = -256 / CHUNK_SIZE as i32;
pub const MAX_CHUNK_Y: i32 = TOTAL_CHUNK_HEIGHT + MIN_CHUNK_Y;

pub const MIN_BLOCK_Y: i32 = MIN_CHUNK_Y * CHUNK_SIZE as i32;
pub const MAX_BLOCK_Y: i32 = MAX_CHUNK_Y * CHUNK_SIZE as i32;
pub const TOTAL_BLOCK_HEIGHT: i32 = TOTAL_CHUNK_HEIGHT * CHUNK_SIZE as i32;

pub const MIN_CHUNK_PAGE_OFFSET: IVec3 = IVec3 {
    x: -3,
    y: MIN_CHUNK_Y,
    z: -3,
};

pub const CHUNK_PAGE_SIZE: IVec3 = IVec3 {
    x: -MIN_CHUNK_PAGE_OFFSET.x * 2,
    y: TOTAL_CHUNK_HEIGHT,
    z: -MIN_CHUNK_PAGE_OFFSET.x * 2,
};

pub const NUM_CHUNKS_PER_PAGE: usize =
    (CHUNK_PAGE_SIZE.x * CHUNK_PAGE_SIZE.y * CHUNK_PAGE_SIZE.z) as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BlockPos(IVec3);

impl BlockPos {
    pub fn as_vec(&self) -> IVec3 {
        self.0
    }
    pub fn as_vec3(&self) -> Vec3 {
        self.0.as_vec3()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChunkPos(IVec3);

impl ChunkPos {
    pub fn get_origin_block_pos(&self) -> BlockPos {
        BlockPos(self.0 * CHUNK_SIZE as i32)
    }
    pub fn get_page_pos_with_offset(&self) -> (PagePos, PageChunkOffset) {
        let page_pos: PagePos = (*self).into();
        let page_center_chunk = page_pos.get_center_chunk_pos();
        let my_offset = self.0 - page_center_chunk.0;
        (page_pos, PageChunkOffset(my_offset))
    }
    pub fn distance_squared(&self, other: ChunkPos) -> i32 {
        self.as_vec().distance_squared(other.as_vec())
    }
    pub fn as_vec(&self) -> IVec3 {
        self.0
    }
    pub fn iter_block_pos(&self) -> impl Iterator<Item = BlockPos> {
        let first_pos = self.get_origin_block_pos().0;
        iproduct!(0..CHUNK_SIZE, 0..CHUNK_SIZE, 0..CHUNK_SIZE)
            .map(move |(x, y, z)| first_pos + ivec3(x as _, y as _, z as _))
            .map(|v| BlockPos(v))
    }
    pub fn safe_add(&self, other: ChunkPos) -> ChunkPos {
        let v = self.0 + other.0;
        ChunkPos(ivec3(v.x, v.y.clamp(MIN_CHUNK_Y, MAX_CHUNK_Y - 1), v.z))
    }
}

impl Hash for ChunkPos {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ((self.0.x & 0xfff) << 20 | (self.0.y & 0xfff) << 10 | (self.0.z & 0xfff)).hash(state);
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub struct WorldPos {
    chunk_pos: ChunkPos,
    offset: Vec3,
}

impl Debug for WorldPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "WorldPos{{ ({:?}, {:?}, {:?})+({:?}, {:?}, {:?}) }}",
            self.chunk_pos.0.x,
            self.chunk_pos.0.y,
            self.chunk_pos.0.z,
            self.offset.x,
            self.offset.y,
            self.offset.z
        )
    }
}

impl WorldPos {
    pub const ZERO: Self = Self {
        chunk_pos: ChunkPos(IVec3::ZERO),
        offset: Vec3::ZERO,
    };
    pub fn new(chunk_pos: ChunkPos, offset: Vec3) -> Self {
        Into::<WorldPos>::into(chunk_pos) + offset
    }
    pub fn chunk_pos(&self) -> ChunkPos {
        self.chunk_pos
    }
    pub fn offset(&self) -> Vec3 {
        self.offset
    }
    pub fn to_screen_pos(&self, player_chunk: ChunkPos) -> Vec3 {
        (self.chunk_pos.0 - player_chunk.0).as_vec3() * CHUNK_SIZE as f32 + self.offset
    }
}

impl Into<WorldPos> for ChunkPos {
    fn into(self) -> WorldPos {
        WorldPos {
            chunk_pos: self,
            offset: Vec3::ZERO,
        }
    }
}

impl AddAssign<WorldPos> for WorldPos {
    fn add_assign(&mut self, rhs: WorldPos) {
        *self = *self + rhs;
    }
}

impl AddAssign<Vec3> for WorldPos {
    fn add_assign(&mut self, rhs: Vec3) {
        *self = *self + rhs;
    }
}

impl Add<Vec3> for WorldPos {
    type Output = WorldPos;
    fn add(self, rhs: Vec3) -> Self::Output {
        let offset = self.offset + rhs;
        // Corner convention: chunk N covers [16N, 16N+16), so offset wraps into [0, 16).
        let cs = CHUNK_SIZE as f32;
        let mut chunks_to_add = (offset / cs).floor().as_ivec3();
        let mut expected_offset = offset - chunks_to_add.as_vec3() * cs;
        // f32 edge: a tiny negative offset can round up to exactly 16.0.
        if expected_offset.x >= cs {
            expected_offset.x -= cs;
            chunks_to_add.x += 1;
        }
        if expected_offset.z >= cs {
            expected_offset.z -= cs;
            chunks_to_add.z += 1;
        }
        let chunk_pos = self.chunk_pos.safe_add(ChunkPos(chunks_to_add));
        let chunks_added = chunk_pos.0 - self.chunk_pos.0;
        let dv = chunks_to_add - chunks_added;
        let offset = expected_offset + (dv.as_vec3() * CHUNK_SIZE as f32);
        let res = WorldPos { chunk_pos, offset };
        res
    }
}

impl Add<WorldPos> for WorldPos {
    type Output = WorldPos;
    fn add(self, rhs: WorldPos) -> Self::Output {
        let offset = self.offset + rhs.offset;
        let new_chunk_pos = self.chunk_pos.safe_add(rhs.chunk_pos);
        let dv = (self.chunk_pos.0 + rhs.chunk_pos.0) - new_chunk_pos.0;
        let world_pos: WorldPos = new_chunk_pos.into();
        let res = world_pos + (offset + (dv.as_vec3() * CHUNK_SIZE as f32));
        res
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PageIndex(usize);

impl PageIndex {}

impl Into<PageIndex> for usize {
    fn into(self) -> PageIndex {
        debug_assert!(self < NUM_CHUNKS_PER_PAGE, "Page index out of bounds");
        PageIndex(self)
    }
}

impl Into<usize> for PageIndex {
    fn into(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PageChunkOffset(IVec3);

impl PageChunkOffset {
    pub fn as_vec(&self) -> IVec3 {
        self.0
    }
    pub fn as_page_index(&self) -> PageIndex {
        let v = self.0 - MIN_CHUNK_PAGE_OFFSET;
        PageIndex(
            (v.x + v.y * CHUNK_PAGE_SIZE.x + v.z * CHUNK_PAGE_SIZE.x * CHUNK_PAGE_SIZE.y) as usize,
        )
    }
    pub fn from_page_index(index: PageIndex) -> PageChunkOffset {
        PageChunkOffset(
            ivec3(
                index.0 as i32 % CHUNK_PAGE_SIZE.x,
                (index.0 as i32 / CHUNK_PAGE_SIZE.x) % CHUNK_PAGE_SIZE.y,
                index.0 as i32 / (CHUNK_PAGE_SIZE.x * CHUNK_PAGE_SIZE.y),
            ) + MIN_CHUNK_PAGE_OFFSET,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PagePos(IVec2);

impl PagePos {
    pub fn get_center_chunk_pos(&self) -> ChunkPos {
        ChunkPos(ivec3(
            self.0.x * CHUNK_PAGE_SIZE.x,
            0,
            self.0.y * CHUNK_PAGE_SIZE.z,
        ))
    }

    pub fn get_chunk_pos_at(&self, offset: PageChunkOffset) -> ChunkPos {
        let center = self.get_center_chunk_pos();
        ChunkPos(center.0 + offset.0)
    }

    pub fn has_chunk_pos(&self, chunk_pos: ChunkPos) -> bool {
        let page_pos: PagePos = chunk_pos.into();
        page_pos.0 == self.0
    }

    pub fn as_vec(&self) -> IVec2 {
        self.0
    }

    pub fn iter_chunk_offsets(&self) -> impl Iterator<Item = PageChunkOffset> {
        (0..NUM_CHUNKS_PER_PAGE).map(|i| PageChunkOffset::from_page_index(PageIndex(i)))
    }

    pub fn iter_chunk_pos(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.iter_chunk_offsets().map(|o| self.get_chunk_pos_at(o))
    }
}

impl Into<PagePos> for ChunkPos {
    fn into(self) -> PagePos {
        PagePos(ivec2(
            proper_rescale_i32(self.0.x + CHUNK_PAGE_SIZE.x / 2, CHUNK_PAGE_SIZE.x),
            proper_rescale_i32(self.0.z + CHUNK_PAGE_SIZE.z / 2, CHUNK_PAGE_SIZE.z),
        ))
    }
}

impl Into<IVec2> for PagePos {
    fn into(self) -> IVec2 {
        self.0
    }
}

impl Into<PagePos> for IVec2 {
    fn into(self) -> PagePos {
        PagePos(self)
    }
}

impl Into<IVec3> for ChunkPos {
    fn into(self) -> IVec3 {
        self.0
    }
}

impl Into<ChunkPos> for IVec3 {
    fn into(self) -> ChunkPos {
        debug_assert!(self.y >= MIN_CHUNK_Y);
        debug_assert!(self.y < MAX_CHUNK_Y);

        ChunkPos(self)
    }
}

impl Into<IVec3> for PageChunkOffset {
    fn into(self) -> IVec3 {
        self.0
    }
}

impl Into<PageChunkOffset> for IVec3 {
    fn into(self) -> PageChunkOffset {
        debug_assert!(self.y >= -CHUNK_PAGE_SIZE.y);
        debug_assert!(self.y < CHUNK_PAGE_SIZE.y);
        PageChunkOffset(self)
    }
}

impl Into<ChunkPos> for BlockPos {
    fn into(self) -> ChunkPos {
        ChunkPos(proper_rescale_ivec3(self.0, CHUNK_SIZE as i32))
    }
}

impl Into<IVec3> for BlockPos {
    fn into(self) -> IVec3 {
        self.0
    }
}

impl Into<BlockPos> for IVec3 {
    fn into(self) -> BlockPos {
        debug_assert!(self.y >= MIN_BLOCK_Y);
        debug_assert!(self.y < MAX_BLOCK_Y);
        BlockPos(self)
    }
}

// pub fn proper_modulo_i32(n: i32, f: i32) -> i32 {
//     (n % f + f) % f
// }

pub fn proper_rescale_i32(n: i32, f: i32) -> i32 {
    if n < 0 {
        (n - f + 1) / f
    } else {
        n / f
    }
}

fn proper_rescale_ivec3(n: IVec3, f: i32) -> IVec3 {
    IVec3::new(
        proper_rescale_i32(n.x, f),
        proper_rescale_i32(n.y, f),
        proper_rescale_i32(n.z, f),
    )
}
