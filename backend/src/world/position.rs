use glam::{ivec2, ivec3, IVec2, IVec3, Vec3};
use log::{info, warn};

pub const CHUNK_SIZE: usize = 16;
pub const BLOCKS_PER_CHUNK: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

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

pub const CHUNK_PAGE_SIZE: IVec3 = IVec3 { x: 4, y: 4, z: 4 };
pub const NUM_CHUNKS_PER_PAGE: usize =
    CHUNK_PAGE_SIZE.x as usize * CHUNK_PAGE_SIZE.y as usize * CHUNK_PAGE_SIZE.y as usize;

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
            self.0.y * CHUNK_PAGE_SIZE.y,
        ))
    }
}

impl Into<PagePos> for ChunkPos {
    fn into(self) -> PagePos {
        PagePos(ivec2(
            self.0.x / CHUNK_PAGE_SIZE.x,
            self.0.z / CHUNK_PAGE_SIZE.y,
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

impl Into<ChunkPos> for BlockPos {
    fn into(self) -> ChunkPos {
        ChunkPos(self.0 / CHUNK_SIZE as i32)
    }
}

impl Into<BlockPos> for ChunkPos {
    fn into(self) -> BlockPos {
        BlockPos(self.0 * CHUNK_SIZE as i32)
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
