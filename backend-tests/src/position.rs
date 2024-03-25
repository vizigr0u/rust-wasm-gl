use glam::{ivec2, ivec3, IVec2, IVec3, Vec3};

pub const CHUNK_SIZE: usize = 16;
pub const BLOCKS_PER_CHUNK: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

pub const MIN_BLOCK_Y: i32 = -256;
pub const TOTAL_BLOCK_HEIGHT: i32 = CHUNK_SIZE as i32 * 20;
pub const MAX_BLOCK_Y: i32 = MIN_BLOCK_Y + TOTAL_BLOCK_HEIGHT;

pub const MIN_CHUNK_Y: i32 = MIN_BLOCK_Y / CHUNK_SIZE as i32;
pub const MAX_CHUNK_Y: i32 = MAX_BLOCK_Y / CHUNK_SIZE as i32;
pub const TOTAL_CHUNK_HEIGHT: i32 = TOTAL_BLOCK_HEIGHT / CHUNK_SIZE as i32;

pub const CHUNK_PAGE_SIZE: IVec3 = IVec3 {
    x: 4,
    y: TOTAL_CHUNK_HEIGHT,
    z: 4,
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
    pub fn from_world_pos(world_pos: IVec3) -> ChunkPos {
        ChunkPos(ivec3(
            world_pos.x / CHUNK_PAGE_SIZE.x,
            world_pos.y / CHUNK_PAGE_SIZE.y,
            world_pos.z / CHUNK_PAGE_SIZE.z,
        ))
    }
    pub fn get_first_block(&self) -> BlockPos {
        BlockPos(self.0 * CHUNK_SIZE as i32)
    }
    pub fn get_center_block(&self) -> BlockPos {
        BlockPos(self.0 + CHUNK_SIZE as i32 / 2 * IVec3::ONE)
    }
    pub fn distance_squared(&self, other: ChunkPos) -> i32 {
        self.as_vec().distance_squared(other.as_vec())
    }
    pub fn as_vec(&self) -> IVec3 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PageChunkOffset(IVec3);

impl PageChunkOffset {
    pub fn as_vec(&self) -> IVec3 {
        self.0
    }
    pub fn as_index_in_page(&self) -> usize {
        (self.0.x + self.0.y * CHUNK_PAGE_SIZE.x + self.0.z * CHUNK_PAGE_SIZE.x * CHUNK_PAGE_SIZE.y)
            as usize
    }
    pub fn from_page_index(index: usize) -> PageChunkOffset {
        PageChunkOffset(ivec3(
            proper_modulo_i32(index as i32, CHUNK_PAGE_SIZE.x),
            proper_modulo_i32(index as i32 / CHUNK_PAGE_SIZE.x, CHUNK_PAGE_SIZE.y) + MIN_CHUNK_Y,
            proper_modulo_i32(index as i32, CHUNK_PAGE_SIZE.x) / CHUNK_PAGE_SIZE.y,
        ))
    }
    pub fn to_chunk_pos(&self, page_pos: PagePos) -> ChunkPos {
        let page_block_position: ChunkPos = page_pos.into();
        let chunk_offset = self.as_vec();
        ChunkPos(page_block_position.0 + chunk_offset)
    }
}

impl Into<PageChunkOffset> for ChunkPos {
    fn into(self) -> PageChunkOffset {
        PageChunkOffset(ivec3(
            proper_modulo_i32(self.0.x, CHUNK_PAGE_SIZE.x),
            proper_modulo_i32(self.0.y - MIN_CHUNK_Y, CHUNK_PAGE_SIZE.y) + MIN_CHUNK_Y,
            proper_modulo_i32(self.0.z, CHUNK_PAGE_SIZE.z),
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PagePos(IVec2);

impl PagePos {
    pub fn get_center_chunk(&self) -> ChunkPos {
        ChunkPos(ivec3(self.0.x, 0, self.0.y) + CHUNK_PAGE_SIZE / 2)
    }

    pub fn as_vec(&self) -> IVec2 {
        self.0
    }
}

impl Into<ChunkPos> for PagePos {
    fn into(self) -> ChunkPos {
        ChunkPos(ivec3(
            self.0.x * CHUNK_PAGE_SIZE.x,
            0,
            self.0.y * CHUNK_PAGE_SIZE.z,
        ))
    }
}

impl Into<PagePos> for ChunkPos {
    fn into(self) -> PagePos {
        PagePos(ivec2(
            proper_rescale_i32(self.0.x, CHUNK_PAGE_SIZE.x),
            proper_rescale_i32(self.0.z, CHUNK_PAGE_SIZE.z),
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
        debug_assert!(self.y >= 0);
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
