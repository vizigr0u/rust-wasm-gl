use std::{collections::HashMap, rc::Rc};

use glam::IVec3;
use glow::{HasContext, WebBufferKey, WebVertexArrayKey};
use log::info;

use crate::{
    graphics::{Camera, ShaderDef, ShaderProgram, TextureType, UniformTypes, VertexAttrType},
    shader_def,
    world::{Chunk, CHUNK_SIZE},
};

use super::{make_base_quad_data, BlockPos, ChunkPos};

const MAX_MESH_TO_KEEP: usize = 1024;

#[derive(Debug)]
struct GraphicContext {
    program: Rc<ShaderProgram>,
    texture: Rc<(TextureType, glow::WebTextureKey)>,
}

#[derive(Debug, Clone)]
pub struct ChunkDrawData {
    pub vertex_data: Vec<i32>,
    pub chunk_pos: ChunkPos,
}

#[derive(Debug)]
pub struct WorldRenderData {
    graphics: Option<GraphicContext>,
    vertex_array: Option<WebVertexArrayKey>,
    num_verts_to_draw: i32,
}

impl ChunkDrawData {
    pub fn load(gl: &glow::Context, chunk: &Chunk, chunk_pos: ChunkPos) -> Result<Self, String> {
        let vertex_data = chunk.to_vertex_data();
        Ok(Self {
            vertex_data,
            chunk_pos,
        })
    }
}

impl WorldRenderData {
    pub fn new() -> Self {
        Self {
            graphics: None,
            vertex_array: None,
            num_verts_to_draw: 0,
        }
    }

    pub fn gather_draw_data(
        &mut self,
        gl: &glow::Context,
        player_chunk_pos: ChunkPos,
        loaded_chunks: &HashMap<ChunkPos, Option<ChunkDrawData>>,
    ) {
        for (pos, draw_data) in loaded_chunks
            .iter()
            .filter(|(_, d)| d.is_some())
            .filter(|(pos, _)| (**pos).as_vec() == IVec3::ZERO)
            .map(|(pos, d)| (*pos, d))
        {
            self.chunks_to_draw.push(draw_data);
        }
    }

    pub fn setup_graphics(
        &mut self,
        gl: &glow::Context,
        texture: Rc<(TextureType, glow::WebTextureKey)>,
    ) -> Result<(), String> {
        let program = compile_shader(gl)?;

        unsafe {
            program.gl_use(gl);

            let vao = Some(gl.create_vertex_array()?);
            gl.bind_vertex_array(vao);

            // upload quad data
            let vertices = make_base_quad_data();
            let vbo = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                vertices.align_to::<u8>().1,
                glow::STATIC_DRAW,
            );

            // setup vertex data attributes
            let vertex_data = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_data));

            let data_attr_pos = 1;
            gl.enable_vertex_attrib_array(data_attr_pos);
            gl.vertex_attrib_pointer_f32(data_attr_pos, 1, glow::INT, false, 0, 0);
            gl.vertex_attrib_divisor(data_attr_pos, 1);

            // setup vertex location attributes
            let chunk_location_data = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(chunk_location_data));

            let location_attr_pos = 2;
            gl.enable_vertex_attrib_array(location_attr_pos);
            gl.vertex_attrib_pointer_f32(location_attr_pos, 3, glow::FLOAT, false, 0, 0);
            gl.vertex_attrib_divisor(location_attr_pos, 1);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            self.vertex_array = vao;
        }
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
                gl.enable(glow::CULL_FACE);
                gl.enable(glow::DEPTH_TEST);
                gl.disable(glow::BLEND);
                gl.bind_vertex_array(self.vertex_array);
                gl.draw_arrays_instanced(glow::TRIANGLE_STRIP, 0, 4, self.num_verts_to_draw as i32);
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
                (UniformTypes::ViewMatrix, "view"),
                (UniformTypes::ProjMatrix, "projection"),
            )
        )
        .compile(gl)?;
        Ok(Rc::new(program))
    }
}
