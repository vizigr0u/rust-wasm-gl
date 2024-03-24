use std::{collections::HashMap, rc::Rc};

use glam::IVec3;
use glow::{HasContext, WebVertexArrayKey};
use log::info;

use crate::{
    graphics::Camera,
    graphics::TextureType,
    graphics::{ShaderDef, ShaderProgram, UniformTypes},
    shader_def,
    world::{Chunk, CHUNK_SIZE},
};

use super::{BlockPos, ChunkPos};

const MAX_MESH_TO_KEEP: usize = 1024;

#[derive(Debug)]
struct GraphicContext {
    program: Rc<ShaderProgram>,
    texture: Rc<(TextureType, glow::WebTextureKey)>,
}

#[derive(Debug, Copy, Clone)]
pub struct ChunkVao {
    pub vertex_array: WebVertexArrayKey,
    pub vertex_count: usize,
}

#[derive(Debug)]
pub struct WorldRenderData {
    graphics: Option<GraphicContext>,
    chunks_to_draw: Vec<(ChunkPos, ChunkVao)>,
}

impl ChunkVao {
    pub fn load(gl: &glow::Context, chunk: &Chunk) -> Result<Self, String> {
        let vertex_data = chunk.to_vertex_data();
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

impl WorldRenderData {
    pub fn new() -> Self {
        Self {
            chunks_to_draw: Vec::with_capacity(MAX_MESH_TO_KEEP),
            graphics: None,
        }
    }

    pub fn compile(
        &mut self,
        gl: &glow::Context,
        player_chunk_pos: ChunkPos,
        loaded_chunks: &HashMap<ChunkPos, Option<ChunkVao>>,
    ) {
        self.chunks_to_draw.clear();
        for data in loaded_chunks
            .iter()
            .filter(|(_, c)| c.is_some())
            .map(|(pos, c)| (*pos, c.unwrap()))
        {
            self.chunks_to_draw.push(data);
        }
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

    pub fn render(&self, gl: &glow::Context, camera: &Camera) {
        if let Some(graphics) = &self.graphics {
            unsafe {
                let program = &graphics.program;
                program.gl_use(gl);
                gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(graphics.texture.1));
                program.set_matrix(gl, UniformTypes::ViewMatrix, &camera.look_at);
                program.set_matrix(gl, UniformTypes::ProjMatrix, &camera.projection);
                let world_pos_position = program.get_uniform_location(UniformTypes::WorldPosition);
                gl.enable(glow::CULL_FACE);
                gl.enable(glow::DEPTH_TEST);
                gl.disable(glow::BLEND);
                for (pos, vao) in self.chunks_to_draw.iter() {
                    let block_pos: BlockPos = (*pos).into();
                    let world_pos = block_pos.as_vec3();
                    gl.uniform_3_f32(world_pos_position, world_pos.x, world_pos.y, world_pos.z);

                    gl.bind_vertex_array(Some(vao.vertex_array));
                    gl.draw_arrays(glow::TRIANGLES, 0, vao.vertex_count as _);
                }
            }
        }
    }
}

fn compile_shader(gl: &glow::Context) -> Result<Rc<ShaderProgram>, String> {
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
