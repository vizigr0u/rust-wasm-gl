use std::{mem::size_of, rc::Rc};

use glam::{UVec3, Vec3};
use log::info;
use tracing::{span, Level};

use crate::{
    camera::Camera,
    chunk::{Chunk, BLOCK_SIZE, CHUNK_SIZE},
    gameobject::GameObject,
    material::TextureType,
    mesh::{Mesh, ToMesh},
    meshrenderer::MeshRenderer,
    shaders::CompiledShader,
    time::Time,
};

struct GraphicContext {
    program: Rc<CompiledShader>,
    texture: Rc<(TextureType, glow::WebTextureKey)>,
}

pub struct World {
    chunks: Vec<Chunk>,
    size: UVec3,
    loaded_vertices: usize,
    gameobjects: Vec<GameObject>,
    graphics: Option<GraphicContext>,
}

impl World {
    pub fn random(size: UVec3) -> World {
        let num_chunks = (size.x * size.y * size.z) as usize;
        let chunks = Vec::with_capacity(num_chunks);
        info!("Creating world with {} chunks", num_chunks);

        World {
            chunks,
            size,
            gameobjects: Vec::with_capacity(num_chunks),
            graphics: None,
            loaded_vertices: 0,
        }
    }

    pub fn get_info(&self) -> String {
        let max_chunks = (self.size.x * self.size.y * self.size.z) as usize;
        if self.chunks.len() < max_chunks {
            return format!("Loading...{}/{}", self.chunks.len(), max_chunks);
        }
        let vertex_count = self.loaded_vertices;
        let memory_used = (vertex_count * size_of::<f32>()) as f32 / 1000000.0;
        format!(
            "size: {}x{}x{} - {}/{} chunks\n{vertex_count} vertices - {memory_used:.3} MB",
            self.size.x,
            self.size.y,
            self.size.z,
            self.gameobjects.len(),
            self.chunks.len(),
        )
    }

    pub fn is_loading(&self) -> bool {
        self.chunks.len() == (self.size.x * self.size.y * self.size.z) as usize
    }

    pub fn set_graphics(
        &mut self,
        program: Rc<CompiledShader>,
        texture: Rc<(TextureType, glow::WebTextureKey)>,
    ) {
        self.graphics = Some(GraphicContext { program, texture });
    }

    pub fn load_all(&mut self, gl: &glow::Context) -> Result<(), String> {
        let mut vertex_count = 0;
        if let Some(graphics) = &self.graphics {
            for chunk in &self.chunks {
                self.gameobjects
                    .push(World::bake_chunk(gl, chunk, &graphics, &mut vertex_count)?);
            }
            self.loaded_vertices += vertex_count;
        }
        Ok(())
    }

    fn bake_chunks(&mut self, gl: &glow::Context, max_count: usize) -> Result<(), String> {
        let span = span!(Level::INFO, "bake_chunks");
        if let Some(graphics) = &self.graphics {
            let start_index = self.gameobjects.len();
            let mut max_index = start_index + max_count;
            if max_index > self.chunks.len() {
                max_index = self.chunks.len();
            }
            let mut vertex_count = 0;
            for i in start_index..max_index {
                let chunk = &self.chunks[i];
                self.gameobjects
                    .push(World::bake_chunk(gl, chunk, graphics, &mut vertex_count)?)
            }
            self.loaded_vertices += vertex_count;
        }
        Ok(())
    }

    fn generate_chunks(&mut self, max_count: usize) {
        let start_index = self.chunks.len();
        let mut max_index = start_index + max_count;
        let max_chunk_count = (self.size.x * self.size.y * self.size.z) as _;
        if max_index > max_chunk_count {
            max_index = max_chunk_count;
        }
        for i in start_index..max_index {
            let i = i as u32;
            let x = i % self.size.x;
            let y = (i / self.size.x) % self.size.y;
            let z = (i / self.size.x) / self.size.y;
            let mut chunk: Chunk = rand::random();
            chunk.world_position = UVec3::new(x, y, z);
            self.chunks.push(chunk);
        }
    }

    fn bake_chunk(
        gl: &glow::Context,
        chunk: &Chunk,
        graphics: &GraphicContext,
        vertex_count: &mut usize,
    ) -> Result<GameObject, String> {
        let mesh = Rc::new(chunk.to_mesh());
        *vertex_count += mesh.get_data().len();
        let mut renderer = MeshRenderer::new(&graphics.program);
        renderer.set_mesh(gl, mesh)?;
        let renderer = Rc::new(renderer);
        let mut gameobject = GameObject::new(&graphics.texture, &renderer);
        gameobject.set_position(chunk.world_position.as_vec3() * CHUNK_SIZE as f32 * BLOCK_SIZE);
        gameobject.update();
        Ok(gameobject)
    }

    pub fn update(&mut self, gl: &glow::Context, _time: &Time) {
        let max_chunk_count = (self.size.x * self.size.y * self.size.z) as _;
        if self.chunks.len() < max_chunk_count {
            self.generate_chunks(8);
        } else {
            if self.gameobjects.len() < self.chunks.len() {
                if let Err(e) = self.bake_chunks(gl, 4) {
                    info!("{}", e);
                }
            }
        }
    }

    pub fn render(&self, gl: &glow::Context, camera: &Camera) {
        for go in &self.gameobjects {
            go.render(gl, camera);
        }
    }
}
