use glam::{ivec2, ivec3, IVec2, IVec3};
use log::{info, warn};

use crate::{math::AABB, world::ChunkPos};

use super::{
    position::{PagePos, CHUNK_PAGE_SIZE, NUM_CHUNKS_PER_PAGE},
    Chunk, WorldGenerator,
};

const PAGE_LOAD_PER_FRAME: usize = 2;
const MAX_NUM_PAGES: usize = 16;

#[derive(Debug, Clone)]
pub struct ChunkPage {
    chunks: Vec<Chunk>,
    position: PagePos,
    content_bounds: AABB<IVec3>,
}

impl Default for ChunkPage {
    fn default() -> Self {
        Self {
            chunks: vec![Chunk::default(); NUM_CHUNKS_PER_PAGE],
            position: Default::default(),
            content_bounds: Default::default(),
        }
    }
}

impl ChunkPage {
    fn get_chunk(&self, chunk_pos: ChunkPos) -> Option<&Chunk> {
        let index = Self::get_chunk_index(chunk_pos);
        self.chunks.get(index)
    }

    fn get_chunk_index(chunk_pos: ChunkPos) -> usize {
        let p = chunk_pos.as_vec();
        (p.x + CHUNK_PAGE_SIZE.x * (p.y + CHUNK_PAGE_SIZE.y * p.z)) as usize
    }

    fn get_pos_at_index(index: usize) -> ChunkPos {
        ivec3(
            index as i32 % CHUNK_PAGE_SIZE.x,
            index as i32 / CHUNK_PAGE_SIZE.x % CHUNK_PAGE_SIZE.y,
            index as i32 / CHUNK_PAGE_SIZE.x / CHUNK_PAGE_SIZE.y,
        )
        .into()
    }

    // fn fill_from<G>(&mut self, generator: &mut G, page_index: PagePos)
    // where
    //     G: WorldGenerator,
    // {
    //     info!("Streaming: Filling chunk page {page_index:?}");
    //     let page_chunk_pos = page_index.0 * CHUNK_PAGE_SIZE as i32;
    //     self.position = page_index;
    //     for x in 0..CHUNK_PAGE_SIZE {
    //         for y in 0..CHUNK_PAGE_SIZE {
    //             for z in 0..CHUNK_PAGE_SIZE {
    //                 let chunk_index = self.get_chunk_index(ivec3(x as i32, y as i32, z as i32));
    //                 let world_chunk_pos = ivec3(
    //                     page_chunk_pos.x + x as i32,
    //                     y as i32,
    //                     page_chunk_pos.y + z as i32,
    //                 );
    //                 let chunk = generator.generate(&world_chunk_pos);

    //                 if !chunk.is_empty() {
    //                     self.content_bounds.add(world_chunk_pos);
    //                 }

    //                 self.chunks[chunk_index] = chunk;
    //             }
    //         }
    //     }
    // }

    fn fill_from<G>(&mut self, generator: &mut G, page_pos: PagePos)
    where
        G: WorldGenerator,
    {
        self.position = page_pos;
        info!("Streaming: Filling chunk page {page_pos:?}");
        let page_chunk_world_offset = Into::<ChunkPos>::into(page_pos).as_vec();
        for i in 0..NUM_CHUNKS_PER_PAGE {
            let chunk_offset = Self::get_pos_at_index(i).as_vec();
            let world_chunk_pos = ivec3(
                page_chunk_world_offset.x + chunk_offset.x,
                chunk_offset.y as i32,
                page_chunk_world_offset.y + chunk_offset.z,
            );
            let chunk = generator.generate(world_chunk_pos);
            if !chunk.is_empty() {
                self.content_bounds.add(world_chunk_pos);
            }
            self.chunks[i] = chunk;
        }
    }
}

#[derive(Debug)]
pub struct ChunkStreamer<G>
where
    G: WorldGenerator,
{
    loaded_chunk_pages: Vec<ChunkPage>,
    chunk_page_pool: Vec<ChunkPage>,
    generator: G,
    content_bounds: AABB<IVec3>,
    last_computed_page_pos: Option<PagePos>,
    pages_to_load: Vec<PagePos>,
}

const PAGE_OFFSETS_PRIORITY: [IVec2; 9] = [
    ivec2(0, 0),
    ivec2(0, -1),
    ivec2(0, 1),
    ivec2(1, 0),
    ivec2(-1, 0),
    ivec2(1, -1),
    ivec2(1, 1),
    ivec2(-1, 1),
    ivec2(-1, -1),
];

impl<G> ChunkStreamer<G>
where
    G: WorldGenerator,
{
    pub fn new(generator: G) -> Self {
        Self {
            loaded_chunk_pages: Vec::with_capacity(MAX_NUM_PAGES),
            chunk_page_pool: vec![ChunkPage::default(); MAX_NUM_PAGES],
            generator,
            content_bounds: Default::default(),
            last_computed_page_pos: None,
            pages_to_load: Vec::with_capacity(MAX_NUM_PAGES),
        }
    }

    pub fn get_info(&self) -> String {
        format!("{} page(s) loaded", self.loaded_chunk_pages.len())
    }

    pub fn is_chunked_streamed(&self, chunk_pos: ChunkPos) -> bool {
        self.get_page_for_chunk(chunk_pos).is_some()
    }

    pub fn get_chunk(&self, chunk_pos: ChunkPos) -> Option<&Chunk> {
        if let Some(page) = self.get_page_for_chunk(chunk_pos) {
            page.get_chunk(chunk_pos)
        } else {
            None
        }
    }

    pub fn tick_streaming(&mut self, player_chunk_pos: ChunkPos) -> i32 {
        let player_page_index: PagePos = player_chunk_pos.into();
        if self.last_computed_page_pos != Some(player_page_index) {
            self.last_computed_page_pos = Some(player_page_index);
            self.on_new_page_index(player_page_index, player_chunk_pos);
        }
        let mut new_pages_loaded = 0;
        if self.pages_to_load.len() > 0 {
            if self.chunk_page_pool.len() < self.pages_to_load.len() {
                info!("Streaming: Pool size too small - trying to free some pages");
                self.free_some_pages(player_page_index);
            }
            for _ in 0..PAGE_LOAD_PER_FRAME {
                if let Some(page) = self.pages_to_load.pop() {
                    self.load_page(page);
                    new_pages_loaded += 1;
                }
            }
        }
        new_pages_loaded
    }

    fn on_new_page_index(&mut self, page_index: PagePos, player_chunk_pos: ChunkPos) {
        let page_offsets: &[IVec2] = &PAGE_OFFSETS_PRIORITY;
        let page_offsets: &[IVec2] = &[ivec2(0, 0)];
        let new_pages_to_load: Vec<PagePos> = page_offsets
            .iter()
            .map(|offset| {
                ivec2(
                    page_index.as_vec().x + offset.x,
                    page_index.as_vec().y + offset.y,
                )
                .into()
            })
            .filter(|index| self.get_page_ref(*index).is_none())
            .collect();
        self.pages_to_load.extend(new_pages_to_load);
        // sort with best last so that we can pop
        self.pages_to_load.sort_by(|a, b| {
            b.get_center_chunk()
                .distance_squared(player_chunk_pos)
                .cmp(&a.get_center_chunk().distance_squared(player_chunk_pos))
        });
    }

    fn free_some_pages(&mut self, player_page_index: PagePos) {
        let max_dist_sq = (2 * CHUNK_PAGE_SIZE).length_squared();

        // Iterate over the loaded_chunk_pages vector in reverse to remove items while iterating
        let mut i = 0;
        let mut changed = false;
        while i < self.loaded_chunk_pages.len() {
            let page = &self.loaded_chunk_pages[i];
            let page_distance_sq = player_page_index
                .as_vec()
                .distance_squared(page.position.as_vec());

            // If the distance exceeds the threshold, move the ChunkPage to the pool
            if page_distance_sq > max_dist_sq {
                // Remove the ChunkPage from loaded_chunk_pages and insert it into chunk_page_pool
                let removed_page = self.loaded_chunk_pages.remove(i);
                self.chunk_page_pool.push(removed_page);
                changed = true;
            } else {
                // Move to the next ChunkPage
                i += 1;
            }
        }
        if changed {
            self.update_bounds();
        }
    }

    fn get_page_for_chunk(&self, chunk_pos: ChunkPos) -> Option<&ChunkPage> {
        let page_index: PagePos = chunk_pos.into();
        self.get_page_ref(page_index)
    }

    fn get_page_ref(&self, page_index: PagePos) -> Option<&ChunkPage> {
        self.loaded_chunk_pages
            .iter()
            .find(|p| p.position == page_index)
    }

    fn get_page(&mut self, page_index: PagePos) -> Option<&mut ChunkPage> {
        self.loaded_chunk_pages
            .iter_mut()
            .find(|p| p.position == page_index)
    }

    fn load_page(&mut self, page_index: PagePos)
    where
        G: WorldGenerator,
    {
        let mut page = self.get_pool_page();
        page.fill_from(&mut self.generator, page_index);
        self.loaded_chunk_pages.push(page);

        self.update_bounds();
    }

    fn update_bounds(&mut self) {
        self.content_bounds = Default::default();
        for page in &self.loaded_chunk_pages {
            self.content_bounds.add(page.content_bounds.min);
            self.content_bounds.add(page.content_bounds.max);
        }
        info!("Streaming: new content bounds: {:?}", self.content_bounds);
    }

    fn get_pool_page(&mut self) -> ChunkPage {
        self.chunk_page_pool.pop().expect("Pool is empty")
    }
}
