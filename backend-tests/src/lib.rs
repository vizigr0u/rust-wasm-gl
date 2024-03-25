mod position;

use position::*;

#[cfg(test)]
mod tests {
    use glam::{ivec2, ivec3, IVec3};

    use crate::{
        proper_modulo_i32, BlockPos, ChunkPos, PageChunkOffset, PagePos, CHUNK_PAGE_SIZE,
        CHUNK_SIZE,
    };

    #[test]
    fn test_page_pos_into_chunk_pos() {
        let convert = |v| Into::<ChunkPos>::into(Into::<PagePos>::into(v));
        assert_eq!(convert(ivec2(0, 0)), ivec3(0, 0, 0).into());
        assert_eq!(
            convert(ivec2(1, 2)),
            ivec3(CHUNK_PAGE_SIZE.x, 0, 2 * CHUNK_PAGE_SIZE.z).into()
        );
        assert_eq!(
            convert(ivec2(-2, -1)),
            ivec3(-CHUNK_PAGE_SIZE.x * 2, 0, -CHUNK_PAGE_SIZE.z).into()
        );
    }

    #[test]
    fn test_chunk_pos_get_first_block_pos() {
        let convert = |v| Into::<ChunkPos>::into(v).get_first_block();
        assert_eq!(convert(ivec3(0, 0, 0)), ivec3(0, 0, 0).into());
        assert_eq!(convert(ivec3(1, 0, 0)), ivec3(CHUNK_SIZE as _, 0, 0).into());
        assert_eq!(
            convert(ivec3(-1, 0, 0)),
            ivec3(-(CHUNK_SIZE as i32), 0, 0).into()
        );
        assert_eq!(
            convert(ivec3(1, 2, 3)),
            (ivec3(1, 2, 3) * CHUNK_SIZE as i32).into()
        );

        assert_eq!(
            convert(ivec3(-1, -2, -3)),
            (ivec3(-1, -2, -3) * CHUNK_SIZE as i32).into()
        );
    }

    #[test]
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

    #[test]
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
    fn test_chunk_pos_into_page_offset() {
        let convert = |v| Into::<PageChunkOffset>::into(Into::<ChunkPos>::into(v));
        assert_eq!(convert(ivec3(0, 1, 0)), ivec3(0, 1, 0).into());
        assert_eq!(convert(ivec3(0, 0, 0)), ivec3(0, 0, 0).into());

        assert_eq!(convert(ivec3(1, 0, 0)), ivec3(1, 0, 0).into());

        assert_eq!(
            convert(ivec3(CHUNK_PAGE_SIZE.x, 0, 0)),
            ivec3(0, 0, 0).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_PAGE_SIZE.x + 1, 0, 0)),
            ivec3(1, 0, 0).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_PAGE_SIZE.x * 2, 0, 0)),
            ivec3(0, 0, 0).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_PAGE_SIZE.x * 2, 0, -1)),
            ivec3(0, 0, -CHUNK_PAGE_SIZE.z - 1).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_PAGE_SIZE.x * 2, 0, -CHUNK_PAGE_SIZE.z)),
            ivec3(0, 0, 0).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_PAGE_SIZE.x * 2, 0, -CHUNK_PAGE_SIZE.z - 1)),
            ivec3(0, 0, -CHUNK_PAGE_SIZE.z - 1).into()
        );

        assert_eq!(
            convert(ivec3(CHUNK_PAGE_SIZE.x * 2, 0, -CHUNK_PAGE_SIZE.z * 2 - 1)),
            ivec3(0, 0, -CHUNK_PAGE_SIZE.z - 1).into()
        );
    }
}
