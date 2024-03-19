use std::{mem::size_of, rc::Rc};

use glam::UVec3;
use glow::{HasContext, WebVertexArrayKey};
use log::info;

use crate::{
    camera::Camera,
    chunk::{Chunk, CHUNK_SIZE},
    material::TextureType,
    mesh::VertexAttrType,
    shader_def,
    shaders::{CompiledShader, ShaderDef, UniformTypes},
    time::Time,
};

#[derive(Debug)]
struct GraphicContext {
    program: Rc<CompiledShader>,
    texture: Rc<(TextureType, glow::WebTextureKey)>,
}

#[derive(Debug)]
struct LoadedChunkMesh {
    pub vertex_array: WebVertexArrayKey,
    pub vertex_count: usize,
}

#[derive(Debug)]
pub struct World {
    chunks: Vec<Chunk>,
    size: UVec3,
    loaded_vertices: usize,
    loaded_meshes: Vec<LoadedChunkMesh>,
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
            loaded_meshes: Vec::with_capacity(num_chunks),
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
            self.loaded_meshes.len(),
            self.chunks.len(),
        )
    }

    pub fn setup_graphics(
        &mut self,
        gl: &glow::Context,
        texture: Rc<(TextureType, glow::WebTextureKey)>,
    ) -> Result<(), String> {
        let program = compile_shader(gl)?;
        self.graphics = Some(GraphicContext { program, texture });
        Ok(())
    }

    // pub fn load_all(&mut self, gl: &glow::Context) -> Result<(), String> {
    //     self.bake_chunks(gl, self.chunks.len())
    // }

    fn bake_chunks(&mut self, gl: &glow::Context, max_count: usize) -> Result<(), String> {
        let start_index = self.loaded_meshes.len();
        let mut max_index = start_index + max_count;
        if max_index > self.chunks.len() {
            max_index = self.chunks.len();
        }
        for i in start_index..max_index {
            let chunk = &self.chunks[i];
            let mesh = LoadedChunkMesh::load(gl, chunk)?;
            self.loaded_meshes.push(mesh)
        }
        self.loaded_vertices = self.loaded_meshes.iter().map(|m| m.vertex_count).sum();
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

    pub fn update(&mut self, gl: &glow::Context, _time: &Time) {
        let max_chunk_count = (self.size.x * self.size.y * self.size.z) as _;
        if self.chunks.len() < max_chunk_count {
            self.generate_chunks(64);
        } else {
            if self.loaded_meshes.len() < self.chunks.len() {
                if let Err(e) = self.bake_chunks(gl, 1) {
                    info!("{}", e);
                }
            }
        }
    }

    pub fn render(&self, gl: &glow::Context, camera: &Camera) {
        if let Some(graphics) = &self.graphics {
            unsafe {
                let program = &graphics.program;
                program.gl_use(gl);
                gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(graphics.texture.1));
                program.set_matrix(gl, UniformTypes::ViewMatrix, &camera.look_at);
                program.set_matrix(gl, UniformTypes::ProjMatrix, &camera.projection);
                let world_pos_position = program.get_uniform_location(UniformTypes::WorldPosition);

                for i in 0..self.loaded_meshes.len() {
                    let chunk = &self.chunks[i];
                    let mesh = &self.loaded_meshes[i];
                    let world_pos = chunk.world_position.as_vec3() * CHUNK_SIZE as f32;
                    gl.uniform_3_f32(world_pos_position, world_pos.x, world_pos.y, world_pos.z);

                    gl.bind_vertex_array(Some(mesh.vertex_array));
                    gl.draw_arrays(glow::TRIANGLES, 0, mesh.vertex_count as _);
                }
            }
        }
    }
}

impl LoadedChunkMesh {
    fn load(gl: &glow::Context, chunk: &Chunk) -> Result<Self, String> {
        let vertex_data = chunk.to_vertex_data();
        info!("chunk vertex count: {}", vertex_data.len());
        unsafe {
            let vao = gl.create_vertex_array()?;
            gl.bind_vertex_array(Some(vao));
            let vbo = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                vertex_data.align_to::<u8>().1,
                glow::STATIC_DRAW,
            );

            let location = 0;
            let size = 1;
            gl.vertex_attrib_pointer_i32(location, size, glow::INT, 0, 0);
            gl.enable_vertex_attrib_array(location);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            Ok(Self {
                vertex_array: vao,
                vertex_count: vertex_data.len(),
            })
        }
    }
}

fn compile_shader(gl: &glow::Context) -> Result<Rc<CompiledShader>, String> {
    unsafe {
        let program = shader_def!(
            "chunk.vert",
            "chunk.frag",
            vec!(),
            vec!(
                (UniformTypes::WorldPosition, "world_pos"),
                (UniformTypes::ViewMatrix, "view"),
                (UniformTypes::ProjMatrix, "projection"),
            )
        )
        .compile(gl)?;
        Ok(Rc::new(program))
    }
}
