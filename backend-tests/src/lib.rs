#[path = "../../backend/src/world/position.rs"]
mod position;

use position::*;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use glam::{ivec2, ivec3, vec3, IVec3};

    use crate::{
        BlockPos, ChunkPos, PageChunkOffset, BLOCKS_PER_CHUNK, PagePos, WorldPos, CHUNK_PAGE_SIZE, CHUNK_SIZE,
        MAX_BLOCK_Y, MAX_CHUNK_Y, MIN_BLOCK_Y, MIN_CHUNK_Y, NUM_CHUNKS_PER_PAGE,
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
        for d in [0.3, -0.3, 16.0, -16.0, 100.5, -100.5, -1e-7, 1e-7, 15.999, 33.0] {
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
