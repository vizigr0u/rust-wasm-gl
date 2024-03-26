use glam::{ivec2, ivec3, IVec2, IVec3, Vec3};
use itertools::iproduct;

pub const CHUNK_SIZE: usize = 16;
pub const BLOCKS_PER_CHUNK: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

pub const TOTAL_CHUNK_HEIGHT: i32 = 20;
pub const MIN_CHUNK_Y: i32 = -16;
pub const MAX_CHUNK_Y: i32 = TOTAL_CHUNK_HEIGHT + MIN_CHUNK_Y;

pub const MIN_BLOCK_Y: i32 = MIN_CHUNK_Y * CHUNK_SIZE as i32;
pub const MAX_BLOCK_Y: i32 = MAX_CHUNK_Y * CHUNK_SIZE as i32;
pub const TOTAL_BLOCK_HEIGHT: i32 = TOTAL_CHUNK_HEIGHT * CHUNK_SIZE as i32;

pub const CHUNK_PAGE_SIZE: IVec3 = IVec3 {
    x: 4,
    y: TOTAL_CHUNK_HEIGHT,
    z: 4,
};

pub const MIN_CHUNK_PAGE_OFFSET: IVec3 = IVec3 {
    x: -CHUNK_PAGE_SIZE.x / 2,
    y: MIN_CHUNK_Y,
    z: -CHUNK_PAGE_SIZE.z / 2,
};

pub const NUM_CHUNKS_PER_PAGE: usize =
    (CHUNK_PAGE_SIZE.x * CHUNK_PAGE_SIZE.y * CHUNK_PAGE_SIZE.y) as usize;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ChunkPos(IVec3);

impl ChunkPos {
    pub fn get_center_block_pos(&self) -> BlockPos {
        BlockPos(self.0 * CHUNK_SIZE as i32)
    }
    pub fn get_page_pos(&self) -> (PagePos, PageChunkOffset) {
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
    pub fn iter_block_pos(&self) -> impl Iterator<Item = ChunkPos> {
        let center_pos = self.get_center_block_pos();
        let first_pos = center_pos.0 - IVec3::ONE * (CHUNK_SIZE as i32 / 2);
        iproduct!(0..CHUNK_SIZE, 0..CHUNK_SIZE, 0..CHUNK_SIZE)
            .map(move |(x, y, z)| first_pos + ivec3(x as _, y as _, z as _))
            .map(|v| ChunkPos(v))
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
        BlockPos(self)
    }
}

pub fn proper_modulo_i32(n: i32, f: i32) -> i32 {
    (n % f + f) % f
}

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
