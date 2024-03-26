mod position;

use position::*;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use glam::{ivec2, ivec3, IVec3};

    use crate::{
        proper_modulo_i32, BlockPos, ChunkPos, PageChunkOffset, PagePos, CHUNK_PAGE_SIZE,
        CHUNK_SIZE, MAX_CHUNK_Y, MIN_CHUNK_Y, NUM_CHUNKS_PER_PAGE,
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
    fn test_high_chunk_pos() {
        let _: ChunkPos = ivec3(0, MAX_CHUNK_Y, 0).into();
    }

    #[test]
    fn test_page_pos_into_chunk_pos() {
        let convert = |v| Into::<PagePos>::into(v).get_center_chunk_pos();
        let invert = |c: ChunkPos| c.get_page_pos().0;

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
        let convert = |v| Into::<ChunkPos>::into(v).get_page_pos();
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

    // #[test]
    fn test_chunk_pos_into_page_pos() {
        let convert = |v| Into::<PagePos>::into(Into::<ChunkPos>::into(v));
        assert_eq!(convert(ivec3(0, 0, 0)), ivec2(0, 0).into());

        assert_eq!(convert(ivec3(1, 0, 0)), ivec2(0, 0).into());

        assert_eq!(convert(ivec3(CHUNK_PAGE_SIZE.x, 0, 0)), ivec2(1, 0).into());

        assert_eq!(
            convert(ivec3(CHUNK_PAGE_SIZE.x + 1, 0, 0)),
            ivec2(1, 0).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_PAGE_SIZE.x * 2, 0, 0)),
            ivec2(2, 0).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_PAGE_SIZE.x * 2, 0, -1)),
            ivec2(2, -1).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_PAGE_SIZE.x * 2, 0, -CHUNK_PAGE_SIZE.z)),
            ivec2(2, -1).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_PAGE_SIZE.x * 2, 0, -CHUNK_PAGE_SIZE.z - 1)),
            ivec2(2, -2).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_PAGE_SIZE.x * 2, 0, -CHUNK_PAGE_SIZE.z * 2 - 1)),
            ivec2(2, -3).into()
        );
    }

    // #[test]
    fn test_block_pos_into_chunk_pos() {
        let convert = |v| Into::<ChunkPos>::into(Into::<BlockPos>::into(v));
        assert_eq!(convert(ivec3(0, 0, 0)), ivec3(0, 0, 0).into());

        assert_eq!(convert(ivec3(1, 0, 0)), ivec3(0, 0, 0).into());

        assert_eq!(
            convert(ivec3(CHUNK_SIZE as i32, 0, 0)),
            ivec3(1, 0, 0).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_SIZE as i32 + 1, 0, 0)),
            ivec3(1, 0, 0).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_SIZE as i32 * 2, 0, 0)),
            ivec3(2, 0, 0).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_SIZE as i32 * 2, 0, -1)),
            ivec3(2, 0, -1).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_SIZE as i32 * 2, 0, -(CHUNK_SIZE as i32))),
            ivec3(2, 0, -1).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_SIZE as i32 * 2, 0, -(CHUNK_SIZE as i32) - 1)),
            ivec3(2, 0, -2).into()
        );

        assert_eq!(
            convert(ivec3(
                CHUNK_SIZE as i32 * 2,
                0,
                -(CHUNK_SIZE as i32) * 2 - 1
            )),
            ivec3(2, 0, -3).into()
        );
    }

    #[test]
    fn test_proper_modulo() {
        assert_eq!(4, proper_modulo_i32(-6, 5));
        assert_eq!(0, proper_modulo_i32(-5, 5));
        assert_eq!(1, proper_modulo_i32(-4, 5));
        assert_eq!(2, proper_modulo_i32(-3, 5));
        assert_eq!(3, proper_modulo_i32(-2, 5));
        assert_eq!(4, proper_modulo_i32(-1, 5));
        assert_eq!(0, proper_modulo_i32(0, 5));
        assert_eq!(1, proper_modulo_i32(1, 5));
        assert_eq!(2, proper_modulo_i32(2, 5));
        assert_eq!(3, proper_modulo_i32(3, 5));
        assert_eq!(4, proper_modulo_i32(4, 5));
        assert_eq!(0, proper_modulo_i32(5, 5));
        assert_eq!(1, proper_modulo_i32(6, 5));
    }

    // #[test]
    // fn test_chunk_pos_into_page_offset() {
    //     let convert = |v| Into::<PageChunkOffset>::into(Into::<ChunkPos>::into(v));
    //     assert_eq!(convert(ivec3(0, 1, 0)), ivec3(0, 1, 0).into());
    //     assert_eq!(convert(ivec3(0, 0, 0)), ivec3(0, 0, 0).into());

    //     assert_eq!(convert(ivec3(1, 0, 0)), ivec3(1, 0, 0).into());

    //     assert_eq!(
    //         convert(ivec3(CHUNK_PAGE_SIZE.x, 0, 0)),
    //         ivec3(0, 0, 0).into()
    //     );

    //     assert_eq!(
    //         convert(ivec3(CHUNK_PAGE_SIZE.x + 1, 0, 0)),
    //         ivec3(1, 0, 0).into()
    //     );

    //     assert_eq!(
    //         convert(ivec3(CHUNK_PAGE_SIZE.x * 2, 0, 0)),
    //         ivec3(0, 0, 0).into()
    //     );

    //     assert_eq!(
    //         convert(ivec3(CHUNK_PAGE_SIZE.x * 2, 0, -1)),
    //         ivec3(0, 0, -CHUNK_PAGE_SIZE.z - 1).into()
    //     );

    //     assert_eq!(
    //         convert(ivec3(CHUNK_PAGE_SIZE.x * 2, 0, -CHUNK_PAGE_SIZE.z)),
    //         ivec3(0, 0, 0).into()
    //     );

    //     assert_eq!(
    //         convert(ivec3(CHUNK_PAGE_SIZE.x * 2, 0, -CHUNK_PAGE_SIZE.z - 1)),
    //         ivec3(0, 0, -CHUNK_PAGE_SIZE.z - 1).into()
    //     );

    //     assert_eq!(
    //         convert(ivec3(CHUNK_PAGE_SIZE.x * 2, 0, -CHUNK_PAGE_SIZE.z * 2 - 1)),
    //         ivec3(0, 0, -CHUNK_PAGE_SIZE.z - 1).into()
    //     );
    // }
}
