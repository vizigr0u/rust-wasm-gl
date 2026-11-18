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
            .map(BlockPos)
    }
    pub fn safe_add(&self, other: ChunkPos) -> ChunkPos {
        let v = self.0 + other.0;
        ChunkPos(ivec3(v.x, v.y.clamp(MIN_CHUNK_Y, MAX_CHUNK_Y - 1), v.z))
    }
}

impl Hash for ChunkPos {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.0.x, self.0.y, self.0.z).hash(state);
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

impl From<ChunkPos> for WorldPos {
    fn from(val: ChunkPos) -> Self {
        WorldPos {
            chunk_pos: val,
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

        WorldPos { chunk_pos, offset }
    }
}

impl Add<WorldPos> for WorldPos {
    type Output = WorldPos;
    fn add(self, rhs: WorldPos) -> Self::Output {
        let offset = self.offset + rhs.offset;
        let new_chunk_pos = self.chunk_pos.safe_add(rhs.chunk_pos);
        let dv = (self.chunk_pos.0 + rhs.chunk_pos.0) - new_chunk_pos.0;
        let world_pos: WorldPos = new_chunk_pos.into();

        world_pos + (offset + (dv.as_vec3() * CHUNK_SIZE as f32))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PageIndex(usize);

impl PageIndex {}

impl From<usize> for PageIndex {
    fn from(val: usize) -> Self {
        debug_assert!(val < NUM_CHUNKS_PER_PAGE, "Page index out of bounds");
        PageIndex(val)
    }
}

impl From<PageIndex> for usize {
    fn from(val: PageIndex) -> Self {
        val.0
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

impl From<ChunkPos> for PagePos {
    fn from(val: ChunkPos) -> Self {
        PagePos(ivec2(
            proper_rescale_i32(val.0.x + CHUNK_PAGE_SIZE.x / 2, CHUNK_PAGE_SIZE.x),
            proper_rescale_i32(val.0.z + CHUNK_PAGE_SIZE.z / 2, CHUNK_PAGE_SIZE.z),
        ))
    }
}

impl From<PagePos> for IVec2 {
    fn from(val: PagePos) -> Self {
        val.0
    }
}

impl From<IVec2> for PagePos {
    fn from(val: IVec2) -> Self {
        PagePos(val)
    }
}

impl From<ChunkPos> for IVec3 {
    fn from(val: ChunkPos) -> Self {
        val.0
    }
}

impl From<IVec3> for ChunkPos {
    fn from(val: IVec3) -> Self {
        debug_assert!(val.y >= MIN_CHUNK_Y);
        debug_assert!(val.y < MAX_CHUNK_Y);

        ChunkPos(val)
    }
}

impl From<PageChunkOffset> for IVec3 {
    fn from(val: PageChunkOffset) -> Self {
        val.0
    }
}

impl From<IVec3> for PageChunkOffset {
    fn from(val: IVec3) -> Self {
        debug_assert!(val.y >= -CHUNK_PAGE_SIZE.y);
        debug_assert!(val.y < CHUNK_PAGE_SIZE.y);
        PageChunkOffset(val)
    }
}

impl From<BlockPos> for ChunkPos {
    fn from(val: BlockPos) -> Self {
        ChunkPos(proper_rescale_ivec3(val.0, CHUNK_SIZE as i32))
    }
}

impl From<BlockPos> for IVec3 {
    fn from(val: BlockPos) -> Self {
        val.0
    }
}

impl From<IVec3> for BlockPos {
    fn from(val: IVec3) -> Self {
        debug_assert!(val.y >= MIN_BLOCK_Y);
        debug_assert!(val.y < MAX_BLOCK_Y);
        BlockPos(val)
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use glam::{ivec2, ivec3, vec3, IVec3};

    use crate::world::{
        BlockPos, ChunkPos, PageChunkOffset, PagePos, WorldPos, BLOCKS_PER_CHUNK, CHUNK_PAGE_SIZE,
        CHUNK_SIZE, MAX_BLOCK_Y, MAX_CHUNK_Y, MIN_BLOCK_Y, MIN_CHUNK_Y, NUM_CHUNKS_PER_PAGE,
    };

    #[test]
    fn test_page_index_to_page_chunk_offset() {
        let convert = |i: usize| PageChunkOffset::from_page_index(i.into());
        let invert = |o: PageChunkOffset| o.as_page_index();

        let mut map = HashMap::<PageChunkOffset, usize>::new();

        // check that each offset is unique
        for i in 0..NUM_CHUNKS_PER_PAGE {
            let offset = convert(i);
            assert!(
                !map.contains_key(&offset),
                "{i} -> {:?}, already as {:?}",
                offset,
                map[&offset]
            );
            map.insert(offset, i);
            assert_eq!(invert(offset), i.into());
        }
    }

    #[test]
    fn test_num_chunks_per_page_is_volume() {
        assert_eq!(
            NUM_CHUNKS_PER_PAGE as i32,
            CHUNK_PAGE_SIZE.x * CHUNK_PAGE_SIZE.y * CHUNK_PAGE_SIZE.z
        );
    }

    #[test]
    #[should_panic]
    fn test_page_index_out_of_bounds() {
        let _ = PageChunkOffset::from_page_index(NUM_CHUNKS_PER_PAGE.into());
    }

    #[test]
    #[should_panic]
    fn test_low_chunk_pos() {
        let _: ChunkPos = ivec3(0, MIN_CHUNK_Y - 1, 0).into();
    }

    #[test]
    #[should_panic]
    fn test_very_low_chunk_pos() {
        let _: ChunkPos = ivec3(0, MIN_CHUNK_Y - 100, 0).into();
    }

    #[test]
    #[should_panic]
    fn test_high_chunk_pos() {
        let _: ChunkPos = ivec3(0, MAX_CHUNK_Y, 0).into();
    }

    #[test]
    #[should_panic]
    fn test_very_high_chunk_pos() {
        let _: ChunkPos = ivec3(0, MAX_CHUNK_Y + 100, 0).into();
    }

    #[test]
    #[should_panic]
    fn test_low_block_pos() {
        let _: BlockPos = ivec3(0, MIN_BLOCK_Y - 1, 0).into();
    }

    #[test]
    #[should_panic]
    fn test_very_low_block_pos() {
        let _: BlockPos = ivec3(0, MIN_BLOCK_Y - 100, 0).into();
    }

    #[test]
    #[should_panic]
    fn test_high_block_pos() {
        let _: BlockPos = ivec3(0, MAX_BLOCK_Y, 0).into();
    }

    #[test]
    #[should_panic]
    fn test_very_high_block_pos() {
        let _: BlockPos = ivec3(0, MAX_BLOCK_Y + 100, 0).into();
    }

    #[test]
    fn test_page_pos_into_chunk_pos() {
        let convert = |v| Into::<PagePos>::into(v).get_center_chunk_pos();
        let invert = |c: ChunkPos| c.get_page_pos_with_offset().0;

        let tests = [
            (ivec2(0, 0), ivec3(0, 0, 0)),
            (
                ivec2(1, 2),
                ivec3(CHUNK_PAGE_SIZE.x, 0, 2 * CHUNK_PAGE_SIZE.z),
            ),
            (
                ivec2(-2, -1),
                ivec3(-CHUNK_PAGE_SIZE.x * 2, 0, -CHUNK_PAGE_SIZE.z),
            ),
        ];
        for (v, expected) in tests {
            let result = convert(v);
            assert_eq!(result, expected.into());
            assert_eq!(invert(result), v.into());
        }
    }

    #[test]
    fn test_chunk_page_pos_and_offset() {
        let convert = |v| Into::<ChunkPos>::into(v).get_page_pos_with_offset();
        let max = CHUNK_PAGE_SIZE.x / 2;

        let tests = [
            (ivec3(0, 0, 0), (ivec2(0, 0), ivec3(0, 0, 0))),
            (ivec3(1, 1, 1), (ivec2(0, 0), ivec3(1, 1, 1))),
            (ivec3(-1, -1, -1), (ivec2(0, 0), ivec3(-1, -1, -1))),
            (ivec3(-max, -1, -max), (ivec2(0, 0), ivec3(-max, -1, -max))),
            (
                ivec3(-max - 1, -1, -max - 1),
                (ivec2(-1, -1), ivec3(max - 1, -1, max - 1)),
            ),
            (
                ivec3(-max - 1, 3, max - 1),
                (ivec2(-1, 0), ivec3(max - 1, 3, max - 1)),
            ),
            (
                ivec3(-max - 1, 3, max),
                (ivec2(-1, 1), ivec3(max - 1, 3, -max)),
            ),
        ];
        for (v, expected) in tests {
            let (page_pos, offset) = convert(v);
            assert_eq!(page_pos, expected.0.into());
            assert_eq!(offset, expected.1.into());
            let chunk_pos = page_pos.get_chunk_pos_at(offset);
            assert_eq!(chunk_pos, v.into());
        }
    }

    #[test]
    fn test_block_pos_into_chunk_pos() {
        let convert = |v| Into::<ChunkPos>::into(Into::<BlockPos>::into(v));
        let max = CHUNK_SIZE as i32;
        assert_eq!(convert(ivec3(0, 0, 0)), ivec3(0, 0, 0).into());

        assert_eq!(convert(ivec3(1, 0, 0)), ivec3(0, 0, 0).into());

        assert_eq!(convert(ivec3(max, 0, 0)), ivec3(1, 0, 0).into());

        assert_eq!(convert(ivec3(max + 1, 0, 0)), ivec3(1, 0, 0).into());

        assert_eq!(convert(ivec3(max * 2, 0, 0)), ivec3(2, 0, 0).into());

        assert_eq!(convert(ivec3(max * 2, 0, -1)), ivec3(2, 0, -1).into());

        assert_eq!(convert(ivec3(max * 2, 0, -max)), ivec3(2, 0, -1).into());

        assert_eq!(convert(ivec3(max * 2, 0, -max - 1)), ivec3(2, 0, -2).into());

        assert_eq!(convert(ivec3(max * 2, 0, -max * 2)), ivec3(2, 0, -2).into());
        assert_eq!(
            convert(ivec3(max * 2, 0, -max * 2 - 2)),
            ivec3(2, 0, -3).into()
        );
    }

    // #[test]
    // fn test_proper_modulo() {
    //     assert_eq!(4, proper_modulo_i32(-6, 5));
    //     assert_eq!(0, proper_modulo_i32(-5, 5));
    //     assert_eq!(1, proper_modulo_i32(-4, 5));
    //     assert_eq!(2, proper_modulo_i32(-3, 5));
    //     assert_eq!(3, proper_modulo_i32(-2, 5));
    //     assert_eq!(4, proper_modulo_i32(-1, 5));
    //     assert_eq!(0, proper_modulo_i32(0, 5));
    //     assert_eq!(1, proper_modulo_i32(1, 5));
    //     assert_eq!(2, proper_modulo_i32(2, 5));
    //     assert_eq!(3, proper_modulo_i32(3, 5));
    //     assert_eq!(4, proper_modulo_i32(4, 5));
    //     assert_eq!(0, proper_modulo_i32(5, 5));
    //     assert_eq!(1, proper_modulo_i32(6, 5));
    // }

    #[test]
    fn test_add_world_pos() {
        let c = CHUNK_SIZE as f32;
        let tests = [
            (
                (ivec3(0, 0, 0), vec3(1.0, 2.0, 3.0)),
                (ivec3(0, 0, 0), vec3(1.0, 2.0, 3.0)),
                (ivec3(0, 0, 0), vec3(2.0, 4.0, 6.0)),
            ),
            (
                (ivec3(0, 0, 0), vec3(1.0, 2.0, 3.0)),
                (ivec3(0, 0, 0), vec3(-1.0, -2.0, -3.0)),
                (ivec3(0, 0, 0), vec3(0.0, 0.0, 0.0)),
            ),
            (
                (ivec3(1, 2, 3), vec3(1.0, 2.0, 3.0)),
                (ivec3(0, 0, 3), vec3(-1.0, -2.0, -3.0)),
                (ivec3(1, 2, 6), vec3(0.0, 0.0, 0.0)),
            ),
            (
                (ivec3(1, 2, 3), vec3(1.0, 2.0, 3.0)),
                (ivec3(0, 0, 3), vec3(c, -2.0, -3.0 - 3.0 * c)),
                (ivec3(2, 2, 3), vec3(1.0, 0.0, 0.0)),
            ),
            (
                (ivec3(1, 2, 3), vec3(1.0, 2.0, 3.0)),
                (ivec3(0, MAX_CHUNK_Y - 1, 0), vec3(-1.0, -2.0, -3.0)),
                (ivec3(1, MAX_CHUNK_Y - 1, 3), vec3(0.0, 2.0 * c, 0.0)),
            ),
            (
                (ivec3(1, 2, 3), vec3(1.0, 2.0, 3.0)),
                (ivec3(0, MIN_CHUNK_Y, 0), vec3(-2.0, 2.0, -5.0)),
                (ivec3(1, MIN_CHUNK_Y + 2, 3), vec3(-1.0, 4.0, -2.0)),
            ),
            (
                (ivec3(1, -1, 3), vec3(1.0, 2.0, 3.0)),
                (ivec3(0, MIN_CHUNK_Y, 0), vec3(-2.0, 2.0, -5.0)),
                (ivec3(1, MIN_CHUNK_Y, 3), vec3(-1.0, 4.0 - c, -2.0)),
            ),
        ];
        let to_pos = |(chunk_pos, offset)| WorldPos::new(Into::<ChunkPos>::into(chunk_pos), offset);
        for (a, b, expected) in tests {
            let (a, b, expected) = (to_pos(a), to_pos(b), to_pos(expected));
            assert_eq!(a + b, expected);
        }
    }

    #[test]
    fn test_world_pos_offset_invariant() {
        for d in [
            0.3, -0.3, 16.0, -16.0, 100.5, -100.5, -1e-7, 1e-7, 15.999, 33.0,
        ] {
            let p = WorldPos::ZERO + vec3(d, 0.0, d);
            let o = p.offset();
            assert!(o.x >= 0.0 && o.x < 16.0, "x {d} -> {o:?}");
            assert!(o.z >= 0.0 && o.z < 16.0, "z {d} -> {o:?}");
        }
    }

    #[test]
    fn test_world_pos_wrap() {
        let p = WorldPos::ZERO + vec3(15.9, 0.0, 0.0) + vec3(0.2, 0.0, 0.0);
        assert_eq!(p.chunk_pos().as_vec().x, 1);
        assert!((p.offset().x - 0.1).abs() < 1e-4);
        let p = WorldPos::ZERO + vec3(0.1, 0.0, 0.0) + vec3(-0.2, 0.0, 0.0);
        assert_eq!(p.chunk_pos().as_vec().x, -1);
        assert!((p.offset().x - 15.9).abs() < 1e-4);
    }

    #[test]
    fn test_world_pos_no_early_wrap() {
        for x in 9..16 {
            let p = WorldPos::ZERO + vec3(x as f32, 0.0, x as f32);
            assert_eq!(p.chunk_pos().as_vec(), ivec3(0, 0, 0));
        }
    }

    #[test]
    fn test_iter_block_pos_corner() {
        let chunk: ChunkPos = ivec3(2, 0, -3).into();
        let blocks: Vec<_> = chunk.iter_block_pos().collect();
        assert_eq!(blocks.len(), BLOCKS_PER_CHUNK);
        assert_eq!(blocks[0].as_vec(), ivec3(32, 0, -48));
        assert_eq!(blocks[blocks.len() - 1].as_vec(), ivec3(47, 15, -33));
        for b in blocks {
            assert_eq!(Into::<ChunkPos>::into(b), chunk);
        }
    }

    #[test]
    fn test_world_pos_to_screen_pos() {
        let c = CHUNK_SIZE as f32;
        let tests = [
            (
                (ivec3(2, 3, 4), vec3(1.0, 2.0, 3.0)),
                ivec3(2, 3, 4),
                vec3(1.0, 2.0, 3.0),
            ),
            (
                (ivec3(2, 3, 4), vec3(1.0, 2.0, 3.0)),
                ivec3(0, 0, 0),
                vec3(1.0, 2.0, 3.0) + ivec3(2, 3, 4).as_vec3() * c,
            ),
            (
                (ivec3(0, 0, 0), vec3(1.0, 2.0, 3.0)),
                ivec3(2, 3, 4),
                vec3(1.0, 2.0, 3.0) - ivec3(2, 3, 4).as_vec3() * c,
            ),
        ];
        let to_pos = |(chunk_pos, offset)| WorldPos::new(Into::<ChunkPos>::into(chunk_pos), offset);
        for (a, b, expected) in tests {
            let a = to_pos(a);
            let player_chunk: ChunkPos = b.into();
            assert_eq!(a.to_screen_pos(player_chunk), expected);
        }
    }
}
