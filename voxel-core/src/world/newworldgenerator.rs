use glam::U16Vec3;

use super::{BlockType, Chunk, ChunkPos, WorldGenerator};

#[derive(Debug, Default)]
pub struct GeneratorV0;

impl GeneratorV0 {
    pub fn new() -> Self {
        Self
    }
}

impl WorldGenerator for GeneratorV0 {
    fn generate(&self, chunk_position: ChunkPos) -> Chunk {
        if chunk_position.as_vec().y < -2 || chunk_position.as_vec().y > 2 {
            return Chunk::empty();
        }
        let mut res = Chunk::empty(); // `set` clears the empty flag on first solid block
        let amplitude = 10.0;
        let frequency = 0.03;

        let first_block = chunk_position.get_origin_block_pos().as_vec();
        for block_pos in chunk_position.iter_block_pos() {
            let pos = block_pos.as_vec();
            let offset: U16Vec3 = (pos - first_block).as_u16vec3();
            // info!("{} {} {}", pos.x, pos.y, pos.z);
            let h = (amplitude
                * ((frequency * pos.x as f32).sin() + (frequency * pos.z as f32).sin()))
            .max(-3.0);
            let yf = block_pos.as_vec3().y;
            if yf <= h {
                let block = if h - yf <= 1.0 {
                    BlockType::Grass
                } else if h - yf <= 3.0 {
                    BlockType::Dirt
                } else if h - yf <= 7.0 {
                    BlockType::Stone
                } else {
                    BlockType::Empty
                };
                res.set(offset, block);
            }
        }
        res.compact();
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::IVec3;

    #[test]
    fn generated_chunk_emptiness_matches_content() {
        let g = GeneratorV0::new();
        // far above terrain: all air -> must be flagged empty
        let air = g.generate(IVec3::new(0, 2, 0).into());
        assert_eq!(
            air.is_empty(),
            air.iter_blocks().all(|b| b == BlockType::Empty)
        );
        // terrain chunk: must be non-empty
        let ground = g.generate(IVec3::new(0, -1, 0).into());
        assert!(ground.iter_blocks().any(|b| b != BlockType::Empty));
        assert!(!ground.is_empty());
    }
}
