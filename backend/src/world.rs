use std::rc::Rc;

use glam::{UVec3, Vec3};
use log::info;

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
    gameobjects: Vec<GameObject>,
    graphics: Option<GraphicContext>,
}

impl World {
    pub fn random(size: UVec3) -> World {
        let num_chunks = (size.x * size.y * size.z) as usize;
        let mut chunks = Vec::with_capacity(num_chunks);
        for x in 0..size.x {
            for y in 0..size.y {
                for z in 0..size.z {
                    let mut chunk: Chunk = rand::random();
                    chunk.world_position = UVec3::new(x, y, z);
                    info!("chunk position: {:?}", chunk.world_position);
                    chunks.push(chunk);
                }
            }
        }

        World {
            chunks,
            size,
            gameobjects: Vec::with_capacity(num_chunks),
            graphics: None,
        }
    }

    pub fn set_graphics(
        &mut self,
        program: Rc<CompiledShader>,
        texture: Rc<(TextureType, glow::WebTextureKey)>,
    ) {
        self.graphics = Some(GraphicContext { program, texture });
    }

    pub fn load_all(&mut self, gl: &glow::Context) -> Result<(), String> {
        if let Some(graphics) = &self.graphics {
            for chunk in &self.chunks {
                self.gameobjects
                    .push(World::bake_chunk(gl, chunk, &graphics)?);
            }
        }
        Ok(())
    }

    fn bake_chunks(&mut self, gl: &glow::Context, max_count: usize) -> Result<(), String> {
        if let Some(graphics) = &self.graphics {
            let start_index = self.gameobjects.len();
            let mut max_index = start_index + max_count;
            if max_index > self.chunks.len() {
                max_index = self.chunks.len();
            }
            for i in start_index..max_index {
                let chunk = &self.chunks[i];
                self.gameobjects
                    .push(World::bake_chunk(gl, chunk, graphics)?)
            }
        }
        Ok(())
    }

    fn bake_chunk(
        gl: &glow::Context,
        chunk: &Chunk,
        graphics: &GraphicContext,
    ) -> Result<GameObject, String> {
        let mesh = Rc::new(chunk.to_mesh());
        let mut renderer = MeshRenderer::new(&graphics.program);
        renderer.set_mesh(gl, mesh)?;
        let renderer = Rc::new(renderer);
        let mut gameobject = GameObject::new(&graphics.texture, &renderer);
        gameobject.set_position(chunk.world_position.as_vec3() * CHUNK_SIZE as f32 * BLOCK_SIZE);
        gameobject.update();
        info!("game object position: {:?}", gameobject.get_position());
        Ok(gameobject)
    }

    pub fn update(&mut self, gl: &glow::Context, _time: &Time) {
        if self.gameobjects.len() < self.chunks.len() {
            if let Err(e) = self.bake_chunks(gl, 4) {
                info!("{}", e);
            }
        }
    }

    pub fn render(&self, gl: &glow::Context, camera: &Camera) {
        for go in &self.gameobjects {
            go.render(gl, camera);
        }
    }

    // pub fn are_meshes_loaded(&self) -> bool {
    //     self.gameobjects.len() == self.chunks.len()
    // }
}
