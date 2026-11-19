use fastrand::Rng;
use glam::{ivec3, U16Vec3, Vec3};
use itertools::iproduct;

use super::Side;

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

impl From<u8> for BlockType {
    fn from(val: u8) -> Self {
        match val {
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

/// Block storage for one chunk. Air and single-block chunks cost no 4 KB array.
#[derive(Debug, Clone)]
pub enum ChunkData {
    /// All air. Never `Uniform(Empty)`.
    Empty,
    Uniform(BlockType),
    Dense(Box<[BlockType; BLOCKS_PER_CHUNK]>),
}

#[derive(Debug, Clone)]
pub struct Chunk {
    data: ChunkData,
}

impl Default for Chunk {
    fn default() -> Self {
        Chunk {
            data: ChunkData::Empty,
        }
    }
}

/// The 6 chunks touching a chunk's faces, indexed by `Side as usize`.
/// `None` means the neighbour is not available (treated as air by the mesher).
pub type Neighbours<'a> = [Option<&'a Chunk>; 6];
pub const NO_NEIGHBOURS: Neighbours<'static> = [None; 6];

impl Chunk {
    pub fn is_empty(&self) -> bool {
        matches!(self.data, ChunkData::Empty)
    }

    pub fn data(&self) -> &ChunkData {
        &self.data
    }

    pub fn random(rng: &mut Rng) -> Chunk {
        let mut res = Self::default();
        for i in 0..BLOCKS_PER_CHUNK {
            res.set_index(i, Into::<BlockType>::into(rng.u8(..9)));
        }
        res
    }

    pub fn empty() -> Chunk {
        Default::default()
    }

    pub fn plain(block: BlockType) -> Chunk {
        Chunk {
            data: if block == BlockType::Empty {
                ChunkData::Empty
            } else {
                ChunkData::Uniform(block)
            },
        }
    }

    pub fn set(&mut self, offset: U16Vec3, block: BlockType) {
        debug_assert!(offset.x < CHUNK_SIZE as u16);
        debug_assert!(offset.y < CHUNK_SIZE as u16);
        debug_assert!(offset.z < CHUNK_SIZE as u16);
        self.set_index(chunk_index_from_offset(&offset), block);
    }

    fn set_index(&mut self, index: usize, block: BlockType) {
        if self.block_at_index(index) == block {
            return;
        }
        let dense = self.make_dense();
        dense[index] = block;
    }

    fn make_dense(&mut self) -> &mut [BlockType; BLOCKS_PER_CHUNK] {
        if !matches!(self.data, ChunkData::Dense(_)) {
            let fill = match self.data {
                ChunkData::Uniform(b) => b,
                _ => BlockType::Empty,
            };
            self.data = ChunkData::Dense(Box::new([fill; BLOCKS_PER_CHUNK]));
        }
        match &mut self.data {
            ChunkData::Dense(d) => d,
            _ => unreachable!(),
        }
    }

    /// Demote to `Empty`/`Uniform` when every block is the same.
    pub fn compact(&mut self) {
        if let ChunkData::Dense(d) = &self.data {
            let first = d[0];
            if d.iter().all(|b| *b == first) {
                *self = Chunk::plain(first);
            }
        }
    }

    fn block_at_index(&self, index: usize) -> BlockType {
        match &self.data {
            ChunkData::Empty => BlockType::Empty,
            ChunkData::Uniform(b) => *b,
            ChunkData::Dense(d) => d[index],
        }
    }

    pub fn get_block(&self, offset: U16Vec3) -> BlockType {
        self.block_at_index(chunk_index_from_offset(&offset))
    }

    pub fn iter_blocks(&self) -> impl Iterator<Item = BlockType> + '_ {
        (0..BLOCKS_PER_CHUNK).map(|i| self.block_at_index(i))
    }

    /// Block at `offset` (may be 1 outside the chunk) looking through neighbours.
    /// Unavailable neighbour => air.
    fn block_with_neighbours(&self, n: &Neighbours, x: i32, y: i32, z: i32) -> BlockType {
        let s = CHUNK_SIZE as i32;
        let (side, wrap) = if x < 0 {
            (Side::Left, ivec3(x + s, y, z))
        } else if x >= s {
            (Side::Right, ivec3(x - s, y, z))
        } else if y < 0 {
            (Side::Bottom, ivec3(x, y + s, z))
        } else if y >= s {
            (Side::Top, ivec3(x, y - s, z))
        } else if z < 0 {
            (Side::Back, ivec3(x, y, z + s))
        } else if z >= s {
            (Side::Front, ivec3(x, y, z - s))
        } else {
            return self.get_block(U16Vec3::new(x as _, y as _, z as _));
        };
        match n[side as usize] {
            Some(c) => c.get_block(U16Vec3::new(wrap.x as _, wrap.y as _, wrap.z as _)),
            None => BlockType::Empty,
        }
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

    pub fn to_vertex_data(&self, neighbours: &Neighbours) -> [Vec<i32>; 6] {
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
            return result;
        }
        // (side, neighbour direction, is top/bottom)
        let faces = [
            (Side::Top, ivec3(0, 1, 0)),
            (Side::Bottom, ivec3(0, -1, 0)),
            (Side::Right, ivec3(1, 0, 0)),
            (Side::Left, ivec3(-1, 0, 0)),
            (Side::Front, ivec3(0, 0, 1)),
            (Side::Back, ivec3(0, 0, -1)),
        ];
        for (x, y, z) in iproduct!(0..CHUNK_SIZE, 0..CHUNK_SIZE, 0..CHUNK_SIZE) {
            let offset = U16Vec3::new(x as _, y as _, z as _);
            let (top, side, bottom) = match self.get_block(offset) {
                BlockType::Empty => continue,
                t => t.into(),
            };
            for (face, dir) in faces {
                let (nx, ny, nz) = (x as i32 + dir.x, y as i32 + dir.y, z as i32 + dir.z);
                if self.block_with_neighbours(neighbours, nx, ny, nz) == BlockType::Empty {
                    let tex = match face {
                        Side::Top => top,
                        Side::Bottom => bottom,
                        _ => side,
                    };
                    add_side_data(&mut result, face, tex, offset);
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

impl From<BlockType> for BlockSideTextures {
    fn from(val: BlockType) -> Self {
        match val {
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

/// Unit-cube corners of `side`'s quad in strip order (mesh-space reference for the shader).
pub fn side_quad_strip(side: Side) -> [Vec3; 4] {
    SIDE_VERTICES[side as usize].get_quad_triangles_strip()
}

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

fn pack_ivec_4bits(v: U16Vec3) -> i32 {
    debug_assert!(v.x < 16);
    debug_assert!(v.y < 16);
    debug_assert!(v.z < 16);
    (v.x | v.y << 4 | v.z << 8) as i32
}

pub fn make_side_quad_data(side: Side, texture: BlockSideTexture, offset: U16Vec3) -> i32 {
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
            result |= pos.x & 63;
            result |= (pos.y & 63) << 6;
            result |= (pos.z & 63) << 12;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn quads(c: &Chunk, n: &Neighbours) -> usize {
        c.to_vertex_data(n).iter().map(Vec::len).sum()
    }

    #[test]
    fn storage_is_sparse_until_written() {
        let mut c = Chunk::empty();
        assert!(c.is_empty());
        c.set(U16Vec3::ZERO, BlockType::Empty);
        assert!(matches!(c.data(), ChunkData::Empty));
        assert!(matches!(
            Chunk::plain(BlockType::Stone).data(),
            ChunkData::Uniform(BlockType::Stone)
        ));
        c.set(U16Vec3::new(1, 2, 3), BlockType::Dirt);
        assert!(!c.is_empty());
        assert_eq!(c.get_block(U16Vec3::new(1, 2, 3)), BlockType::Dirt);
        assert_eq!(c.get_block(U16Vec3::ZERO), BlockType::Empty);
        c.set(U16Vec3::new(1, 2, 3), BlockType::Empty);
        c.compact();
        assert!(c.is_empty());
    }

    #[test]
    fn mesher_culls_faces_against_neighbours() {
        let solid = Chunk::plain(BlockType::Stone);
        // alone: 6 faces of 16x16
        assert_eq!(quads(&solid, &NO_NEIGHBOURS), 6 * 256);
        // solid neighbour on +X hides that whole face
        let mut n = NO_NEIGHBOURS;
        n[Side::Right as usize] = Some(&solid);
        assert_eq!(quads(&solid, &n), 5 * 256);
        // empty neighbour hides nothing
        let air = Chunk::empty();
        n[Side::Right as usize] = Some(&air);
        assert_eq!(quads(&solid, &n), 6 * 256);
    }

    // ---- A4 snapshots ----

    use crate::testutil::hash_words;
    use crate::world::{GeneratorV0, WorldGenerator};
    use glam::IVec3;

    fn unpack(w: i32) -> (U16Vec3, i32, i32) {
        let pos = U16Vec3::new(
            (w & 15) as u16,
            ((w >> 4) & 15) as u16,
            ((w >> 8) & 15) as u16,
        );
        (pos, (w >> 12) & 7, (w >> 15) & 63)
    }

    #[test]
    fn packing_bit_layout_golden() {
        let cases = [
            (Side::Top, BlockSideTexture::Unknown, U16Vec3::new(0, 0, 0)),
            (
                Side::Left,
                BlockSideTexture::GrassTop,
                U16Vec3::new(15, 15, 15),
            ),
            (
                Side::Front,
                BlockSideTexture::Cobblestone,
                U16Vec3::new(1, 2, 3),
            ),
            (Side::Right, BlockSideTexture::Dirt2, U16Vec3::new(15, 0, 8)),
        ];
        let out: Vec<String> = cases
            .iter()
            .map(|(s, t, o)| {
                let w = make_side_quad_data(*s, *t, *o);
                format!("{s:?}/{t:?}/{o} -> {w:#010x} ({w:#034b})")
            })
            .collect();
        insta::assert_yaml_snapshot!(out);
    }

    #[test]
    fn packing_roundtrips_exhaustively() {
        let sides = [
            Side::Top,
            Side::Bottom,
            Side::Front,
            Side::Back,
            Side::Right,
            Side::Left,
        ];
        let textures = [
            BlockSideTexture::Unknown,
            BlockSideTexture::GrassSide,
            BlockSideTexture::Cobblestone,
            BlockSideTexture::RedStone,
            BlockSideTexture::TreeBark,
            BlockSideTexture::Sand,
            BlockSideTexture::Dirt,
            BlockSideTexture::Pickaxe,
            BlockSideTexture::TreeCenter,
            BlockSideTexture::GrassTop,
            BlockSideTexture::Coal,
            BlockSideTexture::Lava,
            BlockSideTexture::Diamond,
            BlockSideTexture::Iron,
            BlockSideTexture::Gold,
            BlockSideTexture::Dirt2,
        ];
        for (x, y, z) in iproduct!(0..16u16, 0..16u16, 0..16u16) {
            for s in sides {
                for t in textures {
                    let o = U16Vec3::new(x, y, z);
                    let w = make_side_quad_data(s, t, o);
                    assert!(w >= 0);
                    assert_eq!(unpack(w), (o, s as i32, t as i32));
                }
            }
        }
    }

    fn mesh_summary(c: &Chunk, n: &Neighbours) -> serde_like::Summary {
        let d = c.to_vertex_data(n);
        let all: Vec<i32> = d.iter().flatten().copied().collect();
        // no quad may face a solid block inside the chunk
        for (side, quads) in d.iter().enumerate() {
            let dir = [
                ivec3(0, 1, 0),
                ivec3(0, -1, 0),
                ivec3(0, 0, 1),
                ivec3(0, 0, -1),
                ivec3(1, 0, 0),
                ivec3(-1, 0, 0),
            ][side];
            for q in quads {
                let (o, s, _) = unpack(*q);
                assert_eq!(s as usize, side);
                assert_ne!(c.get_block(o), BlockType::Empty, "quad on air");
                let nb = o.as_ivec3() + dir;
                if (0..16).contains(&nb.x) && (0..16).contains(&nb.y) && (0..16).contains(&nb.z) {
                    assert_eq!(
                        c.get_block(U16Vec3::new(nb.x as _, nb.y as _, nb.z as _)),
                        BlockType::Empty,
                        "quad between two solids"
                    );
                }
            }
        }
        serde_like::Summary {
            per_side: d.iter().map(Vec::len).collect(),
            total: all.len(),
            hash: hash_words(&all),
        }
    }

    mod serde_like {
        #[derive(serde::Serialize)]
        pub struct Summary {
            pub per_side: Vec<usize>,
            pub total: usize,
            pub hash: String,
        }
    }

    fn single_block() -> Chunk {
        let mut c = Chunk::empty();
        c.set(U16Vec3::new(5, 6, 7), BlockType::Grass);
        c
    }

    fn checkerboard() -> Chunk {
        let mut c = Chunk::empty();
        for (x, y, z) in iproduct!(0..16u16, 0..16u16, 0..16u16) {
            if (x + y + z) % 2 == 0 {
                c.set(U16Vec3::new(x, y, z), BlockType::Stone);
            }
        }
        c
    }

    #[test]
    fn mesher_snapshots_synthetic() {
        let solid = Chunk::plain(BlockType::Stone);
        let out = [
            ("empty", mesh_summary(&Chunk::empty(), &NO_NEIGHBOURS)),
            ("full_stone", mesh_summary(&solid, &NO_NEIGHBOURS)),
            (
                "single_block",
                mesh_summary(&single_block(), &NO_NEIGHBOURS),
            ),
            (
                "checkerboard",
                mesh_summary(&checkerboard(), &NO_NEIGHBOURS),
            ),
        ];
        insta::assert_yaml_snapshot!(out);
    }

    #[test]
    fn mesher_snapshots_generator_with_and_without_neighbours() {
        let g = GeneratorV0::new();
        let mut out = Vec::new();
        for p in [
            IVec3::new(0, 0, 0),
            IVec3::new(3, -1, 2),
            IVec3::new(0, 2, 0),
        ] {
            let chunk = g.generate(p.into());
            let ns: Vec<Chunk> = [
                ivec3(0, 1, 0),
                ivec3(0, -1, 0),
                ivec3(0, 0, 1),
                ivec3(0, 0, -1),
                ivec3(1, 0, 0),
                ivec3(-1, 0, 0),
            ]
            .iter()
            .map(|d| g.generate((p + *d).into()))
            .collect();
            let n: Neighbours = std::array::from_fn(|i| Some(&ns[i]));
            let without = mesh_summary(&chunk, &NO_NEIGHBOURS);
            let with = mesh_summary(&chunk, &n);
            assert!(with.total <= without.total, "neighbours can only cull");
            out.push((format!("{p}"), without, with));
        }
        insta::assert_yaml_snapshot!(out);
    }
}
